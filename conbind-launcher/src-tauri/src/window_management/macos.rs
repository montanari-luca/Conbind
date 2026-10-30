#![cfg(target_os = "macos")]

use std::ffi::c_void;
use std::ptr;
use std::sync::Mutex;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, msg_send_id};
use objc2_foundation::{NSRect, NSString};
use tauri::Manager;

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
use core_foundation::string::CFString;

use crate::error::AppError;
use crate::window_management::backend::{
    EventSink, MonitorInfo, WatchGuard, WindowBackend, WindowEvent, WindowId, WindowInfo,
};
use crate::window_management::types::{AppWindowInfo, WindowBounds};

#[repr(C)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
struct CGSize {
    width: f64,
    height: f64,
}

type AxObserverCallback = extern "C" fn(
    observer: *mut c_void,
    element: *mut c_void,
    notification: *mut c_void,
    refcon: *mut c_void,
);

const K_AX_VALUE_CG_POINT: u32 = 1;
const K_AX_VALUE_CG_SIZE: u32 = 2;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(
        option: u32,
        relative_to_window: u32,
    ) -> core_foundation::array::CFArrayRef;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> *mut c_void;
    fn AXUIElementCopyAttributeValue(
        element: *mut c_void,
        attribute: *mut c_void,
        value: *mut *mut c_void,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: *mut c_void,
        attribute: *mut c_void,
        value: *mut c_void,
    ) -> i32;
    fn AXUIElementPerformAction(element: *mut c_void, action: *mut c_void) -> i32;
    fn AXValueCreate(the_type: u32, value: *const c_void) -> *mut c_void;
    fn AXValueGetValue(value: *mut c_void, the_type: u32, value_ptr: *mut c_void) -> bool;
    fn _AXUIElementGetWindow(element: *mut c_void, wid: *mut u32) -> i32;
    fn AXObserverCreate(pid: i32, callback: AxObserverCallback, out: *mut *mut c_void) -> i32;
    fn AXObserverAddNotification(
        observer: *mut c_void,
        element: *mut c_void,
        notification: *mut c_void,
        refcon: *mut c_void,
    ) -> i32;
    fn AXObserverRemoveNotification(
        observer: *mut c_void,
        element: *mut c_void,
        notification: *mut c_void,
    ) -> i32;
    fn AXObserverGetRunLoopSource(observer: *mut c_void) -> *mut c_void;
    fn CFRelease(cf: *mut c_void);
    fn CFRetain(cf: *mut c_void) -> *mut c_void;
    static kCFBooleanTrue: *mut c_void;
    static kCFBooleanFalse: *mut c_void;
}

/// Owned CoreFoundation reference, released on drop.
struct CfOwned(*mut c_void);

impl Drop for CfOwned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}

unsafe fn copy_attr(elem: *mut c_void, name: &str) -> Result<CfOwned, i32> {
    let attr = NSString::from_str(name);
    let mut out: *mut c_void = ptr::null_mut();
    let err = AXUIElementCopyAttributeValue(elem, Retained::as_ptr(&attr) as *mut _, &mut out);
    if err != 0 || out.is_null() {
        Err(if err != 0 { err } else { -1 })
    } else {
        Ok(CfOwned(out))
    }
}

unsafe fn set_attr(elem: *mut c_void, name: &str, value: *mut c_void) -> i32 {
    let attr = NSString::from_str(name);
    AXUIElementSetAttributeValue(elem, Retained::as_ptr(&attr) as *mut _, value)
}

unsafe fn read_point(elem: *mut c_void) -> Result<CGPoint, AppError> {
    let value = copy_attr(elem, "AXPosition")
        .map_err(|e| AppError::Platform(format!("AXPosition error: {e}")))?;
    let mut point = CGPoint { x: 0.0, y: 0.0 };
    AXValueGetValue(
        value.0,
        K_AX_VALUE_CG_POINT,
        &mut point as *mut _ as *mut c_void,
    );
    Ok(point)
}

unsafe fn read_size(elem: *mut c_void) -> Result<CGSize, AppError> {
    let value =
        copy_attr(elem, "AXSize").map_err(|e| AppError::Platform(format!("AXSize error: {e}")))?;
    let mut size = CGSize {
        width: 0.0,
        height: 0.0,
    };
    AXValueGetValue(
        value.0,
        K_AX_VALUE_CG_SIZE,
        &mut size as *mut _ as *mut c_void,
    );
    Ok(size)
}

