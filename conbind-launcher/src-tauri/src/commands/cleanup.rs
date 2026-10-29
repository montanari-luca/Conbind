//! Tauri command layer for the `cleanup` built-in feature.
//!
//! Host-only surface, like `system_actions`: none of these are mapped in the
//! `conbind:api:*` permission table, so extension workers cannot call them.

use crate::cleanup::analyzer::{self, AnalyzedDir};
use crate::cleanup::network::{self, NetworkOutcome};
use crate::cleanup::remnants;
use crate::cleanup::targets::Category;
use crate::cleanup::{self, CategoryResult, DiskSpace, ItemResult, ScanRegistry};
use crate::error::AppError;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{Manager, State};

async fn spawn_blocking_result<T, F>(work: F) -> Result<T, AppError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| AppError::Other(format!("cleanup task failed: {e}")))
}

fn home_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, AppError> {
    app.path()
        .home_dir()
        .map_err(|e| AppError::Other(format!("Could not resolve home directory: {e}")))
}

/// Measures the requested categories, streaming each finished category over
/// `on_event`, and returns the scan id to pass to `cleanup_clean`.
#[tauri::command]
pub async fn cleanup_scan<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    registry: State<'_, Arc<ScanRegistry>>,
    categories: Vec<String>,
    on_event: Channel<CategoryResult>,
) -> Result<u64, AppError> {
    let categories: Vec<Category> = categories
        .iter()
        .map(|id| {
            Category::from_id(id)
                .ok_or_else(|| AppError::Validation(format!("unknown cleanup category: {id}")))
        })
        .collect::<Result<_, _>>()?;
    let home = home_dir(&app)?;
    let registry = Arc::clone(registry.inner());
    let own_id = app.config().identifier.clone();
    spawn_blocking_result(move || {
        let scan_id = registry.begin()?;
        registry.scan(
            scan_id,
            &home,
            &std::env::temp_dir(),
            &categories,
            |result| {
                let _ = on_event.send(result);
            },
        )?;
        if categories.contains(&Category::AppLeftovers) {
            let mut installed = remnants::installed_bundle_ids(&remnants::default_app_dirs(&home));
            installed.push(own_id);
            let leftovers = if cfg!(target_os = "macos") {
                remnants::find_leftovers(&home, &installed, std::time::SystemTime::now())
            } else {
                Vec::new()
            };
            let _ = on_event.send(registry.scan_leftovers(scan_id, leftovers)?);
        }
        Ok(scan_id)
    })
    .await?
}

/// Moves the chosen items of a scan to the trash. Per-item results are
/// returned so a partial failure is visible to the user.
#[tauri::command]
pub async fn cleanup_clean<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    registry: State<'_, Arc<ScanRegistry>>,
    scan_id: u64,
    item_ids: Vec<u32>,
) -> Result<Vec<ItemResult>, AppError> {
    let home = home_dir(&app)?;
    let registry = Arc::clone(registry.inner());
    spawn_blocking_result(move || {
        registry.clean(scan_id, &item_ids, &home, |path| {
            trash::delete(path).map_err(|e| format!("Failed to move to trash: {e}"))
        })
    })
    .await?
}

#[tauri::command]
pub async fn cleanup_disk_space<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<DiskSpace>, AppError> {
    let home = home_dir(&app)?;
    spawn_blocking_result(move || cleanup::disk_space(&home)).await
}

/// Lists one directory level of the home folder with each child sized. `path`
/// defaults to the home directory itself.
#[tauri::command]
pub async fn cleanup_analyze_dir<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: Option<String>,
) -> Result<AnalyzedDir, AppError> {
    let home = home_dir(&app)?;
    spawn_blocking_result(move || analyzer::analyze_dir(&home, path.as_deref())).await?
}

/// Flushes the DNS (and on macOS ARP) caches. Asks the OS for administrator
/// rights; a dismissed prompt is reported as `Cancelled`, not as an error.
#[tauri::command]
pub async fn cleanup_network() -> Result<NetworkOutcome, AppError> {
    spawn_blocking_result(network::run).await?
}

/// Permanently empties the OS trash. The caller confirms first.
#[tauri::command]
pub async fn cleanup_empty_trash() -> Result<(), AppError> {
    spawn_blocking_result(cleanup::trash_bin::empty).await?
}

/// Whether Conbind can read protected folders. Always `true` off macOS.
#[tauri::command]
pub async fn cleanup_full_disk_access<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<bool, AppError> {
    let home = home_dir(&app)?;
    spawn_blocking_result(move || cleanup::full_disk_access_granted(&home)).await
}

/// Opens System Settings at Privacy & Security > Full Disk Access.
#[tauri::command]
pub async fn cleanup_open_full_disk_access() -> Result<bool, AppError> {
    spawn_blocking_result(|| {
        #[cfg(target_os = "macos")]
        {
            return std::process::Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
        #[cfg(not(target_os = "macos"))]
        false
    })
    .await
}
