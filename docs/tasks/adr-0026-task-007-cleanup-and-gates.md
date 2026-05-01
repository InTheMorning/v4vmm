# ADR 0026 Task 007: Cleanup and Gates

## Status

Implemented.

## Goal

Finalize ADR 0026 by removing stale plan language, tightening projection
architecture tests, and marking the ADR implemented after the verification
gates pass.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `tests/architecture_tests.rs`
- `src/search.rs`

## Files Changed

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-007-cleanup-and-gates.md`
- `docs/reviews/adr-0026-task-007-review.md`
- `tests/architecture_tests.rs`

## Do Not Touch

- Do not change runtime behavior.
- Do not migrate unrelated screen handlers.
- Do not expand ADR 0026 into ADR 0024 service/query work.

## Constraints

- Keep architecture tests source-scan based, matching existing ADR 0023/0025
  enforcement.
- Mark ADR 0026 implemented only after the green criteria are represented in
  code and tests.
- Keep any deferred work explicit rather than implying all GPUI thinning is
  complete.

## Implementation Summary

- Marked ADR 0026 implemented.
- Updated the phase plan to describe current implemented state.
- Replaced stale "task file to create" language with concrete task links.
- Added an architecture test preventing the Discover contributor panel from
  returning to API-shaped contributor state or the deleted screen-local
  `ContributorVm`.

## Acceptance Criteria

- [x] ADR 0026 status reflects implementation.
- [x] Phase plan reflects current code and all task files.
- [x] Architecture tests protect the contributor projection boundary.
- [x] Full verification gates pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `tests/architecture_tests.rs`
- `src/search.rs`

Goal:
- Finalize ADR 0026 docs and architecture gates.

Constraints:
- Do not change runtime behavior.
- Use source-scan architecture tests matching the existing pattern.
- Do not claim ADR 0024 query/service work is done.

Do not touch:
- Library/Discover rendering behavior
- service, DB, download, playlist, playback, or MusicBrainz logic

Acceptance criteria:
- ADR status is implemented.
- Phase plan no longer has stale task-creation wording.
- Architecture tests prevent API-shaped contributor panel regression.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Final gates uncover a behavior failure that requires changing runtime code.
- A green criterion is not actually implemented.
