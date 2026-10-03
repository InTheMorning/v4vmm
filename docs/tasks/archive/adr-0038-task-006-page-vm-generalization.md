# ADR 0038 Task 006: PageVm Generalization

## Status

Completed on 2026-05-04.

## Goal

Apply the ADR 0037 page-VM + shell-helper pattern to every entity
detail surface. Every page renders through a single shell helper that
consumes a `<Entity>DetailPageVm`. Screens supply hero images and
command callbacks; they do not assemble pages from individual VM
accessors.

## Surface Inventory

Already on the pattern:

- Feed/release detail — `ReleaseDetailPageVm`, ADR 0037 Pass 1.
- Track detail — `TrackDetailPageVm` now wraps `TrackDetailVm`, and
  Library/Discover track detail surfaces now pass through
  `src/ui/shells/track.rs::build_track_detail_surface`.
- Artist detail — `ArtistDetailPageVm` now carries shared artist header
  and fact rows, and Library/Discover artist detail surfaces render
  through `src/ui/shells/artist.rs::render_artist_detail_shell`.
- Playlist detail — `PlaylistDetailPageVm` wraps the existing
  `PlaylistDetailVm`, and Library playlist detail renders through
  `src/ui/shells/playlist.rs::render_playlist_detail_shell`.

Search results / recent-feed tiles stay row/list composites for ADR
0038. They are not entity detail pages, and their restored
reset-to-recents command is already VM-owned. Revisit only if a future
feature introduces page-level batch actions that need a batch PageVm.

## Completed Slices

### Track Detail PageVm Parity — 2026-05-04

- Added `TrackDetailPageVm` in `src/view_models/track_detail.rs`.
- Added `TrackDetailBehaviorSlots` and
  `build_track_detail_surface()` in `src/ui/shells/track.rs`.
- Moved Discover and Library track detail call sites off direct
  `TrackDetailSurface::new(...)` construction.
- Added
  `entity_detail_pages_render_through_shell_helper_and_page_vm` with an
  explicit release + track surface list.
- Added a VM unit test for the `TrackDetailPageVm` wrapper.

Per-slice visual smoke was deferred because the change is a
renderer-routing refactor with no intended visual output change. Final
Task 006 visual smoke is recorded below and in
`docs/reviews/adr-0038-review-checklist.md`.

### Artist Detail PageVm Parity — 2026-05-04

- Added `ArtistDetailPageVm` and `ArtistDetailFactVm` in
  `src/view_models/artist_detail.rs`.
- Projected Discover artist detail through `ArtistVm::page()` and the
  shared artist shell helper.
- Projected Library artist detail through `LibraryArtistDetailVm::page()`
  and the same shell helper, with feed rows kept as behavior slots.
- Expanded
  `entity_detail_pages_render_through_shell_helper_and_page_vm` to cover
  Library and Discover artist detail.
- Added VM unit tests for both Discover and Library artist page
  projections.

Per-slice visual smoke was deferred because the shell shape is
unchanged; the refactor moves page assembly ownership into the VM/shell
contract. Final Task 006 visual smoke is recorded below and in
`docs/reviews/adr-0038-review-checklist.md`.

### Playlist Detail PageVm Parity — 2026-05-04

- Added `PlaylistDetailPageVm` in
  `src/view_models/playlist_detail.rs`.
- Added `PlaylistDetailBehaviorSlots`, `PlaylistTrackRowSlot`, and
  `render_playlist_detail_shell()` in `src/ui/shells/playlist.rs`.
- Moved Library playlist detail off screen-local header, fact-grid,
  action-row, and track-row page assembly. `library.rs` now projects
  the existing `PlaylistDetailVm` into a page VM and supplies only
  thumbnail elements plus playlist command callbacks.
- Expanded
  `entity_detail_pages_render_through_shell_helper_and_page_vm` to cover
  Library playlist detail and `PlaylistDetailPageVm`.
