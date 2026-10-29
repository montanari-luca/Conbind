//! Best-effort GPU probe. Every figure is optional: a GPU whose driver exposes
//! no utilisation or memory counter still shows up by name, and a machine with
//! no readable GPU returns an empty list rather than an error.
//!
//! Sources, per platform:
//! - macOS: `ioreg` `IOAccelerator` `PerformanceStatistics`. Apple Silicon has
//!   unified memory, so its "VRAM" is the share of system RAM the GPU holds.
//! - Linux: amdgpu/i915 sysfs counters, plus `nvidia-smi` when it is installed.
//! - Windows: `nvidia-smi` when it is installed.

use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    /// Busy share of the GPU, 0..=100.
    pub usage_percent: Option<f32>,
    pub vram_used: Option<u64>,
    pub vram_total: Option<u64>,
}

pub fn probe() -> Vec<GpuInfo> {
    probe_platform()
}

#[cfg(target_os = "macos")]
fn probe_platform() -> Vec<GpuInfo> {
    let mut command = Command::new("/usr/sbin/ioreg");
    command.args(["-r", "-c", "IOAccelerator", "-d", "1", "-w", "0"]);
    let Some(text) = run_text(command) else {
        return Vec::new();
    };
    let total_ram = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::new().with_memory(sysinfo::MemoryRefreshKind::new().with_ram()),
    )
    .total_memory();
    parse_ioreg(&text, total_ram)
}

#[cfg(target_os = "linux")]
fn probe_platform() -> Vec<GpuInfo> {
    let mut gpus = read_sysfs_gpus(std::path::Path::new("/sys/class/drm"));
    gpus.extend(probe_nvidia_smi());
    gpus
}

#[cfg(target_os = "windows")]
fn probe_platform() -> Vec<GpuInfo> {
    probe_nvidia_smi()
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn probe_platform() -> Vec<GpuInfo> {
    Vec::new()
}

/// Runs a tool and returns its stdout, or `None` when it is missing or fails.
fn run_text(mut command: Command) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(target_os = "linux")]
    crate::platform::linux::sanitize_command(&mut command);
    let output = command.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
fn probe_nvidia_smi() -> Vec<GpuInfo> {
    let mut command = Command::new("nvidia-smi");
    command.args([
        "--query-gpu=name,utilization.gpu,memory.used,memory.total",
        "--format=csv,noheader,nounits",
    ]);
    run_text(command)
        .map(|text| parse_nvidia_smi(&text))
        .unwrap_or_default()
}

const MIB: u64 = 1024 * 1024;

/// Parses `nvidia-smi --format=csv,noheader,nounits` rows:
/// `NVIDIA GeForce RTX 3080, 12, 1024, 10240` (memory in MiB). Fields that are
/// not numbers (`[N/A]`) become `None`.
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn parse_nvidia_smi(text: &str) -> Vec<GpuInfo> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            let [name, usage, used, total] = fields[..] else {
                return None;
            };
            if name.is_empty() {
                return None;
            }
            Some(GpuInfo {
                name: name.to_string(),
                usage_percent: usage.parse().ok(),
                vram_used: used.parse::<u64>().ok().map(|v| v * MIB),
                vram_total: total.parse::<u64>().ok().map(|v| v * MIB),
            })
        })
        .collect()
}

/// Pulls `"key"=<digits>` out of an `ioreg` property dictionary line.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn ioreg_number(text: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"=");
    let start = text.find(&needle)? + needle.len();
    let digits: String = text[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Pulls `"key" = "value"` out of an `ioreg` property line.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn ioreg_string(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\" = \"");
    let start = text.find(&needle)? + needle.len();
    let end = text[start..].find('"')?;
    Some(text[start..start + end].to_string())
}

/// Parses `ioreg -r -c IOAccelerator` output into one entry per accelerator
/// that reports a `PerformanceStatistics` dictionary. `total_ram` is the
/// unified-memory ceiling used when the GPU has no dedicated VRAM counter.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_ioreg(text: &str, total_ram: u64) -> Vec<GpuInfo> {
    text.split("+-o ")
        .filter_map(|block| {
            let stats_line = block
                .lines()
                .find(|l| l.contains("\"PerformanceStatistics\""))?;
            let usage = ioreg_number(stats_line, "Device Utilization %");
            let dedicated_used = ioreg_number(stats_line, "vramUsedBytes");
            let (vram_used, vram_total) = match dedicated_used {
                Some(used) => (
                    Some(used),
                    ioreg_number(stats_line, "vramFreeBytes").map(|free| used + free),
                ),
                None => (
                    ioreg_number(stats_line, "In use system memory"),
                    (total_ram > 0).then_some(total_ram),
                ),
            };
            Some(GpuInfo {
                name: ioreg_string(block, "model").unwrap_or_else(|| "GPU".to_string()),
                usage_percent: usage.map(|v| v.min(100) as f32),
                vram_used,
                vram_total,
            })
        })
        .collect()
}

