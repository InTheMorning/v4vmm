#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use gpui::{
    div, img, prelude::*, px, rgb, size, AnyElement, App, Application, Bounds, ClickEvent, Context,
    Entity, FontWeight, Image, ImageFormat, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, Pixels, Point, Render, SharedString,
    Styled, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Root, Sizable, Size};
use reqwest::blocking::Client as ReqwestClient;
use rusqlite::Connection;

use crate::api::*;
#[cfg(test)]
use crate::audio_tags::Id3Field;
use crate::audio_tags::{read_audio_tags, write_id3v24_edits, EmbeddedArtwork, Id3v24Edit};
use crate::config;
use crate::db;
use crate::media::ImageCache;
use crate::metadata::*;
use crate::musicbrainz::{lookup_recordings, LookupMetadata, MusicBrainzCandidate};
use crate::rss;
use crate::track_compare::{download_track_mp3, local_mp3_path, ComparisonStatus};

#[derive(Clone, Debug)]
struct ResultRow {
    entity_type: String,
    entity_id: String,
    detail: Option<EntityDetail>,
}

#[derive(Clone, Debug)]
enum InspectorDetail {
    Loading(String),
    Error(String),
    Feed(Box<Feed>),
    Track(Box<TrackContext>),
    Publisher(Publisher),
}

#[derive(Clone, Debug, Default)]
enum LazyPanel<T> {
    #[default]
    Hidden,
    Loading,
    Empty(String),
    Loaded(T),
}

#[derive(Clone, Debug)]
struct InspectorFrame {
    entity_type: String,
    entity_id: String,
    title: String,
    detail: InspectorDetail,
    image: Option<Arc<Image>>,
    contributors: LazyPanel<Vec<Contributor>>,
    contributors_collapsed: bool,
    value_routes: LazyPanel<Vec<PaymentRoute>>,
    value_routes_collapsed: bool,
    expanded_id3_frame_groups: BTreeSet<String>,
    expanded_metadata_cells: BTreeSet<String>,
    pending_id3_edits: BTreeMap<String, PendingId3Edit>,
    suppressed_auto_id3_edits: BTreeSet<String>,
    applying_id3_edits: bool,
    id3_apply_error: Option<String>,
    local_subscription: Option<bool>,
    subscription_busy: bool,
    subscription_message: Option<String>,
    tag_compare: LazyPanel<TagCompareResult>,
    musicbrainz_lookup: LazyPanel<MusicBrainzLookupResult>,
    musicbrainz_selected: usize,
}

impl InspectorFrame {
    fn loading(entity_type: String, entity_id: String, title: String) -> Self {
        Self {
            entity_type,
            entity_id,
            title: title.clone(),
            detail: InspectorDetail::Loading(format!("Loading {title}...")),
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
        }
    }
}

struct DetailRow {
    key: String,
    value: AnyElement,
}

struct MetadataDragPreview {
    label: String,
    value: String,
}

impl Render for MetadataDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(320.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(accent())
            .bg(surface())
            .p(px(8.0))
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(muted())
                    .child(SharedString::from(self.label.clone())),
            )
            .child(
                div()
                    .mt(px(4.0))
                    .text_size(px(11.0))
                    .line_height(px(16.0))
                    .text_color(text())
                    .flex()
                    .flex_col()
                    .children(compare_value_line_elements(&self.value, 4)),
            )
    }
}

struct SearchBatch {
    rows: Vec<ResultRow>,
    has_more: bool,
    cursor: Option<String>,
}

#[derive(Clone)]
enum ThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
}

pub struct SearchApp {
    conn: Arc<Mutex<Connection>>,
    cache: Arc<ImageCache>,
    musicindex_endpoint: String,
    input: Entity<InputState>,
    type_filter: usize,
    fuzzy_search: bool,
    results: Vec<ResultRow>,
    loading: bool,
    status: String,
    cursor: Option<String>,
    has_more: bool,
    selected_key: Option<String>,
    inspector_stack: Vec<InspectorFrame>,
    inspector_origin: Option<InspectorOrigin>,
    recent_feeds: Vec<Feed>,
    recent_cursor: Option<String>,
    recent_has_more: bool,
    recent_loading: bool,
    recent_status: String,
    recent_loaded_once: bool,
    left_pane_width: gpui::Pixels,
    resizing: bool,
    thumbnails: BTreeMap<String, ThumbnailState>,
    _input_sub: gpui::Subscription,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorOrigin {
    Recents,
    Search,
}

const TYPE_LABELS: &[&str] = &["All", "Feed", "Track", "Publisher"];
const TYPE_VALUES: &[Option<&str>] = &[None, Some("feed"), Some("track"), Some("publisher")];

impl SearchApp {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        cache: Arc<ImageCache>,
        musicindex_endpoint: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder("Discover feeds, tracks, publishers...")
        });
        let input_sub = cx.subscribe(&input, Self::on_input_event);

        let mut this = Self {
            conn,
            cache,
            musicindex_endpoint,
            input,
            type_filter: 0,
            fuzzy_search: true,
            results: Vec::new(),
            loading: false,
            status: String::new(),
            cursor: None,
            has_more: false,
            selected_key: None,
            inspector_stack: Vec::new(),
            inspector_origin: None,
            recent_feeds: Vec::new(),
            recent_cursor: None,
            recent_has_more: false,
            recent_loading: false,
            recent_status: String::new(),
            recent_loaded_once: false,
            left_pane_width: px(360.0),
            resizing: false,
            thumbnails: BTreeMap::new(),
            _input_sub: input_sub,
        };
        this.load_recent_feeds(false, cx);
        this
    }

    pub fn set_musicindex_endpoint(&mut self, endpoint: String, cx: &mut Context<Self>) {
        if self.musicindex_endpoint == endpoint {
            return;
        }

        self.musicindex_endpoint = endpoint;
        self.results.clear();
        self.loading = false;
        self.status = "MusicIndex endpoint updated".into();
        self.cursor = None;
        self.has_more = false;
        self.selected_key = None;
        self.inspector_stack.clear();
        self.inspector_origin = None;
        self.recent_feeds.clear();
        self.recent_cursor = None;
        self.recent_has_more = false;
        self.recent_loaded_once = false;
        self.recent_status.clear();
        self.load_recent_feeds(false, cx);
        cx.notify();
    }

    fn load_recent_feeds(&mut self, append: bool, cx: &mut Context<Self>) {
        if self.recent_loading {
            return;
        }
        self.recent_loading = true;
        if !append {
            self.recent_feeds.clear();
            self.recent_cursor = None;
            self.recent_has_more = false;
        }
        self.recent_status = if append {
            "Loading more recent feeds...".into()
        } else {
            "Loading recent feeds...".into()
        };
        cx.notify();

        let client = self.api_client();
        let cursor = if append {
            self.recent_cursor.clone()
        } else {
            None
        };
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let result =
                    cx.background_executor()
                        .spawn(async move {
                            client.fetch_recent_feeds(Some(PAGE_LIMIT), cursor.as_deref())
                        })
                        .await;
                let _ = this.update(cx, move |this, cx| {
                    this.recent_loading = false;
                    this.recent_loaded_once = true;
                    match result {
                        Ok(response) => {
                            this.recent_feeds.extend(response.data);
                            this.recent_cursor = response.pagination.cursor;
                            this.recent_has_more = response.pagination.has_more;
                            this.recent_status.clear();
                        }
                        Err(error) => {
                            this.recent_status = format!("Error: {error}");
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
        if self.loading {
            return;
        }

        let query = self.input.read(cx).value().trim().to_string();
        if query.is_empty() {
            return;
        }

        self.loading = true;
        self.status = if append {
            "Loading more...".into()
        } else {
            "Discovering...".into()
        };

        if !append {
            self.results.clear();
            self.cursor = None;
            self.has_more = false;
            self.selected_key = None;
            self.inspector_stack.clear();
            self.inspector_origin = None;
        }
        cx.notify();

        let entity_type = TYPE_VALUES[self.type_filter].map(str::to_string);
        let cursor = if append { self.cursor.clone() } else { None };
        let fuzzy = self.fuzzy_search;
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
                            Ok(batch) => this.apply_search_batch(batch, append),
                            Err(error) => {
                                this.loading = false;
                                this.status = format!("Error: {error}");
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

    fn apply_search_batch(&mut self, batch: SearchBatch, append: bool) {
        if !append && batch.rows.is_empty() {
            self.status.clear();
            self.results.clear();
            self.loading = false;
            self.has_more = false;
            self.cursor = None;
            return;
        }

        self.results.extend(batch.rows);
        self.cursor = batch.cursor;
        self.has_more = batch.has_more;
        self.loading = false;

        let total = self.results.len();
        self.status = format!(
            "{total} result{}{}",
            if total == 1 { "" } else { "s" },
            if self.has_more { "+" } else { "" }
        );
    }

    fn toggle_fuzzy_search(&mut self, cx: &mut Context<Self>) {
        self.fuzzy_search = !self.fuzzy_search;
        let has_query = !self.input.read(cx).value().trim().is_empty();
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
        self.selected_key = Some(entity_key(&entity_type, &entity_id));
        self.inspector_origin = Some(InspectorOrigin::Search);
        self.load_inspector(entity_type, entity_id, title, false, cx);
    }

    fn open_recent_feed(&mut self, feed_guid: String, title: String, cx: &mut Context<Self>) {
        self.selected_key = Some(entity_key("feed", &feed_guid));
        self.inspector_origin = Some(InspectorOrigin::Recents);
        self.load_inspector("feed".into(), feed_guid, title, false, cx);
    }

    fn push_inspector(
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
                                        frame.detail = detail;
                                        frame.image = image;
                                        frame.local_subscription = local_subscription;
                                        frame.subscription_message = None;
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

    fn inspector_back(&mut self, cx: &mut Context<Self>) {
        if self.inspector_stack.is_empty() {
            return;
        }
        self.inspector_stack.pop();
        if self.inspector_stack.is_empty() {
            self.inspector_origin = None;
            self.selected_key = None;
        }
        cx.notify();
    }

    fn toggle_contributors(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };

        if matches!(frame.tag_compare, LazyPanel::Loaded(_)) {
            frame.contributors_collapsed = !frame.contributors_collapsed;
            cx.notify();
            return;
        }

        match frame.contributors {
            LazyPanel::Loaded(_) => {
                frame.contributors_collapsed = !frame.contributors_collapsed;
                cx.notify();
                return;
            }
            LazyPanel::Loading => return,
            LazyPanel::Empty(_) => {
                frame.contributors_collapsed = !frame.contributors_collapsed;
                cx.notify();
                return;
            }
            LazyPanel::Hidden => {
                frame.contributors = LazyPanel::Loading;
                frame.contributors_collapsed = false;
            }
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
                            frame.contributors = match contributors {
                                Ok(items) if items.is_empty() => {
                                    LazyPanel::Empty("No contributors found".into())
                                }
                                Ok(items) => LazyPanel::Loaded(items),
                                Err(error) => LazyPanel::Empty(format!("Error: {error}")),
                            };
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

        if matches!(frame.tag_compare, LazyPanel::Loaded(_)) {
            frame.value_routes_collapsed = !frame.value_routes_collapsed;
            cx.notify();
            return;
        }

        match frame.value_routes {
            LazyPanel::Loaded(_) => {
                frame.value_routes_collapsed = !frame.value_routes_collapsed;
                cx.notify();
                return;
            }
            LazyPanel::Loading => return,
            LazyPanel::Empty(_) => {
                frame.value_routes_collapsed = !frame.value_routes_collapsed;
                cx.notify();
                return;
            }
            LazyPanel::Hidden => {
                frame.value_routes = LazyPanel::Loading;
                frame.value_routes_collapsed = false;
            }
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
                            frame.value_routes = match routes {
                                Ok(items) if items.is_empty() => {
                                    LazyPanel::Empty("No value routes found".into())
                                }
                                Ok(items) => LazyPanel::Loaded(items),
                                Err(error) => LazyPanel::Empty(format!("Error: {error}")),
                            };
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
            | InspectorDetail::Feed(_)
            | InspectorDetail::Publisher(_) => return,
        };
        let rows = track_metadata_rows_for_frame(frame, &track_context, Some(result));
        let pending_id3_edits = auto_populated_pending_id3_edits(
            &rows,
            &frame.pending_id3_edits,
            &frame.suppressed_auto_id3_edits,
        );
        if pending_id3_edits.is_empty() {
            return;
        }
        let conflicts = pending_id3_conflict_descriptions(&pending_id3_edits);
        if !conflicts.is_empty() {
            frame.id3_apply_error = Some(format!(
                "Resolve duplicate ID3 target{}: {}",
                if conflicts.len() == 1 { "" } else { "s" },
                conflicts.join("; ")
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
                                        frame.id3_apply_error =
                                            Some(format!("Error applying ID3 edits: {error}"));
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
                    );
                    let conflicts = pending_id3_conflict_descriptions(&pending);
                    if !conflicts.is_empty() {
                        frame.subscription_message = Some(format!(
                            "Resolve duplicate ID3 target{}: {}",
                            if conflicts.len() == 1 { "" } else { "s" },
                            conflicts.join("; ")
                        ));
                        cx.notify();
                        return;
                    }
                    pending_id3_edits_for_apply(&pending)
                } else {
                    Vec::new()
                };
                SearchSubscribeRequest::Track(Box::new((**track_context).clone()), edits)
            }
            InspectorDetail::Loading(_)
            | InspectorDetail::Error(_)
            | InspectorDetail::Publisher(_) => return,
        };

        frame.subscription_busy = true;
        frame.subscription_message = Some("Subscribing...".into());
        cx.notify();

        let conn = Arc::clone(&self.conn);
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { subscribe_search_request(conn, request) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                frame.subscription_busy = false;
                                match result {
                                    Ok(outcome) => {
                                        frame.local_subscription = Some(true);
                                        frame.subscription_message = Some(outcome.message);
                                        if let Some(compare) = outcome.compare {
                                            frame.tag_compare = LazyPanel::Loaded(compare);
                                            frame.pending_id3_edits.clear();
                                            frame.suppressed_auto_id3_edits.clear();
                                            frame.id3_apply_error = None;
                                        }
                                    }
                                    Err(error) => {
                                        frame.subscription_message =
                                            Some(format!("Subscribe error: {error:#}"));
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
            | InspectorDetail::Publisher(_) => return,
        };

        frame.subscription_busy = true;
        frame.subscription_message = Some("Unsubscribing...".into());
        cx.notify();

        let conn = Arc::clone(&self.conn);
        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { unsubscribe_search_request(conn, request) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                frame.subscription_busy = false;
                                match result {
                                    Ok(message) => {
                                        frame.local_subscription = Some(false);
                                        frame.subscription_message = Some(message);
                                    }
                                    Err(error) => {
                                        frame.subscription_message =
                                            Some(format!("Unsubscribe error: {error:#}"));
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
                                    Err(error) => LazyPanel::Empty(format!("Error: {error}")),
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
                                    Err(error) => LazyPanel::Empty(format!("Error: {error}")),
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
                                    Err(error) => LazyPanel::Empty(format!("Error: {error}")),
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
        let status_text = self.status.clone();
        let status_color = if status_text.starts_with("Error:") {
            rgb(0xff6b6b)
        } else {
            muted()
        };
        let status_empty = status_text.is_empty();

        let rows = self.results.clone();
        let selected_key = self.selected_key.clone();
        let results: Vec<AnyElement> = rows
            .iter()
            .map(|row| {
                let image_url = result_image_url(row);
                let thumbnail = self.thumbnail_for_url(image_url.as_deref(), cx);
                render_result_item(row, selected_key.as_deref(), thumbnail.as_ref(), cx)
            })
            .collect();
        let type_filters: Vec<AnyElement> = TYPE_LABELS
            .iter()
            .enumerate()
            .map(|(idx, label)| render_filter_button(idx, label, idx == self.type_filter, cx))
            .collect();
        let stack = self.inspector_stack.clone();
        let show_back = should_show_inspector_back(stack.len());
        let input_is_empty = self.input.read(cx).value().trim().is_empty();
        let show_recents_root = stack.is_empty()
            && self.inspector_origin.is_none()
            && self.results.is_empty()
            && input_is_empty;
        let inspector = render_inspector(stack.last(), show_back, show_recents_root, self, cx);
        let active_input = self.input.clone();
        let is_loading = self.loading;
        let is_empty = self.results.is_empty();
        let has_more = self.has_more;
        let search_label = "Search Index";

        div()
            .size_full()
            .bg(bg())
            .text_color(text())
            .text_sm()
            .flex()
            .overflow_hidden()
            .child(
                div()
                    .id("pane-container")
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                        if this.resizing {
                            let x = event.position.x;
                            let clamped = x.max(px(200.0)).min(px(800.0));
                            this.left_pane_width = clamped;
                            cx.notify();
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                            if this.resizing {
                                this.resizing = false;
                                cx.notify();
                            }
                        }),
                    )
                    .child(
                        div()
                            .w(self.left_pane_width)
                            .min_w(px(200.0))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .p(px(12.0))
                                    .border_b_1()
                                    .border_color(border())
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(muted())
                                            .child("Search Index"),
                                    )
                                    .child(
                                        Input::new(&active_input)
                                            .cleanable(true)
                                            .with_size(Size::Small),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .flex_wrap()
                                            .gap(px(6.0))
                                            .children(type_filters),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(
                                                Button::new("search-btn")
                                                    .label(search_label)
                                                    .primary()
                                                    .with_size(Size::Small)
                                                    .text_color(rgb(0xffffff))
                                                    .loading(is_loading)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.do_search(false, cx);
                                                    })),
                                            )
                                            .child(
                                                Button::new("fuzzy-toggle")
                                                    .label(if self.fuzzy_search {
                                                        "Fuzzy: On"
                                                    } else {
                                                        "Fuzzy: Off"
                                                    })
                                                    .with_size(Size::Small)
                                                    .when(self.fuzzy_search, |button| {
                                                        button.primary()
                                                    })
                                                    .when(!self.fuzzy_search, |button| {
                                                        button.ghost()
                                                    })
                                                    .text_color(rgb(0xffffff))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_fuzzy_search(cx);
                                                    })),
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
                                    .id("results-scroll")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .p(px(8.0))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.0))
                                            .children(results)
                                            .when(is_empty && !is_loading && status_empty, |el| {
                                                el.child(
                                                    div()
                                                        .text_center()
                                                        .p(px(48.0))
                                                        .text_color(muted())
                                                        .child(div().text_2xl().child("🔍"))
                                                        .child(
                                                            div().mt(px(8.0)).child("No results"),
                                                        ),
                                                )
                                            })
                                            .when(is_empty && !is_loading && !status_empty, |el| {
                                                el.child(
                                                    div()
                                                        .text_center()
                                                        .p(px(48.0))
                                                        .text_color(muted())
                                                        .child(div().text_2xl().child("🔍"))
                                                        .child(
                                                            div().mt(px(8.0)).child("No results"),
                                                        ),
                                                )
                                            })
                                            .when(has_more && !is_loading, |el| {
                                                el.child(
                                                    Button::new("load-more")
                                                        .label("Load more")
                                                        .ghost()
                                                        .with_size(Size::Small)
                                                        .text_color(rgb(0xffffff))
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.do_search(true, cx);
                                                        })),
                                                )
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("resize-handle")
                            .w(px(5.0))
                            .cursor_col_resize()
                            .bg(border())
                            .hover(|s| s.bg(accent()))
                            .flex_shrink_0()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                                    this.resizing = true;
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(inspector),
                    ),
            )
    }
}

