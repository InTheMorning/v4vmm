//! The Releases list in the scope "Everything" (ADR 0084 Decisions 4, 6, 8
//! and 10, packet 016).
//!
//! This module gives the order key of each Releases entry, the state word
//! of a Library release, the row of each `MusicIndex` release of
//! `GET /v1/feeds/by-name`, and the merge of the two in one name order.
//! The merge is a pure function of the Library entries and the loaded
//! pages, so a download changes a row in place.
//!
//! ADR 0084 task 037 adds the cache of the merge. [`ReleasesListCache`]
//! builds the flat items of the list again only when its
//! [`ReleasesListKey`] changes. [`item_id_runs`] gives the changes of the
//! item ids, so the virtual list keeps its scroll position.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::api::Feed;
use crate::view_models::download_progress::DownloadPhase;
use crate::view_models::index_feed_singles::IndexFeedSingles;
use crate::view_models::index_name_list::{
    name_sort_key, watermark_shows, IndexListStatusRowVm, IndexNameListPage, MusicListScope,
};
use crate::view_models::library::{
    release_row_slug, releases_selection_rows, DestinationListRowTarget, DestinationListRowVm,
    ReleasesListEntryVm, ReleasesListItemVm, ReleasesRowId,
};
use crate::view_models::list_selection::{
    ListSelection, RowSelectionVm, SelectionMode, SelectionRow,
};
use crate::view_models::release_download_state::ReleaseDownloadStateVm;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::release_row_actions::{ReleaseRowMenuVm, ReleaseRowState};
use crate::view_models::singles_group::SinglesGroupKey;
use crate::view_models::workspace::ReleaseKey;

/// The separator between the parts of a Releases row text.
const RELEASE_ROW_SEPARATOR: &str = " \u{b7} ";
/// The label of a `MusicIndex` release with no title.
const UNTITLED_RELEASE: &str = "Untitled release";

/// The second part of a Releases order key. A release comes before a group
/// with the same sort key. A release with a feed GUID comes before a
/// Library feed with no feed GUID, which follows by its feed id (packet
/// 016 "The sort key and the merge").
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ReleaseOrderTie {
    Release(ReleaseKey),
    Group(SinglesGroupKey),
}

/// The order position of one Releases entry (packet 016 decision D6): the
/// Stophammer sort key, then the release key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ReleaseOrderKey {
    sort_key: String,
    tie: ReleaseOrderTie,
}

impl ReleaseOrderKey {
    /// The position of a release row with the label `label`.
    #[must_use]
    pub(crate) fn release(label: &str, key: ReleaseKey) -> Self {
        Self {
            sort_key: name_sort_key(label),
            tie: ReleaseOrderTie::Release(key),
        }
    }

    /// The position of a singles group row with the label `label`, in both
    /// scopes (packet 026 decision D5, operator answer of 2026-10-24, and
    /// packet 036 decision D1).
    #[must_use]
    pub(crate) fn group(label: &str, key: SinglesGroupKey) -> Self {
        Self {
            sort_key: name_sort_key(label),
            tie: ReleaseOrderTie::Group(key),
        }
    }

    /// The position of a `MusicIndex` row. `MusicIndex` gives the sort key,
    /// so this position is the position of the row in its pages. `None`
    /// for a row with no feed GUID.
    fn index(feed: &Feed) -> Option<Self> {
        let feed_guid = feed.feed_guid.as_deref().filter(|guid| !guid.is_empty())?;
        let sort_key = feed
            .sort_name
            .clone()
            .unwrap_or_else(|| name_sort_key(feed.title.as_deref().unwrap_or_default()));
        Some(Self {
            sort_key,
            tie: ReleaseOrderTie::Release(ReleaseKey::FeedGuid(feed_guid.to_owned())),
        })
    }
}

/// One Library entry of the Releases list and its order position.
#[derive(Clone, Debug)]
pub(crate) struct LibraryReleaseEntry {
    pub(crate) order: ReleaseOrderKey,
    pub(crate) entry: ReleasesListEntryVm,
}

