#![allow(dead_code)]
#![warn(clippy::pedantic)]
#![expect(
    clippy::pedantic,
    reason = "legacy discover screen is being migrated incrementally under ADR 0023"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use gpui::{
    div, prelude::*, px, size, AnyElement, App, Application, Bounds, ClickEvent, ClipboardItem,
    Context, Entity, FontWeight, Image, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render, Rgba, SharedString,
    Styled, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::spinner::Spinner;
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Root, Size};
use rusqlite::Connection;

use crate::api::*;
use crate::application::commands::download::{
    RemoveTrackFromLibraryByMatch, SubscribeThenAppendToPlaylist, SubscribeTrack,
};
use crate::application::commands::feed::{SubscribeFeed, UnsubscribeFeedByUrl};
use crate::application::commands::playlist::CreatePlaylist;
use crate::application::{ApplicationServices, CommandContext};
#[cfg(test)]
use crate::audio_tags::Id3Field;
use crate::audio_tags::{write_id3v24_edits, Id3v24Edit};
use crate::config;
use crate::db;
use crate::feed_service;
use crate::identity_ingest;
use crate::library_service;
use crate::media::{image_from_bytes, ImageCache};
use crate::metadata::*;
use crate::musicbrainz::MusicBrainzCandidate;
use crate::presentation::GpuiCommandRunner;
use crate::rss;
use crate::subscribe_service::{
    self, compare_downloaded_track_path, download_image, enrich_track_context_from_rss,
    SubscribeTrackRequest,
};
use crate::track_compare::ComparisonStatus;
use crate::ui::composites::{
    action_button, identity_action_button, ActionButtonDisplay, ActionRow, ActionRowMessage,
    AddToPlaylistDisplay, AddToPlaylistPopover, DetailGrid, DetailHeader, DetailHeaderDisplay,
    DetailRow as CompositeDetailRow, DetailTextRow as CompositeDetailTextRow, DisclosureGroup,
    DisclosureGroupDisplay, EntityKind, IdentityActionKind, ListRow, PlaylistOption,
    PlaylistOptionDisplay, ProvenanceRole, RecentFeedTile, ReleaseSurfaceElement, SplitPane,
    StatusRole, TagBadge, TagBadgeDisplay, Thumbnail, ThumbnailSize, TrackDetailSurface,
    TrackInspectorPane, TrackMetadataGrid, TrackSurfaceElement,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::detail_row::DetailRow;
use crate::ui::primitives::SectionHeader;
use crate::ui::primitives::{
    Button as UiButton, Image as ImagePrimitive, ImageSize, Label, LoadingMessage, MultilineText,
};
use crate::ui::shells::entity::{render_contributor_rows, ContributorRowSlot};
use crate::ui::shells::{artist, feed, track};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::tokens::{FontSize, Radius, SemanticColor};
use crate::view_models::entity_detail::{
    ContributorIdentityActionKind, ContributorListVm, ContributorRowVm, EntityActionTarget,
    EntityActionTone, EntitySurfaceContext, MetadataPanelState, TrackMetadataActionState,
};
use crate::view_models::metadata::value_route_recipient_label;
use crate::view_models::playlist_option_displays;
use crate::view_models::search::{
    artist_rows_from_result_rows, normalized_search_query, search_result_type_is_visible,
    ActionRowVm, DeferredPanelKind, InspectorChromeDisplay, LazyPanel, PaymentRouteVm,
    PlaylistAppendIntent, PlaylistAppendOutcome, PublisherInspectorVm, PublisherLinkDisplay,
    RecentFeedTileDisplay, RecentFeedTileVm, ResultRow, ResultRowRenderItem, SearchBatch,
    SearchSubscriptionCommand, SearchViewModel, TrackFeedLinkDisplay, TrackInspectorHeaderVm,
    TrackRowActionVm,
};
use crate::view_models::track::{TrackPlayAudioDisplay, TrackVm};
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
use crate::view_models::track_metadata_grid::{
    TrackMetadataExpandedFieldKind, TrackMetadataGridVm, TrackMetadataId3FrameColorContext,
    TrackMetadataId3FrameColorRole, ValueRouteFieldContext, ValueRoutesSummaryFallback,
};
use crate::views::{ContributorView, FeedRef, TrackView};

#[derive(Clone, Debug)]
pub(crate) enum InspectorDetail {
    Loading(String),
    Error(String),
    Artist(Box<ArtistContext>),
    Feed(Box<Feed>),
    Track(Box<TrackContext>),
    Publisher(Publisher),
}

#[derive(Clone, Debug)]
pub(crate) struct ArtistContext {
    artist: Artist,
    tracks: Vec<Track>,
    feeds: Vec<Feed>,
    has_more_tracks: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct InspectorFrame {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub detail: InspectorDetail,
    pub image: Option<Arc<Image>>,
    pub contributors: LazyPanel<Vec<ContributorView>>,
    pub contributors_collapsed: bool,
    pub value_routes: LazyPanel<Vec<PaymentRoute>>,
    pub value_routes_collapsed: bool,
    pub expanded_id3_frame_groups: BTreeSet<String>,
    pub expanded_metadata_cells: BTreeSet<String>,
    pub pending_id3_edits: BTreeMap<String, PendingId3Edit>,
    pub suppressed_auto_id3_edits: BTreeSet<String>,
    pub applying_id3_edits: bool,
    pub id3_apply_error: Option<String>,
    pub local_subscription: Option<bool>,
    pub subscription_busy: bool,
    pub subscription_message: Option<String>,
    pub tag_compare: LazyPanel<TagCompareResult>,
    pub musicbrainz_lookup: LazyPanel<MusicBrainzLookupResult>,
    pub musicbrainz_selected: usize,
    pub podroll: LazyPanel<Vec<Feed>>,
}

impl InspectorFrame {
    fn loading(entity_type: String, entity_id: String, title: String) -> Self {
        Self {
            entity_type,
            entity_id,
            title: title.clone(),
            detail: InspectorDetail::Loading(SearchViewModel::inspector_loading_message(&title)),
            image: None,
            contributors: LazyPanel::Hidden,
            contributors_collapsed: true,
            value_routes: LazyPanel::Hidden,
            value_routes_collapsed: true,
            expanded_id3_frame_groups: BTreeSet::new(),
            expanded_metadata_cells: BTreeSet::new(),
            pending_id3_edits: BTreeMap::new(),
            suppressed_auto_id3_edits: BTreeSet::new(),
            applying_id3_edits: false,
            id3_apply_error: None,
            local_subscription: None,
            subscription_busy: false,
            subscription_message: None,
            tag_compare: LazyPanel::Hidden,
            musicbrainz_lookup: LazyPanel::Hidden,
            musicbrainz_selected: 0,
            podroll: LazyPanel::Hidden,
        }
    }
}

struct MetadataDragPreview {
    label: String,
    value: String,
}

impl Render for MetadataDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(layout::MENU_MIN_WIDTH)
            .rounded(radius::MD)
            .border_1()
            .border_color(color::accent())
            .bg(color::bg_surface())
            .p(spacing::SM)
            .child(
                div()
                    .text_size(typography::SIZE_MICRO)
                    .font_weight(FontWeight::BOLD)
                    .text_color(color::text_muted())
                    .child(SharedString::from(self.label.clone())),
            )
            .child(
                div().mt(spacing::XS).child(
                    MultilineText::new(self.value.clone())
                        .max_lines(4)
                        .size(FontSize::Micro)
                        .line_height(typography::LINE_BODY)
                        .color(SemanticColor::Label),
                ),
            )
    }
}

#[derive(Clone)]
enum ThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
}

pub struct SearchApp {
    conn: Arc<Mutex<Connection>>,
    application_services: Arc<ApplicationServices>,
    command_runner: GpuiCommandRunner,
    cache: Arc<ImageCache>,
    musicindex_endpoint: String,
    input: Entity<InputState>,
    /// Stateful screen view-model. Owns all pure UI scalars,
    /// pane-state flags, and loaded snapshots (results, recent feeds,
    /// playlists). Fields kept on `SearchApp` itself are GPUI-bound
    /// (`Entity`, `Subscription`, `FocusHandle`), service
    /// handles, screen-only inspector state, or maps that still hold
    /// `Arc<gpui::Image>`. See ADR 0023.
    pub(crate) vm: SearchViewModel,
    inspector_stack: Vec<InspectorFrame>,
    thumbnails: BTreeMap<String, ThumbnailState>,
    _input_sub: gpui::Subscription,
    list_focus: gpui::FocusHandle,
}

/// Events emitted by [`SearchApp`] to notify peer components (e.g. the
/// library tab) that local library state has changed and they should refresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchAppEvent {
    LibraryMutated,
}

impl gpui::EventEmitter<SearchAppEvent> for SearchApp {}

type FeedTrackListContext<'a> = (&'a str, Option<&'a str>, &'a [db::Playlist]);

