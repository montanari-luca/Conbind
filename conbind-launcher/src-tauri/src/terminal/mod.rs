//! Session behind the launcher's `>` prefix: run a shell command, stream its
//! output, and carry the working directory and environment over to the next
//! command until the app quits.
//!
//! Every command runs in a fresh, non-interactive shell. A trailer appended to
//! the script writes the final directory and environment to a state file, which
//! the host reads back once the shell exits. So `cd` and `export` persist;
//! aliases and shell functions do not, because they live in the shell process.
//! The session is in memory only, so quitting the app resets it.

use crate::error::AppError;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

const STATE_FILE_ENV: &str = "CONBIND_TERMINAL_STATE";

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TerminalEvent {
    /// One line of output, without its line ending.
    Chunk { stream: String, data: String },
    /// The shell exited. `cwd` is the directory the next command starts in.
    Done {
        exit_code: Option<i32>,
        cwd: String,
        cancelled: bool,
    },
}

#[derive(Debug)]
struct SessionState {
    cwd: PathBuf,
    /// Empty until the first command finishes; then the full captured
    /// environment, which replaces the inherited one.
    env: HashMap<String, String>,
    running: Option<u32>,
    cancelled: bool,
}

pub struct TerminalSession {
    inner: Mutex<SessionState>,
}

impl Default for TerminalSession {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalSession {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(SessionState {
                cwd: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
                env: HashMap::new(),
                running: None,
                cancelled: false,
            }),
        }
    }

    pub fn cwd(&self) -> Result<String, AppError> {
        let state = self.inner.lock().map_err(|_| AppError::Lock)?;
        Ok(state.cwd.to_string_lossy().into_owned())
    }

    /// Kills the running command and everything it started. Returns whether
    /// there was one.
    pub fn cancel(&self) -> Result<bool, AppError> {
        let mut state = self.inner.lock().map_err(|_| AppError::Lock)?;
        let Some(pid) = state.running else {
            return Ok(false);
        };
        state.cancelled = true;
        kill_process_tree(pid);
        Ok(true)
    }
}

/// Runs `command` in the session, reporting output through `emit`. Resolves
/// once the shell has exited and the session has been updated.
pub async fn run_command(
    session: &TerminalSession,
    command: &str,
    emit: impl Fn(TerminalEvent) + Send + Sync + 'static,
) -> Result<(), AppError> {
    let command = command.trim();
    if command.is_empty() {
        return Err(AppError::Validation("command is empty".into()));
    }

    let (cwd, env) = {
        let mut state = session.inner.lock().map_err(|_| AppError::Lock)?;
        if state.running.is_some() {
            return Err(AppError::Validation("a command is already running".into()));
        }
        if !state.cwd.is_dir() {
            state.cwd = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        }
        (state.cwd.clone(), state.env.clone())
    };

    // The path only: the shell writes to it, and it is deleted when dropped.
    let state_path = tempfile::Builder::new()
        .prefix("conbind-terminal-")
        .tempfile()?
        .into_temp_path();

    let mut cmd = build_command(command, &cwd, &env);
    cmd.env(STATE_FILE_ENV, &state_path);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = tokio::process::Command::from(cmd)
        .kill_on_drop(true)
        .spawn()?;
    let pid = child.id();
    {
        let mut state = session.inner.lock().map_err(|_| AppError::Lock)?;
        state.running = pid;
        state.cancelled = false;
    }

    let emit = std::sync::Arc::new(emit);
    let out = tokio::spawn(pump(
        child.stdout.take().expect("stdout is piped"),
        "stdout",
        emit.clone(),
    ));
    let err = tokio::spawn(pump(
        child.stderr.take().expect("stderr is piped"),
        "stderr",
        emit.clone(),
    ));

    let status = child.wait().await;
    let _ = out.await;
    let _ = err.await;

    let captured = std::fs::read(&state_path)
        .ok()
        .and_then(|raw| parse_state(&raw));

    let (cwd, cancelled) = {
        let mut state = session.inner.lock().map_err(|_| AppError::Lock)?;
        state.running = None;
        let cancelled = state.cancelled;
        if !cancelled {
            if let Some((new_cwd, new_env)) = captured {
                if new_cwd.is_dir() {
                    state.cwd = new_cwd;
                }
                if !new_env.is_empty() {
                    state.env = new_env;
                }
            }
        }
        (state.cwd.to_string_lossy().into_owned(), cancelled)
    };

    emit(TerminalEvent::Done {
        exit_code: status.ok().and_then(|s| s.code()),
        cwd,
        cancelled,
    });
    Ok(())
}

