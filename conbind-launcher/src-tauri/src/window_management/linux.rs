//! Window management on Linux (X11), through EWMH using x11rb.
//!
//! Wayland has no API for moving other apps' windows, so every entry point refuses to run there.

use crate::error::AppError;
use crate::window_management::backend::WindowId;

#[cfg(target_os = "linux")]
use crate::window_management::backend::{
    EventSink, MonitorInfo, WatchGuard, WindowBackend, WindowEvent, WindowInfo,
};
#[cfg(target_os = "linux")]
use crate::window_management::types::{AppWindowInfo, WindowBounds};

/// Refuses to run in a Wayland session.
#[cfg(target_os = "linux")]
pub fn ensure_x11() -> Result<(), AppError> {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return Err(wayland_error());
    }
    Ok(())
}

/// Validates a stored target window id captured when the launcher opened.
#[cfg(target_os = "linux")]
pub fn check_x11(prev_wid: u64) -> Result<(), AppError> {
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    check_x11_inner(prev_wid, is_wayland)
}

#[cfg(any(target_os = "linux", test))]
fn wayland_error() -> AppError {
    AppError::Platform(
        "Window management is not supported on Wayland. \
         Start Conbind in an X11/XOrg session."
            .to_string(),
    )
}

#[cfg(any(target_os = "linux", test))]
fn check_x11_inner(prev_wid: u64, is_wayland: bool) -> Result<(), AppError> {
    if is_wayland {
        return Err(wayland_error());
    }
    if prev_wid == 0 {
        return Err(AppError::NotFound(
            "No previous window captured. Open Conbind via its launcher shortcut first."
                .to_string(),
        ));
    }
    Ok(())
}

pub fn format_window_id(xid: u32) -> WindowId {
    WindowId::new(format!("linux:{xid}"))
}

/// Accepts `linux:<xid>` or a bare `<xid>`.
pub fn parse_window_id(id: &str) -> Result<u32, AppError> {
    let raw = id.strip_prefix("linux:").unwrap_or(id);
    match raw.parse::<u32>() {
        Ok(0) => Err(AppError::NotFound("Null window id".to_string())),
        Ok(xid) => Ok(xid),
        Err(_) => Err(AppError::Validation(format!(
            "Invalid Linux window id: {id}"
        ))),
    }
}

#[cfg(target_os = "linux")]
mod x {
    use super::*;
    use crate::window_management::layout::{work_area_from_struts, Strut};
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::xproto::{
        Atom, AtomEnum, ClientMessageEvent, ConnectionExt as _, EventMask, Window,
    };
    use x11rb::rust_connection::RustConnection;

    pub fn plat(e: impl std::fmt::Display) -> AppError {
        AppError::Platform(format!("X11: {e}"))
    }

    /// `_NET_MOVERESIZE_WINDOW` flags: static gravity, x, y, width and height all valid,
    /// source indication "pager/user action" (2).
    const MOVERESIZE_FLAGS: u32 = 10 | (1 << 8) | (1 << 9) | (1 << 10) | (1 << 11) | (2 << 12);
    const ICONIC_STATE: u32 = 3;

    struct Listed {
        win: Window,
        pid: u32,
        title: String,
        class: Option<String>,
        hidden: bool,
    }

    /// Path of the executable behind `pid`, which identifies the app across sessions.
    fn executable_of(pid: u32) -> Option<String> {
        if pid == 0 {
            return None;
        }
        std::fs::read_link(format!("/proc/{pid}/exe"))
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    }

    pub struct X {
        pub conn: RustConnection,
        pub root: Window,
    }

    impl X {
        pub fn connect() -> Result<Self, AppError> {
            let (conn, screen) = RustConnection::connect(None).map_err(plat)?;
            let root = conn.setup().roots[screen].root;
            Ok(Self { conn, root })
        }

        pub fn atom(&self, name: &str) -> Result<Atom, AppError> {
            Ok(self
                .conn
                .intern_atom(false, name.as_bytes())
                .map_err(plat)?
                .reply()
                .map_err(plat)?
                .atom)
        }

