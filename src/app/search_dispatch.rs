//! Search dispatch, Index async wiring, and drill-down helpers.

use std::rc::Rc;
use std::sync::Arc;

use gpui::{AnyElement, App, Context, Image, Window};

use crate::application::capability::Dependency;
use crate::application::capability_recovery::{RecoveryAction, RecoveryIntent};
use crate::application::commands::download::SubscribeThenAppendToPlaylist;
use crate::application::commands::playlist::CreatePlaylist;
use crate::application::queries::images::{FetchCoverColor, FetchThumbnail};
use crate::application::queries::search::{
    FetchIndexFeedDetail, FetchIndexSearchResults, FetchIndexTrackDetail,
};
use crate::application::{ApplicationCommand, CommandContext};
use crate::db;
use crate::feed_service;
use crate::library::{
    api_track_from_view, index_release_job, index_release_job_by_guid, index_track_job,
    DownloadJob, DownloadOutcome, LibraryApp,
};
use crate::library_service;
use crate::media::cover_color::CoverColor;
use crate::presentation::present_command;
use crate::ui::shells::release_page::{
    click_modifiers, render_release_page, ReleasePageDispatch, ReleasePageImages,
};
use crate::ui::shells::search_results_inspector::render_index_detail_display;
use crate::ui::shells::track_page::{render_track_page, TrackPageDispatch, TrackPageImages};
use crate::view_models::album_page::AlbumNameLinkTarget;
use crate::view_models::artist_identity::{ArtistNameDestination, ArtistNameKey};
use crate::view_models::detail_selection::{index_release_track_rows, DetailPageRows};
use crate::view_models::library::{
    display_description_text, keyed_description_state, toggle_keyed_description_state,
    DescriptionState, LibraryTrackRowVm, TrackRowWork,
};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::publisher_page::PublisherPageContext;
use crate::view_models::release_page::{
    index_feed_artwork_url, index_feed_guid, release_page_vm, ReleasePageCommand, ReleasePageFacts,
    ReleaseTrackRowVm,
};
use crate::view_models::search_results::{
    IndexDetailDisplay, IndexDetailKind, SearchResultTarget, SearchResultsInspectorPageVm,
    SearchResultsTab,
};
use crate::view_models::track_detail::{TrackNameLinkTarget, TrackPageAction};
use crate::view_models::track_page::{
    index_track_description_key, track_page_vm, TrackPageCommand, TrackPageFacts,
};
use crate::view_models::workspace::{
    release_download_updates_open_page, resolve_release_page_source, resolve_track_page_source,
    track_download_updates_open_page, track_release_download_updates_open_page,
    FrameNavigationEntry, FrameNavigationState, NavigationPathStart, ReleaseKey, ReleasePageSource,
    TrackKey, TrackPageSource, WorkspaceFrameId,
};
use crate::views::{FeedView, TrackView};

use super::{AppTab, TopApp};

#[derive(Clone)]
pub(super) enum RemoteDetailThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
}

/// The main color of one Index cover (ADR 0083 task 005).
#[derive(Clone, Copy)]
pub(super) enum RemoteCoverColorState {
    Loading,
    Loaded(Option<CoverColor>),
}

/// Loading lifecycle of the mounted Index feed detail page, reached from an
/// active search (ADR 0075 packet 047, Required Change 3). `None` on
/// `TopApp` means no fetch is tracked for the mounted page: the operator
/// has not opened a feed row from search, or has navigated away from the
/// search flow entirely (`recent_music_index_feed_detail` covers that
/// case with its own, already-fetched data).
#[derive(Clone, Debug)]
pub(super) enum IndexFeedDetailState {
    Loading {
        feed_guid: String,
    },
    Loaded {
        feed_guid: String,
        // Boxed so this variant does not enlarge the whole enum well past
        // the size of the other two (clippy::large_enum_variant).
        feed: Box<FeedView>,
    },
    Failed {
        feed_guid: String,
        message: String,
    },
}

impl IndexFeedDetailState {
    /// The feed GUID this state belongs to, regardless of its lifecycle
    /// step (packet 047: the stale-result fix keys on this value, the same
    /// pattern `publisher_page_result_is_current` follows in
    /// `src/app/publisher_dispatch.rs`).
    fn feed_guid(&self) -> &str {
        match self {
            Self::Loading { feed_guid }
            | Self::Loaded { feed_guid, .. }
            | Self::Failed { feed_guid, .. } => feed_guid,
        }
    }
}

/// `true` when a fetch result for `feed_guid` still belongs to the mounted
/// Index feed detail page (ADR 0075 packet 047), following the pattern of
/// `publisher_page_result_is_current`. A newer "open feed" click, or a
/// navigation restore to a different feed GUID, already replaces `current`
/// by the time an older fetch's result arrives. That call gives `false`,
/// so the caller applies no result (R47-07).
fn index_feed_detail_result_is_current(
    current: Option<&IndexFeedDetailState>,
    feed_guid: &str,
) -> bool {
    current.is_some_and(|state| state.feed_guid() == feed_guid)
}

/// `true` when a feed read result for `feed_guid` still belongs to the open
/// Index track page (ADR 0084 packet 023). The open track page names its
/// feed GUID in `current`. A result for another feed GUID gives `false`,
/// and the caller drops it.
fn index_track_feed_result_is_current(
    current: Option<&IndexFeedDetailState>,
    feed_guid: &str,
) -> bool {
    current.is_some_and(|state| state.feed_guid() == feed_guid)
}

/// The way that an open Index track page gets the feed of its feed GUID
/// (ADR 0084 packet 023).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IndexTrackFeedRead {
    /// The open release page already loaded this feed. No request.
    UseReleasePageFeed,
    /// The track page already tracks a read of this feed. No request.
    AlreadyTracked,
    /// Send one `FetchIndexFeedDetail`.
    Send,
}

/// Selects the way that an Index track page gets the feed of `feed_guid`.
/// It sends no request when the release page state holds the same feed
/// GUID as `Loaded`.
fn index_track_feed_read(
    release_page: Option<&IndexFeedDetailState>,
    track_feed: Option<&IndexFeedDetailState>,
    feed_guid: &str,
) -> IndexTrackFeedRead {
    if matches!(
        release_page,
        Some(IndexFeedDetailState::Loaded { feed_guid: guid, .. }) if guid == feed_guid
    ) {
        return IndexTrackFeedRead::UseReleasePageFeed;
    }
    if index_track_feed_result_is_current(track_feed, feed_guid) {
        return IndexTrackFeedRead::AlreadyTracked;
    }
    IndexTrackFeedRead::Send
}

/// Loading lifecycle of the mounted Index track detail page (ADR 0075
/// packet 047, Required Change 3). `target` is the row's activation target:
/// `<feed_guid>:<track_guid>` for a scoped hit, or a bare `<track_guid>`
/// for an unscoped one. `None` on `TopApp` means no fetch is tracked: the
/// operator has not opened a track row.
#[derive(Clone, Debug)]
pub(super) enum IndexTrackDetailState {
    Loading {
        target: String,
    },
    Loaded {
        target: String,
        // Boxed for the same reason as `IndexFeedDetailState::Loaded`.
        track: Box<TrackView>,
    },
    Failed {
        target: String,
        message: String,
    },
}

impl IndexTrackDetailState {
    /// The activation target this state belongs to, regardless of its
    /// lifecycle step.
    fn target(&self) -> &str {
        match self {
            Self::Loading { target }
            | Self::Loaded { target, .. }
            | Self::Failed { target, .. } => target,
        }
    }
}

/// `true` when a fetch result for `target` still belongs to the mounted
/// Index track detail page (ADR 0075 packet 047), following the pattern of
/// `publisher_page_result_is_current` (R47-07).
fn index_track_detail_result_is_current(
    current: Option<&IndexTrackDetailState>,
    target: &str,
) -> bool {
    current.is_some_and(|state| state.target() == target)
}

/// Splits a track row's activation target into its optional feed GUID and
/// its track GUID (ADR 0075 packet 047). A scoped hit's target holds
/// `<feed_guid>:<track_guid>`; an unscoped hit's target holds the bare
/// track GUID.
fn split_index_track_target(target: &str) -> (Option<&str>, &str) {
    target
        .split_once(':')
        .map_or((None, target), |(feed_guid, track_guid)| {
            (Some(feed_guid), track_guid)
        })
}

