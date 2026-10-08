# Design And Cleanup Overhaul Plan

## Status

Active - 2026-10-03. This plan is advisory. It states no rule.
Each binding change in it needs its own ADR or ADR amendment before code changes.

## Goal

- The app looks like the musicindex.org website: purposeful color and artwork.
- The Library is a browsable music library that feels like a music player. The RSS and ID3 values stay available for inspection.
- Settings has one clear place for each setting. The verbose error reports and logs stay.
- The documents, the guards and the code are smaller, current and correct.

## Recorded Facts - 2026-10-03

- The repository has 78 current ADRs and 5 archived ADRs, 316 task documents, 61 plans and 648 Markdown files in `docs/`.
- `AGENTS.md` has 612 lines. Most lines are a dated status history of packets. Its own rule says that it describes the present.
- `docs/pending-human-checks.md` has 646 lines.
- `src/` has 172179 lines of Rust in 317 files. The largest files are `src/view_models/library.rs` (7961), `src/view_models/show.rs` (5956), `src/db.rs` (5863) and `src/metadata.rs` (4784).
- `tests/architecture_tests.rs` has 19606 lines.
- The defects of 2026-10-03 had one shape: two sources of truth. Examples are live values against stored values, two copies of the channel link, two description forms, and an error that only stderr showed.
- The website is `../musicindex/search.html`. The operator discarded `index.html` on 2026-10-03. The dark theme is the default:
  - background `#0b0b0d`, sidebar `#141417`, surfaces `#1c1c1f` and `#26262a`, text `#f5f5f7`, muted text `#a1a1a6`
  - one accent `#2d7bff`, and `#0a5bd6` in the light theme
  - one color for each entity type, for example artist, track, feed, label, playlist and live
  - gloss, a grain texture, a shadow under artwork, and a blurred top bar of 52 pixels
  - radii of 6 to 12 pixels, pills of 22 pixels, and a grid of tiles of 150 pixels or more
  - the Figtree font, in `../musicindex/assets/fonts/`, with weights 300 to 800
- Each color, size and font of the app comes from `src/ui/tokens.rs` (durable set: token discipline). A token change can change the look of the full app.
- The cue system and the built-in player are not built. The live status shows one song that does not change.
- `cargo build --release` failed in `gpui-pre-macros 0.3.1` until 2026-10-07. `[profile.release.build-override]` in `Cargo.toml` corrects it.

## Phases

### Phase 1: Documents

1. ADR triage. Parallel read-only agents classify each ADR. The operator decides each row. One commit archives each batch. [Triage record](../reviews/adr-triage-2026-10-03.md).
   Decided on 2026-10-03: 25 ADRs archived. ADR 0078 archives with ADR 0082 packet 002.
2. Remove completed task documents and completed plans from the reading path. Version control keeps them.
   Done on 2026-10-03: 265 task documents moved to `docs/tasks/archive/`, and 40 plans deleted. 51 open tasks and 22 plans stay.
3. Guard audit. Delete each guard that cites a superseded ADR. A guard of an ADR that archived because each rule is enforced stays.
4. Retire each pending check whose requirement a later decision replaced.
5. Rewrite `AGENTS.md` to the present only: what the project is, where the work stands, the philosophy and the working rules.
   Done on 2026-10-03: `AGENTS.md` went from 612 to 276 lines.

### Phase 2: Design Language

1. A read-only screen inventory: each screen, its view model, its elements and its actions.
   Done on 2026-10-03: [screen inventory](../architecture/screen-inventory/README.md).
2. Static HTML mockups of three screens with the website tokens: the Library grid, an album page with an "Inspect" disclosure for the RSS and ID3 values, and Settings. The operator reviews them in a browser.
3. A design-language ADR records the accepted mockups as the target.
   Done on 2026-10-03: [ADR 0083](../adr/0083-design-language.md), Accepted.

### Design Direction - 2026-10-03

