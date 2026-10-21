use super::{
    release_download_updates_open_page, resolve_release_page_source, resolve_track_page_source,
    track_download_updates_open_page, track_release_download_updates_open_page, BreadcrumbDisplay,
    BreadcrumbTruncation, ContentFilter, ContentViewMode, ContentViewModeControlDisplay,
    DiscoverView, FilterChipStripDisplay, FilterChipStripWidthClass, FrameNavigationEntry,
    FrameNavigationState, FrameShellDisplay, MusicColumns, MusicDestination, ReleaseKey,
    ReleasePageSource, TrackKey, TrackPageSource, WorkspaceFrameConfig, WorkspaceFrameId,
    WorkspaceFrameKind, WorkspaceFrameState, WorkspaceLayout, WorkspaceLayoutConfig,
    WorkspaceModelError,
};

fn frame(id: u64, kind: WorkspaceFrameKind) -> WorkspaceFrameState {
    WorkspaceFrameState::with_default_title(WorkspaceFrameId::new(id), kind)
}

/// Test helper: the default Music destination root, as a navigation entry.
const fn discover() -> FrameNavigationEntry {
    FrameNavigationEntry::Destination(MusicDestination::Discover)
}

/// Test helper: a `Track` entry with no release, keyed by `id` as its item
/// GUID. Distinct `id` values give distinct, comparable entries.
fn track(id: i64) -> FrameNavigationEntry {
    FrameNavigationEntry::Track(TrackKey::from_index_track_target(None, id.to_string()))
}

/// R6-02 (ADR 0084 Decision 3): the Music opening path gives
/// `Destination(Discover)`, then `DiscoverView(NewReleases)`.
#[test]
fn adr_0084_discover_r6_02_music_opens_on_discover_then_new_releases() {
    let layout = WorkspaceLayout::default_layout();
    let content_list_id = layout
        .frames()
        .iter()
        .find(|frame| frame.kind() == WorkspaceFrameKind::ContentList)
        .map(WorkspaceFrameState::id)
        .expect("the default layout has a content-list frame");
    let nav = layout
        .frame_nav(content_list_id)
        .expect("the content-list frame has navigation");

    assert_eq!(
        nav.path_entries(),
        vec![
            discover(),
            FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases),
        ],
        "Music must open on Discover, then New releases"
    );
}

/// R6-02 (ADR 0084 Decision 3): a sidebar click on Discover gives the
/// same path as the Music opening path. `MusicDestination::default_child_page`
/// is the one place both the opening path and the sidebar click read.
#[test]
fn adr_0084_discover_r6_02_discover_sidebar_click_opens_new_releases() {
    let mut nav =
        FrameNavigationState::new(FrameNavigationEntry::Destination(MusicDestination::Tracks));

    nav.start_path(discover(), MusicDestination::Discover.default_child_page());

    assert_eq!(
        nav.path_entries(),
        vec![
            discover(),
            FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases),
        ],
        "a sidebar click on Discover must give Destination(Discover), then DiscoverView(NewReleases)"
    );
}

/// R6-02: a destination with no default child page starts a path with
/// only its destination.
#[test]
fn adr_0084_discover_r6_02_other_destinations_have_no_default_child_page() {
    for destination in [
        MusicDestination::Releases,
        MusicDestination::Artists,
        MusicDestination::Tracks,
        MusicDestination::NeedsAttention,
        MusicDestination::Playlists,
    ] {
        assert_eq!(
            destination.default_child_page(),
            None,
            "{destination:?} must open no page under it by default"
        );
    }
}

/// R6-03 (ADR 0084 Decision 3): a release opened from "New releases"
/// gives a path with `Destination(Discover)`, `DiscoverView(NewReleases)`
/// and the release, and Back returns to `DiscoverView(NewReleases)`.
#[test]
fn adr_0084_discover_r6_03_release_opens_under_new_releases_and_back_returns() {
    let new_releases = FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases);
    let mut nav = FrameNavigationState::new_with_page(discover(), new_releases.clone());
    let release = FrameNavigationEntry::Release(ReleaseKey::LibraryFeed(42));

    nav.push(release.clone());
    assert_eq!(
        nav.path_entries(),
        vec![discover(), new_releases.clone(), release],
        "a release opened from New releases must give the full Discover path"
    );

    let current = nav
        .go_back()
        .expect("New releases is one back-history entry away");
    assert_eq!(
        *current, new_releases,
        "Back from a release must return to New releases"
    );
}

/// R6-04 (ADR 0084 Decision 3): `DiscoverView` opens a detail page, so
/// the narrow layout shows it.
#[test]
fn adr_0084_discover_r6_04_discover_view_opens_a_detail_page() {
    assert!(
        FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases).opens_detail_page(),
        "DiscoverView must open a detail page, so the narrow layout shows it"
    );
}

#[test]
fn content_view_mode_control_projects_content_list_options() {
    let display = ContentViewModeControlDisplay::default_for_content_list(ContentViewMode::List);

    assert_eq!(
        display.selected,
        ContentViewMode::List,
        "Situational ADR 0062 content-list view mode guard: selected mode should pass through"
    );
    assert_eq!(
        display.options.map(|option| option.mode),
        [ContentViewMode::Tiles, ContentViewMode::List],
        "Situational ADR 0062 content-list view mode guard: content mode control should expose tile and list options"
    );
    assert_eq!(
        display.options.map(|option| option.label),
        ["Tiles", "List"],
        "Situational ADR 0062 content-list view mode guard: mode labels should be VM-owned"
    );
    assert_eq!(
        display.options.map(|option| option.a11y_label),
        ["Show Music content as tiles", "Show Music content as a list"],
        "Situational ADR 0062 content-list view mode guard: content mode accessibility labels should be VM-owned"
    );
}

#[test]
fn search_filter_chip_strip_defaults_use_standard_option_order() {
    let search_inspector =
        FilterChipStripDisplay::default_for_search_inspector(ContentFilter::All, true);

    let search_values: Vec<_> = search_inspector
        .options
        .iter()
        .map(|option| option.value)
        .collect();

    assert_eq!(
        search_values,
        [
            ContentFilter::All,
            ContentFilter::Library,
            ContentFilter::Index
        ],
        "search-inspector filters should keep the ADR 0047 option order"
    );
}

#[test]
fn search_filter_chip_strip_defaults_round_trip_selected_filter() {
    let search_inspector =
        FilterChipStripDisplay::default_for_search_inspector(ContentFilter::Index, true);

    assert_eq!(
        search_inspector.selected,
        ContentFilter::Index,
        "search-inspector filter display should preserve the selected filter"
    );
}

#[test]
fn search_filter_chip_strip_defaults_pass_through_narrow_collapse() {
    let expanded = FilterChipStripDisplay::default_for_search_inspector(ContentFilter::All, false);
    let collapsed = FilterChipStripDisplay::default_for_search_inspector(ContentFilter::All, true);

    assert!(
        !expanded.narrow_collapse_to_pulldown,
        "search-inspector filter display should preserve expanded narrow-mode preference"
    );
    assert!(
        collapsed.narrow_collapse_to_pulldown,
        "search-inspector filter display should preserve collapsed narrow-mode preference"
    );
}

#[test]
fn search_filter_chip_strip_width_class_projects_normal_and_narrow_layouts() {
    let normal = FilterChipStripDisplay::default_for_search_inspector_width_class(
        ContentFilter::All,
        FilterChipStripWidthClass::Normal,
    );
    let narrow = FilterChipStripDisplay::default_for_search_inspector_width_class(
        ContentFilter::All,
        FilterChipStripWidthClass::Narrow,
    );

    assert_eq!(
        normal
            .options
            .iter()
            .map(|option| option.value)
            .collect::<Vec<_>>(),
        [
            ContentFilter::All,
            ContentFilter::Library,
            ContentFilter::Index
        ],
        "ADR 0047 search-inspector content-filter reachability guard: all chip options must be exposed"
    );
    assert!(
        !normal.narrow_collapse_to_pulldown,
        "ADR 0047 search-inspector content-filter reachability guard: normal width must not collapse"
    );
    assert!(
        narrow.narrow_collapse_to_pulldown,
        "ADR 0047 search-inspector content-filter reachability guard: narrow width keeps the pull-down"
    );
}

