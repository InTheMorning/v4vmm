//! Search dispatch, Index async wiring, and drill-down helpers.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use gpui::{prelude::*, AnyElement, Context, Image, SharedString};

use crate::application::commands::download::{SubscribeThenAppendToPlaylist, SubscribeTrack};
use crate::application::commands::feed::SubscribeFeed;
use crate::application::commands::playlist::CreatePlaylist;
use crate::application::CommandContext;
use crate::db;
use crate::feed_service;
use crate::library::{playlist_options, LibraryApp};
use crate::library_service;
use crate::metadata::TrackContext;
use crate::presentation::present_command;
use crate::subscribe_service::{SubscribeFeedRequest, SubscribeTrackRequest};
use crate::ui::composites::{
    action_button, ActionButtonDisplay, AddToPlaylistDisplay, AddToPlaylistPopover,
    DisclosureTextPanel, DisclosureTextPanelDisplay, ReleaseSurfaceElement,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button as UiButton;
use crate::ui::shells::entity::{
    render_release_track_row, ReleaseDetailBehaviorSlots, ReleaseTrackRowSlot,
};
use crate::ui::shells::search_results_inspector::{
    render_index_detail_display, render_index_feed_detail, render_index_track_detail,
};
use crate::ui::shells::track::TrackDetailBehaviorSlots;
use crate::view_models::entity_detail::{EntitySurfaceContext, SharedTrackRowVm};
use crate::view_models::recent_feeds::{
    RecentFeedsPageBatch, RecentFeedsPageVm, RecentFeedsViewMode,
};
use crate::view_models::search_results::{
    ArtistResultDisplay, FeedResultDisplay, IndexSearchResultRows, SearchResultItemId,
    SearchResultOrigin, SearchResultsInspectorPageVm, SearchResultsTab, TrackResultDisplay,
};
use crate::view_models::workspace::{FrameNavigationEntry, FrameNavigationState, WorkspaceFrameId};
use crate::views::{FeedRef, FeedView, TrackRef, TrackView};

use super::TopApp;

#[derive(Clone)]
pub(super) enum RemoteDetailThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
}

impl TopApp {
    pub(super) fn submit_global_search(&mut self, cx: &mut Context<Self>) {
        let query = self.global_search_input.read(cx).value().to_string();
        self.open_search_results_in_content_list(&query, cx);
    }

