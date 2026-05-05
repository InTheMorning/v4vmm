//! Library track-detail metadata value renderers.
//!
//! Owns expandable values, value-route trees, and compact text/tag cells used
//! by the Library metadata grid cells.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, Context, Image, SharedString, Styled};

use crate::library::LibraryApp;
use crate::ui::composites::{
    EntityKind, Thumbnail, ThumbnailSize, TrackMetadataFrameDisplay, TrackMetadataTagCell,
    TrackMetadataTagDisplay, TrackMetadataTextDisplay, TrackMetadataTextValue,
};
use crate::ui::primitives::MultilineText;
use crate::ui::style::{color, spacing, typography};
use crate::view_models::metadata::value_route_recipient_label;
use crate::view_models::track_metadata_grid::{
    TrackMetadataExpandableCellDisplay, TrackMetadataExpandedFieldKind, TrackMetadataGridVm,
    TrackMetadataId3FrameColorContext, TrackMetadataId3FrameColorRole,
    TrackMetadataValueRouteItemDisplay, ValueRouteFieldContext, ValueRoutesSummaryFallback,
};

#[expect(
    clippy::too_many_arguments,
    reason = "UI cell renderer keeps field context explicit"
)]
pub(super) fn metadata_value_cell(
    field: &str,
    row_id: &str,
    raw_value: &str,
    display_value: &str,
    expanded: bool,
    color: gpui::Rgba,
    column: &str,
    expanded_cells: &BTreeSet<String>,
    file_image: Option<&Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let logical_field = TrackMetadataGridVm::logical_field(field);
    let field_kind = TrackMetadataGridVm::expanded_field_kind(logical_field);
    let expandable = TrackMetadataGridVm::field_is_expandable(logical_field, raw_value);
    if !expandable {
        return compare_cell(display_value, Some(color));
    }
    let display = TrackMetadataGridVm::library_expandable_cell_display(column, row_id, expanded);
    let summary = TrackMetadataGridVm::expandable_cell_summary(
        logical_field,
        raw_value,
        display_value,
        ValueRoutesSummaryFallback::DisplayValue,
    );
    if expanded && field_kind == TrackMetadataExpandedFieldKind::ValueRoutes {
        let TrackMetadataExpandableCellDisplay {
            cell_key: header_key,
            header_id,
            disclosure_glyph,
            ..
        } = display;
        return div()
            .text_size(typography::SIZE_MICRO)
            .line_height(typography::LINE_BODY)
            .text_color(color)
            .flex()
            .flex_col()
            .child(
                div()
                    .id(SharedString::from(header_id))
                    .cursor_pointer()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(spacing::XS)
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
                        this.toggle_metadata_cell(header_key.clone(), cx);
                    }))
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .text_color(color::text_muted())
                            .child(disclosure_glyph),
                    ),
            )
            .child(div().flex().flex_col().children(value_routes_tree_elements(
                raw_value,
                column,
                row_id,
                color,
                expanded_cells,
                cx,
            )))
            .into_any_element();
    }
    let content = if expanded {
        expanded_metadata_value(
            field_kind,
            logical_field,
            raw_value,
            display_value,
            color,
            file_image,
        )
    } else {
        div()
            .text_color(color::accent())
            .truncate()
            .child(SharedString::from(summary))
            .into_any_element()
    };
    let TrackMetadataExpandableCellDisplay {
        cell_key,
        cell_id,
        disclosure_glyph,
        ..
    } = display;
    div()
        .id(SharedString::from(cell_id))
        .cursor_pointer()
        .text_size(typography::SIZE_MICRO)
        .line_height(typography::LINE_BODY)
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::XS)
        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }))
        .child(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(color::text_muted())
                .child(disclosure_glyph),
        )
        .child(div().flex_1().min_w_0().child(content))
        .into_any_element()
}

#[expect(
    clippy::too_many_arguments,
    reason = "UI cell renderer keeps field context explicit"
)]
pub(super) fn metadata_tag_cell(
    field: &str,
    row_id: &str,
    raw_value: &str,
    display_value: &str,
    expanded: bool,
    color: gpui::Rgba,
    frame_id: Option<&str>,
    expanded_cells: &BTreeSet<String>,
    file_image: Option<&Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let frame_color = id3_frame_color(TrackMetadataGridVm::id3_frame_color_role(
        frame_id,
        TrackMetadataId3FrameColorContext::Library,
    ));
    let value = metadata_value_cell(
        field,
        row_id,
        raw_value,
        display_value,
        expanded,
        color,
        "id3",
        expanded_cells,
        file_image,
        cx,
    );
    TrackMetadataTagCell::new(TrackMetadataTagDisplay {
        value,
        frame: frame_id.map(|frame_id| TrackMetadataFrameDisplay {
            label: SharedString::from(TrackMetadataGridVm::id3_frame_display_label(Some(frame_id))),
            color: Some(frame_color),
        }),
    })
    .frame_color(frame_color)
    .into_any_element()
}

