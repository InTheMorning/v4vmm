# ADR 0063 Task 003: Collapsible Detail Panel

Status: Mechanical implemented - 2026-09-08; operator visual check open.
This task closes ADR 0063 after the visual check passes.

## Goal

Add the trailing panel. It shows the cuelist by default, and shows the detail of
one selected card when the operator selects a card. It closes, and the transport
keeps working when it is closed.

## Files To Inspect

- `docs/adr/0063-show-dashboard-layout.md`
- `docs/tasks/adr-0063-task-001-card-contract-and-width-class.md`
- `docs/tasks/adr-0063-task-002-card-grid-shell.md`, and the module comments it
  leaves behind for the detail content
- `src/ui/shells/show.rs`
- `src/view_models/show.rs`, for the four section displays and
  `PublisherLogPanelState`
- `src/app/show.rs`, for the publisher command wiring and the log actions
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/shells/show.rs`
- `src/ui/composites/show_detail_panel.rs` (new)
- `src/ui/composites/mod.rs`
- `src/view_models/show.rs`
- `src/app/show.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/**`, `src/runtime/**`
- `QueueNowPlayingPageVm` and its display contract. Only its container changes.
- The four section display types. Their fields become panel content unchanged.

## Constraints

- **The panel shows the cuelist or one card detail, never both.** This is an
  ADR 0063 invariant. A mode enum holds it, not two booleans.
- **The transport stays outside the panel.** Move it out of the queue container
  and onto `Show`, below the card grid. An operator who closes the panel keeps
  play, pause, and skip. This is an ADR 0063 invariant.
- The publisher log is panel content for the `Live Metadata` detail. Delete the
  inline log strip. `PublisherLogPanelState` moves to the panel, or is replaced
  by the panel mode. Do not keep both.
- Selecting a card while the panel is closed opens the panel in detail for that
  card.
- Closing a detail returns the panel to the cuelist. It does not close the
  panel.
- The panel scrolls its own content. The card grid still does not scroll.
- The panel width comes from a token. The card grid keeps working at every width
  class when the panel is open and when it is closed.
- Every action keeps its accessibility label. A closed panel is a state, not a
  removed control.

## Implementation Steps

1. Add `src/ui/composites/show_detail_panel.rs`. It takes the panel mode, the
   open flag, the queue view model and slots, and the four section displays.
2. Render the cuelist mode with `render_queue_now_playing`, unchanged.
3. Render the detail mode for each card kind, from the section display that task
   002 stopped rendering:
   - `Source`: the host rows and the readiness rows, with the readiness action
   - `Live Metadata`: the service rows, the actions, and the log output
   - `Event`: the event rows, the feed tag, and the attach actions
   - `Stream`: the connection rows and the encoder actions
4. Add a header to the detail mode with the card title and a control that
   returns the panel to the cuelist.
5. Add a control that closes the panel, and a control that opens it.
6. Move the transport out of the queue container. Render it on `Show`, below the
   card grid, so it is visible when the panel is closed.
7. Wire `on_select_card` from task 002 to the panel mode in `src/app/show.rs`.
8. Delete the inline log panel from the shell and its state, and delete any
   helper that only it reached.
9. Add view-model tests:
   - selecting a card sets the panel mode to detail for that card
   - closing a detail returns the mode to the cuelist
   - selecting a card while closed opens the panel
   - the mode never holds the cuelist and a detail at once
10. Add a guard: the transport renderer is not inside the panel composite, and
    the card grid holds no scroll container.
11. Update `docs/pending-human-checks.md` with the visual check below.

## Acceptance Criteria

Mechanical, proved by a test:

- The panel mode holds the cuelist or one card kind, never both.
- Selecting a card sets detail mode for that card, and opens a closed panel.
- Closing a detail returns to the cuelist and leaves the panel open.
- The transport renders outside the panel composite.
- The inline log strip and its state are gone, with no dead helper left.
- The card grid holds no scroll container.

Visual, operator only:

- The panel opens, closes, and keeps the card grid usable in both states.
- Selecting a card shows its detail, and the cuelist returns when the detail
  closes.
- The publisher log reads correctly in the panel, at the panel width.
- The transport is reachable while the panel is closed.
- No layout moves when a service changes state while a detail is open.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test show --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

Do not run the app. Write the operator visual check instead, as AGENTS.md
requires.

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns
6. Operator visual check

## Escalation Triggers

- A section detail needs an action that the current slots do not carry.
- The transport cannot move out of the queue container without a change to the
  queue display contract. Report it. Do not change that contract here.
- The panel width and the card grid cannot both work at the compact width class.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0063-show-dashboard-layout.md`
- `docs/tasks/adr-0063-task-002-card-grid-shell.md`
- `src/ui/shells/show.rs`, `src/view_models/show.rs`, `src/app/show.rs`

Goal:
- Add the trailing panel with two modes, the cuelist and one card detail. Move
  the transport out of the panel.

Constraints:
- One mode enum. Never the cuelist and a detail at once.
- The transport stays reachable when the panel is closed.
- The publisher log becomes panel content. Delete the inline strip and its
  state.
- The panel scrolls. The card grid does not.

Do not touch:
- `src/broadcast/**`, `src/runtime/**`, the queue display contract

Acceptance criteria:
- Selecting a card opens detail for it, and opens a closed panel.
- Closing a detail returns to the cuelist and leaves the panel open.
- The transport renders outside the panel composite.

Test commands:
- `cargo fmt -- --check`
- `cargo test show --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
6. operator visual check