    pub(super) fn open_search_results_in_content_list(
        &mut self,
        query: &str,
        cx: &mut Context<Self>,
    ) {
        let query = query.trim().to_string();
        if query.is_empty() {
            return;
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

    pub(super) fn open_recent_feeds_in_content_list(&mut self, cx: &mut Context<Self>) {
        let Some(content_list_id) = self.content_list_frame_id() else {
            self.settings_status = "ContentList frame not found".to_string();
            cx.notify();
            return;
        };

        let navigation_result = match self.workspace_layout.frame_nav_mut(content_list_id) {
            Some(nav) if matches!(nav.current(), FrameNavigationEntry::RecentFeeds) => {
                nav.replace_current(FrameNavigationEntry::RecentFeeds);
                Ok(())
            }
            Some(_) => self
                .workspace_layout
                .push_nav(content_list_id, FrameNavigationEntry::RecentFeeds),
            None => Err(
                crate::view_models::workspace::WorkspaceModelError::FrameNotFound(content_list_id),
            ),
        };

        match navigation_result.and_then(|()| self.workspace_layout.focus_frame(content_list_id)) {
            Ok(()) => {
                let view_mode = self
                    .recent_feeds_detail
                    .as_ref()
                    .map_or(RecentFeedsViewMode::Tiles, RecentFeedsPageVm::view_mode);
                self.recent_feeds_detail =
                    Some(RecentFeedsPageVm::loading().with_view_mode(view_mode));
                self.start_recent_feeds_load(false, cx);
                cx.notify();
            }
            Err(error) => {
                self.settings_status = format!("Error opening Recent Feeds: {error}");
                cx.notify();
            }
        }
    }

    pub(super) fn start_recent_feeds_load(&mut self, append: bool, cx: &mut Context<Self>) {
        if self.recent_feeds_detail.is_none() {
            self.recent_feeds_detail = Some(RecentFeedsPageVm::loading());
        }

        let loaded_row_count = self
            .recent_feeds_detail
            .as_ref()
            .map_or(0, RecentFeedsPageVm::row_count);
        let Some(intent) = self
            .recent_feeds_detail
            .as_mut()
            .and_then(|detail| detail.begin_load(append))
        else {
            return;
        };

        let endpoint = self.endpoint_input.read(cx).value().to_string();
        let cursor = intent.into_cursor();
        cx.notify();
        cx.spawn(
            async move |this: gpui::WeakEntity<TopApp>, cx: &mut gpui::AsyncApp| {
                let recent_feeds_result = cx
                    .background_executor()
                    .spawn(async move {
                        fetch_recent_feed_result_rows(
                            &endpoint,
                            cursor.as_deref(),
                            if append { loaded_row_count } else { 0 },
                        )
                    })
                    .await;

                this.update(cx, move |this: &mut TopApp, cx: &mut Context<TopApp>| {
                    if !this.content_list_nav_is_recent_feeds() {
                        return;
                    }

                    if this.recent_feeds_detail.is_none() {
                        this.recent_feeds_detail = Some(RecentFeedsPageVm::loading());
                    }

                    if let Some(detail) = this.recent_feeds_detail.as_mut() {
                        let mut should_eager_prefetch = false;
                        match recent_feeds_result {
                            Ok(batch) => {
                                detail.finish_load(batch, append);
                                should_eager_prefetch = !append && detail.has_more();
                            }
                            Err(error) => {
                                detail.fail_load(
                                    "Recent Feeds unavailable",
                                    format!("{error:#}"),
                                    append,
                                );
                            }
                        }
                        cx.notify();
                        if should_eager_prefetch {
                            this.start_recent_feeds_load(true, cx);
                        }
                    }
                })
                .ok();
            },
        )
        .detach();
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
        if let Some(detail) = &mut self.search_results_detail {
            if detail.query() == query {
                detail.mark_index_loading();
            }
        }

        let endpoint = self.endpoint_input.read(cx).value().to_string();
        let request_query = query.to_string();
        let update_query = request_query.clone();
        cx.spawn(
            async move |this: gpui::WeakEntity<TopApp>, cx: &mut gpui::AsyncApp| {
                let search_result = cx
                    .background_executor()
                    .spawn(async move { fetch_index_search_result_rows(&endpoint, &request_query) })
                    .await;

                this.update(cx, move |this: &mut TopApp, cx: &mut Context<TopApp>| {
                    if !this.content_list_nav_matches_search(&update_query) {
                        return;
                    }

                    if this.search_results_detail.is_none() {
                        this.search_results_detail =
                            Some(this.search_results_detail_for_query(&update_query));
                    }

                    if let Some(detail) = this
                        .search_results_detail
                        .as_mut()
                        .filter(|detail| detail.query() == update_query)
                    {
                        match search_result {
                            Ok(rows) => detail.replace_index_results(rows),
                            Err(error) => detail
                                .set_index_error("Index search unavailable", format!("{error:#}")),
                        }
                        cx.notify();
                    }
                })
                .ok();
            },
        )
        .detach();
    }

    fn content_list_nav_matches_search(&self, query: &str) -> bool {
        self.content_list_frame_id()
            .and_then(|content_list_id| self.workspace_layout.frame_nav(content_list_id))
            .and_then(FrameNavigationState::active_search_query)
            .is_some_and(|current| current == query)
    }

    fn content_list_nav_is_recent_feeds(&self) -> bool {
        self.content_list_frame_id()
            .and_then(|content_list_id| self.workspace_layout.frame_nav(content_list_id))
            .is_some_and(|nav| matches!(nav.current(), FrameNavigationEntry::RecentFeeds))
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
                        cx.notify();
                    } else {
                        self.settings_status = format!("Feed {feed_id} not found");
                        cx.notify();
                    }
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

    pub(super) fn handle_recent_feed_selected(&mut self, result_id: &str, cx: &mut Context<Self>) {
        let Some(content_frame_id) = self.content_list_frame_id() else {
            self.settings_status = "ContentList frame not found".to_string();
            cx.notify();
            return;
        };

        let Some(feed_guid) = result_id.strip_prefix("index-feed:") else {
            self.settings_status = format!("Unexpected Recent Feeds id format: {result_id}");
            cx.notify();
            return;
        };

        let label = self
            .recent_feeds_detail
            .as_ref()
            .and_then(|detail| detail.index_feed_label(result_id))
            .unwrap_or_else(|| feed_guid.to_string());
        self.push_index_feed_detail(content_frame_id, feed_guid, label, cx);
    }

    fn handle_index_artist_result_selected(
        &mut self,
        artist_name: &str,
        content_frame_id: WorkspaceFrameId,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.workspace_layout.push_nav(
            content_frame_id,
            FrameNavigationEntry::IndexArtistFeedScope(artist_name.to_string()),
        ) {
            self.settings_status = format!("Failed to navigate to index artist: {error}");
        }
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
        let (_feed_guid, track_guid) = target
            .split_once(':')
            .map_or((None, target), |(feed_guid, track_guid)| {
                (Some(feed_guid), track_guid)
            });
        let label = self
            .search_results_detail
            .as_ref()
            .and_then(|detail| detail.index_track_label(&activation_id))
            .unwrap_or_else(|| track_guid.to_string());
        self.push_index_track_detail(content_frame_id, target, label, cx);
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
        TrackDetailBehaviorSlots {
            hero_image: self.index_track_hero_image(track, cx),
            ..TrackDetailBehaviorSlots::default()
        }
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
        if let Some(state) = self.remote_detail_thumbnails.get(url) {
            return match state {
                RemoteDetailThumbnailState::Loading => None,
                RemoteDetailThumbnailState::Loaded(image) => image.clone(),
            };
        }

        let url = url.to_string();
        self.remote_detail_thumbnails
            .insert(url.clone(), RemoteDetailThumbnailState::Loading);
        let cache = Arc::clone(&self.image_cache);
        cx.spawn(
            async move |this: gpui::WeakEntity<TopApp>, cx: &mut gpui::AsyncApp| {
                let fetch_url = url.clone();
                let image = cx
                    .background_executor()
                    .spawn(async move { cache.fetch_static_blocking(&fetch_url) })
                    .await;
                this.update(cx, move |this: &mut TopApp, cx: &mut Context<TopApp>| {
                    this.remote_detail_thumbnails
                        .insert(url, RemoteDetailThumbnailState::Loaded(image));
                    cx.notify();
                })
                .ok();
            },
        )
        .detach();

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

        vec![
            ReleaseSurfaceElement::from_element(download.into_any_element()),
            ReleaseSurfaceElement::from_element(musicbrainz.into_any_element()),
            ReleaseSurfaceElement::from_element(playlist.into_any_element()),
        ]
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
                musicindex_endpoint: self.endpoint_input.read(cx).value().to_string(),
            },
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.settings_status = result.message().to_string();
                this.reload_cached();
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
                track_context: Box::new(TrackContext {
                    track: api_track_from_view(feed, track),
                    feed: Some(api_feed_from_view(feed)),
                }),
                edits: Vec::new(),
                musicindex_endpoint: self.endpoint_input.read(cx).value().to_string(),
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
                this.reload_cached();
                this.library.update(cx, LibraryApp::refresh);
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
            &self.endpoint_input.read(cx).value(),
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
            &self.endpoint_input.read(cx).value(),
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
                api_track.feed_url.as_deref(),
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
                this.reload_cached();
                this.library.update(cx, LibraryApp::refresh);
            },
            |this, error, _cx| {
                this.settings_status = format!("Error adding to playlist: {error:#}");
            },
        );
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

fn index_track_artwork_url(track: &TrackView) -> Option<&str> {
    non_empty_str(track.image_url.as_deref())
}

fn index_track_row_artwork_url<'a>(feed: &'a FeedView, track: &'a TrackView) -> Option<&'a str> {
    index_track_artwork_url(track).or_else(|| index_feed_artwork_url(feed))
}

fn api_feed_from_view(feed: &FeedView) -> crate::api::Feed {
    crate::api::Feed {
        feed_guid: feed_guid_from_view(feed),
        title: feed.title.clone(),
        name: feed.title.clone(),
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
        feed_url: track.feed_url.clone().or_else(|| feed.feed_url.clone()),
        title: track.title.clone(),
        name: track.title.clone(),
        duration_secs: track.duration_secs,
        pub_date: track.pub_date,
        track_number: track.track_number,
        explicit: track.explicit,
        description: track.description.clone(),
        enclosure_url: track.audio_url.clone(),
        enclosure_type: track.mime.clone(),
        enclosure_bytes: track.bytes,
        image_url: track.image_url.clone().or_else(|| feed.image_url.clone()),
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
}

fn fetch_index_search_result_rows(endpoint: &str, query: &str) -> Result<IndexSearchResultRows> {
    let client = crate::api::Client::new_with_base_url(endpoint.to_string());
    let mut rows = IndexSearchResultRows::default();
    let mut artists = BTreeMap::new();

    let feed_rows = fetch_index_feed_result_rows(&client, query);
    let track_rows = fetch_index_track_result_rows(&client, query);

    match (feed_rows, track_rows) {
        (Ok(feeds), Ok(tracks)) => {
            rows.feeds = feeds.rows;
            rows.tracks = tracks.rows;
            merge_index_artist_candidates(&mut artists, feeds.artists);
            merge_index_artist_candidates(&mut artists, tracks.artists);
        }
        (Ok(feeds), Err(_track_error)) => {
            rows.feeds = feeds.rows;
            merge_index_artist_candidates(&mut artists, feeds.artists);
        }
        (Err(_feed_error), Ok(tracks)) => {
            rows.tracks = tracks.rows;
            merge_index_artist_candidates(&mut artists, tracks.artists);
        }
        (Err(feed_error), Err(track_error)) => {
            return Err(anyhow!(
                "feed search failed: {feed_error}; track search failed: {track_error}"
            ));
        }
    }

    rows.artists = artists
        .into_values()
        .enumerate()
        .map(|(index, artist)| {
            (
                index_item_id(INDEX_ARTIST_ID_BASE, index),
                artist.into_display(),
            )
        })
        .collect();
    Ok(rows)
}

fn fetch_recent_feed_result_rows(
    endpoint: &str,
    cursor: Option<&str>,
    start_index: usize,
) -> Result<RecentFeedsPageBatch> {
    let client = crate::api::Client::new_with_base_url(endpoint.to_string());
    let response = client.fetch_recent_feeds(Some(crate::api::PAGE_LIMIT), cursor)?;
    let rows = response
        .data
        .into_iter()
        .enumerate()
        .map(|(index, feed)| {
            let row_index = start_index + index;
            let feed_guid = recent_feed_activation_id(&feed, row_index);
            let detail = feed
                .feed_guid
                .as_deref()
                .and_then(|guid| {
                    client
                        .fetch_feed(guid, Some(INDEX_FEED_DETAIL_INCLUDE))
                        .ok()
                })
                .unwrap_or(feed);
            (
                index_item_id(INDEX_FEED_ID_BASE, row_index),
                index_feed_display(&feed_guid, Some(crate::api::EntityDetail::Feed(detail))),
            )
        })
        .collect();

    Ok(RecentFeedsPageBatch {
        rows,
        cursor: response.pagination.cursor,
        has_more: response.pagination.has_more,
    })
}

fn recent_feed_activation_id(feed: &crate::api::Feed, index: usize) -> String {
    [
        feed.feed_guid.as_deref(),
        feed.feed_url.as_deref(),
        feed.title.as_deref(),
        feed.name.as_deref(),
    ]
    .into_iter()
    .find_map(non_empty_str)
    .map_or_else(|| format!("recent-feed-{index}"), str::to_string)
}

struct IndexFeedSearchRows {
    rows: Vec<(SearchResultItemId, FeedResultDisplay)>,
    artists: Vec<IndexArtistCandidate>,
}

struct IndexTrackSearchRows {
    rows: Vec<(SearchResultItemId, TrackResultDisplay)>,
    artists: Vec<IndexArtistCandidate>,
}

#[derive(Clone, Debug)]
struct IndexArtistCandidate {
    name: String,
    feed_count: i32,
    track_count: i32,
    thumbnail_href: Option<String>,
}

impl IndexArtistCandidate {
    fn new(
        name: impl Into<String>,
        feed_count: i32,
        track_count: i32,
        thumbnail_href: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            feed_count,
            track_count,
            thumbnail_href,
        }
    }

    fn merge(&mut self, other: Self) {
        self.feed_count = self.feed_count.saturating_add(other.feed_count);
        self.track_count = self.track_count.saturating_add(other.track_count);
        if self.thumbnail_href.is_none() {
            self.thumbnail_href = other.thumbnail_href;
        }
    }

    fn into_display(self) -> ArtistResultDisplay {
        let mut display = ArtistResultDisplay::new(
            format!("index-artist:{}", self.name),
            self.name,
            SearchResultOrigin::Index,
        );
        let secondary = count_parts([
            positive_count_label(self.feed_count, "feed"),
            positive_count_label(self.track_count, "track"),
        ]);
        if !secondary.is_empty() {
            display = display.with_secondary_text(secondary);
        }
        if let Some(thumbnail_href) = self.thumbnail_href {
            display = display.with_thumbnail_href(thumbnail_href);
        }
        display
    }
}

fn fetch_index_feed_result_rows(
    client: &crate::api::Client,
    query: &str,
) -> Result<IndexFeedSearchRows> {
    let response = client.search(
        query,
        Some("feed"),
        Some(crate::api::PAGE_LIMIT),
        None,
        true,
    )?;
    let mut rows = Vec::new();
    let mut artists = Vec::new();

    for (index, hit) in response.data.iter().enumerate() {
        let feed_guid = hit.feed_guid.as_deref().unwrap_or(&hit.entity_id);
        let detail = client
            .fetch_feed(feed_guid, Some(INDEX_FEED_DETAIL_INCLUDE))
            .ok();
        if let Some(feed) = detail.as_ref() {
            if let Some(candidate) = index_artist_candidate_from_feed(feed, query) {
                artists.push(candidate);
            }
        }
        rows.push((
            index_item_id(INDEX_FEED_ID_BASE, index),
            index_feed_display(feed_guid, detail.map(crate::api::EntityDetail::Feed)),
        ));
    }

    Ok(IndexFeedSearchRows { rows, artists })
}

fn fetch_index_track_result_rows(
    client: &crate::api::Client,
    query: &str,
) -> Result<IndexTrackSearchRows> {
    let response = client.search(
        query,
        Some("track"),
        Some(crate::api::PAGE_LIMIT),
        None,
        true,
    )?;
    let mut rows = Vec::new();
    let mut artists = Vec::new();

    for (index, hit) in response.data.iter().enumerate() {
        let detail =
            fetch_index_track_detail(client, &hit.entity_id, hit.feed_guid.as_deref()).ok();
        if let Some(track) = detail.as_ref() {
            artists.extend(index_artist_candidates_from_track(track, query));
        }
        let feed_guid = hit
            .feed_guid
            .as_deref()
            .or_else(|| detail.as_ref().and_then(|track| track.feed_guid.as_deref()))
            .map(str::to_string);
        rows.push((
            index_item_id(INDEX_TRACK_ID_BASE, index),
            index_track_display(
                &hit.entity_id,
                feed_guid.as_deref(),
                detail.map(crate::api::EntityDetail::Track),
            ),
        ));
    }

    Ok(IndexTrackSearchRows { rows, artists })
}

fn index_artist_candidate_from_feed(
    feed: &crate::api::Feed,
    query: &str,
) -> Option<IndexArtistCandidate> {
    let name = non_empty_str(feed.release_artist.as_deref())?;
    index_artist_name_matches_query(name, query).then(|| {
        IndexArtistCandidate::new(
            name,
            1,
            feed.episode_count.unwrap_or_default().max(0),
            non_empty_str(feed.image_url.as_deref()).map(str::to_string),
        )
    })
}

fn index_artist_candidates_from_track(
    track: &crate::api::Track,
    query: &str,
) -> Vec<IndexArtistCandidate> {
    [
        track.track_artist.as_deref(),
        track.release_artist.as_deref(),
    ]
    .into_iter()
    .filter_map(non_empty_str)
    .collect::<BTreeSet<_>>()
    .into_iter()
    .filter(|name| index_artist_name_matches_query(name, query))
    .map(|name| {
        IndexArtistCandidate::new(
            name,
            0,
            1,
            non_empty_str(track.image_url.as_deref()).map(str::to_string),
        )
    })
    .collect()
}

fn merge_index_artist_candidates(
    artists: &mut BTreeMap<String, IndexArtistCandidate>,
    candidates: Vec<IndexArtistCandidate>,
) {
    for candidate in candidates {
        let key = candidate.name.to_lowercase();
        if let Some(existing) = artists.get_mut(&key) {
            existing.merge(candidate);
        } else {
            artists.insert(key, candidate);
        }
    }
}

const INDEX_ARTIST_ID_BASE: SearchResultItemId = 1_000_000_000;
const INDEX_FEED_ID_BASE: SearchResultItemId = 2_000_000_000;
const INDEX_TRACK_ID_BASE: SearchResultItemId = 3_000_000_000;
const INDEX_FEED_DETAIL_INCLUDE: &str = "tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes";

fn index_item_id(base: SearchResultItemId, index: usize) -> SearchResultItemId {
    let offset = u64::try_from(index).unwrap_or(SearchResultItemId::MAX.saturating_sub(base));
    base.saturating_add(offset)
}

fn fetch_index_track_detail(
    client: &crate::api::Client,
    track_guid: &str,
    feed_guid: Option<&str>,
) -> Result<crate::api::Track> {
    match feed_guid {
        Some(feed_guid) if !feed_guid.trim().is_empty() => {
            client.fetch_feed_track(feed_guid, track_guid, None)
        }
        _ => client.fetch_track(track_guid, None),
    }
}

fn index_feed_display(
    feed_guid: &str,
    detail: Option<crate::api::EntityDetail>,
) -> FeedResultDisplay {
    let mut display = FeedResultDisplay::new(
        format!("index-feed:{feed_guid}"),
        feed_guid,
        SearchResultOrigin::Index,
    );

    if let Some(crate::api::EntityDetail::Feed(feed)) = detail {
        let remote_feed = crate::views::FeedView::from_api(feed.clone());
        let label = feed
            .title
            .or(feed.name)
            .or(feed.feed_guid)
            .unwrap_or_else(|| feed_guid.to_string());
        display = FeedResultDisplay::new(
            format!("index-feed:{feed_guid}"),
            label,
            SearchResultOrigin::Index,
        );

        let secondary = count_parts([
            feed.release_artist,
            feed.episode_count.map(|count| count_label(count, "track")),
            feed.publisher_text,
        ]);
        if !secondary.is_empty() {
            display = display.with_secondary_text(secondary);
        }
        if let Some(image_url) = non_empty_string(feed.image_url) {
            display = display.with_thumbnail_href(image_url);
        }
        display = display.with_remote_feed(remote_feed);
    }

    display
}

fn index_track_display(
    track_guid: &str,
    feed_guid: Option<&str>,
    detail: Option<crate::api::EntityDetail>,
) -> TrackResultDisplay {
    let activation_id = feed_guid.map_or_else(
        || format!("index-track:{track_guid}"),
        |feed_guid| format!("index-track:{feed_guid}:{track_guid}"),
    );
    let mut display =
        TrackResultDisplay::new(activation_id.clone(), track_guid, SearchResultOrigin::Index);

    if let Some(crate::api::EntityDetail::Track(track)) = detail {
        let remote_track = TrackView::from_api(track.clone());
        let label = track
            .title
            .or(track.name)
            .unwrap_or_else(|| track_guid.to_string());
        display = TrackResultDisplay::new(activation_id, label, SearchResultOrigin::Index);

        let secondary = count_parts([track.track_artist, track.release_artist, track.feed_title]);
        if !secondary.is_empty() {
            display = display.with_secondary_text(secondary);
        }
        if let Some(image_url) = non_empty_string(track.image_url) {
            display = display.with_thumbnail_href(image_url);
        }
        display = display.with_remote_track(remote_track);
    }

    display
}

fn count_label(count: i32, singular: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {singular}s")
    }
}

