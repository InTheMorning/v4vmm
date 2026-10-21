//! The release page commands of `LibraryApp` (ADR 0084 Decision 4, packet
//! 021).
//!
//! The shared release page shell sends each command of a Library release
//! page and of a singles group page here. Each command reads the open page
//! again, so it never acts on the data of an old render.

#![warn(clippy::pedantic)]

use gpui::{ClipboardItem, Context, Window};

use super::{LibraryApp, LibraryDetail};
use crate::db::TrackRow;
use crate::ui::shells::release_page::click_modifiers;
use crate::view_models::release_page::{inspect_sources_entry, ReleasePageCommand};
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart};

impl LibraryApp {
    /// Runs one command of the open Library release page or singles group
    /// page.
    pub(crate) fn run_release_page_command(
        &mut self,
        command: ReleasePageCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            ReleasePageCommand::DownloadTrack(row) => {
                if let Some((track, _)) = self.release_page_row(row) {
                    self.download_library_tracks(&[track.id], cx);
                    cx.notify();
                }
            }
            // ADR 0083 Decision 5: "Remove…" of a row always asks for
            // confirmation.
            ReleasePageCommand::RemoveTrack(row) => {
                if let Some((track, _)) = self.release_page_row(row) {
                    self.remove_track_after_confirmation(track.id, window, cx);
                    cx.notify();
                }
            }
            // ADR 0083 task 006, decision D4: open the track page of the
            // row. The track page opens its "Inspect sources" disclosure
            // when it mounts.
            ReleasePageCommand::InspectSources(row) => {
                if let Some((track, _)) = self.release_page_row(row) {
                    self.request_inspect_sources(track.id);
                    self.open_page(
                        inspect_sources_entry(&track),
                        NavigationPathStart::CurrentPath,
                        cx,
                    );
                }
            }
            ReleasePageCommand::AddTrackToPlaylist(row, playlist_id) => {
                if let Some((track, _)) = self.release_page_row(row) {
                    self.add_track_to_playlist(track.id, playlist_id, cx);
                }
            }
            ReleasePageCommand::CreatePlaylistForTrack(row, name) => {
                if let Some((track, _)) = self.release_page_row(row) {
                    self.create_playlist_and_add_track(&name, track.id, cx);
                }
            }
            ReleasePageCommand::ClickRow(row, modifiers) => {
                if let Some((_, open_entry)) = self.release_page_row(row) {
                    self.click_library_detail_row(&open_entry, click_modifiers(modifiers), cx);
                }
            }
            ReleasePageCommand::ToggleRow(row) => {
                if let Some((_, open_entry)) = self.release_page_row(row) {
                    self.toggle_library_detail_row(&open_entry, cx);
                }
            }
            ReleasePageCommand::OpenNameLink(target) => self.open_album_name_link(&target, cx),
            command => self.run_release_command(command, window, cx),
        }
    }

    /// Runs one release command of the open Library release page.
    fn run_release_command(
        &mut self,
        command: ReleasePageCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let LibraryDetail::Album(album) = &self.detail else {
            return;
        };
        let Some(feed_id) = album.feed_id else {
            return;
        };
        let album = album.clone();
        match command {
            ReleasePageCommand::DownloadRelease => self.download_library_feeds(&[feed_id], cx),
            ReleasePageCommand::CopyFeedUrl => {
                if let Some(url) = album.feed_url {
                    cx.write_to_clipboard(ClipboardItem::new_string(url));
                }
            }
            ReleasePageCommand::MusicBrainzLookup => self.musicbrainz_feed(album, cx),
            ReleasePageCommand::RemoveRelease => {
                self.remove_album_after_confirmation(feed_id, window, cx);
            }
            ReleasePageCommand::AddReleaseToPlaylist(playlist_id) => {
                self.add_album_to_playlist(feed_id, playlist_id, cx);
                return;
            }
            ReleasePageCommand::CreatePlaylistForRelease(name) => {
                self.create_playlist_and_add_album(&name, feed_id, cx);
                return;
            }
            ReleasePageCommand::ToggleDescription => {
                self.toggle_album_description(feed_id, cx);
                return;
            }
            ReleasePageCommand::OpenNameLink(_)
            | ReleasePageCommand::DownloadTrack(_)
            | ReleasePageCommand::RemoveTrack(_)
            | ReleasePageCommand::InspectSources(_)
            | ReleasePageCommand::AddTrackToPlaylist(..)
            | ReleasePageCommand::CreatePlaylistForTrack(..)
            | ReleasePageCommand::ClickRow(..)
            | ReleasePageCommand::ToggleRow(_) => return,
        }
        cx.notify();
    }

    /// The track of row `row` on the open page, and the page that a plain
    /// click on the row opens. A release page row opens its track page. A
    /// singles group page row opens the release page of its single.
    fn release_page_row(&self, row: usize) -> Option<(TrackRow, FrameNavigationEntry)> {
        match &self.detail {
            LibraryDetail::Album(album) => album.release_track_rows().get(row).map(|track| {
                (
                    track.clone(),
                    FrameNavigationEntry::Track(track.track_key()),
                )
            }),
            LibraryDetail::SinglesGroup(key) => {
                let page = self.singles_group_page(key);
                page.rows
                    .into_iter()
                    .nth(row)
                    .map(|single| (single.track, FrameNavigationEntry::Release(single.key)))
            }
            _ => None,
        }
    }
}
