//! The `MusicIndex` name list queries of the scope "Everything" (ADR 0084
//! Decision 8, packet 016).
//!
//! Each command sends one request for one page of a name list. It reads no
//! release detail for a row (ADR 0075 section 6, packet 016 decision D16).
//! `FetchReleaseNamePage` sends `GET /v1/feeds/by-name`, and
//! `FetchArtistNamePage` sends `GET /v1/artist-names`.
//!
//! The release page command also reads the `MusicIndex` singles of each new
//! artist feed of its singles, with one list request for each feed (ADR
//! 0084 Decision 6, packet 026 decision D4). At most
//! [`PARALLEL_DETAIL_READS`] of these requests run at the same time (ADR
//! 0086 Decision 5).
//!
//! `FetchFeedSingles` reads the singles of one feed for the group page and
//! for the single page (packet 026 Part B).

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::api::{ArtistNameRow, Client, Feed};
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::queries::PARALLEL_DETAIL_READS;
use crate::application::request_profiles::{
    INDEX_ARTIST_NAME_LIST, INDEX_FEED_SINGLES, INDEX_FIELD_LIST_ARTIST_NAMES,
    INDEX_RELEASE_NAME_LIST,
};
use crate::view_models::index_feed_singles::{
    FeedReleaseList, FeedSinglesResult, ReleaseNamePageLoad,
};
use crate::view_models::index_name_list::IndexNameListBatch;
use crate::view_models::singles_group::index_single_artist_feed;

/// The result of the request for the singles of one feed: the rows, or
/// the error text.
type FeedSinglesRows = Result<FeedReleaseList, String>;

/// The row count of one name list page (packet 016 decision D7).
pub(crate) const NAME_LIST_PAGE_LIMIT: i32 = 100;
/// The row count of one page of the releases that name one feed (packet
/// 026). The contract permits 1 to 200.
pub(crate) const FEED_SINGLES_PAGE_LIMIT: i32 = 200;
/// The most pages that the app reads for one feed (packet 026 decision
/// D11).
pub(crate) const FEED_SINGLES_MAX_PAGES: usize = 5;
/// The row count of the `artist:` value request of the field list (ADR
/// 0087 packet 005 decision D7). One name can give more than one row.
pub(crate) const FIELD_LIST_ARTIST_NAME_LIMIT: i32 = 20;

/// Fetches one page of `MusicIndex` music releases in name order, and the
/// `MusicIndex` singles of each new artist feed of the singles on that
/// page (packet 026 decision D4).
#[derive(Clone, Debug)]
pub(crate) struct FetchReleaseNamePage {
    endpoint: crate::config::MusicIndexEndpoint,
    cursor: Option<String>,
    /// The rows of a page whose name list request had an answer before.
    /// "Try again" sends them again, and the command sends no name list
    /// request (packet 026 decision D10).
    pending: Option<IndexNameListBatch<Feed>>,
    /// The feeds that page 1 also reads: each Library artist feed with one
    /// or more Library singles.
    library_feeds: Vec<String>,
    /// The feeds with a loaded result, and the feeds whose request already
    /// runs (packet 026 review). The command sends no request for these.
    known_feeds: BTreeSet<String>,
}

impl FetchReleaseNamePage {
    /// Creates the command for the page at `cursor`. `None` asks for page 1.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        cursor: Option<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            cursor,
            pending: None,
            library_feeds: Vec::new(),
            known_feeds: BTreeSet::new(),
        }
    }

    /// Creates the "Try again" command of a page whose rows wait. It sends
    /// no name list request.
    #[must_use]
    pub(crate) fn retry_pending(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        pending: IndexNameListBatch<Feed>,
    ) -> Self {
        Self {
            pending: Some(pending),
            ..Self::new(endpoint, None)
        }
    }

    /// Sets the feeds of the singles requests: `library_feeds` for page 1,
    /// and the `known_feeds` that need no request.
    #[must_use]
    pub(crate) fn with_feeds(
        mut self,
        library_feeds: Vec<String>,
        known_feeds: BTreeSet<String>,
    ) -> Self {
        self.library_feeds = library_feeds;
        self.known_feeds = known_feeds;
        self
    }

    /// The feeds to read for `rows`: the Library feeds, then the artist
    /// feed of each single of `rows`, each one time, with no known feed.
    fn feeds_to_read(&self, rows: &[Feed]) -> Vec<String> {
        let mut seen = self.known_feeds.clone();
        self.library_feeds
            .iter()
            .cloned()
            .chain(rows.iter().filter_map(index_single_artist_feed))
            .filter(|feed_guid| seen.insert(feed_guid.clone()))
            .collect()
    }
}

