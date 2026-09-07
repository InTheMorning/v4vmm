# ADR 0059 Task 013: Final Guards And Readiness Gate

## Goal

Close ADR 0059. Prove every invariant with a guard, record the visual evidence,
and reconcile the document statuses.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
- `docs/architecture/broadcast-chain.md`
- All task packets `adr-0059-task-001` through `adr-0059-task-012`
- `tests/architecture_tests.rs`
- `docs/adr/0057-adr-status-vocabulary-and-amendment-policy.md`

## Files Likely To Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0059-implementation-review.md` (new)
- `docs/adr/0059-broadcast-control-surface.md` (status only)
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md` (status only)
- `docs/runbooks/broadcast-operations.md` (new)

## Do Not Touch

- Any behavior. This task adds guards, documents, and status lines only. If a
  guard fails, open a fix task instead of changing behavior here.

## Constraints

- Every invariant in ADR 0059 needs a guard or a recorded reason for its
  absence.
- `Implemented` needs a named artifact, as ADR 0057 requires. The review
  document is that artifact.
- A status may not claim `Implemented` while a gate is open. An operator visual
  check that has not happened is an open gate.

## Implementation Steps

1. Write one architecture guard for each ADR 0059 invariant:
   - the app sends no metadata to the relay
   - no token text in the database, in a log, or in `Debug` output
   - `src/broadcast/**` is GPUI-free and builds no `reqwest` client
   - `systemctl`, `journalctl`, and `ssh` are called only from
     `src/broadcast/`
   - source kind names appear only in source adapters
   - the drop file is written only by the producer module
   - the `Broadcast` frame and the `QueueNowPlaying` frame stay separate
2. Add a dark-mode parity check and an accessibility label check for the new
   shell, in the shape of the ADR 0038 checks.
3. Write `docs/runbooks/broadcast-operations.md`:
   - create an event and store the token
   - back up the token file, because nobody can replace it
   - start and stop the services for a local host and a remote host
   - read the logs when a unit fails
   - what to do when the relay restarts and every event dies
4. Write `docs/reviews/adr-0059-implementation-review.md` with the reviewed
   artifacts, pass or fail for each invariant, missing tests, architectural
   drift, and a merge recommendation.
5. Run the visual smoke list and attach the screenshots: live event, dead event,
   publisher not installed, failed unit with a reason, open log panel, remote
   host not reachable, and a non-zero readiness count.
6. Set the ADR and the plan to `Implemented` with the review as the named
   artifact, or record the open gate on the second status line.
7. Add the runbook and the review to `docs/README.md`.
8. Answer the three open questions in the phase plan, or move them to
   `docs/plans/deferred-architecture-work-index.md`.

## Acceptance Criteria

- Every ADR 0059 invariant has a guard or a recorded reason.
- The review document names each artifact it checked.
- The runbook covers token backup and relay restart recovery.
- The seven screenshots exist.
- The ADR and the plan carry a status that matches the evidence.
- `docs/README.md` links the runbook and the review.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `python3 /home/citizen/.claude/plugins/marketplaces/local/plugins/ste100/scripts/ste_lint.py docs/runbooks/broadcast-operations.md docs/reviews/adr-0059-implementation-review.md`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Guards added, one line for each invariant
4. Screenshots captured
5. Open gates, if any
6. Merge recommendation

## Escalation Triggers

- An invariant cannot be guarded by a source-text test. Record the reason and
  the manual check that replaces it.
- A guard fails against shipped code. Open a fix task. Do not change behavior in
  this task.
- The relay gains long-lived events during this work. That changes the runbook
  recovery steps and needs a plan amendment.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture. Add no behavior.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
- `docs/adr/0057-adr-status-vocabulary-and-amendment-policy.md`
- `tests/architecture_tests.rs`

Goal:
- One guard for each ADR 0059 invariant, a runbook, a review document, and
  correct status lines.

Constraints:
- Guards, documents, and status lines only.
- `Implemented` needs the review document as its named artifact.
- An open gate keeps the status at `Accepted`.

Do not touch:
- any behavior. If a guard fails, report it and open a fix task.

Acceptance criteria:
- Every invariant guarded or explained.
- Runbook covers token backup and relay restart recovery.
- Seven screenshots captured.
- Status lines match the evidence.

Test commands:
- `cargo fmt -- --check`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. guards added
4. screenshots captured
5. open gates
6. merge recommendation
