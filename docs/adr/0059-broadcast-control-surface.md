# ADR 0059: Broadcast Control Surface

## Status

Accepted - 2026-09-09. Implementation partial: tasks 001-015 complete and
verified by `docs/reviews/adr-0059-implementation-review.md`, task 016
outstanding.

The 2026-09-09 amendment that makes `Event` a row of `Live Metadata` opened that
task. ADR 0057 keeps the status at `Accepted` until every gate closes, and
forbids a fifth status for a partial state.

Amended 2026-09-09: event setup includes explicit Create, Replace, and retryable
Check actions. A dead entry stays selected, and a failed check after successful
registration must not strand the operator or discard the new identity. Task 016
specifies the action states and recovery tests.

Amended 2026-09-06: the `Event` section must show the ready-to-paste
`podcast:liveValue` tag with a copy action. Listener apps discover a live event
only through that tag in the RSS feed of the show. Without the tag, the whole
chain reports success and no listener receives anything.

Amended 2026-09-06: the broadcast surface gains a fourth section, `Stream`, for
the stream encoder. `butt` has a control interface with a status request, a
connect and disconnect pair, and a network address option, so the section can
be built before any remote playback work. The three-section decision below
becomes four. Nothing else changes.

Amended 2026-09-06: the relay has a second death mode that the first draft did
not record. It removes an event after an idle TTL, and the default is 24 hours
(`splitkit`, `src/lib.rs`). The decisions below do not change, because the
liveness test already treats a `404` as a dead event for both modes.

Supersedes ADR 0018 and ADR 0019. This ADR carries the live decision for the
relay client surface. The work follows
`docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`.

## Context

ADR 0018 and ADR 0019 put a live-publish client in this app. `src/api.rs`
creates live items and sends metadata. `src/cli.rs` gives the `v4vmm liveitem`
commands. No operator used this path in a real show.

The path sends the wrong body shape. The relay accepts two body forms. A body
with exactly the keys `event_id` and `metadata` is a wrapped body. Any other
body is a direct live value payload. This app sends the wrapped form.

Listener apps read `remoteValue` and expect the direct form. Payment splits are
in `value.destinations` in the direct form. The relay does not reject the
wrapped form. The failure is therefore silent.

`musicindex-live-publisher` sends the direct form. It is a headless service in
a separate repository. It reads a drop file, transforms the data, and sends the
result to the relay.

The drop-file contract is `musicindex.nowplaying/1`. ADR 0002 of that
repository defines the contract.

This app already starts the payment chain. `src/metadata.rs` writes
`TXXX:MusicIndex Value Routes` and the GUID tags into each download.

`mixxx-now-playing` reads the same tags from the audio file. The publisher then
sends those routes to listeners. The route data comes from this app, but at
download time only.

The operator wants more than one music player. `mpv` is the built-in player for
podcast work. Mixxx is the player for a DJ set.

Liquidsoap or Mixxx AutoDJ can hold the stream when the operator sleeps. The
publisher can run on a different machine than this app.

Three facts about the relay control this design:

- The relay keeps state in memory only. A restart of the relay process discards
  live items, tokens, and snapshots.
- The relay removes an event after an idle TTL. The default is 24 hours. An
  event therefore dies without a restart.
- The relay has no route to list live items. It has no route to delete one.
- The relay returns the broadcaster token one time only. It keeps a hash of the
  token and does not return the token again.

## Decision

### This App Is A Control Surface

The broadcast chain must operate when this app is closed. This app shows state
and sends commands. It is not a part of the chain.

`mpv` is the one exception. `mpv` runs inside this app, so the `mpv` source
stops when the app stops. This limit is correct for desk work and is not a
defect.

### Sources Are Selectable

A source is a component that reports the current track. A source has a name, a
kind, a host, and an observation method. The kinds at this time are `mpv`,
`mixxx`, and `external`. Liquidsoap arrives later as one more kind.

The app must not put the name `mixxx` in shared logic. Only the source adapter
knows the kind.

### Observation Has Two Layers

Layer one is listener truth. The relay holds the payload that listener apps
receive. The app reads that payload with the relay client that it has today.

This layer operates for a local publisher and for a remote publisher, because
the relay is a network service.

Layer two is rig health. Unit state, drop-file presence, and log text tell the
operator if the local equipment operates correctly. This layer needs access to
the machine that runs the publisher.