impl TopApp {
    pub(super) fn submit_global_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.global_search_input.read(cx).value().to_string();
        self.open_search_results_in_content_list(&query, window, cx);
    }

    pub(super) fn open_search_results_in_content_list(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let query = query.trim().to_string();
        if query.is_empty() {
            return;
        }

        if self.tab != AppTab::Music {
            self.select_tab(AppTab::Music, window, cx);
        }
        match self
            .workspace_layout
            .open_search_results_in_content_list(query.clone())
        {
            Ok(_) => {
                self.search_results_detail = Some(self.search_results_detail_for_query(&query, cx));
                self.start_index_search_for_query(&query, cx);
                cx.notify();
            }
            Err(e) => {
                self.settings_status = format!("Error opening search results: {e}");
                cx.notify();
            }
        }
    }

    fn search_results_detail_for_query(
        &self,
        query: &str,
        cx: &App,
    ) -> SearchResultsInspectorPageVm {
        // ADR 0084 Decision 7, packet 017: the artist rows read the match
        // of each name from the current Library tree.
        let artist_facts = self.library.read(cx).artist_search_facts();
        let local_tracks = {
            let conn = self.conn.lock().expect("lock db");
            self.application_services
                .query_service()
                .search_local_library_tracks(&conn, query, None)
                .unwrap_or_default()
        };

        SearchResultsInspectorPageVm::from_local_library_tracks(query, &local_tracks, &artist_facts)
    }

    pub(super) fn start_index_search_for_query(&mut self, query: &str, cx: &mut Context<Self>) {
        if let Err(reason) =
            crate::view_models::app_toolbar::AppToolbarVm::index_search_availability(
                self.feature_availability(),
            )
        {
            if let Some(detail) = self
                .search_results_detail
                .as_mut()
                .filter(|detail| detail.query() == query)
            {
                detail.set_index_error(
                    &crate::application::errors::command::CommandError::Unavailable(reason),
                    self.musicindex_endpoint
                        .require()
                        .unwrap_or("Invalid musicindex_endpoint setting"),
                    std::time::SystemTime::now(),
                );
            }
            self.retain_failed_action(
                RecoveryAction::IndexSearch {
                    query: query.into(),
                },
                reason.dependency,
                cx,
            );
            cx.notify();
            return;
        }
        if let Some(detail) = &mut self.search_results_detail {
            if detail.query() == query {
                detail.mark_index_loading();
            }
        }

        let command = FetchIndexSearchResults::new(
            self.musicindex_endpoint.clone(),
            query.to_owned(),
            self.library.read(cx).name_feed_matches(),
        );
        self.dispatch_index_search(query, command, None, cx);
    }

    pub(super) fn retry_index_search(
        &mut self,
        query: &str,
        intent: RecoveryIntent,
        cx: &mut Context<Self>,
    ) {
        if self
            .workspace_layout
            .open_search_results_in_content_list(query)
            .is_err()
        {
            self.capability_vm.pending.finish(
                intent.id,
                "App could not reopen the original search.".into(),
            );
            return;
        }
        self.search_results_detail = Some(self.search_results_detail_for_query(query, cx));
        if let Some(detail) = &mut self.search_results_detail {
            detail.mark_index_loading();
        }
        let id = intent.id;
        let command = self.retry_command(
            FetchIndexSearchResults::new(
                self.musicindex_endpoint.clone(),
                query.to_owned(),
                self.library.read(cx).name_feed_matches(),
            ),
            intent,
        );
        self.dispatch_index_search(query, command, Some(id), cx);
    }

    fn dispatch_index_search<C>(
        &self,
        query: &str,
        command: C,
        retry_id: Option<u64>,
        cx: &mut Context<Self>,
    ) where
        C: ApplicationCommand<Output = crate::view_models::search_results::IndexSearchResultRows>,
    {
        let success_query = query.to_owned();
        let error_query = query.to_owned();
        let error_endpoint = self
            .musicindex_endpoint
            .require()
            .unwrap_or("Invalid musicindex_endpoint setting")
            .to_owned();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, rows, cx| {
                if let Some(id) = retry_id {
                    this.capability_vm
                        .pending
                        .succeed(id, "App completed the original Index search.".into());
                }
                if !this.content_list_nav_matches_search(&success_query) {
                    return;
                }

                if this.search_results_detail.is_none() {
                    this.search_results_detail =
                        Some(this.search_results_detail_for_query(&success_query, cx));
                }

                // Packet 010 "The stale results": a result for another
                // query changes neither the rows nor the selection.
                if this.search_results_detail.as_mut().is_some_and(|detail| {
                    detail.replace_index_results_for_query(&success_query, rows)
                }) {
                    cx.notify();
                }
            },
            move |this, error, cx| {
                if let Some(id) = retry_id {
                    this.capability_vm.pending.finish(
                        id,
                        crate::diagnostics::redact_endpoint_details(&format!(
                            "App could not complete the original Index search: {error}"
                        )),
                    );
                } else {
                    let dependency = match error {
                        crate::application::CommandError::Unavailable(reason) => reason.dependency,
                        _ => Dependency::MusicIndex,
                    };
                    this.retain_failed_action(
                        RecoveryAction::IndexSearch {
                            query: error_query.clone(),
                        },
                        dependency,
                        cx,
                    );
                }
                if !this.content_list_nav_matches_search(&error_query) {
                    return;
                }

                if this.search_results_detail.is_none() {
                    this.search_results_detail =
                        Some(this.search_results_detail_for_query(&error_query, cx));
                }

                if let Some(detail) = this
                    .search_results_detail
                    .as_mut()
                    .filter(|detail| detail.query() == error_query)
                {
                    detail.set_index_error(&error, &error_endpoint, std::time::SystemTime::now());
                    cx.notify();
                }
            },
        );
    }

    pub(super) fn handle_search_failure_action(
        &mut self,
        action: crate::view_models::search_results::SearchFailureAction,
        cx: &mut Context<Self>,
    ) {
        if let Some(report) = self
            .search_results_detail
            .as_mut()
            .and_then(|detail| detail.activate_failure_action(action))
        {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(report));
        }
        cx.notify();
    }

    fn content_list_nav_matches_search(&self, query: &str) -> bool {
        self.content_list_frame_id()
            .and_then(|content_list_id| self.workspace_layout.frame_nav(content_list_id))
            .and_then(FrameNavigationState::active_search_query)
            .is_some_and(|current| current == query)
    }

    pub(super) fn sync_search_results_detail_with_nav(
        &mut self,
        content_list_id: WorkspaceFrameId,
        cx: &App,
    ) {
        let search_query = self
            .workspace_layout
            .frame_nav(content_list_id)
            .and_then(FrameNavigationState::active_search_query)
            .map(str::to_string);

        if let Some(query) = search_query {
            let needs_refresh = self
                .search_results_detail
                .as_ref()
                .is_none_or(|detail| detail.query() != query);
            if needs_refresh {
                self.search_results_detail = Some(self.search_results_detail_for_query(&query, cx));
            }
        } else {
            self.search_results_detail = None;
        }
    }

    /// Opens the page of a search row (packet 010 "The clicks"). The view
    /// model parses the row id into a [`SearchResultTarget`]. This function
    /// parses no id: it calls the present open function of the target, and
    /// each open function pushes its entry into the one stack.
    pub(super) fn handle_search_result_selected(
        &mut self,
        tab: SearchResultsTab,
        result_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(content_frame_id) = self.content_list_frame_id() else {
            self.settings_status = "ContentList frame not found".to_string();
            cx.notify();
            return;
        };
        let Some(target) = SearchResultTarget::from_row_id(tab, result_id) else {
            self.settings_status = format!("Unexpected search result id format: {result_id}");
            cx.notify();
            return;
        };
        let label = self
            .search_results_detail
            .as_ref()
            .and_then(|detail| detail.row_label(tab, result_id));

        match target {
            SearchResultTarget::Release(ReleaseKey::LibraryFeed(feed_id)) => {
                self.open_library_release_in_frame(feed_id, content_frame_id, cx);
            }
            SearchResultTarget::Release(key @ ReleaseKey::FeedGuid(_)) => {
                let label = label.unwrap_or_else(|| "Release".to_owned());
                self.open_release_in_frame(content_frame_id, &key, label, cx);
            }
            SearchResultTarget::Track(key) => {
                let label = label.unwrap_or_else(|| key.item_guid.clone());
                self.open_track_in_frame(content_frame_id, &key, label, cx);
            }
            SearchResultTarget::LibraryTrack(track_id) => {
                self.open_library_track_result(track_id, content_frame_id, cx);
            }
            // Flow map finding 6: a feed row opens its feed page through
            // `open_publisher_page`, which pushes `PublisherDetail`. The
            // context is `Library` when the Library tree names the feed,
            // with each role, else `Index`.
            SearchResultTarget::FeedPage(feed_guid) => {
                let context = if self.library.read(cx).search_lookup().names_feed(&feed_guid) {
                    PublisherPageContext::Library
                } else {
                    PublisherPageContext::Index
                };
                self.open_publisher_page(feed_guid, context, cx);
            }
            // ADR 0084 Decision 7, packet 017 decision D6: a name row opens
            // the name level page of its name, or the feed page of the one
            // feed that the name matches.
            target @ (SearchResultTarget::NameMatches(_) | SearchResultTarget::ArtistByName(_)) => {
                let matches = self.library.read(cx).name_feed_matches();
                match target.artist_destination(&matches) {
                    Some(ArtistNameDestination::FeedPage(feed_guid)) => {
                        self.open_publisher_page(feed_guid, PublisherPageContext::Library, cx);
                    }
                    Some(ArtistNameDestination::NameLevel(key)) => {
                        self.open_artist_name(key, NavigationPathStart::CurrentPath, cx);
                    }
                    None => {}
                }
            }
        }

        self.sync_search_results_detail_with_nav(content_frame_id, cx);
    }

    /// Opens the track page of a Library track row of the search.
    fn open_library_track_result(
        &mut self,
        track_id: i64,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        let track_row = self.conn.lock().ok().and_then(|conn| {
            library_service::track_row_by_id(&conn, track_id)
                .ok()
                .flatten()
        });
        if let Some(track_row) = track_row {
            let key = track_row.track_key();
            let label = LibraryTrackRowVm::new(&track_row, None).display_title();
            self.open_track_in_frame(content_frame_id, &key, label, cx);
        } else {
            self.settings_status = format!("Track {track_id} not found");
            cx.notify();
        }
    }

    pub(super) fn open_index_feed_detail_from_music(
        &mut self,
        feed_guid: &str,
        label: String,
        cx: &mut Context<Self>,
    ) {
        let Some(content_frame_id) = self.content_list_frame_id() else {
            self.settings_status = "ContentList frame not found".to_string();
            cx.notify();
            return;
        };

        self.open_release_in_frame(
            content_frame_id,
            &ReleaseKey::from_index_feed_guid(feed_guid),
            label,
            cx,
        );
    }

    /// Opens a release page by its key, from any origin (ADR 0084
    /// Decision 4). Replaces `open_library_album_in_frame` and
    /// `push_index_feed_detail`: one function pushes the entry, resolves
    /// its data source, and restores the page. `pub(super)`: a feed page
    /// release row also opens a release through this one function (ADR
    /// 0085 Decision 7 design, Required Change 7).
    pub(super) fn open_release_in_frame(
        &mut self,
        content_frame_id: WorkspaceFrameId,
        key: &ReleaseKey,
        label: String,
        cx: &mut Context<Self>,
    ) {
        if let ReleaseKey::FeedGuid(feed_guid) = key {
            self.release_page_labels.insert(feed_guid.clone(), label);
        }
        if let Err(error) = self
            .workspace_layout
            .push_nav(content_frame_id, FrameNavigationEntry::Release(key.clone()))
        {
            self.settings_status = format!("Failed to navigate to release: {error}");
        }
        self.sync_search_results_detail_with_nav(content_frame_id, cx);
        self.apply_release_page_source(key, cx);
        cx.notify();
    }

    /// Opens the Library release page of `feed_id` in the content frame.
    /// Replaces `open_library_album_in_frame`.
    fn open_library_release_in_frame(
        &mut self,
        feed_id: i64,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        let Some(album) = self.library.read(cx).album_for_detail_by_feed_id(feed_id) else {
            self.settings_status = format!("Feed {feed_id} not found");
            cx.notify();
            return;
        };
        let key = album
            .release_key()
            .unwrap_or(ReleaseKey::LibraryFeed(feed_id));
        let label = album.name.clone();
        self.open_release_in_frame(content_frame_id, &key, label, cx);
    }

    /// Resolves and stores the data source of an open release page, then
    /// mounts it (ADR 0084 Decision 4). The Library source asks the
    /// Library to mount the feed id. The Index source clears the Library
    /// detail and starts its own fetch when no current result exists.
    fn apply_release_page_source(&mut self, key: &ReleaseKey, cx: &mut Context<Self>) {
        let library_feed_id = match key {
            ReleaseKey::LibraryFeed(feed_id) => Some(*feed_id),
            ReleaseKey::FeedGuid(feed_guid) => self
                .conn
                .lock()
                .ok()
                .and_then(|conn| db::find_feed_id_by_guid(&conn, feed_guid).ok().flatten()),
        };
        let source = resolve_release_page_source(key.clone(), library_feed_id);
        match &source {
            ReleasePageSource::Library { feed_id } => {
                let feed_id = *feed_id;
                self.library.update(cx, |library, cx| {
                    if let Some(album) = library.album_for_detail_by_feed_id(feed_id) {
                        library.select_album(&album, cx);
                    }
                });
            }
            ReleasePageSource::Index { feed_guid } => {
                self.library
                    .update(cx, |library, _cx| library.clear_detail());
                if !index_feed_detail_result_is_current(
                    self.index_feed_detail_state.as_ref(),
                    feed_guid,
                ) {
                    self.fetch_index_feed_detail_page(feed_guid.clone(), cx);
                }
            }
        }
        self.release_page_source = Some(source);
    }

    /// Restores the stored release page source for a navigation entry that
    /// a breadcrumb or a history move (Back or Forward) just restored
    /// (ADR 0084 Decision 4). Clears the stored source for every other
    /// entry.
    pub(super) fn restore_release_page_source_for_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        let FrameNavigationEntry::Release(key) = entry else {
            self.release_page_source = None;
            return;
        };
        self.apply_release_page_source(key, cx);
    }

    /// `true` while the mounted Index release page's own fetch has not yet
    /// reached `Loaded` or `Failed` for `feed_guid` (ADR 0084 Decision 4).
    /// The body switch shows the cached content-list row instead, while a
    /// fetch in this state is in flight.
    pub(super) fn index_feed_detail_is_pending(&self, feed_guid: &str) -> bool {
        !matches!(
            self.index_feed_detail_state.as_ref(),
            Some(
                IndexFeedDetailState::Loaded { feed_guid: guid, .. }
                    | IndexFeedDetailState::Failed { feed_guid: guid, .. }
            ) if guid == feed_guid
        )
    }

    /// The title `MusicIndex` gave for a release's own fetch, when it already
    /// loaded (ADR 0084 Decision 4). `None` before the fetch loads, after
    /// it fails, or when the title text is empty.
    pub(super) fn index_feed_loaded_title(&self, feed_guid: &str) -> Option<String> {
        match self.index_feed_detail_state.as_ref()? {
            IndexFeedDetailState::Loaded {
                feed_guid: guid,
                feed,
            } if guid == feed_guid => feed.title.clone().filter(|title| !title.trim().is_empty()),
            _ => None,
        }
    }

    /// The label an opening row gave a release, by feed GUID (ADR 0084
    /// Decision 4), or "Release" when none is known.
    pub(super) fn release_opening_label(&self, feed_guid: &str) -> String {
        self.release_page_labels
            .get(feed_guid)
            .cloned()
            .unwrap_or_else(|| "Release".to_string())
    }

    /// Opens a track page by its key, from any origin (ADR 0084
    /// Decision 4). Replaces `push_index_track_detail`: one function
    /// pushes the entry, resolves its data source, and restores the page.
    pub(super) fn open_track_in_frame(
        &mut self,
        content_frame_id: WorkspaceFrameId,
        key: &TrackKey,
        label: String,
        cx: &mut Context<Self>,
    ) {
        // A `LibraryFeed`-origin key has no feed GUID, so its Index
        // activation target is a bare item GUID that other feeds' tracks
        // could share. Only a `FeedGuid` or a release-less key gives an
        // unambiguous target worth caching a label under.
        if !matches!(key.release, Some(ReleaseKey::LibraryFeed(_))) {
            self.track_page_labels.insert(key.index_target(), label);
        }
        if let Err(error) = self
            .workspace_layout
            .push_nav(content_frame_id, FrameNavigationEntry::Track(key.clone()))
        {
            self.settings_status = format!("Failed to navigate to track: {error}");
        }
        self.sync_search_results_detail_with_nav(content_frame_id, cx);
        self.apply_track_page_source(key, cx);
        cx.notify();
    }

    /// Resolves and stores the data source of an open track page, then
    /// mounts it (ADR 0084 Decision 4). The Library source asks the
    /// Library to mount the track. The Index source clears the Library
    /// detail and starts its own fetch.
    fn apply_track_page_source(&mut self, key: &TrackKey, cx: &mut Context<Self>) {
        let library_track_id = self.library_track_id_for_key(key);
        let source = resolve_track_page_source(key, library_track_id);
        match &source {
            TrackPageSource::Library { track_id } => {
                let track_id = *track_id;
                if let Some(track_row) = self.conn.lock().ok().and_then(|conn| {
                    library_service::track_row_by_id(&conn, track_id)
                        .ok()
                        .flatten()
                }) {
                    self.library
                        .update(cx, |library, cx| library.select_track(&track_row, cx));
                }
            }
            TrackPageSource::Index { target } => {
                self.library
                    .update(cx, |library, _cx| library.clear_detail());
                if !index_track_detail_result_is_current(
                    self.index_track_detail_state.as_ref(),
                    target,
                ) {
                    let (feed_guid, track_guid) = split_index_track_target(target);
                    self.fetch_index_track_detail_page(
                        target.clone(),
                        feed_guid.map(str::to_string),
                        track_guid.to_string(),
                        cx,
                    );
                }
                // ADR 0084 packet 023: the track page reads its feed for
                // the feed identity section. A track with no known feed
                // GUID yet starts the read when its own fetch loads.
                match self.index_track_page_feed_guid(target) {
                    Some(feed_guid) => self.start_index_track_feed_read(&feed_guid, cx),
                    None => self.index_track_feed_state = None,
                }
            }
        }
        self.track_page_source = Some(source);
    }

    /// The Library track id of a track key, through the two read queries
    /// of Required Change 3 (ADR 0084 Decision 4). `None` when the key has
    /// no release, or the Library holds no matching track.
    fn library_track_id_for_key(&self, key: &TrackKey) -> Option<i64> {
        let conn = self.conn.lock().ok()?;
        match &key.release {
            Some(ReleaseKey::FeedGuid(feed_guid)) => {
                db::find_track_id_by_feed_guid_and_item_guid(&conn, feed_guid, &key.item_guid)
                    .ok()
                    .flatten()
            }
            Some(ReleaseKey::LibraryFeed(feed_id)) => {
                db::find_track_id_by_feed_id_and_item_guid(&conn, *feed_id, &key.item_guid)
                    .ok()
                    .flatten()
            }
            None => None,
        }
    }

    /// Restores the stored track page source for a navigation entry that a
    /// breadcrumb or a history move (Back or Forward) just restored (ADR
    /// 0084 Decision 4). Clears the stored source for every other entry.
    pub(super) fn restore_track_page_source_for_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        let FrameNavigationEntry::Track(key) = entry else {
            self.track_page_source = None;
            return;
        };
        self.apply_track_page_source(key, cx);
    }

    /// The title `MusicIndex` gave for a track's own fetch, when it already
    /// loaded (ADR 0084 Decision 4). `None` before the fetch loads, after
    /// it fails, or when the title text is empty.
    pub(super) fn index_track_loaded_title(&self, target: &str) -> Option<String> {
        match self.index_track_detail_state.as_ref()? {
            IndexTrackDetailState::Loaded {
                target: loaded_target,
                track,
            } if loaded_target == target => {
                track.title.clone().filter(|title| !title.trim().is_empty())
            }
            _ => None,
        }
    }

    /// The label an opening row gave a track, by its Index activation
    /// target (ADR 0084 Decision 4), or "Track" when none is known.
    pub(super) fn track_opening_label(&self, target: &str) -> String {
        self.track_page_labels
            .get(target)
            .cloned()
            .unwrap_or_else(|| "Track".to_string())
    }

    /// Starts the Index feed detail-on-open fetch, and records the loading
    /// state right away (ADR 0075 packet 047, Required Change 3).
    fn fetch_index_feed_detail_page(&mut self, feed_guid: String, cx: &mut Context<Self>) {
        self.index_feed_detail_state = Some(IndexFeedDetailState::Loading {
            feed_guid: feed_guid.clone(),
        });
        cx.notify();

        let command =
            FetchIndexFeedDetail::new(self.musicindex_endpoint.clone(), feed_guid.clone());
        let success_guid = feed_guid.clone();
        let failure_guid = feed_guid;
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, feed, cx| {
                // R47-07: a newer "open feed" click, or a navigation
                // restore to a different feed GUID, already replaced the
                // mounted page. This older fetch's result must not
                // overwrite it.
                if !index_feed_detail_result_is_current(
                    this.index_feed_detail_state.as_ref(),
                    &success_guid,
                ) {
                    return;
                }
                this.index_feed_detail_state = Some(IndexFeedDetailState::Loaded {
                    feed_guid: success_guid,
                    feed: Box::new(feed),
                });
                cx.notify();
            },
            move |this, error, cx| {
                if !index_feed_detail_result_is_current(
                    this.index_feed_detail_state.as_ref(),
                    &failure_guid,
                ) {
                    return;
                }
                this.index_feed_detail_state = Some(IndexFeedDetailState::Failed {
                    feed_guid: failure_guid,
                    message: format!("{error}"),
                });
                cx.notify();
            },
        );
    }

    /// The feed GUID of the open Index track page with the activation
    /// target `target`: the feed GUID of the target or of the loaded track.
    /// `None` before the track names one.
    fn index_track_page_feed_guid(&self, target: &str) -> Option<String> {
        let (feed_guid, _) = split_index_track_target(target);
        if let Some(feed_guid) = feed_guid {
            return Some(feed_guid.to_owned());
        }
        match self.index_track_detail_state.as_ref()? {
            IndexTrackDetailState::Loaded {
                target: loaded_target,
                track,
            } if loaded_target == target => track.feed_guid.clone(),
            _ => None,
        }
    }

    /// The loaded feed of `feed_guid` for an Index track page (ADR 0084
    /// packet 023): the feed of the release page state, else the feed of
    /// the track page read. `None` while the read runs or after it fails.
    fn index_track_page_feed(&self, feed_guid: &str) -> Option<&FeedView> {
        [
            self.index_feed_detail_state.as_ref(),
            self.index_track_feed_state.as_ref(),
        ]
        .into_iter()
        .flatten()
        .find_map(|state| match state {
            IndexFeedDetailState::Loaded {
                feed_guid: guid,
                feed,
            } if guid == feed_guid => Some(feed.as_ref()),
            _ => None,
        })
    }

    /// Starts the feed read of the open Index track page (ADR 0084 packet
    /// 023). A failed read gives no feed identity section, and the page
    /// stays usable. The status text names the feed GUID and the
    /// `MusicIndex` error.
    fn start_index_track_feed_read(&mut self, feed_guid: &str, cx: &mut Context<Self>) {
        match index_track_feed_read(
            self.index_feed_detail_state.as_ref(),
            self.index_track_feed_state.as_ref(),
            feed_guid,
        ) {
            IndexTrackFeedRead::UseReleasePageFeed | IndexTrackFeedRead::AlreadyTracked => return,
            IndexTrackFeedRead::Send => {}
        }
        self.index_track_feed_state = Some(IndexFeedDetailState::Loading {
            feed_guid: feed_guid.to_owned(),
        });
        let command =
            FetchIndexFeedDetail::new(self.musicindex_endpoint.clone(), feed_guid.to_owned());
        let success_guid = feed_guid.to_owned();
        let failure_guid = feed_guid.to_owned();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, feed, cx| {
                if !index_track_feed_result_is_current(
                    this.index_track_feed_state.as_ref(),
                    &success_guid,
                ) {
                    return;
                }
                this.index_track_feed_state = Some(IndexFeedDetailState::Loaded {
                    feed_guid: success_guid,
                    feed: Box::new(feed),
                });
                cx.notify();
            },
            move |this, error, cx| {
                if !index_track_feed_result_is_current(
                    this.index_track_feed_state.as_ref(),
                    &failure_guid,
                ) {
                    return;
                }
                this.settings_status = format!(
                    "App could not read the feed {failure_guid} of this track from MusicIndex: {error:#}"
                );
                this.index_track_feed_state = Some(IndexFeedDetailState::Failed {
                    feed_guid: failure_guid,
                    message: format!("{error}"),
                });
                cx.notify();
            },
        );
    }

    /// Starts the Index track detail-on-open fetch, and records the
    /// loading state right away (ADR 0075 packet 047, Required Change 3).
    fn fetch_index_track_detail_page(
        &mut self,
        target: String,
        feed_guid: Option<String>,
        track_guid: String,
        cx: &mut Context<Self>,
    ) {
        self.index_track_detail_state = Some(IndexTrackDetailState::Loading {
            target: target.clone(),
        });
        cx.notify();

        let command =
            FetchIndexTrackDetail::new(self.musicindex_endpoint.clone(), track_guid, feed_guid);
        let success_target = target.clone();
        let failure_target = target;
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, track, cx| {
                // R47-07: the same stale-result guard as the feed page.
                if !index_track_detail_result_is_current(
                    this.index_track_detail_state.as_ref(),
                    &success_target,
                ) {
                    return;
                }
                let feed_guid = track.feed_guid.clone();
                this.index_track_detail_state = Some(IndexTrackDetailState::Loaded {
                    target: success_target,
                    track: Box::new(track),
                });
                if let Some(feed_guid) = feed_guid {
                    this.start_index_track_feed_read(&feed_guid, cx);
                }
                cx.notify();
            },
            move |this, error, cx| {
                if !index_track_detail_result_is_current(
                    this.index_track_detail_state.as_ref(),
                    &failure_target,
                ) {
                    return;
                }
                this.index_track_detail_state = Some(IndexTrackDetailState::Failed {
                    target: failure_target,
                    message: format!("{error}"),
                });
                cx.notify();
            },
        );
    }

    /// Selects the Index feed detail display state to render: the
    /// operator's own fetch when it matches the mounted feed GUID, or a
    /// loading placeholder otherwise (ADR 0075 packet 047). This method
    /// only selects which state applies; the view model decides its text.
    pub(super) fn index_feed_detail_display(&self, id: &str, label: &str) -> IndexDetailDisplay {
        match self.index_feed_detail_state.as_ref() {
            Some(IndexFeedDetailState::Loaded { feed_guid, feed }) if feed_guid == id => {
                IndexDetailDisplay::loaded_feed((**feed).clone(), id)
            }
            Some(IndexFeedDetailState::Failed { feed_guid, message }) if feed_guid == id => {
                IndexDetailDisplay::failed(IndexDetailKind::Release, id, label, message.clone())
            }
            _ => IndexDetailDisplay::loading(IndexDetailKind::Release, id, label),
        }
    }

    /// Selects the Index track detail display state to render (ADR 0075
    /// packet 047): the operator's own fetch, once it matches the mounted
    /// activation target, or a loading placeholder before it does.
    pub(super) fn index_track_detail_display(&self, id: &str, label: &str) -> IndexDetailDisplay {
        match self.index_track_detail_state.as_ref() {
            Some(IndexTrackDetailState::Loaded { target, track }) if target == id => {
                IndexDetailDisplay::loaded_track((**track).clone(), id)
            }
            Some(IndexTrackDetailState::Failed { target, message }) if target == id => {
                IndexDetailDisplay::failed(IndexDetailKind::Track, id, label, message.clone())
            }
            _ => IndexDetailDisplay::loading(IndexDetailKind::Track, id, label),
        }
    }

    /// Opens the Library release page of `feed_id` in the content frame.
    /// The album name link of a track page uses it (ADR 0083 Decision 5).
    pub(super) fn open_library_album_page(&mut self, feed_id: i64, cx: &mut Context<Self>) {
        let Some(content_frame_id) = self.content_list_frame_id() else {
            self.settings_status = "ContentList frame not found".to_string();
            cx.notify();
            return;
        };
        self.open_library_release_in_frame(feed_id, content_frame_id, cx);
    }

    pub(super) fn render_index_feed_or_fallback_detail(
        &mut self,
        detail: &crate::view_models::search_results::IndexDetailDisplay,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(feed) = detail.feed.as_ref() {
            return self.render_index_release_page(feed, cx);
        }
        if let Some(track) = detail.track.as_ref() {
            return self.render_index_track_page(track, cx);
        }
        render_index_detail_display(detail, cx)
    }

    /// The Index release page (ADR 0084 Decision 4, packet 021). It builds
    /// through the one release page view model and shell. Each command goes
    /// to [`Self::run_index_release_page_command`].
    fn render_index_release_page(&mut self, feed: &FeedView, cx: &mut Context<Self>) -> AnyElement {
        let feed_guid = feed_guid_from_view(feed);
        let description_state =
            display_description_text(feed.description.as_deref()).map(|description| {
                self.index_release_description_state(
                    feed_guid.as_deref().unwrap_or("unknown"),
                    description,
                )
            });
        // ADR 0084 packet 018: each row carries the detail selection facts.
        // The Library row and the Index row of one track give the same
        // facts, from the same row key.
        let detail_rows =
            DetailPageRows::Tracks(index_release_track_rows(feed_guid.as_deref(), &feed.tracks));
        let (facts, row_probes, scroll_handle, detail_focus) = {
            let library = self.library.read(cx);
            let selections = library.detail_selection().row_selections(&detail_rows);
            let row_probes = selections
                .iter()
                .map(|selection| library.detail_row_probe(selection))
                .collect::<Vec<_>>();
            // ADR 0084 Decision 6: a release whose key is in a Library
            // singles group links to that group. An Index release that is
            // not in the Library gets no link.
            // ADR 0083 task 006, decision D8: each Index row reads the
            // phase of its own job, then the phase of a release job.
            let facts = ReleasePageFacts::from_index_feed(feed.clone(), description_state)
                .map_name_links(|links, key| library.release_page_name_links(links, key))
                .with_row_selections(selections)
                .with_playlists(library.playlists())
                .with_index_track_work(library.busy_items());
            (
                facts,
                row_probes,
                library.detail_scroll_handle(),
                library.detail_column_focus().clone(),
            )
        };
        let vm = release_page_vm(&facts);
        let row_thumbnails = vm
            .rows
            .iter()
            .map(|row| {
                row.thumbnail_url
                    .as_deref()
                    .and_then(|url| self.index_remote_detail_hero_image(url, cx))
            })
            .collect();
        let images = ReleasePageImages {
            hero_image: self.index_feed_hero_image(feed, cx),
            cover_color: index_feed_artwork_url(feed)
                .map(str::to_owned)
                .and_then(|url| self.index_cover_color(&url, cx)),
            row_thumbnails,
            row_probes,
            contributor_thumbnails: None,
            scroll_handle: Some(scroll_handle),
            row_menu_focus: Some(detail_focus),
        };
        let entity = cx.entity();
        let dispatch: ReleasePageDispatch = Rc::new(move |command, _window, cx| {
            entity.update(cx, |this, cx| {
                this.run_index_release_page_command(command, cx);
            });
        });
        render_release_page(&vm, images, &dispatch, cx)
    }

    /// Runs one command of the open Index release page. The command reads
    /// the open feed again, so it never acts on an old `FeedView`. The
    /// Index downloads and the Index playlist commands stay here.
    fn run_index_release_page_command(
        &mut self,
        command: ReleasePageCommand,
        cx: &mut Context<Self>,
    ) {
        let Some(feed) = self.index_release_page_feed(cx) else {
            return;
        };
        let row_vm = |row: usize| {
            feed.tracks.get(row).map(|track| {
                ReleaseTrackRowVm::index(
                    &feed,
                    row,
                    track,
                    RowSelectionVm::none(),
                    TrackRowWork::Idle,
                )
            })
        };
        match command {
            ReleasePageCommand::DownloadRelease => self.download_index_feed(&feed, cx),
            ReleasePageCommand::CopyFeedUrl => {
                if let Some(url) = feed.feed_url.clone() {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(url));
                }
            }
            ReleasePageCommand::ToggleDescription => {
                let Some(description) = display_description_text(feed.description.as_deref())
                else {
                    return;
                };
                let description = description.to_string();
                let feed_guid = feed_guid_from_view(&feed).unwrap_or_else(|| "unknown".to_string());
                self.toggle_index_release_description(&feed_guid, &description, cx);
            }
            ReleasePageCommand::OpenNameLink(target) => {
                self.open_index_album_name_link(&target, cx);
            }
            ReleasePageCommand::DownloadTrack(row) => {
                if let Some(track) = feed.tracks.get(row) {
                    self.download_index_track(&feed, track, cx);
                }
            }
            ReleasePageCommand::AddTrackToPlaylist(row, playlist_id) => {
                if let Some(track) = feed.tracks.get(row) {
                    self.add_index_track_to_playlist(&feed, track, playlist_id, cx);
                }
            }
            ReleasePageCommand::CreatePlaylistForTrack(row, name) => {
                if let Some(track) = feed.tracks.get(row).cloned() {
                    self.create_playlist_and_add_index_track(&name, feed.clone(), track, cx);
                }
            }
            // ADR 0084 Decision 4: a click on an Index track row opens its
            // track page, the same as a Library row. Packet 018: in
            // selection mode, a click changes the selection instead.
            ReleasePageCommand::ClickRow(row, modifiers) => {
                if let Some(ReleaseTrackRowVm {
                    row_key: Some(key),
                    open_label: Some(label),
                    ..
                }) = row_vm(row)
                {
                    self.click_detail_row(&key, label, click_modifiers(modifiers), cx);
                }
            }
            ReleasePageCommand::ToggleRow(row) => {
                if let Some(key) = row_vm(row).and_then(|row| row.row_key) {
                    self.toggle_detail_row(&key, cx);
                }
            }
            // The Index release page offers no such action. An Index row
            // has no "Remove…" and no "Inspect sources" (ADR 0083 task
            // 006, decision D4).
            ReleasePageCommand::RemoveTrack(_)
            | ReleasePageCommand::InspectSources(_)
            | ReleasePageCommand::MusicBrainzLookup
            | ReleasePageCommand::RemoveRelease
            | ReleasePageCommand::AddReleaseToPlaylist(_)
            | ReleasePageCommand::CreatePlaylistForRelease(_) => {}
        }
    }

    fn index_feed_hero_image(
        &mut self,
        feed: &FeedView,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Image>> {
        let url = index_feed_artwork_url(feed)?;
        self.index_remote_detail_hero_image(url, cx)
    }

    /// The Index track page (ADR 0084 Decision 4, packet 023). It builds
    /// through the one track page view model and shell. Each command goes
    /// to [`Self::run_index_track_page_command`].
    fn render_index_track_page(&mut self, track: &TrackView, cx: &mut Context<Self>) -> AnyElement {
        let feed = track
            .feed_guid
            .as_deref()
            .and_then(|feed_guid| self.index_track_page_feed(feed_guid));
        let mut facts = TrackPageFacts::from_index_track(track.clone(), feed);
        if let Some(description) = display_description_text(track.description.as_deref()) {
            let key = index_track_description_key(track);
            facts =
                facts.with_description_state(self.index_track_description_state(&key, description));
        }
        let vm = track_page_vm(&facts);
        let images = TrackPageImages {
            hero_image: self.index_track_hero_image(track, cx),
            scroll_handle: self.library.read(cx).detail_scroll_handle(),
        };
        let entity = cx.entity();
        let dispatch: TrackPageDispatch = Rc::new(move |command, _window, cx| {
            entity.update(cx, |this, cx| {
                this.run_index_track_page_command(command, cx);
            });
        });
        render_track_page(&vm, images, &dispatch, None, cx)
    }

    /// Runs one command of the open Index track page. The command reads
    /// the open track again, so it never acts on an old `TrackView`.
    fn run_index_track_page_command(&mut self, command: TrackPageCommand, cx: &mut Context<Self>) {
        let Some(track) = self.index_track_page_track() else {
            return;
        };
        let feed_guid = track
            .feed_guid
            .clone()
            .filter(|feed_guid| !feed_guid.trim().is_empty());
        match command {
            TrackPageCommand::Action(TrackPageAction::DownloadAlbum) => {
                if let Some(feed_guid) = feed_guid {
                    self.download_index_album_of_track(feed_guid, track.feed_title.clone(), cx);
                }
            }
            TrackPageCommand::Action(TrackPageAction::CopyFeedUrl) => {
                if let Some(feed_guid) = feed_guid {
                    self.copy_index_feed_url_of_track(feed_guid, cx);
                }
            }
            TrackPageCommand::ToggleDescription => {
                if let Some(description) = display_description_text(track.description.as_deref()) {
                    let key = index_track_description_key(&track);
                    self.toggle_index_track_description(&key, description, cx);
                }
            }
            TrackPageCommand::OpenNameLink(target) => self.open_index_track_name_link(&target, cx),
            // An Index track page offers no such action.
            TrackPageCommand::Action(
                TrackPageAction::DownloadTrack
                | TrackPageAction::AddToPlaylist
                | TrackPageAction::MusicBrainzLookup
                | TrackPageAction::RemoveTrack,
            )
            | TrackPageCommand::AddToPlaylist(_)
            | TrackPageCommand::CreatePlaylist(_) => {}
        }
    }

    /// The track of the open Index track page, when it shows one.
    fn index_track_page_track(&self) -> Option<TrackView> {
        let Some(TrackPageSource::Index { target }) = &self.track_page_source else {
            return None;
        };
        let label = self.track_opening_label(target);
        self.index_track_detail_display(target, &label).track
    }

    /// "Copy feed URL" of an Index track: reads the album feed from
    /// `MusicIndex`, which indexes the feed URL, and copies that URL.
    fn copy_index_feed_url_of_track(&mut self, feed_guid: String, cx: &mut Context<Self>) {
        let command = FetchIndexFeedDetail::new(self.musicindex_endpoint.clone(), feed_guid);
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, feed, cx| match feed.feed_url {
                Some(url) => {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(url));
                    this.settings_status = "App copied the feed URL of this track.".to_string();
                }
                None => {
                    this.settings_status =
                        "MusicIndex gave no feed URL for the album of this track.".to_string();
                }
            },
            |this, error, _cx| {
                this.settings_status = format!(
                    "App could not read the album of this track from MusicIndex: {error:#}"
                );
            },
        );
    }

    /// Opens the page of a name link under an Index track title.
    fn open_index_track_name_link(&mut self, target: &TrackNameLinkTarget, cx: &mut Context<Self>) {
        match target {
            TrackNameLinkTarget::IndexAlbum { feed_guid } => {
                self.open_index_feed_detail_from_music(feed_guid, feed_guid.clone(), cx);
            }
            TrackNameLinkTarget::Publisher(publisher_feed_guid) => {
                self.open_publisher_page(
                    publisher_feed_guid.clone(),
                    PublisherPageContext::Index,
                    cx,
                );
            }
            TrackNameLinkTarget::LibraryAlbum(feed_id) => {
                self.open_library_album_page(*feed_id, cx);
            }
        }
    }

    /// "Download album" of an Index track: queues an Index release job
    /// with no feed (ADR 0084 Decision 10). The job reads the album feed
    /// from `MusicIndex` in its own slot, then downloads it. `label` names
    /// the release until the read gives its title.
    pub(super) fn download_index_album_of_track(
        &mut self,
        feed_guid: String,
        label: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let job = index_release_job_by_guid(feed_guid, label);
        self.library
            .update(cx, |library, cx| library.queue_downloads(vec![job], cx));
    }

    fn index_track_hero_image(
        &mut self,
        track: &TrackView,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Image>> {
        let url = index_track_artwork_url(track)?;
        self.index_remote_detail_hero_image(url, cx)
    }

    pub(super) fn index_remote_detail_hero_image(
        &mut self,
        url: &str,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Image>> {
        if let Some(image) = self.image_cache.peek_static(url) {
            return Some(image);
        }
        // Do not leave a thumbnail marked Loading when dispatch cannot start.
        self.command_runner.availability().ok()?;
        if let Some(state) = self.remote_detail_thumbnails.get(url) {
            return match state {
                RemoteDetailThumbnailState::Loading => None,
                RemoteDetailThumbnailState::Loaded(image) => image.clone(),
            };
        }

        let url = url.to_string();
        self.remote_detail_thumbnails
            .insert(url.clone(), RemoteDetailThumbnailState::Loading);
        let command = FetchThumbnail::new(Arc::clone(&self.image_cache), url.clone(), false);
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, image, cx| {
                this.remote_detail_thumbnails
                    .insert(url, RemoteDetailThumbnailState::Loaded(image));
                cx.notify();
            },
            |_, _, _| {},
        );

        None
    }

    /// Opens the page of a name link under an Index album title. An artist
    /// name opens the feed page of the one Library feed that the name
    /// matches, else the name level page, in the current path (ADR 0084
    /// Decision 7).
    fn open_index_album_name_link(&mut self, target: &AlbumNameLinkTarget, cx: &mut Context<Self>) {
        match target {
            AlbumNameLinkTarget::Artist(name) => {
                self.open_artist_name(
                    ArtistNameKey::from_text(name),
                    NavigationPathStart::CurrentPath,
                    cx,
                );
            }
            AlbumNameLinkTarget::Publisher(publisher_feed_guid) => {
                self.open_publisher_page(
                    publisher_feed_guid.clone(),
                    PublisherPageContext::Index,
                    cx,
                );
            }
            // ADR 0084 Decision 6: the group page goes into the current
            // path.
            AlbumNameLinkTarget::SinglesGroup(key) => {
                self.open_music_page(
                    &FrameNavigationEntry::SinglesGroup(key.clone()),
                    NavigationPathStart::CurrentPath,
                    cx,
                );
            }
        }
    }

    /// The main color of an Index cover for the album page backdrop (ADR
    /// 0083 task 005). The first call starts a background fetch.
    fn index_cover_color(&mut self, url: &str, cx: &mut Context<Self>) -> Option<CoverColor> {
        if let Some(color) = self.image_cache.peek_cover_color(url) {
            return Some(color);
        }
        self.command_runner.availability().ok()?;
        match self.remote_cover_colors.get(url) {
            Some(RemoteCoverColorState::Loaded(color)) => return *color,
            Some(RemoteCoverColorState::Loading) => return None,
            None => {}
        }
        let url = url.to_string();
        self.remote_cover_colors
            .insert(url.clone(), RemoteCoverColorState::Loading);
        let command = FetchCoverColor::new(Arc::clone(&self.image_cache), url.clone());
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, color, cx| {
                this.remote_cover_colors
                    .insert(url, RemoteCoverColorState::Loaded(color));
                cx.notify();
            },
            |_, _, _| {},
        );
        None
    }

    /// "Download" of an Index release: queues an Index release job (ADR
    /// 0084 Decision 10). `finish_index_release_download` runs when it
    /// completes.
    fn download_index_feed(&mut self, feed: &FeedView, cx: &mut Context<Self>) {
        let job = index_release_job(feed);
        self.library
            .update(cx, |library, cx| library.queue_downloads(vec![job], cx));
    }

    /// The completion of an Index download job of the download queue (ADR
    /// 0084 Decision 10). A failure is in the progress item, so only a
    /// success changes the open pages.
    pub(super) fn finish_index_download(
        &mut self,
        job: &DownloadJob,
        outcome: &DownloadOutcome,
        cx: &mut Context<Self>,
    ) {
        if *outcome != DownloadOutcome::Succeeded {
            return;
        }
        match job {
            DownloadJob::IndexRelease {
                feed_guid,
                feed_url,
                ..
            } => self.finish_index_release_download(feed_guid.as_deref(), feed_url.as_deref(), cx),
            DownloadJob::IndexTrack {
                feed_guid,
                track_guid,
                ..
            } => self.finish_index_track_download(feed_guid.as_deref(), track_guid, cx),
            DownloadJob::LibraryFeed { .. } | DownloadJob::LibraryTrack { .. } => {}
        }
    }

    /// The success of an Index release download: the open release page
    /// and the open track page of the release change to the Library in
    /// place (ADR 0084 Decision 4).
    fn finish_index_release_download(
        &mut self,
        feed_guid: Option<&str>,
        feed_url: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.reload_cached(cx);
        self.scan_tags_after_search_download(cx);
        if let Some(feed_id) = self.downloaded_index_feed_id(feed_guid, feed_url) {
            self.show_downloaded_index_feed(feed_id, feed_guid, cx);
        } else {
            self.library.update(cx, LibraryApp::refresh);
        }
    }

    fn downloaded_index_feed_id(
        &self,
        feed_guid: Option<&str>,
        feed_url: Option<&str>,
    ) -> Option<i64> {
        let conn = self.conn.lock().ok()?;
        if let Some(feed_guid) = feed_guid {
            if let Ok(Some(feed_id)) = db::find_feed_id_by_guid(&conn, feed_guid) {
                return Some(feed_id);
            }
        }
        feed_url.and_then(|feed_url| db::feed_id_by_url(&conn, feed_url).ok().flatten())
    }

    /// Changes the stored source of the open release page to the Library
    /// and mounts the Library album, in place (ADR 0084 Decision 4). It
    /// changes no navigation entry: the release keeps the one entry it
    /// opened with, and Back goes to the page before it. When the operator
    /// opened another page during the download, no page changes.
    fn show_downloaded_index_feed(
        &mut self,
        feed_id: i64,
        feed_guid: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let current = self
            .content_list_frame_id()
            .and_then(|id| self.workspace_layout.frame_nav(id))
            .map(|nav| nav.current().clone());
        let updates_open_page = release_download_updates_open_page(current.as_ref(), feed_guid);
        let updates_open_track_page =
            track_release_download_updates_open_page(current.as_ref(), feed_guid);
        self.library.update(cx, |library, cx| {
            if updates_open_page {
                if let Some(album) = library.album_for_detail_by_feed_id(feed_id) {
                    library.select_album(&album, cx);
                }
            }
            library.refresh(cx);
        });
        if updates_open_page {
            self.release_page_source = Some(ReleasePageSource::Library { feed_id });
        }
        // ADR 0084 Decision 4: "Download album" on a track page downloads
        // the whole release. The open track page of that release resolves
        // its source again, the same shared completion the release page
        // uses.
        if updates_open_track_page {
            if let Some(FrameNavigationEntry::Track(key)) = current {
                self.apply_track_page_source(&key, cx);
            }
        }

        if let Some(content_frame_id) = self.content_list_frame_id() {
            self.sync_search_results_detail_with_nav(content_frame_id, cx);
        }
        cx.notify();
    }

    /// "Download" of an Index track: queues an Index track job (ADR 0084
    /// Decision 10). `finish_index_track_download` runs when it completes.
    pub(super) fn download_index_track(
        &mut self,
        feed: &FeedView,
        track: &TrackView,
        cx: &mut Context<Self>,
    ) {
        let job = index_track_job(feed, track);
        self.library
            .update(cx, |library, cx| library.queue_downloads(vec![job], cx));
    }

    /// The success of an Index track download: the open track page of
    /// this track and the open release page of its feed change to the
    /// Library in place (ADR 0084 Decision 4).
    fn finish_index_track_download(
        &mut self,
        downloaded_feed_guid: Option<&str>,
        track_guid: &str,
        cx: &mut Context<Self>,
    ) {
        let downloaded_key =
            TrackKey::from_index_track_target(downloaded_feed_guid, track_guid.to_owned());
        self.reload_cached(cx);
        self.library.update(cx, LibraryApp::refresh);
        self.scan_tags_after_search_download(cx);
        // ADR 0084 Decision 4: the open track page of this exact track
        // resolves its source again, in place.
        let current = self
            .content_list_frame_id()
            .and_then(|id| self.workspace_layout.frame_nav(id))
            .map(|nav| nav.current().clone());
        if track_download_updates_open_page(current.as_ref(), Some(&downloaded_key)) {
            self.apply_track_page_source(&downloaded_key, cx);
        }
        // ADR 0084 Decision 4: the open release page of this track's feed
        // also resolves its source again, in place.
        if release_download_updates_open_page(current.as_ref(), downloaded_feed_guid) {
            if let Some(FrameNavigationEntry::Release(key)) = current {
                self.apply_release_page_source(&key, cx);
            }
        }
    }

    fn add_index_track_to_playlist(
        &mut self,
        feed: &FeedView,
        track: &TrackView,
        playlist_id: i64,
        cx: &mut Context<Self>,
    ) {
        let Some(feed_guid) = feed_guid_from_view(feed) else {
            self.settings_status = "Cannot add track without a MusicIndex feed id".to_string();
            cx.notify();
            return;
        };
        if let Err(error) = feed_service::ensure_feed_in_db(
            &self.conn,
            &feed_guid,
            feed.feed_url.as_deref(),
            &self.musicindex_endpoint,
        ) {
            self.settings_status = format!("Error preparing feed: {error:#}");
            cx.notify();
            return;
        }

        let api_track = api_track_from_view(feed, track);
        let track_id = {
            let conn = self.conn.lock().expect("lock db");
            library_service::find_track_id(
                &conn,
                feed.feed_url.as_deref(),
                api_track.track_guid.as_deref(),
                api_track.enclosure_url.as_deref(),
            )
            .ok()
            .flatten()
        };
        let Some(track_id) = track_id else {
            self.settings_status = "Cannot add track until the feed is indexed locally".to_string();
            cx.notify();
            return;
        };
        self.subscribe_then_append_to_playlist(playlist_id, vec![track_id], cx);
    }

    fn create_playlist_and_add_index_track(
        &mut self,
        name: &str,
        feed: FeedView,
        track: TrackView,
        cx: &mut Context<Self>,
    ) {
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.add_index_track_to_playlist(&feed, &track, result.playlist_id(), cx);
            },
            |this, error, _cx| {
                this.settings_status = format!("Error creating playlist: {error:#}");
            },
        );
    }

    fn subscribe_then_append_to_playlist(
        &mut self,
        playlist_id: i64,
        track_ids: Vec<i64>,
        cx: &mut Context<Self>,
    ) {
        if track_ids.is_empty() {
            self.settings_status = "Feed has no tracks to add".to_string();
            cx.notify();
            return;
        }
        let command = SubscribeThenAppendToPlaylist::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            playlist_id,
            track_ids,
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, result, cx| {
                this.settings_status = format!(
                    "Added {} track{}, downloaded {}",
                    result.appended(),
                    if result.appended() == 1 { "" } else { "s" },
                    result.downloaded()
                );
                this.reload_cached(cx);
                this.library.update(cx, LibraryApp::refresh);
                this.scan_tags_after_search_download(cx);
            },
            |this, error, _cx| {
                this.settings_status = format!("Error adding to playlist: {error:#}");
            },
        );
    }

    /// ADR 0076 packet 005: a completed download from the search results
    /// requests one tag update scan. The scan runs in the Library runtime
    /// actor, and the new file gets its difference count.
    fn scan_tags_after_search_download(&mut self, cx: &mut Context<Self>) {
        self.library
            .update(cx, |library, _cx| library.scan_tag_updates());
    }

    /// The disclosure state of an Index release's description, by feed
    /// GUID (ADR 0084 Decision 4, ADR 0047 item 8 amended 2026-10-16). The
    /// state lives for the session only; it is not stored.
    fn index_release_description_state(
        &self,
        feed_guid: &str,
        description: &str,
    ) -> DescriptionState {
        keyed_description_state(
            &self.index_release_description_states,
            feed_guid,
            description,
        )
    }

    fn toggle_index_release_description(
        &mut self,
        feed_guid: &str,
        description: &str,
        cx: &mut Context<Self>,
    ) {
        toggle_keyed_description_state(
            &mut self.index_release_description_states,
            feed_guid,
            description,
        );
        cx.notify();
    }

    /// The disclosure state of an Index track's description, by its Index
    /// activation target (ADR 0084 Decision 4, ADR 0047 item 8 amended
    /// 2026-10-16). The state lives for the session only; it is not
    /// stored.
    fn index_track_description_state(&self, target: &str, description: &str) -> DescriptionState {
        keyed_description_state(&self.index_track_description_states, target, description)
    }

    fn toggle_index_track_description(
        &mut self,
        target: &str,
        description: &str,
        cx: &mut Context<Self>,
    ) {
        toggle_keyed_description_state(
            &mut self.index_track_description_states,
            target,
            description,
        );
        cx.notify();
    }
}

