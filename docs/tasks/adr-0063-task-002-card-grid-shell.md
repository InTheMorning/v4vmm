# ADR 0063 Task 002: Card Grid Shell

Status: Implemented - 2026-09-08. Mechanical and operator visual acceptance
met. Task 003 needs the grid that this task builds.

## Goal

Replace the vertical stack of full-width sections with a card grid. The grid
reads the column count from the view model. The queue keeps its current
container until task 003 moves it.

## Files To Inspect

- `docs/adr/0063-show-dashboard-layout.md`
- `docs/tasks/adr-0063-task-001-card-contract-and-width-class.md`
- `src/ui/shells/show.rs`, for `render_show` and the four section renderers
- `src/ui/composites/track_metadata_grid.rs`, for `.grid().grid_cols(...)`
- `src/ui/tokens.rs`, for spacing and size tokens
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/composites/show_card.rs` (new)
- `src/ui/composites/mod.rs`
- `src/ui/shells/show.rs`
- `src/view_models/show.rs`, for the width mapping only
- `src/app/show.rs`, for the window-width observation only
- `tests/architecture_tests.rs`

## Do Not Touch

- The card contract types from task 001. Read them, do not change their shape.
  The width mapping in step 4 is the one addition this task makes to
  `src/view_models/show.rs`.
- `src/broadcast/**`, `src/runtime/**`
- `render_queue_now_playing` and its slots
- The transport controls
- `src/app/**` beyond the window-width observation in step 4

## Constraints

- The card composite renders `ShowCardDisplay` and nothing else. It builds no
  label and maps no state to text. Task 001 owns every string.
- **Each card has the same height.** Set it from a token, not from content. A
  card with one summary line and a card with two are the same height.
- The grid column count comes from `ShowPageVm`. The shell reads
  `width_class.columns()`. **No breakpoint arithmetic in the shell.**
- **The width class needs an input, and task 001 did not give it one.**
  `ShowPageVm::width_class` is the default in every case today, so the grid is
  always three columns. The app layer observes the window width and passes it to
  the view model. The view model maps the width to a class. Neither the shell nor
  the composite chooses a class.
- Task 001 added `..` to the `ShowPageVm` destructure in `render_show`, to keep
  the shell compiling. **Restore the exhaustive destructure in this task.** The
  exhaustive form is what forces a later field to reach the renderer, and the
  card list is the field that must reach it now.
- The grid does not scroll. No `overflow_y_scrollbar` and no scroll container
  reaches the card region. ADR 0063 states this as an invariant.
- A card is selectable. Selection sends the card kind through a slot. This task
  adds the slot and the visual selected state. Task 003 makes the panel answer.
- Use existing primitives and tokens. Add no new color and no new spacing value.
- The four section renderers that this task replaces are deleted, not left
  unused. The detail they render moves in task 003, so copy what task 003 needs
  into a module comment before you delete a renderer.

## Implementation Steps

1. Add `src/ui/composites/show_card.rs` with a `ShowCard` composite that takes
   `ShowCardDisplay`, a selected flag, and a click handler.
2. Render the card as a title row with a state badge, then the two summary
   lines. Both lines always render, so the height never changes.
3. Map `ShowCardStateKind` to a semantic color. Never use color alone: the badge
   carries the state label text as well.
4. Add a width observation:
   - the app layer reads the window width and calls a view-model constructor
     with it
   - the view model maps the width to `ShowWidthClass`
   - pick the two breakpoints from the token scale, and state them in the view
     model, not in `src/ui/`
5. Restore the exhaustive `ShowPageVm` destructure in `render_show`, and remove
   the `..`.
6. Add the card grid to `render_show`:
   - a container with `.grid()` and `.grid_cols(vm.width_class.columns())`
   - one `ShowCard` for each entry of `vm.cards`
   - the grid sits under the show summary and above the queue container
7. Add `on_select_card` to `ShowSlots`, taking `ShowCardKind`.
8. Delete `render_source_section`, `render_publisher_section`,
   `render_event_section`, and `render_stream_section`, and the helper functions
   that only they reach.
9. Keep the queue container as it is. Task 003 moves it.
10. Add a guard: `src/ui/shells/show.rs` and `src/ui/composites/show_card.rs`
   hold no pixel breakpoint, no `grid_cols` literal, and no scroll container in
   the card region.
11. Add a guard: the card composite holds no state-to-text mapping, so the view
   model keeps every label.

## Acceptance Criteria

Mechanical, proved by a test:

- The shell reads the column count from the view model, and holds no breakpoint
  constant.
- A window width reaches the view model, and each width class is selected by a
  test at a width that belongs to it.
- The `ShowPageVm` destructure in `render_show` is exhaustive, with no `..`.
- The card region holds no scroll container.
- The card composite holds no state label string and no state branch that picks
  text.
- The four section renderers are gone, and no dead helper is left behind.
- The guard suite does not grow for a rule that a deleted renderer needed.

Visual, operator only:

- Every card is visible at once, with no scrolling, at the window size the
  operator uses.
- Cards fill the width. The middle of the window carries content.
- Making the window narrow reduces the column count, and the cards stay
  readable.
- Every card is the same height, in every state.
- A card state is readable without color, from its label.

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

- A card cannot hold its state badge and two lines at the fixed height that the
  tokens give. Report the token you need before you add a value.
- Deleting a section renderer would drop detail that task 003 needs and no
  module comment records it.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0063-show-dashboard-layout.md`
- `docs/tasks/adr-0063-task-001-card-contract-and-width-class.md`
- `src/ui/shells/show.rs`, `src/ui/composites/track_metadata_grid.rs`

Goal:
- Replace the four full-width section renderers with a card grid that reads the
  column count from the view model.

Constraints:
- The card composite renders the display contract. It builds no string.
- Every card is the same height, set from a token.
- No breakpoint arithmetic and no scroll container in the card region.
- The app layer observes the window width. The view model maps it to a class.
  The shell only reads `columns()`.
- Restore the exhaustive `ShowPageVm` destructure. Remove the `..`.
- Delete the four section renderers. Record what task 003 needs first.

Do not touch:
- `src/broadcast/**`, `src/runtime/**`, the queue renderer, the transport
- The card contract types from task 001, other than adding the width mapping

Acceptance criteria:
- Column count comes from the view model, and the shell holds no breakpoint.
- A window width reaches the view model, and a test picks each class.
- The `render_show` destructure is exhaustive.
- The card region holds no scroll container.
- No dead helper is left behind by the deleted renderers.

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