#[test]
fn frame_shell_display_disables_empty_history_navigation() {
    let frame = frame(7, WorkspaceFrameKind::Detail);
    let nav = FrameNavigationState::new(track(42));

    let display = FrameShellDisplay::from_frame(&frame, &nav, true);

    assert!(
        display.back.disabled,
        "empty back history should disable frame Back"
    );
    assert!(
        display.forward.disabled,
        "empty forward history should disable frame Forward"
    );
    assert_eq!(
        display.back.id, "workspace-frame-7-back",
        "back id should be stable and frame-scoped"
    );
    assert_eq!(
        display.forward.id, "workspace-frame-7-forward",
        "forward id should be stable and frame-scoped"
    );
}

#[test]
fn frame_shell_display_uses_history_for_navigation_availability() {
    let frame = frame(7, WorkspaceFrameKind::Detail);
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));

    nav.push(track(42));
    let with_back = FrameShellDisplay::from_frame(&frame, &nav, true);

    assert!(
        !with_back.back.disabled,
        "back history should enable frame Back"
    );
    assert!(
        with_back.forward.disabled,
        "pushing a new entry should leave frame Forward disabled"
    );

    nav.go_back()
        .expect("pushed navigation should allow going back");
    let with_forward = FrameShellDisplay::from_frame(&frame, &nav, true);

    assert!(
        with_forward.back.disabled,
        "after returning to the first entry, frame Back should be disabled"
    );
    assert!(
        !with_forward.forward.disabled,
        "back navigation should enable frame Forward"
    );
}

#[test]
fn frame_shell_display_hides_close_when_not_allowed() {
    let frame = frame(7, WorkspaceFrameKind::Detail);
    let nav = FrameNavigationState::new(track(42));

    let fixed_frame = FrameShellDisplay::from_frame(&frame, &nav, false);
    let closable_frame = FrameShellDisplay::from_frame(&frame, &nav, true);

    assert_eq!(
        fixed_frame.close, None,
        "non-closable frames should not expose a close command"
    );
    assert_eq!(
        closable_frame.close.map(|close| close.id),
        Some("workspace-frame-7-close".to_string()),
        "closable frames should expose a frame-scoped close command"
    );
}

#[test]
fn frame_shell_display_passes_through_header_text_and_slot_id() {
    let frame = WorkspaceFrameState::new(
        WorkspaceFrameId::new(7),
        WorkspaceFrameKind::ContentList,
        "Playlist",
    )
    .with_subtitle("Seven tracks")
    .with_status("Ready");
    let nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));

    let display = FrameShellDisplay::from_frame(&frame, &nav, true);

    assert_eq!(display.frame_id, WorkspaceFrameId::new(7));
    assert_eq!(display.title, "Playlist");
    assert_eq!(
        display.subtitle,
        Some("Seven tracks".to_string()),
        "subtitle should pass through from frame state"
    );
    assert_eq!(
        display.status,
        Some("Ready".to_string()),
        "status should pass through from frame state"
    );
    assert_eq!(
        display.content_slot_id, "workspace-frame-7-content",
        "content slot id should be stable and frame-scoped"
    );
    assert_eq!(
        display.action_menu_items,
        Vec::new(),
        "transitional workspace must not expose multi-frame actions before real frame content owners exist"
    );
    assert_eq!(
        display.filter_chip_strip, None,
        "filter chips are opt-in frame chrome"
    );
    assert_eq!(
        display.view_mode_control, None,
        "view-mode controls are opt-in frame chrome"
    );
    assert_eq!(
        display.breadcrumb, None,
        "breadcrumbs are opt-in frame chrome"
    );
}

#[test]
fn frame_shell_display_accepts_optional_filter_chip_strip() {
    let frame = frame(7, WorkspaceFrameKind::ContentList);
    let nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));
    let filters =
        FilterChipStripDisplay::default_for_search_inspector(ContentFilter::Library, true);

    let display =
        FrameShellDisplay::from_frame(&frame, &nav, true).with_filter_chip_strip(filters.clone());

    assert_eq!(
        display.filter_chip_strip,
        Some(filters),
        "frame shell should carry optional frame-local filters without applying them"
    );
}

#[test]
fn frame_shell_display_accepts_optional_view_mode_control() {
    let frame = frame(7, WorkspaceFrameKind::ContentList);
    let nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));
    let modes = ContentViewModeControlDisplay::default_for_content_list(ContentViewMode::List);

    let display =
        FrameShellDisplay::from_frame(&frame, &nav, true).with_view_mode_control(modes.clone());

    assert_eq!(
        display.view_mode_control,
        Some(modes),
        "frame shell should carry optional frame-local view modes without applying them"
    );
}

#[test]
fn frame_shell_display_accepts_optional_breadcrumb() {
    let frame = frame(7, WorkspaceFrameKind::ContentList);
    let nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));
    let breadcrumb = BreadcrumbDisplay::project("crumbs", &nav, |_| "Playlist".to_string());

    let display =
        FrameShellDisplay::from_frame(&frame, &nav, true).with_breadcrumb(breadcrumb.clone());

    assert_eq!(
        display.breadcrumb,
        Some(breadcrumb),
        "frame shell should carry optional frame-local breadcrumbs without routing them"
    );
}

#[test]
fn frame_shell_display_hides_header_when_no_header_facts_exist() {
    let frame = WorkspaceFrameState::new(
        WorkspaceFrameId::new(7),
        WorkspaceFrameKind::ContentList,
        "",
    );
    let nav = FrameNavigationState::new(discover());

    let display = FrameShellDisplay::from_frame(&frame, &nav, false);

    assert!(
        !display.header_visible(),
        "Durable ADR 0061 element hierarchy guard: root content frame must not render a redundant section header"
    );
}

#[test]
fn frame_shell_display_shows_header_when_frame_facts_exist() {
    let frame = WorkspaceFrameState::new(
        WorkspaceFrameId::new(7),
        WorkspaceFrameKind::ContentList,
        "Search Results",
    );
    let nav = FrameNavigationState::new(FrameNavigationEntry::Search("beats".to_string()));

    let display = FrameShellDisplay::from_frame(&frame, &nav, false);

    assert!(
        display.header_visible(),
        "Durable ADR 0061 element hierarchy guard: informative frame headers must remain visible"
    );
}

#[test]
fn breadcrumb_display_projects_navigation_path() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));
    nav.push(track(42));

    let display =
        BreadcrumbDisplay::project("library-track-breadcrumb", &nav, |entry| match entry {
            FrameNavigationEntry::PlaylistDetail(_) => "My Playlist".to_string(),
            FrameNavigationEntry::Track(_) => "Lantern Tide".to_string(),
            _ => "Library".to_string(),
        });

    assert_eq!(display.id, "library-track-breadcrumb");
    assert_eq!(display.truncation, BreadcrumbTruncation::MiddleEllipsis);
    assert_eq!(display.segments.len(), 2);
    assert_eq!(display.segments[0].label, "My Playlist");
    assert_eq!(
        display.segments[0].target,
        Some(FrameNavigationEntry::PlaylistDetail(7))
    );
    assert!(
        !display.segments[0].is_current,
        "playlist segment should be a selectable parent"
    );
    assert_eq!(display.segments[1].label, "Lantern Tide");
    assert_eq!(display.segments[1].target, None);
    assert!(
        display.segments[1].is_current,
        "track segment should be the current location"
    );
}

/// R4-04 (ADR 0077 packet 004): the publisher breadcrumb segment shows the
/// title that its `label_for` closure supplies (the view model title at the
/// call site), not the publisher feed GUID.
#[test]
fn adr_0077_publisher_navigation_breadcrumb_shows_view_model_title() {
    let mut nav = FrameNavigationState::new(discover());
    nav.push(FrameNavigationEntry::PublisherDetail(
        "publisher-guid".to_string(),
    ));

    let display = BreadcrumbDisplay::project("publisher-breadcrumb", &nav, |entry| match entry {
        FrameNavigationEntry::PublisherDetail(_) => "Wavlake Artists".to_string(),
        _ => "Library".to_string(),
    });

    assert_eq!(display.segments.len(), 2);
    assert_eq!(display.segments[1].label, "Wavlake Artists");
    assert!(display.segments[1].is_current);
    assert_eq!(display.segments[1].target, None);
}

