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
   Decided on 2026-10-03: 25 ADRs archived. ADR 0078 archived with ADR 0082 packet 002 on 2026-10-17.
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
- Browsing follows Apple Music: album pages with one main Play action. ADR 0084 replaced the Music home and the album grid: Music is a column browser that opens on Discover (Decisions 1 and 3).
- Value for value is visible: the payment split as a bar, and the show readiness of an album.
- Inspection stays one step away: an "Inspect sources" view with RSS, MusicIndex and file tags side by side.
- From Raycast, the app takes a cohesive status bar and simple icons. The app is not a keyboard-driven power tool: each action works with the mouse, and a "⋯" menu holds the secondary actions.

### Phase 3: Tokens

Three packets implement ADR 0083 Decisions 1 to 4 and 10. The operator walks the visual check of each packet right after it.

1. [Task 001](../tasks/archive/adr-0083-task-001-palette-and-color-roles.md): the palette and the entity colors. Done on 2026-10-03.
2. [Task 002](../tasks/archive/adr-0083-task-002-font-and-artwork-tokens.md): Figtree, the display size, the artwork shadow and the monogram placeholder. Done on 2026-10-03.
3. [Task 003](../tasks/archive/adr-0083-task-003-one-icon-set.md): each icon from the Lucide set. It corrects an emoji icon that Figtree caused. Done on 2026-10-07.

### Phase 4: Library

Packets for each surface: the album page with its track list and a play action, and the "Inspect" disclosure. ADR 0084 replaced the album grid with the column browser of Phase 4A. Each packet has its visual check, walked right after it.

The first Phase 4 packet builds the track page once, for a Library track and an Index track. The operator decided this on 2026-10-07.

1. [ADR 0083 task 004](../tasks/archive/adr-0083-task-004-one-track-page.md): one track page, the action hierarchy, name links, credits and "Inspect sources". Done on 2026-10-07.
2. [ADR 0083 task 005](../tasks/archive/adr-0083-task-005-album-page-header-and-actions.md): one album page header and action row for the Library and the Index, the 200 pixel cover and the cover color backdrop. Done on 2026-10-08.
3. Later packets: the inspection view of RSS, MusicIndex and file tags (ADR 0083 Decision 8), not started. The row hover actions (ADR 0083 Decisions 5 and 6) are three Ready packets of 2026-10-20: [task 006](../tasks/adr-0083-task-006-row-hover-actions.md) for the track row (implemented on 2026-10-20, visual check open), [task 007](../tasks/adr-0083-task-007-list-and-playlist-row-actions.md) for the playlist, Tracks and Needs attention rows (implemented on 2026-10-20, visual check open), and [task 008](../tasks/adr-0083-task-008-drag-track-to-playlist.md) for a drag to a playlist (implemented on 2026-10-21, visual check open).
   Also the payment split bar for the track page and the album page.

### Phase 4A: Flow And Information Architecture

The operator decided on 2026-10-08 to add this phase. Phases 2 and 3 set the look and the colors. They did not set the flow. The Music pieces exist, but they are scattered:

- Library and Index are two places, not two states of one album. The Library album page opens from the sidebar tree. The Index album page opens only from search.
- One section has two layouts: the Library has a sidebar and a content pane, and search uses one full-width pane that navigation replaces.
- "Artist" has three pages: the Library artist page by name, the Index name match page, and the publisher page (ADR 0077).
- The sidebar has three jobs: navigation, a tree of artists, albums and tracks, and feed status.
- Search is the only way to reach the Index. No view lets the operator browse new music.

The "Inspect sources" view and the payment split bar are inside pages, so they can continue during this phase. The row hover actions wait for the structure work.

Curator jobs, decided by the operator on 2026-10-08. Each job becomes a scripted flow check:

1. Find new music in the Index.
2. Take music in: download, then confirm the tags and files.
3. Build a show playlist and confirm that it is ready.
4. Keep the Library ready: feed checks, updates, repairs and tracks that need attention.

Design constraints from the operator, 2026-10-08:

- Multiple selection is essential. The operator selects several search results or artist singles and downloads the selection with one action. The selection is a shared list behavior, not a feature of one screen.
- An audition player comes later, with its own audio route, so that the curator can hear a track, seek through it and decide quickly. Each track row and each album page keeps a place for it ([ADR 0068](../adr/0068-show-cue-and-audition-isolation.md) is Proposed).

Steps:

1. Current flow map. A read-only agent records the steps of each job in today's code, each dead end, and each place where the operator must know the origin first. The operator confirms the map in a short walkthrough.
   Written on 2026-10-08: [Music flow map](../architecture/music-flow-map.md). The walkthrough is open.
