use std::io::Read;
use std::net::TcpStream;
use std::path::Path;

use anyhow::{Context, Result};
use ssh2::Session;

use crate::hardware::cpu::{CpuCore, CpuStats};
use crate::hardware::fan::{FanInfo, FanStats};
use crate::hardware::gpu::GpuStats;
use crate::hardware::jetson::{BoardInfo, JetsonModel};
use crate::hardware::memory::{MemoryInfo, MemoryStats};
use crate::hardware::power::PowerStats;
use crate::hardware::temperature::{TempStats, ThermalZone};
use crate::hardware::{HardwareReader, JetsonStats};

const SEP: &str = "---SEP---";

/// Remote hardware reader via SSH
pub struct SshReader {
    host: String,
    user: String,
    port: u16,
    key_path: Option<String>,
    session: Option<Session>,
}

impl SshReader {
    pub fn new(host: String, user: String, port: u16, key_path: Option<String>) -> Self {
        Self {
            host,
            user,
            port,
            key_path,
            session: None,
        }
    }

    fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let tcp = TcpStream::connect(&addr)
            .with_context(|| format!("Failed to connect to {addr}"))?;
        tcp.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;

        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;

        // Try authentication methods in order
        let mut authenticated = false;

        // 1. Explicit key path
        if let Some(ref key) = self.key_path {
            if sess.userauth_pubkey_file(&self.user, None, Path::new(key), None).is_ok() {
                authenticated = sess.authenticated();
            }
        }

        // 2. Try common SSH key paths
        if !authenticated {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            let key_candidates = [
                format!("{home}/.ssh/id_ed25519"),
                format!("{home}/.ssh/id_rsa"),
                format!("{home}/.ssh/id_ecdsa"),
            ];
            for key_path in &key_candidates {
                let p = Path::new(key_path);
                if p.exists() {
                    if sess.userauth_pubkey_file(&self.user, None, p, None).is_ok() && sess.authenticated() {
                        authenticated = true;
                        tracing::info!("SSH authenticated with {}", key_path);
                        break;
                    }
                }
            }
        }

        // 3. Try SSH agent (if available)
        if !authenticated {
            if sess.userauth_agent(&self.user).is_ok() {
                authenticated = sess.authenticated();
            }
        }