#[test]
fn breadcrumb_display_projects_index_search_drilldown_path() {
    let mut nav = FrameNavigationState::new(discover());
    nav.push(FrameNavigationEntry::Search("survival guide".to_string()));
    nav.push(FrameNavigationEntry::IndexNameMatches(
        "Survival Guide".to_string(),
    ));
    nav.push(FrameNavigationEntry::Release(
        ReleaseKey::from_index_feed_guid("feed-guid"),
    ));

    let display = BreadcrumbDisplay::project("index-search-breadcrumb", &nav, |entry| {
        entry.display_label()
    });

    assert_eq!(
        display
            .segments
            .iter()
            .map(|segment| segment.label.as_str())
            .collect::<Vec<_>>(),
        [
            "Discover",
            "Search: survival guide",
            "\"Survival Guide\"",
            // ADR 0084 Decision 4: the entry itself carries no title, so its
            // fallback label is "Release". The top app supplies the real
            // title through its own `label_for` closure at the call site.
            "Release"
        ]
    );
    assert_eq!(
        display.segments[2].target,
        Some(FrameNavigationEntry::IndexNameMatches(
            "Survival Guide".to_string()
        )),
        "the immediate Index parent must stay selectable in the breadcrumb"
    );
    assert_eq!(
        nav.active_search_query(),
        Some("survival guide"),
        "Index drill-down entries must keep their search ancestor active"
    );
    assert_eq!(
        nav.path_entries(),
        vec![
            discover(),
            FrameNavigationEntry::Search("survival guide".to_string()),
            FrameNavigationEntry::IndexNameMatches("Survival Guide".to_string()),
            FrameNavigationEntry::Release(ReleaseKey::from_index_feed_guid("feed-guid")),
        ],
        "breadcrumb labelers need the same full path rendered by frame chrome"
    );
}

#[test]
fn breadcrumb_display_projects_single_segment_as_current() {
    let nav = FrameNavigationState::new(track(42));

    let display = BreadcrumbDisplay::project("crumbs", &nav, |entry| match entry {
        FrameNavigationEntry::Track(key) => format!("Track {}", key.item_guid),
        _ => unreachable!("single-segment test should only project track detail"),
    });

    assert_eq!(display.id, "crumbs");
    assert_eq!(display.truncation, BreadcrumbTruncation::MiddleEllipsis);
    assert_eq!(display.segments.len(), 1);
    assert_eq!(display.segments[0].label, "Track 42");
    assert!(display.segments[0].is_current);
    assert_eq!(display.segments[0].target, None);
}

#[test]
fn breadcrumb_display_projects_four_segment_paths_without_ellipsis() {
    let mut nav = FrameNavigationState::new(discover());
    nav.push(FrameNavigationEntry::PlaylistDetail(7));
    nav.push(track(42));
    nav.push(FrameNavigationEntry::Release(ReleaseKey::LibraryFeed(11)));

    let display = BreadcrumbDisplay::project("crumbs", &nav, |entry| match entry {
        FrameNavigationEntry::Destination(MusicDestination::Discover) => "Discover".to_string(),
        FrameNavigationEntry::PlaylistDetail(id) => format!("Playlist {id}"),
        FrameNavigationEntry::Track(key) => format!("Track {}", key.item_guid),
        FrameNavigationEntry::Release(ReleaseKey::LibraryFeed(id)) => format!("Album {id}"),
        _ => unreachable!("long-path test should only project breadcrumb entries"),
    });

    assert_eq!(display.segments.len(), 4);
    assert_eq!(display.segments[0].label, "Discover");
    assert_eq!(display.segments[0].target, Some(discover()));
    assert_eq!(display.segments[1].label, "Playlist 7");
    assert_eq!(
        display.segments[1].target,
        Some(FrameNavigationEntry::PlaylistDetail(7))
    );
    assert_eq!(display.segments[2].label, "Track 42");
    assert_eq!(display.segments[2].target, Some(track(42)));
    assert_eq!(display.segments[3].label, "Album 11");
    assert!(display.segments[3].is_current);
    assert_eq!(display.segments[3].target, None);
}

#[test]
fn workspace_frame_navigation_isolated_per_frame() {
    let mut layout = WorkspaceLayout::default_layout();
    let first = WorkspaceFrameId::new(2);
    let second = WorkspaceFrameId::new(3);

    layout
        .reset_nav(first, FrameNavigationEntry::PlaylistDetail(7))
        .expect("first frame should exist");
    layout
        .reset_nav(second, track(42))
        .expect("second frame should exist");

    assert_eq!(
        layout
            .frame_nav(first)
            .expect("first frame navigation should exist")
            .current(),
        &FrameNavigationEntry::PlaylistDetail(7)
    );
    assert_eq!(
        layout
            .frame_nav(second)
            .expect("second frame navigation should exist")
            .current(),
        &track(42)
    );

    assert_eq!(
        layout.pop_nav(first),
        None,
        "a single-entry history should not go back"
    );
    layout
        .push_nav(first, track(99))
        .expect("first frame should exist");
    assert_eq!(
        layout.pop_nav(first),
        Some(FrameNavigationEntry::PlaylistDetail(7)),
        "popping one frame should not affect another frame's history"
    );
    assert_eq!(
        layout
            .frame_nav(second)
            .expect("second frame navigation should remain intact")
            .current(),
        &track(42)
    );
}

#[test]
fn default_layout_has_expected_workspace_shape() {
    let layout = WorkspaceLayout::default_layout();
    let kinds: Vec<_> = layout
        .frames()
        .iter()
        .map(WorkspaceFrameState::kind)
        .collect();

    assert_eq!(
        kinds,
        [
            WorkspaceFrameKind::SourceList,
            WorkspaceFrameKind::ContentList,
            WorkspaceFrameKind::Detail,
        ],
        "default workspace should expose the ADR 0060 curation frame order"
    );
    assert_eq!(
        focused_frame_of(&layout).map(WorkspaceFrameState::kind),
        Some(WorkspaceFrameKind::ContentList),
        "default workspace should focus the primary content frame"
    );
    assert_eq!(
        layout
            .frames()
            .iter()
            .filter(|frame| frame.is_focused())
            .count(),
        1,
        "default workspace should mark exactly one focused frame"
    );
}

#[test]
fn empty_layout_has_no_focus_and_rejects_focus_mutation() {
    let mut layout =
        WorkspaceLayout::new(Vec::new(), None).expect("an empty frame list is a valid layout");

    assert!(
        layout.frames().is_empty(),
        "empty layout should contain no frames"
    );
    assert_eq!(
        layout.focused_frame_id(),
        None,
        "empty layout should not carry a focused frame id"
    );
    assert_eq!(
        layout.focus_frame(WorkspaceFrameId::new(1)),
        Err(WorkspaceModelError::EmptyLayout),
        "empty layout should not accept focus mutation"
    );
}

#[test]
fn single_frame_layout_marks_only_frame_focused() {
    let layout = WorkspaceLayout::new(
        vec![frame(10, WorkspaceFrameKind::Detail)],
        Some(WorkspaceFrameId::new(10)),
    )
    .expect("single-frame layout should be valid");

    assert_eq!(
        focused_frame_of(&layout).map(WorkspaceFrameState::id),
        Some(WorkspaceFrameId::new(10)),
        "single-frame layout should focus its only frame"
    );
    assert!(
        layout.frames()[0].is_focused(),
        "single-frame layout should mirror focus into the frame state"
    );
}

#[test]
fn single_frame_layout_without_requested_focus_focuses_only_frame() {
    let layout = WorkspaceLayout::new(vec![frame(10, WorkspaceFrameKind::Detail)], None)
        .expect("single-frame layout should be valid");

    assert_eq!(
        layout.focused_frame_id(),
        Some(WorkspaceFrameId::new(10)),
        "non-empty layouts should preserve a focus invariant"
    );
    assert!(
        layout.frames()[0].is_focused(),
        "implicit focus should mirror into the frame state"
    );
}

