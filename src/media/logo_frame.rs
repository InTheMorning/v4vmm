//! The `MusicIndex` logo frame of a collection (ADR 0084 Decision 12).
//!
//! This module holds the one copy of the logo shape: [`LOGO_FRAME_POLYGONS`].
//! The token `ArtworkShape::LogoFrame` and the SVG asset of the entity color
//! fallback use this constant. No other file holds the numbers.
//!
//! GPUI cannot clip an image to a path. Thus the image cache composes the
//! covers of a collection into one image off the UI thread. The three
//! polygons give the alpha of that image, with 4 by 4 supersampling. This
//! module has no renderer type, because `src/media` does not import `src/ui`.

#![warn(clippy::pedantic)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel geometry converts between bounded pixel counts and f32 coordinates"
)]

use std::fmt::Write as _;
use std::io::Cursor;

use image::{imageops, DynamicImage, GrayImage, Luma, Rgba, RgbaImage};

/// The three polygons of the logo, as fractions of its bounding box.
///
/// The values are the `mi-mark-clip` clip path of `../musicindex/search.html`.
/// The bounding box is 735 by 681 units, not a square.
pub(crate) const LOGO_FRAME_POLYGONS: [&[(f32, f32)]; 3] = [
    &[
        (0.0, 0.1674),
        (0.0612, 0.1674),
        (0.1592, 0.2115),
        (0.1592, 1.0),
        (0.0, 1.0),
    ],
    &[
        (0.2816, 0.0),
        (0.4857, 0.0),
        (0.7143, 0.1762),
        (0.7143, 1.0),
        (0.5633, 1.0),
        (0.5633, 0.3348),
        (0.4408, 0.2511),
        (0.4408, 1.0),
        (0.2816, 1.0),
    ],
    &[
        (1.0, 0.1322),
        (1.0, 0.9251),
        (0.8531, 1.0),
        (0.8408, 1.0),
        (0.8408, 0.2115),
        (0.9878, 0.1322),
    ],
];

/// The width of the logo bounding box, in logo units.
const LOGO_FRAME_WIDTH_UNITS: f32 = 735.0;
/// The height of the logo bounding box, in logo units.
const LOGO_FRAME_HEIGHT_UNITS: f32 = 681.0;

/// The width of the logo divided by its height.
pub(crate) const LOGO_FRAME_ASPECT: f32 = LOGO_FRAME_WIDTH_UNITS / LOGO_FRAME_HEIGHT_UNITS;

/// The version of the shape. A change to the polygons increments it, so an
/// old composed image never matches a new key.
pub(crate) const LOGO_FRAME_SHAPE_VERSION: u8 = 1;

/// The samples along one axis of one pixel. 4 by 4 gives 16 samples.
const SUPERSAMPLES: u32 = 4;

/// The repeat rule of the mosaic (`mosaicCells` of `search.html`).
///
/// Two items give A B / B A. Three items give A B / C A. Four or more give
/// the first four. Fewer than two items give `None`, because one item is
/// not a mosaic.
#[must_use]
pub(crate) fn repeat_cells<T: Clone>(items: &[T]) -> Option<[T; 4]> {
    match items {
        [] | [_] => None,
        [a, b] => Some([a.clone(), b.clone(), b.clone(), a.clone()]),
        [a, b, c] => Some([a.clone(), b.clone(), c.clone(), a.clone()]),
        [a, b, c, d, ..] => Some([a.clone(), b.clone(), c.clone(), d.clone()]),
    }
}

/// The key of one composed logo mosaic.
///
/// The cells come after the repeat rule, so equal art gives one key. The
/// order of the cells is part of the key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LogoMosaicKey {
    /// The cover URL of each cell: top left, top right, bottom left and
    /// bottom right.
    pub cells: [String; 4],
    /// The width of the logo in device pixels.
    pub width_px: u32,
    /// The image includes the baked artwork shadow.
    pub shadow: bool,
    /// The version of the logo shape.
    pub shape_version: u8,
}

impl LogoMosaicKey {
    /// Makes the key of the current shape version.
    #[must_use]
    pub fn new(cells: [String; 4], width_px: u32, shadow: bool) -> Self {
        Self {
            cells,
            width_px,
            shadow,
            shape_version: LOGO_FRAME_SHAPE_VERSION,
        }
    }

    /// Each different cover URL of the cells, in the order of the cells.
    #[must_use]
    pub fn distinct_cells(&self) -> Vec<&str> {
        let mut distinct: Vec<&str> = Vec::with_capacity(4);
        for cell in &self.cells {
            if !distinct.contains(&cell.as_str()) {
                distinct.push(cell);
            }
        }
        distinct
    }
}

