//! Discover track inspector metadata: ID3 frame editor grid,
//! tag-compare diff, and `MusicBrainz` lookup panel.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, Context, SharedString, Styled};

use crate::discover::{InspectorFrame, SearchApp};
use crate::metadata::{
    aligned_compare_rows, auto_populated_pending_id3_edits, expand_woar_metadata_rows,
    track_metadata_rows, MetadataGridRow, MusicBrainzLookupResult, TagCompareResult, TrackContext,
};
use crate::ui::composites::{action_button, ActionButtonDisplay, FileHeader, MusicBrainzPanel};
use crate::ui::primitives::LoadingMessage;
use crate::ui::shells::discover::track_inspector_metadata_grid::discover_track_metadata_grid;
use crate::ui::style::spacing;
use crate::view_models::entity_detail::{
    EntitySurfaceContext, MetadataPanelState, TrackMetadataActionState,
};
use crate::view_models::metadata::FileHeaderVm;
use crate::view_models::musicbrainz_panel::MusicBrainzPanelVm;
use crate::view_models::search::LazyPanel;
use crate::{media::image_from_bytes, musicbrainz::MusicBrainzCandidate};

pub(crate) fn render_discover_track_inspector_metadata(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let metadata_state = track_metadata_action_state(frame);
    let show_id3_panel = metadata_state.show_compare_panel();
    let show_musicbrainz_panel = metadata_state.show_musicbrainz_panel();
    let result = match &frame.tag_compare {
        LazyPanel::Loaded(result) => Some(result),
        LazyPanel::Hidden | LazyPanel::Loading | LazyPanel::Empty(_) => None,
    };

    if !show_id3_panel
        && !show_musicbrainz_panel
        && frame.pending_id3_edits.is_empty()
        && frame.id3_apply_error.is_none()
    {
        return div().into_any_element();
    }

    let rows = track_metadata_rows_for_frame(frame, track_context, result);
    let pending_id3_edits = if let Some(result) = result {
        auto_populated_pending_id3_edits(
            &rows,
            &frame.pending_id3_edits,
            &frame.suppressed_auto_id3_edits,
            result.format,
        )
    } else {
        frame.pending_id3_edits.clone()
    };
    let tag_column_label =
        crate::view_models::track_metadata_grid::TrackMetadataGridVm::tag_column_label(
            result
                .and_then(|result| result.format)
                .map(crate::audio_format::AudioFormat::display_label),
        );

    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .when(show_id3_panel || show_musicbrainz_panel, |el| {
            let columns = u16::from(show_id3_panel) + u16::from(show_musicbrainz_panel);
            el.child(
                div()
                    .grid()
                    .grid_cols(columns)
                    .gap(spacing::XL)
                    .items_start()
                    .when(show_id3_panel, |el| {
                        el.child(render_tag_compare_panel(frame, result, cx))
                    })
                    .when(show_musicbrainz_panel, |el| {
                        el.child(render_musicbrainz_panel(frame, cx))
                    }),
            )
        })
        .child(discover_track_metadata_grid(
            rows,
            show_id3_panel,
            show_musicbrainz_panel,
            &pending_id3_edits,
            &frame.expanded_metadata_cells,
            result.and_then(|result| {
                result
                    .file_image
                    .as_ref()
                    .and_then(|image| image_from_bytes(image.clone()))
            }),
            tag_column_label,
            cx,
        ))
        .into_any_element()
}

pub(crate) fn track_metadata_rows_for_frame(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
) -> Vec<MetadataGridRow> {
    let selected_musicbrainz = match &frame.musicbrainz_lookup {
        LazyPanel::Loaded(lookup) => selected_musicbrainz_candidate(frame, lookup),
        LazyPanel::Hidden | LazyPanel::Loading | LazyPanel::Empty(_) => None,
    };
    let show_musicbrainz = track_metadata_action_state(frame).show_musicbrainz_panel();
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

fn track_metadata_action_state(frame: &InspectorFrame) -> TrackMetadataActionState {
    TrackMetadataActionState::new(
        EntitySurfaceContext::Discover,
        metadata_panel_state(&frame.tag_compare),
        metadata_panel_state(&frame.musicbrainz_lookup),
        frame.entity_type == "track",
    )
}

fn metadata_panel_state<T>(panel: &LazyPanel<T>) -> MetadataPanelState {
    match panel {
        LazyPanel::Hidden => MetadataPanelState::Hidden,
        LazyPanel::Loading => MetadataPanelState::Loading,
        LazyPanel::Loaded(_) => MetadataPanelState::Loaded,
        LazyPanel::Empty(_) => MetadataPanelState::Empty,
    }
}

fn render_tag_compare_panel(
    frame: &InspectorFrame,
    result: Option<&TagCompareResult>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    match (&frame.tag_compare, result) {
        (LazyPanel::Loaded(_), Some(result)) => {
            let file_actions = TrackMetadataActionState::file_actions_display();
            FileHeader::new(FileHeaderVm::new(result))
                .image(
                    result
                        .file_image
                        .as_ref()
                        .and_then(|image| image_from_bytes(image.clone())),
                )
                .action(
                    action_button(
                        ActionButtonDisplay {
                            label: SharedString::from(file_actions.reread_label),
                            a11y_label: SharedString::from(file_actions.reread_label),
                        },
                        cx,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.reread_tag_compare(cx);
                    })),
                )
                .action(
                    action_button(
                        ActionButtonDisplay {
                            label: SharedString::from(file_actions.redownload_label),
                            a11y_label: SharedString::from(file_actions.redownload_label),
                        },
                        cx,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.redownload_tag_compare(cx);
                    })),
                )
                .into_any_element()
        }
        (LazyPanel::Loading, _) => {
            LoadingMessage::new(TrackMetadataActionState::compare_panel_loading_message())
                .into_any_element()
        }
        (LazyPanel::Empty(label), _) => LoadingMessage::from_text(label).into_any_element(),
        (LazyPanel::Hidden, _) | (LazyPanel::Loaded(_), None) => div().into_any_element(),
    }
}

fn render_musicbrainz_panel(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    match &frame.musicbrainz_lookup {
        LazyPanel::Loaded(result) => {
            let vm = MusicBrainzPanelVm::new(result, frame.musicbrainz_selected);
            let image = result
                .image
                .as_ref()
                .and_then(|image| image_from_bytes(image.clone()));
            let select_candidate = cx.listener(|this, idx: &usize, _window, cx| {
                this.select_musicbrainz_candidate(*idx, cx);
            });

            MusicBrainzPanel::new(vm)
                .image(image)
                .on_select(move |idx, window, cx| {
                    select_candidate(&idx, window, cx);
                })
                .into_any_element()
        }
        LazyPanel::Loading => {
            LoadingMessage::new(TrackMetadataActionState::musicbrainz_panel_loading_message())
                .into_any_element()
        }
        LazyPanel::Empty(label) => LoadingMessage::from_text(label).into_any_element(),
        LazyPanel::Hidden => div().into_any_element(),
    }
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
