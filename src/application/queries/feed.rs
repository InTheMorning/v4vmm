//! Feed local query family.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rusqlite::Connection;

use crate::api::{
    Artist, Client, Contributor, Feed, PaymentRoute, Publisher, RecentFeedsResponse, Track,
};
use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::db;
use crate::metadata::{
    sanitize_feed_source_text, sanitize_track_context_source_text, TrackContext,
};
use crate::rss;
use crate::subscribe_service::enrich_track_context_from_rss;
use crate::view_models::recent_feeds::RecentFeedsPageBatch;
use crate::view_models::track::TrackVm;

use super::search::{index_feed_display, index_item_id, non_empty_str, INDEX_FEED_ID_BASE};
use crate::application::request_profiles::{
    INDEX_FEED_DETAIL, INSPECTOR_TRACK_DETAIL_FEED, INSPECTOR_TRACK_DETAIL_TRACK,
};

/// Fetches one remote Recent Feeds page for presentation.
#[derive(Clone, Debug)]
pub(crate) struct FetchRecentFeedsPage {
    endpoint: crate::config::MusicIndexEndpoint,
    cursor: Option<String>,
    resume_after: usize,
}

impl FetchRecentFeedsPage {
    /// Creates a Recent Feeds page query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        cursor: Option<String>,
        resume_after: usize,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            cursor,
            resume_after,
        }
    }
}

impl ApplicationCommand for FetchRecentFeedsPage {
    type Output = RecentFeedsPageBatch;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let batch = fetch_recent_feed_result_rows(
            &self.endpoint,
            self.cursor.as_deref(),
            self.resume_after,
        )
        .map_err(|error| query_error(&error))?;
        Ok(CommandOutcome::without_events(batch))
    }
}

/// Fetches one parked Discover recent-feeds page.
#[derive(Clone, Debug)]
pub(crate) struct FetchDiscoverRecentFeeds {
    endpoint: crate::config::MusicIndexEndpoint,
    cursor: Option<String>,
}

impl FetchDiscoverRecentFeeds {
    /// Creates a parked Discover recent-feeds query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        cursor: Option<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            cursor,
        }
    }
}

impl ApplicationCommand for FetchDiscoverRecentFeeds {
    type Output = RecentFeedsResponse;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.endpoint);
        let response = client
            .fetch_recent_feeds(Some(crate::api::PAGE_LIMIT), self.cursor.as_deref())
            .map_err(|error| query_error(&error))?;
        Ok(CommandOutcome::without_events(response))
    }
}

/// Neutral inspector detail payload for parked Discover.
#[derive(Clone, Debug)]
pub(crate) enum InspectorDetailData {
    /// Artist detail payload.
    Artist(Box<ArtistContextData>),
    /// Feed detail payload.
    Feed(Box<Feed>),
    /// Track detail payload.
    Track(Box<TrackContext>),
    /// Publisher detail payload.
    Publisher(Publisher),
}

/// Neutral artist detail payload for parked Discover.
#[derive(Clone, Debug)]
pub(crate) struct ArtistContextData {
    pub(crate) artist: Artist,
    pub(crate) tracks: Vec<Track>,
    pub(crate) feeds: Vec<Feed>,
    pub(crate) has_more_tracks: bool,
}

/// Structured inspector detail plus the hero image URL to fetch separately.
#[derive(Clone, Debug)]
pub(crate) struct InspectorDetailResult {
    pub(crate) detail: InspectorDetailData,
    pub(crate) image_url: Option<String>,
}

/// Fetches a parked Discover inspector detail payload.
#[derive(Clone, Debug)]
pub(crate) struct FetchInspectorDetail {
    endpoint: crate::config::MusicIndexEndpoint,
    entity_type: String,
    entity_id: String,
    feed_guid: Option<String>,
}

impl FetchInspectorDetail {
    /// Creates an inspector detail query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        feed_guid: Option<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            feed_guid,
        }
    }
}

impl ApplicationCommand for FetchInspectorDetail {
    type Output = InspectorDetailResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.endpoint);
        fetch_inspector_detail(
            &client,
            &self.entity_type,
            &self.entity_id,
            self.feed_guid.as_deref(),
        )
        .map_err(|error| query_error(&error))
        .map(CommandOutcome::without_events)
    }
}

/// Fetches source contributors for a parked Discover inspector entity.
#[derive(Clone, Debug)]
pub(crate) struct FetchContributors {
    endpoint: crate::config::MusicIndexEndpoint,
    entity_type: String,
    entity_id: String,
}

