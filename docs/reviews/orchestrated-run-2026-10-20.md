# Orchestrated Run Of 2026-10-20

## Status

Implementation complete - 2026-10-20. The final walk is open. This record is advisory. It states no rule. Each rule stays in its ADR.
The operator walks the final walk at the end of the run. Until then, each visual gate of the run is open.

## Operator Decisions - 2026-10-20

- One orchestrator session runs the steps below. The orchestrator commits each reviewed, Green step as HeyCitizen, and never pushes.
- The visual checks of the run come last, in one final walk. Mockups can replace an early design review.
- When a stop needs a decision, the orchestrator decides, records it below, and continues.
- Steps: the plan edits, the triage and the flow check, then packet 021 (one release page), packet 022 (the download queue), packet 020 and packet 011. The orchestrator split two of them into packets 025 and 024.
- Packet 022: at most three downloads run at the same time. A status strip item shows the progress of a bulk download, for example "Downloading 3 of 9".
- Packet 020: in selection mode, the playlist page hides its drag handle and its row menu.
  Its bar offers "Remove n from playlist", "Download n tracks", "Add n to playlist", "Move to top", "Move to bottom" and "Remove from Library…".
  A Discover tile shows its checkbox at the top-left corner of its cover, and a selected tile gets an outline.
- Packet 011: an image that the RSS feed of an artist gives replaces the logo mosaic. The mosaic is the fallback for each collection that gives no image.
- Triage: each deferred visual requirement that a later ADR superseded is retired, with that ADR named. Each other requirement that a present screen can show goes into the final walk.

## Orchestrator Decisions

Each entry names the step, the decision and the reason.

- Run start: an image that a feed gives shows as a square, as ADR 0084 Decision 12 shows a collection with one cover. Packet 011 amends Decision 12 for the image rule.
- Run start: the download pool of packet 022 never runs two downloads of one feed at the same time. Two downloads of one feed can write the same feed row.

- Packet 021: an Index track title uses the title rule of `TrackDetailVm::display_title`. Decision 4 gives one page, and thus one title rule. The final walk checks a padded title.
- Packet 021: the contributors panel stays on the Library page only. The phase plan records the Index gap as an open finding.
- Packet 021: the track page also has two assembly paths. The orchestrator registered packet 023 for it, outside this run, because its two surface contexts differ in behavior.
- Packet 021: the shell function `render_release_page` is the one owner of the release page body. `TopApp` and `LibraryApp` each give facts and one command dispatcher. The two other owners need a new observation of `LibraryApp` or a move of Index state.

- Packet 021: a row command holds the row index, and reads the open page again at the click. The orchestrator accepts it, because a render follows each change of the rows.
- Packet 022: a pure scheduler type `DownloadQueue` in `src/library/download_queue.rs`, owned by `LibraryApp`. A runtime actor would add a second executor, and its completions must still change two GPUI entities.
- Packet 022: a finished batch stays visible until the operator dismisses it. A timer could hide a failure, and no decision states a delay.
- Packet 022: no cancel. A stop during a write leaves partial files. The final summary asks the operator about "Stop queued".
- Packet 022: a removal is refused while its feed is queued or in flight.
- Packet 022: the queue rules are a contract between modules, so ADR 0084 gets an amendment before the code (AGENTS.md: ADR first).

- Triage: §3, §15 and §30 go into the final walk. ADR 0083 task 005 V83-55 asked for them, but its pass of 2026-10-08 does not name them. A second walk costs little.
- Triage: §23 V2 and V3 wait for packet 017, not retire. The name match page stays live until packet 017 replaces it.
- Triage: packet 014 V1, V2 and V7 carried the wheel and scrollbar part of §1. The keyboard part goes into the final walk with packet 018 V6. The operator can reject this carry in the audit.
- Triage: the Index track page builds `TrackDetailVm` with no `with_feed_identity`, so it shows no feed identity section and no "Feed publication date" row. Packet 023 owns the fix.
- Triage: the procedures of §8 (task 038), §9 (task 039) and the pending check of ADR 0075 task 012 (schemas 11 and 12) cannot run as written. They stay in the plan with the reason, and they are not in the final walk.
- Triage: the tasks ADR 0077 task 004 and task 007 have no open requirement, so they move to the archive.

