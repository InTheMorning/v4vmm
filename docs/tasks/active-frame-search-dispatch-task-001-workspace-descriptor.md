# Active-Frame Search Dispatch Task 001: Workspace Descriptor

Status: Implemented - 2026-05-16.

## Goal

Add the GPUI-free workspace search descriptor that lets toolbar code ask the
focused frame what a submitted query means, without mutating page view-models or
rendering new toolbar UI.

## Files to Inspect

- `docs/plans/active-frame-search-dispatch-plan.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/adr/0047-library-search-unification.md`
- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/ui/*`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/queue_now_playing.rs`
- Backend, database, playback, and network modules

## Constraints

- Implement only the pure descriptor contract for Phase 1.
- Do not wire toolbar submission or placeholder rendering.
- Do not call or modify page view-models from `WorkspaceLayout`.
- Reuse existing `WorkspaceFrameId`, `WorkspaceFrameKind`, and
  `FrameNavigationEntry` types.
- The descriptor must be a projection of the focused frame plus its current
  navigation entry.
- Content-list mount-specific wording may use the current navigation entry; do
  not add GPUI state or app-tab state to `WorkspaceLayout`.
- Empty layouts return `None`.

## Implementation Steps

1. Add `FrameSearchScope` to `src/view_models/workspace.rs`.
2. Add `FrameSearchDescriptor` with `frame_id`, `kind`, `nav`, `scope`, and
   `placeholder` fields.
3. Add `WorkspaceLayout::focused_search_descriptor() -> Option<FrameSearchDescriptor>`.
4. Map focused frame kinds to scopes and placeholders:
   - `SourceList` -> `Sidebar`, `"Filter sidebar..."`
   - `ContentList` with `FrameNavigationEntry::SourceList` -> `LibraryRows`, `"Search library..."`
   - `ContentList` with any other non-search entry -> `SettingsRows`, `"Search settings..."`
   - `Detail` with `FrameNavigationEntry::Search(_)` -> `InspectorQuery`, `"Refine search..."`
   - `Detail` with entity-detail navigation -> `DetailTracks`, `"Filter tracks..."`
   - `QueueNowPlaying` -> `QueueRows`, `"Filter queue..."`
5. Add focused unit tests in `workspace.rs` for each descriptor branch.
6. Add or strengthen an architecture guard proving the descriptor lives in the
   workspace VM and does not require toolbar or renderer state.

## Acceptance Criteria

- [x] `focused_search_descriptor()` returns `None` for an empty layout.
- [x] Each frame kind and relevant detail navigation projects the correct scope and
  placeholder.
- [x] The returned descriptor carries the focused frame id, kind, and cloned current
  navigation entry.
- [x] No app, backend, database, or playback files change.
- [x] Tests cover descriptor projection and are deterministic.

## Implementation Notes

- Added `FrameSearchScope`, `FrameSearchDescriptor`, and
  `WorkspaceLayout::focused_search_descriptor()` in
  `src/view_models/workspace.rs`.
- The only UI file touched in Phase 1 was a compile-only
  `QueueNowPlayingPageVm` destructuring update after queue VM state gained
  private fields; no toolbar or search dispatch wiring was added.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test workspace
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/active-frame-search-dispatch-plan.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/adr/0047-library-search-unification.md`
- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

Goal:
- Add the GPUI-free focused-frame search descriptor to
  `WorkspaceLayout`.

Constraints:
- Descriptor only. Do not wire toolbar submit, button rendering, app dispatch,
  or page-VM mutation.
- Reuse existing workspace navigation types.
- Keep the descriptor as a pure projection from `WorkspaceLayout`.
- Empty layouts return `None`.

Do not touch:
- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/ui/*`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/queue_now_playing.rs`
- Backend, database, playback, and network modules

Acceptance criteria:
- `FrameSearchScope`, `FrameSearchDescriptor`, and
  `focused_search_descriptor()` exist in `src/view_models/workspace.rs`.
- Unit tests cover source-list, content-list, search-detail, entity-detail,
  queue, and empty-layout cases.
- Architecture guard records that toolbar search must route through the
  workspace descriptor contract.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test workspace`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The descriptor appears to require GPUI, app-tab, or renderer state.
- Placeholder wording cannot be projected from frame kind and current
  navigation entry.
- The implementation requires edits outside the allowed files.
