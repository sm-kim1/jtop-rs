use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PowerStats {
    pub rails: Vec<PowerRail>,
    pub total_power: f32, // milliwatts
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PowerRail {
    pub name: String,
    pub voltage: f32,  // millivolts
    pub current: f32,  // milliamps
    pub power: f32,    // milliwatts
    pub warn: Option<f32>,
    pub crit: Option<f32>,
}

fn read_sysfs(path: &str) -> Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

pub fn read_power_stats() -> Result<PowerStats> {
    let mut rails: Vec<PowerRail> = Vec::new();

    // Primary: /sys/bus/i2c/drivers/ina3221/*/hwmon/hwmon*/
    let ina3221_base = "/sys/bus/i2c/drivers/ina3221";
    if let Ok(driver_dir) = fs::read_dir(ina3221_base) {
        for dev_entry in driver_dir.filter_map(|e| e.ok()) {
            let dev_path = dev_entry.path();
            let hwmon_path = dev_path.join("hwmon");
            if let Ok(hwmon_dir) = fs::read_dir(&hwmon_path) {
                for hwmon_entry in hwmon_dir.filter_map(|e| e.ok()) {
                    let hwmon_str = hwmon_entry.path();
                    let hwmon_str = hwmon_str.to_str().unwrap_or("");
                    read_ina3221_channels(hwmon_str, &mut rails);
                }
            }
        }
    }

    // Fallback: /sys/class/hwmon/hwmon*/ filtered by name == "ina3221"
    if rails.is_empty() {
        if let Ok(hwmon_dir) = fs::read_dir("/sys/class/hwmon") {
            for entry in hwmon_dir.filter_map(|e| e.ok()) {
                let hwmon_path = entry.path();
                let hwmon_str = hwmon_path.to_str().unwrap_or("");
                let name = read_sysfs(&format!("{}/name", hwmon_str)).unwrap_or_default();
                if name == "ina3221" {
                    read_ina3221_channels(hwmon_str, &mut rails);
                }
            }
        }
    }

    let total_power = rails.iter().map(|r| r.power).sum();

    Ok(PowerStats { rails, total_power })
}

fn read_ina3221_channels(hwmon_dir: &str, rails: &mut Vec<PowerRail>) {
    // INA3221 has 3 channels; indices are 1-based in sysfs
    for n in 1..=3u32 {
        // Voltage: in{N}_input in millivolts
        let voltage: f32 = read_sysfs(&format!("{}/in{}_input", hwmon_dir, n))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        // Skip channels with no voltage (not populated)
        if voltage == 0.0 {
            // Try current to confirm channel exists at all
            let curr_check: Option<f32> = read_sysfs(&format!("{}/curr{}_input", hwmon_dir, n))
                .ok()
                .and_then(|s| s.parse().ok());
            if curr_check.is_none() {
                continue;
            }
        }

        // Current: curr{N}_input in milliamps
        let current: f32 = read_sysfs(&format!("{}/curr{}_input", hwmon_dir, n))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        // Power: power{N}_input in microwatts -> divide by 1000 for milliwatts
        let power_uw: f32 = read_sysfs(&format!("{}/power{}_input", hwmon_dir, n))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let power = power_uw / 1000.0;

        // Label: in{N}_label
        let name = read_sysfs(&format!("{}/in{}_label", hwmon_dir, n))
            .unwrap_or_else(|_| format!("Rail {}", n));

        // Critical current threshold: curr{N}_crit in milliamps
        let crit: Option<f32> = read_sysfs(&format!("{}/curr{}_crit", hwmon_dir, n))
            .ok()
            .and_then(|s| s.parse().ok());

        // Warn threshold: curr{N}_max (not always present)
        let warn: Option<f32> = read_sysfs(&format!("{}/curr{}_max", hwmon_dir, n))
            .ok()
            .and_then(|s| s.parse().ok());

        rails.push(PowerRail { name, voltage, current, power, warn, crit });
    }
}
