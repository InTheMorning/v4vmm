# ADR 0045 Task 004: Guards and Readiness

## Goal

Add final architecture guards, run the full verification gate, and update the
ADR 0045 review checklist.

## Files to Inspect

- `docs/adr/0045-track-artist-binding.md`
- `docs/plans/adr-0045-track-artist-binding-phase-plan.md`
- `docs/tasks/adr-0045-task-001-track-artist-binding-schema.md`
- `docs/tasks/adr-0045-task-002-musicindex-binding-ingest.md`
- `docs/tasks/adr-0045-task-003-library-artist-hydration.md`
- `docs/reviews/adr-0045-review-checklist.md`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0045-review-checklist.md`
- Possibly minor fixes in files changed by Tasks 001-003

## Do Not Touch

- Do not add new feature behavior beyond readiness fixes.
- Do not change audio tag write paths.
- Do not refactor unrelated UI.

## Constraints

- Guards must map directly to ADR 0045 invariants.
- Review checklist may mark `Proceed` only after full checks pass.

## Implementation Steps

1. Add guards blocking screen-side name matching or binding inference.
2. Add guards ensuring track-to-artist bindings route through DB/read-model
   helpers.
3. Run the full required gate.
4. Update the review checklist with evidence and merge recommendation.

## Acceptance Criteria

- Architecture tests prevent renderer/screen binding inference.
- Full checks are green.
- Review checklist records a clear pass/fail decision.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0045-track-artist-binding.md`
- `docs/reviews/adr-0045-review-checklist.md`
- `tests/architecture_tests.rs`

Goal:
- Add architecture guards and complete readiness evidence for ADR 0045.

Constraints:
- No new feature behavior.
- Guards must map to ADR invariants.

Do not touch:
- Audio tag write paths
- Unrelated UI

Acceptance criteria:
- Guards are in place and full checks are green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A guard would require broad brittle string baselines.
- Final verification reveals multi-subject display ambiguity.
