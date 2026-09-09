# ADR 0065: Payment Route Tag Repair

## Status

Accepted - 2026-09-08.

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

`Check all feeds` keeps its meaning: it asks MusicIndex what changed. It is not
a repair, and it does not claim to be one.

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
- `Check all feeds` performs no tag repair, and claims none.
- A state label carries no click target.

## Alternatives Considered

### Write The Tag During Feed Refresh, And Drop The Staleness Gate

Rejected. It would fetch every feed on every check, and it hides a file write
inside an operation that reads. An operator who checks for updates does not
expect the app to rewrite the files.

### Repair Every Missing Tag At Start-Up

Rejected. It writes to the operator's files without being asked, and it needs
the network at start. ADR 0064 repairs paths at start because that touches rows
only.

### Read Payment Routes At Broadcast Time, With No Tag

Rejected. The publisher reads the file, not this app's database. The tag is the
contract, and a file that travels to another machine carries its own routes.

## Consequences

- A new repair service reads MusicIndex for one track and writes the tag.
- The readiness list gains a per-row action and a repair-all action.
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