/// The image URL to show for an Index track (ADR 0075 Decision C,
/// packet 048). `TrackView::display_artwork_url` makes this choice. A
/// caller resolves the returned URL to an image.
fn index_track_artwork_url(track: &TrackView) -> Option<&str> {
    track.display_artwork_url()
}

pub(super) fn feed_guid_from_view(feed: &FeedView) -> Option<String> {
    index_feed_guid(feed)
}

#[cfg(test)]
mod index_detail_dispatch_tests {
    use super::*;

    /// R23-05 (ADR 0084 packet 023): a feed read result for a feed GUID
    /// that the open track page does not name is dropped.
    #[test]
    fn adr_0084_track_page_r23_05_feed_result_for_another_feed_is_dropped() {
        let current = Some(IndexFeedDetailState::Loading {
            feed_guid: "feed-b".into(),
        });
        assert!(!index_track_feed_result_is_current(
            current.as_ref(),
            "feed-a"
        ));
        assert!(index_track_feed_result_is_current(
            current.as_ref(),
            "feed-b"
        ));
        assert!(!index_track_feed_result_is_current(None, "feed-a"));
    }

    /// R23-05: the track page sends no feed request when the release page
    /// state holds the same feed GUID as `Loaded`. It sends one request
    /// when the release page holds another feed, or the same feed still
    /// loading.
    #[test]
    fn adr_0084_track_page_r23_05_loaded_release_feed_sends_no_request() {
        let loaded = Some(IndexFeedDetailState::Loaded {
            feed_guid: "feed-a".into(),
            feed: Box::default(),
        });
        assert_eq!(
            index_track_feed_read(loaded.as_ref(), None, "feed-a"),
            IndexTrackFeedRead::UseReleasePageFeed
        );
        assert_eq!(
            index_track_feed_read(loaded.as_ref(), None, "feed-b"),
            IndexTrackFeedRead::Send
        );
        let loading = Some(IndexFeedDetailState::Loading {
            feed_guid: "feed-a".into(),
        });
        assert_eq!(
            index_track_feed_read(loading.as_ref(), None, "feed-a"),
            IndexTrackFeedRead::Send
        );
        assert_eq!(
            index_track_feed_read(None, loading.as_ref(), "feed-a"),
            IndexTrackFeedRead::AlreadyTracked
        );
        assert_eq!(
            index_track_feed_read(None, None, "feed-a"),
            IndexTrackFeedRead::Send
        );
    }

