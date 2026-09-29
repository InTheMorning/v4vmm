# ADR 0075 Task 047: Search Rows From Summary Fields

Status: Implemented - 2026-09-29. Mechanical checks Green. Visual gate open and paused.

## Goal

The Index search draws each result row from the summary fields of the search response. It sends no detail request for each hit.
A row sends its detail request when the operator opens it.

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) section 6 and Decision C, and packets 017 and 018: named request profiles and one shared request owner.
- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decision 6: feed owner text is not an artist.
- [MusicIndex API change request](../plans/musicindex-api-change-request.md) change 1, live since 2026-09-23.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, and "Current-view state must update in place".

## Recorded Facts - 2026-09-29

- The deployed contract is version `0.2.0`. `SearchResponseItem` declares `entity_type`, `entity_id`, `rank`, `quality_score`, `title`, `feed_guid`, `feed_title`, `feed_image_url`, `track_image_url`, `release_artist`, `release_artist_source`, `track_artist`, `pub_date`, `duration_secs`, `episode_count` and `href`.
- A live hit omits each field whose value is null. A feed hit for "Monster" gave no `feed_guid` and no `href`. Its `entity_id` is the feed GUID.
- `api::SearchResult` decodes only `entity_type`, `entity_id`, `feed_guid` and `quality_score`.
- `fetch_index_feed_result_rows` and `fetch_index_track_result_rows` in `src/application/queries/search.rs` send one search each, then one detail request for each hit.
  With `limit=20`, one search sends a maximum of 42 requests. The [request baseline](../notes/adr-0075-request-and-write-baseline.md) records the sequence.
- `index_feed_display` shows the title, `release_artist`, the track count, `publisher_text` and `image_url`. `index_track_display` shows the title, `track_artist`, `release_artist`, `feed_title` and `image_url`.
- The row keeps the fetched detail. The Index feed and track detail pages read that retained detail (`index_feed_detail` and `index_track_detail` in `src/view_models/search_results/mod.rs`).
- `api::Client::search` sends `fuzzy=true`. The contract declares only `q`, `type`, `limit` and `cursor`.

## Required Changes

### 1. Decode The Summary

- Add each declared summary field to `api::SearchResult` as an optional value. An omitted field decodes as absent.

### 2. Rows From The Summary

- A feed row uses `title`, `release_artist`, `episode_count` and `feed_image_url`. It shows no `publisher_text`, because the summary does not carry it and ADR 0077 Decision 6 does not make it an artist.
- A track row uses `title`, `track_artist`, `release_artist` and `feed_title`. Its artwork is `track_image_url`, else `feed_image_url` (ADR 0075 Decision C).
- A feed hit without `feed_guid` uses `entity_id` as the feed GUID.
- The Index name candidates of packet 006 use `release_artist` and `track_artist` of the summary.
- The search sends no detail request. One search sends two requests: one feed search and one track search.

### 3. Detail On Open

- Opening a feed row or a track row sends its detail request through the shared request owner of packet 018, with the existing named profile.
- The detail page shows a loading state from its view model, then the detail. A failure gives a display state, not a transport error.
- A result for an earlier row does not replace the page of a later row. Follow `publisher_page_result_is_current` in `src/app/publisher_dispatch.rs`.
- A second open of the same row inside the reuse window of packet 018 sends no request.

### 4. The Undeclared Parameter

- Stop sending `fuzzy=true`. The contract does not declare it. Record in the implementation result if a live search gives different rows without it.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_search_summary_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R47-01 | A recorded feed hit and a recorded track hit decode each summary field. An omitted field decodes as absent |
| R47-02 | A search with 20 feed hits and 20 track hits sends two requests. Use the packet 016 measurement form |
| R47-03 | A feed row exposes the title, the artist, the track count and the feed artwork, and no `publisher_text` |
| R47-04 | A track row with `track_image_url` uses it. A track row without it uses `feed_image_url` |
| R47-05 | A feed hit without `feed_guid` opens the feed of its `entity_id` |
| R47-06 | Opening a row sends one detail request with the existing profile. A second open inside the reuse window sends none |
| R47-07 | A detail result for row A that arrives after the page changed to row B does not change the page of row B |
| R47-08 | The search request holds no `fuzzy` parameter |
| R47-09 | Name candidates come from the summary `release_artist` and `track_artist` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an Index search shows feed rows and track rows with titles, artists and artwork, faster than before.
- V2: opening a row shows a loading state, then the detail, in place.
- V3: a fast open of row A and then row B shows the detail of row B.
- V4: normal and narrow widths show each element in its place, in Light and Dark themes, with no clipped text.

