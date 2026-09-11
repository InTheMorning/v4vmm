//! Temporary, opt-in Settings draw measurements for ADR 0066 task 003.
//! Remove this probe and its call sites when the remaining stall is diagnosed
//! and its correction passes the operator recheck. It never reads app data.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window,
};

#[derive(Clone)]
pub(crate) struct SettingsTiming(Rc<RefCell<Measurement>>);

struct Measurement {
    started: Instant,
    recorded_at: SystemTime,
    steps: Vec<(&'static str, Duration)>,
}

impl SettingsTiming {
    pub(crate) fn begin() -> Option<Self> {
        // No environment lookup or trace output in release builds. Setting
        // the flag on the launch command does not persist a user preference.
        #[cfg(debug_assertions)]
        if std::env::var_os("V4VMM_SETTINGS_TIMING").is_some_and(|value| value == "1") {
            return Some(Self(Rc::new(RefCell::new(Measurement {
                started: Instant::now(),
                recorded_at: SystemTime::now(),
                steps: Vec::with_capacity(8),
            }))));
        }
        None
    }

    pub(crate) fn mark(&self, subject: &'static str) {
        let mut measurement = self.0.borrow_mut();
        let elapsed = measurement.started.elapsed();
        measurement.steps.push((subject, elapsed));
    }

    pub(crate) fn wrap(self, child: AnyElement) -> AnyElement {
        TimedFrame {
            child,
            timing: self,
        }
        .into_any_element()
    }

    fn report(&self) -> String {
        let measurement = self.0.borrow();
        let at: chrono::DateTime<chrono::Utc> = measurement.recorded_at.into();
        let mut report = format!(
            "[{}] App measured a Settings request. Times below are milliseconds since the app accepted the request.\n",
            at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        for (subject, elapsed) in &measurement.steps {
            let _ = writeln!(
                report,
                "{subject}: {:.3} ms",
                elapsed.as_secs_f64() * 1000.0
            );
        }
        report
    }
}

/// Delegates the original layout node and drawing operations unchanged.
/// Times include child elements that render after TopApp::render returns.
struct TimedFrame {
    child: AnyElement,
    timing: SettingsTiming,
}

impl IntoElement for TimedFrame {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for TimedFrame {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.timing.mark("App began requesting the frame layout");
        let layout = self.child.request_layout(window, cx);
        self.timing.mark("App finished requesting the frame layout");
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.timing
            .mark("GPUI computed the frame layout; app began prepaint");
        self.child.prepaint(window, cx);
        self.timing.mark("App finished frame prepaint");
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        self.timing
            .mark("App finished submitting the frame paint commands");
        // GPUI invokes this on the following frame request, before that next
        // draw. It includes presentation/wait time, not just GPU execution.
        let timing = self.timing.clone();
        window.on_next_frame(move |_, _| {
            timing.mark("App received the following frame callback");
            eprintln!("{}", timing.report());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0066_settings_timing_reports_utc_and_elapsed_stages_only() {
        let timing = SettingsTiming(Rc::new(RefCell::new(Measurement {
            started: Instant::now(),
            recorded_at: SystemTime::UNIX_EPOCH,
            steps: vec![(
                "App finished handling Settings selection",
                Duration::from_millis(25),
            )],
        })));
        let report = timing.report();
        assert!(report.starts_with("[1970-01-01 00:00:00 UTC]"));
        assert!(report.contains("since the app accepted the request"));
        assert!(report.contains("App finished handling Settings selection: 25.000 ms"));
    }
}
