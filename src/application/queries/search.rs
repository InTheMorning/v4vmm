//! Search local query family.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::api::SearchResult;
use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::view_models::name_match_page::{NameMatchPageFacts, NameMatchPageVm};
use crate::view_models::search_results::{
    ArtistResultDisplay, FeedResultDisplay, IndexSearchResultRows, SearchResultItemId,
    SearchResultOrigin, TrackResultDisplay,
};
use crate::views::TrackView;
use crate::{db, library_service};

pub const DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT: usize = 50;

/// Fetches remote Index search result rows for presentation.
#[derive(Clone, Debug)]
pub(crate) struct FetchIndexSearchResults {
    endpoint: crate::config::MusicIndexEndpoint,
    query: String,
}

impl FetchIndexSearchResults {
    /// Creates an Index search query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        query: impl Into<String>,
    ) -> Self {
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

/// Fetches the Index name-match track page (ADR 0077 packet 006, Accepted
/// Refinement "Index artist page by name"). The one request is
/// `GET /v1/tracks?artist=<name>`, through the `INDEX_NAME_MATCH_TRACKS`
/// request profile.
#[derive(Clone, Debug)]
pub(crate) struct FetchNameMatchTracks {
    endpoint: crate::config::MusicIndexEndpoint,
    name: String,
}

impl FetchNameMatchTracks {
    /// Creates an Index name-match track page query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            name: name.into(),
        }
    }
}

impl ApplicationCommand for FetchNameMatchTracks {
    type Output = NameMatchPageFacts;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = crate::api::Client::new_with_base_url(self.endpoint);
        fetch_name_match_tracks(&client, &self.name)
            .map_err(|error| query_error(&error))
            .map(CommandOutcome::without_events)
    }
}

/// Sends the one request of the name-match track page: `/v1/tracks` with
/// `artist=<name>`, through the `INDEX_NAME_MATCH_TRACKS` request profile
/// (Required Change 3, R6-03). This function sends no other route.
fn fetch_name_match_tracks(client: &crate::api::Client, name: &str) -> Result<NameMatchPageFacts> {
    let response = client.fetch_tracks_by_artist_with_profile(
        name,
        Some(crate::api::PAGE_LIMIT),
        None,
        &crate::application::request_profiles::INDEX_NAME_MATCH_TRACKS,
    )?;
    Ok(NameMatchPageFacts {
        name: name.to_string(),
        tracks: response.data,
        has_more: response.pagination.has_more,
    })
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

fn fetch_index_search_result_rows(
    endpoint: &crate::config::MusicIndexEndpoint,
    query: &str,
) -> Result<IndexSearchResultRows> {
    let client = crate::api::Client::new_with_base_url(endpoint.clone());
    let mut rows = IndexSearchResultRows::default();
    let mut artists = BTreeMap::new();

    // ADR 0075 packet 047, Required Change 2: each row comes from this
    // search's own summary fields. Neither call below sends a detail
    // request for a returned hit.
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

    /// ADR 0077 packet 006, Accepted Refinement "Index artist page by
    /// name": this row is a search result, never an artist. Its label and
    /// its accessibility label are the same quoted text
    /// `NameMatchPageVm` uses for the page this row opens (R6-01).
    fn into_display(self) -> ArtistResultDisplay {
        let label = NameMatchPageVm::title_text_for_name(&self.name);
        let mut display = ArtistResultDisplay::new(
            format!("index-artist:{}", self.name),
            label,
            SearchResultOrigin::Index,
        );
        display.a11y_label.clone_from(&display.label);
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

/// Asks the shared owner for this feed (ADR 0075 section 6), instead of
/// `Client` directly. P18-2 covers this Index feed detail response: a
/// 15-minute window, exactly as for any other feed identity.
///
/// ADR 0075 packet 047 moved this call out of the search loop: a search
/// draws its rows from summary fields alone. `FetchIndexFeedDetail`, below,
/// is this function's only caller, and it runs once the operator opens
/// the row.
///
/// This file has no observation recorder — the guard in
/// `tests/architecture_tests.rs` forbids one here — so this closure
/// produces no receipts, and the owner's own empty list is expected and
/// discarded.
fn owner_fetch_feed(
    client: &crate::api::Client,
    provider_identity: &str,
    feed_guid: &str,
) -> Result<crate::api::Feed> {
    use crate::application::request_reuse::{self, RefreshIntent, RequestKey, SharedFetchError};
    let profile = &crate::application::request_profiles::INDEX_FEED_DETAIL;
    let key = RequestKey::feed(provider_identity, feed_guid, profile.include());
    request_reuse::shared()
        .fetch_feed_with_receipts(key, RefreshIntent::Normal, || {
            Ok((
                client.fetch_feed_with_profile(feed_guid, profile)?,
                Vec::new(),
            ))
        })
        .0
        .map(|(feed, _receipts)| feed)
        .map_err(SharedFetchError::into_anyhow)
}

/// Fetches one remote Index feed's own detail, sent only when the operator
/// opens its row (ADR 0075 packet 047, Required Change 3). The search that
/// found this row sent no detail request for it.
#[derive(Clone, Debug)]
pub(crate) struct FetchIndexFeedDetail {
    endpoint: crate::config::MusicIndexEndpoint,
    feed_guid: String,
}

impl FetchIndexFeedDetail {
    /// Creates an Index feed detail-on-open query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        feed_guid: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            feed_guid: feed_guid.into(),
        }
    }
}

impl ApplicationCommand for FetchIndexFeedDetail {
    type Output = crate::views::FeedView;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let provider_identity = self
            .endpoint
            .require()
            .map(str::to_owned)
            .unwrap_or_default();
        let client = crate::api::Client::new_with_base_url(self.endpoint);
        let feed = owner_fetch_feed(&client, &provider_identity, &self.feed_guid)
            .map_err(|error| query_error(&error))?;
        Ok(CommandOutcome::without_events(
            crate::views::FeedView::from_api(feed),
        ))
    }
}