#[test]
fn multi_frame_focus_moves_between_existing_frames() {
    let mut layout = WorkspaceLayout::new(
        vec![
            frame(1, WorkspaceFrameKind::SourceList),
            frame(2, WorkspaceFrameKind::ContentList),
            frame(3, WorkspaceFrameKind::Detail),
        ],
        Some(WorkspaceFrameId::new(1)),
    )
    .expect("multi-frame layout should be valid");

    layout
        .focus_frame(WorkspaceFrameId::new(3))
        .expect("existing frame should be focusable");

    assert_eq!(
        focused_frame_of(&layout).map(WorkspaceFrameState::id),
        Some(WorkspaceFrameId::new(3)),
        "focus should move to the requested frame"
    );
    assert_eq!(
        layout
            .frames()
            .iter()
            .filter(|frame| frame.is_focused())
            .count(),
        1,
        "multi-frame layout should mark exactly one focused frame"
    );
}

#[test]
fn invalid_layout_mutations_return_errors() {
    let mut layout = WorkspaceLayout::new(
        vec![frame(1, WorkspaceFrameKind::SourceList)],
        Some(WorkspaceFrameId::new(1)),
    )
    .expect("initial layout should be valid");

    assert_eq!(
        layout.focus_frame(WorkspaceFrameId::new(99)),
        Err(WorkspaceModelError::FrameNotFound(WorkspaceFrameId::new(
            99
        ))),
        "focusing a missing frame should return an error"
    );
}

#[test]
fn duplicate_frames_are_rejected_at_construction() {
    assert_eq!(
        WorkspaceLayout::new(
            vec![
                frame(1, WorkspaceFrameKind::SourceList),
                frame(1, WorkspaceFrameKind::Detail),
            ],
            Some(WorkspaceFrameId::new(1)),
        ),
        Err(WorkspaceModelError::DuplicateFrameId(
            WorkspaceFrameId::new(1)
        )),
        "constructor should reject duplicate frame ids"
    );
}

#[test]
fn replace_nav_preserves_full_history_for_visible_layout_projection() {
    let mut source = WorkspaceLayout::default_layout();
    let detail_id = detail_frame_id(&source);
    source
        .reset_nav(detail_id, FrameNavigationEntry::Search("jazz".to_string()))
        .expect("detail frame should exist");
    source
        .push_nav(detail_id, track(9))
        .expect("detail frame should accept drill-down");

    let mut projected = WorkspaceLayout::new(
        vec![frame(detail_id.value(), WorkspaceFrameKind::Detail)],
        Some(detail_id),
    )
    .expect("projected layout should be valid");
    projected
        .replace_nav(
            detail_id,
            source
                .frame_nav(detail_id)
                .expect("source detail nav should exist")
                .clone(),
        )
        .expect("projected detail frame should accept copied nav");

    let nav = projected
        .frame_nav(detail_id)
        .expect("projected detail nav should exist");
    assert_eq!(nav.current(), &track(9));
    assert_eq!(
        back_page(nav),
        Some(FrameNavigationEntry::Search("jazz".to_string())),
        "full navigation history should survive projection"
    );
}

#[test]
fn workspace_layout_config_round_trips() {
    let added_id = WorkspaceFrameId::new(4);
    let layout = WorkspaceLayout::new(
        vec![
            frame(1, WorkspaceFrameKind::SourceList),
            frame(2, WorkspaceFrameKind::ContentList),
            frame(3, WorkspaceFrameKind::Detail),
            frame(added_id.value(), WorkspaceFrameKind::Detail),
        ],
        Some(added_id),
    )
    .expect("layout with an extra frame should be valid");

    let restored = WorkspaceLayout::from_config(Some(&layout.to_config()));

    assert_eq!(
        restored.to_config(),
        layout.to_config(),
        "config conversion should preserve frame order, kinds, and focus"
    );
    assert_eq!(
        restored.focused_frame_id(),
        Some(added_id),
        "config conversion should preserve focused frame id"
    );
}

#[test]
fn workspace_layout_config_from_before_broadcast_frame_still_loads() {
    let pre_broadcast_config = WorkspaceLayoutConfig {
        frames: vec![
            WorkspaceFrameConfig {
                id: 1,
                kind: WorkspaceFrameKind::SourceList,
            },
            WorkspaceFrameConfig {
                id: 2,
                kind: WorkspaceFrameKind::ContentList,
            },
            WorkspaceFrameConfig {
                id: 3,
                kind: WorkspaceFrameKind::Detail,
            },
            WorkspaceFrameConfig {
                id: 4,
                kind: WorkspaceFrameKind::QueueNowPlaying,
            },
        ],
        focused_frame_id: Some(2),
    };

    let restored = WorkspaceLayout::from_config(Some(&pre_broadcast_config));

    assert_eq!(
        restored.to_config(),
        pre_broadcast_config,
        "layouts written before Broadcast existed should still load"
    );
    assert_eq!(
        focused_frame_of(&restored).map(WorkspaceFrameState::kind),
        Some(WorkspaceFrameKind::ContentList),
        "old layouts should preserve focus"
    );
}

#[test]
fn malformed_and_empty_workspace_layout_config_falls_back_to_default() {
    let empty = WorkspaceLayoutConfig {
        frames: Vec::new(),
        focused_frame_id: None,
    };
    let duplicate = WorkspaceLayoutConfig {
        frames: vec![
            WorkspaceFrameConfig {
                id: 1,
                kind: WorkspaceFrameKind::SourceList,
            },
            WorkspaceFrameConfig {
                id: 1,
                kind: WorkspaceFrameKind::Detail,
            },
        ],
        focused_frame_id: Some(1),
    };
    let missing_focus = WorkspaceLayoutConfig {
        frames: vec![WorkspaceFrameConfig {
            id: 8,
            kind: WorkspaceFrameKind::Detail,
        }],
        focused_frame_id: Some(99),
    };
    let default = WorkspaceLayout::default_layout().to_config();

    assert_eq!(
        WorkspaceLayout::from_config(None).to_config(),
        default,
        "missing config should fall back to default layout"
    );
    assert_eq!(
        WorkspaceLayout::from_config(Some(&empty)).to_config(),
        default,
        "empty config should fall back to default layout"
    );
    assert_eq!(
        WorkspaceLayout::from_config(Some(&duplicate)).to_config(),
        default,
        "duplicate frame ids should fall back to default layout"
    );
    assert_eq!(
        WorkspaceLayout::from_config(Some(&missing_focus)).to_config(),
        default,
        "missing focused frame should fall back to default layout"
    );
}

#[test]
fn navigation_back_forward_boundaries_return_errors() {
    let mut nav = FrameNavigationState::new(discover());

    assert!(
        !nav.can_go_back(),
        "new navigation state should not have back history"
    );
    assert!(
        !nav.can_go_forward(),
        "new navigation state should not have forward history"
    );
    assert_eq!(
        nav.go_back(),
        Err(WorkspaceModelError::CannotNavigateBack),
        "back at the first entry should return an error"
    );
    assert_eq!(
        nav.go_forward(),
        Err(WorkspaceModelError::CannotNavigateForward),
        "forward without forward history should return an error"
    );
}

#[test]
fn navigation_push_pop_round_trip() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));
    nav.push(track(42));

    assert_eq!(
        nav.current(),
        &track(42),
        "push should update the current entry"
    );
    assert!(nav.can_go_back(), "push should create back history");
    assert!(!nav.can_go_forward(), "push should clear forward history");

    assert_eq!(
        nav.go_back().cloned(),
        Ok(FrameNavigationEntry::PlaylistDetail(7)),
        "go_back should restore the previous entry"
    );
    assert!(
        nav.can_go_forward(),
        "go_back should create forward history"
    );

    assert_eq!(
        nav.go_forward().cloned(),
        Ok(track(42)),
        "go_forward should restore the pushed entry"
    );
    assert!(
        !nav.can_go_forward(),
        "round-trip should consume forward history"
    );
}

/// R15-01 (ADR 0046 Task 015): the chrome display exposes Forward as
/// unavailable before a Back press, and as available once a Back press
/// leaves forward history behind.
#[test]
fn adr_0046_forward_chrome_display_enables_forward_after_back() {
    let frame = frame(7, WorkspaceFrameKind::Detail);
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));
    nav.push(track(42));

    let before_back = FrameShellDisplay::from_frame(&frame, &nav, true);
    assert!(
        before_back.forward.disabled,
        "Forward must stay unavailable before a Back press"
    );

    nav.go_back()
        .expect("pushed navigation should allow going back");
    let after_back = FrameShellDisplay::from_frame(&frame, &nav, true);
    assert!(
        !after_back.forward.disabled,
        "Forward must become available once a Back press leaves forward history"
    );
}

