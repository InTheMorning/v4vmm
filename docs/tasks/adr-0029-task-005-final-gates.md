# ADR 0029 Task 005: Final Gates

## Status

Planned - 2026-05-01. Depends on Tasks 003-004.

## Goal

Close ADR 0029 by verifying the explicit artist-source path and documenting
remaining deferred work.

## Read

- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/tasks/adr-0029-task-003-musicindex-artist-ingest.md`
- `docs/tasks/adr-0029-task-004-local-artist-source-hydration.md`
- `docs/plans/deferred-architecture-work-index.md`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/plans/deferred-architecture-work-index.md`
- `docs/reviews/adr-0029-task-005-review.md`

## Do Not Touch

- Do not add runtime features.
- Do not start the future track-to-artist binding ADR.
- Do not introduce person identity persistence.

## Acceptance Criteria

- [ ] ADR 0029 status reflects completed runtime tasks.
- [ ] Deferred work index names only true remaining work.
- [ ] Architecture tests remain green.
- [ ] Full verification gate passes.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/tasks/adr-0029-task-005-final-gates.md`
- `docs/plans/deferred-architecture-work-index.md`

Goal:
- Finalize ADR 0029 documentation and gates.

Constraints:
- Documentation and verification only.
- Preserve deferred track-binding and person-identity work.

Do not touch:
- Runtime Rust files
- schema migrations

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Any final gate failure requires runtime code changes.
