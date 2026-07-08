// Cross-platform system monitoring for the dashboard and system pages.
//
// A single `SysMonitor` is kept alive in the app state so CPU and network
// deltas are measured between successive polls. Everything here is derived
// from the `sysinfo` crate, which works on macOS, Linux and Windows.

use serde::Serialize;
use std::time::Instant;
use sysinfo::{Components, Disks, Networks, System};

/// Long-lived monitor. Refreshing the same `System` across polls is what
/// makes CPU usage and network throughput meaningful (they are deltas).
pub struct SysMonitor {
    sys: System,
    networks: Networks,
    last_sample: Instant,
}

impl SysMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self {
            sys,
            networks: Networks::new_with_refreshed_list(),
            last_sample: Instant::now(),
        }
    }

    /// Live, cheap-to-compute metrics polled roughly once per second by the UI.
    pub fn sample(&mut self) -> MetricsDto {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.networks.refresh(true);

        let elapsed = self.last_sample.elapsed().as_secs_f64().max(0.001);
        self.last_sample = Instant::now();

        // Network throughput in bytes/sec, summed across all interfaces.
        let mut rx = 0u64;
        let mut tx = 0u64;
        for (_name, data) in self.networks.iter() {
            rx += data.received();
            tx += data.transmitted();
        }
        let net_rx_rate = rx as f64 / elapsed;
        let net_tx_rate = tx as f64 / elapsed;

        let mem_total = self.sys.total_memory();
        let mem_used = self.sys.used_memory();
        let swap_total = self.sys.total_swap();
        let swap_used = self.sys.used_swap();

        let (disk_total, disk_used) = primary_disk_usage();

        MetricsDto {
            cpu: self.sys.global_cpu_usage() as f64,
            mem_used,
            mem_total,
            mem_percent: percent(mem_used, mem_total),
            swap_used,
            swap_total,
            swap_percent: percent(swap_used, swap_total),
            disk_used,
            disk_total,
            disk_percent: percent(disk_used, disk_total),
            net_rx_rate,
            net_tx_rate,
        }
    }

    /// Fuller, slower-changing snapshot for the System page.
    pub fn system_info(&mut self) -> SystemInfoDto {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();

        let cpus = self.sys.cpus();
        let per_core: Vec<f64> = cpus.iter().map(|c| c.cpu_usage() as f64).collect();
        let brand = cpus.first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
        let frequency_mhz = cpus.first().map(|c| c.frequency()).unwrap_or(0);

        let load = System::load_average();

        let disks = Disks::new_with_refreshed_list()
            .iter()
            .map(|d| {
                let total = d.total_space();
                let available = d.available_space();
                let used = total.saturating_sub(available);
                DiskDto {
                    name: d.name().to_string_lossy().to_string(),
                    mount: d.mount_point().to_string_lossy().to_string(),
                    fs: d.file_system().to_string_lossy().to_string(),
                    kind: format!("{:?}", d.kind()),
                    total,
                    available,
                    used,
                    percent: percent(used, total),
                }
            })
            .collect();

        let temperatures = Components::new_with_refreshed_list()
            .iter()
            .filter_map(|c| {
                c.temperature().map(|t| TempDto {
                    label: c.label().to_string(),
                    celsius: t as f64,
                })
            })
            .collect::<Vec<_>>();

        let mem_total = self.sys.total_memory();
        let mem_used = self.sys.used_memory();
        let swap_total = self.sys.total_swap();
        let swap_used = self.sys.used_swap();

        SystemInfoDto {
            os: OsDto {
                name: System::name().unwrap_or_else(|| "Unknown".into()),
                kernel: System::kernel_version().unwrap_or_default(),
                os_version: System::long_os_version().unwrap_or_default(),
                hostname: System::host_name().unwrap_or_default(),
                arch: System::cpu_arch(),
                uptime: System::uptime(),
                boot_time: System::boot_time(),
            },
            cpu: CpuDto {
                brand,
                physical_cores: System::physical_core_count().unwrap_or(0),
                logical_cores: cpus.len(),
                frequency_mhz,
                usage: self.sys.global_cpu_usage() as f64,
                per_core,
            },
            memory: MemoryDto {
                total: mem_total,
                used: mem_used,
                available: self.sys.available_memory(),
                percent: percent(mem_used, mem_total),
                swap_total,
                swap_used,
                swap_percent: percent(swap_used, swap_total),
            },
            load: LoadDto {
                one: load.one,
                five: load.five,
                fifteen: load.fifteen,
            },
            health: build_health(percent(mem_used, mem_total), percent(swap_used, swap_total), &temperatures),
            temperatures,
            disks,
        }
    }
}

/// Total/used bytes of the disk backing the root ("/" or "C:\") mount, falling
/// back to the largest disk when no obvious root is present.
fn primary_disk_usage() -> (u64, u64) {
    let disks = Disks::new_with_refreshed_list();
    let mut best: Option<(u64, u64)> = None;
    let mut root: Option<(u64, u64)> = None;
    for d in disks.iter() {
        let total = d.total_space();
        let used = total.saturating_sub(d.available_space());
        let mount = d.mount_point().to_string_lossy();
        if mount == "/" || mount == "C:\\" {
            root = Some((total, used));
        }
        if best.map(|(t, _)| total > t).unwrap_or(true) {
            best = Some((total, used));
        }
    }
    root.or(best).unwrap_or((0, 0))
}

fn percent(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        (part as f64 / whole as f64) * 100.0
    }
}

/// Derive a coarse health verdict from memory/swap pressure and temperature.
fn build_health(mem_pct: f64, swap_pct: f64, temps: &[TempDto]) -> HealthDto {
    let hottest = temps.iter().fold(0.0_f64, |m, t| m.max(t.celsius));
    let mut issues = Vec::new();
    if mem_pct > 90.0 {
        issues.push("Memory usage is very high".to_string());
    }
    if swap_pct > 50.0 {
        issues.push("Heavy swap usage — system may be under memory pressure".to_string());
    }
    if hottest > 85.0 {
        issues.push(format!("High temperature detected ({hottest:.0}°C)"));
    }
    let status = if issues.iter().any(|_| mem_pct > 95.0 || hottest > 90.0) {
        "critical"
    } else if issues.is_empty() {
        "healthy"
    } else {
        "warning"
    };
    HealthDto {
        status: status.to_string(),
        issues,
    }
}

// ---- DTOs ----

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
