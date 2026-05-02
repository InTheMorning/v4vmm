# One Owner Per Surface Task 003: Composite Display-Contract Audit

## Goal

Audit shared UI composites so policy-bearing labels use a view-model or
co-located display contract instead of loose `String` / `&str` parameters.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/one-owner-per-surface-plan.md`
- `src/ui/composites/*.rs`
- `src/ui/primitives/*.rs`
- `src/view_models/`
- `tests/architecture_tests.rs`

## Files Likely to Change

- Selected files under `src/ui/composites/`
- Selected files under `src/view_models/`
- Call sites in `src/library.rs`, `src/search.rs`, or `src/ui_*.rs` only when
  required by the composite contract migration
- `tests/architecture_tests.rs`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/reviews/one-owner-per-surface-review-checklist.md`

## Do Not Touch

- Backend/service/API modules.
- Broad visual styling or token definitions.
- Composite behavior that is pure passthrough and does not carry display
  policy.

## Constraints

- Do not ban every loose string. Ban loose strings where the label carries
  fallback, truncation, casing, action-role, or hierarchy policy.
- Keep shared UI backend-free and screen-free.
- Public composite doc comments should name the display contract type and
  where policy is owned.
- Any architecture test allowlist must be narrow and explained.

## Implementation Steps

1. Grep composite public constructors and builders for `String`, `&str`, and
   `impl Into<SharedString>` label parameters.
2. Classify each as pure passthrough or policy-bearing.
3. For policy-bearing labels, introduce or reuse a VM/co-located display
   struct and migrate the call sites.
4. Add module-level or type-level comments naming the display contract owner.
5. Add an architecture test or allowlist that fails when a new policy-bearing
   loose string is added without review.
6. Update ADR 0033 with the new test name if a test is added.

## Acceptance Criteria

- Policy-bearing composite labels have one display contract owner.
- Shared UI components remain backend-free and screen-free.
- New or updated architecture tests prevent silent drift.
- The review checklist records any intentionally allowed loose strings.

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
- `docs/tasks/one-owner-per-surface-task-003-composite-display-contract-audit.md`
- `src/ui/composites/*.rs`
- `tests/architecture_tests.rs`

Goal:
- Ensure shared composites do not accept loose strings for policy-bearing
  labels without a named display contract owner.

Constraints:
- Pure passthrough strings can remain with a documented reason.
- Policy-bearing labels move to VM or co-located display structs.
- Shared UI must not import backend or screen modules.

Do not touch:
- Backend/API/schema/ingest/playback modules.
- Theme palette definitions.
- Unrelated layout.

Acceptance criteria:
- Policy-bearing labels have display contracts.
- Tests or documented allowlists guard the boundary.
- ADR 0033 is updated if test names change.

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

- If a composite's current signature is used broadly enough that migration is
  no longer bounded, stop and split by composite family.
- If the audit discovers a missing VM layer for a major surface, create a new
  task packet before implementation.