- Expanded `shared_top_level_ui_shells_do_not_import_screen_modules` to
  include the new playlist shell.
- Added a VM unit test for the `PlaylistDetailPageVm` wrapper.

Per-slice visual smoke was deferred because the visible hierarchy is
intended to be unchanged; the refactor moves playlist page assembly
ownership into the VM/shell contract. Final Task 006 visual smoke is
recorded below and in `docs/reviews/adr-0038-review-checklist.md`.

## Visual Smoke

Completed on 2026-05-04 with operator-navigated app states and
transient `/tmp` captures. Per operator instruction, screenshot
artifacts are not retained or committed.

| Surface | Light | Dark | Fixture | Status |
|---|---|---|---|---|
| Feed/release detail | Covered by Task 004 | Covered by Task 004 | Library/Discover feed detail states | Verified 2026-05-04 |
| Track detail | Covered by Task 004 | Covered by Task 004 | Library/Discover track detail states | Verified 2026-05-04 |
| Library artist detail | Transient pass | Transient pass | `HeyCitizen` artist detail with feed section | Verified 2026-05-04 |
| Discover artist detail | Transient pass | Transient pass | `HeyCitizen` artist detail with feed tiles | Verified 2026-05-04 |
| Library playlist detail | Transient pass | Transient pass | `My Playlist` with seven tracks | Verified 2026-05-04 |

## Files Likely To Change

- `src/view_models/artist_detail.rs` — `ArtistDetailPageVm`.
- `src/view_models/playlist_detail.rs` (new) — `PlaylistDetailPageVm`.
- `src/view_models/track_detail.rs` — possibly a `TrackDetailPageVm`
  wrapper for parity with `ReleaseDetailPageVm`.
- `src/ui/shells/artist.rs`, `src/ui/shells/track.rs`, plus a new
  `src/ui/shells/playlist.rs` — shell helpers consuming the PageVms.
- `src/library.rs`, `src/search.rs` — call sites.
- `tests/architecture_tests.rs` — new guard
  `entity_detail_pages_render_through_shell_helper_and_page_vm`.

## Open Questions

1. **Granularity.** Does `PlaylistDetailPageVm` belong as a sibling of
   `ReleaseDetailPageVm`, or as a wrapper around the existing playlist
   VM? Decide when starting; lean toward sibling (parity).
2. **Cross-cutting `EntityActionTarget`.** Adding `Playlist`,
   `Artist`, `SearchResult` variants may require updates to every
   `match` site in `src/view_models/entity_detail.rs`. Audit the
   downstream impact before extending the enum.
3. **Empty-state policy.** Each PageVm needs an empty-state contract:
   what does the page show when the entity has no tracks, no
   contributors, no actions? Document per PageVm.
4. **Image resolution stays in screens.** Confirm that hero images and
   per-row thumbnails are still resolved screen-side and passed into
   the shell helper via slots, matching ADR 0037.

## Constraints

- Each PageVm migration is a single composite + one shell helper +
  caller updates. Do not bundle multiple entity types.
- The shell helper must be the only place that converts a PageVm into
  GPUI elements. Screens never do this work.
- Reuse `EntityActionVm.payload` for clickable identity/external-link
  actions. Don't introduce a parallel mechanism.

## Definition of Done

- Every entity detail surface renders through a `<Entity>DetailPageVm`
  + shell helper.
- The new guard
  `entity_detail_pages_render_through_shell_helper_and_page_vm` is
  green with an explicit surface list.
- VM unit tests cover each PageVm.
- Visual smoke (light + dark) for every migrated surface.

## Remaining Work

None for Task 006. Task 007 owns screen decomposition, and Task 008 owns
the final readiness gate and any final visual acceptance evidence.

## When To Start

After Tasks 002 (composite contracts), 003 (VM consolidation), 004
(dark mode), and 005 (a11y) all land. PageVm generalization consumes
the cleaned contracts and produces page-level shapes that subsequent
decomposition (Task 007) relies on.
