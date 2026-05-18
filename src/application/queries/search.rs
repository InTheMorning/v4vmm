//! Search local query family.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::api::{Client, EntityDetail, SearchResult, PAGE_LIMIT};
use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::feed_service;
use crate::view_models::search::{
    artist_rows_from_result_rows, search_result_type_is_visible, ResultRow, SearchBatch,
    SearchViewModel,
};
use crate::view_models::search_results::{
    ArtistResultDisplay, FeedResultDisplay, IndexSearchResultRows, SearchResultItemId,
    SearchResultOrigin, TrackResultDisplay,
};
use crate::view_models::workspace::ContentFilter;
use crate::views::TrackView;
use crate::{db, library_service};

pub const DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT: usize = 50;

type SharedConnection = Arc<Mutex<Connection>>;

/// Neutral output for one Discover global search load.
#[derive(Clone, Debug)]
pub(crate) struct DiscoverSearchResults {
    pub(crate) library_rows: Vec<ResultRow>,
    pub(crate) index_batch: Option<SearchBatch>,
}

/// Fetches Discover global search rows without binding to GPUI or Discover.
#[derive(Clone, Debug)]
pub(crate) struct FetchDiscoverSearchResults {
    conn: SharedConnection,
    query_service: Arc<ApplicationQueryService>,
    endpoint: String,
    query: String,
    filter: ContentFilter,
    append: bool,
    type_filter: usize,
    cursor: Option<String>,
    fuzzy: bool,
}

impl FetchDiscoverSearchResults {
    /// Creates a Discover global search query command.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "captures the existing SearchViewModel load intent without inventing a new app-layer intent type"
    )]
    pub(crate) fn new(
        conn: SharedConnection,
        query_service: Arc<ApplicationQueryService>,
        endpoint: impl Into<String>,
        query: impl Into<String>,
        filter: ContentFilter,
        append: bool,
        type_filter: usize,
        cursor: Option<String>,
        fuzzy: bool,
    ) -> Self {
        Self {
            conn,
            query_service,
            endpoint: endpoint.into(),
            query: query.into(),
            filter,
            append,
            type_filter,
            cursor,
            fuzzy,
        }
    }
}

impl ApplicationCommand for FetchDiscoverSearchResults {
    type Output = DiscoverSearchResults;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let result = fetch_discover_search_results(self).map_err(|error| query_error(&error))?;
        Ok(CommandOutcome::without_events(result))
    }
}

/// Fetches remote Index search result rows for presentation.
#[derive(Clone, Debug)]
pub(crate) struct FetchIndexSearchResults {
    endpoint: String,
    query: String,
}

impl FetchIndexSearchResults {
    /// Creates an Index search query command.
    #[must_use]
    pub(crate) fn new(endpoint: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            query: query.into(),
        }
    }
}

impl ApplicationCommand for FetchIndexSearchResults {
    type Output = IndexSearchResultRows;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let rows = fetch_index_search_result_rows(&self.endpoint, &self.query)
            .map_err(|error| query_error(&error))?;
        Ok(CommandOutcome::without_events(rows))
    }
}

