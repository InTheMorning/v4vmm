//! The window groups of a saved search page (ADR 0086 Decision 6).
//!
//! The Releases tab of a saved search page shows one group row for two or
//! more singles of one artist in one window. The groups use the key, the
//! chain and the window length of New releases. A saved search groups only
//! its loaded rows and sends no request for a window. A relevance order
//! never closes a window, so the page gives the open state of each group
//! (packet 005 decisions D2 and D3).
//!
//! The page makes the groups at each call, from the Library facts of the
//! lookup of that call (decision D9). Thus a download, a removal and a new
//! Index reply change the groups in place.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

use crate::view_models::artist_identity::{ArtistIdentity, UNKNOWN_ARTIST_TEXT};
use crate::view_models::detail_selection::DetailRowKey;
use crate::view_models::library::ContentListRowDisplay;
use crate::view_models::list_selection::SelectionRow;
use crate::view_models::singles_group::SinglesGroupKey;
use crate::view_models::window_groups::{
    window_groups, NewReleasesLayout, WindowEntry, WindowGroupSource, WindowGroupTileVm, WindowItem,
};
use crate::view_models::workspace::ReleaseKey;
use crate::views::FeedView;

use super::{FeedResultDisplay, SearchLibraryLookup, SearchResultRowVm};

/// One item of a search page: a row, or a group row of a saved search
/// (ADR 0086 Decision 6).
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "a search page holds one item for each row, and a row was never boxed in that list"
)]
pub(crate) enum SearchResultItemVm {
    /// One row.
    Row(SearchResultRowVm),
    /// One group row at the place of its anchor single.
    WindowGroup(Box<WindowGroupTileVm>),
}

impl SearchResultItemVm {
    /// The selection key of the item: the key of a row, or the key of the
    /// anchor of a group. `None` for a row with no key.
    #[must_use]
    pub(crate) fn key(&self) -> Option<DetailRowKey> {
        match self {
            Self::Row(row) => row.key.clone(),
            Self::WindowGroup(tile) => Some(DetailRowKey::Release(tile.page_key.anchor.clone())),
        }
    }

    /// The row id of the item: the id of a row, or the row id of the
    /// anchor of a group.
    #[must_use]
    pub(crate) fn row_id(&self) -> &str {
        match self {
            Self::Row(row) => &row.id,
            Self::WindowGroup(tile) => &tile.open_row_id,
        }
    }
}

/// One item of the grouped release rows, before the row view models.
pub(super) enum SavedSearchWindowItem {
    /// The release row at this position shows as itself.
    Plain(usize),
    /// A group at the place of its anchor. `members` holds the position of
    /// each member, newest first.
    Group {
        tile: Box<WindowGroupTileVm>,
        members: Vec<usize>,
    },
}

/// The selection row of a group: the anchor key is its identity, and each
/// member is a member (ADR 0086 packet 004 decision D12).
pub(super) fn window_group_selection_row(
    tile: &WindowGroupTileVm,
) -> SelectionRow<DetailRowKey, DetailRowKey> {
    SelectionRow::new(
        DetailRowKey::Release(tile.page_key.anchor.clone()),
        tile.member_keys
            .iter()
            .cloned()
            .map(DetailRowKey::Release)
            .collect(),
    )
}

/// One window entry for each merged release row, in the visible order
/// (ADR 0086 Decision 1, packet 005 "Which rows group").
///
/// - A row that the Library holds takes the kind, the group key and the
///   release date of the Library album (decision D6).
/// - Each other row takes its own kind, the group key of its artist text
///   and its `newest_item_at` (decision D5).
/// - A row that is not a single has no group key. A row with no feed GUID
///   has no group key, by the rule of packet 002 for a `WindowEntry`.
/// - No entry has an order date, because a relevance order never closes a
///   window (decision D2).
#[must_use]
pub(crate) fn saved_search_window_entries(
    rows: &[FeedResultDisplay],
    lookup: &dyn SearchLibraryLookup,
) -> Vec<WindowEntry> {
    rows.iter()
        .enumerate()
        .map(|(position, row)| window_entry(position, row, lookup))
        .collect()
}

