//! Tauri command layer for media transport controls and the Now Playing row.
//!
//! Host-only surface for the `media` built-in feature: like
//! `system_action_run`, these commands are not mapped in the `conbind:api:*`
//! permission table, so extension workers cannot reach them through the
//! broker. Exposing them to extensions would need a dedicated permission plus
//! consent wiring first.

use crate::error::AppError;
use crate::media::{MediaCommand, MediaState, NowPlaying};
use std::sync::Arc;
use tauri::State;

async fn spawn_blocking_result<T, F>(work: F) -> Result<T, AppError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| AppError::Other(format!("media task failed: {e}")))
}

/// Whether this machine can attempt media transport at all.
///
/// All media commands are `async` so the blocking platform work (D-Bus calls
/// on Linux, process spawns on macOS, WinRT calls on Windows) runs on
/// Tauri's thread pool instead of the main thread.
#[tauri::command]
pub async fn media_supported(state: State<'_, Arc<MediaState>>) -> Result<bool, AppError> {
    let state = Arc::clone(state.inner());
    spawn_blocking_result(move || state.supported()).await
}

#[tauri::command]
pub async fn media_command(
    state: State<'_, Arc<MediaState>>,
    command: MediaCommand,
) -> Result<(), AppError> {
    let state = Arc::clone(state.inner());
    spawn_blocking_result(move || state.send(command)).await?
}

/// Current track, or `None` when nothing is playing or readable.
#[tauri::command]
pub async fn media_now_playing(
    state: State<'_, Arc<MediaState>>,
) -> Result<Option<NowPlaying>, AppError> {
    let state = Arc::clone(state.inner());
    spawn_blocking_result(move || state.now_playing()).await?
}

#[cfg(test)]
mod tests {
    use super::spawn_blocking_result;
    use crate::media::fake::FakeBackend;
    use crate::media::{MediaCommand, MediaState};

    #[test]
    fn supported_reflects_backend() {
        assert!(MediaState::new(Box::new(FakeBackend::new(true))).supported());
        assert!(!MediaState::new(Box::new(FakeBackend::new(false))).supported());
    }

    #[test]
    fn send_rejects_when_backend_unsupported() {
        let fake = FakeBackend::new(false);
        let state = MediaState::new(Box::new(fake.clone()));
        assert!(state.send(MediaCommand::Next).is_err());
        assert!(fake.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn blocking_helper_runs_work_off_the_async_thread() {
        let caller = std::thread::current().id();
        let worker = spawn_blocking_result(|| std::thread::current().id())
            .await
            .expect("blocking task");
        assert_ne!(caller, worker);
    }
}
