# ADR 0059 / ADR 0063 Event Row Amendment Review

## Status

Passed for amendment completeness - 2026-09-09. All findings are resolved in
the ADR and task packet. Task 016 is ready for implementation; this review does
not claim that its code or acceptance checks are complete.

## Scope

Reviewed [ADR 0059](../adr/0059-broadcast-control-surface.md),
[ADR 0063](../adr/0063-show-dashboard-layout.md), the
[current ADR index](../adr/README.md), the
[delivery order](../plans/broadcast-chain-delivery-order.md), and
[task 016](../tasks/adr-0059-task-016-event-row-in-live-metadata.md).
Compared the packet with the registry API, actual event/attachment/service
enums, Show refresh path, and existing guard. This is the current review result.

## Resolved Findings

| Finding | Resolution in the revised packet |
|---|---|
| Selected dead event prevents Create | Explicit Replace works with the dead entry still selected, preserves that entry, and requires the new event to appear without a reload. |
| Three summary rows contradict the card contract | Three rows belong in detail; the card retains two summary lines with defined roles. |
| Readiness covers absence only | The ten-row table covers all existing event, attachment, and service variants. Dead events and inactive services prevent ready state. Unavailable attachment observations remain distinct from confirmed NotAttached. |
| Blanket guard deletion | Applicable queue-separation assertions remain; only superseded assertions are updated or removed. |
| Invalid partial ADR status | ADR 0059 and its index entry say Accepted with task 016 outstanding. |
| Replace lacks an explicit no-attach rule | Both Create and Replace are forbidden from attaching, including in acceptance and the coding prompt. |
| ADR 0063 contradicts the new field ownership and lacks a Status amendment note | Its consequences now specify three sections with nested event display; Status records the dated amendment. |
| Newly registered event has no initial liveness check | Both registration actions now request check_event after completion. Its successful read and 404 outcomes match the registry implementation. |
| Failed initial liveness check strands the new event at Unknown | Unknown offers Check / Retry check for the same identifier. Registration success survives check failure; retries preserve the event and token file and cannot register or mutate publisher targets. Command-level failure/retry regression cases are required. |

## Retry Recovery Contract

The final correction closes the last P2. After successful Create or Replace,
the mounted row shows the new identifier and token path before its initial
liveness check. A failed check preserves that registration, displays a separate
check failure, and exposes Retry check. Existing unknown events also have Check
when Show opens. A retry uses the same identifier and never creates another
event or changes publisher configuration.

The packet defines progress and availability in the view model and prevents
overlapping registration/check commands. A successful retry projects Live and
applies the attachment rules. A retry returning 404 establishes Dead before
Replace becomes available. Repeated read failure preserves Unknown and enables
another retry. No path infers liveness from a failed request.

The contract uses the existing ownership boundaries:

- [check_event](../../src/broadcast/registry.rs) owns stored liveness updates.
  Failed relay reads leave that status unchanged; the application reports other
  check errors, including a failed status write, as check failures too.
- [selected_event_input](../../src/app/show.rs) only reads registry status.
  Refreshing Show does not recheck it, so the packet explicitly adds the check
  command and retry path.
- [The observation actor](../../src/runtime/broadcast_observation.rs) explicitly
  does not write registry status. Its next observation therefore cannot be
  assumed to resolve the row's stored Unknown state.

Required application-command tests cover both Create and Replace followed by
initial check failure, retry success, repeated failure, and retry 404. They
assert one registration, checks against the same identifier, preservation of
event identity and token path/file, and no publisher target mutation. Confirmed
NotAttached with otherwise valid attachment inputs enables Attach after a live
retry. Replace also retains the original dead entry. The packet assigns these
tests the `show_event_*` prefix so its existing test command includes them.

The operator criteria include a reproducible relay fixture, visible failure and
progress feedback, same-row recovery, prerequisites, and cleanup. Those checks
belong to implementation acceptance and have not been run.

## Verification And Recommendation

Static review only. Checked the table and retry transitions against registry
error/update behavior and Show refresh. The ADR decision, packet constraints,
steps, acceptance criteria, coding prompt, and delivery order agree.
Documentation links and diff whitespace: Green. No Rust implementation or
visual behavior changed in this amendment.

No required amendment fixes remain. The existing registry and runtime ownership
remain intact, and applicable queue-separation guards remain required. Proceed
with task 016 in a separate implementation session. Its regression tests and
operator gate remain required; this documentation pass does not satisfy them.
Lost-token recovery for a still-live event remains outside task 016's current
Replace availability.

## Operator visual check

No new visual check for this documentation review. Task 016's implementation
must provide desktop commands, prerequisites, expected results, and cleanup for
its own visual gate. Task 004's accepted log and copying checks remain closed.
