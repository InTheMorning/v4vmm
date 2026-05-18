use super::{
    BreadcrumbDisplay, BreadcrumbTruncation, ContentFilter, FilterChipStripDisplay,
    FrameDetachEligibility, FrameDockTarget, FrameNavigationEntry, FrameNavigationState,
    FrameSearchDescriptor, FrameSearchScope, FrameShellDisplay, WorkspaceFrameConfig,
    WorkspaceFrameId, WorkspaceFrameKind, WorkspaceFrameState, WorkspaceLayout,
    WorkspaceLayoutConfig, WorkspaceModelError,
};

fn frame(id: u64, kind: WorkspaceFrameKind) -> WorkspaceFrameState {
    WorkspaceFrameState::with_default_title(WorkspaceFrameId::new(id), kind)
}

fn descriptor_for(
    frame_id: u64,
    kind: WorkspaceFrameKind,
    nav: FrameNavigationEntry,
) -> FrameSearchDescriptor {
    let frame_id = WorkspaceFrameId::new(frame_id);
    let mut layout = WorkspaceLayout::new(vec![frame(frame_id.value(), kind)], Some(frame_id))
        .expect("single-frame descriptor layout should be valid");
    layout
        .reset_nav(frame_id, nav)
        .expect("single-frame descriptor layout should have navigation");
    layout
        .focused_search_descriptor()
        .expect("focused frame should project a descriptor")
}

#[test]
fn focused_search_descriptor_returns_none_for_empty_layout() {
    let layout = WorkspaceLayout::empty();

    assert_eq!(
        layout.focused_search_descriptor(),
        None,
        "empty layouts should not project a search descriptor"
    );
}

#[test]
fn focused_search_descriptor_projects_source_list_sidebar_search() {
    let descriptor = descriptor_for(
        11,
        WorkspaceFrameKind::SourceList,
        FrameNavigationEntry::SourceList,
    );

    assert_eq!(
        descriptor,
        FrameSearchDescriptor {
            frame_id: WorkspaceFrameId::new(11),
            kind: WorkspaceFrameKind::SourceList,
            nav: FrameNavigationEntry::SourceList,
            scope: FrameSearchScope::Sidebar,
            placeholder: "Filter sidebar...",
        },
        "source-list focus should filter sidebar rows"
    );
}

#[test]
fn focused_search_descriptor_projects_content_source_list_as_library_rows() {
    let descriptor = descriptor_for(
        12,
        WorkspaceFrameKind::ContentList,
        FrameNavigationEntry::SourceList,
    );

    assert_eq!(descriptor.frame_id, WorkspaceFrameId::new(12));
    assert_eq!(descriptor.kind, WorkspaceFrameKind::ContentList);
    assert_eq!(descriptor.nav, FrameNavigationEntry::SourceList);
    assert_eq!(descriptor.scope, FrameSearchScope::LibraryRows);
    assert_eq!(descriptor.placeholder, "Search library...");
}

#[test]
fn focused_search_descriptor_projects_content_search_as_library_rows() {
    let descriptor = descriptor_for(
        13,
        WorkspaceFrameKind::ContentList,
        FrameNavigationEntry::Search("ambient".to_string()),
    );

    assert_eq!(
        descriptor.nav,
        FrameNavigationEntry::Search("ambient".to_string()),
        "descriptor should clone the current navigation entry"
    );
    assert_eq!(descriptor.scope, FrameSearchScope::LibraryRows);
    assert_eq!(descriptor.placeholder, "Search library...");
}

#[test]
fn focused_search_descriptor_projects_content_settings_as_settings_rows() {
    let descriptor = descriptor_for(
        14,
        WorkspaceFrameKind::ContentList,
        FrameNavigationEntry::Settings,
    );

    assert_eq!(descriptor.kind, WorkspaceFrameKind::ContentList);
    assert_eq!(descriptor.nav, FrameNavigationEntry::Settings);
    assert_eq!(descriptor.scope, FrameSearchScope::SettingsRows);
    assert_eq!(descriptor.placeholder, "Search settings...");
}

