use std::collections::{BTreeMap, BTreeSet};

use crate::db::TrackRow;
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::detail_selection::{DetailRowClick, DetailRowKey, LibraryReleaseFacts};
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::selection_action_bar::SelectionActionKind;
use crate::view_models::workspace::{ReleaseKey, TrackKey};
use crate::views::{FeedView, TrackView};

use super::{
    ArtistResultDisplay, FeedResultDisplay, IndexDetailKind, IndexSearchResultRows,
    SearchLibraryLookup, SearchLibraryRelease, SearchLibraryTrack, SearchResultItemId,
    SearchResultOrigin, SearchResultTarget, SearchResultsInspectorPageVm, SearchResultsTab,
    SearchRowBadge, TrackResultDisplay,
};

/// Recorded Library facts for the search rows.
#[derive(Default)]
pub(super) struct RecordedLibrary {
    releases: BTreeMap<ReleaseKey, SearchLibraryRelease>,
    tracks: BTreeMap<TrackKey, SearchLibraryTrack>,
    artist_feeds: BTreeSet<String>,
}

impl SearchLibraryLookup for RecordedLibrary {
    fn release(&self, key: &ReleaseKey) -> Option<SearchLibraryRelease> {
        self.releases.get(key).cloned()
    }

    fn track(&self, key: &TrackKey) -> Option<SearchLibraryTrack> {
        self.tracks.get(key).cloned()
    }

    fn is_artist_feed(&self, feed_guid: &str) -> bool {
        self.artist_feeds.contains(feed_guid)
    }
}

impl RecordedLibrary {
    fn with_release(mut self, guid: &str, feed_id: i64, fully: bool, word: &str) -> Self {
        self.releases.insert(
            ReleaseKey::FeedGuid(guid.to_owned()),
            SearchLibraryRelease {
                facts: LibraryReleaseFacts {
                    feed_id,
                    fully_downloaded: fully,
                    download_active: false,
                },
                state_word: word.to_owned(),
                kind: ReleaseKind::Album,
            },
        );
        self
    }

    fn with_track(
        mut self,
        feed_guid: &str,
        item_guid: &str,
        track_id: i64,
        downloaded: bool,
    ) -> Self {
        self.tracks.insert(
            track_key(feed_guid, item_guid),
            SearchLibraryTrack {
                track_id,
                downloaded,
                download_active: false,
                state_word: if downloaded {
                    "In Library"
                } else {
                    "Not downloaded"
                }
                .to_owned(),
            },
        );
        self
    }
}

fn track_key(feed_guid: &str, item_guid: &str) -> TrackKey {
    TrackKey::from_index_track_target(Some(feed_guid), item_guid)
}

fn artist(id: SearchResultItemId, label: &str) -> ArtistResultDisplay {
    ArtistResultDisplay::new(format!("index-artist:{label}"), label)
        .with_thumbnail_href(format!("https://example.test/{id}.png"))
}

/// An Index release row, as `fetch_index_search_result_rows` builds it.
pub(super) fn index_release(
    id: SearchResultItemId,
    guid: &str,
    label: &str,
) -> (SearchResultItemId, FeedResultDisplay) {
    (
        id,
        FeedResultDisplay::new(
            format!("index-feed:{guid}"),
            label,
            SearchResultOrigin::Index,
        )
        .with_release_key(ReleaseKey::FeedGuid(guid.to_owned()), ReleaseKind::Album),
    )
}

/// An Index track row, as `fetch_index_search_result_rows` builds it.
pub(super) fn index_track(
    id: SearchResultItemId,
    feed_guid: Option<&str>,
    item_guid: &str,
    label: &str,
) -> (SearchResultItemId, TrackResultDisplay) {
    let row_id = super::index_track_row_id(feed_guid, item_guid);
    let mut row = TrackResultDisplay::new(row_id, label, SearchResultOrigin::Index);
    if let Some(feed_guid) = feed_guid {
        row = row.with_track_key(track_key(feed_guid, item_guid));
    }
    (id, row)
}

fn index_feed_page(
    id: SearchResultItemId,
    guid: &str,
    label: &str,
) -> (SearchResultItemId, ArtistResultDisplay) {
    let mut row = ArtistResultDisplay::new(
        format!("{}{guid}", super::INDEX_FEED_PAGE_RESULT_PREFIX),
        label,
    );
    row.a11y_label = format!("Open the feed page {label}");
    (id, row)
}

fn remote_track() -> TrackView {
    TrackView {
        title: Some("Remote Track".to_string()),
        artist: Some("Remote Artist".to_string()),
        feed_title: Some("Remote Release".to_string()),
        track_number: Some(7),
        duration_secs: Some(125),
        pub_date: Some(1_712_275_200),
        explicit: Some(true),
        transcript_url: Some("https://example.test/transcript.srt".to_string()),
        ..TrackView::default()
    }
}

fn local_track(id: i64, feed_id: i64, feed_title: &str, title: &str, artist: &str) -> TrackRow {
    TrackRow {
        id,
        feed_id,
        track_title: Some(title.to_string()),
        artist_name: Some(artist.to_string()),
        album_artist_name: Some(artist.to_string()),
        feed_title: Some(feed_title.to_string()),
        is_in_library: true,
        ..TrackRow::default()
    }
}

/// A Library track of the feed `feed_guid`, with the item GUID `item_guid`.
fn local_guid_track(
    id: i64,
    feed_id: i64,
    feed_guid: &str,
    item_guid: &str,
    title: &str,
) -> TrackRow {
    TrackRow {
        feed_guid: Some(feed_guid.to_owned()),
        item_guid: item_guid.to_owned(),
        ..local_track(id, feed_id, "Library Release", title, "Haleen")
    }
}

fn rows(
    vm: &SearchResultsInspectorPageVm,
    lookup: &RecordedLibrary,
) -> Vec<super::SearchResultRowVm> {
    vm.visible_rows(lookup)
}

fn no_index_rows() -> IndexSearchResultRows {
    IndexSearchResultRows::default()
}

