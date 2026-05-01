# ADR 0026 Task 005: Library Projection Adoption

## Status

Implemented.

## Goal

Route Library album detail through the ADR 0026 shared release projection and
slot-based shell while preserving Library-specific actions, MusicBrainz state,
playlist popovers, and track rows.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-004-discover-projection-adoption.md`
- `src/ui_entity.rs`
- `src/library.rs`

## Files Changed

- `src/library.rs`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-005-library-projection-adoption.md`
- `docs/reviews/adr-0026-task-005-review.md`

## Do Not Touch

- Do not change Library track removal, playlist, MusicBrainz, playback, or
  database behavior.
- Do not change Discover rendering.
- Do not import screen or service modules into `src/ui_entity.rs`.

## Constraints

- Preserve the existing Library album header, action row, detail grid,
  add-to-playlist panel, and track rows.
- Keep Library-specific controls as slots supplied by `library.rs`.
- Use `ReleaseDetailVm` with `EntitySurfaceContext::Library`.

## Implementation Summary

- Updated `render_album_detail` to build a `ReleaseDetailVm` and render through
  `render_release_detail_shell`.
- Passed the existing Library header, actions, detail grid, add-to-playlist
  panel, and rendered track rows through shell slots.
- Left existing Library action handlers and row behavior unchanged.

## Acceptance Criteria

- [x] Library album detail renders through `render_release_detail_shell`.
- [x] Existing Library album controls and track rows remain behaviorally
  unchanged.
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
- `docs/tasks/adr-0026-task-005-library-projection-adoption.md`
- `src/ui_entity.rs`
- `src/library.rs`

Goal:
- Route Library album detail through the shared release shell.

Constraints:
- Preserve current Library behavior.
- Do not import screen/service modules into `ui_entity`.
- Do not migrate unrelated Library views.

Do not touch:
- `src/search.rs`
- playlist/download/MusicBrainz/playback behavior

Acceptance criteria:
- Library album detail uses the shell.
- Behavior-sensitive controls remain Library-owned slots.
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

- Preserving behavior requires moving Library handlers into `src/ui_entity.rs`.
- Existing Library row behavior cannot be supplied through slots.
