use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoardInfo {
    pub model: String,
    pub board_type: JetsonModel,
    pub serial: String,
    pub l4t_version: String,
    pub jetpack_version: String,
    pub cuda_version: String,
    pub kernel_version: String,
    pub hostname: String,
    pub uptime: u64, // seconds
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum JetsonModel {
    #[default]
    Unknown,
    AGXOrin,
    OrinNX,
    OrinNano,
    Xavier,
    XavierNX,
    Nano,
    TX2,
    Other(String),
}

impl std::fmt::Display for JetsonModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JetsonModel::Unknown => write!(f, "Unknown"),
            JetsonModel::AGXOrin => write!(f, "AGX Orin"),
            JetsonModel::OrinNX => write!(f, "Orin NX"),
            JetsonModel::OrinNano => write!(f, "Orin Nano"),
            JetsonModel::Xavier => write!(f, "Xavier"),
            JetsonModel::XavierNX => write!(f, "Xavier NX"),
            JetsonModel::Nano => write!(f, "Nano"),
            JetsonModel::TX2 => write!(f, "TX2"),
            JetsonModel::Other(s) => write!(f, "{s}"),
        }
    }
}

fn parse_model(model_str: &str) -> JetsonModel {
    if model_str.contains("AGX Orin") || model_str.contains("agx-orin") {
        JetsonModel::AGXOrin
    } else if model_str.contains("Orin NX") || model_str.contains("orin-nx") {
        JetsonModel::OrinNX
    } else if model_str.contains("Orin Nano") || model_str.contains("orin-nano") {
        JetsonModel::OrinNano
    } else if model_str.contains("Xavier NX") {
        JetsonModel::XavierNX
    } else if model_str.contains("Xavier") {
        JetsonModel::Xavier
    } else if model_str.contains("Nano") {
        JetsonModel::Nano
    } else if model_str.contains("TX2") {
        JetsonModel::TX2
    } else {
        JetsonModel::Other(model_str.to_owned())
    }
}

fn read_l4t_version() -> String {
    // Primary: parse /etc/nv_tegra_release
    // Format: # R36 (release), REVISION: 4.3, ...
    if let Ok(content) = fs::read_to_string("/etc/nv_tegra_release") {
        let mut major = String::new();
        let mut revision = String::new();
        for line in content.lines() {
            let line = line.trim();
            // Extract major: "# R36 (release)" -> "36"
            if line.starts_with('#') {
                if let Some(r_pos) = line.find(" R") {
                    let after_r = &line[r_pos + 2..];
                    major = after_r
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_owned();
                }
            }
            // Extract revision: "REVISION: 4.3"
            if let Some(rev_pos) = line.find("REVISION:") {
                let after_rev = line[rev_pos + 9..].trim();
                revision = after_rev
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_owned();
            }
        }
        if !major.is_empty() && !revision.is_empty() {
            return format!("{}.{}", major, revision);
        }
    }

    // Fallback: dpkg-query
    if let Ok(output) = Command::new("dpkg-query")
        .args(["-W", "nvidia-l4t-core"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        // Output: "nvidia-l4t-core\t36.4.3-..."
        if let Some(line) = text.lines().next() {
            let mut parts = line.split_whitespace();
            parts.next(); // package name
            if let Some(ver) = parts.next() {
                // Strip trailing -... suffixes
                let ver_clean = ver.split('-').next().unwrap_or(ver).trim();
                if !ver_clean.is_empty() {
                    return ver_clean.to_owned();
                }
            }
        }
    }

    String::new()
}

fn l4t_to_jetpack(l4t: &str) -> String {
    // Extract major version number
    let major: u32 = l4t
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    match major {
        36 => {
            // L4T 36.x -> JetPack 6.x
            // 36.3 -> JP6.0, 36.4 -> JP6.1
            let minor: u32 = l4t
                .split('.')
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let jp_minor = if minor >= 3 { minor - 3 } else { 0 };
            format!("6.{}", jp_minor)
        }
        35 => "5.x".to_owned(),
        _ => "unknown".to_owned(),
    }
}

fn read_cuda_version() -> String {
    // Try /usr/local/cuda/version.json
    if let Ok(content) = fs::read_to_string("/usr/local/cuda/version.json") {
        // Parse JSON manually: look for "cuda" -> "version"
        // Minimal approach without full JSON parsing
        if let Some(cuda_pos) = content.find("\"cuda\"") {
            let after_cuda = &content[cuda_pos..];
            if let Some(ver_pos) = after_cuda.find("\"version\"") {
                let after_ver = &after_cuda[ver_pos + 9..];
                // Find the string value after the colon
                if let Some(colon_pos) = after_ver.find(':') {
                    let after_colon = after_ver[colon_pos + 1..].trim();
                    if after_colon.starts_with('"') {
                        let inner = &after_colon[1..];
                        if let Some(end_pos) = inner.find('"') {
                            return inner[..end_pos].to_owned();
                        }
                    }
                }
            }
        }
        // Also try top-level "version" key in the same file
        if let Some(ver_pos) = content.find("\"version\"") {
            let after_ver = &content[ver_pos + 9..];
            if let Some(colon_pos) = after_ver.find(':') {
                let after_colon = after_ver[colon_pos + 1..].trim();
                if after_colon.starts_with('"') {
                    let inner = &after_colon[1..];
                    if let Some(end_pos) = inner.find('"') {
                        return inner[..end_pos].to_owned();
                    }
                }
            }
        }
    }

    // Fallback: first line of /usr/local/cuda/version.txt
    // Format: "CUDA Version 12.2.140"
    if let Ok(content) = fs::read_to_string("/usr/local/cuda/version.txt") {
        if let Some(line) = content.lines().next() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Expect "CUDA Version X.Y.Z" or similar
            if let Some(ver) = parts.last() {
                if ver.contains('.') {
                    return ver.to_string();
                }
            }
        }
    }

    String::new()
}

fn read_kernel_version() -> String {
    fs::read_to_string("/proc/version")
        .ok()
        .and_then(|content| {
            // Format: "Linux version 5.15.122-tegra ..."
            let mut parts = content.split_whitespace();
            // skip "Linux" and "version"
            parts.next(); // "Linux"
            parts.next(); // "version"
            parts.next().map(|s| s.to_owned())
        })
        .unwrap_or_default()
}

fn read_uptime() -> u64 {
    fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|content| {
            content
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .map(|secs| secs as u64)
        .unwrap_or(0)
}

pub fn read_board_info() -> Result<BoardInfo> {
    // Model from /proc/device-tree/model (null-terminated)
    let model = fs::read_to_string("/proc/device-tree/model")
        .map(|s| s.trim_matches('\0').trim().to_owned())
        .unwrap_or_default();

    let board_type = parse_model(&model);

    // Serial number
    let serial = fs::read_to_string("/proc/device-tree/serial-number")
        .map(|s| s.trim_matches('\0').trim().to_owned())
        .unwrap_or_default();

    let l4t_version = read_l4t_version();
    let jetpack_version = if l4t_version.is_empty() {
        "unknown".to_owned()
    } else {
        l4t_to_jetpack(&l4t_version)
    };

    let cuda_version = read_cuda_version();
    let kernel_version = read_kernel_version();

    let hostname = fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_owned())
        .unwrap_or_default();

    let uptime = read_uptime();

    Ok(BoardInfo {
        model,
        board_type,
        serial,
        l4t_version,
        jetpack_version,
        cuda_version,
        kernel_version,
        hostname,
        uptime,
    })
}
