//! Platform-neutral contract for moving, resizing and observing other apps' windows.
//!
//! Each OS implements [`WindowBackend`]; the engine, linked split and workspace logic
//! sit on top of it and are tested against `fake::FakeBackend`.

use std::sync::Arc;

use crate::error::AppError;
use crate::window_management::types::WindowBounds;

/// Opaque, backend-specific window identifier (`macos:<pid>:<wid>`, `win:<hwnd>`, `linux:<xid>`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(pub String);

impl WindowId {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A display's full frame and the part of it apps may use (excludes menu bar, Dock, taskbar, panels).
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    pub frame: WindowBounds,
    pub work_area: WindowBounds,
}

/// A visible application window, with enough identity to find it again in a later session.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowInfo {
    pub id: WindowId,
    /// Stable identity of the owning app: bundle id on macOS, executable path elsewhere.
    pub app_id: String,
    pub app_name: String,
    pub title: String,
    pub bounds: WindowBounds,
}

/// What happened to a watched window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    /// Moved or resized (including by us).
    Changed,
    /// Closed or minimized.
    Gone,
}

/// Receives events on the backend's own thread, so it must only hand them off (e.g. send on a channel).
pub type EventSink = Arc<dyn Fn(WindowEvent) + Send + Sync>;

/// Keeps a watch alive; dropping it unsubscribes.
pub struct WatchGuard(#[allow(dead_code)] Box<dyn Send>);

impl WatchGuard {
    pub fn new(inner: impl Send + 'static) -> Self {
        Self(Box::new(inner))
    }
}

pub trait WindowBackend: Send + Sync {
    fn bounds(&self, id: &WindowId) -> Result<WindowBounds, AppError>;

    /// Moves and resizes the window to exactly `bounds` (visible bounds, not including
    /// invisible resize borders).
    fn set_bounds(&self, id: &WindowId, bounds: &WindowBounds) -> Result<(), AppError>;

    fn set_fullscreen(&self, id: &WindowId, enable: bool) -> Result<(), AppError>;

    fn minimize(&self, id: &WindowId) -> Result<(), AppError>;

    /// Displays in the OS coordinate space used by [`WindowBackend::bounds`].
    fn monitors(&self) -> Result<Vec<MonitorInfo>, AppError>;

    /// Brings the window to the front, restoring it if minimized.
    fn focus(&self, id: &WindowId) -> Result<(), AppError>;

    /// Visible (not minimized) application windows, frontmost first, excluding Conbind's own.
    fn windows(&self) -> Result<Vec<WindowInfo>, AppError>;

    /// Starts the app identified by `app_id` (as reported in [`WindowInfo::app_id`]).
    fn launch(&self, app_id: &str, app_name: &str) -> Result<(), AppError>;

    /// Reports every move, resize, close and minimize of `id` to `sink` until the guard is dropped.
    fn watch(&self, id: &WindowId, sink: EventSink) -> Result<WatchGuard, AppError>;
}

/// Work areas of every display, in the order the backend reports them.
pub fn work_areas(backend: &dyn WindowBackend) -> Result<Vec<WindowBounds>, AppError> {
    Ok(backend
        .monitors()?
        .into_iter()
        .map(|m| m.work_area)
        .collect())
}

/// Stand-in on platforms without a window management backend.
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub struct UnsupportedBackend;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
impl UnsupportedBackend {
    fn unsupported<T>() -> Result<T, AppError> {
        Err(AppError::Platform(
            "Window management is not supported on this platform.".to_string(),
        ))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
impl WindowBackend for UnsupportedBackend {
    fn bounds(&self, _id: &WindowId) -> Result<WindowBounds, AppError> {
        Self::unsupported()
    }

    fn set_bounds(&self, _id: &WindowId, _bounds: &WindowBounds) -> Result<(), AppError> {
        Self::unsupported()
    }

    fn set_fullscreen(&self, _id: &WindowId, _enable: bool) -> Result<(), AppError> {
        Self::unsupported()
    }

    fn minimize(&self, _id: &WindowId) -> Result<(), AppError> {
        Self::unsupported()
    }

    fn monitors(&self) -> Result<Vec<MonitorInfo>, AppError> {
        Self::unsupported()
    }

    fn focus(&self, _id: &WindowId) -> Result<(), AppError> {
        Self::unsupported()
    }

    fn windows(&self) -> Result<Vec<WindowInfo>, AppError> {
        Self::unsupported()
    }

    fn launch(&self, _app_id: &str, _app_name: &str) -> Result<(), AppError> {
        Self::unsupported()
    }

    fn watch(&self, _id: &WindowId, _sink: EventSink) -> Result<WatchGuard, AppError> {
        Self::unsupported()
    }
}