fn fetch_search_batch(
    client: &Client,
    query: &str,
    entity_type: Option<&str>,
    cursor: Option<&str>,
    fuzzy: bool,
) -> Result<SearchBatch> {
    if entity_type == Some("publisher") {
        let response = client.search_publishers(query, Some(PAGE_LIMIT), fuzzy)?;
        let rows = response
            .data
            .into_iter()
            .map(|publisher| {
                let entity_id = publisher.publisher_text.clone().unwrap_or_default();
                ResultRow {
                    entity_type: "publisher".into(),
                    entity_id,
                    detail: Some(EntityDetail::Publisher(publisher)),
                }
            })
            .collect();

        return Ok(SearchBatch {
            rows,
            has_more: response.pagination.has_more,
            cursor: response.pagination.cursor,
        });
    }

    let response = client.search(query, entity_type, Some(PAGE_LIMIT), cursor, fuzzy)?;
    let rows = response
        .data
        .iter()
        .map(|hit| {
            let detail = client
                .fetch_detail(&hit.entity_type, &hit.entity_id)
                .ok()
                .filter(|detail| matches!(detail, EntityDetail::Feed(_) | EntityDetail::Track(_)));
            ResultRow {
                entity_type: hit.entity_type.clone(),
                entity_id: hit.entity_id.clone(),
                detail,
            }
        })
        .collect();

    Ok(SearchBatch {
        rows,
        has_more: response.pagination.has_more,
        cursor: response.pagination.cursor,
    })
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
        "feed" => {
            let feed = client.fetch_feed(
                entity_id,
                Some(
                    "tracks,source_links,source_ids,source_release_claims,source_contributors,payment_routes",
                ),
            )?;
            let image = feed
                .image_url
                .as_deref()
                .and_then(|url| download_image(&client.client, url));
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
                        Some("tracks,source_links,source_ids,source_release_claims,payment_routes"),
                    )
                    .ok()
            });
            enrich_track_context_from_rss(&mut track, feed.as_mut());
            let image = track
                .image_url
                .as_deref()
                .and_then(|url| download_image(&client.client, url));
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

fn download_and_compare_track(
    client: &Client,
    entity_id: &str,
    force_download: bool,
) -> Result<TagCompareResult> {
    let mut track = client.fetch_track(
        entity_id,
        Some("source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes"),
    )?;
    let mut feed = match track.feed_guid.as_deref() {
        Some(feed_guid) => client
            .fetch_feed(
                feed_guid,
                Some("tracks,source_links,source_ids,source_release_claims"),
            )
            .ok(),
        None => None,
    };
    enrich_track_context_from_rss(&mut track, feed.as_mut());
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let path = local_mp3_path(&cfg, &track);
    if force_download || !path.exists() {
        download_track_mp3(&cfg, &client.client, &track)?;
    }
    let track_context = TrackContext { track, feed };

    compare_downloaded_track_path(&path, &track_context)
}

fn enrich_track_context_from_rss(track: &mut Track, feed: Option<&mut Feed>) {
    let feed_url = track
        .feed_url
        .clone()
        .or_else(|| feed.as_ref().and_then(|feed| feed.feed_url.clone()));
    let Some(feed_url) = feed_url else {
        return;
    };
    let _ = rss::enrich_track_from_feed_rss(track, feed, &feed_url);
}

fn compare_downloaded_track_path(
    path: &Path,
    track_context: &TrackContext,
) -> Result<TagCompareResult> {
    let tags = read_audio_tags(path)?;
    let file_image = tags.artwork.as_ref().and_then(image_from_artwork);
    let track = &track_context.track;

    Ok(TagCompareResult {
        path: path.display().to_string(),
        rows: compare_track_rows(track, track_context.feed.as_ref(), &tags),
        file_image,
        contributors: track.source_contributors.clone().unwrap_or_default(),
        value_routes: track.payment_routes.clone().unwrap_or_default(),
        total_tracks: tags.total_tracks.clone(),
        id3_fields: tags.fields,
    })
}

