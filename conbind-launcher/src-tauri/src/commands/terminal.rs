//! Tauri command layer for the launcher's `>` terminal prefix.
//!
//! Host-only surface, like `battery`: not mapped in the `conbind:api:*`
//! permission table, so extension workers cannot call it. The command text is
//! typed by the user in the launcher and never comes from an extension.

use crate::error::AppError;
use crate::terminal::{self, TerminalEvent, TerminalSession};
use tauri::ipc::Channel;
use tauri::State;

/// Runs `command` in the session and streams its output over `on_event`; the
/// last event is `done`.
#[tauri::command]
pub async fn terminal_run(
    session: State<'_, TerminalSession>,
    command: String,
    on_event: Channel<TerminalEvent>,
) -> Result<(), AppError> {
    terminal::run_command(&session, &command, move |event| {
        let _ = on_event.send(event);
    })
    .await
}

/// Kills the running command. Returns whether there was one.
#[tauri::command]
pub fn terminal_cancel(session: State<'_, TerminalSession>) -> Result<bool, AppError> {
    session.cancel()
}

/// The directory the next command starts in.
#[tauri::command]
pub fn terminal_cwd(session: State<'_, TerminalSession>) -> Result<String, AppError> {
    session.cwd()
}
