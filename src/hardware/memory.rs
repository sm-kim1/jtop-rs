use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub ram: MemoryInfo,
    pub swap: MemoryInfo,
    pub zram: Option<ZramInfo>,
    pub emc: Option<EmcInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,   // bytes
    pub used: u64,    // bytes
    pub free: u64,    // bytes
    pub cached: u64,  // bytes
    pub buffers: u64, // bytes
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZramInfo {
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmcInfo {
    pub freq_cur: u64,
    pub freq_max: u64,
    pub usage: f32,
}

fn read_sysfs(path: &str) -> Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

pub fn read_memory_stats() -> Result<MemoryStats> {
    // --- /proc/meminfo ---
    let meminfo = fs::read_to_string("/proc/meminfo")?;
    let mut mem_total_kb: u64 = 0;
    let mut mem_free_kb: u64 = 0;
    let mut mem_available_kb: u64 = 0;
    let mut buffers_kb: u64 = 0;
    let mut cached_kb: u64 = 0;
    let mut swap_total_kb: u64 = 0;
    let mut swap_free_kb: u64 = 0;

    for line in meminfo.lines() {
        let mut parts = line.split_whitespace();
        let key = match parts.next() {
            Some(k) => k,
            None => continue,
        };
        let value: u64 = match parts.next().and_then(|v| v.parse().ok()) {
            Some(v) => v,
            None => continue,
        };
        match key {
            "MemTotal:" => mem_total_kb = value,
            "MemFree:" => mem_free_kb = value,
            "MemAvailable:" => mem_available_kb = value,
            "Buffers:" => buffers_kb = value,
            "Cached:" => cached_kb = value,
            "SwapTotal:" => swap_total_kb = value,
            "SwapFree:" => swap_free_kb = value,
            _ => {}
        }
    }

    let ram_total = mem_total_kb * 1024;
    let ram_free = mem_available_kb * 1024;
    let ram_used = if mem_available_kb <= mem_total_kb {
        (mem_total_kb - mem_available_kb) * 1024
    } else {
        // fallback: total - free - buffers - cached
        mem_total_kb
            .saturating_sub(mem_free_kb)
            .saturating_sub(buffers_kb)
            .saturating_sub(cached_kb)
            * 1024
    };

    let ram = MemoryInfo {
        total: ram_total,
        used: ram_used,
        free: ram_free,
        cached: cached_kb * 1024,
        buffers: buffers_kb * 1024,
    };

    let swap_total = swap_total_kb * 1024;
    let swap_free = swap_free_kb * 1024;
    let swap_used = swap_total.saturating_sub(swap_free);
    let swap = MemoryInfo {
        total: swap_total,
        used: swap_used,
        free: swap_free,
        cached: 0,
        buffers: 0,
    };

    // --- ZRAM: /sys/block/zram0/mm_stat ---
    let zram = read_zram();

    // --- EMC ---
    let emc = read_emc();

    Ok(MemoryStats { ram, swap, zram, emc })
}

fn read_zram() -> Option<ZramInfo> {
    // Try mm_stat first: orig_data_size compr_data_size mem_used_total ...
    if let Ok(content) = fs::read_to_string("/sys/block/zram0/mm_stat") {
        let mut parts = content.split_whitespace();
        let orig: u64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let compr: u64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        return Some(ZramInfo { total: orig, used: compr });
    }
    // Fallback: disksize gives total only
    if let Ok(content) = fs::read_to_string("/sys/block/zram0/disksize") {
        let total: u64 = content.trim().parse().unwrap_or(0);
        return Some(ZramInfo { total, used: 0 });
    }
    None
}

fn read_emc() -> Option<EmcInfo> {
    let usage_str = read_sysfs("/sys/kernel/actmon_avg_activity/mc_all").ok()?;
    let usage: f32 = usage_str.parse().ok()?;

    const EMC_DEVFREQ: &str =
        "/sys/devices/platform/bus@0/31d0000.emc/devfreq/31d0000.emc";

    let freq_cur: u64 = read_sysfs(&format!("{}/cur_freq", EMC_DEVFREQ))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let freq_max: u64 = read_sysfs(&format!("{}/max_freq", EMC_DEVFREQ))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Some(EmcInfo { freq_cur, freq_max, usage })
}
