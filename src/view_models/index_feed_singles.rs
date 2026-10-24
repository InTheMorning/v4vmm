//! The `MusicIndex` singles of each artist feed (ADR 0084 Decision 6,
//! packet 026).
//!
//! The scope "Everything" of the Releases list reads the releases that name
//! one artist feed with `GET /v1/feeds/recent?named_feed_guid=<guid>`. This
//! module keeps the singles of each feed that the app read, for the
//! session, and gives the completion of one page load of the Releases list.
//! The rows of a page show only when each request of that page has an
//! answer (packet 026 decision D10). The group page and the single page
//! read one feed with their own request, and this module keeps the state
//! of that request: running or failed (packet 026 Part B). It has no
//! renderer type.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::api::Feed;
use crate::view_models::index_name_list::{IndexNameListBatch, IndexNameListPage};
use crate::view_models::releases_everything::Revision;
use crate::view_models::singles_group::index_singles_of_feed;

/// The releases that name one feed, from each page that the app read.
#[derive(Clone, Debug, Default)]
pub(crate) struct FeedReleaseList {
    /// The rows of each loaded page, newest first.
    pub(crate) rows: Vec<Feed>,
    /// The last page that the app read has `has_more` true. The app reads
    /// at most 5 pages, so more rows exist (packet 026 decision D11).
    pub(crate) more: bool,
}

impl FeedReleaseList {
    /// The rows of a list with no more rows.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn complete(rows: Vec<Feed>) -> Self {
        Self { rows, more: false }
    }
}

/// The result of the request for the releases that name one feed.
#[derive(Clone, Debug)]
pub(crate) struct FeedSinglesResult {
    /// The GUID of the feed.
    pub(crate) feed_guid: String,
    /// The releases that name the feed, or the error text.
    pub(crate) rows: Result<FeedReleaseList, String>,
}

/// The result of one page load of the Releases list in "Everything": the
/// name list page and the result of each feed request of that page.
#[derive(Clone, Debug)]
pub(crate) struct ReleaseNamePageLoad {
    /// The rows of the name list page.
    pub(crate) batch: IndexNameListBatch<Feed>,
    /// The result of each feed request that the page sent.
    pub(crate) feeds: Vec<FeedSinglesResult>,
}

/// The request that the error row names for a failed feed request.
#[must_use]
pub(crate) fn feed_singles_request(feed_guid: &str) -> String {
    format!("GET /v1/feeds/recent?named_feed_guid={feed_guid}")
}

/// The loaded singles of one feed.
#[derive(Clone, Debug)]
struct LoadedFeedSingles {
    /// The request generation that gave the singles.
    generation: u64,
    /// The singles of the feed, newest first.
    singles: Vec<Feed>,
    /// More releases that name the feed exist (packet 026 decision D11).
    more: bool,
}

/// The request of the group page or of the single page for one feed
/// (packet 026 Part B).
#[derive(Clone, Debug)]
enum FeedRequest {
    /// The request with this generation runs.
    Running(u64),
    /// The request failed with this error text.
    Failed(String),
}

/// The state of the `MusicIndex` singles of one feed, ready for the group
/// page and the single page (packet 026 Part B).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeedSinglesState<'a> {
    /// The app has no result and no request runs.
    NotRead,
    /// The request of the page runs.
    Running,
    /// The last request of the page failed. The value is the error text.
    Failed(&'a str),
    /// The app has the result. `more` tells that more releases that name
    /// the feed exist (decision D11).
    Loaded { more: bool },
}

/// The `MusicIndex` singles of each artist feed that the app read (ADR 0084
/// packet 026). The loaded results stay until the app stops, as the name
/// list pages do (packet 016 decision D9).
#[derive(Clone, Debug, Default)]
pub(crate) struct IndexFeedSingles {
    feeds: BTreeMap<String, LoadedFeedSingles>,
    /// The request of the group page or of the single page for each feed
    /// with no result.
    requests: BTreeMap<String, FeedRequest>,
    generation: u64,
    /// Each change gives a new revision (ADR 0084 task 037).
    revision: Revision,
}

impl IndexFeedSingles {
    /// The revision of the loaded singles (ADR 0084 task 037, Design step
    /// 2). Each method that changes them gives a new revision.
    #[must_use]
    pub(crate) const fn revision(&self) -> Revision {
        self.revision
    }

    /// Starts a new request and gives its generation.
    pub(crate) fn begin_request(&mut self) -> u64 {
        self.generation += 1;
        self.revision.bump();
        self.generation
    }