impl SearchApp {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        cache: Arc<ImageCache>,
        musicindex_endpoint: String,
        application_services: Arc<ApplicationServices>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input_display = SearchViewModel::search_input_display();
        let input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder(input_display.placeholder)
        });
        let input_sub = cx.subscribe(&input, Self::on_input_event);
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
            input,
            vm: SearchViewModel::new(),
            inspector_stack: Vec::new(),
            thumbnails: BTreeMap::new(),
            _input_sub: input_sub,
            list_focus: cx.focus_handle(),
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

    fn load_recent_feeds(&mut self, append: bool, cx: &mut Context<Self>) {
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
                    match result {
                        Ok(response) => {
                            this.vm.finish_recent_feed_load(response);
                        }
                        Err(error) => {
                            this.vm.fail_recent_feed_load(error);
                        }
                    }
                    cx.notify();
                });
            },
        )
        .detach();
    }

    fn api_client(&self) -> Arc<Client> {
        Arc::new(Client::new_with_base_url(self.musicindex_endpoint.clone()))
    }

    pub fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
    }

    pub fn pop_inspector(&mut self, cx: &mut Context<Self>) {
        if !self.inspector_stack.is_empty() {
            self.inspector_stack.pop();
            cx.notify();
        }
    }

    pub fn move_up(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.vm.previous_result_target() {
            let (entity_type, entity_id, title) = target.into_parts();
            self.select_result(entity_type, entity_id, title, cx);
        }
    }

    pub fn move_down(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.vm.next_result_target() {
            let (entity_type, entity_id, title) = target.into_parts();
            self.select_result(entity_type, entity_id, title, cx);
        }
    }

    pub fn confirm(&mut self, _cx: &mut Context<Self>) {
        // In search, confirm might mean "open details" which select_result already does.
        // If we want to focus the inspector, we could do that.
    }

    fn on_input_event(
        &mut self,
        _entity: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::PressEnter { .. } = event {
            self.do_search(false, cx);
        }
    }

    fn do_search(&mut self, append: bool, cx: &mut Context<Self>) {
        let Some(query) = normalized_search_query(&self.input.read(cx).value()) else {
            return;
        };

        let Some(intent) = self.vm.begin_search_load(append) else {
            return;
        };
        if !append {
            self.inspector_stack.clear();
        }
        cx.notify();

        let entity_type =
            SearchViewModel::type_filter_value(intent.type_filter()).map(str::to_string);
        let cursor = intent.cursor().map(str::to_string);
        let fuzzy = intent.fuzzy();
        let client = self.api_client();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let batch = cx
                    .background_executor()
                    .spawn(async move {
                        fetch_search_batch(
                            &client,
                            &query,
                            entity_type.as_deref(),
                            cursor.as_deref(),
                            fuzzy,
                        )
                    })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        match batch {
                            Ok(batch) => {
                                match persist_musicindex_artist_facts(&this.conn, &batch) {
                                    Ok(()) => this.vm.finish_search_load(batch, append),
                                    Err(error) => this.vm.fail_search_load(error),
                                }
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

    fn toggle_fuzzy_search(&mut self, cx: &mut Context<Self>) {
        self.vm.toggle_fuzzy_search();
        let has_query = normalized_search_query(&self.input.read(cx).value()).is_some();
        cx.notify();
        if has_query {
            self.do_search(false, cx);
        }
    }

    fn thumbnail_for_url(
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

    fn select_result(
        &mut self,
        entity_type: String,
        entity_id: String,
        title: String,
        cx: &mut Context<Self>,
    ) {
        self.vm.select_result(&entity_type, &entity_id);
        self.load_inspector(entity_type, entity_id, title, false, cx);
    }

    fn open_recent_feed(&mut self, feed_guid: String, title: String, cx: &mut Context<Self>) {
        self.vm.select_recent_feed(&feed_guid);
        self.load_inspector("feed".into(), feed_guid, title, false, cx);
    }

    pub(crate) fn push_inspector(
        &mut self,
        entity_type: String,
        entity_id: String,
        title: String,
        cx: &mut Context<Self>,
    ) {
        self.load_inspector(entity_type, entity_id, title, true, cx);
    }

    fn load_inspector(
        &mut self,
        entity_type: String,
        entity_id: String,
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
                let detail = cx
                    .background_executor()
                    .spawn(
                        async move { fetch_inspector_detail(&client, &request_type, &request_id) },
                    )
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        let local_subscription = detail.as_ref().ok().and_then(|(detail, _)| {
                            local_subscription_for_detail(&this.conn, detail)
                                .ok()
                                .flatten()
                        });
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                match detail {
                                    Ok((detail, image)) => {
                                        if let InspectorDetail::Artist(ctx) = &detail {
                                            this.vm.merge_artist_result_detail(
                                                &entity_id,
                                                &ctx.artist,
                                            );
                                        }
                                        frame.detail = detail;
                                        frame.image = image;
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
                                    }
                                    Err(error) => {
                                        frame.detail = InspectorDetail::Error(error.to_string());
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

    fn inspector_back(&mut self, cx: &mut Context<Self>) {
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

    fn toggle_contributors(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };

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

    fn toggle_value_routes(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };

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

    fn toggle_id3_frame_group(&mut self, group_key: String, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if !frame.expanded_id3_frame_groups.remove(&group_key) {
            frame.expanded_id3_frame_groups.insert(group_key);
        }
        cx.notify();
    }

    fn toggle_metadata_cell(&mut self, cell_key: String, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if !frame.expanded_metadata_cells.remove(&cell_key) {
            frame.expanded_metadata_cells.insert(cell_key);
        }
        cx.notify();
    }

    fn stage_id3_drag_copy(&mut self, drag: &MetadataDragValue, cx: &mut Context<Self>) {
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

    fn revert_pending_id3_edit(&mut self, row_id: String, cx: &mut Context<Self>) {
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

    fn apply_pending_id3_edits(&mut self, cx: &mut Context<Self>) {
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

    fn clear_pending_id3_edits(&mut self, cx: &mut Context<Self>) {
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

    fn toggle_local_subscription(&mut self, cx: &mut Context<Self>) {
        let is_subscribed = self
            .inspector_stack
            .last()
            .and_then(|frame| frame.local_subscription)
            .unwrap_or(false);
        if is_subscribed {
            self.unsubscribe_current(cx);
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
        cx: &mut Context<Self>,
    ) {
        let key = TrackRowActionVm::new(&track, false, false).key();
        if !self.vm.begin_track_operation(key.clone()) {
            return;
        }
        let command = RemoveTrackFromLibraryByMatch::new(
            Arc::clone(&self.conn),
            track
                .feed_url
                .clone()
                .or_else(|| feed.as_ref().and_then(|feed| feed.feed_url.clone())),
            track.track_guid.clone(),
            track.enclosure_url.clone(),
        );
        cx.notify();

        let success_key = key.clone();
        let error_key = key;
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.vm.finish_track_remove(&success_key, result.message());
                this.refresh_inspector_subscription_state(cx);
                cx.emit(SearchAppEvent::LibraryMutated);
            },
            move |this, error, _cx| this.vm.fail_track_remove(&error_key, error),
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

    fn unsubscribe_current(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        if frame.subscription_busy {
            return;
        }

        let entity_type = frame.entity_type.clone();
        let entity_id = frame.entity_id.clone();
        let request = match &frame.detail {
            InspectorDetail::Feed(feed) => SearchUnsubscribeRequest::Feed {
                feed_url: feed.feed_url.clone(),
            },
            InspectorDetail::Track(track_context) => SearchUnsubscribeRequest::Track {
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

        let subscription_command = SearchSubscriptionCommand::Remove;
        frame.subscription_busy = true;
        frame.subscription_message = Some(subscription_command.begin_message().into());
        cx.notify();

        match request {
            SearchUnsubscribeRequest::Feed { feed_url } => {
                let command = UnsubscribeFeedByUrl::new(Arc::clone(&self.conn), feed_url);
                let success_entity_type = entity_type.clone();
                let success_entity_id = entity_id.clone();
                let error_entity_type = entity_type.clone();
                let error_entity_id = entity_id.clone();
                self.command_runner.run(
                    command,
                    CommandContext::next(),
                    cx,
                    move |this, result, cx| {
                        let mut library_mutated = false;
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == success_entity_type
                                && frame.entity_id == success_entity_id
                            {
                                frame.subscription_busy = false;
                                frame.local_subscription = Some(false);
                                frame.subscription_message = Some(result.message().into());
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
            SearchUnsubscribeRequest::Track {
                feed_url,
                item_guid,
                enclosure_url,
            } => {
                let command = RemoveTrackFromLibraryByMatch::new(
                    Arc::clone(&self.conn),
                    feed_url,
                    item_guid,
                    enclosure_url,
                );
                let success_entity_type = entity_type.clone();
                let success_entity_id = entity_id.clone();
                let error_entity_type = entity_type;
                let error_entity_id = entity_id;
                self.command_runner.run(
                    command,
                    CommandContext::next(),
                    cx,
                    move |this, result, cx| {
                        let mut library_mutated = false;
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == success_entity_type
                                && frame.entity_id == success_entity_id
                            {
                                frame.subscription_busy = false;
                                frame.local_subscription = Some(false);
                                frame.subscription_message = Some(result.message().into());
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

    fn add_feed_to_playlist(
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

    fn create_playlist_and_add_feed(
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

    fn create_playlist_and_add_track(&mut self, name: &str, track_id: i64, cx: &mut Context<Self>) {
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

    fn add_track_to_playlist(&mut self, track_id: i64, playlist_id: i64, cx: &mut Context<Self>) {
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

    fn redownload_tag_compare(&mut self, cx: &mut Context<Self>) {
        self.reload_tag_compare(true, cx);
    }

    fn reread_tag_compare(&mut self, cx: &mut Context<Self>) {
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

    fn select_musicbrainz_candidate(&mut self, idx: usize, cx: &mut Context<Self>) {
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
}

impl Render for SearchApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stack = self.inspector_stack.clone();
        let input_has_search_term = normalized_search_query(&self.input.read(cx).value()).is_some();
        let snapshot = self
            .vm
            .render_snapshot(stack.is_empty(), !input_has_search_term);
        let status_text = snapshot.status.display_text.clone();
        let status_color = if snapshot.status.is_error {
            StatusRole::Danger.color(cx)
        } else {
            color::text_muted()
        };
        let status_empty = snapshot.status.is_empty();

        let list_focused = self.list_focus.is_focused(_window);
        let results: Vec<AnyElement> = snapshot
            .rows
            .iter()
            .map(|row| {
                let item = row.render_item();
                let thumbnail = self.thumbnail_for_url(item.display.image_url.as_deref(), cx);
                render_result_item(
                    item,
                    snapshot.selected_key.as_deref(),
                    thumbnail.clone(),
                    list_focused,
                    cx,
                )
            })
            .collect();
        let type_filters: Vec<AnyElement> = SearchViewModel::type_filter_options()
            .iter()
            .map(|option| {
                render_filter_button(
                    option.index,
                    option.label,
                    option.index == snapshot.type_filter,
                    cx,
                )
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
        let active_input = self.input.clone();
        let is_loading = snapshot.loading;
        let is_empty = snapshot.empty;
        let has_more = snapshot.has_more;
        let fuzzy_search = snapshot.fuzzy_search;
        let pane_display = snapshot.pane_display.clone();
        let search_label = pane_display.search_button_label;

        let leading_pane = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(
                div()
                    .p(spacing::MD)
                    .border_b_1()
                    .border_color(color::border_subtle())
                    .flex()
                    .flex_col()
                    .gap(spacing::SM)
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .font_weight(FontWeight::BOLD)
                            .text_color(color::text_muted())
                            .child(pane_display.heading),
                    )
                    .child(
                        Input::new(&active_input)
                            .cleanable(true)
                            .scaled(Size::Small, cx),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(spacing::SM)
                            .children(type_filters),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(spacing::SM)
                            .child(
                                // CONTROL-COMPAT(reason): native Button does not yet expose loading state.
                                Button::new(pane_display.search_button_id)
                                    .label(search_label)
                                    .primary()
                                    .scaled(Size::Small, cx)
                                    .text_color(color::text_on_accent())
                                    .loading(is_loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.do_search(false, cx);
                                    })),
                            )
                            .child(
                                UiButton::styled(
                                    pane_display.fuzzy_toggle_id,
                                    if fuzzy_search {
                                        ControlStyle::Pill
                                    } else {
                                        ControlStyle::Ghost
                                    },
                                )
                                .label(pane_display.fuzzy_toggle_label)
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.toggle_fuzzy_search(cx);
                                    },
                                )),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(status_color)
                            .child(SharedString::from(status_text)),
                    ),
            )
            .child(
                div()
                    .id(pane_display.results_scroll_id)
                    .track_focus(&self.list_focus)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(spacing::SM)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(spacing::XXS)
                            .children(results)
                            .when(is_empty && !is_loading && status_empty, |el| {
                                el.child(
                                    div()
                                        .text_center()
                                        .p(spacing::XXL)
                                        .text_color(color::text_muted())
                                        .child(div().text_2xl().child(pane_display.empty_icon))
                                        .child(
                                            div().mt(spacing::SM).child(pane_display.empty_label),
                                        ),
                                )
                            })
                            .when(is_empty && !is_loading && !status_empty, |el| {
                                el.child(
                                    div()
                                        .text_center()
                                        .p(spacing::XXL)
                                        .text_color(color::text_muted())
                                        .child(div().text_2xl().child(pane_display.empty_icon))
                                        .child(
                                            div().mt(spacing::SM).child(pane_display.empty_label),
                                        ),
                                )
                            })
                            .when(has_more && !is_loading, |el| {
                                el.child(
                                    UiButton::styled(
                                        pane_display.load_more_button_id,
                                        ControlStyle::Ghost,
                                    )
                                    .label(pane_display.load_more_label)
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.do_search(true, cx);
                                        },
                                    )),
                                )
                            }),
                    ),
            )
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

fn fetch_artist_search_batch(
    client: &Client,
    query: &str,
    cursor: Option<&str>,
    fuzzy: bool,
) -> Result<SearchBatch> {
    let response = client.search(query, None, Some(PAGE_LIMIT), cursor, fuzzy)?;
    let rows: Vec<ResultRow> = response
        .data
        .iter()
        .map(|hit| search_hit_to_result_row(client, hit))
        .collect();

    Ok(SearchBatch {
        rows: {
            let mut artist_rows = artist_rows_from_result_rows(&rows, Some(query));
            enrich_artist_rows(client, &mut artist_rows);
            artist_rows
        },
        has_more: response.pagination.has_more,
        cursor: response.pagination.cursor,
    })
}

fn search_hit_to_result_row(client: &Client, hit: &SearchResult) -> ResultRow {
    let detail = client
        .fetch_detail(&hit.entity_type, &hit.entity_id)
        .ok()
        .filter(|detail| {
            matches!(
                detail,
                EntityDetail::Artist(_) | EntityDetail::Feed(_) | EntityDetail::Track(_)
            )
        });
    ResultRow::new(hit.entity_type.clone(), hit.entity_id.clone(), detail)
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

fn should_show_inspector_back(stack_len: usize) -> bool {
    stack_len > 0
}

fn fetch_inspector_detail(
    client: &Client,
    entity_type: &str,
    entity_id: &str,
) -> Result<(InspectorDetail, Option<Arc<Image>>)> {
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
            let image = image_url
                .as_deref()
                .and_then(|url| download_image(&client.client, url))
                .map(image_from_bytes);
            let artist = Artist {
                name: Some(entity_id.to_string()),
                image_url,
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
                image,
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
            let image = feed
                .image_url
                .as_deref()
                .and_then(|url| download_image(&client.client, url))
                .map(image_from_bytes);
            Ok((InspectorDetail::Feed(Box::new(feed)), image))
        }
        "track" => {
            let mut track = client.fetch_track(
                entity_id,
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
            let image = track
                .image_url
                .as_deref()
                .and_then(|url| download_image(&client.client, url))
                .map(image_from_bytes);
            Ok((
                InspectorDetail::Track(Box::new(TrackContext { track, feed })),
                image,
            ))
        }
        "publisher" => Ok((
            InspectorDetail::Publisher(client.fetch_publisher(entity_id)?),
            None,
        )),
        _ => Err(anyhow!("unknown inspector entity type: {entity_type}")),
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
        let Ok(hydrated) = client.fetch_track(&track_guid, Some("source_enclosures")) else {
            continue;
        };
        merge_track_play_fields(track, hydrated);
    }
}

fn merge_track_play_fields(track: &mut Track, hydrated: Track) {
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

fn feed_rss_url(feed: &Feed) -> Option<String> {
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

enum SearchUnsubscribeRequest {
    Feed {
        feed_url: Option<String>,
    },
    Track {
        feed_url: Option<String>,
        item_guid: Option<String>,
        enclosure_url: Option<String>,
    },
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

fn persist_musicindex_artist_facts(
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

fn render_filter_button(
    idx: usize,
    label: &'static str,
    selected: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    UiButton::styled(
        ("type-filter", idx),
        if selected {
            ControlStyle::Pill
        } else {
            ControlStyle::Ghost
        },
    )
    .label(label)
    .on_click(cx.listener(move |this, _, _, cx| {
        if this.vm.set_type_filter_if_changed(idx) {
            let has_query = normalized_search_query(&this.input.read(cx).value()).is_some();
            cx.notify();
            if has_query {
                this.do_search(false, cx);
            }
        }
    }))
    .into_any_element()
}

fn render_result_item(
    item: ResultRowRenderItem,
    selected_key: Option<&str>,
    thumbnail: Option<Arc<Image>>,
    list_focused: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let ResultRowRenderItem {
        selection_key,
        navigation_target,
        display,
    } = item;
    let element_id = display.element_id;
    let line1 = display.line1;
    let line2 = display.line2;
    let line3 = display.line3;
    let kind_label = display.kind_label;
    let is_selected = selected_key == Some(selection_key.as_str());

    let kind = EntityKind::from_legacy_str(&kind_label);

    ListRow::new(SharedString::from(element_id))
        .selected(is_selected)
        .focused(is_selected && list_focused)
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            let (entity_type, entity_id, title) = navigation_target.clone().into_parts();
            this.select_result(entity_type, entity_id, title, cx);
        }))
        .child(Thumbnail::new(kind, ThumbnailSize::Sm).image(thumbnail))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    Label::new(line1)
                        .size(FontSize::Micro)
                        .weight(FontWeight::MEDIUM)
                        .truncated(),
                )
                .when(!line2.is_empty(), |el| {
                    el.child(
                        Label::new(line2)
                            .size(FontSize::Micro)
                            .color(SemanticColor::TertiaryLabel)
                            .truncated(),
                    )
                })
                .when(!line3.is_empty(), |el| {
                    el.child(
                        div().opacity(0.7).child(
                            Label::new(line3)
                                .size(FontSize::Micro)
                                .color(SemanticColor::TertiaryLabel)
                                .truncated(),
                        ),
                    )
                }),
        )
        .child(TagBadge::new(TagBadgeDisplay {
            kind,
            label: Some(SharedString::from(kind_label)),
        }))
        .into_any_element()
}

fn render_inspector(
    frame: Option<&InspectorFrame>,
    show_back: bool,
    show_recents_root: bool,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let chrome = SearchViewModel::inspector_chrome_display();
    let title = SearchViewModel::inspector_title_display(
        show_recents_root,
        frame.map(|frame| frame.title.as_str()),
    );
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .child(
            div()
                .min_h(layout::ROW_HEIGHT)
                .bg(color::bg_surface())
                .border_b_1()
                .border_color(color::border_subtle())
                .px(spacing::MD)
                .py(spacing::SM)
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::SM)
                .when(show_back, |el| {
                    el.child(
                        UiButton::styled(chrome.back_button_id, ControlStyle::Ghost)
                            .label(chrome.back_label)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.inspector_back(cx);
                            })),
                    )
                })
                .child(
                    div().flex_1().child(
                        Label::new(title)
                            .size(FontSize::Micro)
                            .color(SemanticColor::TertiaryLabel)
                            .truncated(),
                    ),
                ),
        )
        .child(
            div()
                .id(chrome.scroll_id)
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p(spacing::LG)
                .child(match frame {
                    Some(frame) => render_inspector_body(frame, app, cx),
                    None if show_recents_root => render_recent_feeds_tiles(app, cx),
                    None => render_inspector_empty(chrome),
                }),
        )
        .into_any_element()
}

fn render_inspector_body(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    match &frame.detail {
        InspectorDetail::Loading(message) => {
            LoadingMessage::new(message.clone()).into_any_element()
        }
        InspectorDetail::Error(error) => {
            LoadingMessage::new(SearchViewModel::inspector_error_message(error)).into_any_element()
        }
        InspectorDetail::Artist(artist_context) => {
            render_artist_inspector(frame, artist_context, app, cx)
        }
        InspectorDetail::Feed(feed) => render_discover_feed_inspector(frame, feed, app, cx),
        InspectorDetail::Track(track_context) => {
            render_discover_track_inspector(frame, track_context, app, cx)
        }
        InspectorDetail::Publisher(publisher) => render_publisher_inspector(publisher, app, cx),
    }
}

fn render_artist_inspector(
    frame: &InspectorFrame,
    artist_context: &ArtistContext,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let view = crate::views::ArtistView::from_api(artist_context.artist.clone());
    let track_count = artist_context
        .artist
        .track_count
        .unwrap_or(artist_context.tracks.len() as i32);

    let feed_section = (!artist_context.feeds.is_empty())
        .then(|| render_feed_list_section(artist_context.feeds.clone(), app, cx));

    artist::render_artist_view(
        &view,
        &artist_context.feeds,
        frame.image.clone(),
        artist_context.has_more_tracks,
        Some(track_count),
        feed_section,
    )
}

fn render_discover_feed_inspector(
    frame: &InspectorFrame,
    feed: &Feed,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let view = crate::views::FeedView::from_api(feed.clone());
    let tracks = SearchViewModel::feed_inspector_tracks(feed);
    let ctx = crate::ui_context::ViewContext::Discover;
    let mut panels = Vec::new();
    if let Some(section) = podroll_section(frame, app, cx) {
        panels.push(section);
    }
    panels.push(render_lazy_sections(frame, app, cx));

    feed::render_feed_view(&view, &tracks, &ctx, frame, panels, app, cx)
}

fn podroll_section(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> Option<AnyElement> {
    let feeds = match &frame.podroll {
        LazyPanel::Loaded(feeds) if !feeds.is_empty() => feeds.clone(),
        _ => return None,
    };

    let mut tiles: Vec<AnyElement> = Vec::with_capacity(feeds.len());
    for feed in feeds {
        let RecentFeedTileDisplay {
            id,
            podroll_tile_id,
            title,
            image_url,
            ..
        } = RecentFeedTileVm::new(&feed).display();
        if id.trim().is_empty() {
            continue;
        }
        let click_title = title.clone();
        let click_guid = id;
        let thumb = app.thumbnail_for_url(image_url.as_deref(), cx);
        let tile = div()
            .id(SharedString::from(podroll_tile_id))
            .flex_shrink_0()
            .w(layout::FEED_TILE_WIDTH)
            .flex()
            .flex_col()
            .gap(spacing::SM)
            .p(spacing::XS)
            .rounded(radius::MD)
            .cursor_pointer()
            .hover(|el| el.bg(color::bg_surface()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.push_inspector("feed".into(), click_guid.clone(), click_title.clone(), cx);
            }))
            .child(Thumbnail::new(EntityKind::Feed, ThumbnailSize::Lg).image(thumb.clone()))
            .child(
                div().line_height(typography::LINE_COMPACT).child(
                    Label::new(title)
                        .size(FontSize::Caption)
                        .weight(FontWeight::MEDIUM)
                        .truncated(),
                ),
            )
            .into_any_element();
        tiles.push(tile);
    }

    if tiles.is_empty() {
        return None;
    }
    let section_display = SearchViewModel::podroll_section_display(&frame.entity_id);

    Some(
        div()
            .flex()
            .flex_col()
            .gap(spacing::SM)
            .child(
                div()
                    .text_size(typography::SIZE_HEADLINE)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(section_display.heading_label),
            )
            .child(
                div()
                    .id(SharedString::from(section_display.scroll_id))
                    .flex()
                    .flex_row()
                    .gap(spacing::MD)
                    .overflow_x_scroll()
                    .pb(spacing::XS)
                    .children(tiles),
            )
            .into_any_element(),
    )
}

fn render_discover_track_inspector(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let track = &track_context.track;
    let vm = TrackVm::new(track);
    let track_view = TrackView::from_api(track.clone());
    let detail_vm = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Discover);
    let header_vm = TrackInspectorHeaderVm::new(track);
    let feed_link = header_vm.feed_link_display();
    let audio_display = vm.play_audio_display();
    let mut external_links = vec![TrackSurfaceElement::from_element(
        render_track_header_subtitle(feed_link, audio_display, cx),
    )];
    external_links.extend(track::render_track_identity_actions(&detail_vm));

    let surface = TrackDetailSurface::new(&detail_vm)
        .image(frame.image.clone())
        .external_links(external_links)
        .primary_actions(vec![TrackSurfaceElement::from_element(
            discover_inspector_action_row(frame, app, cx),
        )])
        .section_elements(vec![TrackSurfaceElement::from_element(
            render_lazy_sections(frame, app, cx),
        )]);

    TrackInspectorPane::new(surface).into_any_element()
}

fn render_publisher_inspector(
    publisher: &Publisher,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let vm = PublisherInspectorVm::new(publisher);

    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(DetailHeader::new(DetailHeaderDisplay {
            kind: EntityKind::Publisher,
            title: vm.title().into(),
            subtitle: None,
            data_rows: Vec::new(),
        }))
        .child(DetailGrid::new(
            vm.detail_rows()
                .into_iter()
                .map(|(k, v)| {
                    CompositeDetailRow::text(CompositeDetailTextRow {
                        key: k.into(),
                        value: v,
                        max_lines: 6,
                    })
                })
                .collect::<Vec<_>>(),
        ))
        .when(vm.has_feed_list(), |el| {
            el.child(render_feed_list_section(vm.feeds(), app, cx))
        })
        .into_any_element()
}

pub(crate) fn discover_inspector_action_row(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let vm = ActionRowVm::new(
        &frame.entity_type,
        frame.subscription_busy,
        frame.local_subscription,
        frame.subscription_message.as_deref(),
    );

    if !vm.is_visible() {
        return div().into_any_element();
    }

    let is_feed = frame.entity_type == "feed";
    let release_target = EntityActionTarget::Feed(FeedRef::Musicindex(frame.entity_id.clone()));
    let release_subscription_action = vm.release_primary_action(release_target.clone());
    let subscription_label = if is_feed {
        release_subscription_action.label.clone()
    } else {
        vm.subscription_button_label()
    };
    let subscription_disabled = if is_feed {
        !release_subscription_action.enabled
    } else {
        frame.subscription_busy
    };
    let release_playlist_action = if is_feed {
        vm.release_playlist_action(release_target)
    } else {
        None
    };
    let playlist_label = vm.playlist_trigger_label(release_playlist_action.as_ref());
    let playlist_disabled = if is_feed {
        frame.subscription_busy
            || release_playlist_action
                .as_ref()
                .is_some_and(|action| !action.enabled)
    } else {
        frame.subscription_busy
    };
    let playlist_target = inspector_playlist_target(frame, app);
    let create_playlist_target = playlist_target.clone();
    let playlists = app.vm.playlists_snapshot();
    let playlist_display = vm.inspector_playlist_display(&frame.entity_id, playlist_label);

    let controls = vec![
        action_button(
            ActionButtonDisplay {
                label: SharedString::from(subscription_label),
            },
            cx,
        )
        .disabled(subscription_disabled)
        .on_click(cx.listener(|this, _, _, cx| {
            this.toggle_local_subscription(cx);
        }))
        .into_any_element(),
        AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(playlist_display.popover_id),
            playlists: playlist_options(&playlists),
            trigger_label: SharedString::from(playlist_display.trigger_label),
        })
        .disabled(playlist_disabled || playlist_target.is_none())
        .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
            if let Some(target) = &playlist_target {
                match target {
                    InspectorPlaylistTarget::Track(track_id) => {
                        this.add_track_to_playlist(*track_id, *playlist_id, cx);
                    }
                    InspectorPlaylistTarget::TrackPending {
                        feed_url,
                        feed_guid,
                        track_guid,
                    } => {
                        this.add_search_track_to_playlist(
                            feed_guid,
                            feed_url.as_deref(),
                            track_guid,
                            *playlist_id,
                            cx,
                        );
                    }
                    InspectorPlaylistTarget::Feed {
                        feed_url,
                        feed_guid,
                    } => {
                        this.add_feed_to_playlist(feed_guid, feed_url.as_deref(), *playlist_id, cx);
                    }
                }
            }
        }))
        .on_create(cx.listener(move |this, name: &String, _window, cx| {
            if let Some(target) = &create_playlist_target {
                match target {
                    InspectorPlaylistTarget::Track(track_id) => {
                        this.create_playlist_and_add_track(name, *track_id, cx);
                    }
                    InspectorPlaylistTarget::TrackPending {
                        feed_url,
                        feed_guid,
                        track_guid,
                    } => {
                        this.create_playlist_and_add_discover_track(
                            name,
                            feed_guid,
                            feed_url.as_deref(),
                            track_guid,
                            cx,
                        );
                    }
                    InspectorPlaylistTarget::Feed {
                        feed_url,
                        feed_guid,
                    } => {
                        this.create_playlist_and_add_feed(name, feed_guid, feed_url.as_deref(), cx);
                    }
                }
            }
        }))
        .into_any_element(),
    ];

    let mut row = ActionRow::new().control_group(controls);

    if let Some(message) = vm.subscription_message_display() {
        row = row.message(ActionRowMessage::from_status_display(message));
    }

    row.into_any_element()
}

#[derive(Clone, Debug)]
enum InspectorPlaylistTarget {
    Track(i64),
    TrackPending {
        feed_url: Option<String>,
        feed_guid: String,
        track_guid: String,
    },
    Feed {
        feed_url: Option<String>,
        feed_guid: String,
    },
}

fn inspector_playlist_target(
    frame: &InspectorFrame,
    app: &SearchApp,
) -> Option<InspectorPlaylistTarget> {
    match (&frame.detail, frame.entity_type.as_str()) {
        (InspectorDetail::Track(track_context), _) => {
            let track = &track_context.track;
            let local_id = if let Ok(conn) = app.conn.lock() {
                library_service::find_track_id(
                    &conn,
                    track.feed_url.as_deref(),
                    track.track_guid.as_deref(),
                    track.enclosure_url.as_deref(),
                )
                .ok()
                .flatten()
            } else {
                None
            };
            match local_id {
                Some(id) => Some(InspectorPlaylistTarget::Track(id)),
                None => match (track.feed_guid.clone(), track.track_guid.clone()) {
                    (Some(fg), Some(tg)) => Some(InspectorPlaylistTarget::TrackPending {
                        feed_url: track.feed_url.clone(),
                        feed_guid: fg,
                        track_guid: tg,
                    }),
                    _ => None,
                },
            }
        }
        (InspectorDetail::Feed(feed), "feed") => Some(InspectorPlaylistTarget::Feed {
            feed_url: feed.feed_url.clone(),
            feed_guid: frame.entity_id.clone(),
        }),
        _ => None,
    }
}

fn playlist_options(playlists: &[db::Playlist]) -> Vec<PlaylistOption> {
    playlist_option_displays(playlists)
        .into_iter()
        .map(|option| {
            PlaylistOption::new(PlaylistOptionDisplay {
                id: option.id,
                name: SharedString::from(option.name),
            })
        })
        .collect()
}

fn render_lazy_sections(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    render_rss_lazy_sections(frame, app, cx)
}

fn render_rss_lazy_sections(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(render_lazy_contributors(frame, app, cx))
        .child(render_lazy_value_routes(frame, cx))
        .into_any_element()
}

fn render_lazy_contributors(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let collapsed = frame.contributors_collapsed || matches!(frame.contributors, LazyPanel::Hidden);

    div()
        .flex()
        .flex_col()
        .gap(spacing::XS)
        .child(render_contributors_heading(collapsed, cx))
        .when(!collapsed, |el| match &frame.contributors {
            LazyPanel::Loaded(items) => el.children(contributor_elements(items, app, cx)),
            LazyPanel::Loading => el.child(LoadingMessage::new(
                SearchViewModel::deferred_panel_display(DeferredPanelKind::Contributors)
                    .loading_label,
            )),
            LazyPanel::Empty(label) => el.child(muted_line(
                SearchViewModel::deferred_panel_empty_line(label),
            )),
            LazyPanel::Hidden => el,
        })
        .into_any_element()
}

fn render_lazy_value_routes(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    let collapsed = frame.value_routes_collapsed || matches!(frame.value_routes, LazyPanel::Hidden);

    div()
        .flex()
        .flex_col()
        .gap(spacing::XS)
        .child(render_value_routes_heading(collapsed, cx))
        .when(!collapsed, |el| match &frame.value_routes {
            LazyPanel::Loaded(items) => el.children(value_route_elements(items)),
            LazyPanel::Loading => el.child(LoadingMessage::new(
                SearchViewModel::deferred_panel_display(DeferredPanelKind::ValueRoutes)
                    .loading_label,
            )),
            LazyPanel::Empty(label) => el.child(muted_line(
                SearchViewModel::deferred_panel_empty_line(label),
            )),
            LazyPanel::Hidden => el,
        })
        .into_any_element()
}

fn contributor_elements(
    contributors: &[ContributorView],
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> Vec<AnyElement> {
    render_contributor_rows(
        ContributorListVm::new(contributors, EntitySurfaceContext::Discover),
        |contributor| {
            let thumbnail = app.thumbnail_for_url(contributor.image_url(), cx);
            ContributorRowSlot {
                thumbnail,
                actions: contributor_identity_actions(contributor),
            }
        },
    )
}

fn contributor_identity_actions(contributor: &ContributorRowVm<'_>) -> Vec<ReleaseSurfaceElement> {
    contributor
        .identity_actions()
        .into_iter()
        .map(|action| {
            let target_for_click = action.target.clone();
            match action.kind {
                ContributorIdentityActionKind::Website => identity_action_button(
                    SharedString::from(action.id),
                    IdentityActionKind::Website,
                )
                .on_click(move |_, _, _| {
                    let _ = open::that(&target_for_click);
                })
                .into_any_element(),
                ContributorIdentityActionKind::Nostr => {
                    identity_action_button(SharedString::from(action.id), IdentityActionKind::Nostr)
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                target_for_click.clone(),
                            ));
                        })
                        .into_any_element()
                }
            }
        })
        .map(ReleaseSurfaceElement::from_element)
        .collect()
}

fn value_route_elements(routes: &[PaymentRoute]) -> Vec<AnyElement> {
    let mut groups = BTreeMap::<&'static str, Vec<&PaymentRoute>>::new();
    for route in routes {
        let group = PaymentRouteVm::new(route).group();
        groups.entry(group).or_default().push(route);
    }

    groups
        .into_iter()
        .flat_map(|(group, routes)| {
            let group_display = PaymentRouteVm::group_display(group);
            let mut elements = vec![group_heading(group_display.heading)];
            elements.extend(routes.into_iter().map(|route| {
                let vm = PaymentRouteVm::new(route);
                let summary = vm.summary();
                let address = vm.address();
                let custom_fields = vm.custom_fields();
                div()
                    .flex()
                    .flex_col()
                    .gap(spacing::XXS)
                    .text_size(typography::SIZE_MICRO)
                    .child(SharedString::from(summary))
                    .when_some(address, |el, address| {
                        el.child(
                            div()
                                .text_color(color::text_muted())
                                .text_size(typography::SIZE_MICRO)
                                .line_clamp(2)
                                .child(SharedString::from(address)),
                        )
                    })
                    .when_some(custom_fields, |el, custom_fields| {
                        el.child(
                            div()
                                .text_color(color::text_muted())
                                .text_size(typography::SIZE_MICRO)
                                .child(SharedString::from(custom_fields)),
                        )
                    })
                    .into_any_element()
            }));
            elements
        })
        .collect()
}

fn render_contributors_heading(collapsed: bool, cx: &mut Context<SearchApp>) -> AnyElement {
    let display = SearchViewModel::deferred_panel_display(DeferredPanelKind::Contributors);
    DisclosureGroup::new(DisclosureGroupDisplay {
        id: display.section_id.into(),
        label: display.heading_label.into(),
    })
    .collapsed(collapsed)
    .on_toggle(cx.listener(|this, _, _, cx| {
        this.toggle_contributors(cx);
    }))
    .into_any_element()
}

fn render_value_routes_heading(collapsed: bool, cx: &mut Context<SearchApp>) -> AnyElement {
    let display = SearchViewModel::deferred_panel_display(DeferredPanelKind::ValueRoutes);
    DisclosureGroup::new(DisclosureGroupDisplay {
        id: display.section_id.into(),
        label: display.heading_label.into(),
    })
    .collapsed(collapsed)
    .on_toggle(cx.listener(|this, _, _, cx| {
        this.toggle_value_routes(cx);
    }))
    .into_any_element()
}

fn track_metadata_rows_for_frame(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
) -> Vec<MetadataGridRow> {
    let selected_musicbrainz = match &frame.musicbrainz_lookup {
        LazyPanel::Loaded(lookup) => selected_musicbrainz_candidate(frame, lookup),
        _ => None,
    };
    let show_musicbrainz = track_metadata_action_state(frame).show_musicbrainz_panel();
    let rows = result.map_or_else(
        || track_metadata_rows(track_context, selected_musicbrainz, show_musicbrainz),
        |result| {
            aligned_compare_rows(
                result,
                track_context,
                selected_musicbrainz,
                show_musicbrainz,
                &frame.expanded_id3_frame_groups,
            )
        },
    );
    expand_woar_metadata_rows(rows)
}

fn track_metadata_action_state(frame: &InspectorFrame) -> TrackMetadataActionState {
    TrackMetadataActionState::new(
        EntitySurfaceContext::Discover,
        metadata_panel_state(&frame.tag_compare),
        metadata_panel_state(&frame.musicbrainz_lookup),
        frame.entity_type == "track",
    )
}

fn metadata_panel_state<T>(panel: &LazyPanel<T>) -> MetadataPanelState {
    match panel {
        LazyPanel::Hidden => MetadataPanelState::Hidden,
        LazyPanel::Loading => MetadataPanelState::Loading,
        LazyPanel::Loaded(_) => MetadataPanelState::Loaded,
        LazyPanel::Empty(_) => MetadataPanelState::Empty,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "metadata grid needs explicit column state and edit state inputs"
)]
fn discover_track_metadata_grid(
    rows: Vec<MetadataGridRow>,
    show_id3: bool,
    show_musicbrainz: bool,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    expanded_metadata_cells: &BTreeSet<String>,
    file_image: Option<Arc<Image>>,
    tag_column_label: &str,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let vm = TrackMetadataGridVm::new(show_id3, show_musicbrainz, tag_column_label);
    let mut cells: Vec<AnyElement> = Vec::new();

    for row in rows {
        match row {
            MetadataGridRow::Group(group) => {
                cells.push(metadata_group_cell(group, vm.columns(), cx));
            }
            MetadataGridRow::Data(row) => {
                let pending = pending_id3_edits.get(&row.row_id);
                let expansion = vm.expansion_for(&row.row_id, expanded_metadata_cells);
                cells.push(metadata_rss_cell(
                    &row,
                    pending,
                    expansion.rss_expanded,
                    expanded_metadata_cells,
                    cx,
                ));
                if show_id3 {
                    cells.push(metadata_id3_cell(
                        &row,
                        pending,
                        expansion.id3_expanded,
                        expanded_metadata_cells,
                        file_image.clone(),
                        cx,
                    ));
                }
                if show_musicbrainz {
                    cells.push(metadata_musicbrainz_cell(&row, pending, cx));
                }
            }
        }
    }

    TrackMetadataGrid::new(vm).cells(cells).into_any_element()
}

fn metadata_rss_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let value = TrackMetadataGridVm::rss_cell_value(row.rss_value.as_deref());
    let base_display = display_metadata_value(&row.field, value);
    let source_role = pending.and_then(|edit| {
        TrackMetadataGridVm::pending_source_role(
            edit.source,
            &edit.value,
            MetadataColumn::Rss,
            row.rss_value.as_deref(),
        )
        .map(ProvenanceRole::from)
    });
    let value_color = source_role
        .map(|role| role.color(cx))
        .unwrap_or_else(color::text_primary);
    let glyph = source_role.map(ProvenanceRole::glyph);
    let display_value = TrackMetadataGridVm::display_with_glyph(glyph, &base_display);
    let expandable = TrackMetadataGridVm::field_is_expandable(&row.field, value);
    let value_element = if expandable {
        expandable_cell(
            ExpandableCellParams {
                field: &row.field,
                row_id: &row.row_id,
                raw_value: value,
                display_value: &display_value,
                expanded,
                color: value_color,
            },
            expanded_cells,
            cx,
        )
    } else {
        compare_cell(&display_value, Some(value_color))
    };
    let cell = div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::SM)
        .child(
            div()
                .w(layout::COMPACT_COLUMN_WIDTH)
                .flex_shrink_0()
                .text_color(color::text_primary())
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
                .child(SharedString::from(TrackMetadataGridVm::field_label(
                    &row.field,
                ))),
        )
        .child(div().flex_1().min_w_0().child(value_element));
    if !expandable {
        if let Some(drag) = metadata_drag_value(row, MetadataColumn::Rss) {
            let display =
                TrackMetadataGridVm::source_drag_display(MetadataColumn::Rss, &row.row_id);
            return cell
                .id(SharedString::from(display.cell_id))
                .cursor_move()
                .hover(|style| style.bg(color::bg_surface()))
                .on_drag(
                    drag,
                    |drag: &MetadataDragValue, _position: Point<Pixels>, _window, cx: &mut App| {
                        let display =
                            TrackMetadataGridVm::drag_preview_display(&drag.field, &drag.value);
                        cx.new(|_| MetadataDragPreview {
                            label: display.label,
                            value: display.value,
                        })
                    },
                )
                .into_any_element();
        }
    }
    cell.into_any_element()
}

fn metadata_id3_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    file_image: Option<Arc<Image>>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let frame = TrackMetadataGridVm::id3_cell_frame(
        pending.map(|edit| edit.frame.as_str()),
        row.id3_frame.as_deref(),
    );
    let value = TrackMetadataGridVm::id3_cell_value(
        pending.map(|edit| edit.value.as_str()),
        row.id3_value.as_deref(),
    );
    let display_value = display_metadata_value(&row.field, value);
    let color = pending
        .map(|edit| pending_source_color(edit.source, cx))
        .unwrap_or_else(|| id3_cell_status_color(row, cx));
    let frame_color = frame.map(|frame| {
        id3_frame_color(TrackMetadataGridVm::id3_frame_color_role(
            Some(frame),
            TrackMetadataId3FrameColorContext::Discover,
        ))
    });
    let expandable = TrackMetadataGridVm::field_is_expandable(&row.field, value);
    let value_element = if expandable {
        expandable_tag_cell(
            ExpandableTagCellParams {
                base: ExpandableCellParams {
                    field: &row.field,
                    row_id: &row.row_id,
                    raw_value: value,
                    display_value: &display_value,
                    expanded,
                    color,
                },
                frame_id: frame,
                frame_color,
                file_image,
            },
            expanded_cells,
            cx,
        )
    } else {
        compare_tag_cell(&display_value, Some(color), frame, frame_color)
    };
    let mut cell = div()
        .pl(spacing::MD)
        .min_w_0()
        .rounded(radius::SM)
        .child(value_element)
        .when_some(pending, |el, edit| {
            el.border_1()
                .border_color(pending_source_color(edit.source, cx))
        });
    if pending.is_some() {
        let row_id = row.row_id.clone();
        cell = cell.on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.revert_pending_id3_edit(row_id.clone(), cx);
            }),
        );
    }

    if let Some(frame) = frame.filter(|frame| id3v24_drag_copy_frame_is_writable(frame)) {
        let row_id = row.row_id.clone();
        let target_field = row.field.clone();
        let target_frame = frame.to_string();
        let target_existing_value = (!value.is_empty()).then(|| value.to_string());
        cell = cell
            .can_drop(|drag, _window, _cx| drag.downcast_ref::<MetadataDragValue>().is_some())
            .hover(|style| style.bg(color::bg_surface()))
            .on_drop(
                cx.listener(move |this, drag: &MetadataDragValue, _window, cx| {
                    let mut drag = drag.clone();
                    drag.row_id = row_id.clone();
                    drag.field = target_field.clone();
                    drag.frame = target_frame.clone();
                    drag.target_existing_value = target_existing_value.clone();
                    this.stage_id3_drag_copy(&drag, cx);
                }),
            );
    }
    cell.into_any_element()
}

fn metadata_musicbrainz_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let source_role = pending.and_then(|edit| {
        TrackMetadataGridVm::pending_source_role(
            edit.source,
            &edit.value,
            MetadataColumn::MusicBrainz,
            row.musicbrainz_value.as_deref(),
        )
        .map(ProvenanceRole::from)
    });
    let musicbrainz_color = source_role
        .map(|role| role.color(cx))
        .unwrap_or_else(|| comparison_status_color(&row.musicbrainz_status, cx));
    let glyph = source_role.map_or_else(
        || TrackMetadataGridVm::comparison_glyph(&row.musicbrainz_status),
        |role| Some(role.glyph()),
    );
    let value = TrackMetadataGridVm::musicbrainz_cell_value(row.musicbrainz_value.as_deref());
    let display_value = display_metadata_value(&row.field, value);
    let display_value = TrackMetadataGridVm::display_with_glyph(glyph, &display_value);
    let cell = div().pl(spacing::MD).min_w_0().child(compare_tag_cell(
        &display_value,
        Some(musicbrainz_color),
        row.musicbrainz_key.as_deref(),
        None,
    ));
    if let Some(drag) = metadata_drag_value(row, MetadataColumn::MusicBrainz) {
        let display =
            TrackMetadataGridVm::source_drag_display(MetadataColumn::MusicBrainz, &row.row_id);
        cell.id(SharedString::from(display.cell_id))
            .cursor_move()
            .hover(|style| style.bg(color::bg_surface()))
            .on_drag(
                drag,
                |drag: &MetadataDragValue, _position: Point<Pixels>, _window, cx: &mut App| {
                    let display =
                        TrackMetadataGridVm::drag_preview_display(&drag.field, &drag.value);
                    cx.new(|_| MetadataDragPreview {
                        label: display.label,
                        value: display.value,
                    })
                },
            )
            .into_any_element()
    } else {
        cell.into_any_element()
    }
}

fn metadata_drag_value(
    row: &AlignedCompareRow,
    source: MetadataColumn,
) -> Option<MetadataDragValue> {
    let value = match source {
        MetadataColumn::Rss => row.rss_value.as_ref(),
        MetadataColumn::MusicBrainz => row.musicbrainz_value.as_ref(),
    }?;
    let value = normalized_compare_value(Some(value))?;
    Some(MetadataDragValue {
        row_id: row.row_id.clone(),
        field: TrackMetadataGridVm::field_label(&row.field),
        frame: TrackMetadataGridVm::id3_drag_frame(row.id3_frame.as_deref()),
        target_existing_value: None,
        value,
        source,
    })
}

fn pending_source_color(source: MetadataColumn, cx: &mut Context<SearchApp>) -> gpui::Rgba {
    match source {
        MetadataColumn::Rss | MetadataColumn::MusicBrainz => ProvenanceRole::Match.color(cx),
    }
}

fn metadata_group_cell(
    group: MetadataGroupRow,
    columns: u16,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let group_key = group.key;
    let display = TrackMetadataGridVm::group_heading_display(
        &group.label,
        group.unused_count,
        group_key.as_deref(),
    );

    let expanded = group.expanded;
    let mut cell = div().col_span(columns).mt(spacing::SM);
    if let (Some(group_key), Some(disclosure_id)) = (group_key, display.disclosure_id) {
        cell = cell.child(
            DisclosureGroup::new(DisclosureGroupDisplay {
                id: SharedString::from(disclosure_id).into(),
                label: SharedString::from(display.label),
            })
            .collapsed(!expanded)
            .on_toggle(cx.listener(move |this, _, _, cx| {
                this.toggle_id3_frame_group(group_key.clone(), cx);
            })),
        );
    } else {
        cell = cell.child(
            div()
                .text_size(typography::SIZE_MICRO)
                .font_weight(FontWeight::BOLD)
                .text_color(color::text_muted())
                .child(SharedString::from(display.label)),
        );
    }
    cell.into_any_element()
}

#[cfg(test)]
#[allow(dead_code)]
fn metadata_group_row(
    label: impl Into<String>,
    key: Option<&str>,
    expanded: bool,
    unused_count: usize,
) -> MetadataGridRow {
    MetadataGridRow::Group(MetadataGroupRow {
        key: key.map(str::to_string),
        label: label.into(),
        expanded,
        unused_count,
    })
}

#[cfg(test)]
fn metadata_data_row(row: AlignedCompareRow) -> MetadataGridRow {
    MetadataGridRow::Data(row)
}

#[cfg(test)]
#[allow(dead_code)]
fn id3_unused_frame_row(frame_id: &str) -> MetadataGridRow {
    metadata_data_row(AlignedCompareRow {
        row_id: TrackMetadataGridVm::unused_id3_frame_row_id(frame_id),
        field: TrackMetadataGridVm::id3_field_display_label(frame_id),
        rss_value: None,
        id3_value: None,
        id3_frame: Some(frame_id.into()),
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingBoth,
        musicbrainz_status: ComparisonStatus::MissingBoth,
    })
}

#[cfg(test)]
#[allow(dead_code)]
fn used_id3_field_row(field: &Id3Field) -> MetadataGridRow {
    metadata_data_row(AlignedCompareRow {
        row_id: TrackMetadataGridVm::used_id3_field_row_id(&field.frame_id),
        field: TrackMetadataGridVm::id3_field_display_label(&field.frame_id),
        rss_value: None,
        id3_value: Some(field.value.clone()),
        id3_frame: Some(field.frame_id.clone()),
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingSource,
        musicbrainz_status: ComparisonStatus::MissingBoth,
    })
}

#[cfg(test)]
fn unused_id3v24_frames_for_group(result: &TagCompareResult, group_key: &str) -> Vec<&'static str> {
    ID3V24_FRAME_IDS
        .iter()
        .copied()
        .filter(|frame_id| id3_frame_group_key(frame_id) == group_key)
        .filter(|frame_id| {
            !result
                .id3_fields
                .iter()
                .any(|field| id3_frame_base(&field.frame_id) == *frame_id)
        })
        .collect()
}

#[cfg(test)]
#[allow(dead_code)]
fn used_id3_fields_for_group<'a>(
    result: &'a TagCompareResult,
    group_key: &str,
    aligned_frame_ids: &BTreeSet<String>,
) -> Vec<&'a Id3Field> {
    result
        .id3_fields
        .iter()
        .filter(|field| !id3_frame_is_summarized(&field.frame_id))
        .filter(|field| !aligned_frame_ids.contains(&pending_id3_target_key(&field.frame_id)))
        .filter(|field| id3_frame_group_key(&field.frame_id) == group_key)
        .collect()
}

