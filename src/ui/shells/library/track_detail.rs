//! Library track detail core surface.
//!
//! Renders the Library track header, hero artwork, primary actions, external
//! identity links, and summary rows. Metadata editing remains in `src/library.rs`
//! until Slice L6.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, Context, Image, SharedString};

use crate::api::Track;
use crate::db;
use crate::feed_service::track_row_to_track_context;
use crate::library::{InspectorFrame, LazyPanel, LibraryApp};
use crate::metadata::{TagCompareResult, TrackContext};
use crate::ui::composites::TrackSurfaceElement;
use crate::ui::shells::library::track_detail_metadata::{
    pending_id3_edits_for_track_detail, render_library_track_detail_actions,
    render_library_track_detail_metadata,
};
use crate::ui::shells::track;
use crate::ui::style::spacing;
use crate::view_models::library::LibraryChromeDisplay;
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
use crate::views::TrackView;

pub(crate) fn render_library_track_detail(
    frame: &InspectorFrame,
    playlists: &[db::Playlist],
    chrome: &LibraryChromeDisplay,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let context = track_row_to_track_context(&frame.track);
    let context = frame.source_context.as_ref().unwrap_or(&context);
    let result = match &frame.tag_compare {
        LazyPanel::Loaded(result) => Some(result),
        LazyPanel::Loading | LazyPanel::Empty(_) | LazyPanel::Hidden => None,
    };
    div()
        .id(SharedString::from(chrome.track_detail_scroll_id))
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .p(spacing::LG)
        .child(render_library_track_window(
            frame, context, result, playlists, cx,
        ))
        .into_any_element()
}

fn render_library_track_window(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let pending_id3_edits = pending_id3_edits_for_track_detail(frame, track_context, result);
    let track_core = render_library_track_detail_core(
        &track_context.track,
        &frame.title,
        frame.image.clone(),
        render_library_track_detail_actions(frame, &pending_id3_edits, playlists, cx),
    );

    render_library_track_detail_metadata(
        frame,
        track_context,
        result,
        &pending_id3_edits,
        track_core,
        cx,
    )
}

pub(crate) fn render_library_track_detail_core(
    track: &Track,
    override_title: &str,
    hero_image: Option<Arc<Image>>,
    primary_action_row: AnyElement,
) -> AnyElement {
    let track_view = TrackView::from_api(track.clone());
    let detail_page = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Library)
        .with_override_title(Some(override_title))
        .page();

    track::build_track_detail_surface(
        &detail_page,
        track::TrackDetailBehaviorSlots {
            hero_image,
            external_links: track::render_track_page_identity_actions(&detail_page),
            primary_actions: vec![TrackSurfaceElement::from_element(primary_action_row)],
            ..track::TrackDetailBehaviorSlots::default()
        },
    )
    .into_any_element()
}
