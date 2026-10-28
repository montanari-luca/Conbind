use serde::Serialize;
use std::collections::HashMap;
use tauri::{AppHandle, State};

/// The custom URL scheme this running instance is registered for with the
/// OS. Mirrors `plugins.deep-link.desktop.schemes` in the active tauri
/// config — `tauri.conf.json` (`conbind`) for production, `tauri.dev.conf.json`
/// (`conbind-dev`) for the local dev flavor. Production and dev must never
/// register the same scheme, or the OS can deliver an OAuth or extension
/// deep link to the wrong flavor.
pub fn deep_link_scheme(app: &AppHandle) -> &'static str {
    if app.config().identifier == "org.conbind.dev" {
        "conbind-dev"
    } else {
        "conbind"
    }
}

/// Typed payload emitted as `conbind:deeplink:extension` when a deep link
/// targets an extension command: `{scheme}://extensions/{extensionId}/{commandId}?args`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDeeplinkPayload {
    pub extension_id: String,
    pub command_id: String,
    pub args: HashMap<String, String>,
}

/// Attempts to parse a `{expected_scheme}://extensions/{extensionId}/{commandId}?args` URL.
///
/// Returns `None` (and logs a warning) if the URL is not an extension deep link,
/// has missing/empty segments, or contains unsafe characters in the extension ID.
pub fn parse_extension_deeplink(
    raw: &str,
    expected_scheme: &str,
) -> Option<ExtensionDeeplinkPayload> {
    let parsed = url::Url::parse(raw).ok()?;

    // Must be this flavor's registered scheme
    if parsed.scheme() != expected_scheme {
        return None;
    }

    // Host is "extensions" (URL parses conbind://extensions/... as host = "extensions")
    if parsed.host_str() != Some("extensions") {
        return None;
    }

    // Path segments after host: /{extensionId}/{commandId}
    let segments: Vec<&str> = parsed.path_segments()?.filter(|s| !s.is_empty()).collect();

    if segments.len() < 2 {
        log::warn!("Deep link missing extensionId or commandId: {}", raw);
        return None;
    }

    let extension_id = segments[0];
    let command_id = segments[1];

    // Validate extensionId format: alphanumeric, dots, hyphens, underscores only.
    // Prevents path traversal (e.g. "../etc/passwd") and other injection.
    if !extension_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        log::warn!(
            "Deep link extensionId contains invalid characters: {}",
            extension_id
        );
        return None;
    }

    // Validate commandId is non-empty and safe
    if !command_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        log::warn!(
            "Deep link commandId contains invalid characters: {}",
            command_id
        );
        return None;
    }

    // Collect query params with URL decoding (handled by url::Url)
    let args: HashMap<String, String> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    Some(ExtensionDeeplinkPayload {
        extension_id: extension_id.to_string(),
        command_id: command_id.to_string(),
        args,
    })
}

/// Routing decision for an incoming deep link. Pure classification lives in
/// [`classify_deeplink`]; [`dispatch_url`] performs the side-effecting emit.
#[derive(Debug, Clone, PartialEq)]
pub enum DeeplinkRoute {
    /// Targets an extension command → frontend event `conbind:deeplink:extension`.
    Extension(ExtensionDeeplinkPayload),
    /// Any other in-scheme URL (auth / OAuth) → frontend event `conbind:deep-link`.
    Raw(String),
}

/// Classify an incoming URL into its route without touching the app. Returns
/// `None` for URLs that are not this flavor's scheme, or malformed extension
/// links (which must be dropped, never re-routed as raw).
pub fn classify_deeplink(raw: &str, scheme: &str) -> Option<DeeplinkRoute> {
    if !raw.starts_with(&format!("{scheme}://")) {
        return None;
    }
    if raw.starts_with(&format!("{scheme}://extensions/")) {
        // Malformed extension links must be dropped, not re-routed as raw.
        return parse_extension_deeplink(raw, scheme).map(DeeplinkRoute::Extension);
    }
    Some(DeeplinkRoute::Raw(raw.to_string()))
}

