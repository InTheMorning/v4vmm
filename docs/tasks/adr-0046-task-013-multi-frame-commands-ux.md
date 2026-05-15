# ADR 0046 Task 013: Multi-Frame Commands UX

Status: Deferred - blocked by missing per-frame content owners - 2026-05-15.

## Goal

Keep frame add/remove operations model-only until additional frames can render
independent content. Add Phase 5 architecture guards that prevent the
transitional whole-screen Library/Search/Settings mount from exposing fake
"open new frame" menu items or keybindings.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-012-frame-add-remove-and-persistence.md`
- `src/ui/composites/frame_shell.rs`
- `src/ui/shells/workspace.rs`
- `src/app.rs` (keybinding registration)
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0046-review-checklist.md`

## Files Likely to Change

- `src/ui/composites/frame_shell.rs`
- `src/ui/shells/workspace.rs`
- `src/app.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0046-review-checklist.md`

## Do Not Touch

- Playback engine, mpv driver
- `src/db.rs`
- Page VM internals
- Toolbar global search

## Constraints

- Apple HIG: do not expose commands that create misleading duplicate windows or
  frames. Frame chrome may keep shared menu plumbing, but unavailable actions
  must not appear as active controls.
- `WorkspaceLayout::add_frame` / `remove_frame` stay in the VM from Task 012.
  They are not wired to user-visible frame chrome in this task.
- No toolbar menu in this slice (per ADR 0046 open-question 5).
- No `Cmd+Shift+N`, `Ctrl+Shift+N`, `Cmd+W`, or `Ctrl+W` workspace-frame
  keybindings until a real per-frame content owner exists.
- Persisted extra `ContentList` frames from earlier experiments are projected
  out of the visible transitional layout so they do not render identical
  Library/Search/Settings copies.
- Breadcrumbs, move/resize controls, and independent per-frame Library/Search/
  Settings routing are non-goals for this task. They belong to later
  ADR 0047/frame-navigation work.

## Implementation Steps

1. Keep `FrameShellDisplay.action_menu_items` empty for the transitional
   workspace.
2. Keep `frame_shell` generic menu plumbing intact for later real frame
   actions.
3. Remove workspace-frame add/close keybindings and top-level action routing.
4. Project the visible transitional layout to one `ContentList` frame plus
   `QueueNowPlaying`, even when config contains older extra content frames.
5. Add architecture tests that block fake frame actions and duplicate active
   content mounts.
6. Update `docs/reviews/adr-0046-review-checklist.md` with the deferral.

## Acceptance Criteria

- [x] Context-menu add/close items are not exposed while frames would duplicate
  the transitional active screen.
- [x] Workspace-frame keybindings are not registered before they can perform a
  real user-visible action.
- [x] Visible transitional layout projects to a single active content frame plus
  queue.
- [x] Architecture guards lock the deferral and block duplicate frame routing.
- [ ] Review checklist records user visual confirmation that duplicate frames
  no longer appear.

## Implementation Notes

- `FrameShellDisplay` keeps `action_menu_items` empty in the transitional
  workspace; the shared `frame_shell` composite still owns generic context-menu
  chrome through `ContextMenuScope::WorkspaceFrame`.
- `TopApp` does not route `OpenNewContentFrame` or `CloseFocusedFrame` actions
  while `ContentList` lacks a real page VM.
- The initial visible workspace projects persisted layouts into the current
  transitional frame set (`ContentList` + `QueueNowPlaying`) so older full ADR
  defaults do not reintroduce SourceList/Detail placeholders or duplicate
  ContentList frames before those slots have real content owners.
- User feedback showed the frame action menu first opened an unusable
  placeholder frame, then a misleading duplicate Library frame. The bounded
  Task 013 correction is to hide/defer the command instead of creating fake
  frames.
- ADR 0047 Task 010 remains responsible for real per-frame content-list page
  state and independent per-frame Library/Search/Settings routing.
- Local visual smoke was attempted with `cargo run` and
  `LIBGL_ALWAYS_SOFTWARE=1 cargo run`; GPUI failed to initialize the X11/GPU
  context before a window opened. Operator visual confirmation is still required
  before closing Phase 5.

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
- `docs/tasks/adr-0046-task-012-frame-add-remove-and-persistence.md`
- `src/ui/composites/frame_shell.rs`
- `src/ui/shells/workspace.rs`
- `src/app.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0046-review-checklist.md`

Goal:
- Defer add/close frame commands until real per-frame content owners exist,
  keep the focus indicator, add architecture guards, and collect visual proof
  that duplicate frames are not exposed.

Constraints:
- Existing menu primitive stays generic, but unavailable frame actions do not
  render.
- VM add/remove operations stay model-only in this task.
- No toolbar menu.

Do not touch:
- Playback engine, mpv driver
- `src/db.rs`
- Page VM internals
- Toolbar global search

Acceptance criteria:
- Menu items + keybindings for unavailable frame actions are absent.
- Visible transitional layout has one content frame plus queue.
- Guards confirm command deferral and prevent duplicate active content frames.
- Review checklist updated.

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

- Chosen keybinding chord conflicts with an existing global shortcut.
- `frame_shell` cannot host context menu items without a signature
  change beyond slots.
