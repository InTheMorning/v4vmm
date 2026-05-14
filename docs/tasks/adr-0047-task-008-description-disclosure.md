# ADR 0047 Task 008: Description Disclosure

Status: Proposed - 2026-05-14.

## Goal

Add a disclosure control to the Description section on feed and
track inspectors. Auto-collapse when rendered body exceeds the
5-line threshold; default expanded otherwise. User toggle overrides
the auto-default for the inspector instance.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-003-description-collapse-vm.md`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/ui/shells/library/feed_detail*.rs` (if separate)
- `src/ui/primitives/disclosure_indicator.rs`
- `src/view_models/library.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/ui/shells/library/track_detail_metadata.rs`
- `src/ui/shells/library/feed_detail*.rs`
- `src/view_models/library.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend, db, playback
- Contributors / Value-routes disclosure logic (already implemented;
  Description matches its pattern)

## Constraints

- Reuse existing disclosure-indicator primitive.
- `description_state: DescriptionState` from task 003 is the source
  of truth for collapse/expand visibility.
- Initial render: project from a rendered-line-count estimate.
  Estimate may be a conservative character-based proxy if true line
  count is not available pre-render.
- Toggle handler transitions `Auto*` → `User*` per task 003 helpers.
- No raw glyph/color/spacing literals; reuse tokens.

## Implementation Steps

1. Add a disclosure-indicator render to feed and track Description
   section headers, mirroring the existing Contributors / Value-
   routes pattern.
2. Toggle handler dispatches a VM mutator that calls
   `DescriptionState::toggle`.
3. Project `description_state` from a rendered-line-count estimate
   at inspector construction (or lazy on first render).
4. Body render guard: collapsed → render the disclosure header only;
   expanded → render header + body.
5. Architecture guard: assert Description headers in feed and track
   inspectors consume `DescriptionState::is_visible`.

## Acceptance Criteria

- [ ] Description gains a disclosure indicator on feed and track
  inspectors.
- [ ] Default collapsed when body > 5 lines, expanded otherwise.
- [ ] Toggle transitions to user variants and is sticky for the
  inspector instance.
- [ ] No raw glyph/color/spacing literal added.
- [ ] Architecture guard locks the contract.

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
- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-003-description-collapse-vm.md`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/ui/shells/library/feed_detail*.rs`
- `src/ui/primitives/disclosure_indicator.rs`
- `src/view_models/library.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

Goal:
- Add disclosure indicator to Description on feed and track
  inspectors. Auto-collapse at >5 lines; user toggle overrides.

Constraints:
- Reuse existing primitive and tokens.
- `DescriptionState` from task 003 drives visibility.

Do not touch:
- Backend, db, playback
- Existing Contributors / Value-routes disclosure logic

Acceptance criteria:
- Description disclosure renders and toggles correctly.
- Auto vs user variants behave per task 003 contract.
- Architecture guard records consumption.

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

- Pre-render line-count estimate cannot be computed without measuring
  text layout (escalate; conservative proxy is acceptable).
