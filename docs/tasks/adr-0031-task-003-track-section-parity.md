# ADR 0031 Task 003: Track Row Visual Template

## Status

Completed - 2026-05-01.

## Goal

Normalize the visual row template of the track section across Library and
Discovery. Scope is limited to row geometry; the section structure itself must
already come from Task 002.

## Files To Inspect

- `docs/adr/0031-release-detail-presentation-contract.md`
- `docs/tasks/adr-0031-task-002-renderer-adoption.md`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/library.rs`
- `src/ui_feed.rs`
- `src/search.rs`

## Files Likely To Change

- `src/ui_entity.rs`
- `src/library.rs`
- `src/ui_feed.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs` only for additive track-row projection
  fields

## Do Not Touch

- `src/db.rs`
- `migrations/`
- playlist, playback, download, subscription, metadata, or MusicBrainz service
  code

## Constraints

- One row skeleton for both surfaces.
- Row column order is fixed: number, artwork/thumb, title and secondary
  metadata, duration, surface action slot.
- Number column width and row height are constants in one place, applied to
  both surfaces.
- The surface action slot lives at one named position on the row; surfaces
  populate it but cannot reorder it.
- Empty and loading states are owned by `ReleaseTrackSectionVm` or the renamed
  equivalent and rendered by one shared component on both surfaces.
- No visual redesign outside release-like detail track sections.

## Implementation Steps

1. Compare current Library and Discovery track row adapters.
2. Route both through the shared row skeleton.
3. Preserve surface-specific action affordances through existing slots.
4. Keep number, artwork/thumb behavior, title, secondary metadata, duration,
   and trailing actions aligned consistently from one shared template.
5. Add or update focused tests where practical.

## Acceptance Criteria

- [x] Track rows align consistently between surfaces.
- [x] Row column order is number, artwork/thumb, title and secondary metadata,
  duration, surface action slot.
- [x] Number column width and row height are constants in one shared place.
- [x] Surfaces populate one named action slot and cannot reorder it.
- [x] Surface-specific actions remain available through action slots.
- [x] Empty/loading states use the same section placement.

## Implementation Summary

Task 003 introduced `render_release_track_row` and `ReleaseTrackRowSlot` in
`src/ui_entity.rs` as the shared release-track row template. Library and
Discovery now populate the same named behavior slot with thumbnails, click
handlers, trailing actions, and optional row popovers instead of assembling
`TrackRow` directly in screen modules.

The shared `TrackRow` composite now owns fixed row height via
`ui::layouts::ROW_HEIGHT`; it already owned the fixed number column through
`ui::layouts::TRACK_NUMBER_WIDTH`. Library display text fallback moved to
`SharedTrackRowVm` / `TrackView` projection, leaving Library row view-model code
focused on action state.

Verification completed:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test ui_entity`
- `cargo test view_models::entity_detail`
- `cargo test view_models::library`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test view_models::entity_detail
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0031-release-detail-presentation-contract.md`
- `docs/tasks/adr-0031-task-003-track-section-parity.md`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`

Goal:
- Normalize the release track-section visual row template across Library and
  Discovery.

Constraints:
- Keep screen-specific actions in slots.
- Do not change playback, download, playlist, or metadata semantics.
- Do not redesign unrelated lists.

Do not touch:
- `src/db.rs`
- `migrations/`
- service modules

Acceptance criteria:
- One shared row skeleton is used by both surfaces.
- Row geometry constants live in one place.
- Surface-specific actions still render.
- Empty/loading states occupy the same section placement.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::entity_detail`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