- Packet 020: split into packet 020, the Discover tiles, and packet 025, the playlist page, after packet 022. Earlier splits used new numbers. The playlist key is the entry position and the track id, because a playlist can hold one track two times. The tile focus moves in a line, because the number of tiles in a row is a layout fact. Up and Left go back, and Down and Right go forward.
- Packet 020: a move or a removal in a playlist does not change the stored playlist position of a playback session. A single-row move has the same gap today. The packet records it.
- Packet 011: split into packet 011, the logo frame with no schema change, and packet 024, schema 20 for the image of an artist feed. The Library does not store that image today, and the operator decision of 2026-10-20 needs it on the Artists rows. Migration 20 goes through the ADR 0016 registry, and its packet names the backup step.
- Packet 011: the mosaic is composed off the UI thread and masked in the image cache. GPUI 0.3.1 has no clip path and no mask for an image, so a layered element cannot clip four covers to the logo shape.
- Packet 011: the provided image shows as a square. `search.html` shows one image as a square cover, and ADR 0084 Decision 12 gives the same rule for one cover.
- Flow check: the counts of the four jobs fell from 2, about 12, 6 and 6 steps to 1, 3, 5 and 5 steps. The run order stays, because packet 020 covers findings 3 and 9.
- Flow check: findings 13 and 14 name dead code (`ArtistVm` with `render_artist_view`, and the Play action of `TrackActionState`). AGENTS.md says to delete dead code. No packet of this run owns it, so the final summary names it.
- Flow check: finding 16, no browse of the Index artists by name without search, is a question of direction for the operator. The final summary names it.

- Packet 022 review: a queued job shows "Queued", and only a running job shows "Downloading". Nine queued releases that all read "Downloading" give a false state.
- Packet 022 review: an Index track with no track GUID uses its enclosure URL as its job identity. Before the correction, the queue dropped the second such track of one release.
- Packet 022 review: "Remove" is absent while its release is queued or downloading, so no confirmation opens for an action that cannot act. The refusal in the command path stays as a second guard.
- Packet 022 correction: a queued track row shows a disabled "Queued" button, the same as the present "Downloading..." button. The release page shows no state word for a release, so "Download album" is unavailable while its release is queued or running.
- Packet 022 review: a refused job counts as a failure of the batch, so the progress item names it. The orchestrator accepts this deviation.
- Packet 022 review: adding an Index track to a playlist (`SubscribeThenAppendToPlaylist`) stays outside the queue. The packet names no job for it, and its download is one track.

- Packet 020 review: `detail_scroll_handle` now uses the open page entry, so a page that opens from Discover scrolls its own handle. Before the fix, its keys scrolled the hidden Discover list.
- Packet 020: Left and Right move the focus only on the Discover view. The other pages keep the keys of packet 018.

- Packet 025 review: "Remove n from playlist" has no confirmation, because it deletes no file. A selection change during a move can give a wrong key, and the next reload drops it. The row cost of a long playlist is responsiveness item 13.

- Packet 011 review: the singles group row no longer animates a GIF cover on hover, because it shows a mosaic. Each logo is composed at two device pixels for each logical pixel, because the render path has no window scale factor. The playlist image read and the first compositions are responsiveness items 14 and 15.

- Packet 024 review: the Artists list gets the image of an artist feed only after a view of one album of that artist. The phase plan records it as an open finding with no owner.

- Final walk fixes, approved by the operator on 2026-10-20: fixed columns in the track row, no track number on a single row, and one image map for each feed. The packets 021 and 024 record them as amendments.

- Final walk fixes, approved by the operator on 2026-10-20: a feed page stores its channel image for the feed. The playlist rows load their covers, and a release badge reads "release". The packets 021 and 024 record them as amendments.

