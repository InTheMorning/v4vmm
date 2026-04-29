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

use crate::ui::tokens::{Radius, ScaleFactor};

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
    /// 200 px — track-inspector cover.
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

    /// Returns the size in pixels at the current global UI scale.
    #[must_use]
    pub fn scaled(self, cx: &App) -> Pixels {
        gpui::px(self.base() * ScaleFactor::current(cx).multiplier())
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
    /// `size.base() * ScaleFactor::current(cx).multiplier()`.
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

        div()
            .w(dim)
            .h(dim)
            .rounded(radius)
            .overflow_hidden()
            .flex_shrink_0()
            .child(inner)
    }
}
