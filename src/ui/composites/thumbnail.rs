//! Thumbnail composite — square artwork tile with a two-letter type
//! monogram fallback when no image is available (ADR 0083 Decision 4).
//!
//! Sizing is expressed as a semantic [`ThumbnailSize`] (Sm / Md / Lg) which
//! resolves through the global scale, so a "Large" thumbnail in a
//! detail header automatically grows when the user picks a larger UI scale.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, App, Image, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled, Window,
};

use crate::ui::primitives::Image as ImagePrimitive;
use crate::ui::tokens::{color, FontSize, Radius, ScaleFactor, SemanticColor};

use super::tag_badge::EntityKind;

/// Semantic thumbnail sizes. Base (1.0×) values follow Apple HIG list /
/// detail-row conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailSize {
    /// 32 px — list rows, search results.
    Sm,
    /// 48 px — sidebar and now-playing strips.
    Md,
    /// 80 px — detail-view header.
    Lg,
}

impl ThumbnailSize {
    fn base(self) -> f32 {
        match self {
            Self::Sm => 32.0,
            Self::Md => 48.0,
            Self::Lg => 80.0,
        }
    }

    /// Pick a [`ThumbnailSize`] from a legacy raw-pixel hint.
    ///
    /// Used while migrating call sites that still pass `(size: f32, large: bool)`
    /// pairs. The mapping mirrors the historical `render_thumb` helper:
    /// the explicit `large` flag wins, then `>= 64` is `Lg`, then
    /// `>= 40` is `Md`, else `Sm`.
    #[must_use]
    pub fn from_legacy_px(size: f32, large: bool) -> Self {
        if large || size >= 64.0 {
            Self::Lg
        } else if size >= 40.0 {
            Self::Md
        } else {
            Self::Sm
        }
    }

    /// ADR 0039: artwork sizing is geometry — this resolves through CHROME.
    pub(crate) fn scaled(self, cx: &App) -> Pixels {
        gpui::px(self.base() * ScaleFactor::current(cx).chrome_multiplier())
    }

    fn radius(self) -> Radius {
        match self {
            Self::Sm | Self::Md => Radius::SM,
            Self::Lg => Radius::MD,
        }
    }

    fn fallback_font(self) -> FontSize {
        match self {
            Self::Sm => FontSize::Body,
            Self::Md => FontSize::Headline,
            Self::Lg => FontSize::Title2,
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Thumbnail {
    image: Option<Arc<Image>>,
    kind: EntityKind,
    size: ThumbnailSize,
}

impl Thumbnail {
    pub fn new(kind: EntityKind, size: ThumbnailSize) -> Self {
        Self {
            image: None,
            kind,
            size,
        }
    }

    pub fn image(mut self, image: Option<Arc<Image>>) -> Self {
        self.image = image;
        self
    }

    /// The text shown in the fallback tile when no image is set.
    ///
    /// ADR 0083 Decision 4: a missing cover shows the two-letter monogram of
    /// its entity kind, not an emoji.
    fn fallback_label(&self) -> SharedString {
        SharedString::from(self.kind.monogram())
    }
}

impl RenderOnce for Thumbnail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let dim = self.size.scaled(cx);
        let radius = self.size.radius();
        let fallback_label = self.fallback_label();

        if let Some(image) = self.image {
            ImagePrimitive::new(image)
                .dimension(dim)
                .radius(radius)
                .into_any_element()
        } else {
            let radius_px = radius.scaled(cx);
            div()
                .w(dim)
                .h(dim)
                .rounded(radius_px)
                .overflow_hidden()
                .flex_shrink_0()
                .bg(color(cx, SemanticColor::SystemFill))
                .flex()
                .items_center()
                .justify_center()
                .text_size(self.size.fallback_font().scaled(cx))
                .child(fallback_label)
                .into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R83-15: `Thumbnail` with no image produces the monogram of its kind.
    #[test]
    fn thumbnail_fallback_shows_the_entity_kind_monogram() {
        for kind in [
            EntityKind::Artist,
            EntityKind::Feed,
            EntityKind::Track,
            EntityKind::Publisher,
            EntityKind::Release,
            EntityKind::Recording,
            EntityKind::Playlist,
            EntityKind::Generic,
        ] {
            let thumbnail = Thumbnail::new(kind, ThumbnailSize::Sm);
            assert_eq!(
                thumbnail.fallback_label(),
                SharedString::from(kind.monogram())
            );
        }
    }
}
