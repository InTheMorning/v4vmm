# ADR 0026 Task 003: Slot-Based UI Shells

## Status

Implemented.

## Goal

Add a shared GPUI shell for release/feed detail surfaces that consumes ADR
0026 projection VMs and accepts screen-owned action/panel slots, without
migrating Discover or Library rendering yet.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-002-shared-projection-vms.md`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/detail_grid.rs`
- `src/ui/composites/track_row.rs`
- `tests/architecture_tests.rs`

## Files Changed

- `src/ui_entity.rs`
- `src/lib.rs`
- `tests/architecture_tests.rs`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-003-slot-based-ui-shells.md`
- `docs/reviews/adr-0026-task-003-review.md`

## Do Not Touch

- Do not migrate Discover or Library rendering.
- Do not import `SearchApp`, `LibraryApp`, or screen modules.
- Do not import service, database, or API row modules.
- Do not change playlist, download, MusicBrainz, playback, or database
  behavior.

## Constraints

- The shell may import GPUI and design-system components.
- Screen adapters must own click handlers, popover state, image-cache
  resolution, and command dispatch.
- Action controls must be supplied through explicit slots.
- The shell must render existing ADR 0023/0025 composites rather than creating
  new visual vocabulary.

## Implementation Summary

- Added `src/ui_entity.rs` with `ReleaseDetailSlots`,
  `TrackRowActionSlot`, and `render_release_detail_shell`.
- The shell composes `ReleaseDetailSurface`, `DetailHeader`, `DetailGrid`, and
  `TrackRow` from shared `ReleaseDetailVm` data.
- Action rows, identity controls, panels, per-track actions, and after-section
  content are all slot inputs supplied by future screen adapters.
- Added an architecture test preventing `src/ui_entity.rs` from importing
  screens, services, database modules, or API rows.

## Acceptance Criteria

- [x] `src/ui_entity.rs` exists and is exported from `src/lib.rs`.
- [x] The shell consumes `ReleaseDetailVm` and slot structs.
- [x] The shell imports no screen, service, database, or API row modules.
- [x] No Discover or Library rendering behavior changes.
- [x] Architecture tests enforce the shell boundary.
- [x] Verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test ui_entity
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-003-slot-based-ui-shells.md`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/release_detail_surface.rs`
- `tests/architecture_tests.rs`

Goal:
- Add a slot-based shared GPUI shell over the ADR 0026 projections.

Constraints:
- Do not import screen or service modules.
- Do not migrate Discover or Library rendering.
- Do not introduce a new visual system.

Do not touch:
- `src/library.rs`
- `src/search.rs`
- playlist/download/MusicBrainz/playback behavior

Acceptance criteria:
- `src/ui_entity.rs` compiles and is architecture-gated.
- Slot structs keep actions and panels screen-owned.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test ui_entity`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The shell cannot compile without importing a screen module.
- The shell needs service or database access.
- Discover or Library must be migrated for the shell to compile.
