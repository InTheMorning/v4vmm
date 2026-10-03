# ADR 0063 Task 001: Card Contract And Width Class

Status: Implemented - 2026-09-08. Mechanical acceptance met. No visual
criteria; the app was not run for this view-model-only packet. Tasks 002 and
003 read the contract that this task defines.

## Goal

Give `ShowPageVm` a card summary for each section, a width class, a column
count, and a panel mode. View model only. The shell keeps rendering the old
stack until task 002.

## Files To Inspect

- `docs/adr/0063-show-dashboard-layout.md`
- `docs/adr/0059-broadcast-control-surface.md`, for the section set and order
- `src/view_models/show.rs`, for `ShowPageVm` and the four section displays
- `src/view_models/track_metadata_grid.rs`, for the `columns()` precedent
- `src/view_models/queue_now_playing.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/show.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/**`. This task changes no renderer.
- `src/app/**`, `src/broadcast/**`, `src/runtime/**`
- The four section display types. Add beside them, do not rewrite them.
- `QueueNowPlayingPageVm`

## Constraints

- Zero `gpui` imports.
- A card summary is display-ready text and a typed state. No `Option<String>`
  that changes the height of a card. Every card carries the same field set.
- **Every card summary has the same shape**, so the grid keeps one row height.
  A card carries a title, a state label, a typed state kind, and at most two
  summary lines. Both lines are always present, and an unused line is an empty
  string, never an absent field.
- The width class is a small enum. The column count comes from the width class,
  in the view model. No breakpoint arithmetic reaches the shell.
- The panel mode is an enum with two cases: the cuelist, and a detail for one
  named card. It never holds both.
- A card that is absent contributes no cell. The grid does not reserve a hole
  for it.
- Documented public types.

## Implementation Steps

1. Add `ShowCardKind { Source, LiveMetadata, Event, Stream }`. The order of this
   enum is the order of the grid, and it matches ADR 0059.
2. Add `ShowCardStateKind { Ok, Attention, Failed, Absent, Unknown }`. This is
   the state the card badge shows. It is separate from the service states of
   `Live Metadata`, because a card summarises a whole section.
3. Add `ShowCardDisplay`:
   - `kind: ShowCardKind`
   - `title: &'static str`
   - `state_label: String`
   - `state: ShowCardStateKind`
   - `primary: String` and `secondary: String`, both always present
   - `a11y_label: String`
4. Add `ShowPanelMode { Cuelist, Detail(ShowCardKind) }`, with `Cuelist` as the
   default.
5. Add `ShowWidthClass { Compact, Medium, Wide }` with `columns()` returning
   `1`, `2`, and `3`. Follow `track_metadata_grid.rs`, which returns the count
   from the view model.
6. Add to `ShowPageVm`:
   - `cards: Vec<ShowCardDisplay>`, in `ShowCardKind` order, absent sections
     left out
   - `width_class: ShowWidthClass`
   - `panel_mode: ShowPanelMode`
   - `panel_open: bool`
   Keep the four section fields. Task 003 moves the detail content, not this
   task.
7. Add a projector from each section display to its card summary:
   - `Source`: state from reachability, `primary` is the host label, `secondary`
     is the readiness line or an empty string
   - `Live Metadata`: `Failed` when any service failed, `Attention` when a
     service is not installed or not reachable, `Ok` when every service is
     active, `primary` and `secondary` are the two service lines
   - `Event`: state from the event state, `primary` is the event label,
     `secondary` is the target line or an empty string
   - `Stream`: state from the connection state, `primary` is the connection
     line, `secondary` is the listener line
8. Add unit tests:
   - every card carries a non-empty `primary` and a `secondary` that is present,
     in every state of every section
   - an absent section adds no card
   - the card order matches `ShowCardKind` order
   - each width class returns its column count
   - `ShowPanelMode` defaults to `Cuelist`
9. Add a guard: `src/view_models/show.rs` holds no breakpoint constant, and no
   pixel width, so the shell cannot read layout numbers from the view model
   other than the column count.

## Acceptance Criteria

Mechanical, proved by a test:

- Every card summary carries the same field set, and `primary` and `secondary`
  are present in every state.
- An absent section contributes no card, and the remaining cards keep
  `ShowCardKind` order.
- `ShowWidthClass::columns()` returns `1`, `2`, and `3`.
- `ShowPanelMode` defaults to `Cuelist` and holds one card at most.
- The card state for `Live Metadata` is `Failed` when any service failed.
- The module imports no `gpui`.

Visual, operator only:

- None. This task changes no renderer.

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
6. Operator visual check, when this packet has visual criteria

## Escalation Triggers

- A section carries a fact that does not fit two summary lines. Report which
  section and which fact. Do not add a third line to one card only.
- `ShowCardStateKind` cannot express a section state without losing meaning.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0063-show-dashboard-layout.md`
- `src/view_models/show.rs`
- `src/view_models/track_metadata_grid.rs` for the `columns()` precedent

Goal:
- Add a card summary type, a width class with a column count, and a panel mode
  to `ShowPageVm`. View model only.

Constraints:
- GPUI-free. Every card carries the same field set, so the grid keeps one row
  height. `primary` and `secondary` are always present, empty when unused.
- The view model owns the column count. No breakpoint reaches the shell.
- The panel mode holds the cuelist or one card, never both.
- Keep the four section fields. Do not move detail content in this task.

Do not touch:
- `src/ui/**`, `src/app/**`, `src/broadcast/**`, `src/runtime/**`

Acceptance criteria:
- Card field set is uniform and tested in every section state.
- Absent sections add no card, and card order matches the enum order.
- Width classes return 1, 2, and 3 columns.

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
