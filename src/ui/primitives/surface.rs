//! Container primitive — the canonical "panel / card / popover body" surface.
//!
//! `Surface` is a token-driven wrapper around `div()` that picks an
//! appropriate background, border and corner radius based on a semantic
//! [`SurfaceElevation`]. Use it for popover content, dialog bodies, sidebars,
//! cards, and any other raised container.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, App, Div, IntoElement, RenderOnce, Window};

use crate::ui::tokens::{resolve_color, Appearance, Radius, SemanticColor, Spacing};

/// HIG-aligned elevation tiers. Each tier maps to a different background +
/// border combination so nested surfaces remain visually distinguishable
/// without relying on shadow alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceElevation {
    /// Sits flat on the canvas — `secondarySystemBackground` with a hairline
    /// separator. Used for inline cards.
    Sunken,
    /// Default raised panel — `secondarySystemBackground` with a stronger
    /// border and rounded corners. Used for popovers and floating menus.
    Raised,
    /// Highest elevation — `tertiarySystemBackground`. Used for nested
    /// surfaces that must read above a [`Self::Raised`] parent.
    Floating,
}

#[derive(IntoElement)]
#[must_use]
pub struct Surface {
    elevation: SurfaceElevation,
    padding: Spacing,
    radius: Radius,
    appearance: Option<Appearance>,
    children: Vec<AnyElement>,
}

impl Surface {
    pub fn new(elevation: SurfaceElevation) -> Self {
        let radius = match elevation {
            SurfaceElevation::Sunken => Radius::MD,
            SurfaceElevation::Raised | SurfaceElevation::Floating => Radius::LG,
        };
        Self {
            elevation,
            padding: Spacing::MD,
            radius,
            appearance: None,
            children: Vec::new(),
        }
    }

    pub fn padding(mut self, padding: Spacing) -> Self {
        self.padding = padding;
        self
    }

    pub fn radius(mut self, radius: Radius) -> Self {
        self.radius = radius;
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl ParentElement for Surface {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Surface {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bg = match self.elevation {
            SurfaceElevation::Sunken | SurfaceElevation::Raised => {
                SemanticColor::SecondarySystemBackground
            }
            SurfaceElevation::Floating => SemanticColor::TertiarySystemBackground,
        };
        let border = match self.elevation {
            SurfaceElevation::Sunken => SemanticColor::Separator,
            SurfaceElevation::Raised | SurfaceElevation::Floating => SemanticColor::OpaqueSeparator,
        };

        let mut el: Div = div()
            .bg(resolve_color(cx, bg, self.appearance))
            .border_1()
            .border_color(resolve_color(cx, border, self.appearance))
            .rounded(self.radius.px())
            .p(self.padding.px());

        el.extend(self.children);
        el
    }
}
