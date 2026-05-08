# ADR 0044 Task 002: Playlist Shell Drag Handle and Drop Targets

## Goal

Render playlist drag handles, insertion-line drop targets, and row
Actions menu fallback commands in the playlist shell.

## Files to Inspect

- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/tasks/adr-0044-task-001-playlist-reorder-vm-contract.md`
- `src/ui/shells/playlist.rs`
- `src/ui/shells/library/playlist_detail.rs`
- `src/ui/primitives/context_menu.rs`
- `src/ui/icons.rs`
- `src/view_models/library.rs`

## Files Likely to Change

- `src/ui/shells/playlist.rs`
- `src/ui/shells/library/playlist_detail.rs`
- `src/ui/icons.rs`
- `src/view_models/library.rs`

## Do Not Touch

- `src/db.rs`
- `src/playlist_service.rs`
- `src/application/commands/playlist.rs`
- Search/Discover UI
- Settings UI

## Constraints

- Drag starts only from the handle.
- Row body selection/open behavior remains unchanged.
- Drop feedback is an insertion line.
- Commit only on drop.
- Pending paged rows are not draggable.
- Same-playlist drag is move behavior.
- Drag/drop payload must identify playlist id and source position.

## Implementation Steps

1. Add a semantic drag-handle icon to the icon catalog if one does not
   already exist.
2. Add a playlist drag payload type owned by the playlist shell or
   playlist row surface.
3. Render the handle using VM-projected id/a11y label and attach
   `on_drag` only to the handle.
4. Render insertion drop zones before each row and after the last row.
5. Accept drops only when the payload playlist id matches the rendered
   playlist id.
6. Convert insertion index to target position:
   `target = if drop_index > from { drop_index - 1 } else { drop_index }`.
7. Do not dispatch if `target == from`.
8. Add row Actions menu items for Move Up and Move Down using the VM
   menu display contract.
9. Wire eager and paged ready rows to existing `move_playlist_track`.

## Acceptance Criteria

- Visible up/down buttons are gone from playlist rows.
- Drag handle is visible and tokenized.
- Row drag does not start from title/artwork/body area.
- Insertion line appears only for same-playlist drag payloads.
- Move Up/Move Down fallback actions are available in the row Actions
  menu and disabled at boundaries.
- Paged pending rows do not start drag operations.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test playlist
cargo test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `src/ui/shells/playlist.rs`
- `src/ui/shells/library/playlist_detail.rs`
- `src/ui/primitives/context_menu.rs`
- `src/ui/icons.rs`
- `src/view_models/library.rs`

Goal:
- Replace visible playlist reorder arrows with a drag handle,
  insertion-line drop targets, and Move Up/Move Down row menu fallback
  actions.

Constraints:
- Drag from handle only.
- Commit only on drop.
- Same-playlist move only.
- Reuse existing `move_playlist_track` wiring.

Do not touch:
- `src/db.rs`
- `src/playlist_service.rs`
- `src/application/commands/playlist.rs`
- Search/Discover UI

Acceptance criteria:
- Up/down buttons are no longer visible.
- Same-playlist handle drag can reorder rows.
- Menu fallback commands exist and honor boundary disabled states.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test playlist`
- `cargo test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- GPUI drag/drop APIs cannot support insertion-line drop targets without
  broad custom pointer tracking.
- Paged playlist rendering cannot support drop zones without changing
  actor behavior.
