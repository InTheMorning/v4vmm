use std::fmt;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client as ReqwestClient;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "https://api.musicindex.org";
pub const PAGE_LIMIT: i32 = 20;

/// Facts retained when an event metadata request cannot supply an answer (ADR 0059).
#[derive(Debug)]
pub(crate) struct LiveMetadataReadError {
    pub(crate) response_status: Option<u16>,
    detail: String,
}

impl fmt::Display for LiveMetadataReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for LiveMetadataReadError {}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_platforms: Option<Vec<SourcePlatformClaim>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_items: Option<Vec<RemoteItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Vec<PublisherRelationship>>,
    pub payment_routes: Option<Vec<PaymentRoute>>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Track {
    pub track_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_artist_sort: Option<String>,
    pub release_artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_transcripts: Option<Vec<SourceTranscript>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_items: Option<Vec<RemoteItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Vec<PublisherRelationship>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_time_splits: Option<Vec<ValueTimeSplit>>,
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

/// One contributor claim from a source observation (ADR 0075).
///
/// The seven provenance fields keep the owner, position, normalized role,
/// assertion source, extraction path and observation time that the API supplies.
/// They stay `None` for an older payload and for a locally constructed credit.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Contributor {
    pub name: Option<String>,
    pub role: Option<String>,
    pub href: Option<String>,
    pub img: Option<String>,
    pub npub: Option<String>,
    pub group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_norm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<i64>,
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

/// One media enclosure claim from a source observation (ADR 0075).
///
/// The four new fields keep the declared owner, position and observation
/// time that the API supplies. They stay `None` for an older payload and
/// for a locally constructed enclosure.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<i64>,
}

/// One transcript claim from a source observation (ADR 0075).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SourceTranscript {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    pub url: Option<String>,
    pub mime_type: Option<String>,
    pub language: Option<String>,
    pub rel: Option<String>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<i64>,
}

/// One platform claim from a source observation (ADR 0075).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SourcePlatformClaim {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<i64>,
}

/// One remote item reference from an API response (ADR 0075).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RemoteItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_feed_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_feed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// One publisher relationship from an API response (ADR 0075).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PublisherRelationship {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_feed_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_feed_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_feed_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_feed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_feed_medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_feed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_feed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reciprocal_declared: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reciprocal_medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_way_validated: Option<bool>,
}

/// One value time split from an API response (ADR 0075).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ValueTimeSplit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time_secs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_feed_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_item_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<i64>,
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

#[derive(Clone, Serialize, Deserialize)]
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

impl fmt::Debug for LiveItemCreateResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LiveItemCreateResponse")
            .field("event_id", &self.event_id)
            .field("broadcaster_token", &"<redacted>")
            .field("metadata_url", &self.metadata_url)
            .field("remote_value_url", &self.remote_value_url)
            .field("events_url", &self.events_url)
            .field("socket_io_url", &self.socket_io_url)
            .finish()
    }
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
    base_url: crate::config::MusicIndexEndpoint,
    observation_recorder:
        Option<std::sync::Arc<crate::provider_observation::ProviderObservationRecorder>>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            client: crate::http_client::document(),
            base_url: DEFAULT_BASE_URL.into(),
            observation_recorder: None,
        }
    }

    pub fn new_with_base_url(base_url: impl Into<crate::config::MusicIndexEndpoint>) -> Self {
        Self {
            client: crate::http_client::document(),
            base_url: base_url.into(),
            observation_recorder: None,
        }
    }

    pub(crate) fn with_observation_recorder(
        mut self,
        recorder: Option<std::sync::Arc<crate::provider_observation::ProviderObservationRecorder>>,
    ) -> Self {
        self.observation_recorder = recorder;
        self
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
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|error| LiveMetadataReadError {
                response_status: None,
                detail: format!("{:#}", anyhow::Error::new(error)),
            })?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let status = response.status().as_u16();
        response_json(response, "GET").map(Some).map_err(|error| {
            LiveMetadataReadError {
                response_status: Some(status),
                detail: format!("{error:#}"),
            }
            .into()
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
        let response: DetailResponse<T> = if let Some(recorder) = &self.observation_recorder {
            self.get_observed_json(path_segments, query, recorder)?
        } else {
            self.get_json(path_segments, query)?
        };
        Ok(response.data)
    }

    fn get_observed_json<T: DeserializeOwned>(
        &self,
        path: &[&str],
        query: &[(&str, String)],
        recorder: &crate::provider_observation::ProviderObservationRecorder,
    ) -> Result<T> {
        use crate::provider_observation::{
            http, musicindex, ObservationOutcome, ProviderKind, ProviderRequestSpec, SubjectKey,
        };
        let subject = match path {
            ["v1", "feeds", feed] => Some(SubjectKey::guid(feed, None)),
            ["v1", "feeds", feed, "tracks", track] => Some(SubjectKey::guid(feed, Some(track))),
            ["v1", "tracks", _] => None,
            _ => return Err(anyhow!("Unsupported observed metadata request")),
        };
        let url = self.build_url(path, query)?;
        let spec = ProviderRequestSpec {
            provider: ProviderKind::MusicIndex,
            provider_identity: self.base_url.require()?.into(),
            request_uri: url.to_string(),
            requested_subject: subject,
            requested_parameters: serde_json::json!({"path":path,"query":query}),
            profile: serde_json::json!({"version":1,"path":path,"query":query}),
            started_at_us: chrono::Utc::now().timestamp_micros(),
        };
        let token = recorder.begin(spec.clone())?;
        let mut observation = http::capture(self.client.get(url), "musicindex-json-v1");
        let decoded_text = http::decode_text(&mut observation);
        let decoded = if observation.outcome == ObservationOutcome::Failed {
            Err(anyhow!("Metadata request failed"))
        } else if observation.http_status == Some(204) {
            Err(anyhow!("GET returned no content"))
        } else {
            decode_json(&decoded_text, "GET")
        };
        if decoded.is_err() && observation.outcome != ObservationOutcome::Failed {
            observation.fail("json_decode");
        }
        musicindex::extract(&mut observation, &spec, &decoded_text);
        recorder.record(token, observation)?;
        decoded.map_err(|_| anyhow!("Metadata response could not supply the requested data"))
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
        let url = self.build_url(path_segments, &[])?;
        let response = self.client.post(url).json(body).send()?;
        response_json(response, "POST")
    }

    fn build_url(&self, path_segments: &[&str], query: &[(&str, String)]) -> Result<reqwest::Url> {
        let base_url = self.base_url.require()?;
        let mut url = reqwest::Url::parse(&format!("{}/", base_url.trim_end_matches('/')))?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| anyhow!("MusicIndex endpoint cannot be a base URL"))?;
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
    decode_json(&body, method)
}