impl FetchContributors {
    /// Creates a contributors query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
        }
    }
}

impl ApplicationCommand for FetchContributors {
    type Output = Vec<Contributor>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.endpoint);
        client
            .fetch_contributors(&self.entity_type, &self.entity_id)
            .map_err(|error| query_error(&error))
            .map(CommandOutcome::without_events)
    }
}

/// Fetches value routes for a parked Discover inspector entity.
#[derive(Clone, Debug)]
pub(crate) struct FetchValueRoutes {
    endpoint: crate::config::MusicIndexEndpoint,
    entity_type: String,
    entity_id: String,
}

impl FetchValueRoutes {
    /// Creates a value-routes query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
        }
    }
}

impl ApplicationCommand for FetchValueRoutes {
    type Output = Vec<PaymentRoute>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.endpoint);
        client
            .fetch_value_routes(&self.entity_type, &self.entity_id)
            .map_err(|error| query_error(&error))
            .map(CommandOutcome::without_events)
    }
}

/// Resolves podroll feed references for a parked Discover feed inspector.
#[derive(Clone, Debug)]
pub(crate) struct ResolvePodrollFeeds {
    endpoint: crate::config::MusicIndexEndpoint,
    feed_url: String,
}

impl ResolvePodrollFeeds {
    /// Creates a podroll resolution query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        feed_url: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            feed_url: feed_url.into(),
        }
    }
}

impl ApplicationCommand for ResolvePodrollFeeds {
    type Output = Vec<Feed>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.endpoint);
        resolve_podroll_feeds(&client, &self.feed_url)
            .map_err(|error| query_error(&error))
            .map(CommandOutcome::without_events)
    }
}

fn fetch_recent_feed_result_rows(
    endpoint: &crate::config::MusicIndexEndpoint,
    cursor: Option<&str>,
    start_index: usize,
) -> Result<RecentFeedsPageBatch> {
    let client = crate::api::Client::new_with_base_url(endpoint.clone());
    let response = client.fetch_recent_feeds(Some(crate::api::PAGE_LIMIT), cursor)?;
    let rows = response
        .data
        .into_iter()
        .enumerate()
        .map(|(index, feed)| {
            let row_index = start_index + index;
            let feed_guid = recent_feed_activation_id(&feed, row_index);
            let detail = feed
                .feed_guid
                .as_deref()
                .and_then(|guid| {
                    client
                        .fetch_feed_with_profile(guid, &INDEX_FEED_DETAIL)
                        .ok()
                })
                .unwrap_or(feed);
            (
                index_item_id(INDEX_FEED_ID_BASE, row_index),
                index_feed_display(&feed_guid, Some(crate::api::EntityDetail::Feed(detail))),
            )
        })
        .collect();

    Ok(RecentFeedsPageBatch {
        rows,
        cursor: response.pagination.cursor,
        has_more: response.pagination.has_more,
    })
}

fn recent_feed_activation_id(feed: &crate::api::Feed, index: usize) -> String {
    [
        feed.feed_guid.as_deref(),
        feed.feed_url.as_deref(),
        feed.title.as_deref(),
        feed.name.as_deref(),
    ]
    .into_iter()
    .find_map(non_empty_str)
    .map_or_else(|| format!("recent-feed-{index}"), str::to_string)
}

fn fetch_inspector_detail(
    client: &Client,
    entity_type: &str,
    entity_id: &str,
    feed_guid: Option<&str>,
) -> Result<InspectorDetailResult> {
    match entity_type {
        "artist" => fetch_artist_detail(client, entity_id),
        "feed" => fetch_feed_detail(client, entity_id),
        "track" => fetch_track_detail(client, entity_id, feed_guid),
        "publisher" => Ok(InspectorDetailResult {
            detail: InspectorDetailData::Publisher(client.fetch_publisher(entity_id)?),
            image_url: None,
        }),
        _ => Err(anyhow::anyhow!(
            "unknown inspector entity type: {entity_type}"
        )),
    }
}

fn fetch_artist_detail(client: &Client, entity_id: &str) -> Result<InspectorDetailResult> {
    let response =
        client.fetch_tracks_by_artist(entity_id, Some(crate::api::PAGE_LIMIT * 2), None)?;
    let tracks = response.data;
    let has_more_tracks = response.pagination.has_more;
    let (feeds, image_url) = artist_feeds_and_image(client, &tracks);
    let artist = Artist {
        name: Some(entity_id.to_string()),
        image_url: image_url.clone(),
        track_count: Some(bounded_i32_count(tracks.len())),
        feed_count: Some(bounded_i32_count(feeds.len())),
        ..Artist::default()
    };

    Ok(InspectorDetailResult {
        detail: InspectorDetailData::Artist(Box::new(ArtistContextData {
            artist,
            tracks,
            feeds,
            has_more_tracks,
        })),
        image_url,
    })
}

