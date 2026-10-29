//! Window actions on top of a [`WindowBackend`], with one level of per-window undo.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::error::AppError;
use crate::window_management::backend::{work_areas, WindowBackend, WindowId};
use crate::window_management::layout::{
    calculate_preset_action, find_current_monitor_index, resize_about_center, PresetAction,
};
use crate::window_management::link::{split_rects, Axis};
use crate::window_management::types::WindowBounds;

#[derive(Debug, Clone)]
struct Saved {
    bounds: WindowBounds,
    /// The action that replaced `bounds` put the window into fullscreen.
    via_fullscreen: bool,
}

#[derive(Default)]
pub struct WindowEngine {
    history: Mutex<HashMap<WindowId, Saved>>,
}

pub fn engine() -> &'static WindowEngine {
    static ENGINE: OnceLock<WindowEngine> = OnceLock::new();
    ENGINE.get_or_init(WindowEngine::default)
}

impl WindowEngine {
    fn remember(&self, id: &WindowId, bounds: WindowBounds, via_fullscreen: bool) {
        if let Ok(mut history) = self.history.lock() {
            history.insert(
                id.clone(),
                Saved {
                    bounds,
                    via_fullscreen,
                },
            );
        }
    }

    pub fn apply_preset(
        &self,
        backend: &dyn WindowBackend,
        id: &WindowId,
        preset_id: &str,
    ) -> Result<(), AppError> {
        let current = backend.bounds(id)?;
        let areas = work_areas(backend)?;
        match calculate_preset_action(preset_id, &current, &areas)? {
            PresetAction::Noop => Ok(()),
            PresetAction::Fullscreen(enable) => {
                self.remember(id, current, enable);
                backend.set_fullscreen(id, enable)
            }
            PresetAction::Bounds(update) => {
                let target = WindowBounds {
                    x: update.x.unwrap_or(current.x),
                    y: update.y.unwrap_or(current.y),
                    width: update.width.unwrap_or(current.width),
                    height: update.height.unwrap_or(current.height),
                };
                self.remember(id, current, false);
                backend.set_bounds(id, &target)
            }
        }
    }

    /// Moves and resizes the window to `target`, remembering where it was.
    pub fn set_bounds(
        &self,
        backend: &dyn WindowBackend,
        id: &WindowId,
        target: &WindowBounds,
    ) -> Result<(), AppError> {
        let current = backend.bounds(id)?;
        self.remember(id, current, false);
        backend.set_bounds(id, target)
    }