/// The search hits in rank order, each entity one time. MusicIndex can
/// return one entity twice with two ranks (seen on 2026-10-07 for "arbiter").
/// A second row would open the same page as the first, so it is dropped.
/// MusicIndex responses are untrusted input.
fn unique_hits(hits: &[SearchResult]) -> impl Iterator<Item = &SearchResult> {
    let mut seen = std::collections::HashSet::new();
    hits.iter()
        .filter(move |hit| seen.insert((hit.entity_type.as_str(), hit.entity_id.as_str())))
}

/// Packet 047 Required Change 2: this search sends no per-hit detail
/// request. Each row's title, artist, track count and artwork come from
/// the search response's own summary fields (R47-02, R47-03, R47-05).
fn fetch_index_feed_result_rows(
    client: &crate::api::Client,
    query: &str,
) -> Result<IndexFeedSearchRows> {
    let response = client.search(query, Some("feed"), Some(crate::api::PAGE_LIMIT), None)?;
    let mut rows = Vec::new();
    let mut artists = Vec::new();

    for (index, hit) in unique_hits(&response.data).enumerate() {
        if let Some(candidate) = index_artist_candidate_from_feed(hit, query) {
            artists.push(candidate);
        }
        rows.push((
            index_item_id(INDEX_FEED_ID_BASE, index),
            index_feed_result_display(hit),
        ));
    }

    Ok(IndexFeedSearchRows { rows, artists })
}

