//! Workspaces: 1 to 4 windows with their geometry, captured from and restored to the screen.
//!
//! This module is stateless; the frontend keeps the saved workspaces in extension storage and
//! hands them back to [`open`].

use std::thread::sleep;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::window_management::backend::{MonitorInfo, WindowBackend, WindowId, WindowInfo};
use crate::window_management::engine::WindowEngine;
use crate::window_management::layout::{find_current_monitor_index, sort_monitors};
use crate::window_management::types::WindowBounds;

pub const MAX_WINDOWS: usize = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceWindow {
    pub app_id: String,
    pub app_name: String,
    pub title_hint: String,
    /// Index into the displays ordered left to right, then top to bottom.
    pub monitor: usize,
    /// Position and size as fractions of that display's work area.
    pub rect: WindowBounds,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Captured {
    pub windows: Vec<WorkspaceWindow>,
    /// Indices into `windows` of two windows that were linked when captured.
    pub link: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenReport {
    pub placed: usize,
    pub launched: usize,
    /// Apps whose window never showed up.
    pub missing: Vec<String>,
    /// Window ids in workspace order, for restoring a link.
    pub ids: Vec<Option<String>>,
}

pub struct OpenOptions {
    pub attempts: u32,
    pub interval: Duration,
}

impl Default for OpenOptions {
    fn default() -> Self {
        // About 10 seconds per app.
        Self {
            attempts: 40,
            interval: Duration::from_millis(250),
        }
    }
}

pub fn to_fractions(bounds: &WindowBounds, work_area: &WindowBounds) -> WindowBounds {
    WindowBounds {
        x: (bounds.x - work_area.x) / work_area.width,
        y: (bounds.y - work_area.y) / work_area.height,
        width: bounds.width / work_area.width,
        height: bounds.height / work_area.height,
    }
}

pub fn from_fractions(fractions: &WindowBounds, work_area: &WindowBounds) -> WindowBounds {
    WindowBounds {
        x: work_area.x + fractions.x * work_area.width,
        y: work_area.y + fractions.y * work_area.height,
        width: fractions.width * work_area.width,
        height: fractions.height * work_area.height,
    }
}

/// Displays in a stable left-to-right, top-to-bottom order, so indices survive restarts.
fn ordered_monitors(backend: &dyn WindowBackend) -> Result<Vec<MonitorInfo>, AppError> {
    let mut monitors = backend.monitors()?;
    let order: Vec<WindowBounds> =
        sort_monitors(&monitors.iter().map(|m| m.frame).collect::<Vec<_>>());
    monitors.sort_by_key(|m| {
        order
            .iter()
            .position(|f| *f == m.frame)
            .unwrap_or(usize::MAX)
    });
    Ok(monitors)
}

fn monitor_index(monitors: &[MonitorInfo], bounds: &WindowBounds) -> usize {
    let frames: Vec<WindowBounds> = monitors.iter().map(|m| m.frame).collect();
    find_current_monitor_index(bounds, &frames)
}

/// Snapshots windows into workspace entries. With `ids` the caller chose the windows; otherwise
/// it is the target plus the frontmost others on the target's display, up to [`MAX_WINDOWS`].
pub fn capture(
    backend: &dyn WindowBackend,
    target: &WindowId,
    ids: Option<&[WindowId]>,
    link: Option<(WindowId, WindowId)>,
) -> Result<Captured, AppError> {
    let monitors = ordered_monitors(backend)?;
    if monitors.is_empty() {
        return Err(AppError::Platform("No display found".to_string()));
    }
    let all = backend.windows()?;
    let target_info = all
        .iter()
        .find(|w| &w.id == target)
        .ok_or_else(|| AppError::NotFound("The frontmost window can't be saved".to_string()))?;
    let display = monitor_index(&monitors, &target_info.bounds);

    let on_display: Vec<&WindowInfo> = all
        .iter()
        .filter(|w| monitor_index(&monitors, &w.bounds) == display)
        .collect();

    let chosen: Vec<&WindowInfo> = match ids {
        Some(ids) => ids
            .iter()
            .filter_map(|id| on_display.iter().find(|w| &w.id == id).copied())
            .collect(),
        None => std::iter::once(target_info)
            .chain(on_display.iter().copied().filter(|w| &w.id != target))
            .collect(),
    };
    let chosen: Vec<&WindowInfo> = chosen.into_iter().take(MAX_WINDOWS).collect();
    if chosen.is_empty() {
        return Err(AppError::NotFound("No windows to save".to_string()));
    }

    let windows = chosen
        .iter()
        .map(|w| {
            let index = monitor_index(&monitors, &w.bounds);
            WorkspaceWindow {
                app_id: w.app_id.clone(),
                app_name: w.app_name.clone(),
                title_hint: w.title.clone(),
                monitor: index,
                rect: to_fractions(&w.bounds, &monitors[index].work_area),
            }
        })
        .collect();

    let link = link.and_then(|(a, b)| {
        let ia = chosen.iter().position(|w| w.id == a)?;
        let ib = chosen.iter().position(|w| w.id == b)?;
        Some((ia, ib))
    });
    Ok(Captured { windows, link })
}

/// The running window for `entry`: same app, preferring an exact title match, then a title
/// containing the hint, then any unused window of that app.
fn pick_window<'a>(
    running: &'a [WindowInfo],
    entry: &WorkspaceWindow,
    used: &[WindowId],
) -> Option<&'a WindowInfo> {
    let free: Vec<&WindowInfo> = running
        .iter()
        .filter(|w| w.app_id == entry.app_id && !used.contains(&w.id))
        .collect();
    let hint = entry.title_hint.to_lowercase();
    free.iter()
        .find(|w| !hint.is_empty() && w.title.to_lowercase() == hint)
        .or_else(|| {
            free.iter()
                .find(|w| !hint.is_empty() && w.title.to_lowercase().contains(&hint))
        })
        .or_else(|| free.first())
        .copied()
}

