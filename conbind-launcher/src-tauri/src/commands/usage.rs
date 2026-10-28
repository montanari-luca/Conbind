//! Thin command wrappers for the usage module. All logic lives in
//! `crate::usage`; these only adapt managed state to Tauri commands.

use std::sync::Arc;

use crate::usage::{self, UsageError, UsageState};

#[tauri::command]
pub async fn record_active_day<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<(), UsageError> {
    use tauri::Manager;
    if let Some(state) = app_handle.try_state::<Arc<UsageState>>() {
        state.record_active_day(&usage::local_day())
    } else {
        log::warn!("record_active_day called before UsageState managed; ignoring");
        Ok(())
    }
}

#[tauri::command]
pub async fn get_usage_stats(
    state: tauri::State<'_, Arc<UsageState>>,
    search: tauri::State<'_, Arc<crate::search_engine::SearchState>>,
) -> Result<usage::UsageStats, UsageError> {
    let mut stats = state.stats()?;
    // Enrich raw ids with friendly titles from the search index (rust-first).
    for item in stats.top.iter_mut() {
        item.label = search.display_title(&item.id);
    }
    Ok(stats)
}

/// Records locally only; cannot cause egress even when usage sharing is off.
#[tauri::command]
pub async fn record_worker_fallback<R: tauri::Runtime>(
    extension_id: String,
    app_handle: tauri::AppHandle<R>,
) -> Result<(), UsageError> {
    use tauri::Manager;
    if let Some(state) = app_handle.try_state::<Arc<UsageState>>() {
        state.record_worker_fallback(&extension_id, &usage::local_day())
    } else {
        log::warn!("record_worker_fallback called before UsageState managed; ignoring");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;

    #[tokio::test]
    async fn record_active_day_gracefully_handles_unmanaged_state() {
        let app = tauri::test::mock_app();
        let result = record_active_day(app.handle().clone()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn record_active_day_records_when_state_is_managed() {
        let app = tauri::test::mock_app();
        let usage = Arc::new(UsageState::in_memory().unwrap());
        app.manage(usage.clone());

        let result = record_active_day(app.handle().clone()).await;
        assert!(result.is_ok());

        let stats = usage.stats().unwrap();
        assert_eq!(stats.active_days, 1);
    }

    #[tokio::test]
    async fn record_worker_fallback_gracefully_handles_unmanaged_state() {
        let app = tauri::test::mock_app();
        let result = record_worker_fallback("ext_1".into(), app.handle().clone()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn record_worker_fallback_records_when_state_is_managed() {
        let app = tauri::test::mock_app();
        let usage = Arc::new(UsageState::in_memory().unwrap());
        app.manage(usage.clone());

        let result = record_worker_fallback("ext_1".into(), app.handle().clone()).await;
        assert!(result.is_ok());
    }
}