        /// 32-bit property values; empty when the property is missing or has another format.
        pub fn values32(
            &self,
            win: Window,
            prop: &str,
            ty: AtomEnum,
            max: u32,
        ) -> Result<Vec<u32>, AppError> {
            let atom = self.atom(prop)?;
            let reply = self
                .conn
                .get_property(false, win, atom, ty, 0, max)
                .map_err(plat)?
                .reply()
                .map_err(plat)?;
            Ok(reply.value32().map(|v| v.collect()).unwrap_or_default())
        }

        fn send_to_root(&self, win: Window, message: &str, data: [u32; 5]) -> Result<(), AppError> {
            let event = ClientMessageEvent::new(32, win, self.atom(message)?, data);
            self.conn
                .send_event(
                    false,
                    self.root,
                    EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                    event,
                )
                .map_err(plat)?;
            self.conn.flush().map_err(plat)
        }

        /// (left, right, top, bottom) of the window manager's decorations.
        fn frame_extents(&self, win: Window) -> (f64, f64, f64, f64) {
            match self.values32(win, "_NET_FRAME_EXTENTS", AtomEnum::CARDINAL, 4) {
                Ok(v) if v.len() == 4 => (v[0] as f64, v[1] as f64, v[2] as f64, v[3] as f64),
                _ => (0.0, 0.0, 0.0, 0.0),
            }
        }

        /// The window as the user sees it: client area plus decorations, in root coordinates.
        pub fn visible_bounds(&self, win: Window) -> Result<WindowBounds, AppError> {
            let geometry = self
                .conn
                .get_geometry(win)
                .map_err(plat)?
                .reply()
                .map_err(plat)?;
            let origin = self
                .conn
                .translate_coordinates(win, self.root, 0, 0)
                .map_err(plat)?
                .reply()
                .map_err(plat)?;
            let (left, right, top, bottom) = self.frame_extents(win);
            Ok(WindowBounds {
                x: origin.dst_x as f64 - left,
                y: origin.dst_y as f64 - top,
                width: geometry.width as f64 + left + right,
                height: geometry.height as f64 + top + bottom,
            })
        }

        fn change_state(
            &self,
            win: Window,
            action: u32,
            first: &str,
            second: Option<&str>,
        ) -> Result<(), AppError> {
            let first = self.atom(first)?;
            let second = match second {
                Some(name) => self.atom(name)?,
                None => 0,
            };
            self.send_to_root(win, "_NET_WM_STATE", [action, first, second, 1, 0])
        }

        pub fn set_visible_bounds(
            &self,
            win: Window,
            bounds: &WindowBounds,
        ) -> Result<(), AppError> {
            // A maximized window ignores move/resize requests.
            self.change_state(
                win,
                0,
                "_NET_WM_STATE_MAXIMIZED_VERT",
                Some("_NET_WM_STATE_MAXIMIZED_HORZ"),
            )?;
            let (left, right, top, bottom) = self.frame_extents(win);
            let client_x = bounds.x + left;
            let client_y = bounds.y + top;
            let client_w = (bounds.width - left - right).max(1.0);
            let client_h = (bounds.height - top - bottom).max(1.0);
            self.send_to_root(
                win,
                "_NET_MOVERESIZE_WINDOW",
                [
                    MOVERESIZE_FLAGS,
                    client_x.round() as i32 as u32,
                    client_y.round() as i32 as u32,
                    client_w.round() as u32,
                    client_h.round() as u32,
                ],
            )
        }

        pub fn set_fullscreen(&self, win: Window, enable: bool) -> Result<(), AppError> {
            self.change_state(win, enable as u32, "_NET_WM_STATE_FULLSCREEN", None)
        }

        pub fn minimize(&self, win: Window) -> Result<(), AppError> {
            self.send_to_root(win, "WM_CHANGE_STATE", [ICONIC_STATE, 0, 0, 0, 0])
        }