/// R10-01: the tabs are Releases, Artists and Tracks, in that order, with
/// those labels. Releases is the default.
#[test]
fn adr_0084_search_list_r10_01_three_tabs_in_order() {
    assert_eq!(
        SearchResultsTab::ALL.map(SearchResultsTab::label),
        ["Releases", "Artists", "Tracks"]
    );
    assert_eq!(SearchResultsTab::default(), SearchResultsTab::Releases);
    assert_eq!(
        SearchResultsInspectorPageVm::new("q").tab(),
        SearchResultsTab::Releases
    );
}

/// R10-02: a Library release and an Index hit with one feed GUID give one
/// release row with the Library state word. An Index-only hit gives a row
/// with no state word.
#[test]
fn adr_0084_search_list_r10_02_one_release_row_for_each_item() {
    let local = [local_guid_track(10, 7, "g-library", "i1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "q",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    vm.replace_index_results(IndexSearchResultRows {
        feeds: vec![
            index_release(2_000_000_000, "g-library", "Library Release"),
            index_release(2_000_000_001, "g-index", "Index Release"),
        ],
        ..no_index_rows()
    });
    let lookup = RecordedLibrary::default().with_release("g-library", 7, true, "In Library");
    let rows = rows(&vm, &lookup);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].id, "7",
        "the merged row keeps the place of its Library row"
    );
    assert_eq!(rows[0].state_word.as_deref(), Some("In Library"));
    assert_eq!(rows[1].id, "index-feed:g-index");
    assert_eq!(rows[1].state_word, None);
    assert_eq!(
        rows[1].badge, None,
        "the list column shows the kind in the second line"
    );
    assert_eq!(rows[1].secondary_text, "Album");
    assert!(rows.iter().all(|row| !row.secondary_text.contains("Index")));
}

/// Walk of 2026-10-21, operator decision: in the list column, a release row
/// shows its kind as the first word of the second line, with one separator,
/// and no kind badge. The state word stays. A track row does not change.
/// The saved-search page in the detail column keeps the kind badge.
#[test]
fn adr_0084_search_list_release_kind_is_the_first_word_of_the_subtitle() {
    let local = [
        local_guid_track(10, 7, "g-library", "i1", "One"),
        local_guid_track(11, 7, "g-library", "i2", "Two"),
    ];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "haleen",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    let (id, index_row) = index_release(2_000_000_001, "g-index", "Index Release");
    let index_row = index_row.with_secondary_parts(vec!["HeyCitizen".into(), "19 tracks".into()]);
    vm.replace_index_results(IndexSearchResultRows {
        feeds: vec![(id, index_row)],
        tracks: vec![index_track(3_000_000_000, Some("g-index"), "t1", "Track")],
        ..no_index_rows()
    });
    let lookup = RecordedLibrary::default().with_release("g-library", 7, false, "Not downloaded");

    assert_eq!(
        vm.release_kind_placement(),
        super::ReleaseKindPlacement::Subtitle
    );
    let list = rows(&vm, &lookup);
    assert_eq!(
        list[0].secondary_text,
        "Album \u{b7} Haleen \u{b7} 2 tracks"
    );
    assert_eq!(list[0].state_word.as_deref(), Some("Not downloaded"));
    assert_eq!(list[0].badge, None);
    assert_eq!(
        list[1].secondary_text,
        "Album \u{b7} HeyCitizen \u{b7} 19 tracks"
    );
    assert_eq!(list[1].state_word, None, "an Index row shows no state word");
    assert_eq!(list[1].badge, None);

    vm.set_tab(SearchResultsTab::Tracks);
    let tracks = rows(&vm, &lookup);
    assert_eq!(tracks[0].badge, None, "a track row shows no badge");
    assert!(!tracks[0].secondary_text.starts_with("Album"));

    let saved = super::SavedSearch {
        id: super::SavedSearchId(1),
        query: "haleen".to_owned(),
        start_tab: SearchResultsTab::Releases,
    };
    let detail = vm.into_saved_search(&saved);
    assert_eq!(
        detail.release_kind_placement(),
        super::ReleaseKindPlacement::Badge
    );
    let detail_rows = rows(&detail, &lookup);
    assert_eq!(detail_rows[0].secondary_text, "Haleen - 2 tracks");
    assert_eq!(
        detail_rows[0].badge,
        Some(SearchRowBadge::Release(ReleaseKind::Album))
    );
    assert_eq!(detail_rows[1].secondary_text, "HeyCitizen - 19 tracks");
}

/// The merge part of R10-02, for the row image: a Library release row with
/// no image of its own takes the image of the Index row that its key
/// drops.
#[test]
fn adr_0084_search_list_merged_row_takes_the_index_image_when_the_library_has_none() {
    let local = [local_guid_track(10, 7, "g-library", "i1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "q",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    vm.replace_index_results(IndexSearchResultRows {
        feeds: vec![(
            2_000_000_000,
            index_release(2_000_000_000, "g-library", "Library Release")
                .1
                .with_thumbnail_href("https://example.test/index-cover.jpg"),
        )],
        ..no_index_rows()
    });
    let releases = vm.releases().rows();
    assert_eq!(releases.len(), 1, "the merged row keeps one place");
    assert_eq!(
        releases[0].thumbnail_href.as_deref(),
        Some("https://example.test/index-cover.jpg"),
        "the Library row had no image of its own, so it takes the Index image"
    );
}

/// R10-03: an Index release hit whose GUID is a Library feed with no
/// downloaded track gives a Library row with "Not downloaded".
#[test]
fn adr_0084_search_list_r10_03_index_hit_of_a_library_feed_is_a_library_row() {
    let mut vm = SearchResultsInspectorPageVm::new("q");
    vm.replace_index_results(IndexSearchResultRows {
        feeds: vec![index_release(2_000_000_000, "g-stored", "Stored Release")],
        ..no_index_rows()
    });
    let lookup = RecordedLibrary::default().with_release("g-stored", 3, false, "Not downloaded");
    let rows = rows(&vm, &lookup);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state_word.as_deref(), Some("Not downloaded"));
    let selection_rows = vm
        .selection_rows(&lookup)
        .expect("the Releases tab selects");
    let crate::view_models::detail_selection::DetailPageRows::Releases(release_rows) =
        selection_rows
    else {
        panic!("the Releases tab gives release rows");
    };
    assert_eq!(release_rows[0].library.map(|facts| facts.feed_id), Some(3));
}

