//! Linux backend: the UPower display battery (the device whose `Type` is `2`),
//! the same device `system_events::linux` watches for charge changes. UPower
//! reports `TimeToEmpty` / `TimeToFull` in seconds, `0` when unknown.

use super::{nonzero_seconds, BatterySnapshot, BatteryState};
use crate::error::AppError;
use zbus::blocking::Connection;
use zbus::proxy;
use zbus::zvariant::OwnedObjectPath;

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPowerRoot {
    fn enumerate_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower"
)]
trait UPowerDevice {
    #[zbus(property)]
    fn type_(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn time_to_empty(&self) -> zbus::Result<i64>;
    #[zbus(property)]
    fn time_to_full(&self) -> zbus::Result<i64>;
}

/// UPower `State` enum: 1 charging, 2 discharging, 4 fully charged,
/// 5 pending charge, 6 pending discharge, 0 unknown, 3 empty.
fn state_from_upower(state: u32) -> BatteryState {
    match state {
        1 | 5 => BatteryState::Charging,
        2 | 3 | 6 => BatteryState::Discharging,
        4 => BatteryState::Charged,
        _ => BatteryState::Unknown,
    }
}

fn to_app_error(e: zbus::Error) -> AppError {
    AppError::Other(format!("battery read failed: {e}"))
}

pub fn read() -> Result<BatterySnapshot, AppError> {
    // No system bus (containers, minimal installs) simply means no battery data.
    let Ok(conn) = Connection::system() else {
        return Ok(BatterySnapshot::absent());
    };
    let root = UPowerRootProxyBlocking::new(&conn).map_err(to_app_error)?;
    let devices = match root.enumerate_devices() {
        Ok(d) => d,
        Err(_) => return Ok(BatterySnapshot::absent()),
    };

    for path in devices {
        let proxy = UPowerDeviceProxyBlocking::builder(&conn)
            .path(path.as_str())
            .and_then(|b| b.build())
            .map_err(to_app_error)?;
        if proxy.type_().map(|t| t != 2).unwrap_or(true) {
            continue;
        }
        let state = state_from_upower(proxy.state().unwrap_or(0));
        return Ok(BatterySnapshot {
            present: true,
            percent: proxy.percentage().ok().map(|p| p.clamp(0.0, 100.0) as f32),
            state,
            seconds_to_empty: if state == BatteryState::Discharging {
                proxy.time_to_empty().ok().and_then(nonzero_seconds)
            } else {
                None
            },
            seconds_to_full: if state == BatteryState::Charging {
                proxy.time_to_full().ok().and_then(nonzero_seconds)
            } else {
                None
            },
        });
    }
    Ok(BatterySnapshot::absent())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_upower_states() {
        assert_eq!(state_from_upower(1), BatteryState::Charging);
        assert_eq!(state_from_upower(2), BatteryState::Discharging);
        assert_eq!(state_from_upower(4), BatteryState::Charged);
        assert_eq!(state_from_upower(0), BatteryState::Unknown);
    }
}