- Packet answers of the operator, 2026-10-20: the name key is the sort key (packet 017 D3). The drop target is the spring-loaded sidebar "Playlists" row (task 008 D1). Each Needs attention row keeps its main fix visible (task 007 D10).
- Packet answers of the operator, 2026-10-20: the state word shows in "Everything" only, except a partial state (packet 016 D13). Search uses three tabs (packet 010 D1). Saved searches stay as written, and three ideas go to the HIG product polish backlog (packet 012).
- Packet answers of the operator, 2026-10-20: a feed page lists the releases that match its name, marked as inferred, and that name has no row of its own (packet 017 D1, ADR 0084 Decision 7). A name search row opens the name page and is absent when no name-only release exists (packet 017 D6 and D7). A tab change clears the selection (packet 010 D8). One padding value for the track page (packet 023 D14).
- Stophammer request 9 is a draft in both copies of `v4vmm-open-requests.md`. The orchestrator commits nothing in the Stophammer repository. The operator reviews the draft.

- Operator answers of 2026-10-20 to the open questions: add "Stop queued" to the download queue (new packet 027), delete the dead code of flow map findings 13 and 14 now, and fix the test-code lints in one packet that adds `--all-targets` to the gate. The orchestrator implements the new packets now, while the operator walks the screenshot list. The operator reviews Stophammer request 9 before any commit in the Stophammer repository.

- ADR 0083 task 006 review: the icon columns are 44 px, the hit area of a `Button`, not 28 px. A 28 px column cut the hover fill of each icon button.

## Steps

| Step | State | Commit |
|---|---|---|
| Plan edits, triage, flow check | Done | `143a9ff` |
| Packet 021: one release page | Implemented. V1 to V3 open | `6439c00` |
| Packet 022: the download queue | Implemented. V1 to V6 open | `905f3fe` |
| Packet 020: Discover tiles | Implemented. V1 to V6 open | `4cc4540` |
| Packet 025: playlist page | Implemented. V1 to V7 open | `421eb53` |
| Packet 011: the logo frame | Implemented. V1 to V7 open | `a08ec7a` |
| Packet 024: the image of an artist feed (migration 20) | Implemented. V1 to V3 open | `fc0dbcb` |

## Questions For The Operator

The run did not decide these points. Each one needs the operator.

1. Packet 022 has no cancel. A "Stop queued" action could remove the queued jobs and keep the running jobs. Is it wanted?
2. Flow map finding 16: no view lists the Index artists without search. Is an Index artist browse a goal, for example through the "Everything" scope of packet 016?
3. Flow map findings 13 and 14 name dead code: `ArtistVm` with `render_artist_view`, and the Play action of `TrackActionState`. AGENTS.md says to delete dead code. May the next session delete it?
4. Packet 023 (one track page assembly) is registered. It also gives the Index track page the feed identity section that the triage found missing. Is it next?
5. The Artists list shows the image of an artist feed only after a view of one album of that artist. Is a fill for each artist wanted?
6. `cargo clippy --all-targets -- -D warnings` fails in test code that no packet of the run changed. The overhaul plan wants it in the gate. Is it a packet now?

## Walk Log

The operator sends screenshots, and the orchestrator records each result here with its date.

