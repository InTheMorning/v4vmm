//! The scope "Library" or "Everything" of the Releases list and the
//! Artists list (ADR 0084 Decision 8, packet 016).
//!
//! `LibraryApp` keeps the scope of each list in memory (decision D3). The
//! scope "Everything" loads the `MusicIndex` release name pages or artist
//! name pages through `present_command` on the ADR 0040 command runner. A
//! result of an older request generation changes nothing (decision D10).
//! Each release page load also reads the `MusicIndex` singles of each new
//! artist feed of its singles, in the same command (packet 026 decision
//! D4).
//!
//! The singles group page and the page of a single read the `MusicIndex`
//! singles of one feed when the app has no result for it (packet 026 Part
//! B, decisions D12 and D14). That request also runs through
//! `present_command`, and an answer of an older generation changes
//! nothing.

#![warn(clippy::pedantic)]

use gpui::Context;

use super::app_impl::command_error_detail;
use super::{LibraryApp, LibraryDetail};
use crate::application::command_bus::ApplicationCommand;
use crate::application::queries::name_lists::{
    FetchArtistNamePage, FetchFeedSingles, FetchReleaseNamePage,
};
use crate::application::CommandContext;
use crate::presentation::present_command;
use crate::view_models::index_feed_singles::{finish_release_page, FeedSinglesResult};
use crate::view_models::index_name_list::{
    IndexListStatusRowVm, IndexNameListBatch, IndexNameListLoadIntent, IndexNameListPage,
    MusicListScope, MusicScopeControlDisplay,
};
use crate::view_models::library::{AlbumNode, LibraryViewModel, ReleasesListEntryVm};
use crate::view_models::releases_everything::reset_selection_on_scope_change;
use crate::view_models::singles_group::{
    group_page_feed_to_read, index_page_feed_to_read, library_page_feed_to_read, SinglesGroupKey,
};
use crate::view_models::workspace::MusicDestination;
use crate::views::FeedView;

impl LibraryApp {
    /// The scope control of the active destination. Only Releases and
    /// Artists have one (packet 016 decision D1).
    pub(crate) fn music_scope_control(&self) -> Option<MusicScopeControlDisplay> {
        let destination = self.vm.active_destination();
        let selected = match destination {
            MusicDestination::Releases => self.releases_scope,
            MusicDestination::Artists => self.artists_scope,
            MusicDestination::Discover
            | MusicDestination::Tracks
            | MusicDestination::NeedsAttention
            | MusicDestination::Playlists => return None,
        };
        MusicScopeControlDisplay::for_destination(destination, selected)
    }

    /// The entries of the Releases list in its present scope.
    pub(crate) fn releases_entries(&self) -> Vec<ReleasesListEntryVm> {
        self.vm.releases_list(
            self.releases_scope,
            &self.release_name_page,
            self.vm.index_feed_singles(),
            &self.expanded_singles_groups,
        )
    }

    /// The status row of the Releases list in its present scope.
    pub(crate) fn releases_status(&self) -> Option<IndexListStatusRowVm> {
        LibraryViewModel::releases_list_status(self.releases_scope, &self.release_name_page)
    }

    /// Sets the scope of the active destination. The scope "Everything"
    /// loads page 1 of its name list when no request was sent. A new
    /// Releases scope clears the Releases selection and ends selection
    /// mode (decision D15).
    pub(crate) fn set_music_list_scope(&mut self, scope: MusicListScope, cx: &mut Context<Self>) {
        match self.vm.active_destination() {
            MusicDestination::Releases => {
                reset_selection_on_scope_change(
                    self.releases_scope,
                    scope,
                    &mut self.releases_selection,
                    &mut self.releases_selection_mode,
                );
                self.releases_scope = scope;
                if scope == MusicListScope::Everything && self.release_name_page.is_not_started() {
                    let intent = self.release_name_page.begin_load(false);
                    self.send_release_name_request(intent, cx);
                }
            }
            MusicDestination::Artists => {
                self.artists_scope = scope;
                if scope == MusicListScope::Everything && self.artist_name_page.is_not_started() {
                    let intent = self.artist_name_page.begin_load(false);
                    self.send_artist_name_request(intent, cx);
                }
            }
            MusicDestination::Discover
            | MusicDestination::Tracks
            | MusicDestination::NeedsAttention
            | MusicDestination::Playlists => return,
        }
        cx.notify();
    }