    /// R47-07: a detail result for a feed the operator already navigated
    /// away from must not replace the mounted page.
    #[test]
    fn adr_0075_search_summary_r47_07_feed_detail_stale_result_for_replaced_guid_is_ignored() {
        let current = Some(IndexFeedDetailState::Loading {
            feed_guid: "guid-b".into(),
        });

        assert!(!index_feed_detail_result_is_current(
            current.as_ref(),
            "guid-a"
        ));
        assert!(index_feed_detail_result_is_current(
            current.as_ref(),
            "guid-b"
        ));
    }

    /// A result also applies when the mounted page already reached
    /// `Loaded` or `Failed` for that same feed GUID, not only `Loading`.
    #[test]
    fn adr_0075_search_summary_feed_detail_result_is_current_for_any_lifecycle_step() {
        let loaded = Some(IndexFeedDetailState::Loaded {
            feed_guid: "guid-a".into(),
            feed: Box::default(),
        });
        assert!(index_feed_detail_result_is_current(
            loaded.as_ref(),
            "guid-a"
        ));

        let failed = Some(IndexFeedDetailState::Failed {
            feed_guid: "guid-a".into(),
            message: "error".into(),
        });
        assert!(index_feed_detail_result_is_current(
            failed.as_ref(),
            "guid-a"
        ));

        assert!(!index_feed_detail_result_is_current(None, "guid-a"));
    }

