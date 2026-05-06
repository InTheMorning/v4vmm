# ADR 0032 Task 001: Playlist Popover Contract Repair

## Status

Completed - 2026-05-02.

## Goal

Repair the Library add-to-playlist popover regression by routing Library album
and track playlist affordances through the canonical `AddToPlaylistPopover`
composite instead of screen-local full-width panels.

## Files To Inspect

- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `src/library.rs`
- `src/ui/composites/playlist_popover.rs`
- `src/ui/primitives/popover.rs`
- `src/view_models/library.rs`
- `src/ui_track.rs`

## Files Changed

- `src/library.rs`
- `src/ui/composites/playlist_popover.rs`
- `src/view_models/library.rs`
- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `docs/architecture/ui-backend-boundary.md`
- `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- `docs/reviews/adr-0032-review-checklist.md`
- `docs/tasks/adr-0032-task-001-playlist-popover-contract.md`

## Do Not Touch

- `src/db.rs`
- `migrations/`
- playlist service semantics
- download, playback, subscription, and MusicBrainz services

## Constraints

- Shared UI composites own popover chrome.
- Library screens own playlist selection callbacks and command dispatch.
- Do not move service calls into `AddToPlaylistPopover`.
- Do not change playlist append behavior.
- Do not reintroduce full-width row-child panels as popovers.

## Implementation Summary

- Extended `AddToPlaylistPopover` with a configurable trigger label so
  release-level actions can say `Add feed to playlist`.
- Made the composite hide its `New Playlist` mode when no create handler is
  supplied.
- Replaced Library album and track screen-local playlist panels with
  `AddToPlaylistPopover`.
- Removed stale Library view-model popover-open state that only existed to
  drive screen-local panel chrome.
- Visually smoked the repaired Library release and row popovers with an
  isolated app instance.

## Acceptance Criteria

- [x] Library album add-to-playlist uses `AddToPlaylistPopover`.
- [x] Library track row add-to-playlist uses `AddToPlaylistPopover`.
- [x] Discover continues to use the same composite.
- [x] Screen modules still own playlist append command dispatch.
- [x] Raw full-width Library playlist panels are removed.
- [x] View-model popover-open state for Library album rows is removed.
- [x] No playlist service or database semantics changed.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo build
cargo test view_models::library
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
git diff --check
```

Visual smoke screenshots:

- `/tmp/v4vmm-adr32-library-release.png`
- `/tmp/v4vmm-adr32-library-row-popover.png`
- `/tmp/v4vmm-adr32-library-feed-popover.png`

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `src/library.rs`
- `src/ui/composites/playlist_popover.rs`
- `src/ui/primitives/popover.rs`
- `src/view_models/library.rs`
- `src/ui_track.rs`

Goal:
- Route Library add-to-playlist popovers through the shared popover composite.

Constraints:
- Keep command dispatch in screen modules.
- Do not touch playlist service/database semantics.
- Remove screen-local full-width popover panels only after replacing them.

Do not touch:
- `src/db.rs`
- `migrations/`
- service modules

Acceptance criteria:
- Library and Discover use the same add-to-playlist popover chrome.
- Library playlist append callbacks still call existing screen-owned commands.
- No raw Library add-to-playlist panel remains.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::library`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
