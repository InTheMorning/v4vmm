# App Overview

`v4vmm` is a desktop app built with GPUI. It combines four systems:

- a local SQLite database
- MusicIndex HTTP APIs
- direct RSS fetch and parse for subscriptions and enrichment
- local file download plus embedded metadata inspection

It also exposes a non-UI CLI for playback-session work. The CLI assembles and
updates canonical now-playing state from local database facts. The app plays
audio through the mpv driver of ADR 0021. One-shot CLI commands stay in
session-only mode and start no player.

## Main Tabs

### Library

This is the managed-library view.

- Shows tracks marked as part of the local library.
- Groups them by artist, then by release/feed.
- Lets you open album and track inspectors.
- Can compare local files against RSS-derived metadata.
- Can run MusicBrainz lookups against downloaded files.
- Can check subscribed feeds for newer MusicIndex updates and apply tag refreshes.

There is also a `Cached` sub-view for downloaded files that still exist locally.
Those files are no longer marked as subscribed library tracks.

### Index Browsing

`Discover` is no longer a separate tab. ADR 0047 and ADR 0048 moved MusicIndex
browsing into the toolbar search and the `ContentList` frame of the Library
workspace. The toolbar search covers Library rows and Index rows together.

- Searches across artists, feeds, and tracks.
- Shows recent feeds when there is no active query.
- Opens inspectors for artists, feeds, tracks, and linked publishers.
- Can subscribe or unsubscribe feeds and tracks from the Index side.
- Uses MusicIndex detail responses for contributors, value routes, source links, source ids, release claims, and enclosure choices.

The former dedicated Discover modules stay in the tree but the composition root
does not reach them. See `docs/notes/2026-05-discover-module-parked.md`.

### Settings

The settings screen currently manages only the values that matter to the app runtime:

- `MusicIndex endpoint`
- `Music directory`

Saving settings updates the app state immediately and persists the values in `config.toml`.

## CLI Surface

Running `v4vmm` with no arguments starts the desktop UI.

Phase 2 now-playing commands use the configured local SQLite database and the
default playback session:

```bash
v4vmm now-playing --json
v4vmm playlists list --json
v4vmm playlist tracks <playlist-id> --json
v4vmm library tracks --json
v4vmm track inspect <track-id> --json
v4vmm playlist play --dry-run <playlist-id>
v4vmm playlist play --dry-run <playlist-id> --position <zero-based-position>
v4vmm playback set-track <track-id>
v4vmm playback position <ms>
v4vmm playback stop
```

`playlist play --dry-run` is a preview command. It validates the selected
playlist row and prints the `NowPlayingUpdate` JSON without mutating playback
session state.

## Runtime Dependencies

The current app expects:

- network access to the configured MusicIndex endpoint
- network access to RSS feed URLs for subscription and RSS-side enrichment
- optional network access to MusicBrainz and Cover Art Archive when using MusicBrainz lookup
- local filesystem access to the configured music directory and SQLite database path

## First Run

On startup the app:

1. creates `config.toml` if it does not exist
2. ensures the configured music and data directories exist
3. opens or initializes the SQLite database
4. opens the main desktop window

Default Linux-style paths:

- config: `~/.config/v4vmm/config.toml`
- database: `~/.local/share/v4vmm/v4vmm.sqlite`
- managed music root: `~/V4Vmusic`

## Audio Scope

The download layer can classify and normalize several enclosure types:

- MP3
- FLAC
- M4A/MP4
- OGG Vorbis
- Opus
- WAV

The metadata workflows are narrower than the download layer:

- embedded tag read and write paths are currently ID3-focused
- WAV is detected for download purposes but is not treated as a taggable target
- the richest local compare and editing workflows are therefore best on MP3/ID3 files

## Broadcast

The app does not send live metadata to the MusicIndex relay.
`musicindex-live-publisher` does that, as a separate package that can run on a
different machine. ADR 0059 makes this app the control surface and the status
display for that chain.

See `docs/architecture/broadcast-chain.md` for the components and the contract
at each boundary.

## Current Non-Goals

These are outside the current tool boundary:

- live metadata sends to the relay
- acoustic fingerprinting or Picard-style scan workflows
- broad multi-format embedded tag editing parity
- audio routing, mixer control, and stream encoding
- hidden metadata inference from filenames or guessed album structure