    /// R47-07: a detail result for a track the operator already navigated
    /// away from must not replace the mounted page.
    #[test]
    fn adr_0075_search_summary_r47_07_track_detail_stale_result_for_replaced_target_is_ignored() {
        let current = Some(IndexTrackDetailState::Loading {
            target: "feed-guid:track-b".into(),
        });

        assert!(!index_track_detail_result_is_current(
            current.as_ref(),
            "feed-guid:track-a"
        ));
        assert!(index_track_detail_result_is_current(
            current.as_ref(),
            "feed-guid:track-b"
        ));
    }

    /// A result also applies when the mounted page already reached
    /// `Loaded` or `Failed` for that same target, not only `Loading`.
    #[test]
    fn adr_0075_search_summary_track_detail_result_is_current_for_any_lifecycle_step() {
        let loaded = Some(IndexTrackDetailState::Loaded {
            target: "track-a".into(),
            track: Box::default(),
        });
        assert!(index_track_detail_result_is_current(
            loaded.as_ref(),
            "track-a"
        ));

        let failed = Some(IndexTrackDetailState::Failed {
            target: "track-a".into(),
            message: "error".into(),
        });
        assert!(index_track_detail_result_is_current(
            failed.as_ref(),
            "track-a"
        ));

        assert!(!index_track_detail_result_is_current(None, "track-a"));
    }

