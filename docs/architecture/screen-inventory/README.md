# Screen Inventory - 2026-10-03

## Status

Current - 2026-10-03. This inventory is advisory. It states no rule.
It is the map for Phase 2 of the [overhaul plan](../../plans/design-and-cleanup-overhaul-plan.md): the design language and the HTML mockups.

## Files

| File | Scope |
|---|---|
| [music.md](music.md) | 15 screens: toolbar search, search results, name matches, Index feed and track pages, Library list and tiles, Library album and track pages, playlist page, publisher page, artist page, playlist popover, confirmation dialogs, tag update popup |
| [show.md](show.md) | 8 surfaces: Show dashboard, the Source, Live Metadata, Stream and Cuelist side panels, log pane, transport deck, live status strip |
| [settings.md](settings.md) | 10 screens: General, Library, the Diagnostics pages, startup recovery, session drain, shared log frame |
| [design-system.md](design-system.md) | The v4vmm tokens, themes, fonts, primitives, composites and artwork sites, the website tokens of `../musicindex/search.html`, and a mapping table with the gaps |
| [screenshots-2026-10-03.md](screenshots-2026-10-03.md) | The orchestrator's notes from nine operator screenshots, in Dark at normal width |

Subagents wrote the four inventory files from the code. They read no running app. The screenshot notes describe the running app.

## Main Findings

### Look

- The app has one accent color, a blue violet, for tabs, buttons, links and chips. The website has one blue accent and a separate color for each entity type.
- The app reuses its status colors as entity colors. Track and playlist share one color, and publisher is red. The website keeps red for live only.
- The app has no shadow under artwork, no selection sheen and no texture. Its largest radius is 14 pixels. It loads no font of its own.
- Artwork has no consistent size: 24 to 32 pixels in rows, 80 pixels in detail headers, 152 pixels in Library tiles, and none on the publisher page header. Show has no artwork at all.

### Structure

- The Library tile view shows one tile for each track, not for each album.
- The album page and the track page show the inspection data at all times: feed URL, GUID, and the full compare grid with raw frame IDs.
- The Library uses a sidebar and a content pane. The search flow uses one full-width pane.
- The track page stacks its buttons unevenly. The album page mixes outline and filled buttons in one row.
- A permanent accent border surrounds the content area.

### Settings

- `ui_scale`, `theme_profile` and `musicindex_endpoint` each have a normal control and a raw field in Configuration repair.
- `db_path`, `playback.driver`, `playback.mpv_path` and each `broadcast.*` key have only the raw repair editor.
- The five reports (startup, session, configuration, database, background) share one log frame. Each must stay reachable.

### Show

- The transport deck has three icon buttons and no position, time or volume. The mpv driver already reports the position on each poll.
- The Cuelist is a read-only track list. ADR 0068 is Proposed.
- The dashboard state follows local playback, and its cards follow the broadcast chain.
- The previous and next transport icons render as orange boxes in the screenshots.

## Defects Found

These are defects in today's code. The inventory found them and did not correct them.

- **Use Defaults clears the converter path.** `use_default_settings` in `src/app/settings.rs` sets `flac_path` to an empty value and saves. No screen shows `flac_path`.
- **Save writes two values that no screen shows.** `save_settings` in `src/app.rs` reads `music_dir_input` and `flac_path_input`, which no screen renders.
- **Destructive actions are marked in different ways.** "Delete playlist" has no confirmation. "Remove from all playlists" has one. "Write Tags" is coded as not destructive.
- **Two code paths no screen reaches:** a second artist view and shell pair, and a discarded "Remove Feed" action on the Index feed and track pages (see [music.md](music.md)).
- **The live status strip never shows a real health value.** Its health and recording badges stay Unknown in code.
