pub mod image_cache;
pub mod image_type;

pub use image_cache::ImageCache;

use crate::metadata::ImageBytes;
use gpui::{Image, ImageFormat};
use std::sync::Arc;

pub type CachedImage = Arc<Image>;

/// Decode already-classified image bytes for display.
///
/// Returns `None` for a type gpui cannot decode. ADR 0056: no path guesses a
/// format for unrecognized bytes, because guessing turns a clear failure into a
/// mystery.
pub fn image_from_bytes(bytes: ImageBytes) -> Option<CachedImage> {
    let format = ImageFormat::from_mime_type(&bytes.mime_type)?;
    Some(Arc::new(Image::from_bytes(format, bytes.data)))
}