    #[test]
    fn split_index_track_target_separates_a_scoped_target() {
        assert_eq!(
            split_index_track_target("feed-guid:track-guid"),
            (Some("feed-guid"), "track-guid")
        );
        assert_eq!(split_index_track_target("track-guid"), (None, "track-guid"));
    }
}

#[cfg(test)]
mod remote_detail_thumbnail_tests {
    use super::*;

    #[test]
    fn index_track_artwork_url_uses_track_image() {
        let track = TrackView {
            image_url: Some("https://example.test/track.jpg".to_string()),
            ..TrackView::default()
        };

        assert_eq!(
            index_track_artwork_url(&track),
            Some("https://example.test/track.jpg")
        );
    }

    #[test]
    fn index_track_artwork_url_ignores_empty_image() {
        let track = TrackView {
            image_url: Some("   ".to_string()),
            ..TrackView::default()
        };

        assert_eq!(index_track_artwork_url(&track), None);
    }

    /// R48-05 (ADR 0075 packet 048): `api_track_from_view` carries the
    /// track's own claimed image forward. It never turns a feed image
    /// into a track-owned image.
    #[test]
    fn adr_0075_track_artwork_r48_05_api_track_from_view_gives_no_track_image_from_feed() {
        let feed = FeedView {
            image_url: Some("https://example.test/feed.jpg".to_string()),
            ..FeedView::default()
        };
        let track = TrackView {
            track_image_url: None,
            feed_image_url: Some("https://example.test/feed.jpg".to_string()),
            ..TrackView::default()
        };

        let api_track = api_track_from_view(&feed, &track);

        assert_eq!(api_track.track_image_url, None);
        assert_eq!(
            api_track.feed_image_url.as_deref(),
            Some("https://example.test/feed.jpg")
        );
    }
}