/// The artwork shadow that the composition bakes into the image, in device
/// pixels. The token `ArtworkShadow` gives the values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BakedShadow {
    /// The blur radius.
    pub blur_px: f32,
    /// The vertical offset.
    pub offset_y_px: f32,
    /// The opacity of the black shadow.
    pub alpha: f32,
}

impl BakedShadow {
    /// The space on each side of the logo that holds the shadow.
    #[must_use]
    pub fn padding_px(self) -> u32 {
        (self.blur_px + self.offset_y_px).max(0.0).ceil() as u32
    }
}

/// The height of the logo, in device pixels, for a width in device pixels.
#[must_use]
pub fn frame_height_px(width_px: u32) -> u32 {
    ((width_px as f32 / LOGO_FRAME_ASPECT).round() as u32).max(1)
}

/// The size of a composed image: the logo and the shadow space on each side.
#[must_use]
pub fn canvas_size_px(width_px: u32, shadow: Option<BakedShadow>) -> (u32, u32) {
    let padding = shadow.map_or(0, BakedShadow::padding_px);
    (
        width_px + 2 * padding,
        frame_height_px(width_px) + 2 * padding,
    )
}

/// The SVG document of the logo shape, for the entity color fallback. A
/// renderer tints its one color.
#[must_use]
pub fn logo_frame_svg() -> String {
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {LOGO_FRAME_WIDTH_UNITS} {LOGO_FRAME_HEIGHT_UNITS}\">"
    );
    for polygon in LOGO_FRAME_POLYGONS {
        svg.push_str("<path fill=\"#000\" d=\"");
        for (index, (x, y)) in polygon.iter().enumerate() {
            let command = if index == 0 { 'M' } else { 'L' };
            let _ = write!(
                svg,
                "{command}{} {} ",
                x * LOGO_FRAME_WIDTH_UNITS,
                y * LOGO_FRAME_HEIGHT_UNITS
            );
        }
        svg.push_str("Z\"/>");
    }
    svg.push_str("</svg>");
    svg
}

/// Composes the covers into the logo frame.
///
/// `covers` holds each cover that decoded, without repeats. Two or more
/// covers fill the 2 by 2 mosaic through the repeat rule. One cover fills
/// the logo. No cover gives `None`. Each cover is cropped to fill its cell.
/// With `shadow`, the image holds the shadow in the space around the logo.
#[must_use]
pub fn compose_logo_mosaic(
    covers: &[DynamicImage],
    width_px: u32,
    shadow: Option<BakedShadow>,
) -> Option<RgbaImage> {
    if covers.is_empty() || width_px == 0 {
        return None;
    }
    let height_px = frame_height_px(width_px);
    let art = cell_art(covers, width_px, height_px);
    let coverage = coverage_mask(width_px, height_px);
    let padding = shadow.map_or(0, BakedShadow::padding_px);
    let (canvas_width, canvas_height) = canvas_size_px(width_px, shadow);
    let shadow_layer = shadow.map(|shadow| {
        shadow_layer(
            &coverage,
            (width_px, height_px),
            (canvas_width, canvas_height),
            padding,
            shadow,
        )
    });

    let mut canvas = RgbaImage::new(canvas_width, canvas_height);
    for (x, y, pixel) in canvas.enumerate_pixels_mut() {
        let shadow_alpha = shadow_layer.as_ref().map_or(0.0, |(layer, alpha)| {
            f32::from(layer.get_pixel(x, y).0[0]) / 255.0 * alpha
        });
        let (art_rgb, art_alpha) = match (x.checked_sub(padding), y.checked_sub(padding)) {
            (Some(ax), Some(ay)) if ax < width_px && ay < height_px => {
                let source = art.get_pixel(ax, ay).0;
                let mask = f32::from(coverage[(ay * width_px + ax) as usize]) / 255.0;
                (
                    [source[0], source[1], source[2]],
                    mask * f32::from(source[3]) / 255.0,
                )
            }
            _ => ([0, 0, 0], 0.0),
        };
        // Source-over: the art over the black shadow.
        let out_alpha = art_alpha + shadow_alpha * (1.0 - art_alpha);
        if out_alpha <= 0.0 {
            continue;
        }
        let channel = |value: u8| (f32::from(value) * art_alpha / out_alpha).round() as u8;
        *pixel = Rgba([
            channel(art_rgb[0]),
            channel(art_rgb[1]),
            channel(art_rgb[2]),
            (out_alpha * 255.0).round() as u8,
        ]);
    }
    Some(canvas)
}

/// Encodes a composed image as PNG.
#[must_use]
pub fn encode_png(image: &RgbaImage) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .ok()?;
    Some(bytes)
}

