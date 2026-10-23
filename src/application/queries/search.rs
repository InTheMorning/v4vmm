//! Search local query family.
//!
//! ADR 0087 packet 002: the Library search converts the typed query into
//! one `db::LibraryTrackSearch`. The Index search sends only the request of
//! `IndexSearchPlan::for_query`.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::api::SearchResult;
use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::{CancellationToken, CommandContext};
use crate::application::errors::command::CommandError;
use crate::view_models::artist_identity::{NameFeedMatch, NameFeedMatches, UNKNOWN_ARTIST_TEXT};
use crate::view_models::artists_everything::FeedLinkClass;
use crate::view_models::index_name_list::name_sort_key;
use crate::view_models::library_search_facts::LibrarySearchFacts;
use crate::view_models::name_level_page::{
    NameLevelIndexRelease, NameLevelIndexState, NameLevelPageVm,
};
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::search_query::{
    ContentValue, FieldTerm, FieldValue, FieldValueState, LibraryStateValue, QueryFieldName,
    QueryTerm, SearchQuery, TermKind,
};
use crate::view_models::search_results::{
    artist_feed_result_display, index_track_row_id, library_part_runs, ArtistResultDisplay,
    FeedResultDisplay, IndexSearchRequest, IndexSearchResultRows, LibrarySearchRows,
    SearchResultItemId, SearchResultOrigin, TrackResultDisplay, INDEX_FEED_PAGE_RESULT_PREFIX,
};
use crate::view_models::workspace::{ReleaseKey, TrackKey};
use crate::views::TrackView;
use crate::{db, library_service};

pub const DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT: usize = 50;

/// Fetches remote Index search result rows for presentation.
#[derive(Clone, Debug)]
pub(crate) struct FetchIndexSearchResults {
    endpoint: crate::config::MusicIndexEndpoint,
    request: IndexSearchRequest,
    matches: NameFeedMatches,
}

impl FetchIndexSearchResults {
    /// Creates an Index search query command. `request` comes from
    /// `IndexSearchPlan::for_query`, so its `q` holds no field syntax (ADR
    /// 0087 Decision 5). `matches` is the match of each name to the artist
    /// feeds of the Library, read when the search starts (ADR 0084 Decision
    /// 7, packet 017 decision D14).
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        request: IndexSearchRequest,
        matches: NameFeedMatches,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            request,
            matches,
        }
    }
}

impl ApplicationCommand for FetchIndexSearchResults {
    type Output = IndexSearchResultRows;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let rows = fetch_index_search_result_rows(
            &self.endpoint,
            &self.request.q,
            &self.matches,
            context.cancellation(),
        )?;
        Ok(CommandOutcome::without_events(rows))
    }
}

/// Fetches the `MusicIndex` releases of one name (ADR 0084 Decision 7,
/// packet 017). Each request is `GET /v1/feeds/by-name` with
/// `artist_key=<key>` and `limit=200`, through the
/// `INDEX_NAME_LEVEL_RELEASES` request profile.
#[derive(Clone, Debug)]
pub(crate) struct FetchNameLevelReleases {
    endpoint: crate::config::MusicIndexEndpoint,
    key: String,
}

impl FetchNameLevelReleases {
    /// Creates the query command of the name key `key`.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            key: key.into(),
        }
    }
}

impl ApplicationCommand for FetchNameLevelReleases {
    type Output = NameLevelIndexState;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = crate::api::Client::new_with_base_url(self.endpoint);
        fetch_name_level_releases(&client, &self.key)
            .map_err(|error| query_error(&error))
            .map(CommandOutcome::without_events)
    }
}

/// The row count of one name level request (packet 017 decision D10). It
/// is the `limit` maximum of `/v1/feeds/by-name`.
pub(crate) const NAME_LEVEL_PAGE_LIMIT: i32 = 200;
/// The request count of one name level list at most (packet 017 decision
/// D10).
pub(crate) const NAME_LEVEL_MAX_REQUESTS: usize = 5;

/// Sends the requests of one name level list (packet 017 "The `MusicIndex`
/// request"). It follows `pagination.cursor` while `has_more` is true, for
/// [`NAME_LEVEL_MAX_REQUESTS`] requests at most. After the last request
/// with `has_more` true, it sets `more_available`. The key of "Unknown
/// Artist" sends no request. A row with no feed GUID is dropped. The
/// response is untrusted input, and this function sends no other request
/// for a row.
fn fetch_name_level_releases(
    client: &crate::api::Client,
    key: &str,
) -> Result<NameLevelIndexState> {
    if key == crate::view_models::index_name_list::name_sort_key(UNKNOWN_ARTIST_TEXT) {
        return Ok(NameLevelIndexState::NotRequested);
    }
    let mut releases = Vec::new();
    let mut cursor: Option<String> = None;
    let mut more_available = false;
    for request in 0..NAME_LEVEL_MAX_REQUESTS {
        let response = client.fetch_feeds_by_name(
            NAME_LEVEL_PAGE_LIMIT,
            cursor.as_deref(),
            Some(key),
            &crate::application::request_profiles::INDEX_NAME_LEVEL_RELEASES,
        )?;
        releases.extend(response.data.into_iter().filter_map(|feed| {
            let feed_guid = non_empty_string(feed.feed_guid)?;
            Some(NameLevelIndexRelease {
                feed_guid,
                title: feed.title,
                image_url: non_empty_string(feed.image_url),
                release_artist: feed.release_artist,
                episode_count: feed.episode_count,
                newest_item_at: feed.newest_item_at,
            })
        }));
        let next = response
            .pagination
            .cursor
            .filter(|_| response.pagination.has_more);
        match next {
            Some(next) if request + 1 < NAME_LEVEL_MAX_REQUESTS => cursor = Some(next),
            Some(_) => more_available = true,
            None => break,
        }
    }
    Ok(NameLevelIndexState::Loaded {
        releases,
        more_available,
    })
}

/// Whether `MusicIndex` holds a `name` row with the key `key` (packet 017
/// decision D15). It sends one `GET /v1/artist-names?from=<key>&limit=1`
/// through the `INDEX_ARTIST_NAME_CHECK` request profile. The row shows
/// when the first row has the kind `name` and the same key. A failed check
/// shows the row, and the page then shows its failure report.
fn index_name_row_exists(client: &crate::api::Client, key: &str) -> bool {
    match client.fetch_artist_names(
        1,
        None,
        Some(key),
        &crate::application::request_profiles::INDEX_ARTIST_NAME_CHECK,
    ) {
        Ok(response) => response
            .data
            .first()
            .is_some_and(|row| row.kind == crate::api::ArtistNameKind::Name && row.key == key),
        Err(_) => true,
    }
}

impl ApplicationQueryService {
    /// Searches the Library tracks with each term of the typed query (ADR
    /// 0087 Decision 2). A query whose Library part does not run gives no
    /// row and runs no SQL. With a date term, the rows split after SQL: the
    /// release rows read the release date, and the track rows read their
    /// own `pub_date` (packet 002 decision D6).
    ///
    /// # Errors
    ///
    /// Returns an error when local library state cannot be read.
    pub(crate) fn search_local_library_tracks(
        &self,
        conn: &Connection,
        query: &SearchQuery,
        facts: &LibrarySearchFacts,
        limit: Option<usize>,
    ) -> Result<LibrarySearchRows, CommandError> {
        if !library_part_runs(query) {
            return Ok(LibrarySearchRows::default());
        }
        let plan = library_track_search(query, facts);
        let rows = library_service::search_library_tracks(
            conn,
            &plan.search,
            limit.unwrap_or(DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT),
        )
        .map_err(|error| query_error(&error))?;
        let matched_count = rows.len();
        let Some(split) = plan.date_split else {
            return Ok(LibrarySearchRows {
                releases: rows.clone(),
                tracks: rows,
                matched_count,
            });
        };
        let (releases, tracks) = rows.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut releases, mut tracks), row| {
                if split.release_track_ids.contains(&row.id) {
                    releases.push(row.clone());
                }
                if split.track_track_ids.contains(&row.id) {
                    tracks.push(row);
                }
                (releases, tracks)
            },
        );
        Ok(LibrarySearchRows {
            releases,
            tracks,
            matched_count,
        })
    }
}

/// The Library search of one typed query (ADR 0087 packet 002).
#[derive(Clone, Debug, Default)]
pub(crate) struct LibrarySearchPlan {
    search: db::LibraryTrackSearch,
    date_split: Option<DateSplit>,
}

