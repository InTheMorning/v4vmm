# ADR 0024 Task 003: Phase 2 Checkpoint

## Status

Planned.

## Task Goal

Review the application-layer boundary after the playlist vertical slice before
migrating subscription/download, metadata/feed update, or playback workflows.

## Files To Inspect

- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `docs/tasks/adr-0024-task-001-application-skeleton.md`
- `docs/tasks/adr-0024-task-002-playlist-vertical-slice.md`
- `docs/reviews/adr-0024-review-checklist.md`
- `src/application/**`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `docs/adr/0024-command-query-event-application-layer.md` if revision is needed
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `docs/reviews/adr-0024-review-checklist.md`
- `docs/tasks/adr-0024-task-004-subscription-download-slice.md`
- `docs/tasks/adr-0024-task-005-metadata-feed-update-slice.md`
- `docs/tasks/adr-0024-task-006-playback-slice.md`

## Do Not Touch

- Rust implementation files, unless a documentation path or command name is
  stale.

## Constraints

- Be willing to revise ADR 0024 before widening the blast radius.
- Do not start Phase 3 implementation in this task.
- Keep documentation under `docs/`.

## Implementation Steps

1. Review the playlist implementation against ADR 0024.
2. Check whether `CommandContext`, `ApplicationServices`, `ApplicationEventBus`,
   and `ApplicationQueryService` were sufficient.
3. Check whether app-scoped event broadcast updated all relevant views.
4. Check whether source-scan architecture gates were adequate.
5. Record pass/fail and required ADR/task adjustments.
6. Update docs if command names, ports, or phase sequencing need correction.

## Acceptance Criteria

- A checkpoint result is recorded.
- Any required ADR or task-packet changes are made before Phase 3.
- The review explicitly says whether Phase 3 can proceed.

## Test Commands

- `git diff --check`
- `cargo fmt -- --check`
- `cargo check`

## Expected Final Summary Format

1. files changed
2. tests run
3. checkpoint decision
4. required revisions
5. unresolved concerns

## Escalation Triggers

- Playlist migration violates ADR 0024 invariants.
- Events do not broadcast across views.
- `ApplicationQueryService` needs remote network behavior.
- Architecture tests are too brittle for Phase 3.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `docs/tasks/adr-0024-task-001-application-skeleton.md`
- `docs/tasks/adr-0024-task-002-playlist-vertical-slice.md`
- `docs/reviews/adr-0024-review-checklist.md`
- `src/application/**`
- `tests/architecture_tests.rs`

Goal:
- Review whether ADR 0024 should proceed unchanged after the playlist slice.

Constraints:
- Do not implement Phase 3.
- Keep docs organized under `docs/`.

Do not touch:
- Rust implementation files unless fixing stale references.

Acceptance criteria:
- Checkpoint decision is recorded.
- Required ADR/task revisions are made.
- Phase 3 proceed/no-proceed is explicit.

Test commands:
- `git diff --check`
- `cargo fmt -- --check`
- `cargo check`

At the end, report:
1. files changed
2. tests run
3. checkpoint decision
4. required revisions
5. unresolved concerns
