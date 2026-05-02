# ADR 0033 Task 001: Boundary Gates

## Task Goal

Implement the first enforceable HIG UI architecture gates: shared UI components
must be backend-free, and presentation modules must not build local floating
chrome.

## Files to Inspect

- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `src/ui/composites/playlist_popover.rs`
- `src/ui/composites/mod.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/adr-0033-hig-ui-architecture-governance-phase-plan.md`
- `docs/tasks/adr-0033-task-001-boundary-gates.md`
- `docs/reviews/adr-0033-task-001-review.md`
- `src/ui/composites/playlist_popover.rs`
- `src/ui/composites/mod.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Database migrations.
- Service command semantics.
- Playlist append/create behavior.
- Playback behavior.
- Visual theme token values.

## Constraints

- Do not redesign the playlist workflow.
- Preserve every existing playlist select/create callback.
- Shared UI primitives/composites must not import backend, service, API, DB, or
  screen modules.
- Presentation modules may compose shared UI and wire callbacks, but may not
  create raw floating chrome.
- Architecture tests must fail loudly with ADR 0033-specific messages.

## Implementation Steps

1. Add ADR 0033 planning artifacts.
2. Add a display-ready `PlaylistOption` to `AddToPlaylistPopover`.
3. Convert Library, Discover inspector, and Discover row call sites to map
   `db::Playlist` into `PlaylistOption` at the screen boundary.
4. Add an architecture test that scans shared UI primitives/composites for
   backend/screen imports.
5. Add an architecture test that scans presentation modules for raw floating
   chrome patterns.
6. Verify formatting, compilation, focused tests, architecture tests, clippy,
   and whitespace.

## Acceptance Criteria

- `src/ui/composites/playlist_popover.rs` has no `crate::db` import.
- All playlist popovers still expose existing playlist selection and
  `+ New Playlist` creation.
- `tests/architecture_tests.rs` rejects backend imports in shared UI
  primitives/composites.
- `tests/architecture_tests.rs` rejects screen-local floating chrome patterns.
- Verification commands are green.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test view_models::library
cargo test view_models::search
cargo clippy --lib --tests -- -D warnings
git diff --check
```

## Expected Final Summary Format

1. Files changed.
2. Architecture gates added.
3. Tests run.
4. Commit hash.
5. Remaining risks.

## Escalation Triggers

- A legitimate shared UI component needs backend state.
- A screen requires custom floating geometry that cannot be represented by
  existing primitives/composites.
- Architecture tests block existing approved compatibility paths.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0032-ui-backend-boundary-and-popover-contracts.md`
- `src/ui/composites/playlist_popover.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`

Goal:
- Make shared playlist popover input display-ready and add ADR 0033
  architecture gates.

Constraints:
- Preserve playlist select/create behavior.
- Do not change database, service, playback, or subscription semantics.
- Shared UI primitives/composites must not import backend or screen modules.
- Presentation modules must not hand-roll floating chrome.

Do not touch:
- migrations
- service command implementations
- playback driver code
- theme token values

Acceptance criteria:
- Shared UI no longer imports `crate::db`.
- New architecture tests fail on backend imports in shared UI.
- New architecture tests fail on screen-local floating chrome.
- Existing playlist popover call sites compile and still wire `.on_create(...)`.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test view_models::library`
- `cargo test view_models::search`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