/// The row of one `MusicIndex` release (packet 016 "The Releases list in
/// Everything"): the title, then "`release_artist` · Kind", with no state
/// word. The row reads only the fields of the list row, so it needs no
/// detail request (decision D16). `None` for a row with no feed GUID.
///
/// `download_phase` gives the phase of a download job of the release's own
/// key, when one is queued or running (ADR 0084 task 033, review
/// correction of 2026-10-22). It wins over the quiet `NotDownloaded` icon,
/// the same way `phase` wins in `library_release_row`.
#[must_use]
pub(crate) fn index_release_row(
    feed: &Feed,
    download_phase: impl Fn(&ReleaseKey) -> Option<DownloadPhase>,
) -> Option<DestinationListRowVm> {
    let feed_guid = feed.feed_guid.as_deref().filter(|guid| !guid.is_empty())?;
    let key = ReleaseKey::FeedGuid(feed_guid.to_owned());
    let phase = download_phase(&key);
    let title = index_release_title(feed);
    let track_count = feed
        .episode_count
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or(0);
    let release_kind = ReleaseKind::resolve(feed.release_kind.as_deref(), track_count);
    let kind = release_kind.label();
    let secondary_text = match feed
        .release_artist
        .as_deref()
        .map(str::trim)
        .filter(|artist| !artist.is_empty())
    {
        Some(artist) => format!("{artist}{RELEASE_ROW_SEPARATOR}{kind}"),
        None => kind.to_owned(),
    };
    Some(DestinationListRowVm {
        // The element id of a Library row of the same key, so the row
        // keeps its identity when a download adds it to the Library.
        element_id: format!("music-release-row-{}", release_row_slug(&key)),
        a11y_label: format!("Release: {title}"),
        label: title,
        secondary_text,
        thumbnail_href: feed.image_url.clone().filter(|url| !url.is_empty()),
        art: None,
        open: DestinationListRowTarget::Release(key),
        selection: RowSelectionVm::none(),
        release_kind: Some(release_kind),
        menu: None,
        // ADR 0084 task 033, Flag: a `MusicIndex` row that the Library
        // does not hold showed no state word before this packet. "A row
        // shows its state at all times" (ADR 0083 Decision 6), so it now
        // shows the quiet `NotDownloaded` icon, or Queued or Downloading
        // while its own job runs (review correction of 2026-10-22). The
        // app has not read this feed's track list here, so a downloaded
        // row never shows a count.
        download_state: Some(match phase {
            Some(DownloadPhase::Queued) => ReleaseDownloadStateVm::queued(),
            Some(DownloadPhase::InFlight) => ReleaseDownloadStateVm::downloading(),
            None => ReleaseDownloadStateVm::in_library(false),
        }),
    })
}

/// The title of a `MusicIndex` release row.
#[must_use]
pub(crate) fn index_release_title(feed: &Feed) -> String {
    feed.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(UNTITLED_RELEASE)
        .to_owned()
}

/// The title of the loaded `MusicIndex` release `feed_guid`, for the
/// download queue label: from a loaded page, else from the loaded singles
/// of a feed (packet 026). `None` when neither holds it.
#[must_use]
pub(crate) fn loaded_index_release_title(
    page: &IndexNameListPage<Feed>,
    fills: &IndexFeedSingles,
    feed_guid: &str,
) -> Option<String> {
    page.rows()
        .iter()
        .find(|feed| feed.feed_guid.as_deref() == Some(feed_guid))
        .or_else(|| fills.find_release(feed_guid))
        .map(index_release_title)
}

/// The Library entries in the order of [`ReleaseOrderKey`] (decision D6).
#[must_use]
pub(crate) fn library_entries(mut library: Vec<LibraryReleaseEntry>) -> Vec<ReleasesListEntryVm> {
    library.sort_by(|left, right| left.order.cmp(&right.order));
    library.into_iter().map(|entry| entry.entry).collect()
}

