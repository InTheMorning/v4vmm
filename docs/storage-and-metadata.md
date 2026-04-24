# Storage And Metadata Model

## Database Tables

The local SQLite database is the app's durable state.

### `feeds`

Stores feed identity and feed-level source metadata, including:

- RSS URL
- podcast GUID when present
- title, link, language, description
- artwork reference
- people/value blocks serialized from RSS
- subscription state
- last fetched time
- latest known `musicindex_updated_at`

### `tracks`

Stores feed items keyed by `(feed_id, item_guid)` with fields such as:

- enclosure URL and type
- title and artist strings from RSS
- album/feed title
- track number and duration
- per-item people/value blocks
- local `is_in_library` state
- extra JSON for fields such as transcript URL

### `local_files`

Stores the file system link between a downloaded path and a track row, plus size and bookkeeping metadata.

That separation lets the app keep feed history and track metadata even when a local file has been deleted.

## File Layout

Downloaded audio is written under the configured music root:

```text
<music_dir>/artists/<artist>/<feed-or-release>/<filename>
```

Path segments are sanitized to avoid invalid characters, reserved names, and runaway length.

When a feed lies about the enclosure type, the download layer detects the actual format from file bytes and renames the file to the matching extension.

## Enclosure Selection

The library download path uses a simple selection policy:

1. prefer a primary source enclosure when present
2. otherwise use the first supported source enclosure
3. otherwise fall back to the track's direct enclosure URL
4. infer format from MIME type or URL extension, then verify from file bytes after download

Supported format classification today:

- MP3
- FLAC
- M4A/MP4
- OGG Vorbis
- Opus
- WAV

## Metadata Sources

The app keeps several metadata layers visible at once.

### RSS

RSS remains the local source-of-record for subscription import.

The importer pulls:

- core channel and item fields
- podcasting 2.0 extensions such as `guid`, `medium`, `value`, `person`, and `transcript`
- iTunes fields such as item duration and artwork

### MusicIndex

MusicIndex is used for discovery and for richer detail hydration:

- search results
- feed and track detail
- source links, source ids, release claims, contributors, and payment routes
- remote `updated_at` timestamps used for feed freshness checks

### Embedded Tags

The compare and edit path currently centers on ID3:

- files are read through the ID3-backed `AudioTags` representation
- embedded artwork and raw frame listings are surfaced in the UI
- applying changes writes explicit ID3v2.4 frames only

This makes MP3 the strongest path today. Other downloaded formats may be stored locally, but the edit surface is not yet symmetric across all audio containers.

### MusicBrainz

MusicBrainz is an optional enrichment layer for local files.

- lookups are query-based and use local metadata such as title, artist, album, track number, duration, and ISRC when present
- release details are fetched to improve candidate quality
- resulting values are shown as candidate metadata, then staged into ID3 edits only when the operator applies them

## Provenance Rules

The app follows a source-preserving model instead of silently normalizing everything into one inferred answer.

In practice that means:

- RSS, ID3, and MusicBrainz values stay visible as separate columns
- conflicts are shown as conflicts
- generated edits are explicit and reviewable
- duplicate writes to the same effective ID3 target are blocked until resolved

## Current Limits

- search-side download and compare logic still follows an MP3-oriented path
- embedded metadata editing is ID3v2.4-only
- there is no fingerprint lookup or automatic recording identification from audio content
- file deletion is explicit from the library UI; unsubscribe alone does not remove files from disk