| Date | Check | Result | Evidence |
|---|---|---|---|
| 2026-10-20 | 011.1 mosaic at the page size | Pass | Singles group page, Dark |
| 2026-10-20 | 011.2 logo at the row size | Pass | Releases list, group row |
| 2026-10-20 | 011.3 provided image as a square | Pass | Haleen and HeyCitizen feed pages |
| 2026-10-20 | 011.3 mosaic for a feed with no image, 024.2 | No data | Each artist feed of the Library has a channel image. Unit tests R11-04 and R24-04 cover it |
| 2026-10-20 | 021.1 header, facts, layout in Light at narrow width | Pass | Release page "Aged Friends & Old Whiskey" |
| 2026-10-20 | 021.1 selection on the release page | Pass | Two rows checked, bar "Add 2 tracks to playlist", "Remove 2 tracks…" |
| 2026-10-20 | 021.1 row columns, 021.3 track numbers | Fail, fixed in `958a17e` | Walk again on a new build |
| 2026-10-20 | 021.1 badge "● feed" above a release title | Fail, fix in progress | ADR 0084 Decision 5 |
| 2026-10-20 | 021.3 selection on the singles group page | Pass | Two singles checked, release bar |
| 2026-10-20 | 024.1 image on the group page and the group row | Pass | Haleen photo |
| 2026-10-20 | 024.1 HeyCitizen Artists row | Fail, fix in progress | Feed page shows the image, the row shows the mosaic |
| 2026-10-20 | 022.4 finished item text and "Dismiss" | Pass | "Downloaded 1 of 1. Finished 18:44" |
| 2026-10-20 | §13 V1 "Check RSS" button and state | Pass | Playlist "0001" |
| 2026-10-20 | Playlist row covers | Fail, fix in progress | Each row shows the placeholder. The defect is older than the run |
| 2026-10-20 | 021.1 row columns on a new build | Pass | "3:47" lines up with each duration |
| 2026-10-20 | 021.3 single rows with no number | Pass | Singles group page of Haleen |
| 2026-10-20 | 024.1 HeyCitizen Artists row after its feed page | Pass | The row shows the channel image |
| 2026-10-20 | Playlist row covers | Pass | Playlist "0001" |
| 2026-10-20 | Breadcrumb of a group page opened from its list row | Fail, fixed | It read "Releases > Aged Friends & Old Whiskey > Singles by Haleen". The defect is older than the run (packet 015) |
| 2026-10-20 | Release badge reads "release" | Pass | Release page "Aged Friends & Old Whiskey" |
| 2026-10-20 | Breadcrumb of a group page opened from its list row | Pass | "Releases > Singles by Haleen" |

## Final Walk

Walk Part A first, then Part B. Each packet file holds the full steps of its checks, its "Wrong if" lines and its cleanup. Part A gives the order and the shared setup.

### Part A: The Packets Of The Run

#### Setup

1. Close the app.
2. Copy the Library database as the backup, because packet 024 adds migration 20. The path is `db_path` in your `config.toml`. The default path is `~/.local/share/v4vmm/v4vmm.sqlite`.
   `cp ~/.local/share/v4vmm/v4vmm.sqlite ~/v4vmm-before-schema-20.sqlite`
3. Build the binary: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
4. Prepare a test playlist, for example "Packet 025 test", for packet 025. Use tracks that you can download again.

#### Order

The order puts the checks that only read pages first, and the checks that download, remove or move items last.

1. [Packet 021](../tasks/adr-0084-task-021-one-release-page-assembly.md), V1 to V3: one release page for the Library and the Index. These steps read pages only.
2. [Packet 011](../tasks/adr-0084-task-011-logo-frame.md), V1 to V7: the logo frame. V6 turns the network off for a short time. Its cleanup turns it on again.
3. [Packet 024](../tasks/adr-0084-task-024-artist-feed-image.md), V1 to V3: the image of an artist feed. Open one album page of the artist first, so the app stores the image.
4. [Packet 020](../tasks/adr-0084-task-020-discover-selection.md), V1 to V6: selection on Discover. V4 downloads three releases.
5. [Packet 022](../tasks/adr-0084-task-022-download-queue.md), V1 to V6: the download queue and its progress item. V3 sets an unreachable MusicIndex endpoint. Its cleanup restores the endpoint.
6. [Packet 025](../tasks/adr-0084-task-025-playlist-selection.md), V1 to V7: selection on the playlist page. Use the test playlist.

#### Record

For each packet, tell the orchestrator which checks passed and which failed, with what you saw. The orchestrator then closes each gate, or writes a correction.

#### Rollback

To return to schema 19, close the app, revert the commits of the run, and copy `~/v4vmm-before-schema-20.sqlite` back to the database path.

### Part B: Deferred Requirements

This part holds the draft operator walk of the triage of 2026-10-20, corrected for three run decisions. The check of §15 is walkable at schema version 19. §23 V2 and V3 wait for packet 017 and stay out of this walk. §8 and §9 cannot run as written and stay out of this walk.

Each step below is for a person at a Linux desktop session.

#### Setup

1. Close v4vmm.
2. Back up the database with `sqlite3 "$db" ".backup '/tmp/v4vmm-before-walk.sqlite'"`.
3. Type `cargo build --bin v4vmm` in a terminal.
4. Start the app with `target/debug/v4vmm`.
5. Keep the window wider than 1100 px.
6. Do each section in Light and in Dark.
7. Do each section at the usual width and at narrow width.

