//! macOS backend.
//!
//! Layered, because Apple restricted the MediaRemote framework to Apple-signed
//! processes in macOS 15.4:
//!
//! 1. **MediaRemote adapter** (preferred): a bundled helper framework that is
//!    loaded into `/usr/bin/perl`, which still carries the entitlement. It
//!    can both read the system-wide now-playing state and send transport
//!    commands to whichever app owns it. Its `test` command is run once and
//!    cached; if it fails (or the adapter is not bundled) the layers below
//!    take over.
//! 2. **Media keys**: synthetic `NX_SYSDEFINED` events, the same thing the
//!    keyboard's media keys post. Transport only, any player, and needs
//!    Accessibility permission (prompted for on first use; without it the
//!    command goes to layer 3 instead).
//! 3. **AppleScript**: Music and Spotify only, for both transport and reading.
//!    First use triggers a one-time Automation consent prompt per app.
//!
//! When nothing can read the state, `now_playing()` returns `Ok(None)` and the
//! launcher hides the Now Playing row; commands keep working through whichever
//! layer is left.

use crate::error::AppError;
use crate::media::{MediaBackend, MediaCommand, NowPlaying};
use objc2::encode::{Encode, Encoding};
use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};
use objc2_foundation::NSPoint;
use std::ffi::c_void;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Directory (under the app's resources) holding the adapter files.
const ADAPTER_DIR: &str = "mediaremote-adapter";
const ADAPTER_SCRIPT: &str = "mediaremote-adapter.pl";
const ADAPTER_FRAMEWORK: &str = "MediaRemoteAdapter.framework";
const ADAPTER_TEST_CLIENT: &str = "MediaRemoteAdapterTestClient";

/// A `get` should answer in well under this; a hung helper must never stall
/// the launcher's search budget for long.
const READ_TIMEOUT: Duration = Duration::from_millis(1500);
const SEND_TIMEOUT: Duration = Duration::from_millis(3000);
const TEST_TIMEOUT: Duration = Duration::from_millis(5000);

struct AdapterPaths {
    script: PathBuf,
    framework: PathBuf,
    test_client: PathBuf,
}

impl AdapterPaths {
    fn in_dir(dir: &Path) -> Option<Self> {
        let paths = Self {
            script: dir.join(ADAPTER_SCRIPT),
            framework: dir.join(ADAPTER_FRAMEWORK),
            test_client: dir.join(ADAPTER_TEST_CLIENT),
        };
        (paths.script.is_file() && paths.framework.is_dir() && paths.test_client.is_file())
            .then_some(paths)
    }

    /// `perl <script> <framework> [<test client>] <args…>`
    fn command(&self, with_test_client: bool, args: &[&str]) -> Command {
        let mut command = Command::new("/usr/bin/perl");
        command.arg(&self.script).arg(&self.framework);
        if with_test_client {
            command.arg(&self.test_client);
        }
        command.args(args);
        command
    }
}

/// Where the adapter may live, most specific first: the packaged app
/// (`Contents/MacOS/<exe>` → `Contents/Resources/resources/…`) and the
/// `src-tauri/resources` folder used by `tauri dev`.
fn adapter_dir_candidates(exe: Option<&Path>, manifest_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(contents) = exe.and_then(|p| p.parent()).and_then(|p| p.parent()) {
        candidates.push(
            contents
                .join("Resources")
                .join("resources")
                .join(ADAPTER_DIR),
        );
    }
    candidates.push(manifest_dir.join("resources").join(ADAPTER_DIR));
    candidates
}

fn find_adapter() -> Option<AdapterPaths> {
    let exe = std::env::current_exe().ok();
    adapter_dir_candidates(exe.as_deref(), Path::new(env!("CARGO_MANIFEST_DIR")))
        .iter()
        .find_map(|dir| AdapterPaths::in_dir(dir))
}

/// Runs `command`, killing it if it outlives `timeout`. Returns stdout.
fn run_with_timeout(
    label: &str,
    command: &mut Command,
    timeout: Duration,
) -> Result<String, AppError> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Platform(format!("{label}: failed to spawn: {e}")))?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::Platform(format!("{label}: timed out")));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(AppError::Platform(format!("{label}: wait failed: {e}"))),
        }
    };
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut stdout);
    }
    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        return Err(AppError::Platform(format!(
            "{label}: exited with {status}: {}",
            stderr.trim()
        )));
    }
    Ok(stdout)
}

