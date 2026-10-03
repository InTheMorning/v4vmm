# Library Playlist Inline Rename Task 001

## Status

Completed on 2026-05-08.

Implementation summary:

- Added playlist rename edit state to `LibraryViewModel`.
- Added rename editor ids, labels, placeholders, and action labels to
  `PlaylistDetailVm::actions_display`.
- Wired eager and async-runtime paged playlist detail paths through the
  same inline rename affordance.
- Kept `RenamePlaylist` as the only persistence path.
- Preserved playlist delete, reorder, and track removal behavior.

Visual proof:

- Operator confirmed the rename affordance is clear on 2026-05-08 after
  reviewing the running UI.

Verification:

- `cargo fmt -- --check` green.
- `cargo check` green.
- `cargo test` green.
- `cargo clippy -- -D warnings` green.

## Goal

Make the playlist detail Rename action functional without weakening the
existing HIG-oriented shell/view-model split.

## Files to Inspect

- `src/ui/shells/library/playlist_detail.rs`
- `src/ui/shells/playlist.rs`
- `src/library/app_impl.rs`
- `src/library.rs`
- `src/view_models/library.rs`
- `src/view_models/playlist_detail.rs`
- `src/application/commands/playlist.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/ui/shells/library/playlist_detail.rs`
- `src/ui/shells/playlist.rs`
- `src/library/app_impl.rs`
- `src/library.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Do not change playlist persistence schema.
- Do not alter playlist delete, reorder, or track removal behavior.
- Do not add screen-local fallback strings or raw button labels.
- Do not introduce a one-off modal/input component if an existing
  primitive, composite, or shell-owned pattern can carry the interaction.

## Constraints

- The existing `RenamePlaylist` application command remains the
  persistence path.
- Rename display strings, ids, placeholders, and accessibility labels
  must come from view-model display contracts.
- The playlist detail shell may own layout and interaction state, but
  repeated chrome belongs in primitives or composites.
- Empty or whitespace-only names must not dispatch a command.
- The current playlist name should be the initial edit value.
- The interaction must be visually verified in light and dark themes.

## Implementation Steps

1. Decide whether rename is inline in the playlist header or presented
   by a shared dialog/input pattern already consistent with the app.
2. Add any missing rename-edit display contract fields to the relevant
   playlist view-model.
3. Wire both eager and paged playlist detail render paths to the same
   rename interaction.
4. Dispatch `LibraryApp::rename_playlist` only after a non-empty edited
   name is submitted.
5. Add or update architecture guards so rename labels/placeholders/ids
   cannot move back into screen-local literals.
6. Add VM/unit coverage for display contract and empty-name behavior.
7. Capture light/dark visual proof.

## Acceptance Criteria

- Pressing Rename on playlist detail starts a visible rename affordance.
- Submitting a changed non-empty name renames the playlist through
  `RenamePlaylist`.
- Cancel leaves the playlist unchanged.
- Empty or whitespace-only names do not dispatch.
- Eager and async-runtime paged playlist detail paths behave the same.
- Display text and ids are view-model-owned.
- `cargo fmt -- --check`, `cargo check`, `cargo test`, and
  `cargo clippy -- -D warnings` are green.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `src/ui/shells/library/playlist_detail.rs`
- `src/ui/shells/playlist.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/view_models/playlist_detail.rs`
- `src/application/commands/playlist.rs`
- `tests/architecture_tests.rs`

Goal:
- Make playlist detail Rename functional through the existing
  `RenamePlaylist` command while preserving shell/view-model ownership.

Constraints:
- Display strings, ids, placeholders, and a11y labels must come from
  view-model display contracts.
- No schema changes.
- No unrelated playlist behavior changes.
- Do not add one-off repeated chrome.

Do not touch:
- Playlist delete, reorder, or track removal behavior
- Search/Discover UI
- Database schema

Acceptance criteria:
- Rename can be submitted and cancelled.
- Empty names do not dispatch.
- Eager and paged playlist paths are covered.
- Required checks are green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The existing GPUI input/dialog primitives cannot support an accessible
  rename affordance without new shared UI architecture.
- Rename state cannot be shared between eager and paged playlist detail
  without broad screen changes.
