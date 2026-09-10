# ADR 0065: Payment Route Tag Repair

## Status

Implemented - 2026-09-10.

Reconciled 2026-09-10: the operator confirmed Show action feedback task 001 tested
and passed, closing the final feed-result readability gate. Tag-repair
[task 001](../tasks/adr-0065-task-001-tag-repair-service.md) and
[task 002](../tasks/adr-0065-task-002-readiness-list-actions.md) were already complete.

Amended 2026-09-10: the expanded feed-check counts now have their own result row.
`adr_0065_feed_check_result_has_its_own_full_width_row` guards placement without
changing count wording. [Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md#operator-visual-check)
records the passed readability check.

## Context

A track is ready to broadcast when its file carries an embedded
`TXXX:MusicIndex Value Routes` tag. The broadcast readiness report counts a
track without one as not ready.

Two code paths write that tag today:

- `subscribe_service`, at download time
- `refresh_stale_feed`, when a feed changed at MusicIndex

`check_feed_staleness` returns nothing when the stored `musicindex_updated_at`
is not older than the value the API reports. `refresh_stale_feed` then does not
run, and no tag is written.

An operator reported 17 tracks without the tag on 2026-09-08. `Check all feeds`
changed nothing, because no feed was stale. The tracks were downloaded before
the tag logic existed, and their feed has not changed since.

**The repair is gated on a condition that has nothing to do with the defect.**
A file lacks a tag for local reasons: an old download, a failed write, a
replaced file. None of those make a feed stale, so the only path that writes the
tag can never run for them.

The readiness list shows the problem and offers no action. The `Missing routes`
text renders in the badge row beside `In library` and `Track`, so it reads as a
control, and nothing is wired to it.

## Decision

### A Tag Repair Is Gated On The File, Not On The Feed

The app repairs an embedded tag when **the file lacks it**. Whether the feed
changed upstream is a separate question with a separate answer.

### Check All Feeds Repairs What It Finds

Amended 2026-09-08, when an operator asked why the button does not fix the
tracks it reports.

`Check all feeds` does both jobs in one press:

1. ask MusicIndex which feeds changed, and apply those updates as before
2. repair any track whose file lacks the payment-route tag

The two were separate because a repair writes files and a check reads. That
reasoning does not survive the code: `refresh_stale_feed` already writes ID3
tags for every track of a stale feed. The button was never a read.

The rule it replaces is simpler to state and matches what an operator expects.
**A track is repaired because its file lacks the tag, never because its feed
changed.** The check is how the operator asks; staleness decides only which
feeds need new data.

The first press is slow, because each track without the tag costs one fetch. A
later press is cheap, because a recorded `NoRoutesUpstream` is trusted and asks
nobody.

### Repair Is An Operator Action, Not A Start-Up Step

The repair reads from the network and writes to the operator's files. It runs
when the operator asks, for one track or for every not-ready track.

This is unlike the path repair of ADR 0064, which touches only rows and runs on
its own. **A step that writes a file waits to be asked.**

### The Result Separates What This App Can Fix

A repair attempt ends in one of three states, and the report names which:

- `Repaired`. The tag is written and the track is ready.
- `NoRoutesUpstream`. MusicIndex has no payment routes for the track or its
  feed. This app can not fix it, and the publisher must add them.

  The app records that answer, and a repair of every track trusts it rather than
  asking again. **An operator who repairs one track asks again**, because a
  publisher can add the routes at any time. Without that, a track recorded once
  could never be repaired, whatever the publisher did later. Amended 2026-09-08,
  after review found the recorded answer short-circuited both paths.
- `Failed`. The fetch or the write failed, and the reason is recorded.

The second state is the one that matters. Without it an operator can not tell a
missing local tag from a feed that pays nobody, and both read as "not ready".

### A State Label Is Not A Control

`Missing routes` is a state, and it renders as a state. An action renders as an
action, and every action does something.

A surface that shows a problem offers the action that fixes it, or says plainly
that the fix is not here.

## Invariants

- A tag repair is available when the file lacks the tag, whatever the feed says.
- The repair never runs without an operator action.
- A repair result names which of the three outcomes happened.
- A repair of one track asks upstream again. Only a repair of every track
  trusts a recorded `NoRoutesUpstream`.
- `Check all feeds` repairs every track whose file lacks the tag, whatever the
  feed staleness says.
- A state label carries no click target.

## Alternatives Considered

### Keep The Check And The Repair As Two Separate Actions

Rejected on 2026-09-08, after it shipped that way. It asks the operator to know
that a second action exists, at the moment the app looks broken, and it leaves
one button reporting a problem it declines to fix.

The argument for it was that a repair writes files and a check reads. That was
wrong: `refresh_stale_feed` already writes ID3 tags. The check was always a
write.

### Drop The Staleness Gate And Refresh Every Feed On Every Check

Rejected. Feed staleness still decides which feeds need new data. The repair
does not need that gate, and removing it would refetch every feed for no gain.

### Repair Every Missing Tag At Start-Up

Rejected. It writes to the operator's files without being asked, and it needs
the network at start. ADR 0064 repairs paths at start because that touches rows
only.

### Read Payment Routes At Broadcast Time, With No Tag

Rejected. The publisher reads the file, not this app's database. The tag is the
contract, and a file that travels to another machine carries its own routes.

## Consequences

- A new repair service reads MusicIndex for one track and writes the tag.
- `Check all feeds` gains the repair, so an operator who presses it once fixes
  what this app can fix.
- The readiness list keeps a per-row action, for a track an operator wants to
  retry on its own.
- The readiness report gains a state for a track whose feed carries no routes,
  so a second run does not retry what cannot be fixed.
- `Missing routes` stops rendering as a control.
- An operator learns which tracks need a publisher to act.

## Follow-Up Work

- Decide whether the app warns at download time when a feed carries no payment
  routes, so the problem is visible before a show.

## References

- ADR 0059, broadcast control surface, for the readiness report
- ADR 0062, music content surface, for the list that shows the rows
- ADR 0064, local file addressing, for the repair that runs on its own
- `src/feed_service.rs`, `check_feed_staleness` and `refresh_stale_feed`
- `src/metadata_service.rs`, `id3_edits_for_track_context`
