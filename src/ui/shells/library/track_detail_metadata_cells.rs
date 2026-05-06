//! Library track-detail metadata row-cell renderers.
//!
//! Keeps source, RSS, ID3, and `MusicBrainz` cell composition out of the grid
//! assembly module while preserving the same view-model contracts.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;
use std::sync::Arc;

use gpui::{Context, Image, IntoElement, SharedString};

use super::track_detail_metadata_values::{
    compare_tag_cell, metadata_tag_cell, metadata_value_cell,
};
use crate::library::LibraryApp;
use crate::metadata::{display_metadata_value, AlignedCompareRow, MetadataColumn, PendingId3Edit};
use crate::ui::composites::{
    ProvenanceRole, TrackMetadataFieldCell, TrackMetadataFieldDisplay, TrackMetadataGroupCell,
    TrackMetadataGroupDisplay, TrackMetadataSourceCell,
};
use crate::ui::style::color;
use crate::view_models::track_metadata_grid::{TrackMetadataComparisonRole, TrackMetadataGridVm};

pub(super) fn metadata_group_cell(
    group: crate::metadata::MetadataGroupRow,
    columns: u16,
    cx: &mut Context<LibraryApp>,
) -> gpui::AnyElement {
    let group_key = group.key;
    let display = TrackMetadataGridVm::group_heading_display(
        &group.label,
        group.unused_count,
        group_key.as_deref(),
    );
    let expanded = group.expanded;
    if let (Some(group_key), Some(disclosure_id)) = (group_key, display.disclosure_id) {
        return TrackMetadataGroupCell::new(TrackMetadataGroupDisplay {
            label: SharedString::from(display.label),
            columns,
        })
        .disclosure_group(
            SharedString::from(disclosure_id),
            !expanded,
            cx.listener(move |this, _, _, cx| {
                this.toggle_id3_frame_group(group_key.clone(), cx);
            }),
        )
        .into_any_element();
    }
    TrackMetadataGroupCell::new(TrackMetadataGroupDisplay {
        label: SharedString::from(display.label),
        columns,
    })
    .into_any_element()
}

pub(super) fn metadata_rss_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<LibraryApp>,
) -> gpui::AnyElement {
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
    let glyph = source_role.map(ProvenanceRole::glyph);
    let display_value = TrackMetadataGridVm::display_with_glyph(glyph, &base_display);
    let value_color = source_role.map_or_else(color::text_primary, |role| role.color(cx));
    let value_element = metadata_value_cell(
        &row.field,
        &row.row_id,
        value,
        &display_value,
        expanded,
        value_color,
        "rss",
        expanded_cells,
        None,
        cx,
    );
    TrackMetadataFieldCell::new(TrackMetadataFieldDisplay {
        label: SharedString::from(TrackMetadataGridVm::field_label(&row.field)),
        value: value_element,
    })
    .into_any_element()
}

pub(super) fn metadata_id3_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    file_image: Option<&Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> gpui::AnyElement {
    let frame = TrackMetadataGridVm::id3_cell_frame(
        pending.map(|edit| edit.frame.as_str()),
        row.id3_frame.as_deref(),
    );
    let value = TrackMetadataGridVm::id3_cell_value(
        pending.map(|edit| edit.value.as_str()),
        row.id3_value.as_deref(),
    );
    let base_display = display_metadata_value(&row.field, value);
    let glyph = if pending.is_some() {
        Some(TrackMetadataComparisonRole::Match.glyph())
    } else {
        TrackMetadataGridVm::comparison_glyph(&row.id3_status)
    };
    let display_value = TrackMetadataGridVm::display_with_glyph(glyph, &base_display);
    let color = match pending {
        Some(edit) => pending_source_color(edit.source, cx),
        None => id3_cell_status_color(row, cx),
    };
    let value_element = metadata_tag_cell(
        &row.field,
        &row.row_id,
        value,
        &display_value,
        expanded,
        color,
        frame,
        expanded_cells,
        file_image,
        cx,
    );
    let mut cell = TrackMetadataSourceCell::new(value_element);
    if let Some(edit) = pending {
        cell = cell.border_color(pending_source_color(edit.source, cx));
    }
    cell.into_any_element()
}

pub(super) fn metadata_musicbrainz_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    cx: &mut Context<LibraryApp>,
) -> gpui::AnyElement {
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
    let value = TrackMetadataGridVm::musicbrainz_cell_value(row.musicbrainz_value.as_deref());
    let base_display = display_metadata_value(&row.field, value);
    let glyph = source_role.map_or_else(
        || TrackMetadataGridVm::comparison_glyph(&row.musicbrainz_status),
        |role| Some(role.glyph()),
    );
    let display_value = TrackMetadataGridVm::display_with_glyph(glyph, &base_display);
    TrackMetadataSourceCell::new(compare_tag_cell(
        &display_value,
        Some(musicbrainz_color),
        row.musicbrainz_key.as_deref(),
        None,
    ))
    .into_any_element()
}

fn comparison_status_color(
    status: &crate::track_compare::ComparisonStatus,
    cx: &mut Context<LibraryApp>,
) -> gpui::Rgba {
    TrackMetadataGridVm::comparison_role(status)
        .map(ProvenanceRole::from)
        .map_or_else(color::text_muted, |role| role.color(cx))
}

fn id3_cell_status_color(row: &AlignedCompareRow, cx: &mut Context<LibraryApp>) -> gpui::Rgba {
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

fn pending_source_color(source: MetadataColumn, cx: &mut Context<LibraryApp>) -> gpui::Rgba {
    match source {
        MetadataColumn::Rss | MetadataColumn::MusicBrainz => ProvenanceRole::Match.color(cx),
    }
}
