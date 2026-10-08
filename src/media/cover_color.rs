//! The main color of a cover image (ADR 0083 Decision 4, task 005).
//!
//! The album page header draws a backdrop in this color. The value is a
//! plain sRGB triple with no renderer type, so a view model can carry it.

#![warn(clippy::pedantic)]

use image::RgbaImage;

/// The side of the square that the cover is reduced to before the count.
/// A small sample keeps the computation fast and makes it stable against
/// noise in the image.
const SAMPLE_SIDE: u32 = 32;
/// The bits of each channel that select a histogram bucket.
const BUCKET_BITS: u32 = 3;
/// A pixel with less alpha than this does not count.
const MIN_ALPHA: u8 = 128;

/// The main color of a cover, in sRGB.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct CoverColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// The main color of encoded image bytes. `None` when the bytes do not
/// decode or the image has no opaque pixel.
#[must_use]
pub fn cover_color(bytes: &[u8]) -> Option<CoverColor> {
    let image = image::load_from_memory(bytes).ok()?;
    let sample = image.thumbnail_exact(SAMPLE_SIDE, SAMPLE_SIDE).into_rgba8();
    main_color(&sample)
}

/// The main color of decoded pixels: the mean of the most frequent color
/// bucket. A tie selects the bucket with the lower index, so one image
/// always gives one value.
#[must_use]
pub fn main_color(pixels: &RgbaImage) -> Option<CoverColor> {
    let buckets = 1usize << (BUCKET_BITS * 3);
    let mut counts = vec![0u32; buckets];
    let mut sums = vec![[0u64; 3]; buckets];
    for pixel in pixels.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha < MIN_ALPHA {
            continue;
        }
        let index = bucket_index(red, green, blue);
        counts[index] += 1;
        sums[index][0] += u64::from(red);
        sums[index][1] += u64::from(green);
        sums[index][2] += u64::from(blue);
    }
    let (index, count) = counts
        .iter()
        .copied()
        .enumerate()
        .fold(
            (0, 0),
            |best, (index, count)| {
                if count > best.1 {
                    (index, count)
                } else {
                    best
                }
            },
        );
    if count == 0 {
        return None;
    }
    let mean = |sum: u64| u8::try_from(sum / u64::from(count)).unwrap_or(u8::MAX);
    Some(CoverColor {
        red: mean(sums[index][0]),
        green: mean(sums[index][1]),
        blue: mean(sums[index][2]),
    })
}

fn bucket_index(red: u8, green: u8, blue: u8) -> usize {
    let shift = 8 - BUCKET_BITS;
    (usize::from(red >> shift) << (BUCKET_BITS * 2))
        | (usize::from(green >> shift) << BUCKET_BITS)
        | usize::from(blue >> shift)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{Rgba, RgbaImage};

    use super::*;

    fn encoded_png(image: &RgbaImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("encode test image");
        bytes
    }

    /// R83-54: a known image gives one stable value.
    #[test]
    fn adr_0083_cover_color_of_a_known_image_is_stable() {
        let mut image = RgbaImage::from_pixel(40, 40, Rgba([200, 30, 40, 255]));
        for x in 0..10 {
            for y in 0..40 {
                image.put_pixel(x, y, Rgba([10, 10, 250, 255]));
            }
        }
        let bytes = encoded_png(&image);
        let first = cover_color(&bytes);
        assert_eq!(
            first,
            Some(CoverColor {
                red: 200,
                green: 30,
                blue: 40
            }),
            "the color that covers most of the image is the main color"
        );
        assert_eq!(cover_color(&bytes), first, "the value must be stable");
    }

    /// R83-54: an image with no pixels, or with no opaque pixel, gives no
    /// color. Bytes that do not decode give no color.
    #[test]
    fn adr_0083_cover_color_of_an_empty_image_is_none() {
        assert_eq!(main_color(&RgbaImage::new(0, 0)), None);
        assert_eq!(
            main_color(&RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 0]))),
            None
        );
        assert_eq!(cover_color(b"not an image"), None);
        assert_eq!(cover_color(&[]), None);
    }
}