/// The unmasked art of the logo bounding box: one cover, or four cells.
fn cell_art(covers: &[DynamicImage], width_px: u32, height_px: u32) -> RgbaImage {
    let Some(cells) = repeat_cells(covers) else {
        return fill(&covers[0], width_px, height_px);
    };
    let left_width = (width_px / 2).max(1);
    let top_height = (height_px / 2).max(1);
    let right_width = (width_px - left_width).max(1);
    let bottom_height = (height_px - top_height).max(1);
    let mut art = RgbaImage::new(width_px, height_px);
    let places = [
        (0, 0, left_width, top_height),
        (left_width, 0, right_width, top_height),
        (0, top_height, left_width, bottom_height),
        (left_width, top_height, right_width, bottom_height),
    ];
    for (cover, (x, y, width, height)) in cells.iter().zip(places) {
        let cell = fill(cover, width, height);
        imageops::replace(&mut art, &cell, i64::from(x), i64::from(y));
    }
    art
}

/// Crops a cover to fill `width` by `height`, with no distortion.
fn fill(cover: &DynamicImage, width: u32, height: u32) -> RgbaImage {
    cover
        .resize_to_fill(width, height, imageops::FilterType::Triangle)
        .to_rgba8()
}

/// The alpha of the logo for each pixel of the bounding box, 0 to 255.
///
/// Each pixel takes 4 by 4 samples. A sample is in the logo when a
/// horizontal ray from it crosses the polygon edges an odd number of times.
fn coverage_mask(width_px: u32, height_px: u32) -> Vec<u8> {
    let samples = SUPERSAMPLES * SUPERSAMPLES;
    let mut counts = vec![0_u32; (width_px * height_px) as usize];
    let step = 1.0 / SUPERSAMPLES as f32;
    let mut crossings: Vec<f32> = Vec::new();
    for y in 0..height_px {
        for sub_y in 0..SUPERSAMPLES {
            let sample_y = (y as f32 + (sub_y as f32 + 0.5) * step) / height_px as f32;
            crossings.clear();
            for polygon in LOGO_FRAME_POLYGONS {
                for (index, &(x0, y0)) in polygon.iter().enumerate() {
                    let (x1, y1) = polygon[(index + 1) % polygon.len()];
                    if (y0 > sample_y) != (y1 > sample_y) {
                        let x = x0 + (sample_y - y0) * (x1 - x0) / (y1 - y0);
                        crossings.push(x * width_px as f32);
                    }
                }
            }
            crossings.sort_by(f32::total_cmp);
            let mut passed = 0;
            for x in 0..width_px {
                for sub_x in 0..SUPERSAMPLES {
                    let sample_x = x as f32 + (sub_x as f32 + 0.5) * step;
                    while passed < crossings.len() && crossings[passed] < sample_x {
                        passed += 1;
                    }
                    if passed % 2 == 1 {
                        counts[(y * width_px + x) as usize] += 1;
                    }
                }
            }
        }
    }
    counts
        .into_iter()
        .map(|count| ((count * 255 + samples / 2) / samples) as u8)
        .collect()
}

