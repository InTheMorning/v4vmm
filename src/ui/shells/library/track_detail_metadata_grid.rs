//! Library track-detail metadata grid and cell renderers.
//!
//! This sibling exists because the full metadata surface exceeds the Library
//! shell file budget; panel composition and callbacks live in
//! `track_detail_metadata`, while the dense grid/cell helpers stay here.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use gpui::{AnyElement, Context, Image, IntoElement};

use super::track_detail_metadata_cells::{
    metadata_group_cell, metadata_id3_cell, metadata_musicbrainz_cell, metadata_rss_cell,
};
use crate::library::{InspectorFrame, LazyPanel, LibraryApp};
use crate::metadata::{
    aligned_compare_rows, expand_woar_metadata_rows, track_metadata_rows, MetadataGridRow,
    MusicBrainzLookupResult, PendingId3Edit, TagCompareResult, TrackContext,
};
use crate::musicbrainz::MusicBrainzCandidate;
use crate::ui::composites::TrackMetadataGrid;
use crate::view_models::track_metadata_grid::TrackMetadataGridVm;

pub(crate) fn track_metadata_rows_for_frame(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
) -> Vec<MetadataGridRow> {
    let selected_musicbrainz = match &frame.musicbrainz_lookup {
        LazyPanel::Loaded(lookup) => selected_musicbrainz_candidate(frame, lookup),
        LazyPanel::Hidden | LazyPanel::Loading | LazyPanel::Empty(_) => None,
    };
    let show_musicbrainz = !matches!(frame.musicbrainz_lookup, LazyPanel::Hidden);
    let rows = result.map_or_else(
        || track_metadata_rows(track_context, selected_musicbrainz, show_musicbrainz),
        |result| {
            aligned_compare_rows(
                result,
                track_context,
                selected_musicbrainz,
                show_musicbrainz,
                &frame.expanded_id3_frame_groups,
            )
        },
    );
    expand_woar_metadata_rows(rows)
}

#[expect(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "metadata grid needs explicit column state and edit state inputs"
)]
pub(crate) fn library_track_metadata_grid(
    rows: Vec<MetadataGridRow>,
    show_id3: bool,
    show_musicbrainz: bool,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    expanded_metadata_cells: &BTreeSet<String>,
    file_image: Option<Arc<Image>>,
    tag_column_label: &str,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let vm = TrackMetadataGridVm::new(show_id3, show_musicbrainz, tag_column_label);
    let mut cells: Vec<AnyElement> = Vec::new();

    for row in rows {
        match row {
            MetadataGridRow::Group(group) => {
                cells.push(metadata_group_cell(group, vm.columns(), cx));
            }
            MetadataGridRow::Data(row) => {
                let pending = pending_id3_edits.get(&row.row_id);
                let expansion = vm.expansion_for(&row.row_id, expanded_metadata_cells);
                cells.push(metadata_rss_cell(
                    &row,
                    pending,
                    expansion.rss_expanded,
                    expanded_metadata_cells,
                    cx,
                ));
                if show_id3 {
                    cells.push(metadata_id3_cell(
                        &row,
                        pending,
                        expansion.id3_expanded,
                        expanded_metadata_cells,
                        file_image.as_ref(),
                        cx,
                    ));
                }
                if show_musicbrainz {
                    cells.push(metadata_musicbrainz_cell(&row, pending, cx));
                }
            }
        }
    }

    TrackMetadataGrid::new(vm).cells(cells).into_any_element()
}

fn selected_musicbrainz_candidate<'a>(
    frame: &InspectorFrame,
    result: &'a MusicBrainzLookupResult,
) -> Option<&'a MusicBrainzCandidate> {
    result
        .lookup
        .candidates
        .get(frame.musicbrainz_selected)
        .or_else(|| result.lookup.candidates.first())
}
