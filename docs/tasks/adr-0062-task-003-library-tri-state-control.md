# ADR 0062 Task 003: Library Tri-State Control

Status: Ready - 2026-09-07. Do after task 002.

## Goal

Replace the segmented filter control with one control carrying three states:
require in-library, ignore, exclude. The underlying model does not change.

## Files To Inspect

- `docs/adr/0062-music-content-surface.md`, the `Keep The Segmented Control`
  alternative
- `src/view_models/workspace/chrome.rs`, for `ContentFilter` and
  `FilterChipStripDisplay`
- `src/ui/composites/filter_chip_strip.rs`
- `src/app.rs`, for the filter wiring and the width class
- `tests/architecture_tests.rs` guards
  `adr_0047_phase_d_filter_chip_strip_renders_through_frame_shell` and
  `adr_0047_task_010_content_list_filter_chips_are_frame_local`

## Files Likely To Change

- `src/view_models/workspace/chrome.rs`
- `src/ui/composites/library_filter_control.rs` (new)
- `src/ui/composites/filter_chip_strip.rs` (removed or reduced)
- `src/ui/composites/mod.rs`
- `src/app.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- The `ContentFilter` enum and its three values. The model is correct.
- `matches_filter` and `visible_rows`. Filter semantics do not change.
- The narrow-width behavior contract from ADR 0046 task 008, unless the new
  control makes it unnecessary. If it does, say so and remove it deliberately.

## Constraints

- **This replaces a working, guarded control.** The current implementation is
  `SegmentedControl::new(selected).filter_style()` and two ADR 0047 guards
  require it. Both guards change with this task. Do not leave a guard asserting
  a control that no longer exists.
- The three states map exactly onto the existing enum:

  | State | `ContentFilter` | Shows |
  |---|---|---|
  | Highlighted | `Library` | Library rows only |
  | Off | `All` | Library and index |
  | Struck through | `Index` | Index rows only |

- **The state is never carried by visual treatment alone.** The control shows
  an explicit text label for its current state. ADR 0062 requires this.
- The struck-through state announces `Not in library` to assistive technology.
- The three states are reachable by keyboard in a predictable order, and the
  order is documented in the view model.
- The control is built to sit beside other filter axes later. Do not hard-code
  it as the only filter in the row.
- No color-only signalling.

## Implementation Steps

1. Add a tri-state display contract in `chrome.rs` carrying the current state,
   its text label, its accessibility label, and the next state on activation.
2. Add the composite. It renders the label, the state treatment, and one
   activation callback.
3. Wire activation to cycle the state and dispatch the existing
   `set_content_filter` path. The command layer does not change.
4. Remove the chip strip composite, or reduce it to whatever still has a
   caller. Report which.
5. Rewrite the two ADR 0047 guards to assert the new control. Keep every
   requirement that still holds, such as frame-local ownership and view-model
   projection. Drop only what named the segmented control.
6. Add guards, situational, citing ADR 0062:
   - the control exposes a text label for every state
   - the struck-through state carries a `Not in library` accessibility label
   - no state is distinguished by color alone
7. Add view-model tests: each state's label, the activation cycle order, and the
   mapping onto all three `ContentFilter` values.

## Acceptance Criteria

Mechanical:

- The display exposes three states mapping onto `ContentFilter::Library`,
  `All`, and `Index`.
- Every state carries a text label and an accessibility label.
- The excluding state's accessibility label reads `Not in library`.
- Activation cycles in the documented order and dispatches the existing filter
  command.
- The two ADR 0047 guards assert the new control and no guard names
  `SegmentedControl::new(selected).filter_style()`.
- No renderer decides a state.

Visual proof, operator only:

- The current state is readable without knowing the cycle.
- The struck-through state reads as exclusion rather than as disabled.
- The control is usable by keyboard alone.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test workspace --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo run` for the visual check

## Expected Final Report Format

1. Files changed
2. Tests run
3. Which ADR 0047 guard requirements were kept and which were dropped
4. Whether the chip strip composite still has a caller
5. Screenshots captured, or the visual gate reported open
6. Deviations from task
7. Unresolved concerns

## Escalation Triggers

- The struck-through treatment is indistinguishable from a disabled control in
  the current token set. Report it. Disabled and excluded must not look alike.
- Another surface uses the chip strip composite. Report the caller before
  removing it.
- The narrow-width pulldown contract cannot be satisfied by a single control.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0062-music-content-surface.md`
- `src/view_models/workspace/chrome.rs`, `src/ui/composites/filter_chip_strip.rs`
- the two ADR 0047 filter guards in `tests/architecture_tests.rs`

Goal:
- Replace the segmented filter control with one tri-state control: require
  in-library, ignore, exclude.

Constraints:
- The `ContentFilter` model does not change. Only the control and its
  accessibility contract.
- Every state carries a text label. Never signal state by treatment alone.
- The excluding state announces `Not in library`.
- Keyboard reachable in a documented order.
- Rewrite the two ADR 0047 guards. Leave no guard asserting the old control.

Do not touch:
- `ContentFilter`, `matches_filter`, `visible_rows`

Acceptance criteria are split into mechanical and visual. Report the visual
gate as open if no window can be opened.

Test commands:
- `cargo fmt -- --check`
- `cargo test workspace --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. which ADR 0047 guard requirements were kept and which dropped
4. whether the chip strip still has a caller
5. screenshots captured or the visual gate reported open
6. deviations from task
7. unresolved concerns
