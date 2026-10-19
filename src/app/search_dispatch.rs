//! Search dispatch, Index async wiring, and drill-down helpers.

use std::sync::Arc;

use gpui::{prelude::*, AnyElement, Context, Image, SharedString, Window};

use crate::application::capability::Dependency;
use crate::application::capability_recovery::{RecoveryAction, RecoveryIntent};
use crate::application::commands::download::{SubscribeThenAppendToPlaylist, SubscribeTrack};
use crate::application::commands::feed::SubscribeFeed;
use crate::application::commands::playlist::CreatePlaylist;
use crate::application::queries::images::{FetchCoverColor, FetchThumbnail};
use crate::application::queries::search::{
    FetchIndexFeedDetail, FetchIndexSearchResults, FetchIndexTrackDetail,
};
use crate::application::{ApplicationCommand, CommandContext};
use crate::db;
use crate::feed_service;
use crate::library::{playlist_options, LibraryApp};
use crate::library_service;
use crate::media::cover_color::CoverColor;
use crate::metadata::TrackContext;
use crate::presentation::present_command;
use crate::subscribe_service::{SubscribeFeedRequest, SubscribeTrackRequest};
use crate::ui::composites::{
    ActionRow, ActionRowDisplay, AddToPlaylistDisplay, AddToPlaylistPopover, DisclosureTextPanel,
    DisclosureTextPanelDisplay, ReleaseSurfaceElement, TrackSurfaceElement,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{
    Button as UiButton, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
};
use crate::ui::shells::entity::{
    render_album_name_links, render_named_feed_rows, render_release_track_row,
    ReleaseDetailBehaviorSlots, ReleaseTrackRowSlot,
};
use crate::ui::shells::search_results_inspector::{
    render_index_detail_display, render_index_feed_detail, render_index_track_detail,
};
use crate::ui::shells::track::{render_track_name_links, TrackDetailBehaviorSlots};
use crate::view_models::album_page::{
    album_page_actions, AlbumNameLinkTarget, AlbumPageAction, AlbumPageOrigin,
    ALBUM_PAGE_MENU_A11Y_LABEL,
};
use crate::view_models::entity_detail::{EntitySurfaceContext, ReleaseDetailVm, SharedTrackRowVm};
use crate::view_models::library::{
    display_description_text, keyed_description_state, toggle_keyed_description_state,
    DescriptionDisclosureDisplay, DescriptionState, LibraryTrackRowVm,
};
use crate::view_models::publisher_page::PublisherPageContext;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::search_results::{
    IndexDetailDisplay, IndexDetailKind, SearchResultsInspectorPageVm, SearchResultsTab,
};
use crate::view_models::track_detail::{
    TrackDetailPageVm, TrackDetailSurfaceContext, TrackDetailVm, TrackNameLinkTarget,
    TrackPageAction, TrackPageActions,
};
use crate::view_models::workspace::{
    release_download_updates_open_page, resolve_release_page_source, resolve_track_page_source,
    track_download_updates_open_page, track_release_download_updates_open_page,
    FrameNavigationEntry, FrameNavigationState, NavigationPathStart, ReleaseKey, ReleasePageSource,
    TrackKey, TrackPageSource, WorkspaceFrameId,
};
use crate::views::{FeedRef, FeedView, TrackRef, TrackView};

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

/// Loading lifecycle of the mounted Index track detail page (ADR 0075
/// packet 047, Required Change 3). `target` is the row's activation target:
/// `<feed_guid>:<track_guid>` for a scoped hit, or a bare `<track_guid>`
/// for an unscoped one. `None` on `TopApp` means no fetch is tracked: the
/// operator has not opened a track row, or opened one whose full detail
/// already came from the name-match page's own fetch
/// (`name_match_page_track_row` covers that case).
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
                self.search_results_detail = Some(self.search_results_detail_for_query(&query));
                self.start_index_search_for_query(&query, cx);
                cx.notify();
            }
            Err(e) => {
                self.settings_status = format!("Error opening search results: {e}");
                cx.notify();
            }
        }
    }

    fn search_results_detail_for_query(&self, query: &str) -> SearchResultsInspectorPageVm {
        let local_tracks = {
            let conn = self.conn.lock().expect("lock db");
            self.application_services
                .query_service()
                .search_local_library_tracks(&conn, query, None)
                .unwrap_or_default()
        };

        SearchResultsInspectorPageVm::from_local_library_tracks(query, &local_tracks)
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

        let command =
            FetchIndexSearchResults::new(self.musicindex_endpoint.clone(), query.to_owned());
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
        self.search_results_detail = Some(self.search_results_detail_for_query(query));
        if let Some(detail) = &mut self.search_results_detail {
            detail.mark_index_loading();
        }
        let id = intent.id;
        let command = self.retry_command(
            FetchIndexSearchResults::new(self.musicindex_endpoint.clone(), query.to_owned()),
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
                        Some(this.search_results_detail_for_query(&success_query));
                }

                if let Some(detail) = this
                    .search_results_detail
                    .as_mut()
                    .filter(|detail| detail.query() == success_query)
                {
                    detail.replace_index_results(rows);
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
                        Some(this.search_results_detail_for_query(&error_query));
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
                self.search_results_detail = Some(self.search_results_detail_for_query(&query));
            }
        } else {
            self.search_results_detail = None;
        }
    }

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

        match tab {
            SearchResultsTab::Tracks => {
                if let Some(target) = result_id.strip_prefix("index-track:") {
                    self.handle_index_track_result_selected(target, content_frame_id, cx);
                    return;
                }

                let track_id_str = result_id
                    .strip_prefix("library-track:")
                    .unwrap_or(result_id);
                if let Ok(track_id) = track_id_str.parse::<i64>() {
                    if let Some(track_row) = self.conn.lock().ok().and_then(|conn| {
                        library_service::track_row_by_id(&conn, track_id)
                            .ok()
                            .flatten()
                    }) {
                        let key = track_row.track_key();
                        let label = LibraryTrackRowVm::new(&track_row, None).display_title();
                        self.open_track_in_frame(content_frame_id, &key, label, cx);
                    } else {
                        self.settings_status = format!("Track {track_id} not found");
                        cx.notify();
                    }
                } else {
                    self.settings_status = format!("Invalid track id: {track_id_str}");
                    cx.notify();
                }
            }
            SearchResultsTab::Feeds => {
                if let Some(feed_guid) = result_id.strip_prefix("index-feed:") {
                    self.handle_index_feed_result_selected(feed_guid, content_frame_id, cx);
                    return;
                }

                if let Ok(feed_id) = result_id.parse::<i64>() {
                    self.open_library_release_in_frame(feed_id, content_frame_id, cx);
                } else {
                    self.settings_status = format!("Invalid feed id: {result_id}");
                    cx.notify();
                }
            }
            SearchResultsTab::Artists => {
                if let Some(artist_name) = result_id.strip_prefix("index-artist:") {
                    self.handle_index_artist_result_selected(artist_name, content_frame_id, cx);
                    return;
                }

                let Some(artist_name) = result_id.strip_prefix("library-artist:") else {
                    self.settings_status = format!("Unexpected artist id format: {result_id}");
                    cx.notify();
                    return;
                };
                self.open_library_artist_in_frame(artist_name, content_frame_id, cx);
            }
        }

        self.sync_search_results_detail_with_nav(content_frame_id);
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

    /// ADR 0077 packet 006: an Index name candidate opens the name-match
    /// track page, a search result, never an artist detail page.
    fn handle_index_artist_result_selected(
        &mut self,
        artist_name: &str,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        self.open_name_match_page(artist_name.to_string(), content_frame_id, cx);
        self.sync_search_results_detail_with_nav(content_frame_id);
        cx.notify();
    }

    fn handle_index_feed_result_selected(
        &mut self,
        feed_guid: &str,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        let activation_id = format!("index-feed:{feed_guid}");
        let label = self
            .search_results_detail
            .as_ref()
            .and_then(|detail| detail.index_feed_label(&activation_id))
            .unwrap_or_else(|| feed_guid.to_string());
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
        self.sync_search_results_detail_with_nav(content_frame_id);
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

    /// ADR 0084 Decision 4: an Index track row, whatever its origin (a
    /// search result or a name-match row), opens its track page by key.
    fn handle_index_track_result_selected(
        &mut self,
        target: &str,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        let activation_id = format!("index-track:{target}");
        let (feed_guid, track_guid) = split_index_track_target(target);
        let label = self
            .search_results_detail
            .as_ref()
            .and_then(|detail| detail.index_track_label(&activation_id))
            .or_else(|| {
                self.name_match_page_track_row(&activation_id)
                    .map(|row| row.label)
            })
            .unwrap_or_else(|| track_guid.to_string());
        let key = TrackKey::from_index_track_target(feed_guid, track_guid);
        self.open_track_in_frame(content_frame_id, &key, label, cx);
    }

    /// Opens a track page by its key, from any origin (ADR 0084
    /// Decision 4). Replaces `push_index_track_detail`: one function
    /// pushes the entry, resolves its data source, and restores the page.
    fn open_track_in_frame(
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
        self.sync_search_results_detail_with_nav(content_frame_id);
        self.apply_track_page_source(key, cx);
        cx.notify();
    }

    /// Resolves and stores the data source of an open track page, then
    /// mounts it (ADR 0084 Decision 4). The Library source asks the
    /// Library to mount the track. The Index source clears the Library
    /// detail and starts its own fetch, unless the name-match page already
    /// cached this track.
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
                let activation_id = format!("index-track:{target}");
                let already_cached = self.name_match_page_track_row(&activation_id).is_some();
                if !already_cached
                    && !index_track_detail_result_is_current(
                        self.index_track_detail_state.as_ref(),
                        target,
                    )
                {
                    let (feed_guid, track_guid) = split_index_track_target(target);
                    self.fetch_index_track_detail_page(
                        target.clone(),
                        feed_guid.map(str::to_string),
                        track_guid.to_string(),
                        cx,
                    );
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
                this.index_track_detail_state = Some(IndexTrackDetailState::Loaded {
                    target: success_target,
                    track: Box::new(track),
                });
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
                IndexDetailDisplay::failed(IndexDetailKind::Feed, id, label, message.clone())
            }
            _ => IndexDetailDisplay::loading(IndexDetailKind::Feed, id, label),
        }
    }

    /// Selects the Index track detail display state to render (ADR 0075
    /// packet 047). A track reached from the name-match page shows that
    /// page's own cached detail, at no extra request; any other track
    /// shows the operator's own fetch, once it matches the mounted
    /// activation target, or a loading placeholder before it does.
    pub(super) fn index_track_detail_display(&self, id: &str, label: &str) -> IndexDetailDisplay {
        let activation_id = format!("index-track:{id}");
        if let Some(row) = self.name_match_page_track_row(&activation_id) {
            return IndexDetailDisplay::track_or_fallback(Some(&row), id, label);
        }
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

    /// Opens the Library artist page of `name` in the content frame. The
    /// artist name link of an album page uses it (ADR 0083 task 005).
    pub(super) fn open_library_artist_page(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(content_frame_id) = self.content_list_frame_id() else {
            self.settings_status = "ContentList frame not found".to_string();
            cx.notify();
            return;
        };
        self.open_library_artist_in_frame(name, content_frame_id, cx);
    }

    fn open_library_artist_in_frame(
        &mut self,
        name: &str,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        self.library.update(cx, |library, cx| {
            library.select_artist(name, cx);
        });
        if let Err(e) = self.workspace_layout.push_nav(
            content_frame_id,
            FrameNavigationEntry::ArtistDetail(name.to_string()),
        ) {
            self.settings_status = format!("Failed to navigate to artist: {e}");
        }
        cx.notify();
    }

    pub(super) fn render_index_feed_or_fallback_detail(
        &mut self,
        detail: &crate::view_models::search_results::IndexDetailDisplay,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(feed) = detail.feed.as_ref() {
            let slots = self.index_feed_detail_slots(feed, cx);
            return render_index_feed_detail(feed, slots);
        }
        if let Some(track) = detail.track.as_ref() {
            let slots = self.index_track_detail_slots(track, cx);
            return render_index_track_detail(track, slots, cx);
        }
        render_index_detail_display(detail, cx)
    }

    fn index_feed_detail_slots(
        &mut self,
        feed: &FeedView,
        cx: &mut Context<Self>,
    ) -> ReleaseDetailBehaviorSlots {
        let entity = cx.entity();
        // ADR 0084 Decision 6: a release whose key is in a Library singles
        // group links to that group. An Index release that is not in the
        // Library gets no link.
        let release_key = feed
            .feed_guid
            .as_deref()
            .map(ReleaseKey::from_index_feed_guid);
        let projection = ReleaseDetailVm::new(feed, EntitySurfaceContext::Library);
        let links = self
            .library
            .read(cx)
            .release_page_name_links(projection.name_links(), release_key.as_ref());
        let name_links = render_album_name_links(
            links,
            {
                let entity = entity.clone();
                move |target, _window, cx| {
                    entity.update(cx, |this, cx| this.open_index_album_name_link(target, cx));
                }
            },
            cx,
        );
        let named_feed_rows = render_named_feed_rows(
            projection.named_feed_rows(),
            move |target, _window, cx| {
                entity.update(cx, |this, cx| this.open_index_album_name_link(target, cx));
            },
            cx,
        );
        ReleaseDetailBehaviorSlots {
            hero_image: self.index_feed_hero_image(feed, cx),
            cover_color: index_feed_artwork_url(feed)
                .map(str::to_owned)
                .and_then(|url| self.index_cover_color(&url, cx)),
            name_links,
            named_feed_rows,
            primary_actions: Self::index_feed_primary_actions(feed, cx),
            description_panel: self.index_feed_description_panel(feed, cx),
            track_rows: Some(self.index_feed_track_rows(feed, cx)),
            ..ReleaseDetailBehaviorSlots::default()
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

    fn index_track_detail_slots(
        &mut self,
        track: &TrackView,
        cx: &mut Context<Self>,
    ) -> TrackDetailBehaviorSlots {
        let page = TrackDetailVm::new(track, TrackDetailSurfaceContext::Discover)
            .with_named_feeds(&track.named_feeds)
            .page();
        let entity = cx.entity();
        let name_links = render_track_name_links(
            &page,
            move |target, _window, cx| {
                entity.update(cx, |this, cx| this.open_index_track_name_link(target, cx));
            },
            cx,
        );
        TrackDetailBehaviorSlots {
            hero_image: self.index_track_hero_image(track, cx),
            name_links,
            primary_actions: Self::index_track_page_actions(&page.page_actions(), track, cx),
            // ADR 0084 Decision 4, ADR 0047 item 8 (amended 2026-10-16):
            // an Index track page shows its description, with the same
            // panel and the same projection as the Index release page.
            description_panel: self.index_track_description_panel(track, &page, cx),
            ..TrackDetailBehaviorSlots::default()
        }
    }

    /// The action row of an Index track page (ADR 0083 Decision 5): the
    /// filled "Download release". An Index track has no plain action and,
    /// with no feed URL, no menu item. `TrackView` carries no stated kind
    /// and no track count of its release (ADR 0084 Decision 5), and this
    /// packet sends no new `MusicIndex` request for one, so the label
    /// names no known kind.
    fn index_track_page_actions(
        actions: &TrackPageActions,
        track: &TrackView,
        cx: &mut Context<Self>,
    ) -> Vec<TrackSurfaceElement> {
        let filled = actions.filled;
        let action = filled.action;
        let feed_guid = track.feed_guid.clone();
        let button = UiButton::styled(
            SharedString::from(format!("index-track-page-action:{action:?}")),
            ControlStyle::Primary,
        )
        .label(action.label(None))
        .a11y_label(action.a11y_label(None))
        .disabled(!filled.available);
        let button = match (action, feed_guid.clone()) {
            (TrackPageAction::DownloadAlbum, Some(feed_guid)) if filled.available => button
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.download_index_album_of_track(feed_guid.clone(), cx);
                })),
            _ => button,
        };
        let mut elements = vec![TrackSurfaceElement::from_element(button.into_any_element())];
        if let (false, Some(feed_guid)) = (actions.menu.is_empty(), feed_guid) {
            let entity = cx.entity();
            let items = actions.menu.iter().map(|display| {
                let action = display.action;
                let mut item = ContextMenuItem::new(ContextMenuItemDisplay {
                    id: SharedString::from(format!("index-track-page-menu:{action:?}")),
                    label: SharedString::from(action.label(None)),
                    a11y_label: SharedString::from(action.a11y_label(None)),
                    destructive: action.is_destructive(),
                    disabled: !display.available,
                });
                if display.available && action == TrackPageAction::CopyFeedUrl {
                    let entity = entity.clone();
                    let feed_guid = feed_guid.clone();
                    item = item.on_select(move |_window, cx| {
                        entity.update(cx, |this, cx| {
                            this.copy_index_feed_url_of_track(feed_guid.clone(), cx);
                        });
                    });
                }
                item
            });
            elements.push(TrackSurfaceElement::from_element(
                ContextMenu::new(
                    SharedString::from(format!("index-track-page-menu-{feed_guid}")),
                    ContextMenuScope::TrackList,
                    SharedString::from("More actions for this track"),
                )
                .trigger_label("")
                .items(items)
                .into_any_element(),
            ));
        }
        elements
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

    /// "Download album" of an Index track: reads the album feed from
    /// `MusicIndex`, then downloads it like the Index album page does.
    fn download_index_album_of_track(&mut self, feed_guid: String, cx: &mut Context<Self>) {
        let command = FetchIndexFeedDetail::new(self.musicindex_endpoint.clone(), feed_guid);
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, feed, cx| this.download_index_feed(&feed, cx),
            |this, error, _cx| {
                this.settings_status = format!(
                    "App could not read the album of this track from MusicIndex: {error:#}"
                );
            },
        );
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

    /// The action row of an Index album page (ADR 0083 Decision 5): the
    /// filled "Download …" and the "⋯" menu with "Copy feed URL". The
    /// label names the release kind (ADR 0084 Decision 5).
    fn index_feed_primary_actions(
        feed: &FeedView,
        cx: &mut Context<Self>,
    ) -> Vec<ReleaseSurfaceElement> {
        let kind = ReleaseKind::resolve(feed.release_kind.as_deref(), feed.track_count());
        let actions = album_page_actions(AlbumPageOrigin::Index, feed.feed_url.as_deref());
        let filled = actions.filled;
        let action = filled.action;
        let button = UiButton::styled(
            SharedString::from(format!("index-album-page-action:{action:?}")),
            ControlStyle::Primary,
        )
        .label(action.label(kind))
        .a11y_label(action.a11y_label(kind))
        .disabled(!filled.available);
        let button = if action == AlbumPageAction::DownloadAlbum && filled.available {
            let feed_for_download = feed.clone();
            button.on_click(cx.listener(move |this, _, _, cx| {
                this.download_index_feed(&feed_for_download, cx);
            }))
        } else {
            button
        };
        let mut controls = vec![button.into_any_element()];
        if !actions.menu.is_empty() {
            let feed_url = feed.feed_url.clone();
            let items = actions.menu.iter().map(|display| {
                let action = display.action;
                let mut item = ContextMenuItem::new(ContextMenuItemDisplay {
                    id: SharedString::from(format!("index-album-page-menu:{action:?}")),
                    label: SharedString::from(action.label(kind)),
                    a11y_label: SharedString::from(action.a11y_label(kind)),
                    destructive: action.is_destructive(),
                    disabled: !display.available,
                });
                if let (true, AlbumPageAction::CopyFeedUrl, Some(url)) =
                    (display.available, action, feed_url.clone())
                {
                    item = item.on_select(move |_window, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(url.clone()));
                    });
                }
                item
            });
            controls.push(
                ContextMenu::new(
                    SharedString::from(format!(
                        "index-album-page-menu-{}",
                        feed_guid_from_view(feed).unwrap_or_else(|| "unknown".to_string())
                    )),
                    ContextMenuScope::TrackList,
                    SharedString::from(ALBUM_PAGE_MENU_A11Y_LABEL),
                )
                .trigger_label("")
                .items(items)
                .into_any_element(),
            );
        }
        vec![ReleaseSurfaceElement::from_element(
            ActionRow::new(ActionRowDisplay {
                a11y_label: SharedString::from("Album actions"),
            })
            .control_group(controls)
            .into_any_element(),
        )]
    }

    /// Opens the page of a name link under an Index album title. The Index
    /// has no artist page, so an artist name opens the tracks that match it.
    fn open_index_album_name_link(&mut self, target: &AlbumNameLinkTarget, cx: &mut Context<Self>) {
        match target {
            AlbumNameLinkTarget::Artist(name) => {
                let Some(content_frame_id) = self.content_list_frame_id() else {
                    self.settings_status = "ContentList frame not found".to_string();
                    cx.notify();
                    return;
                };
                self.open_name_match_page(name.clone(), content_frame_id, cx);
                self.sync_search_results_detail_with_nav(content_frame_id);
                cx.notify();
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

    fn index_feed_track_rows(
        &mut self,
        feed: &FeedView,
        cx: &mut Context<Self>,
    ) -> Vec<ReleaseSurfaceElement> {
        let feed_guid = feed_guid_from_view(feed);
        feed.tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                let row = SharedTrackRowVm::new(track, EntitySurfaceContext::Library, index);
                let row_id = row.element_id();
                let mut slot = ReleaseTrackRowSlot {
                    thumbnail: self.index_track_row_thumbnail(feed, track, cx),
                    actions: self.index_track_row_actions(feed, track, index, cx),
                    ..ReleaseTrackRowSlot::default()
                };
                // ADR 0084 Decision 4: a click on an Index track row opens
                // its track page, the same as a Library row. A track with
                // no track GUID gets no click, as today.
                if let Some(key) =
                    index_track_row_click_key(feed_guid.as_deref(), track_guid_from_view(track))
                {
                    // The search path uses the same fallback: the track GUID.
                    let label = track.title.clone().unwrap_or_else(|| key.item_guid.clone());
                    slot = slot.on_click(cx.listener(move |this, _, _, cx| {
                        let Some(content_frame_id) = this.content_list_frame_id() else {
                            return;
                        };
                        this.open_track_in_frame(content_frame_id, &key, label.clone(), cx);
                    }));
                }
                render_release_track_row(SharedString::from(row_id), row, slot)
            })
            .collect()
    }

    fn index_track_row_thumbnail(
        &mut self,
        feed: &FeedView,
        track: &TrackView,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Image>> {
        let url = index_track_row_artwork_url(feed, track)?;
        self.index_remote_detail_hero_image(url, cx)
    }

    fn index_track_row_actions(
        &mut self,
        feed: &FeedView,
        track: &TrackView,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Vec<ReleaseSurfaceElement> {
        let track_key = track_guid_from_view(track).unwrap_or_else(|| index.to_string());
        let feed_for_download = feed.clone();
        let track_for_download = track.clone();
        let download = UiButton::styled(
            SharedString::from(format!("index-track-download:{track_key}")),
            ControlStyle::RowAction,
        )
        .label("Download")
        .on_click(cx.listener(move |this, _, _, cx| {
            this.download_index_track(&feed_for_download, &track_for_download, cx);
        }));

        let playlists = self.library.read(cx).playlists().to_vec();
        let feed_for_select = feed.clone();
        let track_for_select = track.clone();
        let feed_for_create = feed.clone();
        let track_for_create = track.clone();
        let playlist = AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(format!("index-track-add:{track_key}")),
            playlists: playlist_options(&playlists),
            trigger_label: SharedString::from("+ Playlist"),
            trigger_a11y_label: SharedString::from("Add track to playlist"),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add track"),
        })
        .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
            this.add_index_track_to_playlist(&feed_for_select, &track_for_select, *playlist_id, cx);
        }))
        .on_create(cx.listener(move |this, name: &String, _window, cx| {
            this.create_playlist_and_add_index_track(
                name,
                feed_for_create.clone(),
                track_for_create.clone(),
                cx,
            );
        }));

        vec![
            ReleaseSurfaceElement::from_element(download.into_any_element()),
            ReleaseSurfaceElement::from_element(playlist.into_any_element()),
        ]
    }

    fn download_index_feed(&mut self, feed: &FeedView, cx: &mut Context<Self>) {
        let feed_guid = feed_guid_from_view(feed);
        let feed_url = feed.feed_url.clone();
        let command = SubscribeFeed::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeFeedRequest {
                feed: api_feed_from_view(feed),
                musicindex_endpoint: self.musicindex_endpoint.clone(),
            },
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.settings_status = result.message().to_string();
                this.reload_cached(cx);
                this.scan_tags_after_search_download(cx);
                if let Some(feed_id) =
                    this.downloaded_index_feed_id(feed_guid.as_deref(), feed_url.as_deref())
                {
                    this.show_downloaded_index_feed(feed_id, feed_guid.as_deref(), cx);
                } else {
                    this.library.update(cx, LibraryApp::refresh);
                }
            },
            |this, error, _cx| {
                this.settings_status = format!("Error downloading feed: {error:#}");
            },
        );
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
            self.sync_search_results_detail_with_nav(content_frame_id);
        }
        cx.notify();
    }

    fn download_index_track(&mut self, feed: &FeedView, track: &TrackView, cx: &mut Context<Self>) {
        let downloaded_key = TrackKey::from_index_track_target(
            feed_guid_from_view(feed).as_deref(),
            track_guid_from_view(track).unwrap_or_default(),
        );
        let command = SubscribeTrack::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeTrackRequest::SearchTrack {
                track_context: Box::new(TrackContext::new(
                    api_track_from_view(feed, track),
                    Some(api_feed_from_view(feed)),
                )),
                musicindex_endpoint: self.musicindex_endpoint.clone(),
                mark_feed_subscribed: false,
                return_tag_compare: true,
            },
            "Downloaded track".to_string(),
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.settings_status = result.message().to_string();
                this.reload_cached(cx);
                this.library.update(cx, LibraryApp::refresh);
                this.scan_tags_after_search_download(cx);
                // ADR 0084 Decision 4: the open track page of this exact
                // track resolves its source again, in place.
                let current = this
                    .content_list_frame_id()
                    .and_then(|id| this.workspace_layout.frame_nav(id))
                    .map(|nav| nav.current().clone());
                if track_download_updates_open_page(current.as_ref(), Some(&downloaded_key)) {
                    this.apply_track_page_source(&downloaded_key, cx);
                }
            },
            |this, error, _cx| {
                this.settings_status = format!("Error downloading track: {error:#}");
            },
        );
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

    /// The description panel of an Index release page (ADR 0084 Decision
    /// 4, ADR 0047 item 8 amended 2026-10-16). `None` when the feed names
    /// no description. The panel keeps its disclosure state in
    /// `index_release_description_states`, by feed GUID, for the session
    /// only.
    fn index_feed_description_panel(
        &mut self,
        feed: &FeedView,
        cx: &mut Context<Self>,
    ) -> Option<ReleaseSurfaceElement> {
        let description = display_description_text(feed.description.as_deref())?.to_string();
        let feed_guid = feed_guid_from_view(feed).unwrap_or_else(|| "unknown".to_string());
        let state = self.index_release_description_state(&feed_guid, &description);
        let display = DescriptionDisclosureDisplay::project(&description, state);
        let id = SharedString::from(format!("index-feed-description:{feed_guid}"));
        let toggle_feed_guid = feed_guid;
        let toggle_description = description;
        Some(ReleaseSurfaceElement::from_element(
            DisclosureTextPanel::new(DisclosureTextPanelDisplay {
                id: id.into(),
                label: SharedString::from("Description"),
                a11y_label: SharedString::from("Toggle feed description"),
                collapsed: display.collapsed,
                body: display.body.map(SharedString::from),
                show_more: display.show_more.map(|show_more| {
                    (
                        SharedString::from(show_more.label),
                        SharedString::from(show_more.a11y_label),
                    )
                }),
            })
            .on_toggle(cx.listener(move |this, _, _, cx| {
                this.toggle_index_release_description(&toggle_feed_guid, &toggle_description, cx);
            }))
            .into_any_element(),
        ))
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

    /// The description panel of an Index track page (ADR 0084 Decision 4,
    /// ADR 0047 item 8 amended 2026-10-16). `None` when the track names no
    /// description. Same panel, same projection and the same id pattern
    /// as the Index release page.
    fn index_track_description_panel(
        &mut self,
        track: &TrackView,
        page: &TrackDetailPageVm<'_>,
        cx: &mut Context<Self>,
    ) -> Option<TrackSurfaceElement> {
        let description = page.detail().description()?;
        let target = index_track_description_target(track);
        let state = self.index_track_description_state(&target, &description);
        let display = DescriptionDisclosureDisplay::project(&description, state);
        let label = SharedString::from(page.detail().labels().description_label());
        let id = SharedString::from(format!("index-track-description:{target}"));
        let toggle_target = target;
        let toggle_description = description;
        Some(TrackSurfaceElement::from_element(
            DisclosureTextPanel::new(DisclosureTextPanelDisplay {
                id: id.into(),
                label,
                a11y_label: SharedString::from("Toggle track description"),
                collapsed: display.collapsed,
                body: display.body.map(SharedString::from),
                show_more: display.show_more.map(|show_more| {
                    (
                        SharedString::from(show_more.label),
                        SharedString::from(show_more.a11y_label),
                    )
                }),
            })
            .on_toggle(cx.listener(move |this, _, _, cx| {
                this.toggle_index_track_description(&toggle_target, &toggle_description, cx);
            }))
            .into_any_element(),
        ))
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

fn index_feed_artwork_url(feed: &FeedView) -> Option<&str> {
    non_empty_str(feed.image_url.as_deref()).or_else(|| {
        feed.tracks
            .iter()
            .find_map(|track| non_empty_str(track.image_url.as_deref()))
    })
}

/// The image URL to show for an Index track (ADR 0075 Decision C,
/// packet 048). `TrackView::display_artwork_url` makes this choice. A
/// caller resolves the returned URL to an image.
fn index_track_artwork_url(track: &TrackView) -> Option<&str> {
    track.display_artwork_url()
}

fn index_track_row_artwork_url<'a>(feed: &'a FeedView, track: &'a TrackView) -> Option<&'a str> {
    index_track_artwork_url(track).or_else(|| index_feed_artwork_url(feed))
}

fn api_feed_from_view(feed: &FeedView) -> crate::api::Feed {
    crate::api::Feed {
        feed_guid: feed_guid_from_view(feed),
        title: feed.title.clone(),
        feed_url: feed.feed_url.clone(),
        release_artist: feed.artist.clone(),
        release_kind: feed.release_kind.clone(),
        release_date: feed.release_date,
        publisher_text: feed.publisher_text.clone(),
        language: feed.language.clone(),
        explicit: feed.explicit,
        episode_count: feed.episode_count,
        description: feed.description.clone(),
        image_url: feed.image_url.clone(),
        tracks: Some(
            feed.tracks
                .iter()
                .map(|track| api_track_from_view(feed, track))
                .collect(),
        ),
        payment_routes: Some(feed.payment_routes.clone()),
        ..crate::api::Feed::default()
    }
}

fn api_track_from_view(feed: &FeedView, track: &TrackView) -> crate::api::Track {
    crate::api::Track {
        track_guid: track_guid_from_view(track),
        feed_guid: track
            .feed_guid
            .clone()
            .or_else(|| feed_guid_from_view(feed)),
        feed_title: track.feed_title.clone().or_else(|| feed.title.clone()),
        title: track.title.clone(),
        duration_secs: track.duration_secs,
        pub_date: track.pub_date,
        track_number: track.track_number,
        explicit: track.explicit,
        description: track.description.clone(),
        enclosure_url: track.audio_url.clone(),
        enclosure_type: track.mime.clone(),
        enclosure_bytes: track.bytes,
        image_url: track.image_url.clone().or_else(|| feed.image_url.clone()),
        // ADR 0075 packet 048: carry the track's own claimed image
        // forward. Never set it from the feed's image (Decision C).
        track_image_url: track.track_image_url.clone(),
        feed_image_url: track.feed_image_url.clone(),
        track_artist: track.artist.clone(),
        release_artist: feed.artist.clone(),
        publisher_text: track.publisher_text.clone(),
        payment_routes: Some(track.payment_routes.clone()),
        ..crate::api::Track::default()
    }
}

fn feed_guid_from_view(feed: &FeedView) -> Option<String> {
    feed.feed_guid.clone().or_else(|| match &feed.id {
        Some(FeedRef::Musicindex(feed_guid)) => Some(feed_guid.clone()),
        Some(FeedRef::LocalFeedId(_)) | None => None,
    })
}

fn track_guid_from_view(track: &TrackView) -> Option<String> {
    track.track_guid.clone().or_else(|| match &track.id {
        Some(TrackRef::Musicindex(track_guid)) => Some(track_guid.clone()),
        Some(TrackRef::LocalTrackId(_)) | None => None,
    })
}

/// The click key of an Index track row (ADR 0084 Decision 4, "The Index
/// track rows"). `None` when the track has no track GUID, so the row
/// gets no click, as today. A pure function: a test calls it with no
/// GPUI.
fn index_track_row_click_key(
    feed_guid: Option<&str>,
    track_guid: Option<String>,
) -> Option<TrackKey> {
    Some(TrackKey::from_index_track_target(feed_guid, track_guid?))
}

/// The Index activation target of a track, for its description state key
/// (ADR 0084 Decision 4). Built the same way as the track-row click key.
fn index_track_description_target(track: &TrackView) -> String {
    TrackKey::from_index_track_target(
        track.feed_guid.as_deref(),
        track_guid_from_view(track).unwrap_or_default(),
    )
    .index_target()
}

#[cfg(test)]
mod index_detail_dispatch_tests {
    use super::*;

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

    /// R13-05 (ADR 0084 task 013): the Index track row click builds the
    /// key `TrackKey::from_index_track_target`, through a pure function.
    /// A track with no GUID gives no key.
    #[test]
    fn adr_0084_index_page_r13_05_track_row_click_key_needs_a_track_guid() {
        assert_eq!(
            index_track_row_click_key(Some("feed-1"), Some("track-1".to_string())),
            Some(TrackKey::from_index_track_target(Some("feed-1"), "track-1"))
        );
        assert_eq!(
            index_track_row_click_key(None, Some("track-1".to_string())),
            Some(TrackKey::from_index_track_target(None, "track-1"))
        );
        assert_eq!(index_track_row_click_key(Some("feed-1"), None), None);
        assert_eq!(index_track_row_click_key(None, None), None);
    }
}

#[cfg(test)]
mod remote_detail_thumbnail_tests {
    use super::*;

    #[test]
    fn index_feed_artwork_url_prefers_feed_image() {
        let feed = FeedView {
            image_url: Some("https://example.test/feed.jpg".to_string()),
            tracks: vec![TrackView {
                image_url: Some("https://example.test/track.jpg".to_string()),
                ..TrackView::default()
            }],
            ..FeedView::default()
        };

        assert_eq!(
            index_feed_artwork_url(&feed),
            Some("https://example.test/feed.jpg")
        );
    }

    #[test]
    fn index_feed_artwork_url_falls_back_to_track_image() {
        let feed = FeedView {
            tracks: vec![
                TrackView {
                    image_url: Some("   ".to_string()),
                    ..TrackView::default()
                },
                TrackView {
                    image_url: Some("https://example.test/track.jpg".to_string()),
                    ..TrackView::default()
                },
            ],
            ..FeedView::default()
        };

        assert_eq!(
            index_feed_artwork_url(&feed),
            Some("https://example.test/track.jpg")
        );
    }

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

    #[test]
    fn index_track_row_artwork_url_falls_back_to_feed_image() {
        let feed = FeedView {
            image_url: Some("https://example.test/feed.jpg".to_string()),
            ..FeedView::default()
        };
        let track = TrackView {
            image_url: None,
            ..TrackView::default()
        };

        assert_eq!(
            index_track_row_artwork_url(&feed, &track),
            Some("https://example.test/feed.jpg")
        );
    }

    #[test]
    fn index_track_row_artwork_url_prefers_track_image() {
        let feed = FeedView {
            image_url: Some("https://example.test/feed.jpg".to_string()),
            ..FeedView::default()
        };
        let track = TrackView {
            image_url: Some("https://example.test/track.jpg".to_string()),
            ..TrackView::default()
        };

        assert_eq!(
            index_track_row_artwork_url(&feed, &track),
            Some("https://example.test/track.jpg")
        );
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

fn non_empty_str(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    /// R5-05: each completed search download requests one tag update scan.
    /// No GPUI test drives `TopApp`, so the test reads the success handler
    /// of each search download command.
    #[test]
    fn adr_0076_follow_up_search_download_starts_one_tag_scan() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/search_dispatch.rs"),
        )
        .unwrap();
        let production = source.split("#[cfg(test)]").next().unwrap();
        for (function, error_handler) in [
            ("fn download_index_feed(", "|this, error, _cx|"),
            ("fn download_index_track(", "|this, error, _cx|"),
            (
                "fn subscribe_then_append_to_playlist(",
                "|this, error, _cx|",
            ),
        ] {
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
