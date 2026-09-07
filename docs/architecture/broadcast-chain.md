# Broadcast Chain

This document names the parts of the live payment chain and the contract at
each boundary. It is the shared map for four repositories that release
separately.

Read this before a change that crosses a repository boundary.

## Components

| Component | Repository | Role | Runs as |
|---|---|---|---|
| `v4vmm` | `v4vmm` | Control surface and status display. Writes MusicIndex tags into downloads. | Desktop app |
| `mixxx-now-playing` | `musicindex-live-publisher` | Producer. Reports the track that Mixxx plays. | User service |
| `musicindex-live-publisher` | `musicindex-live-publisher` | Publisher. Transforms drop files and sends live value payloads. | User service |
| `musicindex-live-relay` | `splitkit` | Relay. Holds the current payload and sends it to listener apps. | Network service |

Liquidsoap arrives later as one more producer. It needs a producer that writes
the same drop-file contract.

## The Chain

```text
v4vmm  ── writes MusicIndex tags into the audio file
             │
             ▼
        audio file in music_dir
             │
             ▼
Mixxx (or mpv, or liquidsoap) plays the file
             │
             ▼
producer ── writes musicindex.nowplaying/1 drop file
             │
             ▼
musicindex-live-publisher ── sends the direct live value payload
             │
             ▼
musicindex-live-relay ── sends remoteValue to listener apps
             │
             ▼
v4vmm reads the same relay snapshot to show listener truth
```

`v4vmm` is at both ends of the chain. It is not in the middle. The chain
operates when `v4vmm` is closed.

## Boundary 1: Audio File Tags

Producer: `v4vmm`. Consumer: each now-playing producer.

`src/metadata.rs` writes these `TXXX` frames into a download:

- `TXXX:MusicIndex Feed Guid`
- `TXXX:MusicIndex Track Guid`
- `TXXX:MusicIndex Image`
- `TXXX:MusicIndex Value Routes`

`Value Routes` holds a JSON array of payment routes. The field names are the
names in `v4vmm/src/api.rs`, `PaymentRoute`.

If `v4vmm` writes no `Value Routes` tag, the chain has no payment splits for
that track. The publisher then sends a payload with no destinations.

## Boundary 2: The Drop File

Producer: each now-playing producer. Consumer: `musicindex-live-publisher`.

The contract is `musicindex.nowplaying/1`. The publisher owns it. ADR 0002 in
the publisher repository defines it.

Rules:

- The presence of a file means that the track plays.
- The removal of a file means that the track stopped.
- A producer writes a temporary file in the same directory and then renames it.
- An unknown schema version is ignored, not guessed.

The `value_routes` array uses the `PaymentRoute` field names, so a producer can
copy the tag content without a translation step.

## Boundary 3: Publish To The Relay

Producer: `musicindex-live-publisher`. Consumer: `musicindex-live-relay`.

The relay accepts two body forms and separates them by exact key match:

- A body with exactly the keys `event_id` and `metadata` is the wrapped form.
- Any other body is a direct live value payload.

Listener apps read `remoteValue` and expect the direct form. Payment splits are
in `value.destinations`. Only the publisher sends payloads. `v4vmm` sends none.

## Boundary 4: Read From The Relay

Producer: `musicindex-live-relay`. Consumers: listener apps and `v4vmm`.

`v4vmm` reads `GET /v1/liveitems/{event_id}/metadata` to show what listeners
receive. This read operates for a local publisher and for a remote publisher,
because the relay is a network service.

## Boundary 5: Service Control

Producer: `v4vmm`. Consumer: `systemd` on the host that runs the publisher.

`v4vmm` runs `systemctl --user` for a local host. For a remote host it runs the
same commands through `ssh`. The publisher repository owns the unit files.

A control API in the publisher is future work. The liquidsoap work needs it.

## Ownership Rules

- `musicindex-live-publisher` owns its configuration file and its setup rules.
  No other component writes that file. `v4vmm` reads it to show its content and
  calls the publisher command-line tools for each change.
- The drop-file contract belongs to the publisher. A change needs a new schema
  version.
- `v4vmm` owns the event registry, because the relay cannot list or delete
  events.
- Broadcaster tokens are files with mode `0600`. No component puts a token in a
  database or in a log.

## Known Limits

- The relay keeps state in memory. A restart discards live items, tokens, and
  snapshots. Every event then dies and listeners must tune again.
- The relay returns a broadcaster token one time. It keeps a hash and cannot
  return the token again.
- The relay has no route to list live items and no route to delete one.
- The drop-file contract has no pause state. A producer reports play or stop
  only.
- Long-lived events are future work in `splitkit`. Weekly shows and permanent
  stations need an event that survives a relay restart.

## References

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
- `musicindex-live-publisher`: `docs/adr/0002-nowplaying-drop-file-contract.md`
- `musicindex-live-publisher`: `docs/architecture/broadcast-chain-boundaries.md`
- `splitkit`: `README.md` and `docs/interoperability.md`