/// R15-03 (ADR 0046 Task 015): a push after a Back clears the forward
/// history, and the chrome display exposes Forward as unavailable again.
#[test]
fn adr_0046_forward_push_after_back_clears_forward_history() {
    let frame = frame(7, WorkspaceFrameKind::Detail);
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));
    nav.push(track(42));
    nav.go_back()
        .expect("pushed navigation should allow going back");
    assert!(
        nav.can_go_forward(),
        "a Back press must leave forward history for this test to be meaningful"
    );

    nav.push(FrameNavigationEntry::Release(ReleaseKey::LibraryFeed(5)));

    let display = FrameShellDisplay::from_frame(&frame, &nav, true);
    assert!(
        display.forward.disabled,
        "a push after a Back must clear forward history"
    );
}

/// R15-02 (ADR 0046 Task 015): `WorkspaceLayout::pop_nav_forward` restores
/// the entry that `pop_nav` (Back) left, and the chrome then shows Forward
/// as unavailable again.
#[test]
fn adr_0046_forward_pop_nav_forward_restores_entry_back_left() {
    let mut layout = WorkspaceLayout::default_layout();
    let content_list_id = layout
        .frames()
        .iter()
        .find(|frame| frame.kind() == WorkspaceFrameKind::ContentList)
        .map(WorkspaceFrameState::id)
        .expect("default layout should include a content-list frame");

    layout
        .push_nav(content_list_id, track(42))
        .expect("content-list frame should exist");

    let entry_back_left = layout
        .pop_nav(content_list_id)
        .expect("a pushed entry should allow going back");
    assert_eq!(
        entry_back_left,
        // ADR 0084 Decision 3: the default layout opens on Discover, then
        // "New releases". A pushed page therefore sits under that page,
        // not under the bare Discover destination.
        FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases),
        "Back should restore the entry that preceded the push"
    );

    let restored_by_forward = layout
        .pop_nav_forward(content_list_id)
        .expect("a Back press should leave a forward entry");
    assert_eq!(
        restored_by_forward,
        track(42),
        "Forward should restore the entry that Back left"
    );

    let frame = frame(content_list_id.value(), WorkspaceFrameKind::ContentList);
    let nav = layout
        .frame_nav(content_list_id)
        .expect("content-list frame navigation should exist");
    let display = FrameShellDisplay::from_frame(&frame, nav, true);
    assert!(
        display.forward.disabled,
        "Forward must become unavailable once its only entry is restored"
    );
}

/// R4-04 (ADR 0077 packet 004): the publisher navigation entry keys on the
/// publisher feed GUID, so two different GUIDs are two different entries
/// and pushing a repeated GUID is a no-op, the same as every other entry.
#[test]
fn adr_0077_publisher_navigation_entry_keys_on_guid() {
    let mut nav = FrameNavigationState::new(discover());

    nav.push(FrameNavigationEntry::PublisherDetail("guid-a".into()));
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::PublisherDetail("guid-a".into())
    );
    assert!(nav.can_go_back());

    nav.push(FrameNavigationEntry::PublisherDetail("guid-a".into()));
    assert!(
        !nav.can_go_forward(),
        "repeating the same publisher feed GUID must not push a duplicate entry"
    );

    nav.push(FrameNavigationEntry::PublisherDetail("guid-b".into()));
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::PublisherDetail("guid-b".into())
    );
    assert_ne!(
        nav.current(),
        &FrameNavigationEntry::PublisherDetail("guid-a".into()),
        "two different publisher feed GUIDs must be two different entries"
    );
}

#[test]
fn navigation_push_current_entry_is_noop() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));

    nav.push(FrameNavigationEntry::PlaylistDetail(7));

    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::PlaylistDetail(7),
        "same-entry push should preserve the current destination"
    );
    assert!(
        !nav.can_go_back(),
        "same-entry push should not create synthetic back history"
    );
    assert!(
        !nav.can_go_forward(),
        "same-entry push should not create synthetic forward history"
    );
}

#[test]
fn navigation_reset_clears_back_and_forward_history() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));
    nav.push(track(42));
    nav.go_back()
        .expect("pushed navigation should allow going back");

    nav.reset(track(99));

    assert_eq!(
        nav.current(),
        &track(99),
        "reset should replace the current entry"
    );
    assert!(!nav.can_go_back(), "reset should clear back history");
    assert!(!nav.can_go_forward(), "reset should clear forward history");
}

#[test]
fn open_search_results_in_content_list_pushes_search_onto_content_list_nav() {
    let mut layout = WorkspaceLayout::default_layout();

    let result = layout.open_search_results_in_content_list("ambient");

    assert_eq!(
        result,
        Ok(WorkspaceFrameId::new(2)),
        "should return the ContentList frame id"
    );
    let nav = layout
        .frame_nav(WorkspaceFrameId::new(2))
        .expect("ContentList should have navigation state");
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::Search("ambient".to_string()),
        "ContentList nav top should be the search query"
    );
    assert_eq!(
        layout.focused_frame_id(),
        Some(WorkspaceFrameId::new(2)),
        "ContentList should be focused"
    );
}

#[test]
fn open_search_results_in_content_list_pushes_from_non_search_nav() {
    let mut layout = WorkspaceLayout::default_layout();
    let content_list_id = WorkspaceFrameId::new(2);

    layout
        .reset_nav(content_list_id, FrameNavigationEntry::PlaylistDetail(7))
        .expect("should reset ContentList nav");
    layout
        .open_search_results_in_content_list("drums")
        .expect("should push search onto ContentList");

    let nav = layout
        .frame_nav(content_list_id)
        .expect("ContentList should have navigation state");
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::Search("drums".to_string()),
        "current should be the new search"
    );
    assert!(
        nav.can_go_back(),
        "back history should contain the playlist detail entry"
    );
    assert_eq!(
        back_page(nav),
        Some(FrameNavigationEntry::PlaylistDetail(7)),
        "back should go to the playlist detail"
    );
}

#[test]
fn open_search_results_in_content_list_replaces_current_search_nav() {
    let mut layout = WorkspaceLayout::default_layout();
    let content_list_id = WorkspaceFrameId::new(2);

    layout
        .reset_nav(content_list_id, FrameNavigationEntry::PlaylistDetail(7))
        .expect("should reset ContentList nav");
    layout
        .open_search_results_in_content_list("drums")
        .expect("should push the first search");
    layout
        .open_search_results_in_content_list("survival guide")
        .expect("should replace the current search");

    let nav = layout
        .frame_nav(content_list_id)
        .expect("ContentList should have navigation state");
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::Search("survival guide".to_string()),
        "current should be the latest search"
    );
    assert_eq!(
        back_page(nav),
        Some(FrameNavigationEntry::PlaylistDetail(7)),
        "back should skip the overwritten search and return to prior content"
    );
}

#[test]
fn open_search_results_in_content_list_replaces_search_ancestor_nav() {
    let mut layout = WorkspaceLayout::default_layout();
    let content_list_id = WorkspaceFrameId::new(2);

    layout
        .reset_nav(content_list_id, FrameNavigationEntry::PlaylistDetail(7))
        .expect("should reset ContentList nav");
    layout
        .open_search_results_in_content_list("heycitizen")
        .expect("should push the first search");
    layout
        .push_nav(
            content_list_id,
            FrameNavigationEntry::ArtistDetail("HeyCitizen".to_string()),
        )
        .expect("should push selected result detail");
    layout
        .open_search_results_in_content_list("survival guide")
        .expect("should replace the active search flow");

    let nav = layout
        .frame_nav(content_list_id)
        .expect("ContentList should have navigation state");
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::Search("survival guide".to_string()),
        "current should be the latest search"
    );
    assert_eq!(
        nav.path_entries(),
        vec![FrameNavigationEntry::Search("survival guide".to_string())],
        "ADR 0084 Decision 9: a search from a detail page starts a new path"
    );
    assert_eq!(
        back_page(nav),
        Some(FrameNavigationEntry::ArtistDetail("HeyCitizen".to_string())),
        "ADR 0084 Decision 9: Back returns to the selected detail of the earlier search"
    );
}

