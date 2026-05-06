# One Owner Per Surface Task 004: Feature-Readiness Gate

## Goal

Decide whether richer playlist/playback UI work can proceed, based on the
structural state of the surfaces that will carry those features.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/one-owner-per-surface-plan.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `docs/reviews/one-owner-per-surface-review-checklist.md`
- `tests/architecture_tests.rs`
- Relevant task review files from Tasks 001-003

## Files Likely to Change

- `docs/reviews/one-owner-per-surface-review-checklist.md`
- `docs/plans/one-owner-per-surface-plan.md` if sequencing needs correction
- `docs/adr/0033-hig-ui-architecture-governance.md` if enforcing-test names
  are stale

## Do Not Touch

- Runtime UI code, unless the review discovers a stale test-name doc mismatch
  that must be corrected with the test change already landed.
- Backend/API/schema/service code.

## Constraints

- This is a gate, not a feature task.
- "Proceed" requires evidence, not confidence.
- A visible symptom with no structural owner blocks feature work on that
  surface.
- Do not accept a non-zero architecture baseline unless a separate task owns
  its retirement.

## Implementation Steps

1. Run the verification commands and record results.
2. Check ADR 0033's enforcing-test list against `tests/architecture_tests.rs`.
3. Confirm Tasks 001-003 landed or explicitly list any remaining blockers.
4. Confirm visual smoke covers Library, Discovery recents, release detail,
   track detail, playlist popover, and now-playing/action chrome.
5. Write the final gate result in the review checklist: `Proceed`, `Proceed
   with listed constraints`, or `Do not proceed`.

## Acceptance Criteria

- The review checklist gives a clear feature-readiness result.
- Any blocker is tied to a structural rule from the plan, not a vague visual
  preference.
- ADR 0033 and architecture-test names are in sync.
- The repository is green on required checks.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/one-owner-per-surface-plan.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `docs/reviews/one-owner-per-surface-review-checklist.md`
- `tests/architecture_tests.rs`

Goal:
- Produce the readiness decision for richer playlist/playback feature work.

Constraints:
- No runtime feature implementation.
- Gate by evidence: tests, visual smoke, structural owners, and review notes.
- Any "Proceed" result must name the surfaces that are safe and their owners.

Do not touch:
- Runtime UI code unless only updating stale docs would be misleading.
- Backend/API/schema/service code.

Acceptance criteria:
- Review checklist has a clear gate result.
- Remaining blockers are concrete and structural.
- Required checks are green or failures are documented with next tasks.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If the readiness review identifies runtime regressions, stop and create
  bounded implementation packets rather than fixing them inside this gate.