#[test]
fn focused_search_descriptor_projects_detail_search_as_inspector_query() {
    let descriptor = descriptor_for(
        15,
        WorkspaceFrameKind::Detail,
        FrameNavigationEntry::Search("drums".to_string()),
    );

    assert_eq!(descriptor.kind, WorkspaceFrameKind::Detail);
    assert_eq!(
        descriptor.nav,
        FrameNavigationEntry::Search("drums".to_string())
    );
    assert_eq!(descriptor.scope, FrameSearchScope::InspectorQuery);
    assert_eq!(descriptor.placeholder, "Refine search...");
}

#[test]
fn focused_search_descriptor_projects_detail_entity_as_detail_tracks() {
    for nav in [
        FrameNavigationEntry::PlaylistDetail(3),
        FrameNavigationEntry::TrackDetail(4),
        FrameNavigationEntry::AlbumDetail(5),
        FrameNavigationEntry::ArtistDetail("Dawn Chorus".to_string()),
        FrameNavigationEntry::IndexArtistFeedScope("Dawn Chorus".to_string()),
        FrameNavigationEntry::IndexFeedDetail {
            id: "feed-guid".to_string(),
            label: "Dawn Chorus Feed".to_string(),
        },
        FrameNavigationEntry::IndexTrackDetail {
            id: "feed-guid:track-guid".to_string(),
            label: "Morning Theme".to_string(),
        },
    ] {
        let descriptor = descriptor_for(16, WorkspaceFrameKind::Detail, nav.clone());

        assert_eq!(
            descriptor.nav, nav,
            "entity-detail descriptor should carry the current navigation entry"
        );
        assert_eq!(descriptor.scope, FrameSearchScope::DetailTracks);
        assert_eq!(descriptor.placeholder, "Filter tracks...");
    }
}

#[test]
fn focused_search_descriptor_projects_queue_rows() {
    let descriptor = descriptor_for(
        17,
        WorkspaceFrameKind::QueueNowPlaying,
        FrameNavigationEntry::QueueNowPlaying,
    );

    assert_eq!(descriptor.frame_id, WorkspaceFrameId::new(17));
    assert_eq!(descriptor.kind, WorkspaceFrameKind::QueueNowPlaying);
    assert_eq!(descriptor.nav, FrameNavigationEntry::QueueNowPlaying);
    assert_eq!(descriptor.scope, FrameSearchScope::QueueRows);
    assert_eq!(descriptor.placeholder, "Filter queue...");
}

#[test]
fn filter_chip_strip_defaults_use_standard_option_order() {
    let content_list = FilterChipStripDisplay::default_for_content_list(ContentFilter::All, true);
    let search_inspector =
        FilterChipStripDisplay::default_for_search_inspector(ContentFilter::All, true);

    let content_values: Vec<_> = content_list
        .options
        .iter()
        .map(|option| option.value)
        .collect();
    let search_values: Vec<_> = search_inspector
        .options
        .iter()
        .map(|option| option.value)
        .collect();

    assert_eq!(
        content_values,
        [
            ContentFilter::All,
            ContentFilter::Library,
            ContentFilter::Index
        ],
        "content-list filters should keep the ADR 0047 option order"
    );
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
fn filter_chip_strip_defaults_round_trip_selected_filter() {
    let content_list =
        FilterChipStripDisplay::default_for_content_list(ContentFilter::Library, true);
    let search_inspector =
        FilterChipStripDisplay::default_for_search_inspector(ContentFilter::Index, true);

    assert_eq!(
        content_list.selected,
        ContentFilter::Library,
        "content-list filter display should preserve the selected filter"
    );
    assert_eq!(
        search_inspector.selected,
        ContentFilter::Index,
        "search-inspector filter display should preserve the selected filter"
    );
}

#[test]
fn filter_chip_strip_defaults_pass_through_narrow_collapse() {
    let expanded = FilterChipStripDisplay::default_for_content_list(ContentFilter::All, false);
    let collapsed = FilterChipStripDisplay::default_for_search_inspector(ContentFilter::All, true);

    assert!(
        !expanded.narrow_collapse_to_pulldown,
        "content-list filter display should preserve expanded narrow-mode preference"
    );
    assert!(
        collapsed.narrow_collapse_to_pulldown,
        "search-inspector filter display should preserve collapsed narrow-mode preference"
    );
}

#[test]
fn frame_shell_display_disables_empty_history_navigation() {
    let frame = frame(7, WorkspaceFrameKind::Detail);
    let nav = FrameNavigationState::new(FrameNavigationEntry::TrackDetail(42));

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

    nav.push(FrameNavigationEntry::TrackDetail(42));
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
    let nav = FrameNavigationState::new(FrameNavigationEntry::TrackDetail(42));

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
        display.breadcrumb, None,
        "breadcrumbs are opt-in frame chrome"
    );
}

