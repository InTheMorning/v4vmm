# ADR 0046 Task 007: Workspace Layout Render

Status: Proposed - 2026-05-14.

## Goal

Render `TopApp` body as a transitional `WorkspaceLayout` using the frame shell
composite. Existing Library, Search, and Settings screens are mounted whole
inside workspace frames; this task does **not** split their current internal
split panes into separate `SourceList`, `ContentList`, and `Detail` slots.
`QueueNowPlaying` renders as a placeholder. Old tab rendering remains
reachable for fallback in this task; later tasks remove it.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-006-frame-shell-composite.md`
- `docs/tasks/adr-0046-task-006a-screen-mount-boundaries.md`
- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui/shells/library/*`
- `src/ui/composites/frame_shell.rs`

## Files Likely to Change

- `src/app.rs`
- `src/ui/shells/workspace.rs` (new)
- `src/library.rs` or `src/search.rs` (mount-point only, if needed)

## Do Not Touch

- Page VM internals
- Screen-local logic, search, settings, library command paths
- `src/db.rs`, playback engine
- `src/app/tab_bar.rs` global search behavior

## Constraints

- Additive: prior tab rendering remains reachable behind a feature
  flag or sibling code path until the workspace render is verified.
- Page VMs and shell helpers render unchanged inside the whole mounted screens.
- No inspector grows a back button.
- QueueNowPlaying frame renders as a placeholder until task 010 lands.
- Reuse `frame_shell` composite for every frame.
- Do not copy Library/Search split-pane internals into
  `src/ui/shells/workspace.rs`; this phase wraps existing screens.

## Implementation Steps

1. Add `src/ui/shells/workspace.rs` exporting `render_workspace`.
2. `render_workspace` consumes a `WorkspaceLayout` projection and a
   slot map keyed by frame kind, returning a row of frames.
3. Wire `TopApp::render` to build a transitional default `WorkspaceLayout` and
   call `render_workspace`. The active Library/Search/Settings entity remains a
   whole mounted content child. Queue renders a placeholder frame.
4. Add a feature flag or build-time toggle to fall back to the prior
   tab rendering if the workspace render misbehaves.
5. Architecture guard: assert `frame_shell` is the only composite
   used to render frame chrome in `src/ui/shells/workspace.rs`.

## Acceptance Criteria

- [ ] `src/ui/shells/workspace.rs` renders the default layout via
  `frame_shell` composites.
- [ ] Library, Search, and Settings screens render inside workspace frames
  without behavior change.
- [ ] Workspace shell does not duplicate Library/Search split-pane internals.
- [ ] No inspector grows a back button.
- [ ] Prior tab rendering still reachable for fallback.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-006-frame-shell-composite.md`
- `docs/tasks/adr-0046-task-006a-screen-mount-boundaries.md`
- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/library.rs`, `src/search.rs`
- `src/ui/composites/frame_shell.rs`

Goal:
- Render `TopApp` body as a default `WorkspaceLayout` using
  `frame_shell` for every frame.

Constraints:
- Additive; prior tab rendering reachable for fallback.
- Page VMs and shell helpers render unchanged inside whole mounted screens.
- Reuse `frame_shell` everywhere.
- QueueNowPlaying renders a placeholder pending task 010.

Do not touch:
- Page VM internals
- `src/db.rs`, playback engine

Acceptance criteria:
- `src/ui/shells/workspace.rs` exists and renders the default layout.
- Library/Search/Settings render inside workspace frames without splitting
  their internals.
- No inspector back button is introduced.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Switching the render path forces refactors inside Library or Search
  beyond the mount-point change.
- `frame_shell` composite cannot host the content slot for an
  existing page VM without a signature change.