/// The two date sets of a query with date terms (packet 002 decision D6).
#[derive(Clone, Debug, Default)]
pub(crate) struct DateSplit {
    /// The tracks whose release date passes each date term.
    release_track_ids: BTreeSet<i64>,
    /// The tracks whose own `pub_date` passes each date term.
    track_track_ids: BTreeSet<i64>,
}

/// Converts the typed query into the clauses of the Library search (ADR
/// 0087 Decision 2). Each word, each phrase and each correct field gives at
/// most one clause. A field with an empty value or an error applies nothing
/// (packet 002 decision D14). The facts that SQL does not hold become bound
/// sets of track ids: `kind:`, `is:ready` and the date terms.
pub(crate) fn library_track_search(
    query: &SearchQuery,
    facts: &LibrarySearchFacts,
) -> LibrarySearchPlan {
    let mut clauses = Vec::new();
    let mut date_terms = Vec::new();
    for term in query.terms() {
        let kind = match &term.kind {
            TermKind::Word(text) | TermKind::Phrase(text) => {
                Some(db::LibraryClauseKind::AnyText(text.clone()))
            }
            TermKind::Field(field) => {
                if let Some(date) = date_term(term, field) {
                    date_terms.push(date);
                    None
                } else {
                    field_clause(field, facts)
                }
            }
        };
        if let Some(kind) = kind {
            clauses.push(db::LibraryTrackClause {
                excluded: term.excluded,
                kind,
            });
        }
    }
    let date_split = (!date_terms.is_empty()).then(|| {
        let passes = |date: Option<i64>| date_terms.iter().all(|term| term.passes(date));
        let mut split = DateSplit::default();
        for (id, track) in facts.tracks() {
            if passes(track.release_date) {
                split.release_track_ids.insert(id);
            }
            if passes(track.pub_date) {
                split.track_track_ids.insert(id);
            }
        }
        split
    });
    if let Some(split) = &date_split {
        let union = split
            .release_track_ids
            .union(&split.track_track_ids)
            .copied()
            .collect();
        clauses.push(db::LibraryTrackClause {
            excluded: false,
            kind: db::LibraryClauseKind::TrackIds(union),
        });
    }
    LibrarySearchPlan {
        search: db::LibraryTrackSearch { clauses },
        date_split,
    }
}

/// One `after:` or `before:` term with its bound in Unix seconds.
struct DateTerm {
    /// `true` for `after:`, `false` for `before:`.
    after: bool,
    /// The first second of `after:`, or the last second of `before:`.
    bound: i64,
    excluded: bool,
}

impl DateTerm {
    /// A missing date does not pass a term without `-`, and passes a term
    /// with `-` (packet 002 "The Library facts from the tree").
    fn passes(&self, date: Option<i64>) -> bool {
        let Some(date) = date else {
            return self.excluded;
        };
        let inside = if self.after {
            date >= self.bound
        } else {
            date <= self.bound
        };
        inside != self.excluded
    }
}

/// The date term of a correct `after:` or `before:` field, else `None`.
fn date_term(term: &QueryTerm, field: &FieldTerm) -> Option<DateTerm> {
    let FieldValueState::Valid(FieldValue::Date(date)) = &field.value else {
        return None;
    };
    match field.name {
        QueryFieldName::After => Some(DateTerm {
            after: true,
            bound: date.start_unix(),
            excluded: term.excluded,
        }),
        QueryFieldName::Before => Some(DateTerm {
            after: false,
            bound: date.end_unix(),
            excluded: term.excluded,
        }),
        _ => None,
    }
}

/// The clause of one correct field other than a date. `in:`, an empty
/// value and a value with an error give `None`.
fn field_clause(field: &FieldTerm, facts: &LibrarySearchFacts) -> Option<db::LibraryClauseKind> {
    let FieldValueState::Valid(value) = &field.value else {
        return None;
    };
    let kind = match (field.name, value) {
        (QueryFieldName::Artist, FieldValue::Text(text)) => {
            db::LibraryClauseKind::Artist(text.clone())
        }
        (QueryFieldName::Title, FieldValue::Text(text)) => {
            db::LibraryClauseKind::Title(text.clone())
        }
        (QueryFieldName::Release, FieldValue::Text(text)) => {
            db::LibraryClauseKind::Release(text.clone())
        }
        (QueryFieldName::Playlist, FieldValue::Text(text)) => {
            db::LibraryClauseKind::Playlist(text.clone())
        }
        (QueryFieldName::Kind, FieldValue::Kinds(kinds)) => {
            let kinds: Vec<ReleaseKind> = kinds
                .iter()
                .map(|kind| ReleaseKind::from_query_kind(*kind))
                .collect();
            db::LibraryClauseKind::TrackIds(
                facts
                    .tracks()
                    .filter(|(_, track)| kinds.contains(&track.release_kind))
                    .map(|(id, _)| id)
                    .collect(),
            )
        }
        (QueryFieldName::Duration, FieldValue::Duration(range)) => {
            db::LibraryClauseKind::Duration {
                min_secs: range.min_secs,
                max_secs: range.max_secs,
            }
        }
        (QueryFieldName::Is, FieldValue::LibraryState(state)) => match state {
            LibraryStateValue::Downloaded => db::LibraryClauseKind::Downloaded,
            LibraryStateValue::Partial => db::LibraryClauseKind::Partial,
            LibraryStateValue::Removed => db::LibraryClauseKind::Removed,
            LibraryStateValue::Ready => db::LibraryClauseKind::TrackIds(
                facts
                    .ready_track_ids()
                    .map(|ready| ready.iter().copied().collect())
                    .unwrap_or_default(),
            ),
        },
        (QueryFieldName::Has, FieldValue::Content(content)) => match content {
            ContentValue::Split => db::LibraryClauseKind::HasSplit,
            ContentValue::Image => db::LibraryClauseKind::HasImage,
        },
        _ => return None,
    };
    Some(kind)
}

fn query_error(error: &anyhow::Error) -> CommandError {
    CommandError::Query(format!("{error:#}"))
}

/// A failure of all three requests is an error. When `MusicIndex`
/// answered each request with HTTP `400`, it did not accept the query
/// text, and the error is [`CommandError::QueryRejected`] (walk of
/// 2026-10-21). Each other failure is [`CommandError::Query`].
///
/// ADR 0087 packet 003: the function examines `cancel` before each
/// request. A canceled token gives [`CommandError::Cancelled`], and no
/// later request starts. A request that started finishes, because the
/// client is blocking.
fn fetch_index_search_result_rows(
    endpoint: &crate::config::MusicIndexEndpoint,
    query: &str,
    matches: &NameFeedMatches,
    cancel: &CancellationToken,
) -> Result<IndexSearchResultRows, CommandError> {
    let client = crate::api::Client::new_with_base_url(endpoint.clone());
    let mut rows = IndexSearchResultRows::default();
    let mut artists = BTreeMap::new();

    // ADR 0075 packet 047, Required Change 2: each row comes from this
    // search's own summary fields. No call below sends a detail request
    // for a returned hit.
    stop_if_cancelled(cancel)?;
    let feed_rows = fetch_index_feed_result_rows(&client, query);
    stop_if_cancelled(cancel)?;
    let track_rows = fetch_index_track_result_rows(&client, query);
    // Packet 010 "The Artists tab": the third request gives the feed rows.
    // Its failure alone shows the other rows.
    stop_if_cancelled(cancel)?;
    let publisher_rows = fetch_index_feed_page_result_rows(&client, query);

    if let (Err(feed_error), Err(track_error), Err(publisher_error)) =
        (&feed_rows, &track_rows, &publisher_rows)
    {
        let message = format!(
            "feed search failed: {feed_error}; track search failed: {track_error}; publisher search failed: {publisher_error}"
        );
        let rejected = [feed_error, track_error, publisher_error]
            .into_iter()
            .all(is_bad_request);
        return Err(if rejected {
            CommandError::QueryRejected(message)
        } else {
            query_error(&anyhow!(message))
        });
    }
    // ADR 0086 packet 005 decision D3: a later feed page, or a failed
    // feed request, can change a window group of a saved search.
    match feed_rows {
        Ok(feeds) => {
            rows.feeds = feeds.rows;
            rows.feed_rows_partial = feeds.has_more;
            merge_index_artist_candidates(&mut artists, feeds.artists);
        }
        Err(_) => rows.feed_rows_partial = true,
    }
    if let Ok(tracks) = track_rows {
        rows.tracks = tracks.rows;
        merge_index_artist_candidates(&mut artists, tracks.artists);
    }

    rows.artists = index_artist_rows(&client, artists, matches, cancel)?;
    if let Ok(feed_page_rows) = publisher_rows {
        rows.artists.extend(feed_page_rows);
    }
    Ok(rows)
}