/// Packet 047 Required Change 2: this search sends no per-hit detail
/// request. Each row's title, artist and feed name come from the search
/// response's own summary fields. Its artwork follows ADR 0075 Decision C
/// through `index_track_result_artwork_url` (R47-02, R47-04).
fn fetch_index_track_result_rows(
    client: &crate::api::Client,
    query: &str,
) -> Result<IndexTrackSearchRows> {
    let response = client.search(query, Some("track"), Some(crate::api::PAGE_LIMIT), None)?;
    let mut rows = Vec::new();
    let mut artists = Vec::new();

    for (index, hit) in unique_hits(&response.data).enumerate() {
        artists.extend(index_artist_candidates_from_track(hit, query));
        rows.push((
            index_item_id(INDEX_TRACK_ID_BASE, index),
            index_track_result_display(hit),
        ));
    }

    Ok(IndexTrackSearchRows { rows, artists })
}

/// Builds a feed search-result row directly from the search response's
/// own summary fields (ADR 0075 packet 047, Required Change 2). It shows
/// no `publisher_text`: the summary carries none, and ADR 0077 Decision 6
/// does not make feed owner text an artist.
fn index_feed_result_display(hit: &SearchResult) -> FeedResultDisplay {
    let feed_guid = hit.feed_guid.as_deref().unwrap_or(&hit.entity_id);
    let label = non_empty_str(hit.title.as_deref())
        .map(str::to_string)
        .unwrap_or_else(|| feed_guid.to_string());
    let mut display = FeedResultDisplay::new(
        format!("index-feed:{feed_guid}"),
        label,
        SearchResultOrigin::Index,
    );

    let secondary = count_parts([
        hit.release_artist.clone(),
        hit.episode_count.map(|count| count_label(count, "track")),
    ]);
    if !secondary.is_empty() {
        display = display.with_secondary_text(secondary);
    }
    if let Some(image_url) = non_empty_string(hit.feed_image_url.clone()) {
        display = display.with_thumbnail_href(image_url);
    }
    display
}

/// Builds a track search-result row directly from the search response's
/// own summary fields (ADR 0075 packet 047, Required Change 2).
fn index_track_result_display(hit: &SearchResult) -> TrackResultDisplay {
    let track_guid = hit.entity_id.as_str();
    let feed_guid = hit.feed_guid.as_deref();
    let activation_id = feed_guid.map_or_else(
        || format!("index-track:{track_guid}"),
        |feed_guid| format!("index-track:{feed_guid}:{track_guid}"),
    );
    let label = non_empty_str(hit.title.as_deref())
        .map(str::to_string)
        .unwrap_or_else(|| track_guid.to_string());
    let mut display = TrackResultDisplay::new(activation_id, label, SearchResultOrigin::Index);

    let secondary = count_parts([
        hit.track_artist.clone(),
        hit.release_artist.clone(),
        hit.feed_title.clone(),
    ]);
    if !secondary.is_empty() {
        display = display.with_secondary_text(secondary);
    }
    if let Some(image_url) = index_track_result_artwork_url(hit) {
        display = display.with_thumbnail_href(image_url);
    }
    display
}

/// The artwork URL for a track row built from summary fields alone (ADR
/// 0075 Decision C, packet 047). This reuses the packet 048 choice,
/// `TrackView::display_artwork_url`: the track's own image, then the
/// feed's image, without repeating that order here.
fn index_track_result_artwork_url(hit: &SearchResult) -> Option<String> {
    let probe = crate::views::TrackView {
        track_image_url: hit.track_image_url.clone(),
        feed_image_url: hit.feed_image_url.clone(),
        ..crate::views::TrackView::default()
    };
    probe.display_artwork_url().map(str::to_string)
}

fn index_artist_candidate_from_feed(
    hit: &SearchResult,
    query: &str,
) -> Option<IndexArtistCandidate> {
    let name = non_empty_str(hit.release_artist.as_deref())?;
    index_artist_name_matches_query(name, query).then(|| {
        IndexArtistCandidate::new(
            name,
            1,
            hit.episode_count.unwrap_or_default().max(0),
            non_empty_str(hit.feed_image_url.as_deref()).map(str::to_string),
        )
    })
}

