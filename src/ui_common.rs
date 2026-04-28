use crate::ui::theme::{badges, color, radius, spacing, typography};
use gpui::{
    div, img, prelude::*, px, rgb, AnyElement, Div, FontWeight, Image, ImageFormat, IntoElement,
    ObjectFit, ParentElement, SharedString, Styled,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{Sizable, Size};
use std::sync::Arc;

pub struct DetailRow {
    pub key: String,
    pub value: AnyElement,
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

pub fn render_thumb(
    image_data: Option<Arc<Image>>,
    entity_type: &str,
    size: f32,
    large: bool,
) -> AnyElement {
    let radius = if large { 6.0 } else { 4.0 };
    if let Some(image) = image_data {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(px(radius))
            .overflow_hidden()
            .flex_shrink_0()
            .child(artwork_img(image, size))
            .into_any_element()
    } else {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(px(radius))
            .bg(color::border_subtle())
            .flex()
            .items_center()
            .justify_center()
            .text_size(if large {
                px(28.0)
            } else {
                typography::SIZE_BODY
            })
            .flex_shrink_0()
            .child(type_emoji(entity_type))
            .into_any_element()
    }
}

pub fn render_detail_header(
    entity_type: &str,
    title: &str,
    subtitle: Option<&str>,
    image: Option<Arc<Image>>,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::LG)
        .child(render_thumb(image, entity_type, 80.0, true))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(typography::SIZE_MICRO)
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_text(entity_type))
                        .bg(type_color(entity_type))
                        .px(spacing::SM)
                        .py(spacing::XXS)
                        .rounded(radius::SM)
                        .mb(spacing::SM)
                        .child(SharedString::from(entity_type.to_string())),
                )
                .child(typography::type_title(div()).child(SharedString::from(title.to_string())))
                .when_some(subtitle.map(str::to_owned), |el, sub| {
                    el.child(
                        typography::type_body(div())
                            .mt(spacing::XS)
                            .text_color(color::text_muted())
                            .child(SharedString::from(sub)),
                    )
                }),
        )
        .into_any_element()
}

pub fn render_detail_grid(rows: Vec<(String, String)>) -> AnyElement {
    render_detail_grid_elements(
        rows.into_iter()
            .map(|(key, value)| DetailRow {
                key,
                value: div()
                    .text_size(typography::SIZE_MICRO)
                    .line_height(px(17.0))
                    .flex()
                    .flex_col()
                    .children(compare_value_line_elements(&value, 6))
                    .into_any_element(),
            })
            .collect(),
    )
}

pub fn render_detail_grid_elements(rows: Vec<DetailRow>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(spacing::XS)
        .children(rows.into_iter().map(|row| {
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(spacing::MD)
                .child(
                    div()
                        .w(px(124.0))
                        .flex_shrink_0()
                        .text_color(color::text_muted())
                        .whitespace_nowrap()
                        .text_size(typography::SIZE_MICRO)
                        .child(SharedString::from(row.key)),
                )
                .child(div().flex_1().min_w_0().child(row.value))
                .into_any_element()
        }))
        .into_any_element()
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
        .text_color(rgb(0xffffff))
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
        // Dark text on bright badges for WCAG AA contrast
        "artist" => rgb(0x111318),
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