enum SearchSubscribeRequest {
    Feed(Box<Feed>, String),
    Track(Box<TrackContext>, Vec<Id3v24Edit>),
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

struct SearchSubscribeOutcome {
    message: String,
    compare: Option<TagCompareResult>,
}

fn subscribe_search_request(
    conn: Arc<Mutex<Connection>>,
    request: SearchSubscribeRequest,
) -> Result<SearchSubscribeOutcome> {
    match request {
        SearchSubscribeRequest::Feed(feed, musicindex_endpoint) => {
            subscribe_feed_from_search(conn, *feed, musicindex_endpoint)
        }
        SearchSubscribeRequest::Track(track_context, edits) => {
            subscribe_track_from_search(conn, *track_context, edits)
        }
    }
}

fn subscribe_feed_from_search(
    conn: Arc<Mutex<Connection>>,
    feed: Feed,
    musicindex_endpoint: String,
) -> Result<SearchSubscribeOutcome> {
    let feed_url = feed
        .feed_url
        .clone()
        .ok_or_else(|| anyhow!("feed has no RSS URL"))?;
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;

    {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        rss::cmd_subscribe(&cfg, &mut db, &feed_url)?;
    }

    let client = ReqwestClient::new();
    let api_client = Client::new_with_base_url(musicindex_endpoint);
    let mut downloaded = 0usize;
    let mut applied_edits = 0usize;
    let mut skipped = 0usize;
    for track in feed.tracks.clone().unwrap_or_default() {
        let mut track = track_with_feed_defaults(track, Some(&feed));
        if let Some(track_guid) = track.track_guid.as_deref() {
            if let Ok(hydrated) = api_client.fetch_track(
                track_guid,
                Some(
                    "source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes",
                ),
            ) {
                track = track_with_feed_defaults(hydrated, Some(&feed));
            }
        }
        let mut context_feed = feed.clone();
        enrich_track_context_from_rss(&mut track, Some(&mut context_feed));
        let track_context = TrackContext {
            track: track.clone(),
            feed: Some(context_feed),
        };
        let edits = id3_edits_for_track_context(&track_context);
        match download_track_for_subscription(&cfg, &client, &track) {
            Ok((path, file_size)) => {
                if !edits.is_empty() {
                    write_id3v24_edits(&path, &edits)?;
                    applied_edits += edits.len();
                }
                let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
                let marked = db::mark_track_downloaded_by_match(
                    &db,
                    track.feed_url.as_deref().or(Some(feed_url.as_str())),
                    track.track_guid.as_deref(),
                    track.enclosure_url.as_deref(),
                    &path,
                    file_size,
                )?;
                if marked {
                    downloaded += 1;
                } else {
                    skipped += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }

    let message = if skipped == 0 {
        format!(
            "Subscribed feed; downloaded {downloaded} track{}, applied {applied_edits} ID3 edit{}",
            plural(downloaded),
            plural(applied_edits)
        )
    } else {
        format!(
            "Subscribed feed; downloaded {downloaded} track{}, applied {applied_edits} ID3 edit{}, skipped {skipped}",
            plural(downloaded),
            plural(applied_edits)
        )
    };
    Ok(SearchSubscribeOutcome {
        message,
        compare: None,
    })
}

fn subscribe_track_from_search(
    conn: Arc<Mutex<Connection>>,
    track_context: TrackContext,
    edits: Vec<Id3v24Edit>,
) -> Result<SearchSubscribeOutcome> {
    let mut feed = track_context.feed;
    let mut track = track_with_feed_defaults(track_context.track.clone(), feed.as_ref());
    enrich_track_context_from_rss(&mut track, feed.as_mut());
    let feed_url = track
        .feed_url
        .clone()
        .or_else(|| feed.as_ref().and_then(|feed| feed.feed_url.clone()))
        .ok_or_else(|| anyhow!("track has no RSS feed URL"))?;
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;

    {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        rss::cmd_subscribe(&cfg, &mut db, &feed_url)?;
    }

    let client = ReqwestClient::new();
    let (path, file_size) = download_track_for_subscription(&cfg, &client, &track)?;
    if !edits.is_empty() {
        write_id3v24_edits(&path, &edits)?;
    }

    {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        db::mark_track_downloaded_by_match(
            &db,
            Some(feed_url.as_str()),
            track.track_guid.as_deref(),
            track.enclosure_url.as_deref(),
            &path,
            file_size,
        )?;
    }

    let refreshed_context = TrackContext { track, feed };
    let compare = compare_downloaded_track_path(&path, &refreshed_context)?;
    let edit_text = if edits.is_empty() {
        String::new()
    } else {
        format!(", applied {} ID3 edit{}", edits.len(), plural(edits.len()))
    };
    Ok(SearchSubscribeOutcome {
        message: format!("Subscribed track{edit_text}"),
        compare: Some(compare),
    })
}

pub fn id3_edits_for_track_context(track_context: &TrackContext) -> Vec<Id3v24Edit> {
    let rows = expand_woar_metadata_rows(track_metadata_rows(track_context, None, false));
    let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new());
    pending_id3_edits_for_apply(&pending)
}

fn unsubscribe_search_request(
    conn: Arc<Mutex<Connection>>,
    request: SearchUnsubscribeRequest,
) -> Result<String> {
    let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    match request {
        SearchUnsubscribeRequest::Feed { feed_url } => {
            let feed_url = feed_url.ok_or_else(|| anyhow!("feed has no RSS URL"))?;
            db::set_feed_subscribed_by_url(&db, &feed_url, false)?;
            Ok("Unsubscribed feed".into())
        }
        SearchUnsubscribeRequest::Track {
            feed_url,
            item_guid,
            enclosure_url,
        } => {
            db::set_track_in_library_by_match(
                &db,
                feed_url.as_deref(),
                item_guid.as_deref(),
                enclosure_url.as_deref(),
                false,
            )?;
            Ok("Unsubscribed track".into())
        }
    }
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
            db::track_is_in_library_by_match(
                &db,
                feed_url,
                track_context.track.track_guid.as_deref(),
                track_context.track.enclosure_url.as_deref(),
            )
            .map(Some)
        }
        InspectorDetail::Loading(_) | InspectorDetail::Error(_) | InspectorDetail::Publisher(_) => {
            Ok(None)
        }
    }
}

fn download_track_for_subscription(
    cfg: &config::Config,
    client: &ReqwestClient,
    track: &Track,
) -> Result<(PathBuf, Option<i64>)> {
    let path = local_mp3_path(cfg, track);
    if !path.exists() {
        download_track_mp3(cfg, client, track)?;
    }
    let file_size = std::fs::metadata(&path)
        .ok()
        .and_then(|metadata| metadata.len().try_into().ok());
    Ok((path, file_size))
}

fn track_with_feed_defaults(track: Track, feed: Option<&Feed>) -> Track {
    crate::api::track_with_feed_defaults(track, feed)
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn lookup_musicbrainz_track(client: &Client, entity_id: &str) -> Result<MusicBrainzLookupResult> {
    let track = client.fetch_track(entity_id, Some("source_enclosures"))?;
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let downloaded = download_track_mp3(&cfg, &client.client, &track)?;
    let tags = read_audio_tags(&downloaded.path)?;
    let metadata = musicbrainz_lookup_metadata(&track, &tags);
    let musicbrainz_client = ReqwestClient::builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&musicbrainz_client, &metadata, 5)?;

    let image = lookup
        .candidates
        .first()
        .and_then(|c| c.release_id.as_deref())
        .and_then(|release_id| {
            let url = format!("https://coverartarchive.org/release/{release_id}/front-250");
            download_image(&musicbrainz_client, &url)
        });

    Ok(MusicBrainzLookupResult { lookup, image })
}

fn musicbrainz_lookup_metadata(
    track: &Track,
    tags: &crate::audio_tags::AudioTags,
) -> LookupMetadata {
    LookupMetadata {
        title: tags
            .title
            .clone()
            .or_else(|| track.title.clone())
            .or_else(|| track.name.clone()),
        artist: tags.artist.clone().or_else(|| track.track_artist.clone()),
        album: tags.album.clone().or_else(|| track.feed_title.clone()),
        track_number: tags
            .track_number
            .clone()
            .or_else(|| track.track_number.map(|number| number.to_string())),
        total_tracks: None,
        duration_secs: track.duration_secs.map(i64::from),
        isrc: tags
            .custom
            .get("ISRC")
            .cloned()
            .or_else(|| tags.custom.get("isrc").cloned()),
    }
}

fn download_image(client: &ReqwestClient, url: &str) -> Option<Arc<Image>> {
    let response = client.get(url).send().ok()?.error_for_status().ok()?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let format = ImageFormat::from_mime_type(content_type).unwrap_or(ImageFormat::Jpeg);
    let bytes = response.bytes().ok()?.to_vec();
    if bytes.is_empty() {
        return None;
    }
    Some(Arc::new(Image::from_bytes(format, bytes)))
}

fn image_from_artwork(artwork: &EmbeddedArtwork) -> Option<Arc<Image>> {
    if artwork.data.is_empty() {
        return None;
    }
    let format = ImageFormat::from_mime_type(&artwork.mime_type).unwrap_or(ImageFormat::Jpeg);
    Some(Arc::new(Image::from_bytes(format, artwork.data.clone())))
}

fn render_filter_button(
    idx: usize,
    label: &str,
    selected: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    Button::new(("type-filter", idx))
        .label(SharedString::from(label.to_string()))
        .with_size(Size::Small)
        .when(selected, |button| button.primary())
        .when(!selected, |button| button.ghost())
        .text_color(rgb(0xffffff))
        .on_click(cx.listener(move |this, _, _, cx| {
            if this.type_filter != idx {
                this.type_filter = idx;
                let has_query = !this.input.read(cx).value().trim().is_empty();
                cx.notify();
                if has_query {
                    this.do_search(false, cx);
                }
            }
        }))
        .into_any_element()
}

fn render_result_item(
    row: &ResultRow,
    selected_key: Option<&str>,
    thumbnail: Option<&Arc<Image>>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let (line1, line2, line3, _image_url) = result_lines(row);
    let is_selected = selected_key == Some(entity_key(&row.entity_type, &row.entity_id).as_str());
    let entity_type = row.entity_type.clone();
    let entity_id = row.entity_id.clone();
    let title = if line1.is_empty() {
        entity_id.clone()
    } else {
        line1.clone()
    };

    div()
        .id(SharedString::from(format!(
            "result-item:{}:{}",
            row.entity_type, row.entity_id
        )))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .p(px(8.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .bg(if is_selected { surface() } else { bg() })
        .border_1()
        .border_color(if is_selected { accent() } else { bg() })
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.select_result(entity_type.clone(), entity_id.clone(), title.clone(), cx);
        }))
        .child(render_thumb(thumbnail, &row.entity_type, 36.0, false))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(truncated(line1).font_weight(FontWeight::MEDIUM))
                .when(!line2.is_empty(), |el| {
                    el.child(truncated_muted(line2).text_size(px(10.5)))
                })
                .when(!line3.is_empty(), |el| {
                    el.child(truncated_muted(line3).text_size(px(10.0)).opacity(0.7))
                }),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(9.0))
                .font_weight(FontWeight::BOLD)
                .text_color(badge_text(&row.entity_type))
                .bg(type_color(&row.entity_type))
                .px(px(5.0))
                .py(px(1.0))
                .rounded(px(3.0))
                .child(SharedString::from(row.entity_type.clone())),
        )
        .into_any_element()
}