fn selected_musicbrainz_candidate<'a>(
    frame: &InspectorFrame,
    result: &'a MusicBrainzLookupResult,
) -> Option<&'a MusicBrainzCandidate> {
    result
        .lookup
        .candidates
        .get(frame.musicbrainz_selected)
        .or_else(|| result.lookup.candidates.first())
}

fn muted_line(display_text: String) -> AnyElement {
    div()
        .text_color(color::text_muted())
        .text_size(typography::SIZE_MICRO)
        .child(SharedString::from(display_text))
        .into_any_element()
}

struct ExpandableCellParams<'a> {
    field: &'a str,
    row_id: &'a str,
    raw_value: &'a str,
    display_value: &'a str,
    expanded: bool,
    color: gpui::Rgba,
}

struct ExpandableTagCellParams<'a> {
    base: ExpandableCellParams<'a>,
    frame_id: Option<&'a str>,
    frame_color: Option<Rgba>,
    file_image: Option<Arc<Image>>,
}

fn expandable_cell(
    params: ExpandableCellParams<'_>,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let ExpandableCellParams {
        field,
        row_id,
        raw_value,
        display_value,
        expanded,
        color,
    } = params;
    let display =
        TrackMetadataGridVm::discover_expandable_cell_display("rss", field, row_id, expanded);
    let glyph = display.disclosure_glyph;
    let field_kind = TrackMetadataGridVm::expanded_field_kind(field);

    if expanded && field_kind == TrackMetadataExpandedFieldKind::ValueRoutes {
        let cell_key_h = display.cell_key.clone();
        return div()
            .text_size(typography::SIZE_MICRO)
            .line_height(typography::LINE_BODY)
            .text_color(color)
            .flex()
            .flex_col()
            .child(
                div()
                    .id(SharedString::from(display.header_id))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.toggle_metadata_cell(cell_key_h.clone(), cx);
                    }))
                    .flex()
                    .flex_row()
                    .gap(spacing::XS)
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .text_color(color::text_muted())
                            .child(glyph),
                    ),
            )
            .children(value_routes_tree_elements(
                raw_value,
                "rss",
                row_id,
                color,
                expanded_cells,
                cx,
            ))
            .into_any_element();
    }

    let cell_key = display.cell_key.clone();
    let mut container = div()
        .id(SharedString::from(display.cell_id))
        .cursor_pointer()
        .text_size(typography::SIZE_MICRO)
        .line_height(typography::LINE_BODY)
        .text_color(color)
        .flex()
        .flex_col()
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }));

    if expanded {
        match TrackMetadataGridVm::expanded_field_kind(field) {
            TrackMetadataExpandedFieldKind::Artwork
                if TrackMetadataGridVm::artwork_url(raw_value).is_some() =>
            {
                let url = raw_value.to_string();
                container = container
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(spacing::XS)
                            .child(
                                div()
                                    .text_size(typography::SIZE_MICRO)
                                    .text_color(color::text_muted())
                                    .child(glyph),
                            )
                            .child(div().text_color(color::accent()).truncate().child(
                                SharedString::from(TrackMetadataGridVm::artwork_url_display(
                                    raw_value,
                                )),
                            )),
                    )
                    .on_mouse_down(
                        MouseButton::Middle,
                        move |_: &MouseDownEvent, _window, _cx| {
                            let _ = open::that(&url);
                        },
                    );
            }
            TrackMetadataExpandedFieldKind::Transcript => {
                container = container.child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(spacing::XS)
                        .items_start()
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .text_color(color::text_muted())
                                .child(glyph),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .children(transcript_text_elements(raw_value, color)),
                        ),
                );
            }
            TrackMetadataExpandedFieldKind::Artwork
            | TrackMetadataExpandedFieldKind::Text
            | TrackMetadataExpandedFieldKind::ValueRoutes => {
                let expanded_display =
                    TrackMetadataGridVm::expanded_display_value(field, raw_value, display_value);
                container =
                    container.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(spacing::XS)
                            .items_start()
                            .child(
                                div()
                                    .text_size(typography::SIZE_MICRO)
                                    .text_color(color::text_muted())
                                    .child(glyph),
                            )
                            .child(
                                div().flex_1().min_w_0().flex().flex_col().children(
                                    json_tree_elements(raw_value, &expanded_display, color),
                                ),
                            ),
                    );
            }
        }
    } else {
        let summary = TrackMetadataGridVm::expandable_cell_summary(
            field,
            raw_value,
            display_value,
            ValueRoutesSummaryFallback::MultilineCount,
        );
        container = container.child(
            div()
                .flex()
                .flex_row()
                .gap(spacing::XS)
                .child(
                    div()
                        .text_size(typography::SIZE_MICRO)
                        .text_color(color::text_muted())
                        .child(glyph),
                )
                .child(
                    div()
                        .text_color(color::accent())
                        .truncate()
                        .child(SharedString::from(summary)),
                ),
        );
    }
    container.into_any_element()
}

