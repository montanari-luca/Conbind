//! In-memory [`WindowBackend`] used by engine, link and workspace tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::AppError;
use crate::window_management::backend::{
    EventSink, MonitorInfo, WatchGuard, WindowBackend, WindowEvent, WindowId, WindowInfo,
};
use crate::window_management::types::WindowBounds;

#[derive(Debug, Clone)]
pub struct FakeWindow {
    pub bounds: WindowBounds,
    pub minimized: bool,
    pub fullscreen: bool,
    pub app_id: String,
    pub app_name: String,
    pub title: String,
}

type Watchers = Arc<Mutex<Vec<(u64, WindowId, EventSink)>>>;

pub struct FakeBackend {
    windows: Mutex<HashMap<WindowId, FakeWindow>>,
    /// Frontmost first.
    order: Mutex<Vec<WindowId>>,
    monitors: Vec<MonitorInfo>,
    watchers: Watchers,
    next_token: Mutex<u64>,
    /// Smallest (width, height) any window accepts, like an app with a minimum size.
    pub min_size: Mutex<(f64, f64)>,
    /// `app_id`s passed to `launch`, in order.
    pub launched: Mutex<Vec<String>>,
}

pub fn rect(x: f64, y: f64, width: f64, height: f64) -> WindowBounds {
    WindowBounds {
        x,
        y,
        width,
        height,
    }
}

struct FakeWatch {
    watchers: Watchers,
    token: u64,
}

impl Drop for FakeWatch {
    fn drop(&mut self) {
        self.watchers
            .lock()
            .unwrap()
            .retain(|(token, _, _)| *token != self.token);
    }
}

impl FakeBackend {
    pub fn new(monitors: Vec<MonitorInfo>) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            order: Mutex::new(Vec::new()),
            monitors,
            watchers: Arc::new(Mutex::new(Vec::new())),
            next_token: Mutex::new(0),
            min_size: Mutex::new((0.0, 0.0)),
            launched: Mutex::new(Vec::new()),
        }
    }

    /// One 1920x1080 display whose top 25px (a menu bar) is not usable.
    pub fn single_display() -> Self {
        Self::new(vec![MonitorInfo {
            frame: rect(0.0, 0.0, 1920.0, 1080.0),
            work_area: rect(0.0, 25.0, 1920.0, 1055.0),
        }])
    }

    /// One display whose usable area is exactly `work`.
    pub fn with_work_area(work: WindowBounds) -> Self {
        Self::new(vec![MonitorInfo {
            frame: work,
            work_area: work,
        }])
    }

    /// Adds a window in front of the existing ones. Its app is `app.<id>`.
    pub fn add_window(&self, id: &str, bounds: WindowBounds) -> WindowId {
        self.add_app_window(id, &format!("app.{id}"), id, id, bounds)
    }

    pub fn add_app_window(
        &self,
        id: &str,
        app_id: &str,
        app_name: &str,
        title: &str,
        bounds: WindowBounds,
    ) -> WindowId {
        let wid = WindowId::new(id);
        self.windows.lock().unwrap().insert(
            wid.clone(),
            FakeWindow {
                bounds,
                minimized: false,
                fullscreen: false,
                app_id: app_id.to_string(),
                app_name: app_name.to_string(),
                title: title.to_string(),
            },
        );
        self.order.lock().unwrap().insert(0, wid.clone());
        wid
    }

    pub fn window(&self, id: &WindowId) -> FakeWindow {
        self.windows.lock().unwrap().get(id).cloned().unwrap()
    }

    pub fn bounds_of(&self, id: &WindowId) -> WindowBounds {
        self.window(id).bounds
    }

    /// The window currently in front.
    pub fn frontmost(&self) -> Option<WindowId> {
        self.order.lock().unwrap().first().cloned()
    }

    /// The user dragging or resizing a window: applies the change and notifies watchers.
    pub fn user_sets(&self, id: &WindowId, bounds: WindowBounds) {
        self.apply_bounds(id, &bounds).unwrap();
    }

    /// The window is closed.
    pub fn close(&self, id: &WindowId) {
        self.windows.lock().unwrap().remove(id);
        self.order.lock().unwrap().retain(|w| w != id);
        self.notify(id, WindowEvent::Gone);
    }

    fn notify(&self, id: &WindowId, event: WindowEvent) {
        let sinks: Vec<EventSink> = self
            .watchers
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, watched, _)| watched == id)
            .map(|(_, _, sink)| sink.clone())
            .collect();
        for sink in sinks {
            sink(event);
        }
    }

    fn apply_bounds(&self, id: &WindowId, bounds: &WindowBounds) -> Result<(), AppError> {
        let (min_w, min_h) = *self.min_size.lock().unwrap();
        self.with(id, |w| {
            w.bounds = WindowBounds {
                width: bounds.width.max(min_w),
                height: bounds.height.max(min_h),
                ..*bounds
            }
        })?;
        self.notify(id, WindowEvent::Changed);
        Ok(())
    }

    fn with<T>(&self, id: &WindowId, f: impl FnOnce(&mut FakeWindow) -> T) -> Result<T, AppError> {
        let mut map = self.windows.lock().map_err(|_| AppError::Lock)?;
        map.get_mut(id)
            .map(f)
            .ok_or_else(|| AppError::NotFound(format!("window {}", id.as_str())))
    }
}

