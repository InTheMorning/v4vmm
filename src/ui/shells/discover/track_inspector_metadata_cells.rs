//! Discover metadata grid cell rendering.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, Context, FontWeight, Image, MouseButton, MouseDownEvent,
    Pixels, Point, SharedString, Styled,
};

use crate::discover::SearchApp;
use crate::metadata::{
    display_metadata_value, id3v24_drag_copy_frame_is_writable, normalized_compare_value,
    AlignedCompareRow, MetadataColumn, MetadataDragValue, MetadataGroupRow, PendingId3Edit,
};
use crate::track_compare::ComparisonStatus;
use crate::ui::composites::{DisclosureGroup, DisclosureGroupDisplay, ProvenanceRole};
use crate::ui::layouts as layout;
use crate::ui::primitives::MultilineText;
use crate::ui::style::{color, radius, spacing, typography};
use crate::ui::tokens::FontSize;
use crate::view_models::track_metadata_grid::{
    TrackMetadataGridVm, TrackMetadataId3FrameColorContext, TrackMetadataId3FrameColorRole,
};

use super::expandable::{
    expandable_cell, expandable_tag_cell, ExpandableCellParams, ExpandableTagCellParams,
};
use super::MetadataDragPreview;

pub(super) fn metadata_rss_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let value = TrackMetadataGridVm::rss_cell_value(row.rss_value.as_deref());
    let base_display = display_metadata_value(&row.field, value);
    let source_role = pending.and_then(|edit| {
        TrackMetadataGridVm::pending_source_role(
            edit.source,
            &edit.value,
            MetadataColumn::Rss,
            row.rss_value.as_deref(),
        )
        .map(ProvenanceRole::from)
    });
    let value_color = source_role.map_or_else(color::text_primary, |role| role.color(cx));
    let glyph = source_role.map(ProvenanceRole::glyph);
    let display_value = TrackMetadataGridVm::display_with_glyph(glyph, &base_display);
    let expandable = TrackMetadataGridVm::field_is_expandable(&row.field, value);
    let value_element = if expandable {
        expandable_cell(
            ExpandableCellParams {
                field: &row.field,
                row_id: &row.row_id,
                raw_value: value,
                display_value: &display_value,
                expanded,
                color: value_color,
            },
            expanded_cells,
            cx,
        )
    } else {
        compare_cell(&display_value, Some(value_color))
    };
    let cell = div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::SM)
        .child(
            div()
                .w(layout::COMPACT_COLUMN_WIDTH)
                .flex_shrink_0()
                .text_color(color::text_primary())
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
                .child(SharedString::from(TrackMetadataGridVm::field_label(
                    &row.field,
                ))),
        )
        .child(div().flex_1().min_w_0().child(value_element));
    if !expandable {
        if let Some(drag) = metadata_drag_value(row, MetadataColumn::Rss) {
            let display =
                TrackMetadataGridVm::source_drag_display(MetadataColumn::Rss, &row.row_id);
            return cell
                .id(SharedString::from(display.cell_id))
                .cursor_move()
                .hover(|style| style.bg(color::bg_surface()))
                .on_drag(
                    drag,
                    |drag: &MetadataDragValue, _position: Point<Pixels>, _window, cx: &mut App| {
                        let display =
                            TrackMetadataGridVm::drag_preview_display(&drag.field, &drag.value);
                        cx.new(|_| MetadataDragPreview { display })
                    },
                )
                .into_any_element();
        }
    }
    cell.into_any_element()
}

