#![cfg(target_os = "windows")]

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, Once, OnceLock};
use std::thread;

use crate::error::AppError;
use crate::window_management::backend::{
    EventSink, MonitorInfo, WatchGuard, WindowBackend, WindowEvent, WindowId, WindowInfo,
};
use crate::window_management::layout::visible_to_outer;
use crate::window_management::types::{AppWindowInfo, WindowBounds};
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, DispatchMessageW, EnumWindows, GetMessageW, GetWindowLongW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    IsZoomed, PostMessageW, SetForegroundWindow, SetWindowPos, ShowWindow, TranslateMessage,
    CHILDID_SELF, EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_MINIMIZESTART,
    GWL_EXSTYLE, MSG, OBJID_WINDOW, SWP_NOACTIVATE, SWP_NOZORDER, SW_MAXIMIZE, SW_MINIMIZE,
    SW_RESTORE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_CLOSE, WS_EX_TOOLWINDOW,
};

/// Converts a Win32 RECT (left, top, right, bottom) to WindowBounds.
pub fn rect_to_bounds(rect: &RECT) -> WindowBounds {
    WindowBounds {
        x: rect.left as f64,
        y: rect.top as f64,
        width: (rect.right - rect.left) as f64,
        height: (rect.bottom - rect.top) as f64,
    }
}

pub fn format_window_id(hwnd: isize) -> WindowId {
    WindowId::new(format!("win:{hwnd}"))
}

fn parse_hwnd(id: &str) -> Result<HWND, AppError> {
    let raw = id.strip_prefix("win:").unwrap_or(id);
    let hwnd_val: isize = raw
        .parse()
        .map_err(|_| AppError::Validation(format!("Invalid Windows HWND: {id}")))?;
    if hwnd_val == 0 {
        return Err(AppError::NotFound("Null HWND".to_string()));
    }
    Ok(HWND(hwnd_val as *mut _))
}

/// The rect the user sees: the DWM frame without the invisible resize borders.
unsafe fn visible_rect(hwnd: HWND, outer: &RECT) -> RECT {
    let mut rect = RECT::default();
    let ok = DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut rect as *mut RECT as *mut c_void,
        size_of::<RECT>() as u32,
    )
    .is_ok();
    if ok {
        rect
    } else {
        *outer
    }
}

pub fn get_window_bounds(id: &WindowId) -> Result<WindowBounds, AppError> {
    let hwnd = parse_hwnd(id.as_str())?;
    unsafe {
        let mut outer = RECT::default();
        GetWindowRect(hwnd, &mut outer)
            .map_err(|e| AppError::Platform(format!("GetWindowRect failed: {e}")))?;
        Ok(rect_to_bounds(&visible_rect(hwnd, &outer)))
    }
}

/// Places the window so its *visible* edges land on `bounds`.
pub fn set_window_bounds(id: &WindowId, bounds: &WindowBounds) -> Result<(), AppError> {
    let hwnd = parse_hwnd(id.as_str())?;
    unsafe {
        if IsIconic(hwnd).as_bool() || IsZoomed(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let mut outer = RECT::default();
        GetWindowRect(hwnd, &mut outer)
            .map_err(|e| AppError::Platform(format!("GetWindowRect failed: {e}")))?;
        let visible = visible_rect(hwnd, &outer);
        let target = visible_to_outer(bounds, &rect_to_bounds(&visible), &rect_to_bounds(&outer));
        SetWindowPos(
            hwnd,
            None,
            target.x.round() as i32,
            target.y.round() as i32,
            target.width.round() as i32,
            target.height.round() as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|e| AppError::Platform(format!("SetWindowPos failed: {e}")))
    }
}

/// Maximizes (fullscreen equivalent on Windows) or restores the window.
pub fn set_window_fullscreen(id: &WindowId, enable: bool) -> Result<(), AppError> {
    let hwnd = parse_hwnd(id.as_str())?;
    unsafe {
        let cmd = if enable { SW_MAXIMIZE } else { SW_RESTORE };
        let _ = ShowWindow(hwnd, cmd);
    }
    Ok(())
}

pub fn minimize_window(id: &WindowId) -> Result<(), AppError> {
    let hwnd = parse_hwnd(id.as_str())?;
    unsafe {
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
    }
    Ok(())
}

/// Displays in physical pixels, with the work area excluding the taskbar.
pub fn monitors() -> Result<Vec<MonitorInfo>, AppError> {
    unsafe extern "system" fn monitor_proc(
        monitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let out = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            out.push(MonitorInfo {
                frame: rect_to_bounds(&info.rcMonitor),
                work_area: rect_to_bounds(&info.rcWork),
            });
        }
        BOOL(1)
    }

    let mut out: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_proc),
            LPARAM(&mut out as *mut _ as isize),
        );
    }
    Ok(out)
}

