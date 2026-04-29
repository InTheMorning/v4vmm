//! Section heading primitive.
//!
//! Borrowed from the `SwiftUI` `Section { ... } header: { Text(...) }` shape:
//! a small, bold, secondary-coloured strip that introduces a group of rows.
//! With [`SectionHeader::disclosure`] it also covers the
//! `DisclosureGroup` header — chevron + label + show/hide hint — but stays
//! pure render: the caller is responsible for wrapping it in a stateful
//! click target since toggle state belongs to the screen / view-model.
//!
//! ```ignore
//! // Static
//! SectionHeader::new("Recently played")
//!
//! // Disclosure (caller wires the click target)
//! div()
//!     .id("section-foo")
//!     .on_click(...)
//!     .child(SectionHeader::new("Foo").disclosure(self.foo_collapsed))
//! ```

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, App, FontWeight, IntoElement, RenderOnce, SharedString, Window};

use crate::ui::tokens::{Appearance, FontSize, SemanticColor, Spacing};

#[derive(IntoElement)]
#[must_use]
pub struct SectionHeader {
    label: SharedString,
    disclosure: Option<bool>,
    appearance: Option<Appearance>,
}

impl SectionHeader {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            disclosure: None,
            appearance: None,
        }
    }

    /// Render as a disclosure header: prefixed with a chevron and suffixed
    /// with a `show` / `hide` hint. Pass the *collapsed* state.
    pub fn disclosure(mut self, collapsed: bool) -> Self {
        self.disclosure = Some(collapsed);
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for SectionHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let appearance = self.appearance.unwrap_or_else(|| Appearance::current(cx));
        let muted = SemanticColor::SecondaryLabel.resolve(appearance);
        let micro = FontSize::Micro.px();

        let row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::SM.px())
            .text_size(micro)
            .text_color(muted);

        if let Some(collapsed) = self.disclosure {
            let glyph: SharedString = if collapsed { ">".into() } else { "v".into() };
            let hint: SharedString = if collapsed {
                "show".into()
            } else {
                "hide".into()
            };
            row.child(div().font_weight(FontWeight::BOLD).child(glyph))
                .child(div().font_weight(FontWeight::BOLD).child(self.label))
                .child(div().child(hint))
        } else {
            row.child(div().font_weight(FontWeight::BOLD).child(self.label))
        }
    }
}
