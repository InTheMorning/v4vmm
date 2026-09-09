# Show Log And Action Feedback Task Review

## Status

Changes required - 2026-09-09. Neither packet is ready for implementation as
written. This reviews the proposed work, not an implementation of it.

## Artifacts Reviewed

- [ADR 0063 task 004](../tasks/adr-0063-task-004-log-bottom-pane.md)
- [Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md)
- [ADR 0063](../adr/0063-show-dashboard-layout.md)
- [HIG product polish backlog](../plans/hig-product-polish-backlog.md), A7-A9
- [Broadcast chain delivery order](../plans/broadcast-chain-delivery-order.md)
- Current Show projections, command callbacks, service watch, shared split
  composite, library feed-update renderer, and architecture guards

Line references below identify the files as reviewed on 2026-09-09.

## Required Fixes

### R1 - P2: Command Completion Does Not Prove Snapshot Freshness

Feedback task lines 58-61 and 91-96 clear the in-flight record when the command
returns. Both service commands currently return `()`, then request an
asynchronous watch refresh (`src/app/show.rs:366` and `:516`). They do not
return an observed final state or cancel a read already in progress.

A permitted ordering is:

1. The watch reads Publisher as `Active`, then continues reading Producer and
   the encoder.
2. Stop completes and its callback clears the in-flight record.
3. The earlier watch read publishes `Active`.
4. The requested refresh finally publishes `Inactive`.

Following the packet still allows the exact `Started` flash that A8 must fix.
An unrelated Show refresh can also reuse the cached pre-command snapshot after
the record clears.

Specify how a command hands presentation back to a fresh observation. Ending
command execution must not make an older observation authoritative. Include a
deterministic test for command success followed by a delayed pre-command read,
for both Start and Stop, and equivalent stream cases.

The existing snapshot has an `at` field, but it is stamped after the batch
finishes (`src/runtime/broadcast_service_watch.rs:141` and `:310`). Comparing
that value with command completion does not establish when Publisher was
actually read. Resolve the freshness contract before delegating; a timestamp
alone is not a specified fix.

### R2 - P2: Observation Agreement Must Not Release Command Ownership

Feedback task lines 86-96 use one record both for command execution and for
the requested transition. An agreeing watch sample clears that record even
when the command callback has not arrived. This conflicts with lines 69-70,
which require controls to remain unavailable while the command is in flight.

If agreement re-enables the controls, an operator can start an opposite command
for the same role. The older completion callback can then clear the newer
record. Keying by role prevents Publisher from clearing Producer, but does not
prevent one Publisher operation from clearing another. `present_command`
delivers each completion independently; it does not discard superseded results.

Separate the command's outstanding state from observation convergence. Keep
controls disabled until the matching command completes, even if the displayed
state has converged. Associate each completion with its operation and define
the agreement rule for `Reset`, which invokes `reset-failed`, as well as Start,
Stop, Connect, and Disconnect.

Test an agreeing sample before command completion, a delayed old completion
after a new operation, and independent simultaneous roles.

### R3 - P2: The Required Shared Resize Control Is Not Reusable As Written

Task 004 lines 44-45 and 75-76 require the workspace resize implementation.
Its owner is `src/ui/composites/split_pane.rs`, which is absent from both file
lists. That composite only supports side-by-side panes: `leading_width`,
`leading_min_width`, `flex_row`, and `cursor_col_resize` (`:24`, `:108`, and
`:119`). Workspace callbacks also forward only the mouse x coordinate
(`src/ui/shells/workspace.rs:480`).

The top/bottom split already meets the packet's escalation condition at lines
132-133. Decide and explicitly scope the shared composite's axis support now.
List its owner, the needed height tokens, and the drag-state owner. Preserve
existing side-by-side behavior and add a regression check for it. Specify
height clamping that reserves space for the grid and transport on resize.

### R4 - P2: The Logs Cycle Has Two Conflicting Contracts

