//! Network cleanup: flush the DNS resolver cache (and, on macOS, the ARP table).
//!
//! These need administrator rights, so each platform asks the OS for them with
//! its own native prompt rather than Conbind handling a password. The commands
//! are fixed strings with no user input in them. Renewing the DHCP lease is
//! deliberately not included: it drops the connection while it renegotiates,
//! which is a surprising side effect for a cleanup command.

use super::run_command;
use crate::error::AppError;
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkOutcome {
    Done,
    /// The user dismissed the OS password prompt.
    Cancelled,
}

/// Fails if the DNS flush fails. An empty ARP table is not an error.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const MACOS_SCRIPT: &str =
    "dscacheutil -flushcache && killall -HUP mDNSResponder && { arp -a -d || true; }";

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn macos_command() -> Command {
    let mut command = Command::new("/usr/bin/osascript");
    command.arg("-e").arg(format!(
        "do shell script \"{MACOS_SCRIPT}\" with administrator privileges"
    ));
    command
}

/// AppleScript reports a dismissed password dialog as error -128.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn classify_osascript(success: bool, stderr: &str) -> Result<NetworkOutcome, AppError> {
    if success {
        return Ok(NetworkOutcome::Done);
    }
    if stderr.contains("(-128)") || stderr.contains("User canceled") {
        return Ok(NetworkOutcome::Cancelled);
    }
    Err(AppError::Platform(format!(
        "network cleanup failed: {}",
        stderr.trim()
    )))
}

/// pkexec exits 126 when the authentication dialog was dismissed, 127 when
/// authorization failed.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn classify_pkexec(code: Option<i32>, stderr: &str) -> Result<NetworkOutcome, AppError> {
    match code {
        Some(0) => Ok(NetworkOutcome::Done),
        Some(126) => Ok(NetworkOutcome::Cancelled),
        _ => Err(AppError::Platform(format!(
            "network cleanup failed: {}",
            stderr.trim()
        ))),
    }
}

#[cfg(target_os = "macos")]
pub fn run() -> Result<NetworkOutcome, AppError> {
    let output = run_command(macos_command())?;
    classify_osascript(
        output.status.success(),
        &String::from_utf8_lossy(&output.stderr),
    )
}

#[cfg(target_os = "windows")]
pub fn run() -> Result<NetworkOutcome, AppError> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new("powershell");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Clear-DnsClientCache -ErrorAction Stop",
        ])
        .creation_flags(CREATE_NO_WINDOW);
    let output = run_command(command)?;
    if output.status.success() {
        return Ok(NetworkOutcome::Done);
    }
    Err(AppError::Platform(format!(
        "network cleanup failed (administrator rights may be required): {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

#[cfg(target_os = "linux")]
pub fn run() -> Result<NetworkOutcome, AppError> {
    // systemd-resolved lets an active session flush without a prompt; fall
    // back to a polkit prompt when it doesn't.
    let mut plain = Command::new("resolvectl");
    plain.arg("flush-caches");
    crate::platform::linux::sanitize_command(&mut plain);
    if run_command(plain)
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok(NetworkOutcome::Done);
    }
    let mut elevated = Command::new("pkexec");
    elevated.args(["resolvectl", "flush-caches"]);
    crate::platform::linux::sanitize_command(&mut elevated);
    let output = run_command(elevated)?;
    classify_pkexec(
        output.status.code(),
        &String::from_utf8_lossy(&output.stderr),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_command_asks_for_administrator_privileges_with_a_fixed_script() {
        let command = macos_command();
        assert_eq!(command.get_program(), "/usr/bin/osascript");
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "-e");
        assert!(args[1].starts_with("do shell script \""));
        assert!(args[1].ends_with("with administrator privileges"));
        assert!(args[1].contains("dscacheutil -flushcache"));
        assert!(args[1].contains("killall -HUP mDNSResponder"));
    }

    #[test]
    fn macos_script_has_no_unescaped_quotes() {
        assert!(!MACOS_SCRIPT.contains('"'));
        assert!(!MACOS_SCRIPT.contains('\\'));
    }

    #[test]
    fn osascript_success_cancel_and_failure_are_told_apart() {
        assert_eq!(classify_osascript(true, "").unwrap(), NetworkOutcome::Done);
        assert_eq!(
            classify_osascript(false, "execution error: User canceled. (-128)").unwrap(),
            NetworkOutcome::Cancelled
        );
        assert!(classify_osascript(false, "execution error: boom (1)").is_err());
    }

    #[test]
    fn pkexec_exit_codes_map_to_outcomes() {
        assert_eq!(classify_pkexec(Some(0), "").unwrap(), NetworkOutcome::Done);
        assert_eq!(
            classify_pkexec(Some(126), "").unwrap(),
            NetworkOutcome::Cancelled
        );
        assert!(classify_pkexec(Some(127), "not authorized").is_err());
        assert!(classify_pkexec(None, "killed").is_err());
    }
}
