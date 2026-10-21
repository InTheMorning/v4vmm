//! The Releases list in the scope "Everything" (ADR 0084 Decisions 4, 6, 8
//! and 10, packet 016).
//!
//! This module gives the order key of each Releases entry, the state word
//! of a Library release, the row of each `MusicIndex` release of
//! `GET /v1/feeds/by-name`, and the merge of the two in one name order.
//! The merge is a pure function of the Library entries and the loaded
//! pages, so a download changes a row in place.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

use crate::api::Feed;
use crate::view_models::index_feed_singles::IndexFeedSingles;
use crate::view_models::index_name_list::{
    name_sort_key, watermark_shows, IndexNameListPage, MusicListScope,
};
use crate::view_models::library::{
    release_row_slug, DestinationListRowTarget, DestinationListRowVm, ReleasesListEntryVm,
    ReleasesRowId,
};
use crate::view_models::list_selection::{ListSelection, RowSelectionVm, SelectionMode};
use crate::view_models::release_kind::ReleaseKind;
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

    /// The position of a singles group row with the label `label`, in the
    /// scope "Library".
    #[must_use]
    pub(crate) fn group(label: &str, key: SinglesGroupKey) -> Self {
        Self::group_at(name_sort_key(label), key)
    }

    /// The position of a singles group row at the sort key `sort_key`. In
    /// the scope "Everything", this is the sort key of the first single of
    /// the group in title order (packet 026 decision D5).
    #[must_use]
    pub(crate) fn group_at(sort_key: String, key: SinglesGroupKey) -> Self {
        Self {
            sort_key,
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

/// The state word of a Library release (ADR 0084 Decision 4, packet 016
/// decision D13).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseStateWord {
    /// Each track is downloaded.
    InLibrary,
    /// Some tracks are downloaded, for example "3 of 7 tracks".
    Partial { downloaded: usize, total: usize },
    /// No track is downloaded, or the release has no track.
    NotDownloaded,
}

impl ReleaseStateWord {
    /// The state of a release with `downloaded` of `total` tracks in the
    /// Library.
    #[must_use]
    pub(crate) const fn of_tracks(downloaded: usize, total: usize) -> Self {
        if downloaded == 0 || total == 0 {
            Self::NotDownloaded
        } else if downloaded >= total {
            Self::InLibrary
        } else {
            Self::Partial { downloaded, total }
        }
    }

    /// The visible word.
    #[must_use]
    pub(crate) fn label(self) -> String {
        match self {
            Self::InLibrary => "In Library".to_owned(),
            Self::Partial { downloaded, total } => format!("{downloaded} of {total} tracks"),
            Self::NotDownloaded => "Not downloaded".to_owned(),
        }
    }

    /// Whether the row shows the word in `scope` (decision D13 and the
    /// operator answer of 2026-10-20). "Everything" shows each word. In
    /// "Library", each row is a Library item, so only a partial state
    /// shows.
    #[must_use]
    pub(crate) const fn shows_in(self, scope: MusicListScope) -> bool {
        match scope {
            MusicListScope::Everything => true,
            MusicListScope::Library => matches!(self, Self::Partial { .. }),
        }
    }
}

/// Adds the state word to the secondary text of a Library release row,
/// when `scope` shows it.
pub(crate) fn add_state_word(
    row: &mut DestinationListRowVm,
    state: ReleaseStateWord,
    scope: MusicListScope,
) {
    if state.shows_in(scope) {
        row.secondary_text = format!(
            "{}{RELEASE_ROW_SEPARATOR}{}",
            row.secondary_text,
            state.label()
        );
    }
}

/// The row of one `MusicIndex` release (packet 016 "The Releases list in
/// Everything"): the title, then "`release_artist` · Kind", with no state
/// word. The row reads only the fields of the list row, so it needs no
/// detail request (decision D16). `None` for a row with no feed GUID.
#[must_use]
pub(crate) fn index_release_row(feed: &Feed) -> Option<DestinationListRowVm> {
    let feed_guid = feed.feed_guid.as_deref().filter(|guid| !guid.is_empty())?;
    let key = ReleaseKey::FeedGuid(feed_guid.to_owned());
    let title = index_release_title(feed);
    let track_count = feed
        .episode_count
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or(0);
    let kind = ReleaseKind::resolve(feed.release_kind.as_deref(), track_count).label();
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
#[must_use]
pub(crate) fn everything_entries(
    library: Vec<LibraryReleaseEntry>,
    hidden_keys: &BTreeSet<ReleaseKey>,
    page: &IndexNameListPage<Feed>,
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
            let row = index_release_row(feed)?;
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::view_models::index_name_list::IndexNameListBatch;

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

        let complete =
            everything_entries(library.clone(), &keys, &loaded_page(recorded_rows(), false));
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

        let partial = everything_entries(library, &keys, &loaded_page(recorded_rows(), true));
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
        assert!(everything_entries(library, &BTreeSet::new(), &page).is_empty());
    }

    /// R16-14: a `MusicIndex` row gives "`release_artist` · Kind" with no
    /// state word. The kind comes from `ReleaseKind::resolve`.
    #[test]
    fn adr_0084_scopes_r16_14_index_row_text() {
        let rows = recorded_rows();
        let liii = index_release_row(&rows[0]).unwrap();
        assert_eq!(liii.label, "#LIII4PRESIDENT");
        assert_eq!(liii.secondary_text, "LIIINEKELA \u{b7} Single");
        assert_eq!(
            liii.open,
            DestinationListRowTarget::Release(ReleaseKey::FeedGuid("liii-album".into()))
        );
        let motive = index_release_row(&rows[2]).unwrap();
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
        assert!(index_release_row(&no_guid).is_none());
    }

    /// R16-14 and decision D13: the state words, and the scope that shows
    /// each of them.
    #[test]
    fn adr_0084_scopes_r16_14_state_words() {
        let full = ReleaseStateWord::of_tracks(7, 7);
        let partial = ReleaseStateWord::of_tracks(3, 7);
        let none = ReleaseStateWord::of_tracks(0, 7);
        assert_eq!(full.label(), "In Library");
        assert_eq!(partial.label(), "3 of 7 tracks");
        assert_eq!(none.label(), "Not downloaded");
        assert_eq!(
            ReleaseStateWord::of_tracks(0, 0),
            ReleaseStateWord::NotDownloaded
        );
        for word in [full, partial, none] {
            assert!(word.shows_in(MusicListScope::Everything));
        }
        assert!(!full.shows_in(MusicListScope::Library));
        assert!(partial.shows_in(MusicListScope::Library));
        assert!(!none.shows_in(MusicListScope::Library));
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
}
