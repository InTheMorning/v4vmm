# v4vmm + Live Relay Roadmap

## Vision

`v4vmm` should be the authoritative local music/V4V brain.

It should manage:

- RSS/V4V music ingestion
- local library + downloads
- ID3 metadata writing
- playlist management
- playback session state
- Value Time Split (VTS) resolution
- now-playing metadata

It should **not** depend on Icecast/Liquidsoap for canonical metadata.

Instead:

- Icecast/Liquidsoap = audio transport only
- `musicindex.org` / StopHammer = catalog + metadata lookup
- `v4v-live-relay` = public realtime metadata + VTS endpoint

This avoids title-string hacks like embedding `{feedGuid}{itemGuid}` into stream titles.

---

# Core Architectural Principle

## Authoritative State

The canonical source of truth is:

```text
PlaybackSession
```

—not the player, not Icecast, not Liquidsoap.

Everything should derive from:

```text
feed_guid
item_guid
local_track_id
position_ms
started_at
duration_ms
value_block
```

This enables:

- mpv
- MPRIS players
- internal player
- Liquidsoap monitoring
- Icecast metadata push
- remote VTS endpoints

…all using the same engine.

---

# Phase Roadmap

---

# Phase 1 — Playlist Foundation

## Goal

Create a stable playlist + queue model.

## Deliverables

### Internal DB tables

```text
playlists
playlist_items
playlist_sources
queue_state
```

### Support

- static playlists
- generated playlists later
- queue snapshots
- ordering persistence

### Export/import

#### M3U / M3U8

Use as universal transport.

#### musicL

Use as rich semantic transport.

musicL should preserve:

- feed GUID
- item GUID
- publisher
- album
- track metadata
- artwork
- value metadata
- local file path
- external IDs (MusicBrainz etc.)

## Technical Rules

### M3U is dumb transport

### musicL is canonical rich transport

Never rely on M3U for identity.

---

# Phase 2 — PlaybackSession Model

## Goal

Create the canonical now-playing state model.

## Rust Model

```rust
pub struct NowPlayingUpdate {
    pub session_id: String,
    pub sequence: u64,

    pub started_at: chrono::DateTime<chrono::Utc>,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,

    pub feed_guid: String,
    pub item_guid: String,
    pub local_track_id: i64,

    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub image: Option<String>,

    pub value_block: serde_json::Value,
    pub raw_extra_json: serde_json::Value,
}
```

## CLI Targets

```bash
v4vmm now-playing --json
v4vmm playlist play --dry-run
```

## Technical Rules

### PlaybackSession owns truth

Player adapters only report state.

Never let a player backend define canonical metadata.

---

# Phase 3 — Relay Contract First

## Goal

Define the protocol before building the full relay.

## Minimal HTTP API

### Private Write API

```text
POST /api/v1/sessions/:id/now-playing
```

### Public Read API

```text
GET /api/v1/sessions/:id/current
GET /health
```

## First Development Flow

```text
playlist row
→ NowPlayingUpdate JSON
→ local file
→ local HTTP relay
→ remote relay POST
→ public GET current
```

## Technical Rules

Do not build full Socket.IO first.

Build the JSON contract first.

---

# Phase 4 — Rust Remote Relay

## Goal

Build a small stable public service.

## Stack

### Rust

Use:

- axum
- tokio
- serde
- tracing
- tower-http
- reqwest
- sqlx OR rusqlite

### Database

Start with:

```text
SQLite
```

Upgrade later only if necessary.

## Service Responsibilities

- authenticated writes
- public reads
- current session storage
- recent history
- optional metrics
- optional rate limiting

## Project Layout

```text
v4v-live-relay/
  crates/
    live-types/
    relay-server/
    icecast-pusher/
    socketio-bridge/
```

## Technical Rules

Avoid premature Postgres/Redis complexity.

SQLite first.

---

# Phase 5 — VTS Resolver

## Goal

Resolve the correct active `podcast:value` block.

## Inputs

```text
feed_guid
item_guid
position_ms
```

## Output

```text
resolved value block
```

## Resolution Logic

```text
track match
AND
split start <= position < split end
```

## Priority Rules

### Identity order

```text
1. feed_guid + item_guid
2. local file path
3. embedded ID3 TXXX GUIDs
4. musicL identifiers
5. artist/title fuzzy match
```

Never use fuzzy match as primary identity.

---

# Phase 6 — Realtime Delivery

## Goal

Support podcast app live updates.

## Required

### HTTP fallback

```text
GET /current
```

### Push updates

Choose:

- WebSocket
- SSE
- Socket.IO compatibility

## Why

Mobile apps may miss socket events.

HTTP fallback is mandatory.

## Technical Rules

Socket push is optimization.

HTTP current state is required.

---

# Phase 7 — Icecast Metadata Push

## Goal

Push cosmetic metadata to Icecast.

## Rules

Icecast is never authoritative.

Only push:

```text
Artist - Title
```

Maybe:

```text
Artist — Track
Album
```

No VTS authority should rely on Icecast.

## Config Example

```toml
[icecast]
enabled = true
base_url = "https://radio.example.com"
mount = "/stream.mp3"
format = "{artist} - {title}"
```

---

# Optional Phase 8 — Player Adapters

## Initial Adapters

### First

```text
mpv IPC
```

### Then

```text
MPRIS
```

### Later

```text
Liquidsoap monitor
internal player
```

## Technical Rules

Do not build the internal player first.

The value is metadata orchestration—not audio decoding.

---

# Development Guidelines

---

# Guideline 1

## Small Vertical Slices

Always build complete end-to-end slices.

Example:

```text
playlist row
→ now-playing JSON
→ relay POST
→ public GET
```

Do not build giant abstractions first.

---

# Guideline 2

## Stable DB Before Fancy UI

Schema stability matters more than UI speed.

Prefer:

```text
simple schema
extra_json escape hatch
explicit migrations
```

---

# Guideline 3

## Provenance Matters

Never silently overwrite metadata.

Track source:

- RSS
- MusicIndex
- embedded ID3
- MusicBrainz
- manual override

Always preserve origin.

---

# Guideline 4

## Linux First

Primary environment:

- Arch Linux
- mpv
- Clementine
- Mixxx
- JACK/PipeWire/Pulse bridges
- MPRIS

Do not optimize for Windows/macOS yet.

---

# Guideline 5

## Avoid Rewrites

Use:

- VISION.md
- ARCH_NOTES.md
- migration discipline
- shared Rust types

Do not let transport details define architecture.

---

# Final Principle

## v4vmm is not a player.

It is:

```text
music-first
metadata-first
V4V-first
playback-session-first
```

The player is replaceable.

The metadata engine is the product.

