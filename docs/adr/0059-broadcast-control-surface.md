# ADR 0059: Broadcast Control Surface

## Status

Accepted - 2026-09-06.

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

### The Panel Is Named Broadcast

The frame is `Broadcast`. It has three sections: `Source`, `Publisher`, and
`Event`.

The `QueueNowPlaying` frame keeps its name and its meaning. It shows local
playback in this app. The two frames can be active at the same time. They must
stay separate in code and in the interface.

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