/// MediaRemote command ids used by the adapter's `send`.
fn adapter_command_id(command: MediaCommand) -> u8 {
    match command {
        MediaCommand::Play => 0,
        MediaCommand::Pause => 1,
        MediaCommand::PlayPause => 2,
        MediaCommand::Stop => 3,
        MediaCommand::Next => 4,
        MediaCommand::Previous => 5,
    }
}

/// Friendly name for a bundle id: well-known players by name, otherwise the
/// last dotted component (`com.example.Player` → `Player`).
fn app_name_from_bundle_id(bundle_id: &str) -> String {
    match bundle_id {
        "com.apple.Music" => "Music".into(),
        "com.spotify.client" => "Spotify".into(),
        "com.apple.Safari" => "Safari".into(),
        "com.google.Chrome" => "Chrome".into(),
        "org.mozilla.firefox" => "Firefox".into(),
        other => other.rsplit('.').next().unwrap_or(other).to_owned(),
    }
}

/// Parses the adapter's `get` output: `null` or a JSON object.
fn parse_adapter_output(output: &str) -> Result<Option<NowPlaying>, AppError> {
    let value: serde_json::Value = serde_json::from_str(output.trim())?;
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    let text = |key: &str| object.get(key).and_then(|v| v.as_str());
    let app = text("parentApplicationBundleIdentifier")
        .or_else(|| text("bundleIdentifier"))
        .map(app_name_from_bundle_id);
    Ok(NowPlaying::from_raw(
        text("title"),
        text("artist"),
        text("album"),
        app.as_deref(),
        object
            .get("playing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    ))
}

enum AdapterProbe {
    Unknown,
    Working(Arc<AdapterPaths>),
    Failed(Instant),
}

/// How long a failed probe is trusted before trying again. A one-off failure
/// (first launch of a freshly installed helper, a slow machine) must not
/// disable the adapter until the app restarts.
const PROBE_RETRY: Duration = Duration::from_secs(60);

/// The adapter, if it is bundled *and* works on this macOS release. A working
/// adapter is remembered for the process: `test` loads the helper into perl,
/// which is too slow to repeat for every search. The lock is held across the
/// probe so concurrent callers share one run.
fn adapter() -> Option<Arc<AdapterPaths>> {
    static PROBE: Mutex<AdapterProbe> = Mutex::new(AdapterProbe::Unknown);
    let mut probe = PROBE.lock().unwrap_or_else(|e| e.into_inner());
    match &*probe {
        AdapterProbe::Working(paths) => return Some(Arc::clone(paths)),
        AdapterProbe::Failed(at) if at.elapsed() < PROBE_RETRY => return None,
        _ => {}
    }
    let working = find_adapter().and_then(|paths| {
        match run_with_timeout(
            "mediaremote adapter test",
            &mut paths.command(true, &["test"]),
            TEST_TIMEOUT,
        ) {
            Ok(_) => Some(Arc::new(paths)),
            Err(e) => {
                log::warn!("[media] MediaRemote adapter unusable, falling back: {e}");
                None
            }
        }
    });
    *probe = match &working {
        Some(paths) => AdapterProbe::Working(Arc::clone(paths)),
        None => AdapterProbe::Failed(Instant::now()),
    };
    working
}

// ── Media keys ──────────────────────────────────────────────────────────

/// `NX_KEYTYPE_*` codes from IOKit's `ev_keymap.h`.
const NX_KEYTYPE_PLAY: i32 = 16;
const NX_KEYTYPE_NEXT: i32 = 17;
const NX_KEYTYPE_PREVIOUS: i32 = 18;

fn media_key_for(command: MediaCommand) -> Option<i32> {
    match command {
        MediaCommand::PlayPause => Some(NX_KEYTYPE_PLAY),
        MediaCommand::Next => Some(NX_KEYTYPE_NEXT),
        MediaCommand::Previous => Some(NX_KEYTYPE_PREVIOUS),
        // The keyboard has no discrete play, pause or stop key.
        MediaCommand::Play | MediaCommand::Pause | MediaCommand::Stop => None,
    }
}

/// `data1` of an `NX_SYSDEFINED` subtype-8 event: key code in the high word,
/// `0xA` (down) or `0xB` (up) in the flags byte.
fn media_key_data1(key: i32, down: bool) -> isize {
    ((key as isize) << 16) | ((if down { 0xA } else { 0xB }) << 8)
}

/// `CGEventRef`, declared so objc2's debug encoding check accepts the
/// `-[NSEvent CGEvent]` return type.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct CGEventRef(*mut c_void);

// SAFETY: transparent wrapper around the pointer `-[NSEvent CGEvent]` returns.
unsafe impl Encode for CGEventRef {
    const ENCODING: Encoding = Encoding::Pointer(&Encoding::Struct("__CGEvent", &[]));
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventPost(tap: u32, event: CGEventRef);
}

const KCG_HID_EVENT_TAP: u32 = 0;
const NS_EVENT_TYPE_SYSTEM_DEFINED: usize = 14;
const NS_EVENT_SUBTYPE_AUX_CONTROL_BUTTONS: i16 = 8;

fn post_media_key(key: i32) -> Result<(), AppError> {
    // Posting HID events needs Accessibility; prompt, since this only runs
    // for a command the user just asked for.
    if !crate::platform::macos::request_accessibility_trust() {
        return Err(AppError::Permission(
            "Accessibility permission required to send media keys. \
             Enable it in System Settings > Privacy & Security > Accessibility."
                .to_string(),
        ));
    }
    let class = AnyClass::get("NSEvent")
        .ok_or_else(|| AppError::Platform("NSEvent class unavailable".into()))?;
    for down in [true, false] {
        let flags: usize = if down { 0xA00 } else { 0xB00 };
        // SAFETY: argument types match
        // `+[NSEvent otherEventWithType:location:modifierFlags:timestamp:
        //   windowNumber:context:subtype:data1:data2:]`; the returned event is
        // autoreleased and only used within this iteration.
        unsafe {
            let event: *mut AnyObject = msg_send![
                class,
                otherEventWithType: NS_EVENT_TYPE_SYSTEM_DEFINED,
                location: NSPoint::new(0.0, 0.0),
                modifierFlags: flags,
                timestamp: 0.0f64,
                windowNumber: 0isize,
                context: std::ptr::null_mut::<AnyObject>(),
                subtype: NS_EVENT_SUBTYPE_AUX_CONTROL_BUTTONS,
                data1: media_key_data1(key, down),
                data2: -1isize
            ];
            if event.is_null() {
                return Err(AppError::Platform(
                    "failed to create media key event".into(),
                ));
            }
            let cg_event: CGEventRef = msg_send![event, CGEvent];
            if cg_event.0.is_null() {
                return Err(AppError::Platform("media key event has no CGEvent".into()));
            }
            CGEventPost(KCG_HID_EVENT_TAP, cg_event);
        }
    }
    Ok(())
}

// ── AppleScript (Music / Spotify) ──────────────────────────────────────

const SCRIPTABLE_PLAYERS: [&str; 2] = ["Music", "Spotify"];

/// Reads one player without launching it. Prints `state\ntitle\nartist\nalbum`
/// or nothing when the app is closed or has no current track.
fn read_script(app: &str) -> String {
    format!(
        "if application \"{app}\" is running then\n\
         tell application \"{app}\"\n\
         if player state is stopped then return \"\"\n\
         set s to (player state as text)\n\
         return s & linefeed & (name of current track) & linefeed & (artist of current track) & linefeed & (album of current track)\n\
         end tell\n\
         end if\n\
         return \"\""
    )
}

fn parse_script_output(app: &str, output: &str) -> Option<NowPlaying> {
    let mut lines = output.trim_end_matches('\n').split('\n');
    let state = lines.next()?.trim();
    if state.is_empty() {
        return None;
    }
    NowPlaying::from_raw(
        lines.next(),
        lines.next(),
        lines.next(),
        Some(app),
        state.eq_ignore_ascii_case("playing"),
    )
}

fn osascript(script: &str) -> Command {
    let mut command = Command::new("/usr/bin/osascript");
    command.arg("-e").arg(script);
    command
}

fn applescript_now_playing() -> Option<NowPlaying> {
    let mut paused = None;
    for app in SCRIPTABLE_PLAYERS {
        let Ok(out) = run_with_timeout(
            "media osascript",
            &mut osascript(&read_script(app)),
            READ_TIMEOUT,
        ) else {
            continue;
        };
        match parse_script_output(app, &out) {
            Some(track) if track.is_playing => return Some(track),
            Some(track) => paused = paused.or(Some(track)),
            None => {}
        }
    }
    paused
}

fn applescript_verb(app: &str, command: MediaCommand) -> &'static str {
    match command {
        MediaCommand::PlayPause => "playpause",
        MediaCommand::Play => "play",
        MediaCommand::Pause => "pause",
        MediaCommand::Next => "next track",
        MediaCommand::Previous => "previous track",
        // Spotify has no stop verb; pausing is the closest equivalent.
        MediaCommand::Stop if app == "Music" => "stop",
        MediaCommand::Stop => "pause",
    }
}

fn applescript_send(command: MediaCommand) -> Result<(), AppError> {
    let track = applescript_now_playing()
        .ok_or_else(|| AppError::Platform("no scriptable media player is active".into()))?;
    let app = track.app_name.as_deref().unwrap_or("Music");
    let script = format!(
        "tell application \"{app}\" to {}",
        applescript_verb(app, command)
    );
    run_with_timeout("media osascript", &mut osascript(&script), SEND_TIMEOUT).map(|_| ())
}

// ── Backend ────────────────────────────────────────────────────────────

pub struct MacMediaBackend;

impl MacMediaBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacMediaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaBackend for MacMediaBackend {
    fn supported(&self) -> bool {
        true
    }

    fn send(&self, command: MediaCommand) -> Result<(), AppError> {
        if let Some(adapter) = adapter() {
            let id = adapter_command_id(command).to_string();
            match run_with_timeout(
                "mediaremote adapter send",
                &mut adapter.command(false, &["send", &id]),
                SEND_TIMEOUT,
            ) {
                Ok(_) => return Ok(()),
                Err(e) => log::warn!("[media] adapter send failed, falling back: {e}"),
            }
        }
        if let Some(key) = media_key_for(command) {
            // Without Accessibility the key cannot be posted; Music/Spotify can
            // still be scripted, so only report the permission error if they
            // cannot take the command either.
            return post_media_key(key).or_else(|e| applescript_send(command).map_err(|_| e));
        }
        applescript_send(command)
    }

    fn now_playing(&self) -> Result<Option<NowPlaying>, AppError> {
        if let Some(adapter) = adapter() {
            match run_with_timeout(
                "mediaremote adapter get",
                &mut adapter.command(false, &["get", "--no-artwork"]),
                READ_TIMEOUT,
            )
            .and_then(|out| parse_adapter_output(&out))
            {
                Ok(track) => return Ok(track),
                Err(e) => log::warn!("[media] adapter get failed, falling back: {e}"),
            }
        }
        Ok(applescript_now_playing())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_command_ids_match_mediaremote_table() {
        assert_eq!(adapter_command_id(MediaCommand::Play), 0);
        assert_eq!(adapter_command_id(MediaCommand::Pause), 1);
        assert_eq!(adapter_command_id(MediaCommand::PlayPause), 2);
        assert_eq!(adapter_command_id(MediaCommand::Stop), 3);
        assert_eq!(adapter_command_id(MediaCommand::Next), 4);
        assert_eq!(adapter_command_id(MediaCommand::Previous), 5);
    }

    #[test]
    fn adapter_command_line_passes_framework_and_optional_test_client() {
        let paths = AdapterPaths {
            script: "/a/mediaremote-adapter.pl".into(),
            framework: "/a/MediaRemoteAdapter.framework".into(),
            test_client: "/a/MediaRemoteAdapterTestClient".into(),
        };
        let args = |c: &Command| -> Vec<String> {
            c.get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect()
        };
        let get = paths.command(false, &["get", "--no-artwork"]);
        assert_eq!(get.get_program(), "/usr/bin/perl");
        assert_eq!(
            args(&get),
            vec![
                "/a/mediaremote-adapter.pl",
                "/a/MediaRemoteAdapter.framework",
                "get",
                "--no-artwork"
            ]
        );
        assert_eq!(
            args(&paths.command(true, &["test"])),
            vec![
                "/a/mediaremote-adapter.pl",
                "/a/MediaRemoteAdapter.framework",
                "/a/MediaRemoteAdapterTestClient",
                "test"
            ]
        );
    }

    #[test]
    fn adapter_is_looked_up_in_bundle_then_dev_resources() {
        let candidates = adapter_dir_candidates(
            Some(Path::new(
                "/Applications/Conbind.app/Contents/MacOS/conbind",
            )),
            Path::new("/repo/src-tauri"),
        );
        assert_eq!(
            candidates,
            vec![
                PathBuf::from(
                    "/Applications/Conbind.app/Contents/Resources/resources/mediaremote-adapter"
                ),
                PathBuf::from("/repo/src-tauri/resources/mediaremote-adapter"),
            ]
        );
    }

    #[test]
    fn parses_adapter_now_playing() {
        let json = r#"{"bundleIdentifier":"com.spotify.client","playing":true,
            "title":"Song","artist":"Artist","album":"Album","duration":200.0}"#;
        let track = parse_adapter_output(json).unwrap().expect("track");
        assert_eq!(track.title, "Song");
        assert_eq!(track.artist.as_deref(), Some("Artist"));
        assert_eq!(track.app_name.as_deref(), Some("Spotify"));
        assert!(track.is_playing);
    }

    #[test]
    fn adapter_prefers_parent_app_for_web_players() {
        let json = r#"{"bundleIdentifier":"com.apple.WebKit.GPU","parentApplicationBundleIdentifier":"com.apple.Safari","playing":false,"title":"Video"}"#;
        let track = parse_adapter_output(json).unwrap().expect("track");
        assert_eq!(track.app_name.as_deref(), Some("Safari"));
        assert!(!track.is_playing);
    }

    #[test]
    fn adapter_null_means_nothing_playing() {
        assert_eq!(parse_adapter_output("null\n").unwrap(), None);
    }

    #[test]
    fn adapter_garbage_is_an_error() {
        assert!(parse_adapter_output("not json").is_err());
    }

    #[test]
    fn unknown_bundle_ids_use_last_component() {
        assert_eq!(app_name_from_bundle_id("com.example.Player"), "Player");
        assert_eq!(app_name_from_bundle_id("com.apple.Music"), "Music");
    }

    #[test]
    fn media_key_events_encode_key_and_direction() {
        // Matches the data1 the keyboard's play key posts: 0x00100A00 / 0x00100B00.
        assert_eq!(media_key_data1(NX_KEYTYPE_PLAY, true), 0x0010_0A00);
        assert_eq!(media_key_data1(NX_KEYTYPE_PLAY, false), 0x0010_0B00);
        assert_eq!(media_key_data1(NX_KEYTYPE_NEXT, true), 0x0011_0A00);
    }

    #[test]
    fn only_toggle_and_skip_have_media_keys() {
        assert_eq!(media_key_for(MediaCommand::PlayPause), Some(16));
        assert_eq!(media_key_for(MediaCommand::Next), Some(17));
        assert_eq!(media_key_for(MediaCommand::Previous), Some(18));
        assert_eq!(media_key_for(MediaCommand::Stop), None);
        assert_eq!(media_key_for(MediaCommand::Play), None);
    }

    #[test]
    fn parses_applescript_player_output() {
        let track = parse_script_output("Music", "playing\nSong\nArtist\nAlbum\n").expect("track");
        assert_eq!(track.title, "Song");
        assert_eq!(track.app_name.as_deref(), Some("Music"));
        assert!(track.is_playing);
        let paused = parse_script_output("Spotify", "paused\nSong\nArtist\nAlbum").expect("track");
        assert!(!paused.is_playing);
    }

    #[test]
    fn empty_applescript_output_means_no_player() {
        assert_eq!(parse_script_output("Music", ""), None);
        assert_eq!(parse_script_output("Music", "\n"), None);
    }

    #[test]
    fn applescript_stop_falls_back_to_pause_for_spotify() {
        assert_eq!(applescript_verb("Music", MediaCommand::Stop), "stop");
        assert_eq!(applescript_verb("Spotify", MediaCommand::Stop), "pause");
        assert_eq!(applescript_verb("Music", MediaCommand::Next), "next track");
    }

    /// Needs `scripts/build-mediaremote-adapter.sh` to have run. Read-only:
    /// never sends a command, so it cannot disturb whatever is playing.
    #[test]
    #[ignore = "needs the built MediaRemote adapter; run with --ignored"]
    fn adapter_reads_now_playing_on_this_machine() {
        let paths = find_adapter().expect("adapter not staged; run the build script");
        run_with_timeout(
            "mediaremote adapter test",
            &mut paths.command(true, &["test"]),
            TEST_TIMEOUT,
        )
        .expect("adapter `test` should pass");
        assert!(
            adapter().is_some(),
            "adapter() should cache a working adapter"
        );
        MacMediaBackend::new()
            .now_playing()
            .expect("adapter get should succeed (None when nothing plays)");
    }

    #[test]
    fn read_script_does_not_launch_the_app() {
        let script = read_script("Music");
        assert!(script.starts_with("if application \"Music\" is running"));
    }
}