impl ApplicationQueryService {
    /// Searches in-library local tracks for global search.
    ///
    /// # Errors
    ///
    /// Returns an error when local library state cannot be read.
    pub fn search_local_library_tracks(
        &self,
        conn: &Connection,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<db::TrackRow>, CommandError> {
        let Some(query) = normalized_global_search_query(query) else {
            return Ok(Vec::new());
        };
        library_service::search_library_tracks(
            conn,
            &query,
            limit.unwrap_or(DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT),
        )
        .map_err(|error| query_error(&error))
    }
}

fn normalized_global_search_query(value: &str) -> Option<String> {
    let query = value.trim();
    if query.chars().any(char::is_alphanumeric) {
        Some(query.to_string())
    } else {
        None
    }
}

fn query_error(error: &anyhow::Error) -> CommandError {
    CommandError::Query(format!("{error:#}"))
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

fn fetch_discover_search_results(
    command: FetchDiscoverSearchResults,
) -> Result<DiscoverSearchResults> {
    let entity_type = if command.filter == ContentFilter::Library {
        None
    } else {
        SearchViewModel::type_filter_value(command.type_filter).map(str::to_string)
    };
    let client = Client::new_with_base_url(command.endpoint);
    let library_rows = if !command.append
        && matches!(command.filter, ContentFilter::All | ContentFilter::Library)
    {
        fetch_local_library_search_rows(&command.conn, &command.query_service, &command.query)?
    } else {
        Vec::new()
    };
    let index_batch = if matches!(command.filter, ContentFilter::All | ContentFilter::Index) {
        Some(fetch_search_batch(
            &client,
            &command.query,
            entity_type.as_deref(),
            command.cursor.as_deref(),
            command.fuzzy,
        )?)
    } else {
        None
    };

    Ok(DiscoverSearchResults {
        library_rows,
        index_batch,
    })
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
    let mut rows = response
        .data
        .iter()
        .map(|hit| search_hit_to_result_row(client, hit))
        .filter(|row| search_result_type_is_visible(&row.entity_type))
        .collect::<Vec<_>>();
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
    conn: &SharedConnection,
    query_service: &ApplicationQueryService,
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
            Some(EntityDetail::Artist(artist)) => artist
                .name
                .clone()
                .or_else(|| artist.artist_id.clone())
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
            .filter_map(|track| {
                track
                    .feed_guid
                    .as_deref()
                    .map(str::trim)
                    .filter(|guid| !guid.is_empty())
                    .map(str::to_string)
            })
            .collect();
        let track_total = bounded_i32_count(tracks.len());
        let feed_total = bounded_i32_count(distinct_feeds.len());
        let first_feed_image = distinct_feeds
            .iter()
            .next()
            .and_then(|guid| client.fetch_feed(guid, None).ok())
            .and_then(|feed| feed.image_url);

        if let Some(EntityDetail::Artist(artist)) = row.detail.as_mut() {
            artist.track_count = Some(track_total);
            artist.feed_count = Some(feed_total);
            if artist.image_url.is_none() {
                artist.image_url = first_feed_image;
            }
        }
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
) -> Result<crate::api::Track> {
    match feed_guid.map(str::trim).filter(|guid| !guid.is_empty()) {
        Some(feed_guid) => client.fetch_feed_track(feed_guid, track_guid, include),
        None => client.fetch_track(track_guid, include),
    }
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
pub(super) const INDEX_FEED_ID_BASE: SearchResultItemId = 2_000_000_000;
const INDEX_TRACK_ID_BASE: SearchResultItemId = 3_000_000_000;
pub(super) const INDEX_FEED_DETAIL_INCLUDE: &str = "tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes";

pub(super) fn index_item_id(base: SearchResultItemId, index: usize) -> SearchResultItemId {
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

pub(super) fn index_feed_display(
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

pub(super) fn non_empty_str(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn non_empty_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn bounded_i32_count(len: usize) -> i32 {
    i32::try_from(len).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Contributor, EntityDetail, SourceEntityId, SourceEntityLink, Track};

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    #[test]
    fn search_queries_return_local_library_matches() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "Needle Feed")?;
        let track_id = create_track(
            &conn,
            feed_id,
            SearchTrack {
                title: "Quiet Track",
                artist: "Alice",
                album: "Needle Album",
                album_artist: "Album Ensemble",
                in_library: true,
            },
        )?;

        let rows =
            ApplicationQueryService::new().search_local_library_tracks(&conn, "needle", None)?;

        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![track_id]
        );

        Ok(())
    }

    #[test]
    fn search_queries_exclude_tracks_not_in_library() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "Needle Feed")?;
        create_track(
            &conn,
            feed_id,
            SearchTrack {
                title: "Needle Track",
                artist: "Alice",
                album: "Album",
                album_artist: "Album Ensemble",
                in_library: false,
            },
        )?;

        let rows =
            ApplicationQueryService::new().search_local_library_tracks(&conn, "needle", None)?;

        assert!(rows.is_empty());

        Ok(())
    }

    #[test]
    fn search_queries_apply_default_and_explicit_limits() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "Limit Feed")?;
        for index in 0..60 {
            create_track(
                &conn,
                feed_id,
                SearchTrack {
                    title: &format!("Limit Track {index:02}"),
                    artist: "Artist",
                    album: "Album",
                    album_artist: "Album Ensemble",
                    in_library: true,
                },
            )?;
        }
        let service = ApplicationQueryService::new();

        assert_eq!(
            service
                .search_local_library_tracks(&conn, "limit", None)?
                .len(),
            DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT
        );
        assert_eq!(
            service
                .search_local_library_tracks(&conn, "limit", Some(3))?
                .len(),
            3
        );

        Ok(())
    }

    #[test]
    fn search_queries_ignore_non_search_terms() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "Symbols")?;
        create_track(
            &conn,
            feed_id,
            SearchTrack {
                title: "Symbols",
                artist: "Artist",
                album: "Album",
                album_artist: "Album Ensemble",
                in_library: true,
            },
        )?;

        let rows =
            ApplicationQueryService::new().search_local_library_tracks(&conn, " *** ", None)?;

        assert!(rows.is_empty());

        Ok(())
    }

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

    struct SearchTrack<'a> {
        title: &'a str,
        artist: &'a str,
        album: &'a str,
        album_artist: &'a str,
        in_library: bool,
    }

    fn create_feed(conn: &Connection, title: &str) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![
                format!("https://example.test/{title}.xml"),
                format!("{title}-guid"),
                title
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(
        conn: &Connection,
        feed_id: i64,
        track: SearchTrack<'_>,
    ) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (
                feed_id, item_guid, track_title, artist_name, album_title,
                album_artist_name, is_in_library
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                feed_id,
                format!("{}-guid", track.title),
                track.title,
                track.artist,
                track.album,
                track.album_artist,
                i64::from(track.in_library),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }
}
