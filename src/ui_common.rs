//! Legacy `ui_common` helpers — kept as a thin compatibility layer above the
//! new [`crate::ui::composites`] components.
//!
//! New code should use the composites directly. Once `library.rs` and
//! `search.rs` finish migrating off these helpers (Track G) this module can
//! be deleted entirely.
//!
//! Every helper here delegates to a composite under the hood so that existing
//! call sites (the inspector panels, search results, library lists) become
//! scale-aware automatically — without each call site having to thread `cx`
//! through.

use crate::ui::composites::{DetailGrid, DetailRow as CompositeDetailRow, EntityKind};
use crate::ui::theme::{badges, color, radius, spacing, typography};
use gpui::{
    div, img, prelude::*, px, AnyElement, Div, FontWeight, Image, ImageFormat, IntoElement,
    ObjectFit, ParentElement, SharedString, Styled,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{Sizable, Size};
use std::sync::Arc;

/// Legacy detail row shape — the value is a pre-rendered `AnyElement` so
/// callers can build rich content. Kept identical to the original shape
/// for source-compatibility with library.rs / search.rs.
pub struct DetailRow {
    pub key: String,
    pub value: AnyElement,
}

impl From<DetailRow> for CompositeDetailRow {
    fn from(row: DetailRow) -> Self {
        Self {
            key: SharedString::from(row.key),
            value: row.value,
        }
    }
}

pub fn artwork_img(image: Arc<Image>, size: f32) -> AnyElement {
    let base = img(image.clone())
        .w(px(size))
        .h(px(size))
        .object_fit(ObjectFit::Cover);
    if image.format == ImageFormat::Gif {
        base.id(SharedString::from(format!("anim-thumb:{}", image.id())))
            .into_any_element()
    } else {
        base.into_any_element()
    }
}

/// Detail header — preserves the legacy positional API but delegates to
/// the [`crate::ui::composites::DetailHeader`] composite.
pub fn render_detail_header(
    entity_type: &str,
    title: &str,
    subtitle: Option<&str>,
    image: Option<Arc<Image>>,
) -> AnyElement {
    let kind = EntityKind::from_legacy_str(entity_type);
    let header = crate::ui::composites::DetailHeader::new(kind, title.to_string()).image(image);
    if let Some(sub) = subtitle {
        header.subtitle(sub.to_string()).into_any_element()
    } else {
        header.into_any_element()
    }
}

pub fn render_detail_grid(rows: Vec<(String, String)>) -> AnyElement {
    let composite_rows: Vec<CompositeDetailRow> = rows
        .into_iter()
        .map(|(key, value)| CompositeDetailRow::text(key, value, 6))
        .collect();
    DetailGrid::new(composite_rows).into_any_element()
}

pub fn render_detail_grid_elements(rows: Vec<DetailRow>) -> AnyElement {
    let composite_rows: Vec<CompositeDetailRow> = rows.into_iter().map(Into::into).collect();
    DetailGrid::new(composite_rows).into_any_element()
}

pub fn compare_value_line_elements(value: &str, max_lines: usize) -> Vec<AnyElement> {
    let mut lines = value.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }
    let truncated = lines.len() > max_lines;
    lines
        .into_iter()
        .take(max_lines)
        .enumerate()
        .map(|(index, line)| {
            let line = if truncated && index + 1 == max_lines {
                "..."
            } else if line.is_empty() {
                " "
            } else {
                line
            };
            div()
                .truncate()
                .child(SharedString::from(line.to_string()))
                .into_any_element()
        })
        .collect::<Vec<_>>()
}

pub fn metadata_action_button(label: &str) -> Button {
    Button::new(SharedString::from(format!("metadata-action:{label}")))
        .label(SharedString::from(label.to_string()))
        .with_size(Size::XSmall)
        .compact()
        .ghost()
        .text_color(color::text_on_accent())
        .text_size(typography::SIZE_MICRO)
        .rounded(radius::SM)
        .border_1()
        .border_color(color::accent())
}

pub fn section_heading(label: &str) -> AnyElement {
    div()
        .text_size(typography::SIZE_MICRO)
        .font_weight(FontWeight::BOLD)
        .text_color(color::text_muted())
        .child(SharedString::from(label.to_string()))
        .into_any_element()
}

pub fn truncated(text: String) -> Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(typography::SIZE_MICRO)
        .child(SharedString::from(text))
}

pub fn truncated_muted(text: String) -> Div {
    truncated(text).text_color(color::text_muted())
}

pub fn optional_row(rows: &mut Vec<(String, String)>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        if !value.is_empty() {
            rows.push((key.into(), value));
        }
    }
}

pub fn type_color(entity_type: &str) -> gpui::Rgba {
    badges::type_color(entity_type)
}

pub fn badge_text(entity_type: &str) -> gpui::Rgba {
    match entity_type {
        // Dark text on bright artist badges for WCAG AA contrast — same
        // dark-on-light pairing already used for feed/track badges.
        "artist" => badges::text_color("track"),
        _ => badges::text_color(entity_type),
    }
}

pub fn type_emoji(entity_type: &str) -> &'static str {
    match entity_type {
        "artist" => "🎤",
        "release" => "💿",
        _ => badges::emoji(entity_type),
    }
}

pub fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

// keep `spacing` re-export indirectly available via the existing
// `crate::ui::theme::spacing` callers — no helper needed.
#[allow(dead_code)]
fn _spacing_kept_in_scope() {
    let _ = spacing::LG;
}
