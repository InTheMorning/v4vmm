pub mod image_cache;

pub use image_cache::ImageCache;

use crate::metadata::ImageBytes;
use gpui::{Image, ImageFormat};
use std::sync::Arc;

pub fn image_from_bytes(bytes: ImageBytes) -> Arc<Image> {
    let format = ImageFormat::from_mime_type(&bytes.mime_type).unwrap_or(ImageFormat::Jpeg);
    Arc::new(Image::from_bytes(format, bytes.data))
}