    /// Scales the window about its center by `factor`, clamped to its display's work area.
    pub fn resize(
        &self,
        backend: &dyn WindowBackend,
        id: &WindowId,
        factor: f64,
    ) -> Result<(), AppError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(AppError::Validation(format!(
                "resize factor must be positive and finite, got {factor}"
            )));
        }
        let current = backend.bounds(id)?;
        let areas = work_areas(backend)?;
        let work_area = areas
            .get(find_current_monitor_index(&current, &areas))
            .ok_or_else(|| AppError::Platform("No display found".to_string()))?;
        let target = resize_about_center(&current, factor, work_area);
        self.remember(id, current, false);
        backend.set_bounds(id, &target)
    }

    /// Tiles `a` and `b` into the two halves of the display `a` is on and returns that display's
    /// work area, which is the span the pair then fills.
    pub fn split_pair(
        &self,
        backend: &dyn WindowBackend,
        a: &WindowId,
        b: &WindowId,
        axis: Axis,
    ) -> Result<WindowBounds, AppError> {
        if a == b {
            return Err(AppError::Validation(
                "Choose a different window to split with".to_string(),
            ));
        }
        let current = backend.bounds(a)?;
        backend.bounds(b)?;
        let monitors = backend.monitors()?;
        let frames: Vec<WindowBounds> = monitors.iter().map(|m| m.frame).collect();
        let work_area = monitors
            .get(find_current_monitor_index(&current, &frames))
            .ok_or_else(|| AppError::Platform("No display found".to_string()))?
            .work_area;
        let (first, second) = split_rects(&work_area, axis);
        self.set_bounds(backend, a, &first)?;
        self.set_bounds(backend, b, &second)?;
        Ok(work_area)
    }

    pub fn minimize(&self, backend: &dyn WindowBackend, id: &WindowId) -> Result<(), AppError> {
        backend.minimize(id)
    }

    /// Puts the window back where it was before the last preset or resize.
    pub fn restore(&self, backend: &dyn WindowBackend, id: &WindowId) -> Result<(), AppError> {
        let saved = self
            .history
            .lock()
            .map_err(|_| AppError::Lock)?
            .remove(id)
            .ok_or_else(|| AppError::NotFound("Nothing to restore".to_string()))?;
        if saved.via_fullscreen {
            backend.set_fullscreen(id, false)?;
        }
        backend.set_bounds(id, &saved.bounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_management::fake::{rect, FakeBackend};

    fn setup() -> (WindowEngine, FakeBackend, WindowId) {
        let backend = FakeBackend::single_display();
        let id = backend.add_window("w1", rect(100.0, 200.0, 800.0, 600.0));
        (WindowEngine::default(), backend, id)
    }

    #[test]
    fn preset_uses_work_area_not_full_frame() {
        let (engine, backend, id) = setup();
        engine.apply_preset(&backend, &id, "left-half").unwrap();
        // Work area starts below a 25px menu bar.
        assert_eq!(backend.window(&id).bounds, rect(0.0, 25.0, 960.0, 1055.0));
    }

    #[test]
    fn restore_returns_to_pre_preset_bounds_then_errors() {
        let (engine, backend, id) = setup();
        engine.apply_preset(&backend, &id, "right-half").unwrap();
        engine.restore(&backend, &id).unwrap();
        assert_eq!(backend.window(&id).bounds, rect(100.0, 200.0, 800.0, 600.0));
        assert!(matches!(
            engine.restore(&backend, &id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn history_is_per_window() {
        let (engine, backend, a) = setup();
        let b = backend.add_window("w2", rect(300.0, 300.0, 400.0, 400.0));
        engine.apply_preset(&backend, &a, "left-half").unwrap();
        engine.apply_preset(&backend, &b, "top-half").unwrap();
        engine.restore(&backend, &a).unwrap();
        assert_eq!(backend.window(&a).bounds, rect(100.0, 200.0, 800.0, 600.0));
        engine.restore(&backend, &b).unwrap();
        assert_eq!(backend.window(&b).bounds, rect(300.0, 300.0, 400.0, 400.0));
    }

    #[test]
    fn maximize_goes_fullscreen_and_restore_leaves_it() {
        let (engine, backend, id) = setup();
        engine.apply_preset(&backend, &id, "maximize").unwrap();
        assert!(backend.window(&id).fullscreen);
        engine.restore(&backend, &id).unwrap();
        let w = backend.window(&id);
        assert!(!w.fullscreen);
        assert_eq!(w.bounds, rect(100.0, 200.0, 800.0, 600.0));
    }

    #[test]
    fn resize_grows_about_center_and_is_undoable() {
        let (engine, backend, id) = setup();
        engine.resize(&backend, &id, 1.1).unwrap();
        let w = backend.window(&id).bounds;
        assert!((w.width - 880.0).abs() < 1e-6);
        assert!((w.height - 660.0).abs() < 1e-6);
        assert!((w.x - 60.0).abs() < 1e-6);
        assert!((w.y - 170.0).abs() < 1e-6);
        engine.restore(&backend, &id).unwrap();
        assert_eq!(backend.window(&id).bounds, rect(100.0, 200.0, 800.0, 600.0));
    }

    #[test]
    fn resize_rejects_bad_factor() {
        let (engine, backend, id) = setup();
        assert!(engine.resize(&backend, &id, 0.0).is_err());
        assert!(engine.resize(&backend, &id, f64::NAN).is_err());
    }

    #[test]
    fn set_bounds_is_undoable() {
        let (engine, backend, id) = setup();
        engine
            .set_bounds(&backend, &id, &rect(0.0, 0.0, 500.0, 500.0))
            .unwrap();
        assert_eq!(backend.window(&id).bounds, rect(0.0, 0.0, 500.0, 500.0));
        engine.restore(&backend, &id).unwrap();
        assert_eq!(backend.window(&id).bounds, rect(100.0, 200.0, 800.0, 600.0));
    }

    #[test]
    fn split_pair_tiles_both_windows_in_the_first_windows_work_area() {
        let (engine, backend, a) = setup();
        let b = backend.add_window("w2", rect(900.0, 300.0, 400.0, 400.0));
        let work = engine.split_pair(&backend, &a, &b, Axis::Side).unwrap();
        assert_eq!(work, rect(0.0, 25.0, 1920.0, 1055.0));
        assert_eq!(backend.window(&a).bounds, rect(0.0, 25.0, 960.0, 1055.0));
        assert_eq!(backend.window(&b).bounds, rect(960.0, 25.0, 960.0, 1055.0));
    }

    #[test]
    fn split_pair_stacked_uses_top_and_bottom() {
        let (engine, backend, a) = setup();
        let b = backend.add_window("w2", rect(900.0, 300.0, 400.0, 400.0));
        engine.split_pair(&backend, &a, &b, Axis::Stack).unwrap();
        assert_eq!(backend.window(&a).bounds.height, 527.5);
        assert_eq!(backend.window(&b).bounds.y, 552.5);
    }

    #[test]
    fn split_pair_is_undoable_per_window() {
        let (engine, backend, a) = setup();
        let b = backend.add_window("w2", rect(900.0, 300.0, 400.0, 400.0));
        engine.split_pair(&backend, &a, &b, Axis::Side).unwrap();
        engine.restore(&backend, &b).unwrap();
        assert_eq!(backend.window(&b).bounds, rect(900.0, 300.0, 400.0, 400.0));
    }

    #[test]
    fn split_pair_rejects_same_or_missing_window() {
        let (engine, backend, a) = setup();
        assert!(matches!(
            engine.split_pair(&backend, &a, &a, Axis::Side),
            Err(AppError::Validation(_))
        ));
        assert!(engine
            .split_pair(&backend, &a, &WindowId::new("ghost"), Axis::Side)
            .is_err());
        // Nothing moved on failure.
        assert_eq!(backend.window(&a).bounds, rect(100.0, 200.0, 800.0, 600.0));
    }

    #[test]
    fn minimize_delegates_to_backend() {
        let (engine, backend, id) = setup();
        engine.minimize(&backend, &id).unwrap();
        assert!(backend.window(&id).minimized);
    }

    #[test]
    fn unknown_window_errors() {
        let (engine, backend, _) = setup();
        let missing = WindowId::new("nope");
        assert!(engine
            .apply_preset(&backend, &missing, "left-half")
            .is_err());
    }
}
