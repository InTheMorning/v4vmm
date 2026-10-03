use std::fmt;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client as ReqwestClient;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::application::request_profiles::{RequestPathShape, RequestProfile};

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

/// One search hit's summary fields (ADR 0075 packet 047). The deployed
/// contract version `0.2.0` declares every field below on
/// `SearchResponseItem`. A hit omits each field whose value is null; an
/// omitted field decodes as `None`. `quality_score` is an integer in the
/// contract; `Option<f64>` still accepts it, because `serde_json` decodes
/// a JSON integer into an `f64` field without an error.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub rank: Option<f64>,
    pub quality_score: Option<f64>,
    pub title: Option<String>,
    pub feed_guid: Option<String>,
    pub feed_title: Option<String>,
    pub feed_image_url: Option<String>,
    pub track_image_url: Option<String>,
    pub release_artist: Option<String>,
    pub release_artist_source: Option<String>,
    pub track_artist: Option<String>,
    pub pub_date: Option<i64>,
    pub duration_secs: Option<i32>,
    pub episode_count: Option<i32>,
    pub href: Option<String>,
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
pub struct Feed {
    pub feed_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    pub title: Option<String>,
    pub feed_url: Option<String>,
    #[serde(alias = "owner_name")]
    pub release_artist: Option<String>,
    pub release_artist_sort: Option<String>,
    pub raw_medium: Option<String>,
    pub release_kind: Option<String>,
    pub release_date: Option<i64>,
    /// The RSS channel `pubDate`, read directly by this app's own RSS parse
    /// (ADR 0075 packet 050). This is a publication date, never a release
    /// date. The MusicIndex contract declares no such field: this app's own
    /// RSS enrichment is the only writer, so this field decodes nothing from
    /// a MusicIndex response and carries no raw JSON of its own.
    #[serde(skip)]
    pub channel_pub_date: Option<i64>,
    /// The original text of [`Self::channel_pub_date`], kept as evidence
    /// even when the text does not parse (ADR 0075 packet 050).
    #[serde(skip)]
    pub channel_pub_date_text: Option<String>,
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
    /// Each other publisher feed that shares a confirmed album with this
    /// feed (ADR 0082 Decision 5). Present only on a read of a publisher
    /// feed with `include=publisher`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co_credited_feeds: Option<Vec<CoCreditedFeed>>,
    /// The title of the publisher feed that this album names (ADR 0077 Decision 5).
    /// MusicIndex derives it. It is null when the album names no publisher.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_feed_title: Option<String>,
    /// The source of `release_artist`, as MusicIndex sends it (ADR 0077).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_artist_source: Option<String>,
    /// The count of distinct `release_artist` values among the albums that
    /// name this publisher and that this publisher also lists. MusicIndex
    /// derives it from Stophammer ADR 0061, and it is present only on a
    /// publisher feed (ADR 0077 Task 007). It never selects the page type
    /// (ADR 0078).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_release_artist_count: Option<i64>,
    /// One raw `release_artist` value for each count in
    /// `confirmed_release_artist_count`, in first-listing order (ADR 0077
    /// Task 007).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_release_artists: Option<Vec<String>>,
    /// The count of distinct `release_artist` values among the albums that
    /// this publisher lists but that do not name it. MusicIndex derives it
    /// from Stophammer ADR 0061 (ADR 0077 Task 007).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unconfirmed_release_artist_count: Option<i64>,
    /// One raw `release_artist` value for each count in
    /// `unconfirmed_release_artist_count`, in first-listing order (ADR 0077
    /// Task 007).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unconfirmed_release_artists: Option<Vec<String>>,
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
    pub title: Option<String>,
    pub duration_secs: Option<i32>,
    pub pub_date: Option<i64>,
    pub track_number: Option<i32>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
    pub enclosure_url: Option<String>,
    pub enclosure_type: Option<String>,
    pub enclosure_bytes: Option<i64>,
    /// The resolved image: the track's own image, else the feed image
    /// (ADR 0075 Decision C). A response with neither owner field below
    /// still carries this value, with unknown ownership (the accepted
    /// legacy-artwork rule, packet 048).
    pub image_url: Option<String>,
    /// The track's own image. `None` when the track has no image of its
    /// own, or when the response states no owner for either image field
    /// (Stophammer ADR 0042, ADR 0075 packet 048).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_image_url: Option<String>,
    /// The feed's image that this track response states, beside the
    /// track's own image (Stophammer ADR 0042, ADR 0075 packet 048).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed_image_url: Option<String>,
    #[serde(alias = "author_name")]
    pub track_artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_artist_sort: Option<String>,
    pub release_artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub publisher_text: Option<String>,
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

/// Backfills a track with its feed's values, for the fields whose fallback
/// stays outside display ownership (ADR 0075 Decision B, packet 022).
///
/// This function does not copy `source_links`, `source_ids` or
/// `description`. A track keeps only its own identity and its own
/// description; a caller that still wants the feed's website, Nostr key or
/// description reads the feed in its `TrackContext` directly
/// (`metadata::source_value_for_metadata_field` does this for a tag row,
/// and `TrackDetailVm::with_feed_identity` does this for the track page).
pub fn track_with_feed_defaults(mut track: Track, feed: Option<&Feed>) -> Track {
    if let Some(feed) = feed {
        if track.feed_guid.is_none() {
            track.feed_guid = feed.feed_guid.clone();
        }
        if track.feed_title.is_none() {
            track.feed_title = feed.title.clone();
        }
        if track.image_url.is_none() {
            track.image_url = feed.image_url.clone();
        }
        if track.publisher_text.is_none() {
            track.publisher_text = feed.publisher_text.clone();
        }
        if track.release_artist.is_none() {
            track.release_artist = feed.release_artist.clone();
        }
        if track.source_contributors.is_none() {
            track.source_contributors = feed.source_contributors.clone();
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

/// How MusicIndex resolved the publisher feed that carries
/// `publisher_lists_music` (ADR 0077 Decision 5).
///
/// An unrecognized value keeps its raw text. It does not fail the response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum PublisherLinkResolution {
    Guid,
    FeedUrl,
    Unresolved,
    Unknown(String),
}

impl PublisherLinkResolution {
    /// Returns the wire text of this value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Guid => "guid",
            Self::FeedUrl => "feed_url",
            Self::Unresolved => "unresolved",
            Self::Unknown(raw) => raw,
        }
    }
}

impl From<String> for PublisherLinkResolution {
    fn from(raw: String) -> Self {
        match raw.as_str() {
            "guid" => Self::Guid,
            "feed_url" => Self::FeedUrl,
            "unresolved" => Self::Unresolved,
            _ => Self::Unknown(raw),
        }
    }
}

impl From<PublisherLinkResolution> for String {
    fn from(value: PublisherLinkResolution) -> Self {
        match value {
            PublisherLinkResolution::Unknown(raw) => raw,
            known => known.as_str().to_owned(),
        }
    }
}

/// The source of the `role` of a publisher relationship (ADR 0077 Decision 3).
///
/// `Default` means that no feed states a role. An unrecognized value keeps
/// its raw text. It does not fail the response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum RoleSource {
    PublisherRel,
    MusicRel,
    Default,
    Conflict,
    Unknown(String),
}

