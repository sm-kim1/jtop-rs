use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FanStats {
    pub fans: Vec<FanInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FanInfo {
    pub name: String,
    pub speed: u32,      // RPM
    pub pwm: u32,        // 0-255
    pub pwm_max: u32,    // typically 255
    pub profile: String, // "quiet", "cool", "manual"
}

fn read_sysfs(path: &str) -> Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

pub fn read_fan_stats() -> Result<FanStats> {
    let mut fans: Vec<FanInfo> = Vec::new();

    // Try primary path: /sys/devices/platform/pwm-fan/hwmon/hwmon*/
    let primary = "/sys/devices/platform/pwm-fan/hwmon";
    if let Ok(dir) = fs::read_dir(primary) {
        for entry in dir.filter_map(|e| e.ok()) {
            let hwmon_path = entry.path();
            if let Some(fan) = read_fan_from_hwmon(hwmon_path.to_str().unwrap_or("")) {
                fans.push(fan);
            }
        }
    }

    // Fallback: /sys/class/hwmon/hwmon*/ filtering by name
    if fans.is_empty() {
        if let Ok(dir) = fs::read_dir("/sys/class/hwmon") {
            for entry in dir.filter_map(|e| e.ok()) {
                let hwmon_path = entry.path();
                let hwmon_str = hwmon_path.to_str().unwrap_or("");
                let name = read_sysfs(&format!("{}/name", hwmon_str)).unwrap_or_default();
                if name.contains("pwm-fan") || name.contains("fan") {
                    if let Some(fan) = read_fan_from_hwmon(hwmon_str) {
                        fans.push(fan);
                    }
                }
            }
        }
    }

    Ok(FanStats { fans })
}

fn read_fan_from_hwmon(hwmon_dir: &str) -> Option<FanInfo> {
    if hwmon_dir.is_empty() {
        return None;
    }

    let hwmon_name = read_sysfs(&format!("{}/name", hwmon_dir)).unwrap_or_default();

    // Detect profile from target_pwm or tach_enable
    let profile = detect_profile(hwmon_dir);

    // Try pwm1 first, then pwm0
    for n in 1..=4u32 {
        let pwm_path = format!("{}/pwm{}", hwmon_dir, n);
        let pwm_val: u32 = match read_sysfs(&pwm_path).ok().and_then(|s| s.parse().ok()) {
            Some(v) => v,
            None => continue,
        };

        // RPM via fan{N}_input
        let rpm: u32 = read_sysfs(&format!("{}/fan{}_input", hwmon_dir, n))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let name = if hwmon_name.is_empty() {
            format!("Fan {}", n)
        } else {
            hwmon_name.clone()
        };

        return Some(FanInfo {
            name,
            speed: rpm,
            pwm: pwm_val,
            pwm_max: 255,
            profile,
        });
    }

    None
}

fn detect_profile(hwmon_dir: &str) -> String {
    // Check /sys/devices/platform/pwm-fan/target_pwm
    if let Ok(target) = read_sysfs("/sys/devices/platform/pwm-fan/target_pwm") {
        let target_val: u32 = target.parse().unwrap_or(0);
        return if target_val == 0 {
            "quiet".to_string()
        } else if target_val >= 200 {
            "cool".to_string()
        } else {
            "manual".to_string()
        };
    }

    // Check tach_enable
    if let Ok(val) = read_sysfs(&format!("{}/tach_enable", hwmon_dir)) {
        if val == "0" {
            return "quiet".to_string();
        }
    }

    "unknown".to_string()
}