    /// The GUID of each feed with a loaded result, and each feed whose
    /// request runs (packet 026 review). A page load sends no request
    /// for these feeds either. It thus reads no feed again while the
    /// group page or the single page already reads it.
    #[must_use]
    pub(crate) fn loaded_or_running_feeds(&self) -> BTreeSet<String> {
        self.feeds
            .keys()
            .cloned()
            .chain(
                self.requests
                    .iter()
                    .filter(|(_, request)| matches!(request, FeedRequest::Running(_)))
                    .map(|(feed_guid, _)| feed_guid.clone()),
            )
            .collect()
    }

    /// Keeps the singles of `feed_guid` from `list`, each row of the list
    /// that names the feed. Only the rows that are singles of the feed
    /// stay ([`index_singles_of_feed`]). Gives `false` and changes nothing
    /// when a result of a newer generation is loaded. A recorded result
    /// ends the request state of the feed: a later answer of that request
    /// changes nothing.
    pub(crate) fn record(
        &mut self,
        feed_guid: &str,
        list: FeedReleaseList,
        generation: u64,
    ) -> bool {
        if self
            .feeds
            .get(feed_guid)
            .is_some_and(|loaded| loaded.generation > generation)
        {
            return false;
        }
        let singles = index_singles_of_feed(feed_guid, list.rows);
        self.feeds.insert(
            feed_guid.to_owned(),
            LoadedFeedSingles {
                generation,
                singles,
                more: list.more,
            },
        );
        self.requests.remove(feed_guid);
        self.revision.bump();
        true
    }

    /// The state of the singles of `feed_guid` (packet 026 Part B).
    #[must_use]
    pub(crate) fn state(&self, feed_guid: &str) -> FeedSinglesState<'_> {
        if let Some(loaded) = self.feeds.get(feed_guid) {
            return FeedSinglesState::Loaded { more: loaded.more };
        }
        match self.requests.get(feed_guid) {
            None => FeedSinglesState::NotRead,
            Some(FeedRequest::Running(_)) => FeedSinglesState::Running,
            Some(FeedRequest::Failed(detail)) => FeedSinglesState::Failed(detail),
        }
    }

    /// Tells if a page must send the request for `feed_guid`: the app has
    /// no result, and no request runs (packet 026 decision D14).
    #[must_use]
    pub(crate) fn needs_request(&self, feed_guid: &str) -> bool {
        matches!(
            self.state(feed_guid),
            FeedSinglesState::NotRead | FeedSinglesState::Failed(_)
        )
    }

    /// Starts the request of a page for `feed_guid`, and gives its
    /// generation. `None` when the app has a result or a request runs, so
    /// a page sends one request for each feed.
    pub(crate) fn begin_feed_request(&mut self, feed_guid: &str) -> Option<u64> {
        if !self.needs_request(feed_guid) {
            return None;
        }
        let generation = self.begin_request();
        self.requests
            .insert(feed_guid.to_owned(), FeedRequest::Running(generation));
        self.revision.bump();
        Some(generation)
    }

    /// Completes the request of a page. Gives `false` and changes nothing
    /// when `generation` is not the running request of the feed, for
    /// example after a page load recorded the feed (packet 026 R26-24).
    pub(crate) fn finish_feed_request(
        &mut self,
        result: FeedSinglesResult,
        generation: u64,
    ) -> bool {
        if !matches!(
            self.requests.get(&result.feed_guid),
            Some(FeedRequest::Running(running)) if *running == generation
        ) {
            return false;
        }
        match result.rows {
            Ok(list) => self.record(&result.feed_guid, list, generation),
            Err(detail) => {
                self.requests
                    .insert(result.feed_guid, FeedRequest::Failed(detail));
                self.revision.bump();
                true
            }
        }
    }

    /// Each feed with a loaded result, and its singles, newest first.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &[Feed])> {
        self.feeds
            .iter()
            .map(|(guid, loaded)| (guid.as_str(), loaded.singles.as_slice()))
    }

    /// The loaded single whose release GUID is `release_guid`, when one
    /// feed result holds it.
    #[must_use]
    pub(crate) fn find_release(&self, release_guid: &str) -> Option<&Feed> {
        self.feeds
            .values()
            .flat_map(|loaded| loaded.singles.iter())
            .find(|feed| feed.feed_guid.as_deref() == Some(release_guid))
    }
}