/// The Releases entries in the scope "Everything" (packet 016 decisions
/// D4, D5, D7 and D8):
///
/// - While page 1 loads, the list shows no item row.
/// - A `MusicIndex` row whose feed GUID is in `hidden_keys` gives no row.
///   `hidden_keys` holds each Library release, also each single of a
///   singles group, so the Library row or the group wins. It also holds
///   each `MusicIndex` single of a group (packet 026).
/// - The entries come in the order of [`ReleaseOrderKey`].
/// - While `MusicIndex` has more pages, an entry shows only at or before
///   the last loaded `MusicIndex` row.
///
/// `download_phase` gives the phase of a download job of a `MusicIndex`
/// release's own key, when one is queued or running (ADR 0084 task 033,
/// review correction of 2026-10-22). [`index_release_row`] reads it.
#[must_use]
pub(crate) fn everything_entries(
    library: Vec<LibraryReleaseEntry>,
    hidden_keys: &BTreeSet<ReleaseKey>,
    page: &IndexNameListPage<Feed>,
    download_phase: impl Fn(&ReleaseKey) -> Option<DownloadPhase>,
) -> Vec<ReleasesListEntryVm> {
    if page.first_page_in_flight() {
        return Vec::new();
    }
    let mut seen: BTreeSet<ReleaseKey> = BTreeSet::new();
    let index_entries: Vec<LibraryReleaseEntry> = page
        .rows()
        .iter()
        .filter_map(|feed| {
            let order = ReleaseOrderKey::index(feed)?;
            let row = index_release_row(feed, &download_phase)?;
            let DestinationListRowTarget::Release(key) = &row.open else {
                return None;
            };
            if hidden_keys.contains(key) || !seen.insert(key.clone()) {
                return None;
            }
            Some(LibraryReleaseEntry {
                order,
                entry: ReleasesListEntryVm::Release(row),
            })
        })
        .collect();

    let last_loaded = page.rows().iter().rev().find_map(ReleaseOrderKey::index);
    library_entries(
        library
            .into_iter()
            .chain(index_entries)
            .filter(|entry| watermark_shows(&entry.order, last_loaded.as_ref(), page.has_more()))
            .collect(),
    )
}

/// A change of the Releases scope (packet 016 decision D15 and Required
/// Change 16). A new scope clears the selection and ends selection mode,
/// because the list shows other rows. The same scope changes nothing.
pub(crate) fn reset_selection_on_scope_change(
    from: MusicListScope,
    to: MusicListScope,
    selection: &mut ListSelection<ReleaseKey, ReleasesRowId>,
    mode: &mut SelectionMode,
) {
    if from != to {
        *selection = ListSelection::new();
        *mode = SelectionMode::Normal;
    }
}

/// The "Download" action of the Releases selection (packet 016 Required
/// Change 15): the Library releases and the `MusicIndex` releases to queue.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ReleasesDownloadPlan {
    /// The Library feed id of each selected Library release that is not
    /// fully downloaded, newest first.
    pub(crate) library_feed_ids: Vec<i64>,
    /// The feed GUID and the title of each selected `MusicIndex` release.
    /// The title is `None` when no loaded page holds the release.
    pub(crate) index_releases: Vec<(String, Option<String>)>,
}

/// The source of each [`Revision`]. Each value comes one time only, so two
/// inputs never share a revision, also after a new input replaces an old
/// one.
static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// The revision of one input of the Releases list (ADR 0084 task 037,
/// Design step 2). Each change of the input gives it a new, larger value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Revision(u64);

impl Revision {
    fn next() -> Self {
        Self(NEXT_REVISION.fetch_add(1, Ordering::Relaxed))
    }

    /// Records one change of the input.
    pub(crate) fn bump(&mut self) {
        *self = Self::next();
    }
}

impl Default for Revision {
    fn default() -> Self {
        Self::next()
    }
}

/// The inputs of one build of the Releases list (ADR 0084 task 037,
/// decision D2). The key holds revisions, not copies of the inputs. The
/// selection, the selection mode, the keyboard focus, the hover and the
/// images are not in the key, so a change of one of them builds no merge
/// (decision D3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleasesListKey {
    pub(crate) scope: MusicListScope,
    /// The revision of the release name page.
    pub(crate) page: Revision,
    /// The revision of the loaded `MusicIndex` singles.
    pub(crate) singles: Revision,
    /// The revision of the Library tree and of the busy state.
    pub(crate) library: Revision,
    /// The open singles groups.
    pub(crate) expanded: BTreeSet<SinglesGroupKey>,
}

/// One build of the Releases list (ADR 0084 task 037, Design step 3): the
/// flat items, the selection rows and the row state of each row key.
///
/// The item order is the order of [`Self::selection_rows`], with a status
/// row at the start or at the end. Each release row and each single row
/// holds its menu for `Normal` mode.
#[derive(Debug, Default)]
pub(crate) struct ReleasesListRows {
    items: Vec<ReleasesListItemVm>,
    item_ids: Vec<String>,
    selection_rows: Vec<SelectionRow<ReleasesRowId, ReleaseKey>>,
    row_states: BTreeMap<ReleaseKey, ReleaseRowState>,
}

