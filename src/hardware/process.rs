use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub mem_usage: u64,   // bytes
    pub gpu_usage: Option<f32>,
    pub state: String,
    pub user: String,
}

/// Previous (utime+stime, system_total) per PID for CPU delta calculation.
static PREV_PROC: OnceLock<Mutex<HashMap<u32, (u64, u64)>>> = OnceLock::new();

fn prev_proc() -> &'static Mutex<HashMap<u32, (u64, u64)>> {
    PREV_PROC.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Read total system CPU time from /proc/stat (aggregate "cpu" line).
fn read_system_total() -> u64 {
    let content = match fs::read_to_string("/proc/stat") {
        Ok(c) => c,
        Err(_) => return 1, // avoid divide-by-zero
    };
    for line in content.lines() {
        if line.starts_with("cpu ") {
            let total: u64 = line
                .split_whitespace()
                .skip(1)
                .filter_map(|s| s.parse::<u64>().ok())
                .sum();
            return total;
        }
    }
    1
}

/// Parse /proc/{pid}/stat.
/// Returns (comm, state, utime, stime) or None on failure.
fn parse_proc_stat(pid: u32) -> Option<(String, String, u64, u64)> {
    let content = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    // comm may contain spaces and is wrapped in parentheses.
    let open = content.find('(')?;
    let close = content.rfind(')')?;
    let comm = content[open + 1..close].to_owned();
    let rest: Vec<&str> = content[close + 2..].split_whitespace().collect();
    // After ')': state(0) ppid(1) pgrp(2) session(3) ... utime(11) stime(12)
    let state = rest.first().map(|s| s.to_string()).unwrap_or_default();
    let utime: u64 = rest.get(11).and_then(|s| s.parse().ok()).unwrap_or(0);
    let stime: u64 = rest.get(12).and_then(|s| s.parse().ok()).unwrap_or(0);
    Some((comm, state, utime, stime))
}

/// Parse VmRSS and Uid from /proc/{pid}/status.
/// Returns (vmrss_bytes, uid) or None.
fn parse_proc_status(pid: u32) -> Option<(u64, u32)> {
    let content = fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
    let mut vmrss_kb: u64 = 0;
    let mut uid: u32 = 0;
    for line in content.lines() {
        if line.starts_with("VmRSS:") {
            vmrss_kb = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("Uid:") {
            uid = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }
    Some((vmrss_kb * 1024, uid))
}

/// Build a UID -> username map by parsing /etc/passwd.
fn build_uid_map() -> HashMap<u32, String> {
    let mut map = HashMap::new();
    if let Ok(content) = fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(uid) = parts[2].parse::<u32>() {
                    map.insert(uid, parts[0].to_owned());
                }
            }
        }
    }
    map
}

pub fn read_processes() -> Result<Vec<ProcessInfo>> {
    let system_total = read_system_total();
    let uid_map = build_uid_map();
    let mut prev_guard = prev_proc().lock().unwrap();
    let mut new_prev: HashMap<u32, (u64, u64)> = HashMap::new();
    let mut processes: Vec<ProcessInfo> = Vec::new();

    let proc_dir = fs::read_dir("/proc")?;
    for entry in proc_dir.flatten() {
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        let pid: u32 = match name_str.parse() {
            Ok(n) => n,
            Err(_) => continue, // skip non-numeric entries
        };

        // Parse stat for name, state, cpu ticks.
        let (comm, state, utime, stime) = match parse_proc_stat(pid) {
            Some(v) => v,
            None => continue, // process may have exited or we lack permission
        };

        // Parse status for memory and uid.
        let (mem_usage, uid) = parse_proc_status(pid).unwrap_or((0, 0));

        let proc_ticks = utime + stime;
        new_prev.insert(pid, (proc_ticks, system_total));

        // CPU usage via delta against previous snapshot.
        let cpu_usage = if let Some(&(prev_ticks, prev_total)) = prev_guard.get(&pid) {
            let d_proc = proc_ticks.saturating_sub(prev_ticks) as f32;
            let d_sys = system_total.saturating_sub(prev_total) as f32;
            if d_sys > 0.0 {
                100.0 * d_proc / d_sys
            } else {
                0.0
            }
        } else {
            0.0 // first read
        };

        let user = uid_map
            .get(&uid)
            .cloned()
            .unwrap_or_else(|| uid.to_string());

        processes.push(ProcessInfo {
            pid,
            name: comm,
            cpu_usage,
            mem_usage,
            gpu_usage: None, // per-process GPU accounting not exposed via standard sysfs
            state,
            user,
        });
    }

    // Update stored snapshot.
    *prev_guard = new_prev;
    drop(prev_guard);

    // Sort by CPU usage descending, return top 20.
    processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
    processes.truncate(20);

    Ok(processes)
}