impl ApplicationCommand for FetchReleaseNamePage {
    type Output = ReleaseNamePageLoad;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.endpoint.clone());
        let batch = if let Some(pending) = self.pending.clone() {
            pending
        } else {
            let response = client
                .fetch_feeds_by_name(
                    NAME_LIST_PAGE_LIMIT,
                    self.cursor.as_deref(),
                    None,
                    &INDEX_RELEASE_NAME_LIST,
                )
                .map_err(|error| CommandError::Query(format!("{error:#}")))?;
            IndexNameListBatch {
                rows: response.data,
                cursor: response.pagination.cursor,
                has_more: response.pagination.has_more,
            }
        };
        let feeds = self.feeds_to_read(&batch.rows);
        let results = fetch_feed_singles_set(&client, &feeds, context)?;
        Ok(CommandOutcome::without_events(ReleaseNamePageLoad {
            batch,
            feeds: results,
        }))
    }
}

/// Reads the releases that name the feed `feed_guid`, newest first: one
/// request for each page of [`FEED_SINGLES_PAGE_LIMIT`] rows, up to
/// [`FEED_SINGLES_MAX_PAGES`] pages (packet 026 decision D11). When the
/// last page that it reads has `has_more` true, the result records that
/// more rows exist. It reads no release detail.
///
/// # Errors
/// Gives the error text of the first failed request.
pub(crate) fn fetch_feed_singles(client: &Client, feed_guid: &str) -> FeedSinglesRows {
    let mut rows = Vec::new();
    let mut cursor: Option<String> = None;
    let mut more = false;
    for _ in 0..FEED_SINGLES_MAX_PAGES {
        let response = client
            .fetch_releases_naming(
                feed_guid,
                FEED_SINGLES_PAGE_LIMIT,
                cursor.as_deref(),
                &INDEX_FEED_SINGLES,
            )
            .map_err(|error| format!("{error:#}"))?;
        rows.extend(response.data);
        more = false;
        match response.pagination.cursor {
            Some(next) if response.pagination.has_more => {
                cursor = Some(next);
                more = true;
            }
            _ => break,
        }
    }
    Ok(FeedReleaseList { rows, more })
}

/// Fetches the `MusicIndex` singles of one feed for the group page or the
/// single page (packet 026 Part B). It sends the requests of
/// [`fetch_feed_singles`] and no other request.
#[derive(Clone, Debug)]
pub(crate) struct FetchFeedSingles {
    endpoint: crate::config::MusicIndexEndpoint,
    feed_guid: String,
}

impl FetchFeedSingles {
    /// Creates the command for the feed `feed_guid`.
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

impl ApplicationCommand for FetchFeedSingles {
    type Output = FeedSinglesResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.endpoint);
        let rows = fetch_feed_singles(&client, &self.feed_guid);
        Ok(CommandOutcome::without_events(FeedSinglesResult {
            feed_guid: self.feed_guid,
            rows,
        }))
    }
}

