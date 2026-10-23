//! Feed local query family.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use rusqlite::Connection;

use crate::api::{Client, Feed};
use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::queries::PARALLEL_DETAIL_READS;
use crate::db;
use crate::view_models::publisher_page::{
    CoCreditedFeedFact, PublisherPageAlbumFact, PublisherPageFacts,
};
use crate::view_models::recent_feeds::{RecentFeedResultRow, RecentFeedsPageBatch};

use super::search::{index_feed_display, index_item_id, non_empty_str, INDEX_FEED_ID_BASE};
use crate::application::request_profiles::{INDEX_FEED_DETAIL, INDEX_PUBLISHER_PAGE};
use crate::application::request_reuse::{self, RefreshIntent, RequestKey, SharedFetchError};

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

/// Reads an Index publisher page (ADR 0077 packet 004).
#[derive(Clone, Debug)]
pub(crate) struct FetchIndexPublisherPage {
    conn: Arc<Mutex<Connection>>,
    endpoint: crate::config::MusicIndexEndpoint,
    publisher_feed_guid: String,
}

impl FetchIndexPublisherPage {
    /// Creates an Index publisher page query command.
    #[must_use]
    pub(crate) fn new(
        conn: Arc<Mutex<Connection>>,
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        publisher_feed_guid: impl Into<String>,
    ) -> Self {
        Self {
            conn,
            endpoint: endpoint.into(),
            publisher_feed_guid: publisher_feed_guid.into(),
        }
    }
}

impl ApplicationCommand for FetchIndexPublisherPage {
    type Output = PublisherPageFacts;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let provider_identity = self
            .endpoint
            .require()
            .map(str::to_owned)
            .unwrap_or_default();
        let client = Client::new_with_base_url(self.endpoint);
        let db = self
            .conn
            .lock()
            .map_err(|_| CommandError::Query("database lock poisoned".into()))?;
        fetch_index_publisher_page(&db, &client, &provider_identity, &self.publisher_feed_guid)
            .map_err(|error| query_error(&error))
            .map(CommandOutcome::without_events)
    }
}

/// Reads the list response, then reads the detail of each feed, at most
/// [`PARALLEL_DETAIL_READS`] detail reads at the same time (ADR 0086
/// Decision 5). This runs the `std::thread::scope` pattern of
/// `fetch_feed_singles_set` in `src/application/queries/name_lists.rs`
/// (ADR 0084 packet 026 decision D4): each worker takes the next feed
/// index from a shared counter, so the rows keep the order of the list
/// response even when the server answers the detail requests in a
/// different order.
///
/// # Errors
///
/// Returns an error when the list request fails or does not decode.
fn fetch_recent_feed_result_rows(
    endpoint: &crate::config::MusicIndexEndpoint,
    cursor: Option<&str>,
    start_index: usize,
) -> Result<RecentFeedsPageBatch> {
    let client = crate::api::Client::new_with_base_url(endpoint.clone());
    let response = client.fetch_recent_feeds(Some(crate::api::PAGE_LIMIT), cursor)?;
    let feeds = response.data;

    let next_index = AtomicUsize::new(0);
    let rows_by_index: Mutex<Vec<Option<RecentFeedResultRow>>> =
        Mutex::new(vec![None; feeds.len()]);
    std::thread::scope(|scope| {
        for _ in 0..PARALLEL_DETAIL_READS.min(feeds.len()) {
            scope.spawn(|| loop {
                let index = next_index.fetch_add(1, Ordering::SeqCst);
                let Some(feed) = feeds.get(index) else {
                    return;
                };
                let row = recent_feed_result_row(&client, feed, start_index + index);
                if let Ok(mut rows_by_index) = rows_by_index.lock() {
                    rows_by_index[index] = Some(row);
                }
            });
        }
    });

    // A missing row only happens when its worker never ran (a lock
    // poisoned by a panic in another worker). Build it here, so the row
    // order stays correct and no row goes missing.
    let rows_by_index = rows_by_index
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rows = feeds
        .iter()
        .zip(rows_by_index)
        .enumerate()
        .map(|(index, (feed, row))| {
            row.unwrap_or_else(|| recent_feed_result_row(&client, feed, start_index + index))
        })
        .collect();

    Ok(RecentFeedsPageBatch {
        rows,
        cursor: response.pagination.cursor,
        has_more: response.pagination.has_more,
    })
}