/// R47-09: this candidate's name comes from the summary `track_artist` and
/// `release_artist` fields, not from a fetched track detail.
fn index_artist_candidates_from_track(
    hit: &SearchResult,
    query: &str,
) -> Vec<IndexArtistCandidate> {
    [hit.track_artist.as_deref(), hit.release_artist.as_deref()]
        .into_iter()
        .filter_map(non_empty_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|name| index_artist_name_matches_query(name, query))
        .map(|name| IndexArtistCandidate::new(name, 0, 1, index_track_result_artwork_url(hit)))
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

pub(super) fn index_item_id(base: SearchResultItemId, index: usize) -> SearchResultItemId {
    let offset = u64::try_from(index).unwrap_or(SearchResultItemId::MAX.saturating_sub(base));
    base.saturating_add(offset)
}

/// Names the Index track detail, scoped and Index track detail, unscoped
/// profiles (ADR 0075 packet 017). Both send the full track include list
/// (ADR 0075, amendment of 2026-10-07).
///
/// Asks the shared owner for this track (ADR 0075 section 6), instead of
/// `Client` directly. The Index route carries no accepted reuse window of
/// its own — P18-1 names a Library track detail response only — so this
/// shares an active request (`MetadataRequestOwner::fetch_track_shared`'s
/// own documentation) without retaining a completed one for a later reuse.
///
/// ADR 0075 packet 047 moved this call out of the search loop: a search
/// draws its rows from summary fields alone. `FetchIndexTrackDetail`,
/// below, is this function's only caller, and it runs once the operator
/// opens the row.
fn fetch_index_track_detail(
    client: &crate::api::Client,
    provider_identity: &str,
    track_guid: &str,
    feed_guid: Option<&str>,
) -> Result<crate::api::Track> {
    use crate::application::request_profiles::{
        INDEX_TRACK_DETAIL_SCOPED, INDEX_TRACK_DETAIL_UNSCOPED,
    };
    use crate::application::request_reuse::{self, RequestKey};
    let feed_guid = feed_guid.map(str::trim).filter(|guid| !guid.is_empty());
    match feed_guid {
        Some(feed_guid) => {
            let key = RequestKey::scoped_track(
                provider_identity,
                feed_guid,
                track_guid,
                INDEX_TRACK_DETAIL_SCOPED.include(),
            );
            request_reuse::shared().fetch_track_shared(key, || {
                client.fetch_feed_track_with_profile(
                    feed_guid,
                    track_guid,
                    &INDEX_TRACK_DETAIL_SCOPED,
                )
            })
        }
        None => {
            let key = RequestKey::unscoped_track(
                provider_identity,
                track_guid,
                INDEX_TRACK_DETAIL_UNSCOPED.include(),
            );
            request_reuse::shared().fetch_track_shared(key, || {
                client.fetch_track_with_profile(track_guid, &INDEX_TRACK_DETAIL_UNSCOPED)
            })
        }
    }
}

/// Fetches one remote Index track's own detail, sent only when the
/// operator opens its row (ADR 0075 packet 047, Required Change 3). The
/// search that found this row sent no detail request for it. `feed_guid`
/// scopes the request when the row's summary carried one; an unscoped hit
/// carries `None`.
#[derive(Clone, Debug)]
pub(crate) struct FetchIndexTrackDetail {
    endpoint: crate::config::MusicIndexEndpoint,
    track_guid: String,
    feed_guid: Option<String>,
}

impl FetchIndexTrackDetail {
    /// Creates an Index track detail-on-open query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        track_guid: impl Into<String>,
        feed_guid: Option<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            track_guid: track_guid.into(),
            feed_guid,
        }
    }
}

