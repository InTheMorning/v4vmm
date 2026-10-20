//! Collection art composite: the logo frame of a singles group, an artist
//! page or a playlist (ADR 0084 Decision 12, ADR 0083 Decision 4).
//!
//! The view model gives the art choice ([`CollectionArtVm`]). The app layer
//! loads the image of that choice off the UI thread and gives this
//! composite a [`CollectionArtImage`]. This composite only draws it. Each
//! size keeps a square slot, so a row stays aligned and a page header does
//! not move when the art changes from a square to the logo.
//!
//! [`CollectionArtVm`]: crate::view_models::collection_art::CollectionArtVm

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, img, px, svg, App, Image, IntoElement, ObjectFit, ParentElement, Pixels, RenderOnce, Rgba,
    Styled, StyledImage, Window,
};

use crate::media::logo_frame::{canvas_size_px, frame_height_px, BakedShadow};
use crate::ui::icons::LOGO_FRAME_ASSET_PATH;
use crate::ui::tokens::{ArtworkShadow, ArtworkShape};
use crate::view_models::collection_art::CollectionKind;

use super::tag_badge::EntityKind;
use super::thumbnail::{Thumbnail, ThumbnailSize};

/// The image state of one collection art, after the app layer loaded it.
#[derive(Clone, Debug)]
pub enum CollectionArtImage {
    /// A provided image or one cover, shown as a square cover.
    Square(Arc<Image>),
    /// The composed logo mosaic.
    Logo(Arc<Image>),
    /// A square image that still loads. It shows the entity color with no
    /// monogram.
    SquarePending,
    /// No cover, a cover that did not load, or a mosaic that still loads.
    /// It shows the logo in the entity color.
    EntityColor,
}

/// The raster request of a composed logo at one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LogoRaster {
    /// The width of the logo in device pixels.
    pub(crate) width_px: u32,
    /// The baked shadow. Only the `XXl` size has one.
    pub(crate) shadow: Option<BakedShadow>,
}

impl LogoRaster {
    /// The raster request of a logo of `size` at the current UI scale.
    #[must_use]
    pub(crate) fn for_size(size: ThumbnailSize, cx: &App) -> Self {
        let width = f32::from(size.scaled(cx)) * ArtworkShape::RASTER_DENSITY;
        let shadow = matches!(size, ThumbnailSize::XXl)
            .then(|| ArtworkShadow::XXl.baked(cx, ArtworkShape::RASTER_DENSITY));
        Self {
            width_px: device_px(width),
            shadow,
        }
    }
}

/// Rounds a device width to whole pixels, one pixel at least.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a scaled artwork width is a small positive pixel count"
)]
fn device_px(width: f32) -> u32 {
    width.round().max(1.0) as u32
}

/// The entity kind whose color a collection uses.
#[must_use]
pub(crate) const fn collection_entity_kind(kind: CollectionKind) -> EntityKind {
    match kind {
        CollectionKind::SinglesGroup => EntityKind::Release,
        CollectionKind::Artist => EntityKind::Artist,
        CollectionKind::Playlist => EntityKind::Playlist,
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct CollectionArt {
    image: CollectionArtImage,
    kind: CollectionKind,
    size: ThumbnailSize,
}

impl CollectionArt {
    pub(crate) fn new(
        image: CollectionArtImage,
        kind: CollectionKind,
        size: ThumbnailSize,
    ) -> Self {
        Self { image, kind, size }
    }
}

/// Converts device pixels of a composed image to logical pixels.
#[expect(
    clippy::cast_precision_loss,
    reason = "a composed image is a few hundred pixels wide"
)]
fn logical(device_px: u32) -> Pixels {
    px(device_px as f32 / ArtworkShape::RASTER_DENSITY)
}

impl RenderOnce for CollectionArt {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let dim = self.size.scaled(cx);
        let entity = collection_entity_kind(self.kind);
        let fill = Rgba {
            a: ArtworkShape::ENTITY_FILL_OPACITY,
            ..entity.fill_color(cx)
        };
        let slot = div()
            .w(dim)
            .h(dim)
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center();
        match self.image {
            CollectionArtImage::Square(image) => slot
                .child(Thumbnail::new(entity, self.size).image(Some(image)))
                .into_any_element(),
            CollectionArtImage::SquarePending => slot
                .child(
                    div()
                        .w(dim)
                        .h(dim)
                        .rounded(self.size.radius().scaled(cx))
                        .bg(fill),
                )
                .into_any_element(),
            CollectionArtImage::EntityColor => slot
                .child(
                    svg()
                        .path(LOGO_FRAME_ASSET_PATH)
                        .w(dim)
                        .h(dim / ArtworkShape::LogoFrame.aspect())
                        .text_color(fill),
                )
                .into_any_element(),
            CollectionArtImage::Logo(image) => {
                let raster = LogoRaster::for_size(self.size, cx);
                let padding = logical(raster.shadow.map_or(0, BakedShadow::padding_px));
                let (canvas_width, canvas_height) = canvas_size_px(raster.width_px, raster.shadow);
                slot.child(
                    div()
                        .relative()
                        .w(logical(raster.width_px))
                        .h(logical(frame_height_px(raster.width_px)))
                        .child(
                            img(image)
                                .absolute()
                                .left(-padding)
                                .top(-padding)
                                .w(logical(canvas_width))
                                .h(logical(canvas_height))
                                .object_fit(ObjectFit::Fill),
                        ),
                )
                .into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0084 Decision 12: each collection kind uses its entity color.
    #[test]
    fn adr_0084_logo_frame_kinds_use_their_entity_colors() {
        assert_eq!(
            collection_entity_kind(CollectionKind::Artist),
            EntityKind::Artist
        );
        assert_eq!(
            collection_entity_kind(CollectionKind::SinglesGroup),
            EntityKind::Release
        );
        assert_eq!(
            collection_entity_kind(CollectionKind::Playlist),
            EntityKind::Playlist
        );
    }
}