unsafe fn write_point(elem: *mut c_void, x: f64, y: f64) -> Result<(), AppError> {
    let point = CGPoint { x, y };
    let value = CfOwned(AXValueCreate(
        K_AX_VALUE_CG_POINT,
        &point as *const _ as *const c_void,
    ));
    if value.0.is_null() {
        return Err(AppError::Platform(
            "AXValueCreate(CGPoint) failed".to_string(),
        ));
    }
    match set_attr(elem, "AXPosition", value.0) {
        0 => Ok(()),
        err => Err(AppError::Platform(format!("AXPosition write error: {err}"))),
    }
}

/// Size writes are best effort: fixed-size windows reject them but can still be moved.
unsafe fn write_size(elem: *mut c_void, width: f64, height: f64) -> Result<(), AppError> {
    let size = CGSize { width, height };
    let value = CfOwned(AXValueCreate(
        K_AX_VALUE_CG_SIZE,
        &size as *const _ as *const c_void,
    ));
    if value.0.is_null() {
        return Err(AppError::Platform(
            "AXValueCreate(CGSize) failed".to_string(),
        ));
    }
    set_attr(elem, "AXSize", value.0);
    Ok(())
}

fn parse_window_id(id: &WindowId) -> Result<(i32, u32), AppError> {
    let parts: Vec<&str> = id.as_str().split(':').collect();
    if parts.len() < 3 || parts[0] != "macos" {
        return Err(AppError::Validation(format!(
            "Invalid macOS window ID: {}",
            id.as_str()
        )));
    }
    let pid = parts[1]
        .parse()
        .map_err(|_| AppError::Validation(format!("Invalid pid in window ID: {}", id.as_str())))?;
    let wid = parts[2]
        .parse()
        .map_err(|_| AppError::Validation(format!("Invalid wid in window ID: {}", id.as_str())))?;
    Ok((pid, wid))
}

fn format_window_id(pid: i32, wid: u32) -> WindowId {
    WindowId::new(format!("macos:{pid}:{wid}"))
}

/// AX element for the window `wid` of process `pid`. Retained; released on drop.
unsafe fn find_window_element(pid: i32, wid: u32) -> Result<CfOwned, AppError> {
    let app = CfOwned(AXUIElementCreateApplication(pid));
    if app.0.is_null() {
        return Err(AppError::Platform(
            "Failed to create AX element for app".to_string(),
        ));
    }
    let windows = copy_attr(app.0, "AXWindows")
        .map_err(|e| AppError::Platform(format!("AXWindows unavailable: {e}")))?;
    let array: CFArray<CFType> = CFArray::wrap_under_get_rule(windows.0 as _);
    for i in 0..array.len() {
        if let Some(w) = array.get(i) {
            let elem = w.as_CFTypeRef() as *mut c_void;
            let mut elem_wid: u32 = 0;
            if _AXUIElementGetWindow(elem, &mut elem_wid) == 0 && elem_wid == wid {
                return Ok(CfOwned(CFRetain(elem)));
            }
        }
    }
    Err(AppError::NotFound(format!("window {pid}:{wid}")))
}

unsafe fn window_element(id: &WindowId) -> Result<CfOwned, AppError> {
    let (pid, wid) = parse_window_id(id)?;
    find_window_element(pid, wid)
}

fn ax_denied() -> AppError {
    AppError::Permission(
        "Accessibility permission required for WindowManagementService. \
         Enable it in System Settings > Privacy & Security > Accessibility."
            .to_string(),
    )
}

pub fn check_ax_permission() -> Result<(), AppError> {
    if !crate::platform::macos::is_accessibility_trusted() {
        return Err(ax_denied());
    }
    Ok(())
}

/// Like [`check_ax_permission`], but when permission is missing it also raises the system
/// Accessibility prompt so the user can grant it. For user-initiated actions only; background
/// observers keep using the silent check.
fn require_ax_permission() -> Result<(), AppError> {
    if !crate::platform::macos::request_accessibility_trust() {
        return Err(ax_denied());
    }
    Ok(())
}

