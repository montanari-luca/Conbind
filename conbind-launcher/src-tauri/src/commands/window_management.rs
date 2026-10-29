//! Tauri command wrappers for the window management feature.
//!
//! Each command requires the `window:manage` permission and runs against the platform's
//! [`WindowBackend`]. Commands that act on "the" window resolve it with [`target_window`].

use std::sync::{Arc, OnceLock};

use crate::error::AppError;
use crate::permissions::ExtensionPermissionRegistry;
use crate::window_management::backend::{work_areas, WindowBackend, WindowId};
use crate::window_management::engine::engine;
use crate::window_management::link::{existing_link_manager, link_manager, Axis};
use crate::window_management::types::{
    validate_bounds_update, AppWindowInfo, WindowBounds, WindowBoundsUpdate,
};
use crate::window_management::workspaces::{
    self, Captured, OpenOptions, OpenReport, WorkspaceWindow, MAX_WINDOWS,
};
use crate::AppState;

fn shared_backend() -> &'static Arc<dyn WindowBackend> {
    static BACKEND: OnceLock<Arc<dyn WindowBackend>> = OnceLock::new();
    BACKEND.get_or_init(|| {
        #[cfg(target_os = "macos")]
        return Arc::new(crate::window_management::macos::MacBackend);

        #[cfg(target_os = "windows")]
        return Arc::new(crate::window_management::windows::WindowsBackend);

        #[cfg(target_os = "linux")]
        return Arc::new(crate::window_management::linux::LinuxBackend);

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        return Arc::new(crate::window_management::backend::UnsupportedBackend);
    })
}

fn backend() -> &'static dyn WindowBackend {
    shared_backend().as_ref()
}

/// Explicitly laying out a window that is part of a linked split ends the link, so the
/// partner does not chase the change.
fn detach_from_split(id: &WindowId) {
    if let Some(manager) = existing_link_manager() {
        if manager.is_linked(id) {
            manager.unlink();
        }
    }
}

/// The window the user wants to act on: the frontmost app's focused window on macOS, or the
/// window that was active when the launcher opened on Windows and Linux.
#[allow(unused_variables)]
fn target_window(state: &AppState) -> Result<WindowId, AppError> {
    #[cfg(target_os = "macos")]
    return crate::window_management::macos::focused_window_id();

    #[cfg(target_os = "windows")]
    {
        let hwnd = *state.previous_hwnd.lock().map_err(|_| AppError::Lock)?;
        if hwnd == 0 {
            return Err(AppError::NotFound(
                "No previous window captured. Open Conbind via its launcher shortcut first."
                    .to_string(),
            ));
        }
        return Ok(crate::window_management::windows::format_window_id(hwnd));
    }

    #[cfg(target_os = "linux")]
    {
        let wid = *state
            .linux_prev_window_id
            .lock()
            .map_err(|_| AppError::Lock)?;
        crate::window_management::linux::check_x11(wid)?;
        return Ok(crate::window_management::linux::format_window_id(
            wid as u32,
        ));
    }

    #[allow(unreachable_code)]
    Err(AppError::Platform(
        "Window management is not supported on this platform.".to_string(),
    ))
}

/// Returns the bounds of the frontmost OS application window.
/// Requires 'window:manage' permission.
#[tauri::command]
pub async fn window_management_get_bounds(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
) -> Result<WindowBounds, AppError> {
    permissions.check(&extension_id, "window:manage")?;
    backend().bounds(&target_window(&state)?)
}

/// Updates the bounds of the frontmost OS application window.
/// Requires 'window:manage' permission.
#[tauri::command]
pub async fn window_management_set_bounds(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;
    let update = WindowBoundsUpdate {
        x,
        y,
        width,
        height,
    };
    validate_bounds_update(&update)?;

    let backend = backend();
    let id = target_window(&state)?;
    detach_from_split(&id);
    let current = backend.bounds(&id)?;
    engine().set_bounds(
        backend,
        &id,
        &WindowBounds {
            x: update.x.unwrap_or(current.x),
            y: update.y.unwrap_or(current.y),
            width: update.width.unwrap_or(current.width),
            height: update.height.unwrap_or(current.height),
        },
    )
}