/// Builds one row of [`fetch_recent_feed_result_rows`]: the detail feed
/// when the list feed names a feed GUID and the detail request gives a
/// result, else the list feed. This fallback does not change with ADR
/// 0086 Task 001.
fn recent_feed_result_row(client: &Client, feed: &Feed, row_index: usize) -> RecentFeedResultRow {
    let feed_guid = recent_feed_activation_id(feed, row_index);
    let detail = feed
        .feed_guid
        .as_deref()
        .and_then(|guid| {
            client
                .fetch_feed_with_profile(guid, &INDEX_FEED_DETAIL)
                .ok()
        })
        .unwrap_or_else(|| feed.clone());
    (
        index_item_id(INDEX_FEED_ID_BASE, row_index),
        index_feed_display(&feed_guid, Some(crate::api::EntityDetail::Feed(detail))),
    )
}

fn recent_feed_activation_id(feed: &crate::api::Feed, index: usize) -> String {
    [
        feed.feed_guid.as_deref(),
        feed.feed_url.as_deref(),
        feed.title.as_deref(),
    ]
    .into_iter()
    .find_map(non_empty_str)
    .map_or_else(|| format!("recent-feed-{index}"), str::to_string)
}

/// Packet 018 R18B-12: asks the shared owner for this feed (ADR 0075
/// section 6), instead of `Client` directly. `profile` names the P18-2
/// window this feed identity shares with every other Index feed request
/// (`RequestKey::feed`'s own subject scoping); `None` names an unprofiled
/// (`include=None`) request, its own distinct identity under the same
/// window.
///
/// Neither this file nor `search.rs` has an observation recorder — a
/// guard in `search.rs` forbids one — so this closure produces no
/// receipts, and the owner's own empty list is expected and discarded.
fn owner_fetch_feed(
    client: &Client,
    provider_identity: &str,
    feed_guid: &str,
    profile: Option<&crate::application::request_profiles::RequestProfile>,
) -> Result<Feed> {
    let include = profile.and_then(crate::application::request_profiles::RequestProfile::include);
    let key = RequestKey::feed(provider_identity, feed_guid, include);
    request_reuse::shared()
        .fetch_feed_with_receipts(key, RefreshIntent::Normal, || {
            let feed = match profile {
                Some(profile) => client.fetch_feed_with_profile(feed_guid, profile)?,
                None => client.fetch_feed(feed_guid, None)?,
            };
            Ok((feed, Vec::new()))
        })
        .0
        .map(|(feed, _receipts)| feed)
        .map_err(SharedFetchError::into_anyhow)
}

/// Reads the albums of one publisher feed (ADR 0077 Task 003). It sends one
/// `INDEX_PUBLISHER_PAGE` request through the packet 018 shared owner, and
/// no request for an album: each album's title, image, artist text, and
/// artist source come from the `remote_*` fields of its own
/// `publisher_to_music` entry (Stophammer ADR 0059).
///
/// Every album's `in_library` flag starts `false`. The Index publisher page
/// query below sets it from local storage; the Library publisher page query
/// in `library.rs` calls this function directly and sets it from its own
/// local read instead.
///
/// # Errors
///
/// Returns an error when the MusicIndex request fails.
pub(crate) fn fetch_index_publisher_page_albums(
    client: &Client,
    provider_identity: &str,
    publisher_feed_guid: &str,
) -> Result<PublisherPageFacts> {
    let feed = owner_fetch_feed(
        client,
        provider_identity,
        publisher_feed_guid,
        Some(&INDEX_PUBLISHER_PAGE),
    )?;
    Ok(publisher_page_facts_from_feed(publisher_feed_guid, &feed))
}

/// Reads the Index publisher page (ADR 0077 Task 003, R3-02b): the albums of
/// `fetch_index_publisher_page_albums`, each marked with whether a stored
/// `music_to_publisher` row already names it as a Library album.
///
/// # Errors
///
/// Returns an error when the MusicIndex request fails, or when the local
/// read of the Library marking fails.
pub(crate) fn fetch_index_publisher_page(
    conn: &Connection,
    client: &Client,
    provider_identity: &str,
    publisher_feed_guid: &str,
) -> Result<PublisherPageFacts> {
    let mut facts =
        fetch_index_publisher_page_albums(client, provider_identity, publisher_feed_guid)?;
    let library_feed_guids: BTreeSet<String> =
        db::publisher_relationships::local_albums_for_publisher(conn, publisher_feed_guid)?
            .into_iter()
            .filter_map(|album| album.feed_guid)
            .collect();
    for album in &mut facts.albums {
        album.in_library = album
            .feed_guid
            .as_deref()
            .is_some_and(|guid| library_feed_guids.contains(guid));
    }
    Ok(facts)
}

