# ADR 0037 Task 002: Track Header/Action Parity (Pass 2 Stub)

## Status

Stub. Not started. Sequenced to land after Task 001 so it can reuse the
`EntityActionVm.payload` extension introduced there.

## Goal

Make the same normal track render an identical header, summary, action row,
external-link strip, and lazy-section grammar across Library and Discover.
Library-only advanced metadata panels remain context-specific and visibly
additive.

## Files To Inspect (preliminary — re-audit when starting)

- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `src/ui_track.rs`
- `src/library.rs` (track-detail call sites)
- `src/search.rs` (Discover track-detail call sites)
- `src/view_models/track.rs`
- `src/view_models/track_detail.rs`
- `src/view_models/entity_detail.rs` (for action-VM patterns)
- `tests/architecture_tests.rs`

## Open Questions To Resolve Before Implementation

1. **Inventory the duplication.** Find every place where Library or Discover
   builds track-detail header text, action chrome, or external-link buttons
   locally instead of consuming a shared composite. Record file:line
   citations before designing the helper.
2. **External-link payload model.** Track external links (e.g., MusicBrainz
   URL, source URL, podcast episode link) must consume
   `EntityActionVm.payload` introduced in Task 001. Confirm the track VM
   populates payload for each link kind. If the existing track VM does not
   yet expose external-link actions in `Vec<EntityActionVm>` form, decide
   whether to (a) extend `TrackDetailVm` with an `external_links` action
   list, or (b) introduce a parallel contract. Default: (a), matching the
   feed approach.
3. **Action-row scope.** Track action rows include playback, download,
   playlist add, and metadata-panel toggles. Some of these have different
   states across Library and Discover (download visibility, in-library
   chip). Decide which are shared chrome vs. screen-specific commands. The
   ADR allows screen-bound primary actions; the goal is to share the
   *layout and ordering*, not collapse command dispatch.
4. **Library-only advanced panels.** Confirm the carve-out: MusicBrainz
   compare, advanced provenance grid, and similar panels stay
   Library-bound. Pass 2 must not touch their content.

## Sketch of Target State

- A new `render_track_action_row` (or extension to
  `render_release_detail_shell` for tracks) consumes the shared track VM
  and renders header, summary, action row, and external-link strip.
- Library and Discover track-detail screens supply only:
  - Hero image
  - Screen-bound command handlers
  - Library-only advanced panels (Library only)
- Architecture guard:
  `track_external_links_use_shared_renderer` (or similar) blocks screen-local
  external-link button construction.

## Constraints (carry over from Pass 1)

- Preserve all click behavior: open, copy, play, download, playlist add,
  metadata-compare, MusicBrainz lookup.
- Keep ElementId prefixes distinct per surface.
- Do not touch backend, schema, RSS/ID3, playback driver, or
  metadata-comparison logic.
- Both light and dark screenshots required for any surface this pass
  changes.

## Definition of Done

- Library and Discover track-detail screens use one shared header + action
  row composite.
- External links route through `EntityActionVm.payload`.
- Architecture guard added and green.
- Light + dark screenshots for both surfaces filed under
  `docs/reviews/screenshots/adr-0037-{library,discover}-track-detail-{light,dark}.png`.
- ADR 0037 status moves to Accepted once both passes are landed.

## When To Start

After Task 001 is merged and the `EntityActionVm.payload` field is available
on `master`. Before starting, replace this stub with a fully-specified task
following the same structure as Task 001 (concrete steps, pinned helper
signature, lower-context model prompt).
