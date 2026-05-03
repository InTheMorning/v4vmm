//! Metadata-action button composite.
//!
//! This is the first consumer of the ADR 0025 control-style boundary. Call
//! sites still receive a chainable native button, while the visual role maps
//! through [`crate::ui::control_styles::ControlStyle`].

#![warn(clippy::pedantic)]

use gpui::{App, SharedString};

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionButtonDisplay {
    pub label: SharedString,
}

/// Build a metadata-action button with the standard accent-bordered style.
pub fn action_button(display: ActionButtonDisplay, _cx: &App) -> Button {
    let label = display.label;
    Button::styled(
        SharedString::from(format!("metadata-action:{label}")),
        ControlStyle::MetadataAction,
    )
    .label(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_contract_carries_button_label() {
        let display = ActionButtonDisplay {
            label: SharedString::from("Apply"),
        };

        assert_eq!(display.label, SharedString::from("Apply"));
    }
}
