//! Discover metadata expandable cell rendering.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, Image, MouseButton, MouseDownEvent, Rgba,
    SharedString, Styled,
};

use crate::discover::SearchApp;
use crate::ui::layouts as layout;
use crate::ui::primitives::{Image as ImagePrimitive, ImageSize};
use crate::ui::style::{color, spacing, typography};
use crate::ui::tokens::Radius;
use crate::view_models::track_metadata_grid::{
    TrackMetadataExpandableCellDisplay, TrackMetadataExpandedFieldKind, TrackMetadataGridVm,
    ValueRoutesSummaryFallback,
};

use super::tree::{json_tree_elements, transcript_text_elements, value_routes_tree_elements};

#[derive(Clone, Copy)]
pub(super) struct ExpandableCellParams<'a> {
    pub(super) field: &'a str,
    pub(super) row_id: &'a str,
    pub(super) raw_value: &'a str,
    pub(super) display_value: &'a str,
    pub(super) expanded: bool,
    pub(super) color: gpui::Rgba,
}

pub(super) struct ExpandableTagCellParams<'a> {
    pub(super) base: ExpandableCellParams<'a>,
    pub(super) frame_id: Option<&'a str>,
    pub(super) frame_color: Option<Rgba>,
    pub(super) file_image: Option<Arc<Image>>,
}

#[expect(
    clippy::too_many_lines,
    reason = "expanded Discover metadata cells preserve the existing JSON, artwork, and transcript layouts"
)]
pub(super) fn expandable_cell(
    params: ExpandableCellParams<'_>,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let ExpandableCellParams {
        field,
        row_id,
        raw_value,
        display_value,
        expanded,
        color,
    } = params;
    let display =
        TrackMetadataGridVm::discover_expandable_cell_display("rss", field, row_id, expanded);
    let field_kind = TrackMetadataGridVm::expanded_field_kind(field);

    if expanded && field_kind == TrackMetadataExpandedFieldKind::ValueRoutes {
        let TrackMetadataExpandableCellDisplay {
            cell_key,
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
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.toggle_metadata_cell(cell_key.clone(), cx);
                    }))
                    .flex()
                    .flex_row()
                    .gap(spacing::XS)
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .text_color(color::text_muted())
                            .child(disclosure_glyph),
                    ),
            )
            .children(value_routes_tree_elements(
                raw_value,
                "rss",
                row_id,
                color,
                expanded_cells,
                cx,
            ))
            .into_any_element();
    }

    let TrackMetadataExpandableCellDisplay {
        cell_key,
        cell_id,
        disclosure_glyph,
        ..
    } = display;
    let mut container = div()
        .id(SharedString::from(cell_id))
        .cursor_pointer()
        .text_size(typography::SIZE_MICRO)
        .line_height(typography::LINE_BODY)
        .text_color(color)
        .flex()
        .flex_col()
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }));

    if expanded {
        match TrackMetadataGridVm::expanded_field_kind(field) {
            TrackMetadataExpandedFieldKind::Artwork
                if TrackMetadataGridVm::artwork_url(raw_value).is_some() =>
            {
                let url = raw_value.to_string();
                container = container
                    .child(
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
                            .child(div().text_color(color::accent()).truncate().child(
                                SharedString::from(TrackMetadataGridVm::artwork_url_display(
                                    raw_value,
                                )),
                            )),
                    )
                    .on_mouse_down(
                        MouseButton::Middle,
                        move |_: &MouseDownEvent, _window, _cx| {
                            let _ = open::that(&url);
                        },
                    );
            }
            TrackMetadataExpandedFieldKind::Transcript => {
                container = container.child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(spacing::XS)
                        .items_start()
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .text_color(color::text_muted())
                                .child(disclosure_glyph),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .children(transcript_text_elements(raw_value, color)),
                        ),
                );
            }
            TrackMetadataExpandedFieldKind::Artwork
            | TrackMetadataExpandedFieldKind::Text
            | TrackMetadataExpandedFieldKind::ValueRoutes => {
                let expanded_display =
                    TrackMetadataGridVm::expanded_display_value(field, raw_value, display_value);
                container =
                    container.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(spacing::XS)
                            .items_start()
                            .child(
                                div()
                                    .text_size(typography::SIZE_MICRO)
                                    .text_color(color::text_muted())
                                    .child(disclosure_glyph),
                            )
                            .child(
                                div().flex_1().min_w_0().flex().flex_col().children(
                                    json_tree_elements(raw_value, &expanded_display, color),
                                ),
                            ),
                    );
            }
        }
    } else {
        let summary = TrackMetadataGridVm::expandable_cell_summary(
            field,
            raw_value,
            display_value,
            ValueRoutesSummaryFallback::MultilineCount,
        );
        container = container.child(
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
                        .text_color(color::accent())
                        .truncate()
                        .child(SharedString::from(summary)),
                ),
        );
    }
    container.into_any_element()
}

