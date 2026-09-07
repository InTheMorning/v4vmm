# Broadcast Recording And Feed Publishing

Research note. Recorded on 2026-09-06. This is a direction, not a decision. No
ADR exists yet and no work is scheduled.

## The Idea

An operator runs a live show. Today the chain sends live payment routing to
listener apps while the show plays, and nothing survives the show.

**One show becomes one podcast episode.** The model is live-to-tape: the
broadcast is the episode, with no track-level division and no editing step.
Decided on 2026-09-06.

The episode is complete when it has:

- one MP3 recording of the whole show
- a valid Podcasting 2.0 RSS item, with the sidecar files it references
- chapters that match the recording timeline
- value time split blocks that match the same timeline, so a listener who plays
  the recording pays the same artists that a live listener paid

The episode then goes to the feed, a podping announces it, and Podcast Index
indexes it. The result is a new episode of a live-to-tape music podcast.

Later, the operator wants to edit the hosted RSS and manage the MP3 files from
this app, when the hosting provider permits it.

## Which Component Records The Timeline

**Not this app.** ADR 0059 says the chain must operate when this app is closed.
For a Mixxx show or a liquidsoap show, this app sees a track change only while
it runs and polls. A recorder that stops when the operator closes the laptop is
not a recorder.

`musicindex-live-publisher` is the correct owner. It watches the drop
directory, so it sees every track change from every producer. It already
handles time, because it compensates for stream delay. It already holds the
payment routes for each track. It is the component that runs for the length of
the show.

This also covers the built-in player for free. The `mpv` producer of ADR 0059
task 011 writes the same drop files, so one recorder serves every source.

The audio is a separate problem. `butt` makes the recording, and `butt` runs
where the audio is. The publisher and the encoder can be on different machines,
so the timeline and the MP3 may be produced in two places and joined
afterward. A design must not assume one host.

The division is settled. `musicindex-live-publisher` keeps a detailed log of
what played and when. This app reads that log and generates the episode XML and
JSON. The publisher generates nothing.

The publisher side is specified in its ADR 0003, the show log contract. This
app builds the generation side, which is a later ADR here.

## Why The Data Already Exists

The live chain already produces the timeline that an episode needs.

Every track change writes a drop file, and the publisher sends a payload with
that track's payment routes. That sequence of track changes, with times, is
the same data that a chapter list and a value time split list need. One
timeline serves both the live path and the recorded path.

The value routes are already correct at that moment. This app wrote them into
the file tags, and the producer read them back. A value time split block for the
recording is the same route set with a start time and a duration.

The encoder already records. `butt` accepts `-r` to start a recording and `-t`
to stop it, and ADR 0059 task 015 already reports the recording state. The
capture half has a hook.

## Recording Sources

The encoder recording bounds the episode. `butt` does that today, and this app
starts and stops it, so this app knows the window. Confirmed on 2026-09-06.

Later, two optional backup recorders join it:

- a local recorder in this app, with `ffmpeg`
- a recorder beside the publisher, on the broadcast host

Redundancy is the goal. A failed encoder recording must not lose the show.

**Every recorder must record its own start time in wall clock.** That is the
only way a recording aligns to the log. Without an accurate start time,
alignment is a guess, and a guess puts every chapter and every payment split in
the wrong place.

**Real time is canonical.** This app, the encoder, and the publisher all assume
and present real time, which is `observed_at` in the show log. Decided on
2026-09-07. There is one timeline. An artifact that does not sit in it must be
corrected into it.

A local recorder already sits in real time. `butt` captures at the encode
point, before the encoder queue, the icecast queue, and the player buffer. An
`ffmpeg` recorder at the player does the same. Both need no correction.

**A publisher backup recorder that pulls the stream after icecast will not sit
in real time.** Note this before that capability is built. The work includes a
resynchronization step from the pulled audio to local timestamps, and that step
is not a subtraction.

The offset is not a constant. Icecast buffering, network conditions, and the
client buffer state all vary, so a pulled recording can drift within one show
rather than sit at a fixed distance from real time. A fixed correction of the
configured `stream_delay_secs` therefore looks right at the start of a show and
is wrong by the end.

That makes a local recorder the reliable backup and a post-icecast pull a
verification artifact at best, until somebody proves a resynchronization method
that holds for a whole show.

**The delay values are estimates. No measurement exists for either one.**
`stream_delay_secs` is an operator setting for live delivery. The latency that
`butt` adds to a local recording is unknown. Treat both as
calibration values: keep the raw times, keep the configured delay, and let a
later measurement correct an episode without a second show.

## Scope And Order

Decided on 2026-09-07.

**Now: local only.** The operator records the show locally, in near real time.
`butt` does it today. An `ffmpeg` recorder in this app is the first backup,
because it also sits in real time and needs no correction.

**Live delivery keeps its own delay.** The publisher holds each payload for a
configurable delay before the public endpoint, so a live listener's app flips
the value block near the moment that listener hears the change. That delay
serves the live path only and never reaches a recording.

**Publisher-side recording is gated.** A post-icecast recording drifts, so it
cannot be used until this app can make serious edits to an episode. The order
is therefore:

1. Local recording and episode generation.
2. Post-processing tooling in this app.
3. Publisher-side backup recording, which depends on step 2.

Building step 3 before step 2 produces a recording that nobody can repair.

**Until step 2 exists, the operator repairs by hand.** A mistake made live, or
a wrong value in the source data, is fixed by editing the generated files
directly. That is tedious and it is accepted for now.

## What Hand Repair Requires

Manual repair is the only fix available for the first stage, so the generated
files are an operator interface, not an internal format.

- Write readable XML and readable JSON. Indent them. Do not emit one long
  line.
