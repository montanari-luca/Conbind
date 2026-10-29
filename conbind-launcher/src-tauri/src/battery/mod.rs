//! One-shot battery reading for the "Estimate battery" view: charge level,
//! charging state and the time the OS estimates is left (or needed to fill).
//!
//! The estimate is the one the OS already computes, never a model of our own.
//! Each platform reports "unknown" differently (a negative number, zero, or
//! `u32::MAX`), so the backends normalise through the pure helpers below, which
//! are platform-neutral and unit-tested on any host.

use crate::error::AppError;
use serde::Serialize;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BatteryState {
    Discharging,
    Charging,
    Charged,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatterySnapshot {
    /// `false` on machines without a battery (desktops, VMs).
    pub present: bool,
    pub percent: Option<f32>,
    pub state: BatteryState,
    pub seconds_to_empty: Option<u64>,
    pub seconds_to_full: Option<u64>,
}

impl BatterySnapshot {
    pub fn absent() -> Self {
        Self {
            present: false,
            percent: None,
            state: BatteryState::Unknown,
            seconds_to_empty: None,
            seconds_to_full: None,
        }
    }
}

/// Minutes as reported by IOKit; negative means "still calculating".
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn minutes_to_seconds(minutes: i64) -> Option<u64> {
    u64::try_from(minutes)
        .ok()
        .filter(|m| *m > 0)
        .map(|m| m * 60)
}

/// Seconds as reported by UPower; zero means unknown.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn nonzero_seconds(seconds: i64) -> Option<u64> {
    u64::try_from(seconds).ok().filter(|s| *s > 0)
}

/// `GetSystemPowerStatus` reports `u32::MAX` when the time is unknown.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn windows_seconds(seconds: u32) -> Option<u64> {
    (seconds != u32::MAX).then_some(u64::from(seconds))
}

pub fn read() -> Result<BatterySnapshot, AppError> {
    #[cfg(target_os = "macos")]
    return macos::read();
    #[cfg(target_os = "linux")]
    return linux::read();
    #[cfg(target_os = "windows")]
    return windows::read();
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    Ok(BatterySnapshot::absent())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minutes_to_seconds_treats_negative_as_unknown() {
        assert_eq!(minutes_to_seconds(-1), None);
        assert_eq!(minutes_to_seconds(0), None);
        assert_eq!(minutes_to_seconds(192), Some(11_520));
    }

    #[test]
    fn nonzero_seconds_treats_zero_as_unknown() {
        assert_eq!(nonzero_seconds(0), None);
        assert_eq!(nonzero_seconds(-5), None);
        assert_eq!(nonzero_seconds(90), Some(90));
    }

    #[test]
    fn windows_seconds_treats_max_as_unknown() {
        assert_eq!(windows_seconds(u32::MAX), None);
        assert_eq!(windows_seconds(3600), Some(3600));
    }

    #[test]
    fn absent_snapshot_has_no_figures() {
        let s = BatterySnapshot::absent();
        assert!(!s.present);
        assert_eq!(s.percent, None);
        assert_eq!(s.seconds_to_empty, None);
    }
}
