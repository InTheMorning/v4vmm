# ADR 0020: Simulated Playlist Playback

## Status

Accepted - 2026-04-27.

## Context

Live relay testing needs now-playing changes that behave like transport controls,
but v4vmm does not yet own a real audio player adapter. Phase 2 already stores an
authoritative playback session with playlist identity, playlist position, current
track, position, and state.

## Decision

Use the playback session as a simulated transport. `playlist play` persists a
playlist track into the default playback session without launching audio.
`playback next` and `playback previous` move within the stored playlist, and
`playback stop` keeps stopping the default session.

## Consequences

- Relay smoke tests can generate realistic now-playing changes from real library
  tracks and value blocks.
- No external music player is required.
- Skip commands require the current session to have playlist context; manually
  setting a loose track remains possible, but it cannot be skipped as a playlist.
