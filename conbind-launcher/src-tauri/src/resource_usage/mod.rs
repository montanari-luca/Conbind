//! Live system resource figures for the "View resource usage" view: CPU, RAM,
//! network throughput, disk capacity and (best effort) GPU/VRAM.
//!
//! The sampler is long-lived managed state. CPU load and network rates are
//! deltas between two samples, so keeping the `sysinfo` handles between calls
//! lets each poll return immediately instead of sleeping to take a second
//! reading the way `process_manager::enumerate` does.

pub mod gpu;

use gpu::GpuInfo;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Instant;
use sysinfo::{Disks, MemoryRefreshKind, Networks, RefreshKind, System};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuUsage {
    pub percent: f32,
    pub cores: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryUsage {
    pub used: u64,
    pub total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsage {
    pub rx_per_sec: u64,
    pub tx_per_sec: u64,
    pub total_rx: u64,
    pub total_tx: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    pub name: String,
    pub mount: String,
    pub used: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub cpu: CpuUsage,
    pub memory: MemoryUsage,
    pub network: NetworkUsage,
    pub disks: Vec<DiskUsage>,
    pub gpus: Vec<GpuInfo>,
}

struct SysState {
    system: System,
    networks: Networks,
    disks: Disks,
    last_net: Option<(Instant, u64, u64)>,
}

pub struct ResourceSampler {
    state: Mutex<SysState>,
}

impl Default for ResourceSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceSampler {
    pub fn new() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::new().with_memory(MemoryRefreshKind::everything()),
        );
        // Prime the CPU counters so the first real sample has a baseline.
        system.refresh_cpu_usage();
        Self {
            state: Mutex::new(SysState {
                system,
                networks: Networks::new_with_refreshed_list(),
                disks: Disks::new_with_refreshed_list(),
                last_net: None,
            }),
        }
    }

    /// Takes one reading. The GPU probe runs outside the lock because it can
    /// spawn a tool and must not stall a concurrent poll.
    pub fn snapshot(&self) -> ResourceSnapshot {
        let (cpu, memory, network, disks) = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let state = &mut *guard;
            state.system.refresh_cpu_usage();
            state.system.refresh_memory();
            state.networks.refresh();
            state.disks.refresh_list();
            state.disks.refresh();

            let cpu = CpuUsage {
                percent: state.system.global_cpu_usage().clamp(0.0, 100.0),
                cores: state.system.cpus().len(),
            };
            let memory = MemoryUsage {
                used: state.system.used_memory(),
                total: state.system.total_memory(),
                swap_used: state.system.used_swap(),
                swap_total: state.system.total_swap(),
            };

            let (total_rx, total_tx) = state
                .networks
                .iter()
                .filter(|(name, _)| !is_loopback(name))
                .fold((0u64, 0u64), |(rx, tx), (_, data)| {
                    (rx + data.total_received(), tx + data.total_transmitted())
                });
            let now = Instant::now();
            let (rx_per_sec, tx_per_sec) = match state.last_net {
                Some((at, prev_rx, prev_tx)) => {
                    let elapsed = now.duration_since(at).as_secs_f64();
                    (
                        rate_per_sec(prev_rx, total_rx, elapsed),
                        rate_per_sec(prev_tx, total_tx, elapsed),
                    )
                }
                None => (0, 0),
            };
            state.last_net = Some((now, total_rx, total_tx));

            let disks = state
                .disks
                .list()
                .iter()
                .filter_map(|d| {
                    let mount = d.mount_point().to_string_lossy().into_owned();
                    let total = d.total_space();
                    include_disk(&mount, total).then(|| DiskUsage {
                        name: d.name().to_string_lossy().into_owned(),
                        used: total.saturating_sub(d.available_space()),
                        mount,
                        total,
                    })
                })
                .collect();

            (
                cpu,
                memory,
                NetworkUsage {
                    rx_per_sec,
                    tx_per_sec,
                    total_rx,
                    total_tx,
                },
                disks,
            )
        };

        ResourceSnapshot {
            cpu,
            memory,
            network,
            disks,
            gpus: gpu::probe(),
        }
    }
}

/// Bytes per second between two cumulative counters. A counter that went
/// backwards (interface reset) or a zero interval reads as no traffic.
fn rate_per_sec(previous: u64, current: u64, elapsed_secs: f64) -> u64 {
    if elapsed_secs <= 0.0 {
        return 0;
    }
    (current.saturating_sub(previous) as f64 / elapsed_secs).round() as u64
}

fn is_loopback(interface: &str) -> bool {
    interface == "lo" || interface == "lo0" || interface.starts_with("Loopback")
}

/// Drops volumes that would only repeat or clutter the list: zero-size
/// mounts and macOS's internal APFS system volumes, which report the same
/// container the `/` entry already shows.
fn include_disk(mount: &str, total: u64) -> bool {
    total > 0 && !mount.starts_with("/System/Volumes") && !mount.starts_with("/private/var/vm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_divides_the_delta_by_elapsed_time() {
        assert_eq!(rate_per_sec(1_000, 3_000, 2.0), 1_000);
    }

    #[test]
    fn rate_is_zero_for_a_reset_counter_or_empty_interval() {
        assert_eq!(rate_per_sec(5_000, 100, 1.0), 0);
        assert_eq!(rate_per_sec(0, 100, 0.0), 0);
    }

    #[test]
    fn loopback_interfaces_are_recognised() {
        assert!(is_loopback("lo"));
        assert!(is_loopback("lo0"));
        assert!(!is_loopback("en0"));
        assert!(!is_loopback("eth0"));
    }

    #[test]
    fn disk_filter_keeps_real_volumes_only() {
        assert!(include_disk("/", 100));
        assert!(include_disk("/Volumes/Backup", 100));
        assert!(!include_disk("/System/Volumes/Data", 100));
        assert!(!include_disk("/Volumes/Empty", 0));
    }

    #[test]
    fn snapshot_reports_plausible_figures() {
        let sampler = ResourceSampler::new();
        let first = sampler.snapshot();
        assert!(first.memory.total > 0);
        assert!(first.memory.used <= first.memory.total);
        assert!(first.cpu.cores > 0);
        assert_eq!(first.network.rx_per_sec, 0);
        let second = sampler.snapshot();
        assert!((0.0..=100.0).contains(&second.cpu.percent));
    }
}