/// Reads the singles of each feed of `feeds` with
/// [`fetch_feed_singles`]. At most [`PARALLEL_DETAIL_READS`] requests run
/// at the same time (packet 026 decision D4, ADR 0086 Decision 5). The
/// results keep the sequence of `feeds`.
///
/// # Errors
/// Gives [`CommandError::Cancelled`] when the command is cancelled.
fn fetch_feed_singles_set(
    client: &Client,
    feeds: &[String],
    context: &CommandContext,
) -> Result<Vec<FeedSinglesResult>, CommandError> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<FeedSinglesRows>>> = Mutex::new(vec![None; feeds.len()]);
    std::thread::scope(|scope| {
        for _ in 0..PARALLEL_DETAIL_READS.min(feeds.len()) {
            scope.spawn(|| loop {
                if context.cancellation().is_cancelled() {
                    return;
                }
                let index = next.fetch_add(1, Ordering::SeqCst);
                let Some(feed_guid) = feeds.get(index) else {
                    return;
                };
                let rows = fetch_feed_singles(client, feed_guid);
                if let Ok(mut results) = results.lock() {
                    results[index] = Some(rows);
                }
            });
        }
    });
    if context.cancellation().is_cancelled() {
        return Err(CommandError::Cancelled);
    }
    let results = results
        .into_inner()
        .map_err(|_| CommandError::Query("a feed singles request stopped".to_owned()))?;
    Ok(feeds
        .iter()
        .zip(results)
        .map(|(feed_guid, rows)| FeedSinglesResult {
            feed_guid: feed_guid.clone(),
            rows: rows.unwrap_or_else(|| Err("the request did not run".to_owned())),
        })
        .collect())
}

/// Fetches one page of `MusicIndex` artist names in name order.
#[derive(Clone, Debug)]
pub(crate) struct FetchArtistNamePage {
    endpoint: crate::config::MusicIndexEndpoint,
    cursor: Option<String>,
}

impl FetchArtistNamePage {
    /// Creates the command for the page at `cursor`. `None` asks for page 1.
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

impl ApplicationCommand for FetchArtistNamePage {
    type Output = IndexNameListBatch<ArtistNameRow>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let response = Client::new_with_base_url(self.endpoint)
            .fetch_artist_names(
                NAME_LIST_PAGE_LIMIT,
                self.cursor.as_deref(),
                None,
                &INDEX_ARTIST_NAME_LIST,
            )
            .map_err(|error| CommandError::Query(format!("{error:#}")))?;
        Ok(CommandOutcome::without_events(IndexNameListBatch {
            rows: response.data,
            cursor: response.pagination.cursor,
            has_more: response.pagination.has_more,
        }))
    }
}

/// Fetches the `MusicIndex` artist names at and after one sort key, for
/// the `artist:` values of the field list (ADR 0087 Decision 3, operator
/// answer 1 of 2026-10-23). The view model keeps only the names that start
/// with the key.
#[derive(Clone, Debug)]
pub(crate) struct FetchArtistNameValues {
    endpoint: crate::config::MusicIndexEndpoint,
    key: String,
}

impl FetchArtistNameValues {
    /// Creates the command for the sort key `key`.
    #[must_use]
    pub(crate) fn new(endpoint: impl Into<crate::config::MusicIndexEndpoint>, key: String) -> Self {
        Self {
            endpoint: endpoint.into(),
            key,
        }
    }
}