impl ReleasesListRows {
    /// Builds the rows from the `entries` of `releases_list` and the status
    /// row of the list. `row_state` gives the state of each row key, and
    /// `row_menu` gives the `Normal` menu of each release row.
    #[must_use]
    pub(crate) fn new(
        entries: Vec<ReleasesListEntryVm>,
        status: Option<IndexListStatusRowVm>,
        row_state: impl Fn(&ReleaseKey) -> Option<ReleaseRowState>,
        row_menu: impl Fn(&DestinationListRowVm) -> Option<ReleaseRowMenuVm>,
    ) -> Self {
        let selection_rows = releases_selection_rows(&entries);
        let row_states = selection_rows
            .iter()
            .flat_map(|row| row.members.iter())
            .filter_map(|key| row_state(key).map(|state| (key.clone(), state)))
            .collect();
        let with_menu = |mut row: DestinationListRowVm| {
            row.menu = row_menu(&row);
            row
        };
        let (leading, trailing) = match status {
            Some(row) if row.at_list_start() => (Some(row), None),
            other => (None, other),
        };
        let mut items: Vec<ReleasesListItemVm> = leading
            .map(ReleasesListItemVm::LeadingStatus)
            .into_iter()
            .collect();
        for entry in entries {
            match entry {
                ReleasesListEntryVm::Release(row) => {
                    items.push(ReleasesListItemVm::Release(with_menu(row)));
                }
                ReleasesListEntryVm::SinglesGroup(mut group) => {
                    let singles = std::mem::take(&mut group.singles);
                    items.push(ReleasesListItemVm::Group(group));
                    items.extend(
                        singles
                            .into_iter()
                            .map(|row| ReleasesListItemVm::Single(with_menu(row))),
                    );
                }
            }
        }
        items.extend(trailing.map(ReleasesListItemVm::TrailingStatus));
        let item_ids = items
            .iter()
            .map(|item| item.element_id().to_owned())
            .collect();
        Self {
            items,
            item_ids,
            selection_rows,
            row_states,
        }
    }

    /// The flat items, in their render order.
    #[must_use]
    pub(crate) fn items(&self) -> &[ReleasesListItemVm] {
        &self.items
    }

    /// The element id of each item, in the order of [`Self::items`].
    #[must_use]
    pub(crate) fn item_ids(&self) -> &[String] {
        &self.item_ids
    }

    /// The selection rows, in their render order (ADR 0084 Decision 10).
    #[must_use]
    pub(crate) fn selection_rows(&self) -> &[SelectionRow<ReleasesRowId, ReleaseKey>] {
        &self.selection_rows
    }

    /// The row state of `key`, when a row of the list holds it.
    #[must_use]
    pub(crate) fn row_state(&self, key: &ReleaseKey) -> Option<ReleaseRowState> {
        self.row_states.get(key).copied()
    }

    /// 1 when a status row is the first item, else 0. The item index of
    /// a selection row is its index plus this count.
    #[must_use]
    pub(crate) fn leading(&self) -> usize {
        usize::from(matches!(
            self.items.first(),
            Some(ReleasesListItemVm::LeadingStatus(_))
        ))
    }
}

/// The last build of the Releases list and its key (ADR 0084 task 037,
/// Design step 3). One frame can read the list more than one time, for
/// the header action and for the rows. Only a new key builds the list
/// again (overhaul plan, responsiveness items 16 and 31).
#[derive(Debug, Default)]
pub(crate) struct ReleasesListCache {
    key: Option<ReleasesListKey>,
    rows: Rc<ReleasesListRows>,
    /// The count of builds, for the tests of task 037.
    #[cfg(test)]
    builds: usize,
}

impl ReleasesListCache {
    /// The rows of `key`. `build` runs only when `key` is not the key of
    /// the last build.
    pub(crate) fn read(
        &mut self,
        key: ReleasesListKey,
        build: impl FnOnce() -> ReleasesListRows,
    ) -> Rc<ReleasesListRows> {
        if self.key.as_ref() != Some(&key) {
            self.rows = Rc::new(build());
            self.key = Some(key);
            #[cfg(test)]
            {
                self.builds += 1;
            }
        }
        Rc::clone(&self.rows)
    }