impl WindowBackend for FakeBackend {
    fn bounds(&self, id: &WindowId) -> Result<WindowBounds, AppError> {
        self.with(id, |w| w.bounds)
    }

    /// Like a real OS, our own moves are reported back to watchers too.
    fn set_bounds(&self, id: &WindowId, bounds: &WindowBounds) -> Result<(), AppError> {
        self.apply_bounds(id, bounds)
    }

    fn set_fullscreen(&self, id: &WindowId, enable: bool) -> Result<(), AppError> {
        self.with(id, |w| w.fullscreen = enable)
    }

    fn minimize(&self, id: &WindowId) -> Result<(), AppError> {
        self.with(id, |w| w.minimized = true)?;
        self.notify(id, WindowEvent::Gone);
        Ok(())
    }

    fn monitors(&self) -> Result<Vec<MonitorInfo>, AppError> {
        Ok(self.monitors.clone())
    }

    fn focus(&self, id: &WindowId) -> Result<(), AppError> {
        self.with(id, |w| w.minimized = false)?;
        let mut order = self.order.lock().unwrap();
        order.retain(|w| w != id);
        order.insert(0, id.clone());
        Ok(())
    }

    fn windows(&self) -> Result<Vec<WindowInfo>, AppError> {
        let map = self.windows.lock().unwrap();
        Ok(self
            .order
            .lock()
            .unwrap()
            .iter()
            .filter_map(|id| map.get(id).map(|w| (id, w)))
            .filter(|(_, w)| !w.minimized)
            .map(|(id, w)| WindowInfo {
                id: id.clone(),
                app_id: w.app_id.clone(),
                app_name: w.app_name.clone(),
                title: w.title.clone(),
                bounds: w.bounds,
            })
            .collect())
    }

    fn launch(&self, app_id: &str, _app_name: &str) -> Result<(), AppError> {
        self.launched.lock().unwrap().push(app_id.to_string());
        Ok(())
    }

    fn watch(&self, id: &WindowId, sink: EventSink) -> Result<WatchGuard, AppError> {
        self.with(id, |_| ())?;
        let mut next = self.next_token.lock().unwrap();
        *next += 1;
        let token = *next;
        self.watchers
            .lock()
            .unwrap()
            .push((token, id.clone(), sink));
        Ok(WatchGuard::new(FakeWatch {
            watchers: self.watchers.clone(),
            token,
        }))
    }
}