## Exclusions

- No change to the Library search.
- No paging of the Index search.
- No change to the detail pages, except that they load on open.
- No stored value. A search result is not stored.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/api.rs`: `SearchResult`, `SearchResponse`, `Client::search`.
- `src/application/queries/search.rs`: `fetch_index_search_result_rows`, `fetch_index_feed_result_rows`, `fetch_index_track_result_rows`, `index_feed_display`, `index_track_display`, the name candidates.
- `src/application/request_profiles.rs` and the packet 018 shared owner.
- `src/view_models/search_results/`: `mod.rs`, `index_detail.rs`.
- `src/app/search_dispatch.rs`, `src/app.rs`: the Index feed and track detail arms.
- `src/app/publisher_dispatch.rs`: the stale-result pattern.
- `docs/notes/adr-0075-request-and-write-baseline.md`: the measurement form.

## Checks

```bash
cargo test --lib adr_0075_search_summary_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

This check needs a desktop session and network access to `api.musicindex.org`. It only reads
pages and requests images over the network. It makes no local or database change, so it needs
no cleanup step. Do not remove `/tmp/v4vmm-governance.ie6k8TQf`. This check does not use it.

**Setup**

1. Assemble the desktop binary and open it:

   ```bash
   cargo build --bin v4vmm && target/debug/v4vmm
   ```

   Do this step first. A prior use of `cargo test` can keep a GPUI test-support binary at
   `target/debug/v4vmm`. Do each next step in the open app.
2. Open Settings. Make sure the MusicIndex endpoint field holds a value, and the service answers.

**V1 - rows show titles, artists and artwork, faster than before**

3. Open Music. Use toolbar search with a query that returns feeds and tracks.
4. Look at the Feeds tab and the Tracks tab. Each row must show a title and an artist or a feed
   name in about one second.
   - This result is incorrect: a row stays blank, or the rows fill in one at a time with a
     pause between them.
5. Compare the artist text of two track rows from the same feed. The two rows can show
   different names, because each row states its own track artist, not the feed's artist.

**V2 - opening a row shows a loading state, then the detail, in place**

6. Open a feed row from the Feeds tab. The content pane must show a brief loading state. Then
   it must show the feed's title, artwork and track list, in the same pane.
   - This result is incorrect: the pane shows a blank or placeholder page with no loading
     state. It is also incorrect when the pane stays on the loading state without showing the
     feed's own detail.
7. Go back to the search results. Open a track row from the Tracks tab. The content pane must
   show a brief loading state, then the track's title, artist and artwork.

**V3 - a fast open of row A, then row B, shows row B's detail**

8. From the search results, open a feed row. Before it completes loading, go back and open a
   different feed row.
9. Wait for loading to complete. The content pane must show the second feed's detail.
   - This result is incorrect: the pane shows the first feed's detail, or a mixture of the two.
10. Repeat steps 8 and 9 for two different track rows.

**V4 - normal and narrow widths, Light and Dark**

11. At about 1400 pixels wide, open a feed row and a track row. Record the position of the
    title, artist, artwork and track list on each page.
12. Narrow the window to about 560 pixels. Open the same two rows again.
    - This result is incorrect: an element overlaps a different element, is clipped, or moves
      out of its position from step 11.