#[test]
fn frame_shell_display_accepts_optional_filter_chip_strip() {
    let frame = frame(7, WorkspaceFrameKind::ContentList);
    let nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));
    let filters = FilterChipStripDisplay::default_for_content_list(ContentFilter::Library, true);

    let display =
        FrameShellDisplay::from_frame(&frame, &nav, true).with_filter_chip_strip(filters.clone());

    assert_eq!(
        display.filter_chip_strip,
        Some(filters),
        "frame shell should carry optional frame-local filters without applying them"
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
fn breadcrumb_display_projects_navigation_path() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));
    nav.push(FrameNavigationEntry::TrackDetail(42));

    let display =
        BreadcrumbDisplay::project("library-track-breadcrumb", &nav, |entry| match entry {
            FrameNavigationEntry::PlaylistDetail(_) => "My Playlist".to_string(),
            FrameNavigationEntry::TrackDetail(_) => "Lantern Tide".to_string(),
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

#[test]
fn breadcrumb_display_projects_index_search_drilldown_path() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::SourceList);
    nav.push(FrameNavigationEntry::Search("survival guide".to_string()));
    nav.push(FrameNavigationEntry::IndexArtistFeedScope(
        "Survival Guide".to_string(),
    ));
    nav.push(FrameNavigationEntry::IndexFeedDetail {
        id: "feed-guid".to_string(),
        label: "deathdreams".to_string(),
    });

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
            "Library",
            "Search: survival guide",
            "Survival Guide",
            "deathdreams"
        ]
    );
    assert_eq!(
        display.segments[2].target,
        Some(FrameNavigationEntry::IndexArtistFeedScope(
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
            FrameNavigationEntry::SourceList,
            FrameNavigationEntry::Search("survival guide".to_string()),
            FrameNavigationEntry::IndexArtistFeedScope("Survival Guide".to_string()),
            FrameNavigationEntry::IndexFeedDetail {
                id: "feed-guid".to_string(),
                label: "deathdreams".to_string(),
            },
        ],
        "breadcrumb labelers need the same full path rendered by frame chrome"
    );
}