fn render_inspector(
    frame: Option<&InspectorFrame>,
    show_back: bool,
    show_recents_root: bool,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let title = if show_recents_root && frame.is_none() {
        "Recent Feeds"
    } else {
        frame.map_or("", |frame| frame.title.as_str())
    };
    div()
        .flex()
        .flex_col()
        .size_full()
        .overflow_hidden()
        .child(
            div()
                .min_h(px(36.0))
                .bg(surface())
                .border_b_1()
                .border_color(border())
                .px(px(12.0))
                .py(px(6.0))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .when(show_back, |el| {
                    el.child(
                        Button::new("inspector-back")
                            .label("← Back")
                            .ghost()
                            .with_size(Size::Small)
                            .text_color(rgb(0xffffff))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.inspector_back(cx);
                            })),
                    )
                })
                .child(truncated_muted(title.to_string()).flex_1()),
        )
        .child(
            div()
                .id("inspector-scroll")
                .flex_1()
                .overflow_y_scroll()
                .p(px(20.0))
                .child(match frame {
                    Some(frame) => render_inspector_body(frame, app, cx),
                    None if show_recents_root => render_recent_feeds_tiles(app, cx),
                    None => render_inspector_empty(),
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
        InspectorDetail::Loading(message) => render_loading(message),
        InspectorDetail::Error(error) => render_loading(&format!("Error: {error}")),
        InspectorDetail::Feed(feed) => render_discover_feed_inspector(frame, feed, app, cx),
        InspectorDetail::Track(track_context) => {
            render_discover_track_inspector(frame, track_context, cx)
        }
        InspectorDetail::Publisher(publisher) => render_publisher_inspector(publisher, app, cx),
    }
}

fn render_discover_feed_inspector(
    frame: &InspectorFrame,
    feed: &Feed,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let title = feed_title(feed);
    let artist = feed
        .release_artist
        .clone()
        .unwrap_or_else(|| "Unknown".into());
    let mut rows = vec![
        (
            "Release Kind".to_string(),
            feed.release_kind
                .clone()
                .unwrap_or_else(|| "Unknown".into()),
        ),
        (
            "Publisher".to_string(),
            feed.publisher_text
                .clone()
                .unwrap_or_else(|| "Unknown".into()),
        ),
    ];
    optional_row(
        &mut rows,
        "Release Date",
        feed.release_date.and_then(fmt_date),
    );
    optional_row(&mut rows, "Language", feed.language.clone());
    if feed.explicit == Some(true) {
        rows.push(("Explicit".into(), "Yes".into()));
    }
    optional_row(
        &mut rows,
        "Tracks",
        feed.episode_count.map(|n| n.to_string()),
    );

    let mut tracks = feed.tracks.clone().unwrap_or_default();
    tracks.sort_by(|a, b| {
        let a_num = a.track_number.unwrap_or(i32::MAX);
        let b_num = b.track_number.unwrap_or(i32::MAX);
        a_num.cmp(&b_num).then_with(|| {
            b.pub_date
                .unwrap_or_default()
                .cmp(&a.pub_date.unwrap_or_default())
        })
    });
    let total_secs: i32 = tracks.iter().filter_map(|track| track.duration_secs).sum();

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(render_detail_header(
            "feed",
            &title,
            Some(artist.as_str()),
            frame.image.as_ref(),
        ))
        .child(render_action_row(frame, &BTreeMap::new(), cx))
        .child(render_detail_grid(rows))
        .when(feed.description.is_some(), |el| {
            el.child(render_collapsed_text_section(
                "Description",
                feed.description.clone().unwrap_or_default(),
            ))
        })
        .when(!tracks.is_empty(), |el| {
            el.child(render_track_list_section(
                "Tracks",
                format!(
                    "{} total{}",
                    tracks.len(),
                    if total_secs > 0 {
                        format!(" · {}", fmt_runtime(total_secs))
                    } else {
                        String::new()
                    }
                ),
                tracks,
                app,
                cx,
            ))
        })
        .child(render_lazy_sections(frame, cx))
        .into_any_element()
}

fn render_discover_track_inspector(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let track = &track_context.track;
    let title = track_title(track);
    let artist = track
        .track_artist
        .clone()
        .or_else(|| track.release_artist.clone())
        .unwrap_or_else(|| "Unknown".into());
    let mut rows = vec![(
        "Release".to_string(),
        track.feed_title.clone().unwrap_or_else(|| "Unknown".into()),
    )];
    optional_row(
        &mut rows,
        "Track #",
        track.track_number.map(|number| number.to_string()),
    );
    optional_row(&mut rows, "Duration", track.duration_secs.map(fmt_dur));
    optional_row(&mut rows, "Release Date", track.pub_date.and_then(fmt_date));
    optional_row(&mut rows, "Publisher", track.publisher_text.clone());

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(render_detail_header(
            "track",
            &title,
            Some(artist.as_str()),
            frame.image.as_ref(),
        ))
        .child(render_action_row(frame, &BTreeMap::new(), cx))
        .child(render_detail_grid(rows))
        .when(track.description.is_some(), |el| {
            el.child(render_collapsed_text_section(
                "Description",
                track.description.clone().unwrap_or_default(),
            ))
        })
        .child(render_lazy_sections(frame, cx))
        .into_any_element()
}

fn render_track_inspector(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    match &frame.tag_compare {
        LazyPanel::Loaded(result) => render_track_window(frame, track_context, Some(result), cx),
        LazyPanel::Loading | LazyPanel::Empty(_) | LazyPanel::Hidden => {
            render_track_window(frame, track_context, None, cx)
        }
    }
}

fn render_track_left_column(
    frame: &InspectorFrame,
    track: &Track,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(render_track_header(frame, track, cx))
        .child(render_action_row(frame, pending_id3_edits, cx))
        .into_any_element()
}

fn render_track_compare_panel(frame: &InspectorFrame) -> AnyElement {
    match &frame.tag_compare {
        LazyPanel::Loaded(_) => div().into_any_element(),
        LazyPanel::Loading => render_loading("Downloading and reading embedded metadata..."),
        LazyPanel::Empty(label) => render_loading(label),
        LazyPanel::Hidden => div().into_any_element(),
    }
}

fn render_track_window(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let track = &track_context.track;
    let show_id3_panel = !matches!(frame.tag_compare, LazyPanel::Hidden);
    let show_musicbrainz_panel = !matches!(frame.musicbrainz_lookup, LazyPanel::Hidden);
    let columns: u16 = 1 + u16::from(show_id3_panel) + u16::from(show_musicbrainz_panel);
    let rows = track_metadata_rows_for_frame(frame, track_context, result);
    let pending_id3_edits = auto_populated_pending_id3_edits(
        &rows,
        &frame.pending_id3_edits,
        &frame.suppressed_auto_id3_edits,
    );

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .grid()
                .grid_cols(columns)
                .gap(px(24.0))
                .items_start()
                .child(render_track_left_column(
                    frame,
                    track,
                    &pending_id3_edits,
                    cx,
                ))
                .when(show_id3_panel, |el| {
                    el.child(if let Some(result) = result {
                        render_file_header(result, cx)
                    } else {
                        render_track_compare_panel(frame)
                    })
                })
                .when(show_musicbrainz_panel, |el| {
                    el.child(render_musicbrainz_panel(frame, cx))
                }),
        )
        .child(render_track_metadata_grid(
            rows,
            show_id3_panel,
            show_musicbrainz_panel,
            &pending_id3_edits,
            &frame.expanded_metadata_cells,
            result.and_then(|r| r.file_image.clone()),
            cx,
        ))
        .into_any_element()
}

fn render_publisher_inspector(
    publisher: &Publisher,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let title = publisher
        .publisher_text
        .clone()
        .unwrap_or_else(|| "Unknown publisher".into());
    let rows = vec![
        (
            "Feeds".to_string(),
            publisher
                .feed_count
                .unwrap_or_else(|| publisher.feeds.as_ref().map_or(0, Vec::len) as i32)
                .to_string(),
        ),
        (
            "Tracks".to_string(),
            publisher
                .track_count
                .unwrap_or_else(|| publisher.tracks.as_ref().map_or(0, Vec::len) as i32)
                .to_string(),
        ),
    ];

    let feeds = publisher.feeds.clone().unwrap_or_default();

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(render_detail_header("publisher", &title, None, None))
        .child(render_detail_grid(rows))
        .when(!feeds.is_empty(), |el| {
            el.child(render_feed_list_section("Feeds", feeds, app, cx))
        })
        .into_any_element()
}

fn render_action_row(
    frame: &InspectorFrame,
    _pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    if !matches!(frame.entity_type.as_str(), "feed" | "track") {
        return div().into_any_element();
    }

    div()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(4.0))
        .child(
            metadata_action_button(&subscription_button_label(frame))
                .disabled(frame.subscription_busy)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_local_subscription(cx);
                })),
        )
        .when_some(frame.subscription_message.clone(), |el, message| {
            el.child(
                div()
                    .max_w(px(220.0))
                    .text_size(px(10.0))
                    .line_height(px(14.0))
                    .text_color(if message.contains("error") || message.contains("Error") {
                        rgb(0xff8a65)
                    } else {
                        muted()
                    })
                    .child(SharedString::from(message)),
            )
        })
        .into_any_element()
}

fn subscription_button_label(frame: &InspectorFrame) -> String {
    if frame.subscription_busy {
        return if frame.local_subscription.unwrap_or(false) {
            "Unsubscribing...".into()
        } else {
            "Subscribing...".into()
        };
    }

    let noun = if frame.entity_type == "feed" {
        "Feed"
    } else {
        "Track"
    };
    if frame.local_subscription.unwrap_or(false) {
        format!("Unsubscribe {noun}")
    } else {
        format!("Subscribe {noun}")
    }
}

fn render_lazy_sections(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    render_rss_lazy_sections(frame, cx)
}

fn render_rss_lazy_sections(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(render_lazy_contributors(frame, cx))
        .child(render_lazy_value_routes(frame, cx))
        .into_any_element()
}

fn render_lazy_contributors(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    let collapsed = frame.contributors_collapsed || matches!(frame.contributors, LazyPanel::Hidden);

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(render_contributors_heading(collapsed, cx))
        .when(!collapsed, |el| match &frame.contributors {
            LazyPanel::Loaded(items) => el.children(contributor_elements(items, cx)),
            LazyPanel::Loading => el.child(render_loading("Loading contributors...")),
            LazyPanel::Empty(label) => el.child(muted_line(label)),
            LazyPanel::Hidden => el,
        })
        .into_any_element()
}

fn render_lazy_value_routes(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    let collapsed = frame.value_routes_collapsed || matches!(frame.value_routes, LazyPanel::Hidden);

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(render_value_routes_heading(collapsed, cx))
        .when(!collapsed, |el| match &frame.value_routes {
            LazyPanel::Loaded(items) => el.children(value_route_elements(items)),
            LazyPanel::Loading => el.child(render_loading("Loading value routes...")),
            LazyPanel::Empty(label) => el.child(muted_line(label)),
            LazyPanel::Hidden => el,
        })
        .into_any_element()
}

fn contributor_elements(
    contributors: &[Contributor],
    cx: &mut Context<SearchApp>,
) -> Vec<AnyElement> {
    let mut groups = BTreeMap::<String, Vec<&Contributor>>::new();
    for contributor in contributors {
        groups
            .entry(contributor.group_name.clone().unwrap_or_default())
            .or_default()
            .push(contributor);
    }

    let mut all_elements: Vec<AnyElement> = Vec::new();
    for (group, members) in groups {
        if !group.is_empty() {
            all_elements.push(group_heading(group));
        }
        for contributor in members {
            let name = contributor.name.clone().unwrap_or_else(|| "Unknown".into());
            let role_str = contributor
                .role
                .as_ref()
                .map_or(String::new(), |r| format!(" ({r})"));

            if let Some(href) = contributor.href.clone() {
                let href_for_click = href.clone();
                let id = SharedString::from(format!("contrib-link:{}:{}", name, href));
                all_elements.push(
                    div()
                        .id(id)
                        .text_size(px(11.5))
                        .text_color(accent())
                        .cursor_pointer()
                        .on_click(cx.listener(move |_this, _: &ClickEvent, _window, _cx| {
                            let _ = open::that(&href_for_click);
                        }))
                        .child(SharedString::from(format!("{name}{role_str}")))
                        .into_any_element(),
                );
            } else {
                all_elements.push(
                    div()
                        .text_size(px(11.5))
                        .child(SharedString::from(format!("{name}{role_str}")))
                        .into_any_element(),
                );
            }
        }
    }

    all_elements
}