The operator chose this direction on 2026-10-03, from the [mockup canvas](https://claude.ai/artifact/CaY8T2NUdDosRV9uoFy9bk) (private to the operator). [ADR 0083](../adr/0083-design-language.md) records it.

- Version 2 of the mockups is the target: real covers, and each album page tinted by a blurred copy of its cover, as on `search.html`.
- Browsing follows Apple Music: a Music home, an album grid, album pages with one main Play action.
- Value for value is visible: the payment split as a bar, and the show readiness of an album.
- Inspection stays one step away: an "Inspect sources" view with RSS, MusicIndex and file tags side by side.
- From Raycast, the app takes a cohesive status bar and simple icons. The app is not a keyboard-driven power tool: each action works with the mouse, and a "⋯" menu holds the secondary actions.

### Phase 3: Tokens

Three packets implement ADR 0083 Decisions 1 to 4 and 10. The operator walks the visual check of each packet right after it.

1. [Task 001](../tasks/archive/adr-0083-task-001-palette-and-color-roles.md): the palette and the entity colors. Done on 2026-10-03.
2. [Task 002](../tasks/archive/adr-0083-task-002-font-and-artwork-tokens.md): Figtree, the display size, the artwork shadow and the monogram placeholder. Done on 2026-10-03.
3. [Task 003](../tasks/archive/adr-0083-task-003-one-icon-set.md): each icon from the Lucide set. It corrects an emoji icon that Figtree caused. Done on 2026-10-07.

### Phase 4: Library

Packets for each surface: the album grid with artwork first, the album page with its track list and a play action, and the "Inspect" disclosure. Each packet has its visual check, walked right after it.

The first Phase 4 packet builds the track page once, for a Library track and an Index track. The operator decided this on 2026-10-07.

1. [ADR 0083 task 004](../tasks/archive/adr-0083-task-004-one-track-page.md): one track page, the action hierarchy, name links, credits and "Inspect sources". Done on 2026-10-07.
2. [ADR 0083 task 005](../tasks/adr-0083-task-005-album-page-header-and-actions.md): one album page header and action row for the Library and the Index, the 200 pixel cover and the cover color backdrop. Mechanical checks are complete on 2026-10-07. Its visual check is open.
3. Later packets: the inspection view of RSS, MusicIndex and file tags, the payment split bar for the track page and the album page, and row hover actions.

### Phase 5: Settings

ADR 0069 and its task 002 continue. The grouped Settings keep each report and log reachable.

### Phase 6: Player And Cue

A separate ADR, after the Library work. The playback defect of one song that does not change is a separate early packet.

### Throughout: Code Correctness

- Single source of truth audit: read-only agents find data written in two places and two paths that compute one output. They also find errors that only stderr shows, and live values where the stored value is the rule. Each finding becomes a failing test and then a fix.
- Split the largest files at their natural seams. No behavior changes in those packets.
- Add `cargo clippy --all-targets -- -D warnings` to the gate.
- No complete rewrite. Change one surface at a time and keep the gate Green.

## Visual Requirements Moved From Pending Checks - 2026-10-07

The operator decided on 2026-10-07 to stop walking checks of screens that
Phase 4 and Phase 5 rebuild. Each packet that rebuilds a listed surface names
the rules it carries in its visual acceptance criteria. A rule leaves this
list only when a person walks it on the new screen, or when an ADR retires it.

### Phase 4: Track Page

The operator walked §2 and §19 on the one track page on 2026-10-07. They left this list.


- **§22 Track Header Identities — ADR 0075 Task 022.** Owner: [task 022](../tasks/adr-0075-task-022-track-header-identities.md).
  - V1: an Index track without its own identities shows no website or Nostr action in its header. The feed section shows them, with the feed named as owner, in Light and Dark themes.
  - V2: a track with its own identities shows them in its header, apart from the feed section.
  - V3: a track without its own description shows no description.
  - V4: normal and narrow widths show each element in its place, with no clipped text.
- **§26 No Derived Release Date — ADR 0075 Task 049.** Owner: [task 049](../tasks/adr-0075-task-049-no-derived-release-date.md).
  - Precondition: the check only reads pages.
  - V1: a track page of a track without its own date shows no release date.
  - V2: a track with its own date shows that date.
  - V3: normal and narrow widths show each row in its place, in Light and Dark themes.
- **§27 Separate Track Artwork — ADR 0075 Task 048.** Owner: [task 048](../tasks/adr-0075-task-048-separate-track-artwork.md).
  - Precondition: the check only reads pages.
  - V1: an Index track with its own image shows that image. An Index track without one shows the album image.
  - V2: normal and narrow widths show the artwork in its place, in Light and Dark themes.

### Phase 4: Album Page

- **§3 Stored Metadata In Details — ADR 0054 Tasks 004 And 005.** Owners: [task 004](../tasks/adr-0054-task-004-feed-read-model-hydration.md) and [task 005](../tasks/adr-0054-task-005-track-read-model-hydration.md).
  - Precondition: needs known persisted feed/track metadata, reachable Index for comparison, and an unavailable endpoint for local fallback. Use the disposable config.
  - Check: [Metadata Hydration](../runbooks/inherited-ui-checks.md#metadata-hydration--adr-0054-tasks-004-and-005).
  - Record feed and track results separately in the [checklist](../reviews/adr-0054-review-checklist.md), in both themes.
- **§15 Stored Value Projection — ADR 0075 Task 020.** Owner: [task 020](../tasks/adr-0075-task-020-stored-value-projection.md).
  - Precondition: needs schema version 16 from packet 002. The check sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.
  - V1: after a check applied a title change, the album page and the track page show the new title without navigation. Check in Light and Dark themes.
  - The track page shows the channel title as the album name. A title with "(old)" after the check is wrong.
- **§30 Feed Dates By Owner — ADR 0075 Task 050.** Owner: [task 050](../tasks/adr-0075-task-050-feed-dates-by-owner.md).
  - Precondition: the check only reads pages.
  - V1: an album page shows "Published" and "First track published" as two facts, each with its source.
  - V2: a track without its own date shows "Feed publication date" apart from its other facts.
  - V3: normal and narrow widths show each fact in its place, in Light and Dark themes, with no clipped text.

### Phase 4: Library Lists, Playlists And Readiness

- **§1 Music And Settings Scrolling — ADR 0030 Task 006 (Music scrolling).** Owner: [task 006](../tasks/adr-0030-task-006-scroll-containers.md).
  - Precondition: needs overflowing artist, release, playlist, track and Index detail content.
  - Check: [Scroll Containers](../runbooks/inherited-ui-checks.md#scroll-containers--adr-0030-task-006).
  - Needs overflowing artist, release, playlist, track, Index detail, and Settings content. Verify wheel, scrollbar, and supported keyboard scrolling in Light and Dark. Missing overflowing content leaves that subcheck open.
  - The Settings portion of this check also moved. See "Phase 5: Settings" below.
- **§9 Feed Check And Feed Update Storage Failure — ADR 0075 Task 039.** Owner: [task 039](../tasks/adr-0075-task-039-feed-check-and-update-observation-retention.md).
  - Precondition: the packet supplies prospective isolated setup, failure injection, desktop inspection, and cleanup commands. Use disposable data and scripted local services.
  - Check: an ordinary feed error keeps its existing per-feed message and placement.
  - Verify that a storage failure ends only the current feed operation and claims no successful persistence.
  - Verify that **Check all feeds** keeps its existing results, controls, and placement.
- **§13 Playlist RSS Check Button And Report — ADR 0076 Task 001.** Owner: [task 001](../tasks/adr-0076-task-001-playlist-rss-document-check.md).
  - Precondition: migration 15 adds two tables. Make the SQLite backup before the first run of the new build. Each check sends real HTTP requests to the feed hosts of the playlist.
  - V1: the playlist page shows the "Check RSS" button with its state, in Light and Dark themes.
  - V2: during a check, the page shows the progress in place, without navigation. Playback from the playlist does not wait for the check.
  - V3: after the check, the page shows each feed outcome and each stopped host in readable text.
- **§14 RSS Comparison, Apply And Report — ADR 0076 Task 002.** Owner: [task 002](../tasks/adr-0076-task-002-rss-comparison-apply-and-report.md).
  - Precondition: migration 16 drops three tables. Make the SQLite backup before the first run of the new build. Each check sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.
  - V1: after a check with differences, the playlist page shows the report in place. Each row gives the feed, the field, the old value, the new value and the time. Each stale feed has a podping.me button. Check in Light and Dark themes.
  - V2: a new track shows with its download action. A removed track shows its mark on the playlist row.
  - V3: the report is readable at normal and narrow widths. Stacked text follows the column text rule.
- **§16 Stored Payment Route And Readiness — ADR 0076 Task 003.** Owner: [task 003](../tasks/adr-0076-task-003-stored-payment-route-and-readiness.md).
  - Precondition: migration 17 adds and fills two columns. Make the SQLite backup before the first run of the new build.
  - V1: the Music readiness list shows a "Route out of date" row with no button and a "Removed from feed" row with a **Confirm** button. Check in Light and Dark themes.
  - V2: the Show Source card counts both states as not ready and names them in its detail.
  - V3: **Confirm** on a removed track changes the row in place, without navigation.
  - The readiness row of a removed track also has **Remove from library**. The procedure opens its ADR 0044 confirmation and cancels it, because a removal deletes the audio file.
  - V4: a playlist row of a removed track shows a row error with **Remove from playlist** and **Remove from all playlists**. The second action lists each playlist in its confirmation. Each action changes the playlist in place.
- **§18 Check And Scan Follow-Ups — ADR 0076 Task 005.** Owner: [task 005](../tasks/adr-0076-task-005-check-and-scan-follow-ups.md).
  - Precondition: needs schema version 17 and the readiness list from packet 003. This packet adds no migration.
  - V1: the check report shows **Copy feed URL** adjacent to the **Open podping.me** button. The pasted text is the feed URL. Check in Light and Dark themes.
  - V2: after a **Confirm** in the readiness list, the playlist page shows no error on that row. No check and no restart occur between the two steps.
  - The check in V1 sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.

### Phase 4: Publisher And Artist Pages

- **§12 Library Artist View Without Artist Storage — ADR 0077 Task 001.** Owner: [task 001](../tasks/adr-0077-task-001-remove-dead-artist-storage.md).
  - Precondition: migration 13 deletes the stored artist rows. Make the SQLite backup before the first run of the new build.
  - V1: a Library artist view shows its tracks and albums, with no aliases, area, active years or source subjects.
  - A missing track, an empty view, or an error report is wrong.
- **§20 Publisher Page And Navigation — ADR 0077 Task 004.** Owner: [task 004](../tasks/adr-0077-task-004-publisher-navigation-and-presentation.md).
  - Precondition: needs a Library album whose album hydration stored a `music_to_publisher` row. V3 changes the MusicIndex endpoint in Settings. Restore it after the check.
  - V1: an album page opens its publisher page, with its title, its type and its albums, in Light and Dark themes.
  - V2: a track page opens the publisher page of its album.
  - V3: a Library publisher page shows the Library albums and the other albums as two groups. With MusicIndex unreachable, the Library group stays and the report comes first.
  - V4: a stated role and an assumed role show different text. A "Not listed" album and a derived artist count are marked.
  - V5: normal and narrow widths show each element in its place, with no clipped text.
  - The Index track page shows no "Open publisher" action. The packet records this as a deviation.
- **§21 Feed Owner Text And Name Grouping — ADR 0077 Task 005.** Owner: [task 005](../tasks/adr-0077-task-005-feed-owner-text-and-name-search.md).
  - V1: an album page shows `publisher_text` as "Feed owner" text that opens nothing, in Light and Dark themes.
  - V3: a Library artist page without a publisher relationship shows "Grouped by name".
  - V4: normal and narrow widths show each element in its place, with no clipped text.
  - V2 cannot be walked. The name search of this packet reached only parked code. Section 23 (packet 006) replaces it. Its own rule moved to "Phase 4: Search" below.
- **§28 Confirmed And Unconfirmed Artists — ADR 0077 Task 007.** Owner: [task 007](../tasks/adr-0077-task-007-confirmed-and-unconfirmed-artists.md).
  - Precondition: the check only reads pages. It needs network access to `api.musicindex.org`.
  - V1: the DETOX publisher page shows "Artists that name this feed: 1" with the name, and the unconfirmed fact with 0.
  - V2: a publisher whose listed albums do not name it shows 0 confirmed artists and its unconfirmed artists with their names.
  - V3: the two labels read clearly. The operator accepts them or gives new labels.
  - V4: normal and narrow widths show each fact in its place, in Light and Dark themes, with no clipped text.

### Phase 4: Inspect Sources

- **§8 Library Comparison And Hydration Storage Failure — ADR 0075 Task 038.** Owner: [task 038](../tasks/adr-0075-task-038-library-reader-observation-retention.md).
  - Precondition: the packet supplies prospective isolated setup, failure injection, desktop inspection, and cleanup commands.
  - Check: ordinary comparison errors stay in their panel. Ordinary hydration errors remain silent.
  - Verify that storage failures remain reported after navigation without changing selection or unrelated Library state.
- **§24 Tag Frames By Owner — ADR 0080 Task 001 (V4 only).** Owner: [task 001](../tasks/adr-0080-task-001-rss-frames-and-idempotent-writes.md).
  - Precondition: uses an isolated database copy and never writes to the Library.
  - V4: the compare grid shows the album description row with its owner, in Light and Dark themes, with no clipped text.
- **§25 MusicBrainz URL Relations By Type — ADR 0080 Task 002 (V1, V2 and V4 only).** Owner: [task 002](../tasks/adr-0080-task-002-musicbrainz-url-relations-by-type.md).
  - Precondition: uses an isolated database copy and never writes to the Library. Needs network access to `musicbrainz.org`.
  - V1: a MusicBrainz lookup shows the release-group homepage in the channel website row, and a "License" row when the release states a license.
  - V2: each other relation shows as a read-only row with its relation type, and it offers no write.
  - V4: normal and narrow widths show each row in its place, in Light and Dark themes, with no clipped text.

### Phase 4: Status Bar And Reports

- **§7 Metadata Observation Storage Failure — ADR 0075 Task 014.** Owner: [task 014](../tasks/adr-0075-task-014-provider-observation-retention.md).
  - Precondition: use disposable data and scripted local services.
  - Check: the existing Library status identifies a metadata storage failure without claiming MusicIndex unavailability or successful persistence.
  - Verify delayed failure reporting after navigation without changing the selected track's metadata or unrelated Library state.
- **§17 Tag Update Confirmation — ADR 0076 Task 004.** Owner: [task 004](../tasks/adr-0076-task-004-tag-update-confirmation.md).
  - Precondition: this packet writes audio tags. Copy the fixture audio files before the first confirm.
  - V1: the Music section shows the "Update n file(s)" button with the count. The popup lists each file with its title, album and frames. Check in Light and Dark themes.
  - V2: a file in use by the show shows the mark "In use by the show". **Write Tags** does not write it. After the write, the button count equals the files in use.
  - V3: the popup is readable at normal and narrow window widths. Stacked text clips and does not show only an ellipsis.
- **§34 Download Failures And Dismissal — ADR 0066 Task 014 (V2 only).** Owner: [task 014](../tasks/adr-0066-task-014-download-failures-and-dismissal.md).
  - Precondition: V2 needs network access and the `flac` binary.
  - V2: after a failed conversion and a successful retry of the same feed, the notice shows no row for those tracks.

### Phase 4: Search

- **§23 Name Matches Are Search Results — ADR 0077 Task 006.** Owner: [task 006](../tasks/adr-0077-task-006-name-matches-are-search-results.md).
  - V1: a search for "Survival Guide" shows a row `Tracks matching "Survival Guide"`. The row does not look like an artist identity.
  - V2: the row opens a page titled "Tracks matching" that lists the tracks of that name. A track opens its detail, and the album opens its publisher page.
  - V3: a name with no exact match shows the empty state text.
  - V4: normal and narrow widths show each element in its place, in Light and Dark themes, with no clipped text.

### Phase 5: Settings

- **§1 Music And Settings Scrolling — ADR 0030 Task 006 (Settings scrolling).** Owner: [task 006](../tasks/adr-0030-task-006-scroll-containers.md).
  - Precondition: needs overflowing Settings content.
  - Check: [Scroll Containers](../runbooks/inherited-ui-checks.md#scroll-containers--adr-0030-task-006).
  - Needs overflowing artist, release, playlist, track, Index detail, and Settings content. Verify wheel, scrollbar, and supported keyboard scrolling in Light and Dark. Missing overflowing content leaves that subcheck open.
  - The Music portion of this check also moved. See "Phase 4: Library Lists, Playlists And Readiness" above.

## Working Rules For This Plan

- Each implementation session owns one packet.
- The operator walks each visual check right after its packet. A backlog of open checks hides defects.
- The orchestrator does not ask the operator again about a decision that the operator made.
