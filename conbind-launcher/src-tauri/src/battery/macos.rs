//! macOS backend: IOKit power sources, the same C API `system_events::macos`
//! polls for the charge level. The description dictionary also carries the
//! system's own time estimates, in minutes (`-1` while it is still working
//! one out).

use super::{minutes_to_seconds, BatterySnapshot, BatteryState};
use crate::error::AppError;
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;

extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> *const std::ffi::c_void;
    fn IOPSCopyPowerSourcesList(blob: *const std::ffi::c_void) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(
        blob: *const std::ffi::c_void,
        ps: *const std::ffi::c_void,
    ) -> CFDictionaryRef;
}

fn number(desc: &CFDictionary<CFString, CFType>, key: &str) -> Option<i64> {
    desc.find(CFString::new(key))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_i64())
}

fn flag(desc: &CFDictionary<CFString, CFType>, key: &str) -> Option<bool> {
    desc.find(CFString::new(key))
        .and_then(|v| v.downcast::<CFBoolean>())
        .map(bool::from)
}

pub fn read() -> Result<BatterySnapshot, AppError> {
    unsafe {
        let blob = IOPSCopyPowerSourcesInfo();
        if blob.is_null() {
            return Ok(BatterySnapshot::absent());
        }
        let list_ref = IOPSCopyPowerSourcesList(blob);
        if list_ref.is_null() {
            core_foundation::base::CFRelease(blob);
            return Ok(BatterySnapshot::absent());
        }
        let list: CFArray<CFType> = CFArray::wrap_under_create_rule(list_ref);
        let mut snapshot = BatterySnapshot::absent();

        for i in 0..list.len() {
            let Some(ps_item) = list.get(i) else { continue };
            let desc_ref = IOPSGetPowerSourceDescription(blob, ps_item.as_CFTypeRef() as *const _);
            if desc_ref.is_null() {
                continue;
            }
            let desc: CFDictionary<CFString, CFType> = CFDictionary::wrap_under_get_rule(desc_ref);

            // "Type" is "InternalBattery" for the built-in cell; skip UPS units.
            let is_internal = desc
                .find(CFString::new("Type"))
                .and_then(|v| v.downcast::<CFString>())
                .map(|s| s == CFString::new("InternalBattery"))
                .unwrap_or(false);
            if !is_internal {
                continue;
            }

            let on_battery = desc
                .find(CFString::new("Power Source State"))
                .and_then(|v| v.downcast::<CFString>())
                .map(|s| s == CFString::new("Battery Power"))
                .unwrap_or(false);
            let charging = flag(&desc, "Is Charging").unwrap_or(false);
            let charged = flag(&desc, "Is Charged").unwrap_or(false);

            let percent = match (
                number(&desc, "Current Capacity"),
                number(&desc, "Max Capacity"),
            ) {
                (Some(cur), Some(max)) if max > 0 => {
                    Some(((cur as f64 / max as f64) * 100.0).clamp(0.0, 100.0) as f32)
                }
                _ => None,
            };

            snapshot = BatterySnapshot {
                present: true,
                percent,
                state: if on_battery {
                    BatteryState::Discharging
                } else if charged {
                    BatteryState::Charged
                } else if charging {
                    BatteryState::Charging
                } else {
                    BatteryState::Unknown
                },
                seconds_to_empty: if on_battery {
                    number(&desc, "Time to Empty").and_then(minutes_to_seconds)
                } else {
                    None
                },
                seconds_to_full: if charging {
                    number(&desc, "Time to Full Charge").and_then(minutes_to_seconds)
                } else {
                    None
                },
            };
            break;
        }
        core_foundation::base::CFRelease(blob);
        Ok(snapshot)
    }
}
