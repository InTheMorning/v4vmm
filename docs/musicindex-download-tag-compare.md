# MusicIndex Download and Tag Compare

## Goal

Build a workflow for selecting a MusicIndex track, downloading its audio file, reading the file's embedded metadata, and comparing those fields side by side against the source facts exposed by the MusicIndex API.

This is an operator/debugging workflow first. It should surface differences clearly and preserve provenance. It must not infer metadata, overwrite tags, or discard source data unless a later ADR and phase plan explicitly approves that behavior.

## Current Starting Point

- `cargo run` launches the GPUI MusicIndex search UI as the `v4vmm` binary.
- `src/search.rs` owns the MusicIndex client models, search flow, track inspector, feed drill-downs, contributors, and value-route loading.
- Track detail responses already include primary `enclosure_url` fields.
- The MusicIndex `/v1/tracks/{guid}` endpoint supports `include=source_enclosures`, which can expose primary and alternate enclosure choices.
- `src/audio_tags.rs` reads basic MP3 ID3 metadata and custom `TXXX` frames for the desktop workflow.

## Implemented Flow

1. The operator searches for a track in the existing MusicIndex GUI.
2. The operator selects a track result.
3. The track inspector offers a download-and-compare action when an MP3 enclosure is available.
4. The app downloads the selected MP3 to a deterministic local staging path under the configured music directory.
5. The app reads embedded MP3 metadata from the downloaded file.
6. The inspector renders a side-by-side comparison table:

   | Field | MusicIndex | File Tag | Status |
   | --- | --- | --- | --- |
   | Title | Track title | ID3 title | match/diff/missing |
   | Artist | Track artist | ID3 artist | match/diff/missing |
   | Album/Feed | Feed title | ID3 album | match/diff/missing |
   | Track # | Track number | ID3 track | match/diff/missing |
   | Publisher | Publisher text | Custom tag if present | match/diff/missing |
   | Nostr handle | Track-level source ID | ID3 tag if present | match/diff/missing |
   | Website | Track-level source link | ID3 tag if present | match/diff/missing |
   | Release pubdate | Track pubdate/release claim | ID3 tag if present | match/diff/missing |

7. If feed-level nostr handle, website, or release pubdate differs from the item-level value, the feed-level value is shown as its own row.
8. The track inspector keeps MusicIndex/RSS data and actions on the left. When compare is active, the file-side panel opens on the right with embedded artwork and all ID3 frames found in the file.

## Metadata Boundary

The feature should be MP3-only for the first implementation, but the public boundary should be format-neutral:

```rust
struct AudioTags {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    track_number: Option<String>,
    date: Option<String>,
    custom: BTreeMap<String, String>,
}
```

Use names like `audio_tags` and "embedded metadata" in new workflow code. Keep ID3-specific behavior inside the MP3 reader. This leaves a clean path to add FLAC/MP4/OGG support later without rewriting the GUI comparison layer.

## Enclosure Selection

For the first implementation:

- Fetch track details with `include=source_enclosures`.
- Prefer an enclosure with MIME type `audio/mpeg`.
- Fall back to a URL ending in `.mp3` when MIME type is absent.
- If no MP3 enclosure is available, show a clear status such as `No MP3 enclosure available`.
- Do not auto-select FLAC or other formats yet.

## Comparison Rules

- Compare normalized display strings only.
- Treat empty and missing values as missing.
- Render missing embedded MP3 tags as blank tag-side fields, not as a hard workflow error.
- Show differing feed-level source values separately from item-level values.
- Do not infer values from filenames, URLs, feed titles, or contributors.
- Do not mutate file tags.
- Surface source conflicts as differences rather than resolving them.

## Phase Status

### Phase 1: Format-Neutral Metadata Boundary

Status: complete.

- Added an `AudioTags` shape and MP3-backed tag reader.
- Kept MP3 tag reading available behind the format-neutral `AudioTags` boundary.
- Added focused tests for tag normalization.

### Phase 2: Download and Compare Core

Status: complete.

- Added a downloader helper that stores selected MP3 enclosures under the configured music directory.
- Added a comparison helper that maps MusicIndex track fields and `AudioTags` into comparison rows.
- Kept this layer independent of GPUI rendering.

### Phase 3: GUI Integration

Status: complete.

- Extended the track inspector with download-and-compare state.
- Fetches track details with `source_enclosures`, `source_links`, `source_ids`, `source_release_claims`, `source_contributors`, and `payment_routes`.
- Renders download status and comparison rows in a right-side file panel.
- Renders RSS data and actions on the left, including contributors and wallet routes when compare is active.
- Renders embedded file artwork and all ID3 frames.

### Phase 4: Later Formats

Status: deferred.

- Replace or extend the MP3-specific reader with a multi-format tag backend only when needed.
- Add FLAC/MP4/OGG enclosure selection rules in a separate ADR or phase plan if the behavior affects operator expectations.

## Non-Goals

- No automatic tag rewriting.
- No local library deduplication.
- No inference from filenames or URL paths.
- No batch download mode.
- No FLAC or MP4 tag support in the first implementation.