        pub fn activate(&self, win: Window) -> Result<(), AppError> {
            self.send_to_root(win, "_NET_ACTIVE_WINDOW", [2, 0, 0, 0, 0])
        }

        pub fn close(&self, win: Window) -> Result<(), AppError> {
            self.send_to_root(win, "_NET_CLOSE_WINDOW", [0, 2, 0, 0, 0])
        }

        pub fn active_window(&self) -> u32 {
            self.values32(self.root, "_NET_ACTIVE_WINDOW", AtomEnum::WINDOW, 1)
                .ok()
                .and_then(|v| v.first().copied())
                .unwrap_or(0)
        }

        fn struts(&self) -> Vec<Strut> {
            let clients = self
                .values32(self.root, "_NET_CLIENT_LIST", AtomEnum::WINDOW, 4096)
                .unwrap_or_default();
            clients
                .into_iter()
                .filter_map(|win| {
                    if let Ok(v) =
                        self.values32(win, "_NET_WM_STRUT_PARTIAL", AtomEnum::CARDINAL, 12)
                    {
                        if v.len() == 12 {
                            let f = |i: usize| v[i] as f64;
                            return Some(Strut {
                                left: f(0),
                                right: f(1),
                                top: f(2),
                                bottom: f(3),
                                left_span: (f(4), f(5)),
                                right_span: (f(6), f(7)),
                                top_span: (f(8), f(9)),
                                bottom_span: (f(10), f(11)),
                            });
                        }
                    }
                    match self.values32(win, "_NET_WM_STRUT", AtomEnum::CARDINAL, 4) {
                        Ok(v) if v.len() == 4 => Some(Strut::full_edges(
                            v[0] as f64,
                            v[1] as f64,
                            v[2] as f64,
                            v[3] as f64,
                        )),
                        _ => None,
                    }
                })
                .collect()
        }

        pub fn monitors(&self) -> Result<Vec<MonitorInfo>, AppError> {
            let root_geometry = self
                .conn
                .get_geometry(self.root)
                .map_err(plat)?
                .reply()
                .map_err(plat)?;
            let screen = WindowBounds {
                x: 0.0,
                y: 0.0,
                width: root_geometry.width as f64,
                height: root_geometry.height as f64,
            };
            let mut frames: Vec<WindowBounds> = self
                .conn
                .randr_get_monitors(self.root, true)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|reply| {
                    reply
                        .monitors
                        .iter()
                        .map(|m| WindowBounds {
                            x: m.x as f64,
                            y: m.y as f64,
                            width: m.width as f64,
                            height: m.height as f64,
                        })
                        .collect()
                })
                .unwrap_or_default();
            if frames.is_empty() {
                frames.push(screen.clone());
            }
            let struts = self.struts();
            Ok(frames
                .into_iter()
                .map(|frame| MonitorInfo {
                    work_area: work_area_from_struts(&frame, &screen, &struts),
                    frame,
                })
                .collect())
        }

        fn text(&self, win: Window, prop: &str, ty: AtomEnum) -> Option<Vec<u8>> {
            let atom = self.atom(prop).ok()?;
            let reply = self
                .conn
                .get_property(false, win, atom, ty, 0, 1024)
                .ok()?
                .reply()
                .ok()?;
            (!reply.value.is_empty()).then_some(reply.value)
        }

        fn title(&self, win: Window) -> String {
            let utf8 = self.atom("UTF8_STRING").ok();
            let net_name = utf8.and_then(|ty| {
                let atom = self.atom("_NET_WM_NAME").ok()?;
                let reply = self
                    .conn
                    .get_property(false, win, atom, ty, 0, 1024)
                    .ok()?
                    .reply()
                    .ok()?;
                (!reply.value.is_empty()).then_some(reply.value)
            });
            let raw = net_name.or_else(|| self.text(win, "WM_NAME", AtomEnum::STRING));
            raw.map(|v| String::from_utf8_lossy(&v).trim().to_string())
                .unwrap_or_default()
        }

