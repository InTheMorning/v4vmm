# ADR 0047 Task 003: Description Collapse VM

Status: Proposed - 2026-05-14.

## Goal

Introduce `DescriptionState` enum with auto-collapse threshold (>5
rendered lines) and a projector that consumes a rendered-line-count
estimate. GPUI-free contract; downstream phases consume it.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `src/view_models/library.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/library.rs` or
  `src/view_models/entity_detail.rs` (host inspector description
  state — pick the one that already owns description text)
- `tests/architecture_tests.rs`

## Do Not Touch

- Any `src/ui/*` rendering
- Backend, playback, db

## Constraints

- GPUI-free; `M-CANONICAL-DOCS` on public types.
- `DescriptionState` enum:
  - `AutoCollapsed` — body exceeds threshold, never toggled
  - `AutoExpanded` — body ≤ threshold, never toggled
  - `UserCollapsed` — user collapsed
  - `UserExpanded` — user expanded
- `DescriptionState::project(line_count: usize)` returns the auto
  variant. User toggles transition to the User variants and stay
  sticky.
- Threshold constant: `DESCRIPTION_AUTO_COLLAPSE_LINES: usize = 5`.
- `is_visible(&self) -> bool` returns whether body should render
  expanded.

## Implementation Steps

1. Define `DescriptionState` and `DESCRIPTION_AUTO_COLLAPSE_LINES`.
2. Add `DescriptionState::project(line_count)` projector.
3. Add transition helpers: `toggle(self) -> Self`.
4. Add `is_visible(&self) -> bool` accessor.
5. Add `description_state: DescriptionState` field to feed and track
   inspector VMs.
6. Unit tests: auto-collapse threshold boundary (5 lines expanded, 6
   lines collapsed); toggle from auto → user variants; user variants
   are sticky.

## Acceptance Criteria

- [ ] `DescriptionState` enum exists with the four variants and the
  threshold constant.
- [ ] Inspector VMs carry `description_state`.
- [ ] Unit tests cover threshold boundary and toggle transitions.
- [ ] No UI module changed.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test description
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `src/view_models/library.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

Goal:
- Add `DescriptionState` enum with auto-collapse threshold (5 lines)
  and project from a rendered line-count estimate. Hook the field
  into feed and track inspector VMs.

Constraints:
- GPUI-free; documented.
- Auto and user variants both modeled.

Do not touch:
- `src/ui/*`
- Backend, playback, db

Acceptance criteria:
- Enum, threshold, projector, and inspector field exist.
- Unit tests cover boundary and toggle.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test description`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Inspector VM cannot host the field without a broader description
  refactor.