2. Object model. Decide the entities that the operator sees and their relations: album, track, artist, publisher and playlist. Decide what "artist" means. Library membership becomes a state or a filter on each list.
   - Artist, decided by the operator on 2026-10-08: one artist page type with two levels. When an album names a publisher feed, the artist page is that publisher, with its identity, its albums from the Library and the Index, and its links. With no publisher feed, the same layout shows a labeled name grouping, for example "Albums that name Ledbetter". The page says which identity it has. Names never merge into one identity (ADR 0077 Decision 1). A publisher feed can be a label and not the artist, so the page shows the feed title.
   - Library membership, decided by the operator on 2026-10-08: a state, not a place. Each album, track and artist has one page, whatever its origin. The page shows its Library state, for example "In Library", "3 of 7 tracks" or "Not downloaded", and the actions for that state. Each list has one scope control, "Library" or "Everything". The sidebar destinations show the Library by default. Search shows everything, with a badge on each Library item. This deletes the separate Index page routes.
   - Navigation, decided by the operator on 2026-10-08: one history stack for the content area. Each page change goes into it: a sidebar click, a row, a link or search. Back and Forward always show. A breadcrumb shows the location, not the history: the destination and the path inside it, for example "Playlists > 000 > Tiddies In My Face". A breadcrumb shows each page one time only.
   - Multiple selection: [ADR 0084](../adr/0084-music-column-browser.md) Decision 10, amended on 2026-10-19, owns the model, the two modes and the action bar.
