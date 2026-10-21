//! The `MusicIndex` singles of each artist feed (ADR 0084 Decision 6,
//! packet 026).
//!
//! The scope "Everything" of the Releases list reads the releases that name
//! one artist feed with `GET /v1/feeds/recent?named_feed_guid=<guid>`. This
//! module keeps the singles of each feed that the app read, for the
//! session, and gives the completion of one page load of the Releases list.
//! The rows of a page show only when each request of that page has an
//! answer (packet 026 decision D10). It has no renderer type.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::api::Feed;
use crate::view_models::index_name_list::{IndexNameListBatch, IndexNameListPage};
use crate::view_models::singles_group::index_singles_of_feed;

/// The result of the request for the releases that name one feed.
#[derive(Clone, Debug)]
pub(crate) struct FeedSinglesResult {
    /// The GUID of the feed.
    pub(crate) feed_guid: String,
    /// The rows of each loaded page, newest first, or the error text.
    pub(crate) rows: Result<Vec<Feed>, String>,
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
}

/// The `MusicIndex` singles of each artist feed that the app read (ADR 0084
/// packet 026). The loaded results stay until the app stops, as the name
/// list pages do (packet 016 decision D9).
#[derive(Clone, Debug, Default)]
pub(crate) struct IndexFeedSingles {
    feeds: BTreeMap<String, LoadedFeedSingles>,
    generation: u64,
}

impl IndexFeedSingles {
    /// Starts a new request and gives its generation.
    pub(crate) fn begin_request(&mut self) -> u64 {
        self.generation += 1;
        self.generation
    }

    /// The GUID of each feed with a loaded result. A page load sends no
    /// request for these feeds.
    #[must_use]
    pub(crate) fn loaded_feeds(&self) -> BTreeSet<String> {
        self.feeds.keys().cloned().collect()
    }

    /// Keeps the singles of `feed_guid` from `rows`, each row of the list
    /// that names the feed. Only the rows that are singles of the feed
    /// stay ([`index_singles_of_feed`]). Gives `false` and changes nothing
    /// when a result of a newer generation is loaded.
    pub(crate) fn record(&mut self, feed_guid: &str, rows: Vec<Feed>, generation: u64) -> bool {
        if self
            .feeds
            .get(feed_guid)
            .is_some_and(|loaded| loaded.generation > generation)
        {
            return false;
        }
        let singles = index_singles_of_feed(feed_guid, rows);
        self.feeds.insert(
            feed_guid.to_owned(),
            LoadedFeedSingles {
                generation,
                singles,
            },
        );
        true
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
            vec![
                single_naming("a", "A", "g", 2),
                album,
                single_naming("b", "B", "other", 1)
            ],
            new,
        ));
        let (guid, singles) = store.iter().next().expect("one feed");
        assert_eq!(guid, "g");
        assert_eq!(singles.len(), 1, "only the singles of the feed stay");
        assert!(!store.record("g", Vec::new(), old), "an older result");
        assert_eq!(store.iter().next().unwrap().1.len(), 1);
        assert_eq!(store.loaded_feeds(), BTreeSet::from(["g".to_owned()]));
        assert!(store.find_release("a").is_some());
        assert!(store.find_release("b").is_none());
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
                    rows: Ok(vec![single_naming("a1", "A1", "feed-a", 2)]),
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
        assert_eq!(store.loaded_feeds(), BTreeSet::from(["feed-a".to_owned()]));
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
                rows: Ok(vec![single_naming("b1", "B1", "feed-b", 1)]),
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
        assert_eq!(store.loaded_feeds().len(), 2);
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
                rows: Ok(vec![single_naming("a1", "A1", "feed-a", 2)]),
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
            store.loaded_feeds().is_empty(),
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
