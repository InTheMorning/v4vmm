//! Library track page host (ADR 0084 Decision 4, packet 023).
//!
//! Copies the Library track facts into the shared track page view model,
//! and renders the page through the shared track page shell. The "Inspect
//! sources" disclosure comes from `track_detail_metadata`. Each command
//! goes to `LibraryApp::run_track_page_command`.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{AnyElement, Context, ScrollHandle};

use crate::db;
use crate::feed_service::track_row_to_track_context;
use crate::library::{InspectorFrame, LazyPanel, LibraryApp};
use crate::ui::shells::library::track_detail_metadata::{
    pending_id3_edits_for_track_detail, render_library_track_detail_metadata,
};
use crate::ui::shells::track_page::{render_track_page, TrackPageDispatch, TrackPageImages};
use crate::view_models::library::LibraryChromeDisplay;
use crate::view_models::named_feeds::NamedFeed;
use crate::view_models::track_detail::LibraryTrackActionState;
use crate::view_models::track_page::{track_page_vm, LibraryTrackPageState, TrackPageFacts};

pub(crate) fn render_library_track_detail(
    frame: &InspectorFrame,
    playlists: &[db::Playlist],
    chrome: &LibraryChromeDisplay,
    named_feeds: &[NamedFeed],
    release_download_active: bool,
    scroll: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let context = track_row_to_track_context(&frame.track);
    let context = frame.source_context.as_ref().unwrap_or(&context);
    let result = match &frame.tag_compare {
        LazyPanel::Loaded(result) => Some(result),
        LazyPanel::Loading | LazyPanel::Empty(_) | LazyPanel::Hidden => None,
    };
    let inspector_display = frame.inspector_display(context.track.description.as_deref());
    let state = LibraryTrackPageState {
        track_id: frame.entity_id,
        album_feed_id: frame.track.feed_id,
        title: frame.title.clone(),
        action_state: LibraryTrackActionState {
            downloaded: frame.local_subscription,
            subscription_busy: frame.subscription_busy,
            musicbrainz_available: inspector_display.musicbrainz_enabled,
            release_download_active,
        },
        subscription_message: frame.subscription_message.clone(),
        // ADR 0047 Phase C: an unavailable metadata action stays visible
        // and disabled, and says why.
        musicbrainz_reason: frame.inspector_display(None).musicbrainz_tooltip_text(),
        container_id: chrome.track_detail_scroll_id,
    };
    let facts = TrackPageFacts::from_library_track(context, state)
        .with_named_feeds(named_feeds.to_vec())
        .with_playlists(playlists)
        .with_description_state(inspector_display.description_state);
    let vm = track_page_vm(&facts);

    let pending_id3_edits = pending_id3_edits_for_track_detail(frame, context, result);
    let inspect_sources =
        render_library_track_detail_metadata(frame, context, result, &pending_id3_edits, cx);
    let entity = cx.entity();
    let dispatch: TrackPageDispatch = Rc::new(move |command, window, cx| {
        entity.update(cx, |this, cx| {
            this.run_track_page_command(command, window, cx);
        });
    });
    render_track_page(
        &vm,
        TrackPageImages {
            hero_image: frame.image.clone(),
            scroll_handle: scroll.clone(),
        },
        &dispatch,
        Some(inspect_sources),
        cx,
    )
}
