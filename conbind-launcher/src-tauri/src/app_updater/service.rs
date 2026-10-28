use log::{info, warn};
use serde::Deserialize;
use std::time::Duration;
use tauri::{AppHandle, Manager};

use crate::app_updater::PendingUpdate;

/// Where the latest release is announced. The file is a small JSON document:
/// `{ "latest": "0.1.0", "windows": "<url>", "macos": "<url>", "linux": "<url>" }`
/// where each platform entry is a direct download link.
pub const UPDATE_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/montanari-luca/MintFlow-Releases/refs/heads/main/conbind.json";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct UpdateManifest {
    pub latest: String,
    #[serde(default)]
    pub windows: String,
    #[serde(default)]
    pub macos: String,
    #[serde(default)]
    pub linux: String,
}

/// Key of the manifest entry that holds this platform's download link.
pub fn platform_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

pub fn parse_manifest(body: &str) -> Result<UpdateManifest, String> {
    serde_json::from_str(body).map_err(|e| format!("invalid update manifest: {e}"))
}

fn parse_version(raw: &str) -> Result<semver::Version, String> {
    semver::Version::parse(raw.trim().trim_start_matches('v'))
        .map_err(|e| format!("invalid version {raw:?}: {e}"))
}

/// Pure decision: is `manifest.latest` newer than `current`? When it is, returns
/// the update together with this platform's download link (`None` while the
/// manifest has no link for the platform yet).
pub fn evaluate(
    manifest: &UpdateManifest,
    current: &str,
    platform: &str,
) -> Result<Option<PendingUpdate>, String> {
    let latest = parse_version(&manifest.latest)?;
    if latest <= parse_version(current)? {
        return Ok(None);
    }
    let link = match platform {
        "macos" => &manifest.macos,
        "windows" => &manifest.windows,
        _ => &manifest.linux,
    };
    let link = link.trim();
    Ok(Some(PendingUpdate {
        version: manifest.latest.trim().trim_start_matches('v').to_string(),
        download_url: (!link.is_empty()).then(|| link.to_string()),
    }))
}

async fn fetch_manifest() -> Result<UpdateManifest, String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(concat!("Conbind/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(UPDATE_MANIFEST_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "update manifest returned HTTP {}",
            response.status()
        ));
    }
    let body = response.text().await.map_err(|e| e.to_string())?;
    parse_manifest(&body)
}

/// Check the update manifest and, when a newer version is announced, remember it.
/// Emits events to the frontend during the flow:
///   - `conbind:app-update:checking` — before the network check
///   - `conbind:app-update:idle`     — when the app is up to date
///   - `conbind:app-update:ready`    — when a newer version is available; the
///     payload carries `version` and `downloadUrl`
///
/// Returns `Ok(Some(update))` when a newer version exists, `Ok(None)` when the
/// app is up to date, or `Err(message)` on failure.
pub async fn check_for_update(app: &AppHandle) -> Result<Option<PendingUpdate>, String> {
    crate::event_bridge::bridge_emit(app, "conbind:app-update:checking", ());

    let manifest = fetch_manifest().await?;
    let current = app.package_info().version.to_string();
    let update = evaluate(&manifest, &current, platform_key())?;

    if let Some(state) = app.try_state::<crate::app_updater::AppUpdaterState>() {
        if let Ok(mut pending) = state.pending.lock() {
            *pending = update.clone();
        }
    }

    match &update {
        None => {
            info!("app_updater: up to date ({current})");
            crate::event_bridge::bridge_emit(app, "conbind:app-update:idle", ());
        }
        Some(u) => {
            info!("app_updater: update available: {}", u.version);
            if u.download_url.is_none() {
                warn!(
                    "app_updater: manifest has no download link for {}",
                    platform_key()
                );
            }
            crate::event_bridge::bridge_emit(app, "conbind:app-update:ready", u);
        }
    }
    Ok(update)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(latest: &str) -> UpdateManifest {
        UpdateManifest {
            latest: latest.into(),
            windows: "https://example.test/conbind.msi".into(),
            macos: "https://example.test/conbind.dmg".into(),
            linux: String::new(),
        }
    }

    #[test]
    fn parses_the_published_shape() {
        let m = parse_manifest(r#"{ "latest": "0.1.0", "windows": "", "macos": "", "linux": "" }"#)
            .unwrap();
        assert_eq!(m.latest, "0.1.0");
        assert!(m.macos.is_empty());
    }

    #[test]
    fn missing_platform_entries_default_to_empty() {
        let m = parse_manifest(r#"{ "latest": "1.2.3" }"#).unwrap();
        assert!(m.windows.is_empty() && m.macos.is_empty() && m.linux.is_empty());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_manifest("not json").is_err());
        assert!(parse_manifest(r#"{ "macos": "x" }"#).is_err());
    }

    #[test]
    fn same_or_older_version_is_not_an_update() {
        assert_eq!(
            evaluate(&manifest("0.1.0"), "0.1.0", "macos").unwrap(),
            None
        );
        assert_eq!(
            evaluate(&manifest("0.0.9"), "0.1.0", "macos").unwrap(),
            None
        );
    }

    #[test]
    fn newer_version_returns_the_platform_link() {
        let u = evaluate(&manifest("0.2.0"), "0.1.0", "macos")
            .unwrap()
            .unwrap();
        assert_eq!(u.version, "0.2.0");
        assert_eq!(
            u.download_url.as_deref(),
            Some("https://example.test/conbind.dmg")
        );
        let w = evaluate(&manifest("0.2.0"), "0.1.0", "windows")
            .unwrap()
            .unwrap();
        assert_eq!(
            w.download_url.as_deref(),
            Some("https://example.test/conbind.msi")
        );
    }

    #[test]
    fn empty_platform_link_is_none() {
        let u = evaluate(&manifest("0.2.0"), "0.1.0", "linux")
            .unwrap()
            .unwrap();
        assert_eq!(u.download_url, None);
    }

    #[test]
    fn leading_v_and_whitespace_are_tolerated() {
        let u = evaluate(&manifest(" v0.3.0 "), "0.1.0", "macos")
            .unwrap()
            .unwrap();
        assert_eq!(u.version, "0.3.0");
    }

    #[test]
    fn prerelease_is_older_than_release() {
        assert!(evaluate(&manifest("0.1.0"), "0.1.0-5", "macos")
            .unwrap()
            .is_some());
    }

    #[test]
    fn unparsable_version_is_an_error() {
        assert!(evaluate(&manifest("soon"), "0.1.0", "macos").is_err());
    }

    #[test]
    fn manifest_url_is_the_release_feed() {
        assert!(UPDATE_MANIFEST_URL.ends_with("/MintFlow-Releases/refs/heads/main/conbind.json"));
    }
}
