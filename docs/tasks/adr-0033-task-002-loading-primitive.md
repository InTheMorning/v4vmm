# ADR 0033 Task 002: Shared Loading Primitive

## Goal

Consolidate the duplicated `render_loading` helpers in `src/library.rs` and
`src/search.rs` into one shared design-system primitive.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/ui/primitives/mod.rs`
- `src/ui/primitives/label.rs`

## Files Likely to Change

- `src/library.rs`
- `src/search.rs`
- `src/ui/primitives/mod.rs`
- `src/ui/primitives/loading.rs`
- `docs/tasks/adr-0033-task-002-loading-primitive.md`
- `docs/reviews/adr-0033-task-002-review.md`

## Do Not Touch

- Backend, database, API, service, and command modules.
- ADR 0031 release-detail behavior.
- Playlist popover behavior from ADR 0032.
- MusicBrainz, action-row, metadata-grid, file-header, and track-header
  helpers; those are later packets.

## Constraints

- Keep the visual behavior equivalent: muted italic text with vertical token
  padding.
- The primitive must be backend-free and screen-free.
- Use token resolution from `src/ui/tokens.rs`; do not introduce raw colors or
  numeric `px(...)` literals.
- Do not add state, async work, or callbacks.

## Implementation Steps

1. Add `src/ui/primitives/loading.rs` with a `LoadingMessage` `RenderOnce`
   primitive.
2. Export `LoadingMessage` from `src/ui/primitives/mod.rs`.
3. Replace both screen-local `render_loading` call sites with
   `LoadingMessage`.
4. Delete both duplicated helper functions.
5. Run the architecture and Rust verification commands.

## Acceptance Criteria

- No `fn render_loading` remains in `src/library.rs` or `src/search.rs`.
- Both screens use `crate::ui::primitives::LoadingMessage`.
- Architecture tests remain green.
- `cargo check` and `cargo fmt -- --check` are green.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/ui/primitives/mod.rs`

Goal:
- Replace duplicated screen-local loading text helpers with one shared
  token-driven primitive.

Constraints:
- Preserve current visual behavior.
- Keep the primitive backend-free and screen-free.
- Use named tokens instead of raw colors or numeric `px(...)`.

Do not touch:
- Backend/service/database/API modules.
- MusicBrainz, action-row, metadata-grid, file-header, and track-header
  consolidation tasks.

Acceptance criteria:
- `render_loading` is removed from both screens.
- Both screens call `LoadingMessage`.
- Verification commands are green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