fn expandable_tag_cell(
    params: ExpandableTagCellParams<'_>,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let ExpandableTagCellParams {
        base:
            ExpandableCellParams {
                field,
                row_id,
                raw_value,
                display_value,
                expanded,
                color,
            },
        frame_id,
        frame_color,
        file_image,
    } = params;
    let display =
        TrackMetadataGridVm::discover_expandable_cell_display("id3", field, row_id, expanded);
    let glyph = display.disclosure_glyph;
    let frame_color = frame_color.unwrap_or_else(color::text_muted);
    let frame_label = TrackMetadataGridVm::id3_frame_display_label(frame_id);
    let field_kind = TrackMetadataGridVm::expanded_field_kind(field);

    let value_el = if expanded {
        match field_kind {
            TrackMetadataExpandedFieldKind::Artwork => {
                if let Some(image) = file_image {
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(spacing::XS)
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .line_height(typography::LINE_BODY)
                                .text_color(color)
                                .child(SharedString::from(
                                    TrackMetadataGridVm::text_value_display(display_value),
                                )),
                        )
                        .child(
                            ImagePrimitive::new(image.clone())
                                .size(ImageSize::XXl)
                                .radius(Radius::MD),
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(typography::SIZE_MICRO)
                        .line_height(typography::LINE_BODY)
                        .text_color(color)
                        .child(SharedString::from(TrackMetadataGridVm::text_value_display(
                            display_value,
                        )))
                        .into_any_element()
                }
            }
            TrackMetadataExpandedFieldKind::Transcript => div()
                .flex_1()
                .min_w_0()
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
                .text_color(color)
                .flex()
                .flex_col()
                .children(transcript_text_elements(raw_value, color))
                .into_any_element(),
            TrackMetadataExpandedFieldKind::Text | TrackMetadataExpandedFieldKind::ValueRoutes => {
                let expanded_display =
                    TrackMetadataGridVm::expanded_display_value(field, raw_value, display_value);
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(typography::SIZE_MICRO)
                    .line_height(typography::LINE_BODY)
                    .text_color(color)
                    .flex()
                    .flex_col()
                    .children(json_tree_elements(raw_value, &expanded_display, color))
                    .into_any_element()
            }
        }
    } else {
        let summary = TrackMetadataGridVm::expandable_cell_summary(
            field,
            raw_value,
            display_value,
            ValueRoutesSummaryFallback::MultilineCount,
        );
        div()
            .flex_1()
            .min_w_0()
            .text_size(typography::SIZE_MICRO)
            .line_height(typography::LINE_BODY)
            .flex()
            .flex_row()
            .gap(spacing::XS)
            .child(
                div()
                    .text_size(typography::SIZE_MICRO)
                    .text_color(color::text_muted())
                    .child(glyph),
            )
            .child(
                div()
                    .text_color(color::accent())
                    .truncate()
                    .child(SharedString::from(summary)),
            )
            .into_any_element()
    };

    // Value Routes when expanded: separate header click from sub-item clicks
    if expanded && field_kind == TrackMetadataExpandedFieldKind::ValueRoutes {
        let cell_key = display.cell_key.clone();
        return div()
            .flex()
            .flex_col()
            .text_size(typography::SIZE_MICRO)
            .line_height(typography::LINE_BODY)
            .text_color(color)
            .child(
                div()
                    .id(SharedString::from(display.header_id))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.toggle_metadata_cell(cell_key.clone(), cx);
                    }))
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(spacing::SM)
                    .child(
                        div()
                            .w(layout::METADATA_LABEL_WIDTH)
                            .flex_shrink_0()
                            .text_color(frame_color)
                            .text_size(typography::SIZE_MICRO)
                            .line_height(typography::LINE_BODY)
                            .child(SharedString::from(frame_label.clone())),
                    )
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .text_color(color::text_muted())
                            .child(glyph),
                    ),
            )
            .child(
                div()
                    .pl(layout::METADATA_VALUE_INDENT)
                    .flex()
                    .flex_col()
                    .children(value_routes_tree_elements(
                        raw_value,
                        "id3",
                        row_id,
                        color,
                        expanded_cells,
                        cx,
                    )),
            )
            .into_any_element();
    }

    let cell_key = display.cell_key.clone();
    div()
        .id(SharedString::from(display.cell_id))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }))
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::SM)
        .child(
            div()
                .w(layout::METADATA_LABEL_WIDTH)
                .flex_shrink_0()
                .text_color(frame_color)
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
                .child(SharedString::from(frame_label)),
        )
        .child(value_el)
        .into_any_element()
}

