//! Emptying the OS trash. This is the one permanent action in the cleanup
//! suite: everything else moves items *to* the trash so it can be undone, and
//! disk space only comes back once the trash is emptied.
//!
//! The `trash` crate can't empty the macOS trash, so each platform uses its
//! native tool. The frontend confirms before calling.

use super::run_command;
use crate::error::AppError;
use std::process::Command;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const MACOS_SCRIPT: &str = "tell application \"Finder\" to empty trash";

#[cfg(target_os = "macos")]
fn command() -> Command {
    let mut command = Command::new("/usr/bin/osascript");
    command.arg("-e").arg(MACOS_SCRIPT);
    command
}

#[cfg(target_os = "windows")]
fn command() -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new("powershell");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Clear-RecycleBin -Force -ErrorAction Stop",
        ])
        .creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(target_os = "linux")]
fn command() -> Command {
    let mut command = Command::new("gio");
    command.args(["trash", "--empty"]);
    crate::platform::linux::sanitize_command(&mut command);
    command
}

pub fn empty() -> Result<(), AppError> {
    let output = run_command(command())?;
    if output.status.success() {
        return Ok(());
    }
    Err(AppError::Platform(format!(
        "could not empty the trash: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_targets_the_platform_trash_tool() {
        let program = command().get_program().to_string_lossy().into_owned();
        if cfg!(target_os = "macos") {
            assert_eq!(program, "/usr/bin/osascript");
        } else if cfg!(target_os = "windows") {
            assert_eq!(program, "powershell");
        } else {
            assert_eq!(program, "gio");
        }
    }
}