/// Gives [`CommandError::Cancelled`] when `cancel` is canceled (ADR 0087
/// packet 003).
fn stop_if_cancelled(cancel: &CancellationToken) -> Result<(), CommandError> {
    if cancel.is_cancelled() {
        return Err(CommandError::Cancelled);
    }
    Ok(())
}

/// Tells if `MusicIndex` answered the request with HTTP `400`. A
/// connection error, a timeout, a `5xx` reply or a bad endpoint is not a
/// `400` reply.
fn is_bad_request(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<crate::api::HttpStatusError>()
        .is_some_and(crate::api::HttpStatusError::is_bad_request)
}

/// The feed rows of the Artists tab (packet 010 "The Artists tab", flow
/// map finding 6). One `GET /v1/search?type=publisher&include=link_facts`
/// request gives them (profile `INDEX_PUBLISHER_SEARCH`, ADR 0084 packet
/// 029). The `entity_id` of a row is the feed GUID. The link facts of a row
/// give its badge, so the search sends no request for a row. No row text
/// says "Publisher" (ADR 0085 Decision 1).
fn fetch_index_feed_page_result_rows(
    client: &crate::api::Client,
    query: &str,
) -> Result<Vec<(SearchResultItemId, ArtistResultDisplay)>> {
    let response = client.search(
        query,
        Some("publisher"),
        Some(crate::api::PAGE_LIMIT),
        None,
        Some(&crate::application::request_profiles::INDEX_PUBLISHER_SEARCH),
    )?;
    Ok(unique_hits(&response.data)
        .filter_map(index_feed_page_result_display)
        .enumerate()
        .map(|(index, display)| (index_item_id(INDEX_FEED_PAGE_ID_BASE, index), display))
        .collect())
}

/// The row of one `type=publisher` hit, with the id
/// `index-feed-page:<entity_id>`. `None` for a hit with an empty
/// `entity_id`. A hit that gives `agreed_roles` or `first_named_count` sets
/// the link class of the row. The row view model adds the badge word to the
/// accessibility label.
fn index_feed_page_result_display(hit: &SearchResult) -> Option<ArtistResultDisplay> {
    let feed_guid = non_empty_str(Some(hit.entity_id.as_str()))?;
    let label = non_empty_str(hit.title.as_deref()).unwrap_or(feed_guid);
    let mut display =
        ArtistResultDisplay::new(format!("{INDEX_FEED_PAGE_RESULT_PREFIX}{feed_guid}"), label);
    display.a11y_label = format!("Open the feed page {label}");
    if hit.agreed_roles.is_some() || hit.first_named_count.is_some() {
        display.link_class = Some(FeedLinkClass::from_link_facts(
            hit.agreed_roles.as_deref(),
            hit.first_named_count,
        ));
    }
    if let Some(image_url) = non_empty_string(hit.feed_image_url.clone()) {
        display = display.with_thumbnail_href(image_url);
    }
    Some(display)
}

/// The artist rows of the `MusicIndex` search results (ADR 0084 Decision 7,
/// packet 017 decisions D6, D7 and D15). A name that matches exactly one
/// Library artist feed gives the row of that feed, one time for each feed.
/// Each other name gives its name row only when `MusicIndex` holds a `name`
/// row with its key. The function examines `cancel` before each name
/// check, and a canceled token gives [`CommandError::Cancelled`] (ADR 0087
/// packet 003).
fn index_artist_rows(
    client: &crate::api::Client,
    artists: BTreeMap<String, IndexArtistCandidate>,
    matches: &NameFeedMatches,
    cancel: &CancellationToken,
) -> Result<Vec<(SearchResultItemId, ArtistResultDisplay)>, CommandError> {
    let mut feeds_shown = BTreeSet::new();
    let mut displays = Vec::new();
    for (key, artist) in artists {
        match matches.get(&key) {
            NameFeedMatch::MatchedFeed(feed_guid) => {
                if feeds_shown.insert(feed_guid.clone()) {
                    let title = matches.feed_title(&feed_guid);
                    displays.push(artist_feed_result_display(&feed_guid, &title));
                }
            }
            NameFeedMatch::Ambiguous | NameFeedMatch::NoFeed => {
                stop_if_cancelled(cancel)?;
                if index_name_row_exists(client, &key) {
                    displays.push(artist.into_display());
                }
            }
        }
    }
    Ok(displays
        .into_iter()
        .enumerate()
        .map(|(index, display)| (index_item_id(INDEX_ARTIST_ID_BASE, index), display))
        .collect())
}

struct IndexFeedSearchRows {
    rows: Vec<(SearchResultItemId, FeedResultDisplay)>,
    artists: Vec<IndexArtistCandidate>,
    /// The `has_more` of the response: a later page holds more feed rows
    /// (ADR 0086 packet 005). The search reads no later page.
    has_more: bool,
}

struct IndexTrackSearchRows {
    rows: Vec<(SearchResultItemId, TrackResultDisplay)>,
    artists: Vec<IndexArtistCandidate>,
}

#[derive(Clone, Debug)]
struct IndexArtistCandidate {
    name: String,
    thumbnail_href: Option<String>,
}

impl IndexArtistCandidate {
    fn new(name: impl Into<String>, thumbnail_href: Option<String>) -> Self {
        Self {
            name: name.into(),
            thumbnail_href,
        }
    }

    fn merge(&mut self, other: Self) {
        if self.thumbnail_href.is_none() {
            self.thumbnail_href = other.thumbnail_href;
        }
    }