        /// The WM_CLASS class name (second field), e.g. "Firefox".
        fn class_name(&self, win: Window) -> Option<String> {
            let raw = self.text(win, "WM_CLASS", AtomEnum::STRING)?;
            let mut parts = raw.split(|b| *b == 0).filter(|p| !p.is_empty());
            let instance = parts.next();
            let class = parts.next().or(instance)?;
            Some(String::from_utf8_lossy(class).to_string())
        }

        fn atoms_of(&self, win: Window, prop: &str) -> Vec<Atom> {
            self.values32(win, prop, AtomEnum::ATOM, 64)
                .unwrap_or_default()
        }

        /// Normal application windows, topmost first, other than `our_pid`'s.
        fn listed(&self, our_pid: u32) -> Result<Vec<Listed>, AppError> {
            let mut stacking = self.values32(
                self.root,
                "_NET_CLIENT_LIST_STACKING",
                AtomEnum::WINDOW,
                4096,
            )?;
            // Bottom-to-top in EWMH; callers want the topmost first.
            stacking.reverse();

            let hidden = self.atom("_NET_WM_STATE_HIDDEN")?;
            let skip_taskbar = self.atom("_NET_WM_STATE_SKIP_TASKBAR")?;
            let type_normal = self.atom("_NET_WM_WINDOW_TYPE_NORMAL")?;
            let type_dialog = self.atom("_NET_WM_WINDOW_TYPE_DIALOG")?;

            let mut results = Vec::new();
            for win in stacking {
                let types = self.atoms_of(win, "_NET_WM_WINDOW_TYPE");
                if !types.is_empty()
                    && !types.contains(&type_normal)
                    && !types.contains(&type_dialog)
                {
                    continue;
                }
                let state = self.atoms_of(win, "_NET_WM_STATE");
                if state.contains(&skip_taskbar) {
                    continue;
                }
                let title = self.title(win);
                if title.is_empty() {
                    continue;
                }
                let pid = self
                    .values32(win, "_NET_WM_PID", AtomEnum::CARDINAL, 1)
                    .ok()
                    .and_then(|v| v.first().copied())
                    .unwrap_or(0);
                if pid != 0 && pid == our_pid {
                    continue;
                }
                results.push(Listed {
                    win,
                    pid,
                    title,
                    class: self.class_name(win),
                    hidden: state.contains(&hidden),
                });
            }
            Ok(results)
        }

        pub fn list_windows(&self, our_pid: u32) -> Result<Vec<AppWindowInfo>, AppError> {
            let active = self.active_window();
            Ok(self
                .listed(our_pid)?
                .into_iter()
                .map(|w| AppWindowInfo {
                    id: format_window_id(w.win).0,
                    pid: w.pid as i32,
                    app_name: w.class.unwrap_or_else(|| w.title.clone()),
                    app_bundle_id: None,
                    title: w.title,
                    is_minimized: w.hidden,
                    is_focused: w.win == active,
                    app_icon: None,
                })
                .collect())
        }

        /// Visible windows with their app identity (executable path, else WM_CLASS) and bounds.
        pub fn windows(&self, our_pid: u32) -> Result<Vec<WindowInfo>, AppError> {
            Ok(self
                .listed(our_pid)?
                .into_iter()
                .filter(|w| !w.hidden)
                .filter_map(|w| {
                    let bounds = self.visible_bounds(w.win).ok()?;
                    let app_id = executable_of(w.pid)
                        .or_else(|| w.class.clone())
                        .unwrap_or_else(|| w.title.clone());
                    Some(WindowInfo {
                        id: format_window_id(w.win),
                        app_id,
                        app_name: w.class.unwrap_or_else(|| w.title.clone()),
                        title: w.title,
                        bounds,
                    })
                })
                .collect())
        }
    }
}