impl RoleSource {
    /// Returns the wire text of this value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::PublisherRel => "publisher_rel",
            Self::MusicRel => "music_rel",
            Self::Default => "default",
            Self::Conflict => "conflict",
            Self::Unknown(raw) => raw,
        }
    }
}

impl From<String> for RoleSource {
    fn from(raw: String) -> Self {
        match raw.as_str() {
            "publisher_rel" => Self::PublisherRel,
            "music_rel" => Self::MusicRel,
            "default" => Self::Default,
            "conflict" => Self::Conflict,
            _ => Self::Unknown(raw),
        }
    }
}

impl From<RoleSource> for String {
    fn from(value: RoleSource) -> Self {
        match value {
            RoleSource::Unknown(raw) => raw,
            known => known.as_str().to_owned(),
        }
    }
}

/// How an album names a publisher feed (ADR 0082 Decision 2, Stophammer ADR
/// 0069 §3).
///
/// The 0.7.0 contract text still names `credit` beside `publisher`, though
/// the live node sends only `publisher` or null (ADR 0082 Recorded Facts,
/// 2026-10-02). This type keeps `credit` as a known value, because the
/// contract still declares it. An unrecognized value keeps its raw text.
/// It does not fail the response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum AlbumNamesAs {
    Publisher,
    Credit,
    Unknown(String),
}

impl AlbumNamesAs {
    /// Returns the wire text of this value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Publisher => "publisher",
            Self::Credit => "credit",
            Self::Unknown(raw) => raw,
        }
    }
}

impl From<String> for AlbumNamesAs {
    fn from(raw: String) -> Self {
        match raw.as_str() {
            "publisher" => Self::Publisher,
            "credit" => Self::Credit,
            _ => Self::Unknown(raw),
        }
    }
}

impl From<AlbumNamesAs> for String {
    fn from(value: AlbumNamesAs) -> Self {
        match value {
            AlbumNamesAs::Unknown(raw) => raw,
            known => known.as_str().to_owned(),
        }
    }
}

/// Whether the two sides of a publisher relationship agree on its role
/// (ADR 0082 Decision 2, Stophammer ADR 0049 §6a).
///
/// `None` on the relationship means that no side states a role. A
/// `Conflict` row is not a confirmed link. An unrecognized value keeps its
/// raw text. It does not fail the response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum RoleAgreement {
    Both,
    OneSide,
    Conflict,
    Unknown(String),
}

impl RoleAgreement {
    /// Returns the wire text of this value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Both => "both",
            Self::OneSide => "one_side",
            Self::Conflict => "conflict",
            Self::Unknown(raw) => raw,
        }
    }
}

impl From<String> for RoleAgreement {
    fn from(raw: String) -> Self {
        match raw.as_str() {
            "both" => Self::Both,
            "one_side" => Self::OneSide,
            "conflict" => Self::Conflict,
            _ => Self::Unknown(raw),
        }
    }
}

impl From<RoleAgreement> for String {
    fn from(value: RoleAgreement) -> Self {
        match value {
            RoleAgreement::Unknown(raw) => raw,
            known => known.as_str().to_owned(),
        }
    }
}

/// One entry of `co_credited_feeds`: another publisher feed that shares a
/// confirmed album with the feed that a response describes (ADR 0082
/// Decision 5, Stophammer ADR 0069 §4).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoCreditedFeed {
    pub feed_guid: Option<String>,
    /// The title of the co-credited feed, or null when the node holds no
    /// feed for it.
    pub title: Option<String>,
    /// The different raw `rel` values the shared albums give this feed,
    /// sorted. Empty when no album states one.
    pub roles: Option<Vec<String>>,
    /// The number of shared confirmed albums.
    pub album_count: Option<i64>,
}