    /// The name row of a `MusicIndex` name (ADR 0084 Decision 7, packet
    /// 017 decision D7). Its label and its accessibility label are the
    /// title of the name level page that the row opens. The row shows no
    /// count, because a search hit count does not count the releases of
    /// the name.
    fn into_display(self) -> ArtistResultDisplay {
        let label = NameLevelPageVm::title_text_for_name(&self.name);
        let mut display = ArtistResultDisplay::new(format!("index-artist:{}", self.name), label);
        display.a11y_label.clone_from(&display.label);
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
/// A track GUID is unique only in its feed, so the key holds the feed GUID.
/// MusicIndex responses are untrusted input.
fn unique_hits(hits: &[SearchResult]) -> impl Iterator<Item = &SearchResult> {
    let mut seen = std::collections::HashSet::new();
    hits.iter().filter(move |hit| {
        seen.insert((
            hit.entity_type.as_str(),
            hit.feed_guid.as_deref(),
            hit.entity_id.as_str(),
        ))
    })
}

/// Packet 047 Required Change 2: this search sends no per-hit detail
/// request. Each row's title, artist, track count and artwork come from
/// the search response's own summary fields (R47-02, R47-03, R47-05).
fn fetch_index_feed_result_rows(
    client: &crate::api::Client,
    query: &str,
) -> Result<IndexFeedSearchRows> {
    let response = client.search(
        query,
        Some("feed"),
        Some(crate::api::PAGE_LIMIT),
        None,
        None,
    )?;
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

    Ok(IndexFeedSearchRows {
        rows,
        artists,
        has_more: response.pagination.has_more,
    })
}

/// Packet 047 Required Change 2: this search sends no per-hit detail
/// request. Each row's title, artist and feed name come from the search
/// response's own summary fields. Its artwork follows ADR 0075 Decision C
/// through `index_track_result_artwork_url` (R47-02, R47-04).
fn fetch_index_track_result_rows(
    client: &crate::api::Client,
    query: &str,
) -> Result<IndexTrackSearchRows> {
    let response = client.search(
        query,
        Some("track"),
        Some(crate::api::PAGE_LIMIT),
        None,
        None,
    )?;
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
    // Packet 010 decision D11: an Index hit resolves its kind from its
    // track count only, because the summary states no release kind.
    let track_count = hit
        .episode_count
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or(0);
    let mut display = FeedResultDisplay::new(
        format!("index-feed:{feed_guid}"),
        label,
        SearchResultOrigin::Index,
    )
    .with_release_key(
        ReleaseKey::from_index_feed_guid(feed_guid),
        ReleaseKind::resolve(None, track_count),
    );

    // Walk of 2026-10-21: the search list joins the parts after the kind.
    let secondary = present_parts([
        hit.release_artist.clone(),
        hit.episode_count.map(|count| count_label(count, "track")),
    ]);
    if !secondary.is_empty() {
        display = display.with_secondary_parts(secondary);
    }
    if let Some(image_url) = non_empty_string(hit.feed_image_url.clone()) {
        display = display.with_thumbnail_href(image_url);
    }
    // ADR 0086 packet 005: a saved search reads the group key of an Index
    // single from this text. The hit gives no `newest_item_at`.
    if let Some(artist) = non_empty_str(hit.release_artist.as_deref()) {
        display = display.with_artist_text(artist);
    }
    display
}

/// Builds a track search-result row directly from the search response's
/// own summary fields (ADR 0075 packet 047, Required Change 2).
fn index_track_result_display(hit: &SearchResult) -> TrackResultDisplay {
    let track_guid = hit.entity_id.as_str();
    let feed_guid = non_empty_str(hit.feed_guid.as_deref());
    let activation_id = index_track_row_id(feed_guid, track_guid);
    let label = non_empty_str(hit.title.as_deref())
        .map(str::to_string)
        .unwrap_or_else(|| track_guid.to_string());
    let mut display = TrackResultDisplay::new(activation_id, label, SearchResultOrigin::Index);
    // Packet 010: an Index track with no feed GUID has no key, so it has
    // no checkbox and it merges with no Library track.
    if let Some(feed_guid) = feed_guid {
        display = display.with_track_key(TrackKey::from_index_track_target(
            Some(feed_guid),
            track_guid,
        ));
    }

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
        .map(|name| IndexArtistCandidate::new(name, index_track_result_artwork_url(hit)))
        .collect()
}

fn merge_index_artist_candidates(
    artists: &mut BTreeMap<String, IndexArtistCandidate>,
    candidates: Vec<IndexArtistCandidate>,
) {
    for candidate in candidates {
        let key = name_sort_key(&candidate.name);
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
const INDEX_FEED_PAGE_ID_BASE: SearchResultItemId = 4_000_000_000;

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
        // ADR 0086 Decision 1: keep the release date of the feed before
        // `FeedView::from_api` takes the feed. `FeedView` has no field for it.
        let newest_item_at = feed.newest_item_at;
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
        display = display
            .with_remote_feed(remote_feed)
            .with_newest_item_at(newest_item_at);
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

fn count_parts<const N: usize>(parts: [Option<String>; N]) -> String {
    present_parts(parts).join(" - ")
}

/// The parts that have text, in their order.
fn present_parts<const N: usize>(parts: [Option<String>; N]) -> Vec<String> {
    parts.into_iter().filter_map(non_empty_string).collect()
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
    use crate::view_models::search_results::{SearchResultTarget, SearchResultsTab};

    /// MusicIndex gave one track twice on 2026-10-07. The rows keep the
    /// first hit of each entity, in rank order. Two tracks with one GUID in
    /// two feeds are two entities.
    #[test]
    fn unique_hits_keep_the_first_hit_of_each_entity() {
        let hit =
            |entity_type: &str, feed_guid: Option<&str>, entity_id: &str, rank: f64| SearchResult {
                entity_type: entity_type.into(),
                feed_guid: feed_guid.map(Into::into),
                entity_id: entity_id.into(),
                rank: Some(rank),
                ..SearchResult::default()
            };
        let hits = vec![
            hit("track", Some("f1"), "t1", -12.5),
            hit("track", Some("f1"), "t1", -11.1),
            hit("track", Some("f2"), "t1", -10.5),
            hit("track", Some("f1"), "t2", -10.0),
            hit("feed", None, "t1", -9.0),
        ];
        let kept: Vec<_> = unique_hits(&hits)
            .map(|hit| {
                (
                    hit.entity_type.as_str(),
                    hit.feed_guid.as_deref(),
                    hit.entity_id.as_str(),
                    hit.rank,
                )
            })
            .collect();
        assert_eq!(
            kept,
            vec![
                ("track", Some("f1"), "t1", Some(-12.5)),
                ("track", Some("f2"), "t1", Some(-10.5)),
                ("track", Some("f1"), "t2", Some(-10.0)),
                ("feed", None, "t1", Some(-9.0)),
            ]
        );
    }

    /// R86-2-31: `index_feed_display` keeps `newest_item_at` of the feed
    /// that it gets: the detail feed, or the list feed of a failed detail
    /// read. With no feed, the value is `None`.
    #[test]
    fn adr_0086_window_groups_r31_index_feed_display_keeps_newest_item_at() {
        let feed = |newest_item_at| crate::api::Feed {
            feed_guid: Some("guid-a".to_owned()),
            title: Some("Single A".to_owned()),
            newest_item_at,
            ..crate::api::Feed::default()
        };
        let detail = index_feed_display(
            "guid-a",
            Some(crate::api::EntityDetail::Feed(feed(Some(1_792_454_400)))),
        );
        assert_eq!(detail.newest_item_at, Some(1_792_454_400));
        assert!(detail.remote_feed.is_some());

        // A failed detail read gives the list feed, with no tracks and no
        // `publisher` include, to the same function.
        let list_feed = index_feed_display(
            "guid-a",
            Some(crate::api::EntityDetail::Feed(feed(Some(1_792_368_000)))),
        );
        assert_eq!(list_feed.newest_item_at, Some(1_792_368_000));

        assert_eq!(index_feed_display("guid-a", None).newest_item_at, None);
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

        let rows = ApplicationQueryService::new()
            .search_local_library_tracks(
                &conn,
                &SearchQuery::parse("needle"),
                &LibrarySearchFacts::default(),
                None,
            )?
            .tracks;

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

        let rows = ApplicationQueryService::new()
            .search_local_library_tracks(
                &conn,
                &SearchQuery::parse("needle"),
                &LibrarySearchFacts::default(),
                None,
            )?
            .tracks;

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
                .search_local_library_tracks(
                    &conn,
                    &SearchQuery::parse("limit"),
                    &LibrarySearchFacts::default(),
                    None
                )?
                .tracks
                .len(),
            DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT
        );
        assert_eq!(
            service
                .search_local_library_tracks(
                    &conn,
                    &SearchQuery::parse("limit"),
                    &LibrarySearchFacts::default(),
                    Some(3)
                )?
                .tracks
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

        let rows = ApplicationQueryService::new()
            .search_local_library_tracks(
                &conn,
                &SearchQuery::parse(" *** "),
                &LibrarySearchFacts::default(),
                None,
            )?
            .tracks;

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

    /// Packet 010: a track hit with a blank feed GUID has no key, so it
    /// has no checkbox, and its row id names the track GUID only.
    #[test]
    fn adr_0084_search_list_blank_feed_guid_gives_no_track_key() {
        let display = index_track_result_display(&SearchResult {
            entity_type: "track".to_string(),
            entity_id: "track-guid".to_string(),
            feed_guid: Some("  ".to_string()),
            ..SearchResult::default()
        });
        assert_eq!(display.key, None);
        assert_eq!(display.id, "index-track-guid:track-guid");
    }

    /// R28-01 (ADR 0084 packet 028 D1): a track hit with no feed GUID and
    /// a track GUID with colons opens a key with no feed and the full
    /// track GUID.
    #[test]
    fn adr_0084_search_open_defects_r28_01_track_guid_with_colons_opens_its_own_key() {
        let display = index_track_result_display(&SearchResult {
            entity_type: "track".to_string(),
            entity_id: "urn:uuid:x".to_string(),
            ..SearchResult::default()
        });
        assert_eq!(display.id, "index-track-guid:urn:uuid:x");
        let target = SearchResultTarget::from_row_id(SearchResultsTab::Tracks, &display.id);
        let expected = TrackKey::from_index_track_target(None, "urn:uuid:x");
        assert_eq!(target, Some(SearchResultTarget::Track(expected.clone())));
        assert_eq!(expected.index_feed_guid(), None);
        assert_eq!(expected.item_guid, "urn:uuid:x");
    }

    /// R28-02 (ADR 0084 packet 028 D1): a track hit with a feed GUID gives
    /// the same row id and key as before.
    #[test]
    fn adr_0084_search_open_defects_r28_02_track_with_feed_keeps_its_row_id() {
        let display = index_track_result_display(&SearchResult {
            entity_type: "track".to_string(),
            entity_id: "urn:uuid:x".to_string(),
            feed_guid: Some("feed-guid".to_string()),
            ..SearchResult::default()
        });
        assert_eq!(display.id, "index-track:feed-guid:urn:uuid:x");
        let expected = TrackKey::from_index_track_target(Some("feed-guid"), "urn:uuid:x");
        assert_eq!(display.key, Some(expected.clone()));
        assert_eq!(
            SearchResultTarget::from_row_id(SearchResultsTab::Tracks, &display.id),
            Some(SearchResultTarget::Track(expected.clone()))
        );
        assert_eq!(expected.index_feed_guid(), Some("feed-guid"));
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
        assert_eq!(
            feed_candidate.thumbnail_href.as_deref(),
            Some("https://example.test/feed.jpg")
        );

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

    /// R87-3-16 (ADR 0087 packet 003): a run with a token that is not
    /// canceled sends the three `GET /v1/search` requests. A run with a
    /// canceled token gives `CommandError::Cancelled` and sends no request.
    #[test]
    fn adr_0087_live_results_r87_3_16_cancelled_run_sends_no_request() {
        use crate::application::command_context::{OperationId, TraceId};

        let context = |token: CancellationToken| {
            CommandContext::new(OperationId::new(1), token, TraceId::new(1))
        };

        let fixture = Fixture::start();
        FetchIndexSearchResults::new(
            fixture.endpoint.clone(),
            IndexSearchRequest {
                q: "live results".to_owned(),
            },
            NameFeedMatches::default(),
        )
        .execute(&context(CancellationToken::new()))
        .expect("the fixture answers each request");
        let requests = fixture.requests.lock().unwrap().clone();
        let searches: Vec<&String> = requests
            .iter()
            .filter(|path| path.starts_with("/v1/search?"))
            .collect();
        assert_eq!(searches.len(), 3, "Got: {requests:?}");
        for kind in ["type=feed", "type=track", "type=publisher"] {
            assert!(
                searches.iter().any(|path| path.contains(kind)),
                "missing {kind}. Got: {requests:?}"
            );
        }

        let fixture = Fixture::start();
        let token = CancellationToken::new();
        token.cancel();
        let error = FetchIndexSearchResults::new(
            fixture.endpoint.clone(),
            IndexSearchRequest {
                q: "live results".to_owned(),
            },
            NameFeedMatches::default(),
        )
        .execute(&context(token))
        .expect_err("a canceled token stops the run");
        assert!(matches!(error, CommandError::Cancelled), "Got: {error:?}");
        assert!(fixture.requests.lock().unwrap().is_empty());
    }
}

/// ADR 0084 packet 017: the `MusicIndex` requests of the name level page and
/// of the search name check, against a local recorded-response server.
#[cfg(test)]
mod adr_0084_name_level_query_tests {
    use super::*;
    use crate::view_models::artist_identity::{name_feed_matches, NameFeedEntry};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    type Responder = Arc<dyn Fn(&str) -> (u16, String) + Send + Sync>;

    /// A local HTTP server that answers `expected` requests through
    /// `respond`, and records each request path.
    fn serve(expected: usize, respond: Responder) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&requests);
        std::thread::spawn(move || {
            for _ in 0..expected {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
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
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or_default()
                    .to_owned();
                received.lock().unwrap().push(path.clone());
                let (status, body) = respond(&path);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        (format!("http://{address}"), requests)
    }

    /// The rows of `GET /v1/feeds/by-name?artist_key=sir%20seat%20sitter`
    /// of 2026-10-20 at 23:20 UTC: the titles, the track counts, the
    /// release artist and the null `publisher_feed_guid` are recorded. The
    /// packet did not record the feed GUIDs, so each row has a placeholder
    /// GUID.
    fn sir_seat_sitter_rows() -> serde_json::Value {
        serde_json::json!({
            "data": [
                {"feed_guid": "gay-and-long-guid", "title": "Gay and Long",
                 "release_artist": "Sir Seat Sitter", "episode_count": 1,
                 "publisher_feed_guid": null},
                {"feed_guid": "gone-guid", "title": "Gone",
                 "release_artist": "Sir Seat Sitter", "episode_count": 1,
                 "publisher_feed_guid": null},
                {"feed_guid": "that-duck-guid", "title": "That Duck",
                 "release_artist": "Sir Seat Sitter", "episode_count": 1,
                 "publisher_feed_guid": null},
                {"feed_guid": "waitin-guid", "title": "Waitin' on Alex",
                 "release_artist": "Sir Seat Sitter", "episode_count": 17,
                 "publisher_feed_guid": null}
            ],
            "pagination": {"has_more": false, "cursor": null}
        })
    }

    /// R17-10: the command sends one request with `artist_key` and
    /// `limit=200` and no `include`. The recorded response gives 4
    /// releases: 3 singles and "Waitin' on Alex" with 17 tracks.
    #[test]
    fn adr_0084_name_level_r17_10_one_request_for_the_recorded_name() {
        let body = sir_seat_sitter_rows().to_string();
        let (endpoint, requests) = serve(1, Arc::new(move |_| (200, body.clone())));

        let index = FetchNameLevelReleases::new(endpoint, "sir seat sitter")
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0)
            .unwrap();

        let NameLevelIndexState::Loaded {
            releases,
            more_available,
        } = index
        else {
            panic!("the command gives the loaded releases");
        };
        assert!(!more_available);
        let counts: Vec<(&str, Option<i32>)> = releases
            .iter()
            .map(|release| {
                (
                    release.title.as_deref().unwrap_or_default(),
                    release.episode_count,
                )
            })
            .collect();
        assert_eq!(
            counts,
            [
                ("Gay and Long", Some(1)),
                ("Gone", Some(1)),
                ("That Duck", Some(1)),
                ("Waitin' on Alex", Some(17)),
            ]
        );
        let requests = requests.lock().unwrap().clone();
        assert_eq!(
            requests,
            ["/v1/feeds/by-name?artist_key=sir+seat+sitter&limit=200"],
            "one request, with no include"
        );
    }

    /// R17-11: with `has_more` true on each response, the command sends 5
    /// requests, follows the cursor and sets `more_available`. The key of
    /// "Unknown Artist" sends no request.
    #[test]
    fn adr_0084_name_level_r17_11_five_requests_at_most() {
        let (endpoint, requests) = serve(
            5,
            Arc::new(|path: &str| {
                let page = requests_page(path);
                (
                    200,
                    serde_json::json!({
                        "data": [{"feed_guid": format!("guid-{page}"), "title": "T"}],
                        "pagination": {"has_more": true, "cursor": format!("c{}", page + 1)}
                    })
                    .to_string(),
                )
            }),
        );

        let index = FetchNameLevelReleases::new(endpoint.clone(), "haleen")
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0)
            .unwrap();
        let NameLevelIndexState::Loaded {
            releases,
            more_available,
        } = index
        else {
            panic!("the command gives the loaded releases");
        };
        assert!(more_available);
        assert_eq!(releases.len(), 5);
        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 5);
        assert!(requests[1].contains("cursor=c1"), "{requests:?}");

        let unknown = FetchNameLevelReleases::new(endpoint, "unknown artist")
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0)
            .unwrap();
        assert_eq!(unknown, NameLevelIndexState::NotRequested);
        assert_eq!(
            requests.len(),
            5,
            "the key of Unknown Artist sends no request"
        );
    }

    fn requests_page(path: &str) -> usize {
        path.split("cursor=c")
            .nth(1)
            .and_then(|rest| rest.split('&').next())
            .and_then(|page| page.parse().ok())
            .unwrap_or(0)
    }

    fn candidate(name: &str) -> (String, IndexArtistCandidate) {
        (name_sort_key(name), IndexArtistCandidate::new(name, None))
    }

    /// R17-29, the `MusicIndex` part: a candidate sends one
    /// `GET /v1/artist-names?from=<key>&limit=1`. A first row `name` with
    /// the same key shows the row. A first row of another key or kind hides
    /// it. A failed check shows it. A `MatchedFeed` candidate sends no
    /// check, and two names that match one feed give one feed row (R17-28).
    #[test]
    fn adr_0084_name_level_r17_29_index_name_check() {
        let (endpoint, requests) = serve(
            4,
            Arc::new(|path: &str| {
                let row = |kind: &str, key: &str| {
                    serde_json::json!({
                        "data": [{"kind": kind, "name": key, "sort_name": key, "key": key}],
                        "pagination": {"has_more": true}
                    })
                    .to_string()
                };
                if path.contains("from=shown") {
                    (200, row("name", "shown"))
                } else if path.contains("from=other+key") {
                    (200, row("name", "other key 2"))
                } else if path.contains("from=feed+only") {
                    (200, row("publisher", "feed only"))
                } else {
                    (500, "{}".to_owned())
                }
            }),
        );
        let client = crate::api::Client::new_with_base_url(endpoint);
        let matches = name_feed_matches([NameFeedEntry {
            feed_guid: "haleen-guid".to_owned(),
            title: Some("Haleen".to_owned()),
            keys: ["haleen".to_owned(), "haleen music".to_owned()]
                .into_iter()
                .collect(),
        }]);
        let candidates: BTreeMap<String, IndexArtistCandidate> = [
            candidate("Shown"),
            candidate("Other Key"),
            candidate("Feed Only"),
            candidate("Failed Check"),
            candidate("Haleen"),
            candidate("Haleen Music"),
        ]
        .into_iter()
        .collect();

        let rows = index_artist_rows(&client, candidates, &matches, &CancellationToken::new())
            .expect("the token is not canceled");
        let labels: Vec<(&str, &str)> = rows
            .iter()
            .map(|(_, row)| (row.id.as_str(), row.label.as_str()))
            .collect();
        assert_eq!(
            labels,
            [
                (
                    "index-artist:Failed Check",
                    "Releases that name Failed Check"
                ),
                ("artist-feed:haleen-guid", "Haleen"),
                ("index-artist:Shown", "Releases that name Shown"),
            ]
        );
        assert!(rows.iter().all(|(_, row)| row.secondary_text.is_empty()));
        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 4, "a MatchedFeed candidate sends no check");
        assert!(requests
            .iter()
            .all(|path| path.starts_with("/v1/artist-names?") && path.contains("limit=1")));
        assert!(!requests.iter().any(|path| path.contains("include")));
    }

    /// The rows of `GET /v1/search?q=Libre&type=publisher` of 2026-10-20
    /// (packet 010 "Recorded Facts").
    fn libre_publisher_rows() -> serde_json::Value {
        serde_json::json!({
            "data": [
                {"entity_type": "publisher",
                 "entity_id": "34bf795f-f551-4db1-8877-ad19aa5f89c8",
                 "title": "Sir Libre Records", "episode_count": 0},
                {"entity_type": "publisher",
                 "entity_id": "c8ec3dba-ed28-5662-9509-094d769f9597",
                 "title": "Kolomona Myer - AKA Sir Libre", "episode_count": 0}
            ],
            "pagination": {"has_more": false, "cursor": null}
        })
    }

    fn empty_search() -> String {
        serde_json::json!({"data": [], "pagination": {"has_more": false}}).to_string()
    }

    /// R10-05: the search sends `type=feed`, `type=track` and
    /// `type=publisher`. The recorded Libre response gives two feed rows
    /// with the ids `index-feed-page:<guid>`, and no other request.
    #[test]
    fn adr_0084_search_list_r10_05_publisher_request_gives_feed_rows() {
        let (endpoint, requests) = serve(
            3,
            Arc::new(|path: &str| {
                if path.contains("type=publisher") {
                    (200, libre_publisher_rows().to_string())
                } else {
                    (200, empty_search())
                }
            }),
        );
        let rows = fetch_index_search_result_rows(
            &endpoint.into(),
            "Libre",
            &NameFeedMatches::default(),
            &CancellationToken::new(),
        )
        .expect("the recorded response decodes");

        let ids: Vec<&str> = rows
            .artists
            .iter()
            .map(|(_, row)| row.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "index-feed-page:34bf795f-f551-4db1-8877-ad19aa5f89c8",
                "index-feed-page:c8ec3dba-ed28-5662-9509-094d769f9597",
            ]
        );
        assert_eq!(rows.artists[0].1.label, "Sir Libre Records");
        assert!(rows
            .artists
            .iter()
            .all(|(_, row)| !row.a11y_label.contains("Publisher")));
        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 3, "Got: {requests:?}");
        for kind in ["type=feed", "type=track", "type=publisher"] {
            assert!(
                requests
                    .iter()
                    .any(|path| path.starts_with("/v1/search?") && path.contains(kind)),
                "missing {kind}. Got: {requests:?}"
            );
        }
    }

