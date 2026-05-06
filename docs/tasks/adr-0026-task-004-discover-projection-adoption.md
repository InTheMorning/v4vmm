# ADR 0026 Task 004: Discover Projection Adoption

## Status

Implemented.

## Goal

Route Discover feed detail through the ADR 0026 shared release projection and
slot-based shell without changing current Discover behavior.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-003-slot-based-ui-shells.md`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/search.rs`

## Files Changed

- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-004-discover-projection-adoption.md`
- `docs/reviews/adr-0026-task-004-review.md`

## Do Not Touch

- Do not migrate Library rendering.
- Do not change Discover action handlers, playlist behavior, download behavior,
  MusicBrainz behavior, or playback behavior.
- Do not import screen or service modules into `src/ui_entity.rs`.

## Constraints

- Preserve the existing Discover feed header behavior, including RSS and Nostr
  controls.
- Preserve the existing clickable publisher row.
- Preserve existing Discover track rows, thumbnails, click handlers, download,
  playlist, and play controls.
- Use explicit slots for behavior-sensitive elements the shared shell must not
  own.

## Implementation Summary

- Extended `ReleaseDetailSlots` with optional header, details, and track-section
  override slots.
- Updated `ui_feed::render_feed_view` to build a `ReleaseDetailVm` and render
  through `render_release_detail_shell`.
- Kept existing Discover header, action row, rich detail grid, description
  panel, existing track rows, and lazy/podroll panels as slots.

## Acceptance Criteria

- [x] Discover feed detail renders through `render_release_detail_shell`.
- [x] Existing Discover feed header, action row, detail grid, description, track
  rows, and after-section panels remain behaviorally unchanged.
- [x] `src/ui_entity.rs` remains screen/service-free.
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
- `docs/tasks/adr-0026-task-004-discover-projection-adoption.md`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/search.rs`

Goal:
- Route Discover feed detail through the shared release shell.

Constraints:
- Preserve current Discover behavior.
- Do not migrate Library.
- Do not import screens/services into `ui_entity`.

Do not touch:
- `src/library.rs`
- playlist/download/MusicBrainz/playback behavior

Acceptance criteria:
- Discover feed detail uses the shell.
- Behavior-sensitive controls remain screen-owned slots.
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

- Preserving behavior requires importing `SearchApp` into `src/ui_entity.rs`.
- Existing Discover track rows cannot be supplied as slots.