fn publisher_page_facts_from_feed(publisher_feed_guid: &str, feed: &Feed) -> PublisherPageFacts {
    let albums = feed
        .publisher
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|entry| entry.direction.as_deref() == Some("publisher_to_music"))
        .map(|entry| PublisherPageAlbumFact {
            // A feed can list an album under a GUID that is not the GUID
            // of the album. Stophammer then resolves the link by the feed
            // URL, and `music_feed_guid` gives the album that it found. The
            // release route and the Library mark need that GUID (operator
            // check of 2026-10-18: each DETOX row opened a 404).
            feed_guid: entry
                .music_feed_guid
                .clone()
                .filter(|guid| !guid.trim().is_empty())
                .or_else(|| entry.remote_feed_guid.clone()),
            title: entry.remote_feed_title.clone(),
            image_url: entry.remote_feed_image_url.clone(),
            artist: entry.remote_release_artist.clone(),
            artist_source: entry.remote_release_artist_source.clone(),
            role: entry.role.clone(),
            music_names_publisher: entry.music_names_publisher,
            publisher_lists_music: entry.publisher_lists_music,
            publisher_link_resolution: entry.publisher_link_resolution.clone(),
            publisher_rel: entry.publisher_rel.clone(),
            music_rel: entry.music_rel.clone(),
            album_names_as: entry.album_names_as.clone(),
            role_agreement: entry.role_agreement.clone(),
            in_library: false,
            named_rank: entry.album_names_rank,
            track_count: entry.remote_episode_count,
            two_way_validated: entry.two_way_validated,
        })
        .collect();
    let co_credited_feeds = feed
        .co_credited_feeds
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|entry| CoCreditedFeedFact {
            feed_guid: entry.feed_guid.clone(),
            title: entry.title.clone(),
            roles: entry.roles.clone().unwrap_or_default(),
            album_count: entry.album_count,
        })
        .collect();
    PublisherPageFacts {
        publisher_feed_guid: publisher_feed_guid.to_owned(),
        feed_title: feed.title.clone(),
        feed_image_url: feed.image_url.clone(),
        confirmed_release_artist_count: feed.confirmed_release_artist_count,
        confirmed_release_artists: feed.confirmed_release_artists.clone().unwrap_or_default(),
        unconfirmed_release_artist_count: feed.unconfirmed_release_artist_count,
        unconfirmed_release_artists: feed.unconfirmed_release_artists.clone().unwrap_or_default(),
        albums,
        co_credited_feeds,
        other_albums_failure: None,
    }
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

    /// The live DETOX feed of 2026-10-18 lists "Through The Looking Glass"
    /// as `16921a59-…`, a GUID that MusicIndex does not hold. Stophammer
    /// resolved the link by the feed URL and gives the album as
    /// `music_feed_guid` `7247715a-…`. The row opens and marks that album.
    #[test]
    fn adr_0085_feed_page_row_uses_the_resolved_album_guid() {
        let feed: Feed = serde_json::from_value(serde_json::json!({
            "feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
            "publisher": [
                {
                    "direction": "publisher_to_music",
                    "publisher_feed_guid": "137aaa9c-75ff-4916-9f23-e02968b2d15e",
                    "remote_feed_guid": "16921a59-19cc-4270-a0a6-4c829a0c583e",
                    "music_feed_guid": "7247715a-3073-56dd-ba2e-c0a7030eca51",
                    "remote_feed_title": "Through The Looking Glass",
                    "publisher_link_resolution": "feed_url"
                },
                {
                    "direction": "publisher_to_music",
                    "remote_feed_guid": "only-remote-guid"
                }
            ]
        }))
        .expect("the recorded rows decode");

        let facts = publisher_page_facts_from_feed("137aaa9c-75ff-4916-9f23-e02968b2d15e", &feed);

        assert_eq!(
            facts.albums[0].feed_guid.as_deref(),
            Some("7247715a-3073-56dd-ba2e-c0a7030eca51")
        );
        assert_eq!(
            facts.albums[1].feed_guid.as_deref(),
            Some("only-remote-guid")
        );
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
        if bare == "/v1/feeds/publisher-guid" {
            return serde_json::json!({"data": {
                "feed_guid": "publisher-guid",
                "title": "Publisher Feed",
                "image_url": "https://example.test/publisher-guid.jpg",
                "publisher": [{
                    "direction": "publisher_to_music",
                    "remote_feed_guid": "album-guid",
                    "publisher_feed_guid": "publisher-guid",
                    "remote_feed_title": "Album Title",
                    "remote_release_artist": "Album Artist",
                    "remote_release_artist_source": "itunes_author",
                    "music_names_publisher": true,
                    "publisher_lists_music": true,
                    "publisher_link_resolution": "feed_url",
                    "role": "artist",
                    "role_source": "default"
                }]
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
    /// `fetch_recent_feed_result_rows`, and this call site sends L2. ADR 0077
    /// Decision 5 adds `publisher` to that include list.
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
            "R17-05: the Recent Feeds feed detail call site must send L2 and publisher (ADR 0077 Decision 5). Got: {requests:?}"
        );
    }

    /// R2-05: the Index feed detail sends the same number of requests as
    /// before ADR 0077 packet 002, and it asks for `publisher` (ADR 0077
    /// Decision 5).
    #[test]
    fn adr_0077_publisher_relationship_index_feed_detail_request_count_is_unchanged() {
        let recent = Fixture::start();
        fetch_recent_feed_result_rows(&recent.endpoint, None, 0).unwrap();
        let listed = recent.requests.lock().unwrap().clone();
        assert_eq!(
            listed.len(),
            2,
            "one recent-feeds request, one feed detail request"
        );

        assert!(
            listed[1].starts_with("/v1/feeds/f1?include=") && listed[1].contains("%2Cpublisher"),
            "the Index feed detail must ask for publisher. Got: {}",
            listed[1]
        );
    }

    /// R3-01 (ADR 0077 Task 003): the Index publisher page query sends one
    /// request, and it asks for the `INDEX_PUBLISHER_PAGE` include list. It
    /// sends no request for an album.
    #[test]
    fn adr_0077_publisher_page_index_query_sends_one_request_and_no_album_request() {
        let fixture = Fixture::start();
        let client = crate::api::Client::new_with_base_url(fixture.endpoint.clone());
        let provider_identity = fixture.endpoint.require().unwrap();

        let facts = fetch_index_publisher_page_albums(&client, provider_identity, "publisher-guid")
            .unwrap();

        assert_eq!(facts.albums.len(), 1, "the fixture names one album entry");
        assert_eq!(facts.albums[0].feed_guid.as_deref(), Some("album-guid"));
        assert_eq!(facts.albums[0].title.as_deref(), Some("Album Title"));
        assert_eq!(
            facts.feed_image_url.as_deref(),
            Some("https://example.test/publisher-guid.jpg"),
            "R85-8-16: the feed's own image_url carries into feed_image_url"
        );
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests,
            vec![format!(
                "/v1/feeds/publisher-guid?include={}",
                encoded_include(INDEX_PUBLISHER_PAGE)
            )],
            "R3-01: one request, and no request for an album"
        );
    }

    /// R3-02b (ADR 0077 Task 003): the Index publisher page marks each
    /// album that a stored `music_to_publisher` row already names as a
    /// Library album.
    #[test]
    fn adr_0077_publisher_page_index_query_marks_library_albums() {
        let fixture = Fixture::start();
        let client = crate::api::Client::new_with_base_url(fixture.endpoint.clone());
        let provider_identity = fixture.endpoint.require().unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::migrate_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO feeds(id, feed_url, feed_guid, title)
             VALUES (1, 'https://example.test/one.xml', 'album-guid', 'Local title')",
            [],
        )
        .unwrap();
        let stored_entry = crate::api::PublisherRelationship {
            direction: Some("music_to_publisher".to_owned()),
            publisher_feed_guid: Some("publisher-guid".to_owned()),
            remote_feed_guid: Some("publisher-guid".to_owned()),
            role: Some("artist".to_owned()),
            ..crate::api::PublisherRelationship::default()
        };
        db::publisher_relationships::upsert_feed_publisher_relationships(
            &mut conn,
            1,
            Some("Publisher Feed"),
            Some(&[stored_entry]),
            10,
        )
        .unwrap();

        let facts = fetch_index_publisher_page(&conn, &client, provider_identity, "publisher-guid")
            .unwrap();

        assert_eq!(facts.albums.len(), 1);
        assert!(
            facts.albums[0].in_library,
            "the album that a stored music_to_publisher row names must be marked as in the Library"
        );
    }
}