/// Toggles the fullscreen state of the frontmost OS application window.
/// Requires 'window:manage' permission.
#[tauri::command]
pub async fn window_management_set_fullscreen(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    enable: bool,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;
    let id = target_window(&state)?;
    detach_from_split(&id);
    backend().set_fullscreen(&id, enable)
}

/// Returns the usable area of every display (excluding menu bar, Dock and taskbar).
/// Requires 'window:manage' permission.
#[tauri::command]
pub fn window_management_get_monitors(
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
) -> Result<Vec<WindowBounds>, AppError> {
    permissions.check(&extension_id, "window:manage")?;
    work_areas(backend())
}

/// Applies a preset layout to the frontmost OS application window relative to the monitor it is on.
/// Requires 'window:manage' permission.
#[tauri::command]
pub fn window_management_apply_preset(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    preset_id: String,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;
    let id = target_window(&state)?;
    detach_from_split(&id);
    engine().apply_preset(backend(), &id, &preset_id)
}

/// Grows or shrinks the frontmost window about its center by `factor` (1.1 = 10% larger),
/// keeping it inside its display's usable area.
/// Requires 'window:manage' permission.
#[tauri::command]
pub fn window_management_resize(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    factor: f64,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;
    let id = target_window(&state)?;
    detach_from_split(&id);
    engine().resize(backend(), &id, factor)
}

/// Minimizes the frontmost OS application window.
/// Requires 'window:manage' permission.
#[tauri::command]
pub fn window_management_minimize(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;
    engine().minimize(backend(), &target_window(&state)?)
}

/// Puts the frontmost window back where it was before the last preset or resize.
/// Requires 'window:manage' permission.
#[tauri::command]
pub fn window_management_restore(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;
    let id = target_window(&state)?;
    detach_from_split(&id);
    engine().restore(backend(), &id)
}

/// Tiles the frontmost window and `partner_id` side by side (`axis` "side", the default) or
/// stacked ("stack") on the frontmost window's display, then links them: resizing one
/// window's shared edge resizes the other. Only one pair is linked at a time.
/// Requires 'window:manage' permission.
#[tauri::command]
pub fn window_management_split(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    partner_id: String,
    axis: Option<String>,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;
    let axis = Axis::parse(axis.as_deref().unwrap_or("side"))?;
    let first = target_window(&state)?;
    let second = WindowId::new(partner_id);
    let span = engine().split_pair(backend(), &first, &second, axis)?;
    link_manager(shared_backend()).link(first, second, axis, span)
}

/// Ends the linked split, returning whether one was active.
/// Requires 'window:manage' permission.
#[tauri::command]
pub fn window_management_unsplit(
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
) -> Result<bool, AppError> {
    permissions.check(&extension_id, "window:manage")?;
    Ok(existing_link_manager().is_some_and(|manager| manager.unlink()))
}

/// Snapshots the frontmost window plus the frontmost others on its display (at most 4), or just
/// `window_ids` when given, as workspace entries. A linked pair among them is reported by index.
/// Requires 'window:manage' permission.
#[tauri::command]
pub fn window_management_workspace_capture(
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    window_ids: Option<Vec<String>>,
) -> Result<Captured, AppError> {
    permissions.check(&extension_id, "window:manage")?;
    let target = target_window(&state)?;
    let ids: Option<Vec<WindowId>> =
        window_ids.map(|ids| ids.into_iter().map(WindowId::new).collect());
    let link = existing_link_manager().and_then(|manager| manager.pair());
    workspaces::capture(backend(), &target, ids.as_deref(), link)
}

