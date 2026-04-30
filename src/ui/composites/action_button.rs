//! Bordered, accent-colored micro action button used by the metadata
//! inspector panels (subscribe, re-read, lookup `MusicBrainz`, …).
//!
//! Wraps [`gpui_component::button::Button`] with the v4vmm-specific
//! styling so call sites stop repeating the same six chained method
//! calls. Returns the underlying `Button` so callers can keep chaining
//! `.on_click(...)` and friends as before.
//!
//! Sizing flows through [`crate::ui::sizable_bridge::SizableScaled`] so
//! the chip rescales with the user's `ScaleFactor`.

#![warn(clippy::pedantic)]

use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor};
use gpui::{App, SharedString, Styled};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::Size;

/// Build a metadata-action button with the standard accent-bordered style.
///
/// The returned [`Button`] already has its id, label, size, ghost variant,
/// border and colors set. Chain `.on_click(...)` to wire behaviour.
#[must_use]
pub fn action_button(label: &str, cx: &App) -> Button {
    Button::new(SharedString::from(format!("metadata-action:{label}")))
        .label(SharedString::from(label.to_string()))
        .scaled(Size::XSmall, cx)
        .compact()
        .ghost()
        .text_color(color(cx, SemanticColor::OnAccent))
        .text_size(FontSize::Micro.scaled(cx))
        .rounded(Radius::SM.scaled(cx))
        .border_1()
        .border_color(color(cx, SemanticColor::Accent))
}
