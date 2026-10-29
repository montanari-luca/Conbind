//! Tauri command layer for the `battery` built-in feature.
//!
//! Host-only surface, like `resource_usage`: not mapped in the `conbind:api:*`
//! permission table, so extension workers cannot call it. The read talks to
//! IOKit / DBus / Win32, so it runs on the blocking pool.

use crate::battery::BatterySnapshot;
use crate::error::AppError;

#[tauri::command]
pub async fn battery_estimate() -> Result<BatterySnapshot, AppError> {
    tauri::async_runtime::spawn_blocking(crate::battery::read)
        .await
        .map_err(|e| AppError::Other(format!("battery task failed: {e}")))?
}
