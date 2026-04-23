# V4V Music Manager

## V4V Music Manager is a small tool that acts as *glue* between RSS-based music feeds (e.g., DeMu) and a local audio file library.

The Rust core will:

- Subscribe to multiple music feeds and store their metadata in a database
- Flag individual tracks and whole albums as **in the library**
- Download audio files into a target directory
- Overwrite / normalize ID3 tags so regular music players display RSS (**source of truth**) titles, artwork, and metadata

## Eventual UI

A GUI (possibly written in Qt) will provide:

- A browser for V4V music
- Controls to flag/unflag items and execute downloads, deletions, and metadata updates
- Some form of local playlist management

## Eventual music player integration (Mixxx, Clementine, etc.)

Your music player will:

- Use **MPRIS / D-Bus** to communicate back to the UI

## Eventual Splitkit™ functionality

Planned features:

- Generate VTS for live RSS streaming, chapter art, and a ready-to-go podcast episode
- Support arbitrary-length talk breaks between songs

# Build and Run

```
cargo build
cargo run
```

To install the desktop app binary:

```
cargo install --path .
v4vmm
```

## MusicIndex UI

The `v4vmm` binary is V4V Music Manager, a GPUI desktop app for local library management and MusicIndex discovery. It searches MusicIndex feeds, tracks, and publishers, shows compact results on the left, and opens the selected result in a right-side inspector with feed tracks, track/feed drill-downs, contributors, value routes, and MP3 embedded-metadata comparison.

The UI uses the configured MusicIndex endpoint and needs network access.

For track results, use `Download + Compare` in the inspector to fetch the MP3 enclosure into `music_dir/artists`, read its embedded MP3 tags, and compare title, artist, album/feed, track number, publisher, nostr handle, website, and release pubdate fields against MusicIndex source facts. The inspector keeps MusicIndex/RSS data and actions on the left and opens file-side ID3 details on the right. It also shows embedded artwork and all ID3 frames found in the file. Missing ID3 tags render as blank fields. This is read-only: it does not rewrite tags.

Running the app will create config file `~/.config/v4vmm/config.toml`.
This file contains defaults:
```
# V4V-only library root
music_dir = "/home/<username>/V4Vmusic"

# SQLite database path (app data)
db_path = "/home/<username>/.local/share/v4vmm/v4vmm.sqlite"
```

The app can subscribe to a feed, add its relevant RSS data to the local database, download tracks, compare embedded metadata, and toggle tracks as *in our library*.

# Project discipline

- Architecture decisions live in `docs/adr/`
- Rust regression and integration tests live in `tests/`
