//! Linked split: two windows that share an edge and resize together.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::error::AppError;
use crate::window_management::backend::{
    EventSink, WatchGuard, WindowBackend, WindowEvent, WindowId,
};
use crate::window_management::types::WindowBounds;

/// Smallest size either side of the divider may be squeezed to.
const MIN_PANE: f64 = 150.0;
/// A window moved this far without changing size is being dragged away: unlink.
const DRAG_THRESHOLD: f64 = 12.0;
/// Rect differences at or below this are rounding noise, not user input.
const NOISE: f64 = 1.5;
/// Events are coalesced over this window so a drag costs one update per frame.
const COALESCE: Duration = Duration::from_millis(14);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Side by side, divider is vertical.
    Side,
    /// Stacked, divider is horizontal.
    Stack,
}

impl Axis {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        match raw {
            "side" => Ok(Axis::Side),
            "stack" => Ok(Axis::Stack),
            other => Err(AppError::Validation(format!(
                "Unknown split axis {other:?}, expected \"side\" or \"stack\""
            ))),
        }
    }
}

fn bounds(x: f64, y: f64, width: f64, height: f64) -> WindowBounds {
    WindowBounds {
        x,
        y,
        width,
        height,
    }
}

/// (main position, main length, cross position, cross length) of a rect along `axis`.
fn split_axes(axis: Axis, r: &WindowBounds) -> (f64, f64, f64, f64) {
    match axis {
        Axis::Side => (r.x, r.width, r.y, r.height),
        Axis::Stack => (r.y, r.height, r.x, r.width),
    }
}

fn join_axes(axis: Axis, mp: f64, ml: f64, cp: f64, cl: f64) -> WindowBounds {
    match axis {
        Axis::Side => bounds(mp, cp, ml, cl),
        Axis::Stack => bounds(cp, mp, cl, ml),
    }
}

/// The two halves of `work`: left/right for [`Axis::Side`], top/bottom for [`Axis::Stack`].
pub fn split_rects(work: &WindowBounds, axis: Axis) -> (WindowBounds, WindowBounds) {
    match axis {
        Axis::Side => {
            let half = work.width / 2.0;
            (
                bounds(work.x, work.y, half, work.height),
                bounds(work.x + half, work.y, half, work.height),
            )
        }
        Axis::Stack => {
            let half = work.height / 2.0;
            (
                bounds(work.x, work.y, work.width, half),
                bounds(work.x, work.y + half, work.width, half),
            )
        }
    }
}

/// Where the other window goes after `driver` changed, plus the updated shared span.
/// The pair always fills `span` along the main axis and shares the driver's cross extent.
pub fn follow(
    axis: Axis,
    driver_first: bool,
    driver: &WindowBounds,
    span: &WindowBounds,
) -> (WindowBounds, WindowBounds) {
    let (dmp, dml, dcp, dcl) = split_axes(axis, driver);
    let (smp, sml, _, _) = split_axes(axis, span);
    let span_end = smp + sml;

    let (other_mp, other_ml, new_span_mp, new_span_ml) = if driver_first {
        let lo = dmp + MIN_PANE;
        let hi = (span_end - MIN_PANE).max(lo);
        let edge = (dmp + dml).clamp(lo, hi);
        (edge, span_end - edge, dmp, span_end - dmp)
    } else {
        let hi = dmp + dml - MIN_PANE;
        let lo = (smp + MIN_PANE).min(hi);
        let edge = dmp.clamp(lo, hi);
        (smp, edge - smp, smp, dmp + dml - smp)
    };
    (
        join_axes(axis, other_mp, other_ml, dcp, dcl),
        join_axes(axis, new_span_mp, new_span_ml, dcp, dcl),
    )
}

fn approx_eq(a: &WindowBounds, b: &WindowBounds, tolerance: f64) -> bool {
    (a.x - b.x).abs() <= tolerance
        && (a.y - b.y).abs() <= tolerance
        && (a.width - b.width).abs() <= tolerance
        && (a.height - b.height).abs() <= tolerance
}

struct Active {
    a: WindowId,
    b: WindowId,
    axis: Axis,
    a_first: bool,
    span: WindowBounds,
    last_a: WindowBounds,
    last_b: WindowBounds,
    _guards: Vec<WatchGuard>,
}

pub struct LinkManager {
    backend: Arc<dyn WindowBackend>,
    active: Mutex<Option<Active>>,
    tx: Sender<WindowEvent>,
    rx: Mutex<Receiver<WindowEvent>>,
}

static MANAGER: OnceLock<Arc<LinkManager>> = OnceLock::new();

