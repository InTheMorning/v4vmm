# ADR 0014: PlaybackSession Authoritative State

## Status
Accepted - 2026-04-26.

## Context
`v4vmm` needs a stable Phase 2 boundary before adding player adapters, relay
transport, Icecast metadata pushes, or a redesigned UI. Playlist rows already
provide ordered track intent, but they do not define canonical now-playing
state on their own.

## Decision
`PlaybackSession` is the authoritative source for now-playing state. Player
adapters may report playback facts such as position, pause state, and local
file progress, but they must not define canonical metadata identity.

The first implementation will expose this state as `NowPlayingUpdate` JSON
assembled from local database facts:

- feed GUID
- item GUID
- local track id
- position
- duration
- display metadata
- source value block
- raw extra JSON

The CLI is the first integration surface:

- `v4vmm playlist play --dry-run <playlist-id>`
- `v4vmm now-playing --json`

## Consequences
- The future UI can control and inspect playback state without owning it.
- Relay work can consume the same JSON contract instead of inventing a second
  metadata format.
- Player adapters remain replaceable because metadata identity stays in the
  local database and `PlaybackSession`.
