//! Metadata-action button composite.
//!
//! This is the first consumer of the ADR 0025 control-style boundary. Call
//! sites still receive a chainable native button, while the visual role maps
//! through [`crate::ui::control_styles::ControlStyle`].
//!
//! Accessibility note (ADR 0038 task 005): [`ActionButtonDisplay`] carries an
//! `a11y_label` sourced from a view-model. GPUI 0.2.x does not yet expose an
//! accessibility-label sink on its button widget, so the field is contract-only
//! today. When the framework grows the surface, the wiring is one-line — the
//! label is already present at the call site.

#![warn(clippy::pedantic)]

use gpui::{App, SharedString};

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionButtonDisplay {
    pub label: SharedString,
    pub a11y_label: SharedString,
}

/// Build a metadata-action button with the standard accent-bordered style.
pub fn action_button(display: ActionButtonDisplay, _cx: &App) -> Button {
    let ActionButtonDisplay { label, a11y_label } = display;
    Button::styled(
        SharedString::from(format!("metadata-action:{label}")),
        ControlStyle::MetadataAction,
    )
    .label(label)
    .a11y_label(a11y_label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_contract_carries_button_label() {
        let display = ActionButtonDisplay {
            label: SharedString::from("Apply"),
            a11y_label: SharedString::from("Apply metadata changes"),
        };

        assert_eq!(display.label, SharedString::from("Apply"));
        assert_eq!(
            display.a11y_label,
            SharedString::from("Apply metadata changes")
        );
    }

    #[test]
    fn display_contract_carries_a11y_label_distinct_from_visible_label() {
        let display = ActionButtonDisplay {
            label: SharedString::from("Remove"),
            a11y_label: SharedString::from("Remove feed from library"),
        };

        assert_ne!(display.label, display.a11y_label);
    }
}
