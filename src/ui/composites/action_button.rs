//! Bordered, accent-colored micro action button used by the metadata
//! inspector panels (subscribe, re-read, lookup `MusicBrainz`, …).
//!
//! Wraps [`gpui_component::button::Button`] with the v4vmm-specific
//! styling so call sites stop repeating the same six chained method
//! calls. Returns the underlying `Button` so callers can keep chaining
//! `.on_click(...)` and friends as before.
//!
//! TODO(gpui-component-scale-bridge): the size still comes from
//! `gpui_component::Size::XSmall` rather than our `ScaleFactor`; once
//! the bridge lands we should derive it from `Environment::current(cx)`.

#![warn(clippy::pedantic)]

use crate::ui::theme::{color, radius, typography};
use gpui::{SharedString, Styled};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{Sizable, Size};

/// Build a metadata-action button with the standard accent-bordered style.
///
/// The returned [`Button`] already has its id, label, size, ghost variant,
/// border and colors set. Chain `.on_click(...)` to wire behaviour.
#[must_use]
pub fn action_button(label: &str) -> Button {
    Button::new(SharedString::from(format!("metadata-action:{label}")))
        .label(SharedString::from(label.to_string()))
        .with_size(Size::XSmall)
        .compact()
        .ghost()
        .text_color(color::text_on_accent())
        .text_size(typography::SIZE_MICRO)
        .rounded(radius::SM)
        .border_1()
        .border_color(color::accent())
}
