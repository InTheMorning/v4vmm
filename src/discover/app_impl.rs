use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use gpui::{
    div, prelude::*, px, Context, Image, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Render, ScrollHandle, Styled, Window,
};
use rusqlite::Connection;

use crate::api::*;
use crate::application::commands::download::{SubscribeThenAppendToPlaylist, SubscribeTrack};
use crate::application::commands::feed::SubscribeFeed;
use crate::application::commands::library_removal::RemoveFromLibrary;
use crate::application::commands::playlist::CreatePlaylist;
use crate::application::library_removal::{LibraryRemovalIntent, LibraryRemovalTarget};
use crate::application::{ApplicationServices, CommandContext};
use crate::audio_tags::{write_id3v24_edits, Id3v24Edit};
use crate::db;
use crate::feed_service;
use crate::identity_ingest;
use crate::library_service;
use crate::media::{image_from_bytes, ImageCache};
use crate::metadata::*;
use crate::presentation::GpuiCommandRunner;
use crate::rss;
use crate::subscribe_service::{
    self, compare_downloaded_track_path, download_image, enrich_track_context_from_rss,
    SubscribeTrackRequest,
};
use crate::ui::composites::{SplitPane, StatusRole};
use crate::ui::detail_row::DetailRow;
use crate::ui::layouts as layout;
use crate::ui::primitives::MultilineText;
use crate::ui::shells::discover::feed_inspector::render_inspector;
use crate::ui::shells::discover::result_list::{
    render_discover_result_list, DiscoverResultEmptyState, DiscoverResultListParams,
    DiscoverResultPagination, DiscoverResultRow, DiscoverResultSection,
};
use crate::ui::shells::discover::search_input::{
    render_discover_search_controls, DiscoverSearchControlsParams,
};
use crate::ui::shells::discover::track_inspector_metadata::track_metadata_rows_for_frame;
use crate::ui::shells::library_removal_confirmation::open_library_removal_confirmation_dialog;
use crate::ui::style::{color, typography};
use crate::ui::tokens::FontSize;
use crate::view_models::entity_detail::TrackMetadataActionState;
use crate::view_models::search::{
    artist_rows_from_result_rows, normalized_search_query, search_result_type_is_visible,
    DeferredPanelKind, LazyPanel, PlaylistAppendIntent, PlaylistAppendOutcome, ResultRow,
    SearchBatch, SearchLibraryMembership, SearchLibraryMembershipDisplay, SearchRemovalOrigin,
    SearchResultSource, SearchSubscriptionCommand, SearchViewModel, TrackRowActionVm,
};
use crate::view_models::track::TrackVm;
use crate::view_models::workspace::ContentFilter;
use crate::views::ContributorView;

use super::{
    ArtistContext, InspectorDetail, InspectorFrame, SearchApp, SearchAppEvent, ThumbnailState,
};

impl SearchApp {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        cache: Arc<ImageCache>,
        musicindex_endpoint: String,
        application_services: Arc<ApplicationServices>,
        #[cfg(feature = "async-runtime")] runtime_host: Option<
            Arc<crate::presentation::RuntimeHost>,
        >,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let _ = window;
        let command_runner = GpuiCommandRunner::new(
            application_services.command_bus(),
            application_services.event_bus(),
        );

