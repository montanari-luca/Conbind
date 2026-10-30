//! Linux backend, via MPRIS over the session bus (`zbus`, blocking — same
//! transport as `system_actions::linux`).
//!
//! Every media player that implements `org.mpris.MediaPlayer2` owns a bus
//! name under that prefix. We list them, prefer one that is currently
//! `Playing` (otherwise the first), and call the `Player` interface on it.

use crate::error::AppError;
use crate::media::{mpris_app_name, pick_player, MediaBackend, MediaCommand, NowPlaying};
use std::collections::HashMap;
use zbus::blocking::{fdo::DBusProxy, Connection, Proxy};
use zbus::zvariant::{OwnedValue, Value};

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const MPRIS_PLAYER: &str = "org.mpris.MediaPlayer2.Player";

pub struct LinuxMediaBackend;

impl LinuxMediaBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxMediaBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn platform_err(context: &str, e: impl std::fmt::Display) -> AppError {
    AppError::Platform(format!("{context}: {e}"))
}

fn session_bus() -> Result<Connection, AppError> {
    Connection::session().map_err(|e| platform_err("session bus unreachable", e))
}

fn player_proxy<'a>(conn: &'a Connection, bus_name: &'a str) -> Result<Proxy<'a>, AppError> {
    Proxy::new(conn, bus_name, MPRIS_PATH, MPRIS_PLAYER).map_err(|e| platform_err("MPRIS proxy", e))
}

fn is_playing(proxy: &Proxy<'_>) -> bool {
    proxy
        .get_property::<String>("PlaybackStatus")
        .map(|status| status == "Playing")
        .unwrap_or(false)
}

/// Bus names of every MPRIS player currently on the session bus.
fn player_names(conn: &Connection) -> Result<Vec<String>, AppError> {
    let dbus = DBusProxy::new(conn).map_err(|e| platform_err("D-Bus proxy", e))?;
    let names = dbus
        .list_names()
        .map_err(|e| platform_err("ListNames", e))?;
    let mut players: Vec<String> = names
        .into_iter()
        .map(|name| name.to_string())
        .filter(|name| name.starts_with(MPRIS_PREFIX))
        .collect();
    players.sort();
    Ok(players)
}

/// The player to act on, with its bus name.
fn active_player(conn: &Connection) -> Result<Option<String>, AppError> {
    let names = player_names(conn)?;
    let playing: Vec<bool> = names
        .iter()
        .map(|name| {
            player_proxy(conn, name)
                .map(|proxy| is_playing(&proxy))
                .unwrap_or(false)
        })
        .collect();
    Ok(pick_player(&playing).map(|i| names[i].clone()))
}

fn method_for(command: MediaCommand) -> &'static str {
    match command {
        MediaCommand::PlayPause => "PlayPause",
        MediaCommand::Play => "Play",
        MediaCommand::Pause => "Pause",
        MediaCommand::Next => "Next",
        MediaCommand::Previous => "Previous",
        MediaCommand::Stop => "Stop",
    }
}

/// `xesam:title`-style string value, looking through variant wrappers.
fn metadata_string(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    fn text(value: &Value<'_>) -> Option<String> {
        match value {
            Value::Str(s) => Some(s.as_str().to_owned()),
            Value::Value(inner) => text(inner),
            // `xesam:artist` is a list of strings; join them for display.
            Value::Array(items) => {
                let parts: Vec<String> = items.iter().filter_map(text).collect();
                (!parts.is_empty()).then(|| parts.join(", "))
            }
            _ => None,
        }
    }
    metadata.get(key).and_then(|v| text(v))
}

impl MediaBackend for LinuxMediaBackend {
    fn supported(&self) -> bool {
        // Without a session bus there is no MPRIS at all.
        Connection::session().is_ok()
    }

    fn send(&self, command: MediaCommand) -> Result<(), AppError> {
        let conn = session_bus()?;
        let bus_name = active_player(&conn)?
            .ok_or_else(|| AppError::Platform("no MPRIS media player is running".into()))?;
        let proxy = player_proxy(&conn, &bus_name)?;
        proxy
            .call_method(method_for(command), &())
            .map(|_| ())
            .map_err(|e| platform_err(command.label(), e))
    }

    fn now_playing(&self) -> Result<Option<NowPlaying>, AppError> {
        let conn = session_bus()?;
        let Some(bus_name) = active_player(&conn)? else {
            return Ok(None);
        };
        let proxy = player_proxy(&conn, &bus_name)?;
        let metadata: HashMap<String, OwnedValue> = proxy
            .get_property("Metadata")
            .map_err(|e| platform_err("Metadata", e))?;
        Ok(NowPlaying::from_raw(
            metadata_string(&metadata, "xesam:title").as_deref(),
            metadata_string(&metadata, "xesam:artist").as_deref(),
            metadata_string(&metadata, "xesam:album").as_deref(),
            Some(&mpris_app_name(&bus_name)),
            is_playing(&proxy),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(value: Value<'_>) -> OwnedValue {
        OwnedValue::try_from(value).expect("owned value")
    }

    #[test]
    fn methods_match_mpris_player_interface() {
        assert_eq!(method_for(MediaCommand::PlayPause), "PlayPause");
        assert_eq!(method_for(MediaCommand::Next), "Next");
        assert_eq!(method_for(MediaCommand::Previous), "Previous");
        assert_eq!(method_for(MediaCommand::Stop), "Stop");
    }

    #[test]
    fn metadata_reads_strings_and_joins_artist_lists() {
        let mut metadata = HashMap::new();
        metadata.insert("xesam:title".to_owned(), owned(Value::from("Song")));
        metadata.insert(
            "xesam:artist".to_owned(),
            owned(Value::from(vec!["A".to_owned(), "B".to_owned()])),
        );
        assert_eq!(
            metadata_string(&metadata, "xesam:title").as_deref(),
            Some("Song")
        );
        assert_eq!(
            metadata_string(&metadata, "xesam:artist").as_deref(),
            Some("A, B")
        );
        assert_eq!(metadata_string(&metadata, "xesam:album"), None);
    }
}
