# Library And Discovery Workflows

## Discover Workflow

The `Discover` tab is the entry point for browsing MusicIndex.

### Search And Inspect

- Enter a query to search MusicIndex entities.
- Leave the query empty to browse recent feeds.
- Open inspectors for artists, feeds, tracks, and publishers linked from feeds or tracks.
- Feed and track inspectors surface source metadata, contributors, and value routes when available.

### Subscribe A Feed

Subscribing a feed does more than flip a local flag.

The app will:

1. fetch the RSS feed directly
2. upsert feed-level metadata into SQLite
3. upsert track rows for feed items with stable GUIDs
4. try to hydrate track details from MusicIndex where track GUIDs exist
5. enrich track metadata from raw RSS for transcript and nostr-related fields
6. download each supported track file into the managed music directory
7. write generated ID3v2.4 edits when the target file supports that path
8. mark downloaded tracks as part of the local library

If a track cannot be downloaded or matched back into the database, the subscription message reports it as skipped rather than pretending it succeeded.

### Subscribe A Track

Subscribing a single track is a narrower version of the same flow:

1. ensure the parent feed has been imported from RSS
2. download the track file
3. apply any prepared ID3 edits
4. mark the matching database track as downloaded and in-library

Track subscription is the fastest path from discovery to a managed local file.

### Unsubscribe In Discover

- Unsubscribing a feed clears the local subscribed state for that feed.
- Unsubscribing a track clears the local in-library state for that track.
- Unsubscribe does not automatically delete files from disk.

## Library Workflow

The `Library` tab is the local operator surface.

### Browse Managed Tracks

- Tracks are grouped by artist and then by release/feed.
- Album inspectors show per-track actions plus album-level MusicBrainz staging.
- Track inspectors show local-file-aware actions once a file exists on disk.

### Compare Embedded Metadata

On a downloaded track you can open the compare view to inspect:

- RSS-derived values
- embedded ID3 values from the local file
- optional MusicBrainz candidate values

The compare grid is provenance-first. It surfaces differences rather than collapsing them into one guessed answer.

### Stage And Apply ID3 Edits

The local track inspector can stage edits from the compare grid.

- auto-staged edits come from the aligned metadata rows
- manual staging can happen by drag-copying source values onto writable ID3 targets
- duplicate effective targets are treated as conflicts and must be resolved before apply
- apply writes explicit ID3v2.4 frames to the local file, then re-reads the file for verification

### MusicBrainz Lookup

MusicBrainz lookup is available from local downloaded content.

- single-track lookup uses local tags plus track context to build a metadata query
- album-level lookup stages candidate-based edits across multiple downloaded tracks
- results are presented as candidates, not silently applied truths

This is metadata matching, not fingerprinting.

### Cached View

The `Cached` sub-tab lists downloaded files whose tracks are not currently marked as subscribed library items.

Use it to:

- inspect cached-but-unsubscribed tracks
- delete individual cached files
- delete all cached files in one action

Deleting from `Cached` removes the file from disk and deletes its `local_files` entry. It does not remove the feed or track rows themselves.

## Feed Refresh Workflow

The library can refresh already-subscribed feeds without re-importing everything from scratch.

### Check Feeds

`Check all feeds` compares each subscribed feed's stored `musicindex_updated_at` value with the current MusicIndex feed detail.

Feeds with newer remote timestamps are staged as stale.

### Apply Updates

`Apply updates` walks downloaded library tracks for those stale feeds and:

1. fetches fresh track and feed detail from MusicIndex
2. regenerates ID3 edits from the current metadata view
3. writes any new edits to local files
4. records the new `musicindex_updated_at` value in SQLite

The status line reports:

- number of tracks updated
- number of edits written
- any file-level ID3 write failures
- any feed-level failures

## Playback Session CLI Workflow

The Phase 2 CLI prepares the now-playing contract without introducing a player
backend. `PlaybackSession` is the authoritative state; player adapters added
later should report position and state into this model rather than define
metadata identity themselves.

### Preview A Playlist Row

Use dry-run to inspect the JSON that a playlist row would produce:

```bash
v4vmm playlist play --dry-run <playlist-id>
v4vmm playlist play --dry-run <playlist-id> --position <zero-based-position>
```

Dry-run validates that the selected track has a local file binding and a feed
GUID. It prints `NowPlayingUpdate` JSON with `sequence` set to `0` and does not
write `playback_sessions`.

### Set And Update The Current Session

Use playback commands when a track should become the current session state:

```bash
v4vmm playback set-track <track-id>
v4vmm playback position <ms>
v4vmm playback stop
v4vmm now-playing --json
```

`set-track` validates the same source facts as dry-run, then persists the
default playback session. `position` updates the current position in
milliseconds. `stop` marks the session stopped; after stop,
`now-playing --json` reports no current playback session.

Every persisted state change increments the session `sequence`.

## CLI Debug Workflow

The CLI exposes structured JSON inspection commands for backend state. These
commands are intended for debugging, scripts, and future UI contract checks.

```bash
v4vmm playlists list --json
v4vmm playlist tracks <playlist-id> --json
v4vmm library tracks --json
v4vmm track inspect <track-id> --json
```

`track inspect` uses the canonical local track identity service. It requires a
local file binding and feed GUID, matching the now-playing identity rules.

## State Model

The UI uses three related states that are worth keeping distinct:

- `subscribed feed`: the feed is tracked locally and participates in feed refresh checks
- `in library`: a track is part of the managed library view
- `cached file`: a local file exists on disk, whether or not the track is still marked in-library
- `playback session`: the CLI-visible canonical now-playing state derived from local track/feed facts