Cleanup: close the app. To remove the database changes, copy the backup file back to `$db`.

#### Track Page (§22, §26, §27, §30 V2)

1. Click the toolbar search.
2. Type a release name and push Enter.
3. Open an Index release, then click a track row.
4. Find an Index track. The track must have no website and no Nostr key of its own.
5. Examine the header. (§22 V1)
   - Wrong if: the header shows a website action or a Nostr action.
6. Examine the page below the header. (§22 V1)
   - Wrong if: the page shows the feed website as a track value.
   - Known result: the Index page shows no feed section today. Record this result.
7. Open Tracks in the sidebar. Open a Library track. Its feed must have a website.
8. Examine the feed section below the header. (§22 V1, V2)
   - Wrong if: the section does not name the feed.
   - Wrong if: the section repeats a website of the track.
9. Open a track that has no description of its own. (§22 V3)
   - Wrong if: the page shows a description.
10. Open a track that has no date of its own. (§26 V1)
    - Wrong if: the summary shows "Release Date".
11. Open a track that has its own date. (§26 V2)
    - Wrong if: "Release Date" is missing or shows a different date.
12. Open a Library track that has no date of its own. (§30 V2)
    - Wrong if: "Feed publication date" is missing when the feed has a date.
13. Open an Index track that has its own image. (§27 V1)
    - Wrong if: the page shows the release image.
14. Open an Index track that has no image of its own. (§27 V1)
    - Wrong if: the page shows no image.
15. Make the window narrow. Examine steps 5 to 14 again. (§22 V4, §26 V3, §27 V2)
    - Wrong if: an element moves out of its position.
    - Wrong if: a text is clipped.

#### Release Page (§3, §15, §30 V1)

This section walks §3, §15 and §30 V1 together, by the orchestrator decision of 2026-10-20.

1. Open Releases in the sidebar. Open a Library release that has a channel date.
2. Examine the facts. (§30 V1)
   - Wrong if: "Published" and "First track published" show as one fact.
   - Wrong if: a fact does not name its source.
3. Do the "Metadata Hydration" runbook section with its disposable config. (§3)
4. Record the release result and the track result separately in the checklist.
   - Wrong if: a stored fact is blank after the Index request fails.
5. Do packet 020 steps 4 to 11. Expect schema version 19 in step 2. (§15)
   - Wrong if: a title shows "(old)" after the check.
   - Wrong if: the new title shows only after a navigation.
6. Make the window narrow. Examine steps 2 and 5 again. (§30 V3)
   - Wrong if: a text is clipped.
7. Cleanup: restore the database backup made in Setup. Step 5 wrote new values into the stored values.

#### Playlist Page And Needs Attention (§13, §14, §16, §18)

These steps send HTTP requests to the feed hosts. They write RSS values into the database.

1. Open Playlists in the sidebar. Open a playlist.
2. Examine the "Check RSS" button and its state. (§13 V1)
3. Click "Check RSS". (§13 V2)
   - Wrong if: the page does not show the progress in place.
4. During the check, click Play. (§13 V2)
   - Wrong if: playback waits until the check stops.
5. Examine the report after the check. (§13 V3, §14 V1)
   - Wrong if: a row does not give the feed, field, old value, new value and time.
   - Wrong if: a stale feed has no "Open podping.me" button.
6. Examine the button adjacent to "Open podping.me". (§18 V1)
7. Click "Copy feed URL". Paste the text into an editor.
   - Wrong if: the pasted text is not the feed URL.
8. Examine a new track and a removed track. (§14 V2)
   - Wrong if: the new track has no download action.
   - Wrong if: the removed track has no mark.
9. Examine the row error of the removed track. (§16 V4)
10. Click "Remove from all playlists". Examine the confirmation, then click "Cancel".
    - Wrong if: the confirmation does not list each playlist.
11. Open Needs attention in the sidebar. (§16 V1)
    - Wrong if: "Route out of date" has a button.
    - Wrong if: "Removed from feed" has no "Confirm" button.
12. Click "Remove from library" on the removed track. Then click "Cancel".
    - Wrong if: no confirmation shows.
