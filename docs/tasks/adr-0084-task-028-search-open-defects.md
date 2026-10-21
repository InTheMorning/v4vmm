# ADR 0084 Task 028: The Open Defects Of Search

Status: Implemented - 2026-10-21. The visual checks V1 to V4 are open. The operator walks them in the final walk of the run of 2026-10-20.

## Goal

Correct five small defects of search and of saved searches. The run of 2026-10-20 recorded each one as an open finding.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 2 (search results in the list column), Decision 3 (saved searches), Decision 7 (the name level), Decision 10 (selection and the keyboard) and Decision 12 (the logo frame).
- Packets [010](adr-0084-task-010-search-results-list-column.md) and [012](adr-0084-task-012-saved-searches.md), with the walk fixes of 2026-10-21.
- Operator decision of 2026-10-21: one packet for the open defects of the run.
- [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, "Every fix gets a guard", and "Column text does not call `truncate()`".
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md).

## Recorded Facts - 2026-10-21

- D1: `index_track_result_display` in `src/application/queries/search.rs` gives the row id `index-track:<feed>:<track>`, or `index-track:<track>` when the hit has no feed GUID. `SearchResultTarget::from_row_id` in `src/view_models/search_results/results.rs` splits the text after the prefix on its first colon. A track GUID with a colon and no feed GUID, for example `urn:uuid:x`, thus gives the feed `urn` and the track `uuid:x`. The row opens a wrong key.
- D2: the search list in the list column (packet 010) and the saved search page in the detail column (packet 012) hold no scroll handle. When the keyboard moves the focus to a row below the visible area, the list does not scroll that row into view. The Releases list does this with `releases_list_scroll.scroll_to_item` in `src/library/app_impl.rs`.
- D3: on 2026-10-21 a search for "matt finlay" showed the Library name row "Releases that name Matt Finlay" with the monogram "AR". The Index name row "Releases that name Matt Finlay (wavlake)" showed an image. The Artists list shows each Library name row with the logo frame of its releases (`artist_name_row` and `newest_name_level_images` in `src/view_models/library.rs`).
- D4: on 2026-10-21 a search for "haleen" showed a second Index feed "Haleen" (GUID `fe80a515-2826-4a46-aa37-b27f38d3c30f`) with the monogram "AR". MusicIndex gives it `feed_image_url` `https://d12wklypp119aj.cloudfront.net/image/fe80a515-2826-4a46-aa37-b27f38d3c30f.jpg`. That URL answers `200` with a JPEG of 1875 × 1000 pixels and 1.7 MB. The other Index feed rows of that search showed their images. The cause is not known.
- D5: when MusicIndex is unavailable and a search tab has Library rows, the page shows the Library rows. It shows no note about the failure of the Index part. A rejected query shows a note after the rows (walk fix of 2026-10-21, `index_notice()` in `src/view_models/search_results/mod.rs`).

## Design

1. D1: a row id names its parts so that no GUID text can move a split. A track hit with no feed GUID gets its own prefix, for example `index-track-guid:<track>`. `from_row_id` reads each prefix. A track GUID with colons opens its own key.
2. D2: the search list and the saved search page each hold one scroll handle. A keyboard focus move scrolls the focused row into view, by the method of the Releases list. A tab change scrolls to the top.
3. D3: a Library name row in search shows the same art as its row in the Artists list: the logo frame of the newest images of its releases. If the search row composite can take that art, use it. Else use the newest image as the thumbnail, and record the difference.
4. D4: find the cause. If it is in v4vmm, for example a size limit, a decode limit or an image type check, correct it at its owner. If it is outside v4vmm, record the cause in this packet and change no code.
5. D5: the Index part fails, and the tab has Library rows. The page then shows the failure note after the rows, as a rejected query does. The note names MusicIndex and keeps "Show details". The retained action and its banner do not change.

## Required Changes

