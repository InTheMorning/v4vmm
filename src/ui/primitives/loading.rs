//! Loading message primitive.
//!
//! A compact, token-driven status line for transient loading or empty states.
//! Screens pass only display text; the design-system layer owns the muted
//! italic presentation so duplicated loading affordances do not drift.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, App, IntoElement, RenderOnce, SharedString, Window};

use crate::ui::tokens::{resolve_color, Appearance, SemanticColor, Spacing};

#[derive(IntoElement)]
#[must_use]
pub struct LoadingMessage {
    message: SharedString,
    appearance: Option<Appearance>,
}

impl LoadingMessage {
    pub fn new(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
            appearance: None,
        }
    }

    pub fn from_text(message: &str) -> Self {
        Self::new(message.to_owned())
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for LoadingMessage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_color(resolve_color(
                cx,
                SemanticColor::SecondaryLabel,
                self.appearance,
            ))
            .italic()
            .py(Spacing::SM.scaled(cx))
            .child(self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_no_overrides() {
        let message = LoadingMessage::new("Loading");
        assert!(message.appearance.is_none());
    }

    #[test]
    fn from_text_owns_borrowed_message() {
        let message = LoadingMessage::from_text("Loading");
        assert_eq!(message.message, SharedString::from("Loading"));
    }

    #[test]
    fn appearance_sets_override() {
        let message = LoadingMessage::new("Loading").appearance(Appearance::Dark);
        assert_eq!(message.appearance, Some(Appearance::Dark));
    }
}