The panel shows both layers together. A difference between the two layers is
the defect that the operator must see.

### This App Owns The Event Registry

The relay has no list route and no delete route. This app therefore keeps the
list of events that the operator created.

- `Create` calls the relay and stores the result.
- `Forget` removes the local record. The relay discards its own record on the
  next restart.
- `Resume` selects a stored event and then tests it. A `404` response shows
  that the event is dead.

A dead event must not cause an automatic replacement. A new event has a new
identifier, and listeners must then tune again. The operator makes that choice.

Amended 2026-09-09: the event row offers `Create` with no selected event and
`Replace` with a selected dead event. Replace registers a new event and leaves
the dead entry for Forget. Neither action attaches; attachment is a separate
operator action that changes publisher configuration.

Successful registration shows the new identifier and token path immediately,
then requests a liveness check for that identifier. Registration success remains
visible if the check fails. An unknown event offers `Check`, or `Retry check`
after failure, for that same identifier. A successful read establishes `Live`;
`404` establishes `Dead`; a failed read preserves the stored status. A retry
never registers another event or changes publisher configuration. Create,
Replace, and Attach remain unavailable while liveness is unknown, and a failed
check makes retry available again.

The existing registry service owns creation and stored liveness updates.
Application commands call it; the view model owns action availability and
separate registration/check feedback; the mounted row presents the result.
[Task 016](../tasks/adr-0059-task-016-event-row-in-live-metadata.md) owns this
flow and its mechanical and operator verification.

### Tokens Are Files

Nobody can replace a broadcaster token. The app writes each token to its own
file with mode `0600`. The app stores the file path in the database. The app
does not store the token in the database.

This matches how `musicindex-live-publisher` reads tokens today. It also lets
the operator copy one token to another machine.

### The Publisher Owns Its Configuration

`musicindex-live-publisher` owns its configuration file and its setup rules.
This app reads that file to show its content. This app does not write it.

All changes go through the publisher command-line tools. This app runs
`musicindex-live-publisher provision` and `setup-mixxx-musicindex`, and reads
the exit code and the output.

### Remote Control Uses SSH First

For a remote host, this app runs `systemctl --user` through `ssh`. The
publisher repository needs no change for this step.

A control API in the publisher is deferred. The liquidsoap work needs a larger
remote surface than start and stop. That surface is easier to design when those
requirements exist.

### The Publish Path Is Removed

Phase 1 removes `publish_live_metadata`, `publish_live_metadata_with_token`,
and the `v4vmm liveitem publish` commands. They send the wrong shape and have
no user.

`create_live_item`, `fetch_live_metadata_optional`, and `health` remain. Event
registration and listener truth need them.

### Lists From The Start

Events, sources, and hosts are lists in the data model and in the service
layer. The first user interface shows one selection at a time. Support for more
than one stream must not need a data model change.

### The Broadcast Sections Live In Show

Amended 2026-09-08 by ADR 0060, which removed the `Broadcast` frame. The four
broadcast sections mount inside the `Show` screen. Amended 2026-09-09 to three,
in this order: `Source`, `Live Metadata`, and `Stream`. `Event` became the first
row of `Live Metadata`.

Amended 2026-09-08. The section that holds the two services is named
`Live Metadata`, not `Publisher`. It holds the metadata producer and the
metadata publisher, so the section name states what the two services do. The
services keep the names `Producer` and `Publisher` inside it.

Amended 2026-09-09. **`Event` is not a section. It is the first row of
`Live Metadata`.** There are three sections: `Source`, `Live Metadata`, and
`Stream`.

An event is not a peer of the two services. It is the identity they publish to,
and without one the publisher has nothing to send. The rows of `Live Metadata`
now read in the order the chain depends on them:

1. `Event`, the identity the publisher writes to
2. `Producer`, which writes the drop file
3. `Publisher`, which sends what the producer wrote

Each row is a precursor of the one under it. A reader who starts at the top and
stops at the first row that is not ready has found the thing to fix.

The state of `Live Metadata` accounts for the event. **No event means the
section is not ready, whatever the two services report.** Two running services
with no event publish nothing, and a card that reads `Active` in that state is
telling the operator a comfortable lie.

