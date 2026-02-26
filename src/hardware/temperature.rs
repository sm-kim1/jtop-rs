use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TempStats {
    pub zones: Vec<ThermalZone>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThermalZone {
    pub name: String,
    pub temp: f32,    // Celsius
    pub zone_type: String,
    pub trip_points: Vec<TripPoint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TripPoint {
    pub temp: f32,
    pub trip_type: String, // "passive", "active", "critical"
}

fn read_sysfs(path: &str) -> Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

pub fn read_temp_stats() -> Result<TempStats> {
    const THERMAL_BASE: &str = "/sys/devices/virtual/thermal";

    let mut zones: Vec<ThermalZone> = Vec::new();

    let read_dir = match fs::read_dir(THERMAL_BASE) {
        Ok(d) => d,
        Err(_) => return Ok(TempStats { zones }),
    };

    let mut entries: Vec<std::path::PathBuf> = read_dir
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("thermal_zone"))
                .unwrap_or(false)
        })
        .collect();

    // Sort by zone number for stable ordering
    entries.sort_by_key(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("thermal_zone"))
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(u32::MAX)
    });

    for zone_path in entries {
        let zone_str = match zone_path.to_str() {
            Some(s) => s,
            None => continue,
        };

        // Read temperature (millidegrees -> Celsius)
        let temp_raw = match read_sysfs(&format!("{}/temp", zone_str)) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let temp_milli: i64 = match temp_raw.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let temp = temp_milli as f32 / 1000.0;

        // Read zone type
        let zone_type = read_sysfs(&format!("{}/type", zone_str)).unwrap_or_default();

        // Read trip points
        let mut trip_points: Vec<TripPoint> = Vec::new();
        let mut m: u32 = 0;
        loop {
            let tp_temp_path = format!("{}/trip_point_{}_temp", zone_str, m);
            let tp_type_path = format!("{}/trip_point_{}_type", zone_str, m);

            let tp_temp_str = match read_sysfs(&tp_temp_path) {
                Ok(s) => s,
                Err(_) => break,
            };
            let tp_temp_milli: i64 = tp_temp_str.parse().unwrap_or(0);
            let tp_temp = tp_temp_milli as f32 / 1000.0;
            let tp_type = read_sysfs(&tp_type_path).unwrap_or_else(|_| "unknown".to_string());

            trip_points.push(TripPoint { temp: tp_temp, trip_type: tp_type });
            m += 1;
        }

        let name = zone_type.clone();
        zones.push(ThermalZone { name, temp, zone_type, trip_points });
    }

    Ok(TempStats { zones })
}
