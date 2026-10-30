//! Windows backend, via the System Media Transport Controls (SMTC) session
//! manager — the same source the volume flyout's media card reads.
//!
//! The "current session" is whichever player Windows considers active, so
//! commands and the Now Playing readout always agree with the OS.

use crate::error::AppError;
use crate::media::{aumid_app_name, MediaBackend, MediaCommand, NowPlaying};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};

pub struct WindowsMediaBackend;

impl WindowsMediaBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsMediaBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn platform_err(context: &str, e: windows::core::Error) -> AppError {
    AppError::Platform(format!("{context}: {e}"))
}

/// The OS's current media session, if any player has registered one.
fn current_session() -> Result<Option<Session>, AppError> {
    let manager = Manager::RequestAsync()
        .and_then(|op| op.join())
        .map_err(|e| platform_err("SMTC session manager", e))?;
    Ok(manager.GetCurrentSession().ok())
}

impl MediaBackend for WindowsMediaBackend {
    fn supported(&self) -> bool {
        Manager::RequestAsync().is_ok()
    }

    fn send(&self, command: MediaCommand) -> Result<(), AppError> {
        let session = current_session()?
            .ok_or_else(|| AppError::Platform("no active media session".into()))?;
        let accepted = match command {
            MediaCommand::PlayPause => session.TryTogglePlayPauseAsync(),
            MediaCommand::Play => session.TryPlayAsync(),
            MediaCommand::Pause => session.TryPauseAsync(),
            MediaCommand::Next => session.TrySkipNextAsync(),
            MediaCommand::Previous => session.TrySkipPreviousAsync(),
            MediaCommand::Stop => session.TryStopAsync(),
        }
        .and_then(|op| op.join())
        .map_err(|e| platform_err(command.label(), e))?;
        if accepted {
            Ok(())
        } else {
            // The player is running but does not support this transport action.
            Err(AppError::Platform(format!(
                "the active player did not accept \"{}\"",
                command.label()
            )))
        }
    }

    fn now_playing(&self) -> Result<Option<NowPlaying>, AppError> {
        let Some(session) = current_session()? else {
            return Ok(None);
        };
        let properties = session
            .TryGetMediaPropertiesAsync()
            .and_then(|op| op.join())
            .map_err(|e| platform_err("media properties", e))?;
        let is_playing = session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map(|status| status == PlaybackStatus::Playing)
            .unwrap_or(false);
        let text = |value: windows::core::Result<windows::core::HSTRING>| {
            value.map(|s| s.to_string()).ok()
        };
        let app = text(session.SourceAppUserModelId()).map(|id| aumid_app_name(&id));
        Ok(NowPlaying::from_raw(
            text(properties.Title()).as_deref(),
            text(properties.Artist()).as_deref(),
            text(properties.AlbumTitle()).as_deref(),
            app.as_deref(),
            is_playing,
        ))
    }
}
