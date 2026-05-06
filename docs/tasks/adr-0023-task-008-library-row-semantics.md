# ADR 0023 Task 008: Library Row Semantics

## Status

Completed 2026-04-30.

## Task Goal

Remove redundant Library album-row downloaded labels and move remaining
album-row action labels/status semantics into GPUI-free projections.

## Files To Inspect

- `src/library.rs`
- `src/view_models/library.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/action_button.rs`
- `docs/plans/adr-0023-finalization-plan.md`

## Files Likely To Change

- `src/library.rs`
- `src/view_models/library.rs`
- Focused tests in `src/view_models/library.rs`
- ADR 0023 task/review docs

## Do Not Touch

- Search/Discover track download behavior unless a shared projection requires
  an import-safe type.
- Database schema.
- MusicBrainz lookup implementation.
- Playlist append service behavior.

## Constraints

- Remove the per-row `dl'd` label from Library album rows.
- Do not remove aggregate downloaded counts from album/artist detail grids
  unless the user explicitly confirms that product decision.
- Keep the `Remove` button for tracks in the library and `Download` for
  tracks not in the library.
- Projection code remains GPUI-free and unit-tested.

## Implementation Steps

1. Extend `LibraryTrackRowVm` or a new narrow projection to expose action label,
   action role, playlist label, and optional MusicBrainz status display.
2. Update `render_library_track_row` to consume the projection instead of
   formatting labels inline.
3. Remove the `dl'd` trailing label.
4. Keep the action/button behavior unchanged.
5. Add focused tests for in-library vs not-in-library labels and absent
   downloaded row label.

## Acceptance Criteria

- [x] Library album rows no longer show `dl'd`.
- [x] Track membership remains communicated through the `Remove` or `Download`
  action.
- [x] `LibraryTrackRowVm` owns relevant row display strings.
- [x] Existing MusicBrainz status display still works.
- [x] Focused VM tests cover the projection.

## Result

- Removed the per-row `dl'd` trailing label from Library album rows.
- Added `LibraryTrackPrimaryAction` plus `LibraryTrackRowVm` accessors for
  primary action labels and playlist action labels.
- Kept aggregate downloaded counts in album/artist detail grids.
- Added focused VM tests for membership labels, busy labels, playlist open
  labels, and the local-path case that previously produced redundant row text.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::library`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Removing `dl'd` reveals a distinct "cached but not in library" state that
  needs product naming.
- Row semantics require changing download/remove service behavior.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0023-design-system-and-view-models.md`
- `docs/plans/adr-0023-finalization-plan.md`
- `src/library.rs`
- `src/view_models/library.rs`
- `src/ui/composites/track_row.rs`

Goal:
- Remove redundant Library album-row downloaded labels and move remaining
  row-display semantics into GPUI-free projections.

Constraints:
- Remove per-row `dl'd`.
- Keep `Remove` / `Download` behavior unchanged.
- Keep aggregate downloaded counts unless explicitly directed otherwise.
- Add focused VM tests.

Do not touch:
- Schema/migrations.
- Service behavior.
- Broad screen architecture.

Acceptance criteria:
- No `dl'd` label remains in Library album rows.
- Projection owns row labels.
- Focused tests pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::library`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
