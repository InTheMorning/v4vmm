//! Placeholder primitive for rows that are not yet loaded.
//!
//! `Skeleton` is the design-system answer to "we asked the runtime for this
//! row but the page has not arrived yet". It is a token-driven, muted block
//! that occupies the row's footprint so the surrounding list does not jump
//! when the real content materializes.
//!
//! Apple HIG note: the macOS / iOS analogue is `.redacted(reason: .placeholder)`
//! — a low-contrast filled surface, no shimmer animation. We intentionally
//! avoid animated shimmer so the placeholder reads as "still loading" without
//! competing with the surrounding content for attention.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, px, App, IntoElement, Pixels, RenderOnce, Window};

use crate::ui::tokens::{resolve_color, Appearance, Radius, SemanticColor, Spacing};

/// Default height of a single text-row skeleton, matching the body text
/// line-height used throughout the library list views.
const DEFAULT_ROW_HEIGHT: f32 = 16.0;

/// Default width of a single text-row skeleton. Rows in lists usually
/// stretch via flex; this is just a sensible fallback when the parent does
/// not constrain the width.
const DEFAULT_ROW_WIDTH: f32 = 160.0;

/// Token-driven placeholder block used while paged rows are loading.
#[derive(IntoElement)]
#[must_use]
pub struct Skeleton {
    width: Pixels,
    height: Pixels,
    radius: Radius,
    appearance: Option<Appearance>,
    full_width: bool,
}

impl Skeleton {
    /// Builds a skeleton block with explicit `width` × `height` in pixels.
    pub fn block(width: Pixels, height: Pixels) -> Self {
        Self {
            width,
            height,
            radius: Radius::SM,
            appearance: None,
            full_width: false,
        }
    }

    /// Builds a single-text-row skeleton sized to match body text height.
    pub fn row() -> Self {
        Self::block(px(DEFAULT_ROW_WIDTH), px(DEFAULT_ROW_HEIGHT))
    }

    /// Lets the skeleton stretch to fill its parent's available width.
    /// Useful when rendered inside a flex row that owns the horizontal axis.
    pub const fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Override the corner radius. Defaults to [`Radius::SM`] which matches
    /// chip / badge corners.
    pub const fn radius(mut self, radius: Radius) -> Self {
        self.radius = radius;
        self
    }

    /// Override the resolved appearance. Mirrors the helper on
    /// [`super::LoadingMessage`] for consistency across loading affordances.
    pub const fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let fill = resolve_color(cx, SemanticColor::TertiaryFill, self.appearance);
        let mut block = div()
            .h(self.height)
            .bg(fill)
            .rounded(self.radius.scaled(cx))
            // A small vertical gutter so consecutive skeleton rows do not
            // touch and read as a single bar.
            .my(Spacing::XXS.scaled(cx));

        if self.full_width {
            block = block.flex_1();
        } else {
            block = block.w(self.width);
        }

        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_uses_explicit_dimensions() {
        let skeleton = Skeleton::block(px(120.0), px(20.0));
        assert_eq!(skeleton.width, px(120.0));
        assert_eq!(skeleton.height, px(20.0));
        assert!(!skeleton.full_width);
    }

    #[test]
    fn row_uses_text_height_default() {
        let skeleton = Skeleton::row();
        assert_eq!(skeleton.height, px(DEFAULT_ROW_HEIGHT));
        assert_eq!(skeleton.width, px(DEFAULT_ROW_WIDTH));
    }

    #[test]
    fn full_width_flag_flips() {
        let skeleton = Skeleton::row().full_width();
        assert!(skeleton.full_width);
    }

    #[test]
    fn radius_override_is_recorded() {
        let skeleton = Skeleton::row().radius(Radius::MD);
        assert_eq!(skeleton.radius, Radius::MD);
    }

    #[test]
    fn appearance_override_is_recorded() {
        let skeleton = Skeleton::row().appearance(Appearance::Dark);
        assert_eq!(skeleton.appearance, Some(Appearance::Dark));
    }

    #[test]
    fn defaults_have_no_appearance_override() {
        let skeleton = Skeleton::row();
        assert!(skeleton.appearance.is_none());
        assert_eq!(skeleton.radius, Radius::SM);
    }
}
