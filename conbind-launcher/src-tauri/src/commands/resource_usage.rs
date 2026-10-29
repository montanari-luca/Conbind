//! Tauri command layer for the `resource-usage` built-in feature.
//!
//! Host-only surface, like `cleanup`: not mapped in the `conbind:api:*`
//! permission table, so extension workers cannot call it. The GPU probe can
//! spawn a tool, so the sample runs on the blocking pool.

use crate::error::AppError;
use crate::resource_usage::{ResourceSampler, ResourceSnapshot};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn resource_usage_snapshot(
    sampler: State<'_, Arc<ResourceSampler>>,
) -> Result<ResourceSnapshot, AppError> {
    let sampler = Arc::clone(&sampler);
    tauri::async_runtime::spawn_blocking(move || sampler.snapshot())
        .await
        .map_err(|e| AppError::Other(format!("resource usage task failed: {e}")))
}