/// Reads amdgpu-style counters under `/sys/class/drm/card*/device`. Cards
/// without either counter (NVIDIA proprietary, virtual displays) are skipped.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn read_sysfs_gpus(drm: &std::path::Path) -> Vec<GpuInfo> {
    let Ok(entries) = std::fs::read_dir(drm) else {
        return Vec::new();
    };
    let mut cards: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.strip_prefix("card")
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .collect();
    cards.sort_by_key(|e| e.file_name());
    cards
        .iter()
        .filter_map(|entry| {
            let device = entry.path().join("device");
            let read_u64 = |file: &str| -> Option<u64> {
                std::fs::read_to_string(device.join(file))
                    .ok()?
                    .trim()
                    .parse()
                    .ok()
            };
            let usage = read_u64("gpu_busy_percent");
            let vram_used = read_u64("mem_info_vram_used");
            let vram_total = read_u64("mem_info_vram_total");
            if usage.is_none() && vram_total.is_none() {
                return None;
            }
            let vendor = std::fs::read_to_string(device.join("vendor")).unwrap_or_default();
            let name = match vendor.trim() {
                "0x1002" => "AMD GPU",
                "0x8086" => "Intel GPU",
                _ => "GPU",
            };
            Some(GpuInfo {
                name: name.to_string(),
                usage_percent: usage.map(|v| v.min(100) as f32),
                vram_used,
                vram_total,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nvidia_smi_rows() {
        let gpus = parse_nvidia_smi("NVIDIA GeForce RTX 3080, 12, 1024, 10240\n");
        assert_eq!(
            gpus,
            vec![GpuInfo {
                name: "NVIDIA GeForce RTX 3080".to_string(),
                usage_percent: Some(12.0),
                vram_used: Some(1024 * MIB),
                vram_total: Some(10240 * MIB),
            }]
        );
    }

    #[test]
    fn nvidia_smi_na_fields_become_none_and_junk_rows_are_skipped() {
        let gpus = parse_nvidia_smi("Tesla T4, [N/A], 0, 15360\nnot a row\n\n");
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].usage_percent, None);
        assert_eq!(gpus[0].vram_used, Some(0));
    }

    const IOREG: &str = "+-o AGXAcceleratorG16G  <class AGXAcceleratorG16G, id 0x1>\n    {\n      \"PerformanceStatistics\" = {\"In use system memory (driver)\"=0,\"Alloc system memory\"=2804826112,\"Device Utilization %\"=32,\"In use system memory\"=738295808}\n      \"model\" = \"Apple M4\"\n    }\n";

    #[test]
    fn parses_apple_silicon_ioreg_as_unified_memory() {
        let gpus = parse_ioreg(IOREG, 16 * 1024 * MIB);
        assert_eq!(
            gpus,
            vec![GpuInfo {
                name: "Apple M4".to_string(),
                usage_percent: Some(32.0),
                vram_used: Some(738_295_808),
                vram_total: Some(16 * 1024 * MIB),
            }]
        );
    }

    #[test]
    fn ioreg_without_statistics_yields_nothing() {
        assert!(parse_ioreg("+-o Foo\n  {\n  \"model\" = \"x\"\n  }\n", 1).is_empty());
    }

    #[test]
    fn ioreg_number_does_not_match_a_longer_key() {
        let line = "{\"In use system memory (driver)\"=5,\"In use system memory\"=7}";
        assert_eq!(ioreg_number(line, "In use system memory"), Some(7));
    }

    #[test]
    fn reads_amdgpu_sysfs_counters() {
        let dir = tempfile::tempdir().unwrap();
        let device = dir.path().join("card0/device");
        std::fs::create_dir_all(&device).unwrap();
        std::fs::write(device.join("vendor"), "0x1002\n").unwrap();
        std::fs::write(device.join("gpu_busy_percent"), "41\n").unwrap();
        std::fs::write(device.join("mem_info_vram_used"), "1000\n").unwrap();
        std::fs::write(device.join("mem_info_vram_total"), "8000\n").unwrap();
        // A render node and a card with no counters must be ignored.
        std::fs::create_dir_all(dir.path().join("card0-HDMI-A-1")).unwrap();
        std::fs::create_dir_all(dir.path().join("card1/device")).unwrap();

        let gpus = read_sysfs_gpus(dir.path());
        assert_eq!(
            gpus,
            vec![GpuInfo {
                name: "AMD GPU".to_string(),
                usage_percent: Some(41.0),
                vram_used: Some(1000),
                vram_total: Some(8000),
            }]
        );
    }
}
