# V4V Music Manager

`v4vmm` is a Linux-first GPUI desktop app for working with MusicIndex-backed music feeds and a local managed audio library.

The current tool is not a generic roadmap project or a future media player shell. It is a desktop operator app that already does four concrete jobs:

- discover artists, feeds, and tracks from MusicIndex
- subscribe feeds by pulling their RSS into a local SQLite database
- download track files into a managed library under `music_dir/artists/...`
- compare and update embedded metadata on local files, with optional MusicBrainz-assisted staging

## What The App Does Today

- `Discover` searches MusicIndex and shows recent feeds when the search box is empty.
- Feed and track inspectors can subscribe or unsubscribe content against the local database.
- Subscribing a feed stores its RSS metadata locally and attempts to download each track.
- Subscribing a track stores the parent feed if needed, downloads the file, and marks that track as part of the managed library.
- `Library` shows subscribed tracks grouped by artist and release/feed.
- `Cached` shows downloaded files that exist locally but are not currently marked as subscribed library tracks.
- Local track inspectors can compare embedded ID3 data against RSS-derived metadata and stage explicit ID3v2.4 edits.
- Local track and album inspectors can run metadata-based MusicBrainz lookups and use the results to stage more ID3 edits.
- The library can check subscribed feeds for newer `updated_at` values from MusicIndex and apply tag refreshes to downloaded files.

## Current Scope

The app is built around four data sources:

- RSS feed data imported into SQLite
- MusicIndex API detail data
- embedded file metadata, currently centered on ID3 workflows
- MusicBrainz metadata lookups for local enrichment

It is not currently a full playback app, a fingerprint scanner, or a general-purpose tag editor.

## Run

```bash
cargo run
```

### Optional dependency: `flac`

When a feed declares MP3 but actually ships WAV (a known RSS quirk), v4vmm
silently re-encodes the download to FLAC so it can be tagged. This path shells
out to the standard `flac` CLI. Install it via your package manager:

- Debian/Ubuntu: `sudo apt install flac`
- Fedora: `sudo dnf install flac`
- Arch: `sudo pacman -S flac`
- macOS (Homebrew): `brew install flac`

Without `flac` installed, WAV downloads are kept as WAV and left untagged. A
custom binary path can be set under `Settings → flac binary` or via the
`flac_path` config key.

Build the desktop binary:

```bash
cargo build
```

Install it locally:

```bash
cargo install --path .
```

## Configuration

On first run the app creates a config file, then ensures the required directories exist.

Default locations on Linux-style systems:

```toml
# ~/.config/v4vmm/config.toml
music_dir = "/home/<user>/V4Vmusic"
db_path = "/home/<user>/.local/share/v4vmm/v4vmm.sqlite"
musicindex_endpoint = "https://api.musicindex.org"
```

The `Settings` tab lets you update:

- `musicindex_endpoint`
- `music_dir`
- `flac_path` (optional override; blank means use `flac` from `$PATH`)

Downloads are organized under:

```text
<music_dir>/artists/<artist>/<release-or-feed>/<track file>
```

The app also keeps a thumbnail cache beside the config directory.

## Storage Model

The SQLite database tracks three related states:

- `feeds`: imported RSS feeds and subscription state
- `tracks`: feed items and whether each track is part of the managed library
- `local_files`: downloaded files on disk

That gives the UI two useful views:

- `Library`: downloaded tracks that are still marked `is_in_library = 1`
- `Cached`: downloaded files whose tracks exist locally but are currently unsubscribed

## Metadata Behavior

The app is provenance-first. It keeps RSS, MusicIndex, embedded tags, and MusicBrainz values separate instead of silently collapsing them into one inferred truth.

Current behavior:

- RSS import stores source feed and item metadata locally.
- RSS enrichment can pull transcript URLs and nostr handles directly from feed XML when MusicIndex detail data is missing them.
- Local compare views line up RSS, ID3, and optionally MusicBrainz values side by side.
- Applying tag changes writes explicit ID3v2.4 frames only.
- MusicBrainz lookup is metadata-based. There is no acoustic fingerprinting flow in this app.

Current limitations:

- embedded metadata inspection and editing are centered on ID3-backed files
- the library download layer recognizes more than MP3, but the richest compare/edit flows are still MP3/ID3-first
- search-side download/compare code still follows an MP3-oriented path

## Keyboard Shortcuts

Global shortcuts wired in the app today:

- `Cmd/Ctrl+1`: switch to `Library`
- `Cmd/Ctrl+2`: switch to `Discover`
- `Cmd/Ctrl+3`: switch to `Settings`
- `Cmd/Ctrl+F`: focus the active tab search field in `Library` or `Discover`
- `Cmd/Ctrl+R`: refresh `Library`
- `Esc`: close the active inspector

## Docs

- [Docs index](docs/README.md)
- [App overview](docs/app-overview.md)
- [Library and discovery workflows](docs/workflows.md)
- [Storage and metadata model](docs/storage-and-metadata.md)
- ADRs remain in [`docs/adr/`](docs/adr/)
