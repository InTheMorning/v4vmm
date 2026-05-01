//! Metadata-action button composite.
//!
//! This is the first consumer of the ADR 0025 control-style boundary. Call
//! sites still receive a chainable native button, while the visual role maps
//! through [`crate::ui::control_styles::ControlStyle`].

#![warn(clippy::pedantic)]

use gpui::{App, SharedString};

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button;

/// Build a metadata-action button with the standard accent-bordered style.
pub fn action_button(label: &str, _cx: &App) -> Button {
    Button::styled(
        SharedString::from(format!("metadata-action:{label}")),
        ControlStyle::MetadataAction,
    )
    .label(SharedString::from(label.to_string()))
}