fn json_tree_elements(raw_value: &str, display_value: &str, color: gpui::Rgba) -> Vec<AnyElement> {
    // Try to parse as JSON array and render structured tree
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) {
        return arr
            .into_iter()
            .map(|item| json_object_element(&item, color, 0))
            .collect();
    }
    // Fall back to showing all lines of the display value
    display_value
        .lines()
        .map(|line| {
            let line = TrackMetadataGridVm::transcript_line_display(line);
            div()
                .truncate()
                .child(SharedString::from(TrackMetadataGridVm::text_value_display(
                    line,
                )))
                .into_any_element()
        })
        .collect()
}

fn json_object_element(value: &serde_json::Value, color: gpui::Rgba, depth: usize) -> AnyElement {
    let indent = px((depth * 12) as f32);
    match value {
        serde_json::Value::Object(map) => {
            let mut container = div()
                .flex()
                .flex_col()
                .pl(indent)
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY);
            for (key, val) in map {
                let key_str = TrackMetadataGridVm::value_route_field_key_label(key);
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        container = container
                            .child(
                                div()
                                    .text_color(color::text_muted())
                                    .child(SharedString::from(key_str)),
                            )
                            .child(json_object_element(val, color, depth + 1));
                    }
                    _ => {
                        let val_str = TrackMetadataGridVm::json_tree_scalar_label(val)
                            .expect("object and array values are handled before scalar display");
                        container = container.child(
                            div().flex().flex_row().gap(spacing::XS).child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .child(
                                        div()
                                            .text_color(color::text_muted())
                                            .child(SharedString::from(key_str)),
                                    )
                                    .child(
                                        div()
                                            .text_color(color)
                                            .truncate()
                                            .child(SharedString::from(val_str)),
                                    ),
                            ),
                        );
                    }
                }
            }
            container.into_any_element()
        }
        serde_json::Value::Array(arr) => {
            let mut container = div().flex().flex_col().pl(indent);
            for item in arr {
                container = container.child(json_object_element(item, color, depth));
            }
            container.into_any_element()
        }
        _ => {
            let text = TrackMetadataGridVm::json_tree_scalar_label(value)
                .expect("object and array values are handled before scalar display");
            div()
                .pl(indent)
                .text_color(color)
                .truncate()
                .child(SharedString::from(text))
                .into_any_element()
        }
    }
}

fn value_routes_tree_elements(
    raw_value: &str,
    column: &str,
    row_id: &str,
    color: gpui::Rgba,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> Vec<AnyElement> {
    let Ok(routes) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) else {
        return json_tree_elements(raw_value, raw_value, color);
    };
    routes
        .into_iter()
        .enumerate()
        .map(|(i, route)| {
            let name = value_route_recipient_label(&route);
            let label = TrackMetadataGridVm::value_route_item_label(&name, None);
            let item_key = TrackMetadataGridVm::value_route_item_key(column, row_id, i);
            let display = TrackMetadataGridVm::discover_value_route_item_display(
                column,
                row_id,
                i,
                expanded_cells.contains(&item_key),
            );
            let sub_expanded = expanded_cells.contains(&display.item_key);

            let mut item = div()
                .id(SharedString::from(display.item_id))
                .cursor_pointer()
                .flex()
                .flex_col();

            let sub_key_click = display.item_key.clone();
            item = item.on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.toggle_metadata_cell(sub_key_click.clone(), cx);
            }));

            item = item.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(spacing::XS)
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .text_color(color::text_muted())
                            .child(display.disclosure_glyph),
                    )
                    .child(
                        div()
                            .text_color(if sub_expanded { color } else { color::accent() })
                            .child(SharedString::from(label)),
                    ),
            );

            if sub_expanded {
                if let serde_json::Value::Object(map) = &route {
                    for (key, val) in map {
                        if !TrackMetadataGridVm::value_route_child_field_is_visible(
                            key,
                            ValueRouteFieldContext::Discover,
                        ) {
                            continue;
                        }
                        let Some(value) = TrackMetadataGridVm::value_route_field_value_label(val)
                        else {
                            continue;
                        };
                        let key_label = TrackMetadataGridVm::value_route_field_key_label(key);
                        item = item.child(
                            div()
                                .pl(spacing::LG)
                                .flex()
                                .flex_row()
                                .gap(spacing::XS)
                                .child(
                                    div()
                                        .text_color(color::text_muted())
                                        .child(SharedString::from(key_label)),
                                )
                                .child(
                                    div()
                                        .text_color(color)
                                        .truncate()
                                        .child(SharedString::from(value)),
                                ),
                        );
                    }
                }
            }

            item.into_any_element()
        })
        .collect()
}

fn transcript_text_elements(raw_value: &str, color: gpui::Rgba) -> Vec<AnyElement> {
    raw_value
        .lines()
        .map(|line| {
            let line = TrackMetadataGridVm::transcript_line_display(line);
            div()
                .text_color(color)
                .child(SharedString::from(TrackMetadataGridVm::text_value_display(
                    line,
                )))
                .into_any_element()
        })
        .collect()
}

fn compare_cell(value: &str, color: Option<gpui::Rgba>) -> AnyElement {
    let mut cell = MultilineText::new(TrackMetadataGridVm::text_value_display(value))
        .max_lines(4)
        .size(FontSize::Micro)
        .line_height(typography::LINE_BODY);
    if let Some(color) = color {
        cell = cell.color_raw(color);
    }
    cell.into_any_element()
}

fn compare_tag_cell(
    value: &str,
    color: Option<gpui::Rgba>,
    frame_id: Option<&str>,
    frame_color: Option<gpui::Rgba>,
) -> AnyElement {
    let frame_label = TrackMetadataGridVm::id3_frame_display_label(frame_id);
    let frame_color = frame_color.unwrap_or_else(color::text_muted);

    let mut body = MultilineText::new(TrackMetadataGridVm::text_value_display(value))
        .max_lines(4)
        .size(FontSize::Micro)
        .line_height(typography::LINE_BODY);
    if let Some(color) = color {
        body = body.color_raw(color);
    }

    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::SM)
        .child(
            div()
                .w(layout::METADATA_LABEL_WIDTH)
                .flex_shrink_0()
                .text_color(frame_color)
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
                .child(SharedString::from(frame_label)),
        )
        .child(div().flex_1().min_w_0().child(body))
        .into_any_element()
}

fn id3_frame_color(role: TrackMetadataId3FrameColorRole) -> gpui::Rgba {
    match role {
        TrackMetadataId3FrameColorRole::Muted => color::text_muted(),
        TrackMetadataId3FrameColorRole::Accent => color::accent(),
        TrackMetadataId3FrameColorRole::V22 => color::id3_frame_v22(),
        TrackMetadataId3FrameColorRole::V23Only => color::id3_frame_v23_only(),
        TrackMetadataId3FrameColorRole::V24Only => color::id3_frame_v24_only(),
        TrackMetadataId3FrameColorRole::Unknown => color::id3_frame_unknown(),
    }
}

#[cfg(test)]
fn id3_frame_hint(field: &str) -> Option<&'static str> {
    match field {
        "Title" => Some("TIT2"),
        "Artist" => Some("TPE1"),
        "Album/Feed" => Some("TALB"),
        "Track #" => Some("TRCK"),
        "Publisher" => Some("TXXX:V4V_PUBLISHER"),
        "RSS feed guid" => Some("TXXX:MusicIndex Feed Guid"),
        "RSS track guid" => Some("TXXX:MusicIndex Track Guid"),
        "Nostr handle" | "RSS feed nostr handle" => Some("TXXX:RSS Nostr Handle"),
        "Label" => Some("TPUB"),
        "Website" => Some("WOAR"),
        "Tempo" => Some("TBPM"),
        "Release date" => Some("TDRC"),
        "Release year" => Some("TYER"),
        "Duration" => Some("TLEN"),
        "Artwork" => Some("APIC"),
        "Description" => Some("COMM:MusicIndex Description"),
        "Transcript" => Some("SYLT:MusicIndex Transcript"),
        "Transcript text" => Some("USLT:MusicIndex Transcript"),
        "Contributors" => Some("TXXX:MusicIndex Contributors"),
        "Composer" => Some("TCOM"),
        "Lyricist" => Some("TEXT"),
        "Lead performer" => Some("TPE1"),
        "Album artist" => Some("TPE2"),
        "Conductor" => Some("TPE3"),
        "Remixer" => Some("TPE4"),
        "Original artist" => Some("TOPE"),
        "Original lyricist" => Some("TOLY"),
        "Involved musicians" => Some("TMCL"),
        "Value Routes" => Some("TXXX:MusicIndex Value Routes"),
        "MusicBrainz recording" => Some("UFID:http://musicbrainz.org"),
        "MusicBrainz release" => Some("TXXX:MusicBrainz Album Id"),
        "MusicBrainz release group" => Some("TXXX:MusicBrainz Release Group Id"),
        "Release country" => Some("TXXX:MusicBrainz Album Release Country"),
        "Release status" => Some("TXXX:MusicBrainz Album Status"),
        "Barcode" => Some("TXXX:BARCODE"),
        "Release type" | "Release secondary types" => Some("TXXX:MusicBrainz Album Type"),
        "Media" => Some("TMED"),
        "Disc #" => Some("TPOS"),
        "Disc subtitle" => Some("TSST"),
        "Total tracks" => Some("TRCK"),
        "ISRC" => Some("TSRC"),
        _ => None,
    }
}

#[cfg(test)]
#[allow(dead_code)]
fn id3_frame_is_summarized(frame_id: &str) -> bool {
    matches!(frame_id, "TIT2" | "TPE1" | "TALB" | "TRCK")
}

#[cfg(test)]
#[allow(dead_code)]
fn format_track_slash_total(track: Option<&str>, total: Option<&str>) -> Option<String> {
    match (track, total) {
        (Some(t), Some(tot)) => Some(format!("{t}/{tot}")),
        (Some(t), None) => Some(t.to_string()),
        (None, Some(tot)) => Some(format!("/{tot}")),
        (None, None) => None,
    }
}

fn comparison_status_color(status: &ComparisonStatus, cx: &mut Context<SearchApp>) -> gpui::Rgba {
    TrackMetadataGridVm::comparison_role(status)
        .map(ProvenanceRole::from)
        .map_or_else(color::text_muted, |role| role.color(cx))
}

fn id3_cell_status_color(row: &AlignedCompareRow, cx: &mut Context<SearchApp>) -> gpui::Rgba {
    let fallback_color = || {
        if TrackMetadataGridVm::id3_status_uses_primary_fallback(
            row.id3_value.as_deref(),
            row.rss_value.as_deref(),
            row.musicbrainz_value.as_deref(),
        ) {
            color::text_primary()
        } else {
            color::text_muted()
        }
    };
    TrackMetadataGridVm::id3_status_role(
        row.id3_value.as_deref(),
        row.rss_value.as_deref(),
        row.musicbrainz_value.as_deref(),
        &row.id3_status,
    )
    .map(ProvenanceRole::from)
    .map_or_else(fallback_color, |role| role.color(cx))
}

pub(crate) fn render_track_list_rows(
    tracks: Vec<Track>,
    feed: Option<Feed>,
    feed_context: Option<FeedTrackListContext<'_>>,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> Vec<AnyElement> {
    let downloaded: Vec<bool> = {
        let db = app.conn.lock().ok();
        tracks
            .iter()
            .map(|track| {
                db.as_ref()
                    .and_then(|db| {
                        library_service::track_is_in_library_by_match(
                            db,
                            track
                                .feed_url
                                .as_deref()
                                .or_else(|| feed.as_ref().and_then(|f| f.feed_url.as_deref())),
                            track.track_guid.as_deref(),
                            track.enclosure_url.as_deref(),
                        )
                        .ok()
                    })
                    .unwrap_or(false)
            })
            .collect()
    };
    let (feed_guid, feed_url, playlists) = match feed_context {
        Some((g, url, pls)) => (Some(g.to_string()), url.map(str::to_string), pls.to_vec()),
        None => (None, None, Vec::new()),
    };

    tracks
        .into_iter()
        .zip(downloaded)
        .map(|(track, is_downloaded)| {
            let key = TrackRowActionVm::new(&track, is_downloaded, false).key();
            let is_in_flight = app.vm.is_track_operation_in_flight(&key);
            let thumb = app.thumbnail_for_url(track.image_url.as_deref(), cx);
            render_track_row(
                track,
                thumb,
                feed.clone(),
                is_downloaded,
                is_in_flight,
                feed_guid.as_deref(),
                feed_url.as_deref(),
                &playlists,
                cx,
            )
        })
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "stage 4 keeps the legacy Discover row wrapper stable while delegating"
)]
fn render_track_row(
    track: Track,
    thumbnail: Option<Arc<Image>>,
    feed: Option<Feed>,
    is_downloaded: bool,
    is_in_flight: bool,
    feed_guid: Option<&str>,
    feed_url: Option<&str>,
    playlists: &[db::Playlist],
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    track::render_track_row(
        track,
        thumbnail,
        feed,
        is_downloaded,
        is_in_flight,
        feed_guid,
        feed_url,
        playlists,
        track::TrackRowMode::Discover,
        cx,
    )
}

pub(crate) fn render_feed_header(
    frame: &InspectorFrame,
    title: &str,
    subtitle: Option<&str>,
) -> AnyElement {
    let display = SearchViewModel::feed_header_display(title, subtitle);
    DetailHeader::new(DetailHeaderDisplay {
        kind: EntityKind::Feed,
        title: display.title.into(),
        subtitle: display.subtitle.map(SharedString::from),
        data_rows: Vec::new(),
    })
    .image(frame.image.clone())
    .into_any_element()
}

pub(crate) fn render_play_icon_button_with_id(
    id: SharedString,
    display: TrackPlayAudioDisplay,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    render_play_icon_button_parts(
        id,
        display.button_label,
        display.url,
        display.tooltip,
        display.disabled,
        cx,
    )
}

fn render_play_icon_button(
    display: TrackPlayAudioDisplay,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let TrackPlayAudioDisplay {
        button_id,
        button_label,
        url,
        tooltip,
        disabled,
    } = display;
    render_play_icon_button_parts(
        SharedString::from(button_id),
        button_label,
        url,
        tooltip,
        disabled,
        cx,
    )
}

fn render_play_icon_button_parts(
    id: SharedString,
    button_label: &'static str,
    click_url: Option<String>,
    tooltip: String,
    disabled: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    // CONTROL-COMPAT(reason): native Button does not yet expose tooltip plus fixed square icon-button geometry.
    Button::new(id)
        .label(button_label)
        .scaled(Size::XSmall, cx)
        .compact()
        .ghost()
        .w(layout::ACTION_ICON_SIZE)
        .h(layout::ACTION_ICON_SIZE)
        .px(spacing::NONE)
        .py(spacing::NONE)
        .text_color(color::text_on_accent())
        .rounded(radius::SM)
        .border_1()
        .border_color(color::accent())
        .tooltip(tooltip)
        .disabled(disabled)
        .on_click(cx.listener(move |_this, _: &ClickEvent, _window, _cx| {
            if let Some(url) = &click_url {
                let _ = open::that(url);
            }
        }))
        .into_any_element()
}

pub(crate) fn render_track_download_button(
    track: Track,
    feed: Option<Feed>,
    is_downloaded: bool,
    is_in_flight: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let action_vm = TrackRowActionVm::new(&track, is_downloaded, is_in_flight);
    let display = action_vm.download_display();

    if action_vm.is_in_flight() {
        let tip = SharedString::from(display.busy_tooltip);
        return div()
            .id(SharedString::from(display.busy_indicator_id))
            .w(layout::ACTION_ICON_SIZE)
            .h(layout::ACTION_ICON_SIZE)
            .flex()
            .items_center()
            .justify_center()
            .rounded(radius::SM)
            .border_1()
            .border_color(color::accent())
            .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
            .child(
                Spinner::new()
                    .scaled(Size::XSmall, cx)
                    .color(color::accent().into()),
            )
            .into_any_element();
    }

    let action = action_vm.primary_action();
    let style = match action.tone {
        EntityActionTone::DestructiveQuiet => ControlStyle::DestructiveRowAction,
        _ => ControlStyle::RowAction,
    };
    let track_for_click = track.clone();
    let feed_for_click = feed.clone();

    UiButton::styled(SharedString::from(display.button_id), style)
        .label(action.label.clone())
        .disabled(!action.enabled)
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            if is_downloaded {
                this.remove_track_row(track_for_click.clone(), feed_for_click.clone(), cx);
            } else {
                this.download_track_row(track_for_click.clone(), feed_for_click.clone(), cx);
            }
        }))
        .into_any_element()
}