async fn pump<R: AsyncRead + Unpin>(
    reader: R,
    stream: &'static str,
    emit: std::sync::Arc<impl Fn(TerminalEvent) + Send + Sync + 'static>,
) {
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                while matches!(buf.last(), Some(b'\n' | b'\r')) {
                    buf.pop();
                }
                emit(TerminalEvent::Chunk {
                    stream: stream.to_string(),
                    data: String::from_utf8_lossy(&buf).into_owned(),
                });
            }
        }
    }
}

#[cfg(unix)]
fn build_command(
    command: &str,
    cwd: &std::path::Path,
    env: &HashMap<String, String>,
) -> std::process::Command {
    use std::os::unix::process::CommandExt;

    let shell = pick_shell(std::env::var("SHELL").ok().as_deref());
    // A GUI launch has a minimal PATH, so the first command goes through a login
    // shell. Its environment is captured and reused afterwards.
    let login = env.is_empty();
    let script = format!(
        "{command}\n\
         __conbind_rc=$?\n\
         pwd -P > \"${STATE_FILE_ENV}\"\n\
         env -0 >> \"${STATE_FILE_ENV}\" 2>/dev/null || env >> \"${STATE_FILE_ENV}\"\n\
         exit $__conbind_rc\n"
    );
    let mut cmd = std::process::Command::new(shell);
    if login {
        cmd.arg("-l");
    }
    cmd.arg("-c").arg(script).current_dir(cwd);
    if !env.is_empty() {
        cmd.env_clear().envs(env);
    }
    // Its own process group, so cancelling reaches whatever the shell started.
    cmd.process_group(0);
    cmd
}

#[cfg(windows)]
fn build_command(
    command: &str,
    cwd: &std::path::Path,
    env: &HashMap<String, String>,
) -> std::process::Command {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let line = format!(
        "\"{command} & call set __CONBIND_RC=%^ERRORLEVEL% & cd > \"%{STATE_FILE_ENV}%\" \
         & set >> \"%{STATE_FILE_ENV}%\" & call exit %^__CONBIND_RC%\""
    );
    let mut cmd = std::process::Command::new("cmd.exe");
    cmd.args(["/D", "/S", "/C"])
        .raw_arg(line)
        .current_dir(cwd)
        .creation_flags(CREATE_NO_WINDOW);
    if !env.is_empty() {
        cmd.env_clear().envs(env);
    }
    cmd
}

/// The user's shell when its `-c` and `pwd -P` / `env` semantics match POSIX
/// sh, otherwise `/bin/sh` (fish and friends would misread the trailer).
#[cfg(unix)]
fn pick_shell(user_shell: Option<&str>) -> String {
    const POSIX: [&str; 5] = ["zsh", "bash", "sh", "dash", "ksh"];
    user_shell
        .filter(|path| {
            std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| POSIX.contains(&name))
        })
        .unwrap_or("/bin/sh")
        .to_string()
}