fn value_route_elements(routes: &[PaymentRoute]) -> Vec<AnyElement> {
    let mut groups = BTreeMap::<String, Vec<&PaymentRoute>>::new();
    for route in routes {
        let group = if route.fee.unwrap_or_default() {
            "Fees"
        } else {
            "Recipients"
        };
        groups.entry(group.into()).or_default().push(route);
    }

    groups
        .into_iter()
        .flat_map(|(group, routes)| {
            let mut elements = vec![group_heading(group)];
            elements.extend(routes.into_iter().map(|route| {
                let name = route
                    .recipient_name
                    .clone()
                    .unwrap_or_else(|| "Unnamed recipient".into());
                let route_type = route.route_type.clone().unwrap_or_else(|| "route".into());
                let split = route.split.unwrap_or_default();
                let fee_label = if route.fee.unwrap_or_default() {
                    "fee"
                } else {
                    "split"
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .text_size(px(11.5))
                    .child(SharedString::from(format!(
                        "{name} ({route_type} · {split}% · {fee_label})"
                    )))
                    .when(route.address.is_some(), |el| {
                        el.child(
                            div()
                                .text_color(muted())
                                .text_size(px(10.5))
                                .line_clamp(2)
                                .child(SharedString::from(
                                    route.address.clone().unwrap_or_default(),
                                )),
                        )
                    })
                    .when(
                        route.custom_key.is_some() || route.custom_value.is_some(),
                        |el| {
                            let mut parts = Vec::new();
                            if let Some(k) = &route.custom_key {
                                parts.push(format!("key {k}"));
                            }
                            if let Some(v) = &route.custom_value {
                                parts.push(format!("value {v}"));
                            }
                            el.child(
                                div()
                                    .text_color(muted())
                                    .text_size(px(10.5))
                                    .child(SharedString::from(parts.join(" · "))),
                            )
                        },
                    )
                    .into_any_element()
            }));
            elements
        })
        .collect()
}

fn render_contributors_heading(collapsed: bool, cx: &mut Context<SearchApp>) -> AnyElement {
    render_clickable_section_heading("Contributors", collapsed)
        .on_click(cx.listener(|this, _, _, cx| {
            this.toggle_contributors(cx);
        }))
        .into_any_element()
}

fn render_value_routes_heading(collapsed: bool, cx: &mut Context<SearchApp>) -> AnyElement {
    render_clickable_section_heading("Value Routes", collapsed)
        .on_click(cx.listener(|this, _, _, cx| {
            this.toggle_value_routes(cx);
        }))
        .into_any_element()
}

fn render_musicbrainz_panel(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    match &frame.musicbrainz_lookup {
        LazyPanel::Loaded(result) => render_musicbrainz_lookup(frame, result, cx),
        LazyPanel::Loading => render_loading("Searching MusicBrainz..."),
        LazyPanel::Empty(label) => render_loading(label),
        LazyPanel::Hidden => div().into_any_element(),
    }
}

fn render_musicbrainz_lookup(
    frame: &InspectorFrame,
    result: &MusicBrainzLookupResult,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let selected = selected_musicbrainz_candidate(frame, result);
    match selected {
        Some(candidate) => render_musicbrainz_header(frame, result, candidate, cx),
        None => div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(render_musicbrainz_title_bar(result, None, cx))
            .child(muted_line("No MusicBrainz recording match found"))
            .into_any_element(),
    }
}

fn render_musicbrainz_header(
    frame: &InspectorFrame,
    result: &MusicBrainzLookupResult,
    candidate: &MusicBrainzCandidate,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(16.0))
        .child(render_thumb(result.image.as_ref(), "track", 80.0, true))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(render_musicbrainz_title_bar(result, Some(candidate), cx))
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(23.0))
                        .child(SharedString::from(candidate.title.clone())),
                )
                .child(
                    div()
                        .text_color(muted())
                        .text_size(px(10.5))
                        .line_clamp(2)
                        .child(SharedString::from(musicbrainz_subtitle(
                            frame, result, candidate,
                        ))),
                ),
        )
        .into_any_element()
}

fn render_musicbrainz_title_bar(
    result: &MusicBrainzLookupResult,
    selected: Option<&MusicBrainzCandidate>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let label = selected.map_or_else(
        || "No MusicBrainz release".into(),
        musicbrainz_release_summary,
    );
    let trigger = Button::new("musicbrainz-release-picker")
        .label(SharedString::from(format!("MusicBrainz: {label}")))
        .with_size(Size::XSmall)
        .compact()
        .ghost()
        .w_full()
        .justify_start()
        .bg(type_color("track"))
        .text_color(rgb(0xffffff))
        .text_size(px(10.0))
        .font_weight(FontWeight::BOLD)
        .px(px(6.0))
        .py(px(2.0))
        .border_1()
        .border_color(type_color("track"))
        .rounded(px(4.0))
        .mb(px(6.0));

    if result.lookup.candidates.is_empty() {
        return trigger.disabled(true).into_any_element();
    }

    let candidates = result.lookup.candidates.clone();
    let selected_idx = selected
        .and_then(|selected| {
            candidates
                .iter()
                .position(|candidate| candidate.release_id == selected.release_id)
        })
        .unwrap_or_default();
    let app = cx.weak_entity();

    trigger
        .dropdown_menu(move |menu, _window, _cx| {
            candidates.iter().enumerate().fold(
                menu.min_w(px(320.0)).max_w(px(520.0)).scrollable(true),
                |menu, (idx, candidate)| {
                    let app = app.clone();
                    menu.item(
                        PopupMenuItem::new(musicbrainz_release_option_label(candidate))
                            .checked(idx == selected_idx)
                            .on_click(move |_, _, cx| {
                                let _ = app.update(cx, |this, cx| {
                                    this.select_musicbrainz_candidate(idx, cx);
                                });
                            }),
                    )
                },
            )
        })
        .into_any_element()
}

fn musicbrainz_release_summary(candidate: &MusicBrainzCandidate) -> String {
    let mut parts = Vec::new();
    if let Some(country) = &candidate.country {
        parts.push(country.clone());
    }
    if let Some(format) = &candidate.format {
        parts.push(format.clone());
    }
    if let Some(tracks) = candidate.total_tracks {
        parts.push(format!("{tracks} tracks"));
    }

    let mut value = if parts.is_empty() {
        candidate
            .release_title
            .clone()
            .unwrap_or_else(|| candidate.title.clone())
    } else {
        parts.join(" - ")
    };

    if let Some(date) = &candidate.release_date {
        value.push_str(&format!(" ({date})"));
    }
    value
}

fn musicbrainz_release_option_label(candidate: &MusicBrainzCandidate) -> SharedString {
    let release = candidate
        .release_title
        .clone()
        .unwrap_or_else(|| candidate.title.clone());
    SharedString::from(format!(
        "{} - {}",
        musicbrainz_release_summary(candidate),
        release
    ))
}