/// Window events come from one shared connection and thread: each watched window selects
/// `StructureNotify` on it, and the thread routes events to that window's sinks.
#[cfg(target_os = "linux")]
mod watch {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::thread;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ChangeWindowAttributesAux, ConnectionExt as _, EventMask};
    use x11rb::protocol::Event;
    use x11rb::rust_connection::RustConnection;

    struct Watcher {
        conn: RustConnection,
        sinks: Mutex<HashMap<u32, Vec<(u64, EventSink)>>>,
    }

    impl Watcher {
        fn run(&self) {
            while let Ok(event) = self.conn.wait_for_event() {
                match event {
                    Event::ConfigureNotify(e) => self.dispatch(e.window, WindowEvent::Changed),
                    Event::DestroyNotify(e) => self.dispatch(e.window, WindowEvent::Gone),
                    // Minimizing unmaps the client window.
                    Event::UnmapNotify(e) => self.dispatch(e.window, WindowEvent::Gone),
                    _ => {}
                }
            }
        }

        fn dispatch(&self, win: u32, event: WindowEvent) {
            let sinks: Vec<EventSink> = self
                .sinks
                .lock()
                .ok()
                .and_then(|map| {
                    map.get(&win)
                        .map(|list| list.iter().map(|(_, sink)| sink.clone()).collect())
                })
                .unwrap_or_default();
            for sink in sinks {
                sink(event);
            }
        }

        fn select_events(&self, win: u32, mask: EventMask) -> Result<(), AppError> {
            self.conn
                .change_window_attributes(win, &ChangeWindowAttributesAux::new().event_mask(mask))
                .map_err(x::plat)?
                .check()
                .map_err(x::plat)
        }
    }

    fn watcher() -> Result<Arc<Watcher>, AppError> {
        static WATCHER: OnceLock<Result<Arc<Watcher>, String>> = OnceLock::new();
        WATCHER
            .get_or_init(|| {
                let (conn, _) = RustConnection::connect(None).map_err(|e| e.to_string())?;
                let watcher = Arc::new(Watcher {
                    conn,
                    sinks: Mutex::new(HashMap::new()),
                });
                let worker = watcher.clone();
                thread::Builder::new()
                    .name("window-watch".to_string())
                    .spawn(move || worker.run())
                    .map_err(|e| e.to_string())?;
                Ok(watcher)
            })
            .clone()
            .map_err(AppError::Platform)
    }

    struct XWatch {
        watcher: Arc<Watcher>,
        win: u32,
        token: u64,
    }

    impl Drop for XWatch {
        fn drop(&mut self) {
            let Ok(mut map) = self.watcher.sinks.lock() else {
                return;
            };
            if let Some(list) = map.get_mut(&self.win) {
                list.retain(|(token, _)| *token != self.token);
                if list.is_empty() {
                    map.remove(&self.win);
                    let _ = self.watcher.select_events(self.win, EventMask::NO_EVENT);
                }
            }
        }
    }

    pub fn watch_window(win: u32, sink: EventSink) -> Result<WatchGuard, AppError> {
        static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
        let watcher = watcher()?;
        watcher.select_events(win, EventMask::STRUCTURE_NOTIFY)?;
        let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
        watcher
            .sinks
            .lock()
            .map_err(|_| AppError::Lock)?
            .entry(win)
            .or_default()
            .push((token, sink));
        Ok(WatchGuard::new(XWatch {
            watcher,
            win,
            token,
        }))
    }
}

/// Id of the window that was active when the launcher was opened, or 0 if unknown.
#[cfg(target_os = "linux")]
pub fn capture_active_window_id() -> u64 {
    x::X::connect()
        .map(|x| x.active_window() as u64)
        .unwrap_or(0)
}

#[cfg(target_os = "linux")]
fn with_window<T>(
    id: &WindowId,
    f: impl FnOnce(&x::X, u32) -> Result<T, AppError>,
) -> Result<T, AppError> {
    ensure_x11()?;
    let win = parse_window_id(id.as_str())?;
    f(&x::X::connect()?, win)
}

