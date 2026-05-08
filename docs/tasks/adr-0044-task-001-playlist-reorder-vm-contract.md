# ADR 0044 Task 001: Playlist Reorder View-Model Contract

## Goal

Replace playlist row arrow-specific display fields with drag-handle and
menu fallback display contracts. Do not change rendered UI behavior in
this task unless required to keep compilation green.

## Files to Inspect

- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `src/view_models/library.rs`
- `src/ui/shells/playlist.rs`
- `src/ui/shells/library/playlist_detail.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/db.rs`
- `src/playlist_service.rs`
- `src/application/commands/playlist.rs`
- Playback behavior
- Search/Discover UI

## Constraints

- Reorder display policy stays in `PlaylistTrackRowVm`.
- Do not add raw glyph strings in renderers.
- Keep play/remove display fields unless needed by the new menu
  contract.
- Boundary availability is owned by the VM.

## Implementation Steps

1. Replace `move_up_button_id`, `move_up_label`,
   `move_up_enabled`, `move_down_button_id`, `move_down_label`, and
   `move_down_enabled` with display fields for:
   - `drag_handle_id`
   - `drag_handle_a11y_label`
   - Move Up menu item id/label/a11y/disabled
   - Move Down menu item id/label/a11y/disabled
2. Keep `can_move_up()` and `can_move_down()` or replace them with
   equivalent VM-owned boundary helpers.
3. Add a small display struct if the menu item shape should not depend
   directly on `ContextMenuItemDisplay`.
4. Update VM tests to assert handle/menu projections and boundary
   disabled states.
5. Update architecture-test expectations that currently name arrow ids
   and glyphs so they describe the new VM-owned handle/menu contract.

## Acceptance Criteria

- Playlist row display no longer exposes arrow labels or arrow button
  ids.
- VM tests cover first, middle, and last row reorder availability.
- The new display contract includes accessibility labels for handle and
  menu items.
- No persistence or command behavior changes.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test playlist_track_row_vm
cargo test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

Goal:
- Replace playlist row arrow-specific display fields with drag-handle
  and Move Up/Move Down menu fallback display contracts.

Constraints:
- Reorder display policy stays in `PlaylistTrackRowVm`.
- Do not change DB/application reorder behavior.
- Preserve play/remove display contracts unless directly required.

Do not touch:
- `src/db.rs`
- `src/playlist_service.rs`
- `src/application/commands/playlist.rs`
- Search/Discover UI

Acceptance criteria:
- No arrow labels or arrow button ids remain in playlist row display.
- Handle/menu display fields include accessibility labels.
- VM tests cover boundary disabled states.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test playlist_track_row_vm`
- `cargo test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The shell cannot compile without adopting the new display contract in
  the same task.
- Existing architecture tests require a broad baseline instead of a
  direct update to the new invariant.
