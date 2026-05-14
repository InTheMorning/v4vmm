# ADR 0047 Task 012: Frame Breadcrumb VM + Multi-Frame Nav State

Status: Proposed - 2026-05-14.

## Goal

Generalize `FrameNavigationState` ownership from `LibraryApp` to the
workspace VM keyed by `WorkspaceFrameId`, and add a `BreadcrumbDisplay`
projection that downstream frame-chrome rendering (task 013) consumes.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-002-frame-history-vm.md`
- `src/view_models/workspace.rs`
- `src/library/app_impl.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/workspace.rs`
- `src/library/app_impl.rs` (move nav-state ownership out)
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/*` composites (chrome render lands in task 013)
- Backend, db, playback

## Constraints

- Workspace VM gains a `frame_navigation: HashMap<WorkspaceFrameId,
  FrameNavigationState>` (or equivalent typed map).
- Existing `LibraryApp`-owned nav state migrates to the workspace
  VM. `LibraryApp` reads/writes via workspace VM helpers.
- `BreadcrumbDisplay { id: String, segments: Vec<BreadcrumbSegment>,
  truncation: BreadcrumbTruncation }` and `BreadcrumbSegment { id,
  label, a11y_label, is_current }`.
- Truncation policy: middle ellipsis. Leftmost (origin) + rightmost
  (current) stay visible.
- Projection consumes `FrameNavigationState`.
- Observable behavior unchanged until task 013 renders breadcrumbs.

## Implementation Steps

1. Add `frame_navigation` map to the workspace VM.
2. Add helpers: `frame_nav(id)`, `frame_nav_mut(id)`,
   `push_nav(id, entry)`, `pop_nav(id) -> Option<entry>`.
3. Migrate `LibraryApp::frame_navigation` callers to the workspace
   VM helpers. `LibraryApp` no longer owns the field.
4. Add `BreadcrumbDisplay` + `BreadcrumbSegment` +
   `BreadcrumbTruncation` enum (`MiddleEllipsis`).
5. Add `BreadcrumbDisplay::project(nav: &FrameNavigationState)`.
6. Unit tests: ownership migration leaves prior behavior intact;
   middle-ellipsis projection collapses correctly; single-segment
   case renders only current; multi-segment case keeps origin +
   current visible.

## Acceptance Criteria

- [ ] Workspace VM owns `frame_navigation` keyed by frame id.
- [ ] `LibraryApp` no longer owns nav state directly.
- [ ] `BreadcrumbDisplay` projector covers single/multi/long cases.
- [ ] Observable behavior unchanged (no UI render of breadcrumbs
  yet).
- [ ] Unit tests cover the boundary cases.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test workspace
cargo test library
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-002-frame-history-vm.md`
- `src/view_models/workspace.rs`
- `src/library/app_impl.rs`
- `tests/architecture_tests.rs`

Goal:
- Move `FrameNavigationState` ownership to the workspace VM keyed by
  frame id; add `BreadcrumbDisplay` projection.

Constraints:
- Observable behavior unchanged.
- Middle-ellipsis truncation.

Do not touch:
- `src/ui/*`
- Backend, db, playback

Acceptance criteria:
- Ownership migrated; projection covers boundary cases; tests pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test workspace`
- `cargo test library`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Migrating nav-state ownership cascades into command paths in
  `LibraryApp` that cannot be cleanly redirected (escalate before
  rewriting unrelated reload paths).
