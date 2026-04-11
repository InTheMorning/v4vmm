use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use gpui::{
    div, img, prelude::*, px, rgb, size, AnyElement, Application, Bounds, ClickEvent, Context,
    Entity, FontWeight, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{Disableable, Root, Sizable, Size};
use reqwest::blocking::Client as ReqwestClient;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://musicindex.org";
const PAGE_LIMIT: i32 = 20;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SearchResponse {
    pub data: Vec<SearchResult>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub quality_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PublisherSearchResponse {
    pub data: Vec<Publisher>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Pagination {
    pub has_more: bool,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Artist {
    pub artist_id: Option<String>,
    pub name: Option<String>,
    pub sort_name: Option<String>,
    pub area: Option<String>,
    pub begin_year: Option<i32>,
    pub end_year: Option<i32>,
    pub url: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub image_url: Option<String>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Release {
    pub release_id: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub release_date: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub artist_credit: Option<ArtistCredit>,
    pub tracks: Option<Vec<Track>>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Recording {
    pub recording_id: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub duration_secs: Option<i32>,
    pub image_url: Option<String>,
    pub artist_credit: Option<ArtistCredit>,
    pub releases: Option<Vec<ReleaseReference>>,
    pub sources: Option<Vec<Source>>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Feed {
    pub feed_guid: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
    pub feed_url: Option<String>,
    pub release_artist: Option<String>,
    pub release_artist_sort: Option<String>,
    pub raw_medium: Option<String>,
    pub release_kind: Option<String>,
    pub release_date: Option<i64>,
    pub publisher_text: Option<String>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub episode_count: Option<i32>,
    pub newest_item_at: Option<i64>,
    pub oldest_item_at: Option<i64>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub tracks: Option<Vec<Track>>,
    pub source_contributors: Option<Vec<Contributor>>,
    pub payment_routes: Option<Vec<PaymentRoute>>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Track {
    pub track_guid: Option<String>,
    pub feed_guid: Option<String>,
    pub feed_title: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
    pub duration_secs: Option<i32>,
    pub pub_date: Option<i64>,
    pub track_number: Option<i32>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
    pub enclosure_url: Option<String>,
    pub image_url: Option<String>,
    pub track_artist: Option<String>,
    pub release_artist: Option<String>,
    pub publisher_text: Option<String>,
    pub artist_credit: Option<ArtistCredit>,
    pub source_contributors: Option<Vec<Contributor>>,
    pub payment_routes: Option<Vec<PaymentRoute>>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Publisher {
    pub publisher_text: Option<String>,
    pub feed_count: Option<i32>,
    pub track_count: Option<i32>,
    pub feeds: Option<Vec<Feed>>,
    pub tracks: Option<Vec<Track>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Contributor {
    pub name: Option<String>,
    pub role: Option<String>,
    pub href: Option<String>,
    pub group_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PaymentRoute {
    pub recipient_name: Option<String>,
    pub route_type: Option<String>,
    pub split: Option<f64>,
    pub fee: Option<bool>,
    pub address: Option<String>,
    pub custom_key: Option<String>,
    pub custom_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ArtistCredit {
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ReleaseReference {
    pub position: Option<i32>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Source {
    pub title: Option<String>,
    pub primary_enclosure_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityDetail {
    Artist(Artist),
    Release(Release),
    Recording(Recording),
    Feed(Feed),
    Track(Track),
    Publisher(Publisher),
}

#[derive(Clone)]
pub struct Client {
    client: ReqwestClient,
    base_url: String,
}

impl Client {
    pub fn new() -> Self {
        Self {
            client: ReqwestClient::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn new_with_base_url(base_url: String) -> Self {
        Self {
            client: ReqwestClient::new(),
            base_url,
        }
    }

    pub fn search(
        &self,
        query: &str,
        entity_type: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<SearchResponse> {
        let mut params = vec![
            ("q", query.to_string()),
            ("limit", limit.unwrap_or(PAGE_LIMIT).to_string()),
        ];

        if let Some(entity_type) = entity_type {
            params.push(("type", entity_type.to_string()));
        }

        if let Some(cursor) = cursor {
            params.push(("cursor", cursor.to_string()));
        }

        self.get_json(&["v1", "search"], &params)
    }

    pub fn search_publishers(
        &self,
        query: &str,
        limit: Option<i32>,
    ) -> Result<PublisherSearchResponse> {
        let params = vec![
            ("q", query.to_string()),
            ("limit", limit.unwrap_or(PAGE_LIMIT).to_string()),
        ];
        self.get_json(&["v1", "publishers"], &params)
    }

    pub fn fetch_detail(&self, entity_type: &str, entity_id: &str) -> Result<EntityDetail> {
        match entity_type {
            "artist" => Ok(EntityDetail::Artist(
                self.fetch_wrapped(&["v1", "artists", entity_id])?,
            )),
            "release" => {
                let params = [("include", "tracks".to_string())];
                Ok(EntityDetail::Release(self.fetch_wrapped_with_query(
                    &["v1", "releases", entity_id],
                    &params,
                )?))
            }
            "recording" => {
                let params = [("include", "sources,releases".to_string())];
                Ok(EntityDetail::Recording(self.fetch_wrapped_with_query(
                    &["v1", "recordings", entity_id],
                    &params,
                )?))
            }
            "feed" => Ok(EntityDetail::Feed(self.fetch_feed(entity_id, None)?)),
            "track" => Ok(EntityDetail::Track(self.fetch_track(entity_id, None)?)),
            "publisher" => Ok(EntityDetail::Publisher(self.fetch_publisher(entity_id)?)),
            _ => Err(anyhow!("unknown entity type: {entity_type}")),
        }
    }

    pub fn fetch_feed(&self, feed_guid: &str, include: Option<&str>) -> Result<Feed> {
        let mut params = Vec::new();
        if let Some(include) = include {
            params.push(("include", include.to_string()));
        }
        self.fetch_wrapped_with_query(&["v1", "feeds", feed_guid], &params)
    }

    pub fn fetch_track(&self, track_guid: &str, include: Option<&str>) -> Result<Track> {
        let mut params = Vec::new();
        if let Some(include) = include {
            params.push(("include", include.to_string()));
        }
        self.fetch_wrapped_with_query(&["v1", "tracks", track_guid], &params)
    }

    pub fn fetch_publisher(&self, publisher_text: &str) -> Result<Publisher> {
        self.fetch_wrapped(&["v1", "publishers", publisher_text])
    }

    pub fn fetch_contributors(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<Contributor>> {
        let detail = match entity_type {
            "feed" => EntityDetail::Feed(self.fetch_feed(entity_id, Some("source_contributors"))?),
            "track" => {
                EntityDetail::Track(self.fetch_track(entity_id, Some("source_contributors"))?)
            }
            _ => return Ok(Vec::new()),
        };

        Ok(match detail {
            EntityDetail::Feed(feed) => feed.source_contributors.unwrap_or_default(),
            EntityDetail::Track(track) => track.source_contributors.unwrap_or_default(),
            _ => Vec::new(),
        })
    }

    pub fn fetch_value_routes(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<PaymentRoute>> {
        let detail = match entity_type {
            "feed" => EntityDetail::Feed(self.fetch_feed(entity_id, Some("payment_routes"))?),
            "track" => EntityDetail::Track(self.fetch_track(entity_id, Some("payment_routes"))?),
            _ => return Ok(Vec::new()),
        };

        Ok(match detail {
            EntityDetail::Feed(feed) => feed.payment_routes.unwrap_or_default(),
            EntityDetail::Track(track) => track.payment_routes.unwrap_or_default(),
            _ => Vec::new(),
        })
    }

    fn fetch_wrapped<T>(&self, path_segments: &[&str]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.fetch_wrapped_with_query(path_segments, &[])
    }

    fn fetch_wrapped_with_query<T>(
        &self,
        path_segments: &[&str],
        query: &[(&str, String)],
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response: DetailResponse<T> = self.get_json(path_segments, query)?;
        Ok(response.data)
    }

    fn get_json<T>(&self, path_segments: &[&str], query: &[(&str, String)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut url = reqwest::Url::parse(&format!("{}/", self.base_url.trim_end_matches('/')))?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow!("base URL cannot be a base: {}", self.base_url))?;
            for segment in path_segments {
                segments.push(segment);
            }
        }

        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }

        let response = self.client.get(url).send()?.error_for_status()?;
        Ok(response.json()?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

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
    Feed(Feed),
    Track(Track),
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
    contributors: LazyPanel<Vec<Contributor>>,
    value_routes: LazyPanel<Vec<PaymentRoute>>,
}

impl InspectorFrame {
    fn loading(entity_type: String, entity_id: String, title: String) -> Self {
        Self {
            entity_type,
            entity_id,
            title: title.clone(),
            detail: InspectorDetail::Loading(format!("Loading {title}...")),
            contributors: LazyPanel::Hidden,
            value_routes: LazyPanel::Hidden,
        }
    }
}

struct SearchBatch {
    rows: Vec<ResultRow>,
    has_more: bool,
    cursor: Option<String>,
}

pub struct SearchApp {
    input: Entity<InputState>,
    type_filter: usize,
    results: Vec<ResultRow>,
    loading: bool,
    status: String,
    cursor: Option<String>,
    has_more: bool,
    selected_key: Option<String>,
    inspector_stack: Vec<InspectorFrame>,
    left_pane_width: gpui::Pixels,
    resizing: bool,
    _input_sub: gpui::Subscription,
}

const TYPE_LABELS: &[&str] = &["All", "Feed", "Track", "Publisher"];
const TYPE_VALUES: &[Option<&str>] = &[None, Some("feed"), Some("track"), Some("publisher")];

impl SearchApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder("Search feeds, tracks, publishers...")
        });
        let input_sub = cx.subscribe(&input, Self::on_input_event);

        Self {
            input,
            type_filter: 0,
            results: Vec::new(),
            loading: false,
            status: String::new(),
            cursor: None,
            has_more: false,
            selected_key: None,
            inspector_stack: Vec::new(),
            left_pane_width: px(360.0),
            resizing: false,
            _input_sub: input_sub,
        }
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
            "Searching...".into()
        };

        if !append {
            self.results.clear();
            self.cursor = None;
            self.has_more = false;
            self.selected_key = None;
            self.inspector_stack.clear();
        }
        cx.notify();

        let entity_type = TYPE_VALUES[self.type_filter].map(str::to_string);
        let cursor = if append { self.cursor.clone() } else { None };
        let client = Arc::new(Client::new());

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

    fn select_result(
        &mut self,
        entity_type: String,
        entity_id: String,
        title: String,
        cx: &mut Context<Self>,
    ) {
        self.selected_key = Some(entity_key(&entity_type, &entity_id));
        self.load_inspector(entity_type, entity_id, title, false, cx);
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

        let client = Arc::new(Client::new());
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
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == entity_type && frame.entity_id == entity_id {
                                frame.detail = match detail {
                                    Ok(detail) => detail,
                                    Err(error) => InspectorDetail::Error(error.to_string()),
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
        if self.inspector_stack.len() > 1 {
            self.inspector_stack.pop();
            cx.notify();
        }
    }

    fn toggle_contributors(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };

        match frame.contributors {
            LazyPanel::Loaded(_) => {
                frame.contributors = LazyPanel::Hidden;
                cx.notify();
                return;
            }
            LazyPanel::Loading | LazyPanel::Empty(_) => return,
            LazyPanel::Hidden => frame.contributors = LazyPanel::Loading,
        }

        let entity_type = frame.entity_type.clone();
        let entity_id = frame.entity_id.clone();
        let client = Arc::new(Client::new());
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

        match frame.value_routes {
            LazyPanel::Loaded(_) => {
                frame.value_routes = LazyPanel::Hidden;
                cx.notify();
                return;
            }
            LazyPanel::Loading | LazyPanel::Empty(_) => return,
            LazyPanel::Hidden => frame.value_routes = LazyPanel::Loading,
        }

        let entity_type = frame.entity_type.clone();
        let entity_id = frame.entity_id.clone();
        let client = Arc::new(Client::new());
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
}

impl Render for SearchApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_text = self.status.clone();
        let status_color = if status_text.starts_with("Error:") {
            rgb(0xff6b6b)
        } else {
            muted()
        };

        let results: Vec<AnyElement> = self
            .results
            .iter()
            .map(|row| render_result_item(row, self.selected_key.as_deref(), cx))
            .collect();
        let type_filters: Vec<AnyElement> = TYPE_LABELS
            .iter()
            .enumerate()
            .map(|(idx, label)| render_filter_button(idx, label, idx == self.type_filter, cx))
            .collect();
        let inspector =
            render_inspector(self.inspector_stack.last(), self.inspector_stack.len(), cx);

        div()
            .size_full()
            .bg(bg())
            .text_color(text())
            .text_sm()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .bg(surface())
                    .border_b_1()
                    .border_color(border())
                    .p(px(12.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.0))
                    .flex_wrap()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(accent())
                            .child("stophammer"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .flex_wrap()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div().flex_1().min_w(px(224.0)).child(
                                    Input::new(&self.input)
                                        .cleanable(true)
                                        .with_size(Size::Small),
                                ),
                            )
                            .children(type_filters)
                            .child(
                                Button::new("search-btn")
                                    .label("Search")
                                    .primary()
                                    .with_size(Size::Small)
                                    .loading(self.loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.do_search(false, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .id("pane-container")
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .on_mouse_move(cx.listener(
                        |this, event: &MouseMoveEvent, _window, cx| {
                            if this.resizing {
                                let x = event.position.x;
                                let clamped = x.max(px(200.0)).min(px(800.0));
                                this.left_pane_width = clamped;
                                cx.notify();
                            }
                        },
                    ))
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
                                    .text_xs()
                                    .text_color(status_color)
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .border_b_1()
                                    .border_color(border())
                                    .child(SharedString::from(status_text)),
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
                                            .when(
                                                self.results.is_empty()
                                                    && !self.loading
                                                    && self.status.is_empty(),
                                                |el| {
                                                    el.child(
                                                        div()
                                                            .text_center()
                                                            .p(px(48.0))
                                                            .text_color(muted())
                                                            .child(div().text_2xl().child("🔍"))
                                                            .child(
                                                                div()
                                                                    .mt(px(8.0))
                                                                    .child("No results"),
                                                            ),
                                                    )
                                                },
                                            )
                                            .when(
                                                self.results.is_empty()
                                                    && !self.loading
                                                    && !self.status.is_empty(),
                                                |el| {
                                                    el.child(
                                                        div()
                                                            .text_center()
                                                            .p(px(48.0))
                                                            .text_color(muted())
                                                            .child(div().text_2xl().child("🔍"))
                                                            .child(
                                                                div()
                                                                    .mt(px(8.0))
                                                                    .child("No results"),
                                                            ),
                                                    )
                                                },
                                            )
                                            .when(self.has_more && !self.loading, |el| {
                                                el.child(
                                                    Button::new("load-more")
                                                        .label("Load more")
                                                        .ghost()
                                                        .with_size(Size::Small)
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
                                cx.listener(
                                    |this, _: &MouseDownEvent, _window, cx| {
                                        this.resizing = true;
                                        cx.notify();
                                    },
                                ),
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
) -> Result<SearchBatch> {
    if entity_type == Some("publisher") {
        let response = client.search_publishers(query, Some(PAGE_LIMIT))?;
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

    let response = client.search(query, entity_type, Some(PAGE_LIMIT), cursor)?;
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

fn fetch_inspector_detail(
    client: &Client,
    entity_type: &str,
    entity_id: &str,
) -> Result<InspectorDetail> {
    match entity_type {
        "feed" => Ok(InspectorDetail::Feed(
            client.fetch_feed(entity_id, Some("tracks"))?,
        )),
        "track" => Ok(InspectorDetail::Track(client.fetch_track(entity_id, None)?)),
        "publisher" => Ok(InspectorDetail::Publisher(
            client.fetch_publisher(entity_id)?,
        )),
        _ => Err(anyhow!("unknown inspector entity type: {entity_type}")),
    }
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
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let (line1, line2, line3, image_url) = result_lines(row);
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
        .child(render_thumb(&image_url, &row.entity_type, 36.0, false))
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
                .text_color(rgb(0xffffff))
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
    stack_len: usize,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let title = frame.map_or("", |frame| frame.title.as_str());
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
                .when(stack_len > 1, |el| {
                    el.child(
                        Button::new("inspector-back")
                            .label("← Back")
                            .ghost()
                            .with_size(Size::Small)
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
                    Some(frame) => render_inspector_body(frame, cx),
                    None => render_inspector_empty(),
                }),
        )
        .into_any_element()
}

fn render_inspector_body(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    match &frame.detail {
        InspectorDetail::Loading(message) => render_loading(message),
        InspectorDetail::Error(error) => render_loading(&format!("Error: {error}")),
        InspectorDetail::Feed(feed) => render_feed_inspector(frame, feed, cx),
        InspectorDetail::Track(track) => render_track_inspector(frame, track, cx),
        InspectorDetail::Publisher(publisher) => render_publisher_inspector(publisher, cx),
    }
}

fn render_feed_inspector(
    frame: &InspectorFrame,
    feed: &Feed,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let title = feed_title(feed);
    let mut rows = vec![
        (
            "Artist".to_string(),
            feed.release_artist
                .clone()
                .unwrap_or_else(|| "Unknown".into()),
        ),
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
    optional_row(
        &mut rows,
        "Latest track",
        feed.newest_item_at.and_then(fmt_date),
    );
    optional_row(
        &mut rows,
        "Oldest track",
        feed.oldest_item_at.and_then(fmt_date),
    );
    optional_row(&mut rows, "Description", feed.description.clone());
    optional_row(&mut rows, "Feed URL", feed.feed_url.clone());
    optional_row(&mut rows, "Feed GUID", feed.feed_guid.clone());
    optional_row(&mut rows, "Updated", feed.updated_at.and_then(fmt_date));

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
            feed.image_url.as_deref(),
        ))
        .child(render_detail_grid(rows))
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
                cx,
            ))
        })
        .when(feed.feed_url.is_some(), |el| {
            let url = feed.feed_url.clone().unwrap_or_default();
            el.child(
                subtle_button("Open RSS Feed").on_click(cx.listener(
                    move |_this, _: &ClickEvent, _window, _cx| {
                        let _ = open::that(&url);
                    },
                )),
            )
        })
        .child(render_action_row(frame, cx))
        .child(render_lazy_sections(frame, cx))
        .into_any_element()
}

fn render_track_inspector(
    frame: &InspectorFrame,
    track: &Track,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let title = track_title(track);
    let mut rows = Vec::new();
    optional_row(&mut rows, "Artist", track.track_artist.clone());
    optional_row(&mut rows, "Publisher", track.publisher_text.clone());
    optional_row(&mut rows, "Duration", track.duration_secs.map(fmt_dur));
    optional_row(&mut rows, "Published", track.pub_date.and_then(fmt_date));
    optional_row(
        &mut rows,
        "Track #",
        track.track_number.map(|n| n.to_string()),
    );
    if track.explicit == Some(true) {
        rows.push(("Explicit".into(), "Yes".into()));
    }
    optional_row(&mut rows, "Description", track.description.clone());
    optional_row(&mut rows, "Audio", track.enclosure_url.clone());
    optional_row(
        &mut rows,
        "Feed",
        Some(
            track
                .feed_title
                .clone()
                .or_else(|| track.feed_guid.clone())
                .unwrap_or_default(),
        ),
    );
    optional_row(&mut rows, "Track GUID", track.track_guid.clone());
    optional_row(&mut rows, "Updated", track.updated_at.and_then(fmt_date));

    let feed_guid = track.feed_guid.clone();
    let feed_title = track.feed_title.clone();

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(render_detail_header(
            "track",
            &title,
            track.image_url.as_deref(),
        ))
        .child(render_detail_grid(rows))
        .when(feed_guid.is_some(), |el| {
            let guid = feed_guid.unwrap_or_default();
            let title = feed_title.unwrap_or_else(|| guid.clone());
            el.child(
                subtle_button("Open Feed").on_click(cx.listener(move |this, _, _, cx| {
                    this.push_inspector("feed".into(), guid.clone(), title.clone(), cx);
                })),
            )
        })
        .when(track.enclosure_url.is_some(), |el| {
            let url = track.enclosure_url.clone().unwrap_or_default();
            el.child(
                subtle_button("▶ Play Audio").on_click(cx.listener(
                    move |_this, _: &ClickEvent, _window, _cx| {
                        let _ = open::that(&url);
                    },
                )),
            )
        })
        .child(render_action_row(frame, cx))
        .child(render_lazy_sections(frame, cx))
        .into_any_element()
}

fn render_publisher_inspector(publisher: &Publisher, cx: &mut Context<SearchApp>) -> AnyElement {
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
    let tracks = publisher.tracks.clone().unwrap_or_default();

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(render_detail_header("publisher", &title, None))
        .child(render_detail_grid(rows))
        .when(!feeds.is_empty(), |el| {
            el.child(render_feed_list_section("Feeds", feeds, cx))
        })
        .when(!tracks.is_empty(), |el| {
            el.child(render_track_list_section(
                "Tracks",
                String::new(),
                tracks,
                cx,
            ))
        })
        .into_any_element()
}

fn render_action_row(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    if frame.entity_type != "feed" && frame.entity_type != "track" {
        return div().into_any_element();
    }

    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(8.0))
        .child(
            subtle_button(match frame.contributors {
                LazyPanel::Loaded(_) => "Hide Contributors",
                LazyPanel::Loading => "Loading...",
                LazyPanel::Empty(ref label) => label.as_str(),
                LazyPanel::Hidden => "Show Contributors",
            })
            .disabled(matches!(
                frame.contributors,
                LazyPanel::Loading | LazyPanel::Empty(_)
            ))
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_contributors(cx);
            })),
        )
        .child(
            subtle_button(match frame.value_routes {
                LazyPanel::Loaded(_) => "Hide Value Routes",
                LazyPanel::Loading => "Loading...",
                LazyPanel::Empty(ref label) => label.as_str(),
                LazyPanel::Hidden => "Show Value Routes",
            })
            .disabled(matches!(
                frame.value_routes,
                LazyPanel::Loading | LazyPanel::Empty(_)
            ))
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_value_routes(cx);
            })),
        )
        .into_any_element()
}

fn render_lazy_sections(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    let mut element = div().flex().flex_col().gap(px(16.0));
    if let LazyPanel::Loaded(items) = &frame.contributors {
        element = element.child(render_contributors(items, cx));
    }
    if let LazyPanel::Loaded(items) = &frame.value_routes {
        element = element.child(render_value_routes(items));
    }
    element.into_any_element()
}

fn render_contributors(
    contributors: &[Contributor],
    cx: &mut Context<SearchApp>,
) -> AnyElement {
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
            let name = contributor
                .name
                .clone()
                .unwrap_or_else(|| "Unknown".into());
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
                        .on_click(cx.listener(
                            move |_this, _: &ClickEvent, _window, _cx| {
                                let _ = open::that(&href_for_click);
                            },
                        ))
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

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(section_heading("Contributors"))
        .children(all_elements)
        .into_any_element()
}

fn render_value_routes(routes: &[PaymentRoute]) -> AnyElement {
    let mut groups = BTreeMap::<String, Vec<&PaymentRoute>>::new();
    for route in routes {
        let group = if route.fee.unwrap_or_default() {
            "Fees"
        } else {
            "Recipients"
        };
        groups.entry(group.into()).or_default().push(route);
    }

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(section_heading("Value Routes"))
        .children(groups.into_iter().flat_map(|(group, routes)| {
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
        }))
        .into_any_element()
}

fn render_track_list_section(
    heading: &str,
    note: String,
    tracks: Vec<Track>,
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
                .map(|track| render_track_row(track, cx))
                .collect::<Vec<_>>(),
        )
        .into_any_element()
}

fn render_track_row(track: Track, cx: &mut Context<SearchApp>) -> AnyElement {
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
        .child(render_thumb(&track.image_url, "track", 28.0, false))
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
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(section_heading(heading))
        .children(
            feeds
                .into_iter()
                .map(|feed| {
                    let guid = feed.feed_guid.clone().unwrap_or_default();
                    let title = feed_title(&feed);
                    div()
                        .id(SharedString::from(format!("feed-row:{guid}")))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(4.0))
                        .py(px(5.0))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            this.push_inspector("feed".into(), guid.clone(), title.clone(), cx);
                        }))
                        .child(render_thumb(&feed.image_url, "feed", 28.0, false))
                        .child(truncated(feed_title(&feed)).flex_1())
                        .when(feed.episode_count.is_some(), |el| {
                            el.child(div().text_color(muted()).text_size(px(11.0)).child(format!(
                                "{} tracks",
                                feed.episode_count.unwrap_or_default()
                            )))
                        })
                        .into_any_element()
                })
                .collect::<Vec<_>>(),
        )
        .into_any_element()
}

fn render_detail_header(entity_type: &str, title: &str, image_url: Option<&str>) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(16.0))
        .child(render_thumb(
            &image_url.map(str::to_string),
            entity_type,
            80.0,
            true,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xffffff))
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
                ),
        )
        .into_any_element()
}

fn render_detail_grid(rows: Vec<(String, String)>) -> AnyElement {
    div()
        .grid()
        .grid_cols(2)
        .gap_x(px(12.0))
        .gap_y(px(5.0))
        .children(rows.into_iter().flat_map(|(key, value)| {
            [
                div()
                    .text_color(muted())
                    .whitespace_nowrap()
                    .text_size(px(11.5))
                    .child(SharedString::from(key))
                    .into_any_element(),
                div()
                    .text_size(px(11.5))
                    .line_height(px(17.0))
                    .child(SharedString::from(value))
                    .into_any_element(),
            ]
        }))
        .into_any_element()
}

fn render_thumb(
    image_url: &Option<String>,
    entity_type: &str,
    size: f32,
    large: bool,
) -> AnyElement {
    if let Some(image_url) = image_url {
        return img(image_url.clone())
            .w(px(size))
            .h(px(size))
            .rounded(px(if large { 6.0 } else { 4.0 }))
            .bg(border())
            .flex_shrink_0()
            .into_any_element();
    }

    div()
        .w(px(size))
        .h(px(size))
        .rounded(px(if large { 6.0 } else { 4.0 }))
        .bg(border())
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(if large { 28.0 } else { 14.0 }))
        .flex_shrink_0()
        .child(type_emoji(entity_type))
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

fn group_heading(label: String) -> AnyElement {
    div()
        .text_size(px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(muted())
        .mt(px(6.0))
        .child(SharedString::from(label))
        .into_any_element()
}

fn subtle_button(label: &str) -> Button {
    Button::new(SharedString::from(format!("subtle-button:{label}")))
        .label(SharedString::from(label.to_string()))
        .with_size(Size::Small)
        .ghost()
        .rounded(px(4.0))
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
    rgb(0x777c91)
}

fn accent() -> gpui::Rgba {
    rgb(0x6c7cff)
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
                let view = cx.new(|cx| SearchApp::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("failed to open window");
    });
}
