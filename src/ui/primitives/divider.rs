//! Hairline divider primitive — a 1px line in the [`SemanticColor::Separator`]
//! token. Pairs naturally with `v_flex` / `h_flex` layouts.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, px, App, IntoElement, RenderOnce, Window};

use crate::ui::tokens::{Appearance, SemanticColor};

/// Orientation of the divider line. The "long" axis stretches to fill the
/// parent; the "short" axis is fixed at 1px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerOrientation {
    Horizontal,
    Vertical,
}

#[derive(IntoElement)]
#[must_use]
pub struct Divider {
    orientation: DividerOrientation,
    appearance: Appearance,
    strong: bool,
}

impl Divider {
    pub fn horizontal() -> Self {
        Self::with_orientation(DividerOrientation::Horizontal)
    }

    pub fn vertical() -> Self {
        Self::with_orientation(DividerOrientation::Vertical)
    }

    fn with_orientation(orientation: DividerOrientation) -> Self {
        Self {
            orientation,
            appearance: Appearance::Dark,
            strong: false,
        }
    }

    /// Use [`SemanticColor::OpaqueSeparator`] instead of [`SemanticColor::Separator`].
    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }
}

impl RenderOnce for Divider {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let token = if self.strong {
            SemanticColor::OpaqueSeparator
        } else {
            SemanticColor::Separator
        };
        let color = token.resolve(self.appearance);
        match self.orientation {
            DividerOrientation::Horizontal => div().w_full().h(px(1.0)).bg(color),
            DividerOrientation::Vertical => div().h_full().w(px(1.0)).bg(color),
        }
    }
}