    /// The rows of
    /// `GET /v1/search?q=Libre&type=publisher&include=link_facts&limit=20` of
    /// 2026-10-21 (packet 029 "Live API facts").
    fn libre_publisher_rows_with_link_facts() -> serde_json::Value {
        serde_json::json!({
            "data": [
                {"entity_type": "publisher",
                 "entity_id": "34bf795f-f551-4db1-8877-ad19aa5f89c8",
                 "title": "Sir Libre Records", "episode_count": 0,
                 "two_way_link_count": 3, "agreed_roles": ["label"],
                 "first_named_count": 0},
                {"entity_type": "publisher",
                 "entity_id": "c8ec3dba-ed28-5662-9509-094d769f9597",
                 "title": "Kolomona Myer - AKA Sir Libre", "episode_count": 0,
                 "two_way_link_count": 1, "agreed_roles": [], "first_named_count": 1}
            ],
            "pagination": {"has_more": false, "cursor": null}
        })
    }

    /// R29-03: the `type=publisher` request sends `include=link_facts`. The
    /// `type=feed` and `type=track` requests send no `include`. The link
    /// facts of the recorded rows give the link class of each row, with no
    /// other request.
    #[test]
    fn adr_0084_search_link_facts_r29_03_publisher_request_sends_the_include() {
        let (endpoint, requests) = serve(
            3,
            Arc::new(|path: &str| {
                if path.contains("type=publisher") {
                    (200, libre_publisher_rows_with_link_facts().to_string())
                } else {
                    (200, empty_search())
                }
            }),
        );
        let rows = fetch_index_search_result_rows(
            &endpoint.into(),
            "Libre",
            &NameFeedMatches::default(),
            &CancellationToken::new(),
        )
        .expect("the recorded response decodes");

        let classes: Vec<Option<FeedLinkClass>> = rows
            .artists
            .iter()
            .map(|(_, row)| row.link_class.clone())
            .collect();
        assert_eq!(
            classes,
            [
                Some(FeedLinkClass::OtherRoles(vec!["label".to_owned()])),
                Some(FeedLinkClass::UnverifiedArtist),
            ]
        );
        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 3, "Got: {requests:?}");
        for path in &requests {
            if path.contains("type=publisher") {
                assert!(path.contains("include=link_facts"), "Got: {path}");
            } else {
                assert!(
                    path.contains("type=feed") || path.contains("type=track"),
                    "Got: {path}"
                );
                assert!(!path.contains("include"), "Got: {path}");
            }
        }
    }

    /// R29-08, the decode part: a `type=publisher` row with no link fact
    /// has no link class. The recorded rows of 2026-10-20 give no link
    /// fact.
    #[test]
    fn adr_0084_search_link_facts_r29_08_row_with_no_link_fact_has_no_class() {
        let response: crate::api::SearchResponse =
            serde_json::from_value(libre_publisher_rows()).expect("the recorded rows decode");
        for hit in &response.data {
            let display = index_feed_page_result_display(hit).expect("the row has an id");
            assert_eq!(display.link_class, None);
        }
    }

    /// Walk of 2026-10-21: when `MusicIndex` answers each search request
    /// with HTTP `400`, the error is `QueryRejected`, with the reply of the
    /// server. A `500` reply keeps the `Query` error of today. The query
    /// text that the app sends does not change.
    #[test]
    fn adr_0084_search_list_http_400_is_a_rejected_query() {
        let rejected =
            r#"{"error":"invalid search query: SQLite error: fts5: syntax error near \"'\""}"#;
        let (endpoint, requests) =
            serve(3, Arc::new(move |_path: &str| (400, rejected.to_owned())));
        let error = fetch_index_search_result_rows(
            &endpoint.into(),
            "haleen'",
            &NameFeedMatches::default(),
            &CancellationToken::new(),
        )
        .expect_err("each request failed");
        let CommandError::QueryRejected(message) = error else {
            panic!("a 400 reply is a rejected query. Got: {error:?}");
        };
        assert!(message.contains("HTTP 400 Bad Request"), "{message}");
        assert!(message.contains("fts5: syntax error"), "{message}");
        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 3, "Got: {requests:?}");
        assert!(
            requests.iter().all(|path| path.contains("q=haleen%27")),
            "the query text does not change. Got: {requests:?}"
        );

        let (endpoint, _requests) = serve(3, Arc::new(|_path: &str| (500, "{}".to_owned())));
        let error = fetch_index_search_result_rows(
            &endpoint.into(),
            "haleen",
            &NameFeedMatches::default(),
            &CancellationToken::new(),
        )
        .expect_err("each request failed");
        assert!(matches!(error, CommandError::Query(_)), "Got: {error:?}");
    }

    /// R10-05, the failure part: a failure of the `type=publisher` request
    /// alone shows the other rows. A failure of all three requests gives
    /// the failure state.
    #[test]
    fn adr_0084_search_list_r10_05_publisher_failure_alone_keeps_the_other_rows() {
        let (endpoint, _requests) = serve(
            3,
            Arc::new(|path: &str| {
                if path.contains("type=publisher") {
                    (500, "{}".to_owned())
                } else if path.contains("type=feed") {
                    (
                        200,
                        serde_json::json!({
                            "data": [{"entity_type": "feed", "entity_id": "f1",
                                      "feed_guid": "f1", "title": "Libre Album",
                                      "episode_count": 9}],
                            "pagination": {"has_more": false}
                        })
                        .to_string(),
                    )
                } else {
                    (200, empty_search())
                }
            }),
        );
        let rows = fetch_index_search_result_rows(
            &endpoint.into(),
            "Libre",
            &NameFeedMatches::default(),
            &CancellationToken::new(),
        )
        .expect("one failed request keeps the other rows");
        assert_eq!(rows.feeds.len(), 1);
        assert!(rows.artists.is_empty());

        let (endpoint, _requests) = serve(3, Arc::new(|_path: &str| (500, "{}".to_owned())));
        assert!(fetch_index_search_result_rows(
            &endpoint.into(),
            "Libre",
            &NameFeedMatches::default(),
            &CancellationToken::new()
        )
        .is_err());
    }

    /// The responder of ADR 0086 packet 005: `feed` answers the feed
    /// search, and each other search gives no row.
    fn feed_search_responder(feed: (u16, String)) -> Responder {
        Arc::new(move |path: &str| {
            if path.contains("type=feed") {
                feed.clone()
            } else {
                (
                    200,
                    serde_json::json!({
                        "data": [],
                        "pagination": {"has_more": false, "cursor": null}
                    })
                    .to_string(),
                )
            }
        })
    }

    /// R86-5-14: the search sends one `type=feed` request with no cursor.
    /// A response with `has_more` true sets `feed_rows_partial`.
    #[test]
    fn adr_0086_saved_search_groups_r14_has_more_marks_the_feed_rows_partial() {
        let feed = serde_json::json!({
            "data": [{"entity_type": "feed", "entity_id": "g1", "feed_guid": "g1",
                      "title": "One", "episode_count": 1}],
            "pagination": {"has_more": true, "cursor": "next"}
        })
        .to_string();
        let (endpoint, requests) = serve(3, feed_search_responder((200, feed)));

        let rows = fetch_index_search_result_rows(
            &endpoint.as_str().into(),
            "zz",
            &NameFeedMatches::default(),
            &CancellationToken::new(),
        )
        .expect("the feed request passes");

        assert!(rows.feed_rows_partial);
        assert_eq!(rows.feeds.len(), 1);
        let requests = requests.lock().unwrap().clone();
        let feed_requests: Vec<&String> = requests
            .iter()
            .filter(|path| path.contains("type=feed"))
            .collect();
        assert_eq!(feed_requests.len(), 1, "{requests:?}");
        assert!(!feed_requests[0].contains("cursor"), "{requests:?}");
    }

    /// R86-5-15: a failed feed request with a passed track request sets
    /// `feed_rows_partial`.
    #[test]
    fn adr_0086_saved_search_groups_r15_failed_feed_request_is_partial() {
        let (endpoint, _) = serve(
            3,
            feed_search_responder((500, "{\"error\":\"down\"}".to_owned())),
        );

        let rows = fetch_index_search_result_rows(
            &endpoint.as_str().into(),
            "zz",
            &NameFeedMatches::default(),
            &CancellationToken::new(),
        )
        .expect("the track request passes");

        assert!(rows.feed_rows_partial);
        assert!(rows.feeds.is_empty());
    }

    /// R86-5-16: the row takes its artist text from `release_artist`. An
    /// empty `release_artist` gives none. `newest_item_at` stays `None`.
    #[test]
    fn adr_0086_saved_search_groups_r16_artist_text_from_release_artist() {
        let hit = |artist: &str| -> SearchResult {
            serde_json::from_value(serde_json::json!({
                "entity_type": "feed", "entity_id": "g1", "feed_guid": "g1",
                "title": "One", "release_artist": artist
            }))
            .expect("a recorded hit")
        };
        let row = index_feed_result_display(&hit("jSmovie"));
        assert_eq!(row.artist_text.as_deref(), Some("jSmovie"));
        assert_eq!(row.newest_item_at, None);
        let row = index_feed_result_display(&hit("  "));
        assert_eq!(row.artist_text, None);
    }

    /// R87-2-23: the command sends the `q` of the plan in three search
    /// requests. No request path holds a field token.
    #[test]
    fn adr_0087_library_fields_r87_2_23_index_request_holds_only_words() {
        use crate::view_models::search_query::SearchQuery;
        use crate::view_models::search_results::IndexSearchPlan;

        let query = SearchQuery::parse("haleen \"night drive\" in:index");
        let IndexSearchPlan::Send(request) = IndexSearchPlan::for_query(&query) else {
            panic!("the query sends the Index search");
        };
        assert_eq!(Some(request.q.clone()), query.index_q());
        let (endpoint, requests) = serve(3, Arc::new(|_: &str| (200, empty_search())));

        FetchIndexSearchResults::new(endpoint, request, NameFeedMatches::default())
            .execute(&CommandContext::next())
            .expect("the recorded responses decode");

        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 3, "Got: {requests:?}");
        for path in &requests {
            assert!(path.starts_with("/v1/search?"), "{path}");
            assert!(path.contains("haleen"), "{path}");
            for token in ["in%3A", "artist%3A", "kind%3A", "after%3A", "in:"] {
                assert!(!path.contains(token), "{path} holds {token}");
            }
        }
    }
}