fn musicbrainz_subtitle(
    frame: &InspectorFrame,
    result: &MusicBrainzLookupResult,
    candidate: &MusicBrainzCandidate,
) -> String {
    let rank = if result
        .lookup
        .candidates
        .get(frame.musicbrainz_selected)
        .is_some()
    {
        frame.musicbrainz_selected + 1
    } else {
        1
    };
    let score = if let Some(musicbrainz_score) = candidate.musicbrainz_score {
        format!(
            "Best: #{} · {}% local · {} MB",
            rank, candidate.similarity_score, musicbrainz_score
        )
    } else {
        format!("Best: #{} · {}% local", rank, candidate.similarity_score)
    };
    if let Some(release_id) = &candidate.release_id {
        format!("{score} · {release_id}")
    } else {
        format!("{score} · {}", candidate.recording_id)
    }
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
    let show_musicbrainz = !matches!(frame.musicbrainz_lookup, LazyPanel::Hidden);
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

fn render_track_metadata_grid(
    rows: Vec<MetadataGridRow>,
    show_id3: bool,
    show_musicbrainz: bool,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    expanded_metadata_cells: &BTreeSet<String>,
    file_image: Option<Arc<Image>>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    render_metadata_grid(
        rows,
        show_id3,
        show_musicbrainz,
        pending_id3_edits,
        expanded_metadata_cells,
        file_image,
        cx,
    )
}

fn render_metadata_grid(
    rows: Vec<MetadataGridRow>,
    show_id3: bool,
    show_musicbrainz: bool,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    expanded_metadata_cells: &BTreeSet<String>,
    file_image: Option<Arc<Image>>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let mut cells: Vec<AnyElement> = Vec::new();
    let columns = 1 + u16::from(show_id3) + u16::from(show_musicbrainz);
    cells.push(metadata_heading_cell("RSS", 96.0));
    if show_id3 {
        cells.push(metadata_heading_cell("ID3", 12.0));
    }
    if show_musicbrainz {
        cells.push(metadata_heading_cell("MusicBrainz", 12.0));
    }

    for row in rows {
        match row {
            MetadataGridRow::Group(group) => {
                cells.push(metadata_group_cell(group, columns, cx));
            }
            MetadataGridRow::Data(row) => {
                let pending = pending_id3_edits.get(&row.row_id);
                let rss_expanded = expanded_metadata_cells.contains(&format!("rss:{}", row.row_id));
                let id3_expanded = expanded_metadata_cells.contains(&format!("id3:{}", row.row_id));
                cells.push(metadata_rss_cell(
                    &row,
                    pending,
                    rss_expanded,
                    expanded_metadata_cells,
                    cx,
                ));
                if show_id3 {
                    cells.push(metadata_id3_cell(
                        &row,
                        pending,
                        id3_expanded,
                        expanded_metadata_cells,
                        file_image.as_ref(),
                        cx,
                    ));
                }
                if show_musicbrainz {
                    cells.push(metadata_musicbrainz_cell(&row, pending));
                }
            }
        }
    }

    div()
        .grid()
        .grid_cols(columns)
        .gap_x(px(24.0))
        .gap_y(px(7.0))
        .children(cells)
        .into_any_element()
}

fn metadata_heading_cell(label: &str, indent: f32) -> AnyElement {
    div()
        .pl(px(indent))
        .text_color(muted())
        .font_weight(FontWeight::BOLD)
        .text_size(px(10.5))
        .child(SharedString::from(label.to_string()))
        .into_any_element()
}

fn metadata_rss_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let value = row.rss_value.as_deref().unwrap_or("");
    let display_value = display_metadata_value(&row.field, value);
    let value_color = source_cell_color(pending, MetadataColumn::Rss, row.rss_value.as_deref())
        .unwrap_or_else(text);
    let expandable = metadata_field_is_expandable(&row.field) && !value.is_empty();
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
        .gap(px(10.0))
        .child(
            div()
                .w(px(86.0))
                .flex_shrink_0()
                .text_color(text())
                .text_size(px(11.0))
                .line_height(px(16.0))
                .child(SharedString::from(row.field.clone())),
        )
        .child(div().flex_1().min_w_0().child(value_element));
    if !expandable {
        if let Some(drag) = metadata_drag_value(row, MetadataColumn::Rss) {
            return cell
                .id(SharedString::from(format!(
                    "metadata-rss-drag-{}",
                    row.row_id
                )))
                .cursor_move()
                .hover(|style| style.bg(surface()))
                .on_drag(
                    drag,
                    |drag: &MetadataDragValue, _position: Point<Pixels>, _window, cx: &mut App| {
                        cx.new(|_| MetadataDragPreview {
                            label: drag.field.clone(),
                            value: drag.value.clone(),
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
    file_image: Option<&Arc<Image>>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let frame = pending
        .map(|edit| edit.frame.as_str())
        .or(row.id3_frame.as_deref());
    let value = pending
        .map(|edit| edit.value.as_str())
        .or(row.id3_value.as_deref())
        .unwrap_or("");
    let display_value = display_metadata_value(&row.field, value);
    let color = pending
        .map(|edit| pending_source_color(edit.source))
        .unwrap_or_else(|| id3_cell_status_color(row));
    let frame_color = frame.map(id3_frame_base).map(id3_frame_version_color);
    let expandable = metadata_field_is_expandable(&row.field) && !value.is_empty();
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
        .pl(px(12.0))
        .min_w_0()
        .rounded(px(4.0))
        .child(value_element)
        .when_some(pending, |el, edit| {
            el.border_1()
                .border_color(pending_source_color(edit.source))
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
            .hover(|style| style.bg(surface()))
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
) -> AnyElement {
    let musicbrainz_color = source_cell_color(
        pending,
        MetadataColumn::MusicBrainz,
        row.musicbrainz_value.as_deref(),
    )
    .unwrap_or_else(|| comparison_status_color(&row.musicbrainz_status));
    let value = row.musicbrainz_value.as_deref().unwrap_or("");
    let display_value = display_metadata_value(&row.field, value);
    let cell = div().pl(px(12.0)).min_w_0().child(compare_tag_cell(
        &display_value,
        Some(musicbrainz_color),
        row.musicbrainz_key.as_deref(),
        None,
    ));
    if let Some(drag) = metadata_drag_value(row, MetadataColumn::MusicBrainz) {
        cell.id(SharedString::from(format!(
            "metadata-musicbrainz-drag-{}",
            row.row_id
        )))
        .cursor_move()
        .hover(|style| style.bg(surface()))
        .on_drag(
            drag,
            |drag: &MetadataDragValue, _position: Point<Pixels>, _window, cx: &mut App| {
                cx.new(|_| MetadataDragPreview {
                    label: drag.field.clone(),
                    value: drag.value.clone(),
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
        field: row.field.clone(),
        frame: row.id3_frame.clone().unwrap_or_default(),
        target_existing_value: None,
        value,
        source,
    })
}

fn pending_source_color(source: MetadataColumn) -> gpui::Rgba {
    match source {
        MetadataColumn::Rss => rgb(0x4caf82),
        MetadataColumn::MusicBrainz => rgb(0x4caf82),
    }
}

fn source_cell_color(
    pending: Option<&PendingId3Edit>,
    column: MetadataColumn,
    cell_value: Option<&str>,
) -> Option<gpui::Rgba> {
    let edit = pending?;
    if edit.source != column {
        return None;
    }
    let cell_value = cell_value.map(str::trim).filter(|v| !v.is_empty())?;
    if cell_value == edit.value.trim() {
        Some(rgb(0x4caf82))
    } else {
        Some(rgb(0xffc857))
    }
}

fn metadata_group_cell(
    group: MetadataGroupRow,
    columns: u16,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let label = if group.unused_count == 0 {
        group.label
    } else {
        format!("{} ({} unused)", group.label, group.unused_count)
    };

    let expanded = group.expanded;
    let mut cell = div().col_span(columns).mt(px(6.0));
    if let Some(group_key) = group.key {
        cell = cell.child(
            render_clickable_section_heading(&label, !expanded).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.toggle_id3_frame_group(group_key.clone(), cx);
                },
            )),
        );
    } else {
        cell = cell.child(
            div()
                .text_size(px(10.5))
                .font_weight(FontWeight::BOLD)
                .text_color(muted())
                .child(SharedString::from(label)),
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
fn compare_row_id(field: &str) -> String {
    let mut out = String::new();
    for ch in field.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
#[allow(dead_code)]
fn id3_unused_frame_row(frame_id: &str) -> MetadataGridRow {
    metadata_data_row(AlignedCompareRow {
        row_id: format!("id3-unused-{}", compare_row_id(frame_id)),
        field: format!("ID3 {frame_id}"),
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
        row_id: format!("id3-field-{}", compare_row_id(&field.frame_id)),
        field: format!("ID3 {}", field.frame_id),
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

fn render_file_header(result: &TagCompareResult, cx: &mut Context<SearchApp>) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(16.0))
        .child(render_thumb(
            result.file_image.as_ref(),
            "track",
            80.0,
            true,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .mb(px(6.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(badge_text("track"))
                                .bg(type_color("track"))
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .child("Embedded id3"),
                        )
                        .child(metadata_action_button("Re-read").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.reread_tag_compare(cx);
                            },
                        )))
                        .child(metadata_action_button("Re-download").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.redownload_tag_compare(cx);
                            },
                        ))),
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(23.0))
                        .child(SharedString::from(id3_header_title(result))),
                )
                .child(
                    div()
                        .text_color(muted())
                        .text_size(px(10.5))
                        .line_clamp(2)
                        .child(SharedString::from(result.path.clone())),
                ),
        )
        .into_any_element()
}

fn id3_header_title(result: &TagCompareResult) -> String {
    result
        .rows
        .iter()
        .find(|row| row.field == "Title")
        .and_then(|row| row.tag_value.clone())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "Embedded id3".into())
}

fn muted_line(value: &str) -> AnyElement {
    div()
        .text_color(muted())
        .text_size(px(10.5))
        .child(SharedString::from(value.to_string()))
        .into_any_element()
}

fn expandable_cell_summary(field: &str, raw_value: &str, display_value: &str) -> String {
    match field {
        "Contributors" => {
            summarize_contributor_value(raw_value).unwrap_or_else(|| display_value.to_string())
        }
        "Value Routes" => {
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) {
                format!("[{} items]", arr.len())
            } else {
                let lines = display_value.lines().count();
                if lines > 1 {
                    format!("[{lines} lines]")
                } else {
                    display_value.to_string()
                }
            }
        }
        "Artwork" => {
            if raw_value.starts_with("http://") || raw_value.starts_with("https://") {
                let filename = raw_value.rsplit('/').next().unwrap_or(raw_value);
                filename.to_string()
            } else {
                display_value.to_string()
            }
        }
        "Transcript" | "Transcript text" => display_value.to_string(),
        _ => display_value.to_string(),
    }
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
    frame_color: Option<gpui::Rgba>,
    file_image: Option<&'a Arc<Image>>,
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
    let cell_key = format!("rss:{row_id}");
    let glyph = if expanded { "v" } else { ">" };

    // Value Routes when expanded: header click collapses, sub-items have own clicks.
    // Use a non-clickable outer container so sub-item clicks don't bubble to toggle.
    if expanded && field == "Value Routes" {
        let cell_key_h = cell_key.clone();
        return div()
            .text_size(px(11.0))
            .line_height(px(16.0))
            .text_color(color)
            .flex()
            .flex_col()
            .child(
                div()
                    .id(SharedString::from(format!("expandable-rss-{}-hdr", field)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.toggle_metadata_cell(cell_key_h.clone(), cx);
                    }))
                    .flex()
                    .flex_row()
                    .gap(px(4.0))
                    .child(div().text_size(px(9.0)).text_color(muted()).child(glyph)),
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

    let mut container = div()
        .id(SharedString::from(format!("expandable-rss-{}", field)))
        .cursor_pointer()
        .text_size(px(11.0))
        .line_height(px(16.0))
        .text_color(color)
        .flex()
        .flex_col()
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }));

    if expanded {
        if matches!(field, "Artwork")
            && (raw_value.starts_with("http://") || raw_value.starts_with("https://"))
        {
            let url = raw_value.to_string();
            container = container
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child(div().text_size(px(9.0)).text_color(muted()).child(glyph))
                        .child(
                            div()
                                .text_color(accent())
                                .truncate()
                                .child(SharedString::from(raw_value.to_string())),
                        ),
                )
                .on_mouse_down(
                    MouseButton::Middle,
                    move |_: &MouseDownEvent, _window, _cx| {
                        let _ = open::that(&url);
                    },
                );
        } else if matches!(field, "Transcript" | "Transcript text") {
            container = container.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(4.0))
                    .items_start()
                    .child(div().text_size(px(9.0)).text_color(muted()).child(glyph))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .children(transcript_text_elements(raw_value, color)),
                    ),
            );
        } else {
            container = container.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(4.0))
                    .items_start()
                    .child(div().text_size(px(9.0)).text_color(muted()).child(glyph))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .children(json_tree_elements(raw_value, display_value, color)),
                    ),
            );
        }
    } else {
        let summary = expandable_cell_summary(field, raw_value, display_value);
        container = container.child(
            div()
                .flex()
                .flex_row()
                .gap(px(4.0))
                .child(div().text_size(px(9.0)).text_color(muted()).child(glyph))
                .child(
                    div()
                        .text_color(accent())
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
    let cell_key = format!("id3:{row_id}");
    let glyph = if expanded { "v" } else { ">" };
    let frame_color = frame_color.unwrap_or_else(muted);
    let frame_id_owned = frame_id.map(ToOwned::to_owned);

    let value_el = if expanded {
        if field == "Artwork" {
            // Show the actual embedded album art image
            if let Some(image) = file_image {
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .line_height(px(16.0))
                            .text_color(color)
                            .child(SharedString::from(display_value.to_string())),
                    )
                    .child(
                        div()
                            .w(px(200.0))
                            .h(px(200.0))
                            .rounded(px(6.0))
                            .overflow_hidden()
                            .child(
                                img(image.clone())
                                    .w(px(200.0))
                                    .h(px(200.0))
                                    .object_fit(ObjectFit::Cover),
                            ),
                    )
                    .into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(11.0))
                    .line_height(px(16.0))
                    .text_color(color)
                    .child(SharedString::from(display_value.to_string()))
                    .into_any_element()
            }
        } else if matches!(field, "Transcript" | "Transcript text") {
            // Show the actual lyrics/transcript text
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(11.0))
                .line_height(px(16.0))
                .text_color(color)
                .flex()
                .flex_col()
                .children(transcript_text_elements(raw_value, color))
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(11.0))
                .line_height(px(16.0))
                .text_color(color)
                .flex()
                .flex_col()
                .children(json_tree_elements(raw_value, display_value, color))
                .into_any_element()
        }
    } else {
        let summary = expandable_cell_summary(field, raw_value, display_value);
        div()
            .flex_1()
            .min_w_0()
            .text_size(px(11.0))
            .line_height(px(16.0))
            .flex()
            .flex_row()
            .gap(px(4.0))
            .child(div().text_size(px(9.0)).text_color(muted()).child(glyph))
            .child(
                div()
                    .text_color(accent())
                    .truncate()
                    .child(SharedString::from(summary)),
            )
            .into_any_element()
    };

    // Value Routes when expanded: separate header click from sub-item clicks
    if expanded && field == "Value Routes" {
        return div()
            .flex()
            .flex_col()
            .text_size(px(11.0))
            .line_height(px(16.0))
            .text_color(color)
            .child(
                div()
                    .id(SharedString::from(format!("expandable-id3-{}-hdr", field)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.toggle_metadata_cell(cell_key.clone(), cx);
                    }))
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(6.0))
                    .child(
                        div()
                            .w(px(136.0))
                            .flex_shrink_0()
                            .text_color(frame_color)
                            .text_size(px(9.5))
                            .line_height(px(16.0))
                            .child(SharedString::from(frame_id_owned.unwrap_or_default())),
                    )
                    .child(div().text_size(px(9.0)).text_color(muted()).child(glyph)),
            )
            .child(
                div()
                    .pl(px(142.0))
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

    div()
        .id(SharedString::from(format!("expandable-id3-{}", field)))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }))
        .flex()
        .flex_row()
        .items_start()
        .gap(px(6.0))
        .child(
            div()
                .w(px(136.0))
                .flex_shrink_0()
                .text_color(frame_color)
                .text_size(px(9.5))
                .line_height(px(16.0))
                .child(SharedString::from(frame_id_owned.unwrap_or_default())),
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
            let line = if line.is_empty() { " " } else { line };
            div()
                .truncate()
                .child(SharedString::from(line.to_string()))
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
                .text_size(px(11.0))
                .line_height(px(16.0));
            for (key, val) in map {
                let key_str = format!("{key}: ");
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        container = container
                            .child(div().text_color(muted()).child(SharedString::from(key_str)))
                            .child(json_object_element(val, color, depth + 1));
                    }
                    _ => {
                        let val_str = match val {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Null => "null".into(),
                            other => other.to_string(),
                        };
                        container = container.child(
                            div().flex().flex_row().gap(px(4.0)).child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .child(
                                        div()
                                            .text_color(muted())
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
            let text = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => "null".into(),
                other => other.to_string(),
            };
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
            let name = route
                .get("recipient_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let sub_key = format!("{column}:{row_id}:{i}");
            let sub_expanded = expanded_cells.contains(&sub_key);
            let sub_glyph = if sub_expanded { "v" } else { ">" };

            let mut item = div()
                .id(SharedString::from(format!("vr-{column}-{i}")))
                .cursor_pointer()
                .flex()
                .flex_col();

            let sub_key_click = sub_key.clone();
            item = item.on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.toggle_metadata_cell(sub_key_click.clone(), cx);
            }));

            item = item.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(9.0))
                            .text_color(muted())
                            .child(sub_glyph),
                    )
                    .child(
                        div()
                            .text_color(if sub_expanded { color } else { accent() })
                            .child(SharedString::from(name.to_string())),
                    ),
            );

            if sub_expanded {
                if let serde_json::Value::Object(map) = &route {
                    for (key, val) in map {
                        if key == "recipient_name" {
                            continue;
                        }
                        let val_str = match val {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Null => continue,
                            serde_json::Value::Bool(b) => b.to_string(),
                            other => other.to_string(),
                        };
                        if val_str.is_empty() {
                            continue;
                        }
                        item = item.child(
                            div()
                                .pl(px(16.0))
                                .flex()
                                .flex_row()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .text_color(muted())
                                        .child(SharedString::from(format!("{key}: "))),
                                )
                                .child(
                                    div()
                                        .text_color(color)
                                        .truncate()
                                        .child(SharedString::from(val_str)),
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
            let line = if line.is_empty() { " " } else { line };
            div()
                .text_color(color)
                .child(SharedString::from(line.to_string()))
                .into_any_element()
        })
        .collect()
}

fn compare_cell(value: &str, color: Option<gpui::Rgba>) -> AnyElement {
    let mut cell = div()
        .text_size(px(11.0))
        .line_height(px(16.0))
        .flex()
        .flex_col();
    if let Some(color) = color {
        cell = cell.text_color(color);
    }
    cell.children(compare_value_line_elements(value, 4))
        .into_any_element()
}

fn compare_tag_cell(
    value: &str,
    color: Option<gpui::Rgba>,
    frame_id: Option<&str>,
    frame_color: Option<gpui::Rgba>,
) -> AnyElement {
    let mut value_cell = div().text_size(px(11.0)).line_height(px(16.0));
    if let Some(color) = color {
        value_cell = value_cell.text_color(color);
    }
    let frame_id = frame_id.map(ToOwned::to_owned);
    let frame_color = frame_color.unwrap_or_else(muted);

    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(6.0))
        .child(
            div()
                .w(px(136.0))
                .flex_shrink_0()
                .text_color(frame_color)
                .text_size(px(9.5))
                .line_height(px(16.0))
                .child(SharedString::from(frame_id.unwrap_or_default())),
        )
        .child(
            value_cell
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .children(compare_value_line_elements(value, 4)),
        )
        .into_any_element()
}