/// N1-01 (ADR 0084 Decision 9): the location path holds each page one time
/// at most, also after a sequence that opens a page two times.
#[test]
fn adr_0084_nav_location_path_holds_each_page_once() {
    let mut nav = FrameNavigationState::new(discover());
    nav.start_path(discover(), Some(FrameNavigationEntry::PlaylistDetail(0)));
    nav.push(track(7));
    nav.push(FrameNavigationEntry::Release(ReleaseKey::LibraryFeed(3)));
    nav.push(track(7));

    assert_eq!(
        nav.path_entries(),
        vec![
            discover(),
            FrameNavigationEntry::PlaylistDetail(0),
            track(7),
        ],
        "the second open of the track must cut the path at the track"
    );
}

/// N1-02 (ADR 0084 Decision 9): a push of a page that the path holds cuts
/// the path at that page, and Back returns to the longer path.
#[test]
fn adr_0084_nav_push_of_path_page_cuts_path_and_back_restores() {
    let mut nav = FrameNavigationState::new(discover());
    nav.push(FrameNavigationEntry::PlaylistDetail(0));
    nav.push(track(7));
    let longer_path = nav.path_entries();

    nav.push(discover());

    assert_eq!(
        nav.path_entries(),
        vec![discover()],
        "a breadcrumb segment must cut the path at that segment"
    );
    assert_eq!(
        nav.go_back().cloned(),
        Ok(track(7)),
        "Back must return to the page before the cut"
    );
    assert_eq!(
        nav.path_entries(),
        longer_path,
        "Back must restore the longer path"
    );
}

/// ADR 0084 Decision 9, operator check of 2026-10-18: a second row of the
/// Artists list replaced no page. The path read "Artists > `HeyCitizen` >
/// Haleen". A list row now opens directly under the destination, and Back
/// returns to the first row's page.
#[test]
fn adr_0084_nav_list_row_opens_under_the_destination() {
    let artists = FrameNavigationEntry::Destination(MusicDestination::Artists);
    let heycitizen = FrameNavigationEntry::PublisherDetail("heycitizen-guid".to_string());
    let haleen = FrameNavigationEntry::ArtistDetail("Haleen".to_string());
    let mut nav = FrameNavigationState::new(artists.clone());

    nav.open_list_row(heycitizen.clone());
    nav.open_list_row(haleen.clone());
    assert_eq!(nav.path_entries(), vec![artists.clone(), haleen.clone()]);

    nav.push(track(7));
    nav.open_list_row(heycitizen.clone());
    assert_eq!(
        nav.path_entries(),
        vec![artists.clone(), heycitizen],
        "a list row replaces each open page under the destination"
    );

    nav.go_back().expect("Back must be available");
    assert_eq!(nav.path_entries(), vec![artists.clone(), haleen, track(7)]);

    let mut root_only = FrameNavigationState::new(artists.clone());
    root_only.open_list_row(artists.clone());
    assert_eq!(
        root_only.path_entries(),
        vec![artists],
        "the destination itself stays one entry"
    );
}

/// N1-03 (ADR 0084 Decision 9): a new path holds the destination and the
/// page only, and Back returns to the earlier location.
#[test]
fn adr_0084_nav_start_path_holds_destination_and_page() {
    let mut nav = FrameNavigationState::new(discover());
    nav.push(FrameNavigationEntry::PlaylistDetail(0));
    nav.push(track(7));
    let earlier_path = nav.path_entries();

    nav.start_path(
        discover(),
        Some(FrameNavigationEntry::ArtistDetail("Haleen".to_string())),
    );

    assert_eq!(
        nav.path_entries(),
        vec![
            discover(),
            FrameNavigationEntry::ArtistDetail("Haleen".to_string()),
        ],
        "a sidebar click must start a new path"
    );
    assert!(nav.can_go_back(), "a sidebar click must add history");
    nav.go_back().expect("Back must be available");
    assert_eq!(
        nav.path_entries(),
        earlier_path,
        "Back must restore the earlier location"
    );
}

/// N1-04 (ADR 0084 Decision 9): a push after a Back clears the forward
/// history.
#[test]
fn adr_0084_nav_push_after_back_clears_forward_history() {
    let mut nav = FrameNavigationState::new(discover());
    nav.start_path(discover(), Some(FrameNavigationEntry::PlaylistDetail(0)));
    nav.go_back().expect("Back must be available");
    assert!(nav.can_go_forward(), "Back must create forward history");

    nav.push(FrameNavigationEntry::Release(ReleaseKey::LibraryFeed(3)));

    assert!(
        !nav.can_go_forward(),
        "a push after Back must clear forward history"
    );
}

/// N1-05 (ADR 0084 Decision 9): a new query on the search page replaces
/// the query in place. A search from another page adds one history entry.
#[test]
fn adr_0084_nav_search_replaces_in_place_or_adds_one_entry() {
    let mut nav = FrameNavigationState::new(discover());
    nav.push(FrameNavigationEntry::PlaylistDetail(0));

    nav.replace_active_search_or_push(FrameNavigationEntry::Search("drums".to_string()));
    nav.replace_active_search_or_push(FrameNavigationEntry::Search("bass".to_string()));

    assert_eq!(
        nav.path_entries(),
        vec![FrameNavigationEntry::Search("bass".to_string())],
        "a search must start its own path, and a new query must replace it"
    );
    assert_eq!(
        nav.go_back().cloned(),
        Ok(FrameNavigationEntry::PlaylistDetail(0)),
        "the two queries must add one history entry"
    );
}

/// N1-06 (ADR 0084 Decision 9): the Music content frame always shows Back
/// and Forward, each with its typed availability. Other frames show each
/// control only when it is available.
#[test]
fn adr_0084_nav_music_chrome_always_shows_history_controls() {
    let frame = frame(2, WorkspaceFrameKind::ContentList);
    let nav = FrameNavigationState::new(discover());

    let other = FrameShellDisplay::from_frame(&frame, &nav, false);
    assert!(
        !other.back_shown(),
        "an unavailable Back stays hidden by default"
    );
    assert!(
        !other.forward_shown(),
        "an unavailable Forward stays hidden by default"
    );

    let music =
        FrameShellDisplay::from_frame(&frame, &nav, false).with_history_controls_always_shown();
    assert!(music.back_shown(), "Music must always show Back");
    assert!(music.forward_shown(), "Music must always show Forward");
    assert!(
        music.back.disabled,
        "Back must stay unavailable with no history"
    );
    assert!(
        music.forward.disabled,
        "Forward must stay unavailable with no forward history"
    );
    assert!(
        music.header_visible(),
        "the header must show for the history controls"
    );
}

/// N1-07 (ADR 0084 Decision 9): the breadcrumb projects the location path,
/// not the history.
#[test]
fn adr_0084_nav_breadcrumb_projects_location_not_history() {
    let mut nav = FrameNavigationState::new(discover());
    nav.start_path(discover(), Some(FrameNavigationEntry::PlaylistDetail(0)));
    nav.push(track(7));
    nav.push(FrameNavigationEntry::PlaylistDetail(0));
    nav.push(track(7));

    let display = BreadcrumbDisplay::project("crumbs", &nav, FrameNavigationEntry::display_label);
    let ids: Vec<_> = display
        .segments
        .iter()
        .map(|segment| segment.id.as_str())
        .collect();

    assert_eq!(
        ids,
        vec!["destination-discover", "playlist-0", "track-7"],
        "the breadcrumb must not repeat the playlist or the track"
    );
    assert!(
        nav.has_location_path(),
        "a path with a page has a location path"
    );
}

/// R2-01 (ADR 0084 Decision 4): the release key of a Library album with a
/// feed GUID equals the release key of an Index row with the same feed
/// GUID.
#[test]
fn adr_0084_release_feed_guid_key_matches_across_origins() {
    let library_key = ReleaseKey::FeedGuid("feed-guid".to_string());
    let index_key = ReleaseKey::from_index_feed_guid("feed-guid");

    assert_eq!(library_key, index_key);
}

/// R2-02 (ADR 0084 Decision 4): the release key of a Library album with
/// no feed GUID is `LibraryFeed` with its feed id.
#[test]
fn adr_0084_release_library_feed_key_carries_its_feed_id() {
    let key = ReleaseKey::LibraryFeed(42);

    assert_eq!(key, ReleaseKey::LibraryFeed(42));
    assert_ne!(key, ReleaseKey::FeedGuid("42".to_string()));
}

