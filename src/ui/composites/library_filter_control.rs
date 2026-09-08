//! Frame-local library-membership tri-state filter control.
//!
//! ADR 0062 replaces the content-list source chip strip with a single
//! library-membership axis. This composite consumes the VM display contract and
//! dispatches the next filter selected by activation.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{div, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window};

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button, ButtonLabelTreatment};
use crate::ui::tokens::{SemanticColor, Spacing};
use crate::view_models::workspace::{
    ContentFilter, LibraryFilterControlDisplay, LibraryFilterControlTreatment,
};

type LibraryFilterActivateHandler = Rc<dyn Fn(ContentFilter, &mut Window, &mut App) + 'static>;

/// Callback slots for [`LibraryFilterControl`].
#[derive(Default)]
#[must_use]
pub(crate) struct LibraryFilterControlSlots {
    on_activate: Option<LibraryFilterActivateHandler>,
}

impl LibraryFilterControlSlots {
    /// Creates empty library-filter-control slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the activation callback.
    pub(crate) fn on_activate(
        mut self,
        handler: impl Fn(ContentFilter, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_activate = Some(Rc::new(handler));
        self
    }
}

/// Shared frame-local library-membership filter control.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct LibraryFilterControl {
    display: LibraryFilterControlDisplay,
    slots: LibraryFilterControlSlots,
}

impl LibraryFilterControl {
    /// Creates a library filter control from display data and slots.
    pub(crate) fn new(
        display: LibraryFilterControlDisplay,
        slots: LibraryFilterControlSlots,
    ) -> Self {
        Self { display, slots }
    }
}

/// Creates a shared frame-local library-membership filter control.
pub(crate) fn library_filter_control(
    display: LibraryFilterControlDisplay,
    slots: LibraryFilterControlSlots,
) -> LibraryFilterControl {
    LibraryFilterControl::new(display, slots)
}

impl RenderOnce for LibraryFilterControl {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { display, slots } = self;
        let next_filter = display.next_filter;
        let mut button = Button::styled(
            SharedString::from(display.id),
            control_style(display.current.treatment),
        )
        .label(display.current.text_label)
        .a11y_label(display.current.a11y_label)
        .tooltip(display.current.a11y_label)
        .label_treatment(label_treatment(display.current.treatment));
        if let Some(foreground) = foreground(display.current.treatment) {
            button = button.foreground(foreground);
        }

        if let Some(handler) = slots.on_activate {
            button = button.on_activate(move |window, cx| {
                handler(next_filter, window, cx);
            });
        }

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::XS.scaled(cx))
            .child(button)
    }
}

const fn control_style(treatment: LibraryFilterControlTreatment) -> ControlStyle {
    match treatment {
        LibraryFilterControlTreatment::Highlighted
        | LibraryFilterControlTreatment::StruckThrough => ControlStyle::Secondary,
        LibraryFilterControlTreatment::Off => ControlStyle::Ghost,
    }
}

const fn label_treatment(treatment: LibraryFilterControlTreatment) -> ButtonLabelTreatment {
    match treatment {
        LibraryFilterControlTreatment::StruckThrough => ButtonLabelTreatment::LineThrough,
        LibraryFilterControlTreatment::Highlighted | LibraryFilterControlTreatment::Off => {
            ButtonLabelTreatment::Plain
        }
    }
}

const fn foreground(treatment: LibraryFilterControlTreatment) -> Option<SemanticColor> {
    match treatment {
        LibraryFilterControlTreatment::Off => Some(SemanticColor::SecondaryLabel),
        LibraryFilterControlTreatment::Highlighted
        | LibraryFilterControlTreatment::StruckThrough => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_accept_activation_callback() {
        let slots = LibraryFilterControlSlots::new().on_activate(|_, _, _| {});

        assert!(slots.on_activate.is_some());
    }

    #[test]
    fn treatment_projects_control_style_without_color_only_state() {
        assert_eq!(
            control_style(LibraryFilterControlTreatment::Highlighted),
            ControlStyle::Secondary
        );
        assert_eq!(
            control_style(LibraryFilterControlTreatment::Off),
            ControlStyle::Ghost
        );
        assert_eq!(
            control_style(LibraryFilterControlTreatment::StruckThrough),
            ControlStyle::Secondary
        );
        assert_eq!(
            label_treatment(LibraryFilterControlTreatment::StruckThrough),
            ButtonLabelTreatment::LineThrough
        );
        assert_eq!(
            foreground(LibraryFilterControlTreatment::Off),
            Some(SemanticColor::SecondaryLabel)
        );
        assert_eq!(foreground(LibraryFilterControlTreatment::Highlighted), None);
    }
}
