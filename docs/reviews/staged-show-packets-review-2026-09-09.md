# Staged Show Packets Review

## Status

Changes required - 2026-09-09. The initial review covers the nine staged
documentation files. A follow-up below reviews the operator's unstaged packet
corrections against the same implementation. The index still contains the
versions covered by the initial findings.

This review follows the
[earlier task review](show-log-and-action-feedback-task-review.md).

The revisions explicitly scope the shared splitter extension, correlate log
reads, remove forced panel navigation, and specify horizontal log scrolling.
The initial review found three issues. The working-tree corrections address
their substantive behavior; one duplicated scope instruction remains.

## Working-Tree Follow-Up

Rechecked 2026-09-09 after the operator revised both packets:

- R1's freshness mechanism is corrected: the packet requires a separate batch
  start stamp and a test for a batch that crosses command completion. Its main
  scope permits the required change to the watch module.
- R2's indefinite display hold is addressed by fresh terminal-failure release
  and bounded nonconvergence. The packet adds corresponding mechanical checks.
- R3's contradictory close instruction is corrected in the log prompt, which
  now permits a second Logs press on the displayed service to close the pane.

### R4 - P2: Carry The Runtime Exception Into The Abbreviated Prompt

Location: `docs/tasks/show-action-feedback-task-001-command-state-and-result.md`,
lines 243-244 in the corrected working tree.

The abbreviated prompt still lists all of `src/runtime/**` under "Do not
touch". Its preceding freshness requirement cannot be implemented with the
existing snapshot, while the main packet now explicitly requires and permits
a batch-start stamp in `src/runtime/broadcast_service_watch.rs`.

Repeat that exact exception in the abbreviated prompt, including the stamp
and its wiring. Otherwise an implementer given only that prompt must either
violate its scope or leave the freshness requirement unmet.

The two packet corrections are unstaged. This follow-up does not close the
findings against the older versions still in the index. No source or task
packet was edited by this review.

## Initial Staged Review Findings

Locations and descriptions in this section refer to the staged versions,
before the operator's working-tree corrections.

### R1 - P2: The Existing Timestamp Does Not Prove Observation Freshness

Location: `docs/tasks/show-action-feedback-task-001-command-state-and-result.md`,
lines 104-107.

The instruction says a sample is fresh when its `at` is later than command
completion and prohibits runtime changes on that basis. The actual type is
`BroadcastServiceWatchSnapshot`. Its constructor stamps `at` after collecting
all service and encoder results (`src/runtime/broadcast_service_watch.rs`,
lines 141-147 and 305-323).

Publisher can be read before a command returns, with a slower Producer or
encoder read keeping the batch outstanding until afterward. That old Publisher
observation then passes the prescribed freshness comparison. If it agrees with
the requested state, it can release the transition despite the explicit
requirement to reject agreeing observations taken before completion.

Specify an observation-start boundary or an acknowledged refresh generation
that proves the read began after completion, and scope any required runtime
change explicitly. Test an early Publisher read followed by a delayed batch
completion; assigning an artificially old snapshot timestamp misses this race.

### R2 - P2: Agreement-Only Release Can Hide A Terminal Failure Indefinitely

Location: `docs/tasks/show-action-feedback-task-001-command-state-and-result.md`,
lines 108-118.

The transition may end only on a fresh observation that agrees with the
request, except when the command itself fails. Command success is only process
command success: `PublisherServiceCommand` and `StreamEncoderCommand` return
`()`, and encoder connection does not wait for an observed connection
(`src/app/show.rs`, lines 660-680 and 857-877;
`src/broadcast/encoder.rs`, lines 271-278).

A successful Start followed by a service crash before the next poll yields
fresh `Failed` observations without an `Active` observation. Similarly, a
successful control request followed by an unreachable encoder can yield only
`NotReachable`. Following the packet keeps the requested transition displayed
indefinitely instead of revealing the failure and restoring appropriate
recovery actions.

Define how fresh terminal failures and nonconvergence end the display hold,
separately from command errors, and specify the agreement rule for Reset as
well. Add tests for command success followed by fresh failure, unreachable
state, and persistent disagreement.

### R3 - P2: The Abbreviated Log Prompt Still Forbids The Required Toggle

Location: `docs/tasks/adr-0063-task-004-log-bottom-pane.md`, lines 58-62;
conflicting abbreviated instruction at lines 181-182.

The revised main constraints permit Close and a second press on the same
service's Logs action to close the pane. The lower-context prompt still says
the pane "closes only from its own control" and its acceptance criteria only
require Logs to open it. An implementer working from that prompt is directed
to omit the same-service toggle that the full packet requires.

Carry the complete cycle into the abbreviated prompt and its acceptance
criteria: Close or the same service closes; another service switches content;
card selection preserves the pane.

## Recommendation And Validation

Correct the remaining runtime-scope instruction before implementation, and
include the intended packet revisions in the eventual staged change. These
findings concern the proposed behavior; neither diff changes Rust code. No
additional architectural drift or optional improvements are asserted here.

Static inspection covered the staged diff, current ADR index, ADR 0057,
ADR 0063, delivery order, referenced review, service watch, command paths,
Show projection, encoder control, and shared split composite.

Staged whitespace: Green. Local Markdown links were checked separately.
No Cargo suite was run for this documentation-only review. The staging area
was left unchanged; this report and its documentation-index entry are unstaged.

## Operator Visual Check

No new UI was implemented, so this review creates no additional desktop check.
Existing visual gates remain open or accepted exactly as recorded. The packet
implementations must provide their own terminal setup, expected observations,
and cleanup before a person can accept their visual changes.