/// ADR 0087 packet 002: the Library search applies the fields of the typed
/// query, with recorded databases and facts from the recorded tree.
#[cfg(test)]
mod adr_0087_library_fields_tests {
    use super::*;
    use crate::view_models::library::{AlbumNode, ArtistNode, LibraryTree};
    use crate::views::{FeedMetadataFacts, LocalIdentityFacts};

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    /// One recorded feed with its stated release kind.
    fn feed(conn: &Connection, title: &str, stated_kind: Option<&str>) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, title, extra_json) VALUES (?1, ?2, '{}')",
            rusqlite::params![format!("https://example.test/{title}.xml"), title],
        )?;
        let id = conn.last_insert_rowid();
        if let Some(kind) = stated_kind {
            KINDS.with(|kinds| kinds.borrow_mut().insert(id, kind.to_owned()));
        }
        Ok(id)
    }

    thread_local! {
        /// The stated release kind of each recorded feed of this thread.
        static KINDS: std::cell::RefCell<BTreeMap<i64, String>> =
            const { std::cell::RefCell::new(BTreeMap::new()) };
    }

    /// One recorded Library track. `pub_date` is RFC 2822 text.
    fn track(
        conn: &Connection,
        feed_id: i64,
        title: &str,
        number: i64,
        pub_date: Option<&str>,
    ) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, track_number, pub_date, is_in_library)
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            rusqlite::params![feed_id, format!("{feed_id}-{title}"), title, number, pub_date],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// The Library tree of the recorded tracks: one album for each feed,
    /// with the stated kind of the feed.
    fn tree(conn: &Connection) -> anyhow::Result<LibraryTree> {
        let mut albums = BTreeMap::<i64, Vec<db::TrackRow>>::new();
        for row in db::library_tracks(conn)? {
            albums.entry(row.feed_id).or_default().push(row);
        }
        let albums = albums
            .into_iter()
            .map(|(feed_id, tracks)| AlbumNode {
                name: format!("feed {feed_id}"),
                feed_id: Some(feed_id),
                feed_guid: None,
                feed_url: None,
                language: None,
                description: None,
                image_href: None,
                named_feeds: Vec::new(),
                identity_facts: LocalIdentityFacts::default(),
                metadata_facts: Box::new(FeedMetadataFacts {
                    release_kind: KINDS.with(|kinds| kinds.borrow().get(&feed_id).cloned()),
                    ..FeedMetadataFacts::default()
                }),
                stored_values: None,
                tracks,
            })
            .collect();
        Ok(LibraryTree {
            artists: vec![ArtistNode {
                name: "Artist".to_owned(),
                albums,
            }],
        })
    }

    fn facts(conn: &Connection) -> anyhow::Result<LibrarySearchFacts> {
        Ok(LibrarySearchFacts::from_tree(&tree(conn)?, None))
    }

    fn search(
        conn: &Connection,
        text: &str,
        facts: &LibrarySearchFacts,
    ) -> anyhow::Result<LibrarySearchRows> {
        Ok(ApplicationQueryService::new().search_local_library_tracks(
            conn,
            &SearchQuery::parse(text),
            facts,
            None,
        )?)
    }

    fn ids(rows: &[db::TrackRow]) -> Vec<i64> {
        let mut ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
        ids.sort_unstable();
        ids
    }

    /// R87-2-05: `kind:` reads the kind of the tree release.
    #[test]
    fn adr_0087_library_fields_r87_2_05_kind_reads_the_tree() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let single_feed = feed(&conn, "S", Some("single"))?;
        let single = track(&conn, single_feed, "Needle single", 1, None)?;
        let album_feed = feed(&conn, "A", None)?;
        let album_one = track(&conn, album_feed, "Needle one", 1, None)?;
        let album_two = track(&conn, album_feed, "Needle two", 2, None)?;
        let ep_feed = feed(&conn, "E", Some("ep"))?;
        let ep = track(&conn, ep_feed, "Needle ep", 1, None)?;
        let facts = facts(&conn)?;

        assert_eq!(
            ids(&search(&conn, "kind:single", &facts)?.tracks),
            vec![single]
        );
        assert_eq!(
            ids(&search(&conn, "kind:album,ep", &facts)?.tracks),
            vec![album_one, album_two, ep]
        );
        assert_eq!(
            ids(&search(&conn, "needle -kind:single", &facts)?.tracks),
            vec![album_one, album_two, ep]
        );
        Ok(())
    }

    /// R87-2-07: a release row reads the release date. A track row reads
    /// its own `pub_date`.
    #[test]
    fn adr_0087_library_fields_r87_2_07_dates_split_the_rows() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let album = feed(&conn, "Album", None)?;
        let early = track(
            &conn,
            album,
            "Early",
            1,
            Some("Thu, 01 Oct 2026 12:00:00 +0000"),
        )?;
        let late = track(
            &conn,
            album,
            "Late",
            2,
            Some("Tue, 20 Oct 2026 12:00:00 +0000"),
        )?;
        let facts = facts(&conn)?;

        let before = search(&conn, "before:2026-10-10", &facts)?;
        assert_eq!(ids(&before.tracks), vec![early]);
        assert!(before.releases.is_empty());
        assert_eq!(before.matched_count, 1);

        let after = search(&conn, "after:2026-10-15", &facts)?;
        assert_eq!(ids(&after.releases), vec![early, late]);
        assert_eq!(ids(&after.tracks), vec![late]);
        Ok(())
    }

    /// R87-2-08: a month includes its first and its last second. A missing
    /// date does not pass a term, and passes the exclusion.
    #[test]
    fn adr_0087_library_fields_r87_2_08_month_bounds_and_missing_date() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let first = feed(&conn, "First", None)?;
        let first_second = track(
            &conn,
            first,
            "Needle first",
            1,
            Some("Thu, 01 Oct 2026 00:00:00 +0000"),
        )?;
        let last = feed(&conn, "Last", None)?;
        let last_second = track(
            &conn,
            last,
            "Needle last",
            1,
            Some("Sat, 31 Oct 2026 23:59:59 +0000"),
        )?;
        let undated_feed = feed(&conn, "Undated", None)?;
        let undated = track(&conn, undated_feed, "Needle undated", 1, None)?;
        let facts = facts(&conn)?;

        assert!(ids(&search(&conn, "after:2026-10", &facts)?.tracks).contains(&first_second));
        assert!(ids(&search(&conn, "before:2026-10", &facts)?.tracks).contains(&last_second));
        assert!(!ids(&search(&conn, "after:2026-01-01", &facts)?.tracks).contains(&undated));
        assert!(ids(&search(&conn, "needle -after:2026-01-01", &facts)?.tracks).contains(&undated));
        Ok(())
    }

    /// R87-2-12: `is:ready` reads the ready set of the last readiness
    /// report. With no report, it matches no track.
    #[test]
    fn adr_0087_library_fields_r87_2_12_ready_reads_the_kept_report() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed", None)?;
        let ready = track(&conn, feed_id, "Needle ready", 1, None)?;
        let other = track(&conn, feed_id, "Needle other", 2, None)?;
        let tree = tree(&conn)?;
        let set = BTreeSet::from([ready]);
        let with_report = LibrarySearchFacts::from_tree(&tree, Some(&set));
        let no_report = LibrarySearchFacts::from_tree(&tree, None);

        assert_eq!(
            ids(&search(&conn, "needle is:ready", &with_report)?.tracks),
            vec![ready]
        );
        assert!(search(&conn, "needle is:ready", &no_report)?
            .tracks
            .is_empty());
        assert_eq!(
            ids(&search(&conn, "needle -is:ready", &no_report)?.tracks),
            vec![ready, other]
        );
        Ok(())
    }

    /// R87-2-18: the id set goes into SQL, so `LIMIT` gives the first rows
    /// that pass each term.
    #[test]
    fn adr_0087_library_fields_r87_2_18_limit_counts_rows_that_pass() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let album = feed(&conn, "A Album", None)?;
        for number in 0..50 {
            track(&conn, album, &format!("Needle {number:02}"), number, None)?;
        }
        for index in 0..10 {
            let single = feed(&conn, &format!("Z Single {index}"), Some("single"))?;
            track(&conn, single, "Needle single", 1, None)?;
        }
        let facts = facts(&conn)?;

        let singles = search(&conn, "needle kind:single", &facts)?;
        assert_eq!(singles.tracks.len(), 10);
        let all = search(&conn, "needle", &facts)?;
        assert_eq!(all.tracks.len(), DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT);
        assert_eq!(all.matched_count, 50);
        Ok(())
    }

    /// R87-2-19: a field that applies nothing does not change the rows. A
    /// query with no term that can run gives no row and runs no SQL.
    #[test]
    fn adr_0087_library_fields_r87_2_19_fields_that_apply_nothing() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed", None)?;
        track(&conn, feed_id, "Haleen", 1, None)?;
        track(&conn, feed_id, "Haleen remix", 2, None)?;
        let facts = facts(&conn)?;
        let plain = ids(&search(&conn, "haleen", &facts)?.tracks);
        assert_eq!(plain.len(), 2);
        assert_eq!(
            ids(&search(&conn, "haleen after:yesterday", &facts)?.tracks),
            plain
        );
        assert_eq!(ids(&search(&conn, "haleen artist:", &facts)?.tracks), plain);

        // A connection with no table fails each statement, so an empty
        // result proves that no SQL ran.
        let no_tables = Connection::open_in_memory()?;
        for text in ["-remix", "!!!", "in:library", "after:yesterday"] {
            let rows = search(&no_tables, text, &facts)?;
            assert!(rows.tracks.is_empty() && rows.releases.is_empty(), "{text}");
            assert_eq!(rows.matched_count, 0, "{text}");
        }
        Ok(())
    }

    /// R87-2-20: a query for the Index only, or with a scope conflict,
    /// runs no SQL.
    #[test]
    fn adr_0087_library_fields_r87_2_20_index_scope_runs_no_sql() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed", None)?;
        track(&conn, feed_id, "Haleen", 1, None)?;
        let facts = facts(&conn)?;
        let no_tables = Connection::open_in_memory()?;
        for text in ["in:index haleen", "in:index is:ready"] {
            assert!(search(&conn, text, &facts)?.tracks.is_empty(), "{text}");
            assert!(
                search(&no_tables, text, &facts)?.tracks.is_empty(),
                "{text}"
            );
        }
        Ok(())
    }
}