fn artist_feeds_and_image(client: &Client, tracks: &[Track]) -> (Vec<Feed>, Option<String>) {
    let mut feed_order: Vec<String> = Vec::new();
    let mut artist_track_count_by_feed: BTreeMap<String, i32> = BTreeMap::new();
    for track in tracks {
        let Some(guid) = track
            .feed_guid
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let key = guid.to_string();
        let entry = artist_track_count_by_feed.entry(key.clone()).or_insert(0);
        if *entry == 0 {
            feed_order.push(key);
        }
        *entry = entry.saturating_add(1);
    }

    let feeds = feed_order
        .iter()
        .map(|guid| artist_feed_for_guid(client, tracks, &artist_track_count_by_feed, guid))
        .collect::<Vec<_>>();
    let image_url = feeds
        .iter()
        .find_map(|feed| nonempty_url(feed.image_url.as_deref()).map(str::to_string))
        .or_else(|| {
            tracks
                .iter()
                .find_map(|track| nonempty_url(track.image_url.as_deref()).map(str::to_string))
        });
    (feeds, image_url)
}

fn artist_feed_for_guid(
    client: &Client,
    tracks: &[Track],
    artist_track_count_by_feed: &BTreeMap<String, i32>,
    guid: &str,
) -> Feed {
    let artist_tracks_in_feed = artist_track_count_by_feed
        .get(guid)
        .copied()
        .unwrap_or_default();
    match client.fetch_feed(guid, None).ok() {
        Some(mut feed) => {
            feed.episode_count = Some(artist_tracks_in_feed);
            feed
        }
        None => {
            let fallback_title = tracks
                .iter()
                .find(|track| track.feed_guid.as_deref() == Some(guid))
                .and_then(|track| track.feed_title.clone());
            Feed {
                feed_guid: Some(guid.to_string()),
                title: fallback_title,
                episode_count: Some(artist_tracks_in_feed),
                ..Feed::default()
            }
        }
    }
}

fn fetch_feed_detail(client: &Client, entity_id: &str) -> Result<InspectorDetailResult> {
    let mut feed = client.fetch_feed_with_profile(entity_id, &INDEX_FEED_DETAIL)?;
    hydrate_feed_track_play_urls(client, &mut feed);
    sanitize_feed_source_text(&mut feed);
    let image_url = feed
        .image_url
        .as_deref()
        .and_then(|url| nonempty_url(Some(url)))
        .map(str::to_string);
    Ok(InspectorDetailResult {
        detail: InspectorDetailData::Feed(Box::new(feed)),
        image_url,
    })
}

fn fetch_track_detail(
    client: &Client,
    entity_id: &str,
    feed_guid: Option<&str>,
) -> Result<InspectorDetailResult> {
    let track = fetch_scoped_track(
        client,
        entity_id,
        feed_guid,
        INSPECTOR_TRACK_DETAIL_TRACK.include(),
    )?;
    let feed = track.feed_guid.as_deref().and_then(|guid| {
        client
            .fetch_feed_with_profile(guid, &INSPECTOR_TRACK_DETAIL_FEED)
            .ok()
    });
    let mut track_context = TrackContext::new(track, feed);
    enrich_track_context_from_rss(&mut track_context);
    sanitize_track_context_source_text(&mut track_context);
    let image_url = track_context
        .track
        .image_url
        .as_deref()
        .and_then(|url| nonempty_url(Some(url)))
        .map(str::to_string);
    Ok(InspectorDetailResult {
        detail: InspectorDetailData::Track(Box::new(track_context)),
        image_url,
    })
}

