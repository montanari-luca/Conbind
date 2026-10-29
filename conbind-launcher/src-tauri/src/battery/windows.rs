//! Windows backend: `GetSystemPowerStatus`. `BatteryFlag` bit 128 means "no
//! battery", bit 8 means charging; `BatteryLifeTime` is `u32::MAX` when the
//! remaining time is unknown (always the case while charging).

use super::{windows_seconds, BatterySnapshot, BatteryState};
use crate::error::AppError;
use ::windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

const FLAG_CHARGING: u8 = 8;
const FLAG_NO_BATTERY: u8 = 128;
const FLAG_UNKNOWN: u8 = 255;
const PERCENT_UNKNOWN: u8 = 255;

pub fn read() -> Result<BatterySnapshot, AppError> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }
        .map_err(|e| AppError::Other(format!("battery read failed: {e}")))?;
    Ok(snapshot_from(&status))
}

fn snapshot_from(status: &SYSTEM_POWER_STATUS) -> BatterySnapshot {
    let flag = status.BatteryFlag;
    if flag == FLAG_NO_BATTERY || flag == FLAG_UNKNOWN {
        return BatterySnapshot::absent();
    }
    let on_ac = status.ACLineStatus == 1;
    let charging = flag & FLAG_CHARGING != 0;
    let percent = (status.BatteryLifePercent != PERCENT_UNKNOWN)
        .then_some(f32::from(status.BatteryLifePercent));
    let state = if charging {
        BatteryState::Charging
    } else if on_ac {
        BatteryState::Charged
    } else {
        BatteryState::Discharging
    };
    BatterySnapshot {
        present: true,
        percent,
        state,
        seconds_to_empty: if state == BatteryState::Discharging {
            windows_seconds(status.BatteryLifeTime)
        } else {
            None
        },
        seconds_to_full: None,
    }
}