/// Id of the frontmost app's focused window: the target of keyboard-driven actions.
pub fn focused_window_id() -> Result<WindowId, AppError> {
    require_ax_permission()?;
    unsafe {
        let pid = crate::platform::macos::get_frontmost_app_pid()
            .ok_or_else(|| AppError::NotFound("frontmost app pid".to_string()))?;
        let app = CfOwned(AXUIElementCreateApplication(pid));
        if app.0.is_null() {
            return Err(AppError::Platform(
                "AXUIElementCreateApplication returned null".to_string(),
            ));
        }
        let window = copy_attr(app.0, "AXFocusedWindow").map_err(|e| {
            AppError::Platform(format!(
                "AXFocusedWindow unavailable (code {e}). Ensure Accessibility permission is granted."
            ))
        })?;
        let mut wid: u32 = 0;
        if _AXUIElementGetWindow(window.0, &mut wid) != 0 || wid == 0 {
            return Err(AppError::Platform("Focused window has no id".to_string()));
        }
        Ok(format_window_id(pid, wid))
    }
}

pub fn get_window_bounds(id: &WindowId) -> Result<WindowBounds, AppError> {
    check_ax_permission()?;
    unsafe {
        let window = window_element(id)?;
        let point = read_point(window.0)?;
        let size = read_size(window.0)?;
        Ok(WindowBounds {
            x: point.x,
            y: point.y,
            width: size.width,
            height: size.height,
        })
    }
}

/// Apps clamp a window's size against its current position, so the size is written
/// before and after the move.
pub fn set_window_bounds(id: &WindowId, bounds: &WindowBounds) -> Result<(), AppError> {
    check_ax_permission()?;
    unsafe {
        let window = window_element(id)?;
        write_size(window.0, bounds.width, bounds.height)?;
        write_point(window.0, bounds.x, bounds.y)?;
        write_size(window.0, bounds.width, bounds.height)
    }
}

pub fn set_window_fullscreen(id: &WindowId, enable: bool) -> Result<(), AppError> {
    check_ax_permission()?;
    unsafe {
        let window = window_element(id)?;
        let value = if enable {
            kCFBooleanTrue
        } else {
            kCFBooleanFalse
        };
        set_attr(window.0, "AXFullScreen", value);
        Ok(())
    }
}

pub fn minimize_window(id: &WindowId) -> Result<(), AppError> {
    check_ax_permission()?;
    unsafe {
        let window = window_element(id)?;
        match set_attr(window.0, "AXMinimized", kCFBooleanTrue) {
            0 => Ok(()),
            err => Err(AppError::Platform(format!(
                "AXMinimized write error: {err}"
            ))),
        }
    }
}

/// Displays in the AX coordinate space (points, origin at the top-left of the primary display),
/// with the work area excluding the menu bar and Dock.
pub fn monitors() -> Result<Vec<MonitorInfo>, AppError> {
    let cls = AnyClass::get("NSScreen")
        .ok_or_else(|| AppError::Platform("NSScreen unavailable".to_string()))?;
    unsafe {
        let screens: *mut AnyObject = msg_send![cls, screens];
        if screens.is_null() {
            return Ok(Vec::new());
        }
        // `screens` may be a Swift-backed array whose `count` is declared as a signed integer,
        // which objc2's checked `msg_send!` rejects for `usize`. CoreFoundation's toll-free
        // bridging reads it regardless of the concrete class.
        let array = screens as core_foundation_sys::array::CFArrayRef;
        let count = core_foundation_sys::array::CFArrayGetCount(array).max(0);
        let mut raw: Vec<(NSRect, NSRect)> = Vec::with_capacity(count as usize);
        for i in 0..count {
            let screen =
                core_foundation_sys::array::CFArrayGetValueAtIndex(array, i) as *mut AnyObject;
            if screen.is_null() {
                continue;
            }
            let frame: NSRect = msg_send![screen, frame];
            let visible: NSRect = msg_send![screen, visibleFrame];
            raw.push((frame, visible));
        }
        // AppKit puts the origin at the bottom-left of the primary display (screens[0]).
        let primary_height = raw.first().map(|(f, _)| f.size.height).unwrap_or(0.0);
        let to_top_left = |r: &NSRect| WindowBounds {
            x: r.origin.x,
            y: primary_height - (r.origin.y + r.size.height),
            width: r.size.width,
            height: r.size.height,
        };
        Ok(raw
            .iter()
            .map(|(frame, visible)| MonitorInfo {
                frame: to_top_left(frame),
                work_area: to_top_left(visible),
            })
            .collect())
    }
}