impl ApplicationCommand for FetchIndexTrackDetail {
    type Output = TrackView;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let provider_identity = self
            .endpoint
            .require()
            .map(str::to_owned)
            .unwrap_or_default();
        let client = crate::api::Client::new_with_base_url(self.endpoint);
        let track = fetch_index_track_detail(
            &client,
            &provider_identity,
            &self.track_guid,
            self.feed_guid.as_deref(),
        )
        .map_err(|error| query_error(&error))?;
        // ADR 0024: the detail-on-open path carries a rich `TrackView`, not
        // the raw decoded response, to the page that shows it.
        let remote_track = TrackView::from_api(track);
        Ok(CommandOutcome::without_events(remote_track))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// MusicIndex gave one track twice on 2026-10-07. The rows keep the
    /// first hit of each entity, in rank order.
    #[test]
    fn unique_hits_keep_the_first_hit_of_each_entity() {
        let hit = |entity_type: &str, entity_id: &str, rank: f64| SearchResult {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            rank: Some(rank),
            ..SearchResult::default()
        };
        let hits = vec![
            hit("track", "t1", -12.5),
            hit("track", "t1", -11.1),
            hit("track", "t2", -10.0),
            hit("feed", "t1", -9.0),
        ];
        let kept: Vec<_> = unique_hits(&hits)
            .map(|hit| (hit.entity_type.as_str(), hit.entity_id.as_str(), hit.rank))
            .collect();
        assert_eq!(
            kept,
            vec![
                ("track", "t1", Some(-12.5)),
                ("track", "t2", Some(-10.0)),
                ("feed", "t1", Some(-9.0)),
            ]
        );
    }

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

    /// R47-03, R47-04 (ADR 0075 packet 047): a track row built from summary
    /// fields alone shows its title, `track_artist`, `release_artist` and
    /// `feed_title`, and its artwork follows Decision C: `track_image_url`
    /// first, then `feed_image_url`. It carries no fetched `TrackView`; the
    /// search sent no detail request for it.
    #[test]
    fn adr_0075_search_summary_track_row_uses_summary_fields_and_decision_c_artwork() {
        let with_track_image = SearchResult {
            entity_type: "track".to_string(),
            entity_id: "track-guid".to_string(),
            feed_guid: Some("feed-guid".to_string()),
            title: Some("Remote Track".to_string()),
            feed_title: Some("Remote Release".to_string()),
            track_artist: Some("Track Artist".to_string()),
            track_image_url: Some("https://example.test/track.jpg".to_string()),
            feed_image_url: Some("https://example.test/feed.jpg".to_string()),
            ..SearchResult::default()
        };
        let display = index_track_result_display(&with_track_image);

        assert_eq!(display.label, "Remote Track");
        assert_eq!(display.secondary_text, "Track Artist - Remote Release");
        assert_eq!(
            display.thumbnail_href.as_deref(),
            Some("https://example.test/track.jpg"),
            "R47-04: a track_image_url must win over feed_image_url"
        );
        assert!(
            display.remote_track.is_none(),
            "the search must attach no fetched TrackView to the row"
        );

        let feed_image_only = SearchResult {
            feed_image_url: Some("https://example.test/feed.jpg".to_string()),
            ..with_track_image
        };
        let feed_fallback = index_track_result_display(&SearchResult {
            track_image_url: None,
            ..feed_image_only
        });
        assert_eq!(
            feed_fallback.thumbnail_href.as_deref(),
            Some("https://example.test/feed.jpg"),
            "R47-04: with no track_image_url, the row must use feed_image_url"
        );
    }