13. Repeat steps 11 and 12 in Dark theme (Settings > Appearance).

Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0075-task-047-search-rows-from-summary-fields.md`
- ADR 0075 section 6 and Decision C, and the packet 017 and 018 documents
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": decode the summary, draw rows from it, load the detail on open, and stop the undeclared parameter.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use recorded JSON. No test sends a request to `api.musicindex.org`.
- The view model gives each label, loading state and failure state. The screen only composes. A screen uses the scaled tokens of ADR 0039 and existing shared composites.
- Background work uses the ADR 0040 runtime. Never call `cx.spawn` from a screen.
- Treat each MusicIndex response as untrusted input.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The Library search and the Library route.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R47-01 to R47-09 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0075_search_summary_`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo check --all-targets`
- `cargo build --bin v4vmm`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A detail page needs a field that the summary does not carry, and no open-time request can supply it.
- The shared request owner cannot serve an open-time request without a change to another profile.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-09-29

No "Stop and report" condition happened. The detail page needed no field the summary omits.
The shared request owner served each open-time request with its existing profile. No change
touched a "Do not touch" file.

### 1. Files Changed

- `src/api.rs`: `SearchResult` decodes each declared summary field of the `0.2.0` contract:
  `rank`, `title`, `feed_title`, `feed_image_url`, `track_image_url`, `release_artist`,
  `release_artist_source`, `track_artist`, `pub_date`, `duration_secs`, `episode_count` and
  `href`. It keeps the existing `entity_type`, `entity_id`, `feed_guid` and `quality_score`. One
  new test, R47-01, decodes the recorded feed hit and track hit from the packet and checks each
  omitted field decodes as `None`.
- `src/application/queries/search.rs`:
  - `fetch_index_feed_result_rows` and `fetch_index_track_result_rows` build each row from the
    search response's own summary fields. Neither sends a detail request for a hit, and neither
    sends `fuzzy=true`.
  - Two new functions, `index_feed_result_display` and `index_track_result_display`, build a
    row from one `SearchResult` hit. `index_track_result_artwork_url` picks the track's own
    image, then the feed's image, by building a bare `TrackView` and calling the packet 048
    method `TrackView::display_artwork_url`.
  - `index_artist_candidate_from_feed` and `index_artist_candidates_from_track` read the
    summary `release_artist` and `track_artist` fields, not a fetched detail.
  - Two new commands, `FetchIndexFeedDetail` and `FetchIndexTrackDetail`, send the detail
    request for one row. Each sends its request only when the operator opens the row. Each
    reuses the helper function the search loop called before this packet (`owner_fetch_feed`,
    `fetch_index_track_detail`). Each keeps its named profile and its packet 018 reuse behavior.
    `FetchIndexTrackDetail` builds a `TrackView` from the fetched `api::Track`. That is the same
    rich projection the detail page always used.
  - The old function `index_track_display`, which attached a fetched `TrackView` to a row
    during search, is removed. `index_feed_display` (shared with the Recent Feeds route,
    packet 017) is unchanged.
  - New tests cover R47-02 through R47-06, R47-08 and R47-09, using the file's existing local
    HTTP fixture.
- `src/app/search_dispatch.rs`:
  - Two new types, `IndexFeedDetailState` and `IndexTrackDetailState`, hold the loading
    lifecycle of the mounted Index feed or track detail page. Each boxes its loaded view to
    keep the enum small.
  - `fetch_index_feed_detail_page` and `fetch_index_track_detail_page` start the open-time
    fetch and dispatch it through `present_command`, the same ADR 0040 pattern
    `publisher_dispatch.rs` and `name_match_dispatch.rs` already use.
  - `index_feed_detail_result_is_current` and `index_track_detail_result_is_current` follow
    `publisher_page_result_is_current`: an older fetch's result does not replace a page the
    operator already navigated away from (R47-07).
  - `restore_index_feed_detail_for_nav` and `restore_index_track_detail_for_nav` restart the
    fetch when a breadcrumb or a history move restores the page and its state does not match.
  - `handle_index_feed_result_selected` and `handle_index_track_result_selected` start the
    open-time fetch after pushing the navigation entry. A track reached from the name-match
    page skips the fetch, because that page's own fetch (packet 006) already carries the
    track's full detail.
  - `index_feed_detail_display` and `index_track_detail_display` select which display state to
    show: the loaded fetch, a failure report, or a loading placeholder.
- `src/view_models/search_results/index_detail.rs`: `IndexDetailDisplay` gains four
  constructors, `loading`, `failed`, `loaded_feed` and `loaded_track`, for the states above.
- `src/view_models/search_results/mod.rs`: `SearchResultsInspectorPageVm::index_feed_detail`
  and `index_track_detail` are removed. They projected a row's retained fetch into a detail
  page. No row retains one after this packet.
- `src/app.rs`: `TopApp` gains `index_feed_detail_state` and `index_track_detail_state`. The
  `IndexFeedDetail` and `IndexTrackDetail` render arms now read the new detail-on-open state.
  They no longer read a cached row. The back-navigation handler calls the two new restore
  functions.
- `src/app/breadcrumb.rs`: the breadcrumb handler calls the same two restore functions.
- `src/application/request_profiles.rs`: one doc comment names the new call site of
  `INDEX_FEED_DETAIL`.
- `src/view_models/search_results/tests.rs`: one test that exercised the removed cached-row
  projection is narrowed to the label lookups that still apply. Four new tests cover the new
  `IndexDetailDisplay` constructors.
- `tests/architecture_tests.rs`: three guards changed, listed in section 6 below.
- `docs/tasks/adr-0075-task-047-search-rows-from-summary-fields.md`: this packet. The Status
  line, this section, and the Operator Visual Check section.

### 2. Tests Run

Each command ran at the repository root.

- `cargo test --lib adr_0075_search_summary_`: 13 passed. No test failed.
- `cargo test`: 1,919 lib tests, 283 architecture tests and 10 doc tests. The doc tests are
  ignored by design. No test failed.
- `cargo test --test architecture_tests`: 283 passed. No test failed.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green. No warning appeared.
- `cargo build --bin v4vmm`: Green.

No test opens a network connection. The file's own local HTTP fixture serves each request.

### 3. Behavior Changed

An Index search sends two requests, one feed search and one track search, and no longer sends
`fuzzy=true`. It no longer sends a detail request for each returned hit. Using the packet 016
measurement form:

| Case | Index HTTP requests before this packet | Index HTTP requests after this packet |
|---|---:|---:|
| One Index search (any number of returned hits) | `2 + F + T`, where `F` and `T` are the returned feed and track hit counts (a maximum of 42 for 20 of each, per the request baseline) | 2 (one feed search, one track search) |
| Opening one feed row | 0 (its detail was already retained from the search) | 1, reused inside the 15-minute window of packet 018 (P18-2) for a second open |
| Opening one track row | 0 (its detail was already retained from the search) | 1, or 0 when it joins a concurrent open of the same row (P18-8) |

Each feed row shows its title, `release_artist`, its track count and its feed artwork, from the
summary alone. It shows no `publisher_text`. Each track row shows its title, `track_artist`,
`release_artist` and `feed_title`, with its artwork from `track_image_url`, then
`feed_image_url` (Decision C). A feed hit with no `feed_guid` opens the feed of its
`entity_id`, unchanged from before this packet.

Opening a feed row or a track row now shows a loading state, then the fetched detail, in the
same page. A failed fetch shows a failure report, not a transport error. A result for a row the
operator already left does not replace a later row's page (R47-07). A track reached from the
Index name-match page (packet 006) shows its already-fetched detail at once. That page sends no
extra request. Its own fetch already carries the full track.

The Index feed detail reached from the Library "recent feeds" list is unchanged. That list
already fetches each row's full detail as it loads. This packet does not touch it.

### 4. Deviations From Task

None of the required changes were skipped or altered. Two implementation choices are not
stated word for word in the packet, so they are recorded here:

- A track opened from the Index name-match page sends no open-time detail request, because
  that page's own fetch (packet 006) already supplies the full track. Required Change 3 covers
  a row from this search. It does not ask for a second fetch of data the app already holds.
- The old row-projection methods `SearchResultsInspectorPageVm::index_feed_detail` and
  `index_track_detail` are removed, not kept unused, under the "Delete dead code" rule in
  `AGENTS.md`.

### 5. Unresolved Concerns

- A row in packet 018's reuse window still shows a brief loading state before its cached detail
  appears. The fetch still runs through the async command runner. The window removes the
  network request. It does not remove the one-frame loading step. V2 and V3 still hold for this
  case.
- `IndexDetailDisplay::feed` and `track` (the row-cached constructors) are unchanged. The
  Library "recent feeds" list and the Index name-match page still use them. Each still retains
  a full fetch on its own rows. Only the Index search's own rows stopped retaining one.

### 6. Guards Changed

Three architecture guards named ADR 0024, ADR 0049 or ADR 0075 packet 018, and asserted the
old per-hit detail fetch. Each keeps its ADR citation and now asserts the packet 047 shape:

- `adr_0024_index_track_detail_uses_rich_track_view_path`: its required literal moved from the
  removed `index_track_display` to `FetchIndexTrackDetail`, which still builds a `TrackView`
  from the fetched `api::Track` before the detail page reads it.
- `adr_0049_inspector_source_ownership_is_guarded`: no longer requires
  `SearchResultsInspectorPageVm::index_feed_detail` and `index_track_detail`. It now fails if
  either reappears, naming ADR 0075 packet 047.
- `adr_0075_request_reuse_index_routes_ask_the_owner`: its required owner call moved from the
  search loop to `FetchIndexFeedDetail` and `FetchIndexTrackDetail`. A new check fails if the
  search loop itself calls `owner_fetch_feed` or `fetch_index_track_detail` (R47-02).

## Orchestrator Review - 2026-09-29

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,919 unit tests, 283 guards, and no warning.

- One Index search sends two requests. Before this packet it sent a maximum of 42.
- A failed open gives a display state with the entity kind and the reason, from `IndexDetailDisplay::failed`.
- The late-result check follows the publisher page and the name-match page.
- The three changed guards keep their ADR citations.
- The live search passes `fuzzy` as `false`. Only the parked `SearchApp` path in `src/application/queries/search.rs` can still send it. The parameter can go when that parked code is deleted.
- A reuse inside the packet 018 window shows a short loading state before the retained detail. The visual check decides if this is acceptable.