fn fetch_scoped_track(
    client: &Client,
    track_guid: &str,
    feed_guid: Option<&str>,
    include: Option<&str>,
) -> Result<Track> {
    match feed_guid.map(str::trim).filter(|guid| !guid.is_empty()) {
        Some(feed_guid) => client.fetch_feed_track(feed_guid, track_guid, include),
        None => client.fetch_track(track_guid, include),
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
            .filter(|value| !value.is_empty());
        let key = guid
            .map(str::to_string)
            .or_else(|| entry.feed_url.clone())
            .unwrap_or_default();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        let fetched = guid.and_then(|value| client.fetch_feed(value, None).ok());
        let feed = match fetched {
            Some(feed) => feed,
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
        let Ok(hydrated) = fetch_scoped_track(
            client,
            &track_guid,
            track.feed_guid.as_deref(),
            Some("source_enclosures"),
        ) else {
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

fn nonempty_url(url: Option<&str>) -> Option<&str> {
    url.map(str::trim).filter(|url| !url.is_empty())
}

fn bounded_i32_count(len: usize) -> i32 {
    i32::try_from(len).unwrap_or(i32::MAX)
}

impl ApplicationQueryService {
    /// Lists subscribed feeds that can be checked for remote updates.
    ///
    /// # Errors
    ///
    /// Returns an error when local feed state cannot be read.
    pub fn subscribed_feeds_for_stale_check(
        &self,
        conn: &Connection,
    ) -> Result<Vec<db::FeedStaleCheckRow>, CommandError> {
        db::subscribed_feeds_for_stale_check(conn).map_err(|error| query_error(&error))
    }
}

fn query_error(error: &anyhow::Error) -> CommandError {
    CommandError::Query(format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    #[test]
    fn feed_queries_return_local_stale_check_rows() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title, is_subscribed)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;

        let rows = ApplicationQueryService::new().subscribed_feeds_for_stale_check(&conn)?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].feed_guid, "feed-guid");

        Ok(())
    }
}

/// ADR 0075 packet 017: request-profile behavior at the Recent Feeds page,
/// the Feed detail, and the inspector track detail call sites.
#[cfg(test)]
mod adr_0075_request_profile_tests {
    use super::*;
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
        if bare == "/v1/feeds/recent" {
            return serde_json::json!({
                "data": [{"feed_guid": "f1"}],
                "pagination": {"has_more": false}
            })
            .to_string();
        }
        if bare.contains("/tracks/") {
            return serde_json::json!({"data": {
                "track_guid": bare.rsplit('/').next(),
                "feed_guid": "f1"
            }})
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

    /// R17-05: the Index feed detail profile serves
    /// `fetch_recent_feed_result_rows`, and this call site sends L2.
    #[test]
    fn adr_0075_request_profile_recent_feed_result_rows_sends_l2() {
        let fixture = Fixture::start();

        let batch = fetch_recent_feed_result_rows(&fixture.endpoint, None, 0).unwrap();

        assert_eq!(batch.rows.len(), 1);
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests.len(),
            2,
            "one recent-feeds request, one feed detail request"
        );
        assert!(
            requests[1].starts_with(&format!(
                "/v1/feeds/f1?include={}",
                encoded_include(INDEX_FEED_DETAIL)
            )),
            "R17-05: the Recent Feeds feed detail call site must send L2. Got: {requests:?}"
        );
    }

    /// R17-05: the Index feed detail profile serves `fetch_feed_detail`,
    /// and this call site sends L2.
    #[test]
    fn adr_0075_request_profile_inspector_feed_detail_sends_l2() {
        let fixture = Fixture::start();
        let client = crate::api::Client::new_with_base_url(fixture.endpoint.clone());

        let result = fetch_feed_detail(&client, "f1").unwrap();

        assert!(matches!(result.detail, InspectorDetailData::Feed(_)));
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests,
            vec![format!(
                "/v1/feeds/f1?include={}",
                encoded_include(INDEX_FEED_DETAIL)
            )],
            "R17-05: the inspector feed detail call site must send L2"
        );
    }

    /// R17-06: the inspector track detail route sends L3 for its track
    /// request, then L4 for its feed request.
    #[test]
    fn adr_0075_request_profile_inspector_track_detail_sends_l3_then_l4() {
        let fixture = Fixture::start();
        let client = crate::api::Client::new_with_base_url(fixture.endpoint.clone());

        let result = fetch_track_detail(&client, "t1", Some("f1")).unwrap();

        assert!(matches!(result.detail, InspectorDetailData::Track(_)));
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests,
            vec![
                format!(
                    "/v1/feeds/f1/tracks/t1?include={}",
                    encoded_include(INSPECTOR_TRACK_DETAIL_TRACK)
                ),
                format!(
                    "/v1/feeds/f1?include={}",
                    encoded_include(INSPECTOR_TRACK_DETAIL_FEED)
                ),
            ],
            "R17-06: the inspector track detail route must send L3 then L4, in that order"
        );
    }
}
