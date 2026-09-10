# ADR 0059 / ADR 0063 Event Row Amendment Review

## Status

Changes required after follow-up review - 2026-09-09. The previous three
requested corrections are present. One recovery gap remains in the newly added
post-registration liveness-check flow.

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

## Remaining Finding

### P2: A failed initial liveness check still leaves no in-app recovery action

Location: task 016, lines 71 and 91–104.

The revised flow registers an event, then requests a liveness check. If
registration succeeds but that read fails, the registry correctly preserves
`Unknown`. The table offers nothing in that state, and the packet specifies
neither another check nor an automatic retry.

This is reachable after a transient connection failure between Create/Replace
and the subsequent read. When connectivity returns, both registration actions
remain disabled and the event cannot advance to Attach. The operator is again
left needing the CLI despite already having a valid new event and token file.

There is no existing background status update to rely on:

- [check_event](../../src/broadcast/registry.rs), lines 160–175, updates stored
  status only after a successful relay response. Its current production caller
  is the CLI.
- [selected_event_input](../../src/app/show.rs), lines 934–945, only reads the
  registry status. Refreshing Show does not recheck it.
- [The observation actor](../../src/runtime/broadcast_observation.rs) explicitly
  does not write registry status. Its next observation therefore cannot be
  assumed to resolve the row's stored Unknown state.

Add an explicit Check/Retry check action for the selected Unknown event. It
must call check_event for that same identifier, never create a replacement,
report read failures in the mounted row, and become available again after a
failed check. Preserve the successfully registered event and token path if its
follow-up check fails; distinguish registration success from check failure.

Add a command-level regression case: registration succeeds, its first check
fails, retry succeeds, and the same event becomes eligible for Attach. Assert
that registration ran once, the registry entry and token path are unchanged,
and no target command ran. Also test retry returning 404 so the same row offers
Replace only after death is established.

## Verification And Recommendation

Static review only. Checked the ten table rows against the real enum variants,
read the registry error/update order, and traced production check_event callers
and Show refresh. Documentation links and diff whitespace: Green. No Rust
implementation or visual behavior changed in this review.

Resolve the single remaining retry path before marking this review passed.
All previously reported document conflicts are closed. Lost-token recovery for
a still-live event remains outside task 016's current Replace availability.

## Operator visual check

No new visual check for this documentation review. Task 016's implementation
must provide desktop commands, prerequisites, expected results, and cleanup for
its own visual gate. Task 004's accepted log and copying checks remain closed.