    /// R47-03 (ADR 0075 packet 047): a feed row built from summary fields
    /// alone shows its title, `release_artist`, its track count and its
    /// feed artwork. It shows no `publisher_text`, because the summary
    /// carries none and ADR 0077 Decision 6 does not make feed owner text
    /// an artist.
    #[test]
    fn adr_0075_search_summary_r47_03_feed_row_uses_summary_fields_with_no_publisher_text() {
        let hit = SearchResult {
            entity_type: "feed".to_string(),
            entity_id: "f1".to_string(),
            feed_guid: Some("f1".to_string()),
            title: Some("Monster".to_string()),
            release_artist: Some("Official DETOX Music".to_string()),
            episode_count: Some(3),
            feed_image_url: Some("https://example.test/feed.jpg".to_string()),
            ..SearchResult::default()
        };

        let display = index_feed_result_display(&hit);

        assert_eq!(display.label, "Monster");
        assert_eq!(display.secondary_text, "Official DETOX Music - 3 tracks");
        assert_eq!(
            display.thumbnail_href.as_deref(),
            Some("https://example.test/feed.jpg")
        );
        assert!(
            !display.secondary_text.contains("publisher"),
            "the summary carries no publisher_text, so the row must show none"
        );
    }

    /// R47-05: a feed hit without `feed_guid` opens the feed of its
    /// `entity_id`.
    #[test]
    fn adr_0075_search_summary_r47_05_feed_hit_without_feed_guid_uses_entity_id() {
        let hit = SearchResult {
            entity_type: "feed".to_string(),
            entity_id: "entity-only".to_string(),
            title: Some("No Feed Guid".to_string()),
            ..SearchResult::default()
        };

        let display = index_feed_result_display(&hit);

        assert_eq!(display.id, "index-feed:entity-only");
    }

