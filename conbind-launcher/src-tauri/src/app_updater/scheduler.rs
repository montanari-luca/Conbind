use log::{error, info};
use std::time::Duration;
use tauri::AppHandle;

const STARTUP_DELAY_SECS: u64 = 60;
const CHECK_INTERVAL_SECS: u64 = 6 * 3600; // 6 hours

pub const DEFAULT_AUTO_CHECK: bool = true;

/// Read `autoCheck` from `settings.dat`. Returns the default if the store or
/// key is unavailable.
fn read_auto_check(app: &AppHandle) -> bool {
    use tauri_plugin_store::StoreExt;
    app.store("settings.dat")
        .ok()
        .and_then(|s| s.get("settings"))
        .and_then(|s| s.get("updates").cloned())
        .and_then(|u| u.get("autoCheck").and_then(|v| v.as_bool()))
        .unwrap_or(DEFAULT_AUTO_CHECK)
}

/// Run one scheduled update check, respecting the user's `autoCheck` setting.
async fn run_check(app: &AppHandle) {
    if read_auto_check(app) {
        info!("app_updater: scheduled check running...");
        if let Err(e) = crate::app_updater::service::check_for_update(app).await {
            error!("app_updater: scheduled check failed: {}", e);
            crate::event_bridge::bridge_emit(
                app,
                "conbind:app-update:error",
                serde_json::json!({ "message": e }),
            );
        }
    } else {
        info!("app_updater: auto-check is disabled, skipping scheduled check");
    }
}

/// Scheduler job: check for app updates 60s after launch, then every 6h.
pub fn job(app: AppHandle) -> crate::scheduler::Job {
    crate::scheduler::Job::fixed_interval(
        "app-update-check",
        Duration::from_secs(STARTUP_DELAY_SECS),
        Duration::from_secs(CHECK_INTERVAL_SECS),
        move || {
            let app = app.clone();
            async move { run_check(&app).await }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_auto_check_is_true() {
        const { assert!(DEFAULT_AUTO_CHECK) };
    }

    #[test]
    fn test_startup_delay_is_positive() {
        const { assert!(STARTUP_DELAY_SECS > 0) };
    }

    #[test]
    fn test_check_interval_is_reasonable() {
        // 6 hours = 21600 seconds; sanity-check it's in [3600, 86400]
        const { assert!(CHECK_INTERVAL_SECS >= 3600) };
        const { assert!(CHECK_INTERVAL_SECS <= 86400) };
    }
}
