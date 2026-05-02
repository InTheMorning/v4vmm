# ADR 0035 Track Surface Consolidation - Phase Plan

## Status

Proposed - 2026-05-02.

## Goal

Consolidate track presentation across Library and Discover so the app has one
recognizable track HI structure, one display contract family, shared row,
inspector, and detail surface owners, and regression guards that stop
screen-local track UI drift.

## Non-Goals

- No backend or schema changes.
- No playlist or playback feature expansion.
- No MusicBrainz or ID3 behavior changes beyond moving existing panels into
  shared slots.
- No broad redesign of Library or Discover.
- No automated pointer/xdotool visual smoke.

## Assumptions

- ADR 0033 and ADR 0034 are active boundaries.
- `TrackHeader`, `TrackRow`, `ActionRow`, `DetailGrid`,
  `AddToPlaylistPopover`, `TrackMetadataGrid`, `FileHeader`, and
  `MusicBrainzPanel` are existing shared building blocks.
- Library and Discover can keep distinct commands if those commands are passed
  as slots into a shared surface.
- Image lookup stays screen-owned per ADR 0033; artwork display is owned by
  the shared composites per ADR 0035.

## Current State

- Library track detail composition lives in `render_track_window`,
  `render_track_left_column`, and `library_track_action_row`.
- Discover track detail composition lives in `render_discover_track_inspector`
  and `render_track_header_subtitle`.
- Library and Discover also compose track rows and inspector-pane track chrome
  locally enough that spacing, labels, fallback strings, and action placement
  can drift.
- Shared pieces exist, but summary labels, action placement, description
  placement, artwork handoff, loading states, row chrome, inspector chrome, and
  advanced panel placement are still screen-local.

## Target State

- `TrackDetailVm` projects shared display facts, labels, slots, fallback
  strings, and load state.
- `TrackRowVm` projects row-shaped data from `TrackDetailVm`; screens do not
  build row chrome from raw track rows.
- `TrackDetailSurface`, `TrackInspectorPane`, and `TrackRow` are the only
  composite owners for full detail, inspector-pane detail, and row layout.
- Library and Discover produce slot content for actions, advanced panels, and
  surface-specific lazy sections.
- Architecture tests fail if new screen-local track surface chrome, labels,
  fallbacks, untyped slots, or inspector composition are added.
- Visual smoke verifies Library full detail, Library inspector pane, Discover
  full detail, and Discover inspector pane for the same or comparable track.

## Affected Modules

- `src/view_models/track_detail.rs`
- `src/view_models/mod.rs`
- `src/ui/composites/track_detail_surface.rs`
- `src/ui/composites/track_inspector_pane.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/mod.rs`
- `src/ui/composites/track_header.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `src/ui_entity.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0035-review-checklist.md`

## Proposed Sequence

### Task 001 - Track Detail VM Contract

Add the GPUI-free display contract family. It owns track detail labels,
fallback policy, row projection, load state, typed slot descriptors, summary
row order, optional description, and section descriptors.

### Task 002 - Track Surface Composites

Add or bind the shared composites that own full detail, inspector-pane detail,
and row structure. They accept typed slots for actions, links, contributors,
value routes, advanced panels, and lazy sections.

### Task 003 - Discover Migration

Route Discover track rows, inspector pane, and full detail through the shared
VM/composite family. Preserve Discover-specific links/actions as slots.

### Task 004 - Library Migration

Route Library track rows, inspector pane, and full detail through the shared
VM/composite family. Preserve advanced ID3, MusicBrainz, staged tag edits, and
metadata grid behavior as slots.

### Task 005 - Guards and Visual Gate

Add the named ADR 0035 architecture tests, update ADR 0033 enforcing-test list
if needed, update the review checklist, run checks, and request screenshots for
visual smoke.

## Schema/API Implications

None.

## Risk Areas

- Accidentally weakening Library's advanced metadata workflows.
- Changing Discover action availability while moving UI into slots.
- Moving labels without preserving existing VM fallback behavior.
- Making slots too generic and therefore smuggling screen logic back into the
  composite.
- Letting the shared surface import backend/service types.
- Letting row or inspector-pane consolidation lag behind detail consolidation,
  which would preserve the drift path this ADR is meant to close.
- Breaking the build mid-migration: Task 002 must add the new `TrackRow`
  VM constructor *additively* and keep legacy callers compiling. Tasks 003
  and 004 migrate every caller; Task 005 deletes the transitional API.
- `src/ui_track.rs` and `TrackInspectorPane` becoming dual owners of track
  inspector chrome. Task 005 resolves this by emptying `ui_track.rs` to a
  re-export (and removing it from `KNOWN_SHARED_UI_SHELL_FILES`) or
  documenting exactly what non-track-surface logic remains.
- The `track_detail_labels_owns_canonical_field_labels` matcher
  false-positiving on log/error/match-arm string literals. Task 001 step 14
  pins the matcher to render-call-site literals only and documents the
  chosen form in the test doc comment.

## Test Strategy

Each task should run:

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
git diff --check
```

Task 001 should add focused VM unit tests. Task 005 should run full
`cargo test` and `cargo clippy -- -D warnings`.

Guards land staggered across tasks so each task ships with its own
mechanical signal of completeness, rather than waiting for Task 005:

- Task 001 lands at baseline zero:
  - `screens_do_not_inline_unknown_artist_or_album_fallbacks`
  - `screens_do_not_inline_untitled_fallback`
  - `track_detail_labels_owns_canonical_field_labels` (narrow matcher;
    see Task 001 step 14)
- Task 002 lands at baseline zero:
  - `track_surface_slots_are_typed`
- Task 005 lands the remaining four:
  - `screens_do_not_define_local_track_detail_surface_chrome`
  - `screens_do_not_define_local_track_row_chrome`
  - `screens_do_not_construct_track_inspector_pane_locally`
  - `track_surface_consumers_use_track_detail_vm`

Task 005 also confirms the four earlier guards still pass at zero, deletes
the transitional `TrackRow` constructor introduced in Task 002, resolves
the `src/ui_track.rs` dual-owner question, and updates ADR 0033's
"Enforcing tests" list with all eight names.

Visual smoke is per-task, not deferred to Task 005:

- Task 003 captures Discover screenshots (rows, inspector, full detail if
  present, lazy sections if touched).
- Task 004 captures Library screenshots (rows, inspector, full detail,
  advanced metadata panels if touched).
- Task 005 captures the side-by-side comparison (Library vs Discover at
  full detail and inspector pane for the same track) and confirms
  shared structure across surfaces.

If the user is unavailable for screenshots before the next task starts,
log explicit residual visual risk in the review checklist for the
unfinished gate rather than implicitly piling everything onto Task 005.

## Rollback Strategy

Each task should be independently reversible:

- Task 001 can be reverted without UI behavior changes.
- Task 002 can be reverted before screen adoption if it has not changed
  existing row behavior.
- Task 003 can revert Discover to its current rows/inspector/detail while
  retaining the VM and composite for Library migration.
- Task 004 can revert Library rows/inspector/detail adoption without affecting
  Discover.
- Task 005 can relax only the new guard if it blocks a legitimate follow-up,
  but must keep the review checklist explicit about the residual risk.