1. D1: the new prefix, the parse and the tests.
2. D2: the two scroll handles, the scroll on a focus move, and the scroll to the top on a tab change.
3. D3: the art of a Library name row in search, with a test.
4. D4: the cause, and a correction with a test if the cause is in v4vmm.
5. D5: the failure note after the Library rows, with a test.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_search_open_defects_` for unit tests beside the owning code.

| Case | Required proof |
|---|---|
| R28-01 | A track hit has no feed GUID and the track GUID `urn:uuid:x`. `from_row_id` reads its row id as a track key with no feed and the track GUID `urn:uuid:x` |
| R28-02 | A track hit with a feed GUID gives the same row id and key as before |
| R28-03 | The view model or the app layer gives the index of the focused row of the search list and of the saved search page. A focus move asks for a scroll to that index. A unit test or a guard proves the call |
| R28-04 | A Library name row of search carries the art or the image of its newest Library release |
| R28-05 | D4: a test of the correction, or the recorded cause outside v4vmm |
| R28-06 | With Library rows and an unavailable MusicIndex, the view model gives the rows and a failure note. With no Library rows, it gives the present empty state |
| R28-07 | Each test of packets 010 and 012 still passes |

## Visual Acceptance Criteria

These are for the operator, in the final walk of the run. No test proves them.

- V1: search "hey", Tracks tab. Click the first row, then press Down until the focus passes the last visible row. The list scrolls, and the focused row stays visible. Do the same on the saved search page.
- V2: search "matt finlay", Artists tab. The row "Releases that name Matt Finlay" shows the art of its releases, the same as its row in the Artists list.
- V3: search "haleen", Artists tab. The second "Haleen" feed row shows its image, or the packet records why it cannot.
- V4: set an unreachable MusicIndex endpoint in Settings, then search "hey". The Library rows show, and a note after them tells that MusicIndex was not available. Restore the endpoint.

## Exclusions

- No change to the query text that v4vmm sends (Stophammer request 10).
- No change to the render cost of the search rows (responsiveness item 18).
- No change to the retained action of an unavailable MusicIndex.

## Files To Inspect

- `src/application/queries/search.rs`, `src/view_models/search_results/{mod,results,rows,paged_tab,saved_search}.rs`.
- `src/app/search_selection.rs`, `src/app/search_dispatch.rs`, `src/app/saved_search_dispatch.rs`, `src/library/app_impl.rs`.
- `src/ui/shells/search_results_inspector.rs`, `src/ui/shells/search_result_rows.rs`, `src/ui/composites/music_column_scroll.rs`.
- The image load path of a row thumbnail (find it from `thumbnail_href` in `src/ui/`).

## Checks

```bash
cargo test --lib adr_0084_search_open_defects_
cargo test --lib adr_0084_search_list_
cargo test --lib adr_0084_saved_search_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-028-search-open-defects.md`
- Packets 010 and 012, with their walk fixes
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each text, image and availability. The screen only composes. Use named tokens.
- Tests use recorded values. No test sends a request to a live service.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app.

Do not touch:
- The database schema.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R28-01 to R28-07 has a passing test, a guard or a recorded cause.
- Each command in "Checks" is Green.
- The packet has an "Implementation Result - 2026-10-21" section and an "Operator Visual Check" section for V1 to V4.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-21

### What changed

- D1: `index_track_row_id` in `src/view_models/search_results/results.rs` makes the row id of each Index track hit. A hit with a feed GUID keeps `index-track:<feed>:<track>`. A hit with no feed GUID gets `index-track-guid:<track>`. `SearchResultTarget::from_row_id` reads each prefix.
- D1, continued: `TrackKey::index_feed_guid` gives the feed GUID of a key. The open of an Index track page now gets the feed GUID and the track GUID from the key. It does not split the target text. Before, the split of the target `urn:uuid:x` also gave the feed `urn`. `split_index_track_target` and its test are deleted.
- D2: `TopApp` holds `search_list_scroll` and `saved_search_scroll`. Each row of the search list is now a direct child of the scroll container, in the order of the view model rows. The note of the Index part comes after the rows.
- D2, continued: `SearchResultsInspectorPageVm::focused_row_index` gives the index of the focused row among all the rows of the tab. After a key moves the focus, `move_search_selection_focus` calls `scroll_to_item` with that index. A tab change sets the scroll offset to the top.
- D3: `name_level_art` in `src/view_models/library.rs` gives the logo frame art of a name row. The Artists list row and the search row use this one function. `ArtistSearchFacts.name_level_art` replaces `name_level_keys`. The search name row carries the art in `ArtistResultDisplay.art` and `SearchResultRowVm.art`.
- D3, continued: the app layer loads the art through `collection_art_image`, the path of the Artists list. The row renderer shows the `CollectionArt` composite for a row with art, and the `Thumbnail` composite for each other row. A Library name row with art does not take the image of a merged Index name row.
- D5: `index_notice` gives the failure note of the Index part for each failure, when the tab has rows. Before, it gave the note only for a rejected query. The flag `query_rejected` of `SearchFailureDisplay` had no other reader, and it is deleted. The retained action and its banner do not change.

### D4: the recorded facts and the cause

The cause is not found. This packet changes no code for D4. These facts are recorded on 2026-10-21:

- `GET https://api.musicindex.org/v1/search?q=haleen&type=publisher` gives two rows. The row of the feed `fe80a515-2826-4a46-aa37-b27f38d3c30f` has the `feed_image_url` of the packet. The other row is the feed `64569161-a8f7-52e5-8aae-9e0385043149`, and the Library artist feed row takes its place. Thus the row of the second "Haleen" has its image URL.
- The image URL answers `200` with `content-type: image/jpeg` and 1,711,199 bytes. The JPEG is baseline, 1875 × 1000 pixels.
- A local probe sent the same bytes to `ImageCache::fetch_static_blocking` from a local server. The cache accepted the type, made a 512 × 273 JPEG of 33,209 bytes, and gave an image. The probe is not kept. The decode and the downscale take about 1.1 seconds in a debug build.
- The thumbnail cache of the operator holds the entry of this URL. The app wrote it on 2026-10-21 at 07:17:26 EDT, in the walk. The entry is the 512 × 273 JPEG. Thus the app downloaded and decoded the image in that walk.
- `Thumbnail` shows the monogram "AR" only when the row has no image. `index_remote_detail_hero_image` gives no image while the load runs.

