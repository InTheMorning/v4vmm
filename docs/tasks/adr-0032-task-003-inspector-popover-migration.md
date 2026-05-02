# ADR 0032 Task 003: Inspector Popover Migration

## Status

Completed - 2026-05-02.

## Goal

Migrate the remaining Library and Discover inspector add-to-playlist panels to
the canonical `AddToPlaylistPopover` composite and reduce the architecture-test
screen-local playlist panel baseline to zero. Keep `+ New Playlist` available
on every playlist popover.

## Files To Inspect

- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/ui_feed.rs`
- `src/ui_track.rs`
- `src/ui/composites/playlist_popover.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `tests/architecture_tests.rs`

## Files Changed

- `src/library.rs`
- `src/search.rs`
- `src/ui_feed.rs`
- `src/ui_track.rs`
- `src/ui/composites/playlist_popover.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `tests/architecture_tests.rs`
- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `docs/plans/adr-0032-ui-backend-boundary-phase-plan.md`
- `docs/reviews/adr-0032-review-checklist.md`
- `docs/tasks/adr-0032-task-003-inspector-popover-migration.md`
- `docs/reviews/adr-0032-task-003-review.md`

## Do Not Touch

- `src/db.rs`
- `migrations/`
- playlist service behavior
- subscription/download/playback/MusicBrainz semantics
- unrelated visual refactors

## Constraints

- Shared UI composites own trigger and floating popover chrome.
- Screens still own playlist target resolution and command dispatch callbacks.
- Preserve existing disabled behavior for busy inspector playlist actions.
- Do not introduce new service calls into `AddToPlaylistPopover`.
- Every `AddToPlaylistPopover` call site must wire create mode with
  `.on_create(...)`; screens own the create-then-append callback.
- Tighten architecture tests only after removing the legacy screen-local panel
  patterns.

## Implementation Summary

- Added a `disabled` builder to `AddToPlaylistPopover` so inspector actions can
  preserve their existing busy/unavailable behavior.
- Replaced the Library track inspector raw playlist panel with
  `AddToPlaylistPopover`.
- Replaced the Discover feed/track inspector raw playlist panel with
  `AddToPlaylistPopover`.
- Removed the stale Discover row popup compatibility wrapper and unused
  row-open plumbing.
- Removed view-model and screen state that only described visual popover-open
  chrome.
- Tightened ADR0032 architecture-test baselines for screen-local playlist
  panels to zero.
- Wired `+ New Playlist` create mode for Library release, Library inspector,
  Discover inspector, and Discover row playlist popovers.
- Added an architecture test that rejects `AddToPlaylistPopover` call sites
  without `.on_create(...)`.

## Acceptance Criteria

- [x] Library track inspector playlist action uses `AddToPlaylistPopover`.
- [x] Discover feed/track inspector playlist action uses
      `AddToPlaylistPopover`.
- [x] Stale Discover row popup wrapper is removed.
- [x] View models no longer carry playlist popover open/closed chrome state.
- [x] Architecture tests fail on any reintroduced screen-local playlist panel
      helper/toggle pattern.
- [x] Architecture tests fail on playlist popover call sites that omit
      `.on_create(...)`.
- [x] Every playlist popover includes the `+ New Playlist` affordance.
- [x] Playlist append command dispatch remains in screen modules.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test view_models::library
cargo test view_models::search
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `src/library.rs`
- `src/search.rs`
- `src/ui_feed.rs`
- `src/ui_track.rs`
- `src/ui/composites/playlist_popover.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `tests/architecture_tests.rs`

Goal:
- Migrate inspector add-to-playlist panels to the canonical shared popover.

Constraints:
- Keep command dispatch in screen modules.
- Do not touch playlist service/database semantics.
- Preserve existing disabled behavior for busy inspector actions.
- Preserve `+ New Playlist` create mode on every shared playlist popover.
- Remove stale screen-local popover state only after replacement.

Do not touch:
- `src/db.rs`
- `migrations/`
- unrelated services
- unrelated UI components

Acceptance criteria:
- Library and Discover inspector playlist actions use `AddToPlaylistPopover`.
- No `render_add_to_playlist_panel*` helper remains.
- No `render_row_playlist_popup` helper remains.
- Every `AddToPlaylistPopover` call site wires `.on_create(...)`.
- Architecture-test screen-local playlist popover baselines are zero.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::library`
- `cargo test view_models::search`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
