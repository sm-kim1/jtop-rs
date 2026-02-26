pub mod cpu;
pub mod fan;
pub mod gpu;
pub mod jetson;
pub mod memory;
pub mod power;
pub mod process;
pub mod temperature;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Complete snapshot of Jetson hardware state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JetsonStats {
    pub cpu: cpu::CpuStats,
    pub gpu: gpu::GpuStats,
    pub memory: memory::MemoryStats,
    pub temperature: temperature::TempStats,
    pub fan: fan::FanStats,
    pub power: power::PowerStats,
    pub processes: Vec<process::ProcessInfo>,
    pub board: jetson::BoardInfo,
}

/// Trait for reading hardware stats (local or remote)
pub trait HardwareReader: Send + Sync {
    fn read_stats(&mut self) -> Result<JetsonStats>;
}

/// Local hardware reader - reads from sysfs directly
pub struct LocalReader;

impl LocalReader {
    pub fn new() -> Self {
        Self
    }
}

impl HardwareReader for LocalReader {
    fn read_stats(&mut self) -> Result<JetsonStats> {
        Ok(JetsonStats {
            cpu: cpu::read_cpu_stats()?,
            gpu: gpu::read_gpu_stats()?,
            memory: memory::read_memory_stats()?,
            temperature: temperature::read_temp_stats()?,
            fan: fan::read_fan_stats()?,
            power: power::read_power_stats()?,
            processes: process::read_processes()?,
            board: jetson::read_board_info()?,
        })
    }
}