pub(super) fn metadata_id3_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    file_image: Option<Arc<Image>>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let frame = TrackMetadataGridVm::id3_cell_frame(
        pending.map(|edit| edit.frame.as_str()),
        row.id3_frame.as_deref(),
    );
    let value = TrackMetadataGridVm::id3_cell_value(
        pending.map(|edit| edit.value.as_str()),
        row.id3_value.as_deref(),
    );
    let display_value = display_metadata_value(&row.field, value);
    let color = match pending {
        Some(edit) => pending_source_color(edit.source, cx),
        None => id3_cell_status_color(row, cx),
    };
    let frame_color = frame.map(|frame| {
        id3_frame_color(TrackMetadataGridVm::id3_frame_color_role(
            Some(frame),
            TrackMetadataId3FrameColorContext::Discover,
        ))
    });
    let expandable = TrackMetadataGridVm::field_is_expandable(&row.field, value);
    let value_element = if expandable {
        expandable_tag_cell(
            ExpandableTagCellParams {
                base: ExpandableCellParams {
                    field: &row.field,
                    row_id: &row.row_id,
                    raw_value: value,
                    display_value: &display_value,
                    expanded,
                    color,
                },
                frame_id: frame,
                frame_color,
                file_image,
            },
            expanded_cells,
            cx,
        )
    } else {
        compare_tag_cell(&display_value, Some(color), frame, frame_color)
    };
    let mut cell = div()
        .pl(spacing::MD)
        .min_w_0()
        .rounded(radius::SM)
        .child(value_element)
        .when_some(pending, |el, edit| {
            el.border_1()
                .border_color(pending_source_color(edit.source, cx))
        });
    if pending.is_some() {
        let row_id = row.row_id.clone();
        cell = cell.on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.revert_pending_id3_edit(row_id.clone(), cx);
            }),
        );
    }

    if let Some(frame) = frame.filter(|frame| id3v24_drag_copy_frame_is_writable(frame)) {
        let row_id = row.row_id.clone();
        let target_field = row.field.clone();
        let target_frame = frame.to_string();
        let target_existing_value = (!value.is_empty()).then(|| value.to_string());
        cell = cell
            .can_drop(|drag, _window, _cx| drag.downcast_ref::<MetadataDragValue>().is_some())
            .hover(|style| style.bg(color::bg_surface()))
            .on_drop(
                cx.listener(move |this, drag: &MetadataDragValue, _window, cx| {
                    let mut drag = drag.clone();
                    drag.row_id.clone_from(&row_id);
                    drag.field.clone_from(&target_field);
                    drag.frame.clone_from(&target_frame);
                    drag.target_existing_value
                        .clone_from(&target_existing_value);
                    this.stage_id3_drag_copy(&drag, cx);
                }),
            );
    }
    cell.into_any_element()
}

pub(super) fn metadata_musicbrainz_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let source_role = pending.and_then(|edit| {
        TrackMetadataGridVm::pending_source_role(
            edit.source,
            &edit.value,
            MetadataColumn::MusicBrainz,
            row.musicbrainz_value.as_deref(),
        )
        .map(ProvenanceRole::from)
    });
    let musicbrainz_color = match source_role {
        Some(role) => role.color(cx),
        None => comparison_status_color(&row.musicbrainz_status, cx),
    };
    let glyph = source_role.map_or_else(
        || TrackMetadataGridVm::comparison_glyph(&row.musicbrainz_status),
        |role| Some(role.glyph()),
    );
    let value = TrackMetadataGridVm::musicbrainz_cell_value(row.musicbrainz_value.as_deref());
    let display_value = display_metadata_value(&row.field, value);
    let display_value = TrackMetadataGridVm::display_with_glyph(glyph, &display_value);
    let cell = div().pl(spacing::MD).min_w_0().child(compare_tag_cell(
        &display_value,
        Some(musicbrainz_color),
        row.musicbrainz_key.as_deref(),
        None,
    ));
    if let Some(drag) = metadata_drag_value(row, MetadataColumn::MusicBrainz) {
        let display =
            TrackMetadataGridVm::source_drag_display(MetadataColumn::MusicBrainz, &row.row_id);
        cell.id(SharedString::from(display.cell_id))
            .cursor_move()
            .hover(|style| style.bg(color::bg_surface()))
            .on_drag(
                drag,
                |drag: &MetadataDragValue, _position: Point<Pixels>, _window, cx: &mut App| {
                    let display =
                        TrackMetadataGridVm::drag_preview_display(&drag.field, &drag.value);
                    cx.new(|_| MetadataDragPreview { display })
                },
            )
            .into_any_element()
    } else {
        cell.into_any_element()
    }
}