- Keep element order and attribute order stable between runs, so a second
  generation produces a small difference and not a whole-file change.
- Keep a chapter and its value time split adjacent in the output where the
  format allows, so an operator who corrects one can see the other.
- Never generate a file that only this app can read.

A later post-processing tool replaces the tedium. It does not replace the need
for the output to be legible.

## Editing Policy

The operator may change the value routes of a generated episode. A route in the
log is a source fact. An operator change is an override. The two stay separate,
as the provenance rule of this repository requires, so a reader can always see
what actually played and what the operator chose to publish.

Nothing else is editable. **This is strictly live-to-tape.** There is no trim,
no reorder, no chapter time adjustment, and no audio edit. The recording is the
show.

Post-production support is deliberately deferred. It changes the timeline, and
the whole design rests on one timeline that the log and the audio share.

## The Pieces

| Piece | State |
|---|---|
| Audio capture | `butt` records today. The app can start and stop it, and `butt -S` reports `record path`, `record seconds`, and `record kBytes`. Backup recorders are later work. |
| Timeline capture | Specified. `musicindex-live-publisher` ADR 0003, the show log contract. Two task packets written. |
| Chapter generation | Not built. Derives from the timeline. |
| Value time split generation | Not built. Derives from the same timeline plus the route set for each track. |
| RSS item generation | Not built. |
| Upload and synchronization | Not built. See below. |
| Hosted feed edit | Not built. Depends on the provider. |

## Value Time Split Form

`curiohoster/sk/getvts.js` builds the elements. It emits two forms inside one
`<podcast:valueTimeSplit>`, and it tests `feedGuid` first:

- `<podcast:remoteItem feedGuid="..." itemGuid="..."/>` when the track is known
  in a feed. Payment resolves through that feed's own value block.
- `<podcast:valueRecipient .../>` for each destination otherwise. The splits
  are copied into the episode.

**A show from this app must use the remote item form.** Every track this app
downloads carries `TXXX:MusicIndex Feed Guid` and `TXXX:MusicIndex Track Guid`,
so both identifiers are always available. The remote item form sends the
listener to the artist's own feed, so a later change to the artist splits still
reaches the artist.

The inline recipient form is the obvious implementation and the worse one for
this case. It freezes the splits at the moment of recording.

The element also carries `startTime`, `duration`, and `remotePercentage`.
`getvts.js` defaults `remotePercentage` to 100 and lets a setting override it.
That value decides how much of a boost reaches the remote item instead of the
show itself, so it is a product decision and not a detail.

## Announce And Index

The last step is a podping, which tells Podcast Index that the feed changed.

`~/build/podping-gossipwatcher` is local prior art. It receives podpings over
Iroh gossip, verifies ed25519 signatures against a trusted-publisher list, and
needs no account and no API key. It is the receive side, so the send side is
still unwritten, but the signing and the transport are understood in this
codebase already.

## Transport Options To Investigate

The first target is a fully controllable VPS, which is the operator setup
today. Support what works there first.

- **rsync.** Simple, proven, and enough for a VPS. Likely the first
  implementation.
- **Syncthing.** Continuous synchronization with no explicit push step. Useful
  when the operator machine and the server are both long lived.
- **Git.** Version control for the feed XML. Attractive because a feed edit
  becomes reviewable and reversible, which matters when a bad edit breaks every
  subscriber.
- **Hosting provider APIs.** Each provider differs. Survey what common
  providers permit before any design. Some permit no upload at all.

## Prior Art To Read

- `curiohoster/sk/getvts.js`. A working value time split generator. Read first.
- `~/build/curio-pub`, described as a Podcasting 2.0 compliant RSS publishing
  tool. Cloned on 2026-09-06.
- `~/build/podping-gossipwatcher`. Podping transport and ed25519 signing, on
  the receive side.
- `thebells1111/sovereign-feeds`. The feed hosting side of the same ecosystem,
  and the place the live value URI is pasted today.

## Open Questions

- How does a backup recorder report its start time to this app, when it runs on
  the broadcast host?
- Which recording wins when two of them cover the same show?
- How does a post-icecast recording resynchronize to real time, given that the
  offset drifts? An in-band marker and an audio correlation against a local
  recording are both worth testing.
- What is the real post-icecast delay, and does `butt` add latency to a local
  recording? Both need a measurement before an episode can claim frame
  accuracy.
- `stream_delay_secs` defaults to `0` today, which means no compensation until
  an operator sets it. Should the default change to a measured value once one
  exists?
- Does an operator route override need a reason field for the listener, or is
  the changed split enough?
- How does the timeline find the matching audio file when the encoder runs on
  another host? For a local encoder this is answered: `butt -S` reports
  `record path`, verified on 2026-09-07. A remote encoder needs the path and a
  transfer.
- Does a backup recorder belong in the publisher repository, or in a third
  component?
- Does the recorded episode reuse the live event identifier, or is it separate?
- How does a value time split block express a route set that changed during one
  track?
- What `remotePercentage` should a show use by default?
- Does the operator publish the podping, or does the hosting provider?
- Which provider surface, if any, is worth supporting beyond a VPS?
- Does feed editing need its own conflict model, given that a hosting provider
  may also edit the same file?

## Route

A future ADR, after the ADR 0059 broadcast control surface ships. The live path
must be stable before a recorded path derives from its timeline.

The ADR probably belongs to `musicindex-live-publisher`, because that
repository owns the component that sees the whole show. This repository gets a
second ADR later, for the feed edit and upload surface only.

Nothing in the ADR 0059 phases forecloses this work. No decision is needed
before those phases ship.

Tracked in `docs/plans/deferred-architecture-work-index.md`.