/// The blurred shadow of the logo on the canvas, and its opacity.
fn shadow_layer(
    coverage: &[u8],
    (width_px, height_px): (u32, u32),
    (canvas_width, canvas_height): (u32, u32),
    padding: u32,
    shadow: BakedShadow,
) -> (GrayImage, f32) {
    let offset = shadow.offset_y_px.round().max(0.0) as u32;
    let mut layer = GrayImage::new(canvas_width, canvas_height);
    for y in 0..height_px {
        for x in 0..width_px {
            let target_y = y + padding + offset;
            if target_y < canvas_height {
                layer.put_pixel(
                    x + padding,
                    target_y,
                    Luma([coverage[(y * width_px + x) as usize]]),
                );
            }
        }
    }
    // A CSS blur radius is two standard deviations.
    let sigma = shadow.blur_px / 2.0;
    let blurred = if sigma > 0.0 {
        imageops::blur(&layer, sigma)
    } else {
        layer
    };
    (blurred, shadow.alpha.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recorded colors of the 2 by 2 fixture: red, green, blue, white.
    const FIXTURE_COLORS: [[u8; 4]; 4] = [
        [200, 30, 30, 255],
        [30, 200, 30, 255],
        [30, 30, 200, 255],
        [240, 240, 240, 255],
    ];

    fn solid(color: [u8; 4]) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, Rgba(color)))
    }

    fn fixture() -> Vec<DynamicImage> {
        FIXTURE_COLORS.into_iter().map(solid).collect()
    }

    fn pixel_at(image: &RgbaImage, padding: u32, width_px: u32, x: f32, y: f32) -> Rgba<u8> {
        let height_px = frame_height_px(width_px);
        let px = (x * width_px as f32) as u32 + padding;
        let py = (y * height_px as f32) as u32 + padding;
        *image.get_pixel(px, py)
    }

    /// R11-08: the gap between the first two pillars is transparent, and
    /// the first pillar is opaque, in the color of its cell.
    #[test]
    fn adr_0084_logo_frame_r11_08_alpha_follows_the_pillars() {
        let image = compose_logo_mosaic(&fixture(), 64, None).expect("the fixture composes");
        assert_eq!(image.dimensions(), (64, frame_height_px(64)));

        let gap = pixel_at(&image, 0, 64, 0.22, 0.5);
        assert_eq!(
            gap.0[3], 0,
            "the gap between the pillars shows the background"
        );

        let pillar = pixel_at(&image, 0, 64, 0.08, 0.6);
        assert_eq!(pillar.0[3], 255, "a pillar is opaque");
        assert_eq!(
            [pillar.0[0], pillar.0[1], pillar.0[2]],
            [30, 30, 200],
            "the bottom left cell holds the third cover"
        );
    }

    /// R11-08: with the baked shadow, the image grows by the shadow space,
    /// the gap stays clear above the shadow, and the pillar stays opaque.
    #[test]
    fn adr_0084_logo_frame_r11_08_shadow_adds_space_and_keeps_the_pillars() {
        let shadow = BakedShadow {
            blur_px: 8.0,
            offset_y_px: 2.0,
            alpha: 0.34,
        };
        let image =
            compose_logo_mosaic(&fixture(), 64, Some(shadow)).expect("the fixture composes");
        let padding = shadow.padding_px();
        assert_eq!(padding, 10);
        assert_eq!(image.dimensions(), canvas_size_px(64, Some(shadow)));
        assert_eq!(pixel_at(&image, padding, 64, 0.08, 0.6).0[3], 255);
        assert_eq!(image.get_pixel(0, 0).0[3], 0, "a far corner stays clear");
    }

    /// R11-09: no cover gives no image.
    #[test]
    fn adr_0084_logo_frame_r11_09_no_cover_gives_none() {
        assert!(compose_logo_mosaic(&[], 64, None).is_none());
    }

    /// One cover fills the whole logo.
    #[test]
    fn adr_0084_logo_frame_one_cover_fills_the_logo() {
        let image =
            compose_logo_mosaic(&[solid(FIXTURE_COLORS[0])], 64, None).expect("one cover composes");
        let left = pixel_at(&image, 0, 64, 0.08, 0.6);
        let right = pixel_at(&image, 0, 64, 0.92, 0.5);
        assert_eq!(left, right);
        assert_eq!(left.0, FIXTURE_COLORS[0]);
    }

    /// R11-01: the repeat rule of `search.html`.
    #[test]
    fn adr_0084_logo_frame_r11_01_repeat_rule() {
        assert_eq!(repeat_cells::<u8>(&[]), None);
        assert_eq!(repeat_cells(&[1]), None);
        assert_eq!(repeat_cells(&[1, 2]), Some([1, 2, 2, 1]));
        assert_eq!(repeat_cells(&[1, 2, 3]), Some([1, 2, 3, 1]));
        assert_eq!(repeat_cells(&[1, 2, 3, 4, 5]), Some([1, 2, 3, 4]));
    }

    /// R11-07: equal cells and an equal size give one key. A changed cover
    /// order gives a different key.
    #[test]
    fn adr_0084_logo_frame_r11_07_key_identity() {
        let cells = |order: [&str; 4]| order.map(str::to_owned);
        let first = LogoMosaicKey::new(cells(["a", "b", "b", "a"]), 64, false);
        let same = LogoMosaicKey::new(cells(["a", "b", "b", "a"]), 64, false);
        let reordered = LogoMosaicKey::new(cells(["b", "a", "a", "b"]), 64, false);
        let wider = LogoMosaicKey::new(cells(["a", "b", "b", "a"]), 128, false);
        assert_eq!(first, same);
        assert_ne!(first, reordered);
        assert_ne!(first, wider);
        assert_eq!(first.distinct_cells(), vec!["a", "b"]);
    }

    /// The SVG asset carries the three polygons and the bounding box.
    #[test]
    fn adr_0084_logo_frame_svg_has_three_paths() {
        let svg = logo_frame_svg();
        assert_eq!(svg.matches("<path").count(), 3);
        assert!(svg.contains("viewBox=\"0 0 735 681\""));
    }
}