    /// R47-09: an Index name candidate's name comes from the search
    /// response's own `release_artist` and `track_artist` summary fields,
    /// not from a fetched detail.
    #[test]
    fn adr_0075_search_summary_r47_09_name_candidates_come_from_summary_fields() {
        let feed_hit = SearchResult {
            entity_type: "feed".to_string(),
            entity_id: "f1".to_string(),
            release_artist: Some("Survival Guide".to_string()),
            episode_count: Some(4),
            feed_image_url: Some("https://example.test/feed.jpg".to_string()),
            ..SearchResult::default()
        };
        let feed_candidate = index_artist_candidate_from_feed(&feed_hit, "survival")
            .expect("a matching release_artist should produce a candidate");
        assert_eq!(feed_candidate.name, "Survival Guide");
        assert_eq!(feed_candidate.feed_count, 1);
        assert_eq!(feed_candidate.track_count, 4);

        let track_hit = SearchResult {
            entity_type: "track".to_string(),
            entity_id: "t1".to_string(),
            track_artist: Some("Survival Guide".to_string()),
            release_artist: Some("Album Artist".to_string()),
            ..SearchResult::default()
        };
        let track_candidates = index_artist_candidates_from_track(&track_hit, "survival");
        assert_eq!(
            track_candidates
                .iter()
                .map(|candidate| candidate.name.clone())
                .collect::<Vec<_>>(),
            vec!["Survival Guide".to_string()]
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

/// ADR 0075 packet 017: request-profile behavior at the Index feed result
/// rows call site.
#[cfg(test)]
mod adr_0075_request_profile_tests {
    use super::*;
    use crate::application::request_profiles;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// A minimal local HTTP server that records each request path.
    struct Fixture {
        endpoint: crate::config::MusicIndexEndpoint,
        address: String,
        requests: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl Fixture {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap().to_string();
            let endpoint: crate::config::MusicIndexEndpoint = format!("http://{address}").into();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let received = Arc::clone(&requests);
            let stopped = Arc::clone(&stop);
            let worker = std::thread::spawn(move || {
                while !stopped.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let mut bytes = Vec::new();
                            let mut buffer = [0; 4096];
                            while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                                match stream.read(&mut buffer) {
                                    Ok(0) | Err(_) => break,
                                    Ok(read) => bytes.extend_from_slice(&buffer[..read]),
                                }
                            }
                            let request = String::from_utf8_lossy(&bytes);
                            let Some(path) = request
                                .lines()
                                .next()
                                .and_then(|line| line.split_whitespace().nth(1))
                            else {
                                continue;
                            };
                            received.lock().unwrap().push(path.to_string());
                            let body = response(path);
                            write!(
                                stream,
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                body.len()
                            )
                            .unwrap();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(error) => panic!("fixture listener: {error}"),
                    }
                }
            });
            Self {
                endpoint,
                address,
                requests,
                stop,
                worker: Some(worker),
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = std::net::TcpStream::connect(&self.address);
            self.worker.take().unwrap().join().unwrap();
        }
    }

    fn response(path: &str) -> String {
        let bare = path.split('?').next().unwrap_or(path);
        if bare == "/v1/search" {
            return serde_json::json!({
                "data": [{"entity_type": "feed", "entity_id": "f1", "feed_guid": "f1"}],
                "pagination": {"has_more": false}
            })
            .to_string();
        }
        if bare == "/v1/tracks" {
            return serde_json::json!({
                "data": [{"track_guid": "t1", "feed_guid": "f1", "title": "Track One"}],
                "pagination": {"has_more": false}
            })
            .to_string();
        }
        // ADR 0075 packet 047: a track detail-on-open request names its
        // track GUID in the path. The plain `/v1/tracks` search path above
        // is matched first, so this only serves a scoped or unscoped
        // single-track request.
        if let Some(track_guid) = bare.rsplit_once("/tracks/").map(|(_, id)| id) {
            return serde_json::json!({
                "data": {"track_guid": track_guid, "feed_guid": "f1", "title": "Track One"}
            })
            .to_string();
        }
        serde_json::json!({"data": {"feed_guid": "f1", "title": "Feed"}}).to_string()
    }

    fn encoded_include(profile: crate::application::request_profiles::RequestProfile) -> String {
        profile
            .include()
            .map(|include| include.replace(',', "%2C"))
            .unwrap_or_default()
    }

    /// R47-02, R47-08: an Index feed search sends one request, the search
    /// itself, with no `fuzzy` parameter and no per-hit detail request.
    /// This replaces the pre-packet-047 count of one search request plus
    /// one feed detail request for each returned hit.
    #[test]
    fn adr_0075_search_summary_r47_02_feed_search_sends_no_detail_request() {
        let fixture = Fixture::start();
        let client = crate::api::Client::new_with_base_url(fixture.endpoint.clone());

        let rows = fetch_index_feed_result_rows(&client, "needle").unwrap();

        assert_eq!(rows.rows.len(), 1);
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests.len(),
            1,
            "the search must send no per-hit feed detail request. Got: {requests:?}"
        );
        assert!(requests[0].starts_with("/v1/search?"), "Got: {requests:?}");
        assert!(
            !requests[0].contains("fuzzy"),
            "R47-08: the search request must hold no fuzzy parameter. Got: {requests:?}"
        );
    }