pub(crate) fn render_feed_list_section(
    feeds: Vec<Feed>,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let section_display = SearchViewModel::feed_list_section_display();
    let tiles: Vec<AnyElement> = feeds
        .into_iter()
        .map(|feed| {
            let RecentFeedTileDisplay {
                id,
                feed_list_tile_id,
                title,
                episode_note,
                image_url,
                ..
            } = RecentFeedTileVm::new(&feed).display();
            let click_guid = id;
            let click_title = title.clone();
            let thumb = app.thumbnail_for_url(image_url.as_deref(), cx);
            div()
                .id(SharedString::from(feed_list_tile_id))
                .w(layout::FEED_TILE_WIDTH)
                .flex()
                .flex_col()
                .gap(spacing::SM)
                .p(spacing::XS)
                .rounded(radius::MD)
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.push_inspector("feed".into(), click_guid.clone(), click_title.clone(), cx);
                }))
                .child(Thumbnail::new(EntityKind::Feed, ThumbnailSize::Lg).image(thumb.clone()))
                .child(
                    div().line_height(typography::LINE_COMPACT).child(
                        Label::new(title)
                            .size(FontSize::Caption)
                            .weight(FontWeight::MEDIUM)
                            .truncated(),
                    ),
                )
                .when_some(episode_note, |el, episode_note| {
                    el.child(
                        div()
                            .text_color(color::text_muted())
                            .text_size(typography::SIZE_MICRO)
                            .child(SharedString::from(episode_note)),
                    )
                })
                .into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(spacing::SM)
        .child(SectionHeader::new(section_display.heading))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(spacing::MD)
                .children(tiles),
        )
        .into_any_element()
}

fn render_track_header_subtitle(
    feed_link: Option<TrackFeedLinkDisplay>,
    audio_display: TrackPlayAudioDisplay,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::SM)
        .min_w_0()
        .when_some(feed_link, |el, link| {
            el.child(render_feed_link_value(link, cx))
        })
        .child(render_play_icon_button(audio_display, cx))
        .into_any_element()
}

fn render_feed_link_value(link: TrackFeedLinkDisplay, cx: &mut Context<SearchApp>) -> AnyElement {
    let TrackFeedLinkDisplay {
        element_id,
        guid,
        label,
        tooltip,
        ..
    } = link;
    let title = label;
    let click_title = title.clone();
    div()
        .id(SharedString::from(element_id))
        .cursor_pointer()
        .text_color(color::accent())
        .text_size(typography::SIZE_MICRO)
        .line_height(typography::LINE_DETAIL)
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.push_inspector("feed".into(), guid.clone(), click_title.clone(), cx);
        }))
        .child(SharedString::from(title))
        .into_any_element()
}

pub(crate) fn render_publisher_link_value(
    publisher_text: String,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let PublisherLinkDisplay {
        id,
        title,
        target,
        tooltip,
    } = PublisherLinkDisplay::new(publisher_text);
    let click_title = title.clone();
    div()
        .id(SharedString::from(id))
        .cursor_pointer()
        .text_color(color::accent())
        .text_size(typography::SIZE_MICRO)
        .line_height(typography::LINE_DETAIL)
        .truncate()
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.push_inspector("publisher".into(), target.clone(), click_title.clone(), cx);
        }))
        .child(SharedString::from(title))
        .into_any_element()
}

fn render_recent_feeds_tiles(app: &mut SearchApp, cx: &mut Context<SearchApp>) -> AnyElement {
    let snapshot = app.vm.recent_feeds_snapshot();
    let display = snapshot.display;
    let feeds = snapshot.feeds;
    let status = snapshot.status;
    let has_more = snapshot.has_more;
    let loading = snapshot.loading;
    let is_empty = snapshot.empty;

    let mut tiles: Vec<AnyElement> = Vec::with_capacity(feeds.len());
    for feed in feeds {
        let tile_vm = RecentFeedTileVm::new(&feed);
        let display = tile_vm.display();
        let target = display.open_target();
        if target.guid.trim().is_empty() {
            continue;
        }
        let thumbnail = app.thumbnail_for_url(display.image_url.as_deref(), cx);
        let tile = RecentFeedTile::new(display)
            .thumbnail(thumbnail)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_recent_feed(target.guid.clone(), target.title.clone(), cx);
            }))
            .into_any_element();
        tiles.push(tile);
    }

    div()
        .flex()
        .flex_col()
        .gap(spacing::MD)
        .child(
            div()
                .text_size(typography::SIZE_HEADLINE)
                .font_weight(FontWeight::SEMIBOLD)
                .child(display.heading),
        )
        .when(!status.is_empty(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(color::text_muted())
                    .child(SharedString::from(status)),
            )
        })
        .when(is_empty && !loading, |el| {
            el.child(
                div()
                    .text_center()
                    .p(spacing::XXL)
                    .text_color(color::text_muted())
                    .child(display.empty_label),
            )
        })
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(spacing::MD)
                .children(tiles),
        )
        .when(has_more && !loading, |el| {
            el.child(
                div().pt(spacing::SM).child(
                    UiButton::styled(display.load_more_button_id, ControlStyle::Ghost)
                        .label(display.load_more_label)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.load_recent_feeds(true, cx);
                        })),
                ),
            )
        })
        .into_any_element()
}

fn render_inspector_empty(display: InspectorChromeDisplay) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_color(color::text_muted())
        .gap(spacing::SM)
        .child(div().text_3xl().opacity(0.4).child(display.empty_icon))
        .child(display.empty_label)
        .into_any_element()
}

fn group_heading(label: &'static str) -> AnyElement {
    div()
        .text_size(typography::SIZE_MICRO)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color::text_muted())
        .mt(spacing::SM)
        .child(label)
        .into_any_element()
}

#[cfg(test)]
#[allow(dead_code)]
fn fmt_ms(ms: i64) -> String {
    crate::view_models::track::fmt_dur((ms / 1000).try_into().unwrap_or(i32::MAX))
}

#[cfg(test)]
#[allow(dead_code)]
fn join_values(values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.join(" · "))
    }
}

use crate::ui::layouts as layout;
use crate::ui::style::{color, radius, spacing, typography};

