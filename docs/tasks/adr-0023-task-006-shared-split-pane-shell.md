# ADR 0023 Task 006: Shared Split-Pane Shell

## Status

Planned.

## Task Goal

Give Discover and Library the same resizable two-pane shell while keeping
resize state GPUI-free and event wiring in the screens.

## Files To Inspect

- `src/search.rs`
- `src/library.rs`
- `src/view_models/search.rs`
- `src/view_models/library.rs`
- `src/ui/composites/list_row.rs`
- `src/ui/theme.rs`
- `docs/architecture/architecture-diagrams.md`

## Files Likely To Change

- `src/search.rs`
- `src/library.rs`
- `src/view_models/search.rs`
- `src/view_models/library.rs`
- `src/ui/composites/mod.rs`
- New: `src/ui/composites/split_pane.rs` or `src/ui/layouts/split_pane.rs`
- `src/ui/theme.rs`
- Focused VM tests in `src/view_models/search.rs` and/or
  `src/view_models/library.rs`

## Do Not Touch

- Database schema and migrations.
- Service modules.
- MusicBrainz, metadata, playback, or playlist behavior.
- Broad file/directory splitting of `library.rs` or `search.rs`.

## Constraints

- Do not build a general layout framework. Implement only the split-pane
  contract needed by Discover and Library.
- Keep GPUI event handlers in screen/component code; pure pane width, dragging
  state, and clamp logic should live in VM-safe Rust types or VM methods.
- Discover behavior must remain functionally equivalent.
- Library must gain resize behavior instead of keeping a fixed pane width.
- Use named layout constants from `theme::layout`; do not add numeric screen
  `px(...)` literals.

## Implementation Steps

1. Extract the Discover split-pane layout pattern into a reusable component or
   layout helper with slots for left content, handle, and right content.
2. Move shared clamp and drag lifecycle semantics into a pure projection/state
   type, or mirror the existing `SearchViewModel` methods in
   `LibraryViewModel`.
3. Wire Discover through the shared shell without changing visual behavior.
4. Wire Library through the same shell and expose the resize handle.
5. Add focused tests for any new pure resize state.
6. Update docs to mark this task complete only after verification is green.

## Acceptance Criteria

- Discover and Library both render through the same split-pane shell.
- Both screens have the same min/max width behavior and resize affordance.
- No `gpui` or `gpui_component` imports are added under `src/view_models`.
- No screen-level numeric `px(...)` literals are introduced.
- Existing inspector/list focus and scroll behavior still works.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::search`
- `cargo test --lib view_models::library`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The shared shell requires changing app navigation, inspector stack shape, or
  service dispatch.
- Library and Discover need incompatible resize semantics.
- GPUI types appear necessary in view-model state.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0023-design-system-and-view-models.md`
- `docs/plans/adr-0023-finalization-plan.md`
- `src/search.rs`
- `src/library.rs`
- `src/view_models/search.rs`
- `src/view_models/library.rs`
- `src/ui/theme.rs`

Goal:
- Give Discover and Library the same resizable split-pane shell while keeping
  resize state GPUI-free.

Constraints:
- Preserve Discover behavior.
- Add resize behavior to Library.
- Keep screen event wiring in GPUI code.
- Do not introduce screen-level numeric `px(...)` literals.
- Do not introduce a broad command bus or split screen files.

Do not touch:
- Database schema.
- Service modules.
- Metadata/MusicBrainz/playback behavior.

Acceptance criteria:
- Both screens use the same split-pane component/helper.
- Both screens clamp width consistently.
- No `gpui` imports under `src/view_models`.
- Focused VM tests cover any pure resize state.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::search`
- `cargo test --lib view_models::library`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