const WATCHED_NOTIFICATIONS: [&str; 4] = [
    "AXMoved",
    "AXResized",
    "AXUIElementDestroyed",
    "AXWindowMiniaturized",
];

/// State shared with the AX callback. Deliberately leaked (a few bytes per watch): the callback
/// can run on the main thread while a guard is dropped elsewhere, so the context must stay valid.
/// Dropping the guard clears `sink`, which turns any late callback into a no-op.
struct WatchCtx {
    sink: Mutex<Option<EventSink>>,
}

extern "C" fn on_ax_notification(
    _observer: *mut c_void,
    _element: *mut c_void,
    notification: *mut c_void,
    refcon: *mut c_void,
) {
    if refcon.is_null() || notification.is_null() {
        return;
    }
    let ctx = unsafe { &*(refcon as *const WatchCtx) };
    let name = unsafe { CFString::wrap_under_get_rule(notification as _) }.to_string();
    let event = match name.as_str() {
        "AXUIElementDestroyed" | "AXWindowMiniaturized" => WindowEvent::Gone,
        _ => WindowEvent::Changed,
    };
    if let Ok(sink) = ctx.sink.lock() {
        if let Some(sink) = sink.as_ref() {
            sink(event);
        }
    }
}

struct AxWatch {
    observer: CfOwned,
    element: CfOwned,
    source: CFRunLoopSource,
    ctx: &'static WatchCtx,
}

// The raw CF references are only touched in `drop`, which CoreFoundation allows from any thread.
unsafe impl Send for AxWatch {}

impl Drop for AxWatch {
    fn drop(&mut self) {
        if let Ok(mut sink) = self.ctx.sink.lock() {
            *sink = None;
        }
        unsafe {
            for name in WATCHED_NOTIFICATIONS {
                let name = NSString::from_str(name);
                AXObserverRemoveNotification(
                    self.observer.0,
                    self.element.0,
                    Retained::as_ptr(&name) as *mut _,
                );
            }
        }
        CFRunLoop::get_main().remove_source(&self.source, unsafe { kCFRunLoopCommonModes });
    }
}

/// Observes `id` through the Accessibility API; callbacks arrive on the main run loop, in
/// common modes so they keep firing while the user drags a window.
pub fn watch_window(id: &WindowId, sink: EventSink) -> Result<WatchGuard, AppError> {
    check_ax_permission()?;
    let (pid, wid) = parse_window_id(id)?;
    unsafe {
        let element = find_window_element(pid, wid)?;
        let mut observer_ref: *mut c_void = ptr::null_mut();
        let err = AXObserverCreate(pid, on_ax_notification, &mut observer_ref);
        if err != 0 || observer_ref.is_null() {
            return Err(AppError::Platform(format!("AXObserverCreate error: {err}")));
        }
        let observer = CfOwned(observer_ref);

        let ctx: &'static WatchCtx = Box::leak(Box::new(WatchCtx {
            sink: Mutex::new(Some(sink)),
        }));
        let refcon = ctx as *const WatchCtx as *mut c_void;
        let mut added = 0;
        for name in WATCHED_NOTIFICATIONS {
            let name = NSString::from_str(name);
            if AXObserverAddNotification(
                observer.0,
                element.0,
                Retained::as_ptr(&name) as *mut _,
                refcon,
            ) == 0
            {
                added += 1;
            }
        }
        if added == 0 {
            if let Ok(mut sink) = ctx.sink.lock() {
                *sink = None;
            }
            return Err(AppError::Platform(
                "Window does not support change notifications".to_string(),
            ));
        }

        let source =
            CFRunLoopSource::wrap_under_get_rule(AXObserverGetRunLoopSource(observer.0)
                as core_foundation::runloop::CFRunLoopSourceRef);
        CFRunLoop::get_main().add_source(&source, kCFRunLoopCommonModes);
        Ok(WatchGuard::new(AxWatch {
            observer,
            element,
            source,
            ctx,
        }))
    }
}