/// The app-wide manager, created (with its worker thread) on first use.
pub fn link_manager(backend: &Arc<dyn WindowBackend>) -> &'static Arc<LinkManager> {
    MANAGER.get_or_init(|| LinkManager::new(backend.clone(), true))
}

/// The app-wide manager if a split was ever started; never creates it.
pub fn existing_link_manager() -> Option<&'static Arc<LinkManager>> {
    MANAGER.get()
}

impl LinkManager {
    /// `spawn_worker` is false in tests, which drive `pump` by hand.
    pub fn new(backend: Arc<dyn WindowBackend>, spawn_worker: bool) -> Arc<Self> {
        let (tx, rx) = channel();
        let manager = Arc::new(Self {
            backend,
            active: Mutex::new(None),
            tx,
            rx: Mutex::new(rx),
        });
        if spawn_worker {
            let weak = Arc::downgrade(&manager);
            thread::spawn(move || loop {
                let Some(manager) = weak.upgrade() else { break };
                if !manager.pump_blocking() {
                    break;
                }
            });
        }
        manager
    }

    /// Links two windows that already sit where they should; `span` is the area the pair fills.
    pub fn link(
        &self,
        a: WindowId,
        b: WindowId,
        axis: Axis,
        span: WindowBounds,
    ) -> Result<(), AppError> {
        self.unlink();
        let ra = self.backend.bounds(&a)?;
        let rb = self.backend.bounds(&b)?;
        let a_first = split_axes(axis, &ra).0 <= split_axes(axis, &rb).0;
        let sink: EventSink = {
            let tx = self.tx.clone();
            Arc::new(move |event| {
                let _ = tx.send(event);
            })
        };
        let guards = vec![
            self.backend.watch(&a, sink.clone())?,
            self.backend.watch(&b, sink)?,
        ];
        *self.active.lock().map_err(|_| AppError::Lock)? = Some(Active {
            a,
            b,
            axis,
            a_first,
            span,
            last_a: ra,
            last_b: rb,
            _guards: guards,
        });
        Ok(())
    }

    /// Link two windows that are already arranged (used when restoring a workspace).
    pub fn link_existing(&self, a: WindowId, b: WindowId) -> Result<(), AppError> {
        let ra = self.backend.bounds(&a)?;
        let rb = self.backend.bounds(&b)?;
        let (ca, cb) = (center(&ra), center(&rb));
        let axis = if (ca.0 - cb.0).abs() >= (ca.1 - cb.1).abs() {
            Axis::Side
        } else {
            Axis::Stack
        };
        self.link(a, b, axis, bounding(&ra, &rb))
    }

    /// Returns whether a link was active.
    pub fn unlink(&self) -> bool {
        // Dropping the guards unsubscribes; do it outside the lock to keep it short.
        let old = self.active.lock().ok().and_then(|mut a| a.take());
        old.is_some()
    }

    pub fn pair(&self) -> Option<(WindowId, WindowId)> {
        self.active
            .lock()
            .ok()?
            .as_ref()
            .map(|l| (l.a.clone(), l.b.clone()))
    }

    pub fn is_linked(&self, id: &WindowId) -> bool {
        self.pair().is_some_and(|(a, b)| &a == id || &b == id)
    }

    fn pump_blocking(&self) -> bool {
        let first = match self.rx.lock().map(|rx| rx.recv()) {
            Ok(Ok(event)) => event,
            _ => return false,
        };
        thread::sleep(COALESCE);
        self.process(first);
        true
    }

    /// Handle one queued event batch without blocking. Used by tests.
    #[cfg(test)]
    pub fn pump(&self) {
        let first = self.rx.lock().unwrap().try_recv().ok();
        if let Some(event) = first {
            self.process(event);
        }
    }

    fn process(&self, first: WindowEvent) {
        let mut gone = first == WindowEvent::Gone;
        if let Ok(rx) = self.rx.lock() {
            while let Ok(event) = rx.try_recv() {
                gone |= event == WindowEvent::Gone;
            }
        }
        if gone {
            self.unlink();
            return;
        }
        let Ok(mut guard) = self.active.lock() else {
            return;
        };
        let Some(link) = guard.as_mut() else { return };
        let (Ok(cur_a), Ok(cur_b)) = (self.backend.bounds(&link.a), self.backend.bounds(&link.b))
        else {
            drop(guard);
            self.unlink();
            return;
        };
        let changed_a = !approx_eq(&cur_a, &link.last_a, NOISE);
        let changed_b = !approx_eq(&cur_b, &link.last_b, NOISE);
        let (driver_is_a, driver, prev) = match (changed_a, changed_b) {
            (false, false) => return, // echo of our own move
            (true, _) => (true, cur_a, link.last_a),
            (false, true) => (false, cur_b, link.last_b),
        };

        let same_size =
            (driver.width - prev.width).abs() <= 2.0 && (driver.height - prev.height).abs() <= 2.0;
        let moved = (driver.x - prev.x).abs().max((driver.y - prev.y).abs());
        if same_size && moved > DRAG_THRESHOLD {
            drop(guard);
            self.unlink();
            return;
        }

        let driver_first = if driver_is_a {
            link.a_first
        } else {
            !link.a_first
        };
        let (other_target, span) = follow(link.axis, driver_first, &driver, &link.span);
        let other_id = if driver_is_a {
            link.b.clone()
        } else {
            link.a.clone()
        };
        let _ = self.backend.set_bounds(&other_id, &other_target);
        // Read back: apps may clamp, and this is what makes the echo event a no-op.
        let other_actual = self.backend.bounds(&other_id).unwrap_or(other_target);
        link.span = span;
        if driver_is_a {
            link.last_a = driver;
            link.last_b = other_actual;
        } else {
            link.last_b = driver;
            link.last_a = other_actual;
        }
    }
}

