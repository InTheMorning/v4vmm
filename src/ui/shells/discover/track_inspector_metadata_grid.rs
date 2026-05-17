//! Discover track inspector metadata grid rendering.
//!
//! This module owns the Discover metadata grid entry point and drag preview.
//! Cell, expansion, tree, and test-helper details live in bounded sibling
//! modules so screen-shell files stay small.

#![warn(clippy::pedantic)]

#[path = "track_inspector_metadata_cells.rs"]
mod cells;
#[path = "track_inspector_metadata_expandable.rs"]
mod expandable;
#[cfg(test)]
#[path = "track_inspector_metadata_test_helpers.rs"]
mod test_helpers;
#[path = "track_inspector_metadata_tree.rs"]
mod tree;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, Context, FontWeight, Image, IntoElement, Render, SharedString,
    Styled, Window,
};

use crate::discover::SearchApp;
use crate::metadata::{MetadataGridRow, PendingId3Edit};
use crate::ui::composites::TrackMetadataGrid;
use crate::ui::layouts as layout;
use crate::ui::primitives::MultilineText;
use crate::ui::style::{color, radius, spacing, typography};
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::track_metadata_grid::{
    TrackMetadataDragPreviewDisplay, TrackMetadataGridVm,
};

#[cfg(test)]
pub(crate) use cells::metadata_drag_value;
#[cfg(test)]
pub(crate) use test_helpers::{
    id3_frame_hint, metadata_data_row, unused_id3v24_frames_for_group, used_id3_fields_for_group,
};

pub(crate) struct MetadataDragPreview {
    pub(crate) display: TrackMetadataDragPreviewDisplay,
}

impl Render for MetadataDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(layout::MENU_MIN_WIDTH)
            .rounded(radius::MD)
            .border_1()
            .border_color(color::accent())
            .bg(color::bg_surface())
            .p(spacing::SM)
            .child(
                div()
                    .text_size(typography::SIZE_MICRO)
                    .font_weight(FontWeight::BOLD)
                    .text_color(color::text_muted())
                    .child(SharedString::from(self.display.label.clone())),
            )
            .child(
                div().mt(spacing::XS).child(
                    MultilineText::from_text(&self.display.value)
                        .max_lines(4)
                        .size(FontSize::Micro)
                        .line_height(typography::LINE_BODY)
                        .color(SemanticColor::Label),
                ),
            )
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "metadata grid needs explicit column state and edit state inputs"
)]
pub(crate) fn discover_track_metadata_grid(
    rows: Vec<MetadataGridRow>,
    show_id3: bool,
    show_musicbrainz: bool,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    expanded_metadata_cells: &BTreeSet<String>,
    file_image: Option<Arc<Image>>,
    tag_column_label: &str,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let vm = TrackMetadataGridVm::new(show_id3, show_musicbrainz, tag_column_label);
    let mut cells: Vec<AnyElement> = Vec::new();

    for row in rows {
        match row {
            MetadataGridRow::Group(group) => {
                cells.push(cells::metadata_group_cell(group, vm.columns(), cx));
            }
            MetadataGridRow::Data(row) => {
                let pending = pending_id3_edits.get(&row.row_id);
                let expansion = vm.expansion_for(&row.row_id, expanded_metadata_cells);
                cells.push(cells::metadata_rss_cell(
                    &row,
                    pending,
                    expansion.rss_expanded,
                    expanded_metadata_cells,
                    cx,
                ));
                if show_id3 {
                    cells.push(cells::metadata_id3_cell(
                        &row,
                        pending,
                        expansion.id3_expanded,
                        expanded_metadata_cells,
                        file_image.clone(),
                        cx,
                    ));
                }
                if show_musicbrainz {
                    cells.push(cells::metadata_musicbrainz_cell(&row, pending, cx));
                }
            }
        }
    }

    TrackMetadataGrid::new(vm).cells(cells).into_any_element()
}