/// R10-04: a Library track and an Index hit with one `TrackKey` give one
/// track row. The Library rows come before the Index rows.
#[test]
fn adr_0084_search_list_r10_04_one_track_row_for_each_item() {
    let local = [local_guid_track(10, 7, "g1", "item-1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "q",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    vm.set_tab(SearchResultsTab::Tracks);
    vm.replace_index_results(IndexSearchResultRows {
        tracks: vec![
            index_track(3_000_000_000, Some("g2"), "item-2", "Two"),
            index_track(3_000_000_001, Some("g1"), "item-1", "One"),
            index_track(3_000_000_002, None, "item-3", "Three"),
        ],
        ..no_index_rows()
    });
    let lookup = RecordedLibrary::default().with_track("g1", "item-1", 10, true);
    let rows = rows(&vm, &lookup);
    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        ["10", "index-track:g2:item-2", "index-track-guid:item-3"]
    );
    assert_eq!(rows[0].state_word.as_deref(), Some("In Library"));
    assert_eq!(rows[1].state_word, None);
    assert_eq!(rows[0].badge, None, "a track row shows no badge");
}

/// R10-06: a feed row has the badge "Artist" when the Library Artists list
/// has its GUID, else "Feed". No row label or accessibility label says
/// "Publisher".
#[test]
fn adr_0084_search_list_r10_06_feed_row_badges() {
    let mut vm = SearchResultsInspectorPageVm::new("libre");
    vm.replace_index_results(IndexSearchResultRows {
        artists: vec![
            index_feed_page(
                4_000_000_000,
                "34bf795f-f551-4db1-8877-ad19aa5f89c8",
                "Sir Libre Records",
            ),
            index_feed_page(
                4_000_000_001,
                "c8ec3dba-ed28-5662-9509-094d769f9597",
                "Kolomona Myer - AKA Sir Libre",
            ),
        ],
        ..no_index_rows()
    });
    assert_eq!(vm.tab(), SearchResultsTab::Artists);
    let mut lookup = RecordedLibrary::default();
    lookup
        .artist_feeds
        .insert("c8ec3dba-ed28-5662-9509-094d769f9597".to_owned());
    let rows = rows(&vm, &lookup);
    assert_eq!(rows[0].badge, Some(SearchRowBadge::Feed));
    assert_eq!(rows[1].badge, Some(SearchRowBadge::Artist));
    for row in &rows {
        assert!(!row.label.contains("Publisher"));
        assert!(!row.a11y_label.contains("Publisher"));
        assert!(!row
            .badge
            .is_some_and(|badge| badge.label().contains("Publisher")));
        assert_eq!(row.state_word, None);
    }
}

/// R10-06, the merge part: one feed GUID gives one row, and the Library
/// row stays.
#[test]
fn adr_0084_search_list_r10_06_one_row_for_each_feed_guid() {
    use crate::view_models::artist_identity::{name_feed_matches, NameFeedEntry};

    let facts = super::ArtistSearchFacts {
        matches: name_feed_matches([NameFeedEntry {
            feed_guid: "haleen-guid".to_owned(),
            title: Some("Haleen".to_owned()),
            keys: ["haleen".to_owned()].into_iter().collect(),
        }]),
        name_level_art: BTreeMap::new(),
        feed_images: BTreeMap::new(),
    };
    let local = [local_track(10, 1, "Feed", "One", "Haleen")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks("haleen", &local, &facts);
    vm.set_tab(SearchResultsTab::Artists);
    vm.replace_index_results(IndexSearchResultRows {
        artists: vec![index_feed_page(4_000_000_000, "haleen-guid", "Haleen")],
        ..no_index_rows()
    });
    let rows = rows(&vm, &RecordedLibrary::default());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "artist-feed:haleen-guid");
}

/// The row of a Library artist feed carries the stored channel image of
/// the feed (ADR 0084 Decision 12), the same image that the Artists list
/// shows.
#[test]
fn adr_0084_search_list_library_artist_feed_row_carries_its_image() {
    use crate::view_models::artist_identity::{name_feed_matches, NameFeedEntry};

    let matches = name_feed_matches([NameFeedEntry {
        feed_guid: "haleen-guid".to_owned(),
        title: Some("Haleen".to_owned()),
        keys: ["haleen".to_owned()].into_iter().collect(),
    }]);
    let facts = super::ArtistSearchFacts {
        matches,
        name_level_art: BTreeMap::new(),
        feed_images: BTreeMap::from([(
            "haleen-guid".to_owned(),
            "https://example.test/haleen.jpg".to_owned(),
        )]),
    };
    let local = [local_track(10, 1, "Feed", "One", "Haleen")];
    let artists = library_artist_rows(&local, &facts);
    assert_eq!(artists.len(), 1);
    assert_eq!(artists[0].id, "artist-feed:haleen-guid");
    assert_eq!(
        artists[0].thumbnail_href.as_deref(),
        Some("https://example.test/haleen.jpg")
    );
}

/// R10-07: `SearchResultTarget::from_row_id` gives the correct target for
/// each id form.
#[test]
fn adr_0084_search_list_r10_07_row_id_targets() {
    let target = SearchResultTarget::from_row_id;
    assert_eq!(
        target(SearchResultsTab::Releases, "index-feed:g1"),
        Some(SearchResultTarget::Release(ReleaseKey::FeedGuid(
            "g1".to_owned()
        )))
    );
    assert_eq!(
        target(SearchResultsTab::Releases, "42"),
        Some(SearchResultTarget::Release(ReleaseKey::LibraryFeed(42)))
    );
    assert_eq!(
        target(SearchResultsTab::Tracks, "index-track:g1:t1"),
        Some(SearchResultTarget::Track(track_key("g1", "t1")))
    );
    assert_eq!(
        target(SearchResultsTab::Tracks, "index-track-guid:t1"),
        Some(SearchResultTarget::Track(
            TrackKey::from_index_track_target(None, "t1")
        ))
    );
    assert_eq!(
        target(SearchResultsTab::Tracks, "10"),
        Some(SearchResultTarget::LibraryTrack(10))
    );
    assert_eq!(
        target(SearchResultsTab::Artists, "index-feed-page:g9"),
        Some(SearchResultTarget::FeedPage("g9".to_owned()))
    );
    assert_eq!(
        target(SearchResultsTab::Artists, "artist-feed:g8"),
        Some(SearchResultTarget::FeedPage("g8".to_owned()))
    );
    assert_eq!(
        target(SearchResultsTab::Artists, "index-artist:Survival Guide"),
        Some(SearchResultTarget::NameMatches("Survival Guide".to_owned()))
    );
    assert_eq!(
        target(SearchResultsTab::Artists, "library-artist:Haleen"),
        Some(SearchResultTarget::ArtistByName("Haleen".to_owned()))
    );
    assert_eq!(target(SearchResultsTab::Releases, "nonsense"), None);
    assert_eq!(target(SearchResultsTab::Artists, "a1"), None);
}

fn selectable_page() -> (SearchResultsInspectorPageVm, RecordedLibrary) {
    let local = [local_guid_track(10, 7, "g-library", "i1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "q",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    vm.replace_index_results(IndexSearchResultRows {
        artists: vec![(1_000_000_000, artist(1, "Haleen"))],
        feeds: vec![index_release(2_000_000_000, "g-index", "Index Release")],
        tracks: vec![
            index_track(3_000_000_000, Some("g-index"), "t1", "Index Track"),
            index_track(3_000_000_001, None, "t2", "No Feed"),
        ],
    });
    let lookup = RecordedLibrary::default()
        .with_release("g-library", 7, true, "In Library")
        .with_track("g-library", "i1", 10, true);
    (vm, lookup)
}

/// R10-09: in `Normal`, no row shows a checkbox. In `Selecting`, each
/// Releases row and each Tracks row with a key shows one. The Artists tab
/// gives no header action.
#[test]
fn adr_0084_search_list_r10_09_checkboxes_follow_the_mode() {
    let (mut vm, lookup) = selectable_page();
    assert!(rows(&vm, &lookup)
        .iter()
        .all(|row| !row.selection.checkbox_visible));
    let action = vm
        .header_action(&lookup)
        .expect("Releases has a Select button");
    assert_eq!(action.label, "Select");

    vm.toggle_mode();
    assert_eq!(
        vm.header_action(&lookup).map(|action| action.label),
        Some("Done")
    );
    assert!(rows(&vm, &lookup)
        .iter()
        .all(|row| row.selection.checkbox_visible));

    vm.set_tab(SearchResultsTab::Tracks);
    vm.toggle_mode();
    let track_rows = rows(&vm, &lookup);
    assert_eq!(
        track_rows
            .iter()
            .map(|row| row.selection.checkbox_visible)
            .collect::<Vec<_>>(),
        [true, true, false],
        "an Index track with no feed GUID has no checkbox"
    );

    vm.set_tab(SearchResultsTab::Artists);
    assert!(vm.header_action(&lookup).is_none());
    assert!(vm.selection_rows(&lookup).is_none());
    assert!(rows(&vm, &lookup)
        .iter()
        .all(|row| !row.selection.checkbox_visible));
}

/// R10-10: a tab change clears the selection and gives `Normal`. The
/// arrival of the Index rows keeps a selected Library row selected.
#[test]
fn adr_0084_search_list_r10_10_tab_change_clears_and_index_arrival_keeps() {
    let local = [local_guid_track(10, 7, "g-library", "i1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "q",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    vm.mark_index_loading();
    let lookup = RecordedLibrary::default().with_release("g-library", 7, true, "In Library");
    let library_key = DetailRowKey::Release(ReleaseKey::FeedGuid("g-library".to_owned()));
    vm.toggle_row(&library_key);
    assert_eq!(vm.selection().selected_count(), 1);

    vm.replace_index_results(IndexSearchResultRows {
        feeds: vec![
            index_release(2_000_000_000, "g-library", "Library Release"),
            index_release(2_000_000_001, "g-index", "Index Release"),
        ],
        ..no_index_rows()
    });
    assert!(vm.selection().mode().is_selecting());
    assert_eq!(vm.selection().selected_count(), 1);
    assert!(rows(&vm, &lookup)[0].selection.is_marked());

    vm.set_tab(SearchResultsTab::Tracks);
    assert!(!vm.selection().mode().is_selecting());
    assert_eq!(vm.selection().selected_count(), 0);
}

/// R10-11: the two search bars give the counts of packet 018 R18-06 for
/// recorded Library rows and Index rows.
#[test]
fn adr_0084_search_list_r10_11_bar_counts() {
    let (mut vm, lookup) = selectable_page();
    vm.select_all(&lookup);
    let bar = vm.action_bar(&lookup).expect("Selecting shows a bar");
    let label = |kind| bar.action(kind).map(|action| action.label.clone());
    assert_eq!(
        label(SelectionActionKind::Download).as_deref(),
        Some("Download 1 release")
    );
    assert_eq!(
        label(SelectionActionKind::AddToPlaylist).as_deref(),
        Some("Add 1 release to playlist")
    );
    assert_eq!(
        label(SelectionActionKind::Remove).as_deref(),
        Some("Remove 1 release\u{2026}")
    );

    vm.set_tab(SearchResultsTab::Tracks);
    vm.select_all(&lookup);
    let bar = vm.action_bar(&lookup).expect("Selecting shows a bar");
    let label = |kind| bar.action(kind).map(|action| action.label.clone());
    assert_eq!(
        label(SelectionActionKind::Download).as_deref(),
        Some("Download 1 track")
    );
    assert_eq!(
        label(SelectionActionKind::AddToPlaylist).as_deref(),
        Some("Add 1 track to playlist")
    );
    assert_eq!(
        label(SelectionActionKind::Remove).as_deref(),
        Some("Remove 1 track\u{2026}")
    );
}

/// R10-13: a result for query A after the location changed to query B
/// changes neither the rows nor the selection of query B.
#[test]
fn adr_0084_search_list_r10_13_stale_result_changes_nothing() {
    let local = [local_guid_track(10, 7, "g-library", "i1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "query b",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    let lookup = RecordedLibrary::default();
    let key = DetailRowKey::Release(ReleaseKey::FeedGuid("g-library".to_owned()));
    vm.toggle_row(&key);
    let rows_before = rows(&vm, &lookup);

    let applied = vm.replace_index_results_for_query(
        "query a",
        IndexSearchResultRows {
            feeds: vec![index_release(2_000_000_000, "g-other", "Other")],
            ..no_index_rows()
        },
    );
    assert!(!applied);
    assert_eq!(rows(&vm, &lookup), rows_before);
    assert_eq!(vm.selection().selected_count(), 1);
    assert!(vm.selection().mode().is_selecting());

    assert!(vm.replace_index_results_for_query("query b", no_index_rows()));
}

/// R10-14: this packet does not change the name row label. Packet 017
/// gave the label "Releases that name <name>".
#[test]
fn adr_0084_search_list_r10_14_name_row_label_does_not_change() {
    let local = [local_track(10, 1, "Feed", "One", "Survival Guide")];
    let vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "survival guide",
        &local,
        &name_level_facts(&["survival guide"]),
    );
    let mut vm = vm;
    vm.set_tab(SearchResultsTab::Artists);
    let rows = rows(&vm, &RecordedLibrary::default());
    assert_eq!(rows[0].label, "Releases that name Survival Guide");
    assert_eq!(rows[0].badge, None, "a name row has no badge");
}

/// A click in `Normal` opens the page. In `Selecting`, a plain click
/// changes the selection and opens nothing.
#[test]
fn adr_0084_search_list_click_opens_in_normal_and_selects_in_selecting() {
    let (mut vm, lookup) = selectable_page();
    let key = DetailRowKey::Release(ReleaseKey::FeedGuid("g-index".to_owned()));
    assert!(vm.click_row(&key, DetailRowClick::Plain, &lookup));
    vm.toggle_mode();
    assert!(!vm.click_row(&key, DetailRowClick::Plain, &lookup));
    assert_eq!(vm.selection().selected_count(), 1);
}

/// Packet 010 decision D13: the empty state names the tab and the query.
/// It has no clear-filter action.
#[test]
fn adr_0084_search_list_empty_state_names_the_tab_and_the_query() {
    let vm = SearchResultsInspectorPageVm::new("bluegrass");
    let empty = vm.empty_state().expect("no row gives an empty state");
    assert_eq!(empty.title, "No releases");
    assert_eq!(empty.secondary, "No releases matched \"bluegrass\".");
}

#[test]
fn index_loading_suppresses_empty_state() {
    let mut vm = SearchResultsInspectorPageVm::new("ambient");
    vm.mark_index_loading();
    assert!(vm.is_index_loading());
    assert!(vm.empty_state().is_none());
}

#[test]
fn index_results_auto_select_first_populated_tab_until_user_selects_tab() {
    let mut vm = SearchResultsInspectorPageVm::new("delta");
    vm.replace_index_results(IndexSearchResultRows {
        tracks: vec![index_track(3_000_000_000, Some("g"), "t", "Remote Track")],
        ..no_index_rows()
    });
    assert_eq!(vm.tab(), SearchResultsTab::Tracks);
    assert!(vm.empty_state().is_none());

    vm.set_tab(SearchResultsTab::Artists);
    vm.replace_index_results(IndexSearchResultRows {
        feeds: vec![index_release(2_000_000_000, "g", "Second Feed")],
        ..no_index_rows()
    });
    assert_eq!(
        vm.tab(),
        SearchResultsTab::Artists,
        "explicit user tab selection must not be overwritten by remote refresh"
    );
}

/// ADR 0075 packet 047: the search page still reads a row's cached label
/// for an opening row's placeholder title.
#[test]
fn row_label_uses_cached_result_rows() {
    let mut vm = SearchResultsInspectorPageVm::new("delta");
    vm.replace_index_results(IndexSearchResultRows {
        feeds: vec![index_release(2_000_000_000, "feed-guid", "Remote Feed")],
        tracks: vec![index_track(
            3_000_000_000,
            Some("feed-guid"),
            "track-guid",
            "Remote Track",
        )],
        ..no_index_rows()
    });
    assert_eq!(
        vm.row_label(SearchResultsTab::Releases, "index-feed:feed-guid")
            .as_deref(),
        Some("Remote Feed")
    );
    assert_eq!(
        vm.row_label(SearchResultsTab::Tracks, "index-track:feed-guid:track-guid")
            .as_deref(),
        Some("Remote Track")
    );
}

#[test]
fn visible_thumbnail_hrefs_follow_the_active_tab() {
    let mut vm = SearchResultsInspectorPageVm::new("survival guide");
    vm.replace_index_results(IndexSearchResultRows {
        feeds: vec![(
            2_000_000_000,
            index_release(2_000_000_000, "g", "deathdreams")
                .1
                .with_thumbnail_href("https://example.test/deathdreams.jpg"),
        )],
        ..no_index_rows()
    });
    assert_eq!(
        vm.thumbnail_hrefs(),
        vec!["https://example.test/deathdreams.jpg".to_string()]
    );
}

/// A Library release row carries the album image of its feed, the same
/// image that the Releases list of the Library shows.
#[test]
fn adr_0084_search_list_library_release_row_carries_its_image() {
    let mut track = local_track(10, 7, "The Heycitizen Experience", "Opening", "HeyCitizen");
    track.album_image_href = Some("https://example.test/cover.jpg".to_owned());
    let vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "heycitizen",
        &[track],
        &super::ArtistSearchFacts::default(),
    );
    let releases = vm.releases().rows();
    assert_eq!(releases.len(), 1);
    assert_eq!(
        releases[0].thumbnail_href.as_deref(),
        Some("https://example.test/cover.jpg")
    );
}

/// A Library track row carries its own image. With no image of its own, it
/// falls back to the album image of its feed.
#[test]
fn adr_0084_search_list_library_track_row_carries_its_image() {
    let mut own_image = local_track(10, 7, "The Heycitizen Experience", "Opening", "HeyCitizen");
    own_image.item_guid = "opening".to_owned();
    own_image.track_image_href = Some("https://example.test/track.jpg".to_owned());
    own_image.album_image_href = Some("https://example.test/cover.jpg".to_owned());
    let mut album_only = local_track(11, 7, "The Heycitizen Experience", "Side B", "HeyCitizen");
    album_only.item_guid = "side-b".to_owned();
    album_only.album_image_href = Some("https://example.test/cover.jpg".to_owned());
    let vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "heycitizen",
        &[own_image, album_only],
        &super::ArtistSearchFacts::default(),
    );
    let tracks = vm.tracks().rows();
    assert_eq!(tracks.len(), 2);
    let track_image = tracks
        .iter()
        .find(|row| row.id == "10")
        .expect("track 10 is a row");
    assert_eq!(
        track_image.thumbnail_href.as_deref(),
        Some("https://example.test/track.jpg"),
        "the track's own image wins over the album image"
    );
    let album_image = tracks
        .iter()
        .find(|row| row.id == "11")
        .expect("track 11 is a row");
    assert_eq!(
        album_image.thumbnail_href.as_deref(),
        Some("https://example.test/cover.jpg"),
        "with no image of its own, the track row falls back to the album image"
    );
}

#[test]
fn local_library_tracks_populate_ready_rows() {
    let rows_in = [
        local_track(10, 1, "The Heycitizen Experience", "Opening", "HeyCitizen"),
        local_track(
            11,
            2,
            "HeyCitizen's Lo-Fi Hip-Hop Beats",
            "Side B",
            "HeyCitizen",
        ),
    ];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "heycitizen",
        &rows_in,
        &name_level_facts(&["heycitizen"]),
    );
    assert!(vm.empty_state().is_none());
    assert_eq!(vm.artists().window().total(), 1);
    assert_eq!(vm.releases().window().total(), 2);
    assert_eq!(vm.tracks().window().total(), 2);

    let lookup = RecordedLibrary::default();
    let release_rows = rows(&vm, &lookup);
    assert_eq!(
        release_rows[0].a11y_label,
        "Release: The Heycitizen Experience"
    );
    vm.set_tab(SearchResultsTab::Tracks);
    let track_rows = rows(&vm, &lookup);
    assert_eq!(track_rows[0].label, "Opening");
    assert_eq!(track_rows[0].secondary_text, "HeyCitizen");
}

#[test]
fn index_error_surfaces_when_the_tab_has_no_rows() {
    let mut vm = SearchResultsInspectorPageVm::new("field recordings");
    vm.mark_index_loading();
    vm.set_index_error(
        &crate::application::CommandError::Query("connection refused".into()),
        "http://127.0.0.1:9",
        std::time::SystemTime::UNIX_EPOCH,
    );
    let empty = vm.empty_state().expect("index error should surface");
    assert_eq!(empty.title, "MusicIndex search unavailable");
    assert!(empty
        .secondary
        .contains("App could not get search results from MusicIndex."));
}

/// Walk of 2026-10-21: an HTTP `400` reply to the search is a query error.
/// The Library rows stay, and the Index part shows a note that names the
/// query. The note names no endpoint and no connection. "Show details"
/// keeps the redacted reply of the server. Packet 028 D5 gives a note for
/// a connection error too: see
/// `adr_0084_search_open_defects_r28_06_unavailable_index_gives_a_note_after_the_library_rows`.
#[test]
fn adr_0084_search_list_rejected_query_shows_a_note_with_the_library_rows() {
    use super::SearchFailureAction;
    let local = [local_guid_track(10, 7, "g-library", "i1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "haleen'",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    vm.mark_index_loading();
    vm.set_index_error(
        &crate::application::CommandError::QueryRejected(
            "feed search failed: GET failed with HTTP 400 Bad Request: {\"error\":\"invalid search query: SQLite error: fts5: syntax error near \\\"'\\\"\"} (https://user:secret@index.test/v1/search)".into(),
        ),
        "https://user:secret@index.test",
        std::time::SystemTime::UNIX_EPOCH,
    );
    let lookup = RecordedLibrary::default();
    assert_eq!(rows(&vm, &lookup).len(), 1, "the Library row stays");
    assert!(vm.empty_state().is_none());
    let notice = vm.index_notice().expect("the Index part shows a note");
    assert_eq!(notice.title, "MusicIndex did not accept the query");
    assert_eq!(
        notice.secondary,
        "MusicIndex did not accept the query \"haleen'\". Change the query, then search again."
    );
    for forbidden in ["Edit endpoint", "Check your connection", "unavailable"] {
        assert!(!notice.secondary.contains(forbidden), "{forbidden}");
    }
    let report = vm
        .activate_failure_action(SearchFailureAction::CopyReport)
        .expect("the report stays available");
    assert!(report.contains("fts5: syntax error"));
    assert!(!report.contains("secret"));

    let mut no_rows = SearchResultsInspectorPageVm::new("haleen'");
    no_rows.set_index_error(
        &crate::application::CommandError::QueryRejected("HTTP 400".into()),
        "https://index.test",
        std::time::SystemTime::UNIX_EPOCH,
    );
    assert!(
        no_rows.index_notice().is_none(),
        "a tab with no rows shows the empty state"
    );
    let empty = no_rows
        .empty_state()
        .expect("the empty state shows the note");
    assert_eq!(empty.title, "MusicIndex did not accept the query");
}

/// R28-06 (packet 028 D5): with Library rows and an unavailable
/// `MusicIndex`, the view model gives the rows and a failure note after
/// them. The note names `MusicIndex` and keeps "Show details". With no
/// Library rows, the view model gives the present empty state and no note.
#[test]
fn adr_0084_search_open_defects_r28_06_unavailable_index_gives_a_note_after_the_library_rows() {
    use super::SearchFailureAction;
    let local = [local_guid_track(10, 7, "g-library", "i1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "hey",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    vm.mark_index_loading();
    vm.set_index_error(
        &crate::application::CommandError::Query("connection refused".into()),
        "http://127.0.0.1:9",
        std::time::SystemTime::UNIX_EPOCH,
    );
    let lookup = RecordedLibrary::default();
    assert_eq!(rows(&vm, &lookup).len(), 1, "the Library row stays");
    assert!(vm.empty_state().is_none());
    let notice = vm.index_notice().expect("the Index part shows a note");
    assert_eq!(notice.title, "MusicIndex search unavailable");
    assert!(notice
        .secondary
        .contains("App could not get search results from MusicIndex."));
    let failure = notice.failure.as_ref().expect("the note keeps its report");
    assert_eq!(
        failure.action(SearchFailureAction::ToggleDetails).label,
        "Show details"
    );

    let mut no_rows = SearchResultsInspectorPageVm::new("hey");
    no_rows.mark_index_loading();
    no_rows.set_index_error(
        &crate::application::CommandError::Query("connection refused".into()),
        "http://127.0.0.1:9",
        std::time::SystemTime::UNIX_EPOCH,
    );
    assert!(no_rows.index_notice().is_none());
    assert_eq!(
        no_rows.empty_state().map(|empty| empty.title.as_str()),
        Some("MusicIndex search unavailable")
    );
}

/// R28-03 (packet 028 D2): the view model gives the index of the focused
/// row among all the rows of the active tab. A row with no key counts,
/// because it is a child of the scroll container too. A tab change and the
/// Artists tab give no index.
#[test]
fn adr_0084_search_open_defects_r28_03_focused_row_index_counts_each_row() {
    use crate::view_models::list_selection::FocusStep;
    let local = [local_guid_track(10, 7, "g-library", "i1", "One")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks(
        "q",
        &local,
        &super::ArtistSearchFacts::default(),
    );
    vm.replace_index_results(IndexSearchResultRows {
        tracks: vec![
            index_track(3_000_000_000, None, "t2", "No Feed"),
            index_track(3_000_000_001, Some("g-index"), "t1", "Index Track"),
        ],
        ..no_index_rows()
    });
    let lookup = RecordedLibrary::default().with_track("g-library", "i1", 10, true);
    vm.set_tab(SearchResultsTab::Tracks);
    assert_eq!(vm.focused_row_index(), None);

    vm.move_focus(FocusStep::Home, &lookup);
    assert_eq!(vm.focused_row_index(), Some(0));
    vm.move_focus(FocusStep::Down, &lookup);
    assert_eq!(
        vm.focused_row_index(),
        Some(2),
        "the row with no key is the child at index 1"
    );

    vm.set_tab(SearchResultsTab::Releases);
    assert_eq!(vm.focused_row_index(), None);
    vm.set_tab(SearchResultsTab::Artists);
    assert_eq!(vm.focused_row_index(), None);
}

#[test]
fn adr_0066_search_failure_disclosure_resets_on_retry() {
    use super::SearchFailureAction;
    let mut vm = SearchResultsInspectorPageVm::new("runtime-retry");
    vm.set_index_error(
        &crate::application::CommandError::Query("connection refused".into()),
        "http://127.0.0.1:9",
        std::time::SystemTime::UNIX_EPOCH,
    );
    let report = vm
        .activate_failure_action(SearchFailureAction::CopyReport)
        .unwrap();
    vm.activate_failure_action(SearchFailureAction::ToggleDetails);
    assert_eq!(
        vm.empty_state()
            .unwrap()
            .failure
            .as_ref()
            .unwrap()
            .visible_report(),
        Some(report.as_str())
    );
    vm.mark_index_loading();
    assert!(vm.empty_state().is_none());
    assert_eq!(
        vm.activate_failure_action(SearchFailureAction::CopyReport),
        None
    );
    vm.set_index_error(
        &crate::application::CommandError::Query("another failure".into()),
        "http://127.0.0.1:9",
        std::time::SystemTime::UNIX_EPOCH,
    );
    assert!(vm
        .empty_state()
        .unwrap()
        .failure
        .as_ref()
        .unwrap()
        .visible_report()
        .is_none());
}

/// ADR 0075 packet 047, Required Change 3: the loading state names the
/// entity kind, and it carries no feed or track detail.
#[test]
fn index_detail_display_loading_names_the_kind_and_carries_no_detail() {
    let detail =
        super::IndexDetailDisplay::loading(IndexDetailKind::Release, "feed-guid", "Monster");

    assert_eq!(detail.kind, IndexDetailKind::Release);
    assert_eq!(detail.id, "feed-guid");
    assert!(detail.secondary_text.contains("release"));
    assert!(detail.feed.is_none());
    assert!(detail.track.is_none());
}

/// The failed state reports the already-readable detail the app layer
/// supplied. It carries no transport error of its own.
#[test]
fn index_detail_display_failed_reports_the_supplied_detail() {
    let detail = super::IndexDetailDisplay::failed(
        IndexDetailKind::Track,
        "track-guid",
        "Monster",
        "connection refused",
    );

    assert_eq!(detail.kind, IndexDetailKind::Track);
    assert!(detail.secondary_text.contains("connection refused"));
    assert!(detail.track.is_none());
}

/// The loaded state carries the fetched feed or track, and it reports no
/// stale label or secondary text of its own; the caller reads the
/// attached view.
#[test]
fn index_detail_display_loaded_feed_and_track_carry_the_fetched_view() {
    let feed_detail = super::IndexDetailDisplay::loaded_feed(
        FeedView {
            title: Some("Monster".to_string()),
            ..FeedView::default()
        },
        "feed-guid",
    );
    assert_eq!(feed_detail.kind, IndexDetailKind::Release);
    assert_eq!(
        feed_detail
            .feed
            .as_ref()
            .and_then(|feed| feed.title.clone()),
        Some("Monster".to_string())
    );

    let track_detail = super::IndexDetailDisplay::loaded_track(remote_track(), "track-guid");
    assert_eq!(track_detail.kind, IndexDetailKind::Track);
    assert_eq!(
        track_detail.track.as_ref().map(|track| track.track_number),
        Some(Some(7))
    );
}

/// The artist facts of a Library with a name level release for each of
/// `keys`, and no artist feed.
fn name_level_facts(keys: &[&str]) -> super::ArtistSearchFacts {
    super::ArtistSearchFacts {
        matches: crate::view_models::artist_identity::NameFeedMatches::default(),
        name_level_art: keys
            .iter()
            .map(|key| ((*key).to_owned(), artist_entity_color()))
            .collect(),
        feed_images: BTreeMap::new(),
    }
}

/// The logo frame art of a name with no Library image.
fn artist_entity_color() -> CollectionArtVm {
    CollectionArtVm::EntityColor(CollectionKind::Artist)
}

/// R28-04 (packet 028 D3): a Library name row of search carries the logo
/// frame art of its Library releases, the art of its Artists list row. It
/// shows no thumbnail, and an Index row of the same name does not change
/// that art.
#[test]
fn adr_0084_search_open_defects_r28_04_library_name_row_carries_the_artists_list_art() {
    use crate::view_models::library::name_level_art;
    use crate::view_models::name_level_page::NameLevelLibraryRelease;

    let release = |title: &str, image: &str, newest: i64| NameLevelLibraryRelease {
        release_key: None,
        feed_guid: None,
        title: title.to_owned(),
        image_url: Some(image.to_owned()),
        artist_text: "Matt Finlay".to_owned(),
        kind: crate::view_models::release_kind::ReleaseKind::Album,
        newest_date: Some(newest),
        in_library: true,
    };
    let members = [
        release("Old", "https://example.test/old.png", 1),
        release("New", "https://example.test/new.png", 2),
    ];
    let art = name_level_art(&members);
    assert_eq!(
        art,
        CollectionArtVm::resolve(
            None,
            [
                "https://example.test/new.png",
                "https://example.test/old.png"
            ],
            CollectionKind::Artist,
        ),
        "the newest image comes first"
    );
    let facts = super::ArtistSearchFacts {
        matches: crate::view_models::artist_identity::NameFeedMatches::default(),
        name_level_art: BTreeMap::from([("matt finlay".to_owned(), art.clone())]),
        feed_images: BTreeMap::new(),
    };
    let local = [local_track(10, 1, "Feed", "One", "Matt Finlay")];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks("q", &local, &facts);
    vm.replace_index_results(IndexSearchResultRows {
        artists: vec![(
            1_000_000_000,
            ArtistResultDisplay::new("index-artist:Matt Finlay", "Matt Finlay")
                .with_thumbnail_href("https://example.test/index.png"),
        )],
        ..no_index_rows()
    });
    vm.set_tab(SearchResultsTab::Artists);
    let visible = rows(&vm, &RecordedLibrary::default());
    assert_eq!(
        visible.len(),
        1,
        "the Index name row merges into the Library row"
    );
    assert_eq!(visible[0].id, "library-artist:Matt Finlay");
    assert_eq!(visible[0].art, Some(art));
    assert_eq!(visible[0].thumbnail_href, None);
}

/// The artist rows of the Library search for `rows` with `facts`.
fn library_artist_rows(
    rows: &[crate::db::TrackRow],
    facts: &super::ArtistSearchFacts,
) -> Vec<super::ArtistResultDisplay> {
    let vm = SearchResultsInspectorPageVm::from_local_library_tracks("q", rows, facts);
    vm.artists().rows()
}

/// R17-15: a search artist row of a `NoFeed` name with Library members has
/// the label "Releases that name Survival Guide", an equal accessibility
/// label, and no secondary text.
#[test]
fn adr_0084_name_level_r17_15_library_name_row_text() {
    let rows = [
        local_track(10, 1, "Feed", "One", "Survival Guide"),
        local_track(11, 1, "Feed", "Two", "survival  guide"),
    ];
    let artists = library_artist_rows(&rows, &name_level_facts(&["survival guide"]));
    assert_eq!(artists.len(), 1, "two texts of one key give one row");
    assert_eq!(artists[0].label, "Releases that name Survival Guide");
    assert_eq!(artists[0].a11y_label, artists[0].label);
    assert_eq!(artists[0].secondary_text, "");
    assert_eq!(artists[0].id, "library-artist:Survival Guide");
}

/// R17-28: a `MatchedFeed` name gives no name row. It gives one feed row
/// with the feed title, its accessibility label and the feed page target.
/// Two names that match one feed give one feed row.
#[test]
fn adr_0084_name_level_r17_28_matched_name_gives_the_feed_row() {
    use crate::view_models::artist_identity::{name_feed_matches, NameFeedEntry};

    let matches = name_feed_matches([NameFeedEntry {
        feed_guid: "haleen-guid".to_owned(),
        title: Some("Haleen".to_owned()),
        keys: ["haleen".to_owned(), "haleen music".to_owned()]
            .into_iter()
            .collect(),
    }]);
    let facts = super::ArtistSearchFacts {
        matches,
        name_level_art: BTreeMap::from([("haleen".to_owned(), artist_entity_color())]),
        feed_images: BTreeMap::new(),
    };
    let rows = [
        local_track(10, 1, "Feed", "One", "Haleen"),
        local_track(11, 2, "Feed", "Two", "Haleen Music"),
    ];
    let artists = library_artist_rows(&rows, &facts);
    assert_eq!(artists.len(), 1);
    assert_eq!(artists[0].id, "artist-feed:haleen-guid");
    assert_eq!(artists[0].label, "Haleen");
    assert_eq!(artists[0].a11y_label, "Open the artist page Haleen");
}

/// R17-29, the Library part: a `NoFeed` name with no Library member of the
/// name level gives no row.
#[test]
fn adr_0084_name_level_r17_29_name_with_no_member_gives_no_row() {
    let rows = [local_track(10, 1, "Feed", "One", "Nobody")];
    assert!(library_artist_rows(&rows, &name_level_facts(&[])).is_empty());
}

/// R17-14, the search part: the two name row ids open the name level page
/// of the key of their text for `NoFeed`, and the feed page for
/// `MatchedFeed`. A feed row id opens its feed page.
#[test]
fn adr_0084_name_level_r17_14_search_row_destinations() {
    use crate::view_models::artist_identity::{
        name_feed_matches, ArtistNameDestination, ArtistNameKey, NameFeedEntry,
    };

    let matches = name_feed_matches([NameFeedEntry {
        feed_guid: "haleen-guid".to_owned(),
        title: Some("Haleen".to_owned()),
        keys: ["haleen".to_owned()].into_iter().collect(),
    }]);
    let destination = |id: &str| {
        SearchResultTarget::from_row_id(SearchResultsTab::Artists, id)?.artist_destination(&matches)
    };
    for prefix in ["index-artist:", "library-artist:"] {
        assert_eq!(
            destination(&format!("{prefix}Survival Guide")),
            Some(ArtistNameDestination::NameLevel(ArtistNameKey::from_text(
                "Survival Guide"
            )))
        );
        assert_eq!(
            destination(&format!("{prefix}HALEEN")),
            Some(ArtistNameDestination::FeedPage("haleen-guid".to_owned()))
        );
    }
    assert_eq!(
        destination("artist-feed:haleen-guid"),
        Some(ArtistNameDestination::FeedPage("haleen-guid".to_owned()))
    );
    assert_eq!(destination("a1"), None);
}