fn decode_json<T: DeserializeOwned>(body: &str, method: &str) -> Result<T> {
    serde_json::from_str(body).with_context(|| format!("decode {method} JSON response"))
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

#[cfg(test)]
mod tests {

    #[test]
    fn adr_0066_invalid_endpoint_rejects_requests_before_transport() {
        let client =
            Client::new_with_base_url("https://secret:credential@invalid host/?token=hidden");
        let error = client
            .search("local", None, None, None, false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("musicindex_endpoint"));
        for secret in ["secret", "credential", "hidden"] {
            assert!(!error.contains(secret));
        }
    }

    use super::{
        Client, Contributor, Feed, PaymentRoute, SourceEnclosure, SourceEntityId, SourceTranscript,
        Track,
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
    fn live_metadata_read_url_uses_event_id_as_routing_key() {
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

    /// The operator's selected MusicIndex response from 2026-09-19 (ADR 0075 packet 001).
    /// Keep the three credits and their values unchanged.
    const ADR_0075_SUPPLIED_RESPONSE: &str = r#"{
  "title": "MoeFactz",
  "source_links": [],
  "source_ids": [],
  "source_contributors": [
    {
      "entity_type": "track",
      "entity_id": "d489101a-4e62-492f-812e-9fe51def9423",
      "position": 0,
      "name": "HeyCitizen",
      "role": "musician",
      "role_norm": "musician",
      "group_name": "music",
      "href": null,
      "img": "https://files.heycitizen.xyz/Songs/HeyCitizen.jpg",
      "npub": "npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck",
      "source": "podcast_person",
      "extraction_path": "track.podcast:person",
      "observed_at": 1779240280
    },
    {
      "entity_type": "track",
      "entity_id": "d489101a-4e62-492f-812e-9fe51def9423",
      "position": 1,
      "name": "HeyCitizen",
      "role": "audio engineer",
      "role_norm": "audio engineer",
      "group_name": "audio-production",
      "href": null,
      "img": "https://files.heycitizen.xyz/Songs/HeyCitizen.jpg",
      "npub": "npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck",
      "source": "podcast_person",
      "extraction_path": "track.podcast:person",
      "observed_at": 1779240280
    },
    {
      "entity_type": "track",
      "entity_id": "d489101a-4e62-492f-812e-9fe51def9423",
      "position": 2,
      "name": "Moe Factz",
      "role": "host",
      "role_norm": "host",
      "group_name": "cast",
      "href": "https://www.moefactz.com/",
      "img": null,
      "npub": null,
      "source": "podcast_person",
      "extraction_path": "track.podcast:person",
      "observed_at": 1779240280
    }
  ]
}"#;

    /// The seven claim fields that ADR 0075 packet 001 adds to the contributor DTO.
    const ADR_0075_PROVENANCE_KEYS: [&str; 7] = [
        "entity_type",
        "entity_id",
        "position",
        "role_norm",
        "source",
        "extraction_path",
        "observed_at",
    ];

    struct Adr0075ExpectedCredit {
        entity_type: &'static str,
        entity_id: &'static str,
        position: i64,
        name: &'static str,
        role: &'static str,
        role_norm: &'static str,
        group_name: &'static str,
        href: Option<&'static str>,
        img: Option<&'static str>,
        npub: Option<&'static str>,
        source: &'static str,
        extraction_path: &'static str,
        observed_at: i64,
    }

    const ADR_0075_HEYCITIZEN_IMG: &str = "https://files.heycitizen.xyz/Songs/HeyCitizen.jpg";
    const ADR_0075_HEYCITIZEN_NPUB: &str =
        "npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck";
    const ADR_0075_TRACK_ID: &str = "d489101a-4e62-492f-812e-9fe51def9423";
    const ADR_0075_OBSERVED_AT: i64 = 1_779_240_280;

    const ADR_0075_EXPECTED_CREDITS: [Adr0075ExpectedCredit; 3] = [
        Adr0075ExpectedCredit {
            entity_type: "track",
            entity_id: ADR_0075_TRACK_ID,
            position: 0,
            name: "HeyCitizen",
            role: "musician",
            role_norm: "musician",
            group_name: "music",
            href: None,
            img: Some(ADR_0075_HEYCITIZEN_IMG),
            npub: Some(ADR_0075_HEYCITIZEN_NPUB),
            source: "podcast_person",
            extraction_path: "track.podcast:person",
            observed_at: ADR_0075_OBSERVED_AT,
        },
        Adr0075ExpectedCredit {
            entity_type: "track",
            entity_id: ADR_0075_TRACK_ID,
            position: 1,
            name: "HeyCitizen",
            role: "audio engineer",
            role_norm: "audio engineer",
            group_name: "audio-production",
            href: None,
            img: Some(ADR_0075_HEYCITIZEN_IMG),
            npub: Some(ADR_0075_HEYCITIZEN_NPUB),
            source: "podcast_person",
            extraction_path: "track.podcast:person",
            observed_at: ADR_0075_OBSERVED_AT,
        },
        Adr0075ExpectedCredit {
            entity_type: "track",
            entity_id: ADR_0075_TRACK_ID,
            position: 2,
            name: "Moe Factz",
            role: "host",
            role_norm: "host",
            group_name: "cast",
            href: Some("https://www.moefactz.com/"),
            img: None,
            npub: None,
            source: "podcast_person",
            extraction_path: "track.podcast:person",
            observed_at: ADR_0075_OBSERVED_AT,
        },
    ];

    fn adr_0075_contributor_transport_supplied_values() -> Vec<serde_json::Value> {
        let response: serde_json::Value = serde_json::from_str(ADR_0075_SUPPLIED_RESPONSE)
            .expect("the supplied response should parse as JSON");
        response
            .get("source_contributors")
            .and_then(serde_json::Value::as_array)
            .expect("the supplied response should hold a contributor array")
            .clone()
    }

    fn adr_0075_contributor_transport_supplied_contributors() -> Vec<Contributor> {
        serde_json::from_value(serde_json::Value::Array(
            adr_0075_contributor_transport_supplied_values(),
        ))
        .expect("the supplied contributors should decode")
    }

    #[test]
    fn adr_0075_contributor_transport_decodes_every_supplied_field() {
        let contributors = adr_0075_contributor_transport_supplied_contributors();
        assert_eq!(contributors.len(), ADR_0075_EXPECTED_CREDITS.len());

        for (credit, expected) in contributors.iter().zip(&ADR_0075_EXPECTED_CREDITS) {
            assert_eq!(credit.entity_type.as_deref(), Some(expected.entity_type));
            assert_eq!(credit.entity_id.as_deref(), Some(expected.entity_id));
            assert_eq!(credit.position, Some(expected.position));
            assert_eq!(credit.name.as_deref(), Some(expected.name));
            assert_eq!(credit.role.as_deref(), Some(expected.role));
            assert_eq!(credit.role_norm.as_deref(), Some(expected.role_norm));
            assert_eq!(credit.group_name.as_deref(), Some(expected.group_name));
            assert_eq!(credit.href.as_deref(), expected.href);
            assert_eq!(credit.img.as_deref(), expected.img);
            assert_eq!(credit.npub.as_deref(), expected.npub);
            assert_eq!(credit.source.as_deref(), Some(expected.source));
            assert_eq!(
                credit.extraction_path.as_deref(),
                Some(expected.extraction_path)
            );
            assert_eq!(credit.observed_at, Some(expected.observed_at));
        }
    }

    #[test]
    fn adr_0075_contributor_transport_keeps_existing_identity_fields() {
        let contributors = adr_0075_contributor_transport_supplied_contributors();

        assert_eq!(contributors[0].href, None);
        assert_eq!(
            contributors[0].img.as_deref(),
            Some(ADR_0075_HEYCITIZEN_IMG)
        );
        assert_eq!(
            contributors[0].npub.as_deref(),
            Some(ADR_0075_HEYCITIZEN_NPUB)
        );
        assert_eq!(
            contributors[2].href.as_deref(),
            Some("https://www.moefactz.com/")
        );
        assert_eq!(contributors[2].img, None);
        assert_eq!(contributors[2].npub, None);
    }

    #[test]
    fn adr_0075_contributor_transport_serializes_each_supplied_credit_unchanged() {
        let values = adr_0075_contributor_transport_supplied_values();
        let contributors = adr_0075_contributor_transport_supplied_contributors();

        for (contributor, expected) in contributors.iter().zip(&values) {
            let serialized =
                serde_json::to_value(contributor).expect("the contributor should serialize");
            // A JSON object comparison ignores key order.
            assert_eq!(&serialized, expected);
        }
    }

    #[test]
    fn adr_0075_contributor_transport_second_decode_keeps_values_and_order() {
        let values = adr_0075_contributor_transport_supplied_values();
        let first = adr_0075_contributor_transport_supplied_contributors();
        let text = serde_json::to_string(&first).expect("the credits should serialize");
        let second: Vec<Contributor> =
            serde_json::from_str(&text).expect("the credits should decode a second time");

        assert_eq!(
            second
                .iter()
                .map(|credit| credit.position)
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1), Some(2)]
        );
        assert_eq!(
            serde_json::to_value(&second).expect("the second decode should serialize"),
            serde_json::Value::Array(values)
        );
    }

    #[test]
    fn adr_0075_contributor_transport_keeps_repeated_name_with_separate_roles() {
        let contributors = adr_0075_contributor_transport_supplied_contributors();
        let heycitizen = contributors
            .iter()
            .filter(|credit| credit.name.as_deref() == Some("HeyCitizen"))
            .collect::<Vec<_>>();

        assert_eq!(heycitizen.len(), 2);
        assert_eq!(heycitizen[0].role.as_deref(), Some("musician"));
        assert_eq!(heycitizen[1].role.as_deref(), Some("audio engineer"));
        assert_eq!(heycitizen[0].role_norm.as_deref(), Some("musician"));
        assert_eq!(heycitizen[1].role_norm.as_deref(), Some("audio engineer"));
        assert_eq!(heycitizen[0].position, Some(0));
        assert_eq!(heycitizen[1].position, Some(1));
        assert_eq!(heycitizen[0].group_name.as_deref(), Some("music"));
        assert_eq!(
            heycitizen[1].group_name.as_deref(),
            Some("audio-production")
        );
    }

    #[test]
    fn adr_0075_contributor_transport_keeps_feed_owned_credit_in_a_track() {
        let track: Track = serde_json::from_str(
            r#"{
                "track_guid": "track-1",
                "feed_guid": "feed-1",
                "persons": [
                    {
                        "entity_type": "feed",
                        "entity_id": "feed-1",
                        "position": 0,
                        "name": "Alice",
                        "role": "musician",
                        "source": "podcast_person",
                        "extraction_path": "channel.podcast:person",
                        "observed_at": 1779240280
                    },
                    {
                        "entity_type": "track",
                        "entity_id": "track-1",
                        "position": 0,
                        "name": "Bob",
                        "role": "host",
                        "source": "podcast_person",
                        "extraction_path": "track.podcast:person",
                        "observed_at": 1779240281
                    }
                ]
            }"#,
        )
        .expect("the track fixture should decode");
        let text = serde_json::to_string(&track).expect("the track should serialize");
        let round_tripped: Track =
            serde_json::from_str(&text).expect("the track should decode again");

        let credits = round_tripped
            .source_contributors
            .expect("the track should keep its credits");
        assert_eq!(credits.len(), 2);
        assert_eq!(credits[0].entity_type.as_deref(), Some("feed"));
        assert_eq!(credits[0].entity_id.as_deref(), Some("feed-1"));
        assert_eq!(
            credits[0].extraction_path.as_deref(),
            Some("channel.podcast:person")
        );
        assert_eq!(credits[1].entity_type.as_deref(), Some("track"));
        assert_eq!(credits[1].entity_id.as_deref(), Some("track-1"));
    }

    #[test]
    fn adr_0075_contributor_transport_keeps_supplied_positions_and_input_order() {
        let contributors: Vec<Contributor> = serde_json::from_str(
            r#"[
                {"name": "Alice", "position": 7},
                {"name": "Bob", "position": 2},
                {"name": "Carol", "position": 19}
            ]"#,
        )
        .expect("the position fixture should decode");
        let text = serde_json::to_string(&contributors).expect("the credits should serialize");
        let round_tripped: Vec<Contributor> =
            serde_json::from_str(&text).expect("the credits should decode again");

        assert_eq!(
            round_tripped
                .iter()
                .map(|credit| (credit.name.clone(), credit.position))
                .collect::<Vec<_>>(),
            vec![
                (Some("Alice".to_string()), Some(7)),
                (Some("Bob".to_string()), Some(2)),
                (Some("Carol".to_string()), Some(19)),
            ]
        );
    }

    #[test]
    fn adr_0075_contributor_transport_decodes_an_older_payload_without_provenance() {
        let older = r#"{
            "name": "Alice",
            "role": "vocals",
            "group_name": "Band",
            "href": "https://example.com/alice",
            "img": "https://example.com/alice.jpg",
            "npub": "npub1alice"
        }"#;
        let contributor: Contributor =
            serde_json::from_str(older).expect("the older payload should decode");

        assert_eq!(contributor.entity_type, None);
        assert_eq!(contributor.entity_id, None);
        assert_eq!(contributor.position, None);
        assert_eq!(contributor.role_norm, None);
        assert_eq!(contributor.source, None);
        assert_eq!(contributor.extraction_path, None);
        assert_eq!(contributor.observed_at, None);

        let serialized =
            serde_json::to_value(&contributor).expect("the older payload should serialize");
        let object = serialized
            .as_object()
            .expect("the credit should serialize as an object");
        for key in ADR_0075_PROVENANCE_KEYS {
            assert!(!object.contains_key(key), "serialization added {key}");
        }
        assert_eq!(object.len(), 6);
        assert_eq!(
            serialized,
            serde_json::from_str::<serde_json::Value>(older).expect("the older payload is JSON")
        );
    }

    #[test]
    fn adr_0075_contributor_transport_reads_explicit_null_provenance_as_absent() {
        let contributor: Contributor = serde_json::from_str(
            r#"{
                "name": "Alice",
                "role": "vocals",
                "group_name": "Band",
                "href": "https://example.com/alice",
                "img": "https://example.com/alice.jpg",
                "npub": "npub1alice",
                "entity_type": null,
                "entity_id": null,
                "position": null,
                "role_norm": null,
                "source": null,
                "extraction_path": null,
                "observed_at": null
            }"#,
        )
        .expect("explicit null provenance should decode");

        assert_eq!(contributor.entity_type, None);
        assert_eq!(contributor.entity_id, None);
        assert_eq!(contributor.position, None);
        assert_eq!(contributor.role_norm, None);
        assert_eq!(contributor.source, None);
        assert_eq!(contributor.extraction_path, None);
        assert_eq!(contributor.observed_at, None);

        let serialized = serde_json::to_value(&contributor).expect("the credit should serialize");
        let object = serialized
            .as_object()
            .expect("the credit should serialize as an object");
        for key in ADR_0075_PROVENANCE_KEYS {
            assert!(!object.contains_key(key), "serialization kept {key}");
        }
        assert_eq!(object.len(), 6);
        assert_eq!(contributor.name.as_deref(), Some("Alice"));
    }

    #[test]
    fn adr_0075_contributor_transport_keeps_unknown_entity_type_and_separate_role_norm() {
        let contributor: Contributor = serde_json::from_str(
            r#"{
                "entity_type": "collection",
                "entity_id": "collection-1",
                "name": "Alice",
                "role": "Lead Vocals",
                "role_norm": "vocals"
            }"#,
        )
        .expect("an unknown entity type should decode");
        let text = serde_json::to_string(&contributor).expect("the credit should serialize");
        let round_tripped: Contributor =
            serde_json::from_str(&text).expect("the credit should decode again");

        assert_eq!(round_tripped.entity_type.as_deref(), Some("collection"));
        assert_eq!(round_tripped.entity_id.as_deref(), Some("collection-1"));
        assert_eq!(round_tripped.role.as_deref(), Some("Lead Vocals"));
        assert_eq!(round_tripped.role_norm.as_deref(), Some("vocals"));
    }

    #[test]
    fn adr_0075_contributor_transport_rejects_a_string_position_or_observed_at() {
        let position_error =
            serde_json::from_str::<Contributor>(r#"{"name": "Alice", "position": "0"}"#);
        assert!(
            position_error.is_err(),
            "a string position should not decode"
        );

        let observed_at_error = serde_json::from_str::<Contributor>(
            r#"{"name": "Alice", "observed_at": "1779240280"}"#,
        );
        assert!(
            observed_at_error.is_err(),
            "a string observation time should not decode"
        );
    }

    /// ADR 0075 packet 030: the four provenance fields that
    /// `SourceEnclosure` adds for a MusicIndex media enclosure claim.
    mod adr_0075_enclosure_transport {
        use super::*;

        const ADR_0075_ENCLOSURE_PROVENANCE_KEYS: [&str; 4] =
            ["entity_type", "entity_id", "position", "observed_at"];

        const ADR_0075_ENCLOSURE_FIXTURE: &str = r#"{
            "entity_type": "track",
            "entity_id": "track-a",
            "position": 7,
            "url": "https://example.test/audio.mp3",
            "mime_type": "audio/mpeg",
            "bytes": 12345,
            "rel": null,
            "title": "Primary audio",
            "is_primary": true,
            "source": "rss_enclosure",
            "extraction_path": "item.enclosure",
            "observed_at": 1779240280
        }"#;

        #[test]
        fn decodes_and_serializes_the_fixture_without_losing_supplied_fields() {
            let enclosure: SourceEnclosure = serde_json::from_str(ADR_0075_ENCLOSURE_FIXTURE)
                .expect("the fixture should decode");

            assert_eq!(enclosure.entity_type.as_deref(), Some("track"));
            assert_eq!(enclosure.entity_id.as_deref(), Some("track-a"));
            assert_eq!(enclosure.position, Some(7));
            assert_eq!(
                enclosure.url.as_deref(),
                Some("https://example.test/audio.mp3")
            );
            assert_eq!(enclosure.mime_type.as_deref(), Some("audio/mpeg"));
            assert_eq!(enclosure.bytes, Some(12345));
            assert_eq!(enclosure.rel, None);
            assert_eq!(enclosure.title.as_deref(), Some("Primary audio"));
            assert_eq!(enclosure.is_primary, Some(true));
            assert_eq!(enclosure.source.as_deref(), Some("rss_enclosure"));
            assert_eq!(enclosure.extraction_path.as_deref(), Some("item.enclosure"));
            assert_eq!(enclosure.observed_at, Some(1_779_240_280));

            let serialized =
                serde_json::to_value(&enclosure).expect("the enclosure should serialize");
            let expected: serde_json::Value = serde_json::from_str(ADR_0075_ENCLOSURE_FIXTURE)
                .expect("the fixture should parse as JSON");
            // A JSON object comparison ignores key order.
            assert_eq!(serialized, expected);
        }

        #[test]
        fn keeps_supplied_positions_and_input_order() {
            let enclosures: Vec<SourceEnclosure> = serde_json::from_str(
                r#"[
                    {"url": "https://example.test/a.mp3", "position": 7},
                    {"url": "https://example.test/b.mp3", "position": 2},
                    {"url": "https://example.test/c.mp3", "position": 19}
                ]"#,
            )
            .expect("the position fixture should decode");
            let text = serde_json::to_string(&enclosures).expect("the enclosures should serialize");
            let round_tripped: Vec<SourceEnclosure> =
                serde_json::from_str(&text).expect("the enclosures should decode again");

            assert_eq!(
                round_tripped
                    .iter()
                    .map(|enclosure| (enclosure.url.clone(), enclosure.position))
                    .collect::<Vec<_>>(),
                vec![
                    (Some("https://example.test/a.mp3".to_string()), Some(7)),
                    (Some("https://example.test/b.mp3".to_string()), Some(2)),
                    (Some("https://example.test/c.mp3".to_string()), Some(19)),
                ]
            );
        }

        #[test]
        fn keeps_unknown_entity_type_without_substituting_the_track_owner() {
            let track: Track = serde_json::from_str(
                r#"{
                    "track_guid": "track-1",
                    "feed_guid": "feed-1",
                    "source_enclosures": [
                        {
                            "entity_type": "feed",
                            "entity_id": "feed-1",
                            "position": 0,
                            "url": "https://example.test/feed-level.mp3",
                            "source": "rss_enclosure",
                            "extraction_path": "channel.enclosure",
                            "observed_at": 1779240280
                        },
                        {
                            "entity_type": "collection",
                            "entity_id": "collection-9",
                            "position": 1,
                            "url": "https://example.test/collection-level.mp3",
                            "source": "rss_enclosure",
                            "extraction_path": "item.enclosure",
                            "observed_at": 1779240281
                        }
                    ]
                }"#,
            )
            .expect("the track fixture should decode");
            let text = serde_json::to_string(&track).expect("the track should serialize");
            let round_tripped: Track =
                serde_json::from_str(&text).expect("the track should decode again");

            let enclosures = round_tripped
                .source_enclosures
                .expect("the track should keep its enclosures");
            assert_eq!(enclosures.len(), 2);
            assert_eq!(enclosures[0].entity_type.as_deref(), Some("feed"));
            assert_eq!(enclosures[0].entity_id.as_deref(), Some("feed-1"));
            assert_eq!(enclosures[1].entity_type.as_deref(), Some("collection"));
            assert_eq!(enclosures[1].entity_id.as_deref(), Some("collection-9"));
        }

        #[test]
        fn decodes_an_older_payload_without_provenance() {
            let older = r#"{
                "url": "https://example.test/legacy.mp3",
                "mime_type": "audio/mpeg",
                "bytes": 999,
                "rel": "enclosure",
                "title": "Legacy audio",
                "is_primary": false,
                "source": "rss_enclosure",
                "extraction_path": "item.enclosure"
            }"#;
            let enclosure: SourceEnclosure =
                serde_json::from_str(older).expect("the older payload should decode");

            assert_eq!(enclosure.entity_type, None);
            assert_eq!(enclosure.entity_id, None);
            assert_eq!(enclosure.position, None);
            assert_eq!(enclosure.observed_at, None);

            let serialized =
                serde_json::to_value(&enclosure).expect("the older payload should serialize");
            let object = serialized
                .as_object()
                .expect("the enclosure should serialize as an object");
            for key in ADR_0075_ENCLOSURE_PROVENANCE_KEYS {
                assert!(!object.contains_key(key), "serialization added {key}");
            }
            assert_eq!(object.len(), 8);
            assert_eq!(
                serialized,
                serde_json::from_str::<serde_json::Value>(older)
                    .expect("the older payload is JSON")
            );
        }

        #[test]
        fn reads_explicit_null_provenance_as_absent() {
            let enclosure: SourceEnclosure = serde_json::from_str(
                r#"{
                    "url": "https://example.test/legacy.mp3",
                    "mime_type": "audio/mpeg",
                    "bytes": 999,
                    "rel": "enclosure",
                    "title": "Legacy audio",
                    "is_primary": false,
                    "source": "rss_enclosure",
                    "extraction_path": "item.enclosure",
                    "entity_type": null,
                    "entity_id": null,
                    "position": null,
                    "observed_at": null
                }"#,
            )
            .expect("explicit null provenance should decode");

            assert_eq!(enclosure.entity_type, None);
            assert_eq!(enclosure.entity_id, None);
            assert_eq!(enclosure.position, None);
            assert_eq!(enclosure.observed_at, None);

            let serialized =
                serde_json::to_value(&enclosure).expect("the enclosure should serialize");
            let object = serialized
                .as_object()
                .expect("the enclosure should serialize as an object");
            for key in ADR_0075_ENCLOSURE_PROVENANCE_KEYS {
                assert!(!object.contains_key(key), "serialization kept {key}");
            }
            assert_eq!(object.len(), 8);
            assert_eq!(
                enclosure.url.as_deref(),
                Some("https://example.test/legacy.mp3")
            );
        }

        #[test]
        fn rejects_a_string_position_or_observed_at() {
            let position_error = serde_json::from_str::<SourceEnclosure>(
                r#"{"url": "https://example.test/a.mp3", "position": "7"}"#,
            );
            assert!(
                position_error.is_err(),
                "a string position should not decode"
            );

            let observed_at_error = serde_json::from_str::<SourceEnclosure>(
                r#"{"url": "https://example.test/a.mp3", "observed_at": "1779240280"}"#,
            );
            assert!(
                observed_at_error.is_err(),
                "a string observation time should not decode"
            );
        }
    }

    mod adr_0075_transcript_transport {
        use super::*;

        const TRANSCRIPT_FIXTURE: &str = r#"{
            "track_guid": "track-1",
            "source_transcripts": [
                {
                    "entity_type": "track",
                    "entity_id": "track-1",
                    "position": 7,
                    "url": "https://example.test/transcript.vtt",
                    "mime_type": "text/vtt",
                    "language": "en",
                    "rel": "captions",
                    "source": "podcast_transcript",
                    "extraction_path": "item.podcast:transcript[0]",
                    "observed_at": 1779240280
                },
                {
                    "entity_type": "unrecognized_owner",
                    "entity_id": "source-9",
                    "position": 2,
                    "url": "https://example.test/transcript.vtt",
                    "mime_type": "text/plain",
                    "language": "fr",
                    "rel": "transcript",
                    "source": "rss_transcript",
                    "extraction_path": "item.transcript[1]",
                    "observed_at": 1779240281
                }
            ]
        }"#;

        #[test]
        fn adr_0075_transcript_transport_round_trips_every_field_and_occurrence() {
            let track: Track = serde_json::from_str(TRANSCRIPT_FIXTURE)
                .expect("the transcript fixture should decode");
            let transcripts = track
                .source_transcripts
                .as_ref()
                .expect("the track should keep its transcripts");

            assert_eq!(transcripts.len(), 2);
            assert_eq!(transcripts[0].entity_type.as_deref(), Some("track"));
            assert_eq!(transcripts[0].entity_id.as_deref(), Some("track-1"));
            assert_eq!(transcripts[0].position, Some(7));
            assert_eq!(
                transcripts[0].url.as_deref(),
                Some("https://example.test/transcript.vtt")
            );
            assert_eq!(transcripts[0].mime_type.as_deref(), Some("text/vtt"));
            assert_eq!(transcripts[0].language.as_deref(), Some("en"));
            assert_eq!(transcripts[0].rel.as_deref(), Some("captions"));
            assert_eq!(transcripts[0].source.as_deref(), Some("podcast_transcript"));
            assert_eq!(
                transcripts[0].extraction_path.as_deref(),
                Some("item.podcast:transcript[0]")
            );
            assert_eq!(transcripts[0].observed_at, Some(1_779_240_280));
            assert_eq!(
                transcripts[1].entity_type.as_deref(),
                Some("unrecognized_owner")
            );
            assert_eq!(transcripts[1].position, Some(2));
            assert_eq!(
                transcripts[1].url.as_deref(),
                Some("https://example.test/transcript.vtt")
            );

            let serialized = serde_json::to_value(&track).expect("the track should serialize");
            let expected: serde_json::Value = serde_json::from_str(TRANSCRIPT_FIXTURE)
                .expect("the transcript fixture should parse as JSON");
            assert_eq!(
                serialized["source_transcripts"],
                expected["source_transcripts"]
            );

            let text = serde_json::to_string(&track).expect("the track should serialize");
            let second: Track =
                serde_json::from_str(&text).expect("the track should decode a second time");
            let second_transcripts = second
                .source_transcripts
                .expect("the second track should keep its transcripts");
            assert_eq!(
                second_transcripts
                    .iter()
                    .map(|transcript| (transcript.url.clone(), transcript.position))
                    .collect::<Vec<_>>(),
                vec![
                    (
                        Some("https://example.test/transcript.vtt".to_string()),
                        Some(7)
                    ),
                    (
                        Some("https://example.test/transcript.vtt".to_string()),
                        Some(2)
                    ),
                ]
            );
        }

        #[test]
        fn adr_0075_transcript_transport_omits_a_missing_collection_when_serializing() {
            let track: Track = serde_json::from_str(r#"{"track_guid": "track-1"}"#)
                .expect("a track without transcripts should decode");
            assert!(track.source_transcripts.is_none());

            let serialized = serde_json::to_value(track).expect("the track should serialize");
            assert!(
                !serialized
                    .as_object()
                    .expect("the track should serialize as an object")
                    .contains_key("source_transcripts"),
                "a missing collection should stay absent"
            );
        }

        #[test]
        fn adr_0075_transcript_transport_omits_a_null_collection_when_serializing() {
            let track: Track =
                serde_json::from_str(r#"{"track_guid": "track-1", "source_transcripts": null}"#)
                    .expect("a null collection should decode");
            assert!(track.source_transcripts.is_none());

            let serialized = serde_json::to_value(track).expect("the track should serialize");
            assert!(
                !serialized
                    .as_object()
                    .expect("the track should serialize as an object")
                    .contains_key("source_transcripts"),
                "a null collection should become absent"
            );
        }

        #[test]
        fn adr_0075_transcript_transport_keeps_an_empty_collection() {
            let track: Track =
                serde_json::from_str(r#"{"track_guid": "track-1", "source_transcripts": []}"#)
                    .expect("an empty collection should decode");
            assert!(track.source_transcripts.as_ref().is_some_and(Vec::is_empty));

            let serialized = serde_json::to_value(track).expect("the track should serialize");
            assert_eq!(serialized["source_transcripts"], serde_json::json!([]));
        }

        #[test]
        fn adr_0075_transcript_transport_keeps_an_older_member_without_provenance() {
            let transcript: SourceTranscript = serde_json::from_str(
                r#"{
                    "url": "https://example.test/legacy.txt"
                }"#,
            )
            .expect("an older transcript should decode");

            assert_eq!(transcript.entity_type, None);
            assert_eq!(transcript.entity_id, None);
            assert_eq!(transcript.position, None);
            assert_eq!(transcript.observed_at, None);

            let serialized =
                serde_json::to_value(transcript).expect("the transcript should serialize");
            let object = serialized
                .as_object()
                .expect("the transcript should serialize as an object");
            for key in ["entity_type", "entity_id", "position", "observed_at"] {
                assert!(!object.contains_key(key), "serialization added {key}");
            }
            assert_eq!(object.len(), 6);
            assert_eq!(object["url"], "https://example.test/legacy.txt");
            for key in ["mime_type", "language", "rel", "source", "extraction_path"] {
                assert!(object[key].is_null(), "serialization omitted {key}");
            }
        }

        #[test]
        fn adr_0075_transcript_transport_rejects_string_integers() {
            let position_error =
                serde_json::from_str::<Track>(r#"{"source_transcripts": [{"position": "7"}]}"#);
            assert!(
                position_error.is_err(),
                "a string position should not decode"
            );

            let observed_at_error = serde_json::from_str::<Track>(
                r#"{"source_transcripts": [{"observed_at": "1779240280"}]}"#,
            );
            assert!(
                observed_at_error.is_err(),
                "a string observation time should not decode"
            );
        }

        #[test]
        fn adr_0075_transcript_transport_adds_no_facts_during_feed_defaulting() {
            let track = Track {
                track_guid: Some("track-1".into()),
                ..Default::default()
            };
            let feed = Feed {
                feed_guid: Some("feed-1".into()),
                ..Default::default()
            };

            let hydrated = crate::api::track_with_feed_defaults(track, Some(&feed));
            assert!(hydrated.source_transcripts.is_none());
        }
    }

    mod adr_0075_track_scalar_transport {
        use super::*;

        const SCALAR_FIXTURE: &str = r#"{
            "track_guid": "track-1",
            "language": "x-unknown-language",
            "track_artist_sort": "Ångström, The"
        }"#;

        #[test]
        fn adr_0075_track_scalar_transport_round_trips_supplied_text() {
            let track: Track =
                serde_json::from_str(SCALAR_FIXTURE).expect("the scalar fixture should decode");

            assert_eq!(track.language.as_deref(), Some("x-unknown-language"));
            assert_eq!(track.track_artist_sort.as_deref(), Some("Ångström, The"));

            let serialized = serde_json::to_value(&track).expect("the track should serialize");
            let expected: serde_json::Value =
                serde_json::from_str(SCALAR_FIXTURE).expect("the scalar fixture should parse");
            assert_eq!(serialized["language"], expected["language"]);
            assert_eq!(
                serialized["track_artist_sort"],
                expected["track_artist_sort"]
            );

            let text = serde_json::to_string(&track).expect("the track should serialize");
            let second: Track =
                serde_json::from_str(&text).expect("the track should decode a second time");
            assert_eq!(second.language.as_deref(), Some("x-unknown-language"));
            assert_eq!(second.track_artist_sort.as_deref(), Some("Ångström, The"));
        }

        #[test]
        fn adr_0075_track_scalar_transport_keeps_empty_text() {
            let track: Track = serde_json::from_str(r#"{"language": "", "track_artist_sort": ""}"#)
                .expect("empty scalar text should decode");

            assert_eq!(track.language.as_deref(), Some(""));
            assert_eq!(track.track_artist_sort.as_deref(), Some(""));

            let serialized = serde_json::to_value(track).expect("the track should serialize");
            assert_eq!(serialized["language"], "");
            assert_eq!(serialized["track_artist_sort"], "");
        }

        #[test]
        fn adr_0075_track_scalar_transport_omits_older_and_missing_fields() {
            let older: Track = serde_json::from_str(r#"{"track_guid": "legacy-track"}"#)
                .expect("an older track should decode");
            assert!(older.language.is_none());
            assert!(older.track_artist_sort.is_none());

            let older_serialized = serde_json::to_value(older).expect("the track should serialize");
            let older_object = older_serialized
                .as_object()
                .expect("the track should serialize as an object");
            for key in ["language", "track_artist_sort"] {
                assert!(!older_object.contains_key(key), "serialization added {key}");
            }

            let track: Track = serde_json::from_str(r#"{"language": "en"}"#)
                .expect("a track with one scalar should decode");
            assert_eq!(track.language.as_deref(), Some("en"));
            assert!(track.track_artist_sort.is_none());

            let serialized = serde_json::to_value(track).expect("the track should serialize");
            assert_eq!(serialized["language"], "en");
            assert!(serialized.get("track_artist_sort").is_none());
        }

        #[test]
        fn adr_0075_track_scalar_transport_omits_null_fields() {
            let track: Track =
                serde_json::from_str(r#"{"language": null, "track_artist_sort": null}"#)
                    .expect("null scalar text should decode");
            assert!(track.language.is_none());
            assert!(track.track_artist_sort.is_none());

            let serialized = serde_json::to_value(track).expect("the track should serialize");
            let object = serialized
                .as_object()
                .expect("the track should serialize as an object");
            for key in ["language", "track_artist_sort"] {
                assert!(!object.contains_key(key), "serialization kept {key}");
            }
        }

        #[test]
        fn adr_0075_track_scalar_transport_adds_no_facts_during_feed_defaulting() {
            let track = Track {
                track_guid: Some("track-1".into()),
                ..Default::default()
            };
            let feed = Feed {
                language: Some("fr".into()),
                release_artist_sort: Some("Artist, Feed".into()),
                ..Default::default()
            };

            let hydrated = crate::api::track_with_feed_defaults(track, Some(&feed));
            assert!(hydrated.language.is_none());
            assert!(hydrated.track_artist_sort.is_none());
        }
    }

    mod adr_0075_remaining_transport {
        use super::*;

        const FEED_FIXTURE: &str = r#"{
            "feed_guid": "feed-1",
            "created_at": 2147483648,
            "source_platforms": [{
                "platform_key": "unrecognized-platform",
                "url": "",
                "owner_name": "",
                "source": "future-source",
                "extraction_path": "channel.platform[0]",
                "observed_at": 2147483649
            }],
            "remote_items": [
                {
                    "position": 7,
                    "medium": "future-medium",
                    "remote_feed_guid": "remote-feed-1",
                    "remote_feed_url": "",
                    "source": "remote-item"
                },
                {
                    "position": 2,
                    "remote_feed_guid": "remote-feed-1",
                    "source": "remote-item"
                }
            ],
            "publisher": [{
                "direction": "unrecognized-direction",
                "remote_feed_guid": "remote-feed-1",
                "publisher_feed_guid": "publisher-feed-1",
                "music_feed_guid": "music-feed-1",
                "remote_feed_url": "",
                "remote_feed_medium": "future-medium",
                "publisher_feed_url": "",
                "music_feed_url": "",
                "reciprocal_declared": false,
                "reciprocal_medium": "future-reciprocal-medium",
                "two_way_validated": false
            }]
        }"#;

        const TRACK_FIXTURE: &str = r#"{
            "track_guid": "track-1",
            "created_at": 2147483650,
            "remote_items": [
                {
                    "position": 5,
                    "medium": "future-medium",
                    "remote_feed_guid": "remote-feed-2",
                    "remote_feed_url": "https://example.test/remote",
                    "source": "remote-item"
                },
                {
                    "position": 1,
                    "medium": "future-medium",
                    "remote_feed_guid": "remote-feed-2",
                    "remote_feed_url": "https://example.test/remote",
                    "source": "remote-item"
                }
            ],
            "publisher": [
                {
                    "direction": "unrecognized-direction",
                    "remote_feed_guid": "remote-feed-2",
                    "publisher_feed_guid": "publisher-feed-2",
                    "music_feed_guid": "music-feed-2",
                    "remote_feed_url": "https://example.test/remote",
                    "remote_feed_medium": "future-medium",
                    "publisher_feed_url": "https://example.test/publisher",
                    "music_feed_url": "https://example.test/music",
                    "reciprocal_declared": false,
                    "reciprocal_medium": "future-reciprocal-medium",
                    "two_way_validated": false
                },
                {
                    "direction": "unrecognized-direction",
                    "remote_feed_guid": "remote-feed-3",
                    "publisher_feed_guid": "publisher-feed-3",
                    "music_feed_guid": "music-feed-3"
                }
            ],
            "value_time_splits": [
                {
                    "start_time_secs": 2147483651,
                    "remote_feed_guid": "remote-feed-2",
                    "remote_item_guid": "remote-item-1",
                    "split": -50
                },
                {
                    "start_time_secs": 0,
                    "duration_secs": 0,
                    "remote_feed_guid": "remote-feed-2",
                    "remote_item_guid": "remote-item-2",
                    "split": 0
                }
            ]
        }"#;

        #[test]
        fn adr_0075_remaining_transport_round_trips_feed_fields() {
            let feed: Feed =
                serde_json::from_str(FEED_FIXTURE).expect("the feed fixture should decode");
            assert_eq!(feed.created_at, Some(2_147_483_648));
            assert_eq!(feed.remote_items.as_ref().map(Vec::len), Some(2));
            assert_eq!(
                feed.remote_items.as_ref().map(|items| items[0].position),
                Some(Some(7))
            );
            assert_eq!(
                feed.remote_items.as_ref().map(|items| items[1].position),
                Some(Some(2))
            );
            assert_eq!(
                feed.publisher
                    .as_ref()
                    .map(|relationships| relationships[0].reciprocal_declared),
                Some(Some(false))
            );
            assert_eq!(
                feed.publisher
                    .as_ref()
                    .map(|relationships| relationships[0].two_way_validated),
                Some(Some(false))
            );

            let serialized = serde_json::to_value(&feed).expect("the feed should serialize");
            let expected: serde_json::Value =
                serde_json::from_str(FEED_FIXTURE).expect("the feed fixture should parse");
            for key in [
                "created_at",
                "source_platforms",
                "remote_items",
                "publisher",
            ] {
                assert_eq!(serialized[key], expected[key], "the feed should keep {key}");
            }

            let text = serde_json::to_string(&feed).expect("the feed should serialize");
            let second: Feed =
                serde_json::from_str(&text).expect("the feed should decode a second time");
            let second_serialized =
                serde_json::to_value(second).expect("the second feed should serialize");
            for key in [
                "created_at",
                "source_platforms",
                "remote_items",
                "publisher",
            ] {
                assert_eq!(
                    second_serialized[key], expected[key],
                    "the second feed lost {key}"
                );
            }
        }

        #[test]
        fn adr_0075_remaining_transport_round_trips_track_fields() {
            let track: Track =
                serde_json::from_str(TRACK_FIXTURE).expect("the track fixture should decode");
            assert_eq!(track.created_at, Some(2_147_483_650));
            assert_eq!(track.remote_items.as_ref().map(Vec::len), Some(2));
            assert_eq!(
                track
                    .value_time_splits
                    .as_ref()
                    .map(|splits| splits[0].duration_secs),
                Some(None)
            );
            assert_eq!(
                track
                    .value_time_splits
                    .as_ref()
                    .map(|splits| splits[1].duration_secs),
                Some(Some(0))
            );
            assert_eq!(
                track
                    .publisher
                    .as_ref()
                    .map(|relationships| relationships[1].reciprocal_declared),
                Some(None)
            );
            assert_eq!(
                track
                    .publisher
                    .as_ref()
                    .map(|relationships| relationships[1].two_way_validated),
                Some(None)
            );

            let serialized = serde_json::to_value(&track).expect("the track should serialize");
            let expected: serde_json::Value =
                serde_json::from_str(TRACK_FIXTURE).expect("the track fixture should parse");
            for key in [
                "created_at",
                "remote_items",
                "publisher",
                "value_time_splits",
            ] {
                assert_eq!(
                    serialized[key], expected[key],
                    "the track should keep {key}"
                );
            }

            let text = serde_json::to_string(&track).expect("the track should serialize");
            let second: Track =
                serde_json::from_str(&text).expect("the track should decode a second time");
            let second_serialized =
                serde_json::to_value(second).expect("the second track should serialize");
            for key in [
                "created_at",
                "remote_items",
                "publisher",
                "value_time_splits",
            ] {
                assert_eq!(
                    second_serialized[key], expected[key],
                    "the second track lost {key}"
                );
            }
        }

        #[test]
        fn adr_0075_remaining_transport_omits_missing_and_null_fields_and_keeps_empty_collections()
        {
            let missing: Feed = serde_json::from_str(r#"{"feed_guid": "feed-1"}"#)
                .expect("a feed without new fields should decode");
            let missing_serialized =
                serde_json::to_value(missing).expect("the feed should serialize");
            let missing_object = missing_serialized
                .as_object()
                .expect("the feed should serialize as an object");
            for key in [
                "created_at",
                "source_platforms",
                "remote_items",
                "publisher",
            ] {
                assert!(
                    !missing_object.contains_key(key),
                    "serialization added {key}"
                );
            }

            let nulls: Track = serde_json::from_str(
                r#"{
                    "created_at": null,
                    "remote_items": null,
                    "publisher": null,
                    "value_time_splits": null
                }"#,
            )
            .expect("null track fields should decode");
            let null_serialized = serde_json::to_value(nulls).expect("the track should serialize");
            let null_object = null_serialized
                .as_object()
                .expect("the track should serialize as an object");
            for key in [
                "created_at",
                "remote_items",
                "publisher",
                "value_time_splits",
            ] {
                assert!(!null_object.contains_key(key), "serialization kept {key}");
            }

            let feed: Feed = serde_json::from_str(
                r#"{"source_platforms": [], "remote_items": [], "publisher": []}"#,
            )
            .expect("empty feed collections should decode");
            let track: Track = serde_json::from_str(
                r#"{"remote_items": [], "publisher": [], "value_time_splits": []}"#,
            )
            .expect("empty track collections should decode");
            let feed_serialized = serde_json::to_value(feed).expect("the feed should serialize");
            let track_serialized = serde_json::to_value(track).expect("the track should serialize");
            for key in ["source_platforms", "remote_items", "publisher"] {
                assert_eq!(feed_serialized[key], serde_json::json!([]));
            }
            for key in ["remote_items", "publisher", "value_time_splits"] {
                assert_eq!(track_serialized[key], serde_json::json!([]));
            }
        }

        #[test]
        fn adr_0075_remaining_transport_rejects_string_numbers_and_booleans() {
            for payload in [
                r#"{"created_at": "2147483648"}"#,
                r#"{"source_platforms": [{"observed_at": "2147483649"}]}"#,
                r#"{"remote_items": [{"position": "7"}]}"#,
                r#"{"publisher": [{"reciprocal_declared": "false"}]}"#,
            ] {
                assert!(
                    serde_json::from_str::<Feed>(payload).is_err(),
                    "the feed should reject {payload}"
                );
            }

            for payload in [
                r#"{"created_at": "2147483650"}"#,
                r#"{"remote_items": [{"position": "5"}]}"#,
                r#"{"publisher": [{"two_way_validated": "false"}]}"#,
                r#"{"value_time_splits": [{"start_time_secs": "2147483651"}]}"#,
            ] {
                assert!(
                    serde_json::from_str::<Track>(payload).is_err(),
                    "the track should reject {payload}"
                );
            }
        }

        #[test]
        fn adr_0075_remaining_transport_adds_no_facts_during_feed_defaulting() {
            let feed: Feed =
                serde_json::from_str(FEED_FIXTURE).expect("the feed fixture should decode");
            let track = Track {
                track_guid: Some("track-1".into()),
                ..Default::default()
            };

            let hydrated = crate::api::track_with_feed_defaults(track, Some(&feed));
            assert!(hydrated.created_at.is_none());
            assert!(hydrated.remote_items.is_none());
            assert!(hydrated.publisher.is_none());
            assert!(hydrated.value_time_splits.is_none());
        }
    }
}