Two possible causes stay in v4vmm. Neither one is proved:

- `index_remote_detail_hero_image` keeps a failed load for the session. A load that gives no image stays `Loaded(None)`, and a command error leaves `Loading`. The app does not try again. The header color of a feed page can write the cache entry through `fetch_cover_color_blocking`, and this does not change the state of the row.
- The row showed "AR" for the 1 second of the decode, before the image arrived.

V3 below tells the operator how to separate the two causes. The cache now holds the image, so a new session reads it from the disk.

### Tests

| Case | Proof |
|---|---|
| R28-01 | `adr_0084_search_open_defects_r28_01_track_guid_with_colons_opens_its_own_key` |
| R28-02 | `adr_0084_search_open_defects_r28_02_track_with_feed_keeps_its_row_id` |
| R28-03 | `adr_0084_search_open_defects_r28_03_focused_row_index_counts_each_row`, and the guard `adr_0084_search_open_defects_r28_03_focus_move_scrolls_to_the_focused_row` |
| R28-04 | `adr_0084_search_open_defects_r28_04_library_name_row_carries_the_artists_list_art` |
| R28-05 | The recorded facts in "D4: the recorded facts and the cause" |
| R28-06 | `adr_0084_search_open_defects_r28_06_unavailable_index_gives_a_note_after_the_library_rows` |
| R28-07 | `cargo test` is Green. Each `adr_0084_search_list_` test and each `adr_0084_saved_search_` test passes |

### Deviations from the packet text

- D1 also changes the open of an Index track page. Without this change, the row opens the correct key, but the page asks `MusicIndex` for the feed `urn` and the track `uuid:x`.
- D2 uses `scroll_to_item`, the method of the Releases list. Thus the rows of the search list moved out of their inner wrapper. The right padding keeps the scrollbar clearance and the column inset.
- D5: the last part of `adr_0084_search_list_rejected_query_shows_a_note_with_the_library_rows` asked for no note after a connection error. D5 changes that rule, so that part is deleted. R28-06 proves the new rule.
- The guards `adr_0024_loading_shape_readiness_gate_is_locked` and `adr_0047_task_014_search_results_inspector_shell_contract` now name the new prefix constants and the new argument `leading` of `render_result_row`.
- For D4, one read-only `GET` went to the `MusicIndex` search, in addition to the image URL. No test sends a live request.

### Open points

- D4 is open. V3 gives the check.

## Orchestrator Review - 2026-10-21

Pass, with two corrections that the orchestrator made.

1. The note after the search rows is now a child of the scroll container, so it could shrink under the rows. It now calls `flex_shrink_0()`.
2. A submitted search and each run of a saved search now scroll their list to the top. Before, only a tab change did this.

The guard `adr_0084_search_open_defects_r28_03_focus_move_scrolls_to_the_focused_row` now also pins the two corrections.

The review found two small points, and the phase plan records them:

- `index_target` in `src/view_models/workspace/nav.rs` makes one text from the feed GUID and the track GUID. Two keys can thus give the same text. No code divides that text now, and a collision needs a feed GUID that is the start of a track GUID.
- The renderer selects the artist frame of the art of a name row, because the row has art. The Artists list does the same.

## Operator Visual Check

The visual checks V1 to V4 are open. The operator walks them in the final walk of the run of 2026-10-20.

Necessary state: a desktop session, a reachable `MusicIndex` endpoint, a Library with one or more downloaded releases, and one saved search of "hey".

1. Build and start the binary:

   ```bash
   cd ~/build/v4vmm
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   ```

2. V1: in the toolbar search, type `hey` and push Enter. Open the Tracks tab, and click the first row. Push Down until the focus goes below the last visible row.
   - Make sure that the list scrolls and that the focused row stays fully visible.
   - Push Home, then push End. Make sure that the first row and then the last row show.
   - A focused row out of view is wrong.
3. V1, continued: open the Releases tab. Make sure that the list starts at the top. Open the saved search "hey" in Discover, and do step 2 again on its page.
4. V2: search `matt finlay`, and open the Artists tab. Look at the row "Releases that name Matt Finlay". Then open the Artists destination, and find the same name row. Make sure that the two rows show the same art. The monogram "AR" on the search row is wrong when the Artists list row shows art.
5. V3: quit the app, then start it again. Search `haleen`, and open the Artists tab. Look at the second "Haleen" row.
   - When it shows the image, the earlier "AR" came from a load that failed in that session. Record "D4: no retry after a failed load".
   - When it shows "AR", record "D4: the row does not show a cached image". Both results go to the run record.
6. V4: open Settings. Write the MusicIndex endpoint `http://127.0.0.1:9`, and save it. Search `hey`.
   - Make sure that the Library rows show.
   - Make sure that a note after the rows tells that the MusicIndex search was not available.
   - Click "Show details". Make sure that the report shows.
   - A page with only the Library rows and no note is wrong.

Cleanup: in Settings, write the earlier MusicIndex endpoint again (`https://api.musicindex.org` on this computer), and save it.
