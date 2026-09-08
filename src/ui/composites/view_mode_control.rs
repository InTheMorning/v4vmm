//! Frame-local content view-mode selector.
//!
//! ADR 0062 shares one content presentation-mode enum between Recent Feeds and
//! the Music content list. This composite adapts the VM display contract to the
//! existing segmented-control primitive.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{App, IntoElement, RenderOnce, SharedString, Window};

use crate::ui::composites::{Segment, SegmentDisplay, SegmentedControl};
use crate::view_models::workspace::{ContentViewMode, ContentViewModeControlDisplay};

type ViewModeSelectHandler = Rc<dyn Fn(ContentViewMode, &mut Window, &mut App) + 'static>;

/// Callback slots for [`ViewModeControl`].
#[derive(Default)]
#[must_use]
pub(crate) struct ViewModeControlSlots {
    on_select: Option<ViewModeSelectHandler>,
}

impl ViewModeControlSlots {
    /// Creates empty view-mode control slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the selection callback.
    pub(crate) fn on_select(
        mut self,
        handler: impl Fn(ContentViewMode, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

/// Shared frame-local view-mode control.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct ViewModeControl {
    display: ContentViewModeControlDisplay,
    slots: ViewModeControlSlots,
}

impl ViewModeControl {
    /// Creates a view-mode control from display data and slots.
    pub(crate) fn new(display: ContentViewModeControlDisplay, slots: ViewModeControlSlots) -> Self {
        Self { display, slots }
    }
}

/// Creates a shared frame-local view-mode control.
pub(crate) fn view_mode_control(
    display: ContentViewModeControlDisplay,
    slots: ViewModeControlSlots,
) -> ViewModeControl {
    ViewModeControl::new(display, slots)
}

impl RenderOnce for ViewModeControl {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let selected = self.display.selected;
        let mut control =
            SegmentedControl::new(selected)
                .filter_style()
                .segments(self.display.options.map(|option| {
                    Segment::new(SegmentDisplay {
                        id: SharedString::from(option.id).into(),
                        key: option.mode,
                        label: SharedString::from(option.label),
                        a11y_label: SharedString::from(option.a11y_label),
                    })
                }));

        if let Some(handler) = self.slots.on_select {
            control = control.on_select(move |mode, window, cx| {
                handler(*mode, window, cx);
            });
        }

        control
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_accept_selection_callback() {
        let slots = ViewModeControlSlots::new().on_select(|_, _, _| {});

        assert!(slots.on_select.is_some());
    }
}