/// One publisher relationship from an API response (ADRs 0075 and 0077).
///
/// The fields follow the live `PublisherResponse` contract of 2026-09-24,
/// extended on 2026-10-02 with the Stophammer 0.7.0 link facts of ADR 0082
/// Decision 2. The legacy fields stay, because the live contract still
/// sends them. Stophammer ADR 0059 added the four `remote_*` summary
/// fields. Stophammer deployed that change on 2026-09-26 (ADR 0077 Task
/// 003).
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
    /// `true` when the album names this publisher feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_names_publisher: Option<bool>,
    /// `true` when the publisher feed lists this album.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_lists_music: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_link_resolution: Option<PublisherLinkResolution>,
    /// The time of the URL observation behind `publisher_link_resolution`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_link_observed_at: Option<i64>,
    /// The raw `rel` of the publisher feed item that lists this album.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_rel: Option<String>,
    /// The raw `rel` of the album feed item that names this publisher.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_rel: Option<String>,
    /// The role set of this link, sorted and joined by `", "`. Null when
    /// neither side states one, and null on a conflict (ADR 0082 Decision
    /// 2, amended 2026-10-02: the node gives no assumed default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_source: Option<RoleSource>,
    /// How the album names this publisher feed (ADR 0082 Decision 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_names_as: Option<AlbumNamesAs>,
    /// Whether the two sides agree on `role` (ADR 0082 Decision 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_agreement: Option<RoleAgreement>,
    /// The title of the feed that `remote_feed_guid` names (Stophammer ADR
    /// 0059, ADR 0077 Task 003). On a `music_to_publisher` entry, this
    /// describes the publisher feed. On a `publisher_to_music` entry, this
    /// describes the album.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_feed_title: Option<String>,
    /// The channel image URL of the feed that `remote_feed_guid` names
    /// (Stophammer ADR 0059, ADR 0077 Task 003).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_feed_image_url: Option<String>,
    /// The `release_artist` of the feed that `remote_feed_guid` names
    /// (Stophammer ADR 0059, ADR 0077 Task 003).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_release_artist: Option<String>,
    /// The source of `remote_release_artist`, as Stophammer sends it
    /// (Stophammer ADR 0059, ADR 0077 Task 003).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_release_artist_source: Option<String>,
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
    Feed(Feed),
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

    /// Fetches a feed using a named ADR 0075 request profile.
    ///
    /// The profile supplies the include list. An L0 profile sends no
    /// `include` query parameter. The wire format stays the same as
    /// `fetch_feed`.
    ///
    /// # Errors
    /// Returns the same errors as `fetch_feed`.
    pub(crate) fn fetch_feed_with_profile(
        &self,
        feed_guid: &str,
        profile: &RequestProfile,
    ) -> Result<Feed> {
        debug_assert_eq!(
            profile.path_shape(),
            RequestPathShape::Feed,
            "ADR 0075 packet 017: {profile:?} must name the feed path shape"
        );
        self.fetch_feed(feed_guid, profile.include())
    }

    /// Fetches an unscoped track using a named ADR 0075 request profile.
    ///
    /// The profile supplies the include list. The wire format stays the
    /// same as `fetch_track`.
    ///
    /// # Errors
    /// Returns the same errors as `fetch_track`.
    pub(crate) fn fetch_track_with_profile(
        &self,
        track_guid: &str,
        profile: &RequestProfile,
    ) -> Result<Track> {
        debug_assert!(
            profile.path_shape() == RequestPathShape::UnscopedTrack,
            "ADR 0075 packet 017: {profile:?} must name an unscoped track path shape"
        );
        self.fetch_track(track_guid, profile.include())
    }

    /// Fetches a scoped track using a named ADR 0075 request profile.
    ///
    /// The profile supplies the include list. The wire format stays the
    /// same as `fetch_feed_track`.
    ///
    /// # Errors
    /// Returns the same errors as `fetch_feed_track`.
    pub(crate) fn fetch_feed_track_with_profile(
        &self,
        feed_guid: &str,
        track_guid: &str,
        profile: &RequestProfile,
    ) -> Result<Track> {
        debug_assert!(
            profile.path_shape() == RequestPathShape::ScopedTrack,
            "ADR 0075 packet 017: {profile:?} must name a scoped track path shape"
        );
        self.fetch_feed_track(feed_guid, track_guid, profile.include())
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

    /// Fetches tracks by artist name using a named ADR 0075 request profile
    /// (ADR 0077 packet 006). The profile supplies the include list. An L0
    /// profile sends no `include` query parameter. The wire format stays
    /// the same as `fetch_tracks_by_artist`.
    ///
    /// # Errors
    /// Returns the same errors as `fetch_tracks_by_artist`.
    pub(crate) fn fetch_tracks_by_artist_with_profile(
        &self,
        artist: &str,
        limit: Option<i32>,
        cursor: Option<&str>,
        profile: &RequestProfile,
    ) -> Result<TrackListResponse> {
        debug_assert_eq!(
            profile.path_shape(),
            RequestPathShape::TracksByArtistName,
            "ADR 0077 packet 006: {profile:?} must name the tracks-by-artist-name path shape"
        );
        let mut params = vec![
            ("artist", artist.to_string()),
            ("limit", limit.unwrap_or(PAGE_LIMIT).to_string()),
        ];
        if let Some(cursor) = cursor {
            params.push(("cursor", cursor.to_string()));
        }
        if let Some(include) = profile.include() {
            params.push(("include", include.to_string()));
        }
        self.get_json(&["v1", "tracks"], &params)
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
        // This caller retains no response, so it keeps no receipt.
        let _receipt = recorder.record(token, observation)?;
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
pub(crate) mod tests {

    #[test]
    fn adr_0066_invalid_endpoint_rejects_requests_before_transport() {
        let client =
            Client::new_with_base_url("https://secret:credential@invalid host/?token=hidden");
        let error = client
            .search("local", None, None, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("musicindex_endpoint"));
        for secret in ["secret", "credential", "hidden"] {
            assert!(!error.contains(secret));
        }
    }

    use super::{
        Client, Contributor, Feed, PaymentRoute, SearchResult, SourceEnclosure, SourceEntityId,
        SourceEntityLink, SourceReleaseClaim, SourceTranscript, Track,
    };
    use crate::application::request_profiles;

    /// R17-03: an L0 profile sends no `include` query parameter, and the
    /// recorded request path equals the path before this packet.
    #[test]
    fn adr_0075_request_profile_l0_profile_sends_no_include_parameter() {
        let client = Client::new();
        let mut params = Vec::new();
        if let Some(include) = request_profiles::INDEX_TRACK_DETAIL_UNSCOPED.include() {
            params.push(("include", include.to_string()));
        }
        let url = client
            .build_url(&["v1", "tracks", "t1"], &params)
            .expect("url");
        assert_eq!(url.path(), "/v1/tracks/t1");
        assert_eq!(
            url.query(),
            None,
            "an L0 profile must send no include parameter"
        );
    }

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
    fn track_with_feed_defaults_inherits_missing_feed_level_metadata() {
        let track = Track {
            track_guid: Some("track-guid".into()),
            ..Default::default()
        };
        let feed = Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("Feed".into()),
            publisher_text: Some("publisher".into()),
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
        assert_eq!(hydrated.source_contributors.as_ref().map(Vec::len), Some(1));
        assert_eq!(hydrated.payment_routes.as_ref().map(Vec::len), Some(1));
    }

    /// R22-01 (ADR 0075 packet 022): `track_with_feed_defaults` does not
    /// copy a feed's `source_links`, `source_ids` or `description` into a
    /// track that has none of its own. It still copies each other value.
    #[test]
    fn adr_0075_track_header_r22_01_stops_identity_and_description_copies() {
        let track = Track {
            track_guid: Some("track-guid".into()),
            ..Default::default()
        };
        let feed = Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("Feed".into()),
            publisher_text: Some("publisher".into()),
            description: Some("feed description".into()),
            image_url: Some("https://example.test/feed.jpg".into()),
            release_artist: Some("Feed Artist".into()),
            source_links: Some(vec![SourceEntityLink {
                link_type: Some("website".into()),
                url: Some("https://example.test/feed".into()),
                ..Default::default()
            }]),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1feed".into()),
                ..Default::default()
            }]),
            source_contributors: Some(vec![Contributor {
                name: Some("Alice".into()),
                role: Some("musician".into()),
                ..Default::default()
            }]),
            source_release_claims: Some(vec![SourceReleaseClaim {
                claim_type: Some("description".into()),
                claim_value: Some("claim".into()),
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

        // The three stopped copies: the track keeps no identity and no
        // description from the feed.
        assert!(hydrated.source_links.is_none());
        assert!(hydrated.source_ids.is_none());
        assert!(hydrated.description.is_none());

        // Each other value still copies.
        assert_eq!(hydrated.feed_guid.as_deref(), Some("feed-guid"));
        assert_eq!(hydrated.feed_title.as_deref(), Some("Feed"));
        assert_eq!(hydrated.publisher_text.as_deref(), Some("publisher"));
        assert_eq!(
            hydrated.image_url.as_deref(),
            Some("https://example.test/feed.jpg")
        );
        assert_eq!(hydrated.release_artist.as_deref(), Some("Feed Artist"));
        assert_eq!(hydrated.source_contributors.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            hydrated.source_release_claims.as_ref().map(Vec::len),
            Some(1)
        );
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
        assert_eq!(track.source_contributors.as_ref().map(Vec::len), Some(1));
        assert_eq!(track.source_links.as_ref().map(Vec::len), Some(1));
        assert_eq!(track.source_ids.as_ref().map(Vec::len), Some(1));
    }

    /// R48-01 (ADR 0075 packet 048): a recorded response that carries a
    /// track image and a feed image decodes both owner fields, and it
    /// still decodes the resolved `image_url` (Stophammer ADR 0042).
    #[test]
    fn adr_0075_track_artwork_r48_01_decodes_both_owner_fields_and_resolved_image() {
        let track: Track = serde_json::from_str(
            r#"{
                "track_guid": "track-1",
                "image_url": "https://example.test/track.jpg",
                "track_image_url": "https://example.test/track.jpg",
                "feed_image_url": "https://example.test/feed.jpg"
            }"#,
        )
        .expect("a recorded track response with both owner fields should decode");

        assert_eq!(
            track.track_image_url.as_deref(),
            Some("https://example.test/track.jpg")
        );
        assert_eq!(
            track.feed_image_url.as_deref(),
            Some("https://example.test/feed.jpg")
        );
        assert_eq!(
            track.image_url.as_deref(),
            Some("https://example.test/track.jpg")
        );
    }

    /// R47-01 (ADR 0075 packet 047): a recorded feed hit and a recorded
    /// track hit, each captured against the deployed contract on
    /// 2026-09-29, decode every summary field the hit supplies. A field
    /// the hit omits decodes as `None`.
    #[test]
    fn adr_0075_search_summary_r47_01_decodes_summary_fields_and_omits_absent_ones() {
        let feed_hit: SearchResult = serde_json::from_str(
            r#"{
                "entity_type": "feed",
                "entity_id": "1429d28c-9af0-5529-98e5-0f400afca76b",
                "rank": -8.03,
                "quality_score": 85,
                "title": "Monster",
                "feed_image_url": "https://d12wklypp119aj.cloudfront.net/image/3a64dbaf-7d8d-4f05-ac8d-92adbfce991b.jpg",
                "release_artist": "Official DETOX Music",
                "release_artist_source": "itunes_author",
                "episode_count": 1
            }"#,
        )
        .expect("a recorded feed hit should decode");

        assert_eq!(feed_hit.entity_type, "feed");
        assert_eq!(feed_hit.entity_id, "1429d28c-9af0-5529-98e5-0f400afca76b");
        assert_eq!(feed_hit.rank, Some(-8.03));
        assert_eq!(feed_hit.quality_score, Some(85.0));
        assert_eq!(feed_hit.title.as_deref(), Some("Monster"));
        assert_eq!(
            feed_hit.feed_image_url.as_deref(),
            Some("https://d12wklypp119aj.cloudfront.net/image/3a64dbaf-7d8d-4f05-ac8d-92adbfce991b.jpg")
        );
        assert_eq!(
            feed_hit.release_artist.as_deref(),
            Some("Official DETOX Music")
        );
        assert_eq!(
            feed_hit.release_artist_source.as_deref(),
            Some("itunes_author")
        );
        assert_eq!(feed_hit.episode_count, Some(1));
        assert_eq!(feed_hit.feed_guid, None);
        assert_eq!(feed_hit.feed_title, None);
        assert_eq!(feed_hit.track_image_url, None);
        assert_eq!(feed_hit.track_artist, None);
        assert_eq!(feed_hit.pub_date, None);
        assert_eq!(feed_hit.duration_secs, None);
        assert_eq!(feed_hit.href, None);

        let track_hit: SearchResult = serde_json::from_str(
            r#"{
                "entity_type": "track",
                "entity_id": "ddf821be-1139-47b9-ab1e-44594b0cd779",
                "rank": -9.34,
                "quality_score": 80,
                "feed_guid": "bcb53b0b-d88e-5e01-9e9d-dfeeae52e2c9",
                "href": "/v1/feeds/bcb53b0b-d88e-5e01-9e9d-dfeeae52e2c9/tracks/ddf821be-1139-47b9-ab1e-44594b0cd779",
                "title": "Monster",
                "feed_title": "Shadows of Light",
                "feed_image_url": "https://d12wklypp119aj.cloudfront.net/image/897e093c-ef1c-454b-9985-18310f04f916.jpg",
                "pub_date": 1728711846,
                "track_artist": "Absolut Absolem",
                "duration_secs": 215
            }"#,
        )
        .expect("a recorded track hit should decode");

        assert_eq!(track_hit.entity_type, "track");
        assert_eq!(track_hit.entity_id, "ddf821be-1139-47b9-ab1e-44594b0cd779");
        assert_eq!(track_hit.rank, Some(-9.34));
        assert_eq!(track_hit.quality_score, Some(80.0));
        assert_eq!(
            track_hit.feed_guid.as_deref(),
            Some("bcb53b0b-d88e-5e01-9e9d-dfeeae52e2c9")
        );
        assert_eq!(
            track_hit.href.as_deref(),
            Some("/v1/feeds/bcb53b0b-d88e-5e01-9e9d-dfeeae52e2c9/tracks/ddf821be-1139-47b9-ab1e-44594b0cd779")
        );
        assert_eq!(track_hit.title.as_deref(), Some("Monster"));
        assert_eq!(track_hit.feed_title.as_deref(), Some("Shadows of Light"));
        assert_eq!(
            track_hit.feed_image_url.as_deref(),
            Some("https://d12wklypp119aj.cloudfront.net/image/897e093c-ef1c-454b-9985-18310f04f916.jpg")
        );
        assert_eq!(track_hit.pub_date, Some(1_728_711_846));
        assert_eq!(track_hit.track_artist.as_deref(), Some("Absolut Absolem"));
        assert_eq!(track_hit.duration_secs, Some(215));
        assert_eq!(track_hit.release_artist, None);
        assert_eq!(track_hit.release_artist_source, None);
        assert_eq!(track_hit.track_image_url, None);
        assert_eq!(track_hit.episode_count, None);
    }

    /// R46-01: `api::Feed` and `api::Track` decode a response that contains
    /// `name` and `feed_url` without an error, and the undeclared values
    /// are ignored (ADR 0075 packet 046). The deployed contract sends
    /// neither field on a track.
    #[test]
    fn adr_0075_undeclared_fields_r46_01_feed_and_track_ignore_undeclared_fields() {
        let feed: Feed = serde_json::from_str(
            r#"{
                "feed_guid": "feed-1",
                "title": "Feed Title",
                "name": "Legacy Feed Name",
                "feed_url": "https://example.test/feed.xml"
            }"#,
        )
        .expect("a feed response with an undeclared name field must still decode");
        assert_eq!(feed.title.as_deref(), Some("Feed Title"));
        assert_eq!(
            feed.feed_url.as_deref(),
            Some("https://example.test/feed.xml")
        );

        let track: Track = serde_json::from_str(
            r#"{
                "track_guid": "track-1",
                "title": "Track Title",
                "name": "Legacy Track Name",
                "feed_url": "https://example.test/feed.xml"
            }"#,
        )
        .expect("a track response with undeclared name and feed_url fields must still decode");
        assert_eq!(track.title.as_deref(), Some("Track Title"));
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

    /// ADR 0077 packet 002: the live publisher relationship contract.
    pub(crate) mod adr_0077_publisher_relationship {
        use crate::api::{DetailResponse, Feed, PublisherLinkResolution, RoleSource};

        /// Recorded on 2026-09-24 from
        /// `GET https://api.musicindex.org/v1/feeds/1ac44a3c-e148-54db-9d12-72191222888f?include=publisher`.
        /// Trimmed to the fields that the test needs. The `publisher` entry is unchanged.
        const RECORDED_ALBUM_RESPONSE: &str = r#"{
            "data": {
                "feed_guid": "1ac44a3c-e148-54db-9d12-72191222888f",
                "feed_url": "https://wavlake.com/feed/music/80288280-09a0-4d6e-bdfb-cdef4e987b63",
                "title": "Genesis 2",
                "raw_medium": "music",
                "release_artist": "Liberthea Anadara",
                "release_artist_source": "itunes_author",
                "publisher_text": "Wavlake",
                "publisher_feed_title": "Liberthea Anadara",
                "explicit": false,
                "created_at": 1788982326,
                "updated_at": 1790216976,
                "publisher": [
                    {
                        "direction": "music_to_publisher",
                        "remote_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "remote_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "remote_feed_medium": "publisher",
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "1ac44a3c-e148-54db-9d12-72191222888f",
                        "music_feed_url": "https://wavlake.com/feed/music/80288280-09a0-4d6e-bdfb-cdef4e987b63",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1788982326,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "music",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    }
                ]
            },
            "pagination": {"cursor": null, "has_more": false}
        }"#;

        /// Recorded on 2026-09-24 from
        /// `GET https://api.musicindex.org/v1/feeds/bcbe7207-9338-474e-ba18-09e6b1b69979?include=publisher`.
        /// Trimmed to the feed fields that the tests need. The nine
        /// `publisher_to_music` entries are unchanged.
        pub(crate) const RECORDED_PUBLISHER_RESPONSE: &str = r#"{
            "data": {
                "feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                "feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                "title": "Liberthea Anadara",
                "raw_medium": "publisher",
                "release_artist": "Unknown Artist",
                "release_artist_source": "placeholder",
                "publisher_text": null,
                "publisher_feed_title": null,
                "distinct_release_artist_count": 1,
                "distinct_release_artists": [
                    "Liberthea Anadara"
                ],
                "explicit": false,
                "created_at": 1790239904,
                "updated_at": 1790253973,
                "publisher": [
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "74e70e30-5852-4217-a7ea-c7dbec9ea09a",
                        "remote_feed_url": "https://wavlake.com/feed/music/74e70e30-5852-4217-a7ea-c7dbec9ea09a",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "7c94444d-6074-5fec-a841-72ade85b3fb0",
                        "music_feed_url": "https://wavlake.com/feed/music/74e70e30-5852-4217-a7ea-c7dbec9ea09a",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1776628950,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "75bdcdb1-b4c4-4f28-8c04-67ade3d1dd2d",
                        "remote_feed_url": "https://wavlake.com/feed/music/75bdcdb1-b4c4-4f28-8c04-67ade3d1dd2d",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "9f2d9ec8-4115-5206-adfb-f4af078ed5cd",
                        "music_feed_url": "https://wavlake.com/feed/music/75bdcdb1-b4c4-4f28-8c04-67ade3d1dd2d",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1776628949,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "8ae40c2b-c8ed-423d-b499-e96cbc7dd85d",
                        "remote_feed_url": "https://wavlake.com/feed/music/8ae40c2b-c8ed-423d-b499-e96cbc7dd85d",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "0d17637c-45a4-5cf4-a8b7-0c6ed2828b70",
                        "music_feed_url": "https://wavlake.com/feed/music/8ae40c2b-c8ed-423d-b499-e96cbc7dd85d",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1776628950,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "14ecf918-26f9-4f72-aef1-ae811cce563d",
                        "remote_feed_url": "https://wavlake.com/feed/music/14ecf918-26f9-4f72-aef1-ae811cce563d",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "65e6cb77-c7d5-55bf-a747-a76f09ff1255",
                        "music_feed_url": "https://wavlake.com/feed/music/14ecf918-26f9-4f72-aef1-ae811cce563d",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1776628951,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "1895f73b-0262-4b22-a6ee-19fc58a8fd87",
                        "remote_feed_url": "https://wavlake.com/feed/music/1895f73b-0262-4b22-a6ee-19fc58a8fd87",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "9c59b4bb-1597-5949-ad01-ba04f4952508",
                        "music_feed_url": "https://wavlake.com/feed/music/1895f73b-0262-4b22-a6ee-19fc58a8fd87",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1776628952,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "c0e99622-1559-4011-b292-d54f302fb396",
                        "remote_feed_url": "https://wavlake.com/feed/music/c0e99622-1559-4011-b292-d54f302fb396",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "8861620d-555b-5280-abf7-71a516094c2a",
                        "music_feed_url": "https://wavlake.com/feed/music/c0e99622-1559-4011-b292-d54f302fb396",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1781464203,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "80288280-09a0-4d6e-bdfb-cdef4e987b63",
                        "remote_feed_url": "https://wavlake.com/feed/music/80288280-09a0-4d6e-bdfb-cdef4e987b63",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "1ac44a3c-e148-54db-9d12-72191222888f",
                        "music_feed_url": "https://wavlake.com/feed/music/80288280-09a0-4d6e-bdfb-cdef4e987b63",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1788982326,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "e975a870-d325-4960-b89e-c6e60634057a",
                        "remote_feed_url": "https://wavlake.com/feed/music/e975a870-d325-4960-b89e-c6e60634057a",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "2471eceb-e522-5718-b896-2658bb073d23",
                        "music_feed_url": "https://wavlake.com/feed/music/e975a870-d325-4960-b89e-c6e60634057a",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1781464334,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "50dbc8ab-0d79-49a1-9b49-6651687cf168",
                        "remote_feed_url": "https://wavlake.com/feed/music/50dbc8ab-0d79-49a1-9b49-6651687cf168",
                        "remote_feed_medium": null,
                        "publisher_feed_guid": "bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "publisher_feed_url": "https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979",
                        "music_feed_guid": "478639b6-ff12-574b-9756-74ddd0add8dd",
                        "music_feed_url": "https://wavlake.com/feed/music/50dbc8ab-0d79-49a1-9b49-6651687cf168",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "publisher_link_observed_at": 1788982592,
                        "reciprocal_declared": true,
                        "reciprocal_medium": "publisher",
                        "two_way_validated": true,
                        "publisher_rel": null,
                        "music_rel": null,
                        "role": "artist",
                        "role_source": "default"
                    }
                ]
            },
            "pagination": {
                "cursor": null,
                "has_more": false
            }
        }"#;

        /// R2-01: the recorded album response decodes each `PublisherResponse` field.
        #[test]
        fn adr_0077_publisher_relationship_recorded_album_decodes_each_field() {
            let response: DetailResponse<Feed> = serde_json::from_str(RECORDED_ALBUM_RESPONSE)
                .expect("the recorded album response should decode");
            let feed = response.data;
            assert_eq!(
                feed.publisher_feed_title.as_deref(),
                Some("Liberthea Anadara")
            );
            assert_eq!(feed.release_artist_source.as_deref(), Some("itunes_author"));

            let entries = feed.publisher.expect("the album should carry publisher");
            assert_eq!(entries.len(), 1);
            let entry = &entries[0];
            assert_eq!(entry.direction.as_deref(), Some("music_to_publisher"));
            assert_eq!(
                entry.remote_feed_guid.as_deref(),
                Some("bcbe7207-9338-474e-ba18-09e6b1b69979")
            );
            assert_eq!(
                entry.remote_feed_url.as_deref(),
                Some("https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979")
            );
            assert_eq!(entry.remote_feed_medium.as_deref(), Some("publisher"));
            assert_eq!(
                entry.publisher_feed_guid.as_deref(),
                Some("bcbe7207-9338-474e-ba18-09e6b1b69979")
            );
            assert_eq!(
                entry.publisher_feed_url.as_deref(),
                Some("https://wavlake.com/feed/artist/bcbe7207-9338-474e-ba18-09e6b1b69979")
            );
            assert_eq!(
                entry.music_feed_guid.as_deref(),
                Some("1ac44a3c-e148-54db-9d12-72191222888f")
            );
            assert_eq!(
                entry.music_feed_url.as_deref(),
                Some("https://wavlake.com/feed/music/80288280-09a0-4d6e-bdfb-cdef4e987b63")
            );
            assert_eq!(entry.music_names_publisher, Some(true));
            assert_eq!(entry.publisher_lists_music, Some(true));
            assert_eq!(
                entry.publisher_link_resolution,
                Some(PublisherLinkResolution::FeedUrl)
            );
            assert_eq!(entry.publisher_link_observed_at, Some(1_788_982_326));
            assert_eq!(entry.reciprocal_declared, Some(true));
            assert_eq!(entry.reciprocal_medium.as_deref(), Some("music"));
            assert_eq!(entry.two_way_validated, Some(true));
            assert_eq!(entry.publisher_rel, None);
            assert_eq!(entry.music_rel, None);
            assert_eq!(entry.role.as_deref(), Some("artist"));
            assert_eq!(entry.role_source, Some(RoleSource::Default));

            let recorded: serde_json::Value =
                serde_json::from_str(RECORDED_ALBUM_RESPONSE).unwrap();
            let recorded_entry = recorded["data"]["publisher"][0]
                .as_object()
                .unwrap()
                .clone();
            let serialized = serde_json::to_value(entry).unwrap();
            for (key, value) in recorded_entry {
                if value.is_null() {
                    assert!(serialized.get(&key).is_none(), "{key} should stay absent");
                } else {
                    assert_eq!(serialized[&key], value, "{key} should round trip");
                }
            }
        }

        /// R2-02: the recorded publisher feed response decodes each
        /// `publisher_to_music` entry. The recorded body keeps each field of
        /// the response as evidence. `Feed` decodes only the declared fields
        /// that the app reads (ADR 0077 Task 007).
        #[test]
        fn adr_0077_publisher_relationship_recorded_publisher_feed_decodes_entries() {
            let response: DetailResponse<Feed> = serde_json::from_str(RECORDED_PUBLISHER_RESPONSE)
                .expect("the recorded publisher response should decode");
            let feed = response.data;
            assert_eq!(feed.release_artist_source.as_deref(), Some("placeholder"));
            assert_eq!(feed.publisher_feed_title, None);

            let entries = feed
                .publisher
                .expect("the publisher feed should carry publisher");
            let music_feeds = entries
                .iter()
                .map(|entry| entry.music_feed_guid.as_deref())
                .collect::<Vec<_>>();
            assert_eq!(music_feeds.len(), 9);
            assert_eq!(music_feeds[0], Some("7c94444d-6074-5fec-a841-72ade85b3fb0"));
            assert_eq!(music_feeds[6], Some("1ac44a3c-e148-54db-9d12-72191222888f"));
            for entry in &entries {
                assert_eq!(entry.direction.as_deref(), Some("publisher_to_music"));
                assert_eq!(
                    entry.publisher_feed_guid.as_deref(),
                    Some("bcbe7207-9338-474e-ba18-09e6b1b69979")
                );
                assert_eq!(entry.remote_feed_medium, None);
                assert_eq!(entry.music_names_publisher, Some(true));
                assert_eq!(entry.publisher_lists_music, Some(true));
                assert_eq!(
                    entry.publisher_link_resolution,
                    Some(PublisherLinkResolution::FeedUrl)
                );
                assert!(entry.publisher_link_observed_at.is_some());
                assert_eq!(entry.reciprocal_medium.as_deref(), Some("publisher"));
                assert_eq!(entry.role.as_deref(), Some("artist"));
                assert_eq!(entry.role_source, Some(RoleSource::Default));
            }
            assert_eq!(entries[0].publisher_link_observed_at, Some(1_776_628_950));
            assert_eq!(entries[6].publisher_link_observed_at, Some(1_788_982_326));
        }

        /// R2-03: an unknown `role_source` or `publisher_link_resolution` value
        /// decodes to the unknown variant with its raw text.
        #[test]
        fn adr_0077_publisher_relationship_unknown_enum_values_keep_raw_text() {
            let feed: Feed = serde_json::from_str(
                r#"{"publisher": [{
                    "direction": "music_to_publisher",
                    "publisher_feed_guid": "publisher-feed-1",
                    "publisher_link_resolution": "future_resolution",
                    "role_source": "future_source",
                    "role": null
                }]}"#,
            )
            .expect("unknown enum values must not fail the response");
            let entry = &feed.publisher.unwrap()[0];
            assert_eq!(
                entry.publisher_link_resolution,
                Some(PublisherLinkResolution::Unknown(
                    "future_resolution".to_owned()
                ))
            );
            assert_eq!(
                entry.role_source,
                Some(RoleSource::Unknown("future_source".to_owned()))
            );
            assert_eq!(entry.role, None);
            let serialized = serde_json::to_value(entry).unwrap();
            assert_eq!(serialized["publisher_link_resolution"], "future_resolution");
            assert_eq!(serialized["role_source"], "future_source");

            for (raw, expected) in [
                ("guid", PublisherLinkResolution::Guid),
                ("feed_url", PublisherLinkResolution::FeedUrl),
                ("unresolved", PublisherLinkResolution::Unresolved),
            ] {
                assert_eq!(PublisherLinkResolution::from(raw.to_owned()), expected);
                assert_eq!(expected.as_str(), raw);
            }
            for (raw, expected) in [
                ("publisher_rel", RoleSource::PublisherRel),
                ("music_rel", RoleSource::MusicRel),
                ("default", RoleSource::Default),
                ("conflict", RoleSource::Conflict),
            ] {
                assert_eq!(RoleSource::from(raw.to_owned()), expected);
                assert_eq!(expected.as_str(), raw);
            }
            assert!(
                serde_json::from_str::<Feed>(r#"{"publisher": [{"role_source": 7}]}"#).is_err()
            );
        }
    }

    /// ADR 0077 Task 003: the four `remote_*` summary fields of Stophammer
    /// ADR 0059. Stophammer deployed this change on 2026-09-26.
    mod adr_0077_publisher_page {
        use crate::api::{DetailResponse, Feed};

        /// Recorded on 2026-09-26 from
        /// `GET https://api.musicindex.org/v1/feeds/137aaa9c-75ff-4916-9f23-e02968b2d15e?include=publisher`.
        /// Trimmed to the fields the tests need. The first entry names each
        /// summary field. The second entry names none of them, as an entry
        /// with no linked feed sends null for each one.
        const RECORDED_PUBLISHER_FEED_WITH_SUMMARY_FIELDS: &str = r#"{
            "data": {
                "feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
                "title": "Official DETOX Music",
                "publisher": [
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "album-guid-1",
                        "publisher_feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
                        "remote_feed_title": "Think'n Bout Ya",
                        "remote_feed_image_url": "https://d12wklypp119aj.cloudfront.net/image/0ea057e2-fcf3-4e1d-881e-51a87b582b92.jpg",
                        "remote_release_artist": "Official DETOX Music",
                        "remote_release_artist_source": "itunes_author",
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "role": "artist",
                        "role_source": "default"
                    },
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "album-guid-2",
                        "publisher_feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
                        "remote_feed_title": null,
                        "remote_feed_image_url": null,
                        "remote_release_artist": null,
                        "remote_release_artist_source": null,
                        "music_names_publisher": true,
                        "publisher_lists_music": true,
                        "publisher_link_resolution": "feed_url",
                        "role": "artist",
                        "role_source": "default"
                    }
                ]
            },
            "pagination": {"cursor": null, "has_more": false}
        }"#;

        /// R3-13: `PublisherRelationship` decodes each `remote_*` summary
        /// field, and it keeps the value on a round trip.
        #[test]
        fn adr_0077_publisher_page_summary_fields_decode_each_stated_value() {
            let response: DetailResponse<Feed> =
                serde_json::from_str(RECORDED_PUBLISHER_FEED_WITH_SUMMARY_FIELDS)
                    .expect("the recorded response should decode");
            let entries = response
                .data
                .publisher
                .expect("the response should carry publisher entries");
            let entry = &entries[0];
            assert_eq!(entry.remote_feed_title.as_deref(), Some("Think'n Bout Ya"));
            assert_eq!(
                entry.remote_feed_image_url.as_deref(),
                Some(
                    "https://d12wklypp119aj.cloudfront.net/image/0ea057e2-fcf3-4e1d-881e-51a87b582b92.jpg"
                )
            );
            assert_eq!(
                entry.remote_release_artist.as_deref(),
                Some("Official DETOX Music")
            );
            assert_eq!(
                entry.remote_release_artist_source.as_deref(),
                Some("itunes_author")
            );

            let serialized = serde_json::to_value(entry).unwrap();
            assert_eq!(serialized["remote_feed_title"], "Think'n Bout Ya");
            assert_eq!(serialized["remote_release_artist_source"], "itunes_author");
        }

        /// R3-13/R3-14: an entry with a null summary field, and an entry
        /// with no summary field at all, both decode to `None` in each
        /// `remote_*` field.
        #[test]
        fn adr_0077_publisher_page_summary_fields_absent_or_null_decode_to_none() {
            let response: DetailResponse<Feed> =
                serde_json::from_str(RECORDED_PUBLISHER_FEED_WITH_SUMMARY_FIELDS)
                    .expect("the recorded response should decode");
            let entries = response.data.publisher.expect("entries");
            let null_entry = &entries[1];
            assert_eq!(null_entry.remote_feed_title, None);
            assert_eq!(null_entry.remote_feed_image_url, None);
            assert_eq!(null_entry.remote_release_artist, None);
            assert_eq!(null_entry.remote_release_artist_source, None);

            let feed: Feed =
                serde_json::from_str(r#"{"publisher": [{"direction": "music_to_publisher"}]}"#)
                    .expect("an entry with no summary field must still decode");
            let entry = &feed.publisher.unwrap()[0];
            assert_eq!(entry.remote_feed_title, None);
            assert_eq!(entry.remote_feed_image_url, None);
            assert_eq!(entry.remote_release_artist, None);
            assert_eq!(entry.remote_release_artist_source, None);
            let serialized = serde_json::to_value(entry).unwrap();
            assert!(
                serialized.get("remote_feed_title").is_none(),
                "an absent summary field must stay out of the serialized entry"
            );
        }
    }

    /// ADR 0082 packet 001: the Stophammer 0.7.0 link-fact contract. The
    /// node deployed this contract on 2026-10-02 (ADR 0082 Context).
    mod adr_0082_link_facts {
        use crate::api::{AlbumNamesAs, DetailResponse, Feed, RoleAgreement, RoleSource};

        /// Recorded on 2026-10-02 from
        /// `GET https://api.musicindex.org/v1/feeds/137aaa9c-75ff-4916-9f23-e02968b2d15e?include=publisher`
        /// (the DETOX publisher feed). Trimmed to the fields the test
        /// needs. ADR 0082 Context names this exact row shape.
        const RECORDED_DETOX_PUBLISHER_ROW: &str = r#"{
            "data": {
                "feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
                "publisher": [
                    {
                        "direction": "publisher_to_music",
                        "remote_feed_guid": "album-guid-1",
                        "publisher_feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
                        "album_names_as": "publisher",
                        "role": null,
                        "role_source": "default",
                        "role_agreement": null,
                        "two_way_validated": true,
                        "music_names_publisher": true
                    }
                ]
            },
            "pagination": {"cursor": null, "has_more": false}
        }"#;

        /// R82-1-02: the recorded 0.7.0 DETOX publisher row decodes
        /// `album_names_as` `publisher`, a null `role`, `role_source`
        /// `default` and a null `role_agreement`.
        #[test]
        fn adr_0082_link_facts_recorded_detox_row_decodes_0_7_0_shape() {
            let response: DetailResponse<Feed> = serde_json::from_str(RECORDED_DETOX_PUBLISHER_ROW)
                .expect("the recorded DETOX publisher row should decode");
            let entries = response
                .data
                .publisher
                .expect("the feed should carry publisher");
            let entry = &entries[0];
            assert_eq!(entry.album_names_as, Some(AlbumNamesAs::Publisher));
            assert_eq!(entry.role, None);
            assert_eq!(entry.role_source, Some(RoleSource::Default));
            assert_eq!(entry.role_agreement, None);
            assert_eq!(entry.two_way_validated, Some(true));
            assert_eq!(entry.music_names_publisher, Some(true));
        }

        /// R82-1-03: a recorded row with each declared `role_agreement`
        /// value decodes it, and an unknown value decodes to the fallback.
        #[test]
        fn adr_0082_link_facts_role_agreement_values_decode() {
            for (raw, expected) in [
                ("both", RoleAgreement::Both),
                ("one_side", RoleAgreement::OneSide),
                ("conflict", RoleAgreement::Conflict),
                (
                    "future_agreement",
                    RoleAgreement::Unknown("future_agreement".to_owned()),
                ),
            ] {
                let feed: Feed = serde_json::from_str(&format!(
                    r#"{{"publisher": [{{
                        "direction": "publisher_to_music",
                        "publisher_feed_guid": "publisher-feed-1",
                        "role": "artist, label",
                        "role_source": "publisher_rel",
                        "role_agreement": "{raw}"
                    }}]}}"#
                ))
                .unwrap_or_else(|error| {
                    panic!("a recorded role_agreement of {raw} must not fail the response: {error}")
                });
                let entry = &feed.publisher.unwrap()[0];
                assert_eq!(
                    entry.role_agreement,
                    Some(expected),
                    "role_agreement {raw} should round trip"
                );
            }
        }

        /// R82-1-04: a recorded publisher read decodes `co_credited_feeds`
        /// with each field.
        #[test]
        fn adr_0082_link_facts_co_credited_feeds_decode_each_field() {
            let feed: Feed = serde_json::from_str(
                r#"{
                    "feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
                    "co_credited_feeds": [
                        {
                            "feed_guid": "other-publisher-feed",
                            "title": "Other Publisher",
                            "roles": ["artist", "label"],
                            "album_count": 3
                        },
                        {
                            "feed_guid": "untitled-publisher-feed",
                            "title": null,
                            "roles": [],
                            "album_count": 1
                        }
                    ]
                }"#,
            )
            .expect("a recorded co_credited_feeds list should decode");
            let co_credited = feed
                .co_credited_feeds
                .expect("the feed should carry co_credited_feeds");
            assert_eq!(co_credited.len(), 2);
            assert_eq!(
                co_credited[0].feed_guid.as_deref(),
                Some("other-publisher-feed")
            );
            assert_eq!(co_credited[0].title.as_deref(), Some("Other Publisher"));
            assert_eq!(
                co_credited[0].roles,
                Some(vec!["artist".to_owned(), "label".to_owned()])
            );
            assert_eq!(co_credited[0].album_count, Some(3));
            assert_eq!(co_credited[1].title, None);
            assert_eq!(co_credited[1].roles, Some(vec![]));
        }

        /// The 0.7.0 contract text still declares `credit` beside
        /// `publisher`. `AlbumNamesAs` keeps it as a known value, and an
        /// unrecognized value keeps its own raw text.
        #[test]
        fn adr_0082_link_facts_album_names_as_known_and_unknown_values() {
            for (raw, expected) in [
                ("publisher", AlbumNamesAs::Publisher),
                ("credit", AlbumNamesAs::Credit),
                (
                    "future_label",
                    AlbumNamesAs::Unknown("future_label".to_owned()),
                ),
            ] {
                let feed: Feed = serde_json::from_str(&format!(
                    r#"{{"publisher": [{{
                        "direction": "music_to_publisher",
                        "publisher_feed_guid": "publisher-feed-1",
                        "album_names_as": "{raw}"
                    }}]}}"#
                ))
                .unwrap_or_else(|error| {
                    panic!("a recorded album_names_as of {raw} must not fail the response: {error}")
                });
                let entry = &feed.publisher.unwrap()[0];
                assert_eq!(
                    entry.album_names_as,
                    Some(expected),
                    "album_names_as {raw} should round trip"
                );
                assert_eq!(entry.album_names_as.as_ref().unwrap().as_str(), raw);
            }
        }

        /// A null `role` round-trips as an absent field, the same as every
        /// other optional `PublisherRelationship` value.
        #[test]
        fn adr_0082_link_facts_null_role_stays_absent_on_round_trip() {
            let feed: Feed = serde_json::from_str(
                r#"{"publisher": [{
                    "direction": "publisher_to_music",
                    "publisher_feed_guid": "publisher-feed-1",
                    "role": null,
                    "role_source": "default"
                }]}"#,
            )
            .expect("a null role must not fail the response");
            let entry = &feed.publisher.unwrap()[0];
            assert_eq!(entry.role, None);
            let serialized = serde_json::to_value(entry).unwrap();
            assert!(
                serialized.get("role").is_none(),
                "a null role should stay absent on round trip"
            );
        }
    }

    /// ADR 0077 Task 007: the confirmed and unconfirmed artist fields of a
    /// publisher feed (Stophammer ADR 0061).
    mod adr_0077_confirmed_artists {
        use crate::api::{DetailResponse, Feed};

        /// Recorded on 2026-09-29 from
        /// `GET https://api.musicindex.org/v1/feeds/137aaa9c-75ff-4916-9f23-e02968b2d15e?include=publisher`,
        /// on Stophammer 0.2.0.
        const RECORDED_PUBLISHER_FEED_WITH_CONFIRMED_ARTISTS: &str = r#"{
            "data": {
                "feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
                "title": "Official DETOX Music",
                "confirmed_release_artist_count": 1,
                "confirmed_release_artists": ["Official DETOX Music"],
                "unconfirmed_release_artist_count": 0,
                "unconfirmed_release_artists": []
            },
            "pagination": {"cursor": null, "has_more": false}
        }"#;

        /// R7-01: the recorded feed response decodes the four fields.
        #[test]
        fn adr_0077_confirmed_artists_recorded_response_decodes_each_field() {
            let response: DetailResponse<Feed> =
                serde_json::from_str(RECORDED_PUBLISHER_FEED_WITH_CONFIRMED_ARTISTS)
                    .expect("the recorded response should decode");
            let feed = response.data;
            assert_eq!(feed.confirmed_release_artist_count, Some(1));
            assert_eq!(
                feed.confirmed_release_artists,
                Some(vec!["Official DETOX Music".to_owned()])
            );
            assert_eq!(feed.unconfirmed_release_artist_count, Some(0));
            assert_eq!(feed.unconfirmed_release_artists, Some(vec![]));
        }

        /// R7-01: an omitted field decodes as absent.
        #[test]
        fn adr_0077_confirmed_artists_omitted_field_decodes_to_none() {
            let feed: Feed = serde_json::from_str(r#"{"feed_guid": "publisher-guid"}"#)
                .expect("a feed with none of the four fields should still decode");
            assert_eq!(feed.confirmed_release_artist_count, None);
            assert_eq!(feed.confirmed_release_artists, None);
            assert_eq!(feed.unconfirmed_release_artist_count, None);
            assert_eq!(feed.unconfirmed_release_artists, None);
        }
    }
}
