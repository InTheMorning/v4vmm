use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client as ReqwestClient;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "https://api.musicindex.org";
pub const PAGE_LIMIT: i32 = 20;

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
    pub feed_guid: Option<String>,
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
pub struct TrackListResponse {
    pub data: Vec<Track>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RecentFeedsResponse {
    pub data: Vec<Feed>,
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
    pub feed_count: Option<i32>,
    pub track_count: Option<i32>,
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
    #[serde(alias = "owner_name")]
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
    #[serde(alias = "persons")]
    pub source_contributors: Option<Vec<Contributor>>,
    #[serde(alias = "links")]
    pub source_links: Option<Vec<SourceEntityLink>>,
    #[serde(alias = "entity_ids")]
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
    #[serde(alias = "author_name")]
    pub track_artist: Option<String>,
    pub release_artist: Option<String>,
    pub publisher_text: Option<String>,
    pub artist_credit: Option<ArtistCredit>,
    #[serde(alias = "persons")]
    pub source_contributors: Option<Vec<Contributor>>,
    #[serde(alias = "links")]
    pub source_links: Option<Vec<SourceEntityLink>>,
    #[serde(alias = "entity_ids")]
    pub source_ids: Option<Vec<SourceEntityId>>,
    pub source_release_claims: Option<Vec<SourceReleaseClaim>>,
    pub source_enclosures: Option<Vec<SourceEnclosure>>,
    pub payment_routes: Option<Vec<PaymentRoute>>,
    pub updated_at: Option<i64>,
}

pub fn track_with_feed_defaults(mut track: Track, feed: Option<&Feed>) -> Track {
    if let Some(feed) = feed {
        if track.feed_url.is_none() {
            track.feed_url = feed.feed_url.clone();
        }
        if track.feed_guid.is_none() {
            track.feed_guid = feed.feed_guid.clone();
        }
        if track.feed_title.is_none() {
            track.feed_title = feed.title.clone().or_else(|| feed.name.clone());
        }
        if track.image_url.is_none() {
            track.image_url = feed.image_url.clone();
        }
        if track.publisher_text.is_none() {
            track.publisher_text = feed.publisher_text.clone();
        }
        if track.description.is_none() {
            track.description = feed.description.clone();
        }
        if track.release_artist.is_none() {
            track.release_artist = feed.release_artist.clone();
        }
        if track.source_contributors.is_none() {
            track.source_contributors = feed.source_contributors.clone();
        }
        if track.source_links.is_none() {
            track.source_links = feed.source_links.clone();
        }
        if track.source_ids.is_none() {
            track.source_ids = feed.source_ids.clone();
        }
        if track.source_release_claims.is_none() {
            track.source_release_claims = feed.source_release_claims.clone();
        }
        if track.payment_routes.is_none() {
            track.payment_routes = feed.payment_routes.clone();
        }
    }
    track
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
    pub img: Option<String>,
    pub npub: Option<String>,
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
    pub artist_id: Option<String>,
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
pub struct LiveItemCreateResponse {
    pub event_id: String,
    pub broadcaster_token: String,
    pub metadata_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_value_url: Option<String>,
    pub events_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_io_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMetadataPublishRequest {
    pub event_id: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMetadataPublishResponse {
    pub event_id: String,
    pub accepted: bool,
    pub seq: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMetadataSnapshot {
    pub event_id: String,
    pub seq: u64,
    pub updated_at: String,
    pub metadata: serde_json::Value,
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
    pub client: ReqwestClient,
    base_url: String,
}

impl Client {
    pub fn new() -> Self {
        Self {
            client: crate::http_client::document(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn new_with_base_url(base_url: String) -> Self {
        Self {
            client: crate::http_client::document(),
            base_url,
        }
    }

    pub fn search(
        &self,
        query: &str,
        entity_type: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
        fuzzy: bool,
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
        if fuzzy {
            params.push(("fuzzy", "true".to_string()));
        }

        self.get_json(&["v1", "search"], &params)
    }

    pub fn search_publishers(
        &self,
        query: &str,
        limit: Option<i32>,
        fuzzy: bool,
    ) -> Result<PublisherSearchResponse> {
        let params = vec![
            ("q", query.to_string()),
            ("limit", limit.unwrap_or(PAGE_LIMIT).to_string()),
            ("fuzzy", fuzzy.to_string()),
        ];
        self.get_json(&["v1", "publishers"], &params)
    }

    pub fn fetch_recent_feeds(
        &self,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<RecentFeedsResponse> {
        let mut params = vec![("limit", limit.unwrap_or(PAGE_LIMIT).to_string())];
        if let Some(cursor) = cursor {
            params.push(("cursor", cursor.to_string()));
        }
        self.get_json(&["v1", "feeds", "recent"], &params)
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

    pub fn fetch_feed_track(
        &self,
        feed_guid: &str,
        track_guid: &str,
        include: Option<&str>,
    ) -> Result<Track> {
        let mut params = Vec::new();
        if let Some(include) = include {
            params.push(("include", include.to_string()));
        }
        self.fetch_wrapped_with_query(&["v1", "feeds", feed_guid, "tracks", track_guid], &params)
    }

    pub fn fetch_tracks_by_artist(
        &self,
        artist: &str,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<TrackListResponse> {
        let mut params = vec![
            ("artist", artist.to_string()),
            ("limit", limit.unwrap_or(PAGE_LIMIT).to_string()),
        ];
        if let Some(cursor) = cursor {
            params.push(("cursor", cursor.to_string()));
        }
        self.get_json(&["v1", "tracks"], &params)
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

    pub fn health(&self) -> Result<String> {
        self.get_text(&["health"], &[])
    }

    pub fn create_live_item(&self) -> Result<LiveItemCreateResponse> {
        self.post_json(&["v1", "liveitems"], &serde_json::json!({}))
    }

    pub fn fetch_live_metadata(&self, event_id: &str) -> Result<LiveMetadataSnapshot> {
        validate_live_metadata_event_id("path event_id", event_id)?;
        self.get_json(&["v1", "liveitems", event_id, "metadata"], &[])
    }

    pub fn fetch_live_metadata_optional(
        &self,
        event_id: &str,
    ) -> Result<Option<LiveMetadataSnapshot>> {
        validate_live_metadata_event_id("path event_id", event_id)?;
        let url = self.build_url(&["v1", "liveitems", event_id, "metadata"], &[])?;
        let response = self.client.get(url).send()?;
        if response.status() == StatusCode::NOT_FOUND {
            let body = response.text()?;
            if response_error_matches(&body, "metadata_not_found") {
                return Ok(None);
            }
            return Err(anyhow!("GET failed with HTTP 404 Not Found: {body}"));
        }
        response_json(response, "GET").map(Some)
    }

    pub fn publish_live_metadata(
        &self,
        event_id: &str,
        request: &LiveMetadataPublishRequest,
    ) -> Result<LiveMetadataPublishResponse> {
        validate_live_metadata_request(event_id, request)?;
        self.post_json(&["v1", "liveitems", event_id, "metadata"], request)
    }

    pub fn publish_live_metadata_with_token(
        &self,
        event_id: &str,
        token: &str,
        request: &LiveMetadataPublishRequest,
    ) -> Result<LiveMetadataPublishResponse> {
        validate_live_metadata_request(event_id, request)?;
        validate_bearer_token(token)?;
        self.post_json_with_bearer(
            &["v1", "liveitems", event_id, "metadata"],
            request,
            Some(token),
        )
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
        let url = self.build_url(path_segments, query)?;
        let response = self.client.get(url).send()?;
        response_json(response, "GET")
    }

    fn get_text(&self, path_segments: &[&str], query: &[(&str, String)]) -> Result<String> {
        let url = self.build_url(path_segments, query)?;
        let response = self.client.get(url).send()?;
        response_text(response, "GET")
    }

    fn post_json<T, B>(&self, path_segments: &[&str], body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        self.post_json_with_bearer(path_segments, body, None)
    }

    fn post_json_with_bearer<T, B>(
        &self,
        path_segments: &[&str],
        body: &B,
        bearer_token: Option<&str>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        let url = self.build_url(path_segments, &[])?;
        let mut request = self.client.post(url).json(body);
        if let Some(token) = bearer_token {
            request = request.bearer_auth(token);
        }
        let response = request.send()?;
        response_json(response, "POST")
    }

    fn build_url(&self, path_segments: &[&str], query: &[(&str, String)]) -> Result<reqwest::Url> {
        let mut url = reqwest::Url::parse(&format!("{}/", self.base_url.trim_end_matches('/')))?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow!("base URL cannot be a base: {}", self.base_url))?;
            for segment in path_segments {
                segments.push(&sanitize_api_path_segment(segment)?);
            }
        }

        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, &sanitize_api_query_value(value));
            }
        }

        Ok(url)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

fn sanitize_api_path_segment(value: &str) -> Result<String> {
    let sanitized = sanitize_api_query_value(value);
    if sanitized.is_empty() {
        return Err(anyhow!("API path segment is empty"));
    }
    Ok(sanitized)
}

fn sanitize_api_query_value(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch == '\0' || ch == '\\' || ch.is_control() {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn validate_live_metadata_request(
    event_id: &str,
    request: &LiveMetadataPublishRequest,
) -> Result<()> {
    validate_live_metadata_event_id("path event_id", event_id)?;
    validate_live_metadata_event_id("request event_id", &request.event_id)?;
    if event_id != request.event_id {
        return Err(anyhow!(
            "live metadata event_id mismatch: path {event_id:?}, body {:?}",
            request.event_id
        ));
    }
    Ok(())
}

fn validate_bearer_token(token: &str) -> Result<()> {
    if token.trim().is_empty() {
        return Err(anyhow!("live metadata bearer token is empty"));
    }
    if token.chars().any(|ch| ch == '\0' || ch.is_control()) {
        return Err(anyhow!("live metadata bearer token contains control bytes"));
    }
    Ok(())
}

fn validate_live_metadata_event_id(label: &str, value: &str) -> Result<()> {
    let sanitized = sanitize_api_query_value(value);
    if sanitized.is_empty() {
        return Err(anyhow!("live metadata {label} is empty"));
    }
    if sanitized != value {
        return Err(anyhow!(
            "live metadata {label} contains invalid whitespace or control bytes"
        ));
    }
    Ok(())
}

fn response_json<T>(response: reqwest::blocking::Response, method: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if status == StatusCode::NO_CONTENT {
        return Err(anyhow!("{method} returned no content"));
    }
    let body = response_text_with_status(response, method, status)?;
    serde_json::from_str(&body).with_context(|| format!("decode {method} JSON response"))
}

fn response_text(response: reqwest::blocking::Response, method: &str) -> Result<String> {
    let status = response.status();
    response_text_with_status(response, method, status)
}

fn response_text_with_status(
    response: reqwest::blocking::Response,
    method: &str,
    status: StatusCode,
) -> Result<String> {
    let body = response.text()?;
    if !status.is_success() {
        return Err(anyhow!("{method} failed with HTTP {status}: {body}"));
    }
    Ok(body)
}

fn response_error_matches(body: &str, expected: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .as_deref()
        == Some(expected)
}

#[cfg(test)]
mod tests {
    use super::{
        Client, Contributor, Feed, LiveMetadataPublishRequest, PaymentRoute, SourceEntityId, Track,
    };

    #[test]
    fn build_url_sanitizes_metadata_path_segments_and_query_values() {
        let client = Client::new();
        let url = client
            .build_url(
                &["v1", "publishers", "Den+ did this / Records"],
                &[("q", "Den+\0 did this".into())],
            )
            .expect("url");

        let segments = url
            .path_segments()
            .expect("path segments")
            .collect::<Vec<_>>();
        assert_eq!(
            segments,
            vec!["v1", "publishers", "Den+%20did%20this%20%2F%20Records"]
        );

        let query_pairs = url.query_pairs().collect::<Vec<_>>();
        assert_eq!(query_pairs, vec![("q".into(), "Den+ did this".into())]);
    }

    #[test]
    fn build_url_sanitizes_backslash_query_values() {
        let client = Client::new();
        let url = client
            .build_url(
                &["v1", "search"],
                &[
                    ("q", r"john\doe".into()),
                    ("slashes", r"\\".into()),
                    ("mixed", r"one\\two\three".into()),
                ],
            )
            .expect("url");

        assert!(
            !url.as_str().contains("%5C"),
            "url should not contain encoded backslashes: {url}"
        );

        let query_pairs = url.query_pairs().collect::<Vec<_>>();
        assert_eq!(
            query_pairs,
            vec![
                ("q".into(), "john doe".into()),
                ("slashes".into(), "".into()),
                ("mixed".into(), "one two three".into()),
            ]
        );
    }

    #[test]
    fn track_with_feed_defaults_inherits_missing_source_metadata() {
        let track = Track {
            track_guid: Some("track-guid".into()),
            ..Default::default()
        };
        let feed = Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("Feed".into()),
            publisher_text: Some("publisher".into()),
            description: Some("description".into()),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1test".into()),
                ..Default::default()
            }]),
            source_contributors: Some(vec![Contributor {
                name: Some("Alice".into()),
                role: Some("musician".into()),
                ..Default::default()
            }]),
            payment_routes: Some(vec![PaymentRoute {
                recipient_name: Some("Alice".into()),
                split: Some(100.0),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let hydrated = super::track_with_feed_defaults(track, Some(&feed));
        assert_eq!(hydrated.feed_guid.as_deref(), Some("feed-guid"));
        assert_eq!(hydrated.feed_title.as_deref(), Some("Feed"));
        assert_eq!(hydrated.publisher_text.as_deref(), Some("publisher"));
        assert_eq!(hydrated.description.as_deref(), Some("description"));
        assert_eq!(hydrated.source_ids.as_ref().map(Vec::len), Some(1));
        assert_eq!(hydrated.source_contributors.as_ref().map(Vec::len), Some(1));
        assert_eq!(hydrated.payment_routes.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn contributor_deserializes_identity_fields() {
        let contributor: Contributor = serde_json::from_str(
            r#"{
                "name": "Alice",
                "role": "vocals",
                "group_name": "Band",
                "href": "https://example.com/alice",
                "img": "https://example.com/alice.jpg",
                "npub": "npub1alice"
            }"#,
        )
        .expect("contributor identity JSON should deserialize");

        assert_eq!(contributor.name.as_deref(), Some("Alice"));
        assert_eq!(contributor.role.as_deref(), Some("vocals"));
        assert_eq!(contributor.group_name.as_deref(), Some("Band"));
        assert_eq!(
            contributor.href.as_deref(),
            Some("https://example.com/alice")
        );
        assert_eq!(
            contributor.img.as_deref(),
            Some("https://example.com/alice.jpg")
        );
        assert_eq!(contributor.npub.as_deref(), Some("npub1alice"));
    }

    #[test]
    fn feed_and_track_deserialize_current_musicindex_identity_aliases() {
        let feed: Feed = serde_json::from_str(
            r#"{
                "feed_guid": "feed-1",
                "owner_name": "Album Artist",
                "persons": [{"name": "Alice", "role": "vocals"}],
                "links": [{"link_type": "website", "url": "https://example.com"}],
                "entity_ids": [{"scheme": "nostr_npub", "value": "npub1feed"}]
            }"#,
        )
        .expect("feed aliases should deserialize");
        let track: Track = serde_json::from_str(
            r#"{
                "track_guid": "track-1",
                "author_name": "Track Artist",
                "artist_credit": {
                    "artist_id": "artist-123",
                    "display_name": "Track Artist"
                },
                "persons": [{"name": "Bob", "role": "guitar"}],
                "links": [{"link_type": "website", "url": "https://example.com/track"}],
                "entity_ids": [{"scheme": "nostr_npub", "value": "npub1track"}]
            }"#,
        )
        .expect("track aliases should deserialize");

        assert_eq!(feed.release_artist.as_deref(), Some("Album Artist"));
        assert_eq!(feed.source_contributors.as_ref().map(Vec::len), Some(1));
        assert_eq!(feed.source_links.as_ref().map(Vec::len), Some(1));
        assert_eq!(feed.source_ids.as_ref().map(Vec::len), Some(1));
        assert_eq!(track.track_artist.as_deref(), Some("Track Artist"));
        let artist_credit = track
            .artist_credit
            .expect("artist_credit should deserialize");
        assert_eq!(artist_credit.artist_id.as_deref(), Some("artist-123"));
        assert_eq!(artist_credit.display_name.as_deref(), Some("Track Artist"));
        assert_eq!(track.source_contributors.as_ref().map(Vec::len), Some(1));
        assert_eq!(track.source_links.as_ref().map(Vec::len), Some(1));
        assert_eq!(track.source_ids.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn live_metadata_publish_url_uses_event_id_as_routing_key() {
        let client = Client::new();
        let url = client
            .build_url(&["v1", "liveitems", "event/one", "metadata"], &[])
            .expect("url");

        let segments = url
            .path_segments()
            .expect("path segments")
            .collect::<Vec<_>>();
        assert_eq!(segments, vec!["v1", "liveitems", "event%2Fone", "metadata"]);
    }

    #[test]
    fn live_metadata_request_requires_matching_event_id() {
        let request = LiveMetadataPublishRequest {
            event_id: "event-two".into(),
            metadata: serde_json::json!({"title": "Song"}),
        };

        let error = super::validate_live_metadata_request("event-one", &request)
            .expect_err("mismatched event ids should fail");
        assert!(
            error.to_string().contains("event_id mismatch"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn live_metadata_request_rejects_collapsed_event_id() {
        let request = LiveMetadataPublishRequest {
            event_id: "event one".into(),
            metadata: serde_json::json!({"title": "Song"}),
        };

        let error = super::validate_live_metadata_request("event\none", &request)
            .expect_err("collapsed path event id should fail");
        assert!(
            error.to_string().contains("invalid whitespace"),
            "unexpected error: {error}"
        );
    }
}
