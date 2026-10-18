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

use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};

/// The heading level of a [`SectionHeader`] (ADR 0082 packet 002
/// correction). `Standard` is the default, and it renders exactly as this
/// primitive always has. `Container` is for a heading that contains other
/// section headings, such as a page scope above its groups: it must read
/// as visibly stronger than a `Standard` heading below it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SectionHeaderLevel {
    #[default]
    Standard,
    Container,
}

impl SectionHeaderLevel {
    /// The type size and the label color this level uses, as named tokens.
    /// Both levels keep the same bold weight; only the size and the color
    /// carry the difference. Pure data, with no GPUI context, so a unit
    /// test can check it directly.
    #[must_use]
    pub const fn text_style(self) -> (FontSize, SemanticColor) {
        match self {
            Self::Standard => (FontSize::Micro, SemanticColor::SecondaryLabel),
            Self::Container => (FontSize::Body, SemanticColor::Label),
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SectionHeader {
    label: SharedString,
    disclosure: Option<bool>,
    appearance: Option<Appearance>,
    level: SectionHeaderLevel,
}

impl SectionHeader {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            disclosure: None,
            appearance: None,
            level: SectionHeaderLevel::Standard,
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

    /// Sets the heading level (ADR 0082 packet 002 correction). The
    /// default is `SectionHeaderLevel::Standard`, unchanged from before
    /// this method existed.
    pub fn level(mut self, level: SectionHeaderLevel) -> Self {
        self.level = level;
        self
    }
}

impl RenderOnce for SectionHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (font_size, label_color) = self.level.text_style();
        let muted = resolve_color(cx, label_color, self.appearance);
        let micro = font_size.scaled(cx);

        let row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::SM.scaled(cx))
            .text_size(micro)
            .text_color(muted);

        if let Some(collapsed) = self.disclosure {
            // ADR 0083 Decision 10: a Lucide chevron, not a character. It
            // turns from pointing right to pointing down when it opens.
            let icon_name = if collapsed {
                IconName::ChevronRight
            } else {
                IconName::ChevronDown
            };
            let hint: SharedString = if collapsed {
                "show".into()
            } else {
                "hide".into()
            };
            row.child(Icon::new(icon_name).size(IconSize::Transport).color(muted))
                .child(div().font_weight(FontWeight::BOLD).child(self.label))
                .child(div().child(hint))
        } else {
            row.child(div().font_weight(FontWeight::BOLD).child(self.label))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Orchestrator review (ADR 0082 packet 002 correction): the default
    /// level must render exactly as this primitive did before this
    /// change: `FontSize::Micro` and `SemanticColor::SecondaryLabel`. The
    /// `Container` level must give a different size and a different
    /// color, so it reads as visibly stronger than a `Standard` heading.
    #[test]
    fn section_header_default_level_is_unchanged_and_container_level_differs() {
        assert_eq!(
            SectionHeaderLevel::default().text_style(),
            (FontSize::Micro, SemanticColor::SecondaryLabel),
            "the default level must render exactly as it did before this change"
        );
        assert_ne!(
            SectionHeaderLevel::Container.text_style(),
            SectionHeaderLevel::Standard.text_style(),
            "the container level must look different from the default level"
        );
    }
}