fn window_entry(
    position: usize,
    row: &FeedResultDisplay,
    lookup: &dyn SearchLibraryLookup,
) -> WindowEntry {
    let mut entry = WindowEntry {
        position,
        key: row
            .key
            .clone()
            .unwrap_or_else(|| ReleaseKey::from_index_feed_guid(row.id.as_str())),
        group_key: None,
        release_date: None,
        order_date: None,
        in_library: row
            .key
            .as_ref()
            .is_some_and(|key| lookup.release(key).is_some()),
    };
    let Some(key @ ReleaseKey::FeedGuid(_)) = &row.key else {
        return entry;
    };
    if let Some(album) = lookup.release_window_facts(key) {
        entry.release_date = album.release_date;
        if album.kind.is_single() {
            entry.group_key = album.group_key;
        }
    } else {
        entry.release_date = row.newest_item_at;
        if row.kind.is_single() {
            entry.group_key = row
                .artist_text
                .as_deref()
                .and_then(|text| lookup.window_key_for_name(text));
        }
    }
    entry
}

/// Tells if the group key of `row` comes from a name that the tree matches
/// to one artist feed (packet 002 decision D4).
fn joined_by_name(row: &FeedResultDisplay, lookup: &dyn SearchLibraryLookup) -> bool {
    let Some(key) = &row.key else {
        return false;
    };
    match lookup.release_window_facts(key) {
        Some(album) => album.joined_by_name,
        None => row
            .artist_text
            .as_deref()
            .and_then(|text| lookup.window_key_for_name(text))
            .is_some_and(|key| matches!(key.0, ArtistIdentity::Publisher(_))),
    }
}

/// The title of a group, from its anchor (packet 005 "The items of one
/// render"). A Library anchor gives the name of its artist node. An Index
/// anchor with the key of a Library artist feed gives the title of that
/// feed. Each other anchor gives its artist text.
fn group_title(
    anchor: &FeedResultDisplay,
    entry: &WindowEntry,
    lookup: &dyn SearchLibraryLookup,
) -> String {
    if let Some(album) = anchor
        .key
        .as_ref()
        .and_then(|key| lookup.release_window_facts(key))
    {
        return album.artist_title;
    }
    match &entry.group_key {
        Some(SinglesGroupKey(ArtistIdentity::Publisher(feed_guid))) => {
            lookup.window_feed_title(feed_guid)
        }
        _ => anchor
            .artist_text
            .clone()
            .unwrap_or_else(|| UNKNOWN_ARTIST_TEXT.to_owned()),
    }
}

/// The items of the release rows of a saved search (ADR 0086 Decision 6).
///
/// A plain item gives its position. A group comes at the place of its
/// anchor, and each other member gives no item. `window_closed` is the
/// state of each group: true only when the page holds each row of the
/// query (packet 005 decision D3). The page key of each tile holds the
/// rows of its members, so the window group page lists them (decision
/// D11).
pub(super) fn saved_search_window_items(
    rows: &[FeedResultDisplay],
    lookup: &dyn SearchLibraryLookup,
    window_closed: bool,
) -> Vec<SavedSearchWindowItem> {
    let entries = saved_search_window_entries(rows, lookup);
    window_groups(&entries, lookup.window_length())
        .into_iter()
        .map(|item| match item {
            WindowItem::Plain(position) => SavedSearchWindowItem::Plain(position),
            WindowItem::Group(group) => {
                let anchor = &rows[group.anchor];
                let group_name = group_title(anchor, &entries[group.anchor], lookup);
                let mut tile = WindowGroupTileVm::new(
                    &group,
                    &entries,
                    &anchor.id,
                    &group_name,
                    anchor.thumbnail_href.as_deref(),
                );
                tile.set_window_closed(window_closed);
                tile.page_key.source = WindowGroupSource::SavedSearch(
                    group
                        .members
                        .iter()
                        .map(|position| rows[*position].clone())
                        .collect(),
                );
                SavedSearchWindowItem::Group {
                    tile: Box::new(tile),
                    members: group.members,
                }
            }
        })
        .collect()
}

