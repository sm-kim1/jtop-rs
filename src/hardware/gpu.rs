use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuStats {
    pub usage: f32,       // percentage
    pub freq_cur: u64,    // Hz
    pub freq_min: u64,    // Hz
    pub freq_max: u64,    // Hz
    pub governor: String,
}

/// Try to read a sysfs file, returning None if it does not exist or is unreadable.
fn try_read_u64(path: &str) -> Option<u64> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn try_read_string(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_owned())
}

/// Walk /sys/devices/platform/*/devfreq/*/load looking for the GPU devfreq entry.
/// Returns the path to the devfreq directory if found.
fn find_gpu_devfreq_dir() -> Option<String> {
    let platform = Path::new("/sys/devices/platform");
    let platform_entries = fs::read_dir(platform).ok()?;
    for entry in platform_entries.flatten() {
        let devfreq_dir = entry.path().join("devfreq");
        if !devfreq_dir.is_dir() {
            continue;
        }
        let devfreq_entries = fs::read_dir(&devfreq_dir).ok();
        for df_entry in devfreq_entries.into_iter().flatten().flatten() {
            let load_path = df_entry.path().join("load");
            if load_path.exists() {
                return Some(df_entry.path().to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Read GPU load percentage (0.0 – 100.0).
/// Tries several known sysfs paths in priority order.
fn read_gpu_load() -> f32 {
    // Primary path for Orin series.
    let primary = "/sys/devices/platform/bus@0/17000000.gpu/load";
    // Fallback for older Jetson models.
    let fallback1 = "/sys/devices/gpu.0/load";

    let raw = try_read_u64(primary)
        .or_else(|| try_read_u64(fallback1))
        .or_else(|| {
            // Dynamic glob fallback.
            find_gpu_devfreq_dir().and_then(|dir| {
                try_read_u64(&format!("{}/load", dir))
            })
        });

    // Value is 0–1000; divide by 10 for percentage.
    raw.map(|v| v as f32 / 10.0).unwrap_or(0.0)
}

/// Returns the devfreq sysfs directory for the GPU, trying multiple paths.
fn gpu_devfreq_dir() -> Option<String> {
    // Orin primary.
    let primary = "/sys/devices/platform/bus@0/17000000.gpu/devfreq/17000000.gpu";
    if Path::new(primary).is_dir() {
        return Some(primary.to_owned());
    }
    // Dynamic search.
    find_gpu_devfreq_dir()
}

pub fn read_gpu_stats() -> Result<GpuStats> {
    let usage = read_gpu_load();

    let (freq_cur, freq_min, freq_max, governor) = if let Some(dir) = gpu_devfreq_dir() {
        let cur = try_read_u64(&format!("{}/cur_freq", dir)).unwrap_or(0);
        let min = try_read_u64(&format!("{}/min_freq", dir)).unwrap_or(0);
        let max = try_read_u64(&format!("{}/max_freq", dir)).unwrap_or(0);
        let gov = try_read_string(&format!("{}/governor", dir)).unwrap_or_default();
        (cur, min, max, gov)
    } else {
        // No devfreq directory found; return zeros rather than hard-failing.
        // On non-Jetson hosts (dev machines) this is the expected path.
        (0, 0, 0, String::new())
    };

    // If we couldn't read anything meaningful from hardware, surface a soft error
    // only when usage is 0 AND all frequencies are 0 AND we are likely on a real
    // Jetson (i.e., /sys/devices/platform/bus@0 exists but the GPU path is missing).
    if usage == 0.0 && freq_cur == 0 && Path::new("/sys/devices/platform/bus@0").exists() {
        let gpu_path = "/sys/devices/platform/bus@0/17000000.gpu";
        if !Path::new(gpu_path).exists() {
            return Err(anyhow!(
                "GPU sysfs path not found; device may not have a discrete GPU or paths have changed"
            ));
        }
    }

    Ok(GpuStats {
        usage,
        freq_cur,
        freq_min,
        freq_max,
        governor,
    })
}
