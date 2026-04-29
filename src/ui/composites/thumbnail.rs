//! Thumbnail composite — square artwork tile with an emoji fallback when
//! no image is available.
//!
//! Sizing is expressed as a semantic [`ThumbnailSize`] (Sm / Md / Lg) which
//! resolves through the global scale, so a "Large" thumbnail in a
//! detail header automatically grows when the user picks a larger UI scale.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, img, prelude::*, App, Image, ImageFormat, IntoElement, ObjectFit, ParentElement, Pixels,
    RenderOnce, SharedString, Styled, Window,
};

use crate::ui::tokens::{FontSize, Radius, ScaleFactor, SemanticColor};

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

    fn scaled(self, cx: &App) -> Pixels {
        gpui::px(self.base() * ScaleFactor::current(cx).multiplier())
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
}

impl RenderOnce for Thumbnail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let dim = self.size.scaled(cx);
        let radius = self.size.radius().scaled(cx);

        let base = div()
            .w(dim)
            .h(dim)
            .rounded(radius)
            .overflow_hidden()
            .flex_shrink_0();

        if let Some(image) = self.image {
            let inner = img(image.clone())
                .w(dim)
                .h(dim)
                .object_fit(ObjectFit::Cover);
            let inner = if image.format == ImageFormat::Gif {
                // Animated GIFs need a stable id to drive frame ticking.
                inner
                    .id(SharedString::from(format!("anim-thumb:{}", image.id())))
                    .into_any_element()
            } else {
                inner.into_any_element()
            };
            base.child(inner)
        } else {
            base.bg(SemanticColor::SystemFill.resolve(crate::ui::tokens::Appearance::current(cx)))
                .flex()
                .items_center()
                .justify_center()
                .text_size(self.size.fallback_font().scaled(cx))
                .child(SharedString::from(self.kind.emoji()))
        }
    }
}
