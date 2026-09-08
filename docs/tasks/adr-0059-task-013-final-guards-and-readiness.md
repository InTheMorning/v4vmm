# ADR 0059 Task 013: Final Guards And Readiness Gate

Status: Ready - 2026-09-08. Gate task. Do last.

## Goal

Close ADR 0059. Prove every invariant with a guard, record the visual evidence,
and reconcile the document statuses.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
- `docs/plans/curator-workflow-ui-design-brief.md`
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
   - **paste the `podcast:liveValue` tag into the RSS feed of the show.**
     Listener apps find a live event only through that tag. State that the
     chain reports success without it and no listener receives anything
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

## Inherited Visual Gates

These gates opened in earlier packets and stay open. This packet inherits every
one of them. **Do not report this packet complete while any line below is open,
and never mark one met because the mechanical suite is green.**

Each needs real hardware that an agent session does not have. An operator clears
them, one line at a time, and records the result in the owning packet first.

| Packet | Open gate | What it needs |
|---|---|---|
| 010 remote hosts | The `Source` host and reachability row reads correctly | A configured SSH host that is unreachable |
| 015 stream encoder | The connected, disconnected, and not-installed states read correctly | An installed `butt` binary |
| 012 readiness report | The readiness count is legible in place, and the action reaches the filtered list | A working display only |
| 0063 003 detail panel | The panel, the card grid, and the transport all work together | A working display only |
| 002, 003, 004 | Met on 2026-09-08 | Nothing. Recorded here so the set is complete |

The open checks and the method to reach each state live in
`docs/pending-human-checks.md`. That file holds what is open today, not a
history. When a gate closes, update the owning packet `Status:` line, update the
row in `docs/plans/broadcast-chain-delivery-order.md`, and remove the section
from that file, all in the same change.

If an operator cannot clear a gate before this packet ships, that is an
acceptable outcome. Report the gate as open in the readiness summary, name the
hardware it waits on, and say so in the `Status:` line of this packet. A chain
declared ready on an unperformed visual check is not.

## Acceptance Criteria

- Mechanical: the readiness summary lists every inherited visual gate and its
  state, and the list matches the owning packet `Status:` lines.
- Every ADR 0059 invariant has a guard or a recorded reason.
- The review document names each artifact it checked.
- The runbook covers token backup and relay restart recovery.
- The report carries operator steps for every gate still open, and the
  readiness summary names the hardware each one waits on.
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
4. Operator visual check, with numbered steps for every gate in the inherited
   table that is still open, and the hardware each one needs
5. Open gates, one line for each, naming what it waits on
6. Merge recommendation

Do not run the app and do not attempt a headless display. Write the operator
steps instead, as AGENTS.md requires.

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
- `docs/plans/curator-workflow-ui-design-brief.md`
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
