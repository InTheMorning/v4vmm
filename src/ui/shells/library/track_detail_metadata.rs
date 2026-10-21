//! Library track detail metadata surface.
//!
//! Owns the ID3 compare panel, `MusicBrainz` lookup panel, metadata action row,
//! and staged edit controls. The metadata grid/cell renderers are split into
//! sibling module `track_detail_metadata_grid` to keep this shell bounded.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;

use gpui::{div, prelude::*, AnyElement, Context, SharedString, Styled};

use super::track_detail_metadata_grid::library_track_metadata_grid;
pub(crate) use super::track_detail_metadata_grid::track_metadata_rows_for_frame;
use crate::library::{InspectorFrame, LazyPanel, LibraryApp};
use crate::media::image_from_bytes;
use crate::metadata::{
    auto_populated_pending_id3_edits, pending_id3_conflict_descriptions, PendingId3Edit,
    TagCompareResult, TrackContext,
};
use crate::ui::composites::{
    action_button, ActionButtonDisplay, ActionRow, ActionRowDisplay, ActionRowMessage,
    DisclosureGroup, DisclosureGroupDisplay, FileHeader, MusicBrainzPanel,
};
use crate::ui::primitives::LoadingMessage;
use crate::ui::style::spacing;
use crate::view_models::entity_detail::{
    EntityActionTarget, EntitySurfaceContext, MetadataPanelState, TrackMetadataActionState,
};
use crate::view_models::metadata::FileHeaderVm;
use crate::view_models::musicbrainz_panel::MusicBrainzPanelVm;
use crate::view_models::track_metadata_grid::TrackMetadataGridVm;
use crate::views::TrackRef;

/// The "Inspect sources" disclosure of a Library track page (ADR 0083
/// Decision 8). The shared track page shell adds it below the page. The
/// disclosure is closed by default. Opened, it holds Compare ID3, the
/// `MusicBrainz` panel, waiting ID3 edits and the compare grid.
pub(crate) fn render_library_track_detail_metadata(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let inspector_display = frame.inspector_display(track_context.track.description.as_deref());
    let open = inspector_display.inspect_sources_open();

    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(
            DisclosureGroup::new(DisclosureGroupDisplay {
                id: "library-track-inspect-sources".into(),
                label: SharedString::from(INSPECT_SOURCES_LABEL),
                a11y_label: SharedString::from(INSPECT_SOURCES_A11Y_LABEL),
            })
            .collapsed(!open)
            .on_toggle(cx.listener(|this, _, _, cx| {
                this.toggle_inspect_sources(cx);
            })),
        )
        .when(open, |el| {
            el.child(render_inspect_sources_body(
                frame,
                track_context,
                result,
                pending_id3_edits,
                cx,
            ))
        })
        .into_any_element()
}

/// The label of the "Inspect sources" disclosure (ADR 0083 Decision 8).
const INSPECT_SOURCES_LABEL: &str = "Inspect sources";
const INSPECT_SOURCES_A11Y_LABEL: &str = "Show or hide the source values of this track";