#[test]
fn breadcrumb_display_projects_single_segment_as_current() {
    let nav = FrameNavigationState::new(FrameNavigationEntry::TrackDetail(42));

    let display = BreadcrumbDisplay::project("crumbs", &nav, |entry| match entry {
        FrameNavigationEntry::TrackDetail(id) => format!("Track {id}"),
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
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::SourceList);
    nav.push(FrameNavigationEntry::PlaylistDetail(7));
    nav.push(FrameNavigationEntry::TrackDetail(42));
    nav.push(FrameNavigationEntry::AlbumDetail(11));

    let display = BreadcrumbDisplay::project("crumbs", &nav, |entry| match entry {
        FrameNavigationEntry::SourceList => "Library".to_string(),
        FrameNavigationEntry::PlaylistDetail(id) => format!("Playlist {id}"),
        FrameNavigationEntry::TrackDetail(id) => format!("Track {id}"),
        FrameNavigationEntry::AlbumDetail(id) => format!("Album {id}"),
        _ => unreachable!("long-path test should only project breadcrumb entries"),
    });

    assert_eq!(display.segments.len(), 4);
    assert_eq!(display.segments[0].label, "Library");
    assert_eq!(
        display.segments[0].target,
        Some(FrameNavigationEntry::SourceList)
    );
    assert_eq!(display.segments[1].label, "Playlist 7");
    assert_eq!(
        display.segments[1].target,
        Some(FrameNavigationEntry::PlaylistDetail(7))
    );
    assert_eq!(display.segments[2].label, "Track 42");
    assert_eq!(
        display.segments[2].target,
        Some(FrameNavigationEntry::TrackDetail(42))
    );
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
        .reset_nav(second, FrameNavigationEntry::TrackDetail(42))
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
        &FrameNavigationEntry::TrackDetail(42)
    );

    assert_eq!(
        layout.pop_nav(first),
        None,
        "a single-entry history should not go back"
    );
    layout
        .push_nav(first, FrameNavigationEntry::TrackDetail(99))
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
        &FrameNavigationEntry::TrackDetail(42)
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
            WorkspaceFrameKind::QueueNowPlaying,
        ],
        "default workspace should expose the ADR 0046 frame order"
    );
    assert_eq!(
        layout.focused_frame().map(WorkspaceFrameState::kind),
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
    let mut layout = WorkspaceLayout::empty();

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
        layout.focused_frame().map(WorkspaceFrameState::id),
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
        layout.focused_frame().map(WorkspaceFrameState::id),
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
    assert_eq!(
        layout.remove_frame(WorkspaceFrameId::new(99)),
        Err(WorkspaceModelError::FrameNotFound(WorkspaceFrameId::new(
            99
        ))),
        "removing a missing frame should return an error"
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
fn add_frame_appends_and_focuses_new_frame() {
    let mut layout = WorkspaceLayout::new(
        vec![
            frame(2, WorkspaceFrameKind::SourceList),
            frame(4, WorkspaceFrameKind::ContentList),
        ],
        Some(WorkspaceFrameId::new(2)),
    )
    .expect("initial layout should be valid");

    let id = layout
        .add_frame(WorkspaceFrameKind::Detail)
        .expect("typed frame addition should succeed");

    assert_eq!(
        id,
        WorkspaceFrameId::new(5),
        "add_frame should allocate the next stable frame id"
    );
    assert_eq!(
        layout.frames().last().map(WorkspaceFrameState::kind),
        Some(WorkspaceFrameKind::Detail),
        "add_frame should append the requested kind"
    );
    assert_eq!(
        layout.focused_frame_id(),
        Some(id),
        "add_frame should focus the new frame"
    );
    assert_eq!(
        layout
            .frames()
            .iter()
            .filter(|frame| frame.is_focused())
            .count(),
        1,
        "add_frame should preserve exactly one focused frame"
    );
}

#[test]
fn replace_nav_preserves_full_history_for_visible_layout_projection() {
    let mut source = WorkspaceLayout::default_layout();
    let detail_id = WorkspaceLayout::default_detail_frame_id();
    source
        .reset_nav(detail_id, FrameNavigationEntry::Search("jazz".to_string()))
        .expect("detail frame should exist");
    source
        .push_nav(detail_id, FrameNavigationEntry::TrackDetail(9))
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
    assert_eq!(nav.current(), &FrameNavigationEntry::TrackDetail(9));
    assert_eq!(
        nav.back_destination(),
        Some(&FrameNavigationEntry::Search("jazz".to_string())),
        "full navigation history should survive projection"
    );
}

#[test]
fn add_frame_state_rejects_duplicate_ids() {
    let mut layout = WorkspaceLayout::new(
        vec![frame(1, WorkspaceFrameKind::SourceList)],
        Some(WorkspaceFrameId::new(1)),
    )
    .expect("initial layout should be valid");

    assert_eq!(
        layout.add_frame_state(frame(1, WorkspaceFrameKind::Detail)),
        Err(WorkspaceModelError::DuplicateFrameId(
            WorkspaceFrameId::new(1)
        )),
        "adding an explicit duplicate frame should return an error"
    );
}

#[test]
fn removing_focused_frame_moves_focus_left_when_possible() {
    let mut layout = WorkspaceLayout::new(
        vec![
            frame(1, WorkspaceFrameKind::SourceList),
            frame(2, WorkspaceFrameKind::ContentList),
            frame(3, WorkspaceFrameKind::Detail),
        ],
        Some(WorkspaceFrameId::new(2)),
    )
    .expect("multi-frame layout should be valid");

    layout
        .remove_frame(WorkspaceFrameId::new(2))
        .expect("focused frame should be removable");

    assert_eq!(
        layout.focused_frame_id(),
        Some(WorkspaceFrameId::new(1)),
        "focus should move to the left sibling after removing the focused frame"
    );
    assert_eq!(
        layout
            .frames()
            .iter()
            .filter(|frame| frame.is_focused())
            .count(),
        1,
        "layout should still mark exactly one focused frame"
    );
}

#[test]
fn removing_first_focused_frame_moves_focus_to_first_remaining_frame() {
    let mut layout = WorkspaceLayout::new(
        vec![
            frame(1, WorkspaceFrameKind::SourceList),
            frame(2, WorkspaceFrameKind::ContentList),
        ],
        Some(WorkspaceFrameId::new(1)),
    )
    .expect("multi-frame layout should be valid");

    layout
        .remove_frame(WorkspaceFrameId::new(1))
        .expect("focused frame should be removable");

    assert_eq!(
        layout.focused_frame_id(),
        Some(WorkspaceFrameId::new(2)),
        "first focused frame removal should focus the first remaining frame"
    );
}

#[test]
fn removing_last_frame_returns_error() {
    let mut layout = WorkspaceLayout::new(
        vec![frame(1, WorkspaceFrameKind::SourceList)],
        Some(WorkspaceFrameId::new(1)),
    )
    .expect("single-frame layout should be valid");

    assert_eq!(
        layout.remove_frame(WorkspaceFrameId::new(1)),
        Err(WorkspaceModelError::LastFrameRemoval),
        "removing the last frame should be rejected"
    );
    assert_eq!(
        layout.focused_frame_id(),
        Some(WorkspaceFrameId::new(1)),
        "failed removal should preserve focus"
    );
}

#[test]
fn workspace_frame_kind_projects_detach_eligibility() {
    assert_eq!(
        WorkspaceFrameKind::SourceList.detach_eligibility(),
        FrameDetachEligibility::NotDetachable,
        "source list frames should stay anchored to the workspace"
    );
    for kind in [
        WorkspaceFrameKind::ContentList,
        WorkspaceFrameKind::Detail,
        WorkspaceFrameKind::QueueNowPlaying,
    ] {
        assert_eq!(
            kind.detach_eligibility(),
            FrameDetachEligibility::Detachable,
            "{kind:?} frames should be detach-eligible"
        );
    }
}

#[test]
fn detach_and_dock_requests_defer_for_detachable_frames() {
    let layout = WorkspaceLayout::default_layout();

    assert_eq!(
        layout.request_detach(WorkspaceFrameId::new(2)),
        Err(WorkspaceModelError::DetachDeferred(WorkspaceFrameId::new(
            2
        ))),
        "content-list detach should be recognized but deferred"
    );
    assert_eq!(
        layout.request_dock(WorkspaceFrameId::new(3), FrameDockTarget::Center),
        Err(WorkspaceModelError::DockDeferred {
            frame_id: WorkspaceFrameId::new(3),
            target: FrameDockTarget::Center,
        }),
        "detail dock should be recognized but deferred"
    );
    assert_eq!(
        layout.request_dock(WorkspaceFrameId::new(4), FrameDockTarget::Trailing),
        Err(WorkspaceModelError::DockDeferred {
            frame_id: WorkspaceFrameId::new(4),
            target: FrameDockTarget::Trailing,
        }),
        "queue dock should be recognized but deferred"
    );
}

#[test]
fn detach_and_dock_requests_reject_anchored_source_list() {
    let layout = WorkspaceLayout::default_layout();

    assert_eq!(
        layout.request_detach(WorkspaceFrameId::new(1)),
        Err(WorkspaceModelError::NotDetachable(WorkspaceFrameId::new(1))),
        "source list detach should be rejected"
    );
    assert_eq!(
        layout.request_dock(WorkspaceFrameId::new(1), FrameDockTarget::Leading),
        Err(WorkspaceModelError::NotDetachable(WorkspaceFrameId::new(1))),
        "source list dock should be rejected"
    );
}

#[test]
fn detach_and_dock_requests_validate_frame_id() {
    let layout = WorkspaceLayout::default_layout();

    assert_eq!(
        layout.request_detach(WorkspaceFrameId::new(99)),
        Err(WorkspaceModelError::FrameNotFound(WorkspaceFrameId::new(
            99
        ))),
        "detaching a missing frame should return FrameNotFound"
    );
    assert_eq!(
        layout.request_dock(WorkspaceFrameId::new(99), FrameDockTarget::Center),
        Err(WorkspaceModelError::FrameNotFound(WorkspaceFrameId::new(
            99
        ))),
        "docking a missing frame should return FrameNotFound"
    );
}

#[test]
fn workspace_layout_config_round_trips() {
    let mut layout = WorkspaceLayout::default_layout();
    let added = layout
        .add_frame(WorkspaceFrameKind::Detail)
        .expect("adding a frame should succeed");

    let restored = WorkspaceLayout::from_config(Some(&layout.to_config()));

    assert_eq!(
        restored.to_config(),
        layout.to_config(),
        "config conversion should preserve frame order, kinds, and focus"
    );
    assert_eq!(
        restored.focused_frame_id(),
        Some(added),
        "config conversion should preserve focused frame id"
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
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::SourceList);

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
    nav.push(FrameNavigationEntry::TrackDetail(42));

    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::TrackDetail(42),
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
        Ok(FrameNavigationEntry::TrackDetail(42)),
        "go_forward should restore the pushed entry"
    );
    assert!(
        !nav.can_go_forward(),
        "round-trip should consume forward history"
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
fn navigation_back_destination_peeks_without_mutating() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));

    assert_eq!(
        nav.back_destination(),
        None,
        "initial navigation should not have a back destination"
    );

    nav.push(FrameNavigationEntry::TrackDetail(42));

    assert_eq!(
        nav.back_destination(),
        Some(&FrameNavigationEntry::PlaylistDetail(7)),
        "back destination should expose the previous entry"
    );
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::TrackDetail(42),
        "peeking should not change current navigation"
    );
}

