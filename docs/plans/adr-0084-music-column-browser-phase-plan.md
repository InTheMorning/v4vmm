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
| 004 | 3 | The three-column shell: the destinations, the collapsible sidebar, Discover with "New releases", Needs attention and Playlists (Decisions 1 to 3). It deletes the sidebar tree and fixes finding 5 | 003, ADR 0081 task 001 | Not written |
| 005 | 4 | Releases: the kind, the two scopes, the singles groups and the group page (Decisions 5, 6 and 8) | 004 | Not written |
| 006 | 5 | The artist page with two levels and its scope (Decisions 7 and 8). Fixes finding 6 | 005, ADR 0082 tasks 002 and 003 | Not written |
| 007 | 6 | Multiple selection, the action bar and the reserved audition place (Decisions 10 and 11). Fixes finding 3 | 004 | Not written |
| 008 | 7 | Search results in the list column, and saved searches as Discover views (Decisions 2 and 3) | 004 | Not written |
| 009 | 8 | The logo frame for collections (Decision 12) | 005, 006 | Not written |

The overhaul plan names eight steps. Step 2 has two packets, 002 and 003, because the release route and the track route each change many files.

## Sequence

1. Packets 002 and 003, one at a time. Each packet runs in its own session.
2. [ADR 0081 task 001](../tasks/adr-0081-task-001-remove-the-staged-frame-model.md) before packet 004. Both change `src/view_models/workspace/`, and packet 004 changes the frame layout.
3. [ADR 0082 tasks 002 and 003](adr-0082-publisher-roles-phase-plan.md) before packet 006. Packet 006 replaces the publisher page layout that task 002 changes.
4. Packets 004 to 009 in register order. Write each packet at the start of its session, from the result of the packet before it.

## Open Findings

- 2026-10-11: a track row on an Index release page has no click action, so it opens no track page. An Index track page opens only from a search result or a name-match page. ADR 0084 Decision 4 gives each track one page, and a Library release page row opens its track page. No packet owns this yet. Packet 007, the shared row and selection model, is the first packet that changes these rows (`index_feed_track_rows` in `src/app/search_dispatch.rs`).

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