/// The layout and the rows of the window group page of a saved search
/// (ADR 0086 packet 005 decision D11). `rows` are the member rows that
/// the page key holds. The layout reads the Library facts of `lookup`, so
/// the page shows the current Library state of each member.
///
/// The page finds a member by the feed GUID of its row. A search row has
/// no remote feed, so its page row gets a feed view with only that GUID.
#[must_use]
pub(crate) fn saved_search_window_page(
    rows: &[FeedResultDisplay],
    lookup: &dyn SearchLibraryLookup,
) -> (NewReleasesLayout, Vec<ContentListRowDisplay>) {
    let entries = saved_search_window_entries(rows, lookup);
    let joined: BTreeSet<usize> = rows
        .iter()
        .zip(&entries)
        .filter(|(row, entry)| entry.group_key.is_some() && joined_by_name(row, lookup))
        .map(|(_, entry)| entry.position)
        .collect();
    let row_ids = rows.iter().map(|row| row.id.clone()).collect();
    let page_rows = rows.iter().map(page_row).collect();
    (
        NewReleasesLayout::from_entries(entries, row_ids, joined),
        page_rows,
    )
}

/// The page row of one member row.
fn page_row(row: &FeedResultDisplay) -> ContentListRowDisplay {
    let mut display = row.clone();
    if display.remote_feed.is_none() {
        if let Some(ReleaseKey::FeedGuid(feed_guid)) = &row.key {
            display.remote_feed = Some(FeedView {
                feed_guid: Some(feed_guid.clone()),
                ..FeedView::default()
            });
        }
    }
    ContentListRowDisplay::from_release_result(display)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{local_guid_track, RecordedLibrary};
    use super::super::{
        ArtistSearchFacts, IndexSearchResultRows, SavedSearch, SavedSearchId,
        SearchResultsInspectorPageVm, SearchResultsTab,
    };
    use super::*;
    use crate::view_models::detail_selection::DetailPageRows;
    use crate::view_models::index_name_list::name_sort_key;
    use crate::view_models::list_selection::FocusStep;
    use crate::view_models::release_kind::ReleaseKind;
    use crate::view_models::search_results::{SearchReleaseWindowFacts, SearchResultOrigin};
    use crate::view_models::window_groups::{WindowGroupStateVm, WindowLength};

    /// The UTC midnight of `date` in October 2026.
    fn oct(date: u32) -> i64 {
        chrono::NaiveDate::from_ymd_opt(2026, 10, date)
            .and_then(|day| day.and_hms_opt(0, 0, 0))
            .expect("a valid date")
            .and_utc()
            .timestamp()
    }

    fn publisher(feed_guid: &str) -> SinglesGroupKey {
        SinglesGroupKey(ArtistIdentity::Publisher(feed_guid.to_owned()))
    }

    fn guid_key(guid: &str) -> ReleaseKey {
        ReleaseKey::FeedGuid(guid.to_owned())
    }

    /// A search release row of the kind `kind`, as an Index search gives
    /// it.
    fn release(guid: &str, kind: ReleaseKind) -> FeedResultDisplay {
        FeedResultDisplay::new(
            format!("index-feed:{guid}"),
            format!("Release {guid}"),
            SearchResultOrigin::Index,
        )
        .with_release_key(guid_key(guid), kind)
    }

    /// An Index single with a release date and an artist text.
    fn index_single(guid: &str, date: Option<i64>, artist: &str) -> FeedResultDisplay {
        release(guid, ReleaseKind::Single)
            .with_newest_item_at(date)
            .with_artist_text(artist)
    }

    /// Adds the Library release `guid` with its window facts.
    fn library(
        lookup: RecordedLibrary,
        guid: &str,
        feed_id: i64,
        kind: ReleaseKind,
        date: i64,
    ) -> RecordedLibrary {
        let mut lookup = lookup.with_release(guid, feed_id, false);
        lookup.window_facts.insert(
            guid_key(guid),
            SearchReleaseWindowFacts {
                kind,
                group_key: Some(publisher("feed-jsmovie")),
                joined_by_name: false,
                release_date: Some(date),
                artist_title: "jSmovie".to_owned(),
            },
        );
        lookup
    }

    fn saved_search() -> SavedSearch {
        SavedSearch {
            id: SavedSearchId(1),
            query: "q".to_owned(),
            start_tab: SearchResultsTab::Releases,
        }
    }

    fn index_rows(rows: Vec<FeedResultDisplay>) -> IndexSearchResultRows {
        IndexSearchResultRows {
            feeds: rows
                .into_iter()
                .zip(2_000_000_000..)
                .map(|(row, id)| (id, row))
                .collect(),
            ..IndexSearchResultRows::default()
        }
    }

    /// A saved search page on the Releases tab with the Index rows `rows`,
    /// with each row of the query loaded.
    fn saved_page(rows: Vec<FeedResultDisplay>) -> SearchResultsInspectorPageVm {
        let mut vm = SearchResultsInspectorPageVm::new("q").into_saved_search(&saved_search());
        vm.replace_index_results(index_rows(rows));
        vm
    }

    /// The five rows of R86-5-01: Library singles of one key on Oct 8 and
    /// Oct 20 at positions 1 and 3, and three albums.
    fn r01() -> (SearchResultsInspectorPageVm, RecordedLibrary) {
        let vm = saved_page(vec![
            release("a0", ReleaseKind::Album),
            release("s8", ReleaseKind::Single),
            release("a2", ReleaseKind::Album),
            release("s20", ReleaseKind::Single),
            release("a4", ReleaseKind::Album),
        ]);
        let lookup = library(
            RecordedLibrary::default(),
            "s8",
            8,
            ReleaseKind::Single,
            oct(8),
        );
        let lookup = library(lookup, "s20", 20, ReleaseKind::Single, oct(20));
        (vm, lookup)
    }

    fn item_ids(items: &[SearchResultItemVm]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                SearchResultItemVm::Row(row) => row.id.clone(),
                SearchResultItemVm::WindowGroup(tile) => format!("group:{}", tile.open_row_id),
            })
            .collect()
    }

    fn tiles(items: &[SearchResultItemVm]) -> Vec<&WindowGroupTileVm> {
        items
            .iter()
            .filter_map(|item| match item {
                SearchResultItemVm::WindowGroup(tile) => Some(tile.as_ref()),
                SearchResultItemVm::Row(_) => None,
            })
            .collect()
    }

    /// R86-5-01, R86-5-02 and R86-5-03: four items. The group is at the
    /// place of the Oct 20 row, also when the Oct 8 row has the better
    /// rank. The other rows keep their order, and the Oct 8 row gives no
    /// item.
    #[test]
    fn adr_0086_saved_search_groups_r01_r02_r03_group_at_the_newest_single() {
        let (vm, lookup) = r01();
        let items = vm.visible_items(&lookup);
        assert_eq!(
            item_ids(&items),
            [
                "index-feed:a0",
                "index-feed:a2",
                "group:index-feed:s20",
                "index-feed:a4"
            ]
        );
    }

    /// R86-5-05: a Library row whose search matched one track of a
    /// ten-track album has no group key. Its entry takes the album kind.
    #[test]
    fn adr_0086_saved_search_groups_r05_library_album_kind_wins() {
        let rows = [release("tenth", ReleaseKind::Single)];
        let lookup = library(
            RecordedLibrary::default(),
            "tenth",
            10,
            ReleaseKind::Album,
            oct(20),
        );
        let entries = saved_search_window_entries(&rows, &lookup);
        assert_eq!(entries[0].group_key, None);
        assert!(entries[0].in_library);
    }

    /// R86-5-06: a Library single and an Index single whose artist text
    /// matches the feed of the Library key give one group of two. The
    /// state word is "New" and the count text is "1 in library".
    #[test]
    fn adr_0086_saved_search_groups_r06_index_row_joins_by_name() {
        let vm = saved_page(vec![
            release("lib", ReleaseKind::Single),
            index_single("idx", Some(oct(15)), "jSmovie"),
        ]);
        let mut lookup = library(
            RecordedLibrary::default(),
            "lib",
            1,
            ReleaseKind::Single,
            oct(20),
        );
        lookup
            .name_feeds
            .insert(name_sort_key("jSmovie"), "feed-jsmovie".to_owned());
        let items = vm.visible_items(&lookup);
        let groups = tiles(&items);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 2);
        assert_eq!(groups[0].state.word(), "New");
        assert_eq!(
            groups[0].state.count_text().as_deref(),
            Some("1 in library")
        );
    }

    /// R86-5-07: an Index single with no `newest_item_at` gives a row. It
    /// never anchors a group and never joins one.
    #[test]
    fn adr_0086_saved_search_groups_r07_index_row_with_no_date_stays_a_row() {
        let vm = saved_page(vec![
            index_single("idx", None, "jSmovie"),
            index_single("other", None, "jSmovie"),
            release("lib", ReleaseKind::Single),
        ]);
        let mut lookup = library(
            RecordedLibrary::default(),
            "lib",
            1,
            ReleaseKind::Single,
            oct(20),
        );
        lookup
            .name_feeds
            .insert(name_sort_key("jSmovie"), "feed-jsmovie".to_owned());
        let items = vm.visible_items(&lookup);
        assert!(tiles(&items).is_empty());
        assert_eq!(
            item_ids(&items),
            ["index-feed:idx", "index-feed:other", "index-feed:lib"]
        );
    }

    /// R86-5-08: a Library row and an Index row with one release key give
    /// one entry. The group counts the release one time, with the
    /// Library date.
    #[test]
    fn adr_0086_saved_search_groups_r08_merged_release_counts_once() {
        let local = [local_guid_track(10, 7, "both", "i1", "One")];
        let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
            "q",
            &local,
            &ArtistSearchFacts::default(),
        )
        .into_saved_search(&saved_search());
        vm.replace_index_results(index_rows(vec![
            index_single("both", Some(oct(1)), "jSmovie"),
            release("other", ReleaseKind::Single),
        ]));
        let lookup = library(
            RecordedLibrary::default(),
            "both",
            7,
            ReleaseKind::Single,
            oct(20),
        );
        let lookup = library(lookup, "other", 8, ReleaseKind::Single, oct(18));
        let items = vm.visible_items(&lookup);
        let groups = tiles(&items);
        assert_eq!(items.len(), 1);
        assert_eq!(groups[0].count, 2);
        assert_eq!(groups[0].member_keys, [guid_key("both"), guid_key("other")]);
        assert_eq!(groups[0].date_span, "Oct 18 to Oct 20");
    }

    /// R86-5-09: an album and an EP of the key, dated between two singles,
    /// give rows at their places. The two singles still make one group.
    #[test]
    fn adr_0086_saved_search_groups_r09_album_and_ep_stay_rows() {
        let vm = saved_page(vec![
            release("s20", ReleaseKind::Single),
            release("album", ReleaseKind::Album),
            release("ep", ReleaseKind::Ep),
            release("s10", ReleaseKind::Single),
        ]);
        let lookup = library(
            RecordedLibrary::default(),
            "s20",
            1,
            ReleaseKind::Single,
            oct(20),
        );
        let lookup = library(lookup, "album", 2, ReleaseKind::Album, oct(16));
        let lookup = library(lookup, "ep", 3, ReleaseKind::Ep, oct(14));
        let lookup = library(lookup, "s10", 4, ReleaseKind::Single, oct(10));
        let items = vm.visible_items(&lookup);
        assert_eq!(
            item_ids(&items),
            ["group:index-feed:s20", "index-feed:album", "index-feed:ep"]
        );
        assert_eq!(tiles(&items)[0].count, 2);
    }

    /// R86-5-10: with `feed_rows_partial` true, the rows are not complete
    /// and the window of the tile is open.
    #[test]
    fn adr_0086_saved_search_groups_r10_partial_feed_rows_open_each_window() {
        let (mut vm, lookup) = r01();
        let mut rows = index_rows(vec![
            release("a0", ReleaseKind::Album),
            release("s8", ReleaseKind::Single),
            release("s20", ReleaseKind::Single),
        ]);
        rows.feed_rows_partial = true;
        vm.replace_index_results(rows);
        assert!(!vm.release_rows_complete());
        let items = vm.visible_items(&lookup);
        let groups = tiles(&items);
        assert!(!groups[0].window_closed);
        assert_eq!(groups[0].kind_badge, "2+ Singles");
    }

    /// R86-5-11: with each condition of "The open state" met, the rows are
    /// complete and the window of the tile is closed.
    #[test]
    fn adr_0086_saved_search_groups_r11_complete_rows_close_each_window() {
        let (vm, lookup) = r01();
        assert!(vm.release_rows_complete());
        let items = vm.visible_items(&lookup);
        assert!(tiles(&items)[0].window_closed);
    }

    /// R86-5-12: the rows are not complete while the Index request loads,
    /// and after an Index error.
    #[test]
    fn adr_0086_saved_search_groups_r12_loading_and_error_are_not_complete() {
        let (mut vm, _) = r01();
        vm.mark_index_loading();
        assert!(!vm.release_rows_complete());
        vm.set_index_error(
            &crate::application::CommandError::Query("connection refused".into()),
            "https://musicindex.test",
            std::time::SystemTime::UNIX_EPOCH,
        );
        assert!(!vm.release_rows_complete());
    }

    /// R86-5-13: a Library search that reached its limit makes the rows
    /// not complete, with each other condition met.
    #[test]
    fn adr_0086_saved_search_groups_r13_library_limit_is_not_complete() {
        let mut vm = SearchResultsInspectorPageVm::new("q")
            .with_library_rows_partial(true)
            .into_saved_search(&saved_search());
        vm.replace_index_results(index_rows(Vec::new()));
        assert!(!vm.release_rows_complete());
    }

    /// R86-5-17: the toolbar search, and the Artists tab and the Tracks
    /// tab of a saved search page, give only rows.
    #[test]
    fn adr_0086_saved_search_groups_r17_other_views_do_not_group() {
        let (_, lookup) = r01();
        let mut toolbar = SearchResultsInspectorPageVm::new("q");
        toolbar.replace_index_results(index_rows(vec![
            release("s8", ReleaseKind::Single),
            release("s20", ReleaseKind::Single),
        ]));
        assert!(tiles(&toolbar.visible_items(&lookup)).is_empty());
        assert!(toolbar
            .window_group_for_row("index-feed:s20", &lookup)
            .is_none());

        let (mut vm, lookup) = r01();
        for tab in [SearchResultsTab::Artists, SearchResultsTab::Tracks] {
            vm.set_tab(tab);
            assert!(tiles(&vm.visible_items(&lookup)).is_empty());
        }
    }

    /// R86-5-18: a window length of 7 days gives two rows for the singles
    /// of R86-5-01.
    #[test]
    fn adr_0086_saved_search_groups_r18_window_length_of_new_releases() {
        let (vm, mut lookup) = r01();
        lookup.window_length = WindowLength::from_days(7).expect("a valid length");
        let items = vm.visible_items(&lookup);
        assert!(tiles(&items).is_empty());
        assert_eq!(items.len(), 5);
    }

    /// R86-5-19: in `Normal`, the selection rows are each plain row and
    /// each anchor row, in item order. The focused anchor gives the index
    /// of its item.
    #[test]
    fn adr_0086_saved_search_groups_r19_selection_rows_follow_the_items() {
        let (mut vm, lookup) = r01();
        let rows = vm
            .selection_rows(&lookup)
            .expect("the Releases tab has selection rows");
        assert!(matches!(rows, DetailPageRows::GroupedReleases { .. }));
        let ids: Vec<DetailRowKey> = rows
            .selection_rows()
            .into_iter()
            .map(|row| row.id)
            .collect();
        assert_eq!(
            ids,
            ["a0", "a2", "s20", "a4"].map(|guid| DetailRowKey::Release(guid_key(guid)))
        );
        vm.move_focus(FocusStep::Home, &lookup);
        vm.move_focus(FocusStep::Down, &lookup);
        vm.move_focus(FocusStep::Down, &lookup);
        assert_eq!(vm.focused_row_index(&lookup), Some(2));
    }

    /// R86-5-19, selection mode: the checkbox of a group row selects each
    /// member, and the bar counts each member (packet 004 decision D12).
    #[test]
    fn adr_0086_saved_search_groups_r19_group_checkbox_selects_each_member() {
        let (mut vm, lookup) = r01();
        vm.toggle_mode();
        vm.toggle_row(&DetailRowKey::Release(guid_key("s20")), &lookup);
        assert_eq!(vm.selection().selected_count(), 2);
        let items = vm.visible_items(&lookup);
        let tile = tiles(&items)[0];
        assert!(tile.selection.checkbox_visible);
        assert!(tile.selection.check_state.is_checked());
    }

    /// R86-5-20: the anchor row id gives the tile with both members. A
    /// plain row id gives `None`.
    #[test]
    fn adr_0086_saved_search_groups_r20_group_of_the_anchor_row() {
        let (vm, lookup) = r01();
        let tile = vm
            .window_group_for_row("index-feed:s20", &lookup)
            .expect("the anchor row has a group");
        assert_eq!(tile.member_keys, [guid_key("s20"), guid_key("s8")]);
        let WindowGroupSource::SavedSearch(rows) = &tile.page_key.source else {
            panic!("the page key holds the member rows");
        };
        assert_eq!(rows.len(), 2);
        assert!(vm.window_group_for_row("index-feed:a2", &lookup).is_none());
    }

    /// R86-5-21: a new Index single in the window gives the group at the
    /// next call. A lookup that now holds a member changes the Library
    /// count at the next call.
    #[test]
    fn adr_0086_saved_search_groups_r21_groups_change_in_place() {
        let mut vm = saved_page(vec![release("lib", ReleaseKind::Single)]);
        let lookup = library(
            RecordedLibrary::default(),
            "lib",
            1,
            ReleaseKind::Single,
            oct(20),
        );
        let mut lookup = lookup;
        lookup
            .name_feeds
            .insert(name_sort_key("jSmovie"), "feed-jsmovie".to_owned());
        assert!(tiles(&vm.visible_items(&lookup)).is_empty());

        vm.replace_index_results(index_rows(vec![
            release("lib", ReleaseKind::Single),
            index_single("idx", Some(oct(15)), "jSmovie"),
        ]));
        let items = vm.visible_items(&lookup);
        assert_eq!(
            tiles(&items)[0].state,
            WindowGroupStateVm::New { in_library: 1 }
        );

        let lookup = lookup.with_release("idx", 2, true);
        let items = vm.visible_items(&lookup);
        assert_eq!(tiles(&items)[0].state, WindowGroupStateVm::InLibrary);
    }

    /// R86-5-22: the tile of R86-5-01 gives the artist node name, the date
    /// span and the kind badge.
    #[test]
    fn adr_0086_saved_search_groups_r22_tile_texts() {
        let (vm, lookup) = r01();
        let items = vm.visible_items(&lookup);
        let tile = tiles(&items)[0];
        assert_eq!(tile.title, "jSmovie");
        assert_eq!(tile.date_span, "Oct 8 to Oct 20");
        assert_eq!(tile.kind_badge, "2 Singles");
    }

    /// R86-5-23: an Index anchor with the key of a feed takes the feed
    /// title. An Index anchor with a name key takes its artist text.
    #[test]
    fn adr_0086_saved_search_groups_r23_index_anchor_titles() {
        let vm = saved_page(vec![
            index_single("m20", Some(oct(20)), "jsmovie"),
            index_single("m18", Some(oct(18)), "JSMOVIE"),
            index_single("n20", Some(oct(20)), "Rain Band"),
            index_single("n18", Some(oct(18)), "rain band"),
        ]);
        let mut lookup = RecordedLibrary::default();
        lookup
            .name_feeds
            .insert(name_sort_key("jSmovie"), "feed-jsmovie".to_owned());
        lookup
            .feed_titles
            .insert("feed-jsmovie".to_owned(), "jSmovie".to_owned());
        let items = vm.visible_items(&lookup);
        let titles: Vec<&str> = tiles(&items)
            .iter()
            .map(|tile| tile.title.as_str())
            .collect();
        assert_eq!(titles, ["jSmovie", "Rain Band"]);
    }

    /// The window group page of a saved search finds each member by the
    /// feed GUID of its row (packet 005 decision D11).
    #[test]
    fn adr_0086_saved_search_groups_page_layout_lists_the_members() {
        let (vm, lookup) = r01();
        let tile = vm
            .window_group_for_row("index-feed:s20", &lookup)
            .expect("the anchor row has a group");
        let WindowGroupSource::SavedSearch(rows) = &tile.page_key.source else {
            panic!("the page key holds the member rows");
        };
        let (layout, page_rows) = saved_search_window_page(rows, &lookup);
        assert_eq!(
            layout.window_member_row_ids(&tile.page_key),
            ["index-feed:s20", "index-feed:s8"]
        );
        assert_eq!(page_rows.len(), 2);
    }
}
