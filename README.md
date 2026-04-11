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

# Usage

```
mkdir -p ~/.cargo/bin
cd rust
cargo build
cargo install --path .
```
This creates the binary in `~/.cargo/bin` so you can add that folder to your `$PATH` or run `~/.cargo/bin/v4vmm` directly

So far it is a CLI tool:
```
Usage:
  v4vmm show-config
  v4vmm id3-dump <path-to-mp3>
  v4vmm subscribe <feed-url>
  v4vmm rss-dump <feed-url>
```

Running the command will create config file `~/.config/v4vmm/config.toml`
This file contains defaults:
```
# V4V-only library root
music_dir = "/home/<username>/V4VMusic"

# SQLite database path (app data)
db_path = "/home/<username>/.local/share/v4vmm/v4vmm.sqlite"
```

For now it only supports subcribing to a feed which adds its relevant RSS data to our database along with additional fields for eventual id3 tag info and toggling as *in our library*.

It can also print id3 data from any local (mp3-only, for now) file, and for development purposes print what our rss library sees when it looks at a feed.

# Project discipline

- Architecture decisions live in `docs/adr/`
- Rust regression and integration tests live in `rust/tests/`