/// Launch-time deep links captured before the frontend's listeners exist. The
/// frontend drains these via `flush_pending_deeplinks` once ready, so the emit
/// can't race listener registration (same reason `restore_workers` is pulled).
#[derive(Default)]
pub struct PendingDeeplinks(std::sync::Mutex<Vec<String>>);

impl PendingDeeplinks {
    /// Buffer a launch URL captured during setup.
    pub fn push(&self, url: String) {
        if let Ok(mut guard) = self.0.lock() {
            guard.push(url);
        }
    }

    /// Drain every buffered URL, leaving the buffer empty.
    pub fn take(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|mut guard| std::mem::take(&mut *guard))
            .unwrap_or_default()
    }
}

/// Route a single incoming deep-link URL to the correct frontend event.
/// Shared by the warm path (`on_open_url`) and the cold-start flush.
pub fn dispatch_url(app: &AppHandle, scheme: &str, raw: &str) {
    match classify_deeplink(raw, scheme) {
        Some(DeeplinkRoute::Extension(payload)) => {
            log::info!(
                "[Deeplink] Extension trigger: {}/{}",
                payload.extension_id,
                payload.command_id
            );
            crate::event_bridge::bridge_emit(app, "conbind:deeplink:extension", payload);
        }
        Some(DeeplinkRoute::Raw(url)) => {
            crate::event_bridge::bridge_emit(app, "conbind:deep-link", url);
        }
        None => log::warn!("[Deeplink] Ignoring unroutable deep link: {raw}"),
    }
}

