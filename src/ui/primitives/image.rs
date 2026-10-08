//! Image primitive — token-sized, scale-aware artwork tile.
//!
//! `Image` is the lowest-level renderer for raster artwork in the design
//! system. It owns:
//!
//! * a semantic size that resolves through the global [`Environment`] scale,
//! * a corner radius taken from the [`Radius`] token (defaults to `MD`),
//! * `ObjectFit::Cover` so the image fills its box without distortion, and
//! * the GIF-id stamping required by GPUI to drive frame ticking on animated
//!   images.
//!
//! Composites that need a *fallback when the image is missing* (e.g.
//! [`crate::ui::composites::Thumbnail`]) wrap this primitive — they don't
//! re-implement it.
//!
//! [`Environment`]: crate::ui::tokens::Environment
//! [`Radius`]: crate::ui::tokens::Radius

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, img, prelude::*, App, ImageFormat, IntoElement, ObjectFit, Pixels, RenderOnce,
    SharedString, Window,
};

use crate::ui::tokens::{ArtworkShadow, Radius, ScaleFactor};

/// Semantic image sizes. Base values follow the artwork conventions used
/// across the app (list rows, headers, large detail-view tiles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSize {
    /// 32 px — list rows, search results.
    Sm,
    /// 48 px — sidebar and now-playing strips.
    Md,
    /// 80 px — detail-view header artwork.
    Lg,
    /// 152 px — feed-detail hero artwork.
    Xl,
    /// 200 px — page-header cover.
    XXl,
}

impl ImageSize {
    fn base(self) -> f32 {
        match self {
            Self::Sm => 32.0,
            Self::Md => 48.0,
            Self::Lg => 80.0,
            Self::Xl => 152.0,
            Self::XXl => 200.0,
        }
    }

    /// The base size in pixels, before the UI scale.
    #[must_use]
    pub fn base_px(self) -> f32 {
        self.base()
    }

    /// Returns the size in pixels at the current global UI scale.
    ///
    /// ADR 0039: artwork sizing is geometry — this resolves through CHROME.
    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        gpui::px(self.base() * ScaleFactor::current(cx).chrome_multiplier())
    }

    /// The artwork shadow role for this `ImageSize`.
    ///
    /// ADR 0083 Decision 4: only `Xl` and `XXl` artwork draws a shadow.
    fn artwork_shadow(self) -> Option<ArtworkShadow> {
        match self {
            Self::Sm | Self::Md | Self::Lg => None,
            Self::Xl => Some(ArtworkShadow::Xl),
            Self::XXl => Some(ArtworkShadow::XXl),
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Image {
    handle: Arc<gpui::Image>,
    size: ImageSize,
    dimension: Option<Pixels>,
    radius: Radius,
}

impl Image {
    pub fn new(image: Arc<gpui::Image>) -> Self {
        Self {
            handle: image,
            size: ImageSize::Md,
            dimension: None,
            radius: Radius::MD,
        }
    }

    /// Pick a [`ImageSize`] preset; the rendered dimension is
    /// `size.base() * ScaleFactor::current(cx).chrome_multiplier()`.
    pub fn size(mut self, size: ImageSize) -> Self {
        self.size = size;
        self.dimension = None;
        self
    }

    /// Force an exact dimension (overrides [`Self::size`]). Composites that
    /// have already computed a scaled `Pixels` value pass it here.
    pub fn dimension(mut self, dimension: Pixels) -> Self {
        self.dimension = Some(dimension);
        self
    }

    pub fn radius(mut self, radius: Radius) -> Self {
        self.radius = radius;
        self
    }
}

impl RenderOnce for Image {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let dim = self.dimension.unwrap_or_else(|| self.size.scaled(cx));
        let radius = self.radius.scaled(cx);
        let shadow = self.size.artwork_shadow().map(|role| role.shadow(cx));
        let image = self.handle;

        let inner = img(image.clone())
            .w(dim)
            .h(dim)
            .object_fit(ObjectFit::Cover);
        let inner = if image.format == ImageFormat::Gif {
            // Animated GIFs need a stable id to drive frame ticking.
            inner
                .id(SharedString::from(format!("anim-img:{}", image.id())))
                .into_any_element()
        } else {
            inner.into_any_element()
        };

        let mut container = div()
            .w(dim)
            .h(dim)
            .rounded(radius)
            .overflow_hidden()
            .flex_shrink_0();
        if let Some(shadow) = shadow {
            container = container.shadow(shadow);
        }
        container.child(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0083 Decision 4: only `Xl` and `XXl` artwork draws a shadow. No
    /// other `ImageSize` draws one.
    #[test]
    fn only_xl_and_xxl_resolve_an_artwork_shadow() {
        assert_eq!(ImageSize::Sm.artwork_shadow(), None);
        assert_eq!(ImageSize::Md.artwork_shadow(), None);
        assert_eq!(ImageSize::Lg.artwork_shadow(), None);
        assert_eq!(ImageSize::Xl.artwork_shadow(), Some(ArtworkShadow::Xl));
        assert_eq!(ImageSize::XXl.artwork_shadow(), Some(ArtworkShadow::XXl));
    }
}