fn render_inspect_sources_body(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let inspector_display = frame.inspector_display(track_context.track.description.as_deref());
    let show_id3_panel = inspector_display.show_compare_id3_panel();
    let show_musicbrainz_panel = inspector_display.show_musicbrainz_panel();
    let panel_columns = u16::from(show_id3_panel) + u16::from(show_musicbrainz_panel);
    let rows = track_metadata_rows_for_frame(frame, track_context, result);
    let tag_column_label = TrackMetadataGridVm::tag_column_label(
        result
            .and_then(|result| result.format)
            .map(crate::audio_format::AudioFormat::display_label),
    );

    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(render_inspect_sources_controls(
            frame,
            pending_id3_edits,
            cx,
        ))
        .when(panel_columns > 0, |el| {
            el.child(
                div()
                    .grid()
                    .grid_cols(panel_columns)
                    .gap(spacing::XL)
                    .items_start()
                    .when(show_id3_panel, |el| {
                        el.child(render_track_compare_panel(frame, result, cx))
                    })
                    .when(show_musicbrainz_panel, |el| {
                        el.child(library_musicbrainz_panel(frame, cx))
                    }),
            )
        })
        .child(library_track_metadata_grid(
            rows,
            show_id3_panel,
            show_musicbrainz_panel,
            pending_id3_edits,
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

pub(crate) fn pending_id3_edits_for_track_detail(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
) -> BTreeMap<String, PendingId3Edit> {
    let rows = track_metadata_rows_for_frame(frame, track_context, result);
    if let Some(result) = result {
        auto_populated_pending_id3_edits(
            &rows,
            &frame.pending_id3_edits,
            &frame.suppressed_auto_id3_edits,
            result.format,
        )
    } else {
        frame.pending_id3_edits.clone()
    }
}

/// The controls inside "Inspect sources": Compare ID3, and the waiting ID3
/// edits with Apply and Discard.
fn render_inspect_sources_controls(
    frame: &InspectorFrame,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let pending_conflicts = pending_id3_conflict_descriptions(pending_id3_edits);
    let metadata_state = track_metadata_action_state(frame);
    let inspector_display = frame.inspector_display(None);
    let metadata_target = EntityActionTarget::Track(TrackRef::LocalTrackId(frame.entity_id));
    let compare_action = metadata_state.compare_action(metadata_target);

    let mut row = ActionRow::new(ActionRowDisplay {
        a11y_label: SharedString::from(INSPECT_SOURCES_A11Y_LABEL),
    });

    if let Some(action) = compare_action {
        let a11y = action.a11y_label();
        let disabled = !action.enabled || !inspector_display.compare_id3_enabled;
        let mut button = action_button(
            ActionButtonDisplay {
                label: SharedString::from(action.label),
                a11y_label: SharedString::from(a11y),
            },
            cx,
        )
        .disabled(disabled);
        if let Some(tooltip) = inspector_display.compare_id3_tooltip_text() {
            button = button.tooltip(tooltip);
        }
        row = if disabled {
            row.control(button)
        } else {
            row.control(button.on_click(cx.listener(|this, _, _, cx| {
                this.toggle_tag_compare(cx);
            })))
        };
    }

    let conflict_text = (!pending_conflicts.is_empty()).then(|| pending_conflicts.join("; "));
    if let Some(staged_display) = metadata_state.staged_id3_edits_display(
        pending_id3_edits.len(),
        frame.applying_id3_edits,
        conflict_text.as_deref(),
    ) {
        row = row.message(ActionRowMessage::from_status_display(
            staged_display.message,
        ));
        let mut staged = vec![action_button(
            ActionButtonDisplay {
                label: SharedString::from(staged_display.apply_label.clone()),
                a11y_label: SharedString::from(staged_display.apply_label),
            },
            cx,
        )
        .disabled(!staged_display.apply_enabled)
        .on_click(cx.listener(|this, _, _, cx| {
            this.apply_pending_id3_edits(cx);
        }))
        .into_any_element()];
        if staged_display.show_discard {
            staged.push(
                action_button(
                    ActionButtonDisplay {
                        label: SharedString::from(staged_display.discard_label),
                        a11y_label: SharedString::from(staged_display.discard_label),
                    },
                    cx,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.clear_pending_id3_edits(cx);
                }))
                .into_any_element(),
            );
        }
        row = row.control_group(staged);
        if let Some(conflict_message) = staged_display.conflict_message {
            row = row.message(ActionRowMessage::from_status_display(conflict_message));
        }
    }

    if let Some(error) = frame.id3_apply_error.clone() {
        row = row.message(ActionRowMessage::from_status_display(
            TrackMetadataActionState::id3_apply_error_display(error),
        ));
    }

    row.into_any_element()
}

fn track_metadata_action_state(frame: &InspectorFrame) -> TrackMetadataActionState {
    let inspector_display = frame.inspector_display(None);
    let compare = if inspector_display.show_compare_id3_panel() {
        metadata_panel_state(&frame.tag_compare)
    } else {
        MetadataPanelState::Hidden
    };
    let musicbrainz = if inspector_display.show_musicbrainz_panel() {
        metadata_panel_state(&frame.musicbrainz_lookup)
    } else {
        MetadataPanelState::Hidden
    };
    TrackMetadataActionState::new(
        EntitySurfaceContext::Library,
        compare,
        musicbrainz,
        inspector_display.compare_id3_enabled && inspector_display.musicbrainz_enabled,
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

fn render_track_compare_panel(
    frame: &InspectorFrame,
    result: Option<&TagCompareResult>,
    cx: &mut Context<LibraryApp>,
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

fn library_musicbrainz_panel(frame: &InspectorFrame, cx: &mut Context<LibraryApp>) -> AnyElement {
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