/// Drain deep links that launched the app and dispatch them now that the
/// frontend's listeners are registered. Frontend-invoked from appInitializer
/// so the emit can't race listener setup.
#[tauri::command]
pub fn flush_pending_deeplinks(app: AppHandle, pending: State<'_, PendingDeeplinks>) {
    let scheme = deep_link_scheme(&app);
    for url in pending.take() {
        dispatch_url(&app, scheme, &url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_deeplink_with_args() {
        let result = parse_extension_deeplink(
            "conbind://extensions/com.example.weather/check?city=Berlin&units=metric",
            "conbind",
        );
        let payload = result.expect("should parse successfully");
        assert_eq!(payload.extension_id, "com.example.weather");
        assert_eq!(payload.command_id, "check");
        assert_eq!(payload.args.get("city"), Some(&"Berlin".to_string()));
        assert_eq!(payload.args.get("units"), Some(&"metric".to_string()));
    }

    #[test]
    fn parses_valid_deeplink_without_args() {
        let result =
            parse_extension_deeplink("conbind://extensions/com.example.calc/run", "conbind");
        let payload = result.expect("should parse successfully");
        assert_eq!(payload.extension_id, "com.example.calc");
        assert_eq!(payload.command_id, "run");
        assert!(payload.args.is_empty());
    }

    #[test]
    fn rejects_missing_command_id() {
        assert!(
            parse_extension_deeplink("conbind://extensions/com.example.calc", "conbind").is_none()
        );
    }

    #[test]
    fn rejects_empty_extension_id() {
        assert!(parse_extension_deeplink("conbind://extensions//run", "conbind").is_none());
    }

    #[test]
    fn rejects_path_traversal_in_extension_id() {
        // Dots in isolation are fine (e.g. "com.example"), but characters like
        // slashes or percent-encoded slashes would be caught by the URL parser
        // or the character allowlist. Test that special chars are rejected:
        assert!(
            parse_extension_deeplink("conbind://extensions/ext%2F..%2Fetc/run", "conbind")
                .is_none()
        );
        assert!(parse_extension_deeplink("conbind://extensions/ext%00id/run", "conbind").is_none());
    }

    #[test]
    fn rejects_non_extension_deeplinks() {
        assert!(parse_extension_deeplink("conbind://auth/callback?code=abc", "conbind").is_none());
    }

    #[test]
    fn handles_url_encoded_args() {
        let result = parse_extension_deeplink(
            "conbind://extensions/ext/cmd?q=hello%20world&n=42",
            "conbind",
        );
        let payload = result.expect("should parse successfully");
        assert_eq!(payload.args.get("q"), Some(&"hello world".to_string()));
        assert_eq!(payload.args.get("n"), Some(&"42".to_string()));
    }

    #[test]
    fn rejects_non_conbind_scheme() {
        assert!(
            parse_extension_deeplink("https://extensions/com.example/cmd", "conbind").is_none()
        );
    }

    #[test]
    fn accepts_hyphenated_and_underscored_ids() {
        let result =
            parse_extension_deeplink("conbind://extensions/my-ext_v2/do-thing_now", "conbind");
        let payload = result.expect("should parse successfully");
        assert_eq!(payload.extension_id, "my-ext_v2");
        assert_eq!(payload.command_id, "do-thing_now");
    }

    #[test]
    fn serializes_payload_as_camel_case() {
        let payload = ExtensionDeeplinkPayload {
            extension_id: "test".to_string(),
            command_id: "cmd".to_string(),
            args: HashMap::new(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("extensionId"));
        assert!(json.contains("commandId"));
        assert!(!json.contains("extension_id"));
    }

    #[test]
    fn parses_store_browse_deeplink_with_slug() {
        let result = parse_extension_deeplink(
            "conbind://extensions/store/browse?slug=pomodoro-timer",
            "conbind",
        );
        let payload = result.expect("should parse store browse deeplink");
        assert_eq!(payload.extension_id, "store");
        assert_eq!(payload.command_id, "browse");
        assert_eq!(
            payload.args.get("slug"),
            Some(&"pomodoro-timer".to_string())
        );
    }

    #[test]
    fn accepts_dev_scheme_when_expected() {
        let result = parse_extension_deeplink(
            "conbind-dev://extensions/com.example.calc/run",
            "conbind-dev",
        );
        let payload = result.expect("should parse the dev-flavor scheme");
        assert_eq!(payload.extension_id, "com.example.calc");
        assert_eq!(payload.command_id, "run");
    }

    #[test]
    fn rejects_prod_scheme_when_dev_expected() {
        // A link for the installed production app must not be accepted by a
        // dev-flavor instance, and vice versa — the two must stay isolated.
        assert!(parse_extension_deeplink(
            "conbind://extensions/com.example.calc/run",
            "conbind-dev"
        )
        .is_none());
    }

    #[test]
    fn classifies_extension_link() {
        match classify_deeplink("conbind://extensions/com.example/run?x=1", "conbind") {
            Some(DeeplinkRoute::Extension(p)) => {
                assert_eq!(p.extension_id, "com.example");
                assert_eq!(p.command_id, "run");
            }
            other => panic!("expected Extension route, got {other:?}"),
        }
    }

    #[test]
    fn classifies_raw_auth_link() {
        assert_eq!(
            classify_deeplink("conbind://auth/callback?code=abc", "conbind"),
            Some(DeeplinkRoute::Raw(
                "conbind://auth/callback?code=abc".to_string()
            )),
        );
    }

    #[test]
    fn drops_malformed_extension_link() {
        // Starts with the extensions prefix but has no commandId: must be
        // dropped, never leaked to the raw auth listeners.
        assert_eq!(
            classify_deeplink("conbind://extensions/onlyext", "conbind"),
            None
        );
    }

    #[test]
    fn ignores_foreign_scheme() {
        assert_eq!(classify_deeplink("https://evil.example/x", "conbind"), None);
        assert_eq!(classify_deeplink("conbind://auth/x", "conbind-dev"), None);
    }

    #[test]
    fn pending_deeplinks_push_then_take_drains() {
        let pending = PendingDeeplinks::default();
        pending.push("conbind://extensions/a/b".to_string());
        pending.push("conbind://auth/cb".to_string());
        assert_eq!(
            pending.take(),
            vec![
                "conbind://extensions/a/b".to_string(),
                "conbind://auth/cb".to_string()
            ]
        );
        assert!(pending.take().is_empty(), "second take must be empty");
    }
}