/// Visible windows with their app identity, for workspaces. Skips minimized windows.
pub fn windows() -> Result<Vec<WindowInfo>, AppError> {
    require_ax_permission()?;
    Ok(collect_windows(None)?
        .into_iter()
        .filter(|(info, _)| !info.is_minimized)
        .map(|(info, bounds)| WindowInfo {
            id: WindowId::new(info.id),
            app_id: info
                .app_bundle_id
                .clone()
                .unwrap_or_else(|| info.app_name.clone()),
            app_name: info.app_name,
            title: info.title,
            bounds,
        })
        .collect())
}

/// Starts an app by bundle id, or by name when only the name is known.
pub fn launch_app(app_id: &str, app_name: &str) -> Result<(), AppError> {
    let mut command = std::process::Command::new("open");
    if app_id.contains('.') {
        command.args(["-b", app_id]);
    } else {
        command.args(["-a", app_name]);
    }
    let status = command
        .status()
        .map_err(|e| AppError::Platform(format!("open failed: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::Platform(format!("Could not launch {app_name}")))
    }
}

pub struct MacBackend;

impl WindowBackend for MacBackend {
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

/// Attempts to retrieve a window's title via the Accessibility API as a fallback.
unsafe fn get_window_title_via_ax(pid: i32, target_wid: u32) -> Option<String> {
    let app_elem = AXUIElementCreateApplication(pid);
    if app_elem.is_null() {
        return None;
    }
    let windows_attr = NSString::from_str("AXWindows");
    let mut windows_ref: *mut c_void = ptr::null_mut();
    let err = AXUIElementCopyAttributeValue(
        app_elem,
        Retained::as_ptr(&windows_attr) as *mut _,
        &mut windows_ref,
    );
    CFRelease(app_elem);
    if err != 0 || windows_ref.is_null() {
        return None;
    }

    let win_array: CFArray<CFType> = CFArray::wrap_under_create_rule(windows_ref as _);
    for i in 0..win_array.len() {
        if let Some(w) = win_array.get(i) {
            let win_elem = w.as_CFTypeRef() as *mut c_void;
            let mut elem_wid: u32 = 0;
            if _AXUIElementGetWindow(win_elem, &mut elem_wid) == 0 && elem_wid == target_wid {
                let title_attr = NSString::from_str("AXTitle");
                let mut title_ref: *mut c_void = ptr::null_mut();
                if AXUIElementCopyAttributeValue(
                    win_elem,
                    Retained::as_ptr(&title_attr) as *mut _,
                    &mut title_ref,
                ) == 0
                    && !title_ref.is_null()
                {
                    let cf_title: CFString = CFString::wrap_under_create_rule(title_ref as _);
                    let title_str = cf_title.to_string();
                    if !title_str.trim().is_empty() {
                        return Some(title_str);
                    }
                }
            }
        }
    }
    None
}

/// Enumerates on-screen and active windows using CoreGraphics and Accessibility.
pub fn list_windows(app: &tauri::AppHandle) -> Result<Vec<AppWindowInfo>, AppError> {
    let icon_cache_dir = app.path().app_data_dir().map(|d| d.join("icon_cache")).ok();
    Ok(collect_windows(icon_cache_dir.as_deref())?
        .into_iter()
        .map(|(info, _)| info)
        .collect())
}

/// On-screen windows frontmost first, each with its bounds in the AX coordinate space.
/// App icons are only extracted when `icon_cache_dir` is given.
fn collect_windows(
    icon_cache_dir: Option<&std::path::Path>,
) -> Result<Vec<(AppWindowInfo, WindowBounds)>, AppError> {
    let our_pid = std::process::id() as i32;

    unsafe {
        // kCGWindowListOptionOnScreenOnly (1) | kCGWindowListExcludeDesktopElements (16)
        let window_list_ref = CGWindowListCopyWindowInfo(1 | 16, 0);
        if window_list_ref.is_null() {
            return Ok(Vec::new());
        }

        let window_list: CFArray<CFType> = CFArray::wrap_under_create_rule(window_list_ref);
        let count = window_list.len();

        let mut results = Vec::new();
        let mut found_focused = false;

        let layer_key = CFString::new("kCGWindowLayer");
        let bounds_key = CFString::new("kCGWindowBounds");
        let name_key = CFString::new("kCGWindowName");
        let owner_name_key = CFString::new("kCGWindowOwnerName");
        let pid_key = CFString::new("kCGWindowOwnerPID");
        let num_key = CFString::new("kCGWindowNumber");
        let is_onscreen_key = CFString::new("kCGWindowIsOnscreen");

        let app_class = AnyClass::get("NSRunningApplication");

        for i in 0..count {
            let item = match window_list.get(i) {
                Some(it) => it,
                None => continue,
            };

            let dict_ref = item.as_CFTypeRef() as core_foundation::dictionary::CFDictionaryRef;
            if dict_ref.is_null() {
                continue;
            }
            let dict: CFDictionary<CFString, CFType> = CFDictionary::wrap_under_get_rule(dict_ref);

            // Layer 0 is normal user application windows
            let layer = dict
                .find(&layer_key)
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i32())
                .unwrap_or(-1);
            if layer != 0 {
                continue;
            }

            // PID check
            let pid = dict
                .find(&pid_key)
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i32())
                .unwrap_or(0);
            if pid <= 0 || pid == our_pid {
                continue;
            }

            // Window ID
            let wid = dict
                .find(&num_key)
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i64())
                .unwrap_or(0) as u32;
            if wid == 0 {
                continue;
            }

            // Window geometry; anything under 50x50 is an invisible/helper window.
            let mut win_bounds = WindowBounds {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            };
            if let Some(bounds_val) = dict.find(&bounds_key) {
                let bounds_dict_ref =
                    bounds_val.as_CFTypeRef() as core_foundation::dictionary::CFDictionaryRef;
                if !bounds_dict_ref.is_null() {
                    let bdict: CFDictionary<CFString, CFType> =
                        CFDictionary::wrap_under_get_rule(bounds_dict_ref);
                    let number = |key: &str| {
                        bdict
                            .find(CFString::new(key))
                            .and_then(|v| v.downcast::<CFNumber>())
                            .and_then(|n| n.to_f64())
                            .unwrap_or(0.0)
                    };
                    win_bounds = WindowBounds {
                        x: number("X"),
                        y: number("Y"),
                        width: number("Width"),
                        height: number("Height"),
                    };
                    if win_bounds.width < 50.0 || win_bounds.height < 50.0 {
                        continue;
                    }
                }
            }

            // Running application metadata
            let running_app: *mut AnyObject = if let Some(cls) = app_class {
                msg_send![cls, runningApplicationWithProcessIdentifier: pid]
            } else {
                ptr::null_mut()
            };

            let mut app_name = String::new();
            let mut app_bundle_id = None;
            let mut app_icon = None;

            if !running_app.is_null() {
                // Filter regular GUI apps (activationPolicy == 0)
                let policy: isize = msg_send![running_app, activationPolicy];
                if policy != 0 {
                    continue;
                }

                let name_obj: Option<Retained<NSString>> = msg_send_id![running_app, localizedName];
                if let Some(n) = name_obj {
                    app_name = n.to_string();
                }

                let bid_obj: Option<Retained<NSString>> =
                    msg_send_id![running_app, bundleIdentifier];
                if let Some(b) = bid_obj {
                    app_bundle_id = Some(b.to_string());
                }

                let url: *mut AnyObject = msg_send![running_app, bundleURL];
                if !url.is_null() {
                    let path_obj: Option<Retained<NSString>> = msg_send_id![url, path];
                    if let Some(p) = path_obj {
                        let path_str = p.to_string();
                        if let Some(cache_dir) = icon_cache_dir {
                            app_icon =
                                crate::application::service::extract_app_icon(&path_str, cache_dir);
                        }
                    }
                }
            }

            if app_name.is_empty() {
                app_name = dict
                    .find(&owner_name_key)
                    .and_then(|v| v.downcast::<CFString>())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
            }

            let mut title = dict
                .find(&name_key)
                .and_then(|v| v.downcast::<CFString>())
                .map(|s| s.to_string())
                .unwrap_or_default();

            if title.trim().is_empty() {
                if let Some(ax_title) = get_window_title_via_ax(pid, wid) {
                    title = ax_title;
                }
            }

            if title.trim().is_empty() {
                continue;
            }

            let is_onscreen = dict
                .find(&is_onscreen_key)
                .and_then(|v| v.downcast::<CFBoolean>())
                .map(bool::from)
                .unwrap_or(true);

            let is_focused = if !found_focused {
                found_focused = true;
                true
            } else {
                false
            };

            results.push((
                AppWindowInfo {
                    id: format!("macos:{pid}:{wid}"),
                    pid,
                    app_name,
                    app_bundle_id,
                    title: title.trim().to_string(),
                    is_minimized: !is_onscreen,
                    is_focused,
                    app_icon,
                },
                win_bounds,
            ));
        }

        Ok(results)
    }
}