    /// R47-02, R47-08: an Index track search sends one request, with no
    /// `fuzzy` parameter and no per-hit detail request.
    #[test]
    fn adr_0075_search_summary_r47_02_track_search_sends_no_detail_request() {
        let fixture = Fixture::start();
        let client = crate::api::Client::new_with_base_url(fixture.endpoint.clone());

        let rows = fetch_index_track_result_rows(&client, "needle").unwrap();

        assert_eq!(rows.rows.len(), 1);
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests.len(),
            1,
            "the search must send no per-hit track detail request. Got: {requests:?}"
        );
        assert!(
            !requests[0].contains("fuzzy"),
            "R47-08: the search request must hold no fuzzy parameter. Got: {requests:?}"
        );
    }

    /// R47-06: opening a feed row sends its own detail request, with the
    /// existing `INDEX_FEED_DETAIL` profile (L2, plus `publisher` under
    /// ADR 0077 Decision 5). A second open inside packet 018's 15-minute
    /// reuse window (P18-2) sends none.
    #[test]
    fn adr_0075_search_summary_r47_06_feed_detail_sends_one_request_and_reuses_it() {
        let fixture = Fixture::start();

        let first = FetchIndexFeedDetail::new(fixture.endpoint.clone(), "f1")
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0)
            .unwrap();
        let second = FetchIndexFeedDetail::new(fixture.endpoint.clone(), "f1")
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0)
            .unwrap();

        assert_eq!(first.feed_guid.as_deref(), Some("f1"));
        assert_eq!(second.feed_guid.as_deref(), Some("f1"));
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests.len(),
            1,
            "a second open inside the reuse window must send no request. Got: {requests:?}"
        );
        assert!(
            requests[0].starts_with(&format!(
                "/v1/feeds/f1?include={}",
                encoded_include(request_profiles::INDEX_FEED_DETAIL)
            )) && requests[0].contains("%2Cpublisher"),
            "the detail request must use the existing INDEX_FEED_DETAIL profile, \
including publisher (ADR 0077 Decision 5). Got: {requests:?}"
        );
    }

    /// R47-06: opening a track row sends its own detail request, scoped by
    /// the feed GUID the row's summary carried, with the full track include
    /// list (ADR 0075, amendment of 2026-10-07).
    #[test]
    fn adr_0075_search_summary_r47_06_track_detail_sends_its_own_scoped_request() {
        let fixture = Fixture::start();

        let track = FetchIndexTrackDetail::new(fixture.endpoint.clone(), "t1", Some("f1".into()))
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0)
            .unwrap();

        assert_eq!(track.track_guid.as_deref(), Some("t1"));
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 1, "Got: {requests:?}");
        assert_eq!(
            requests[0],
            format!(
                "/v1/feeds/f1/tracks/t1?include={}",
                encoded_include(crate::application::request_profiles::INDEX_TRACK_DETAIL_SCOPED)
            ),
            "ADR 0075, amendment of 2026-10-07: the scoped track detail request asks for every track collection"
        );
    }

    /// R6-01 (ADR 0077 packet 006): an Index name candidate exposes the
    /// label `Tracks matching "<name>"` and an equal accessibility label.
    #[test]
    fn adr_0077_name_matches_candidate_label_is_the_quoted_name_and_equals_its_a11y_label() {
        let display = IndexArtistCandidate::new("Survival Guide", 2, 5, None).into_display();

        assert_eq!(display.label, "Tracks matching \"Survival Guide\"");
        assert_eq!(display.a11y_label, display.label);
    }

    /// R6-01: the row keeps its secondary count text and its thumbnail.
    #[test]
    fn adr_0077_name_matches_candidate_keeps_its_secondary_text_and_thumbnail() {
        let display = IndexArtistCandidate::new(
            "Survival Guide",
            2,
            5,
            Some("https://example.test/art.jpg".to_string()),
        )
        .into_display();

        assert_eq!(display.secondary_text, "2 feeds - 5 tracks");
        assert_eq!(
            display.thumbnail_href.as_deref(),
            Some("https://example.test/art.jpg")
        );
    }

    /// R6-03: the name-match track page sends exactly one request,
    /// `/v1/tracks` with `artist=<name>`, through the
    /// `INDEX_NAME_MATCH_TRACKS` request profile, and no other route.
    #[test]
    fn adr_0077_name_matches_page_command_sends_one_tracks_by_artist_request() {
        let fixture = Fixture::start();
        let client = crate::api::Client::new_with_base_url(fixture.endpoint.clone());

        let facts = fetch_name_match_tracks(&client, "DETOX").unwrap();

        assert_eq!(facts.name, "DETOX");
        assert_eq!(facts.tracks.len(), 1);
        assert!(!facts.has_more);
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 1, "the page must send no other route");
        assert!(
            requests[0].starts_with("/v1/tracks?") && requests[0].contains("artist=DETOX"),
            "R6-03: the page must request /v1/tracks with artist=<name>. Got: {requests:?}"
        );
    }
}
