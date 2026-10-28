use crate::app_updater::{AppUpdaterState, PendingUpdate};
use tauri::AppHandle;

/// Manual "Check for updates now" — called by the About tab button.
/// Delegates entirely to the service layer; no logic here.
#[tauri::command]
pub async fn app_updater_check_now(app: AppHandle) -> Result<Option<PendingUpdate>, String> {
    crate::app_updater::service::check_for_update(&app).await
}

/// Returns the available update (if one was found) so the frontend can restore
/// its badge state after the webview loads.
#[tauri::command]
pub fn app_updater_get_pending(state: tauri::State<'_, AppUpdaterState>) -> Option<PendingUpdate> {
    state.pending.lock().ok()?.clone()
}

/// Returns true when the What's New panel should be shown after an app update.
///
/// Rules (mirrors the design from the approved plan):
/// - `None`          → fresh install; caller should record the version silently — do NOT show
/// - `Some("")`      → corrupted storage safe-fail; show the panel
/// - `Some(v) == current` → already seen this version; do NOT show
/// - `Some(v) != current` → updated since last launch; show the panel
#[tauri::command]
pub fn app_updater_should_show_whats_new(
    last_seen_version: Option<String>,
    current_version: String,
) -> bool {
    match last_seen_version {
        None => false,
        Some(v) if v.is_empty() => true,
        Some(v) => v != current_version,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_show_whats_new_fresh_install() {
        assert!(!app_updater_should_show_whats_new(
            None,
            "1.0.0".to_string()
        ));
    }

    #[test]
    fn test_should_show_whats_new_same_version() {
        assert!(!app_updater_should_show_whats_new(
            Some("1.0.0".to_string()),
            "1.0.0".to_string()
        ));
    }

    #[test]
    fn test_should_show_whats_new_updated() {
        assert!(app_updater_should_show_whats_new(
            Some("1.0.0".to_string()),
            "1.1.0".to_string()
        ));
    }

    #[test]
    fn test_should_show_whats_new_empty_string_safe_fail() {
        assert!(app_updater_should_show_whats_new(
            Some("".to_string()),
            "1.0.0".to_string()
        ));
    }
}