/// R2-03 (ADR 0084 Decision 4): the resolver gives the Library source for
/// `LibraryFeed`, and for `FeedGuid` when the lookup finds a feed. It gives
/// the Index source when the lookup finds no feed.
#[test]
fn adr_0084_release_resolver_follows_the_lookup_result() {
    assert_eq!(
        resolve_release_page_source(ReleaseKey::LibraryFeed(7), None),
        ReleasePageSource::Library { feed_id: 7 },
        "a Library feed key never needs a lookup"
    );
    assert_eq!(
        resolve_release_page_source(ReleaseKey::FeedGuid("guid".to_string()), Some(9)),
        ReleasePageSource::Library { feed_id: 9 },
        "a found feed resolves to the Library source"
    );
    assert_eq!(
        resolve_release_page_source(ReleaseKey::FeedGuid("guid".to_string()), None),
        ReleasePageSource::Index {
            feed_guid: "guid".to_string()
        },
        "no found feed resolves to the Index source"
    );
}

/// R2-04 (ADR 0084 Decision 4): a path that opens one release from the
/// Library and then from the Index holds the release one time. The two
/// opens push the same key, so the second push cuts the path at the first.
#[test]
fn adr_0084_release_path_holds_one_release_opened_from_both_origins() {
    let mut nav = FrameNavigationState::new(discover());
    let library_open = FrameNavigationEntry::Release(ReleaseKey::FeedGuid("feed-guid".into()));
    let index_open = FrameNavigationEntry::Release(ReleaseKey::from_index_feed_guid("feed-guid"));

    nav.push(library_open);
    nav.push(FrameNavigationEntry::ArtistDetail("Haleen".to_string()));
    nav.push(index_open);

    assert_eq!(
        nav.path_entries(),
        vec![
            discover(),
            FrameNavigationEntry::Release(ReleaseKey::FeedGuid("feed-guid".into())),
        ],
        "the Index open must cut the path at the earlier Library open of the same release"
    );
}

/// R2-05 (ADR 0084 Decision 4): the breadcrumb label of a release entry
/// with no known title is "Release".
#[test]
fn adr_0084_release_breadcrumb_label_with_no_known_title_is_release() {
    let nav = FrameNavigationState::new(FrameNavigationEntry::Release(ReleaseKey::LibraryFeed(5)));

    let display = BreadcrumbDisplay::project("crumbs", &nav, FrameNavigationEntry::display_label);

    assert_eq!(display.segments.len(), 1);
    assert_eq!(display.segments[0].label, "Release");
}

/// Returns the page that a Back press would show, without a change to
/// `nav`.
fn back_page(nav: &FrameNavigationState) -> Option<FrameNavigationEntry> {
    let mut copy = nav.clone();
    copy.go_back().ok().cloned()
}

/// R2-07 (ADR 0084 Decision 4): a completed download changes only the open
/// release page of the downloaded feed GUID.
#[test]
fn adr_0084_release_download_updates_only_the_open_release_page() {
    let open = FrameNavigationEntry::Release(ReleaseKey::FeedGuid("feed-a".to_string()));
    let other = FrameNavigationEntry::Release(ReleaseKey::FeedGuid("feed-b".to_string()));

    assert!(release_download_updates_open_page(
        Some(&open),
        Some("feed-a")
    ));
    assert!(
        !release_download_updates_open_page(Some(&other), Some("feed-a")),
        "a download must not change another open release"
    );
    assert!(
        !release_download_updates_open_page(Some(&discover()), Some("feed-a")),
        "a download must not change a page that is not a release"
    );
    assert!(
        !release_download_updates_open_page(Some(&open), None),
        "a download with no feed GUID must not change the open page"
    );
}

/// R3-01 (ADR 0084 Decision 4): the track key of a Library track with a
/// feed GUID equals the track key of an Index target with the same feed
/// GUID and item GUID.
#[test]
fn adr_0084_track_feed_guid_key_matches_across_origins() {
    let library_key = TrackKey {
        release: Some(ReleaseKey::FeedGuid("feed-guid".to_string())),
        item_guid: "item-guid".to_string(),
    };
    let index_key = TrackKey::from_index_track_target(Some("feed-guid"), "item-guid");

    assert_eq!(library_key, index_key);
}

/// R3-02 (ADR 0084 Decision 4): the track key of a Library track in a feed
/// with no feed GUID carries `ReleaseKey::LibraryFeed`.
#[test]
fn adr_0084_track_library_feed_key_carries_its_feed_id() {
    let key = TrackKey {
        release: Some(ReleaseKey::LibraryFeed(42)),
        item_guid: "item-guid".to_string(),
    };

    assert_eq!(key.release, Some(ReleaseKey::LibraryFeed(42)));
    assert_ne!(key.release, Some(ReleaseKey::FeedGuid("42".to_string())));
}

/// R3-03 (ADR 0084 Decision 4): the resolver gives the Library source when
/// the lookup finds the track, and the Index source when it does not. A
/// key with no release gives the Index source, whatever the lookup gives.
#[test]
fn adr_0084_track_resolver_follows_the_lookup_result() {
    let feed_guid_key = TrackKey::from_index_track_target(Some("guid"), "item-guid");
    assert_eq!(
        resolve_track_page_source(&feed_guid_key, Some(9)),
        TrackPageSource::Library { track_id: 9 },
        "a found track resolves to the Library source"
    );
    assert_eq!(
        resolve_track_page_source(&feed_guid_key, None),
        TrackPageSource::Index {
            target: "guid:item-guid".to_string()
        },
        "no found track resolves to the Index source"
    );

    let no_release_key = TrackKey::from_index_track_target(None, "item-guid");
    assert_eq!(
        resolve_track_page_source(&no_release_key, None),
        TrackPageSource::Index {
            target: "item-guid".to_string()
        },
        "a key with no release gives the Index source"
    );
    assert_eq!(
        resolve_track_page_source(&no_release_key, Some(9)),
        TrackPageSource::Index {
            target: "item-guid".to_string()
        },
        "a key with no release gives the Index source even when a lookup result is given"
    );
}

/// R3-06 support (ADR 0084 Decision 4): a completed single-track download
/// changes only the open track page of that exact track.
#[test]
fn adr_0084_track_download_updates_only_the_open_track_page() {
    let open =
        FrameNavigationEntry::Track(TrackKey::from_index_track_target(Some("feed-a"), "item-a"));
    let other =
        FrameNavigationEntry::Track(TrackKey::from_index_track_target(Some("feed-a"), "item-b"));
    let downloaded = TrackKey::from_index_track_target(Some("feed-a"), "item-a");

    assert!(track_download_updates_open_page(
        Some(&open),
        Some(&downloaded)
    ));
    assert!(
        !track_download_updates_open_page(Some(&other), Some(&downloaded)),
        "a download must not change another open track"
    );
    assert!(
        !track_download_updates_open_page(Some(&discover()), Some(&downloaded)),
        "a download must not change a page that is not a track"
    );
    assert!(
        !track_download_updates_open_page(Some(&open), None),
        "a download with no key must not change the open page"
    );
}

/// R3-06 support (ADR 0084 Decision 4): a completed album download changes
/// only the open track page of a track in the downloaded release.
#[test]
fn adr_0084_track_release_download_updates_only_the_open_track_page() {
    let open =
        FrameNavigationEntry::Track(TrackKey::from_index_track_target(Some("feed-a"), "item-a"));
    let other =
        FrameNavigationEntry::Track(TrackKey::from_index_track_target(Some("feed-b"), "item-b"));
    let no_release = FrameNavigationEntry::Track(TrackKey::from_index_track_target(None, "item-c"));

    assert!(track_release_download_updates_open_page(
        Some(&open),
        Some("feed-a")
    ));
    assert!(
        !track_release_download_updates_open_page(Some(&other), Some("feed-a")),
        "a download must not change a track of a different release"
    );
    assert!(
        !track_release_download_updates_open_page(Some(&no_release), Some("feed-a")),
        "a track with no release never matches a downloaded release"
    );
    assert!(
        !track_release_download_updates_open_page(Some(&discover()), Some("feed-a")),
        "a download must not change a page that is not a track"
    );
    assert!(
        !track_release_download_updates_open_page(Some(&open), None),
        "a download with no feed GUID must not change the open page"
    );
}

