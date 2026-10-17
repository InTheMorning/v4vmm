# ADR 0084 Music Column Browser Phase Plan

## Status

Active - 2026-10-08. This plan is advisory. It states no rule.
[ADR 0084](../adr/0084-music-column-browser.md) owns the rules. The [overhaul plan](design-and-cleanup-overhaul-plan.md#phase-4a-flow-and-information-architecture) owns the order of Phase 4A.

## Goal

Music becomes the column browser of ADR 0084. It gets one navigation stack, one page for each item, three columns and the destinations. It also gets the release kinds and groups, one artist page, one selection model and the logo frame.

## Non-Goals

- No audition player. [ADR 0068](../adr/0068-show-cue-and-audition-isolation.md) owns it. Packet 008 keeps its place only.
- No Show change, and no Settings change.
- No new MusicIndex endpoint. A packet that needs one stops and asks.
- No schema change unless a packet names it and adds its migration through the ADR 0016 registry.

## Assumptions

- The album page and the track page already have one view model for each origin (ADR 0083 tasks 004 and 005). Two assembly paths still exist: `LibraryApp` owns the Library page data, and `TopApp` owns the Index page data.
- A Library feed can have no feed GUID: `feeds.feed_guid` can be null. An Index track result can have no feed GUID (`split_index_track_target`).
- The operator walks the visual check of each packet before the next packet starts.

## Packet Register

| Packet | Plan step | Scope | Depends on | State |
|---|---|---|---|---|
| [001](../tasks/archive/adr-0084-task-001-one-navigation-stack.md) | 1 | One navigation stack and the location breadcrumb (Decision 9) | None | Complete 2026-10-08 |
| [002](../tasks/archive/adr-0084-task-002-one-release-route.md) | 2 | One route for each release (Decision 4). One navigation entry keyed by release identity, and one resolver that selects the data source | 001 | Complete 2026-10-11 |
| [003](../tasks/archive/adr-0084-task-003-one-track-route.md) | 2 | One route for each track, by the same method. A Library track row opens its track page. Fixes flow map finding 2 | 002 | Complete 2026-10-11 |
| [004](../tasks/archive/adr-0084-task-004-three-column-shell.md) | 3 | The three-column shell: the sidebar, the list and the detail as three frames. Back, Forward and the breadcrumb move to the detail column (Decisions 1 and 9) | 003, ADR 0081 task 001 | Complete 2026-10-12 |
| [005](../tasks/archive/adr-0084-task-005-destinations-sidebar.md) | 3 | The destinations sidebar: the six destinations, the icon-only sidebar and its attention mark, the destination lists, Needs attention and Playlists. It deletes the sidebar tree and fixes finding 5 (Decisions 1 and 2) | 004 | Complete 2026-10-16 |
| [006](../tasks/archive/adr-0084-task-006-discover.md) | 3 | Discover: the default destination, its views column and "New releases". It deletes the three-state filter control (Decision 8) and the saved-search code that no path loads (operator decision of 2026-10-16) (Decisions 3 and 8) | 005 | Complete 2026-10-16 |
| [007](../tasks/adr-0084-task-007-release-kind.md) | 4 | The release kind: one `ReleaseKind` function, the kind on the release page, the release badges and the Releases rows, and action labels such as "Download single" (Decision 5). The orchestrator split the old packet 007 into 007, 015 and 016 on 2026-10-17 | 006 | Implemented 2026-10-17. Mechanical checks Green. Visual check open |
| 008 | 5 | The artist page with two levels and its scope (Decisions 7 and 8). Fixes finding 6 | 007, ADR 0082 tasks 002 and 003 | Not written |
| 009 | 6 | Multiple selection, the action bar and the reserved audition place (Decisions 10 and 11). Fixes finding 3. It also gives keyboard selection and scroll in the list and detail columns. Up and Down move the selection through the visible rows and keep the selected row in view. Page Up, Page Down, Home and End scroll. It replaces `LibraryApp::move_down` and `move_up`, which still use the old Library selection model (operator decision of 2026-10-16) | 006 | Not written |
| 010 | 7 | The search results view for the list column width (Decision 2). Task 004 put the present search view in the list column on 2026-10-12, by operator decision | 006 | Not written |
| 011 | 8 | The logo frame for collections (Decision 12) | 007, 008 | Not written |
| 012 | 3 | Saved searches: their storage (a migration through the ADR 0016 registry), a "Save search" action on the search results, and one Discover view for each (Decisions 2 and 3). The operator decided on 2026-10-16 to add them as a separate packet | 006, 010 | Not written |
| [013](../tasks/archive/adr-0084-task-013-index-release-page-parity.md) | 3 | Index release page parity: the track row click, the description toggle, the Index track page description, and the collapsed description preview with "Show more" (Decision 4, ADR 0047 item 8 amended 2026-10-16). The operator decided on 2026-10-16 to do it before packet 007 | 006 | Complete 2026-10-17 |
| [014](../tasks/archive/adr-0084-task-014-music-column-scrollbars.md) | 3 | A scrollbar on each scrolling Music column: the sidebar, the list and the detail column, from one shared composite, with named tokens (Decision 1). Today a Music column scrolls only with a mouse wheel or a touchpad (operator decision of 2026-10-16) | 013 | Complete 2026-10-17 |
| 015 | 4 | The singles groups in the Releases list, the group page, and the single page link "All n singles" (Decision 6), in the Library scope. The group checkbox waits for packet 009, and the order "Albums and EPs" before "Singles" on the artist page waits for packet 008 | 007 | Not written |
| 016 | 4 | The scope control "Library" and "Everything" for Releases and Artists (Decision 8). It waits for Stophammer request 6, a list of releases and of artists by name (operator decision of 2026-10-17) | 015 | Not written. Waits for Stophammer |

The overhaul plan names eight steps. Step 2 has two packets, 002 and 003, because the release route and the track route each change many files. Step 3 has three packets, 004 to 006, decided on 2026-10-12: the icon-only sidebar needs the destinations, and Discover needs the destination model.

## Sequence

1. Packets 002 and 003, one at a time. Each packet runs in its own session.
2. [ADR 0081 task 001](../tasks/archive/adr-0081-task-001-remove-the-staged-frame-model.md) before packet 004. Complete on 2026-10-11. Both change `src/view_models/workspace/`, and packet 004 changes the frame layout.
3. [ADR 0082 tasks 002 and 003](adr-0082-publisher-roles-phase-plan.md) before packet 008. Packet 008 replaces the publisher page layout that task 002 changes.
4. Packets 013 and then 014 before packet 007, by operator decisions of 2026-10-16. Then packets 007, 015 and 016, then packets 008 to 012 in register order. Write each packet at the start of its session, from the result of the packet before it.

## Open Findings

- 2026-10-17: Decision 8 gives Releases and Artists a scope "Everything", which adds the Index items. MusicIndex 0.7.0 has no list of releases or artists by name. Only `/v1/feeds/recent` pages the Index, newest first, and `/v1/publishers` searches publishers by text. The operator decided on 2026-10-17 to ask Stophammer for a list of releases and of artists by name: request 6 of the [open requests](v4vmm-open-requests.md). Packet 016 waits for a release that gives it.
- 2026-10-17: a track, artist, playlist, publisher or name-match page has one fixed scroll id for each page type. Two pages of one type, opened one after the other, thus share one scroll position. A release page has one scroll id for each release since task 014. No packet owns this yet (`track_detail_scroll_id` and the other scroll ids in `src/view_models/library.rs` chrome, and `src/ui/shells/`).

## Affected Modules

- Navigation: `src/view_models/workspace/`, `src/app.rs`, `src/app/breadcrumb.rs`.
- Page data and routes: `src/app/search_dispatch.rs`, `src/library/app_impl.rs`, `src/view_models/library.rs`, `src/view_models/album_page.rs`, `src/view_models/track_detail.rs`.
- Renderers: `src/ui/shells/library/`, `src/ui/shells/search_results_inspector.rs`, `src/ui/shells/workspace.rs`.
- Guards: `tests/architecture_tests.rs`.

## Schema And API Implications

- Packets 002 and 003 add read queries only, for example a Library track by its feed GUID and item GUID. No migration.
- Packet 005 can need a stored release kind. The feed states it in RSS. A migration needs its own record in the packet.
- No packet changes the MusicIndex contract.

## Risk Areas

- Stale fetch results. Each Index fetch keeps its stale-result check when the route changes.
- In-place updates. A download changes the data source of the open page. The page must update without navigation.
- Guards that assert the Index routes of ADR 0049 and ADR 0024. Each packet deletes the guard text of a replaced rule in the same change.
- Width. Packet 004 adds a column. The narrow layout needs the collapsed sidebar.

## Test Strategy

- Each mechanical criterion is a unit test at the owning layer, with the prefix of its packet.
- Each packet adds or updates a situational guard that cites ADR 0084 and its decision.
- Each packet has a visual check that the operator walks, and a flow check that counts the steps of each curator job before and after.

## Rollback Strategy

Each packet is one commit with no migration, unless it names one. Revert the commit. A packet with a migration names its backup step in its `Status:` line.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line, in
[pending human checks](../pending-human-checks.md) and in the [delivery order](broadcast-chain-delivery-order.md).