    /// "Load more" at the end of the Artists list (packet 016 decision D7).
    pub(crate) fn load_more_artist_names(&mut self, cx: &mut Context<Self>) {
        let intent = self.artist_name_page.begin_load(true);
        self.send_artist_name_request(intent, cx);
    }

    /// "Try again" on the error row of the Artists list (packet 016
    /// decision D8).
    pub(crate) fn retry_artist_names(&mut self, cx: &mut Context<Self>) {
        let intent = self.artist_name_page.begin_retry();
        self.send_artist_name_request(intent, cx);
    }

    /// "Load more" at the end of the Releases list (packet 016 decision
    /// D7).
    pub(crate) fn load_more_release_names(&mut self, cx: &mut Context<Self>) {
        let intent = self.release_name_page.begin_load(true);
        self.send_release_name_request(intent, cx);
    }

    /// "Try again" on the error row of the Releases list (packet 016
    /// decision D8). When the rows of the page wait for a failed feed
    /// request, the command sends only the failed requests (packet 026
    /// decision D10).
    pub(crate) fn retry_release_names(&mut self, cx: &mut Context<Self>) {
        if let Some((pending, generation)) = self.release_name_page.begin_pending_retry() {
            let command =
                FetchReleaseNamePage::retry_pending(self.musicindex_endpoint.clone(), pending);
            self.send_release_page_command(command, generation, cx);
            return;
        }
        let intent = self.release_name_page.begin_retry();
        self.send_release_name_request(intent, cx);
    }

    fn send_release_name_request(
        &mut self,
        intent: Option<IndexNameListLoadIntent>,
        cx: &mut Context<Self>,
    ) {
        let Some(IndexNameListLoadIntent { cursor, generation }) = intent else {
            return;
        };
        let command = FetchReleaseNamePage::new(self.musicindex_endpoint.clone(), cursor);
        self.send_release_page_command(command, generation, cx);
    }

