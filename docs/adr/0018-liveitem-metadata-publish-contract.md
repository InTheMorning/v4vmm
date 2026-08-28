# ADR 0018: Live Item Metadata Publish Contract

## Status

Accepted - 2026-04-27.

## Context

Phase 2 playback state already produces `NowPlayingUpdate` JSON from local
database facts. Future metadata streaming needs a narrow relay integration so
this app can publish live metadata without making podcast apps poll the local
desktop process.

Podcast apps listening to live items need to subscribe by the event identifier
declared in the live value block. The publish contract therefore must keep that
event identifier explicit and stable at the relay boundary.

## Decision

The MusicIndex API client will publish live metadata to:

```text
POST /v1/liveitems/{event_id}/metadata
```

The JSON body carries the same `event_id` plus the streamed metadata payload.
The path and body identifiers must match before the request is sent.

The first payload contract is deliberately source-fact oriented:

- `event_id`: live item event identifier used for relay routing
- `metadata`: arbitrary JSON metadata assembled by the playback/relay layer

This client contract does not infer metadata identity from titles, filenames,
or fuzzy matching. The relay is expected to route by `event_id`; later adapters
can decide which `NowPlayingUpdate` fields to include in `metadata`.

## Consequences

- Podcast apps can listen for updates tied to the same event identifier present
  in their live value block.
- The desktop app keeps relay publishing outside GPUI event handlers.
- The first implementation avoids adding a local HTTP server or relay transport
  before playback adapters exist.
- Future authentication, signing, and transport retry behavior can wrap this
  client contract without changing the event routing key.