pub(super) fn compare_tag_cell(
    value: &str,
    color: Option<gpui::Rgba>,
    frame_id: Option<&str>,
    frame_color: Option<gpui::Rgba>,
) -> AnyElement {
    let mut body = TrackMetadataTextValue::new(TrackMetadataTextDisplay {
        value: SharedString::from(TrackMetadataGridVm::text_value_display(value)),
    });
    if let Some(color) = color {
        body = body.color_raw(color);
    }
    let mut cell = TrackMetadataTagCell::new(TrackMetadataTagDisplay {
        value: body.into_any_element(),
        frame: frame_id.map(|frame_id| TrackMetadataFrameDisplay {
            label: SharedString::from(TrackMetadataGridVm::id3_frame_display_label(Some(frame_id))),
            color: frame_color,
        }),
    });
    if let Some(frame_color) = frame_color {
        cell = cell.frame_color(frame_color);
    }
    cell.into_any_element()
}

fn expanded_metadata_value(
    field_kind: TrackMetadataExpandedFieldKind,
    field: &str,
    raw_value: &str,
    display_value: &str,
    color: gpui::Rgba,
    file_image: Option<&Arc<Image>>,
) -> AnyElement {
    if field_kind == TrackMetadataExpandedFieldKind::Artwork {
        if let Some(image) = file_image {
            return div()
                .flex()
                .flex_col()
                .gap(spacing::XS)
                .child(SharedString::from(TrackMetadataGridVm::text_value_display(
                    display_value,
                )))
                .child(
                    Thumbnail::new(EntityKind::Track, ThumbnailSize::Lg).image(Some(image.clone())),
                )
                .into_any_element();
        }
    }
    let value = TrackMetadataGridVm::expanded_display_value(field, raw_value, display_value);
    MultilineText::new(value)
        .max_lines(20)
        .color_raw(color)
        .into_any_element()
}

fn value_routes_tree_elements(
    raw_value: &str,
    column: &str,
    row_id: &str,
    color: gpui::Rgba,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<LibraryApp>,
) -> Vec<AnyElement> {
    let Ok(routes) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) else {
        return vec![
            MultilineText::new(TrackMetadataGridVm::text_value_display(raw_value))
                .max_lines(20)
                .color_raw(color)
                .into_any_element(),
        ];
    };

    routes
        .into_iter()
        .enumerate()
        .map(|(index, route)| {
            value_route_tree_element(route, column, row_id, index, color, expanded_cells, cx)
        })
        .collect()
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "route values are moved from the parsed JSON array into row renderers"
)]
fn value_route_tree_element(
    route: serde_json::Value,
    column: &str,
    row_id: &str,
    index: usize,
    color: gpui::Rgba,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let name = value_route_recipient_label(&route);
    let split = route
        .get("split")
        .and_then(TrackMetadataGridVm::value_route_split_label);
    let label = TrackMetadataGridVm::value_route_item_label(&name, split.as_deref());
    let item_key = TrackMetadataGridVm::value_route_item_key(column, row_id, index);
    let display = TrackMetadataGridVm::library_value_route_item_display(
        column,
        row_id,
        index,
        expanded_cells.contains(&item_key),
    );
    let TrackMetadataValueRouteItemDisplay {
        item_key: header_key,
        item_id,
        header_id,
        disclosure_glyph,
    } = display;
    let sub_expanded = expanded_cells.contains(&header_key);

    let mut item = div()
        .id(SharedString::from(item_id))
        .flex()
        .flex_col()
        .gap(spacing::XXS)
        .child(
            div()
                .id(SharedString::from(
                    header_id.expect("Library value-route rows have header ids"),
                ))
                .cursor_pointer()
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::XS)
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
                    this.toggle_metadata_cell(header_key.clone(), cx);
                }))
                .child(
                    div()
                        .text_size(typography::SIZE_MICRO)
                        .text_color(color::text_muted())
                        .child(disclosure_glyph),
                )
                .child(
                    div()
                        .text_color(if sub_expanded { color } else { color::accent() })
                        .truncate()
                        .child(SharedString::from(label)),
                ),
        );

    if sub_expanded {
        item = item.children(value_route_child_elements(&route, color));
    }

    item.into_any_element()
}

fn value_route_child_elements(route: &serde_json::Value, color: gpui::Rgba) -> Vec<AnyElement> {
    let serde_json::Value::Object(map) = route else {
        return Vec::new();
    };

    map.iter()
        .filter_map(|(key, value)| {
            if !TrackMetadataGridVm::value_route_child_field_is_visible(
                key,
                ValueRouteFieldContext::Library,
            ) {
                return None;
            }
            let value = TrackMetadataGridVm::value_route_field_value_label(value)?;
            let key_label = TrackMetadataGridVm::value_route_field_key_label(key);
            Some(
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
                    )
                    .into_any_element(),
            )
        })
        .collect()
}

fn compare_cell(value: &str, color: Option<gpui::Rgba>) -> AnyElement {
    let mut cell = TrackMetadataTextValue::new(TrackMetadataTextDisplay {
        value: SharedString::from(TrackMetadataGridVm::text_value_display(value)),
    });
    if let Some(color) = color {
        cell = cell.color_raw(color);
    }
    cell.into_any_element()
}

fn id3_frame_color(role: TrackMetadataId3FrameColorRole) -> gpui::Rgba {
    match role {
        TrackMetadataId3FrameColorRole::Muted => color::text_muted(),
        TrackMetadataId3FrameColorRole::Accent => color::accent(),
        TrackMetadataId3FrameColorRole::V22 => color::id3_frame_v22(),
        TrackMetadataId3FrameColorRole::V23Only => color::id3_frame_v23_only(),
        TrackMetadataId3FrameColorRole::V24Only => color::id3_frame_v24_only(),
        TrackMetadataId3FrameColorRole::Unknown => color::id3_frame_unknown(),
    }
}