13. Click "Confirm" on the removed track. (§16 V3)
    - Wrong if: the row does not change in place.
14. Open the playlist page again. Do not start a check. (§18 V2)
    - Wrong if: the confirmed row shows an error.
15. Open Show. Examine the Source card. (§16 V2)
    - Wrong if: the card counts a removed track as ready.
    - Wrong if: the detail does not name both states.
16. Make the window narrow. Examine the report again. (§14 V3)
    - Wrong if: stacked text shows only an ellipsis.
17. Cleanup: restore the database backup made in Setup. The check wrote RSS values, and step 13 confirmed a removal.

#### Settings And Column Scroll (§1)

1. Open Settings. Open a pane that is longer than the window.
2. Use the wheel, the scrollbar and the keyboard to go to the end. (§1 Settings)
   - Wrong if: you cannot go to the last line.
3. Open Music. Open a feed page, a playlist page and an Index release page that overflow.
4. Click in the detail column. Push Page Down until the end. (§1 Music)
   - Wrong if: the parent scrolls in place of the column.

#### Sidebar Footer Reports (§7, §17, §34)

1. Do the packet 014 isolated procedure, steps 1 to 14. (§7)
2. In step 4, open Tracks in the sidebar. Then click a.wav.
   - Wrong if: the footer claims that MusicIndex is unavailable.
   - Wrong if: the footer claims that the data is stored.
   - Wrong if: the selected track changes during the delayed failure.
3. Do the packet 014 fixture copy of packet 066 task 014 V2. (§34 V2)
   - Wrong if: a "Conversion" row stays after the successful retry.
4. Copy the fixture audio files before you continue. (§17)
5. Click "Update n files" in the sidebar footer. (§17 V1)
   - Wrong if: the popup does not list each file with its title, album and frames.
6. Start a show with one of the files. Open the popup again. (§17 V2)
   - Wrong if: the file has no mark "In use by the show".
7. Click "Write Tags". Examine the button count.
   - Wrong if: the count is not equal to the files in use.
8. Make the window narrow while the popup is open. (§17 V3)
   - Wrong if: stacked text shows only an ellipsis.
9. Cleanup: delete the fixture audio files copied in step 4. Step 7 wrote tags into them.

#### Inspect Sources (§24, §25, P3)

Use an isolated database copy for each step. Do not write to the Library.

1. Open a Library track with no item description. Open "Inspect sources". (§24 V4)
   - Wrong if: the grid shows no "Album description" row.
   - Wrong if: the row does not name its owner.
2. Click "⋯", then "MusicBrainz lookup". (§25 V1)
   - Wrong if: the channel website row does not show the release-group homepage.
   - Wrong if: a release with a license shows no "License" row.
3. Examine each other relation row. (§25 V2)
   - Wrong if: a relation row offers a write.
4. Apply the homepage and the license on a test copy. Open the file in an external tag reader. (P3)
   - Wrong if: `WOAR` or `WCOP` holds text that is not a plain URL.
5. Make the window narrow. Examine steps 1 to 3 again. (§24 V4, §25 V4)
6. Cleanup: discard the isolated database copies used in this section.

#### Search (§23 V1, V4)

§23 V2 and V3 wait for ADR 0084 packet 017 and stay out of this walk.

1. Click the toolbar search. Type "Survival Guide" and push Enter.
2. Examine the result rows. (§23 V1)
   - Wrong if: no row reads `Tracks matching "Survival Guide"`.
   - Wrong if: the row looks like an artist identity.
3. Make the window narrow. Examine the row again. (§23 V4)
   - Wrong if: the row text is clipped.

#### Startup Fixture Reports (P1) And macOS Keys (P4)

1. Do runbook task 004 steps 1, 2, 4, 5 and 6. (P1)
2. In step 2, open the fixture playlist from Playlists in the sidebar.
3. Do not use Music Play for a Show check.
4. On a macOS desktop, push `cmd-[` and `cmd-]` outside a text box. (P4)
   - Wrong if: Back or Forward does not occur.
5. Push the same keys in a text box.
   - Wrong if: indent or outdent does not occur.
