# Deferred Work Integration Task 001: Status Reconciliation

## Goal

Reconcile completed ADR/task/review documentation before starting the
next unimplemented ADR, and make every known deferred item point to an
owned follow-up artifact.

## Files to Inspect

- `docs/plans/deferred-architecture-work-index.md`
- `docs/architecture/architecture-current-snapshot.md`
- `docs/reviews/adr-0038-review-checklist.md`
- `docs/adr/0040-async-vm-runtime.md`
- `docs/adr/0041-windowed-paged-view-models.md`
- `docs/adr/0042-layer-consolidation.md`
- `docs/reviews/adr-0043-review-checklist.md`
- `docs/reviews/adr-0044-review-checklist.md`
- `src/ui/shells/library/playlist_detail.rs`

## Files Likely to Change

- `docs/plans/deferred-architecture-work-index.md`
- `docs/reviews/adr-0038-review-checklist.md`
- `docs/adr/0040-async-vm-runtime.md`
- `docs/adr/0041-windowed-paged-view-models.md`
- `docs/tasks/library-playlist-inline-rename-task-001.md`
- `docs/reviews/deferred-work-integration-review-checklist.md`
- `src/ui/shells/library/playlist_detail.rs`

## Do Not Touch

- Do not implement ADR 0043 or ADR 0044.
- Do not change database schema.
- Do not change runtime behavior except for replacing untracked TODO
  comments with task references.
- Do not close deferred architecture items without proof from code and
  review artifacts.

## Constraints

- This task is a readiness gate, not a feature task.
- Any user-visible unfinished behavior discovered during the sweep must
  either be fixed in a separate implementation commit or captured in a
  bounded task file.
- ADR 0043 and ADR 0044 must remain pending until their implementation
  tasks and review checklists are complete.
- Status updates must be concrete and date-stamped.

## Implementation Steps

1. Remove stale stub/in-progress labels from completed ADR 0038 review
   artifacts.
2. Reconcile ADR 0040 and ADR 0041 status text against the default-on
   async-runtime commits.
3. Keep ADR 0043 and ADR 0044 marked as proposed/not-started.
4. Add explicit deferred follow-up routing for remaining
   `GpuiCommandRunner` and no-default-feature cleanup.
5. Convert the playlist inline-rename TODO into a tracked task.
6. Record the readiness decision in
   `docs/reviews/deferred-work-integration-review-checklist.md`.

## Acceptance Criteria

- No completed ADR review checklist advertises stale stub/in-progress
  task labels.
- ADR 0040/0041 status text no longer contradicts the default-on
  async-runtime feature flip.
- All known deferred work is either in the deferred architecture index
  or in a bounded task file.
- The playlist rename button's inert behavior is tracked by a task
  before ADR 0043/0044 work begins.
- `cargo fmt -- --check` and `cargo check` are green.

## Test Commands

```bash
cargo fmt -- --check
cargo check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/deferred-architecture-work-index.md`
- `docs/reviews/adr-0038-review-checklist.md`
- `docs/adr/0040-async-vm-runtime.md`
- `docs/adr/0041-windowed-paged-view-models.md`
- `src/ui/shells/library/playlist_detail.rs`

Goal:
- Reconcile stale status documentation and convert untracked deferred
  work into explicit follow-up tasks before the next ADR starts.

Constraints:
- Add no ADR 0043 or ADR 0044 implementation.
- Do not change database schema.
- Do not close deferred work without evidence.
- Keep changes small and status-focused.

Do not touch:
- Runtime behavior beyond replacing TODO comments with task references
- Unrelated UI surfaces
- Archived docs unless a contradiction points to active docs

Acceptance criteria:
- Status docs no longer contradict shipped async-runtime/default-on
  work.
- Playlist inline rename is tracked in a task file.
- Required checks are green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Evidence shows an ADR marked complete has unimplemented runtime
  behavior that is not already covered by a task.
- Fixing a stale TODO requires introducing new shared UI primitives or
  modal architecture.
