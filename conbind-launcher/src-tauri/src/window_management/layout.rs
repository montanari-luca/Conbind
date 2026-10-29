//! Multi-display window positioning and layout calculations.
//!
//! Provides pure, testable geometry and layout calculations for moving windows
//! across monitors and applying layout presets.

use crate::error::AppError;
use crate::window_management::types::{WindowBounds, WindowBoundsUpdate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayDirection {
    Next,
    Previous,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PresetAction {
    Fullscreen(bool),
    Bounds(WindowBoundsUpdate),
    Noop,
}

/// Sorts monitors by physical/logical coordinates: primary x ascending, secondary y ascending.
pub fn sort_monitors(monitors: &[WindowBounds]) -> Vec<WindowBounds> {
    let mut sorted = monitors.to_vec();
    sorted.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    sorted
}

/// Finds the index in `monitors` that contains the largest area of `current_bounds`.
/// If no overlap occurs, returns the index of the monitor whose center is closest.
pub fn find_current_monitor_index(
    current_bounds: &WindowBounds,
    monitors: &[WindowBounds],
) -> usize {
    if monitors.is_empty() {
        return 0;
    }

    let mut current_idx = 0;
    let mut max_overlap = -1.0;

    for (i, m) in monitors.iter().enumerate() {
        let overlap_x = (current_bounds.x + current_bounds.width).min(m.x + m.width)
            - current_bounds.x.max(m.x);
        let overlap_y = (current_bounds.y + current_bounds.height).min(m.y + m.height)
            - current_bounds.y.max(m.y);
        let overlap = overlap_x.max(0.0) * overlap_y.max(0.0);

        if overlap > max_overlap {
            max_overlap = overlap;
            current_idx = i;
        }
    }

    if max_overlap <= 0.0 {
        let wcx = current_bounds.x + current_bounds.width / 2.0;
        let wcy = current_bounds.y + current_bounds.height / 2.0;
        let mut min_dist = f64::INFINITY;
        for (i, m) in monitors.iter().enumerate() {
            let mcx = m.x + m.width / 2.0;
            let mcy = m.y + m.height / 2.0;
            let dist = ((wcx - mcx).powi(2) + (wcy - mcy).powi(2)).sqrt();
            if dist < min_dist {
                min_dist = dist;
                current_idx = i;
            }
        }
    }

    current_idx
}

/// Computes the new bounds for moving `current_bounds` to the next/previous display.
///
/// Returns `None` if fewer than two monitors are available (single-display no-op).
pub fn calculate_target_display_bounds(
    current_bounds: &WindowBounds,
    monitors: &[WindowBounds],
    direction: DisplayDirection,
) -> Option<WindowBoundsUpdate> {
    if monitors.len() <= 1 {
        return None;
    }

    let sorted = sort_monitors(monitors);
    let current_idx = find_current_monitor_index(current_bounds, &sorted);

    let target_idx = match direction {
        DisplayDirection::Next => (current_idx + 1) % sorted.len(),
        DisplayDirection::Previous => (current_idx + sorted.len() - 1) % sorted.len(),
    };

    let src = &sorted[current_idx];
    let dst = &sorted[target_idx];

    let src_w = if src.width > 0.0 { src.width } else { 1920.0 };
    let src_h = if src.height > 0.0 { src.height } else { 1080.0 };

    // Relative offset and dimensions on source monitor
    let rel_x = (current_bounds.x - src.x) / src_w;
    let rel_y = (current_bounds.y - src.y) / src_h;
    let rel_w = (current_bounds.width / src_w).clamp(0.0, 1.0);
    let rel_h = (current_bounds.height / src_h).clamp(0.0, 1.0);

    // Apply relative size to the destination monitor
    let new_w = (rel_w * dst.width).min(dst.width).max(50.0);
    let new_h = (rel_h * dst.height).min(dst.height).max(50.0);

    // Clamp coordinates so the window is kept within the destination screen bounds
    let max_x = (dst.x + dst.width - new_w).max(dst.x);
    let max_y = (dst.y + dst.height - new_h).max(dst.y);

    let new_x = (dst.x + rel_x * dst.width).clamp(dst.x, max_x);
    let new_y = (dst.y + rel_y * dst.height).clamp(dst.y, max_y);

    Some(WindowBoundsUpdate {
        x: Some(new_x),
        y: Some(new_y),
        width: Some(new_w),
        height: Some(new_h),
    })
}

/// Computes the action to take for a layout preset ID.
pub fn calculate_preset_action(
    preset_id: &str,
    current_bounds: &WindowBounds,
    monitors: &[WindowBounds],
) -> Result<PresetAction, AppError> {
    if preset_id == "maximize" {
        return Ok(PresetAction::Fullscreen(true));
    }

    if preset_id == "next-display" || preset_id == "previous-display" {
        let direction = if preset_id == "next-display" {
            DisplayDirection::Next
        } else {
            DisplayDirection::Previous
        };
        return match calculate_target_display_bounds(current_bounds, monitors, direction) {
            Some(bounds) => Ok(PresetAction::Bounds(bounds)),
            None => Ok(PresetAction::Noop),
        };
    }

    let sorted = sort_monitors(monitors);
    let (mx, my, mw, mh) = if !sorted.is_empty() {
        let idx = find_current_monitor_index(current_bounds, &sorted);
        let m = &sorted[idx];
        (m.x, m.y, m.width, m.height)
    } else {
        (0.0, 0.0, 1920.0, 1080.0)
    };

    let (x, y, w, h) = match preset_id {
        "left-half" => (mx, my, mw / 2.0, mh),
        "right-half" => (mx + mw / 2.0, my, mw / 2.0, mh),
        "top-half" => (mx, my, mw, mh / 2.0),
        "bottom-half" => (mx, my + mh / 2.0, mw, mh / 2.0),
        "top-left-quarter" => (mx, my, mw / 2.0, mh / 2.0),
        "top-right-quarter" => (mx + mw / 2.0, my, mw / 2.0, mh / 2.0),
        "bottom-left-quarter" => (mx, my + mh / 2.0, mw / 2.0, mh / 2.0),
        "bottom-right-quarter" => (mx + mw / 2.0, my + mh / 2.0, mw / 2.0, mh / 2.0),
        "left-third" => (mx, my, mw / 3.0, mh),
        "center-third" => (mx + mw / 3.0, my, mw / 3.0, mh),
        "right-third" => (mx + (mw / 3.0) * 2.0, my, mw / 3.0, mh),
        "left-two-thirds" => (mx, my, (mw / 3.0) * 2.0, mh),
        "right-two-thirds" => (mx + mw / 3.0, my, (mw / 3.0) * 2.0, mh),
        "first-fourth" => (mx, my, mw / 4.0, mh),
        "second-fourth" => (mx + mw / 4.0, my, mw / 4.0, mh),
        "third-fourth" => (mx + (mw / 4.0) * 2.0, my, mw / 4.0, mh),
        "last-fourth" => (mx + (mw / 4.0) * 3.0, my, mw / 4.0, mh),
        "center" => (mx + mw * 0.1, my + mh * 0.1, mw * 0.8, mh * 0.8),
        "almost-maximize" => (mx + mw * 0.05, my + mh * 0.05, mw * 0.9, mh * 0.9),
        _ => {
            return Err(AppError::Validation(format!(
                "Unknown preset ID: {preset_id}"
            )))
        }
    };

    Ok(PresetAction::Bounds(WindowBoundsUpdate {
        x: Some(x),
        y: Some(y),
        width: Some(w),
        height: Some(h),
    }))
}

const MIN_RESIZE_WIDTH: f64 = 120.0;
const MIN_RESIZE_HEIGHT: f64 = 80.0;

/// Scales `window` by `factor` about its center, keeping it inside `work_area`.
/// The result is never smaller than 120x80 (unless the work area itself is) nor larger than the work area.
pub fn resize_about_center(
    window: &WindowBounds,
    factor: f64,
    work_area: &WindowBounds,
) -> WindowBounds {
    let width = (window.width * factor)
        .max(MIN_RESIZE_WIDTH)
        .min(work_area.width);
    let height = (window.height * factor)
        .max(MIN_RESIZE_HEIGHT)
        .min(work_area.height);
    let center_x = window.x + window.width / 2.0;
    let center_y = window.y + window.height / 2.0;
    let max_x = (work_area.x + work_area.width - width).max(work_area.x);
    let max_y = (work_area.y + work_area.height - height).max(work_area.y);
    WindowBounds {
        x: (center_x - width / 2.0).clamp(work_area.x, max_x),
        y: (center_y - height / 2.0).clamp(work_area.y, max_y),
        width,
        height,
    }
}

/// Converts the `target` *visible* rect into the outer rect to hand to the OS, for windows
/// whose frame has invisible borders (`visible` is inside `outer`). Windows 10/11 draw an
/// invisible resize border, so placing the outer rect directly leaves visible gaps.
pub fn visible_to_outer(
    target: &WindowBounds,
    visible: &WindowBounds,
    outer: &WindowBounds,
) -> WindowBounds {
    let left = visible.x - outer.x;
    let top = visible.y - outer.y;
    let right = (outer.x + outer.width) - (visible.x + visible.width);
    let bottom = (outer.y + outer.height) - (visible.y + visible.height);
    WindowBounds {
        x: target.x - left,
        y: target.y - top,
        width: target.width + left + right,
        height: target.height + top + bottom,
    }
}

/// Screen space reserved by a panel or dock (EWMH `_NET_WM_STRUT_PARTIAL`): the width of each
/// reserved strip, measured from the edge of the whole screen, and the span of that edge it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct Strut {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub left_span: (f64, f64),
    pub right_span: (f64, f64),
    pub top_span: (f64, f64),
    pub bottom_span: (f64, f64),
}

impl Strut {
    /// A strut that covers the whole edge (the 4-value `_NET_WM_STRUT` form).
    pub fn full_edges(left: f64, right: f64, top: f64, bottom: f64) -> Self {
        let whole = (0.0, f64::MAX);
        Self {
            left,
            right,
            top,
            bottom,
            left_span: whole,
            right_span: whole,
            top_span: whole,
            bottom_span: whole,
        }
    }
}

fn spans_overlap(span: (f64, f64), start: f64, len: f64) -> bool {
    span.0 < start + len && span.1 >= start
}

/// The part of `monitor` left after removing strips reserved by `struts`. `screen` is the
/// bounding box of all displays, which is what strut distances are measured against.
pub fn work_area_from_struts(
    monitor: &WindowBounds,
    screen: &WindowBounds,
    struts: &[Strut],
) -> WindowBounds {
    let mut left = monitor.x;
    let mut top = monitor.y;
    let mut right = monitor.x + monitor.width;
    let mut bottom = monitor.y + monitor.height;

    for s in struts {
        if s.left > 0.0 {
            let edge = screen.x + s.left;
            if edge > monitor.x && spans_overlap(s.left_span, monitor.y, monitor.height) {
                left = left.max(edge);
            }
        }
        if s.right > 0.0 {
            let edge = screen.x + screen.width - s.right;
            if edge < monitor.x + monitor.width
                && spans_overlap(s.right_span, monitor.y, monitor.height)
            {
                right = right.min(edge);
            }
        }
        if s.top > 0.0 {
            let edge = screen.y + s.top;
            if edge > monitor.y && spans_overlap(s.top_span, monitor.x, monitor.width) {
                top = top.max(edge);
            }
        }
        if s.bottom > 0.0 {
            let edge = screen.y + screen.height - s.bottom;
            if edge < monitor.y + monitor.height
                && spans_overlap(s.bottom_span, monitor.x, monitor.width)
            {
                bottom = bottom.min(edge);
            }
        }
    }

    WindowBounds {
        x: left,
        y: top,
        width: (right - left).max(0.0),
        height: (bottom - top).max(0.0),
    }
}

#[cfg(test)]
mod strut_tests {
    use super::*;

    fn mon(x: f64, y: f64, w: f64, h: f64) -> WindowBounds {
        WindowBounds {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn top_panel_shrinks_the_monitor_it_sits_on() {
        let screen = mon(0.0, 0.0, 1920.0, 1080.0);
        let panel = Strut::full_edges(0.0, 0.0, 28.0, 0.0);
        let out = work_area_from_struts(&screen, &screen, &[panel]);
        assert_eq!(out, mon(0.0, 28.0, 1920.0, 1052.0));
    }

    #[test]
    fn partial_strut_only_affects_the_monitor_it_spans() {
        let screen = mon(0.0, 0.0, 3840.0, 1080.0);
        let mut panel = Strut::full_edges(0.0, 0.0, 30.0, 0.0);
        panel.top_span = (0.0, 1919.0);
        let first =
            work_area_from_struts(&mon(0.0, 0.0, 1920.0, 1080.0), &screen, &[panel.clone()]);
        let second = work_area_from_struts(&mon(1920.0, 0.0, 1920.0, 1080.0), &screen, &[panel]);
        assert_eq!(first, mon(0.0, 30.0, 1920.0, 1050.0));
        assert_eq!(second, mon(1920.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    fn bottom_dock_and_left_panel_combine() {
        let screen = mon(0.0, 0.0, 1920.0, 1080.0);
        let dock = Strut::full_edges(0.0, 0.0, 0.0, 48.0);
        let side = Strut::full_edges(60.0, 0.0, 0.0, 0.0);
        let out = work_area_from_struts(&screen, &screen, &[dock, side]);
        assert_eq!(out, mon(60.0, 0.0, 1860.0, 1032.0));
    }

    #[test]
    fn bottom_strut_does_not_touch_a_monitor_above_it() {
        // Two stacked displays; the dock hugs the bottom edge of the whole screen.
        let screen = mon(0.0, 0.0, 1920.0, 2160.0);
        let dock = Strut::full_edges(0.0, 0.0, 0.0, 48.0);
        let top_monitor = work_area_from_struts(&mon(0.0, 0.0, 1920.0, 1080.0), &screen, &[dock]);
        assert_eq!(top_monitor, mon(0.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    fn no_struts_returns_the_monitor() {
        let m = mon(10.0, 20.0, 800.0, 600.0);
        assert_eq!(work_area_from_struts(&m, &m, &[]), m);
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;

    #[test]
    fn invisible_borders_are_added_around_the_target() {
        let outer = WindowBounds {
            x: 92.0,
            y: 100.0,
            width: 816.0,
            height: 607.0,
        };
        // 8px invisible border left/right/bottom, none on top.
        let visible = WindowBounds {
            x: 100.0,
            y: 100.0,
            width: 800.0,
            height: 599.0,
        };
        let target = WindowBounds {
            x: 0.0,
            y: 25.0,
            width: 960.0,
            height: 1055.0,
        };
        let out = visible_to_outer(&target, &visible, &outer);
        assert_eq!(
            out,
            WindowBounds {
                x: -8.0,
                y: 25.0,
                width: 976.0,
                height: 1063.0,
            }
        );
    }

    #[test]
    fn borderless_window_is_unchanged() {
        let r = WindowBounds {
            x: 10.0,
            y: 10.0,
            width: 300.0,
            height: 200.0,
        };
        assert_eq!(visible_to_outer(&r, &r, &r), r);
    }
}

#[cfg(test)]
mod resize_tests {
    use super::*;

    fn area() -> WindowBounds {
        WindowBounds {
            x: 0.0,
            y: 25.0,
            width: 1000.0,
            height: 700.0,
        }
    }

    #[test]
    fn growing_past_the_work_area_is_clamped() {
        let win = WindowBounds {
            x: 50.0,
            y: 100.0,
            width: 950.0,
            height: 650.0,
        };
        let out = resize_about_center(&win, 1.5, &area());
        assert_eq!(out, area());
    }

    #[test]
    fn shrinking_stops_at_minimum_size() {
        let win = WindowBounds {
            x: 400.0,
            y: 300.0,
            width: 130.0,
            height: 90.0,
        };
        let out = resize_about_center(&win, 0.1, &area());
        assert_eq!((out.width, out.height), (120.0, 80.0));
    }

    #[test]
    fn grown_window_near_an_edge_is_shifted_back_inside() {
        let win = WindowBounds {
            x: 800.0,
            y: 100.0,
            width: 200.0,
            height: 200.0,
        };
        let out = resize_about_center(&win, 2.0, &area());
        assert!(out.x + out.width <= 1000.0 + 1e-9);
        assert_eq!(out.width, 400.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_monitors_two() -> Vec<WindowBounds> {
        vec![
            WindowBounds {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            WindowBounds {
                x: 1920.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
        ]
    }

    fn sample_monitors_three() -> Vec<WindowBounds> {
        vec![
            WindowBounds {
                x: 0.0,
                y: 0.0,
                width: 2560.0,
                height: 1440.0,
            },
            WindowBounds {
                x: 2560.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            WindowBounds {
                x: -1920.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
        ]
    }

    #[test]
    fn single_monitor_returns_none_for_next_and_previous() {
        let single = vec![WindowBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        }];
        let win = WindowBounds {
            x: 100.0,
            y: 100.0,
            width: 800.0,
            height: 600.0,
        };

        assert_eq!(
            calculate_target_display_bounds(&win, &single, DisplayDirection::Next),
            None
        );
        assert_eq!(
            calculate_target_display_bounds(&win, &single, DisplayDirection::Previous),
            None
        );
    }

    #[test]
    fn empty_monitors_returns_none() {
        let win = WindowBounds {
            x: 100.0,
            y: 100.0,
            width: 800.0,
            height: 600.0,
        };
        assert_eq!(
            calculate_target_display_bounds(&win, &[], DisplayDirection::Next),
            None
        );
    }

    #[test]
    fn two_monitors_moves_next_and_previous() {
        let monitors = sample_monitors_two();
        let win_on_first = WindowBounds {
            x: 100.0,
            y: 200.0,
            width: 800.0,
            height: 600.0,
        };

        // Next from monitor 0 moves to monitor 1
        let update_next =
            calculate_target_display_bounds(&win_on_first, &monitors, DisplayDirection::Next)
                .expect("should move to next monitor");
        assert_eq!(update_next.x, Some(2020.0));
        assert_eq!(update_next.y, Some(200.0));
        assert_eq!(update_next.width, Some(800.0));
        assert_eq!(update_next.height, Some(600.0));

        // Moving next from monitor 1 wraps back to monitor 0
        let win_on_second = WindowBounds {
            x: 2020.0,
            y: 200.0,
            width: 800.0,
            height: 600.0,
        };
        let update_wrap =
            calculate_target_display_bounds(&win_on_second, &monitors, DisplayDirection::Next)
                .expect("should wrap to monitor 0");
        assert_eq!(update_wrap.x, Some(100.0));
        assert_eq!(update_wrap.y, Some(200.0));
        assert_eq!(update_wrap.width, Some(800.0));
        assert_eq!(update_wrap.height, Some(600.0));

        // Previous from monitor 0 moves to monitor 1
        let update_prev =
            calculate_target_display_bounds(&win_on_first, &monitors, DisplayDirection::Previous)
                .expect("should wrap to monitor 1");
        assert_eq!(update_prev.x, Some(2020.0));
        assert_eq!(update_prev.y, Some(200.0));
    }

    #[test]
    fn three_monitors_wraps_in_both_directions() {
        let monitors = sample_monitors_three();
        // Sorted monitors:
        // [0]: -1920..0 (width 1920)
        // [1]: 0..2560 (width 2560)
        // [2]: 2560..4480 (width 1920)
        let win_on_middle = WindowBounds {
            x: 256.0,
            y: 144.0,
            width: 2048.0,
            height: 1152.0,
        }; // 80% size, 10% offset

        // Next from middle (index 1) -> right (index 2)
        let to_right =
            calculate_target_display_bounds(&win_on_middle, &monitors, DisplayDirection::Next)
                .expect("to right monitor");
        // Right monitor: x=2560, width=1920. 80% w = 1536. 10% x = 192 -> 2560 + 192 = 2752.
        assert_eq!(to_right.x, Some(2752.0));
        assert_eq!(to_right.y, Some(108.0));
        assert_eq!(to_right.width, Some(1536.0));
        assert_eq!(to_right.height, Some(864.0));

        // Previous from middle (index 1) -> left (index 0)
        let to_left =
            calculate_target_display_bounds(&win_on_middle, &monitors, DisplayDirection::Previous)
                .expect("to left monitor");
        // Left monitor: x=-1920, width=1920. 80% w = 1536. 10% x = 192 -> -1920 + 192 = -1728.
        assert_eq!(to_left.x, Some(-1728.0));
        assert_eq!(to_left.y, Some(108.0));
        assert_eq!(to_left.width, Some(1536.0));
        assert_eq!(to_left.height, Some(864.0));
    }

    #[test]
    fn proportional_scaling_different_resolutions() {
        let monitors = vec![
            WindowBounds {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            WindowBounds {
                x: 1920.0,
                y: 0.0,
                width: 2560.0,
                height: 1440.0,
            },
        ];
        let left_half_1080p = WindowBounds {
            x: 0.0,
            y: 0.0,
            width: 960.0,
            height: 1080.0,
        };

        let result =
            calculate_target_display_bounds(&left_half_1080p, &monitors, DisplayDirection::Next)
                .expect("should scale to 1440p monitor");

        assert_eq!(result.x, Some(1920.0));
        assert_eq!(result.y, Some(0.0));
        assert_eq!(result.width, Some(1280.0));
        assert_eq!(result.height, Some(1440.0));
    }

    #[test]
    fn preset_action_dispatches_correctly() {
        let monitors = sample_monitors_two();
        let win = WindowBounds {
            x: 100.0,
            y: 100.0,
            width: 800.0,
            height: 600.0,
        };

        // Maximize
        assert_eq!(
            calculate_preset_action("maximize", &win, &monitors).unwrap(),
            PresetAction::Fullscreen(true)
        );

        // Next-display on multi-monitor
        let next_act = calculate_preset_action("next-display", &win, &monitors).unwrap();
        assert!(matches!(next_act, PresetAction::Bounds(_)));

        // Next-display on single monitor is Noop
        let single = vec![monitors[0]];
        assert_eq!(
            calculate_preset_action("next-display", &win, &single).unwrap(),
            PresetAction::Noop
        );
        assert_eq!(
            calculate_preset_action("previous-display", &win, &single).unwrap(),
            PresetAction::Noop
        );

        // Left-half
        let half_act = calculate_preset_action("left-half", &win, &monitors).unwrap();
        if let PresetAction::Bounds(u) = half_act {
            assert_eq!(u.x, Some(0.0));
            assert_eq!(u.y, Some(0.0));
            assert_eq!(u.width, Some(960.0));
            assert_eq!(u.height, Some(1080.0));
        } else {
            panic!("Expected PresetAction::Bounds");
        }

        // Unknown preset returns Validation error
        assert!(calculate_preset_action("invalid-preset", &win, &monitors).is_err());
    }

    #[test]
    fn preset_action_calculates_fourths_correctly() {
        let monitors = sample_monitors_two();
        let win = WindowBounds {
            x: 100.0,
            y: 100.0,
            width: 800.0,
            height: 600.0,
        };

        // First fourth: x=0, w=1920/4=480
        let act1 = calculate_preset_action("first-fourth", &win, &monitors).unwrap();
        assert_eq!(
            act1,
            PresetAction::Bounds(WindowBoundsUpdate {
                x: Some(0.0),
                y: Some(0.0),
                width: Some(480.0),
                height: Some(1080.0),
            })
        );

        // Second fourth: x=480, w=480
        let act2 = calculate_preset_action("second-fourth", &win, &monitors).unwrap();
        assert_eq!(
            act2,
            PresetAction::Bounds(WindowBoundsUpdate {
                x: Some(480.0),
                y: Some(0.0),
                width: Some(480.0),
                height: Some(1080.0),
            })
        );

        // Third fourth: x=960, w=480
        let act3 = calculate_preset_action("third-fourth", &win, &monitors).unwrap();
        assert_eq!(
            act3,
            PresetAction::Bounds(WindowBoundsUpdate {
                x: Some(960.0),
                y: Some(0.0),
                width: Some(480.0),
                height: Some(1080.0),
            })
        );

        // Last fourth: x=1440, w=480
        let act4 = calculate_preset_action("last-fourth", &win, &monitors).unwrap();
        assert_eq!(
            act4,
            PresetAction::Bounds(WindowBoundsUpdate {
                x: Some(1440.0),
                y: Some(0.0),
                width: Some(480.0),
                height: Some(1080.0),
            })
        );
    }
}