#[test]
fn navigation_reset_clears_back_and_forward_history() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));
    nav.push(FrameNavigationEntry::TrackDetail(42));
    nav.go_back()
        .expect("pushed navigation should allow going back");

    nav.reset(FrameNavigationEntry::TrackDetail(99));

    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::TrackDetail(99),
        "reset should replace the current entry"
    );
    assert!(!nav.can_go_back(), "reset should clear back history");
    assert!(!nav.can_go_forward(), "reset should clear forward history");
}

#[test]
fn has_history_returns_false_for_fresh_state() {
    let nav = FrameNavigationState::new(FrameNavigationEntry::SourceList);

    assert!(
        !nav.has_history(),
        "fresh navigation state should have no history"
    );
}

#[test]
fn has_history_returns_true_after_push() {
    let mut nav = FrameNavigationState::new(FrameNavigationEntry::SourceList);
    nav.push(FrameNavigationEntry::PlaylistDetail(7));

    assert!(
        nav.has_history(),
        "navigation state with a back-stack entry should have history"
    );
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
        nav.back_destination(),
        Some(&FrameNavigationEntry::PlaylistDetail(7)),
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
        nav.back_destination(),
        Some(&FrameNavigationEntry::PlaylistDetail(7)),
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
        nav.back_destination(),
        Some(&FrameNavigationEntry::PlaylistDetail(7)),
        "back should skip the previous search and selected detail"
    );
}