#[cfg(target_os = "linux")]
pub struct LinuxBackend;

#[cfg(target_os = "linux")]
impl WindowBackend for LinuxBackend {
    fn bounds(&self, id: &WindowId) -> Result<WindowBounds, AppError> {
        with_window(id, |x, win| x.visible_bounds(win))
    }

    fn set_bounds(&self, id: &WindowId, bounds: &WindowBounds) -> Result<(), AppError> {
        with_window(id, |x, win| x.set_visible_bounds(win, bounds))
    }

    fn set_fullscreen(&self, id: &WindowId, enable: bool) -> Result<(), AppError> {
        with_window(id, |x, win| x.set_fullscreen(win, enable))
    }

    fn minimize(&self, id: &WindowId) -> Result<(), AppError> {
        with_window(id, |x, win| x.minimize(win))
    }

    fn monitors(&self) -> Result<Vec<MonitorInfo>, AppError> {
        ensure_x11()?;
        x::X::connect()?.monitors()
    }

    fn focus(&self, id: &WindowId) -> Result<(), AppError> {
        with_window(id, |x, win| x.activate(win))
    }

    fn windows(&self) -> Result<Vec<WindowInfo>, AppError> {
        ensure_x11()?;
        x::X::connect()?.windows(std::process::id())
    }

    /// Starts the app from its executable path, or from the lowercased class name as a guess.
    fn launch(&self, app_id: &str, app_name: &str) -> Result<(), AppError> {
        let program = if app_id.starts_with('/') {
            app_id.to_string()
        } else {
            app_name.to_lowercase()
        };
        std::process::Command::new(&program)
            .spawn()
            .map(|_| ())
            .map_err(|e| AppError::Platform(format!("Could not launch {app_name}: {e}")))
    }

    fn watch(&self, id: &WindowId, sink: EventSink) -> Result<WatchGuard, AppError> {
        ensure_x11()?;
        watch::watch_window(parse_window_id(id.as_str())?, sink)
    }
}

/// Enumerates visible application windows on Linux (X11).
#[cfg(target_os = "linux")]
pub fn list_windows(_app: &tauri::AppHandle) -> Result<Vec<AppWindowInfo>, AppError> {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        log::warn!(
            "[window_management] list_windows: Wayland does not support foreign window management"
        );
        return Ok(Vec::new());
    }
    match x::X::connect() {
        Ok(x) => x.list_windows(std::process::id()),
        Err(_) => Ok(Vec::new()),
    }
}

/// Activates and brings the target window to the foreground on Linux (X11).
#[cfg(target_os = "linux")]
pub fn focus_window(id: &str) -> Result<(), AppError> {
    with_window(&WindowId::new(id), |x, win| x.activate(win))
}

/// Closes the target window on Linux (X11).
#[cfg(target_os = "linux")]
pub fn close_window(id: &str) -> Result<(), AppError> {
    with_window(&WindowId::new(id), |x, win| x.close(win))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prefixed_and_bare_ids() {
        assert_eq!(parse_window_id("linux:12345").unwrap(), 12345);
        assert_eq!(parse_window_id("12345").unwrap(), 12345);
    }

    #[test]
    fn rejects_null_and_malformed_ids() {
        assert!(matches!(
            parse_window_id("linux:0"),
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            parse_window_id("linux:abc"),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(parse_window_id(""), Err(AppError::Validation(_))));
    }

    #[test]
    fn format_round_trips() {
        let id = format_window_id(777);
        assert_eq!(id.as_str(), "linux:777");
        assert_eq!(parse_window_id(id.as_str()).unwrap(), 777);
    }

    #[test]
    fn rejects_zero_wid_on_x11() {
        let result = check_x11_inner(0, false);
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[test]
    fn rejects_wayland() {
        let result = check_x11_inner(12345, true);
        assert!(result.unwrap_err().to_string().contains("Wayland"));
    }
}