type Watchers = Mutex<HashMap<isize, Vec<(u64, EventSink)>>>;

fn watchers() -> &'static Watchers {
    static WATCHERS: OnceLock<Watchers> = OnceLock::new();
    WATCHERS.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe extern "system" fn on_win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    // LOCATIONCHANGE also fires for carets and cursors; only the window itself matters.
    if hwnd.0.is_null() || id_object != OBJID_WINDOW.0 || id_child != CHILDID_SELF as i32 {
        return;
    }
    let kind = if event == EVENT_OBJECT_LOCATIONCHANGE {
        WindowEvent::Changed
    } else {
        WindowEvent::Gone
    };
    let sinks: Vec<EventSink> = watchers()
        .lock()
        .ok()
        .and_then(|map| {
            map.get(&(hwnd.0 as isize))
                .map(|list| list.iter().map(|(_, sink)| sink.clone()).collect())
        })
        .unwrap_or_default();
    for sink in sinks {
        sink(kind);
    }
}

/// Out-of-context WinEvent hooks are delivered through the message queue of the thread that
/// installed them, so one dedicated thread installs them all and pumps messages for good.
fn ensure_hook_thread() {
    static START: Once = Once::new();
    START.call_once(|| {
        let _ = thread::Builder::new()
            .name("window-watch".to_string())
            .spawn(|| unsafe {
                let hooks = [
                    EVENT_OBJECT_LOCATIONCHANGE,
                    EVENT_OBJECT_DESTROY,
                    EVENT_SYSTEM_MINIMIZESTART,
                ]
                .map(|event| {
                    SetWinEventHook(
                        event,
                        event,
                        None,
                        Some(on_win_event),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    )
                });
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                for hook in hooks {
                    let _ = UnhookWinEvent(hook);
                }
            });
    });
}

struct WinWatch {
    hwnd: isize,
    token: u64,
}

impl Drop for WinWatch {
    fn drop(&mut self) {
        if let Ok(mut map) = watchers().lock() {
            if let Some(list) = map.get_mut(&self.hwnd) {
                list.retain(|(token, _)| *token != self.token);
                if list.is_empty() {
                    map.remove(&self.hwnd);
                }
            }
        }
    }
}

pub fn watch_window(id: &WindowId, sink: EventSink) -> Result<WatchGuard, AppError> {
    let hwnd = parse_hwnd(id.as_str())?;
    // Fail now, not silently later, if the window is already gone.
    get_window_bounds(id)?;
    static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
    let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let key = hwnd.0 as isize;
    watchers()
        .lock()
        .map_err(|_| AppError::Lock)?
        .entry(key)
        .or_default()
        .push((token, sink));
    ensure_hook_thread();
    Ok(WatchGuard::new(WinWatch { hwnd: key, token }))
}

/// Path of the executable behind `pid`, which identifies the app across sessions.
fn executable_of(pid: u32) -> Option<String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = vec![0u16; 1024];
        let mut len = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

/// Visible, non-minimized application windows, topmost first, with their app identity.
pub fn windows() -> Result<Vec<WindowInfo>, AppError> {
    struct EnumState {
        windows: Vec<WindowInfo>,
        our_pid: u32,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut EnumState);

        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }
        if (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0) != 0 {
            return BOOL(1);
        }
        // Windows hidden on other virtual desktops are "cloaked".
        let mut cloaked: u32 = 0;
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
        {
            return BOOL(1);
        }

        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return BOOL(1);
        }
        let mut title_buf = vec![0u16; (len + 1) as usize];
        let actual_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..actual_len.max(0) as usize])
            .trim()
            .to_string();
        if title.is_empty() {
            return BOOL(1);
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 || pid == state.our_pid {
            return BOOL(1);
        }

        let mut outer = RECT::default();
        if GetWindowRect(hwnd, &mut outer).is_err() {
            return BOOL(1);
        }
        let bounds = rect_to_bounds(&visible_rect(hwnd, &outer));
        if bounds.width < 50.0 || bounds.height < 50.0 {
            return BOOL(1);
        }

        let app_id = executable_of(pid).unwrap_or_else(|| title.clone());
        let app_name = std::path::Path::new(&app_id)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| title.clone());
        state.windows.push(WindowInfo {
            id: format_window_id(hwnd.0 as isize),
            app_id,
            app_name,
            title,
            bounds,
        });
        BOOL(1)
    }

    let mut state = EnumState {
        windows: Vec::new(),
        our_pid: std::process::id(),
    };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut state as *mut _ as isize));
    }
    Ok(state.windows)
}

/// Starts an app from its executable path.
pub fn launch_app(app_id: &str, app_name: &str) -> Result<(), AppError> {
    std::process::Command::new(app_id)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::Platform(format!("Could not launch {app_name}: {e}")))
}

pub struct WindowsBackend;