        let mut this = Self {
            conn,
            application_services,
            command_runner,
            cache,
            musicindex_endpoint,
            vm: SearchViewModel::new(),
            inspector_stack: Vec::new(),
            thumbnails: BTreeMap::new(),
            list_focus: cx.focus_handle(),
            results_scroll: ScrollHandle::new(),
            recents_scroll: ScrollHandle::new(),
            #[cfg(feature = "async-runtime")]
            runtime_host,
        };
        this.load_playlists();
        this.load_recent_feeds(false, cx);
        this
    }

    pub fn set_musicindex_endpoint(&mut self, endpoint: String, cx: &mut Context<Self>) {
        if self.musicindex_endpoint == endpoint {
            return;
        }

        self.musicindex_endpoint = endpoint;
        self.vm.reset_for_endpoint_change();
        self.inspector_stack.clear();
        self.load_recent_feeds(false, cx);
        cx.notify();
    }

    pub(crate) fn load_recent_feeds(&mut self, append: bool, cx: &mut Context<Self>) {
        let Some(intent) = self.vm.begin_recent_feed_load(append) else {
            return;
        };
        cx.notify();

        let client = self.api_client();
        let cursor = intent.into_cursor();
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let result =
                    cx.background_executor()
                        .spawn(async move {
                            client.fetch_recent_feeds(Some(PAGE_LIMIT), cursor.as_deref())
                        })
                        .await;
                let _ = this.update(cx, move |this, cx| {
                    let mut should_eager_prefetch = false;
                    match result {
                        Ok(response) => {
                            this.vm.finish_recent_feed_load(response);
                            // After the first page lands, eagerly request the
                            // next one in the background so the user's first
                            // scroll-to-bottom never pauses on a network round
                            // trip. `begin_recent_feed_load(true)` is
                            // idempotent against in-flight loads.
                            if !append && this.vm.recent_has_more {
                                should_eager_prefetch = true;
                            }
                        }
                        Err(error) => {
                            this.vm.fail_recent_feed_load(error);
                        }
                    }
                    cx.notify();
                    if should_eager_prefetch {
                        this.load_recent_feeds(true, cx);
                    }
                });
            },
        )
        .detach();
    }

    fn api_client(&self) -> Arc<Client> {
        Arc::new(Client::new_with_base_url(self.musicindex_endpoint.clone()))
    }

    pub fn pop_inspector(&mut self, cx: &mut Context<Self>) {
        if !self.inspector_stack.is_empty() {
            self.inspector_stack.pop();
            cx.notify();
        }
    }

    pub fn move_up(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.vm.previous_result_target() {
            let (source, entity_type, entity_id, feed_guid, title) = target.into_parts();
            self.select_result(source, entity_type, entity_id, feed_guid, title, cx);
        }
    }

    pub fn move_down(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.vm.next_result_target() {
            let (source, entity_type, entity_id, feed_guid, title) = target.into_parts();
            self.select_result(source, entity_type, entity_id, feed_guid, title, cx);
        }
    }

    pub fn confirm(&mut self, _cx: &mut Context<Self>) {
        // In search, confirm might mean "open details" which select_result already does.
        // If we want to focus the inspector, we could do that.
    }

    pub(crate) fn do_search(&mut self, append: bool, cx: &mut Context<Self>) {
        let Some(query) = self.vm.active_query.clone() else {
            return;
        };
        self.run_global_search_page(query, self.vm.active_filter, append, cx);
    }

    pub(crate) fn run_global_search(&mut self, query: impl Into<String>, cx: &mut Context<Self>) {
        let query = query.into();
        if normalized_search_query(&query).is_none() {
            let should_load = self.vm.return_to_recent_feeds();
            self.inspector_stack.clear();
            cx.notify();
            if should_load {
                self.load_recent_feeds(false, cx);
            }
            return;
        }
        self.run_global_search_page(query, ContentFilter::All, false, cx);
    }

    pub(crate) fn rerun_global_search_with_active_filter(
        &mut self,
        query: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.run_global_search_page(query.into(), self.vm.active_filter, false, cx);
    }

    fn run_global_search_page(
        &mut self,
        query: String,
        filter: ContentFilter,
        append: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(intent) = self
            .vm
            .begin_global_search_load(query.clone(), filter, append)
        else {
            return;
        };
        if !append {
            self.inspector_stack.clear();
        }
        cx.notify();

        let entity_type = if filter == ContentFilter::Library {
            None
        } else {
            SearchViewModel::type_filter_value(intent.type_filter()).map(str::to_string)
        };
        let cursor = intent.cursor().map(str::to_string);
        let fuzzy = intent.fuzzy();
        let client = self.api_client();
        let conn = Arc::clone(&self.conn);
        let query_service = self.application_services.query_service();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let search_result = cx
                    .background_executor()
                    .spawn(async move {
                        let library_rows = if !append
                            && matches!(filter, ContentFilter::All | ContentFilter::Library)
                        {
                            fetch_local_library_search_rows(&conn, &query_service, &query)?
                        } else {
                            Vec::new()
                        };
                        let index_batch =
                            if matches!(filter, ContentFilter::All | ContentFilter::Index) {
                                Some(fetch_search_batch(
                                    &client,
                                    &query,
                                    entity_type.as_deref(),
                                    cursor.as_deref(),
                                    fuzzy,
                                )?)
                            } else {
                                None
                            };
                        Ok::<_, anyhow::Error>((library_rows, index_batch))
                    })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        match search_result {
                            Ok((library_rows, index_batch)) => {
                                if let Some(batch) = index_batch.as_ref() {
                                    if let Err(error) =
                                        persist_musicindex_artist_facts(&this.conn, batch)
                                    {
                                        this.vm.fail_search_load(error);
                                        cx.notify();
                                        return;
                                    }
                                }
                                this.vm.finish_global_search_load(
                                    library_rows,
                                    index_batch,
                                    filter,
                                    append,
                                );
                            }
                            Err(error) => this.vm.fail_search_load(error),
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    pub(crate) fn toggle_fuzzy_search(&mut self, cx: &mut Context<Self>) {
        self.vm.toggle_fuzzy_search();
        let query = self.vm.active_query.clone();
        cx.notify();
        if let Some(query) = query {
            self.run_global_search_page(query, self.vm.active_filter, false, cx);
        }
    }

    pub(crate) fn show_recent_feeds(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = window;
        self.inspector_stack.clear();
        let should_load = self.vm.return_to_recent_feeds();
        cx.notify();
        if should_load {
            self.load_recent_feeds(false, cx);
        }
    }

    pub(crate) fn thumbnail_for_url(
        &mut self,
        url: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Image>> {
        let url = url?.trim();
        if url.is_empty() {
            return None;
        }

        // Fast path: check hot cache first
        if let Some(image) = self.cache.peek(url) {
            return Some(image);
        }

        match self.thumbnails.get(url) {
            Some(ThumbnailState::Loaded(image)) => return image.clone(),
            Some(ThumbnailState::Loading) => return None,
            None => {}
        }

        self.thumbnails
            .insert(url.to_string(), ThumbnailState::Loading);
        let url = url.to_string();
        let cache = Arc::clone(&self.cache);
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let cache_url = url.clone();
                let cache_clone = Arc::clone(&cache);
                let image = cx
                    .background_executor()
                    .spawn(async move { cache_clone.fetch_blocking(&cache_url) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        this.thumbnails.insert(url, ThumbnailState::Loaded(image));
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();

        None
    }

    pub(crate) fn select_result(
        &mut self,
        source: SearchResultSource,
        entity_type: String,
        entity_id: String,
        feed_guid: Option<String>,
        title: String,
        cx: &mut Context<Self>,
    ) {
        self.vm
            .select_result_from_source(source, &entity_type, &entity_id, feed_guid.as_deref());
        if source == SearchResultSource::Library && entity_type == "track" {
            self.load_local_track_inspector(entity_id, title, false, cx);
        } else {
            self.load_inspector(entity_type, entity_id, feed_guid, title, false, cx);
        }
    }

    pub(crate) fn open_recent_feed(
        &mut self,
        feed_guid: String,
        title: String,
        cx: &mut Context<Self>,
    ) {
        self.vm.select_recent_feed(&feed_guid);
        self.load_inspector("feed".into(), feed_guid, None, title, false, cx);
    }

    pub(crate) fn push_inspector(
        &mut self,
        entity_type: String,
        entity_id: String,
        title: String,
        cx: &mut Context<Self>,
    ) {
        self.load_inspector(entity_type, entity_id, None, title, true, cx);
    }

    fn load_inspector(
        &mut self,
        entity_type: String,
        entity_id: String,
        feed_guid: Option<String>,
        title: String,
        push: bool,
        cx: &mut Context<Self>,
    ) {
        let frame = InspectorFrame::loading(entity_type.clone(), entity_id.clone(), title);
        if push {
            self.inspector_stack.push(frame);
        } else {
            self.inspector_stack.clear();
            self.inspector_stack.push(frame);
        }
        cx.notify();

        let client = self.api_client();
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let request_type = entity_type.clone();
                let request_id = entity_id.clone();
                let request_feed_guid = feed_guid.clone();
                let detail = cx
                    .background_executor()
                    .spawn(async move {
                        fetch_inspector_detail(
                            &client,
                            &request_type,
                            &request_id,
                            request_feed_guid.as_deref(),
                        )
                    })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        let local_subscription = detail.as_ref().ok().and_then(|(detail, _)| {
                            local_subscription_for_detail(&this.conn, detail)
                                .ok()
                                .flatten()
                        });
                        let mut image_url_to_fetch: Option<String> = None;
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                match detail {
                                    Ok((detail, image_url)) => {
                                        if let InspectorDetail::Artist(ctx) = &detail {
                                            this.vm.merge_artist_result_detail(
                                                &entity_id,
                                                &ctx.artist,
                                            );
                                        }
                                        frame.detail = detail;
                                        frame.image = None;
                                        frame.local_subscription = local_subscription;
                                        frame.subscription_message = None;
                                        if let InspectorDetail::Feed(feed) = &frame.detail {
                                            if let Some(feed_url) =
                                                feed.feed_url.clone().filter(|s| !s.is_empty())
                                            {
                                                frame.podroll = LazyPanel::Loading;
                                                this.load_podroll(entity_id.clone(), feed_url, cx);
                                            }
                                        }
                                        image_url_to_fetch = image_url;
                                    }
                                    Err(error) => {
                                        frame.detail = InspectorDetail::Error(error.to_string());
                                    }
                                }
                            }
                        }
                        cx.notify();
                        if let Some(url) = image_url_to_fetch {
                            this.load_inspector_image(
                                entity_type.clone(),
                                entity_id.clone(),
                                url,
                                cx,
                            );
                        }
                        // Eagerly prefetch the deferred panels in the
                        // background. The user is consuming the populated
                        // text content; by the time they reach for
                        // contributors / value routes the data is already
                        // resident. We do not flip the collapsed flags —
                        // disclosures stay in their default closed state,
                        // they just open instantly when the user clicks.
                        this.prefetch_contributors(entity_type.clone(), entity_id.clone(), cx);
                        this.prefetch_value_routes(entity_type, entity_id, cx);
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn load_local_track_inspector(
        &mut self,
        entity_id: String,
        title: String,
        push: bool,
        cx: &mut Context<Self>,
    ) {
        let track_id = match entity_id.parse::<i64>() {
            Ok(track_id) => track_id,
            Err(error) => {
                self.vm
                    .fail_search_load(format!("Error loading local track: {error}"));
                cx.notify();
                return;
            }
        };
        let frame_entity_id = format!("local:{track_id}");
        let frame = InspectorFrame::loading("track".into(), frame_entity_id.clone(), title);
        if push {
            self.inspector_stack.push(frame);
        } else {
            self.inspector_stack.clear();
            self.inspector_stack.push(frame);
        }
        cx.notify();

        let conn = Arc::clone(&self.conn);
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let detail = cx
                    .background_executor()
                    .spawn(async move {
                        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
                        let Some(track) = library_service::track_row_by_id(&db, track_id)? else {
                            return Err(anyhow!("local track not found: {track_id}"));
                        };
                        let context = feed_service::track_row_to_track_context_with_local_identity(
                            &db, &track,
                        )?;
                        let image_url = context
                            .track
                            .image_url
                            .as_deref()
                            .and_then(|url| nonempty_url(Some(url)))
                            .map(str::to_string);
                        Ok::<_, anyhow::Error>((context, image_url))
                    })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        let mut image_url_to_fetch = None;
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == "track" && frame.entity_id == frame_entity_id {
                                match detail {
                                    Ok((context, image_url)) => {
                                        frame.detail = InspectorDetail::Track(Box::new(context));
                                        frame.image = None;
                                        frame.local_subscription = Some(true);
                                        frame.subscription_message = None;
                                        frame.contributors = LazyPanel::Empty(
                                            SearchViewModel::deferred_panel_display(
                                                DeferredPanelKind::Contributors,
                                            )
                                            .empty_label
                                            .into(),
                                        );
                                        frame.value_routes = LazyPanel::Empty(
                                            SearchViewModel::deferred_panel_display(
                                                DeferredPanelKind::ValueRoutes,
                                            )
                                            .empty_label
                                            .into(),
                                        );
                                        image_url_to_fetch = image_url;
                                    }
                                    Err(error) => {
                                        frame.detail = InspectorDetail::Error(error.to_string());
                                    }
                                }
                            }
                        }
                        cx.notify();
                        if let Some(url) = image_url_to_fetch {
                            this.load_inspector_image("track".into(), frame_entity_id, url, cx);
                        }
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    /// Fetch + decode an inspector hero image as a follow-up to the
    /// structured detail fetch. The text/structured payload is rendered
    /// the moment it lands; the image slots in via a second `cx.notify()`
    /// when the bytes arrive. Keeping image work off the critical path
    /// is the largest perceived-latency win in the discover flow because
    /// HTTP-GET-then-decode for cover art was previously bundled into the
    /// same background task as the structured fetch.
    fn load_inspector_image(
        &mut self,
        entity_type: String,
        entity_id: String,
        image_url: String,
        cx: &mut Context<Self>,
    ) {
        let client = self.api_client();
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let image = cx
                    .background_executor()
                    .spawn(async move {
                        download_image(&client.client, &image_url).map(image_from_bytes)
                    })
                    .await;
                if image.is_none() {
                    return;
                }
                let _ = this.update(cx, move |this, cx| {
                    if let Some(frame) = this.inspector_stack.last_mut() {
                        if frame.entity_type == entity_type
                            && frame.entity_id == entity_id
                            && frame.image.is_none()
                        {
                            frame.image = image;
                            cx.notify();
                        }
                    }
                });
            },
        )
        .detach();
    }

    /// Background-prefetch the contributor list for the inspector frame
    /// matching `entity_type`/`entity_id`. Unlike [`Self::toggle_contributors`]
    /// this never touches `contributors_collapsed` — the disclosure stays in
    /// its default closed state. The point is to have the data resident by
    /// the time the user reaches for it.
    fn prefetch_contributors(
        &mut self,
        entity_type: String,
        entity_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.entity_type != entity_type || frame.entity_id != entity_id {
            return;
        }
        if !matches!(frame.contributors, LazyPanel::Hidden) {
            return;
        }
        frame.contributors = LazyPanel::Loading;
        let client = self.api_client();
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let req_type = entity_type.clone();
                let req_id = entity_id.clone();
                let contributors = cx
                    .background_executor()
                    .spawn(async move { client.fetch_contributors(&req_type, &req_id) })
                    .await;
                let _ = this.update(cx, move |this, cx| {
                    if let Some(frame) = this.inspector_stack.last_mut() {
                        if frame.entity_type == entity_type && frame.entity_id == entity_id {
                            let contributors = contributors.map(|contributors| {
                                contributors
                                    .into_iter()
                                    .map(ContributorView::from)
                                    .collect()
                            });
                            let display = SearchViewModel::deferred_panel_display(
                                DeferredPanelKind::Contributors,
                            );
                            frame.contributors =
                                LazyPanel::from_items_result(contributors, display.empty_label);
                            cx.notify();
                        }
                    }
                });
            },
        )
        .detach();
    }

    /// Background-prefetch the value-routes list. Same contract as
    /// [`Self::prefetch_contributors`]: never expands the disclosure,
    /// only hydrates the panel data.
    fn prefetch_value_routes(
        &mut self,
        entity_type: String,
        entity_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.entity_type != entity_type || frame.entity_id != entity_id {
            return;
        }
        if !matches!(frame.value_routes, LazyPanel::Hidden) {
            return;
        }
        frame.value_routes = LazyPanel::Loading;
        let client = self.api_client();
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let req_type = entity_type.clone();
                let req_id = entity_id.clone();
                let routes = cx
                    .background_executor()
                    .spawn(async move { client.fetch_value_routes(&req_type, &req_id) })
                    .await;
                let _ = this.update(cx, move |this, cx| {
                    if let Some(frame) = this.inspector_stack.last_mut() {
                        if frame.entity_type == entity_type && frame.entity_id == entity_id {
                            let display = SearchViewModel::deferred_panel_display(
                                DeferredPanelKind::ValueRoutes,
                            );
                            frame.value_routes =
                                LazyPanel::from_items_result(routes, display.empty_label);
                            cx.notify();
                        }
                    }
                });
            },
        )
        .detach();
    }

    fn load_podroll(&mut self, feed_guid: String, feed_url: String, cx: &mut Context<Self>) {
        let client = self.api_client();
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { resolve_podroll_feeds(&client, &feed_url) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == "feed" && frame.entity_id == feed_guid {
                                frame.podroll = match result {
                                    Ok(feeds) if feeds.is_empty() => LazyPanel::Hidden,
                                    Ok(feeds) => LazyPanel::Loaded(feeds),
                                    Err(_) => LazyPanel::Hidden,
                                };
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    pub(crate) fn inspector_back(&mut self, cx: &mut Context<Self>) {
        if self.inspector_stack.is_empty() {
            return;
        }
        self.inspector_stack.pop();
        if self.inspector_stack.is_empty() {
            self.vm.clear_inspector_origin();
            self.vm.clear_selection();
        }
        cx.notify();
    }

    pub(crate) fn toggle_contributors(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.is_local_library_track() && matches!(frame.contributors, LazyPanel::Hidden) {
            frame.contributors = LazyPanel::Empty(
                SearchViewModel::deferred_panel_display(DeferredPanelKind::Contributors)
                    .empty_label
                    .into(),
            );
        }

        let action = frame.contributors.begin_collapsible_toggle(
            &mut frame.contributors_collapsed,
            matches!(frame.tag_compare, LazyPanel::Loaded(_)),
        );
        if !action.should_fetch() {
            if action.should_notify() {
                cx.notify();
            }
            return;
        }

        let entity_type = frame.entity_type.clone();
        let entity_id = frame.entity_id.clone();
        let client = self.api_client();
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let contributors = cx
                    .background_executor()
                    .spawn(async move { client.fetch_contributors(&entity_type, &entity_id) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            let contributors = contributors.map(|contributors| {
                                contributors
                                    .into_iter()
                                    .map(ContributorView::from)
                                    .collect()
                            });
                            let display = SearchViewModel::deferred_panel_display(
                                DeferredPanelKind::Contributors,
                            );
                            frame.contributors =
                                LazyPanel::from_items_result(contributors, display.empty_label);
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    pub(crate) fn toggle_value_routes(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.is_local_library_track() && matches!(frame.value_routes, LazyPanel::Hidden) {
            frame.value_routes = LazyPanel::Empty(
                SearchViewModel::deferred_panel_display(DeferredPanelKind::ValueRoutes)
                    .empty_label
                    .into(),
            );
        }

        let action = frame.value_routes.begin_collapsible_toggle(
            &mut frame.value_routes_collapsed,
            matches!(frame.tag_compare, LazyPanel::Loaded(_)),
        );
        if !action.should_fetch() {
            if action.should_notify() {
                cx.notify();
            }
            return;
        }

        let entity_type = frame.entity_type.clone();
        let entity_id = frame.entity_id.clone();
        let client = self.api_client();
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let routes = cx
                    .background_executor()
                    .spawn(async move { client.fetch_value_routes(&entity_type, &entity_id) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            let display = SearchViewModel::deferred_panel_display(
                                DeferredPanelKind::ValueRoutes,
                            );
                            frame.value_routes =
                                LazyPanel::from_items_result(routes, display.empty_label);
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    pub(crate) fn toggle_id3_frame_group(&mut self, group_key: String, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if !frame.expanded_id3_frame_groups.remove(&group_key) {
            frame.expanded_id3_frame_groups.insert(group_key);
        }
        cx.notify();
    }

    pub(crate) fn toggle_metadata_cell(&mut self, cell_key: String, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if !frame.expanded_metadata_cells.remove(&cell_key) {
            frame.expanded_metadata_cells.insert(cell_key);
        }
        cx.notify();
    }

    pub(crate) fn stage_id3_drag_copy(&mut self, drag: &MetadataDragValue, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if !id3v24_drag_copy_frame_is_writable(&drag.frame) {
            return;
        }
        let Some(value) = format_source_value_for_id3v24(
            &drag.frame,
            &drag.field,
            drag.source,
            drag.target_existing_value.as_deref(),
            &drag.value,
        ) else {
            return;
        };
        frame.pending_id3_edits.insert(
            drag.row_id.clone(),
            PendingId3Edit {
                field: drag.field.clone(),
                frame: drag.frame.clone(),
                value,
                source: drag.source,
            },
        );
        frame.suppressed_auto_id3_edits.remove(&drag.row_id);
        frame.id3_apply_error = None;
        cx.notify();
    }

    pub(crate) fn revert_pending_id3_edit(&mut self, row_id: String, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.applying_id3_edits {
            return;
        }
        frame.pending_id3_edits.remove(&row_id);
        frame.suppressed_auto_id3_edits.insert(row_id);
        frame.id3_apply_error = None;
        cx.notify();
    }

    pub(crate) fn apply_pending_id3_edits(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.entity_type != "track" || frame.applying_id3_edits {
            return;
        }
        let LazyPanel::Loaded(result) = &frame.tag_compare else {
            return;
        };
        let track_context = match &frame.detail {
            InspectorDetail::Track(track_context) => (**track_context).clone(),
            InspectorDetail::Loading(_)
            | InspectorDetail::Error(_)
            | InspectorDetail::Artist(_)
            | InspectorDetail::Feed(_)
            | InspectorDetail::Publisher(_) => return,
        };
        let rows = track_metadata_rows_for_frame(frame, &track_context, Some(result));
        let pending_id3_edits = auto_populated_pending_id3_edits(
            &rows,
            &frame.pending_id3_edits,
            &frame.suppressed_auto_id3_edits,
            result.format,
        );
        if pending_id3_edits.is_empty() {
            return;
        }
        let conflicts = pending_id3_conflict_descriptions(&pending_id3_edits);
        if !conflicts.is_empty() {
            frame.id3_apply_error = Some(TrackMetadataActionState::duplicate_id3_target_message(
                &conflicts,
            ));
            cx.notify();
            return;
        }

        let entity_id = frame.entity_id.clone();
        let entity_type = frame.entity_type.clone();
        let path = PathBuf::from(result.path.clone());
        let edits = pending_id3_edits_for_apply(&pending_id3_edits);
        frame.applying_id3_edits = true;
        frame.id3_apply_error = None;
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        write_id3v24_edits(&path, &edits)?;
                        compare_downloaded_track_path(&path, &track_context)
                    })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                frame.applying_id3_edits = false;
                                match result {
                                    Ok(result) => {
                                        frame.tag_compare = LazyPanel::Loaded(result);
                                        frame.pending_id3_edits.clear();
                                        frame.suppressed_auto_id3_edits.clear();
                                        frame.id3_apply_error = None;
                                    }
                                    Err(error) => {
                                        frame.id3_apply_error = Some(
                                            TrackMetadataActionState::id3_apply_error_message(
                                                error,
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    pub(crate) fn clear_pending_id3_edits(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.applying_id3_edits {
            return;
        }
        frame.pending_id3_edits.clear();
        frame.suppressed_auto_id3_edits.clear();
        frame.id3_apply_error = None;
        cx.notify();
    }

    pub(crate) fn toggle_local_subscription(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let is_subscribed = self
            .inspector_stack
            .last()
            .and_then(|frame| frame.local_subscription)
            .unwrap_or(false);
        if is_subscribed {
            self.unsubscribe_current(window, cx);
        } else {
            self.subscribe_current(cx);
        }
    }

    /// Re-query `local_subscription` for every frame in the inspector stack so
    /// the inspector header reflects DB changes triggered by sibling actions
    /// (per-row download/remove buttons, etc.).
    fn refresh_inspector_subscription_state(&mut self, _cx: &mut Context<Self>) {
        if self.inspector_stack.is_empty() {
            return;
        }
        let snapshots: Vec<Option<bool>> = self
            .inspector_stack
            .iter()
            .map(|frame| {
                local_subscription_for_detail(&self.conn, &frame.detail)
                    .ok()
                    .flatten()
            })
            .collect();
        for (frame, snap) in self.inspector_stack.iter_mut().zip(snapshots) {
            frame.local_subscription = snap;
        }
    }

    pub fn refresh_application_state(&mut self, cx: &mut Context<Self>) {
        self.load_playlists();
        self.refresh_inspector_subscription_state(cx);
        cx.notify();
    }

    pub(crate) fn download_track_row(
        &mut self,
        track: Track,
        feed: Option<Feed>,
        cx: &mut Context<Self>,
    ) {
        let key = TrackRowActionVm::new(&track, false, false).key();
        if !self.vm.begin_track_operation(key.clone()) {
            return;
        }
        let command = SubscribeTrack::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeTrackRequest::SearchTrack {
                track_context: Box::new(TrackContext {
                    track: track.clone(),
                    feed,
                }),
                edits: Vec::new(),
                musicindex_endpoint: self.musicindex_endpoint.clone(),
                mark_feed_subscribed: false,
                return_tag_compare: true,
            },
            SearchSubscriptionCommand::track_download_success_message(),
        );
        cx.notify();

        let success_key = key.clone();
        let error_key = key;
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, outcome, cx| {
                this.vm
                    .finish_track_download(&success_key, outcome.message());
                this.refresh_inspector_subscription_state(cx);
                cx.emit(SearchAppEvent::LibraryMutated);
            },
            move |this, error, _cx| this.vm.fail_track_download(&error_key, error),
        );
    }

    pub(crate) fn remove_track_row(
        &mut self,
        track: Track,
        feed: Option<Feed>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = TrackRowActionVm::new(&track, false, false).key();
        self.request_library_removal(
            LibraryRemovalIntent::TrackMatch {
                feed_url: track
                    .feed_url
                    .clone()
                    .or_else(|| feed.as_ref().and_then(|feed| feed.feed_url.clone())),
                item_guid: track.track_guid.clone(),
                enclosure_url: track.enclosure_url.clone(),
            },
            SearchRemovalOrigin::Row { key },
            window,
            cx,
        );
    }

    fn subscribe_current(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.subscription_busy {
            return;
        }

        let entity_type = frame.entity_type.clone();
        let entity_id = frame.entity_id.clone();
        let musicindex_endpoint = self.musicindex_endpoint.clone();
        let request = match &frame.detail {
            InspectorDetail::Feed(feed) => {
                SearchSubscribeRequest::Feed(Box::new((**feed).clone()), musicindex_endpoint)
            }
            InspectorDetail::Track(track_context) => {
                let edits = if let LazyPanel::Loaded(result) = &frame.tag_compare {
                    let rows = track_metadata_rows_for_frame(frame, track_context, Some(result));
                    let pending = auto_populated_pending_id3_edits(
                        &rows,
                        &frame.pending_id3_edits,
                        &frame.suppressed_auto_id3_edits,
                        result.format,
                    );
                    let conflicts = pending_id3_conflict_descriptions(&pending);
                    if !conflicts.is_empty() {
                        frame.subscription_message = Some(
                            TrackMetadataActionState::duplicate_id3_target_message(&conflicts),
                        );
                        cx.notify();
                        return;
                    }
                    pending_id3_edits_for_apply(&pending)
                } else {
                    Vec::new()
                };
                SearchSubscribeRequest::Track(
                    Box::new((**track_context).clone()),
                    edits,
                    musicindex_endpoint,
                    true,
                )
            }
            InspectorDetail::Loading(_)
            | InspectorDetail::Error(_)
            | InspectorDetail::Artist(_)
            | InspectorDetail::Publisher(_) => return,
        };

        let subscription_command = SearchSubscriptionCommand::Download;
        frame.subscription_busy = true;
        frame.subscription_message = Some(subscription_command.begin_message().into());
        cx.notify();

        match request {
            SearchSubscribeRequest::Feed(feed, musicindex_endpoint) => {
                let app_command = SubscribeFeed::new(
                    Arc::clone(&self.conn),
                    self.application_services.download_manager(),
                    subscribe_service::SubscribeFeedRequest {
                        feed: *feed,
                        musicindex_endpoint,
                    },
                );
                let success_entity_type = entity_type.clone();
                let success_entity_id = entity_id.clone();
                let error_entity_type = entity_type;
                let error_entity_id = entity_id;
                self.command_runner.run(
                    app_command,
                    CommandContext::next(),
                    cx,
                    move |this, outcome, cx| {
                        let mut library_mutated = false;
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == success_entity_type
                                && frame.entity_id == success_entity_id
                            {
                                frame.subscription_busy = false;
                                frame.local_subscription = Some(true);
                                frame.subscription_message = Some(outcome.message().into());
                                library_mutated = true;
                            }
                        }
                        if library_mutated {
                            this.refresh_inspector_subscription_state(cx);
                            cx.emit(SearchAppEvent::LibraryMutated);
                        }
                    },
                    move |this, error, _cx| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == error_entity_type
                                && frame.entity_id == error_entity_id
                            {
                                frame.subscription_busy = false;
                                frame.subscription_message =
                                    Some(subscription_command.error_message(error));
                            }
                        }
                    },
                );
            }
            SearchSubscribeRequest::Track(
                track_context,
                edits,
                musicindex_endpoint,
                mark_feed_subscribed,
            ) => {
                let app_command = SubscribeTrack::new(
                    Arc::clone(&self.conn),
                    self.application_services.download_manager(),
                    SubscribeTrackRequest::SearchTrack {
                        track_context,
                        edits,
                        musicindex_endpoint,
                        mark_feed_subscribed,
                        return_tag_compare: true,
                    },
                    SearchSubscriptionCommand::track_download_success_message(),
                );
                let success_entity_type = entity_type.clone();
                let success_entity_id = entity_id.clone();
                let error_entity_type = entity_type;
                let error_entity_id = entity_id;
                self.command_runner.run(
                    app_command,
                    CommandContext::next(),
                    cx,
                    move |this, outcome, cx| {
                        let mut library_mutated = false;
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == success_entity_type
                                && frame.entity_id == success_entity_id
                            {
                                frame.subscription_busy = false;
                                frame.local_subscription = Some(true);
                                frame.subscription_message = Some(
                                    subscription_command.success_message(outcome.applied_edits()),
                                );
                                if let Some(compare) = outcome.into_compare() {
                                    frame.tag_compare = LazyPanel::Loaded(compare);
                                    frame.pending_id3_edits.clear();
                                    frame.suppressed_auto_id3_edits.clear();
                                    frame.id3_apply_error = None;
                                }
                                library_mutated = true;
                            }
                        }
                        if library_mutated {
                            this.refresh_inspector_subscription_state(cx);
                            cx.emit(SearchAppEvent::LibraryMutated);
                        }
                    },
                    move |this, error, _cx| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == error_entity_type
                                && frame.entity_id == error_entity_id
                            {
                                frame.subscription_busy = false;
                                frame.subscription_message =
                                    Some(subscription_command.error_message(error));
                            }
                        }
                    },
                );
            }
        }
    }

    fn unsubscribe_current(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.subscription_busy {
            return;
        }

        let entity_type = frame.entity_type.clone();
        let entity_id = frame.entity_id.clone();
        let intent = match &frame.detail {
            InspectorDetail::Feed(feed) => {
                let Some(feed_url) = feed.feed_url.clone() else {
                    frame.subscription_message = Some(
                        SearchSubscriptionCommand::Remove.error_message("feed has no RSS URL"),
                    );
                    cx.notify();
                    return;
                };
                LibraryRemovalIntent::FeedUrl(feed_url)
            }
            InspectorDetail::Track(track_context) => LibraryRemovalIntent::TrackMatch {
                feed_url: track_context.track.feed_url.clone().or_else(|| {
                    track_context
                        .feed
                        .as_ref()
                        .and_then(|feed| feed.feed_url.clone())
                }),
                item_guid: track_context.track.track_guid.clone(),
                enclosure_url: track_context.track.enclosure_url.clone(),
            },
            InspectorDetail::Loading(_)
            | InspectorDetail::Error(_)
            | InspectorDetail::Artist(_)
            | InspectorDetail::Publisher(_) => return,
        };

        self.request_library_removal(
            intent,
            SearchRemovalOrigin::Inspector {
                entity_type,
                entity_id,
                command: SearchSubscriptionCommand::Remove,
            },
            window,
            cx,
        );
    }

    fn request_library_removal(
        &mut self,
        intent: LibraryRemovalIntent,
        origin: SearchRemovalOrigin,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let plan_result = {
            let conn = self.conn.lock().expect("lock db");
            self.application_services
                .query_service()
                .library_removal_plan(&conn, &intent)
        };
        let plan = match plan_result {
            Ok(plan) => plan,
            Err(error) => {
                self.fail_library_removal_origin(&origin, error);
                cx.notify();
                return;
            }
        };
        if !self.vm.confirm_library_removal_from(plan, origin.clone()) {
            let Some(display) = self.vm.pending_library_removal_confirmation() else {
                cx.notify();
                return;
            };
            open_library_removal_confirmation_dialog(
                window,
                cx,
                display,
                |this, cx| {
                    this.vm.cancel_pending_library_removal();
                    cx.notify();
                },
                |this, cx| {
                    this.execute_pending_library_removal(cx);
                },
            );
            cx.notify();
            return;
        }

        self.execute_library_removal_target(plan.target(), origin, cx);
    }

    fn execute_pending_library_removal(&mut self, cx: &mut Context<Self>) {
        let Some((target, origin)) = self.vm.take_pending_library_removal() else {
            cx.notify();
            return;
        };
        self.execute_library_removal_target(target, origin, cx);
    }

    fn execute_library_removal_target(
        &mut self,
        target: LibraryRemovalTarget,
        origin: SearchRemovalOrigin,
        cx: &mut Context<Self>,
    ) {
        if !self.begin_library_removal_origin(&origin, cx) {
            return;
        }
        self.execute_library_removal_command(target, origin, cx);
    }

    fn begin_library_removal_origin(
        &mut self,
        origin: &SearchRemovalOrigin,
        cx: &mut Context<Self>,
    ) -> bool {
        match origin {
            SearchRemovalOrigin::Row { key } => {
                let started = self.vm.begin_track_operation(key.clone());
                if started {
                    cx.notify();
                }
                started
            }
            SearchRemovalOrigin::Inspector {
                entity_type,
                entity_id,
                command,
            } => {
                let Some(frame) = self.inspector_stack.last_mut() else {
                    return false;
                };
                if frame.subscription_busy
                    || frame.entity_type != *entity_type
                    || frame.entity_id != *entity_id
                {
                    return false;
                }
                frame.subscription_busy = true;
                frame.subscription_message = Some(command.begin_message().into());
                cx.notify();
                true
            }
        }
    }

    fn execute_library_removal_command(
        &mut self,
        target: LibraryRemovalTarget,
        origin: SearchRemovalOrigin,
        cx: &mut Context<Self>,
    ) {
        let command = RemoveFromLibrary::new(Arc::clone(&self.conn), target);
        let success_origin = origin.clone();
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.finish_library_removal_origin(&success_origin, result.message(), cx);
            },
            move |this, error, _cx| this.fail_library_removal_origin(&origin, error),
        );
    }

    fn finish_library_removal_origin(
        &mut self,
        origin: &SearchRemovalOrigin,
        message: &str,
        cx: &mut Context<Self>,
    ) {
        let mut library_mutated = false;
        match origin {
            SearchRemovalOrigin::Row { key } => {
                self.vm.finish_track_remove(key, message);
                library_mutated = true;
            }
            SearchRemovalOrigin::Inspector {
                entity_type,
                entity_id,
                ..
            } => {
                if let Some(frame) = self.inspector_stack.last_mut() {
                    if frame.entity_type == *entity_type && frame.entity_id == *entity_id {
                        frame.subscription_busy = false;
                        frame.local_subscription = Some(false);
                        frame.subscription_message = Some(message.into());
                        library_mutated = true;
                    }
                }
            }
        }
        if library_mutated {
            self.refresh_inspector_subscription_state(cx);
            cx.emit(SearchAppEvent::LibraryMutated);
        }
    }

    fn fail_library_removal_origin(
        &mut self,
        origin: &SearchRemovalOrigin,
        error: impl std::fmt::Display,
    ) {
        match origin {
            SearchRemovalOrigin::Row { key } => self.vm.fail_track_remove(key, error),
            SearchRemovalOrigin::Inspector {
                entity_type,
                entity_id,
                command,
            } => {
                if let Some(frame) = self.inspector_stack.last_mut() {
                    if frame.entity_type == *entity_type && frame.entity_id == *entity_id {
                        frame.subscription_busy = false;
                        frame.subscription_message = Some(command.error_message(error));
                    }
                }
            }
        }
    }

    fn load_playlists(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match self.application_services.query_service().playlists(&conn) {
            Ok(list) => self.vm.replace_playlists(list),
            Err(err) => self.vm.fail_playlist_load(err),
        }
    }

    fn ensure_feed_in_db(
        &mut self,
        feed_guid: &str,
        feed_url: Option<&str>,
    ) -> anyhow::Result<i64> {
        feed_service::ensure_feed_in_db(&self.conn, feed_guid, feed_url, &self.musicindex_endpoint)
    }

    pub(crate) fn add_feed_to_playlist(
        &mut self,
        feed_guid: &str,
        feed_url: Option<&str>,
        playlist_id: i64,
        cx: &mut Context<Self>,
    ) {
        let feed_id = match self.ensure_feed_in_db(feed_guid, feed_url) {
            Ok(id) => id,
            Err(err) => {
                self.vm.fail_feed_subscription(err);
                cx.notify();
                return;
            }
        };
        let track_ids: Vec<i64> = {
            let conn = self.conn.lock().expect("lock db");
            match db::feed_tracks(&conn, feed_id) {
                Ok(t) => t.into_iter().map(|row| row.id).collect(),
                Err(err) => {
                    self.vm.fail_feed_tracks_load(err);
                    cx.notify();
                    return;
                }
            }
        };
        if track_ids.is_empty() {
            self.vm.set_feed_has_no_tracks();
            cx.notify();
            return;
        }
        if let Some(intent) = self.vm.begin_playlist_append(playlist_id, track_ids) {
            self.spawn_subscribe_then_append(intent, cx);
        }
    }

    pub(crate) fn create_playlist_and_add_feed(
        &mut self,
        name: &str,
        feed_guid: &str,
        feed_url: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        let feed_guid = feed_guid.to_string();
        let feed_url = feed_url.map(str::to_string);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.load_playlists();
                this.add_feed_to_playlist(
                    &feed_guid,
                    feed_url.as_deref(),
                    result.playlist_id(),
                    cx,
                );
            },
            |this, err, _cx| this.vm.fail_playlist_create(err),
        );
    }

    pub(crate) fn create_playlist_and_add_track(
        &mut self,
        name: &str,
        track_id: i64,
        cx: &mut Context<Self>,
    ) {
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.load_playlists();
                this.add_track_to_playlist(track_id, result.playlist_id(), cx);
            },
            |this, err, _cx| this.vm.fail_playlist_create(err),
        );
    }

    pub(crate) fn create_playlist_and_add_discover_track(
        &mut self,
        name: &str,
        feed_guid: &str,
        feed_url: Option<&str>,
        track_guid: &str,
        cx: &mut Context<Self>,
    ) {
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        let feed_guid = feed_guid.to_string();
        let feed_url = feed_url.map(str::to_string);
        let track_guid = track_guid.to_string();
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.load_playlists();
                this.add_search_track_to_playlist(
                    &feed_guid,
                    feed_url.as_deref(),
                    &track_guid,
                    result.playlist_id(),
                    cx,
                );
            },
            |this, err, _cx| this.vm.fail_playlist_create(err),
        );
    }

    pub(crate) fn add_search_track_to_playlist(
        &mut self,
        feed_guid: &str,
        feed_url: Option<&str>,
        track_guid: &str,
        playlist_id: i64,
        cx: &mut Context<Self>,
    ) {
        let feed_id = match self.ensure_feed_in_db(feed_guid, feed_url) {
            Ok(id) => id,
            Err(err) => {
                self.vm.fail_feed_subscription(err);
                cx.notify();
                return;
            }
        };
        let track_id: Option<i64> = {
            let conn = self.conn.lock().expect("lock db");
            conn.query_row(
                "SELECT id FROM tracks WHERE feed_id = ?1 AND item_guid = ?2 LIMIT 1",
                rusqlite::params![feed_id, track_guid],
                |row| row.get(0),
            )
            .ok()
        };
        let Some(track_id) = track_id else {
            self.vm.set_track_not_in_library();
            cx.notify();
            return;
        };
        if let Some(intent) = self.vm.begin_playlist_append(playlist_id, vec![track_id]) {
            self.spawn_subscribe_then_append(intent, cx);
        }
    }

    pub(crate) fn add_track_to_playlist(
        &mut self,
        track_id: i64,
        playlist_id: i64,
        cx: &mut Context<Self>,
    ) {
        if let Some(intent) = self.vm.begin_playlist_append(playlist_id, vec![track_id]) {
            self.spawn_subscribe_then_append(intent, cx);
        }
    }

    fn spawn_subscribe_then_append(
        &mut self,
        intent: PlaylistAppendIntent,
        cx: &mut Context<Self>,
    ) {
        let playlist_id = intent.playlist_id();
        let track_ids = intent.track_ids().to_vec();
        cx.notify();

        let command = SubscribeThenAppendToPlaylist::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            playlist_id,
            track_ids,
        );
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, outcome, cx| {
                this.vm.finish_playlist_append(
                    &intent,
                    PlaylistAppendOutcome::new(
                        outcome.appended(),
                        outcome.downloaded(),
                        outcome.failed().len(),
                    ),
                );
                this.load_playlists();
                cx.emit(SearchAppEvent::LibraryMutated);
            },
            |this, err, cx| {
                this.vm.fail_playlist_append(err);
                this.load_playlists();
                cx.emit(SearchAppEvent::LibraryMutated);
            },
        );
    }

    fn toggle_tag_compare(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.entity_type != "track" {
            return;
        }

        match frame.tag_compare {
            LazyPanel::Loaded(_) => {
                frame.tag_compare = LazyPanel::Hidden;
                cx.notify();
                return;
            }
            LazyPanel::Loading => return,
            LazyPanel::Empty(_) | LazyPanel::Hidden => frame.tag_compare = LazyPanel::Loading,
        }

        let entity_id = frame.entity_id.clone();
        let entity_type = frame.entity_type.clone();
        let client = self.api_client();
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let request_id = entity_id.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { download_and_compare_track(&client, &request_id, false) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                frame.tag_compare = match result {
                                    Ok(result) => LazyPanel::Loaded(result),
                                    Err(error) => LazyPanel::error(error),
                                };
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    pub(crate) fn redownload_tag_compare(&mut self, cx: &mut Context<Self>) {
        self.reload_tag_compare(true, cx);
    }

    pub(crate) fn reread_tag_compare(&mut self, cx: &mut Context<Self>) {
        self.reload_tag_compare(false, cx);
    }

    fn reload_tag_compare(&mut self, force_download: bool, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.entity_type != "track" || !matches!(frame.tag_compare, LazyPanel::Loaded(_)) {
            return;
        }
        frame.tag_compare = LazyPanel::Loading;
        let entity_id = frame.entity_id.clone();
        let entity_type = frame.entity_type.clone();
        let client = self.api_client();
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let request_id = entity_id.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        download_and_compare_track(&client, &request_id, force_download)
                    })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                frame.tag_compare = match result {
                                    Ok(result) => LazyPanel::Loaded(result),
                                    Err(error) => LazyPanel::error(error),
                                };
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn toggle_musicbrainz_lookup(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.entity_type != "track" {
            return;
        }

        match frame.musicbrainz_lookup {
            LazyPanel::Loaded(_) => {
                frame.musicbrainz_lookup = LazyPanel::Hidden;
                frame.musicbrainz_selected = 0;
                cx.notify();
                return;
            }
            LazyPanel::Loading => return,
            LazyPanel::Empty(_) | LazyPanel::Hidden => {
                frame.musicbrainz_lookup = LazyPanel::Loading;
            }
        }

        let entity_id = frame.entity_id.clone();
        let entity_type = frame.entity_type.clone();
        let client = self.api_client();
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let request_id = entity_id.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { lookup_musicbrainz_track(&client, &request_id) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                frame.musicbrainz_lookup = match result {
                                    Ok(result) => {
                                        frame.musicbrainz_selected = 0;
                                        LazyPanel::Loaded(result)
                                    }
                                    Err(error) => LazyPanel::error(error),
                                };
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    pub(crate) fn select_musicbrainz_candidate(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if let LazyPanel::Loaded(result) = &frame.musicbrainz_lookup {
            if idx < result.lookup.candidates.len() {
                frame.musicbrainz_selected = idx;
                cx.notify();
            }
        }
    }

    fn library_membership_display_for_row(
        &self,
        row: &ResultRow,
    ) -> Option<SearchLibraryMembershipDisplay> {
        match row.source {
            SearchResultSource::Library => Some(SearchLibraryMembership::InLibrary.display()),
            SearchResultSource::MusicIndex => {
                self.musicindex_row_is_in_library(row).map(|is_in_library| {
                    if is_in_library {
                        SearchLibraryMembership::InLibrary
                    } else {
                        SearchLibraryMembership::NotInLibrary
                    }
                    .display()
                })
            }
        }
    }

    fn musicindex_row_is_in_library(&self, row: &ResultRow) -> Option<bool> {
        let db = self.conn.lock().ok()?;
        match row.detail.as_ref()? {
            EntityDetail::Feed(feed) => feed
                .feed_url
                .as_deref()
                .map(|feed_url| db::feed_is_subscribed_by_url(&db, feed_url).unwrap_or(false)),
            EntityDetail::Track(track) => library_service::track_is_in_library_by_match(
                &db,
                track.feed_url.as_deref(),
                track.track_guid.as_deref(),
                track.enclosure_url.as_deref(),
            )
            .ok(),
            EntityDetail::Artist(_)
            | EntityDetail::Release(_)
            | EntityDetail::Recording(_)
            | EntityDetail::Publisher(_) => None,
        }
    }
}

impl Render for SearchApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stack = self.inspector_stack.clone();
        let snapshot = self
            .vm
            .render_snapshot(stack.is_empty(), self.vm.active_query.is_none());
        let status_color = if snapshot.status.is_error {
            StatusRole::Danger.color(cx)
        } else {
            color::text_muted()
        };
        let status_empty = snapshot.status.is_empty();
        let status_text = snapshot.status.display_text;

        let list_focused = self.list_focus.is_focused(window);
        let sections: Vec<DiscoverResultSection> = snapshot
            .sections
            .iter()
            .map(|section| {
                let rows = section
                    .rows
                    .iter()
                    .map(|row| {
                        let item = row.render_item();
                        let thumbnail =
                            self.thumbnail_for_url(item.display.image_url.as_deref(), cx);
                        DiscoverResultRow::new(
                            item,
                            thumbnail,
                            self.library_membership_display_for_row(row),
                        )
                    })
                    .collect();
                DiscoverResultSection {
                    display: section.display.clone(),
                    rows,
                    show_empty: section.show_empty,
                }
            })
            .collect();
        let show_back = should_show_inspector_back(stack.len());
        let inspector = render_inspector(
            stack.last(),
            show_back,
            snapshot.show_recents_root,
            self,
            cx,
        );
        let is_loading = snapshot.loading;
        let is_empty = snapshot.empty;
        let has_more = snapshot.has_more;
        let fuzzy_search = snapshot.fuzzy_search;
        let index_controls = snapshot.index_controls;
        let show_recents_command = snapshot.show_recents_command;
        let pane_display = snapshot.pane_display.clone();

        let leading_pane = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(render_discover_search_controls(
                DiscoverSearchControlsParams {
                    type_filter: snapshot.type_filter,
                    is_loading,
                    fuzzy_search,
                    index_controls,
                    show_recents_command,
                    pane_display: pane_display.clone(),
                    status_color,
                    status_text,
                },
                cx,
            ))
            .child(render_discover_result_list(
                DiscoverResultListParams {
                    sections,
                    selected_key: snapshot.selected_key.clone(),
                    list_focused,
                    empty_state: DiscoverResultEmptyState {
                        is_empty,
                        is_loading,
                        status_empty,
                    },
                    pagination: DiscoverResultPagination { has_more },
                    pane_display: pane_display.clone(),
                    list_focus: &self.list_focus,
                    scroll_handle: &self.results_scroll,
                },
                cx,
            ))
            .into_any_element();

        let trailing_pane = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .child(inspector)
            .into_any_element();
        let split_pane = SplitPane::new(pane_display.split_pane_id)
            .resize_handle_id(pane_display.resize_handle_id)
            .leading_width(px(self.vm.split_pane_width()))
            .leading_min_width(layout::INSPECTOR_MIN_WIDTH)
            .leading(leading_pane)
            .trailing(trailing_pane)
            .on_resize_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                if this.vm.is_resizing() {
                    this.vm.resize_split_pane(
                        f32::from(event.position.x),
                        f32::from(layout::INSPECTOR_MIN_WIDTH),
                        f32::from(layout::INSPECTOR_MAX_WIDTH),
                    );
                    cx.notify();
                }
            }))
            .on_resize_end(cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                if this.vm.is_resizing() {
                    this.vm.end_resize();
                    cx.notify();
                }
            }))
            .on_resize_start(cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.vm.begin_resize();
                cx.notify();
            }));

        div()
            .size_full()
            .bg(color::bg_canvas())
            .text_color(color::text_primary())
            .text_sm()
            .flex()
            .overflow_hidden()
            .child(split_pane)
    }
}