#[cfg(test)]
mod tests {
    /// R5-05: each completed search download requests one tag update scan.
    /// No GPUI test drives `TopApp`, so the test reads the success code of
    /// each search download. Packet 022 (ADR 0084 Decision 10) moved the
    /// success code of the Index downloads to the completion handlers of
    /// the download queue.
    #[test]
    fn adr_0076_follow_up_search_download_starts_one_tag_scan() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/search_dispatch.rs"),
        )
        .unwrap();
        let production = source.split("#[cfg(test)]").next().unwrap();
        for function in [
            "fn finish_index_release_download(",
            "fn finish_index_track_download(",
        ] {
            let body = production
                .split(function)
                .nth(1)
                .and_then(|rest| rest.split("\n    fn ").next())
                .unwrap_or_else(|| panic!("{function} exists"));
            assert_eq!(
                body.matches("self.scan_tags_after_search_download(cx);")
                    .count(),
                1,
                "{function}"
            );
        }
        let completion = production
            .split("pub(super) fn finish_index_download(")
            .nth(1)
            .and_then(|rest| rest.split("\n    fn ").next())
            .expect("finish_index_download exists");
        assert!(
            completion.contains("if *outcome != DownloadOutcome::Succeeded {"),
            "only a successful download starts the tag scan"
        );
        {
            let (function, error_handler) = (
                "fn subscribe_then_append_to_playlist(",
                "|this, error, _cx|",
            );
            let body = production
                .split(function)
                .nth(1)
                .and_then(|rest| rest.split("\n    fn ").next())
                .unwrap_or_else(|| panic!("{function} exists"));
            let success = body
                .split("present_command(")
                .nth(1)
                .and_then(|rest| rest.split(error_handler).next())
                .unwrap_or_else(|| panic!("{function} has a success handler"));
            assert_eq!(
                success
                    .matches("this.scan_tags_after_search_download(cx);")
                    .count(),
                1,
                "{function}"
            );
        }
        let helper = production
            .split("fn scan_tags_after_search_download(")
            .nth(1)
            .unwrap();
        assert_eq!(helper.matches("scan_tag_updates()").count(), 1);
        assert_eq!(production.matches("scan_tag_updates()").count(), 1);
    }
}
