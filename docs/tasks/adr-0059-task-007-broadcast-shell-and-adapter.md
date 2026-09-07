# ADR 0059 Task 007: Broadcast Shell And Frame Adapter

## Goal

Render the `Broadcast` frame from `BroadcastPageVm` with shared primitives, and
wire the frame adapter. Add the phase guards.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `src/ui/shells/queue_now_playing.rs` (shell precedent)
- `src/app/queue_now_playing.rs` (adapter precedent)
- `src/ui/primitives/**`, `src/ui/composites/**`
- `src/view_models/broadcast.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/shells/broadcast.rs` (new)
- `src/ui/shells/mod.rs`
- `src/app/broadcast.rs` (new)
- `src/app.rs` (module declaration and frame build)
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/view_models/broadcast.rs` (task 005 owns the contract)
- `src/broadcast/**`
- `src/db.rs`, `src/api.rs`
- `src/ui/shells/queue_now_playing.rs`

## Constraints

- The shell reads the view model only. It holds no service handle, opens no
  file, and runs no command.
- The shell imports no screen module and no backend module.
- Use shared primitives and composites. No raw glyph strings, no one-off
  disabled styling, and no local color or size literals.
- Three visually separate sections in a fixed order: `Source`, `Publisher`,
  `Event`. Apple guidance for macOS asks for fewer nested levels and less
  modality, so keep the sections inline and avoid a wizard.
- Every action button carries the accessibility label from the view model.
- The empty states come from the view model. The shell writes no fallback text.
- The section content must scroll inside its own bounded container. Verify the
  scroll chain, as the AGENTS.md ratchet requires.

## Implementation Steps

1. Add `src/ui/shells/broadcast.rs` with `render_broadcast` and a
   `BroadcastSlots` builder for the callbacks, in the shape of the queue shell.
2. Slots: create event, resume event, forget event, start service, stop service,
   reset service, open logs, and select source.
3. Render the three sections with the shared section or card composite that the
   repository already uses. Do not invent a new container.
4. Render each action through the shared button primitive with its typed
   availability.
5. Render the empty states with the shared empty-state composite.
6. Add `src/app/broadcast.rs` with `build_broadcast_frame`, which projects the
   view model and binds the slots, in the shape of
   `src/app/queue_now_playing.rs`.
7. Declare both modules and add the frame to the workspace render path.
8. Add architecture guards, in the shape of the queue frame guards:
   - the shell holds `render_broadcast` and the section render helpers
   - the shell imports no screen or backend module
   - the adapter holds `build_broadcast_frame`
   - `src/app.rs` declares the adapter module and builds the frame
9. Capture a screenshot of the frame in the running app with a live event, a
   dead event, and a publisher that is not installed.

## Acceptance Criteria

- The frame renders the three sections in order.
- Disabled actions come from the view model, not from renderer conditionals.
- No raw glyph string, color literal, or size literal in the new shell.
- The panes scroll inside bounded height.
- The four architecture guards pass.
- Screenshots exist for the three states above.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo run` for the visual check

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Screenshots captured
5. Deviations from task
6. Unresolved concerns

## Escalation Triggers

- No shared composite fits a section and a new composite is needed. Say so
  before you write a screen-local container.
- The view model does not carry a label that the shell needs. Do not add the
  label in the shell.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `src/ui/shells/queue_now_playing.rs`
- `src/app/queue_now_playing.rs`
- `src/view_models/broadcast.rs`

Goal:
- Add the broadcast shell and the frame adapter, and add the phase guards.

Constraints:
- The shell reads the view model only. No service handle, no command, no file.
- Shared primitives and composites only. No raw glyphs, colors, or sizes.
- Three inline sections: Source, Publisher, Event.
- Empty states come from the view model.
- Panes scroll inside bounded height.

Do not touch:
- `src/view_models/broadcast.rs`, `src/broadcast/**`, database, API

Acceptance criteria:
- Guards for shell contents, shell imports, adapter, and app wiring all pass.
- Screenshots for live event, dead event, and publisher not installed.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. screenshots captured
5. deviations from task
6. unresolved concerns
