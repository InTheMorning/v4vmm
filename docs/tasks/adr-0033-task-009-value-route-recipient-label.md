# ADR 0033 Task 009: Value Route Recipient Label Fallback

## Goal

Audit the remaining screen-local empty-name fallbacks called out by the
post-ADR 0033 plan and hoist the repeated value-route recipient display rule
out of Library and Discover screen code.

## Files to Inspect

- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/metadata.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/metadata.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend metadata parsing, download services, schema, and API clients.
- Value-route storage shape.
- Unrelated metadata row rendering.

## Constraints

- The fallback rule must stay GPUI-free.
- Screens may render expanded JSON rows, but must not reconstruct the
  recipient display fallback inline.
- Preserve the stricter Library behavior: trim whitespace and use `"Unknown"`
  for missing or blank recipient names.

## Implementation Steps

1. Add a shared metadata view-model helper for value-route recipient labels.
2. Replace Library and Discover inline `recipient_name` fallback logic.
3. Add unit coverage for present, blank, and missing names.
4. Add an architecture guard so screens cannot reintroduce direct
   `recipient_name` display fallback projection.

## Acceptance Criteria

- No screen file calls `.get("recipient_name")` to build the display label.
- Blank recipient names render as `"Unknown"` in both Library and Discover.
- Architecture tests include a regression guard.
- Verification is green.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/metadata.rs`
- `tests/architecture_tests.rs`

Goal:
- Hoist repeated value-route recipient display fallback logic from screens
  into a GPUI-free metadata view-model helper.

Constraints:
- Keep screen code as event/render wiring only.
- Preserve `"Unknown"` for missing or blank names.
- Do not change backend value-route data.

Do not touch:
- Backend services.
- Database migrations.
- Unrelated metadata presentation.

Acceptance criteria:
- Both screens call the shared helper.
- Architecture tests fail if screens reintroduce direct `recipient_name`
  label projection.
- Unit tests cover the helper.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The helper would need GPUI types.
- The fallback rule differs by screen after closer inspection.
- The architecture guard blocks legitimate non-display JSON handling.