impl ApplicationCommand for FetchArtistNameValues {
    type Output = Vec<ArtistNameRow>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let response = Client::new_with_base_url(self.endpoint)
            .fetch_artist_names(
                FIELD_LIST_ARTIST_NAME_LIMIT,
                None,
                Some(&self.key),
                &INDEX_FIELD_LIST_ARTIST_NAMES,
            )
            .map_err(|error| CommandError::Query(format!("{error:#}")))?;
        Ok(CommandOutcome::without_events(response.data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// A local HTTP server that answers each request with `body` and
    /// records each request path.
    fn serve(body: String, expected_requests: usize) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&requests);
        std::thread::spawn(move || {
            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
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
                if let Some(path) = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                {
                    received.lock().unwrap().push(path.to_owned());
                }
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        (format!("http://{address}"), requests)
    }

    /// R16-05: the artist name command sends one request for a page, with
    /// `limit=100`, the profile include and the cursor.
    #[test]
    fn adr_0084_scopes_r16_05_artist_page_sends_one_request() {
        let body = serde_json::json!({
            "data": [{"kind": "name", "name": "$2 Holla", "sort_name": "$2 holla",
                      "key": "$2 holla", "feed_guid": null, "image_url": null,
                      "release_count": 3}],
            "pagination": {"cursor": "next-cursor", "has_more": true}
        })
        .to_string();
        let (endpoint, requests) = serve(body, 1);

        let batch = FetchArtistNamePage::new(endpoint, Some("abc".to_owned()))
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;

        assert_eq!(batch.rows.len(), 1);
        assert_eq!(batch.cursor.as_deref(), Some("next-cursor"));
        assert!(batch.has_more);
        assert_eq!(
            *requests.lock().unwrap(),
            ["/v1/artist-names?limit=100&cursor=abc&include=link_facts"]
        );
    }

    /// R87-5-16 (ADR 0087 packet 005): the field list command sends one
    /// request with `limit=20`, the include `link_facts` and `from`, and
    /// decodes the rows. A body that does not decode gives an error.
    #[test]
    fn adr_0087_field_values_r87_5_16_value_request_sends_one_request() {
        let body = serde_json::json!({
            "data": [
                {"kind": "name", "name": "Haleen", "sort_name": "haleen",
                 "key": "haleen", "feed_guid": null, "image_url": null,
                 "release_count": 2},
                {"kind": "publisher", "name": "Halo Child", "sort_name": "halo child",
                 "key": "halo-guid", "feed_guid": "halo-guid", "image_url": null,
                 "release_count": 1, "agreed_roles": ["artist"],
                 "first_named_count": 1}
            ],
            "pagination": {"cursor": null, "has_more": false}
        })
        .to_string();
        let (endpoint, requests) = serve(body, 1);

        let rows = FetchArtistNameValues::new(endpoint, "hal".to_owned())
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[1].agreed_roles.as_deref(),
            Some(&["artist".to_owned()][..])
        );
        assert_eq!(
            *requests.lock().unwrap(),
            ["/v1/artist-names?limit=20&include=link_facts&from=hal"]
        );

        let (endpoint, _) = serve("not json".to_owned(), 1);
        let result =
            FetchArtistNameValues::new(endpoint, "hal".to_owned()).execute(&CommandContext::next());
        assert!(matches!(result, Err(CommandError::Query(_))));
    }

    /// R16-05: the release name command sends one request for a page, with
    /// `limit=100`, the profile include and the cursor. It reads no release
    /// detail for a row.
    #[test]
    fn adr_0084_scopes_r16_05_release_page_sends_one_request() {
        let body = serde_json::json!({
            "data": [
                {"feed_guid": "motive-album", "title": "Motive to Kill",
                 "sort_name": "motive to kill", "release_kind": "unknown",
                 "episode_count": 1, "publisher_feed_guid": "motive-guid"},
                {"feed_guid": "liii-album", "title": "#LIII4PRESIDENT",
                 "sort_name": "#liii4president", "release_artist": "LIIINEKELA",
                 "publisher_feed_guid": null}
            ],
            "pagination": {"cursor": "next-cursor", "has_more": true}
        })
        .to_string();
        let (endpoint, requests) = serve(body, 1);

        let batch = FetchReleaseNamePage::new(endpoint, None)
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;

        assert_eq!(batch.batch.rows.len(), 2);
        assert_eq!(batch.batch.cursor.as_deref(), Some("next-cursor"));
        assert!(batch.batch.has_more);
        assert!(batch.feeds.is_empty(), "no single has an artist feed");
        assert_eq!(
            *requests.lock().unwrap(),
            ["/v1/feeds/by-name?limit=100&include=named_feeds"]
        );
    }

    /// A single row that names `feed_guid` at rank 1 with a confirmed link.
    fn single_json(release_guid: &str, feed_guid: &str) -> serde_json::Value {
        serde_json::json!({
            "feed_guid": release_guid, "title": release_guid,
            "sort_name": release_guid, "release_kind": "unknown",
            "episode_count": 1, "newest_item_at": 1_700_000_000,
            "named_feeds": [
                {"publisher_feed_guid": feed_guid, "remote_feed_title": "Haleen",
                 "album_names_rank": 1, "role": null, "role_agreement": null,
                 "two_way_validated": true}
            ]
        })
    }

    /// A local HTTP server that answers each request with `answer(path)`
    /// and records each request path. Each connection has its own thread,
    /// and each answer waits a short time, so that requests can overlap.
    /// `peak` gives the most requests that ran at the same time.
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

    /// R26-06: the singles of one feed send `GET /v1/feeds/recent` with
    /// `named_feed_guid`, `limit=200` and `include=named_feeds`. The read
    /// follows the cursor, up to 5 pages, and sends no detail request.
    #[test]
    fn adr_0084_index_singles_r26_06_feed_singles_follow_the_cursor_up_to_five_pages() {
        fn answer(path: &str) -> String {
            let page = path
                .split("cursor=")
                .nth(1)
                .and_then(|rest| rest.split('&').next())
                .map_or(0, |cursor| cursor.parse::<usize>().unwrap());
            serde_json::json!({
                "data": [single_json(&format!("r{page}"), "feed-g")],
                "pagination": {"cursor": (page + 1).to_string(), "has_more": true}
            })
            .to_string()
        }
        let (endpoint, requests, _) = serve_routes(answer, 5);
        let client = Client::new_with_base_url(endpoint);

        let list = fetch_feed_singles(&client, "feed-g").unwrap();

        assert_eq!(list.rows.len(), FEED_SINGLES_MAX_PAGES);
        assert!(list.more, "the fifth page has more rows (decision D11)");
        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 5, "at most 5 pages");
        assert_eq!(
            requests[0],
            "/v1/feeds/recent?named_feed_guid=feed-g&limit=200&include=named_feeds"
        );
        assert_eq!(
            requests[1],
            "/v1/feeds/recent?named_feed_guid=feed-g&limit=200&cursor=1&include=named_feeds"
        );
        assert!(requests
            .iter()
            .all(|path| path.starts_with("/v1/feeds/recent?")));
    }

