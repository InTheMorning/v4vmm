# ADR 0065 Task 002: Readiness List Actions

Status: Ready - 2026-09-08. Do after task 001, which writes the tag this task
asks for.

## Goal

Give the broadcast readiness list an action that fixes a row, and stop the
`Missing routes` state from reading as a control.

## Files To Inspect

- `docs/adr/0065-payment-route-tag-repair.md`
- `docs/tasks/adr-0065-task-001-tag-repair-service.md`
- `src/view_models/library.rs`, `from_broadcast_readiness_track` and
  `state_label`
- `src/ui/shells/library/**`, for the row badge strip
- `src/view_models/show.rs`, for the `Source` card readiness rows
- `docs/adr/0062-music-content-surface.md`, for the row contract
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/library.rs`
- `src/ui/shells/library/**`
- `src/app/library*.rs` or the owner that dispatches list actions
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/application/commands/payment_routes.rs`. Task 001 owns the repair.
- `src/feed_service.rs`
- `src/broadcast/**`, `src/runtime/**`
- The `Show` card contract from ADR 0063

## Constraints

- **A state label carries no click target.** `Missing routes` renders as a
  state, like `In library`. It is not a button and must not look like one. This
  is an ADR 0065 invariant, written because the label looked pressable and did
  nothing.
- The row action is a separate control with its own label, its own
  accessibility label, and a typed availability.
- **A row whose feed carries no routes offers no repair action.** Task 001
  reports `NoRoutesUpstream`, and this app can not fix that track. Say what the
  operator must do instead: the publisher adds the routes.
- The list also carries a repair-all action. Seventeen rows is not a per-row
  job, and that count is the reported case.
- While a repair runs, the row reports it. Follow the `Working` precedent in
  `src/view_models/show.rs`, which answers a press at once instead of waiting
  for the result.
- The view model owns every label and every availability rule. The renderer
  adapts the contract, as ADR 0062 requires.
- Text stacked in a row does not call `truncate()`. See
  `docs/troubleshooting/column-text-truncation.md`.

## Implementation Steps

1. Separate the state badge from the action in
   `ContentListRowDisplay::from_broadcast_readiness_track`. The state keeps
   `state_label`. The action is a new optional field with an identifier, a
   label, an accessibility label, and a typed availability.
2. Set the action availability from the readiness state:
   - `NoRouteTag` gives an available `Fix routes` action
   - `NoRoutesUpstream` gives no action, and the row says the publisher must add
     the routes
   - `FileMissing` and `NotDownloaded` give no repair action, because this
     record does not cover them
3. Add a repair-all action to the readiness list header, with a count in its
   label.
4. Wire both actions to the task 001 commands.
5. Mark a row as working while its repair runs, and clear it from the result.
6. Refresh the readiness snapshot after a repair completes, so the list shrinks
   without a manual reload.
7. Add view-model tests:
   - a `NoRouteTag` row carries an available action
   - a `NoRoutesUpstream` row carries no action and names the publisher
   - a running row reports it, and the action is unavailable meanwhile
   - the repair-all label names the count
8. Add a guard: no readiness row renders a state label with a click handler, and
   the row action label is built in the view model.

## Acceptance Criteria

Mechanical, proved by a test:

- A `NoRouteTag` row carries an available action with an accessibility label.
- A `NoRoutesUpstream` row carries no repair action and names what must happen.
- A row reports that a repair is running, and its action is unavailable then.
- The repair-all label names the number of rows it will attempt.
- The readiness snapshot refreshes after a repair, without an operator reload.
- The guard blocks a click handler on a state label.

Visual, operator only:

- `Missing routes` reads as a state and not as a button.
- The repair action is the obvious thing to press on a not-ready row.
- Pressing it changes the row at once, before the repair finishes.
- After a repair, the list holds only the rows a publisher must fix, and the row
  says so.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
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

- The ADR 0062 row contract can not carry a row action without a change that
  this task does not cover. Report it before you add a second row type.
- The readiness list header has no place for a repair-all action. Report the
  layout, and do not put the action inside `Show`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0065-payment-route-tag-repair.md`
- `docs/tasks/adr-0065-task-001-tag-repair-service.md`
- `src/view_models/library.rs`, `from_broadcast_readiness_track`

Goal:
- A repair action on a readiness row and on the list, and a state label that
  stops looking like a control.

Constraints:
- A state label has no click target. The action is a separate control.
- A `NoRoutesUpstream` row offers no repair, and says the publisher must act.
- A running row reports it at once. Follow the `Working` precedent in
  `src/view_models/show.rs`.
- The view model owns every label and availability rule.
- No `truncate()` on stacked row text.

Do not touch:
- `src/application/commands/payment_routes.rs`, `src/feed_service.rs`,
  `src/broadcast/**`, `src/runtime/**`

Acceptance criteria:
- A `NoRoutesUpstream` row carries no action and names what must happen.
- A guard blocks a click handler on a state label.
- The readiness snapshot refreshes after a repair.

Test commands:
- `cargo fmt -- --check`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
6. operator visual check
