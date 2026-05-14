# ADR 0046 Task 013: Multi-Frame Commands UX

Status: Proposed - 2026-05-14.

## Goal

Expose user-visible commands for adding and removing frames via the
frame-chrome context menu plus a keybinding. Add Phase 5 architecture
guards and visual proof (default layout, second content frame open,
focus indicator) in light and dark themes.

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

- Apple HIG: context menu items render through the existing menu
  primitive. Close button uses the standard close glyph. Focus
  indicator uses existing focus tokens.
- Commands route through the workspace VM (`add_frame`,
  `remove_frame`), never through screen-local state.
- No toolbar menu in this slice (per ADR 0046 open-question 5).
- Keybindings: default to a single chord (e.g., `Cmd+Shift+N` /
  `Ctrl+Shift+N`) for "Open new content frame" and `Cmd+W` /
  `Ctrl+W` for "Close focused frame". Conflict checks required.
- Visual proof captures: default layout, second content frame open,
  focus indicator visible, frame close affordance present, light and
  dark themes.

## Implementation Steps

1. Add context menu items in `frame_shell` for "Open New Frame" +
   "Close Frame". Hook the items to callbacks supplied via slots.
2. Wire the workspace shell to dispatch these callbacks to
   `WorkspaceLayout::add_frame` / `remove_frame`.
3. Register keybindings in `src/app.rs`. Document the chosen chords
   in the review checklist.
4. Add a focus indicator on the focused frame in the workspace shell
   using existing focus-state tokens.
5. Add architecture tests:
   - `frame_shell` exposes "Open New Frame" / "Close Frame" menu item
     identifiers.
   - workspace shell dispatches frame add/remove through workspace VM
     ops, not screen-local state.
   - keybinding registration exists for the chosen chords.
6. Update `docs/reviews/adr-0046-review-checklist.md` with Phase 5
   readiness section and visual-proof entries.

## Acceptance Criteria

- [ ] Context-menu add/close items render and dispatch to workspace
  VM ops.
- [ ] Keybindings registered without conflicting with existing
  global commands.
- [ ] Focus indicator visible on the focused frame in both themes.
- [ ] Architecture guards lock the command routing and menu identifiers.
- [ ] Review checklist records readiness with visual proof.

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
- Expose add/close frame commands via context menu + keybinding,
  with focus indicator, architecture guards, and Phase 5 visual
  proof.

Constraints:
- Existing menu primitive only; standard close glyph; existing focus
  tokens.
- Commands route through workspace VM ops, never screen-local state.
- No toolbar menu.

Do not touch:
- Playback engine, mpv driver
- `src/db.rs`
- Page VM internals
- Toolbar global search

Acceptance criteria:
- Menu items + keybindings exist and dispatch correctly.
- Focus indicator renders in both themes.
- Guards confirm command routing and menu identifiers.
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
