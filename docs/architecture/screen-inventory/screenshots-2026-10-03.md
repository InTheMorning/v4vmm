# Operator screenshots 2026-10-03 (Dark, normal width) — orchestrator observations

Global
- One accent: lavender/periwinkle (~#8a8cff) for tabs, primary buttons, links, outline buttons, selected chips. No orange, no second accent. Status badges: green (Reachable/Connected/Active), red (Not ready/Dead/Failed), yellow ("feed" badge).
- A lavender 1px frame border surrounds the whole content area at all times (looks like a permanent focus ring).
- Toolbar: logo, Music/Show/Settings tabs (selected = filled lavender pill), centered "Search Library and Index" field, filled "Search" button.
- "Live status: HeyCitizen - To be in Nashville / Paused · Health unknown · Open Show" band shows on Music and Settings (stuck single song). Takes ~70 px of height.
- Typography: one sans family, small sizes, low contrast gray secondary text. Dense.

Library tiles
- Grid of TRACKS, not albums: six identical Delta OG covers in a row. Each tile: 150 px square art, title, artist, "Track" badge + "In library" text.
- Left sidebar: "64 library tracks", Check all feeds / Update n files buttons, a status line, Playlists tree (playlist → album → tracks with 24 px art). Sidebar duplicates the album/track list.

Album page (Library)
- 80 px art top-left, yellow "feed" badge, title, artist link.
- Button row mixes outline (Remove Feed, MusicBrainz, Open publisher) and filled (Add feed to playlist ▾) styles; Website and RSS links far right.
- Metadata table (Release Kind "unknown", Published (RSS), First track published (MusicIndex), Tracks, Duration, Language) always shown.
- Description in a bordered box with "hide".
- Website, Feed URL, GUID raw rows always shown.
- Track rows: 32 px art, title, duration, red "Remove" text, "+ Playlist" button.

Track page (Library)
- Two columns: left RSS (art, "track" badge, title, artist, buttons stacked unevenly: Remove Track, Add to playlist, Open publisher, Hide Compare, MusicBrainz), right MP3 file (same art, "Embedded MP3" badge, Re-read, Re-download, title, truncated absolute path).
- Release, Duration, Release Date rows; Description box; feed box with Website link.
- Compare grid expanded by default under it: grouped sections (URL link frames, Lyrics/comments/artwork, Identity/linking, Timing, Music-disc/commerce, Other metadata) with raw frame IDs (WOAR, APIC, SYLT:MusicIndex Transcript, COMM:iTunNORM, TXXX:MusicIndex Value Routes), green "=" equality marks, purple frame IDs.

Settings
- "Settings" title + group selector shown as small dropdown links ("v General", "v Library", "v Diagnostics  v Configuration").
- General: UI scale XS-XL chips, Theme chips (System, Dark, Light, High Contrast Dark, High Contrast Light), explanatory sentences, Save + Use Defaults. Large empty space.
- Library: MusicIndex endpoint field, Music directory read-only text + instruction to use Configuration repair, Converter setup link, Save/Use Defaults, Configuration repair link.
- Configuration repair: left button column (Close editor, Test draft and paths, Save correction, Copy draft (redacted), Reload file, Copy repair report), raw config keys as chips (music_dir, db_path, musicindex_endpoint, playback.driver, broadcast.hosts, workspace.layout.content_pane_width ...), one text editor for the selected key, long instructional paragraphs.

Show
- Title (track), artist, time, Paused.
- Three status cards (Source: Reachable / Local / 64 tracks ready; Live Metadata: Not ready / Event: Dead; Stream: Connected / Listeners unknown) with large empty interiors.
- Right pane: Cuelist (one item: To be in Nashville 4:24) or a card detail (Live Metadata: Event Dead with timestamp and id, Replace/Logs/More, Producer Active mixxx-now-playing.service Start/Stop/Reset/Logs, Publisher Failed musicindex-live-publisher@mixxx.service).
- Bottom log pane (systemd lines, "Following latest entries", Go to latest).
- Transport: tiny prev/play/next at the bottom center; prev/next icons look like broken/emoji glyphs (orange boxes).
