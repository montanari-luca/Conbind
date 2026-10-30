//! Media transport controls (play/pause, next, previous, stop) and the
//! "now playing" readout.
//!
//! Platform-neutral state + backend trait, mirroring `system_actions`.
//! Per-platform backends live in sibling modules (`macos`, `linux`,
//! `windows`) and implement [`MediaBackend`].
//!
//! Transport is fire-and-forget and targets whichever player the OS
//! considers active. Reading what is playing is best-effort: a backend that
//! cannot see any player returns `Ok(None)` rather than an error so the
//! launcher can simply hide the Now Playing row.

use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum MediaCommand {
    PlayPause,
    Play,
    Pause,
    Next,
    Previous,
    Stop,
}

impl MediaCommand {
    pub fn label(&self) -> &'static str {
        match self {
            MediaCommand::PlayPause => "play/pause",
            MediaCommand::Play => "play",
            MediaCommand::Pause => "pause",
            MediaCommand::Next => "next track",
            MediaCommand::Previous => "previous track",
            MediaCommand::Stop => "stop",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Display name of the player app, when the platform reports one.
    pub app_name: Option<String>,
    pub is_playing: bool,
}

impl NowPlaying {
    /// Builds a readout from raw platform strings, treating blank values as
    /// absent. Returns `None` when there is no title: a player that reports
    /// nothing but a status is not worth a row.
    pub(crate) fn from_raw(
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        app_name: Option<&str>,
        is_playing: bool,
    ) -> Option<Self> {
        let clean = |value: Option<&str>| {
            value
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        Some(Self {
            title: clean(title)?,
            artist: clean(artist),
            album: clean(album),
            app_name: clean(app_name),
            is_playing,
        })
    }
}

/// Picks the index of the player to control: the first one that is playing,
/// otherwise the first one listed. Used by backends that enumerate several
/// players (MPRIS).
#[cfg(any(target_os = "linux", test))]
pub(crate) fn pick_player(playing: &[bool]) -> Option<usize> {
    playing
        .iter()
        .position(|p| *p)
        .or(if playing.is_empty() { None } else { Some(0) })
}

/// Display name for an MPRIS bus name: `org.mpris.MediaPlayer2.spotify` →
/// `spotify`, and `…vlc.instance1234` → `vlc` (multi-instance players append
/// a per-process suffix).
#[cfg(any(target_os = "linux", test))]
pub(crate) fn mpris_app_name(bus_name: &str) -> String {
    let name = bus_name
        .strip_prefix("org.mpris.MediaPlayer2.")
        .unwrap_or(bus_name);
    match name.rsplit_once(".instance") {
        Some((base, suffix)) if suffix.chars().all(|c| c.is_ascii_digit()) => base.to_owned(),
        _ => name.to_owned(),
    }
}

/// Display name for a Windows AppUserModelId: `Spotify.exe` → `Spotify`,
/// `Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic` → `ZuneMusic`.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn aumid_app_name(aumid: &str) -> String {
    let id = aumid.rsplit('!').next().unwrap_or(aumid);
    let id = id.strip_suffix(".exe").unwrap_or(id);
    id.rsplit('.').next().unwrap_or(id).to_owned()
}

/// Implemented by each per-platform backend.
pub trait MediaBackend: Send + Sync {
    /// Whether transport commands can be attempted on this machine.
    fn supported(&self) -> bool;

    /// Send a transport command to the active player.
    fn send(&self, command: MediaCommand) -> Result<(), AppError>;

    /// Current track, or `None` when nothing is playing or readable.
    fn now_playing(&self) -> Result<Option<NowPlaying>, AppError>;
}

/// Tauri managed state wrapping the platform backend.
pub struct MediaState {
    backend: Box<dyn MediaBackend>,
}

impl MediaState {
    pub fn new(backend: Box<dyn MediaBackend>) -> Self {
        Self { backend }
    }

    pub fn supported(&self) -> bool {
        self.backend.supported()
    }

    /// Fail-closed: re-checks `supported()` so a command never reaches a
    /// backend that cannot perform it.
    pub fn send(&self, command: MediaCommand) -> Result<(), AppError> {
        if !self.backend.supported() {
            return Err(AppError::Platform(format!(
                "media command \"{}\" is not supported on this machine",
                command.label()
            )));
        }
        self.backend.send(command)
    }

    pub fn now_playing(&self) -> Result<Option<NowPlaying>, AppError> {
        if !self.backend.supported() {
            return Ok(None);
        }
        self.backend.now_playing()
    }
}

/// In-memory fake backend for tests and unsupported platforms.
pub mod fake {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub struct FakeBackend {
        pub supported: bool,
        pub now_playing: Arc<Mutex<Option<NowPlaying>>>,
        pub sent: Arc<Mutex<Vec<MediaCommand>>>,
        pub fail_next: Arc<Mutex<bool>>,
    }

    impl FakeBackend {
        pub fn new(supported: bool) -> Self {
            Self {
                supported,
                ..Self::default()
            }
        }

        pub fn set_now_playing(&self, value: Option<NowPlaying>) {
            *self.now_playing.lock().unwrap() = value;
        }

        pub fn fail_next_send(&self) {
            *self.fail_next.lock().unwrap() = true;
        }
    }

    impl MediaBackend for FakeBackend {
        fn supported(&self) -> bool {
            self.supported
        }