3. Structure options. Two or three clickable HTML prototypes of the Music section, scored against the four jobs and the two constraints.
   Built on 2026-10-08: [structure options canvas](https://claude.ai/artifact/Uy8HeawDJGpuyVWvutNme6) (private to the operator), with option A (destinations sidebar), option B (column browser) and option C (home first).
   - Structure, chosen by the operator on 2026-10-08: option B, the column browser. The sidebar holds the destinations, a list column shows the items of the destination, and a detail column shows the selected item. At a narrow width, the sidebar collapses to icons only.
   - The first destination is Discover, a browser of new releases as tiles or rows. Later views go in the Discover column: shared musicL playlists and genre tags, scope to be decided.
   - The operator renamed "Albums" to "Releases" on 2026-10-08, because many releases are singles. A release shows its kind: Album, EP or Single.
   - Singles: in the Releases list, two or more singles of one artist show as one group row, for example "Haleen · 7 singles · 5 in Library". The row opens to show each single, and its checkbox selects them all. The group page lists the singles newest first and offers "Download n singles". An artist page shows "Albums and EPs" apart from "Singles". The group uses the artist identity of the object model, so names never merge across publisher feeds. The refined option B on the canvas shows this design. ADR 0084 Decision 6 owns it, and packet 015 built it.
   - Scope controls, decided by the operator on 2026-10-08: Releases and Artists each have the control "Library" or "Everything". Tracks has no scope control and shows the Library only.
   - Collection art, requested by the operator on 2026-10-08: as on `../musicindex/search.html`, the MusicIndex logo shape frames the art of a collection. A collection is a singles group, an artist page or a playlist. The frame holds a mosaic of up to four covers, or the entity color when no cover exists. A collection with one cover shows that cover as a square. One release always shows its cover as a square.
4. A decision record for the chosen structure. It amends ADR 0060 and the decisions on frames, the content list and the Library control that the structure changes.
   [ADR 0084](../adr/0084-music-column-browser.md), accepted by the operator on 2026-10-08. It owns each structure decision listed above.
5. Packets, each with its flow check walked right after it. A flow check counts the steps of each job before and after, and lists the dead ends.
   Proposed order on 2026-10-08. Each packet is one session, and the next packet starts only after the flow check of the one before it:
   1. One navigation stack and the location breadcrumb (ADR 0084 Decision 9). It fixes flow map findings 1 and 7.
      Complete on 2026-10-08 as [ADR 0084 task 001](../tasks/archive/adr-0084-task-001-one-navigation-stack.md). The operator passed its visual check on 2026-10-08.
   2. One page for each item (Decision 4). Library and Index rows open the same release and track pages. It fixes finding 2.
      Written on 2026-10-08 as two packets: [task 002](../tasks/archive/adr-0084-task-002-one-release-route.md) for releases, complete on 2026-10-11, and [task 003](../tasks/archive/adr-0084-task-003-one-track-route.md) for tracks, complete on 2026-10-11. The [ADR 0084 phase plan](adr-0084-music-column-browser-phase-plan.md) holds the packet register.
   3. The three-column shell: the destinations, the collapsible sidebar, Discover with "New releases", Needs attention and Playlists (Decisions 1 to 3). It deletes the sidebar tree and fixes finding 5. Complete as packets 004 to 006, 013 and 014.
   4. Releases: the kind of each release, the two scopes and the singles groups with their group page (Decisions 5, 6 and 8). Packets 007 and 015 are complete. Packet 016, the scopes, waits for no Stophammer change since release 0.12.0.
   5. The artist page with two levels and its scope (Decisions 7 and 8). It replaces the Library artist page by name and the name match page, and fixes finding 6.
      The operator split it on 2026-10-18. [ADR 0084 packet 008](../tasks/archive/adr-0084-task-008-feed-page-and-artists.md) builds the feed level and the Artists list. Packet 017 builds the name level, and replaces the two old pages.
      Packet 010 fixes finding 6, and packet 016 adds the scope.
   6. Multiple selection and the action bar, with the reserved audition place in the row composite (Decisions 10 and 11). It fixes finding 3.
      The operator split it on 2026-10-18: [ADR 0084 packet 009](../tasks/archive/adr-0084-task-009-list-selection-and-action-bar.md) builds the list column, and [packet 018](../tasks/archive/adr-0084-task-018-detail-column-selection.md) the detail column.
      The operator split packet 020 from packet 018 on 2026-10-20: the playlist page and the Discover tiles. Packet 010 adds the selection of the search results. Packets 009, 018 and 019 are complete.
      The audit of 2026-10-20 added packet 021, one release page assembly for both origins, and packet 022, the download queue ([run record](../reviews/orchestrated-run-2026-10-20.md)).
   7. Search results in the list column (Decision 2). Saved searches need storage first. The [ADR 0084 phase plan](adr-0084-music-column-browser-phase-plan.md) registers them as packet 012, by operator decision of 2026-10-16.
   8. The logo frame for collections (Decision 12).
   9. The window groups of New releases and the saved searches. The operator asked for them on 2026-10-22 and answered the open questions on 2026-10-23. [ADR 0086](../adr/0086-new-releases-singles-window.md) is Accepted. Its [phase plan](adr-0086-new-releases-window-groups-phase-plan.md) holds six packets.
   10. Search fields, the field list and live results. The operator asked for them and accepted [ADR 0087](../adr/0087-search-fields-and-live-results.md) on 2026-10-23.

### Phase 5: Settings

ADR 0069 and its task 002 continue. The grouped Settings keep each report and log reachable.

### Phase 6: Player And Cue

A separate ADR, after the Library work. The playback defect of one song that does not change is a separate early packet.

## Remaining Order - 2026-10-23

The orchestrator wrote this order on 2026-10-23, from the operator answers on ADR 0086 of the same day. It replaces the phase numbers as the order of work. Each step is one session or more. A step starts only after the visual check of the step before it.

The publisher show-log trigger of the [delivery order](broadcast-chain-delivery-order.md#current-delivery-order) stays first: when a real show is scheduled, publisher show-log task 001 comes before each step below.

### Phase 4A: Close

1. The operator accepted [ADR 0086](../adr/0086-new-releases-singles-window.md) on 2026-10-23.
2. The operator walks the open checks of the surfaces that ADR 0086 changes: [packet 020](../tasks/adr-0084-task-020-discover-selection.md) V1 to V6 and [packet 026](../tasks/adr-0084-task-026-index-singles-groups.md) V1 to V10. A defect that these checks find is then not mixed with a defect of ADR 0086. The operator moved this walk to the start of the final walk of the [run of 2026-10-23](../reviews/orchestrated-run-2026-10-23.md).
3. [ADR 0086 packets](adr-0086-new-releases-window-groups-phase-plan.md) 001 to 005, in register order. Before packet 005, the operator walks the open checks of [packet 012](../tasks/adr-0084-task-012-saved-searches.md).
4. [ADR 0087 packets](adr-0087-search-fields-phase-plan.md) 001 to 005: the search fields, the field list and the live results (operator decision of 2026-10-23). Packet 006 waits for the Stophammer release of open request 10.

### Phase 5: Settings

5. [ADR 0069 task 002](../tasks/adr-0069-task-002-shared-guarded-editor.md), the shared guarded editor. It is Ready.
6. ADR 0086 packet 006, the window length in Settings. It uses the editor of step 5.

ADR 0069 phases 003 to 005 wait for the full ADR 0066 release (step 12).

### Ready Defects

7. [The live value URI plan](livevalue-uri-is-a-url.md). It is Ready and small. It must be complete before the relay adoption of row 3 of the delivery order.

### Phase 4: Remaining Library Work

Each step needs its packet written first.

8. The download state of the release rows of the feed page and the name page: a count of a partial release, "Queued" and "Downloading" (open finding of ADR 0084 packet 033 D1).
9. The inspection view of RSS, MusicIndex and file tags (ADR 0083 Decision 8). It includes the "Inspect sources" view of an Index track page and the contributors panel of an Index release page (open findings of the ADR 0084 phase plan).
10. The `TYER` difference of an MP4 file in "Update n files" ([ADR 0080 plan](adr-0080-tag-frames-phase-plan.md)).
11. The payment split bar for the track page and the release page.

### Phase 6: Player And Cue

12. The operator decides [ADR 0068](../adr/0068-show-cue-and-audition-isolation.md), and an agent diagnoses the mpv IPC error. Then the playback defect of one song that does not change. Then the playback position after a move or a removal of playlist rows (open finding of the ADR 0084 phase plan). ADR 0066 task 004 can then pass, and the configuration-format gate opens for ADR 0069 phase 003 and deferred item 7.

### Later

13. The responsiveness phase: the open items of "Throughout: Responsiveness And Background Feedback".
14. The broadcast chain rows 3 to 6 of the [delivery order](broadcast-chain-delivery-order.md#current-delivery-order), in their order.
15. Procedures for the visual requirements §8 and §9, which cannot run as written.

### Throughout: Code Correctness

- Single source of truth audit: read-only agents find data written in two places and two paths that compute one output. They also find errors that only stderr shows, and live values where the stored value is the rule. Each finding becomes a failing test and then a fix.
- Split the largest files at their natural seams. No behavior changes in those packets.
- Add `cargo clippy --all-targets -- -D warnings` to the gate. Done on 2026-10-20 by [overhaul correctness task 001](../tasks/archive/overhaul-correctness-task-001-clippy-all-targets.md).
- No complete rewrite. Change one surface at a time and keep the gate Green.

### Throughout: Responsiveness And Background Feedback

The operator asked on 2026-10-08 for a later phase that tunes how fast the app responds, and gives clear feedback, with motion, for work that runs in the background, such as downloads and fetches. Each session adds each concern it finds to this list, with its date and its owner file. The list stays here until a packet takes it. This list is advisory. It states no rule.

1. 2026-10-07: the debug build is slow in daily use. The release build is fast enough. Use `./target/release/v4vmm` before a performance measurement.
2. 2026-10-07: each MusicIndex request asks for each collection of its endpoint (ADR 0075 amendment of 2026-10-07). Measure the cost of the larger responses before the tuning phase.
3. 2026-10-08: a cover color arrives after its page opens. The album page backdrop shows late, with no transition (`ui::composites::CoverBackdrop`).
4. 2026-10-08: thumbnails and Index pages load with no shared loading state. A page shows its cover or its facts when each arrives (`thumbnail_for_url` in `src/library/app_impl.rs`, `index_remote_detail_hero_image` in `src/app/search_dispatch.rs`).
5. 2026-10-08: a download shows only a busy label, such as "Downloading feed..." or "Downloading...". It shows no progress, no count of done tracks and no motion (`begin_busy_feed` in `src/library/app_impl.rs`, `ReleaseMembershipState::Downloading`).
   [ADR 0084 packet 022](../tasks/adr-0084-task-022-download-queue.md) owns it from 2026-10-20: one progress item names the batch, the item and the failures. It shows no count of done tracks inside one release.
6. 2026-10-08: "Download album" and "Copy feed URL" on an Index track page first read the album feed from MusicIndex. Nothing on the page shows that read (`download_index_album_of_track` and `copy_index_feed_url_of_track` in `src/app/search_dispatch.rs`). Their results show only in `settings_status`.
   ADR 0084 packet 022 owns the "Download album" part from 2026-10-20: the read runs in the slot of its queue job, and the progress item shows it. The "Copy feed URL" part has no owner.
7. 2026-10-08: the toolbar search shows no state for an unreachable MusicIndex before it sends the request. A failure shows only after the request (flow map, job 1).
8. 2026-10-08: multiple selection (Phase 4A) adds downloads of many releases at one time. The action bar needs a progress state for the whole selection.
   ADR 0084 packet 022 owns it from 2026-10-20.
9. 2026-10-12: the Needs attention count runs the broadcast readiness report on the UI thread after each Library event that can change it: load, feed check, repair, download and removal. The report reads the database and examines each track file (`refresh_current_broadcast_readiness_report` in `src/library/app_impl.rs`, ADR 0084 task 005).
10. 2026-10-16: scroll is slow in "New releases" while the tiles show placeholders, in the debug build and in the release build. The possible cause is the thumbnail fetch: each tile starts one `FetchThumbnail` command on the tokio blocking pool, with no limit, and each command downloads, decodes, scales and encodes one cover. Many of these commands at one time can use all processor cores and slow the UI thread. No measurement confirms this cause (`thumbnail_for_url` in `src/library/app_impl.rs`, `read_or_download` in `src/media/image_cache.rs`, ADR 0084 task 006). On 2026-10-16 the operator found no lag in a normal debug build. The lag came from a binary with GPUI test-support, which draws at once after each effect. The next page still takes about 400 ms to show, and the list shows no loading state in that time. On 2026-10-17 the orchestrator found the cause of the 400 ms. `fetch_recent_feed_result_rows` in `src/application/queries/feed.rs` reads the full detail of each release on the page. It sends one MusicIndex request for each of the 20 releases, one after the other.
    [ADR 0086 packet 001](adr-0086-new-releases-window-groups-phase-plan.md) owns the 400 ms from 2026-10-23: at most 4 reads run at the same time. It is implemented on 2026-10-23. No measurement confirms the new time.
11. 2026-10-16: the broadcast service watch runs the `butt` status command one second after each previous run ends, also in Music. In a 41-second profile of task 006 check V6, the app started the `butt` AppImage 16 times. These runs used 72% of the processor samples of the profile, and the UI thread used 14%. Each AppImage start costs much more than the status read (`BROADCAST_SERVICE_POLL_INTERVAL` in `src/runtime/broadcast_service_watch.rs`, `status` in `src/broadcast/encoder.rs`).
    [ADR 0059 task 018](../tasks/archive/adr-0059-task-018-service-watch-pace.md) owns it from 2026-10-18.
12. 2026-10-18: the operator heard the processor fan run faster with the debug build. The same watch also runs `systemctl --user show` two times each second, for the publisher unit and the producer unit (`ServiceControl::show` in `src/broadcast/control.rs`).
    Each snapshot redraws the full window, also when no value changed. `publish_snapshot` uses `send`, not `send_if_modified`, and `bridge_watch` in `src/presentation/gpui_vm_bridge.rs` calls `cx.notify()` after each snapshot. The playback poll of `src/runtime/playback_polling.rs` also redraws each second.
    ADR 0059 task 018 owns the service watch part, and the playback poll part has no owner.
13. 2026-10-20: the top app builds the playlist page rows of the detail selection several times in each render. A long playlist can thus render slowly (`detail_page_rows` in `src/app/detail_selection.rs`, ADR 0084 packet 025). No measurement confirms it.
14. 2026-10-20: `reload_playlists` reads the cover images of each playlist on the UI thread, beside the playlist list read (`playlist_cover_images` in `src/db.rs`, ADR 0084 packet 011). No measurement confirms a delay.
15. 2026-10-20: the first show of the Artists list or the Playlists list starts one logo composition for each row at the same time. They run on the command runner (`FetchLogoMosaic` in `src/application/queries/images.rs`, ADR 0084 packet 011). The rows show the entity color until each image arrives.
16. 2026-10-21: one render computes the merge of the Releases list in "Everything" more than one time: for the header action, for the rows and for the selection keys (`src/view_models/releases_everything.rs`, ADR 0084 packet 016). With many loaded pages, this can slow the render. No measurement confirms it.
17. 2026-10-21: a search sends one `GET /v1/artist-names?from=<key>&limit=1` check for each MusicIndex candidate name, one after the other, before it shows the name rows (`src/application/queries/search.rs`, ADR 0084 packet 017 decision D15). Several candidates add their waits. No measurement confirms a delay.
18. 2026-10-21: one render of the search list builds the rows of the open tab more than one time: for the header action, for the bar and for the rows. Each row finds its Library album with a scan of the tree, and each call of `search_lookup` reads the whole tree again. A large Library makes this slow (`with_search_results_list` in `src/app.rs`, `LibrarySearchLookup` in `src/view_models/library.rs`, ADR 0084 packet 010).
19. 2026-10-21: by packet 026 decision D4, the Releases list in "Everything" reads each new artist feed of the singles on a page before the page shows. The first pages each send 27 to 42 more requests, at most 4 at a time, so page 1 can show 1 s to 4 s later (`src/application/queries/name_lists.rs`, ADR 0084 packet 026). Measured on 2026-10-21 at 17:52 UTC: page 1 sent 37 feed requests in 2.36 s after its list request of 1.06 s, and page 2 sent 28 in 1.06 s. Page 1 also reads each Library artist feed with singles.
20. 2026-10-21: a search row whose image load fails does not try again in the same session. The row keeps its monogram until the next start (`src/app/search_dispatch.rs`, ADR 0084 packet 028).
21. 2026-10-21: while the Index part of a search loads and the tab has Library rows, the page shows no loading row. The Index rows then appear with no notice, for example after Back to a search (`src/view_models/search_results/mod.rs`, ADR 0084 packet 010).
22. 2026-10-22: once in each session, after the first load of the Library tree, the app sends one `HydrateAlbumIdentity` request for each Library release that has a feed GUID and no stored feed link, at most 4 at a time, with no progress state. A release whose recorded MusicIndex answer names no publisher is not sent again (`start_album_identity_backfill` in `src/library/app_impl.rs`, ADR 0084 packet 034).
23. 2026-10-23: by ADR 0086 packet 003, one New releases page request can send a maximum of 6 list requests. Their detail reads also run before the tiles show. The loading state shows no progress in that time (`start_recent_music_load` in `src/library/app_impl.rs`, ADR 0086 packet 003). No measurement confirms a delay.
24. 2026-10-23: by ADR 0087 Decision 4, a search runs after each pause in the typing. Each run sends three Index requests, and the field list sends one `GET /v1/artist-names` request for `artist:`. The field `is:ready` reads the show readiness of each matched track (`src/app/search_dispatch.rs`, ADR 0087). Each run also keeps the artist name checks of item 17, so each pause can send them again (ADR 0087 packet 003 D13). No measurement confirms a delay.
25. 2026-10-23: the Library search runs on the main thread while it holds the database lock. The `EXISTS` conditions and the `json_each` id sets of ADR 0087 packet 002 add time to each run. Packet 003 runs the search after each pause (`src/app/search_dispatch.rs`, `src/db/library_search.rs`). No measurement confirms a delay.
26. 2026-10-23: `present_after` notifies its entity at the end of each timer, also when the ticket is old. Each typed character in the search field thus gives one more render of `TopApp`, one pause later (`src/presentation/delayed_presenter.rs`, ADR 0087 packet 003). No measurement confirms a delay.
27. 2026-10-23: an Index search request that started cannot stop, because the HTTP client blocks. A run that is not current still sends its started request to MusicIndex (`src/application/queries/search.rs`, ADR 0087 packet 003).

## Visual Requirements Moved From Pending Checks - 2026-10-07

The operator decided on 2026-10-07 to stop walking checks of screens that
Phase 4 and Phase 5 rebuild. Each packet that rebuilds a listed surface names
the rules it carries in its visual acceptance criteria. A rule leaves this
list only when a person walks it on the new screen, or when an ADR retires it.

### Phase 4: Track Page

The operator walked §2 and §19 on the one track page on 2026-10-07. They left this list.


- **§22 Track Header Identities — ADR 0075 Task 022.** Owner: [task 022](../tasks/adr-0075-task-022-track-header-identities.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - V1: an Index track without its own identities shows no website or Nostr action in its header. The feed section shows them, with the feed named as owner, in Light and Dark themes.
  - V2: a track with its own identities shows them in its header, apart from the feed section.
  - V3: a track without its own description shows no description.
  - V4: normal and narrow widths show each element in its place, with no clipped text.
- **§26 No Derived Release Date — ADR 0075 Task 049.** Owner: [task 049](../tasks/adr-0075-task-049-no-derived-release-date.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: the check only reads pages.
  - V1: a track page of a track without its own date shows no release date.
  - V2: a track with its own date shows that date.
  - V3: normal and narrow widths show each row in its place, in Light and Dark themes.
- **§27 Separate Track Artwork — ADR 0075 Task 048.** Owner: [task 048](../tasks/adr-0075-task-048-separate-track-artwork.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: the check only reads pages.
  - V1: an Index track with its own image shows that image. An Index track without one shows the album image.
  - V2: normal and narrow widths show the artwork in its place, in Light and Dark themes.

### Phase 4: Album Page

- **§3 Stored Metadata In Details — ADR 0054 Tasks 004 And 005.** Owners: [task 004](../tasks/adr-0054-task-004-feed-read-model-hydration.md) and [task 005](../tasks/adr-0054-task-005-track-read-model-hydration.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk). The operator's pass of 2026-10-08 on ADR 0083 task 005 did not name this rule. A second walk now confirms it (orchestrator decision of 2026-10-20).
  - Precondition: needs known persisted feed/track metadata, reachable Index for comparison, and an unavailable endpoint for local fallback. Use the disposable config.
  - Check: [Metadata Hydration](../runbooks/inherited-ui-checks.md#metadata-hydration--adr-0054-tasks-004-and-005).
  - Record feed and track results separately in the [checklist](../reviews/adr-0054-review-checklist.md), in both themes.
- **§15 Stored Value Projection — ADR 0075 Task 020.** Owner: [task 020](../tasks/adr-0075-task-020-stored-value-projection.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk). The check is walkable at schema version 19, which is current (triage of 2026-10-20). The packet text that names schema version 16 is stale.
  - Precondition: the check sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.
  - V1: after a check applied a title change, the album page and the track page show the new title without navigation. Check in Light and Dark themes.
  - The track page shows the channel title as the album name. A title with "(old)" after the check is wrong.
- **§30 Feed Dates By Owner — ADR 0075 Task 050.** Owner: [task 050](../tasks/adr-0075-task-050-feed-dates-by-owner.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: the check only reads pages.
  - V1: an album page shows "Published" and "First track published" as two facts, each with its source.
  - V2: a track without its own date shows "Feed publication date" apart from its other facts.
  - V3: normal and narrow widths show each fact in its place, in Light and Dark themes, with no clipped text.

### Phase 4: Library Lists, Playlists And Readiness

- **§1 Music And Settings Scrolling — ADR 0030 Task 006 (Music scrolling).** Owner: [task 006](../tasks/adr-0030-task-006-scroll-containers.md).
  - Carried from 2026-10-17: ADR 0084 packet 014 V1, V2 and V7 carry the wheel part and the scrollbar part, in Light and Dark.
  - The keyboard part goes into the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk). ADR 0084 packet 018 V6 already carries the keyboard part on the track page.
  - The Settings portion of this check also moved. See "Phase 5: Settings" below.
- **§9 Feed Check And Feed Update Storage Failure — ADR 0075 Task 039.** Owner: [task 039](../tasks/adr-0075-task-039-feed-check-and-update-observation-retention.md).
  - The procedure cannot run as written. The packet names no concrete terminal step. An agent must write the steps first, from the packet 014 fixture (triage of 2026-10-20). This item is not in the final walk of the run.
  - Precondition: the packet supplies prospective isolated setup, failure injection, desktop inspection, and cleanup commands. Use disposable data and scripted local services.
  - Check: an ordinary feed error keeps its existing per-feed message and placement.
  - Verify that a storage failure ends only the current feed operation and claims no successful persistence.
  - Verify that **Check all feeds** keeps its existing results, controls, and placement.
- **§13 Playlist RSS Check Button And Report — ADR 0076 Task 001.** Owner: [task 001](../tasks/adr-0076-task-001-playlist-rss-document-check.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: migration 15 adds two tables. Make the SQLite backup before the first run of the new build. Each check sends real HTTP requests to the feed hosts of the playlist.
  - V1: the playlist page shows the "Check RSS" button with its state, in Light and Dark themes.
  - V2: during a check, the page shows the progress in place, without navigation. Playback from the playlist does not wait for the check.
  - V3: after the check, the page shows each feed outcome and each stopped host in readable text.
- **§14 RSS Comparison, Apply And Report — ADR 0076 Task 002.** Owner: [task 002](../tasks/adr-0076-task-002-rss-comparison-apply-and-report.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: migration 16 drops three tables. Make the SQLite backup before the first run of the new build. Each check sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.
  - V1: after a check with differences, the playlist page shows the report in place. Each row gives the feed, the field, the old value, the new value and the time. Each stale feed has a podping.me button. Check in Light and Dark themes.
  - V2: a new track shows with its download action. A removed track shows its mark on the playlist row.
  - V3: the report is readable at normal and narrow widths. Stacked text follows the column text rule.
- **§16 Stored Payment Route And Readiness — ADR 0076 Task 003.** Owner: [task 003](../tasks/adr-0076-task-003-stored-payment-route-and-readiness.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: migration 17 adds and fills two columns. Make the SQLite backup before the first run of the new build.
  - V1: the Music readiness list shows a "Route out of date" row with no button and a "Removed from feed" row with a **Confirm** button. Check in Light and Dark themes.
  - V2: the Show Source card counts both states as not ready and names them in its detail.
  - V3: **Confirm** on a removed track changes the row in place, without navigation.
  - The readiness row of a removed track also has **Remove from library**. The procedure opens its ADR 0044 confirmation and cancels it, because a removal deletes the audio file.
  - V4: a playlist row of a removed track shows a row error with **Remove from playlist** and **Remove from all playlists**. The second action lists each playlist in its confirmation. Each action changes the playlist in place.
- **§18 Check And Scan Follow-Ups — ADR 0076 Task 005.** Owner: [task 005](../tasks/adr-0076-task-005-check-and-scan-follow-ups.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: needs schema version 17 and the readiness list from packet 003. This packet adds no migration.
  - V1: the check report shows **Copy feed URL** adjacent to the **Open podping.me** button. The pasted text is the feed URL. Check in Light and Dark themes.
  - V2: after a **Confirm** in the readiness list, the playlist page shows no error on that row. No check and no restart occur between the two steps.
  - The check in V1 sends real HTTP requests to the feed hosts of the playlist, and it writes RSS values into the stored values.

### Phase 4: Publisher And Artist Pages

- **§12 Library Artist View Without Artist Storage — ADR 0077 Task 001.** Owner: [task 001](../tasks/adr-0077-task-001-remove-dead-artist-storage.md).
  - Carried by [ADR 0084 packet 017](../tasks/adr-0084-task-017-name-level-artist-page.md) V1 and V3 from 2026-10-21. Retired: the track list of V1 (ADR 0084 Decision 7).
- **§20 Publisher Page And Navigation — ADR 0077 Task 004.** Owner: [task 004](../tasks/archive/adr-0077-task-004-publisher-navigation-and-presentation.md).
  - Carried from 2026-10-18: [ADR 0082 packet 003](../tasks/archive/adr-0082-task-003-release-page-named-feeds.md) V2 and V7 carry V1, apart from its type, and V4 carries V2. [ADR 0084 packet 008](../tasks/archive/adr-0084-task-008-feed-page-and-artists.md) V5 carries the failure part of V3.
    Packet 008 V3 carries the "Not listed" mark of V4, V6 carries its derived count, and V8 carries V5.
  - Retired on 2026-10-20: the type in V1 (ADR 0085 Decision 3, which supersedes ADR 0082 Decision 1).
  - Retired on 2026-10-20: the two groups of V3 (ADR 0084 Decision 4).
  - Retired on 2026-10-20: the stated and assumed role text of V4 (ADR 0085 Decision 3).
- **§21 Feed Owner Text And Name Grouping — ADR 0077 Task 005.** Owner: [task 005](../tasks/adr-0077-task-005-feed-owner-text-and-name-search.md).
  - Carried from 2026-10-18: ADR 0082 packet 003 V5 carries V1.
  - Retired on 2026-10-20: V2, the name search (ADR 0084 Decision 7, which replaces the Library artist page by name and the Index name match page). The deleted surface was removed in commit a32636e.
  - Carried by ADR 0084 packet 017 V2 and V10 from 2026-10-21. Retired: the text "Grouped by name" of V3 (ADR 0085 Decision 2).
- **§28 Confirmed And Unconfirmed Artists — ADR 0077 Task 007.** Owner: [task 007](../tasks/archive/adr-0077-task-007-confirmed-and-unconfirmed-artists.md).
  - Carried from 2026-10-18: ADR 0084 packet 008 V6 carries V1 to V3, and V8 carries V4.

### Phase 4: Inspect Sources

- **§8 Library Comparison And Hydration Storage Failure — ADR 0075 Task 038.** Owner: [task 038](../tasks/adr-0075-task-038-library-reader-observation-retention.md).
  - The procedure cannot run as written. Its fixture matches an include list that packet 052 replaced on 2026-10-07. Update the match first (triage of 2026-10-20). This item is not in the final walk of the run.
  - Precondition: the packet supplies prospective isolated setup, failure injection, desktop inspection, and cleanup commands.
  - Check: ordinary comparison errors stay in their panel. Ordinary hydration errors remain silent.
  - Verify that storage failures remain reported after navigation without changing selection or unrelated Library state.
- **§24 Tag Frames By Owner — ADR 0080 Task 001 (V4 only).** Owner: [task 001](../tasks/adr-0080-task-001-rss-frames-and-idempotent-writes.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: uses an isolated database copy and never writes to the Library.
  - V4: the compare grid shows the album description row with its owner, in Light and Dark themes, with no clipped text.
- **§25 MusicBrainz URL Relations By Type — ADR 0080 Task 002 (V1, V2 and V4 only).** Owner: [task 002](../tasks/adr-0080-task-002-musicbrainz-url-relations-by-type.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: uses an isolated database copy and never writes to the Library. Needs network access to `musicbrainz.org`.
  - V1: a MusicBrainz lookup shows the release-group homepage in the channel website row, and a "License" row when the release states a license.
  - V2: each other relation shows as a read-only row with its relation type, and it offers no write.
  - V4: normal and narrow widths show each row in its place, in Light and Dark themes, with no clipped text.

### Phase 4: Status Bar And Reports

- **§7 Metadata Observation Storage Failure — ADR 0075 Task 014.** Owner: [task 014](../tasks/adr-0075-task-014-provider-observation-retention.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: use disposable data and scripted local services.
  - Check: the existing Library status identifies a metadata storage failure without claiming MusicIndex unavailability or successful persistence.
  - Verify delayed failure reporting after navigation without changing the selected track's metadata or unrelated Library state.
- **§17 Tag Update Confirmation — ADR 0076 Task 004.** Owner: [task 004](../tasks/adr-0076-task-004-tag-update-confirmation.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: this packet writes audio tags. Copy the fixture audio files before the first confirm.
  - V1: the Music section shows the "Update n file(s)" button with the count. The popup lists each file with its title, album and frames. Check in Light and Dark themes.
  - V2: a file in use by the show shows the mark "In use by the show". **Write Tags** does not write it. After the write, the button count equals the files in use.
  - V3: the popup is readable at normal and narrow window widths. Stacked text clips and does not show only an ellipsis.
- **§34 Download Failures And Dismissal — ADR 0066 Task 014 (V2 only).** Owner: [task 014](../tasks/adr-0066-task-014-download-failures-and-dismissal.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: V2 needs network access and the `flac` binary.
  - V2: after a failed conversion and a successful retry of the same feed, the notice shows no row for those tracks.

### Phase 4: Search

- **§23 Name Matches Are Search Results — ADR 0077 Task 006.** Owner: [task 006](../tasks/adr-0077-task-006-name-matches-are-search-results.md).
  - V1: a search for "Survival Guide" shows a row `Tracks matching "Survival Guide"`. The row does not look like an artist identity. In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Carried by ADR 0084 packet 017 V5, V6 and V10 from 2026-10-21. Retired: the "Tracks matching" page of V2 and the row text of V1 (ADR 0084 Decision 7), and the empty text of V3 (packet 017 decision D6, operator decision of 2026-10-20). V1 keeps its rule that the row does not look like an artist identity.
  - V4 (row only): normal and narrow widths show each element in its place, in Light and Dark themes, with no clipped text. In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).

### Phase 5: Settings

- **§1 Music And Settings Scrolling — ADR 0030 Task 006 (Settings scrolling).** Owner: [task 006](../tasks/adr-0030-task-006-scroll-containers.md).
  - In the final walk of the [run of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md#final-walk).
  - Precondition: needs overflowing Settings content.
  - Check: [Scroll Containers](../runbooks/inherited-ui-checks.md#scroll-containers--adr-0030-task-006).
  - Needs overflowing artist, release, playlist, track, Index detail, and Settings content. Verify wheel, scrollbar, and supported keyboard scrolling in Light and Dark. Missing overflowing content leaves that subcheck open.
  - The Music portion of this check also moved. See "Phase 4: Library Lists, Playlists And Readiness" above.

## Working Rules For This Plan

- Each implementation session owns one packet.
- The operator walks each visual check right after its packet. A backlog of open checks hides defects.
- The orchestrator does not ask the operator again about a decision that the operator made.