A section is an optional field on `ShowPageVm` and a group of callbacks on
`ShowSlots`. An absent section renders nothing. It does not render as
unavailable.

`Event` shows the live item and the exact RSS tag that lets listener apps find
it:

```xml
<podcast:liveValue uri="EVENT_ID" protocol="socket.io"/>
```

The operator copies that tag into the feed of the show. This app does not write
the feed. Feed publication is future work, recorded in
`docs/research/broadcast-recording-and-feed-publishing.md`.

`Stream` shows the encoder that feeds the listeners, which is `butt` today.
`butt` accepts control on the command line and over a network address, so a
local encoder and a remote encoder use one code path and neither needs `ssh`.
The section reports the connection state and the recording state, and offers
connect and disconnect.

The app does not send a song title to the encoder. The producer already writes
the text file that the encoder reads.

The queue keeps its name and its meaning. It shows local playback in this app.
ADR 0060 moved it into `Show` beside these sections. The queue and the
broadcast sections stay separate in code and in the interface.

## Invariants

- The broadcast chain operates when this app is closed, for every source except
  `mpv`.
- No component writes the configuration file of `musicindex-live-publisher`
  except the publisher tools.
- Broadcaster tokens are files with mode `0600`. No token is in the database.
- The app reports a dead event to the operator. The app does not replace it
  automatically.
- The app sends no metadata to the relay. The publisher is the only sender.
- Source kind names appear in source adapters only.
- Encoder commands run only in the broadcast service layer, never in a screen.
- The app never sends a song title to the encoder.
- Relay clients come from `src/http_client.rs`, as ADR 0058 requires.
- A runtime actor runs all work that blocks, as ADR 0040 requires.

## Alternatives Considered

### Keep The Built-In Publish Path

Rejected. The path sends a body that listener apps cannot read for payment
splits, and no show used it. A second sender also gives two owners for one
event, with no lock between them.

### Add `musicindex-live-publisher` As A Library Dependency

Rejected. The publisher makes its own HTTP client that blocks. It owns its own
timeout constants. ADR 0058 forbids both in this app. The publisher is also a
separate package that can be absent or remote. A library dependency cannot show
that condition.

### Write The Publisher Configuration Directly

Rejected. The publisher setup script holds marker checks, backup steps, and
placeholder tests. A second writer would copy that logic and then drift from
it.

### Add A Control API To The Publisher Now

Rejected for this stage, but not rejected forever. An endpoint that starts and
stops services is a remote-command surface and needs authentication, a local
default bind address, and review. SSH gives the same result now with no new
surface. The liquidsoap work is the correct moment for that API.

### Make This App A Drop-File Producer For Every Source

Rejected. Mixxx and remote hosts already have producers. Only the `mpv` source
needs a producer in this app.

## Consequences

Positive:

- The app shows what listeners receive, for a local publisher and a remote
  publisher, with the client that already exists.
- Removal of the publish path deletes the only untested network write.
- The event registry keeps tokens that the relay cannot return again.
- One source model holds `mpv`, Mixxx, and liquidsoap.
- The three panel sections give the later interface work three separate owners.

Negative and risks:

- This app becomes the keeper of secrets that nobody can replace. If the
  operator loses a token file, that event dies.
- A relay restart kills every event. The operator must then create new events
  and tell listeners.
- SSH control needs key access to the remote host. An operator without keys
  cannot use the buttons.
- A publisher unit that fails five times in 300 seconds stays in the `failed`
  state. The panel must show that state, or `Start` appears to do nothing.
- The `mpv` producer adds a second writer of drop files in this project.

## Follow-Up Work

- `splitkit`: add long-lived live items. Weekly shows and permanent stations
  need an event that survives a relay restart.
- `musicindex-live-publisher`: add a remote control API when liquidsoap
  control starts.
- Liquidsoap source support, after the control API exists.
- Icecast state, if the operator needs it next to the publisher state.

## References

- ADR 0014 - PlaybackSession authoritative state
- ADR 0016 - Schema migration discipline
- ADR 0017 - CLI debug contracts
- ADR 0040 - Async view-model runtime
- ADR 0046 - Workspace frame architecture
- ADR 0057 - ADR status vocabulary and amendment policy
- ADR 0058 - Outbound HTTP client policy
- `musicindex-live-publisher` ADR 0002 - Now-playing drop-file contract
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
