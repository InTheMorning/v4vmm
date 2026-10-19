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
| [007](../tasks/archive/adr-0084-task-007-release-kind.md) | 4 | The release kind: one `ReleaseKind` function, the kind on the release page, the release badges and the Releases rows, and action labels such as "Download single" (Decision 5). The orchestrator split the old packet 007 into 007, 015 and 016 on 2026-10-17 | 006 | Complete 2026-10-17 |
| [008](../tasks/archive/adr-0084-task-008-feed-page-and-artists.md) | 5 | The feed level of the artist page (Decision 7). It adds the level, the sections, one page shape, the Artists list and the ADR 0085 guard. Packet 017 owns the name level. Packet 010 owns finding 6, and packet 016 owns the scope (operator decisions of 2026-10-18) | 007, 015, ADR 0082 task 003, Stophammer release 0.11.0 | Complete 2026-10-18. The operator passed V1 to V9 |
| 009 | 6 | Multiple selection, the action bar and the reserved audition place (Decisions 10 and 11). Fixes finding 3. It also gives keyboard selection and scroll in the list and detail columns. Up and Down move the selection through the visible rows and keep the selected row in view. Page Up, Page Down, Home and End scroll. It replaces `LibraryApp::move_down` and `move_up`, which still use the old Library selection model (operator decision of 2026-10-16) | 006 | Not written |
| 010 | 7 | The search results view for the list column width (Decision 2). It adds the search entry of a feed page and fixes finding 6 (operator decision of 2026-10-18). Task 004 put the present search view in the list column on 2026-10-12, by operator decision | 006 | Not written. Its search entry of a feed page waits for item 5 of Stophammer request 8 |
| 011 | 8 | The logo frame for collections (Decision 12). It also gives each Artists list row its image: today each row shows the monogram "AR" (`thumbnail_href: None` in `artist_list_row`, operator decision of 2026-10-18) | 007, 008 | Not written |
| 012 | 3 | Saved searches: their storage (a migration through the ADR 0016 registry), a "Save search" action on the search results, and one Discover view for each (Decisions 2 and 3). The operator decided on 2026-10-16 to add them as a separate packet | 006, 010 | Not written |
| [013](../tasks/archive/adr-0084-task-013-index-release-page-parity.md) | 3 | Index release page parity: the track row click, the description toggle, the Index track page description, and the collapsed description preview with "Show more" (Decision 4, ADR 0047 item 8 amended 2026-10-16). The operator decided on 2026-10-16 to do it before packet 007 | 006 | Complete 2026-10-17 |
| [014](../tasks/archive/adr-0084-task-014-music-column-scrollbars.md) | 3 | A scrollbar on each scrolling Music column: the sidebar, the list and the detail column, from one shared composite, with named tokens (Decision 1). Today a Music column scrolls only with a mouse wheel or a touchpad (operator decision of 2026-10-16) | 013 | Complete 2026-10-17 |
| [015](../tasks/archive/adr-0084-task-015-singles-groups.md) | 4 | The singles groups in the Releases list, the group page, and the single page link "All n singles" (Decision 6), in the Library scope. The group checkbox waits for packet 009, and the order "Albums and EPs" before "Singles" on the artist page waits for packet 008. It adds the artist identity type of Decision 7, which packet 008 uses | 007 | Complete 2026-10-17 |
| 016 | 4 | The scope control "Library" and "Everything" for Releases and Artists (Decision 8). It uses the name lists of Stophammer request 6 (operator decision of 2026-10-17): `GET /v1/feeds/by-name` and `GET /v1/artist-names`. It replaces the stored contract with the 0.11.0 contract, unless packet 008 did it first | 015, 008 | Not written. Waits for items 2 to 4 of Stophammer [request 8](v4vmm-open-requests.md), recorded on 2026-10-18 |
| 017 | 5 | The name level of the artist page (Decision 7). It replaces the Library artist page by name and the Index name match page. The operator split it from packet 008 on 2026-10-18. The old page shows each release with the artist text, also the releases that name a feed. The "Haleen · by name" row says 3 releases, and its page shows 6 (operator decision of 2026-10-18: this waits for packet 017) | 008 | Not written. Waits for item 4 of Stophammer request 8 |

The overhaul plan names eight steps. Step 2 has two packets, 002 and 003, because the release route and the track route each change many files. Step 3 has three packets, 004 to 006, decided on 2026-10-12: the icon-only sidebar needs the destinations, and Discover needs the destination model.

## Sequence

1. Packets 002 and 003, one at a time. Each packet runs in its own session.
2. [ADR 0081 task 001](../tasks/archive/adr-0081-task-001-remove-the-staged-frame-model.md) before packet 004. Complete on 2026-10-11. Both change `src/view_models/workspace/`, and packet 004 changes the frame layout.
3. [ADR 0082 tasks 002 and 003](adr-0082-publisher-roles-phase-plan.md) before packet 008. Packet 008 replaces the publisher page layout that task 002 changes.
4. Packets 013 and then 014 before packet 007, by operator decisions of 2026-10-16. Then packets 007, 015 and 016, then packets 008 to 012 and 017 in register order. Packet 017 comes after packet 008.
   Packets 016 and 017 wait for Stophammer request 8. Write each packet at the start of its session, from the result of the packet before it.

## Open Findings

- 2026-10-18: the Library stores one `publisher_feed_title` for each album response, the title of the primary party. ADR 0085 permits no other storage change, so a second named feed has no stored title.
  [ADR 0082 packet 003](../tasks/archive/adr-0082-task-003-release-page-named-feeds.md) records the effect. Albums with two named feeds are rare until podcast-namespace PR #793 is merged.
- 2026-10-18: Stophammer ADR 0071 declined item 6 of request 8. The Index `name` rows follow Stophammer ADR 0070 §4, and the Library follows ADR 0085 Decision 6.
  The two rules differ only for an album that names an agreed label first and a feed with no role second. No album did this on 2026-10-18. Packet 016 records the case and a check after each Stophammer release.
- 2026-10-18: a click on a row of the Artists list adds the page to the breadcrumb path, and does not replace the open page. The operator saw "Artists > HeyCitizen > Haleen > HeyCitizen" and "Artists > HeyCitizen > Elijah Lied".
  Decision 9 makes the breadcrumb a location, not a history. No packet owns this defect.
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