/// Returns the focused frame of `layout`, through its live accessors.
fn focused_frame_of(layout: &WorkspaceLayout) -> Option<&WorkspaceFrameState> {
    let focused_id = layout.focused_frame_id()?;
    layout
        .frames()
        .iter()
        .find(|frame| frame.id() == focused_id)
}

/// Returns the id of the Detail frame of `layout`.
fn detail_frame_id(layout: &WorkspaceLayout) -> WorkspaceFrameId {
    layout
        .frames()
        .iter()
        .find(|frame| frame.kind() == WorkspaceFrameKind::Detail)
        .map(WorkspaceFrameState::id)
        .expect("the default layout has a Detail frame")
}

// ---------------------------------------------------------------------------
// ADR 0084 Decision 1: the three-column Music shell (task 004).
// ---------------------------------------------------------------------------

/// C4-02: at the breakpoint and wider, all three columns show, whatever the
/// page state.
#[test]
fn adr_0084_columns_c4_02_three_columns_show_at_the_breakpoint_and_wider() {
    assert_eq!(
        MusicColumns::for_width(940.0, 940.0, false),
        MusicColumns::Three
    );
    assert_eq!(
        MusicColumns::for_width(1200.0, 940.0, true),
        MusicColumns::Three
    );
}

/// C4-02: narrower than the breakpoint, the open-page state picks the list
/// or the detail column. The sidebar always shows.
#[test]
fn adr_0084_columns_c4_02_narrow_width_picks_list_or_detail_by_page_state() {
    let no_page_open = MusicColumns::for_width(700.0, 940.0, false);
    assert_eq!(no_page_open, MusicColumns::SidebarList);
    assert!(no_page_open.shows_list());
    assert!(!no_page_open.shows_detail());

    let page_open = MusicColumns::for_width(700.0, 940.0, true);
    assert_eq!(page_open, MusicColumns::SidebarDetail);
    assert!(!page_open.shows_list());
    assert!(page_open.shows_detail());
}

/// C4-03: the Detail frame chrome shows Back and Forward always, with each
/// control's typed availability still visible through `back`/`forward`.
#[test]
fn adr_0084_columns_c4_03_detail_frame_shows_history_controls_always() {
    let frame = frame(3, WorkspaceFrameKind::Detail);
    let nav = FrameNavigationState::new(discover());
    let display =
        FrameShellDisplay::from_frame(&frame, &nav, false).with_history_controls_always_shown();

    assert!(display.back_shown());
    assert!(display.forward_shown());
    assert!(display.back.disabled, "no back history exists yet");
}

/// C4-03: the `ContentList` frame chrome in Music shows neither Back nor
/// Forward, even when the projected navigation has history.
#[test]
fn adr_0084_columns_c4_03_content_list_frame_hides_history_controls() {
    let frame = frame(2, WorkspaceFrameKind::ContentList);
    let mut nav = FrameNavigationState::new(discover());
    nav.push(track(1));
    assert!(nav.can_go_back());
    let display = FrameShellDisplay::from_frame(&frame, &nav, false).with_history_controls_hidden();

    assert!(!display.back_shown());
    assert!(!display.forward_shown());
}

/// C4-04: the breadcrumb attaches to the Detail frame's display, not to a
/// `ContentList` display built from the same navigation state.
#[test]
fn adr_0084_columns_c4_04_breadcrumb_attaches_to_detail_not_content_list() {
    let mut nav = FrameNavigationState::new(discover());
    nav.push(track(1));
    let breadcrumb = BreadcrumbDisplay::project(
        "detail-breadcrumb",
        &nav,
        FrameNavigationEntry::display_label,
    );

    let detail_frame = frame(3, WorkspaceFrameKind::Detail);
    let detail_display =
        FrameShellDisplay::from_frame(&detail_frame, &nav, false).with_breadcrumb(breadcrumb);
    assert!(detail_display.breadcrumb.is_some());

    let content_list_frame = frame(2, WorkspaceFrameKind::ContentList);
    let content_list_display = FrameShellDisplay::from_frame(&content_list_frame, &nav, false);
    assert!(content_list_display.breadcrumb.is_none());
}

/// C4-02 (ADR 0084 Decisions 1 and 2): a list root opens no detail page, so
/// the narrow layout keeps the list column for it.
#[test]
fn adr_0084_columns_list_roots_open_no_detail_page() {
    for root in [
        FrameNavigationEntry::Destination(MusicDestination::Discover),
        FrameNavigationEntry::Search("drums".to_string()),
        FrameNavigationEntry::Destination(MusicDestination::NeedsAttention),
    ] {
        assert!(!root.opens_detail_page(), "{root:?} must open no page");
    }
    for page in [
        FrameNavigationEntry::PlaylistDetail(1),
        FrameNavigationEntry::Release(ReleaseKey::FeedGuid("feed".to_string())),
        FrameNavigationEntry::IndexNameMatches("Haleen".to_string()),
    ] {
        assert!(page.opens_detail_page(), "{page:?} must open a page");
    }
}

/// D5-02 (ADR 0084 Decisions 2 and 9): a sidebar click starts a new path
/// at the destination alone. A row of a destination list then gives the
/// path the destination, then the page.
#[test]
fn adr_0084_destinations_d5_02_sidebar_click_starts_a_path_a_row_extends_it() {
    let mut nav = FrameNavigationState::new(discover());

    nav.start_path(
        FrameNavigationEntry::Destination(MusicDestination::Releases),
        None,
    );
    assert_eq!(
        nav.path_entries(),
        vec![FrameNavigationEntry::Destination(
            MusicDestination::Releases
        )],
        "a sidebar click must give a path with the destination alone"
    );

    let release = FrameNavigationEntry::Release(ReleaseKey::LibraryFeed(7));
    nav.push(release.clone());
    assert_eq!(
        nav.path_entries(),
        vec![
            FrameNavigationEntry::Destination(MusicDestination::Releases),
            release
        ],
        "a row must give the path the destination, then the page"
    );
}

/// ADR 0084 task 015, operator fix 2026-10-17: `set_frame_title` renames
/// the named frame and leaves every other frame title untouched. An
/// unknown frame id gives `FrameNotFound`.
#[test]
fn adr_0084_singles_group_set_frame_title_renames_one_frame() {
    let mut layout = WorkspaceLayout::new(
        vec![
            frame(1, WorkspaceFrameKind::ContentList),
            frame(2, WorkspaceFrameKind::Detail),
        ],
        None,
    )
    .expect("two distinct frame ids make a valid layout");

    layout
        .set_frame_title(WorkspaceFrameId::new(2), "Singles by Haleen")
        .expect("frame 2 exists in this layout");

    assert_eq!(layout.frames()[1].title(), "Singles by Haleen");
    assert_eq!(
        layout.frames()[0].title(),
        WorkspaceFrameKind::ContentList.default_title(),
        "set_frame_title must leave the other frame title untouched"
    );

    assert_eq!(
        layout.set_frame_title(WorkspaceFrameId::new(99), "x"),
        Err(WorkspaceModelError::FrameNotFound(WorkspaceFrameId::new(
            99
        ))),
        "an unknown frame id must give FrameNotFound"
    );
}

/// D5-03 (ADR 0084 Decision 3): Music opens on `Destination(Discover)`,
/// the default navigation entry of each Music frame kind.
#[test]
fn adr_0084_destinations_d5_03_music_opens_on_discover() {
    for kind in [
        WorkspaceFrameKind::SourceList,
        WorkspaceFrameKind::ContentList,
    ] {
        assert_eq!(
            super::default_navigation_entry(kind),
            FrameNavigationEntry::Destination(MusicDestination::Discover),
            "{kind:?} must default to the Discover destination"
        );
    }
}

/// R15-10 (ADR 0084 Decision 6): a singles group entry opens a page in the
/// detail column, and its fallback label is "Singles".
#[test]
fn adr_0084_singles_group_r15_10_group_entry_opens_a_detail_page() {
    use crate::view_models::artist_identity::ArtistIdentity;
    use crate::view_models::singles_group::SinglesGroupKey;

    let entry = FrameNavigationEntry::SinglesGroup(SinglesGroupKey(ArtistIdentity::Name(
        "Haleen".to_string(),
    )));

    assert!(entry.opens_detail_page());
    assert_eq!(entry.display_label(), "Singles");
}