pub fn run_search_app() {
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        crate::ui::theme_bridge::install_theme(
            crate::theme_profile::ThemeProfile::Dark,
            crate::ui::tokens::ScaleFactor::Medium,
            cx,
        );
        let cfg_path = config::config_path().expect("config path");
        let cfg = config::load_config(&cfg_path).expect("load config");
        config::ensure_dirs(&cfg).expect("ensure dirs");
        let conn = db::open_db(&cfg).expect("open db");
        let conn = Arc::new(Mutex::new(conn));
        let musicindex_endpoint =
            config::load_musicindex_endpoint(&cfg_path).expect("load MusicIndex endpoint");

        crate::ui::theme_bridge::install_theme(cfg.theme_profile, cfg.ui_scale.into(), cx);

        let thumbnail_cache_dir = cfg_path
            .parent()
            .expect("config path has parent")
            .join("thumbnail-cache");
        let http = reqwest::blocking::Client::new();
        let image_cache = ImageCache::new(http, thumbnail_cache_dir);
        let application_services = Arc::new(
            ApplicationServices::local_with_service_adapters()
                .expect("application services are fully wired"),
        );

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(layout::WINDOW_WIDTH, layout::WINDOW_HEIGHT),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| {
                    SearchApp::new(
                        conn,
                        image_cache,
                        musicindex_endpoint,
                        application_services,
                        window,
                        cx,
                    )
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("failed to open window");
    });
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{Arc, Mutex};

    use super::{
        aligned_compare_rows, aligned_id3_frame_ids, artist_rows_from_result_rows,
        auto_populated_pending_id3_edits, expand_woar_metadata_rows, feed_rss_url,
        format_drag_value_for_id3v24, id3_frame_group_key, id3_frame_version,
        merge_track_play_fields, metadata_data_row, metadata_drag_value, metadata_field_group_key,
        musicbrainz_remainder_rows, pending_id3_conflict_descriptions, pending_id3_edits_for_apply,
        pending_id3_target_key, persist_musicindex_artist_facts, search_result_type_is_visible,
        should_show_inspector_back, track_metadata_rows, unused_id3v24_frames_for_group,
        AlignedCompareRow, Artist, EntityDetail, Feed, Id3FrameVersion, MetadataColumn,
        MetadataGridRow, PendingId3Edit, ResultRow, SearchBatch, SourceEnclosure, SourceEntityId,
        SourceEntityLink, TagCompareResult, Track, TrackContext, ID3V24_FRAME_GROUPS,
        ID3V24_FRAME_IDS,
    };
    use crate::audio_tags::{id3v24_edit_label_is_writable, Id3Field};
    use crate::db;
    use crate::metadata::{
        compare_id3_field_values, contributor_id3_rows, display_metadata_value,
        musicindex_contributors_id3_value,
    };
    use crate::musicbrainz::MusicBrainzCandidate;
    use crate::track_compare::{ComparisonRow, ComparisonStatus};
    use crate::view_models::track_metadata_grid::TrackMetadataGridVm;

    #[test]
    fn discover_back_button_is_visible_for_any_open_inspector() {
        assert!(
            !should_show_inspector_back(0),
            "empty inspector stack should not show Back"
        );
        assert!(
            should_show_inspector_back(1),
            "first opened feed, track, or publisher should show Back"
        );
        assert!(
            should_show_inspector_back(2),
            "nested inspector frames should keep showing Back"
        );
    }

    #[test]
    fn search_results_are_limited_to_artist_feed_and_track() {
        assert!(
            search_result_type_is_visible("artist"),
            "artist results should remain searchable"
        );
        assert!(
            search_result_type_is_visible("feed"),
            "feed results should remain searchable"
        );
        assert!(
            search_result_type_is_visible("track"),
            "track results should remain searchable"
        );
        assert!(
            !search_result_type_is_visible("publisher"),
            "publisher results should only be opened from feed or track links"
        );
    }

    #[test]
    fn search_batch_persists_explicit_musicindex_artist_facts() -> anyhow::Result<()> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        let conn = Arc::new(Mutex::new(conn));
        let batch = SearchBatch {
            rows: vec![
                ResultRow::new(
                    "artist",
                    "Alice",
                    Some(EntityDetail::Artist(Artist {
                        artist_id: Some("artist-123".into()),
                        name: Some("Alice".into()),
                        image_url: Some("https://example.test/alice.jpg".into()),
                        url: Some("https://example.test/alice".into()),
                        ..Artist::default()
                    })),
                ),
                ResultRow::new(
                    "artist",
                    "Name Only",
                    Some(EntityDetail::Artist(Artist {
                        name: Some("Name Only".into()),
                        ..Artist::default()
                    })),
                ),
            ],
            has_more: false,
            cursor: None,
        };

        persist_musicindex_artist_facts(&conn, &batch)?;

        let db = conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let row = db::artist_source_fact(&db, "musicindex", "artist-123")?
            .expect("explicit artist id should be persisted");
        assert_eq!(row.name.as_deref(), Some("Alice"));
        assert_eq!(
            row.image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(
            row.website_url.as_deref(),
            Some("https://example.test/alice")
        );
        assert_eq!(
            db::artist_source_fact(&db, "musicindex", "Name Only")?,
            None,
            "synthetic/name-only artist should not be persisted"
        );

        Ok(())
    }

    #[test]
    fn feed_and_track_action_urls_skip_empty_values() {
        let feed = Feed {
            feed_url: Some(" https://example.test/feed.xml ".into()),
            ..Feed::default()
        };
        assert_eq!(
            feed_rss_url(&feed).as_deref(),
            Some("https://example.test/feed.xml")
        );

        let direct_track = Track {
            enclosure_url: Some(" https://example.test/audio.mp3 ".into()),
            ..Track::default()
        };
        assert_eq!(
            crate::view_models::track::TrackVm::new(&direct_track)
                .play_url()
                .as_deref(),
            Some("https://example.test/audio.mp3")
        );

        let source_track = Track {
            enclosure_url: Some(" ".into()),
            source_enclosures: Some(vec![
                SourceEnclosure {
                    url: Some("https://example.test/alternate.mp3".into()),
                    ..SourceEnclosure::default()
                },
                SourceEnclosure {
                    url: Some("https://example.test/primary.mp3".into()),
                    is_primary: Some(true),
                    ..SourceEnclosure::default()
                },
            ]),
            ..Track::default()
        };
        assert_eq!(
            crate::view_models::track::TrackVm::new(&source_track)
                .play_url()
                .as_deref(),
            Some("https://example.test/primary.mp3")
        );
    }

    #[test]
    fn artist_rows_are_derived_from_feed_and_track_details() {
        let rows = vec![
            ResultRow {
                entity_type: "track".into(),
                entity_id: "track-1".into(),
                detail: Some(EntityDetail::Track(Track {
                    track_artist: Some("The Doerfels".into()),
                    release_artist: Some("The Doerfels".into()),
                    image_url: Some("https://example.test/track.png".into()),
                    ..Track::default()
                })),
            },
            ResultRow {
                entity_type: "feed".into(),
                entity_id: "feed-1".into(),
                detail: Some(EntityDetail::Feed(Feed {
                    release_artist: Some("The Doerfels".into()),
                    image_url: Some("https://example.test/feed.png".into()),
                    ..Feed::default()
                })),
            },
            ResultRow {
                entity_type: "artist".into(),
                entity_id: "other".into(),
                detail: Some(EntityDetail::Artist(Artist {
                    name: Some("Other Artist".into()),
                    ..Artist::default()
                })),
            },
        ];

        let artist_rows = artist_rows_from_result_rows(&rows, Some("doerfels"));

        assert_eq!(artist_rows.len(), 1);
        assert_eq!(artist_rows[0].entity_type, "artist");
        assert_eq!(artist_rows[0].entity_id, "The Doerfels");
        let Some(EntityDetail::Artist(artist)) = &artist_rows[0].detail else {
            panic!("expected artist detail");
        };
        assert_eq!(artist.track_count, Some(1));
        assert_eq!(artist.feed_count, Some(1));
        assert_eq!(
            artist.image_url.as_deref(),
            Some("https://example.test/track.png")
        );
    }

    #[test]
    fn feed_track_play_hydration_merges_only_missing_audio_fields() {
        let mut track = Track {
            enclosure_url: Some(" ".into()),
            title: Some("Local title".into()),
            ..Track::default()
        };
        let hydrated = Track {
            enclosure_url: Some("https://example.test/audio.mp3".into()),
            enclosure_type: Some("audio/mpeg".into()),
            enclosure_bytes: Some(123),
            source_enclosures: Some(vec![SourceEnclosure {
                url: Some("https://example.test/source.mp3".into()),
                is_primary: Some(true),
                ..SourceEnclosure::default()
            }]),
            title: Some("Hydrated title".into()),
            ..Track::default()
        };

        merge_track_play_fields(&mut track, hydrated);

        assert_eq!(
            track.enclosure_url.as_deref(),
            Some("https://example.test/audio.mp3")
        );
        assert_eq!(track.enclosure_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(track.enclosure_bytes, Some(123));
        assert_eq!(track.source_enclosures.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            track.title.as_deref(),
            Some("Local title"),
            "hydrating play fields should not replace displayed feed row metadata"
        );
    }

    #[test]
    fn id3v24_frame_registry_covers_83_grouped_frames() {
        assert_eq!(
            ID3V24_FRAME_IDS.len(),
            83,
            "expected the complete ID3v2.4 frame registry"
        );
        for frame_id in ID3V24_FRAME_IDS {
            let group = id3_frame_group_key(frame_id);
            assert!(
                ID3V24_FRAME_GROUPS.iter().any(|(key, _)| *key == group),
                "frame {frame_id} should have a visible group"
            );
        }

        let grouped_count: usize = ID3V24_FRAME_GROUPS
            .iter()
            .map(|(group_key, _)| {
                ID3V24_FRAME_IDS
                    .iter()
                    .filter(|frame_id| id3_frame_group_key(frame_id) == *group_key)
                    .count()
            })
            .sum();
        assert_eq!(
            grouped_count, 83,
            "every ID3v2.4 frame should map into the proposed HTML table groups"
        );

        let expected_counts = [
            ("identification-release-structure", 9),
            ("people-credits", 11),
            ("descriptive-technical-rights-text", 26),
            ("url-link-frames", 9),
            ("lyrics-comments-artwork-user-facing-content", 8),
            ("identity-linking-private-registration", 7),
            ("timing-seeking-audio-analysis-playback-control", 10),
            ("music-disc-acquisition-commerce", 3),
        ];
        for (group_key, expected_count) in expected_counts {
            let actual_count = ID3V24_FRAME_IDS
                .iter()
                .filter(|frame_id| id3_frame_group_key(frame_id) == group_key)
                .count();
            assert_eq!(
                actual_count, expected_count,
                "group {group_key} should match the proposed HTML table count"
            );
        }
    }

    #[test]
    fn unused_id3v24_frames_are_grouped_and_exclude_present_frames() {
        let result = TagCompareResult {
            path: String::new(),
            rows: Vec::new(),
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![
                Id3Field {
                    frame_id: "TIT2".into(),
                    value: "Title".into(),
                },
                Id3Field {
                    frame_id: "APIC".into(),
                    value: "cover".into(),
                },
            ],
        };

        let identification_unused =
            unused_id3v24_frames_for_group(&result, "identification-release-structure");
        let content_unused =
            unused_id3v24_frames_for_group(&result, "lyrics-comments-artwork-user-facing-content");

        assert!(
            !identification_unused.contains(&"TIT2"),
            "present title frame should not be listed"
        );
        assert!(
            !content_unused.contains(&"APIC"),
            "present artwork frame should not be listed"
        );
        assert!(
            identification_unused.contains(&"TALB"),
            "absent album frame should remain available"
        );
    }

    #[test]
    fn descriptor_id3_rows_are_suppressed_when_semantic_row_exists() {
        let result = TagCompareResult {
            path: String::new(),
            rows: Vec::new(),
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![Id3Field {
                frame_id: "TXXX:MusicIndex Value Routes".into(),
                value: r#"[{"recipient_name":"Alice","split":1.0}]"#.into(),
            }],
        };
        let mut grouped = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
        grouped.insert(
            "music-disc-acquisition-commerce",
            vec![metadata_data_row(test_compare_row(
                "Value Routes",
                Some(r#"[{"recipient_name":"Alice","split":1.0}]"#),
                Some(r#"[{"recipient_name":"Alice","split":1.0}]"#),
                Some("TXXX:MusicIndex Value Routes"),
                None,
            ))],
        );

        let aligned = aligned_id3_frame_ids(&result, &grouped);
        let used = super::used_id3_fields_for_group(
            &result,
            "descriptive-technical-rights-text",
            &aligned,
        );

        assert!(
            used.is_empty(),
            "descriptor-specific TXXX rows should not also appear as raw ID3 rows"
        );
    }

    #[test]
    fn tempo_aliases_are_displayed_as_one_metadata_row() {
        let result = TagCompareResult {
            path: String::new(),
            rows: Vec::new(),
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![
                Id3Field {
                    frame_id: "TBPM".into(),
                    value: "100.0".into(),
                },
                Id3Field {
                    frame_id: "TXXX:IBPM".into(),
                    value: "100.0".into(),
                },
                Id3Field {
                    frame_id: "TXXX:tempo".into(),
                    value: "100.0".into(),
                },
                Id3Field {
                    frame_id: "TXXX:bpm".into(),
                    value: "100.0".into(),
                },
            ],
        };
        let track_context = TrackContext {
            track: Track::default(),
            feed: None,
        };

        let rows = aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());
        let tempo = rows
            .iter()
            .find_map(|row| match row {
                MetadataGridRow::Data(row) if row.field == "Tempo" => Some(row),
                MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
            })
            .expect("tempo row");

        assert_eq!(
            tempo.id3_value.as_deref(),
            Some("TBPM: 100.0\nTXXX:IBPM: 100.0\nTXXX:tempo: 100.0\nTXXX:bpm: 100.0")
        );

        let mut grouped = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
        grouped.insert("timing-seeking-audio-analysis-playback-control", rows);
        let aligned = aligned_id3_frame_ids(&result, &grouped);
        let used = super::used_id3_fields_for_group(
            &result,
            "descriptive-technical-rights-text",
            &aligned,
        );
        assert!(
            used.is_empty(),
            "tempo aliases should not be repeated as separate raw ID3 rows"
        );
    }

    #[test]
    fn sort_order_aliases_are_grouped_with_primary_rows() {
        let result = TagCompareResult {
            path: String::new(),
            rows: vec![
                ComparisonRow {
                    field: "Title",
                    source_value: Some("The Platform".into()),
                    tag_value: Some("The Platform".into()),
                    status: ComparisonStatus::Match,
                },
                ComparisonRow {
                    field: "Artist",
                    source_value: Some("HeyCitizen".into()),
                    tag_value: Some("HeyCitizen".into()),
                    status: ComparisonStatus::Match,
                },
                ComparisonRow {
                    field: "Album/Feed",
                    source_value: Some("Lofi Experience".into()),
                    tag_value: Some("Lofi Experience".into()),
                    status: ComparisonStatus::Match,
                },
            ],
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![
                Id3Field {
                    frame_id: "TIT2".into(),
                    value: "The Platform".into(),
                },
                Id3Field {
                    frame_id: "TSOT".into(),
                    value: "Platform, The".into(),
                },
                Id3Field {
                    frame_id: "TPE1".into(),
                    value: "HeyCitizen".into(),
                },
                Id3Field {
                    frame_id: "TSOP".into(),
                    value: "Citizen, Hey".into(),
                },
                Id3Field {
                    frame_id: "TALB".into(),
                    value: "Lofi Experience".into(),
                },
                Id3Field {
                    frame_id: "TSOA".into(),
                    value: "Experience, Lofi".into(),
                },
            ],
        };
        let track_context = TrackContext {
            track: Track::default(),
            feed: None,
        };

        let rows = aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());
        let title = rows
            .iter()
            .find_map(|row| match row {
                MetadataGridRow::Data(row) if row.field == "Title" => Some(row),
                MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
            })
            .expect("title row");
        let artist = rows
            .iter()
            .find_map(|row| match row {
                MetadataGridRow::Data(row) if row.field == "Artist" => Some(row),
                MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
            })
            .expect("artist row");
        let album = rows
            .iter()
            .find_map(|row| match row {
                MetadataGridRow::Data(row) if row.field == "Album/Feed" => Some(row),
                MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
            })
            .expect("album row");

        assert_eq!(
            title.id3_value.as_deref(),
            Some("TIT2: The Platform\nTSOT: Platform, The")
        );
        assert_eq!(
            artist.id3_value.as_deref(),
            Some("TPE1: HeyCitizen\nTSOP: Citizen, Hey")
        );
        assert_eq!(
            album.id3_value.as_deref(),
            Some("TALB: Lofi Experience\nTSOA: Experience, Lofi")
        );
        assert_eq!(title.id3_status, ComparisonStatus::Match);

        let mut grouped = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
        grouped.insert("identification-release-structure", rows);
        let aligned = aligned_id3_frame_ids(&result, &grouped);
        let used = super::used_id3_fields_for_group(
            &result,
            "descriptive-technical-rights-text",
            &aligned,
        );
        assert!(
            used.is_empty(),
            "sort-order aliases should not also appear as separate raw ID3 rows"
        );
    }

    #[test]
    fn contributor_related_id3_frames_roll_up_into_contributors_row() {
        let result = TagCompareResult {
            path: String::new(),
            rows: Vec::new(),
            file_image: None,
            contributors: vec![
                crate::api::Contributor {
                    name: Some("Alice".into()),
                    role: Some("guitarist".into()),
                    ..Default::default()
                },
                crate::api::Contributor {
                    name: Some("Bob".into()),
                    role: Some("audio engineer".into()),
                    ..Default::default()
                },
                crate::api::Contributor {
                    name: Some("Cara".into()),
                    role: Some("composer".into()),
                    ..Default::default()
                },
                crate::api::Contributor {
                    name: Some("Dana".into()),
                    role: Some("musician".into()),
                    ..Default::default()
                },
            ],
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![
                Id3Field {
                    frame_id: "TXXX:MUSICIANCREDITS".into(),
                    value: "guitar: Alice / musician: Dana".into(),
                },
                Id3Field {
                    frame_id: "TCOM".into(),
                    value: "Cara".into(),
                },
                Id3Field {
                    frame_id: "TIPL".into(),
                    value: "engineer: Bob".into(),
                },
                Id3Field {
                    frame_id: "TMCL".into(),
                    value: "guitar: Alice".into(),
                },
                Id3Field {
                    frame_id: "TPE1".into(),
                    value: "Dana".into(),
                },
            ],
        };
        let track_context = TrackContext {
            track: Track::default(),
            feed: None,
        };

        let rows = aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());
        let contributors = rows
            .iter()
            .find_map(|row| match row {
                MetadataGridRow::Data(row) if row.field == "Contributors" => Some(row),
                MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
            })
            .expect("contributors row");

        assert_eq!(
            display_metadata_value(
                "Contributors",
                contributors
                    .id3_value
                    .as_deref()
                    .expect("contributors value")
            ),
            "Alice: guitar\nBob: engineer\nCara: composer\nDana: musician"
        );
        assert_eq!(contributors.id3_status, ComparisonStatus::Match);

        let mut grouped = BTreeMap::<&'static str, Vec<MetadataGridRow>>::new();
        grouped.insert("people-credits", rows);
        let aligned = aligned_id3_frame_ids(&result, &grouped);
        let used = super::used_id3_fields_for_group(&result, "people-credits", &aligned);
        assert!(
            used.is_empty(),
            "contributor-related ID3 frames should stay grouped under Contributors"
        );
    }

    #[test]
    fn rss_and_musicbrainz_rows_use_semantic_groups() {
        assert_eq!(metadata_field_group_key("Artist"), "people-credits");
        assert_eq!(metadata_field_group_key("Website"), "url-link-frames");
        assert_eq!(
            metadata_field_group_key("ISRC"),
            "identification-release-structure"
        );
        assert_eq!(
            metadata_field_group_key("Value Routes"),
            "music-disc-acquisition-commerce"
        );
    }

    #[test]
    fn release_date_prefers_item_then_feed_then_oldest_item_pubdate() {
        let track_context = TrackContext {
            track: Track {
                pub_date: Some(1_704_067_200),
                ..Default::default()
            },
            feed: Some(Feed {
                release_date: Some(1_672_531_200),
                oldest_item_at: Some(1_640_995_200),
                ..Default::default()
            }),
        };
        assert_eq!(
            super::musicindex_release_date(&track_context).as_deref(),
            Some("Jan 1, 2024")
        );

        let track_context = TrackContext {
            track: Track {
                pub_date: Some(1_704_067_200),
                ..Default::default()
            },
            feed: Some(Feed {
                oldest_item_at: Some(1_640_995_200),
                ..Default::default()
            }),
        };
        assert_eq!(
            super::musicindex_release_date(&track_context).as_deref(),
            Some("Jan 1, 2024")
        );

        let track_context = TrackContext {
            track: Track::default(),
            feed: Some(Feed {
                release_date: Some(1_672_531_200),
                oldest_item_at: Some(1_640_995_200),
                ..Default::default()
            }),
        };
        assert_eq!(
            super::musicindex_release_date(&track_context).as_deref(),
            Some("Jan 1, 2023")
        );
    }

    #[test]
    fn musicbrainz_rows_align_with_id3_and_rss_equivalents() {
        let track_context = TrackContext {
            track: Track {
                title: Some("Song".into()),
                track_artist: Some("Artist".into()),
                track_number: Some(4),
                duration_secs: Some(199),
                source_ids: Some(vec![SourceEntityId {
                    scheme: Some("isrc".into()),
                    value: Some("USRC17607839".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            feed: None,
        };
        let result = TagCompareResult {
            path: String::new(),
            rows: vec![
                ComparisonRow {
                    field: "Title",
                    source_value: Some("Song".into()),
                    tag_value: Some("Song".into()),
                    status: ComparisonStatus::Match,
                },
                ComparisonRow {
                    field: "Artist",
                    source_value: Some("Artist".into()),
                    tag_value: Some("Artist".into()),
                    status: ComparisonStatus::Match,
                },
                ComparisonRow {
                    field: "Track #",
                    source_value: Some("4".into()),
                    tag_value: Some("4".into()),
                    status: ComparisonStatus::Match,
                },
            ],
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![Id3Field {
                frame_id: "TSRC".into(),
                value: "USRC17607839".into(),
            }],
        };
        let candidate = MusicBrainzCandidate {
            recording_id: "recording-id".into(),
            track_length_ms: Some(199_000),
            isrcs: vec!["USRC17607839".into()],
            ..Default::default()
        };

        let rows = musicbrainz_remainder_rows(&candidate, &track_context, Some(&result));
        let isrc_row = rows
            .iter()
            .find(|row| row.field == "ISRC")
            .expect("ISRC row should be present");
        assert_eq!(isrc_row.rss_value.as_deref(), Some("USRC17607839"));
        assert_eq!(isrc_row.id3_frame.as_deref(), Some("TSRC"));
        assert_eq!(isrc_row.id3_value.as_deref(), Some("USRC17607839"));
    }

    #[test]
    fn id3_frame_version_classifies_frame_generations() {
        assert_eq!(id3_frame_version("TT2"), Id3FrameVersion::V22);
        assert_eq!(id3_frame_version("TYER"), Id3FrameVersion::V23Only);
        assert_eq!(id3_frame_version("TDRC"), Id3FrameVersion::V24Only);
        assert_eq!(id3_frame_version("TIT2"), Id3FrameVersion::V23V24);
        assert_eq!(id3_frame_version("ZZZZ"), Id3FrameVersion::Unknown);
        assert_eq!(
            id3_frame_group_key("TYER"),
            "descriptive-technical-rights-text"
        );
    }

    #[test]
    fn drag_value_does_not_require_source_frame_hint() {
        let row = AlignedCompareRow {
            row_id: TrackMetadataGridVm::compare_row_id("RSS feed guid"),
            field: "RSS feed guid".into(),
            rss_value: Some("feed-guid".into()),
            id3_value: None,
            id3_frame: None,
            musicbrainz_value: None,
            musicbrainz_key: None,
            id3_status: ComparisonStatus::MissingTag,
            musicbrainz_status: ComparisonStatus::MissingTag,
        };

        let drag = metadata_drag_value(&row, MetadataColumn::Rss)
            .expect("RSS values without source ID3 hints should still be draggable");
        assert_eq!(drag.value, "feed-guid");
        assert_eq!(
            drag.frame, "",
            "the ID3 target cell supplies the destination frame on drop"
        );
    }

    #[test]
    fn drag_copy_formats_values_for_id3v24_target_frames() {
        assert_eq!(
            format_drag_value_for_id3v24("TRCK", "Track #", None, "3 / 12").as_deref(),
            Some("3/12")
        );
        assert_eq!(
            format_drag_value_for_id3v24(
                "TXXX:MusicIndex Contributors",
                "Contributors",
                None,
                " Alice \0 Bob ",
            )
            .as_deref(),
            Some("Alice   Bob")
        );
        assert_eq!(
            format_drag_value_for_id3v24("TIT2", "Title", None, " \0 "),
            None
        );
        assert_eq!(
            format_drag_value_for_id3v24("TIT2", "Title", None, " - Song").as_deref(),
            Some("Song")
        );
        assert_eq!(
            format_drag_value_for_id3v24("TRCK", "Track #", Some("3/12"), "4").as_deref(),
            Some("4/12")
        );
        assert_eq!(
            format_drag_value_for_id3v24("TRCK", "Total tracks", Some("4"), "12").as_deref(),
            Some("4/12")
        );
        assert_eq!(
            format_drag_value_for_id3v24("TRCK", "Total tracks", None, "12"),
            None
        );
        assert_eq!(
            format_drag_value_for_id3v24("TDRC", "Release date", None, "Dec 8, 2025").as_deref(),
            Some("2025-12-08")
        );
        assert_eq!(
            format_drag_value_for_id3v24("TYER", "Release year", None, "Dec 8, 2025").as_deref(),
            Some("2025")
        );
        assert_eq!(
            format_drag_value_for_id3v24(
                "WOAR",
                "Website",
                None,
                "https://a.test · https://b.test"
            )
            .as_deref(),
            Some("https://a.test")
        );
        assert_eq!(
            format_drag_value_for_id3v24(
                "WXXX:Official audio",
                "Website",
                None,
                "https://a.test · https://b.test",
            )
            .as_deref(),
            Some("https://a.test")
        );
    }

    #[test]
    fn all_compare_id3_hints_are_writable_id3v24_targets() {
        let fields = [
            "Title",
            "Artist",
            "Album/Feed",
            "Track #",
            "Publisher",
            "Nostr handle",
            "RSS feed nostr handle",
            "Label",
            "Website",
            "Tempo",
            "Release date",
            "Release year",
            "Duration",
            "Artwork",
            "Description",
            "Transcript",
            "Transcript text",
            "Contributors",
            "Composer",
            "Lyricist",
            "Lead performer",
            "Album artist",
            "Conductor",
            "Remixer",
            "Original artist",
            "Original lyricist",
            "Involved musicians",
            "Value Routes",
            "MusicBrainz recording",
            "MusicBrainz release",
            "MusicBrainz release group",
            "Release country",
            "Release status",
            "Barcode",
            "Release type",
            "Release secondary types",
            "Media",
            "Disc #",
            "Disc subtitle",
            "Total tracks",
            "ISRC",
        ];

        for field in fields {
            let hint = super::id3_frame_hint(field).expect("field should have an ID3 hint");
            assert!(
                id3v24_edit_label_is_writable(hint),
                "{field} should map to a writable ID3v2.4 target, got {hint}"
            );
        }
    }

    #[test]
    fn auto_populates_missing_id3_from_rss_then_musicbrainz_without_source_conflicts() {
        let rows = vec![
            metadata_data_row(test_compare_row(
                "Title",
                Some("RSS Song"),
                None,
                Some("TIT2"),
                None,
            )),
            metadata_data_row(test_compare_row(
                "Artist",
                Some("RSS Artist"),
                None,
                Some("TPE1"),
                Some("MusicBrainz Artist"),
            )),
            metadata_data_row(test_compare_row(
                "Label",
                None,
                None,
                Some("TPUB"),
                Some("MusicBrainz Label"),
            )),
            metadata_data_row(test_compare_row(
                "Album/Feed",
                Some("Existing Album"),
                Some("Existing Album"),
                Some("TALB"),
                None,
            )),
            metadata_data_row(test_compare_row(
                "Release date",
                Some("2025"),
                None,
                Some("TDRC"),
                Some("2025"),
            )),
        ];

        let pending =
            auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new(), None);
        assert_eq!(pending.len(), 3);
        assert_eq!(pending["title"].value, "RSS Song");
        assert_eq!(pending["title"].source, MetadataColumn::Rss);
        assert_eq!(pending["label"].value, "MusicBrainz Label");
        assert_eq!(pending["label"].source, MetadataColumn::MusicBrainz);
        assert_eq!(pending["release-date"].value, "2025");
        assert_eq!(pending["release-date"].source, MetadataColumn::Rss);
        assert!(
            !pending.contains_key("artist"),
            "conflicting RSS and MusicBrainz values should remain manual"
        );
        assert!(
            !pending.contains_key("album-feed"),
            "existing ID3 values should not be auto-staged"
        );
    }

    #[test]
    fn auto_populates_composite_track_number_targets_once_for_apply() {
        let rows = vec![
            metadata_data_row(test_compare_row(
                "Track #",
                Some("4"),
                None,
                Some("TRCK"),
                None,
            )),
            metadata_data_row(test_compare_row(
                "Total tracks",
                None,
                None,
                Some("TRCK"),
                Some("10"),
            )),
        ];

        let pending =
            auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new(), None);
        assert_eq!(pending["track"].value, "4/10");
        assert_eq!(pending["total-tracks"].value, "4/10");
        assert!(
            pending_id3_conflict_descriptions(&pending).is_empty(),
            "same target and same staged value should not be a conflict"
        );

        let edits = pending_id3_edits_for_apply(&pending);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].frame_label, "TRCK");
        assert_eq!(edits[0].value, "4/10");
    }

    #[test]
    fn auto_populates_multiple_woar_rows_for_distinct_outer_urls() {
        let rows = vec![metadata_data_row(test_compare_row(
            "Website",
            Some("https://rss.example/artist"),
            None,
            Some("WOAR"),
            Some("https://mb.example/artist"),
        ))];

        let expanded = expand_woar_metadata_rows(rows);
        let pending =
            auto_populated_pending_id3_edits(&expanded, &BTreeMap::new(), &BTreeSet::new(), None);
        assert_eq!(pending.len(), 2);
        assert_eq!(
            pending["compare:website"].value,
            "download for free (url, forward): https://rss.example/artist"
        );
        assert_eq!(pending["compare:website"].source, MetadataColumn::Rss);
        assert_eq!(
            pending["compare:website-2"].value,
            "https://mb.example/artist"
        );
        assert_eq!(
            pending["compare:website-2"].source,
            MetadataColumn::MusicBrainz
        );
        assert!(
            pending_id3_conflict_descriptions(&pending).is_empty(),
            "distinct WOAR URLs should be staged as repeatable URL frames"
        );

        let edits = pending_id3_edits_for_apply(&pending);
        assert_eq!(edits.len(), 2);
        assert!(edits.iter().all(|edit| edit.frame_label == "WOAR"));
    }

    #[test]
    fn wrapped_woar_url_counts_as_existing_website() {
        let url = "https://lnbeats.com/album/a2d2e313-9cbd-5169-b89c-ab7b33ecc33";
        let rows = vec![metadata_data_row(test_compare_row(
            "Website",
            Some(url),
            Some(&format!("download for free (url, forward): {url}")),
            Some("WOAR"),
            None,
        ))];

        let expanded = expand_woar_metadata_rows(rows);
        let row = expanded
            .iter()
            .find_map(|row| match row {
                MetadataGridRow::Data(row) => Some(row),
                MetadataGridRow::Group(_) => None,
            })
            .expect("website row");
        assert_eq!(row.id3_status, ComparisonStatus::Match);

        let pending =
            auto_populated_pending_id3_edits(&expanded, &BTreeMap::new(), &BTreeSet::new(), None);
        assert!(
            pending.is_empty(),
            "matching wrapped WOAR should not stage a duplicate website"
        );
    }

    #[test]
    fn id3_compare_normalizes_dates_people_and_wrapped_urls() {
        assert_eq!(
            compare_id3_field_values("Release date", Some("Nov 7, 2023"), Some("2023-11-07")),
            ComparisonStatus::Match
        );
        assert_eq!(
            compare_id3_field_values(
                "Website",
                Some("https://example.test/album"),
                Some("download for free (url, forward): https://example.test/album")
            ),
            ComparisonStatus::Match
        );
        assert_eq!(
            compare_id3_field_values("Album artist", Some("HeyCitizen"), Some("Hey Citizen")),
            ComparisonStatus::Match
        );
        assert_eq!(
            compare_id3_field_values(
                "Performer [vocals]",
                Some("HeyCitizen / DuhLaurien / MaryKateUltra"),
                Some("Hey Citizen / DuhLaurien / Mary KateUltra")
            ),
            ComparisonStatus::Match
        );
    }

    #[test]
    fn tagger_stages_transcript_url_as_sylt_and_uslt() {
        let context = TrackContext {
            track: Track {
                title: Some("Song".into()),
                source_links: Some(vec![SourceEntityLink {
                    link_type: Some("transcript".into()),
                    url: Some("https://example.com/song.srt".into()),
                    extraction_path: Some("podcast:transcript@url".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            feed: None,
        };

        let edits = crate::metadata_service::id3_edits_for_track_context(&context);
        assert!(edits.iter().any(|edit| {
            edit.frame_label == "SYLT:MusicIndex Transcript"
                && edit.value == "https://example.com/song.srt"
        }));
        assert!(edits.iter().any(|edit| {
            edit.frame_label == "USLT:MusicIndex Transcript"
                && edit.value == "https://example.com/song.srt"
        }));
    }

    #[test]
    fn tagger_stages_nostr_handles_as_txxx() {
        let context = TrackContext {
            track: Track {
                title: Some("Song".into()),
                source_ids: Some(vec![SourceEntityId {
                    scheme: Some("nostr_npub".into()),
                    value: Some("npub1track".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            feed: None,
        };

        let edits = crate::metadata_service::id3_edits_for_track_context(&context);
        assert!(edits.iter().any(|edit| {
            edit.frame_label == "TXXX:RSS Nostr Handle" && edit.value == "npub1track"
        }));
    }

    #[test]
    fn tagger_stages_musicindex_guids_as_txxx() {
        let context = TrackContext {
            track: Track {
                title: Some("Song".into()),
                track_guid: Some("track-guid".into()),
                feed_guid: Some("feed-guid".into()),
                ..Default::default()
            },
            feed: None,
        };

        let edits = crate::metadata_service::id3_edits_for_track_context(&context);
        assert!(edits.iter().any(|edit| {
            edit.frame_label == "TXXX:MusicIndex Track Guid" && edit.value == "track-guid"
        }));
        assert!(edits.iter().any(|edit| {
            edit.frame_label == "TXXX:MusicIndex Feed Guid" && edit.value == "feed-guid"
        }));
    }

    #[test]
    fn contributors_map_to_picard_like_people_frames() {
        let contributors = vec![
            crate::api::Contributor {
                name: Some("Alice".into()),
                role: Some("guitarist".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("Bob".into()),
                role: Some("audio engineer".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("Cara".into()),
                role: Some("composer".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("Dana".into()),
                role: Some("musician".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("Band".into()),
                role: Some("album artist".into()),
                ..Default::default()
            },
            crate::api::Contributor {
                name: Some("Eli".into()),
                role: Some("Performer [keyboards]".into()),
                ..Default::default()
            },
        ];

        let rows = contributor_id3_rows(&contributors);
        assert!(rows.iter().any(|(field, frame, value)| {
            field == "Performer [guitar]" && *frame == "TMCL" && value == "Alice"
        }));
        assert!(rows
            .iter()
            .any(|(_, frame, value)| { *frame == "TIPL" && value == "engineer: Bob" }));
        assert!(rows
            .iter()
            .any(|(_, frame, value)| { *frame == "TCOM" && value == "Cara" }));
        assert!(rows
            .iter()
            .any(|(_, frame, value)| { *frame == "TPE1" && value == "Dana" }));
        assert!(rows
            .iter()
            .any(|(_, frame, value)| { *frame == "TPE2" && value == "Band" }));
        assert!(rows.iter().any(|(field, frame, value)| {
            field == "Performer [keyboards]" && *frame == "TMCL" && value == "Eli"
        }));

        let musicindex = musicindex_contributors_id3_value(&contributors)
            .expect("contributors should have a MusicIndex ID3 payload");
        assert_eq!(
            musicindex,
            "guitarist: Alice / audio engineer: Bob / composer: Cara / musician: Dana / album artist: Band / Performer [keyboards]: Eli"
        );
        assert_eq!(
            display_metadata_value("Contributors", &musicindex),
            "Alice: guitarist\nBand: album artist\nBob: audio engineer\nCara: composer\nDana: musician\nEli: Performer [keyboards]"
        );
    }

    #[test]
    fn value_routes_keep_json_payload_but_display_pretty() {
        let value = r#"[{"recipient_name":"Alice","route_type":"node","split":75.0,"fee":false,"address":"abc","custom_key":null,"custom_value":null},{"recipient_name":"Hosting","route_type":"node","split":25.0,"fee":true,"address":"def","custom_key":null,"custom_value":null}]"#;
        assert_eq!(
            display_metadata_value("Value Routes", value),
            "[\n  {\n    \"recipient_name\": \"Alice\",\n    \"route_type\": \"node\",\n    \"split\": 75.0,\n    \"fee\": false,\n    \"address\": \"abc\",\n    \"custom_key\": null,\n    \"custom_value\": null\n  },\n  {\n    \"recipient_name\": \"Hosting\",\n    \"route_type\": \"node\",\n    \"split\": 25.0,\n    \"fee\": true,\n    \"address\": \"def\",\n    \"custom_key\": null,\n    \"custom_value\": null\n  }\n]"
        );
    }

    #[test]
    fn tmcl_rows_match_picard_like_performer_fields() {
        let result = TagCompareResult {
            path: String::new(),
            rows: Vec::new(),
            file_image: None,
            contributors: vec![
                crate::api::Contributor {
                    name: Some("HeyCitizen".into()),
                    role: Some("vocal".into()),
                    ..Default::default()
                },
                crate::api::Contributor {
                    name: Some("DuhLaurien".into()),
                    role: Some("vocals".into()),
                    ..Default::default()
                },
                crate::api::Contributor {
                    name: Some("MaryKateUltra".into()),
                    role: Some("vocalist".into()),
                    ..Default::default()
                },
            ],
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![Id3Field {
                frame_id: "TMCL".into(),
                value: "Hey Citizen - vocals / vocals:DuhLaurien / vocals: Mary KateUltra".into(),
            }],
        };
        let rows = aligned_compare_rows(
            &result,
            &TrackContext {
                track: Track::default(),
                feed: None,
            },
            None,
            false,
            &BTreeSet::new(),
        );
        let performer = rows
            .iter()
            .find_map(|row| match row {
                MetadataGridRow::Data(row) if row.field == "Performer [vocals]" => Some(row),
                MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
            })
            .expect("performer row");

        assert_eq!(performer.id3_status, ComparisonStatus::Match);
        assert_eq!(
            performer.id3_value.as_deref(),
            Some("Hey Citizen · DuhLaurien · Mary KateUltra")
        );
    }

    #[test]
    fn transcript_rows_visible_even_when_content_group_collapsed() {
        let result = TagCompareResult {
            path: String::new(),
            rows: Vec::new(),
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![Id3Field {
                frame_id: "USLT:MusicIndex Transcript".into(),
                value: "line one\nline two".into(),
            }],
        };
        let track_context = TrackContext {
            track: Track {
                source_links: Some(vec![SourceEntityLink {
                    link_type: Some("transcript".into()),
                    url: Some("https://example.test/transcript.srt".into()),
                    extraction_path: Some("podcast:transcript@url".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            feed: None,
        };

        // Transcript rows should be visible even when the content group is collapsed
        let collapsed =
            aligned_compare_rows(&result, &track_context, None, false, &BTreeSet::new());
        let collapsed_fields = collapsed
            .iter()
            .filter_map(|row| match row {
                MetadataGridRow::Data(row) => Some(row.field.as_str()),
                MetadataGridRow::Group(_) => None,
            })
            .collect::<Vec<_>>();
        assert!(collapsed_fields.contains(&"Transcript"));
        assert!(collapsed_fields.contains(&"Transcript text"));

        let transcript_text = collapsed
            .iter()
            .find_map(|row| match row {
                MetadataGridRow::Data(row) if row.field == "Transcript text" => Some(row),
                MetadataGridRow::Data(_) | MetadataGridRow::Group(_) => None,
            })
            .expect("transcript text row");
        assert_eq!(transcript_text.id3_status, ComparisonStatus::Match);
    }

    #[test]
    fn suppressed_auto_id3_rows_are_not_reselected() {
        let rows = vec![metadata_data_row(test_compare_row(
            "Title",
            Some("RSS Song"),
            None,
            Some("TIT2"),
            None,
        ))];
        let suppressed = BTreeSet::from(["title".to_string()]);

        let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &suppressed, None);
        assert!(pending.is_empty());
    }

    #[test]
    fn track_rows_show_parent_feed_total_tracks_after_track_number() {
        let track_context = TrackContext {
            track: Track {
                track_number: Some(4),
                ..Default::default()
            },
            feed: Some(Feed {
                episode_count: Some(10),
                ..Default::default()
            }),
        };

        let rows = track_metadata_rows(&track_context, None, false);
        let fields = rows
            .iter()
            .filter_map(|row| match row {
                MetadataGridRow::Data(row) => Some(row.field.as_str()),
                MetadataGridRow::Group(_) => None,
            })
            .collect::<Vec<_>>();
        assert!(fields.contains(&"Track #"), "track row should exist");
        assert!(
            !fields.contains(&"Total tracks"),
            "total tracks should be merged into Track # row"
        );
    }

    #[test]
    fn guid_rows_only_read_matching_txxx_frames() {
        let result = TagCompareResult {
            path: String::new(),
            rows: Vec::new(),
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            total_tracks: None,
            format: None,
            id3_fields: vec![
                Id3Field {
                    frame_id: "TXXX:MusicIndex Track Guid".into(),
                    value: "track-guid".into(),
                },
                Id3Field {
                    frame_id: "TXXX:MusicIndex Feed Guid".into(),
                    value: "feed-guid".into(),
                },
                Id3Field {
                    frame_id: "TXXX:MusicIndex Value Routes".into(),
                    value: "[4 items]".into(),
                },
                Id3Field {
                    frame_id: "TXXX:MusicIndex Contributors".into(),
                    value: "musician: HeyCitizen".into(),
                },
            ],
        };

        assert_eq!(
            super::id3_value_for_field("RSS track guid", &result).as_deref(),
            Some("track-guid")
        );
        assert_eq!(
            super::id3_value_for_field("RSS feed guid", &result).as_deref(),
            Some("feed-guid")
        );
    }

    #[test]
    fn pending_id3_conflicts_detect_duplicate_effective_targets() {
        let edits = BTreeMap::from([
            (
                "track".into(),
                PendingId3Edit {
                    field: "Track #".into(),
                    frame: "TRCK".into(),
                    value: "4".into(),
                    source: MetadataColumn::Rss,
                },
            ),
            (
                "total".into(),
                PendingId3Edit {
                    field: "Total tracks".into(),
                    frame: "TRCK".into(),
                    value: "10".into(),
                    source: MetadataColumn::MusicBrainz,
                },
            ),
            (
                "release".into(),
                PendingId3Edit {
                    field: "MusicBrainz release".into(),
                    frame: "TXXX:MusicBrainz Album Id".into(),
                    value: "release-id".into(),
                    source: MetadataColumn::MusicBrainz,
                },
            ),
        ]);

        assert_eq!(
            pending_id3_target_key("TXXX:MusicBrainz Album Id"),
            "TXXX:musicbrainz album id"
        );
        assert_eq!(
            pending_id3_conflict_descriptions(&edits),
            vec!["TRCK (Total tracks, Track #)"]
        );
    }

    fn test_compare_row(
        field: &str,
        rss_value: Option<&str>,
        id3_value: Option<&str>,
        id3_frame: Option<&str>,
        musicbrainz_value: Option<&str>,
    ) -> AlignedCompareRow {
        AlignedCompareRow {
            row_id: TrackMetadataGridVm::compare_row_id(field),
            field: field.into(),
            rss_value: rss_value.map(str::to_string),
            id3_value: id3_value.map(str::to_string),
            id3_frame: id3_frame.map(str::to_string),
            musicbrainz_value: musicbrainz_value.map(str::to_string),
            musicbrainz_key: None,
            id3_status: ComparisonStatus::MissingTag,
            musicbrainz_status: ComparisonStatus::MissingTag,
        }
    }
}