fn compare_value_line_elements(value: &str, max_lines: usize) -> Vec<AnyElement> {
    let mut lines = value.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }
    let truncated = lines.len() > max_lines;
    lines
        .into_iter()
        .take(max_lines)
        .enumerate()
        .map(|(index, line)| {
            let line = if truncated && index + 1 == max_lines {
                "..."
            } else if line.is_empty() {
                " "
            } else {
                line
            };
            div()
                .truncate()
                .child(SharedString::from(line.to_string()))
                .into_any_element()
        })
        .collect::<Vec<_>>()
}

fn id3_frame_version_color(frame_id: &str) -> gpui::Rgba {
    match id3_frame_version(frame_id) {
        Id3FrameVersion::V22 => rgb(0xb06cf4),
        Id3FrameVersion::V23Only => rgb(0xffc857),
        Id3FrameVersion::V24Only => rgb(0x3ac4c4),
        Id3FrameVersion::V23V24 => accent(),
        Id3FrameVersion::Unknown => rgb(0xff8a65),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id3FrameVersion {
    V22,
    V23Only,
    V24Only,
    V23V24,
    Unknown,
}

fn id3_frame_version(frame_id: &str) -> Id3FrameVersion {
    let frame_id = id3_frame_base(frame_id);
    if frame_id.len() == 3 {
        return Id3FrameVersion::V22;
    }
    if matches!(
        frame_id,
        "ASPI"
            | "EQU2"
            | "RVA2"
            | "SEEK"
            | "SIGN"
            | "TDEN"
            | "TDOR"
            | "TDRC"
            | "TDRL"
            | "TDTG"
            | "TIPL"
            | "TMCL"
            | "TMOO"
            | "TPRO"
            | "TSOA"
            | "TSOP"
            | "TSOT"
            | "TSST"
    ) {
        return Id3FrameVersion::V24Only;
    }
    if matches!(
        frame_id,
        "CRM" | "EQUA" | "IPLS" | "RVAD" | "TDAT" | "TIME" | "TORY" | "TRDA" | "TSIZ" | "TYER"
    ) {
        return Id3FrameVersion::V23Only;
    }
    if ID3V24_FRAME_IDS.contains(&frame_id) {
        Id3FrameVersion::V23V24
    } else {
        Id3FrameVersion::Unknown
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

fn comparison_status_color(status: &ComparisonStatus) -> gpui::Rgba {
    match status {
        ComparisonStatus::Match => rgb(0x4caf82),
        ComparisonStatus::Different => rgb(0xffc857),
        ComparisonStatus::MissingSource | ComparisonStatus::MissingTag => rgb(0xff8a65),
        ComparisonStatus::MissingBoth => muted(),
    }
}

fn id3_cell_status_color(row: &AlignedCompareRow) -> gpui::Rgba {
    if row.id3_value.is_some() && row.rss_value.is_none() && row.musicbrainz_value.is_none() {
        return text();
    }
    comparison_status_color(&row.id3_status)
}

fn render_track_list_section(
    heading: &str,
    note: String,
    tracks: Vec<Track>,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(section_heading(heading))
                .when(!note.is_empty(), |el| {
                    el.child(div().text_color(muted()).text_size(px(10.5)).child(note))
                }),
        )
        .children(
            tracks
                .into_iter()
                .map(|track| {
                    let thumb = app.thumbnail_for_url(track.image_url.as_deref(), cx);
                    render_track_row(track, thumb, cx)
                })
                .collect::<Vec<_>>(),
        )
        .into_any_element()
}

fn render_track_row(
    track: Track,
    thumbnail: Option<Arc<Image>>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let guid = track.track_guid.clone().unwrap_or_default();
    let title = track_title(&track);
    div()
        .id(SharedString::from(format!("track-row:{guid}")))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(4.0))
        .py(px(5.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.push_inspector("track".into(), guid.clone(), title.clone(), cx);
        }))
        .child(
            div()
                .w(px(24.0))
                .text_right()
                .text_color(muted())
                .text_size(px(11.0))
                .child(
                    track
                        .track_number
                        .map_or_else(|| "·".into(), |n| n.to_string()),
                ),
        )
        .child(render_thumb(thumbnail.as_ref(), "track", 28.0, false))
        .child(truncated(track_title(&track)).flex_1())
        .when(track.duration_secs.is_some(), |el| {
            el.child(
                div()
                    .text_color(muted())
                    .text_size(px(11.0))
                    .child(SharedString::from(fmt_dur(
                        track.duration_secs.unwrap_or_default(),
                    ))),
            )
        })
        .into_any_element()
}

fn render_feed_list_section(
    heading: &str,
    feeds: Vec<Feed>,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let tiles: Vec<AnyElement> = feeds
        .into_iter()
        .map(|feed| {
            let guid = feed.feed_guid.clone().unwrap_or_default();
            let title = feed_title(&feed);
            let thumb = app.thumbnail_for_url(feed.image_url.as_deref(), cx);
            let episode_note = feed
                .episode_count
                .map(|n| format!("{n} tracks"))
                .unwrap_or_default();
            div()
                .id(SharedString::from(format!("feed-tile:{guid}")))
                .w(px(140.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .p(px(6.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.push_inspector("feed".into(), guid.clone(), title.clone(), cx);
                }))
                .child(render_thumb(thumb.as_ref(), "feed", 128.0, true))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .line_height(px(15.0))
                        .child(truncated(feed_title(&feed))),
                )
                .when(!episode_note.is_empty(), |el| {
                    el.child(
                        div()
                            .text_color(muted())
                            .text_size(px(10.5))
                            .child(SharedString::from(episode_note)),
                    )
                })
                .into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(section_heading(heading))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(px(12.0))
                .children(tiles),
        )
        .into_any_element()
}

fn render_detail_header(
    entity_type: &str,
    title: &str,
    subtitle: Option<&str>,
    image: Option<&Arc<Image>>,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(16.0))
        .child(render_thumb(image, entity_type, 80.0, true))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_text(entity_type))
                        .bg(type_color(entity_type))
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .mb(px(6.0))
                        .child(SharedString::from(entity_type.to_string())),
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(23.0))
                        .child(SharedString::from(title.to_string())),
                )
                .when_some(subtitle.map(str::to_owned), |el, sub| {
                    el.child(
                        div()
                            .mt(px(4.0))
                            .text_size(px(15.0))
                            .font_weight(FontWeight::MEDIUM)
                            .line_height(px(20.0))
                            .text_color(muted())
                            .child(SharedString::from(sub)),
                    )
                }),
        )
        .into_any_element()
}

fn render_track_header(
    frame: &InspectorFrame,
    track: &Track,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let title = track_title(track);
    let feed_guid = track.feed_guid.clone();
    let feed_url = track.feed_url.clone().or_else(|| track.feed_guid.clone());
    let audio_url = track.enclosure_url.clone();

    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(16.0))
        .child(render_thumb(frame.image.as_ref(), "track", 80.0, true))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_text("track"))
                        .bg(type_color("track"))
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .mb(px(6.0))
                        .child("track"),
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(23.0))
                        .child(SharedString::from(title)),
                )
                .child(render_track_header_subtitle(
                    feed_guid, feed_url, audio_url, cx,
                )),
        )
        .into_any_element()
}

fn render_track_header_subtitle(
    feed_guid: Option<String>,
    feed_url: Option<String>,
    audio_url: Option<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .min_w_0()
        .when_some(feed_guid, |el, guid| {
            el.child(render_feed_link_value(guid.clone(), guid, feed_url, cx))
        })
        .when_some(audio_url.filter(|url| !url.is_empty()), |el, url| {
            el.child(render_play_icon_button(url, cx))
        })
        .into_any_element()
}

fn render_detail_grid(rows: Vec<(String, String)>) -> AnyElement {
    render_detail_grid_elements(
        rows.into_iter()
            .map(|(key, value)| DetailRow {
                key,
                value: div()
                    .text_size(px(11.5))
                    .line_height(px(17.0))
                    .flex()
                    .flex_col()
                    .children(compare_value_line_elements(&value, 6))
                    .into_any_element(),
            })
            .collect(),
    )
}

fn render_collapsed_text_section(label: &str, value: String) -> AnyElement {
    div()
        .border_1()
        .border_color(border())
        .rounded(px(6.0))
        .p(px(10.0))
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(muted())
                .child(SharedString::from(label.to_string())),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(px(11.5))
                .line_height(px(17.0))
                .text_color(text())
                .flex()
                .flex_col()
                .children(compare_value_line_elements(&value, 3)),
        )
        .into_any_element()
}

fn render_feed_link_value(
    guid: String,
    title: String,
    url: Option<String>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let tooltip = url.unwrap_or_else(|| guid.clone());
    let click_title = title.clone();
    div()
        .id(SharedString::from(format!("track-feed-link:{guid}")))
        .cursor_pointer()
        .text_color(accent())
        .text_size(px(11.5))
        .line_height(px(17.0))
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.push_inspector("feed".into(), guid.clone(), click_title.clone(), cx);
        }))
        .child(SharedString::from(title))
        .into_any_element()
}

fn render_play_icon_button(url: String, cx: &mut Context<SearchApp>) -> AnyElement {
    Button::new("track-play-audio")
        .label("▶")
        .with_size(Size::XSmall)
        .compact()
        .ghost()
        .w(px(18.0))
        .h(px(18.0))
        .px(px(0.0))
        .py(px(0.0))
        .text_color(rgb(0xffffff))
        .rounded(px(4.0))
        .border_1()
        .border_color(accent())
        .tooltip(url.clone())
        .on_click(cx.listener(move |_this, _: &ClickEvent, _window, _cx| {
            let _ = open::that(&url);
        }))
        .into_any_element()
}

fn render_detail_grid_elements(rows: Vec<DetailRow>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(5.0))
        .children(rows.into_iter().map(|row| {
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(12.0))
                .child(
                    div()
                        .w(px(124.0))
                        .flex_shrink_0()
                        .text_color(muted())
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .child(SharedString::from(row.key)),
                )
                .child(div().flex_1().min_w_0().child(row.value))
                .into_any_element()
        }))
        .into_any_element()
}

fn render_thumb(
    image_data: Option<&Arc<Image>>,
    entity_type: &str,
    size: f32,
    large: bool,
) -> AnyElement {
    let radius = if large { 6.0 } else { 4.0 };
    if let Some(image) = image_data {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(px(radius))
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                img(image.clone())
                    .w(px(size))
                    .h(px(size))
                    .object_fit(ObjectFit::Cover),
            )
            .into_any_element()
    } else {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(px(radius))
            .bg(border())
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(if large { 28.0 } else { 14.0 }))
            .flex_shrink_0()
            .child(type_emoji(entity_type))
            .into_any_element()
    }
}

