//! Discover metadata tree rendering.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context, SharedString, Styled};

use crate::discover::SearchApp;
use crate::ui::style::{color, spacing, typography};
use crate::view_models::metadata::value_route_recipient_label;
use crate::view_models::track_metadata_grid::{
    TrackMetadataGridVm, TrackMetadataValueRouteItemDisplay, ValueRouteFieldContext,
};

pub(super) fn json_tree_elements(
    raw_value: &str,
    display_value: &str,
    color: gpui::Rgba,
) -> Vec<AnyElement> {
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) {
        return arr
            .into_iter()
            .map(|item| json_object_element(&item, color, 0))
            .collect();
    }
    display_value
        .lines()
        .map(|line| {
            let line = TrackMetadataGridVm::transcript_line_display(line);
            div()
                .truncate()
                .child(SharedString::from(TrackMetadataGridVm::text_value_display(
                    line,
                )))
                .into_any_element()
        })
        .collect()
}

fn json_object_element(value: &serde_json::Value, color: gpui::Rgba, depth: usize) -> AnyElement {
    let indent_px = u16::try_from(depth.saturating_mul(12)).unwrap_or(u16::MAX);
    let indent = px(f32::from(indent_px));
    match value {
        serde_json::Value::Object(map) => {
            let mut container = div()
                .flex()
                .flex_col()
                .pl(indent)
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY);
            for (key, val) in map {
                let key_str = TrackMetadataGridVm::value_route_field_key_label(key);
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        container = container
                            .child(
                                div()
                                    .text_color(color::text_muted())
                                    .child(SharedString::from(key_str)),
                            )
                            .child(json_object_element(val, color, depth + 1));
                    }
                    _ => {
                        let val_str = TrackMetadataGridVm::json_tree_scalar_label(val)
                            .expect("object and array values are handled before scalar display");
                        container = container.child(
                            div().flex().flex_row().gap(spacing::XS).child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .child(
                                        div()
                                            .text_color(color::text_muted())
                                            .child(SharedString::from(key_str)),
                                    )
                                    .child(
                                        div()
                                            .text_color(color)
                                            .truncate()
                                            .child(SharedString::from(val_str)),
                                    ),
                            ),
                        );
                    }
                }
            }
            container.into_any_element()
        }
        serde_json::Value::Array(arr) => {
            let mut container = div().flex().flex_col().pl(indent);
            for item in arr {
                container = container.child(json_object_element(item, color, depth));
            }
            container.into_any_element()
        }
        _ => {
            let text = TrackMetadataGridVm::json_tree_scalar_label(value)
                .expect("object and array values are handled before scalar display");
            div()
                .pl(indent)
                .text_color(color)
                .truncate()
                .child(SharedString::from(text))
                .into_any_element()
        }
    }
}

pub(super) fn value_routes_tree_elements(
    raw_value: &str,
    column: &str,
    row_id: &str,
    color: gpui::Rgba,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> Vec<AnyElement> {
    let Ok(routes) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) else {
        return json_tree_elements(raw_value, raw_value, color);
    };
    routes
        .into_iter()
        .enumerate()
        .map(|(i, route)| {
            let name = value_route_recipient_label(&route);
            let label = TrackMetadataGridVm::value_route_item_label(&name, None);
            let item_key = TrackMetadataGridVm::value_route_item_key(column, row_id, i);
            let display = TrackMetadataGridVm::discover_value_route_item_display(
                column,
                row_id,
                i,
                expanded_cells.contains(&item_key),
            );
            let TrackMetadataValueRouteItemDisplay {
                item_key,
                item_id,
                disclosure_glyph,
                ..
            } = display;
            let sub_expanded = expanded_cells.contains(&item_key);

            let mut item = div()
                .id(SharedString::from(item_id))
                .cursor_pointer()
                .flex()
                .flex_col();

            item = item.on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.toggle_metadata_cell(item_key.clone(), cx);
            }));

            item = item.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(spacing::XS)
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .text_color(color::text_muted())
                            .child(disclosure_glyph),
                    )
                    .child(
                        div()
                            .text_color(if sub_expanded { color } else { color::accent() })
                            .child(SharedString::from(label)),
                    ),
            );

            if sub_expanded {
                if let serde_json::Value::Object(map) = &route {
                    for (key, val) in map {
                        if !TrackMetadataGridVm::value_route_child_field_is_visible(
                            key,
                            ValueRouteFieldContext::Discover,
                        ) {
                            continue;
                        }
                        let Some(value) = TrackMetadataGridVm::value_route_field_value_label(val)
                        else {
                            continue;
                        };
                        let key_label = TrackMetadataGridVm::value_route_field_key_label(key);
                        item = item.child(
                            div()
                                .pl(spacing::LG)
                                .flex()
                                .flex_row()
                                .gap(spacing::XS)
                                .child(
                                    div()
                                        .text_color(color::text_muted())
                                        .child(SharedString::from(key_label)),
                                )
                                .child(
                                    div()
                                        .text_color(color)
                                        .truncate()
                                        .child(SharedString::from(value)),
                                ),
                        );
                    }
                }
            }

            item.into_any_element()
        })
        .collect()
}

pub(super) fn transcript_text_elements(raw_value: &str, color: gpui::Rgba) -> Vec<AnyElement> {
    raw_value
        .lines()
        .map(|line| {
            let line = TrackMetadataGridVm::transcript_line_display(line);
            div()
                .text_color(color)
                .child(SharedString::from(TrackMetadataGridVm::text_value_display(
                    line,
                )))
                .into_any_element()
        })
        .collect()
}
