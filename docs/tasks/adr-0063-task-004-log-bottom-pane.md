# ADR 0063 Task 004: Log Output Bottom Pane

Status: Ready - 2026-09-08. Do after task 003. It moves the log output that task
003 put in the side panel.

## Goal

Move service log output out of the detail panel and into a pane across the
bottom of the main region. The pane resizes and closes.

## Files To Inspect

- `docs/adr/0063-show-dashboard-layout.md`, the `Log Output Is A Bottom Pane`
  section
- `docs/tasks/adr-0063-task-003-collapsible-detail-panel.md`
- `src/ui/composites/show_detail_panel.rs`, for the log render it gives up
- `src/ui/shells/show.rs`, for the main region and the transport
- `src/view_models/show.rs`, for the panel mode and the publisher log state
- `src/app/show.rs`, for the log actions and the log read
- `src/ui/shells/workspace.rs`, for the existing resizable split precedent
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/composites/show_log_pane.rs` (new)
- `src/ui/composites/show_detail_panel.rs`
- `src/ui/composites/mod.rs`
- `src/ui/shells/show.rs`
- `src/view_models/show.rs`
- `src/app/show.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/**`, `src/runtime/**`
- The card contract and the width class
- The cuelist mode of the panel, and the queue display contract
- The transport. Task 003 placed it, and this task keeps it reachable.

## Constraints

- **The log pane belongs to `Show`, not to the panel.** It renders in the main
  region, under the card grid, beside the transport.
- The pane resizes. Take the drag precedent from `src/ui/shells/workspace.rs`
  and reuse it. Add no second resize implementation.
- The pane closes. A closed pane leaves the card grid and the transport working.
- The pane names the unit it shows. An operator reads two services from one
  pane, one at a time, so the name is not optional.
- **The pane stays open while the operator selects another card.** It closes
  only from its own close control.
- **The `Logs` action cycles.** A second press on the service already shown
  closes the pane. A press on the other service switches the pane to it, and
  does not close it. The action keeps its accessibility label.
- The detail panel keeps the service rows and the service actions. It gives up
  the log text only.
- The card grid still does not scroll. The pane scrolls its own content.
- **Log text is not truncated per line.** `docs/troubleshooting/column-text-truncation.md`
  records why `truncate()` is wrong for stacked text. Use `overflow_hidden()`,
  or wrap the line.

## Implementation Steps

1. Add `ShowLogPaneDisplay` to `src/view_models/show.rs`:
   - the unit name label
   - the log text and the line count
   - an open flag and a height, both display-ready
   - a close action with an accessibility label
2. Add the pane state to `ShowPageVm`, beside the panel state. The pane state is
   independent of `ShowPanelMode`, because the pane survives a card change.
3. Add `src/ui/composites/show_log_pane.rs`. It renders the display contract and
   builds no string.
4. Render the pane in `render_show`, under the card grid and above or beside the
   transport. Keep the transport reachable when the pane is open and when it is
   closed.
5. Reuse the workspace resize handle. The pane height is a view-model value that
   the drag updates.
6. Delete the log render from `src/ui/composites/show_detail_panel.rs`, and any
   helper that only it reached.
7. Wire the `Logs` action of both services to open the pane for that unit.
8. Add view-model tests:
   - the `Logs` action of a service opens the pane and names that unit
   - selecting another card leaves the pane open and its text unchanged
   - the close action closes the pane
   - closing the pane leaves the panel mode unchanged
9. Add a guard: `src/ui/composites/show_detail_panel.rs` renders no log text,
   and the log pane composite holds no `truncate()` on a stacked line.

## Acceptance Criteria

Mechanical, proved by a test:

- The pane state lives on `ShowPageVm` and is independent of `ShowPanelMode`.
- A `Logs` action opens the pane and names the unit it shows.
- A second `Logs` press on the same service closes the pane, and a press on
  the other service switches the pane without closing it.
- Selecting another card leaves the pane open with the same text.
- The close action closes the pane and changes no panel state.
- The detail panel holds no log text render.
- The log pane holds no `truncate()` on a stacked line.
- One resize implementation exists, shared with the workspace.

Visual, operator only:

- A log line reads on one line at the pane width, without wrapping into a
  paragraph.
- The pane resizes by dragging, and the card grid keeps working at every height.
- The pane closes, and the transport stays reachable in both states.
- Selecting another card while the pane is open does not disturb the pane.

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

- The workspace resize handle cannot serve a horizontal split without a change
  that this task does not cover. Report it before you write a second handle.
- The transport and the pane cannot both stay reachable at the smallest window
  height. Report the height and what you would give up.
- The pane and the card grid together need a scroll region on the grid. That
  contradicts an ADR 0063 invariant. Report it, do not add the scroll.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0063-show-dashboard-layout.md`, `Log Output Is A Bottom Pane`
- `docs/troubleshooting/column-text-truncation.md`
- `src/ui/composites/show_detail_panel.rs`, `src/ui/shells/show.rs`
- `src/ui/shells/workspace.rs` for the resize precedent

Goal:
- Move the service log out of the side panel and into a resizable, closable pane
  across the bottom of the main region.

Constraints:
- The pane belongs to `Show`. It survives a card change and closes only from its
  own control.
- Reuse the workspace resize handle. Do not write a second one.
- The pane names the unit it shows.
- No `truncate()` on a stacked log line. Use `overflow_hidden()`.
- The card grid still does not scroll. The transport stays reachable.

Do not touch:
- `src/broadcast/**`, `src/runtime/**`, the card contract, the queue contract

Acceptance criteria:
- The pane state is independent of the panel mode, and tests prove it.
- A `Logs` action opens the pane for that unit.
- The detail panel renders no log text.

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