/// Arranges a saved workspace: reuses running windows, launches missing apps (waiting up to
/// about 10 seconds each), places and raises everything, and re-links a saved split pair.
/// Requires 'window:manage' permission.
#[tauri::command]
pub async fn window_management_workspace_open(
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    windows: Vec<WorkspaceWindow>,
    link: Option<(usize, usize)>,
) -> Result<OpenReport, AppError> {
    permissions.check(&extension_id, "window:manage")?;
    if windows.is_empty() || windows.len() > MAX_WINDOWS {
        return Err(AppError::Validation(format!(
            "A workspace holds 1 to {MAX_WINDOWS} windows"
        )));
    }

    tauri::async_runtime::spawn_blocking(move || {
        // Opening a workspace replaces whatever arrangement was linked.
        if let Some(manager) = existing_link_manager() {
            manager.unlink();
        }
        let report = workspaces::open(backend(), engine(), &windows, &OpenOptions::default())?;
        if let Some((a, b)) = link {
            if let (Some(Some(first)), Some(Some(second))) = (report.ids.get(a), report.ids.get(b))
            {
                // A pair that can't be re-linked is still correctly arranged.
                let _ = link_manager(shared_backend()).link_existing(
                    WindowId::new(first.as_str()),
                    WindowId::new(second.as_str()),
                );
            }
        }
        Ok(report)
    })
    .await
    .map_err(|e| AppError::Platform(format!("workspace task failed: {e}")))?
}

/// Returns all open OS application windows across displays.
/// Requires 'window:manage' permission.
#[tauri::command]
#[allow(unused_variables)]
pub async fn window_management_list_windows(
    app: tauri::AppHandle,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
) -> Result<Vec<AppWindowInfo>, AppError> {
    permissions.check(&extension_id, "window:manage")?;

    #[cfg(target_os = "macos")]
    return crate::window_management::macos::list_windows(&app);

    #[cfg(target_os = "windows")]
    return crate::window_management::windows::list_windows(&app);

    #[cfg(target_os = "linux")]
    return crate::window_management::linux::list_windows(&app);

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Ok(vec![])
}

/// Brings the target application window to the foreground and hides the launcher.
/// Requires 'window:manage' permission.
#[tauri::command]
#[allow(unused_variables)]
pub async fn window_management_focus_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    id: String,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;

    #[cfg(target_os = "macos")]
    crate::window_management::macos::focus_window(&id)?;

    #[cfg(target_os = "windows")]
    crate::window_management::windows::focus_window(&id)?;

    #[cfg(target_os = "linux")]
    crate::window_management::linux::focus_window(&id)?;

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return Err(AppError::Platform(
        "Window management is not supported on this platform.".to_string(),
    ));

    #[allow(unreachable_code)]
    {
        let _ = crate::commands::app::hide(app, state);
        Ok(())
    }
}

/// Closes the target application window.
/// Requires 'window:manage' permission.
#[tauri::command]
#[allow(unused_variables)]
pub async fn window_management_close_window(
    permissions: tauri::State<'_, ExtensionPermissionRegistry>,
    extension_id: Option<String>,
    id: String,
) -> Result<(), AppError> {
    permissions.check(&extension_id, "window:manage")?;

    #[cfg(target_os = "macos")]
    return crate::window_management::macos::close_window(&id);

    #[cfg(target_os = "windows")]
    return crate::window_management::windows::close_window(&id);

    #[cfg(target_os = "linux")]
    return crate::window_management::linux::close_window(&id);

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Err(AppError::Platform(
        "Window management is not supported on this platform.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::ExtensionPermissionRegistry;

    fn make_registry_with(id: &str, perms: &[&str]) -> ExtensionPermissionRegistry {
        let reg = ExtensionPermissionRegistry::new();
        let mut inner = reg.inner.lock().unwrap();
        inner.insert(
            id.to_string(),
            perms.iter().map(|s| s.to_string()).collect(),
        );
        drop(inner);
        reg
    }

    #[test]
    fn permission_check_blocks_missing_permission() {
        let reg = make_registry_with("ext-1", &["clipboard:read"]);
        let result = reg.check(&Some("ext-1".to_string()), "window:manage");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Permission(_)));
    }

    #[test]
    fn permission_check_allows_with_correct_permission() {
        let reg = make_registry_with("ext-1", &["window:manage"]);
        assert!(reg
            .check(&Some("ext-1".to_string()), "window:manage")
            .is_ok());
    }

    #[test]
    fn validate_bounds_update_rejects_all_none_in_command() {
        let update = WindowBoundsUpdate {
            x: None,
            y: None,
            width: None,
            height: None,
        };
        assert!(validate_bounds_update(&update).is_err());
    }
}