#[test]
fn pop_nav_until_pops_to_target() {
    let mut layout = WorkspaceLayout::default_layout();
    let detail_id = WorkspaceFrameId::new(3);

    layout
        .reset_nav(detail_id, FrameNavigationEntry::PlaylistDetail(1))
        .expect("should reset Detail nav");
    layout
        .push_nav(detail_id, FrameNavigationEntry::TrackDetail(42))
        .expect("should push track detail");
    layout
        .push_nav(detail_id, FrameNavigationEntry::AlbumDetail(5))
        .expect("should push album detail");

    layout
        .pop_nav_until(detail_id, &FrameNavigationEntry::PlaylistDetail(1))
        .expect("should pop until playlist detail");

    let nav = layout
        .frame_nav(detail_id)
        .expect("Detail should have navigation state");
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::PlaylistDetail(1),
        "should be back at the playlist detail after pop_nav_until"
    );
}

#[test]
fn pop_nav_until_noop_when_already_at_target() {
    let mut layout = WorkspaceLayout::default_layout();
    let detail_id = WorkspaceFrameId::new(3);

    layout
        .reset_nav(detail_id, FrameNavigationEntry::TrackDetail(42))
        .expect("should reset Detail nav");

    layout
        .pop_nav_until(detail_id, &FrameNavigationEntry::TrackDetail(42))
        .expect("should not error when already at target");

    let nav = layout
        .frame_nav(detail_id)
        .expect("Detail should have navigation state");
    assert_eq!(
        nav.current(),
        &FrameNavigationEntry::TrackDetail(42),
        "should remain at the target entry"
    );
    assert!(!nav.can_go_back(), "back history should remain empty");
}
