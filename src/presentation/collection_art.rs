//! Loads the art of a collection for the logo frame (ADR 0084 Decision 12).
//!
//! The view model gives a [`CollectionArtVm`]. This module loads its image
//! through the async command runner (ADR 0040), so no decode and no
//! composition runs on the UI thread. The UI thread only reads the cache
//! and the load state. The result is a [`CollectionArtImage`] for the
//! `CollectionArt` composite.

#![warn(clippy::pedantic)]

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Context, Image};

use crate::application::async_command_runner::AsyncCommandRunner;
use crate::application::command_context::CommandContext;
use crate::application::queries::images::{FetchLogoMosaic, FetchThumbnail};
use crate::media::logo_frame::LogoMosaicKey;
use crate::media::ImageCache;
use crate::presentation::present_command;
use crate::ui::composites::collection_art::LogoRaster;
use crate::ui::composites::{CollectionArtImage, ThumbnailSize};
use crate::view_models::collection_art::CollectionArtVm;

/// The load state of one image.
#[derive(Clone)]
enum ArtLoad {
    Loading,
    Loaded(Option<Arc<Image>>),
}

/// The result of one load request.
enum LoadResult {
    Ready(Arc<Image>),
    Pending,
    Failed,
}

/// The load states of the collection art of one window entity.
#[derive(Default)]
pub(crate) struct CollectionArtLoads {
    squares: HashMap<String, ArtLoad>,
    mosaics: HashMap<LogoMosaicKey, ArtLoad>,
}

/// The parts of a host that a load uses.
pub(crate) struct CollectionArtParts<'a> {
    pub(crate) loads: &'a mut CollectionArtLoads,
    pub(crate) cache: &'a Arc<ImageCache>,
    pub(crate) runner: &'a AsyncCommandRunner,
}

/// An entity that shows collection art.
pub(crate) trait CollectionArtHost: Sized + 'static {
    /// The load states, the image cache and the command runner.
    fn collection_art_parts(&mut self) -> CollectionArtParts<'_>;
}

/// The image state of `art` at `size`. It starts each load that is not
/// known, and returns the state that the loads have now.
///
/// - A provided image that does not load gives the image of its fallback.
/// - A single cover that does not load gives the entity color.
/// - A mosaic shows the entity color until the composition is ready, and
///   when no cover decodes.
pub(crate) fn collection_art_image<T: CollectionArtHost>(
    host: &mut T,
    art: &CollectionArtVm,
    size: ThumbnailSize,
    cx: &mut Context<T>,
) -> CollectionArtImage {
    match art {
        CollectionArtVm::ProvidedImage { url, .. } => match load_square(host, url, cx) {
            LoadResult::Ready(image) => CollectionArtImage::Square(image),
            LoadResult::Pending => CollectionArtImage::SquarePending,
            LoadResult::Failed => collection_art_image(host, &art.without_provided(), size, cx),
        },
        CollectionArtVm::SingleCover(url) => match load_square(host, url, cx) {
            LoadResult::Ready(image) => CollectionArtImage::Square(image),
            LoadResult::Pending => CollectionArtImage::SquarePending,
            LoadResult::Failed => CollectionArtImage::EntityColor,
        },
        CollectionArtVm::Mosaic(cells) => match load_mosaic(host, cells, size, cx) {
            LoadResult::Ready(image) => CollectionArtImage::Logo(image),
            LoadResult::Pending | LoadResult::Failed => CollectionArtImage::EntityColor,
        },
        CollectionArtVm::EntityColor(_) => CollectionArtImage::EntityColor,
    }
}

fn load_square<T: CollectionArtHost>(host: &mut T, url: &str, cx: &mut Context<T>) -> LoadResult {
    let parts = host.collection_art_parts();
    if let Some(image) = parts.cache.peek_static(url) {
        return LoadResult::Ready(image);
    }
    if let Some(state) = parts.loads.squares.get(url) {
        return state_result(state);
    }
    if parts.runner.availability().is_err() {
        return LoadResult::Failed;
    }
    parts.loads.squares.insert(url.to_owned(), ArtLoad::Loading);
    let command = FetchThumbnail::new(Arc::clone(parts.cache), url, false);
    let success_url = url.to_owned();
    let error_url = url.to_owned();
    present_command(
        parts.runner,
        command,
        CommandContext::next(),
        cx,
        move |this: &mut T, image, cx| {
            this.collection_art_parts()
                .loads
                .squares
                .insert(success_url, ArtLoad::Loaded(image));
            cx.notify();
        },
        move |this: &mut T, _, cx| {
            this.collection_art_parts()
                .loads
                .squares
                .insert(error_url, ArtLoad::Loaded(None));
            cx.notify();
        },
    );
    LoadResult::Pending
}

fn load_mosaic<T: CollectionArtHost>(
    host: &mut T,
    cells: &[String; 4],
    size: ThumbnailSize,
    cx: &mut Context<T>,
) -> LoadResult {
    let raster = LogoRaster::for_size(size, cx);
    let key = LogoMosaicKey::new(cells.clone(), raster.width_px, raster.shadow.is_some());
    let parts = host.collection_art_parts();
    if let Some(image) = parts.cache.peek_logo_mosaic(&key) {
        return LoadResult::Ready(image);
    }
    if let Some(state) = parts.loads.mosaics.get(&key) {
        return state_result(state);
    }
    if parts.runner.availability().is_err() {
        return LoadResult::Failed;
    }
    parts.loads.mosaics.insert(key.clone(), ArtLoad::Loading);
    let command = FetchLogoMosaic::new(Arc::clone(parts.cache), key.clone(), raster.shadow);
    let error_key = key.clone();
    present_command(
        parts.runner,
        command,
        CommandContext::next(),
        cx,
        move |this: &mut T, image, cx| {
            this.collection_art_parts()
                .loads
                .mosaics
                .insert(key, ArtLoad::Loaded(image));
            cx.notify();
        },
        move |this: &mut T, _, cx| {
            this.collection_art_parts()
                .loads
                .mosaics
                .insert(error_key, ArtLoad::Loaded(None));
            cx.notify();
        },
    );
    LoadResult::Pending
}

fn state_result(state: &ArtLoad) -> LoadResult {
    match state {
        ArtLoad::Loading => LoadResult::Pending,
        ArtLoad::Loaded(Some(image)) => LoadResult::Ready(Arc::clone(image)),
        ArtLoad::Loaded(None) => LoadResult::Failed,
    }
}