/// Brings a specific window to the foreground and focuses it.
pub fn focus_window(id: &str) -> Result<(), AppError> {
    let (pid, wid) = parse_window_id(&WindowId::new(id))?;
    require_ax_permission()?;

    unsafe {
        if let Some(cls) = AnyClass::get("NSRunningApplication") {
            let app: *mut AnyObject = msg_send![cls, runningApplicationWithProcessIdentifier: pid];
            if !app.is_null() {
                // NSApplicationActivateIgnoringOtherApps = 1 << 1 = 2
                let _: bool = msg_send![app, activateWithOptions: 2u64];
            }
        }

        let Ok(window) = find_window_element(pid, wid) else {
            return Ok(());
        };
        set_attr(window.0, "AXMinimized", kCFBooleanFalse);
        let raise_action = NSString::from_str("AXRaise");
        AXUIElementPerformAction(window.0, Retained::as_ptr(&raise_action) as *mut _);

        let app = CfOwned(AXUIElementCreateApplication(pid));
        if !app.0.is_null() {
            set_attr(app.0, "AXFocusedWindow", window.0);
        }
    }

    Ok(())
}

/// Closes a specific window using the Accessibility API close action.
pub fn close_window(id: &str) -> Result<(), AppError> {
    let (pid, wid) = parse_window_id(&WindowId::new(id))?;
    require_ax_permission()?;

    unsafe {
        let window = find_window_element(pid, wid)?;
        if let Ok(close_button) = copy_attr(window.0, "AXCloseButton") {
            let press_action = NSString::from_str("AXPress");
            AXUIElementPerformAction(close_button.0, Retained::as_ptr(&press_action) as *mut _);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitors_reads_the_real_displays() {
        let monitors = monitors().expect("NSScreen lookup must not fail or abort");
        assert!(
            !monitors.is_empty(),
            "a Mac always has at least one display"
        );
        for m in &monitors {
            assert!(m.frame.width > 0.0 && m.frame.height > 0.0);
            // The usable area is the frame minus menu bar/Dock, so it can't be larger.
            assert!(m.work_area.width <= m.frame.width + 0.5);
            assert!(m.work_area.height <= m.frame.height + 0.5);
        }
        // The primary display's top-left is the origin of the AX coordinate space.
        assert_eq!((monitors[0].frame.x, monitors[0].frame.y), (0.0, 0.0));
    }

    #[test]
    fn window_enumeration_runs_without_aborting() {
        let windows = collect_windows(None).expect("CGWindowList enumeration must not fail");
        for (info, bounds) in windows {
            assert!(info.id.starts_with("macos:"));
            assert!(bounds.width >= 50.0 && bounds.height >= 50.0);
        }
    }

    #[test]
    fn check_ax_permission_returns_permission_error_type() {
        // If accessibility is not trusted, the error variant is Permission.
        // If it IS trusted, the function succeeds — both are valid.
        let result = check_ax_permission();
        match result {
            Ok(()) => {}
            Err(AppError::Permission(_)) => {}
            Err(other) => panic!("Unexpected error type: {:?}", other),
        }
    }

    #[test]
    fn focus_window_rejects_malformed_id() {
        let err = focus_window("invalid-id").unwrap_err();
        assert!(matches!(
            err,
            AppError::Permission(_) | AppError::Validation(_)
        ));
    }

    #[test]
    fn close_window_rejects_malformed_id() {
        let err = close_window("invalid-id").unwrap_err();
        assert!(matches!(
            err,
            AppError::Permission(_) | AppError::Validation(_)
        ));
    }
}
