//! Text primitive — token-driven label.
//!
//! `Label` is a thin wrapper around a styled `div()` that locks the text
//! color to a [`SemanticColor`] and applies a HIG type preset. Use it instead
//! of raw `div().child("…")` whenever a piece of text is part of the design
//! system (which is to say: almost always).

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, App, FontWeight, IntoElement, RenderOnce, SharedString, Window};

use crate::ui::tokens::{Appearance, FontSize, SemanticColor};

/// HIG type-role presets. The variant chooses both the size and the default
/// foreground token. Callers can override the foreground via [`Label::color`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelVariant {
    /// 20pt semibold — section titles.
    Title,
    /// 15pt semibold — list-item titles, popover headers.
    Headline,
    /// 13pt regular — primary body text.
    Body,
    /// 12pt regular — secondary text, footnotes.
    Caption,
    /// 11pt medium — uppercase labels, badges.
    Micro,
}

#[derive(IntoElement)]
#[must_use]
pub struct Label {
    text: SharedString,
    variant: LabelVariant,
    color: Option<SemanticColor>,
    appearance: Option<Appearance>,
}

impl Label {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            variant: LabelVariant::Body,
            color: None,
            appearance: None,
        }
    }

    pub fn variant(mut self, variant: LabelVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn color(mut self, color: SemanticColor) -> Self {
        self.color = Some(color);
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let appearance = self.appearance.unwrap_or_else(|| Appearance::current(cx));
        let (size, weight, default_color) = match self.variant {
            LabelVariant::Title => (FontSize::Title2, FontWeight::SEMIBOLD, SemanticColor::Label),
            LabelVariant::Headline => (
                FontSize::Headline,
                FontWeight::SEMIBOLD,
                SemanticColor::Label,
            ),
            LabelVariant::Body => (FontSize::Body, FontWeight::NORMAL, SemanticColor::Label),
            LabelVariant::Caption => (
                FontSize::Caption,
                FontWeight::NORMAL,
                SemanticColor::SecondaryLabel,
            ),
            LabelVariant::Micro => (
                FontSize::Micro,
                FontWeight::MEDIUM,
                SemanticColor::TertiaryLabel,
            ),
        };
        let color = self.color.unwrap_or(default_color);
        div()
            .text_size(size.px())
            .font_weight(weight)
            .text_color(color.resolve(appearance))
            .child(self.text)
    }
}