        // 4. Try password authentication (interactive prompt)
        if !authenticated {
            eprintln!("SSH key/agent authentication failed. Trying password authentication.");
            let prompt = format!("{}@{}'s password: ", self.user, self.host);
            match rpassword::read_password_from_tty(Some(&prompt)) {
                Ok(password) => {
                    if sess.userauth_password(&self.user, &password).is_ok() {
                        authenticated = sess.authenticated();
                        if authenticated {
                            tracing::info!("SSH authenticated with password");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read password: {e}");
                }
            }
        }

        if !authenticated {
            anyhow::bail!(
                "SSH authentication failed for {}@{}. Try: jtop-rs -H {} -u {} -k ~/.ssh/id_rsa",
                self.user, self.host, self.host, self.user
            );
        }

        self.session = Some(sess);
        Ok(())
    }

    fn session(&mut self) -> Result<&Session> {
        if self.session.is_none() {
            self.connect()?;
        }
        self.session.as_ref().ok_or_else(|| anyhow::anyhow!("No SSH session"))
    }
}

fn exec_remote(session: &Session, cmd: &str) -> Result<String> {
    let mut channel = session.channel_session()?;
    channel.exec(cmd)?;
    let mut output = String::new();
    channel.read_to_string(&mut output)?;
    channel.wait_close()?;
    Ok(output)
}

impl HardwareReader for SshReader {
    fn read_stats(&mut self) -> Result<JetsonStats> {
        let session = self.session()?.clone();

        // Batch command to read all stats in one SSH roundtrip
        let batch_cmd = [
            "cat /proc/stat 2>/dev/null",
            "echo '---SEP---'",
            // CPU frequencies: one line per core
            "for i in /sys/devices/system/cpu/cpu[0-9]*/cpufreq/scaling_cur_freq; do cat $i 2>/dev/null; done",
            "echo '---SEP---'",
            // GPU load
            "cat /sys/devices/platform/bus@0/17000000.gpu/load 2>/dev/null || cat /sys/devices/gpu.0/load 2>/dev/null || echo 0",
            "echo '---SEP---'",
            // GPU devfreq cur_freq
            "cat /sys/devices/platform/bus@0/17000000.gpu/devfreq/17000000.gpu/cur_freq 2>/dev/null || echo 0",
            "echo '---SEP---'",
            // Memory
            "cat /proc/meminfo 2>/dev/null",
            "echo '---SEP---'",
            // Temperatures
            "for z in /sys/devices/virtual/thermal/thermal_zone*; do echo \"$(cat $z/type 2>/dev/null):$(cat $z/temp 2>/dev/null)\"; done",
            "echo '---SEP---'",
            // Fan
            "for f in /sys/class/hwmon/hwmon*/pwm1; do d=$(dirname $f); echo \"$(cat $d/name 2>/dev/null):$(cat $f 2>/dev/null):$(cat ${d}/fan1_input 2>/dev/null || echo 0)\"; done",
            "echo '---SEP---'",
            // Board model
            "cat /proc/device-tree/model 2>/dev/null | tr -d '\\0'",
            "echo '---SEP---'",
            // Hostname
            "cat /etc/hostname 2>/dev/null",
            "echo '---SEP---'",
            // Uptime
            "cat /proc/uptime 2>/dev/null",
            "echo '---SEP---'",
            // Kernel
            "uname -r 2>/dev/null",
        ].join("; ");

        let output = exec_remote(&session, &batch_cmd)?;
        let sections: Vec<&str> = output.split(SEP).collect();

        let cpu = parse_cpu_section(sections.first().copied().unwrap_or(""), sections.get(1).copied().unwrap_or(""));
        let gpu = parse_gpu_section(sections.get(2).copied().unwrap_or(""), sections.get(3).copied().unwrap_or(""));
        let memory = parse_memory_section(sections.get(4).copied().unwrap_or(""));
        let temperature = parse_temp_section(sections.get(5).copied().unwrap_or(""));
        let fan = parse_fan_section(sections.get(6).copied().unwrap_or(""));
        let board = parse_board_section(
            sections.get(7).copied().unwrap_or(""),
            sections.get(8).copied().unwrap_or(""),
            sections.get(9).copied().unwrap_or(""),
            sections.get(10).copied().unwrap_or(""),
        );

        Ok(JetsonStats {
            cpu,
            gpu,
            memory,
            temperature,
            fan,
            power: PowerStats::default(), // INA3221 enumeration too complex for batch
            processes: Vec::new(),         // Process list requires iterating /proc remotely
            board,
        })
    }
}

fn parse_cpu_section(stat_output: &str, freq_output: &str) -> CpuStats {
    let mut cores = Vec::new();
    let freqs: Vec<u64> = freq_output
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect();

    for line in stat_output.lines() {
        if !line.starts_with("cpu") || line.starts_with("cpu ") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let id: u32 = match parts[0].trim_start_matches("cpu").parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let freq_cur = freqs.get(id as usize).copied().unwrap_or(0);
        cores.push(CpuCore {
            id,
            online: true,
            usage: 0.0, // Delta calculation not available for single-shot remote reads
            freq_cur,
            freq_min: 0,
            freq_max: 0,
            governor: String::new(),
        });
    }

    let total_usage = 0.0;
    CpuStats { cores, total_usage }
}

fn parse_gpu_section(load_output: &str, freq_output: &str) -> GpuStats {
    let load_raw: u64 = load_output.trim().parse().unwrap_or(0);
    let usage = load_raw as f32 / 10.0;
    let freq_cur: u64 = freq_output.trim().parse().unwrap_or(0);

    GpuStats {
        usage,
        freq_cur,
        freq_min: 0,
        freq_max: 0,
        governor: String::new(),
    }
}

fn parse_memory_section(meminfo: &str) -> MemoryStats {
    let mut total: u64 = 0;
    let mut _free: u64 = 0;
    let mut available: u64 = 0;
    let mut buffers: u64 = 0;
    let mut cached: u64 = 0;
    let mut swap_total: u64 = 0;
    let mut swap_free: u64 = 0;

    for line in meminfo.lines() {
        let mut parts = line.split_whitespace();
        let key = match parts.next() { Some(k) => k, None => continue };
        let val: u64 = match parts.next().and_then(|v| v.parse().ok()) { Some(v) => v, None => continue };
        match key {
            "MemTotal:" => total = val,
            "MemFree:" => _free = val,
            "MemAvailable:" => available = val,
            "Buffers:" => buffers = val,
            "Cached:" => cached = val,
            "SwapTotal:" => swap_total = val,
            "SwapFree:" => swap_free = val,
            _ => {}
        }
    }

    let ram_total = total * 1024;
    let ram_used = (total.saturating_sub(available)) * 1024;
    let ram_free = available * 1024;

    MemoryStats {
        ram: MemoryInfo {
            total: ram_total,
            used: ram_used,
            free: ram_free,
            cached: cached * 1024,
            buffers: buffers * 1024,
        },
        swap: MemoryInfo {
            total: swap_total * 1024,
            used: (swap_total.saturating_sub(swap_free)) * 1024,
            free: swap_free * 1024,
            cached: 0,
            buffers: 0,
        },
        zram: None,
        emc: None,
    }
}

fn parse_temp_section(output: &str) -> TempStats {
    let mut zones = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, ':');
        let name = parts.next().unwrap_or("").to_string();
        let temp_raw: i64 = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
        let temp = temp_raw as f32 / 1000.0;
        zones.push(ThermalZone {
            name: name.clone(),
            temp,
            zone_type: name,
            trip_points: Vec::new(),
        });
    }
    TempStats { zones }
}

fn parse_fan_section(output: &str) -> FanStats {
    let mut fans = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].to_string();
        let pwm: u32 = parts.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        let rpm: u32 = parts.get(2).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        fans.push(FanInfo {
            name: if name.is_empty() { "Fan".to_string() } else { name },
            speed: rpm,
            pwm,
            pwm_max: 255,
            profile: "unknown".to_string(),
        });
    }
    FanStats { fans }
}

fn parse_board_section(model: &str, hostname: &str, uptime: &str, kernel: &str) -> BoardInfo {
    let model = model.trim().to_string();
    let board_type = if model.contains("AGX Orin") {
        JetsonModel::AGXOrin
    } else if model.contains("Orin NX") {
        JetsonModel::OrinNX
    } else if model.contains("Orin Nano") {
        JetsonModel::OrinNano
    } else if model.is_empty() {
        JetsonModel::Unknown
    } else {
        JetsonModel::Other(model.clone())
    };

    let uptime_secs: u64 = uptime
        .trim()
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .map(|f| f as u64)
        .unwrap_or(0);

    BoardInfo {
        model,
        board_type,
        serial: String::new(),
        l4t_version: String::new(),
        jetpack_version: String::new(),
        cuda_version: String::new(),
        kernel_version: kernel.trim().to_string(),
        hostname: hostname.trim().to_string(),
        uptime: uptime_secs,
    }
}