/// ADR 0086 Task 001: the parallel feed detail reads of
/// `fetch_recent_feed_result_rows`.
#[cfg(test)]
mod adr_0086_parallel_reads_tests {
    use super::*;
    use crate::application::queries::PARALLEL_DETAIL_READS;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// A local HTTP server that gives each connection its own thread, so
    /// the parallel detail reads of ADR 0086 Decision 5 can run at the
    /// same time. `answer` builds the body of one request from its path.
    /// An empty body answers with a server error. `running` holds the
    /// request count in flight now and the peak count reached, for
    /// R86-1-02.
    #[expect(
        clippy::type_complexity,
        reason = "a test server gives its address, its request log and its request counts"
    )]
    fn serve_routes(
        answer: fn(&str) -> String,
        expected_requests: usize,
    ) -> (String, Arc<Mutex<Vec<String>>>, Arc<Mutex<(usize, usize)>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let running = Arc::new(Mutex::new((0_usize, 0_usize)));
        let received = Arc::clone(&requests);
        let counter = Arc::clone(&running);
        std::thread::spawn(move || {
            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                let received = Arc::clone(&received);
                let counter = Arc::clone(&counter);
                std::thread::spawn(move || {
                    {
                        let mut count = counter.lock().unwrap();
                        count.0 += 1;
                        count.1 = count.1.max(count.0);
                    }
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
                    std::thread::sleep(Duration::from_millis(60));
                    let body = answer(&path);
                    counter.lock().unwrap().0 -= 1;
                    let status = if body.is_empty() {
                        "500 Internal Server Error"
                    } else {
                        "200 OK"
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                });
            }
        });
        (format!("http://{address}"), requests, running)
    }

    /// A local HTTP server like `serve_routes`, whose delay before each
    /// answer comes from `delay_millis(path)` instead of one fixed delay,
    /// so an earlier feed can answer after a later one (R86-1-03).
    fn serve_routes_with_delay(
        answer: fn(&str) -> String,
        delay_millis: fn(&str) -> u64,
        expected_requests: usize,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                std::thread::spawn(move || {
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
                    std::thread::sleep(Duration::from_millis(delay_millis(&path)));
                    let body = answer(&path);
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                });
            }
        });
        format!("http://{address}")
    }

    /// One `GET /v1/feeds/recent` list response of `count` feeds, each
    /// named `feed-0` to `feed-<count - 1>`, with no detail fields.
    fn list_response(count: usize) -> String {
        let data: Vec<_> = (0..count)
            .map(|index| serde_json::json!({"feed_guid": format!("feed-{index}")}))
            .collect();
        serde_json::json!({"data": data, "pagination": {"has_more": false, "cursor": null}})
            .to_string()
    }

    /// The feed GUID that a detail request path names, read from
    /// `/v1/feeds/<guid>?include=...`.
    fn detail_feed_guid(path: &str) -> &str {
        path.trim_start_matches("/v1/feeds/")
            .split('?')
            .next()
            .unwrap_or_default()
    }

    /// One `GET /v1/feeds/<guid>` detail response that gives the row its
    /// own title, so a test can tell a detail row from a list row.
    fn detail_response(guid: &str) -> String {
        serde_json::json!({"data": {"feed_guid": guid, "title": format!("Detail {guid}")}})
            .to_string()
    }

    /// R86-1-01: with a list of 20 feeds, the query gives 20 rows, each
    /// holding its own detail feed.
    #[test]
    fn adr_0086_parallel_reads_r86_1_01_twenty_feeds_give_twenty_rows() {
        fn answer(path: &str) -> String {
            if path.starts_with("/v1/feeds/recent") {
                return list_response(20);
            }
            detail_response(detail_feed_guid(path))
        }
        let (endpoint, _requests, _running) = serve_routes(answer, 21);

        let batch = fetch_recent_feed_result_rows(&endpoint.into(), None, 0).unwrap();

        assert_eq!(batch.rows.len(), 20, "R86-1-01: 20 feeds give 20 rows");
        for (index, (_, display)) in batch.rows.iter().enumerate() {
            assert_eq!(
                display.label,
                format!("Detail feed-{index}"),
                "R86-1-01: row {index} must hold its own detail feed"
            );
        }
    }

    /// R86-1-02: the peak count of detail requests at one time is at
    /// most `PARALLEL_DETAIL_READS`, and more than 1. The counter pattern
    /// of R26-08 in `name_lists.rs`.
    #[test]
    fn adr_0086_parallel_reads_r86_1_02_peak_is_bounded_and_above_one() {
        fn answer(path: &str) -> String {
            if path.starts_with("/v1/feeds/recent") {
                return list_response(20);
            }
            detail_response(detail_feed_guid(path))
        }
        let (endpoint, _requests, running) = serve_routes(answer, 21);

        let batch = fetch_recent_feed_result_rows(&endpoint.into(), None, 0).unwrap();

        assert_eq!(batch.rows.len(), 20);
        let peak = running.lock().unwrap().1;
        assert!(
            peak <= PARALLEL_DETAIL_READS,
            "R86-1-02: {peak} detail requests ran at the same time, more than the shared limit"
        );
        assert!(
            peak > 1,
            "R86-1-02: the detail requests must run at the same time, peak was {peak}"
        );
    }

    /// R86-1-03: the rows keep the order of the list response, also when
    /// the server answers the detail requests in a different order. Each
    /// feed's own delay falls as its index rises, so a later feed answers
    /// before an earlier one.
    #[test]
    fn adr_0086_parallel_reads_r86_1_03_rows_keep_order_with_shuffled_answers() {
        const FEED_COUNT: usize = 8;
        fn answer(path: &str) -> String {
            if path.starts_with("/v1/feeds/recent") {
                return list_response(FEED_COUNT);
            }
            detail_response(detail_feed_guid(path))
        }
        fn delay_millis(path: &str) -> u64 {
            if path.starts_with("/v1/feeds/recent") {
                return 0;
            }
            let guid = detail_feed_guid(path);
            let index: u64 = guid
                .strip_prefix("feed-")
                .and_then(|digits| digits.parse().ok())
                .unwrap_or(0);
            (FEED_COUNT as u64 - index) * 15
        }
        let endpoint = serve_routes_with_delay(answer, delay_millis, FEED_COUNT + 1);

        let batch = fetch_recent_feed_result_rows(&endpoint.into(), None, 0).unwrap();

        assert_eq!(batch.rows.len(), FEED_COUNT);
        for (index, (_, display)) in batch.rows.iter().enumerate() {
            assert_eq!(
                display.label,
                format!("Detail feed-{index}"),
                "R86-1-03: row {index} must keep the order of the list response"
            );
        }
    }

    /// R86-1-04: a failed detail request gives the row of the list feed,
    /// the same as before this packet.
    #[test]
    fn adr_0086_parallel_reads_r86_1_04_failed_detail_falls_back_to_list_feed() {
        fn answer(path: &str) -> String {
            if path.starts_with("/v1/feeds/recent") {
                return serde_json::json!({
                    "data": [
                        {"feed_guid": "feed-0", "title": "List Feed 0"},
                        {"feed_guid": "broken", "title": "List Feed Broken"},
                        {"feed_guid": "feed-2", "title": "List Feed 2"}
                    ],
                    "pagination": {"has_more": false, "cursor": null}
                })
                .to_string();
            }
            let guid = detail_feed_guid(path);
            if guid == "broken" {
                return String::new();
            }
            detail_response(guid)
        }
        let (endpoint, _requests, _running) = serve_routes(answer, 4);

        let batch = fetch_recent_feed_result_rows(&endpoint.into(), None, 0).unwrap();

        assert_eq!(batch.rows.len(), 3);
        assert_eq!(batch.rows[0].1.label, "Detail feed-0");
        assert_eq!(
            batch.rows[1].1.label, "List Feed Broken",
            "R86-1-04: a failed detail request gives the row of the list feed"
        );
        assert_eq!(batch.rows[2].1.label, "Detail feed-2");
    }

    /// R86-1-05: a list feed with no feed GUID sends no detail request,
    /// and gives the row of the list feed.
    #[test]
    fn adr_0086_parallel_reads_r86_1_05_no_feed_guid_sends_no_detail_request() {
        fn answer(_path: &str) -> String {
            serde_json::json!({
                "data": [
                    {"feed_guid": null, "feed_url": "https://example.test/no-guid.xml",
                     "title": "No GUID Feed"}
                ],
                "pagination": {"has_more": false, "cursor": null}
            })
            .to_string()
        }
        let (endpoint, requests, _running) = serve_routes(answer, 1);

        let batch = fetch_recent_feed_result_rows(&endpoint.into(), None, 0).unwrap();

        assert_eq!(batch.rows.len(), 1);
        assert_eq!(
            batch.rows[0].1.label, "No GUID Feed",
            "R86-1-05: the row of the list feed, with no detail request"
        );
        assert_eq!(
            *requests.lock().unwrap(),
            ["/v1/feeds/recent?limit=20"],
            "R86-1-05: a list feed with no feed GUID sends no detail request"
        );
    }
}