#[expect(
    clippy::too_many_lines,
    reason = "expanded Discover tag cells preserve the existing drop target and tree layouts"
)]
pub(super) fn expandable_tag_cell(
    params: ExpandableTagCellParams<'_>,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let ExpandableTagCellParams {
        base:
            ExpandableCellParams {
                field,
                row_id,
                raw_value,
                display_value,
                expanded,
                color,
            },
        frame_id,
        frame_color,
        file_image,
    } = params;
    let display =
        TrackMetadataGridVm::discover_expandable_cell_display("id3", field, row_id, expanded);
    let display_disclosure_glyph = display.disclosure_glyph;
    let frame_color = frame_color.unwrap_or_else(color::text_muted);
    let frame_label = TrackMetadataGridVm::id3_frame_display_label(frame_id);
    let field_kind = TrackMetadataGridVm::expanded_field_kind(field);

    let value_el = if expanded {
        match field_kind {
            TrackMetadataExpandedFieldKind::Artwork => {
                if let Some(image) = file_image {
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(spacing::XS)
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .line_height(typography::LINE_BODY)
                                .text_color(color)
                                .child(SharedString::from(
                                    TrackMetadataGridVm::text_value_display(display_value),
                                )),
                        )
                        .child(
                            ImagePrimitive::new(image.clone())
                                .size(ImageSize::XXl)
                                .radius(Radius::MD),
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(typography::SIZE_MICRO)
                        .line_height(typography::LINE_BODY)
                        .text_color(color)
                        .child(SharedString::from(TrackMetadataGridVm::text_value_display(
                            display_value,
                        )))
                        .into_any_element()
                }
            }
            TrackMetadataExpandedFieldKind::Transcript => div()
                .flex_1()
                .min_w_0()
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
                .text_color(color)
                .flex()
                .flex_col()
                .children(transcript_text_elements(raw_value, color))
                .into_any_element(),
            TrackMetadataExpandedFieldKind::Text | TrackMetadataExpandedFieldKind::ValueRoutes => {
                let expanded_display =
                    TrackMetadataGridVm::expanded_display_value(field, raw_value, display_value);
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(typography::SIZE_MICRO)
                    .line_height(typography::LINE_BODY)
                    .text_color(color)
                    .flex()
                    .flex_col()
                    .children(json_tree_elements(raw_value, &expanded_display, color))
                    .into_any_element()
            }
        }
    } else {
        let summary = TrackMetadataGridVm::expandable_cell_summary(
            field,
            raw_value,
            display_value,
            ValueRoutesSummaryFallback::MultilineCount,
        );
        div()
            .flex_1()
            .min_w_0()
            .text_size(typography::SIZE_MICRO)
            .line_height(typography::LINE_BODY)
            .flex()
            .flex_row()
            .gap(spacing::XS)
            .child(
                div()
                    .text_size(typography::SIZE_MICRO)
                    .text_color(color::text_muted())
                    .child(display_disclosure_glyph),
            )
            .child(
                div()
                    .text_color(color::accent())
                    .truncate()
                    .child(SharedString::from(summary)),
            )
            .into_any_element()
    };

    if expanded && field_kind == TrackMetadataExpandedFieldKind::ValueRoutes {
        let TrackMetadataExpandableCellDisplay {
            cell_key,
            header_id,
            disclosure_glyph,
            ..
        } = display;
        return div()
            .flex()
            .flex_col()
            .text_size(typography::SIZE_MICRO)
            .line_height(typography::LINE_BODY)
            .text_color(color)
            .child(
                div()
                    .id(SharedString::from(header_id))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.toggle_metadata_cell(cell_key.clone(), cx);
                    }))
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(spacing::SM)
                    .child(
                        div()
                            .w(layout::METADATA_LABEL_WIDTH)
                            .flex_shrink_0()
                            .text_color(frame_color)
                            .text_size(typography::SIZE_MICRO)
                            .line_height(typography::LINE_BODY)
                            .child(SharedString::from(frame_label)),
                    )
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .text_color(color::text_muted())
                            .child(disclosure_glyph),
                    ),
            )
            .child(
                div()
                    .pl(layout::METADATA_VALUE_INDENT)
                    .flex()
                    .flex_col()
                    .children(value_routes_tree_elements(
                        raw_value,
                        "id3",
                        row_id,
                        color,
                        expanded_cells,
                        cx,
                    )),
            )
            .into_any_element();
    }

    let TrackMetadataExpandableCellDisplay {
        cell_key, cell_id, ..
    } = display;
    div()
        .id(SharedString::from(cell_id))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }))
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::SM)
        .child(
            div()
                .w(layout::METADATA_LABEL_WIDTH)
                .flex_shrink_0()
                .text_color(frame_color)
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
                .child(SharedString::from(frame_label)),
        )
        .child(value_el)
        .into_any_element()
}
