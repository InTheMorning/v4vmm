use crate::db::TrackRow;
use crate::runtime::paged_list_vm::RowSlot;
use crate::view_models::workspace::ContentFilter;
use crate::views::{FeedView, TrackView};

use super::{
    ArtistResultDisplay, FeedResultDisplay, IndexDetailKind, IndexSearchResultRows,
    SearchResultItemId, SearchResultOrigin, SearchResultsInspectorPageVm, SearchResultsPagedTab,
    SearchResultsTab, TrackResultDisplay,
};

fn artist(id: SearchResultItemId, label: &str) -> ArtistResultDisplay {
    ArtistResultDisplay::new(id.to_string(), label, SearchResultOrigin::Index)
}

fn feed(id: SearchResultItemId, label: &str) -> FeedResultDisplay {
    FeedResultDisplay::new(id.to_string(), label, SearchResultOrigin::Index)
}

fn track(id: SearchResultItemId, label: &str) -> TrackResultDisplay {
    TrackResultDisplay::new(id.to_string(), label, SearchResultOrigin::Index)
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

#[test]
fn tab_and_filter_state_are_independent() {
    let mut vm = SearchResultsInspectorPageVm::new("jazz")
        .with_artists(SearchResultsPagedTab::new(vec![1], Vec::new(), vec![1]))
        .with_feeds(SearchResultsPagedTab::new(vec![2], vec![2], Vec::new()));

    vm.set_filter(ContentFilter::Library);
    vm.set_tab(SearchResultsTab::Feeds);

    assert_eq!(vm.tab(), SearchResultsTab::Feeds);
    assert_eq!(vm.filter(), ContentFilter::Library);
    assert!(
        !vm.is_empty(SearchResultsTab::Feeds, ContentFilter::Library),
        "tab switch must not reset the selected content filter"
    );
}

#[test]
fn per_tab_paged_windows_operate_independently() {
    let mut vm = SearchResultsInspectorPageVm::new("ambient")
        .with_artists(SearchResultsPagedTab::new(
            vec![10, 11],
            Vec::new(),
            vec![10, 11],
        ))
        .with_feeds(SearchResultsPagedTab::new(vec![20], vec![20], Vec::new()))
        .with_tracks(SearchResultsPagedTab::new(vec![30], Vec::new(), vec![30]));

    let artist_window = vm.artists_mut().window_mut(ContentFilter::All);
    assert!(matches!(artist_window.row(0), RowSlot::Pending(_)));
    let artist_requests = artist_window.drain_requests();
    assert_eq!(artist_requests.len(), 1);
    artist_window.fulfill_page(0, [(10, artist(10, "A")), (11, artist(11, "B"))]);

    assert!(
        vm.feeds_mut()
            .window_mut(ContentFilter::All)
            .drain_requests()
            .is_empty(),
        "reading the artist window must not enqueue feed requests"
    );
    assert!(
        vm.tracks_mut()
            .window_mut(ContentFilter::All)
            .drain_requests()
            .is_empty(),
        "reading the artist window must not enqueue track requests"
    );
    assert!(matches!(
        vm.artists().window(ContentFilter::All).peek_row(1),
        RowSlot::Ready(_)
    ));
}

#[test]
fn empty_state_tracks_active_tab_and_filter() {
    let mut vm = SearchResultsInspectorPageVm::new("noise")
        .with_artists(SearchResultsPagedTab::new(vec![1], Vec::new(), vec![1]))
        .with_feeds(SearchResultsPagedTab::new(vec![2], vec![2], Vec::new()));

    vm.set_tab(SearchResultsTab::Artists);
    vm.set_filter(ContentFilter::Library);
    let empty = vm.empty_state().expect("library artists should be empty");
    assert_eq!(empty.title, "No artists results");
    assert_eq!(
        empty.clear_filter_action_id,
        Some("search-results.clear-filter")
    );

    vm.set_filter(ContentFilter::Index);
    assert!(
        vm.empty_state().is_none(),
        "index artists should have one result"
    );

    vm.set_tab(SearchResultsTab::Feeds);
    assert!(
        vm.empty_state().is_some(),
        "index feeds should be empty for the active filter"
    );
}

#[test]
fn result_display_builders_project_accessible_labels() {
    let artist = ArtistResultDisplay::new("a1", "Alice", SearchResultOrigin::Index)
        .with_secondary_text("3 feeds")
        .with_thumbnail_href("https://example.test/a.png");
    let feed = feed(7, "Morning Show");
    let track = track(9, "Theme");

    assert_eq!(artist.a11y_label, "Artist: Alice");
    assert_eq!(artist.secondary_text, "3 feeds");
    assert_eq!(
        artist.thumbnail_href.as_deref(),
        Some("https://example.test/a.png")
    );
    assert_eq!(feed.a11y_label, "Feed: Morning Show");
    assert_eq!(track.a11y_label, "Track: Theme");
    assert!(SearchResultOrigin::Index.matches_filter(ContentFilter::All));
    assert!(!SearchResultOrigin::Index.matches_filter(ContentFilter::Library));
}

#[test]
fn visible_thumbnail_hrefs_include_index_feed_rows() {
    let mut feeds = SearchResultsPagedTab::empty();
    feeds.replace_index_rows(vec![
        (
            20,
            feed(20, "deathdreams").with_thumbnail_href("https://example.test/deathdreams.jpg"),
        ),
        (
            21,
            feed(21, "Way to Go").with_thumbnail_href("https://example.test/way-to-go.jpg"),
        ),
    ]);
    let vm = SearchResultsInspectorPageVm::new("survival guide").with_feeds(feeds);

    assert_eq!(
        vm.thumbnail_hrefs_for_scope(SearchResultsTab::Feeds, ContentFilter::Index),
        vec![
            "https://example.test/deathdreams.jpg".to_string(),
            "https://example.test/way-to-go.jpg".to_string(),
        ]
    );
}

#[test]
fn track_result_display_can_carry_remote_track_view() {
    let display = track(9, "Theme").with_remote_track(remote_track());
    let remote = display
        .remote_track
        .as_ref()
        .expect("remote Index track detail should stay attached to result row");

    assert_eq!(remote.title.as_deref(), Some("Remote Track"));
    assert_eq!(remote.feed_title.as_deref(), Some("Remote Release"));
    assert_eq!(remote.track_number, Some(7));
    assert_eq!(remote.duration_secs, Some(125));
    assert_eq!(remote.pub_date, Some(1_712_275_200));
    assert_eq!(remote.explicit, Some(true));
    assert_eq!(
        remote.transcript_url.as_deref(),
        Some("https://example.test/transcript.srt")
    );
}

#[test]
fn index_detail_display_preserves_remote_track_view() {
    let row = TrackResultDisplay::new(
        "index-track:feed-guid:track-guid",
        "Remote Track",
        SearchResultOrigin::Index,
    )
    .with_secondary_text("Remote Artist")
    .with_remote_track(remote_track());

    let detail = super::IndexDetailDisplay::track(&row, "feed-guid:track-guid");
    let track = detail
        .track
        .as_ref()
        .expect("Index track detail should preserve rich remote track projection");

    assert_eq!(detail.kind, IndexDetailKind::Track);
    assert_eq!(detail.title, "Remote Track");
    assert_eq!(detail.secondary_text, "Remote Artist");
    assert_eq!(track.feed_title.as_deref(), Some("Remote Release"));
    assert_eq!(track.track_number, Some(7));
    assert_eq!(track.duration_secs, Some(125));
}

#[test]
fn local_library_tracks_populate_ready_artist_feed_and_track_results() {
    let rows = [
        local_track(10, 1, "The Heycitizen Experience", "Opening", "HeyCitizen"),
        local_track(
            11,
            2,
            "HeyCitizen's Lo-Fi Hip-Hop Beats",
            "Side B",
            "HeyCitizen",
        ),
    ];

    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks("heycitizen", &rows);

    assert!(
        vm.empty_state().is_none(),
        "default Artists/All tab should not show empty state when local artist rows exist"
    );
    assert_eq!(vm.artists().window(ContentFilter::All).total(), 1);
    assert_eq!(vm.artists().window(ContentFilter::Library).total(), 1);
    assert_eq!(vm.artists().window(ContentFilter::Index).total(), 0);
    assert_eq!(vm.feeds().window(ContentFilter::All).total(), 2);
    assert_eq!(vm.tracks().window(ContentFilter::All).total(), 2);

    let RowSlot::Ready(artist) = vm.artists().window(ContentFilter::All).peek_row(0) else {
        panic!("local artist row should be preloaded");
    };
    assert_eq!(artist.label, "HeyCitizen");
    assert_eq!(artist.secondary_text, "2 albums - 2 tracks");

    vm.set_tab(SearchResultsTab::Tracks);
    let RowSlot::Ready(track) = vm.tracks().window(ContentFilter::All).peek_row(0) else {
        panic!("local track row should be preloaded");
    };
    assert_eq!(track.label, "Opening");
    assert_eq!(track.secondary_text, "HeyCitizen");
}

#[test]
fn index_loading_suppresses_empty_state_for_index_and_all() {
    let mut vm = SearchResultsInspectorPageVm::new("ambient");

    vm.mark_index_loading();

    assert!(vm.is_index_loading());
    assert!(
        vm.empty_state().is_none(),
        "All filter should keep pending remote search visually pending"
    );

    vm.set_filter(ContentFilter::Index);
    assert!(
        vm.empty_state().is_none(),
        "Index filter should keep pending remote search visually pending"
    );

    vm.set_filter(ContentFilter::Library);
    assert!(
        vm.empty_state().is_some(),
        "Library filter is not waiting on remote Index results"
    );
}

#[test]
fn index_rows_populate_index_and_all_but_not_library() {
    let local_rows = [local_track(
        10,
        1,
        "Local Feed",
        "Local Track",
        "Local Artist",
    )];
    let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks("mix", &local_rows);

    vm.replace_index_results(IndexSearchResultRows {
        artists: vec![(101, artist(101, "Remote Artist"))],
        feeds: vec![(201, feed(201, "Remote Feed"))],
        tracks: vec![(301, track(301, "Remote Track"))],
    });

    assert!(!vm.is_index_loading());
    assert_eq!(vm.artists().window(ContentFilter::Index).total(), 1);
    assert_eq!(vm.artists().window(ContentFilter::All).total(), 2);
    assert_eq!(vm.artists().window(ContentFilter::Library).total(), 1);
    assert_eq!(vm.feeds().window(ContentFilter::Index).total(), 1);
    assert_eq!(vm.feeds().window(ContentFilter::All).total(), 2);
    assert_eq!(vm.feeds().window(ContentFilter::Library).total(), 1);
    assert_eq!(vm.tracks().window(ContentFilter::Index).total(), 1);
    assert_eq!(vm.tracks().window(ContentFilter::All).total(), 2);
    assert_eq!(vm.tracks().window(ContentFilter::Library).total(), 1);

    let RowSlot::Ready(local_artist) = vm.artists().window(ContentFilter::All).peek_row(0) else {
        panic!("local All artist row should stay cached after remote rows arrive");
    };
    assert_eq!(local_artist.label, "Local Artist");

    let RowSlot::Ready(remote_artist) = vm.artists().window(ContentFilter::All).peek_row(1) else {
        panic!("remote All artist row should be cached after replacement");
    };
    assert_eq!(remote_artist.label, "Remote Artist");

    let RowSlot::Ready(remote_feed) = vm.feeds().window(ContentFilter::All).peek_row(1) else {
        panic!("remote All feed row should be cached after replacement");
    };
    assert_eq!(remote_feed.label, "Remote Feed");

    let RowSlot::Ready(remote_track) = vm.tracks().window(ContentFilter::All).peek_row(1) else {
        panic!("remote All track row should be cached after replacement");
    };
    assert_eq!(remote_track.label, "Remote Track");
}

#[test]
fn index_results_auto_select_first_populated_tab_until_user_selects_tab() {
    let mut vm = SearchResultsInspectorPageVm::new("delta");

    vm.replace_index_results(IndexSearchResultRows {
        artists: Vec::new(),
        feeds: vec![(201, feed(201, "Remote Feed"))],
        tracks: Vec::new(),
    });

    assert_eq!(
        vm.tab(),
        SearchResultsTab::Feeds,
        "automatic search landing should move off an empty default tab"
    );
    assert!(
        vm.empty_state().is_none(),
        "populated remote feed rows should be visible after auto-tab selection"
    );

    vm.set_tab(SearchResultsTab::Artists);
    vm.replace_index_results(IndexSearchResultRows {
        artists: Vec::new(),
        feeds: vec![(202, feed(202, "Second Feed"))],
        tracks: Vec::new(),
    });

    assert_eq!(
        vm.tab(),
        SearchResultsTab::Artists,
        "explicit user tab selection must not be overwritten by remote refresh"
    );
}

#[test]
fn index_detail_projection_uses_cached_result_rows() {
    let mut vm = SearchResultsInspectorPageVm::new("delta");
    vm.replace_index_results(IndexSearchResultRows {
        artists: Vec::new(),
        feeds: vec![(
            201,
            FeedResultDisplay::new(
                "index-feed:feed-guid",
                "Remote Feed",
                SearchResultOrigin::Index,
            )
            .with_secondary_text("Remote Artist - 6 tracks")
            .with_remote_feed(FeedView {
                title: Some("Remote Feed".to_string()),
                tracks: vec![TrackView {
                    title: Some("Remote Track".to_string()),
                    ..TrackView::default()
                }],
                ..FeedView::default()
            }),
        )],
        tracks: vec![(
            301,
            TrackResultDisplay::new(
                "index-track:feed-guid:track-guid",
                "Remote Track",
                SearchResultOrigin::Index,
            )
            .with_secondary_text("Remote Artist")
            .with_remote_track(remote_track()),
        )],
    });

    assert_eq!(
        vm.index_feed_label("index-feed:feed-guid").as_deref(),
        Some("Remote Feed")
    );
    let feed = vm.index_feed_detail("index-feed:feed-guid", "feed-guid", "feed-guid");
    assert_eq!(feed.kind, IndexDetailKind::Feed);
    assert_eq!(feed.title, "Remote Feed");
    assert_eq!(feed.secondary_text, "Remote Artist - 6 tracks");
    assert!(
        feed.feed.is_some(),
        "Index feed detail should preserve rich remote feed projection when search fetched it"
    );
    assert_eq!(
        feed.feed.as_ref().map(|feed| feed.tracks.len()),
        Some(1),
        "Index feed detail should preserve remote track rows for release-detail rendering"
    );

    assert_eq!(
        vm.index_track_label("index-track:feed-guid:track-guid")
            .as_deref(),
        Some("Remote Track")
    );
    let track = vm.index_track_detail(
        "index-track:feed-guid:track-guid",
        "feed-guid:track-guid",
        "track-guid",
    );
    assert_eq!(track.kind, IndexDetailKind::Track);
    assert_eq!(track.title, "Remote Track");
    assert_eq!(track.secondary_text, "Remote Artist");
    assert!(
        track.track.is_some(),
        "Index track detail should preserve rich remote track projection when search fetched it"
    );
    assert_eq!(
        track.track.as_ref().and_then(|track| track.track_number),
        Some(7),
        "Index track detail should preserve track fields for shared track-detail rendering"
    );
}

#[test]
fn scoped_empty_state_does_not_mutate_root_tab_or_filter() {
    let mut vm = SearchResultsInspectorPageVm::new("delta");
    vm.set_tab(SearchResultsTab::Artists);
    vm.set_filter(ContentFilter::All);

    let empty = vm
        .empty_state_for_scope(SearchResultsTab::Feeds, ContentFilter::Index)
        .expect("scoped feeds/index render should compute its own empty state");

    assert_eq!(empty.title, "No feeds results");
    assert_eq!(vm.tab(), SearchResultsTab::Artists);
    assert_eq!(vm.filter(), ContentFilter::All);
}

#[test]
fn index_error_surfaces_for_index_when_no_index_rows_exist() {
    let mut vm = SearchResultsInspectorPageVm::new("field recordings");

    vm.mark_index_loading();
    vm.set_filter(ContentFilter::Index);
    vm.set_index_error("Index unavailable", "Try again later.");

    let empty = vm
        .empty_state()
        .expect("index error should surface as empty-state display");
    assert_eq!(empty.title, "Index unavailable");
    assert_eq!(empty.secondary, "Try again later.");
    assert_eq!(empty.clear_filter_action_id, None);
}

#[test]
fn filter_chip_strip_uses_search_inspector_contract() {
    let mut vm = SearchResultsInspectorPageVm::new("beats");
    vm.set_filter(ContentFilter::Index);

    let strip = vm.filter_chip_strip();

    assert_eq!(strip.id, "workspace-search-inspector-filter");
    assert_eq!(strip.selected, ContentFilter::Index);
    assert!(
        strip.narrow_collapse_to_pulldown,
        "search inspector filters should collapse in narrow detail frames"
    );
}

#[test]
fn query_update_refreshes_empty_state_copy() {
    let mut vm = SearchResultsInspectorPageVm::new("old query");

    vm.set_query("new query".to_string());

    assert_eq!(vm.query(), "new query");
    assert_eq!(
        vm.empty_state()
            .expect("empty inspector should expose empty state")
            .secondary,
        "No results matched \"new query\"."
    );
}

#[test]
fn clear_query_refreshes_empty_state_copy() {
    let mut vm = SearchResultsInspectorPageVm::new("old query");

    vm.clear_query();

    assert_eq!(vm.query(), "");
    assert_eq!(
        vm.empty_state()
            .expect("empty inspector should expose empty state")
            .secondary,
        "No results matched \"\"."
    );
}