fn kill_process_tree(pid: u32) {
    #[cfg(unix)]
    {
        // Same guard as `shell::kill_pid`: a pid past i32::MAX would turn into
        // a negative, group-wide target.
        let Ok(signed) = i32::try_from(pid) else {
            log::warn!("[terminal] refusing to signal out-of-range pid {pid}");
            return;
        };
        if signed <= 0 {
            return;
        }
        // SAFETY: plain syscall; a negative pid targets the process group the
        // command was started in.
        unsafe {
            libc::kill(-signed, libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .status();
    }
}

/// Reads the state file: the working directory on the first line, then the
/// environment (NUL-separated, or one `NAME=value` per line when the platform's
/// `env` has no `-0`).
fn parse_state(raw: &[u8]) -> Option<(PathBuf, HashMap<String, String>)> {
    let text = String::from_utf8_lossy(raw);
    let (first, rest) = text.split_once('\n')?;
    let cwd = first.trim_end_matches('\r');
    if cwd.is_empty() {
        return None;
    }
    let entries: Vec<&str> = if rest.contains('\0') {
        rest.split('\0').collect()
    } else {
        rest.lines().collect()
    };
    let env = entries
        .into_iter()
        .filter_map(|entry| entry.split_once('='))
        .filter(|(key, _)| !key.is_empty() && *key != STATE_FILE_ENV)
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    Some((PathBuf::from(cwd), env))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn collect() -> (
        Arc<Mutex<Vec<TerminalEvent>>>,
        impl Fn(TerminalEvent) + Send + Sync + 'static,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        (events, move |e| sink.lock().unwrap().push(e))
    }

    fn output(events: &[TerminalEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|e| match e {
                TerminalEvent::Chunk { data, .. } => Some(data.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn parse_state_reads_cwd_and_nul_separated_env() {
        let (cwd, env) = parse_state(b"/tmp\nFOO=1\0BAR=a=b\0").unwrap();
        assert_eq!(cwd, PathBuf::from("/tmp"));
        assert_eq!(env.get("FOO").map(String::as_str), Some("1"));
        assert_eq!(env.get("BAR").map(String::as_str), Some("a=b"));
    }

    #[test]
    fn parse_state_falls_back_to_line_separated_env() {
        let (cwd, env) = parse_state(b"C:\\work\r\nFOO=1\r\n=C:=C:\\work\r\n").unwrap();
        assert_eq!(cwd, PathBuf::from("C:\\work"));
        assert_eq!(env.len(), 1);
        assert_eq!(env.get("FOO").map(String::as_str), Some("1"));
    }

    #[test]
    fn parse_state_rejects_empty_input() {
        assert!(parse_state(b"").is_none());
        assert!(parse_state(b"\nFOO=1\0").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn pick_shell_only_trusts_posix_shells() {
        assert_eq!(pick_shell(Some("/bin/zsh")), "/bin/zsh");
        assert_eq!(
            pick_shell(Some("/usr/local/bin/bash")),
            "/usr/local/bin/bash"
        );
        assert_eq!(pick_shell(Some("/usr/bin/fish")), "/bin/sh");
        assert_eq!(pick_shell(None), "/bin/sh");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn streams_output_and_exit_code() {
        let session = TerminalSession::new();
        let (events, emit) = collect();
        run_command(&session, "echo one; echo two >&2; exit 3", emit)
            .await
            .unwrap();
        let events = events.lock().unwrap();
        let mut lines = output(&events);
        lines.sort();
        assert_eq!(lines, ["one", "two"]);
        assert!(matches!(
            events.last(),
            Some(TerminalEvent::Done {
                exit_code: Some(3),
                cancelled: false,
                ..
            })
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cd_and_export_carry_over_to_the_next_command() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let session = TerminalSession::new();

        let (_, emit) = collect();
        run_command(
            &session,
            &format!(
                "cd '{}' && export CONBIND_TEST_VAR=kept",
                canonical.display()
            ),
            emit,
        )
        .await
        .unwrap();
        assert_eq!(session.cwd().unwrap(), canonical.to_string_lossy());

        let (events, emit) = collect();
        run_command(&session, "pwd; echo $CONBIND_TEST_VAR", emit)
            .await
            .unwrap();
        let lines = output(&events.lock().unwrap());
        assert_eq!(
            lines,
            [canonical.to_string_lossy().into_owned(), "kept".to_string()]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_empty_command() {
        let session = TerminalSession::new();
        let (_, emit) = collect();
        assert!(run_command(&session, "   ", emit).await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancel_stops_a_running_command() {
        let session = Arc::new(TerminalSession::new());
        let runner = session.clone();
        let (events, emit) = collect();
        let task = tokio::spawn(async move { run_command(&runner, "sleep 30", emit).await });

        for _ in 0..100 {
            if session.cancel().unwrap() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .expect("cancelled command should exit promptly")
            .unwrap()
            .unwrap();
        assert!(matches!(
            events.lock().unwrap().last(),
            Some(TerminalEvent::Done {
                cancelled: true,
                ..
            })
        ));
    }
}