Task 004 lines 49-50 say the pane closes only through its own close control.
Lines 51-53 require a second `Logs` press on the same service to close it. The
lower-context prompt repeats only the first rule at lines 155-157 and omits
the cycle from its acceptance criteria.

Keep one rule throughout: Close or the same service's `Logs` closes the pane;
the other service's `Logs` switches its content; selecting a card preserves it.
Include the complete cycle in the implementation steps, tests, and abbreviated
prompt. The cycle is present today, but the packet can still direct a model to
remove it.

### R5 - P2: Log Completion Still Owns The Detail Panel

Task 004 lines 68-84 require pane independence, but do not retire the existing
completion behavior. `src/app/show.rs:408` unconditionally opens the returned
log and then calls `show_card_detail(LiveMetadata)` at `:420`.

An operator can request logs and select Stream before the read completes; the
completion then returns the panel to Live Metadata. While switching log units,
Close can also be undone by the pending read's success. Two reads can finish
out of order and leave the earlier unit displayed. The state-only card tests
in the packet do not cover these callbacks.

Require the requested log selection to take effect before dispatch. Correlate
results with that selection, invalidate pending results on Close or another
selection, and apply results without changing panel mode or visibility.

Retire the obsolete portions of
`adr_0063_show_detail_panel_owns_detail_and_transport_stays_on_show`: it
currently requires the forced panel navigation and log rendering in the panel
(`tests/architecture_tests.rs:14146`, `:14202`, and `:14204`). Keep its unrelated
panel and transport checks. Add tests for delayed reads after card selection,
Close, and a second unit selection, including reversed completion order.

### R6 - P2: Long-Line Readability Has No Consistent Overflow Rule

Task 004 lines 57-59 permit either clipping with `overflow_hidden()` or line
wrapping. Its visual criterion at lines 104-105 requires an unwrapped line.
Clipping can hide the error at the end of a long journal line, while wrapping
can violate the visual criterion. Avoiding `truncate()` alone does not make
the complete log readable.

Specify unwrapped lines with horizontal scrolling, alongside vertical log
scrolling. Require an operator to reach and read the end of a line wider than
the pane. Carry the same rule into the lower-context prompt.

## Verification Gaps

- Preserve new interaction state through every Show reprojection. The current
  `reproject_show_page` rebuilds the VM and carries only panel mode and open
  state (`src/app/show.rs:293`). Test that queue and readiness refreshes keep
  pending commands and the pane's open state, selected unit, and chosen height.
- A7's row placement is called mechanical acceptance, but the packet specifies
  no structural guard for it. Add a focused ownership guard or classify the
  placement as an explicit operator check. The existing full-message visual
  check remains necessary at the narrowest supported sidebar width.
- Run the relevant Library tests as well as Show tests if the feedback packet
  changes `src/view_models/library.rs`. The listed `cargo test show --lib`
  filter does not cover that module's feed-update tests.
- Every new architecture guard must name its class and owning ADR. Keep the
  packet status, delivery-order row, and pending-human-checks entry open until
  the implementation's visual checks pass.

## Scope And Order

Keeping A7-A9 in one bounded feedback packet is reasonable. They affect two
different surfaces and must retain their separate owners: A7 belongs to the
Library feed-update surface; A8 and A9 belong to Show. They do not require one
generic command-feedback abstraction.

Run task 004 before the feedback packet. Record this prerequisite in the
feedback packet and the delivery-order dependency table; a Ready progress row
alone does not record it.

## Recommendation And Validation

Revise both packets before implementation. Their goals fit the existing ADRs,
but the callback lifecycle and shared resize scope need concrete decisions.
Neither packet nor any Rust source was changed by this review.

This was static task and source inspection. No Cargo suite was run because no
implementation changed. The review's local links and diff whitespace were
checked separately. No operator visual gate was accepted by this review.

## Operator Visual Check

No new UI was implemented in this review, so it creates no additional desktop
check. Each implementation must supply its own terminal setup, observable
failure criteria, and cleanup, including the race and overflow cases above.