    /// R26-21 and R26-23 (packet 026 Part B): the command of the group page
    /// and of the single page sends the requests of one feed only. A list
    /// with no more page gives no "more" fact, and a failed request gives
    /// its error text in the result.
    #[test]
    fn adr_0084_index_singles_page_command_reads_one_feed() {
        fn answer(path: &str) -> String {
            if path.contains("named_feed_guid=broken&") {
                return String::new();
            }
            serde_json::json!({
                "data": [single_json("r1", "feed-g"), single_json("r2", "feed-g")],
                "pagination": {"cursor": null, "has_more": false}
            })
            .to_string()
        }
        let (endpoint, requests, _) = serve_routes(answer, 2);

        let result = FetchFeedSingles::new(endpoint.clone(), "feed-g")
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;
        assert_eq!(result.feed_guid, "feed-g");
        let list = result.rows.expect("the rows");
        assert_eq!(list.rows.len(), 2);
        assert!(!list.more);

        let failed = FetchFeedSingles::new(endpoint, "broken")
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;
        assert!(failed.rows.is_err(), "the error text of the failed request");
        assert_eq!(
            *requests.lock().unwrap(),
            [
                "/v1/feeds/recent?named_feed_guid=feed-g&limit=200&include=named_feeds",
                "/v1/feeds/recent?named_feed_guid=broken&limit=200&include=named_feeds"
            ]
        );
    }