fn fetch_search_batch(
    client: &Client,
    query: &str,
    entity_type: Option<&str>,
    cursor: Option<&str>,
    fuzzy: bool,
) -> Result<SearchBatch> {
    if entity_type == Some("artist") {
        return fetch_artist_search_batch(client, query, cursor, fuzzy);
    }

    if entity_type.is_none() {
        return fetch_partitioned_search_batch(client, query, cursor, fuzzy);
    }

    if entity_type.is_some_and(|kind| !search_result_type_is_visible(kind)) {
        return Ok(SearchBatch {
            rows: Vec::new(),
            has_more: false,
            cursor: None,
        });
    }

    let response = client.search(query, entity_type, Some(PAGE_LIMIT), cursor, fuzzy)?;
    let mut rows: Vec<ResultRow> = response
        .data
        .iter()
        .map(|hit| search_hit_to_result_row(client, hit))
        .filter(|row| search_result_type_is_visible(&row.entity_type))
        .collect();
    if entity_type.is_none() {
        let mut artist_rows = artist_rows_from_result_rows(&rows, Some(query));
        enrich_artist_rows(client, &mut artist_rows);
        rows.splice(0..0, artist_rows);
    }

    Ok(SearchBatch {
        rows,
        has_more: response.pagination.has_more,
        cursor: response.pagination.cursor,
    })
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct PartitionedSearchCursor {
    feed: Option<String>,
    track: Option<String>,
}

fn fetch_partitioned_search_batch(
    client: &Client,
    query: &str,
    cursor: Option<&str>,
    fuzzy: bool,
) -> Result<SearchBatch> {
    let parsed_cursor = cursor.and_then(decode_partitioned_search_cursor);
    let feed_cursor = parsed_cursor
        .as_ref()
        .and_then(|cursor| cursor.feed.as_deref());
    let track_cursor = parsed_cursor
        .as_ref()
        .and_then(|cursor| cursor.track.as_deref());

    let feed_batch = if cursor.is_some() && feed_cursor.is_none() {
        Ok(SearchBatch {
            rows: Vec::new(),
            has_more: false,
            cursor: None,
        })
    } else {
        fetch_typed_search_batch(client, query, "feed", feed_cursor, fuzzy)
    };
    let track_batch = if cursor.is_some() && track_cursor.is_none() {
        Ok(SearchBatch {
            rows: Vec::new(),
            has_more: false,
            cursor: None,
        })
    } else {
        fetch_typed_search_batch(client, query, "track", track_cursor, fuzzy)
    };

    match (feed_batch, track_batch) {
        (Ok(mut feeds), Ok(tracks)) => {
            feeds.rows.extend(tracks.rows);
            let mut rows = artist_rows_from_result_rows(&feeds.rows, Some(query));
            enrich_artist_rows(client, &mut rows);
            rows.extend(feeds.rows);
            Ok(SearchBatch {
                rows,
                has_more: feeds.has_more || tracks.has_more,
                cursor: encode_partitioned_search_cursor(
                    feeds.cursor.as_deref(),
                    tracks.cursor.as_deref(),
                ),
            })
        }
        (Ok(mut feeds), Err(_track_error)) => {
            let mut rows = artist_rows_from_result_rows(&feeds.rows, Some(query));
            enrich_artist_rows(client, &mut rows);
            rows.append(&mut feeds.rows);
            Ok(SearchBatch {
                rows,
                has_more: feeds.has_more,
                cursor: encode_partitioned_search_cursor(feeds.cursor.as_deref(), None),
            })
        }
        (Err(_feed_error), Ok(mut tracks)) => {
            let mut rows = artist_rows_from_result_rows(&tracks.rows, Some(query));
            enrich_artist_rows(client, &mut rows);
            rows.append(&mut tracks.rows);
            Ok(SearchBatch {
                rows,
                has_more: tracks.has_more,
                cursor: encode_partitioned_search_cursor(None, tracks.cursor.as_deref()),
            })
        }
        (Err(feed_error), Err(track_error)) => Err(anyhow!(
            "feed search failed: {feed_error}; track search failed: {track_error}"
        )),
    }
}

fn fetch_typed_search_batch(
    client: &Client,
    query: &str,
    entity_type: &str,
    cursor: Option<&str>,
    fuzzy: bool,
) -> Result<SearchBatch> {
    let response = client.search(query, Some(entity_type), Some(PAGE_LIMIT), cursor, fuzzy)?;
    Ok(SearchBatch {
        rows: response
            .data
            .iter()
            .map(|hit| search_hit_to_result_row(client, hit))
            .filter(|row| search_result_type_is_visible(&row.entity_type))
            .collect(),
        has_more: response.pagination.has_more,
        cursor: response.pagination.cursor,
    })
}

fn encode_partitioned_search_cursor(feed: Option<&str>, track: Option<&str>) -> Option<String> {
    if feed.is_none() && track.is_none() {
        return None;
    }
    serde_json::to_string(&PartitionedSearchCursor {
        feed: feed.map(str::to_string),
        track: track.map(str::to_string),
    })
    .ok()
    .map(|cursor| format!("partitioned:{cursor}"))
}

fn decode_partitioned_search_cursor(cursor: &str) -> Option<PartitionedSearchCursor> {
    cursor
        .strip_prefix("partitioned:")
        .and_then(|value| serde_json::from_str(value).ok())
}

fn fetch_local_library_search_rows(
    conn: &Arc<Mutex<Connection>>,
    query_service: &crate::application::ApplicationQueryService,
    query: &str,
) -> Result<Vec<ResultRow>> {
    let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    let tracks = query_service
        .search_local_library_tracks(&db, query, None)
        .map_err(|error| anyhow!("{error}"))?;
    tracks
        .into_iter()
        .map(|track| {
            let track_id = track.id;
            let context =
                feed_service::track_row_to_track_context_with_local_identity(&db, &track)?;
            Ok(ResultRow::local_library_track(
                track_id,
                EntityDetail::Track(context.track),
            ))
        })
        .collect()
}

fn fetch_artist_search_batch(
    client: &Client,
    query: &str,
    cursor: Option<&str>,
    fuzzy: bool,
) -> Result<SearchBatch> {
    let batch = fetch_partitioned_search_batch(client, query, cursor, fuzzy)?;

    Ok(SearchBatch {
        rows: {
            let mut artist_rows = artist_rows_from_result_rows(&batch.rows, Some(query));
            enrich_artist_rows(client, &mut artist_rows);
            artist_rows
        },
        has_more: batch.has_more,
        cursor: batch.cursor,
    })
}

fn search_hit_to_result_row(client: &Client, hit: &SearchResult) -> ResultRow {
    let detail = fetch_scoped_detail(
        client,
        &hit.entity_type,
        &hit.entity_id,
        hit.feed_guid.as_deref(),
        None,
    )
    .ok()
    .filter(|detail| {
        matches!(
            detail,
            EntityDetail::Artist(_) | EntityDetail::Feed(_) | EntityDetail::Track(_)
        )
    });
    if hit.entity_type == "track" {
        ResultRow::musicindex_track(hit.entity_id.clone(), hit.feed_guid.clone(), detail)
    } else {
        ResultRow::new(hit.entity_type.clone(), hit.entity_id.clone(), detail)
    }
}

fn enrich_artist_rows(client: &Client, rows: &mut [ResultRow]) {
    for row in rows.iter_mut() {
        if row.entity_type != "artist" {
            continue;
        }
        let artist_name = match row.detail.as_ref() {
            Some(EntityDetail::Artist(a)) => a
                .name
                .clone()
                .or_else(|| a.artist_id.clone())
                .unwrap_or_else(|| row.entity_id.clone()),
            _ => row.entity_id.clone(),
        };
        if artist_name.is_empty() {
            continue;
        }
        let Ok(response) = client.fetch_tracks_by_artist(&artist_name, Some(PAGE_LIMIT * 2), None)
        else {
            continue;
        };
        let tracks = response.data;
        let distinct_feeds: BTreeSet<String> = tracks
            .iter()
            .filter_map(|t| {
                t.feed_guid
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            })
            .collect();
        let track_total = tracks.len() as i32;
        let feed_total = distinct_feeds.len() as i32;
        let first_feed_image = distinct_feeds
            .iter()
            .next()
            .and_then(|g| client.fetch_feed(g, None).ok())
            .and_then(|f| f.image_url);

        if let Some(EntityDetail::Artist(artist)) = row.detail.as_mut() {
            artist.track_count = Some(track_total);
            artist.feed_count = Some(feed_total);
            if artist.image_url.is_none() {
                artist.image_url = first_feed_image;
            }
        }
    }
}

pub(super) fn should_show_inspector_back(stack_len: usize) -> bool {
    stack_len > 0
}

fn fetch_inspector_detail(
    client: &Client,
    entity_type: &str,
    entity_id: &str,
    feed_guid: Option<&str>,
) -> Result<(InspectorDetail, Option<String>)> {
    match entity_type {
        "artist" => {
            let response = client.fetch_tracks_by_artist(entity_id, Some(PAGE_LIMIT * 2), None)?;
            let tracks = response.data;
            let has_more_tracks = response.pagination.has_more;

            let mut feed_order: Vec<String> = Vec::new();
            let mut artist_track_count_by_feed: BTreeMap<String, i32> = BTreeMap::new();
            for track in &tracks {
                let Some(guid) = track
                    .feed_guid
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                else {
                    continue;
                };
                let key = guid.to_string();
                let entry = artist_track_count_by_feed.entry(key.clone()).or_insert(0);
                if *entry == 0 {
                    feed_order.push(key);
                }
                *entry += 1;
            }

            let mut feeds: Vec<Feed> = Vec::with_capacity(feed_order.len());
            for guid in &feed_order {
                let fetched = client.fetch_feed(guid, None).ok();
                let artist_tracks_in_feed =
                    artist_track_count_by_feed.get(guid).copied().unwrap_or(0);
                let feed = match fetched {
                    Some(mut f) => {
                        f.episode_count = Some(artist_tracks_in_feed);
                        f
                    }
                    None => {
                        let fallback_title = tracks
                            .iter()
                            .find(|t| t.feed_guid.as_deref() == Some(guid.as_str()))
                            .and_then(|t| t.feed_title.clone());
                        Feed {
                            feed_guid: Some(guid.clone()),
                            title: fallback_title,
                            episode_count: Some(artist_tracks_in_feed),
                            ..Feed::default()
                        }
                    }
                };
                feeds.push(feed);
            }

            let image_url = feeds
                .iter()
                .find_map(|f| nonempty_url(f.image_url.as_deref()).map(str::to_string))
                .or_else(|| {
                    tracks.iter().find_map(|track| {
                        nonempty_url(track.image_url.as_deref()).map(str::to_string)
                    })
                });
            let artist = Artist {
                name: Some(entity_id.to_string()),
                image_url: image_url.clone(),
                track_count: Some(tracks.len() as i32),
                feed_count: Some(feeds.len() as i32),
                ..Artist::default()
            };
            Ok((
                InspectorDetail::Artist(Box::new(ArtistContext {
                    artist,
                    tracks,
                    feeds,
                    has_more_tracks,
                })),
                image_url,
            ))
        }
        "feed" => {
            let mut feed = client.fetch_feed(
                entity_id,
                Some(
                    "tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes",
                ),
            )?;
            hydrate_feed_track_play_urls(client, &mut feed);
            sanitize_feed_source_text(&mut feed);
            let image_url = feed
                .image_url
                .as_deref()
                .and_then(|u| nonempty_url(Some(u)))
                .map(str::to_string);
            Ok((InspectorDetail::Feed(Box::new(feed)), image_url))
        }
        "track" => {
            let mut track = fetch_scoped_track(
                client,
                entity_id,
                feed_guid,
                Some(
                    "source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes",
                ),
            )?;
            let mut feed = track.feed_guid.as_deref().and_then(|feed_guid| {
                client
                    .fetch_feed(
                        feed_guid,
                        Some(
                            "tracks,source_enclosures,source_links,source_ids,source_release_claims,payment_routes",
                        ),
                    )
                    .ok()
            });
            enrich_track_context_from_rss(&mut track, feed.as_mut());
            let mut track_context = TrackContext { track, feed };
            sanitize_track_context_source_text(&mut track_context);
            let image_url = track_context
                .track
                .image_url
                .as_deref()
                .and_then(|u| nonempty_url(Some(u)))
                .map(str::to_string);
            Ok((InspectorDetail::Track(Box::new(track_context)), image_url))
        }
        "publisher" => Ok((
            InspectorDetail::Publisher(client.fetch_publisher(entity_id)?),
            None,
        )),
        _ => Err(anyhow!("unknown inspector entity type: {entity_type}")),
    }
}

fn fetch_scoped_detail(
    client: &Client,
    entity_type: &str,
    entity_id: &str,
    feed_guid: Option<&str>,
    include: Option<&str>,
) -> Result<EntityDetail> {
    match entity_type {
        "track" => Ok(EntityDetail::Track(fetch_scoped_track(
            client, entity_id, feed_guid, include,
        )?)),
        _ => client.fetch_detail(entity_type, entity_id),
    }
}

fn fetch_scoped_track(
    client: &Client,
    track_guid: &str,
    feed_guid: Option<&str>,
    include: Option<&str>,
) -> Result<Track> {
    match feed_guid.map(str::trim).filter(|guid| !guid.is_empty()) {
        Some(feed_guid) => client.fetch_feed_track(feed_guid, track_guid, include),
        None => client.fetch_track(track_guid, include),
    }
}

fn resolve_podroll_feeds(client: &Client, feed_url: &str) -> Result<Vec<Feed>> {
    let entries = rss::fetch_feed_podroll(feed_url)?;
    let mut feeds: Vec<Feed> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for entry in entries {
        let guid = entry
            .feed_guid
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let key = guid
            .map(str::to_string)
            .or_else(|| entry.feed_url.clone())
            .unwrap_or_default();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        let fetched = guid.and_then(|g| client.fetch_feed(g, None).ok());
        let feed = match fetched {
            Some(f) => f,
            None => Feed {
                feed_guid: entry.feed_guid.clone(),
                feed_url: entry.feed_url.clone(),
                ..Feed::default()
            },
        };
        feeds.push(feed);
    }
    Ok(feeds)
}

fn hydrate_feed_track_play_urls(client: &Client, feed: &mut Feed) {
    let Some(tracks) = feed.tracks.as_mut() else {
        return;
    };

    for track in tracks
        .iter_mut()
        .filter(|track| TrackVm::new(track).play_url().is_none())
    {
        let Some(track_guid) = nonempty_url(track.track_guid.as_deref()).map(str::to_string) else {
            continue;
        };
        let Ok(hydrated) = fetch_scoped_track(
            client,
            &track_guid,
            track.feed_guid.as_deref(),
            Some("source_enclosures"),
        ) else {
            continue;
        };
        merge_track_play_fields(track, hydrated);
    }
}

pub(super) fn merge_track_play_fields(track: &mut Track, hydrated: Track) {
    if nonempty_url(track.enclosure_url.as_deref()).is_none() {
        track.enclosure_url = hydrated.enclosure_url;
    }
    if track.enclosure_type.is_none() {
        track.enclosure_type = hydrated.enclosure_type;
    }
    if track.enclosure_bytes.is_none() {
        track.enclosure_bytes = hydrated.enclosure_bytes;
    }
    if track.source_enclosures.as_ref().is_none_or(Vec::is_empty) {
        track.source_enclosures = hydrated.source_enclosures;
    }
}

pub(super) fn feed_rss_url(feed: &Feed) -> Option<String> {
    nonempty_url(feed.feed_url.as_deref()).map(str::to_string)
}

fn nonempty_url(url: Option<&str>) -> Option<&str> {
    url.map(str::trim).filter(|url| !url.is_empty())
}

pub(crate) fn detail_rows_from_strings(rows: Vec<(String, String)>) -> Vec<DetailRow> {
    rows.into_iter()
        .map(|(key, value)| DetailRow {
            key,
            value: MultilineText::new(value)
                .max_lines(6)
                .size(FontSize::Micro)
                .line_height(typography::LINE_DETAIL)
                .into_any_element(),
        })
        .collect()
}

fn download_and_compare_track(
    client: &Client,
    entity_id: &str,
    force_download: bool,
) -> Result<TagCompareResult> {
    subscribe_service::download_and_compare_track(client, entity_id, force_download)
}

enum SearchSubscribeRequest {
    Feed(Box<Feed>, String),
    Track(Box<TrackContext>, Vec<Id3v24Edit>, String, bool),
}

fn local_subscription_for_detail(
    conn: &Arc<Mutex<Connection>>,
    detail: &InspectorDetail,
) -> Result<Option<bool>> {
    let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    match detail {
        InspectorDetail::Feed(feed) => feed
            .feed_url
            .as_deref()
            .map(|feed_url| db::feed_is_subscribed_by_url(&db, feed_url))
            .transpose(),
        InspectorDetail::Track(track_context) => {
            let feed_url = track_context.track.feed_url.as_deref().or_else(|| {
                track_context
                    .feed
                    .as_ref()
                    .and_then(|feed| feed.feed_url.as_deref())
            });
            library_service::track_is_in_library_by_match(
                &db,
                feed_url,
                track_context.track.track_guid.as_deref(),
                track_context.track.enclosure_url.as_deref(),
            )
            .map(Some)
        }
        InspectorDetail::Loading(_)
        | InspectorDetail::Error(_)
        | InspectorDetail::Artist(_)
        | InspectorDetail::Publisher(_) => Ok(None),
    }
}

pub(super) fn persist_musicindex_artist_facts(
    conn: &Arc<Mutex<Connection>>,
    batch: &SearchBatch,
) -> Result<()> {
    let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    for row in &batch.rows {
        let Some(EntityDetail::Artist(artist)) = &row.detail else {
            continue;
        };
        identity_ingest::persist_musicindex_artist(&mut db, artist)?;
    }
    Ok(())
}

fn lookup_musicbrainz_track(client: &Client, entity_id: &str) -> Result<MusicBrainzLookupResult> {
    subscribe_service::lookup_musicbrainz_track(client, entity_id)
}