fn positive_count_label(count: i32, singular: &str) -> Option<String> {
    (count > 0).then(|| count_label(count, singular))
}

fn count_parts<const N: usize>(parts: [Option<String>; N]) -> String {
    parts
        .into_iter()
        .filter_map(non_empty_string)
        .collect::<Vec<_>>()
        .join(" - ")
}

fn index_artist_name_matches_query(name: &str, query: &str) -> bool {
    let normalized_name = name.to_lowercase();
    query
        .split_whitespace()
        .map(str::to_lowercase)
        .all(|term| normalized_name.contains(&term))
}

fn non_empty_str(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn non_empty_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Contributor, EntityDetail, SourceEntityId, SourceEntityLink, Track};

    #[test]
    fn index_track_display_attaches_fetched_track_view() {
        let display = index_track_display(
            "track-guid",
            Some("feed-guid"),
            Some(EntityDetail::Track(Track {
                track_guid: Some("track-guid".to_string()),
                feed_guid: Some("feed-guid".to_string()),
                feed_title: Some("Remote Release".to_string()),
                title: Some("Remote Track".to_string()),
                duration_secs: Some(125),
                pub_date: Some(1_712_275_200),
                track_number: Some(7),
                explicit: Some(true),
                image_url: Some("https://example.test/track.jpg".to_string()),
                track_artist: Some("Track Artist".to_string()),
                source_contributors: Some(vec![Contributor {
                    name: Some("Contributor".to_string()),
                    role: Some("producer".to_string()),
                    ..Contributor::default()
                }]),
                source_links: Some(vec![SourceEntityLink {
                    link_type: Some("transcript".to_string()),
                    url: Some("https://example.test/transcript.srt".to_string()),
                    ..SourceEntityLink::default()
                }]),
                source_ids: Some(vec![SourceEntityId {
                    scheme: Some("nostr_npub".to_string()),
                    value: Some("npub1track".to_string()),
                    ..SourceEntityId::default()
                }]),
                ..Track::default()
            })),
        );

        assert_eq!(display.label, "Remote Track");
        assert_eq!(display.secondary_text, "Track Artist - Remote Release");
        assert_eq!(
            display.thumbnail_href.as_deref(),
            Some("https://example.test/track.jpg")
        );

        let track = display
            .remote_track
            .as_ref()
            .expect("fetched Index detail should attach a TrackView to the result row");
        assert_eq!(track.title.as_deref(), Some("Remote Track"));
        assert_eq!(track.feed_title.as_deref(), Some("Remote Release"));
        assert_eq!(track.track_number, Some(7));
        assert_eq!(track.duration_secs, Some(125));
        assert_eq!(track.pub_date, Some(1_712_275_200));
        assert_eq!(track.explicit, Some(true));
        assert_eq!(track.identity.nostr_npub.as_deref(), Some("npub1track"));
        assert_eq!(track.contributors.len(), 1);
        assert_eq!(
            track.transcript_url.as_deref(),
            Some("https://example.test/transcript.srt")
        );
    }
}
