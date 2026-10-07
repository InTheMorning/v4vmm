//! Search dispatch, Index async wiring, and drill-down helpers.

use std::sync::Arc;

use anyhow::Result;
use gpui::{prelude::*, AnyElement, Context, Image, SharedString, Window};

use crate::application::capability::Dependency;
use crate::application::capability_recovery::{RecoveryAction, RecoveryIntent};
use crate::application::commands::download::{SubscribeThenAppendToPlaylist, SubscribeTrack};
use crate::application::commands::feed::SubscribeFeed;
use crate::application::commands::playlist::CreatePlaylist;
use crate::application::queries::images::FetchThumbnail;
use crate::application::queries::search::{
    FetchIndexFeedDetail, FetchIndexSearchResults, FetchIndexTrackDetail,
};
use crate::application::{ApplicationCommand, CommandContext};
use crate::db;
use crate::feed_service;
use crate::library::{playlist_options, LibraryApp};
use crate::library_service;
use crate::metadata::TrackContext;
use crate::presentation::present_command;
use crate::subscribe_service::{SubscribeFeedRequest, SubscribeTrackRequest};
use crate::ui::composites::{
    action_button, ActionButtonDisplay, AddToPlaylistDisplay, AddToPlaylistPopover,
    DisclosureTextPanel, DisclosureTextPanelDisplay, ReleaseSurfaceElement, TrackSurfaceElement,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button as UiButton;
use crate::ui::shells::entity::{
    render_release_track_row, ReleaseDetailBehaviorSlots, ReleaseTrackRowSlot,
};
use crate::ui::shells::search_results_inspector::{
    render_index_detail_display, render_index_feed_detail, render_index_track_detail,
};
use crate::ui::shells::track::{render_track_name_links, TrackDetailBehaviorSlots};
use crate::view_models::entity_detail::{
    EntityActionTarget, EntitySurfaceContext, ReleaseDetailVm, SharedTrackRowVm,
};
use crate::view_models::publisher_page::PublisherPageContext;
use crate::view_models::search_results::{
    IndexDetailDisplay, IndexDetailKind, SearchResultsInspectorPageVm, SearchResultsTab,
};
use crate::view_models::track_detail::{
    TrackDetailSurfaceContext, TrackDetailVm, TrackNameLinkTarget, TrackPageAction,
    TrackPageActions,
};
use crate::view_models::workspace::{FrameNavigationEntry, FrameNavigationState, WorkspaceFrameId};
use crate::views::{ArtistRef, FeedRef, FeedView, TrackRef, TrackView};

use super::{AppTab, TopApp};

#[derive(Clone)]
pub(super) enum RemoteDetailThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
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
                        self.library.update(cx, |library, cx| {
                            library.select_track(&track_row, cx);
                        });
                        if let Err(e) = self.workspace_layout.push_nav(
                            content_frame_id,
                            FrameNavigationEntry::TrackDetail(track_id),
                        ) {
                            self.settings_status = format!("Failed to navigate to track: {e}");
                        }
                        cx.notify();
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
                    self.open_library_album_in_frame(feed_id, content_frame_id, cx);
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
                self.library.update(cx, |library, cx| {
                    library.select_artist(artist_name, cx);
                });
                if let Err(e) = self.workspace_layout.push_nav(
                    content_frame_id,
                    FrameNavigationEntry::ArtistDetail(artist_name.to_string()),
                ) {
                    self.settings_status = format!("Failed to navigate to artist: {e}");
                }
                cx.notify();
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

        self.push_index_feed_detail(content_frame_id, feed_guid, label, cx);
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
        self.push_index_feed_detail(content_frame_id, feed_guid, label, cx);
        // ADR 0075 packet 047, Required Change 3: the search sent this
        // row's summary only. Opening it sends its own detail request.
        self.fetch_index_feed_detail_page(feed_guid.to_string(), cx);
    }

    fn push_index_feed_detail(
        &mut self,
        content_frame_id: WorkspaceFrameId,
        feed_guid: &str,
        label: String,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.workspace_layout.push_nav(
            content_frame_id,
            FrameNavigationEntry::IndexFeedDetail {
                id: feed_guid.to_string(),
                label,
            },
        ) {
            self.settings_status = format!("Failed to navigate to index feed: {error}");
        }
        self.sync_search_results_detail_with_nav(content_frame_id);
        cx.notify();
    }

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
            .unwrap_or_else(|| track_guid.to_string());
        self.push_index_track_detail(content_frame_id, target, label, cx);
        // ADR 0075 packet 047, Required Change 3: a track reached from the
        // name-match page already carries its full detail from that
        // page's own fetch (packet 006). Only a search-drawn summary row
        // needs its own detail request here.
        if self.name_match_page_track_row(&activation_id).is_none() {
            self.fetch_index_track_detail_page(
                target.to_string(),
                feed_guid.map(str::to_string),
                track_guid.to_string(),
                cx,
            );
        }
    }

    fn push_index_track_detail(
        &mut self,
        content_frame_id: WorkspaceFrameId,
        target: &str,
        label: String,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.workspace_layout.push_nav(
            content_frame_id,
            FrameNavigationEntry::IndexTrackDetail {
                id: target.to_string(),
                label,
            },
        ) {
            self.settings_status = format!("Failed to navigate to index track: {error}");
        }
        self.sync_search_results_detail_with_nav(content_frame_id);
        cx.notify();
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

    /// Restores the Index feed detail-on-open fetch for a navigation entry
    /// that a breadcrumb or a history move (back or forward) just restored
    /// (ADR 0075 packet 047). Starts no fetch when `entry` is not an Index
    /// feed detail page reached from an active search, or when the mounted
    /// page already matches its feed GUID.
    pub(super) fn restore_index_feed_detail_for_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        let FrameNavigationEntry::IndexFeedDetail { id, .. } = entry else {
            return;
        };
        let search_is_active = self
            .workspace_layout
            .frame_nav(content_frame_id)
            .and_then(FrameNavigationState::active_search_query)
            .is_some();
        if !search_is_active {
            return;
        }
        if index_feed_detail_result_is_current(self.index_feed_detail_state.as_ref(), id) {
            return;
        }
        self.fetch_index_feed_detail_page(id.clone(), cx);
    }

    /// Restores the Index track detail-on-open fetch for a navigation
    /// entry that a breadcrumb or a history move just restored (ADR 0075
    /// packet 047). Starts no fetch when `entry` is not an Index track
    /// detail page, when the operator reached it from the name-match page
    /// (that page's own fetch already carries its full detail), or when
    /// the mounted page already matches its activation target.
    pub(super) fn restore_index_track_detail_for_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        let FrameNavigationEntry::IndexTrackDetail { id, .. } = entry else {
            return;
        };
        if self.search_results_detail.is_none() && self.name_match_page.is_none() {
            return;
        }
        let activation_id = format!("index-track:{id}");
        if self.name_match_page_track_row(&activation_id).is_some() {
            return;
        }
        if index_track_detail_result_is_current(self.index_track_detail_state.as_ref(), id) {
            return;
        }
        let (feed_guid, track_guid) = split_index_track_target(id);
        self.fetch_index_track_detail_page(
            id.clone(),
            feed_guid.map(str::to_string),
            track_guid.to_string(),
            cx,
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

    /// Opens the Library album page of `feed_id` in the content frame.
    /// The album name link of a track page uses it (ADR 0083 Decision 5).
    pub(super) fn open_library_album_page(&mut self, feed_id: i64, cx: &mut Context<Self>) {
        let Some(content_frame_id) = self.content_list_frame_id() else {
            self.settings_status = "ContentList frame not found".to_string();
            cx.notify();
            return;
        };
        self.open_library_album_in_frame(feed_id, content_frame_id, cx);
    }

    fn open_library_album_in_frame(
        &mut self,
        feed_id: i64,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        let album_found = self.library.read(cx).album_for_detail_by_feed_id(feed_id);
        if let Some(album) = album_found {
            self.library.update(cx, |library, cx| {
                library.select_album(&album, cx);
            });
            if let Err(e) = self
                .workspace_layout
                .push_nav(content_frame_id, FrameNavigationEntry::AlbumDetail(feed_id))
            {
                self.settings_status = format!("Failed to navigate to feed: {e}");
            }
        } else {
            self.settings_status = format!("Feed {feed_id} not found");
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
        ReleaseDetailBehaviorSlots {
            hero_image: self.index_feed_hero_image(feed, cx),
            primary_actions: self.index_feed_primary_actions(feed, cx),
            description_panel: index_feed_description_panel(feed),
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
        let page = TrackDetailVm::new(track, TrackDetailSurfaceContext::Discover).page();
        let entity = cx.entity();
        let name_links = render_track_name_links(&page, move |target, _window, cx| {
            entity.update(cx, |this, cx| this.open_index_track_name_link(target, cx));
        });
        TrackDetailBehaviorSlots {
            hero_image: self.index_track_hero_image(track, cx),
            name_links,
            primary_actions: Self::index_track_page_actions(&page.page_actions(), track, cx),
            ..TrackDetailBehaviorSlots::default()
        }
    }

    /// The action row of an Index track page (ADR 0083 Decision 5): the
    /// filled "Download album". An Index track has no plain action and, with
    /// no feed URL, no menu item.
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
        .label(action.label())
        .a11y_label(action.a11y_label())
        .disabled(!filled.available);
        let button = match (action, feed_guid) {
            (TrackPageAction::DownloadAlbum, Some(feed_guid)) if filled.available => button
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.download_index_album_of_track(feed_guid.clone(), cx);
                })),
            _ => button,
        };
        vec![TrackSurfaceElement::from_element(button.into_any_element())]
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

    fn index_feed_primary_actions(
        &mut self,
        feed: &FeedView,
        cx: &mut Context<Self>,
    ) -> Vec<ReleaseSurfaceElement> {
        let feed_for_download = feed.clone();
        let download = action_button(
            ActionButtonDisplay {
                label: SharedString::from("Download Feed"),
                a11y_label: SharedString::from("Download feed"),
            },
            cx,
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            this.download_index_feed(&feed_for_download, cx);
        }));

        let musicbrainz = action_button(
            ActionButtonDisplay {
                label: SharedString::from("MusicBrainz"),
                a11y_label: SharedString::from("Look up missing MusicBrainz fields"),
            },
            cx,
        )
        .disabled(true);

        let playlists = self.library.read(cx).playlists().to_vec();
        let feed_for_select = feed.clone();
        let feed_for_create = feed.clone();
        let playlist = AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(format!(
                "index-feed-add:{}",
                feed_guid_from_view(feed).unwrap_or_else(|| "unknown".to_string())
            )),
            playlists: playlist_options(&playlists),
            trigger_label: SharedString::from("Add feed to playlist ▾"),
            trigger_a11y_label: SharedString::from("Add feed to playlist"),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add feed"),
        })
        .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
            this.add_index_feed_to_playlist(&feed_for_select, *playlist_id, cx);
        }))
        .on_create(cx.listener(move |this, name: &String, _window, cx| {
            this.create_playlist_and_add_index_feed(name, feed_for_create.clone(), cx);
        }));

        let mut actions = vec![
            ReleaseSurfaceElement::from_element(download.into_any_element()),
            ReleaseSurfaceElement::from_element(musicbrainz.into_any_element()),
            ReleaseSurfaceElement::from_element(playlist.into_any_element()),
        ];
        // ADR 0077 packet 004: an Index album with a received publisher
        // relationship exposes an "open publisher" action. This reads the
        // relationship that the L2 feed detail request already carries
        // (ADR 0077 Decision 5); it sends no new request.
        if let Some(publisher_action) =
            ReleaseDetailVm::new(feed, EntitySurfaceContext::Library).publisher_action()
        {
            let a11y_label = publisher_action.a11y_label();
            let EntityActionTarget::Artist(ArtistRef::PublisherFeed(publisher_feed_guid)) =
                publisher_action.target
            else {
                unreachable!(
                    "ReleaseDetailVm::publisher_action always targets a publisher feed GUID"
                )
            };
            let open_publisher = action_button(
                ActionButtonDisplay {
                    label: SharedString::from(publisher_action.label),
                    a11y_label: SharedString::from(a11y_label),
                },
                cx,
            )
            .disabled(!publisher_action.enabled)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_publisher_page(
                    publisher_feed_guid.clone(),
                    PublisherPageContext::Index,
                    cx,
                );
            }));
            actions.push(ReleaseSurfaceElement::from_element(
                open_publisher.into_any_element(),
            ));
        }

        actions
    }

    fn index_feed_track_rows(
        &mut self,
        feed: &FeedView,
        cx: &mut Context<Self>,
    ) -> Vec<ReleaseSurfaceElement> {
        feed.tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                let row = SharedTrackRowVm::new(track, EntitySurfaceContext::Library, index);
                let row_id = row.element_id();
                render_release_track_row(
                    SharedString::from(row_id),
                    row,
                    ReleaseTrackRowSlot {
                        thumbnail: self.index_track_row_thumbnail(feed, track, cx),
                        actions: self.index_track_row_actions(feed, track, index, cx),
                        ..ReleaseTrackRowSlot::default()
                    },
                )
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
                    this.show_downloaded_index_feed(feed_id, cx);
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

    fn show_downloaded_index_feed(&mut self, feed_id: i64, cx: &mut Context<Self>) {
        self.library.update(cx, |library, cx| {
            if let Some(album) = library.album_for_detail_by_feed_id(feed_id) {
                library.select_album(&album, cx);
            }
            library.refresh(cx);
        });

        if let Some(content_frame_id) = self.content_list_frame_id() {
            if let Some(nav) = self.workspace_layout.frame_nav_mut(content_frame_id) {
                if matches!(nav.current(), FrameNavigationEntry::IndexFeedDetail { .. }) {
                    nav.replace_current(FrameNavigationEntry::AlbumDetail(feed_id));
                }
            }
            self.sync_search_results_detail_with_nav(content_frame_id);
        }
        cx.notify();
    }

    fn download_index_track(&mut self, feed: &FeedView, track: &TrackView, cx: &mut Context<Self>) {
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
            |this, result, cx| {
                this.settings_status = result.message().to_string();
                this.reload_cached(cx);
                this.library.update(cx, LibraryApp::refresh);
                this.scan_tags_after_search_download(cx);
            },
            |this, error, _cx| {
                this.settings_status = format!("Error downloading track: {error:#}");
            },
        );
    }

    fn add_index_feed_to_playlist(
        &mut self,
        feed: &FeedView,
        playlist_id: i64,
        cx: &mut Context<Self>,
    ) {
        let Some(feed_guid) = feed_guid_from_view(feed) else {
            self.settings_status = "Cannot add feed without a MusicIndex feed id".to_string();
            cx.notify();
            return;
        };
        let feed_id = match feed_service::ensure_feed_in_db(
            &self.conn,
            &feed_guid,
            feed.feed_url.as_deref(),
            &self.musicindex_endpoint,
        ) {
            Ok(feed_id) => feed_id,
            Err(error) => {
                self.settings_status = format!("Error preparing feed: {error:#}");
                cx.notify();
                return;
            }
        };
        let track_ids = match self.feed_track_ids(feed_id) {
            Ok(track_ids) => track_ids,
            Err(error) => {
                self.settings_status = format!("Error reading feed tracks: {error:#}");
                cx.notify();
                return;
            }
        };
        self.subscribe_then_append_to_playlist(playlist_id, track_ids, cx);
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

    fn create_playlist_and_add_index_feed(
        &mut self,
        name: &str,
        feed: FeedView,
        cx: &mut Context<Self>,
    ) {
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.add_index_feed_to_playlist(&feed, result.playlist_id(), cx);
            },
            |this, error, _cx| {
                this.settings_status = format!("Error creating playlist: {error:#}");
            },
        );
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

    fn feed_track_ids(&self, feed_id: i64) -> Result<Vec<i64>> {
        let conn = self.conn.lock().expect("lock db");
        Ok(db::feed_tracks(&conn, feed_id)?
            .into_iter()
            .map(|track| track.id)
            .collect())
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
}

fn index_feed_description_panel(feed: &FeedView) -> Option<ReleaseSurfaceElement> {
    let description = feed.description.as_deref()?.trim();
    if description.is_empty() {
        return None;
    }

    Some(ReleaseSurfaceElement::from_element(
        DisclosureTextPanel::new(DisclosureTextPanelDisplay {
            id: SharedString::from(format!(
                "index-feed-description:{}",
                feed_guid_from_view(feed).unwrap_or_else(|| "unknown".to_string())
            ))
            .into(),
            label: SharedString::from("Description"),
            a11y_label: SharedString::from("Toggle feed description"),
            body: SharedString::from(description.to_string()),
            collapsed: false,
        })
        .into_any_element(),
    ))
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