/// Completes one page load of the Releases list (packet 026 decision D10).
///
/// Each feed result with rows goes into `singles`, also when another
/// request of the page failed. Thus "Try again" does not send that request
/// again. When each request has an answer, the page adds its rows. Else
/// the page keeps its rows apart and shows the error row, which names the
/// first failed request. Gives `false` and changes no page state when
/// `page_generation` is old.
pub(crate) fn finish_release_page(
    page: &mut IndexNameListPage<Feed>,
    singles: &mut IndexFeedSingles,
    load: ReleaseNamePageLoad,
    page_generation: u64,
    singles_generation: u64,
) -> bool {
    // Packet 026 review: an old result changes nothing, also in `singles`.
    if !page.is_current(page_generation) {
        return false;
    }
    let mut failed: Vec<(String, String)> = Vec::new();
    for result in load.feeds {
        match result.rows {
            Ok(rows) => {
                singles.record(&result.feed_guid, rows, singles_generation);
            }
            Err(detail) => failed.push((result.feed_guid, detail)),
        }
    }
    let Some((first_guid, first_detail)) = failed.first() else {
        return page.finish_load(load.batch, page_generation);
    };
    let detail = match failed.len() - 1 {
        0 => first_detail.clone(),
        1 => format!("{first_detail}. The request for 1 more feed also failed"),
        more => format!("{first_detail}. The requests for {more} more feeds also failed"),
    };
    page.fail_pending(
        load.batch,
        feed_singles_request(first_guid),
        detail,
        page_generation,
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::view_models::index_name_list::{IndexListStatusRowVm, IndexNameList};

    /// A `MusicIndex` single row that names `feed_guid` at rank 1 with a
    /// confirmed link, as the recorded Haleen rows of 2026-10-21 do.
    pub(crate) fn single_naming(
        release_guid: &str,
        title: &str,
        feed_guid: &str,
        newest_item_at: i64,
    ) -> Feed {
        serde_json::from_value(serde_json::json!({
            "feed_guid": release_guid, "title": title,
            "sort_name": crate::view_models::index_name_list::name_sort_key(title),
            "release_artist": "Haleen", "release_kind": "unknown", "episode_count": 1,
            "newest_item_at": newest_item_at,
            "image_url": format!("https://img.test/{release_guid}.jpg"),
            "named_feeds": [
                {"publisher_feed_guid": feed_guid, "remote_feed_title": "Haleen",
                 "album_names_rank": 1, "role": null, "role_agreement": null,
                 "two_way_validated": true}
            ]
        }))
        .expect("the recorded row decodes")
    }

    fn batch(rows: Vec<Feed>) -> IndexNameListBatch<Feed> {
        IndexNameListBatch {
            rows,
            cursor: Some("next".to_owned()),
            has_more: true,
        }
    }

    /// The store keeps only the singles of the feed, and a result of an
    /// older generation changes nothing.
    #[test]
    fn adr_0084_index_singles_store_keeps_singles_and_generation() {
        let mut store = IndexFeedSingles::default();
        let old = store.begin_request();
        let new = store.begin_request();
        let mut album = single_naming("album", "An album", "g", 5);
        album.episode_count = Some(9);
        assert!(store.record(
            "g",
            FeedReleaseList::complete(vec![
                single_naming("a", "A", "g", 2),
                album,
                single_naming("b", "B", "other", 1)
            ]),
            new,
        ));
        let (guid, singles) = store.iter().next().expect("one feed");
        assert_eq!(guid, "g");
        assert_eq!(singles.len(), 1, "only the singles of the feed stay");
        assert!(
            !store.record("g", FeedReleaseList::default(), old),
            "an older result"
        );
        assert_eq!(store.iter().next().unwrap().1.len(), 1);
        assert_eq!(
            store.loaded_or_running_feeds(),
            BTreeSet::from(["g".to_owned()])
        );
        assert!(store.find_release("a").is_some());
        assert!(store.find_release("b").is_none());
    }

    /// Packet 026 review: `loaded_or_running_feeds` gives each feed with a
    /// loaded result, and each feed whose request still runs. A running
    /// request is one that the group page or the single page started. A
    /// feed whose request failed is neither loaded nor running, so a
    /// page load reads it again.
    #[test]
    fn adr_0084_index_singles_loaded_or_running_feeds_includes_running_requests() {
        let mut store = IndexFeedSingles::default();
        let loaded_generation = store.begin_request();
        store.record("loaded", FeedReleaseList::default(), loaded_generation);
        store
            .begin_feed_request("running")
            .expect("a feed with no result starts a request");
        let failed_generation = store
            .begin_feed_request("failed")
            .expect("a feed with no result starts a request");
        store.finish_feed_request(
            FeedSinglesResult {
                feed_guid: "failed".to_owned(),
                rows: Err("timeout".to_owned()),
            },
            failed_generation,
        );

        assert_eq!(
            store.loaded_or_running_feeds(),
            BTreeSet::from(["loaded".to_owned(), "running".to_owned()]),
            "the failed feed is neither loaded nor running"
        );
    }

    /// R26-09: the page gives no rows while one of its requests runs. A
    /// failure of one feed request gives the error row that names it, and
    /// keeps the results that had an answer. "Try again" sends the same
    /// page rows, and the loaded feeds are known, so the retry plans only
    /// the failed request.
    #[test]
    fn adr_0084_index_singles_r26_09_a_failed_feed_request_fails_the_page() {
        let mut page = IndexNameListPage::<Feed>::default();
        let mut store = IndexFeedSingles::default();
        let intent = page.begin_load(false).expect("page 1 starts");
        assert!(page.first_page_in_flight(), "no row while the page loads");

        let rows = vec![
            single_naming("a1", "A1", "feed-a", 2),
            single_naming("b1", "B1", "feed-b", 1),
        ];
        let generation = store.begin_request();
        let load = ReleaseNamePageLoad {
            batch: batch(rows.clone()),
            feeds: vec![
                FeedSinglesResult {
                    feed_guid: "feed-a".to_owned(),
                    rows: Ok(FeedReleaseList::complete(vec![single_naming(
                        "a1", "A1", "feed-a", 2,
                    )])),
                },
                FeedSinglesResult {
                    feed_guid: "feed-b".to_owned(),
                    rows: Err("connection refused".to_owned()),
                },
            ],
        };
        assert!(finish_release_page(
            &mut page,
            &mut store,
            load,
            intent.generation,
            generation
        ));
        assert!(page.rows().is_empty(), "the page rows wait");
        assert_eq!(
            store.loaded_or_running_feeds(),
            BTreeSet::from(["feed-a".to_owned()])
        );
        let Some(IndexListStatusRowVm::Failed { detail, .. }) =
            page.status_row(IndexNameList::Releases)
        else {
            panic!("the error row");
        };
        assert_eq!(
            detail,
            "GET /v1/feeds/recent?named_feed_guid=feed-b: connection refused"
        );

        let (pending, retry_generation) = page
            .begin_pending_retry()
            .expect("the retry uses the page rows");
        assert_eq!(pending.rows.len(), 2);
        assert!(page.first_page_in_flight(), "no row while the retry runs");
        assert!(page.begin_pending_retry().is_none());

        let generation = store.begin_request();
        let load = ReleaseNamePageLoad {
            batch: pending,
            feeds: vec![FeedSinglesResult {
                feed_guid: "feed-b".to_owned(),
                rows: Ok(FeedReleaseList::complete(vec![single_naming(
                    "b1", "B1", "feed-b", 1,
                )])),
            }],
        };
        assert!(finish_release_page(
            &mut page,
            &mut store,
            load,
            retry_generation,
            generation
        ));
        assert_eq!(page.rows().len(), 2, "the rows show after each answer");
        assert_eq!(store.loaded_or_running_feeds().len(), 2);
    }

    /// Packet 026 review: the result of an old page generation changes
    /// nothing, also in the store of feed singles.
    #[test]
    fn adr_0084_index_singles_old_page_result_records_no_singles() {
        let mut page = IndexNameListPage::<Feed>::default();
        let mut store = IndexFeedSingles::default();
        let old = page.begin_load(false).expect("page 1 starts");
        let _new = page
            .begin_load(false)
            .expect("a reload starts page 1 again");
        let generation = store.begin_request();
        let load = ReleaseNamePageLoad {
            batch: batch(vec![single_naming("a1", "A1", "feed-a", 2)]),
            feeds: vec![FeedSinglesResult {
                feed_guid: "feed-a".to_owned(),
                rows: Ok(FeedReleaseList::complete(vec![single_naming(
                    "a1", "A1", "feed-a", 2,
                )])),
            }],
        };
        assert!(!finish_release_page(
            &mut page,
            &mut store,
            load,
            old.generation,
            generation
        ));
        assert!(
            store.loaded_or_running_feeds().is_empty(),
            "the old result records nothing"
        );
    }

    /// Two failed requests name the first one and count the other.
    #[test]
    fn adr_0084_index_singles_r26_09_two_failures_name_the_first() {
        let mut page = IndexNameListPage::<Feed>::default();
        let mut store = IndexFeedSingles::default();
        let intent = page.begin_load(false).expect("page 1 starts");
        let failure = |guid: &str| FeedSinglesResult {
            feed_guid: guid.to_owned(),
            rows: Err("timeout".to_owned()),
        };
        let load = ReleaseNamePageLoad {
            batch: batch(Vec::new()),
            feeds: vec![failure("x"), failure("y")],
        };
        let generation = store.begin_request();
        assert!(finish_release_page(
            &mut page,
            &mut store,
            load,
            intent.generation,
            generation
        ));
        let Some(IndexListStatusRowVm::Failed { detail, .. }) =
            page.status_row(IndexNameList::Releases)
        else {
            panic!("the error row");
        };
        assert_eq!(
            detail,
            "GET /v1/feeds/recent?named_feed_guid=x: timeout. The request for 1 more feed also failed"
        );
    }
}