fn center(r: &WindowBounds) -> (f64, f64) {
    (r.x + r.width / 2.0, r.y + r.height / 2.0)
}

fn bounding(a: &WindowBounds, b: &WindowBounds) -> WindowBounds {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    bounds(
        x,
        y,
        (a.x + a.width).max(b.x + b.width) - x,
        (a.y + a.height).max(b.y + b.height) - y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window_management::fake::{rect, FakeBackend};

    fn work() -> WindowBounds {
        rect(0.0, 25.0, 1200.0, 800.0)
    }

    #[test]
    fn follow_left_driver_widens() {
        let (o, span) = follow(Axis::Side, true, &rect(0.0, 25.0, 700.0, 800.0), &work());
        assert_eq!(o, rect(700.0, 25.0, 500.0, 800.0));
        assert_eq!(span, work());
    }

    #[test]
    fn follow_right_driver_widens() {
        let (o, _) = follow(Axis::Side, false, &rect(500.0, 25.0, 700.0, 800.0), &work());
        assert_eq!(o, rect(0.0, 25.0, 500.0, 800.0));
    }

    #[test]
    fn follow_stacked() {
        let (o, _) = follow(Axis::Stack, true, &rect(0.0, 25.0, 1200.0, 300.0), &work());
        assert_eq!(o, rect(0.0, 325.0, 1200.0, 500.0));
    }

    #[test]
    fn follow_never_squeezes_below_minimum() {
        let (o, _) = follow(Axis::Side, true, &rect(0.0, 25.0, 1190.0, 800.0), &work());
        assert_eq!(o.width, MIN_PANE);
        assert_eq!(o.x + o.width, work().x + work().width);
    }

    #[test]
    fn follow_shares_cross_extent() {
        let (o, span) = follow(Axis::Side, true, &rect(0.0, 100.0, 600.0, 500.0), &work());
        assert_eq!((o.y, o.height), (100.0, 500.0));
        assert_eq!((span.y, span.height), (100.0, 500.0));
    }

    #[test]
    fn split_rects_halve_the_work_area() {
        assert_eq!(
            split_rects(&work(), Axis::Side),
            (
                rect(0.0, 25.0, 600.0, 800.0),
                rect(600.0, 25.0, 600.0, 800.0)
            )
        );
        assert_eq!(
            split_rects(&work(), Axis::Stack),
            (
                rect(0.0, 25.0, 1200.0, 400.0),
                rect(0.0, 425.0, 1200.0, 400.0)
            )
        );
    }

    #[test]
    fn axis_parse_accepts_known_values_only() {
        assert_eq!(Axis::parse("side").unwrap(), Axis::Side);
        assert_eq!(Axis::parse("stack").unwrap(), Axis::Stack);
        assert!(Axis::parse("diagonal").is_err());
    }

    struct Setup {
        backend: Arc<FakeBackend>,
        manager: Arc<LinkManager>,
        a: WindowId,
        b: WindowId,
    }

    /// Two tiled windows, linked, with the tiling's own events already drained.
    fn linked() -> Setup {
        let backend = Arc::new(FakeBackend::with_work_area(work()));
        let a = backend.add_window("a", rect(50.0, 100.0, 300.0, 300.0));
        let b = backend.add_window("b", rect(400.0, 100.0, 300.0, 300.0));
        let manager = LinkManager::new(backend.clone(), false);
        let (ra, rb) = split_rects(&work(), Axis::Side);
        backend.set_bounds(&a, &ra).unwrap();
        backend.set_bounds(&b, &rb).unwrap();
        manager
            .link(a.clone(), b.clone(), Axis::Side, work())
            .unwrap();
        let setup = Setup {
            backend,
            manager,
            a,
            b,
        };
        settle(&setup.manager);
        setup
    }

    fn settle(manager: &LinkManager) {
        // Echo events from our own moves arrive after the user's; drain them all.
        for _ in 0..6 {
            manager.pump();
        }
    }

    #[test]
    fn linking_pair_is_reported_and_own_tiling_does_not_unlink() {
        let s = linked();
        assert_eq!(s.manager.pair(), Some((s.a.clone(), s.b.clone())));
        assert!(s.manager.is_linked(&s.a));
        assert_eq!(s.backend.bounds_of(&s.b), rect(600.0, 25.0, 600.0, 800.0));
    }

    #[test]
    fn resizing_one_window_moves_the_other() {
        let s = linked();
        s.backend.user_sets(&s.a, rect(0.0, 25.0, 800.0, 800.0));
        settle(&s.manager);
        assert_eq!(s.backend.bounds_of(&s.b), rect(800.0, 25.0, 400.0, 800.0));
        // And back from the other side.
        s.backend.user_sets(&s.b, rect(300.0, 25.0, 900.0, 800.0));
        settle(&s.manager);
        assert_eq!(s.backend.bounds_of(&s.a), rect(0.0, 25.0, 300.0, 800.0));
        assert!(s.manager.pair().is_some());
    }

    #[test]
    fn dragging_a_window_away_unlinks() {
        let s = linked();
        s.backend.user_sets(&s.a, rect(200.0, 300.0, 600.0, 800.0));
        settle(&s.manager);
        assert!(s.manager.pair().is_none());
        assert_eq!(s.backend.bounds_of(&s.b), rect(600.0, 25.0, 600.0, 800.0));
    }

    #[test]
    fn closing_a_window_unlinks() {
        let s = linked();
        s.backend.close(&s.b);
        settle(&s.manager);
        assert!(s.manager.pair().is_none());
    }

    #[test]
    fn minimizing_a_window_unlinks() {
        let s = linked();
        s.backend.minimize(&s.a).unwrap();
        settle(&s.manager);
        assert!(s.manager.pair().is_none());
    }

    #[test]
    fn app_minimum_size_does_not_cause_a_fight() {
        let s = linked();
        *s.backend.min_size.lock().unwrap() = (450.0, 0.0);
        s.backend.user_sets(&s.a, rect(0.0, 25.0, 900.0, 800.0));
        settle(&s.manager);
        assert_eq!(s.backend.bounds_of(&s.b).width, 450.0);
        let after = s.backend.bounds_of(&s.a);
        settle(&s.manager);
        assert_eq!(s.backend.bounds_of(&s.a), after, "no further adjustments");
    }

    #[test]
    fn unlink_stops_following_and_reports_whether_it_was_linked() {
        let s = linked();
        assert!(s.manager.unlink());
        assert!(!s.manager.unlink());
        s.backend.user_sets(&s.a, rect(0.0, 25.0, 800.0, 800.0));
        settle(&s.manager);
        assert_eq!(s.backend.bounds_of(&s.b), rect(600.0, 25.0, 600.0, 800.0));
    }

    #[test]
    fn link_existing_detects_axis() {
        let backend = Arc::new(FakeBackend::with_work_area(work()));
        let a = backend.add_window("a", rect(0.0, 25.0, 1200.0, 400.0));
        let b = backend.add_window("b", rect(0.0, 425.0, 1200.0, 400.0));
        let manager = LinkManager::new(backend.clone(), false);
        manager.link_existing(a.clone(), b.clone()).unwrap();
        settle(&manager);
        backend.user_sets(&a, rect(0.0, 25.0, 1200.0, 500.0));
        settle(&manager);
        assert_eq!(backend.bounds_of(&b), rect(0.0, 525.0, 1200.0, 300.0));
    }

    #[test]
    fn linking_a_missing_window_fails_without_leaving_a_link() {
        let backend = Arc::new(FakeBackend::with_work_area(work()));
        let a = backend.add_window("a", rect(0.0, 25.0, 600.0, 800.0));
        let manager = LinkManager::new(backend, false);
        let result = manager.link(a, WindowId::new("ghost"), Axis::Side, work());
        assert!(result.is_err());
        assert!(manager.pair().is_none());
    }

    #[test]
    fn a_new_link_replaces_the_old_one() {
        let s = linked();
        let c = s.backend.add_window("c", rect(0.0, 25.0, 600.0, 800.0));
        s.manager
            .link(s.a.clone(), c.clone(), Axis::Side, work())
            .unwrap();
        assert_eq!(s.manager.pair(), Some((s.a.clone(), c)));
        assert!(!s.manager.is_linked(&s.b));
    }
}