    /// Sends the release page `command` of the page `generation`. Each page
    /// also reads each Library artist feed with one or more Library singles
    /// (packet 026). The command sends no request for a feed with a loaded
    /// result, and no request for a feed whose request already runs, for
    /// example a request that the group page or the single page started
    /// (packet 026 review). A feed that a Library change adds after page 1
    /// is thus read with the next page.
    fn send_release_page_command(
        &mut self,
        command: FetchReleaseNamePage,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        let library_feeds = self.vm.library_single_artist_feeds();
        let command = command.with_feeds(
            library_feeds,
            self.vm.index_feed_singles().loaded_or_running_feeds(),
        );
        let singles_generation = self.vm.index_feed_singles_mut().begin_request();
        cx.notify();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, load, cx| {
                if finish_release_page(
                    &mut this.release_name_page,
                    this.vm.index_feed_singles_mut(),
                    load,
                    generation,
                    singles_generation,
                ) {
                    cx.notify();
                }
            },
            move |this, error, cx| {
                if this
                    .release_name_page
                    .fail_load(command_error_detail(error), generation)
                {
                    cx.notify();
                }
            },
        );
    }

    /// Reads the `MusicIndex` singles of the feed of the group page `key`,
    /// when the app has no result for it and no request runs (packet 026
    /// Part B). A name identity sends no request.
    pub(crate) fn request_singles_group_feed(
        &mut self,
        key: &SinglesGroupKey,
        cx: &mut Context<Self>,
    ) {
        if let Some(feed_guid) = group_page_feed_to_read(key, self.vm.index_feed_singles()) {
            self.request_feed_singles(feed_guid, cx);
        }
    }

    /// "Try again" on the error row of the open group page (packet 026
    /// Part B). It sends the request for the feed of the page again.
    pub(crate) fn retry_singles_group_feed(&mut self, cx: &mut Context<Self>) {
        if let LibraryDetail::SinglesGroup(key) = &self.detail {
            let key = key.clone();
            self.request_singles_group_feed(&key, cx);
        }
    }

    /// Reads the `MusicIndex` singles of the artist feed of the Library
    /// single `album`, when the app has no result for it (packet 026
    /// decision D14). The page then shows "All n singles".
    pub(crate) fn request_library_single_feed(
        &mut self,
        album: &AlbumNode,
        cx: &mut Context<Self>,
    ) {
        if let Some(feed_guid) = library_page_feed_to_read(album, self.vm.index_feed_singles()) {
            self.request_feed_singles(feed_guid, cx);
        }
    }

    /// Reads the `MusicIndex` singles of the artist feed of the loaded
    /// Index single `feed`, when the app has no result for it (packet 026
    /// decision D14).
    pub(crate) fn request_index_single_feed(&mut self, feed: &FeedView, cx: &mut Context<Self>) {
        if let Some(feed_guid) = index_page_feed_to_read(feed, self.vm.index_feed_singles()) {
            self.request_feed_singles(feed_guid, cx);
        }
    }

    /// Sends one request for the singles of `feed_guid`. It sends nothing
    /// when the app has a result or a request runs, so each page open
    /// sends at most one request (packet 026 Part B).
    fn request_feed_singles(&mut self, feed_guid: String, cx: &mut Context<Self>) {
        let Some(generation) = self
            .vm
            .index_feed_singles_mut()
            .begin_feed_request(&feed_guid)
        else {
            return;
        };
        cx.notify();
        let failed_guid = feed_guid.clone();
        present_command(
            &self.command_runner,
            FetchFeedSingles::new(self.musicindex_endpoint.clone(), feed_guid),
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                if this
                    .vm
                    .index_feed_singles_mut()
                    .finish_feed_request(result, generation)
                {
                    cx.notify();
                }
            },
            move |this, error, cx| {
                let result = FeedSinglesResult {
                    feed_guid: failed_guid,
                    rows: Err(command_error_detail(error)),
                };
                if this
                    .vm
                    .index_feed_singles_mut()
                    .finish_feed_request(result, generation)
                {
                    cx.notify();
                }
            },
        );
    }

    fn send_artist_name_request(
        &mut self,
        intent: Option<IndexNameListLoadIntent>,
        cx: &mut Context<Self>,
    ) {
        let endpoint = self.musicindex_endpoint.clone();
        self.send_name_request(
            intent,
            |cursor| FetchArtistNamePage::new(endpoint, cursor),
            |this| &mut this.artist_name_page,
            cx,
        );
    }

    /// Sends the request of `intent`, when there is one. `command` makes
    /// the command for the cursor, and `page` names the page state of the
    /// list. The result changes the page only when its generation is
    /// current.
    fn send_name_request<C, Row>(
        &mut self,
        intent: Option<IndexNameListLoadIntent>,
        command: impl FnOnce(Option<String>) -> C,
        page: fn(&mut Self) -> &mut IndexNameListPage<Row>,
        cx: &mut Context<Self>,
    ) where
        C: ApplicationCommand<Output = IndexNameListBatch<Row>>,
        Row: 'static,
    {
        let Some(IndexNameListLoadIntent { cursor, generation }) = intent else {
            return;
        };
        cx.notify();
        present_command(
            &self.command_runner,
            command(cursor),
            CommandContext::next(),
            cx,
            move |this, batch, cx| {
                if page(this).finish_load(batch, generation) {
                    cx.notify();
                }
            },
            move |this, error, cx| {
                if page(this).fail_load(command_error_detail(error), generation) {
                    cx.notify();
                }
            },
        );
    }
}
