use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuStats {
    pub cores: Vec<CpuCore>,
    pub total_usage: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuCore {
    pub id: u32,
    pub online: bool,
    pub usage: f32,
    pub freq_cur: u64,  // kHz
    pub freq_min: u64,  // kHz
    pub freq_max: u64,  // kHz
    pub governor: String,
}

/// Stores previous (idle, total) per-core for delta calculation.
static PREV_STAT: OnceLock<Mutex<HashMap<u32, (u64, u64)>>> = OnceLock::new();

fn prev_stat() -> &'static Mutex<HashMap<u32, (u64, u64)>> {
    PREV_STAT.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Parse /proc/stat and return a map of core_id -> (idle, total).
fn parse_proc_stat() -> Result<HashMap<u32, (u64, u64)>> {
    let content = fs::read_to_string("/proc/stat")?;
    let mut map = HashMap::new();
    for line in content.lines() {
        if !line.starts_with("cpu") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        // Only per-core lines (cpu0, cpu1, …), skip aggregate "cpu" line.
        let label = parts[0];
        if label == "cpu" {
            continue;
        }
        let id: u32 = match label.trim_start_matches("cpu").parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        // Fields: user nice system idle iowait irq softirq steal (guest guest_nice optional)
        let values: Vec<u64> = parts[1..]
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        if values.len() < 4 {
            continue;
        }
        let idle = values[3]; // idle + iowait treated as idle
        let iowait = values.get(4).copied().unwrap_or(0);
        let idle_total = idle + iowait;
        let total: u64 = values.iter().sum();
        map.insert(id, (idle_total, total));
    }
    Ok(map)
}

/// Count how many cpuN directories exist under /sys/devices/system/cpu/.
fn count_cpu_cores() -> u32 {
    let mut count = 0u32;
    loop {
        let path = format!("/sys/devices/system/cpu/cpu{}", count);
        if std::path::Path::new(&path).exists() {
            count += 1;
        } else {
            break;
        }
    }
    // Always at least 1 (cpu0).
    count.max(1)
}

fn read_sysfs_u64(path: &str) -> u64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn read_sysfs_string(path: &str) -> String {
    fs::read_to_string(path)
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

pub fn read_cpu_stats() -> Result<CpuStats> {
    let num_cores = count_cpu_cores();

    // Snapshot current /proc/stat.
    let current = parse_proc_stat()?;

    // Compute per-core usage using deltas against previous snapshot.
    let mut prev_guard = prev_stat().lock().unwrap();

    let mut cores = Vec::with_capacity(num_cores as usize);
    let mut online_usages: Vec<f32> = Vec::new();

    for id in 0..num_cores {
        // Online status: cpu0 is always online; others read from sysfs.
        let online = if id == 0 {
            true
        } else {
            let online_path = format!("/sys/devices/system/cpu/cpu{}/online", id);
            read_sysfs_u64(&online_path) != 0
        };

        // Frequency info.
        let base = format!("/sys/devices/system/cpu/cpu{}/cpufreq", id);
        let freq_cur = read_sysfs_u64(&format!("{}/scaling_cur_freq", base));
        let freq_min = read_sysfs_u64(&format!("{}/scaling_min_freq", base));
        let freq_max = read_sysfs_u64(&format!("{}/scaling_max_freq", base));
        let governor = read_sysfs_string(&format!("{}/scaling_governor", base));

        // CPU usage via delta.
        let usage = if let Some(&(cur_idle, cur_total)) = current.get(&id) {
            if let Some(&(prev_idle, prev_total)) = prev_guard.get(&id) {
                let d_total = cur_total.saturating_sub(prev_total);
                let d_idle = cur_idle.saturating_sub(prev_idle);
                if d_total == 0 {
                    0.0
                } else {
                    100.0 * (1.0 - d_idle as f32 / d_total as f32)
                }
            } else {
                // First read — no delta available yet.
                0.0
            }
        } else {
            0.0
        };

        if online {
            online_usages.push(usage);
        }

        cores.push(CpuCore {
            id,
            online,
            usage,
            freq_cur,
            freq_min,
            freq_max,
            governor,
        });
    }

    // Update stored snapshot.
    *prev_guard = current;
    drop(prev_guard);

    let total_usage = if online_usages.is_empty() {
        0.0
    } else {
        online_usages.iter().sum::<f32>() / online_usages.len() as f32
    };

    Ok(CpuStats { cores, total_usage })
}