    /// The count of builds since the cache was made.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn builds(&self) -> usize {
        self.builds
    }
}

/// One change of a list of item ids (ADR 0084 task 037, Design step 8):
/// the items in `old` are replaced by `new_count` new items.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ItemIdRun {
    pub(crate) old: Range<usize>,
    pub(crate) new_count: usize,
}

/// The runs of added or removed ids from `old` to `new`, in the order of
/// `old`. An id in both lists, in the same relative order, is in no run,
/// so its item keeps its measured size. The kept ids are the longest
/// sequence of ids that both lists hold in the same order. Apply the runs
/// from the last to the first, so each `old` range stays correct.
#[must_use]
pub(crate) fn item_id_runs(old: &[String], new: &[String]) -> Vec<ItemIdRun> {
    let old_positions: HashMap<&str, usize> = old
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    // Each new id that `old` holds, as (new index, old index).
    let pairs: Vec<(usize, usize)> = new
        .iter()
        .enumerate()
        .filter_map(|(index, id)| old_positions.get(id.as_str()).map(|&at| (index, at)))
        .collect();
    let kept = longest_increasing_pairs(&pairs);

    let mut runs = Vec::new();
    let (mut old_start, mut new_start) = (0, 0);
    for (new_at, old_at) in kept
        .into_iter()
        .chain(std::iter::once((new.len(), old.len())))
    {
        if old_at > old_start || new_at > new_start {
            runs.push(ItemIdRun {
                old: old_start..old_at,
                new_count: new_at - new_start,
            });
        }
        old_start = old_at + 1;
        new_start = new_at + 1;
    }
    runs
}