impl WindowBackend for WindowsBackend {
    fn bounds(&self, id: &WindowId) -> Result<WindowBounds, AppError> {
        get_window_bounds(id)
    }

    fn set_bounds(&self, id: &WindowId, bounds: &WindowBounds) -> Result<(), AppError> {
        set_window_bounds(id, bounds)
    }

    fn set_fullscreen(&self, id: &WindowId, enable: bool) -> Result<(), AppError> {
        set_window_fullscreen(id, enable)
    }

    fn minimize(&self, id: &WindowId) -> Result<(), AppError> {
        minimize_window(id)
    }

    fn monitors(&self) -> Result<Vec<MonitorInfo>, AppError> {
        monitors()
    }

    fn focus(&self, id: &WindowId) -> Result<(), AppError> {
        focus_window(id.as_str())
    }

    fn windows(&self) -> Result<Vec<WindowInfo>, AppError> {
        windows()
    }

    fn launch(&self, app_id: &str, app_name: &str) -> Result<(), AppError> {
        launch_app(app_id, app_name)
    }

    fn watch(&self, id: &WindowId, sink: EventSink) -> Result<WatchGuard, AppError> {
        watch_window(id, sink)
    }
}

/// Enumerates visible top-level application windows on Windows.
pub fn list_windows(_app: &tauri::AppHandle) -> Result<Vec<AppWindowInfo>, AppError> {
    struct EnumState {
        windows: Vec<AppWindowInfo>,
        our_pid: u32,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut EnumState);

        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
            return BOOL(1);
        }

        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return BOOL(1);
        }

        let mut title_buf = vec![0u16; (len + 1) as usize];
        let actual_len = GetWindowTextW(hwnd, &mut title_buf);
        if actual_len == 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&title_buf[..actual_len as usize]);
        let title = title.trim().to_string();
        if title.is_empty() {
            return BOOL(1);
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 || pid == state.our_pid {
            return BOOL(1);
        }

        let is_minimized = IsIconic(hwnd).as_bool();
        let is_focused = state.windows.is_empty();

        let app_name = title.clone();

        state.windows.push(AppWindowInfo {
            id: format!("win:{}", hwnd.0 as isize),
            pid: pid as i32,
            app_name,
            app_bundle_id: None,
            title,
            is_minimized,
            is_focused,
            app_icon: None,
        });

        BOOL(1)
    }

    let mut state = EnumState {
        windows: Vec::new(),
        our_pid: std::process::id(),
    };

    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut state as *mut _ as isize));
    }

    Ok(state.windows)
}

/// Restores and brings the target window to the foreground on Windows.
pub fn focus_window(id: &str) -> Result<(), AppError> {
    let hwnd = parse_hwnd(id)?;

    unsafe {
        if IsIconic(hwnd).as_bool() {
            ShowWindow(hwnd, SW_RESTORE);
        }
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
    }
    Ok(())
}

/// Closes the target window on Windows by posting WM_CLOSE.
pub fn close_window(id: &str) -> Result<(), AppError> {
    let hwnd = parse_hwnd(id)?;

    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_to_bounds_basic() {
        let rect = RECT {
            left: 100,
            top: 200,
            right: 1380,
            bottom: 1000,
        };
        let bounds = rect_to_bounds(&rect);
        assert_eq!(bounds.x, 100.0);
        assert_eq!(bounds.y, 200.0);
        assert_eq!(bounds.width, 1280.0);
        assert_eq!(bounds.height, 800.0);
    }

    #[test]
    fn rect_to_bounds_at_origin() {
        let rect = RECT {
            left: 0,
            top: 0,
            right: 800,
            bottom: 600,
        };
        let bounds = rect_to_bounds(&rect);
        assert_eq!(bounds.x, 0.0);
        assert_eq!(bounds.y, 0.0);
        assert_eq!(bounds.width, 800.0);
        assert_eq!(bounds.height, 600.0);
    }

    #[test]
    fn get_window_bounds_rejects_null_hwnd() {
        let result = get_window_bounds(&WindowId::new("win:0"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[test]
    fn set_window_bounds_rejects_null_hwnd() {
        let bounds = WindowBounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert!(set_window_bounds(&WindowId::new("win:0"), &bounds).is_err());
    }

    #[test]
    fn set_window_fullscreen_rejects_null_hwnd() {
        assert!(set_window_fullscreen(&WindowId::new("win:0"), true).is_err());
    }

    #[test]
    fn minimize_window_rejects_null_hwnd() {
        assert!(minimize_window(&WindowId::new("win:0")).is_err());
    }

    #[test]
    fn focus_window_rejects_invalid_hwnd() {
        assert!(focus_window("win:0").is_err());
        assert!(focus_window("invalid").is_err());
    }

    #[test]
    fn close_window_rejects_invalid_hwnd() {
        assert!(close_window("win:0").is_err());
        assert!(close_window("invalid").is_err());
    }
}
