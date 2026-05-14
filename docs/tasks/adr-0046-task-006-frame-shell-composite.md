# ADR 0046 Task 006: Frame Shell Composite

Status: Implemented - 2026-05-14.

## Goal

Add a shared `frame_shell` composite at
`src/ui/composites/frame_shell.rs` that renders `FrameShellDisplay`.
Owns title/subtitle/status chrome, back/forward/close buttons,
action menu, and a content slot supplied by callers. No screen owns
this chrome.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/adr/0038-presentation-contract-enforcement.md`
- `docs/tasks/adr-0046-task-005-frame-shell-display-vm.md`
- `src/view_models/workspace.rs`
- `src/ui/composites.rs`
- `src/ui/composites/file_header.rs` (composite precedent)
- `src/ui/primitives/button.rs`, `context_menu.rs`
- `src/ui/icons.rs`

## Files Likely to Change

- `src/ui/composites/frame_shell.rs` (new)
- `src/ui/composites.rs`
- `src/ui/icons.rs` (add back/forward chevrons if absent)

## Do Not Touch

- `src/library.rs`, `src/search.rs`, `src/app.rs`
- Page VM internals
- `src/db.rs`, playback engine

## Constraints

- Composite signature accepts `FrameShellDisplay` plus callbacks; no
  raw `String`/`&str` policy params, no `db::*` types.
- No raw `rgb(...)` or `px(...)` literals; consume named tokens.
- No `.absolute()`, `.fixed()`, `.z_index(...)`, or `gpui_component::popover`
  outside permitted primitives.
- Reuse existing Button/IconButton/Menu primitives. No hand-rolled
  floating chrome.
- Apple HIG: back/forward render as chevron glyphs (not text); close
  uses the standard close glyph; title placement matches macOS frame
  convention.
- Composite takes a content-slot child (`impl IntoElement` or
  equivalent) for the body.

## Implementation Steps

1. Add `IconName::ChevronLeft`, `IconName::ChevronRight`, and a close
   glyph entry to `src/ui/icons.rs` if not already present.
2. Add `frame_shell.rs` module and re-export from
   `src/ui/composites.rs`.
3. Implement `pub fn frame_shell(display: FrameShellDisplay, slots:
   FrameShellSlots) -> impl IntoElement` (or struct-builder
   equivalent) that lays out chrome row + content slot.
4. `FrameShellSlots` carries content child + callbacks: on_back,
   on_forward, on_close, on_menu_select.
5. Disabled visual state honors `display.back.disabled` /
   `display.forward.disabled` (no click handler when disabled).
6. Action menu items render through the existing menu primitive,
   reading from `display.action_menu_items`.
7. Unit-style render tests if the project pattern supports them;
   otherwise rely on architecture guards added in task 008.

## Acceptance Criteria

- [x] `frame_shell` composite exists and consumes `FrameShellDisplay`.
- [x] Back/forward/close render through the icon catalog, not raw
  glyph strings.
- [x] No raw color/spacing literals in the composite.
- [x] No screen module changed.
- [x] Composite slots accept a content child supplied by the caller.

## Implementation Notes

- Added `src/ui/composites/frame_shell.rs` with `FrameShell`,
  `FrameShellSlots`, and `frame_shell(...)`.
- The composite consumes `FrameShellDisplay`, renders title/subtitle/status,
  history buttons, optional close, optional action menu, and a caller-owned
  content child.
- Added semantic icon catalog entries for `ChevronLeft`, `ChevronRight`, and
  `Close`.
- Added a `WorkspaceFrame` context-menu scope so frame action menus do not
  reuse row/search semantics.
- Added an architecture guard for the shared frame-shell composite contract.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/tasks/adr-0046-task-005-frame-shell-display-vm.md`
- `src/view_models/workspace.rs`
- `src/ui/composites.rs`
- `src/ui/composites/file_header.rs`
- `src/ui/primitives/button.rs`
- `src/ui/icons.rs`

Goal:
- Add `src/ui/composites/frame_shell.rs` rendering `FrameShellDisplay`
  with content-slot support and a back/forward/close/menu chrome row.

Constraints:
- No raw glyph strings, raw `rgb()`/`px()` literals, or floating
  chrome APIs.
- Reuse existing primitives.
- HIG-compliant chevron/close glyphs.

Do not touch:
- Screens (`src/library*`, `src/search*`, `src/app*`)
- `src/db.rs`, playback engine

Acceptance criteria:
- Composite compiles, consumes the display contract, supports a
  content child.
- Icon catalog references only.
- No screen change.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- GPUI APIs require `.absolute()`/`.fixed()` to anchor frame chrome
  (escalate before adding raw floating-chrome calls).
- No suitable existing primitive renders the action menu without
  hand-rolling popover anchoring.
