# ADR 0038 Task 006: PageVm Generalization (Stub)

## Status

Stub. Starts after Tasks 002, 003, 004, 005 land.

## Goal

Apply the ADR 0037 page-VM + shell-helper pattern to every entity
detail surface. Every page renders through a single shell helper that
consumes a `<Entity>DetailPageVm`. Screens supply hero images and
command callbacks; they do not assemble pages from individual VM
accessors.

## Surface Inventory

Already on the pattern:

- Feed/release detail — `ReleaseDetailPageVm`, ADR 0037 Pass 1.
- Track detail — `TrackDetailVm` + `TrackDetailSurface`, but verify
  whether a `TrackDetailPageVm` wrapper is needed for parity with the
  release pattern.

Not yet on the pattern:

- Artist detail — `src/ui/shells/artist.rs` (post-relocation).
- Playlist detail — currently spread across
  `library.rs`/`search.rs`/`view_models/library.rs`.
- Search results / recent-feed tiles — multi-row composite that may
  benefit from a page-level VM for batch operations.

## Files Likely To Change

- `src/view_models/artist_detail.rs` (new) — `ArtistDetailPageVm`.
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

## When To Start

After Tasks 002 (composite contracts), 003 (VM consolidation), 004
(dark mode), and 005 (a11y) all land. PageVm generalization consumes
the cleaned contracts and produces page-level shapes that subsequent
decomposition (Task 007) relies on.