        fn send(&self, command: MediaCommand) -> Result<(), AppError> {
            let mut fail = self.fail_next.lock().map_err(|_| AppError::Lock)?;
            if *fail {
                *fail = false;
                return Err(AppError::Platform("fake failure".into()));
            }
            self.sent.lock().map_err(|_| AppError::Lock)?.push(command);
            Ok(())
        }

        fn now_playing(&self) -> Result<Option<NowPlaying>, AppError> {
            Ok(self.now_playing.lock().map_err(|_| AppError::Lock)?.clone())
        }
    }
}

/// Returns the default per-platform backend.
#[cfg(target_os = "macos")]
pub fn default_backend() -> Box<dyn MediaBackend> {
    Box::new(macos::MacMediaBackend::new())
}
#[cfg(target_os = "linux")]
pub fn default_backend() -> Box<dyn MediaBackend> {
    Box::new(linux::LinuxMediaBackend::new())
}
#[cfg(target_os = "windows")]
pub fn default_backend() -> Box<dyn MediaBackend> {
    Box::new(windows::WindowsMediaBackend::new())
}
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn default_backend() -> Box<dyn MediaBackend> {
    Box::new(fake::FakeBackend::new(false))
}

#[cfg(test)]
mod tests {
    use super::fake::FakeBackend;
    use super::*;

    #[test]
    fn send_reaches_backend() {
        let fake = FakeBackend::new(true);
        let state = MediaState::new(Box::new(fake.clone()));
        state.send(MediaCommand::Next).expect("send ok");
        assert_eq!(*fake.sent.lock().unwrap(), vec![MediaCommand::Next]);
    }

    #[test]
    fn send_on_unsupported_backend_is_rejected_before_backend() {
        let fake = FakeBackend::new(false);
        let state = MediaState::new(Box::new(fake.clone()));
        let err = state.send(MediaCommand::PlayPause).unwrap_err();
        assert!(matches!(err, AppError::Platform(_)), "got: {err:?}");
        assert!(fake.sent.lock().unwrap().is_empty());
    }

    #[test]
    fn backend_failure_is_propagated() {
        let fake = FakeBackend::new(true);
        fake.fail_next_send();
        let state = MediaState::new(Box::new(fake));
        let err = state.send(MediaCommand::Stop).unwrap_err();
        assert!(matches!(err, AppError::Platform(_)), "got: {err:?}");
    }

    #[test]
    fn now_playing_is_none_when_unsupported_even_if_backend_has_data() {
        let fake = FakeBackend::new(false);
        fake.set_now_playing(NowPlaying::from_raw(Some("Song"), None, None, None, true));
        let state = MediaState::new(Box::new(fake));
        assert_eq!(state.now_playing().unwrap(), None);
    }

    #[test]
    fn now_playing_passes_through() {
        let fake = FakeBackend::new(true);
        let track = NowPlaying::from_raw(Some("Song"), Some("Artist"), None, Some("Music"), true);
        fake.set_now_playing(track.clone());
        let state = MediaState::new(Box::new(fake));
        assert_eq!(state.now_playing().unwrap(), track);
    }

    #[test]
    fn from_raw_trims_and_drops_blank_fields() {
        let track = NowPlaying::from_raw(
            Some("  Song  "),
            Some("   "),
            Some(""),
            Some(" Spotify "),
            false,
        )
        .expect("title present");
        assert_eq!(track.title, "Song");
        assert_eq!(track.artist, None);
        assert_eq!(track.album, None);
        assert_eq!(track.app_name.as_deref(), Some("Spotify"));
        assert!(!track.is_playing);
    }

    #[test]
    fn from_raw_requires_a_title() {
        assert_eq!(
            NowPlaying::from_raw(None, Some("Artist"), None, None, true),
            None
        );
        assert_eq!(
            NowPlaying::from_raw(Some("  "), Some("Artist"), None, None, true),
            None
        );
    }

    #[test]
    fn pick_player_prefers_playing_then_first() {
        assert_eq!(pick_player(&[]), None);
        assert_eq!(pick_player(&[false, false]), Some(0));
        assert_eq!(pick_player(&[false, true, true]), Some(1));
    }

    #[test]
    fn mpris_app_name_strips_prefix_and_instance_suffix() {
        assert_eq!(mpris_app_name("org.mpris.MediaPlayer2.spotify"), "spotify");
        assert_eq!(
            mpris_app_name("org.mpris.MediaPlayer2.vlc.instance4242"),
            "vlc"
        );
        assert_eq!(
            mpris_app_name("org.mpris.MediaPlayer2.chromium.instancex"),
            "chromium.instancex"
        );
    }

    #[test]
    fn aumid_app_name_handles_exe_and_store_ids() {
        assert_eq!(aumid_app_name("Spotify.exe"), "Spotify");
        assert_eq!(
            aumid_app_name("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"),
            "ZuneMusic"
        );
        assert_eq!(aumid_app_name("chrome.exe"), "chrome");
    }

    #[test]
    fn command_serializes_camel_case() {
        // The frontend keys manifest commands off these exact strings.
        assert_eq!(
            serde_json::to_string(&MediaCommand::PlayPause).unwrap(),
            "\"playPause\""
        );
        let round: MediaCommand = serde_json::from_str("\"previous\"").unwrap();
        assert_eq!(round, MediaCommand::Previous);
    }

    #[test]
    fn now_playing_serializes_camel_case() {
        let track = NowPlaying::from_raw(Some("Song"), None, None, Some("Music"), true).unwrap();
        let json = serde_json::to_value(&track).unwrap();
        assert_eq!(json["appName"], "Music");
        assert_eq!(json["isPlaying"], true);
    }
}