/// Arranges a workspace: reuses running windows, launches missing apps, places everything and
/// raises the windows back to front so the first one ends up on top.
pub fn open(
    backend: &dyn WindowBackend,
    engine: &WindowEngine,
    windows: &[WorkspaceWindow],
    opts: &OpenOptions,
) -> Result<OpenReport, AppError> {
    let monitors = ordered_monitors(backend)?;
    if monitors.is_empty() {
        return Err(AppError::Platform("No display found".to_string()));
    }
    let mut report = OpenReport::default();

    // Launch everything that is missing first so the apps start up in parallel.
    let running = backend.windows().unwrap_or_default();
    let mut used: Vec<WindowId> = Vec::new();
    let mut launched_apps: Vec<&str> = Vec::new();
    for entry in windows {
        match pick_window(&running, entry, &used) {
            Some(found) => used.push(found.id.clone()),
            None => {
                if !launched_apps.contains(&entry.app_id.as_str()) {
                    if backend.launch(&entry.app_id, &entry.app_name).is_ok() {
                        report.launched += 1;
                    }
                    launched_apps.push(&entry.app_id);
                }
            }
        }
    }

    let mut used: Vec<WindowId> = Vec::new();
    for entry in windows {
        let mut found: Option<WindowInfo> = None;
        for attempt in 0..=opts.attempts {
            let current = backend.windows().unwrap_or_default();
            found = pick_window(&current, entry, &used).cloned();
            if found.is_some() || attempt == opts.attempts {
                break;
            }
            sleep(opts.interval);
        }
        match found {
            Some(window) => {
                let monitor = monitors.get(entry.monitor).unwrap_or(&monitors[0]);
                let target = from_fractions(&entry.rect, &monitor.work_area);
                if engine.set_bounds(backend, &window.id, &target).is_ok() {
                    report.placed += 1;
                }
                report.ids.push(Some(window.id.as_str().to_string()));
                used.push(window.id);
            }
            None => {
                report.missing.push(entry.app_name.clone());
                report.ids.push(None);
            }
        }
    }

    for id in report.ids.iter().rev().flatten() {
        let _ = backend.focus(&WindowId::new(id.as_str()));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_management::backend::MonitorInfo;
    use crate::window_management::fake::{rect, FakeBackend};

    fn work() -> WindowBounds {
        rect(0.0, 25.0, 1200.0, 800.0)
    }

    fn fast() -> OpenOptions {
        OpenOptions {
            attempts: 2,
            interval: Duration::from_millis(0),
        }
    }

    /// Notes (left half), Slack (top right), Safari (bottom right); Safari is frontmost.
    fn backend() -> (FakeBackend, WindowId, WindowId, WindowId) {
        let b = FakeBackend::with_work_area(work());
        let notes = b.add_app_window(
            "1",
            "com.test.Notes",
            "Notes",
            "Notes",
            rect(0.0, 25.0, 600.0, 800.0),
        );
        let slack = b.add_app_window(
            "2",
            "com.test.Slack",
            "Slack",
            "Slack",
            rect(600.0, 25.0, 600.0, 400.0),
        );
        let safari = b.add_app_window(
            "3",
            "com.test.Safari",
            "Safari",
            "Safari",
            rect(600.0, 425.0, 600.0, 400.0),
        );
        (b, notes, slack, safari)
    }

    #[test]
    fn fractions_round_trip() {
        let bounds = rect(300.0, 225.0, 600.0, 400.0);
        let f = to_fractions(&bounds, &work());
        assert_eq!(f, rect(0.25, 0.25, 0.5, 0.5));
        assert_eq!(from_fractions(&f, &work()), bounds);
    }

    #[test]
    fn capture_puts_target_first_and_stores_fractions() {
        let (b, _, slack, _) = backend();
        let captured = capture(&b, &slack, None, None).unwrap();
        assert_eq!(captured.windows[0].app_name, "Slack");
        assert_eq!(captured.windows.len(), 3);
        assert_eq!(captured.windows[0].rect, rect(0.5, 0.0, 0.5, 0.5));
        assert_eq!(captured.link, None);
    }

    #[test]
    fn capture_limits_to_four_windows() {
        let (b, notes, _, _) = backend();
        b.add_window("4", rect(0.0, 25.0, 100.0, 100.0));
        b.add_window("5", rect(0.0, 25.0, 100.0, 100.0));
        let captured = capture(&b, &notes, None, None).unwrap();
        assert_eq!(captured.windows.len(), MAX_WINDOWS);
    }

    #[test]
    fn capture_respects_explicit_selection_and_remaps_the_link() {
        let (b, notes, _, safari) = backend();
        let captured = capture(
            &b,
            &notes,
            Some(&[safari.clone(), notes.clone()]),
            Some((notes.clone(), safari.clone())),
        )
        .unwrap();
        let apps: Vec<&str> = captured
            .windows
            .iter()
            .map(|w| w.app_name.as_str())
            .collect();
        assert_eq!(apps, ["Safari", "Notes"]);
        assert_eq!(captured.link, Some((1, 0)));
    }

    #[test]
    fn capture_ignores_a_link_to_a_window_that_was_not_saved() {
        let (b, notes, slack, safari) = backend();
        let captured = capture(
            &b,
            &notes,
            Some(&[notes.clone(), slack]),
            Some((notes.clone(), safari)),
        )
        .unwrap();
        assert_eq!(captured.link, None);
    }

    #[test]
    fn capture_only_takes_windows_on_the_targets_display() {
        let b = FakeBackend::new(vec![
            MonitorInfo {
                frame: rect(0.0, 0.0, 1000.0, 800.0),
                work_area: rect(0.0, 0.0, 1000.0, 800.0),
            },
            MonitorInfo {
                frame: rect(1000.0, 0.0, 1000.0, 800.0),
                work_area: rect(1000.0, 0.0, 1000.0, 800.0),
            },
        ]);
        let left = b.add_app_window("1", "a", "A", "A", rect(0.0, 0.0, 500.0, 800.0));
        b.add_app_window("2", "b", "B", "B", rect(1000.0, 0.0, 500.0, 800.0));
        let captured = capture(&b, &left, None, None).unwrap();
        assert_eq!(captured.windows.len(), 1);
        assert_eq!(captured.windows[0].monitor, 0);
    }

    #[test]
    fn capture_fails_when_the_target_is_not_a_listed_window() {
        let (b, ..) = backend();
        assert!(matches!(
            capture(&b, &WindowId::new("ghost"), None, None),
            Err(AppError::NotFound(_))
        ));
    }

    fn saved(b: &FakeBackend, target: &WindowId) -> Vec<WorkspaceWindow> {
        capture(b, target, None, None).unwrap().windows
    }

    #[test]
    fn open_places_running_windows_and_raises_the_first() {
        let (b, notes, slack, _) = backend();
        let windows = saved(&b, &notes);
        b.user_sets(&notes, rect(10.0, 40.0, 300.0, 300.0));
        b.user_sets(&slack, rect(10.0, 40.0, 300.0, 300.0));
        let report = open(&b, &WindowEngine::default(), &windows, &fast()).unwrap();
        assert_eq!(report.placed, 3);
        assert_eq!(report.launched, 0);
        assert_eq!(b.bounds_of(&notes), rect(0.0, 25.0, 600.0, 800.0));
        assert_eq!(b.bounds_of(&slack), rect(600.0, 25.0, 600.0, 400.0));
        assert_eq!(b.frontmost(), Some(notes));
    }

    #[test]
    fn open_is_undoable_per_window() {
        let (b, notes, ..) = backend();
        let windows = saved(&b, &notes);
        b.user_sets(&notes, rect(10.0, 40.0, 300.0, 300.0));
        let engine = WindowEngine::default();
        open(&b, &engine, &windows, &fast()).unwrap();
        engine.restore(&b, &notes).unwrap();
        assert_eq!(b.bounds_of(&notes), rect(10.0, 40.0, 300.0, 300.0));
    }

    #[test]
    fn open_launches_missing_apps_and_reports_ones_that_never_appear() {
        let (b, notes, ..) = backend();
        let mut windows = saved(&b, &notes);
        windows.push(WorkspaceWindow {
            app_id: "com.test.Ghost".into(),
            app_name: "Ghost".into(),
            title_hint: String::new(),
            monitor: 0,
            rect: rect(0.0, 0.0, 0.5, 0.5),
        });
        let report = open(&b, &WindowEngine::default(), &windows, &fast()).unwrap();
        assert_eq!(report.launched, 1);
        assert_eq!(report.missing, vec!["Ghost".to_string()]);
        assert_eq!(report.ids.last(), Some(&None));
        assert_eq!(b.launched.lock().unwrap().as_slice(), ["com.test.Ghost"]);
    }

    #[test]
    fn an_app_is_launched_once_even_if_two_windows_are_missing() {
        let (b, ..) = backend();
        let entry = |title: &str| WorkspaceWindow {
            app_id: "com.test.Ghost".into(),
            app_name: "Ghost".into(),
            title_hint: title.into(),
            monitor: 0,
            rect: rect(0.0, 0.0, 0.5, 0.5),
        };
        let report = open(
            &b,
            &WindowEngine::default(),
            &[entry("one"), entry("two")],
            &fast(),
        )
        .unwrap();
        assert_eq!(report.launched, 1);
        assert_eq!(report.missing.len(), 2);
    }

    #[test]
    fn two_windows_of_one_app_are_matched_by_title() {
        let b = FakeBackend::with_work_area(work());
        let docs = b.add_app_window(
            "1",
            "com.test.Safari",
            "Safari",
            "Docs",
            rect(0.0, 25.0, 100.0, 100.0),
        );
        let mail = b.add_app_window(
            "2",
            "com.test.Safari",
            "Safari",
            "Mail",
            rect(0.0, 25.0, 100.0, 100.0),
        );
        let entry = |title: &str, x: f64| WorkspaceWindow {
            app_id: "com.test.Safari".into(),
            app_name: "Safari".into(),
            title_hint: title.into(),
            monitor: 0,
            rect: rect(x, 0.0, 0.5, 1.0),
        };
        open(
            &b,
            &WindowEngine::default(),
            &[entry("Mail", 0.0), entry("Docs", 0.5)],
            &fast(),
        )
        .unwrap();
        assert_eq!(b.bounds_of(&mail).x, 0.0);
        assert_eq!(b.bounds_of(&docs).x, 600.0);
    }

    #[test]
    fn a_partial_title_still_matches() {
        let b = FakeBackend::with_work_area(work());
        let doc = b.add_app_window(
            "1",
            "app",
            "App",
            "Report - Draft 3",
            rect(0.0, 25.0, 100.0, 100.0),
        );
        let entries = [WorkspaceWindow {
            app_id: "app".into(),
            app_name: "App".into(),
            title_hint: "report".into(),
            monitor: 0,
            rect: rect(0.0, 0.0, 0.5, 1.0),
        }];
        let report = open(&b, &WindowEngine::default(), &entries, &fast()).unwrap();
        assert_eq!(report.placed, 1);
        assert_eq!(b.bounds_of(&doc).width, 600.0);
    }

    #[test]
    fn a_missing_monitor_falls_back_to_the_first() {
        let (b, notes, ..) = backend();
        let entries = [WorkspaceWindow {
            app_id: "com.test.Notes".into(),
            app_name: "Notes".into(),
            title_hint: String::new(),
            monitor: 7,
            rect: rect(0.0, 0.0, 0.5, 1.0),
        }];
        let report = open(&b, &WindowEngine::default(), &entries, &fast()).unwrap();
        assert_eq!(report.placed, 1);
        assert_eq!(b.bounds_of(&notes), rect(0.0, 25.0, 600.0, 800.0));
    }

    #[test]
    fn displays_are_indexed_left_to_right_regardless_of_report_order() {
        let right = MonitorInfo {
            frame: rect(1000.0, 0.0, 1000.0, 800.0),
            work_area: rect(1000.0, 0.0, 1000.0, 800.0),
        };
        let left = MonitorInfo {
            frame: rect(0.0, 0.0, 1000.0, 800.0),
            work_area: rect(0.0, 0.0, 1000.0, 800.0),
        };
        let b = FakeBackend::new(vec![right, left]);
        let on_right = b.add_app_window("1", "a", "A", "A", rect(1000.0, 0.0, 500.0, 800.0));
        let captured = capture(&b, &on_right, None, None).unwrap();
        assert_eq!(captured.windows[0].monitor, 1);
        assert_eq!(captured.windows[0].rect, rect(0.0, 0.0, 0.5, 1.0));
    }
}