pub(crate) fn metadata_drag_value(
    row: &AlignedCompareRow,
    source: MetadataColumn,
) -> Option<MetadataDragValue> {
    let value = match source {
        MetadataColumn::Rss => row.rss_value.as_ref(),
        MetadataColumn::MusicBrainz => row.musicbrainz_value.as_ref(),
    }?;
    let value = normalized_compare_value(Some(value))?;
    Some(MetadataDragValue {
        row_id: row.row_id.clone(),
        field: TrackMetadataGridVm::field_label(&row.field),
        frame: TrackMetadataGridVm::id3_drag_frame(row.id3_frame.as_deref()),
        target_existing_value: None,
        value,
        source,
    })
}

fn pending_source_color(source: MetadataColumn, cx: &mut Context<SearchApp>) -> gpui::Rgba {
    match source {
        MetadataColumn::Rss | MetadataColumn::MusicBrainz => ProvenanceRole::Match.color(cx),
    }
}

pub(super) fn metadata_group_cell(
    group: MetadataGroupRow,
    columns: u16,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let group_key = group.key;
    let display = TrackMetadataGridVm::group_heading_display(
        &group.label,
        group.unused_count,
        group_key.as_deref(),
    );

    let expanded = group.expanded;
    let mut cell = div().col_span(columns).mt(spacing::SM);
    if let (Some(group_key), Some(disclosure_id)) = (group_key, display.disclosure_id) {
        cell = cell.child(
            DisclosureGroup::new(DisclosureGroupDisplay {
                id: SharedString::from(disclosure_id).into(),
                label: SharedString::from(display.label),
                a11y_label: SharedString::from(display.a11y_label),
            })
            .collapsed(!expanded)
            .on_toggle(cx.listener(move |this, _, _, cx| {
                this.toggle_id3_frame_group(group_key.clone(), cx);
            })),
        );
    } else {
        cell = cell.child(
            div()
                .text_size(typography::SIZE_MICRO)
                .font_weight(FontWeight::BOLD)
                .text_color(color::text_muted())
                .child(SharedString::from(display.label)),
        );
    }
    cell.into_any_element()
}
fn compare_cell(value: &str, color: Option<gpui::Rgba>) -> AnyElement {
    let mut cell = MultilineText::new(TrackMetadataGridVm::text_value_display(value))
        .max_lines(4)
        .size(FontSize::Micro)
        .line_height(typography::LINE_BODY);
    if let Some(color) = color {
        cell = cell.color_raw(color);
    }
    cell.into_any_element()
}

fn compare_tag_cell(
    value: &str,
    color: Option<gpui::Rgba>,
    frame_id: Option<&str>,
    frame_color: Option<gpui::Rgba>,
) -> AnyElement {
    let frame_label = TrackMetadataGridVm::id3_frame_display_label(frame_id);
    let frame_color = frame_color.unwrap_or_else(color::text_muted);

    let mut body = MultilineText::new(TrackMetadataGridVm::text_value_display(value))
        .max_lines(4)
        .size(FontSize::Micro)
        .line_height(typography::LINE_BODY);
    if let Some(color) = color {
        body = body.color_raw(color);
    }

    div()
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
        .child(div().flex_1().min_w_0().child(body))
        .into_any_element()
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
fn comparison_status_color(status: &ComparisonStatus, cx: &mut Context<SearchApp>) -> gpui::Rgba {
    TrackMetadataGridVm::comparison_role(status)
        .map(ProvenanceRole::from)
        .map_or_else(color::text_muted, |role| role.color(cx))
}

fn id3_cell_status_color(row: &AlignedCompareRow, cx: &mut Context<SearchApp>) -> gpui::Rgba {
    let fallback_color = || {
        if TrackMetadataGridVm::id3_status_uses_primary_fallback(
            row.id3_value.as_deref(),
            row.rss_value.as_deref(),
            row.musicbrainz_value.as_deref(),
        ) {
            color::text_primary()
        } else {
            color::text_muted()
        }
    };
    TrackMetadataGridVm::id3_status_role(
        row.id3_value.as_deref(),
        row.rss_value.as_deref(),
        row.musicbrainz_value.as_deref(),
        &row.id3_status,
    )
    .map(ProvenanceRole::from)
    .map_or_else(fallback_color, |role| role.color(cx))
}
