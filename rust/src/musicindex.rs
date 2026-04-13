use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use gpui::{
    div, img, prelude::*, px, rgb, size, AnyElement, Application, Bounds, ClickEvent, Context,
    Entity, FontWeight, Image, ImageFormat, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ObjectFit, Render, SharedString, Styled, Window, WindowBounds,
    WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Root, Sizable, Size};
use reqwest::blocking::Client as ReqwestClient;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::audio_tags::{read_audio_tags, EmbeddedArtwork, Id3Field};
use crate::config;
use crate::musicbrainz::{
    lookup_recordings, LookupMetadata, MusicBrainzCandidate, MusicBrainzLookup,
};
use crate::track_compare::{
    compare_track_tags, download_track_mp3, ComparisonRow, ComparisonStatus,
};

const DEFAULT_BASE_URL: &str = "https://musicindex.org";
const PAGE_LIMIT: i32 = 20;
const ID3V24_FRAME_IDS: &[&str] = &[
    "AENC", "APIC", "ASPI", "COMM", "COMR", "ENCR", "EQU2", "ETCO", "GEOB", "GRID", "LINK", "MCDI",
    "MLLT", "OWNE", "PRIV", "PCNT", "POPM", "POSS", "RBUF", "RVA2", "RVRB", "SEEK", "SIGN", "SYLT",
    "SYTC", "TALB", "TBPM", "TCOM", "TCON", "TCOP", "TDEN", "TDLY", "TDOR", "TDRC", "TDRL", "TDTG",
    "TENC", "TEXT", "TFLT", "TIPL", "TIT1", "TIT2", "TIT3", "TKEY", "TLAN", "TLEN", "TMCL", "TMED",
    "TMOO", "TOAL", "TOFN", "TOLY", "TOPE", "TOWN", "TPE1", "TPE2", "TPE3", "TPE4", "TPOS", "TPRO",
    "TPUB", "TRCK", "TRSN", "TRSO", "TSOA", "TSOP", "TSOT", "TSRC", "TSSE", "TSST", "TXXX", "UFID",
    "USER", "USLT", "WCOM", "WCOP", "WOAF", "WOAR", "WOAS", "WORS", "WPAY", "WPUB", "WXXX",
];

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
    pub source_links: Option<Vec<SourceEntityLink>>,
    pub source_ids: Option<Vec<SourceEntityId>>,
    pub source_release_claims: Option<Vec<SourceReleaseClaim>>,
    pub payment_routes: Option<Vec<PaymentRoute>>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Track {
    pub track_guid: Option<String>,
    pub feed_guid: Option<String>,
    pub feed_title: Option<String>,
    pub feed_url: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
    pub duration_secs: Option<i32>,
    pub pub_date: Option<i64>,
    pub track_number: Option<i32>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
    pub enclosure_url: Option<String>,
    pub enclosure_type: Option<String>,
    pub enclosure_bytes: Option<i64>,
    pub image_url: Option<String>,
    pub track_artist: Option<String>,
    pub release_artist: Option<String>,
    pub publisher_text: Option<String>,
    pub artist_credit: Option<ArtistCredit>,
    pub source_contributors: Option<Vec<Contributor>>,
    pub source_links: Option<Vec<SourceEntityLink>>,
    pub source_ids: Option<Vec<SourceEntityId>>,
    pub source_release_claims: Option<Vec<SourceReleaseClaim>>,
    pub source_enclosures: Option<Vec<SourceEnclosure>>,
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
pub struct SourceEntityLink {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub link_type: Option<String>,
    pub url: Option<String>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SourceEntityId {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub scheme: Option<String>,
    pub value: Option<String>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SourceReleaseClaim {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub claim_type: Option<String>,
    pub claim_value: Option<String>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SourceEnclosure {
    pub url: Option<String>,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub rel: Option<String>,
    pub title: Option<String>,
    pub is_primary: Option<bool>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
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
    image: Option<Arc<Image>>,
    contributors: LazyPanel<Vec<Contributor>>,
    contributors_collapsed: bool,
    value_routes: LazyPanel<Vec<PaymentRoute>>,
    value_routes_collapsed: bool,
    unused_id3_frames_collapsed: bool,
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
            unused_id3_frames_collapsed: true,
            tag_compare: LazyPanel::Hidden,
            musicbrainz_lookup: LazyPanel::Hidden,
            musicbrainz_selected: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct TagCompareResult {
    path: String,
    rows: Vec<ComparisonRow>,
    file_image: Option<Arc<Image>>,
    contributors: Vec<Contributor>,
    value_routes: Vec<PaymentRoute>,
    id3_fields: Vec<Id3Field>,
}

#[derive(Clone, Debug)]
struct MusicBrainzLookupResult {
    lookup: MusicBrainzLookup,
}

struct DetailRow {
    key: String,
    value: AnyElement,
}

struct AlignedCompareRow {
    field: String,
    rss_value: Option<String>,
    id3_value: Option<String>,
    id3_frame: Option<String>,
    musicbrainz_value: Option<String>,
    musicbrainz_key: Option<String>,
    id3_status: ComparisonStatus,
    musicbrainz_status: ComparisonStatus,
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
                                match detail {
                                    Ok((detail, image)) => {
                                        frame.detail = detail;
                                        frame.image = image;
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
        if self.inspector_stack.len() > 1 {
            self.inspector_stack.pop();
            cx.notify();
        }
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

    fn toggle_unused_id3_frames(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.inspector_stack.last_mut() else {
            return;
        };
        frame.unused_id3_frames_collapsed = !frame.unused_id3_frames_collapsed;
        cx.notify();
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
        let client = Arc::new(Client::new());
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<SearchApp>, cx: &mut gpui::AsyncApp| {
                let request_id = entity_id.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { download_and_compare_track(&client, &request_id) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut SearchApp, cx: &mut Context<SearchApp>| {
                        if let Some(frame) = this.inspector_stack.last_mut() {
                            if frame.entity_type == "track" && frame.entity_id == entity_id {
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
        let client = Arc::new(Client::new());
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
                            if frame.entity_type == "track" && frame.entity_id == entity_id {
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
) -> Result<(InspectorDetail, Option<Arc<Image>>)> {
    match entity_type {
        "feed" => {
            let feed = client.fetch_feed(entity_id, Some("tracks"))?;
            let image = feed
                .image_url
                .as_deref()
                .and_then(|url| download_image(&client.client, url));
            Ok((InspectorDetail::Feed(feed), image))
        }
        "track" => {
            let track = client.fetch_track(entity_id, None)?;
            let image = track
                .image_url
                .as_deref()
                .and_then(|url| download_image(&client.client, url));
            Ok((InspectorDetail::Track(track), image))
        }
        "publisher" => Ok((
            InspectorDetail::Publisher(client.fetch_publisher(entity_id)?),
            None,
        )),
        _ => Err(anyhow!("unknown inspector entity type: {entity_type}")),
    }
}

fn download_and_compare_track(client: &Client, entity_id: &str) -> Result<TagCompareResult> {
    let track = client.fetch_track(
        entity_id,
        Some("source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes"),
    )?;
    let feed = match track.feed_guid.as_deref() {
        Some(feed_guid) => client
            .fetch_feed(
                feed_guid,
                Some("source_links,source_ids,source_release_claims"),
            )
            .ok(),
        None => None,
    };
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let downloaded = download_track_mp3(&cfg, &client.client, &track)?;
    let tags = read_audio_tags(&downloaded.path)?;
    let file_image = tags.artwork.as_ref().and_then(image_from_artwork);

    Ok(TagCompareResult {
        path: downloaded.path.display().to_string(),
        rows: compare_track_rows(&track, feed.as_ref(), &tags),
        file_image,
        contributors: track.source_contributors.unwrap_or_default(),
        value_routes: track.payment_routes.unwrap_or_default(),
        id3_fields: tags.fields,
    })
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

    Ok(MusicBrainzLookupResult { lookup })
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

fn compare_track_rows(
    track: &Track,
    feed: Option<&Feed>,
    tags: &crate::audio_tags::AudioTags,
) -> Vec<ComparisonRow> {
    let mut rows = compare_track_tags(track, tags);

    push_compare_row(&mut rows, "Nostr handle", track_nostr(track), None);
    push_compare_row(&mut rows, "Website", track_website(track), None);
    push_compare_row(
        &mut rows,
        "Release pubdate",
        track_release_pubdate(track),
        None,
    );

    if let Some(feed) = feed {
        push_if_differs(
            &mut rows,
            "Feed Nostr handle",
            feed_nostr(feed),
            track_nostr(track),
        );
        push_if_differs(
            &mut rows,
            "Feed Website",
            feed_website(feed),
            track_website(track),
        );
        push_if_differs(
            &mut rows,
            "Feed Release pubdate",
            feed_release_pubdate(feed),
            track_release_pubdate(track),
        );
    }

    rows
}

fn push_if_differs(
    rows: &mut Vec<ComparisonRow>,
    field: &'static str,
    feed_value: Option<String>,
    track_value: Option<String>,
) {
    if normalized_compare_value(feed_value.as_deref())
        != normalized_compare_value(track_value.as_deref())
    {
        push_compare_row(rows, field, feed_value, None);
    }
}

fn push_compare_row(
    rows: &mut Vec<ComparisonRow>,
    field: &'static str,
    source_value: Option<String>,
    tag_value: Option<String>,
) {
    if normalized_compare_value(source_value.as_deref()).is_some()
        || normalized_compare_value(tag_value.as_deref()).is_some()
    {
        rows.push(ComparisonRow {
            field,
            source_value,
            tag_value,
            status: ComparisonStatus::MissingTag,
        });
    }
}

fn track_nostr(track: &Track) -> Option<String> {
    nostr_from_ids(track.source_ids.as_deref())
}

fn feed_nostr(feed: &Feed) -> Option<String> {
    nostr_from_ids(feed.source_ids.as_deref())
}

fn nostr_from_ids(ids: Option<&[SourceEntityId]>) -> Option<String> {
    ids?.iter().find_map(|id| {
        if id.scheme.as_deref() == Some("nostr_npub") {
            id.value.clone()
        } else {
            None
        }
    })
}

fn track_website(track: &Track) -> Option<String> {
    website_from_links(track.source_links.as_deref())
}

fn feed_website(feed: &Feed) -> Option<String> {
    website_from_links(feed.source_links.as_deref())
}

fn website_from_links(links: Option<&[SourceEntityLink]>) -> Option<String> {
    links?.iter().find_map(|link| {
        let link_type = link.link_type.as_deref()?;
        if link_type == "website" || link_type == "web_page" {
            link.url.clone()
        } else {
            None
        }
    })
}

fn track_release_pubdate(track: &Track) -> Option<String> {
    release_pubdate_from_claims(track.source_release_claims.as_deref())
        .or_else(|| track.pub_date.and_then(fmt_date))
}

fn feed_release_pubdate(feed: &Feed) -> Option<String> {
    release_pubdate_from_claims(feed.source_release_claims.as_deref())
        .or_else(|| feed.release_date.and_then(fmt_date))
}

fn release_pubdate_from_claims(claims: Option<&[SourceReleaseClaim]>) -> Option<String> {
    claims?.iter().find_map(|claim| {
        if claim.claim_type.as_deref() != Some("release_date") {
            return None;
        }

        let value = claim.claim_value.as_deref()?;
        value
            .parse::<i64>()
            .ok()
            .and_then(fmt_date)
            .or_else(|| Some(value.to_string()))
    })
}

fn normalized_compare_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
        .child(render_thumb(None, &row.entity_type, 36.0, false))
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
        .child(render_detail_header("feed", &title, frame.image.as_ref()))
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
            el.child(subtle_button("Open RSS Feed").on_click(cx.listener(
                move |_this, _: &ClickEvent, _window, _cx| {
                    let _ = open::that(&url);
                },
            )))
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
    match &frame.tag_compare {
        LazyPanel::Loaded(result) => render_track_window(frame, track, Some(result), cx),
        LazyPanel::Loading | LazyPanel::Empty(_) | LazyPanel::Hidden => {
            render_track_window(frame, track, None, cx)
        }
    }
}

fn render_track_left_column(
    frame: &InspectorFrame,
    track: &Track,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(render_track_header(frame, track, cx))
        .child(render_action_row(frame, cx))
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
    track: &Track,
    result: Option<&TagCompareResult>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let show_id3_panel = !matches!(frame.tag_compare, LazyPanel::Hidden);
    let show_musicbrainz_panel = !matches!(frame.musicbrainz_lookup, LazyPanel::Hidden);
    let columns: u16 = 1 + u16::from(show_id3_panel) + u16::from(show_musicbrainz_panel);

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
                .child(render_track_left_column(frame, track, cx))
                .when(show_id3_panel, |el| {
                    el.child(if let Some(result) = result {
                        render_file_header(result)
                    } else {
                        render_track_compare_panel(frame)
                    })
                })
                .when(show_musicbrainz_panel, |el| {
                    el.child(render_musicbrainz_panel(frame, cx))
                }),
        )
        .child(render_track_metadata_grid(frame, track, result, cx))
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
        .flex_col()
        .items_start()
        .gap(px(4.0))
        .when(frame.entity_type == "track", |el| {
            el.child(
                metadata_action_button(match frame.tag_compare {
                    LazyPanel::Loaded(_) => "Hide Compare",
                    LazyPanel::Loading => "Downloading...",
                    LazyPanel::Empty(_) | LazyPanel::Hidden => "Download + Compare",
                })
                .disabled(matches!(frame.tag_compare, LazyPanel::Loading))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_tag_compare(cx);
                })),
            )
            .child(
                metadata_action_button(match frame.musicbrainz_lookup {
                    LazyPanel::Loaded(_) => "Hide MusicBrainz",
                    LazyPanel::Loading => "Searching MusicBrainz...",
                    LazyPanel::Empty(_) | LazyPanel::Hidden => "MusicBrainz",
                })
                .disabled(matches!(frame.musicbrainz_lookup, LazyPanel::Loading))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_musicbrainz_lookup(cx);
                })),
            )
        })
        .into_any_element()
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
        .child(render_thumb(None, "track", 80.0, true))
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
        .text_color(badge_text("track"))
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

fn render_track_metadata_grid(
    frame: &InspectorFrame,
    track: &Track,
    result: Option<&TagCompareResult>,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let selected_musicbrainz = match &frame.musicbrainz_lookup {
        LazyPanel::Loaded(lookup) => selected_musicbrainz_candidate(frame, lookup),
        _ => None,
    };
    let show_musicbrainz = !matches!(frame.musicbrainz_lookup, LazyPanel::Hidden);
    let show_id3 = !matches!(frame.tag_compare, LazyPanel::Hidden);
    let rows = result.map_or_else(
        || track_metadata_rows(track, selected_musicbrainz),
        |result| aligned_compare_rows(result, selected_musicbrainz),
    );

    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(render_metadata_grid(rows, show_id3, show_musicbrainz))
        .when_some(result.filter(|_| show_id3), |el, result| {
            el.child(render_unused_id3v24_frames(
                result,
                frame.unused_id3_frames_collapsed,
                cx,
            ))
        })
        .into_any_element()
}

fn render_metadata_grid(
    rows: Vec<AlignedCompareRow>,
    show_id3: bool,
    show_musicbrainz: bool,
) -> AnyElement {
    let mut cells: Vec<AnyElement> = Vec::new();
    cells.push(metadata_heading_cell("RSS"));
    if show_id3 {
        cells.push(metadata_heading_cell("ID3"));
    }
    if show_musicbrainz {
        cells.push(metadata_heading_cell("MusicBrainz"));
    }

    for row in rows {
        cells.push(metadata_rss_cell(
            row.field,
            row.rss_value.as_deref().unwrap_or(""),
        ));
        if show_id3 {
            let id3_color = comparison_status_color(&row.id3_status);
            cells.push(metadata_value_cell(compare_tag_cell(
                row.id3_value.as_deref().unwrap_or(""),
                Some(id3_color),
                row.id3_frame.as_deref(),
            )));
        }
        if show_musicbrainz {
            let musicbrainz_color = comparison_status_color(&row.musicbrainz_status);
            cells.push(metadata_value_cell(compare_tag_cell(
                row.musicbrainz_value.as_deref().unwrap_or(""),
                Some(musicbrainz_color),
                row.musicbrainz_key.as_deref(),
            )));
        }
    }

    div()
        .grid()
        .grid_cols(1 + u16::from(show_id3) + u16::from(show_musicbrainz))
        .gap_x(px(24.0))
        .gap_y(px(7.0))
        .children(cells)
        .into_any_element()
}

fn metadata_heading_cell(label: &str) -> AnyElement {
    div()
        .pl(px(96.0))
        .text_color(muted())
        .font_weight(FontWeight::BOLD)
        .text_size(px(10.5))
        .child(SharedString::from(label.to_string()))
        .into_any_element()
}

fn metadata_rss_cell(field: String, value: &str) -> AnyElement {
    div()
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
                .child(SharedString::from(field)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(compare_cell(value, Some(text()))),
        )
        .into_any_element()
}

fn metadata_value_cell(value: AnyElement) -> AnyElement {
    div().pl(px(96.0)).min_w_0().child(value).into_any_element()
}

fn render_unused_id3v24_frames(
    result: &TagCompareResult,
    collapsed: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let unused = unused_id3v24_frames(result);
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            render_clickable_section_heading(
                &format!("Unused ID3v2.4 frames ({})", unused.len()),
                collapsed,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_unused_id3_frames(cx);
            })),
        )
        .when(!collapsed, |el| {
            el.child(
                div()
                    .pl(px(96.0))
                    .text_color(muted())
                    .text_size(px(10.0))
                    .line_height(px(15.0))
                    .child(SharedString::from(unused.join(" "))),
            )
        })
        .into_any_element()
}

fn unused_id3v24_frames(result: &TagCompareResult) -> Vec<&'static str> {
    ID3V24_FRAME_IDS
        .iter()
        .copied()
        .filter(|frame_id| {
            !result
                .id3_fields
                .iter()
                .any(|field| field.frame_id == *frame_id)
        })
        .collect()
}

fn track_metadata_rows(
    track: &Track,
    musicbrainz: Option<&MusicBrainzCandidate>,
) -> Vec<AlignedCompareRow> {
    let mut rows = Vec::new();
    push_track_metadata_row(
        &mut rows,
        "Artist",
        track.track_artist.clone(),
        musicbrainz_value_for_field("Artist", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "Album/Feed",
        track.feed_title.clone(),
        musicbrainz_value_for_field("Album/Feed", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "Track #",
        track.track_number.map(|number| number.to_string()),
        musicbrainz_value_for_field("Track #", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "Publisher",
        track.publisher_text.clone(),
        musicbrainz_value_for_field("Publisher", musicbrainz),
    );
    push_track_metadata_row(&mut rows, "Nostr handle", track_nostr(track), None);
    push_track_metadata_row(&mut rows, "Website", track_website(track), None);
    push_track_metadata_row(
        &mut rows,
        "Release pubdate",
        track_release_pubdate(track),
        musicbrainz_value_for_field("Release pubdate", musicbrainz),
    );
    push_track_metadata_row(
        &mut rows,
        "Contributors",
        track
            .source_contributors
            .as_deref()
            .and_then(summarize_contributors),
        None,
    );
    push_track_metadata_row(
        &mut rows,
        "Value Routes",
        track
            .payment_routes
            .as_deref()
            .and_then(summarize_value_routes),
        None,
    );

    if let Some(candidate) = musicbrainz {
        rows.extend(musicbrainz_remainder_rows(candidate));
    }

    rows
}

fn push_track_metadata_row(
    rows: &mut Vec<AlignedCompareRow>,
    field: &str,
    rss_value: Option<String>,
    musicbrainz_value: Option<String>,
) {
    let musicbrainz_status =
        compare_optional_values(rss_value.as_deref(), musicbrainz_value.as_deref());
    rows.push(AlignedCompareRow {
        field: field.into(),
        rss_value,
        id3_value: None,
        id3_frame: None,
        musicbrainz_value,
        musicbrainz_key: musicbrainz_key_for_field(field).map(str::to_string),
        id3_status: ComparisonStatus::MissingTag,
        musicbrainz_status,
    });
}

fn aligned_compare_rows(
    result: &TagCompareResult,
    musicbrainz: Option<&MusicBrainzCandidate>,
) -> Vec<AlignedCompareRow> {
    let mut rows = result
        .rows
        .iter()
        .filter(|row| row.field != "Title")
        .map(|row| {
            let musicbrainz_value = musicbrainz_value_for_field(row.field, musicbrainz);
            AlignedCompareRow {
                field: row.field.to_string(),
                rss_value: row.source_value.clone(),
                id3_value: row.tag_value.clone(),
                id3_frame: id3_frame_hint(row.field).map(str::to_string),
                musicbrainz_status: compare_optional_values(
                    row.source_value.as_deref(),
                    musicbrainz_value.as_deref(),
                ),
                musicbrainz_value,
                musicbrainz_key: musicbrainz_key_for_field(row.field).map(str::to_string),
                id3_status: row.status.clone(),
            }
        })
        .collect::<Vec<_>>();

    rows.push(AlignedCompareRow {
        field: "Contributors".into(),
        rss_value: summarize_contributors(&result.contributors),
        id3_value: None,
        id3_frame: None,
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingTag,
        musicbrainz_status: ComparisonStatus::MissingTag,
    });
    rows.push(AlignedCompareRow {
        field: "Value Routes".into(),
        rss_value: summarize_value_routes(&result.value_routes),
        id3_value: None,
        id3_frame: None,
        musicbrainz_value: None,
        musicbrainz_key: None,
        id3_status: ComparisonStatus::MissingTag,
        musicbrainz_status: ComparisonStatus::MissingTag,
    });

    for field in result
        .id3_fields
        .iter()
        .filter(|field| !id3_frame_is_summarized(&field.frame_id))
    {
        rows.push(AlignedCompareRow {
            field: format!("ID3 {}", field.frame_id),
            rss_value: None,
            id3_value: Some(field.value.clone()),
            id3_frame: Some(field.frame_id.clone()),
            musicbrainz_value: None,
            musicbrainz_key: None,
            id3_status: ComparisonStatus::MissingSource,
            musicbrainz_status: ComparisonStatus::MissingBoth,
        });
    }

    if let Some(candidate) = musicbrainz {
        rows.extend(musicbrainz_remainder_rows(candidate));
    }

    rows
}

fn musicbrainz_value_for_field(
    field: &str,
    candidate: Option<&MusicBrainzCandidate>,
) -> Option<String> {
    let candidate = candidate?;
    match field {
        "Title" => Some(candidate.title.clone()),
        "Artist" => candidate.artist.clone(),
        "Album/Feed" => candidate.release_title.clone(),
        "Track #" => candidate.track_number.clone(),
        "Publisher" => join_values(&candidate.labels),
        "Website" | "Feed Website" => join_values(&candidate.urls),
        "Release pubdate" | "Feed Release pubdate" => candidate.release_date.clone(),
        _ => None,
    }
}

fn musicbrainz_key_for_field(field: &str) -> Option<&'static str> {
    match field {
        "Title" => Some("recording.title"),
        "Artist" => Some("artist-credit.name"),
        "Album/Feed" => Some("release.title"),
        "Track #" => Some("track.number"),
        "Publisher" => Some("label-info.label.name"),
        "Website" | "Feed Website" => Some("relation.url.resource"),
        "Release pubdate" | "Feed Release pubdate" => Some("release.date"),
        _ => None,
    }
}

fn musicbrainz_remainder_rows(candidate: &MusicBrainzCandidate) -> Vec<AlignedCompareRow> {
    let mut rows = Vec::new();
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz recording",
        "recording.id",
        Some(candidate.recording_id.clone()),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz release",
        "release.id",
        candidate.release_id.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz release group",
        "release-group.id",
        candidate.release_group_id.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz country",
        "release.country",
        candidate.country.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz status",
        "release.status",
        candidate.release_status.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz packaging",
        "release.packaging",
        candidate.release_packaging.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz barcode",
        "release.barcode",
        candidate.release_barcode.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz release note",
        "release.disambiguation",
        candidate.release_disambiguation.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz release group type",
        "release-group.primary-type",
        candidate.release_group_type.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz release group secondary types",
        "release-group.secondary-types",
        join_values(&candidate.release_group_secondary_types),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz labels",
        "label-info",
        join_values(&candidate.labels),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz format",
        "medium.format",
        candidate.format.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz medium position",
        "medium.position",
        candidate
            .medium_position
            .map(|position| position.to_string()),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz medium title",
        "medium.title",
        candidate.medium_title.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz track position",
        "track.position",
        candidate
            .track_position
            .map(|position| position.to_string()),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz track title",
        "track.title",
        candidate.track_title.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz track artist",
        "track.artist-credit.name",
        candidate.track_artist.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz track note",
        "recording.disambiguation",
        candidate.track_disambiguation.clone(),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz track length",
        "track.length",
        candidate.track_length_ms.map(fmt_ms),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz tracks",
        "medium.track-count",
        candidate.total_tracks.map(|count| count.to_string()),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz ISRCs",
        "recording.isrcs",
        join_values(&candidate.isrcs),
    );
    push_musicbrainz_only_row(
        &mut rows,
        "MusicBrainz URLs",
        "relation.url.resource",
        join_values(&candidate.urls),
    );
    rows
}

fn push_musicbrainz_only_row(
    rows: &mut Vec<AlignedCompareRow>,
    field: &str,
    musicbrainz_key: &str,
    value: Option<String>,
) {
    if normalized_compare_value(value.as_deref()).is_some() {
        rows.push(AlignedCompareRow {
            field: field.into(),
            rss_value: None,
            id3_value: None,
            id3_frame: None,
            musicbrainz_value: value,
            musicbrainz_key: Some(musicbrainz_key.into()),
            id3_status: ComparisonStatus::MissingBoth,
            musicbrainz_status: ComparisonStatus::MissingSource,
        });
    }
}

fn compare_optional_values(source: Option<&str>, target: Option<&str>) -> ComparisonStatus {
    let source = normalized_compare_value(source);
    let target = normalized_compare_value(target);
    match (&source, &target) {
        (Some(source), Some(target)) if source == target => ComparisonStatus::Match,
        (Some(_), Some(_)) => ComparisonStatus::Different,
        (Some(_), None) => ComparisonStatus::MissingTag,
        (None, Some(_)) => ComparisonStatus::MissingSource,
        (None, None) => ComparisonStatus::MissingBoth,
    }
}

fn summarize_contributors(contributors: &[Contributor]) -> Option<String> {
    if contributors.is_empty() {
        return None;
    }
    Some(
        contributors
            .iter()
            .map(|contributor| {
                let name = contributor.name.as_deref().unwrap_or("Unknown");
                contributor
                    .role
                    .as_ref()
                    .map_or_else(|| name.to_string(), |role| format!("{name} ({role})"))
            })
            .collect::<Vec<_>>()
            .join(" · "),
    )
}

fn summarize_value_routes(routes: &[PaymentRoute]) -> Option<String> {
    if routes.is_empty() {
        return None;
    }
    Some(
        routes
            .iter()
            .map(|route| {
                let name = route
                    .recipient_name
                    .as_deref()
                    .unwrap_or("Unnamed recipient");
                let split = route.split.unwrap_or_default();
                let route_type = route.route_type.as_deref().unwrap_or("route");
                format!("{name} ({route_type} {split}%)")
            })
            .collect::<Vec<_>>()
            .join(" · "),
    )
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

fn render_file_header(result: &TagCompareResult) -> AnyElement {
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
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_text("track"))
                        .bg(type_color("track"))
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .mb(px(6.0))
                        .child("Embedded id3"),
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

fn compare_cell(value: &str, color: Option<gpui::Rgba>) -> AnyElement {
    let mut cell = div().text_size(px(11.0)).line_height(px(16.0));
    if let Some(color) = color {
        cell = cell.text_color(color);
    }
    cell.child(SharedString::from(value.to_string()))
        .into_any_element()
}

fn compare_tag_cell(value: &str, color: Option<gpui::Rgba>, frame_id: Option<&str>) -> AnyElement {
    let mut value_cell = div().text_size(px(11.0)).line_height(px(16.0));
    if let Some(color) = color {
        value_cell = value_cell.text_color(color);
    }
    let frame_id = frame_id.map(ToOwned::to_owned);

    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(6.0))
        .when(frame_id.is_some(), |el| {
            el.child(
                div()
                    .text_color(muted())
                    .text_size(px(9.5))
                    .line_height(px(16.0))
                    .child(SharedString::from(frame_id.clone().unwrap_or_default())),
            )
        })
        .child(value_cell.child(SharedString::from(value.to_string())))
        .into_any_element()
}

fn id3_frame_hint(field: &str) -> Option<&'static str> {
    match field {
        "Title" => Some("TIT2"),
        "Artist" => Some("TPE1"),
        "Album/Feed" => Some("TALB"),
        "Track #" => Some("TRCK"),
        "Publisher" => Some("TXXX"),
        _ => None,
    }
}

fn id3_frame_is_summarized(frame_id: &str) -> bool {
    matches!(frame_id, "TIT2" | "TPE1" | "TALB" | "TRCK")
}

fn comparison_status_color(status: &ComparisonStatus) -> gpui::Rgba {
    match status {
        ComparisonStatus::Match => rgb(0x4caf82),
        ComparisonStatus::Different => rgb(0xffc857),
        ComparisonStatus::MissingSource | ComparisonStatus::MissingTag => rgb(0xff8a65),
        ComparisonStatus::MissingBoth => muted(),
    }
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
        .child(render_thumb(None, "track", 28.0, false))
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
                        .child(render_thumb(None, "feed", 28.0, false))
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

fn render_detail_header(entity_type: &str, title: &str, image: Option<&Arc<Image>>) -> AnyElement {
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
                ),
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
                    .child(SharedString::from(value))
                    .into_any_element(),
            })
            .collect(),
    )
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
        .text_color(text())
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

fn subtle_button(label: &str) -> Button {
    Button::new(SharedString::from(format!("subtle-button:{label}")))
        .label(SharedString::from(label.to_string()))
        .with_size(Size::Small)
        .ghost()
        .text_color(text())
        .rounded(px(4.0))
        .border_1()
        .border_color(accent())
}

fn metadata_action_button(label: &str) -> Button {
    Button::new(SharedString::from(format!("metadata-action:{label}")))
        .label(SharedString::from(label.to_string()))
        .with_size(Size::XSmall)
        .compact()
        .ghost()
        .text_color(text())
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

fn fmt_ms(ms: i64) -> String {
    fmt_dur((ms / 1000).try_into().unwrap_or(i32::MAX))
}

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

#[cfg(test)]
mod tests {
    use super::{unused_id3v24_frames, TagCompareResult};
    use crate::audio_tags::Id3Field;

    #[test]
    fn unused_id3v24_frames_excludes_present_frames() {
        let result = TagCompareResult {
            path: String::new(),
            rows: Vec::new(),
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
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

        let unused = unused_id3v24_frames(&result);

        assert!(
            !unused.contains(&"TIT2"),
            "present title frame should not be listed"
        );
        assert!(
            !unused.contains(&"APIC"),
            "present artwork frame should not be listed"
        );
        assert!(
            unused.contains(&"TPE1"),
            "absent artist frame should remain available"
        );
    }
}