    /// R26-07 and R26-08: a page command sends one request for each new
    /// artist feed of its singles, and for each Library feed on page 1. It
    /// sends no request for a known feed, and never runs more than four
    /// requests at the same time. A row at rank 2, as a label, gives no
    /// request.
    #[test]
    fn adr_0084_index_singles_r26_07_page_reads_each_new_feed_four_at_a_time() {
        fn answer(path: &str) -> String {
            if path.starts_with("/v1/feeds/by-name") {
                let mut rows: Vec<serde_json::Value> = (0..8)
                    .map(|index| single_json(&format!("s{index}"), &format!("feed-{index}")))
                    .collect();
                rows.push(single_json("s-again", "feed-0"));
                rows.push(single_json("s-known", "known-feed"));
                rows.push(serde_json::json!({
                    "feed_guid": "label-single", "title": "Label single",
                    "release_kind": "unknown", "episode_count": 1,
                    "named_feeds": [
                        {"publisher_feed_guid": "34bf795f-f551-4db1-8877-ad19aa5f89c8",
                         "remote_feed_title": "Sir Libre Records", "album_names_rank": 2,
                         "role": "label", "role_agreement": "both",
                         "two_way_validated": true}
                    ]
                }));
                return serde_json::json!({
                    "data": rows,
                    "pagination": {"cursor": "next", "has_more": true}
                })
                .to_string();
            }
            if path.contains("named_feed_guid=feed-3&") {
                return String::new();
            }
            serde_json::json!({"data": [], "pagination": {"cursor": null, "has_more": false}})
                .to_string()
        }
        // The page, eight new feeds and one Library feed.
        let (endpoint, requests, running) = serve_routes(answer, 10);

        let load = FetchReleaseNamePage::new(endpoint, None)
            .with_feeds(
                vec!["library-feed".to_owned(), "known-feed".to_owned()],
                BTreeSet::from(["known-feed".to_owned()]),
            )
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;

        assert_eq!(load.batch.rows.len(), 11);
        let read: Vec<&str> = load
            .feeds
            .iter()
            .map(|result| result.feed_guid.as_str())
            .collect();
        assert_eq!(
            read,
            [
                "library-feed",
                "feed-0",
                "feed-1",
                "feed-2",
                "feed-3",
                "feed-4",
                "feed-5",
                "feed-6",
                "feed-7"
            ]
        );
        assert!(load.feeds[4].rows.is_err(), "the failed request of feed-3");
        assert!(load.feeds[1].rows.is_ok());
        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 10);
        assert_eq!(
            requests
                .iter()
                .filter(|path| path.starts_with("/v1/feeds/recent?named_feed_guid="))
                .count(),
            9,
            "one request for each new feed"
        );
        assert!(!requests.iter().any(|path| path.contains("known-feed")));
        assert!(!requests.iter().any(|path| path.contains("34bf795f")));
        let peak = running.lock().unwrap().1;
        assert!(peak <= PARALLEL_DETAIL_READS, "{peak} requests at one time");
        assert!(peak > 1, "the requests run at the same time");
    }

    /// R26-09: "Try again" of a page whose rows wait sends no name list
    /// request. It sends only the requests of the feeds with no result.
    #[test]
    fn adr_0084_index_singles_r26_09_retry_sends_only_the_failed_requests() {
        fn answer(_path: &str) -> String {
            serde_json::json!({"data": [], "pagination": {"cursor": null, "has_more": false}})
                .to_string()
        }
        let (endpoint, requests, _) = serve_routes(answer, 1);
        let pending: Vec<Feed> = serde_json::from_value(serde_json::json!([
            single_json("a1", "feed-a"),
            single_json("b1", "feed-b")
        ]))
        .unwrap();

        let load = FetchReleaseNamePage::retry_pending(
            endpoint,
            IndexNameListBatch {
                rows: pending,
                cursor: None,
                has_more: false,
            },
        )
        .with_feeds(Vec::new(), BTreeSet::from(["feed-a".to_owned()]))
        .execute(&CommandContext::next())
        .unwrap()
        .into_parts()
        .0;

        assert_eq!(load.batch.rows.len(), 2);
        assert_eq!(
            *requests.lock().unwrap(),
            ["/v1/feeds/recent?named_feed_guid=feed-b&limit=200&include=named_feeds"]
        );
    }
}