fn render_recent_feeds_tiles(app: &mut SearchApp, cx: &mut Context<SearchApp>) -> AnyElement {
    let feeds = app.recent_feeds.clone();
    let status = app.recent_status.clone();
    let has_more = app.recent_has_more;
    let loading = app.recent_loading;
    let is_empty = feeds.is_empty();

    let mut tiles: Vec<AnyElement> = Vec::with_capacity(feeds.len());
    for feed in feeds {
        let guid = match feed.feed_guid.clone() {
            Some(guid) if !guid.trim().is_empty() => guid,
            _ => continue,
        };
        let title = feed_title(&feed);
        let artist = feed
            .release_artist
            .clone()
            .or_else(|| feed.publisher_text.clone())
            .unwrap_or_default();
        let image_url = feed.image_url.clone();
        let thumbnail = app.thumbnail_for_url(image_url.as_deref(), cx);
        let click_guid = guid.clone();
        let click_title = title.clone();
        let tile = div()
            .id(SharedString::from(format!("recent-tile:{guid}")))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .w(px(168.0))
            .p(px(8.0))
            .rounded(px(8.0))
            .cursor_pointer()
            .hover(|el| el.bg(surface()))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_recent_feed(click_guid.clone(), click_title.clone(), cx);
            }))
            .child(
                div()
                    .w(px(152.0))
                    .h(px(152.0))
                    .rounded(px(6.0))
                    .overflow_hidden()
                    .flex_shrink_0()
                    .when_some(thumbnail, |el, image| {
                        el.child(
                            img(image)
                                .w(px(152.0))
                                .h(px(152.0))
                                .object_fit(ObjectFit::Cover),
                        )
                    })
                    .when(image_url.is_none(), |el| {
                        el.bg(border())
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(28.0))
                            .child(type_emoji("feed"))
                    }),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .child(truncated(title)),
            )
            .when(!artist.is_empty(), |el| {
                el.child(
                    div()
                        .text_size(px(10.5))
                        .text_color(muted())
                        .child(truncated(artist)),
                )
            })
            .into_any_element();
        tiles.push(tile);
    }

    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Recent Feeds"),
        )
        .when(!status.is_empty(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(muted())
                    .child(SharedString::from(status)),
            )
        })
        .when(is_empty && !loading, |el| {
            el.child(
                div()
                    .text_center()
                    .p(px(48.0))
                    .text_color(muted())
                    .child("No recent feeds"),
            )
        })
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(px(12.0))
                .children(tiles),
        )
        .when(has_more && !loading, |el| {
            el.child(
                div().pt(px(8.0)).child(
                    Button::new("recent-load-more")
                        .label("Load more")
                        .ghost()
                        .with_size(Size::Small)
                        .text_color(rgb(0xffffff))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.load_recent_feeds(true, cx);
                        })),
                ),
            )
        })
        .into_any_element()
}

fn render_inspector_empty() -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_color(muted())
        .gap(px(8.0))
        .child(div().text_3xl().opacity(0.4).child("🔍"))
        .child("Select a result to inspect")
        .into_any_element()
}

fn render_loading(message: &str) -> AnyElement {
    div()
        .text_color(muted())
        .italic()
        .py(px(8.0))
        .child(SharedString::from(message.to_string()))
        .into_any_element()
}

fn section_heading(label: &str) -> AnyElement {
    div()
        .text_size(px(10.5))
        .font_weight(FontWeight::BOLD)
        .text_color(muted())
        .child(SharedString::from(label.to_string()))
        .into_any_element()
}

fn render_clickable_section_heading(label: &str, collapsed: bool) -> gpui::Stateful<gpui::Div> {
    let state = if collapsed { "show" } else { "hide" };
    let glyph = if collapsed { ">" } else { "v" };

    div()
        .id(SharedString::from(format!("section-heading:{label}")))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .cursor_pointer()
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::BOLD)
                .text_color(muted())
                .child(glyph),
        )
        .child(
            div()
                .text_size(px(10.5))
                .font_weight(FontWeight::BOLD)
                .text_color(muted())
                .child(SharedString::from(label.to_string())),
        )
        .child(
            div()
                .text_size(px(9.5))
                .text_color(muted())
                .child(SharedString::from(state.to_string())),
        )
}

fn group_heading(label: String) -> AnyElement {
    div()
        .text_size(px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(muted())
        .mt(px(6.0))
        .child(SharedString::from(label))
        .into_any_element()
}

fn metadata_action_button(label: &str) -> Button {
    Button::new(SharedString::from(format!("metadata-action:{label}")))
        .label(SharedString::from(label.to_string()))
        .with_size(Size::XSmall)
        .compact()
        .ghost()
        .text_color(rgb(0xffffff))
        .text_size(px(10.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(accent())
}

fn truncated(text: String) -> gpui::Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(px(11.5))
        .child(SharedString::from(text))
}

fn truncated_muted(text: String) -> gpui::Div {
    truncated(text).text_color(muted())
}

fn optional_row(rows: &mut Vec<(String, String)>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        if !value.is_empty() {
            rows.push((key.into(), value));
        }
    }
}

fn result_lines(row: &ResultRow) -> (String, String, String, Option<String>) {
    match &row.detail {
        Some(EntityDetail::Feed(feed)) => {
            let count = feed
                .episode_count
                .map_or_else(String::new, |count| format!("{count} tracks"));
            (
                feed_title(feed),
                feed.release_artist
                    .clone()
                    .unwrap_or_else(|| "Unknown".into()),
                count,
                feed.image_url.clone(),
            )
        }
        Some(EntityDetail::Track(track)) => {
            let duration = track.duration_secs.map(fmt_dur);
            let line1 = [Some(track_title(track)), duration]
                .into_iter()
                .flatten()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" – ");
            let feed_title = track.feed_title.clone().unwrap_or_default();
            let release_artist = track.release_artist.clone().unwrap_or_default();
            let line3 = if release_artist.is_empty() {
                feed_title
            } else {
                format!("{feed_title} by {release_artist}")
            };

            (
                line1,
                track
                    .track_artist
                    .clone()
                    .unwrap_or_else(|| "Unknown".into()),
                line3,
                track.image_url.clone(),
            )
        }
        Some(EntityDetail::Publisher(publisher)) => {
            let mut parts = Vec::new();
            if let Some(count) = publisher.feed_count {
                parts.push(format!("{count} feeds"));
            }
            if let Some(count) = publisher.track_count {
                parts.push(format!("{count} tracks"));
            }
            (
                publisher.publisher_text.clone().unwrap_or_default(),
                parts.join(" · "),
                String::new(),
                None,
            )
        }
        _ => (row.entity_id.clone(), String::new(), String::new(), None),
    }
}

fn result_image_url(row: &ResultRow) -> Option<String> {
    match &row.detail {
        Some(EntityDetail::Feed(feed)) => feed.image_url.clone(),
        Some(EntityDetail::Track(track)) => track.image_url.clone(),
        Some(EntityDetail::Artist(artist)) => artist.image_url.clone(),
        Some(EntityDetail::Release(release)) => release.image_url.clone(),
        Some(EntityDetail::Recording(recording)) => recording.image_url.clone(),
        Some(EntityDetail::Publisher(_)) | None => None,
    }
}

fn entity_key(entity_type: &str, entity_id: &str) -> String {
    format!("{entity_type}:{entity_id}")
}

fn feed_title(feed: &Feed) -> String {
    feed.title
        .clone()
        .or_else(|| feed.name.clone())
        .or_else(|| feed.feed_guid.clone())
        .unwrap_or_else(|| "Untitled".into())
}

fn track_title(track: &Track) -> String {
    track
        .title
        .clone()
        .or_else(|| track.name.clone())
        .or_else(|| track.track_guid.clone())
        .unwrap_or_else(|| "Untitled".into())
}

fn fmt_dur(secs: i32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
#[allow(dead_code)]
fn fmt_ms(ms: i64) -> String {
    fmt_dur((ms / 1000).try_into().unwrap_or(i32::MAX))
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

fn fmt_runtime(total_secs: i32) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    if hours > 0 {
        format!("{hours} h {minutes} min")
    } else {
        format!("{minutes} min")
    }
}

fn fmt_date(ts: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.format("%b %-d, %Y").to_string())
}

fn bg() -> gpui::Rgba {
    rgb(0x0f1117)
}

fn surface() -> gpui::Rgba {
    rgb(0x1a1d27)
}

fn border() -> gpui::Rgba {
    rgb(0x2a2d3a)
}

fn text() -> gpui::Rgba {
    rgb(0xe2e4ed)
}

fn muted() -> gpui::Rgba {
    rgb(0x9298ab)
}

fn accent() -> gpui::Rgba {
    rgb(0x8b9bff)
}

fn type_color(entity_type: &str) -> gpui::Rgba {
    match entity_type {
        "feed" => rgb(0xe8943a),
        "track" => rgb(0x3ac4c4),
        "publisher" => rgb(0xe84393),
        "artist" => rgb(0x4caf82),
        "release" => rgb(0x6c7cff),
        "recording" => rgb(0xb06cf4),
        _ => accent(),
    }
}

fn badge_text(entity_type: &str) -> gpui::Rgba {
    match entity_type {
        // Dark text on bright badges for WCAG AA contrast
        "feed" | "track" | "artist" => rgb(0x111318),
        // White text on darker badges
        _ => rgb(0xffffff),
    }
}

fn type_emoji(entity_type: &str) -> &'static str {
    match entity_type {
        "feed" => "📡",
        "track" => "🎶",
        "publisher" => "🏢",
        "artist" => "🎤",
        "release" => "💿",
        "recording" => "🎵",
        _ => "?",
    }
}

pub fn run_search_app() {
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        let cfg_path = config::config_path().expect("config path");
        let cfg = config::load_config(&cfg_path).expect("load config");
        config::ensure_dirs(&cfg).expect("ensure dirs");
        let conn = db::open_db(&cfg).expect("open db");
        let conn = Arc::new(Mutex::new(conn));
        let musicindex_endpoint =
            config::load_musicindex_endpoint(&cfg_path).expect("load MusicIndex endpoint");

        let thumbnail_cache_dir = cfg_path
            .parent()
            .expect("config path has parent")
            .join("thumbnail-cache");
        let http = reqwest::blocking::Client::new();
        let image_cache = ImageCache::new(http, thumbnail_cache_dir);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1120.0), px(760.0)),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| {
                let view =
                    cx.new(|cx| SearchApp::new(conn, image_cache, musicindex_endpoint, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("failed to open window");
    });
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        aligned_compare_rows, aligned_id3_frame_ids, auto_populated_pending_id3_edits,
        compare_row_id, expand_woar_metadata_rows, format_drag_value_for_id3v24,
        id3_frame_group_key, id3_frame_version, metadata_data_row, metadata_drag_value,
        metadata_field_group_key, musicbrainz_remainder_rows, pending_id3_conflict_descriptions,
        pending_id3_edits_for_apply, pending_id3_target_key, should_show_inspector_back,
        track_metadata_rows, unused_id3v24_frames_for_group, AlignedCompareRow, Feed,
        Id3FrameVersion, MetadataColumn, MetadataGridRow, PendingId3Edit, SourceEntityId,
        SourceEntityLink, TagCompareResult, Track, TrackContext, ID3V24_FRAME_GROUPS,
        ID3V24_FRAME_IDS,
    };
    use crate::audio_tags::{id3v24_edit_label_is_writable, Id3Field};
    use crate::metadata::{
        compare_id3_field_values, contributor_id3_rows, display_metadata_value,
        musicindex_contributors_id3_value,
    };
    use crate::musicbrainz::MusicBrainzCandidate;
    use crate::track_compare::{ComparisonRow, ComparisonStatus};

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
            row_id: compare_row_id("RSS feed guid"),
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

        let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new());
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

        let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &BTreeSet::new());
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
            auto_populated_pending_id3_edits(&expanded, &BTreeMap::new(), &BTreeSet::new());
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
            auto_populated_pending_id3_edits(&expanded, &BTreeMap::new(), &BTreeSet::new());
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

        let edits = super::id3_edits_for_track_context(&context);
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

        let edits = super::id3_edits_for_track_context(&context);
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

        let edits = super::id3_edits_for_track_context(&context);
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

        let pending = auto_populated_pending_id3_edits(&rows, &BTreeMap::new(), &suppressed);
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
        assert!(
            fields.iter().any(|field| *field == "Track #"),
            "track row should exist"
        );
        assert!(
            !fields.iter().any(|field| *field == "Total tracks"),
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
            row_id: compare_row_id(field),
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
