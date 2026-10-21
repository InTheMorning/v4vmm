//! The track page commands of `LibraryApp` (ADR 0084 Decision 4, packet
//! 023).
//!
//! The shared track page shell sends each command of a Library track page
//! here. Each command reads the open track again, so it never acts on the
//! data of an old render.

#![warn(clippy::pedantic)]

use gpui::{ClipboardItem, Context, Window};

use super::{LibraryApp, LibraryDetail};
use crate::feed_service::track_row_to_track_context;
use crate::view_models::track_detail::TrackPageAction;
use crate::view_models::track_page::{TrackFeedFacts, TrackPageCommand};

impl LibraryApp {
    /// Runs one command of the open Library track page.
    pub(crate) fn run_track_page_command(
        &mut self,
        command: TrackPageCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let LibraryDetail::Track(frame) = &self.detail else {
            return;
        };
        let track_id = frame.entity_id;
        match command {
            TrackPageCommand::Action(action) => match action {
                TrackPageAction::DownloadTrack => self.toggle_local_subscription(window, cx),
                TrackPageAction::CopyFeedUrl => {
                    if let Some(url) = self.open_track_feed_url() {
                        cx.write_to_clipboard(ClipboardItem::new_string(url));
                    }
                }
                TrackPageAction::MusicBrainzLookup => self.show_musicbrainz_lookup(cx),
                TrackPageAction::RemoveTrack => {
                    self.remove_track_after_confirmation(track_id, window, cx);
                }
                // A Library track page has no "Download album" action, and
                // "Add to playlist" opens its popover.
                TrackPageAction::DownloadAlbum | TrackPageAction::AddToPlaylist => {}
            },
            TrackPageCommand::AddToPlaylist(playlist_id) => {
                self.add_track_to_playlist(track_id, playlist_id, cx);
            }
            TrackPageCommand::CreatePlaylist(name) => {
                self.create_playlist_and_add_track(&name, track_id, cx);
            }
            TrackPageCommand::ToggleDescription => self.toggle_track_description(cx),
            TrackPageCommand::OpenNameLink(target) => self.open_track_name_link(&target, cx),
        }
    }

    /// The feed URL of the open Library track page, from the facts of its
    /// feed. `None` when the feed states no URL.
    fn open_track_feed_url(&self) -> Option<String> {
        let LibraryDetail::Track(frame) = &self.detail else {
            return None;
        };
        let context = track_row_to_track_context(&frame.track);
        let context = frame.source_context.as_ref().unwrap_or(&context);
        context
            .feed
            .as_ref()
            .map(TrackFeedFacts::from_api_feed)
            .and_then(|feed| feed.feed_url)
    }
}