/// The longest sequence of `pairs` whose old indices increase. `pairs`
/// comes in the order of its new indices.
fn longest_increasing_pairs(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    // `tails[length]` is the index in `pairs` of the smallest old index
    // that ends a sequence of `length + 1` pairs.
    let mut tails: Vec<usize> = Vec::new();
    let mut previous: Vec<Option<usize>> = vec![None; pairs.len()];
    for (index, &(_, old_at)) in pairs.iter().enumerate() {
        let length = tails.partition_point(|&tail| pairs[tail].1 < old_at);
        previous[index] = length.checked_sub(1).map(|before| tails[before]);
        if length == tails.len() {
            tails.push(index);
        } else {
            tails[length] = index;
        }
    }
    let mut kept = Vec::with_capacity(tails.len());
    let mut at = tails.last().copied();
    while let Some(index) = at {
        kept.push(pairs[index]);
        at = previous[index];
    }
    kept.reverse();
    kept
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::view_models::index_name_list::IndexNameListBatch;
    use crate::view_models::release_download_state::ReleaseDownloadState;

    /// The recorded rows of `GET /v1/feeds/by-name?include=named_feeds` on
    /// 2026-10-20, with the fields that the row reads.
    pub(crate) fn recorded_rows() -> Vec<Feed> {
        serde_json::from_value(serde_json::json!([
            {"feed_guid": "liii-album", "title": "#LIII4PRESIDENT",
             "sort_name": "#liii4president", "release_artist": "LIIINEKELA",
             "release_kind": "unknown", "episode_count": 1, "publisher_feed_guid": null},
            {"feed_guid": "take-album", "title": "1 Take First Freestyle (No Edit)",
             "sort_name": "1 take first freestyle (no edit)", "release_artist": "jSmovie",
             "release_kind": "unknown", "episode_count": 1, "publisher_feed_guid": null},
            {"feed_guid": "motive-album", "title": "Motive to Kill",
             "sort_name": "motive to kill", "release_artist": "Motive to Kill",
             "release_kind": "unknown", "episode_count": 9,
             "image_url": "https://example.test/motive.jpg",
             "publisher_feed_guid": "motive-guid"}
        ]))
        .expect("the recorded rows decode")
    }

    /// A loaded page of `rows`.
    pub(crate) fn loaded_page(rows: Vec<Feed>, has_more: bool) -> IndexNameListPage<Feed> {
        let mut page = IndexNameListPage::default();
        let intent = page.begin_load(false).expect("page 1 starts");
        assert!(page.finish_load(
            IndexNameListBatch {
                rows,
                cursor: has_more.then(|| "next".to_owned()),
                has_more,
            },
            intent.generation,
        ));
        page
    }

    fn library_row(label: &str, key: ReleaseKey) -> LibraryReleaseEntry {
        LibraryReleaseEntry {
            order: ReleaseOrderKey::release(label, key.clone()),
            entry: ReleasesListEntryVm::Release(DestinationListRowVm {
                element_id: format!("music-release-row-{}", release_row_slug(&key)),
                label: label.to_owned(),
                secondary_text: "Library".to_owned(),
                a11y_label: format!("Release: {label}"),
                thumbnail_href: None,
                art: None,
                open: DestinationListRowTarget::Release(key),
                selection: RowSelectionVm::none(),
                release_kind: None,
                menu: None,
                download_state: None,
            }),
        }
    }

    fn labels(entries: &[ReleasesListEntryVm]) -> Vec<&str> {
        entries.iter().map(ReleasesListEntryVm::label).collect()
    }

    /// R16-13: a Library release and a `MusicIndex` row of the same GUID
    /// give one Library row. A `LibraryFeed` key never merges. The order
    /// follows decision D6, and the watermark follows decision D7.
    #[test]
    fn adr_0084_scopes_r16_13_merge_order_and_watermark() {
        let library = vec![
            library_row(
                "Motive to Kill",
                ReleaseKey::FeedGuid("motive-album".into()),
            ),
            library_row("Motive to Kill", ReleaseKey::LibraryFeed(7)),
            library_row("Zebra", ReleaseKey::FeedGuid("zebra".into())),
            library_row("  #LIII4President ", ReleaseKey::FeedGuid("aaa".into())),
        ];
        let keys: BTreeSet<ReleaseKey> = library
            .iter()
            .filter_map(|entry| match &entry.entry {
                ReleasesListEntryVm::Release(row) => match &row.open {
                    DestinationListRowTarget::Release(key) => Some(key.clone()),
                    _ => None,
                },
                ReleasesListEntryVm::SinglesGroup(_) => None,
            })
            .collect();

        let complete = everything_entries(
            library.clone(),
            &keys,
            &loaded_page(recorded_rows(), false),
            |_| None,
        );
        assert_eq!(
            labels(&complete),
            [
                "  #LIII4President ",
                "#LIII4PRESIDENT",
                "1 Take First Freestyle (No Edit)",
                "Motive to Kill",
                "Motive to Kill",
                "Zebra",
            ]
        );
        let ReleasesListEntryVm::Release(motive) = &complete[3] else {
            panic!("a release row");
        };
        assert_eq!(motive.secondary_text, "Library", "the Library row wins");
        assert_eq!(
            motive.open,
            DestinationListRowTarget::Release(ReleaseKey::FeedGuid("motive-album".into()))
        );
        let ReleasesListEntryVm::Release(no_guid) = &complete[4] else {
            panic!("a release row");
        };
        assert_eq!(
            no_guid.open,
            DestinationListRowTarget::Release(ReleaseKey::LibraryFeed(7)),
            "a LibraryFeed key never merges and comes after equal keys"
        );

        let partial =
            everything_entries(library, &keys, &loaded_page(recorded_rows(), true), |_| {
                None
            });
        assert_eq!(
            labels(&partial),
            [
                "  #LIII4President ",
                "#LIII4PRESIDENT",
                "1 Take First Freestyle (No Edit)",
                "Motive to Kill",
            ],
            "a row after the last loaded MusicIndex row waits for its page"
        );
    }

    /// Decision D8: while page 1 loads, the list shows no item row.
    #[test]
    fn adr_0084_scopes_r16_13_first_page_in_flight_shows_no_row() {
        let mut page = IndexNameListPage::<Feed>::default();
        let _ = page.begin_load(false);
        let library = vec![library_row("Zebra", ReleaseKey::FeedGuid("zebra".into()))];
        assert!(everything_entries(library, &BTreeSet::new(), &page, |_| None).is_empty());
    }

    /// R16-14: a `MusicIndex` row gives "`release_artist` · Kind" with no
    /// state word. The kind comes from `ReleaseKind::resolve`.
    #[test]
    fn adr_0084_scopes_r16_14_index_row_text() {
        let rows = recorded_rows();
        let liii = index_release_row(&rows[0], |_| None).unwrap();
        assert_eq!(liii.label, "#LIII4PRESIDENT");
        assert_eq!(liii.secondary_text, "LIIINEKELA \u{b7} Single");
        assert_eq!(
            liii.open,
            DestinationListRowTarget::Release(ReleaseKey::FeedGuid("liii-album".into()))
        );
        let motive = index_release_row(&rows[2], |_| None).unwrap();
        assert_eq!(motive.secondary_text, "Motive to Kill \u{b7} Album");
        assert_eq!(
            motive.thumbnail_href.as_deref(),
            Some("https://example.test/motive.jpg")
        );
        for row in [liii, motive] {
            for word in ["In Library", "Not downloaded", "tracks"] {
                assert!(!row.secondary_text.contains(word), "{}", row.secondary_text);
            }
        }
        let no_guid: Feed = serde_json::from_value(serde_json::json!({"title": "x"})).unwrap();
        assert!(index_release_row(&no_guid, |_| None).is_none());
    }

    /// R33-10 (ADR 0083 Decision 6, amended 2026-10-22, ADR 0084 task
    /// 033): a `MusicIndex` row of the Releases list in "Everything" that
    /// the Library does not hold shows the quiet `NotDownloaded` icon,
    /// with no count text. This replaces R16-14 and packet 016 decision
    /// D13: the state word no longer depends on the scope, because the
    /// icon is compact enough to show at all times (ADR 0083 Decision 6).
    #[test]
    fn adr_0084_scopes_r33_10_index_row_shows_the_not_downloaded_icon() {
        let feed = recorded_rows()
            .into_iter()
            .next()
            .expect("one recorded row");
        let row = index_release_row(&feed, |_| None).expect("the feed has a GUID");
        let state = row.download_state.expect("every release row has one");
        assert_eq!(state.state, ReleaseDownloadState::NotDownloaded);
        assert_eq!(state.count_text, None);
    }

    /// Review correction of 2026-10-22 (ADR 0084 task 033): a `MusicIndex`
    /// row shows the phase of its own download job: Queued, Downloading,
    /// or the quiet `NotDownloaded` icon with no job active. Before this
    /// correction, `index_release_row` always gave `NotDownloaded`, so a
    /// queued or running download of a `MusicIndex` release never showed
    /// in its row.
    #[test]
    fn adr_0084_task_033_index_release_row_shows_its_own_job_phase() {
        let feed = recorded_rows()
            .into_iter()
            .next()
            .expect("one recorded row");
        let key = ReleaseKey::FeedGuid("liii-album".into());

        let idle = index_release_row(&feed, |_| None)
            .and_then(|row| row.download_state)
            .expect("every release row has one");
        assert_eq!(idle.state, ReleaseDownloadState::NotDownloaded);

        let queued = index_release_row(&feed, |found| {
            (*found == key).then_some(DownloadPhase::Queued)
        })
        .and_then(|row| row.download_state)
        .expect("every release row has one");
        assert_eq!(queued.state, ReleaseDownloadState::Queued);

        let downloading = index_release_row(&feed, |found| {
            (*found == key).then_some(DownloadPhase::InFlight)
        })
        .and_then(|row| row.download_state)
        .expect("every release row has one");
        assert_eq!(downloading.state, ReleaseDownloadState::Downloading);
    }

    /// Review correction of 2026-10-22 (ADR 0084 task 033):
    /// `everything_entries` gives the Releases list row of a `MusicIndex`
    /// release the phase of its own download job.
    #[test]
    fn adr_0084_task_033_everything_entries_threads_the_index_release_phase() {
        let key = ReleaseKey::FeedGuid("motive-album".into());
        let entries = everything_entries(
            Vec::new(),
            &BTreeSet::new(),
            &loaded_page(recorded_rows(), false),
            |found| (*found == key).then_some(DownloadPhase::InFlight),
        );
        let motive_state = entries
            .iter()
            .find_map(|entry| match entry {
                ReleasesListEntryVm::Release(row)
                    if row.open == DestinationListRowTarget::Release(key.clone()) =>
                {
                    row.download_state.as_ref()
                }
                _ => None,
            })
            .expect("the Motive to Kill row");
        assert_eq!(motive_state.state, ReleaseDownloadState::Downloading);
    }

    /// R16-18: a scope change clears the selection and ends selection
    /// mode. The same scope changes nothing.
    #[test]
    fn adr_0084_scopes_r16_18_scope_change_resets_selection() {
        use crate::view_models::list_selection::SelectionRow;
        let key = ReleaseKey::FeedGuid("motive-album".into());
        let row = SelectionRow::new(ReleasesRowId::Release(key.clone()), vec![key]);
        let mut selection = ListSelection::new();
        selection.toggle(&row);
        let mut mode = SelectionMode::Selecting;

        reset_selection_on_scope_change(
            MusicListScope::Library,
            MusicListScope::Library,
            &mut selection,
            &mut mode,
        );
        assert_eq!(selection.selected_count(), 1);
        assert!(mode.is_selecting());

        reset_selection_on_scope_change(
            MusicListScope::Library,
            MusicListScope::Everything,
            &mut selection,
            &mut mode,
        );
        assert_eq!(selection.selected_count(), 0);
        assert_eq!(selection.focused(), None);
        assert!(!mode.is_selecting());
    }

    /// The download label of a loaded `MusicIndex` release.
    #[test]
    fn adr_0084_scopes_loaded_index_release_title() {
        let page = loaded_page(recorded_rows(), false);
        assert_eq!(
            loaded_index_release_title(&page, &IndexFeedSingles::default(), "motive-album")
                .as_deref(),
            Some("Motive to Kill")
        );
        assert_eq!(
            loaded_index_release_title(&page, &IndexFeedSingles::default(), "absent"),
            None
        );
    }

    fn ids(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    /// R37-01 (ADR 0084 task 037, decision D2): two reads with the same key
    /// give one build, as the header action and the rows of one frame do.
    /// A new key builds again.
    #[test]
    fn adr_0084_task_037_r37_01_one_build_for_one_key() {
        let key = ReleasesListKey {
            scope: MusicListScope::Everything,
            page: Revision::default(),
            singles: Revision::default(),
            library: Revision::default(),
            expanded: BTreeSet::new(),
        };
        let mut cache = ReleasesListCache::default();
        let header = cache.read(key.clone(), ReleasesListRows::default);
        let rows = cache.read(key.clone(), || {
            panic!("the second read of the same key must not build")
        });
        assert_eq!(cache.builds(), 1);
        assert!(Rc::ptr_eq(&header, &rows));

        let mut changed = key;
        changed.page.bump();
        let _ = cache.read(changed, ReleasesListRows::default);
        assert_eq!(cache.builds(), 2);
    }

    /// R37-06 (ADR 0084 task 037, Design step 8): the id comparison gives
    /// one run for one row added above, for the rows that "Load more" adds
    /// at the end, for one row removed, and for the singles of an open
    /// group. An item with the same id is in no run.
    #[test]
    fn adr_0084_task_037_r37_06_one_run_for_each_change() {
        let old = ids(&["a", "b", "c", "load-more"]);
        assert!(item_id_runs(&old, &old).is_empty());
        assert_eq!(
            item_id_runs(&old, &ids(&["group", "a", "b", "c", "load-more"])),
            [ItemIdRun {
                old: 0..0,
                new_count: 1
            }],
            "one row added above"
        );
        assert_eq!(
            item_id_runs(&old, &ids(&["a", "b", "c", "d", "e", "load-more"])),
            [ItemIdRun {
                old: 3..3,
                new_count: 2
            }],
            "Load more adds rows at the end"
        );
        assert_eq!(
            item_id_runs(&old, &ids(&["a", "c", "load-more"])),
            [ItemIdRun {
                old: 1..2,
                new_count: 0
            }],
            "one row removed"
        );
        assert_eq!(
            item_id_runs(
                &ids(&["a", "group", "b"]),
                &ids(&["a", "group", "single-1", "single-2", "b"])
            ),
            [ItemIdRun {
                old: 2..2,
                new_count: 2
            }],
            "an open group"
        );
        assert_eq!(
            item_id_runs(&old, &ids(&["a", "b", "c", "error"])),
            [ItemIdRun {
                old: 3..4,
                new_count: 1
            }],
            "a status row of another kind replaces the old one"
        );
        assert_eq!(
            item_id_runs(&ids(&["a", "b", "c"]), &ids(&["c", "a", "b"])),
            [
                ItemIdRun {
                    old: 0..0,
                    new_count: 1
                },
                ItemIdRun {
                    old: 2..3,
                    new_count: 0
                }
            ],
            "a moved row is removed and added, and the others keep their items"
        );
    }
}
