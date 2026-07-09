//! Serializable snapshots returned to the browser by the monitor.

use serde::Serialize;

#[derive(Serialize)]
pub struct MetricsDto {
    pub cpu: f64,
    pub mem_used: u64,
    pub mem_total: u64,
    pub mem_percent: f64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub swap_percent: f64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub disk_percent: f64,
    pub net_rx_rate: f64,
    pub net_tx_rate: f64,
}

#[derive(Serialize)]
pub struct SystemInfoDto {
    pub os: OsDto,
    pub cpu: CpuDto,
    pub memory: MemoryDto,
    pub load: LoadDto,
    pub health: HealthDto,
    pub temperatures: Vec<TempDto>,
    pub disks: Vec<DiskDto>,
}

#[derive(Serialize)]
pub struct OsDto {
    pub name: String,
    pub kernel: String,
    pub os_version: String,
    pub hostname: String,
    pub arch: String,
    pub uptime: u64,
    pub boot_time: u64,
}

#[derive(Serialize)]
pub struct CpuDto {
    pub brand: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub frequency_mhz: u64,
    pub usage: f64,
    pub per_core: Vec<f64>,
}

#[derive(Serialize)]
pub struct MemoryDto {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub percent: f64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_percent: f64,
}

#[derive(Serialize)]
pub struct LoadDto {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

#[derive(Serialize)]
pub struct HealthDto {
    pub status: String,
    pub issues: Vec<String>,
}

#[derive(Serialize)]
pub struct TempDto {
    pub label: String,
    pub celsius: f64,
}

#[derive(Serialize)]
pub struct DiskDto {
    pub name: String,
    pub mount: String,
    pub fs: String,
    pub kind: String,
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub percent: f64,
}
