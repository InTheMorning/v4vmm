# ADR 0084 Task 035: A Download Stores The Feed Links Of Its Release

Status: Ready - 2026-10-24. The orchestrator decided Q1 by its recommendation on 2026-10-24 (see "Orchestrator Answers").

## Goal

After a successful download of a `MusicIndex` release, the app stores the feed links of that
release in `feed_publisher_relationships`. This includes a download from a singles group row.
A downloaded single then stays in the singles group of its artist feed, and the group row does
not move. The once-per-session pass at app start repairs each Library release that a download
left with no link.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 6, amended 2026-10-22: a release with
  a confirmed named feed joins the singles group of that feed.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decisions 5 and 6: a named feed, its RSS
  position and its confirmation.
- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decision 5: one row of
  `feed_publisher_relationships` for each publisher entry, with the time of the observation
  that supplied it.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) section 6 and packet 018: the
  shared request owner, its reuse window and `RefreshIntent`.
- [ADR 0040](../adr/0040-async-vm-runtime.md): background work runs through `present_command`.
- [Packet 034](adr-0084-task-034-store-feed-links-on-download.md) decisions D1 and D2: the
  hydration after a download and the once-per-session pass. This packet corrects them.
- [Packet 026](adr-0084-task-026-index-singles-groups.md) V6, which failed on 2026-10-23.
- [Run record of 2026-10-23](../reviews/orchestrated-run-2026-10-23.md), question 2 and operator
  answer 2 of 2026-10-24.
- [AGENTS.md](../../AGENTS.md): "Current-view state must update in place" and "Every fix gets a
  guard".

## Recorded Facts - 2026-10-24

### The defect

- The operator found it on 2026-10-23, in the walk of packet 026 V6. The operator downloaded 37
  `MusicIndex` singles from a singles group row in Releases "Everything".
- After the download, `feed_publisher_relationships` held a link to the artist feed `64569161`
  for 18 Library releases only. "A Greater Call", "Build on Stone" and "A Phoenix Rising" were
  in the download and have no link. "Whispers" was in the Library before and has a link.
- `GET /v1/feeds/{guid}?include=publisher` gives a `music_to_publisher` entry to `64569161` for
  "A Greater Call" (`67ae17fe`). The MusicIndex answer is correct.
- The stored contract `tests/fixtures/musicindex-openapi-0.13.0.json` agrees with the decode.
  `FeedResponse.publisher` is a list of `PublisherResponse`, each with a `direction`. The
  request of the hydration asks for `publisher`. Thus the request and the decode are not the
  cause.

### The download path of a single from a group row

1. `src/library/downloads.rs:60-71` `download_release_plan` makes one job for each
   `MusicIndex` release with `index_release_job_by_guid`. The job has a feed GUID and no feed.
   The Releases action bar and the singles group page use this function.
2. `src/library/downloads.rs:276-279` sends a job with no feed to `read_index_release_job`.
3. `src/library/downloads.rs:393-427` `read_index_release_job` sends `FetchIndexFeedDetail`.
4. `src/application/queries/search.rs:674-692` `owner_fetch_feed` reads the feed through the
   shared request owner with `RefreshIntent::Normal` and the profile `INDEX_FEED_DETAIL`. The
   owner keeps the response for 15 minutes (`src/application/request_reuse.rs:35`,
   `FEED_RESPONSE_REUSE_WINDOW`). This file has no observation recorder, so the kept response
   has an empty receipt list.
5. `src/library/downloads.rs:343-389` `run_index_release_job` sends `SubscribeFeed`. Its
   success closure calls `hydrate_album_identity_by_feed_guid` at line 378.
6. `src/library/app_impl.rs:2435-2453` finds the new feed id and calls
   `hydrate_album_identity` (`:2396-2427`). That function sends `HydrateAlbumIdentity`.
7. `src/application/queries/library.rs:602-606` makes the key
   `RequestKey::feed(provider, feed_guid, LIBRARY_ALBUM_HYDRATION_FEED.include())`. Line 621
   asks the owner with `RefreshIntent::Normal`.

### The root cause

- `INDEX_FEED_DETAIL` and `LIBRARY_ALBUM_HYDRATION_FEED` send the same include list,
  `FEED_FULL` (`src/application/request_profiles.rs:80`, `:106-107` and `:112-113`). Thus the two
  requests have one `RequestKey`. The test `adr_0075_request_profile_value_is_a_stable_identity`
  (`:293-300`) states this on purpose.
- The hydration starts a few seconds after the read of step 4. The owner thus gives the kept
  Index response, with `REUSED_GENERATION` (`src/application/request_reuse.rs:536-543`).
- `src/application/queries/library.rs:650` runs the write block only when
  `generation != REUSED_GENERATION`. The block holds `persist_musicindex_feed`, the
  `upsert_feed_publisher_relationships` call at line 707 and `write_named_feed_positions` at
  line 716. The hydration skips all of them.
- The skip has a correct purpose for a response that the hydration itself wrote before. It is
  wrong for a response that only the Index read got. No caller wrote that response to the
  Library.
- The kept response also has no receipt. Thus it has no observation time. If the write block
  ran on it, line 676 would fail with "the publisher relationship response has no observation
  time". A correct fix thus needs a new, observed request.
- Line 735 then reads `named_feeds` from the table. The list is empty, so the new single gets
  the name level of its artist text, not the feed level. It leaves the group of the feed, and
  the group row moves.

### The download from a release page

- `src/app/search_dispatch.rs:1723-1727` `download_index_feed` makes a job with the feed of the
  open Index release page. That page got its feed from `FetchIndexFeedDetail`, with the same
  key. When the page opened less than 15 minutes before the download ends, the hydration skips
  its writes in the same way.

### Facts that this packet does not prove

- The source of the 18 stored links is not known. Possible sources are an earlier session, the
  pass at app start, a hydration after the 15-minute window, and the RSS check
  (`src/rss/check_apply.rs:359-381`, `set_music_to_publisher_remote`). The fix does not depend
  on it.
- The pass at app start of packet 034 D2 can already repair the 37 singles. The hydration of a
  kept response records no observation, so `feed_guids_with_hydration_observation`
  (`src/db/provider_observations.rs:1120`) does not exclude the feed. The run record does not
  say whether the operator read the database before or after a restart. Step 2 of the operator
  check tells the two cases apart.

## Design

### The trigger of a hydration

`HydrateAlbumIdentity` gets a typed trigger, `AlbumHydrationTrigger`, with three values:

| Value | Caller | `RefreshIntent` |
|---|---|---|
| `PageView` | `hydrate_album_identity_on_view` | `Normal` |
| `AfterDownload` | the four download jobs of packet 034 D1 | `Explicit` |
| `Backfill` | `pump_album_identity_backfill` | `Explicit` |

`hydrate_album_identity_facts` passes the intent of its trigger to
`fetch_feed_with_receipts`. An `Explicit` request never reuses a kept response
(`src/application/request_reuse.rs:142-150`).

An `Explicit` hydration sends a new, observed request. Its generation is
not `REUSED_GENERATION`, so the write block runs, and its receipt gives the observation time.
Its result replaces the kept response, so a later `PageView` hydration reuses a response that
the hydration wrote.

The `continue_backfill` flag of `hydrate_album_identity` is replaced by the trigger. The value
`Backfill` continues the pass.

### The repair of the Library

The pass at app start of packet 034 D2 is the repair. It already selects each Library release
with a feed GUID, no stored row and no observation with an empty publisher list. With the
trigger `Backfill`, its request is always new, so a kept response cannot block it. The repair
needs no new code path, no migration and no operator action other than a restart.

## Decisions

- **D1.** A hydration after a download sends an explicit request. It never reuses a kept
  response. Reason: the download changed the Library, and only an observed request can supply
  the observation time that ADR 0077 Decision 5 requires.
- **D2.** The pass at app start also sends an explicit request. Reason: a kept Index response
  from the same session must not block the repair.
- **D3.** A hydration on a page view keeps `RefreshIntent::Normal`. Reason: a page view must
  not send a request each time. See question Q1.
- **D4.** The trigger is a typed enum, not a boolean. Reason: three callers need three
  behaviors, and a boolean names only one of them.
- **D5.** The hydration keeps one call of `fetch_feed_with_receipts`. The guard of ADR 0075
  packet 018 Part A (`tests/architecture_tests.rs:18899-18909`) stays unchanged.

## Questions

- **Q1.** A `PageView` hydration has the same skip after any other read of the same key in the
  last 15 minutes. One example is the Library track detail read
  (`LIBRARY_TRACK_DETAIL_FEED`, the same include list). Should a later packet change the skip
  rule of `src/application/queries/library.rs:650`? The new rule would skip a write only when
  the hydration itself wrote that response.

  Recommendation: yes, but in a separate packet. This packet corrects the download defect and
  its repair only. The next app start repairs a link that a page view did not store.

## Required Changes

1. `src/application/queries/library.rs`: add `AlbumHydrationTrigger` with its intent method.
   Add the trigger to `HydrateAlbumIdentity` and to `hydrate_album_identity_facts`. Pass its
   intent at line 621.
2. `src/library/app_impl.rs`: replace `continue_backfill` of `hydrate_album_identity` with the
   trigger. `hydrate_album_identity_on_view` passes `PageView`. `pump_album_identity_backfill`
   passes `Backfill`. `run_library_feed_job` and `run_library_track_job` pass
   `AfterDownload`. `hydrate_album_identity_by_feed_guid` passes `AfterDownload`.
3. `src/library/downloads.rs`: no change of the two call sites, because
   `hydrate_album_identity_by_feed_guid` supplies the trigger.
4. `tests/architecture_tests.rs`: change the guards R34-04 to R34-08 and R34-12 so that they
   name the trigger. Add the guard R35-05.
5. Add each test of "Mechanical Acceptance Criteria".
6. Update the doc comments of `hydrate_album_identity`, `HydrateAlbumIdentity` and
   `REUSED_GENERATION` so that they state the trigger rule.

## Mechanical Acceptance Criteria

Each criterion here is mechanical. A unit test beside its owning code has the prefix
`adr_0084_task_035_`. A guard is in `tests/architecture_tests.rs` and has the same case number
in its name.

| Case | Required proof |
|---|---|
| R35-01 | Mechanical, unit test in `src/application/queries/library.rs`. The test first reads the feed `f1` with `FetchIndexFeedDetail`. Then it runs `HydrateAlbumIdentity` with `AfterDownload`. The rows of `feed_publisher_relationships` for feed id 1 are not empty, and `named_feeds` of the result is not empty |
| R35-02 | Mechanical, unit test, same file. With the same first read, `AfterDownload` sends a second request to `/v1/feeds/f1`. The fixture request list has two feed requests |
| R35-03 | Mechanical, unit test, same file. With the same first read, `Backfill` stores the rows, as in R35-01 |
| R35-04 | Mechanical, unit test, same file. After an `AfterDownload` hydration, a `PageView` hydration sends no request and changes no row. This keeps the packet 018 rule R18B-01 for a response that the hydration wrote |
| R35-05 | Mechanical, guard. Situational: it cites ADR 0084 Decision 6 and this packet, and is deleted when that decision is superseded. `hydrate_album_identity_by_feed_guid`, `run_library_feed_job` and `run_library_track_job` pass `AlbumHydrationTrigger::AfterDownload`. `pump_album_identity_backfill` passes `AlbumHydrationTrigger::Backfill`. Its failure message names ADR 0084 Decision 6, this packet and the fix |
| R35-06 | Mechanical, unit test. `AlbumHydrationTrigger::AfterDownload` and `Backfill` give `RefreshIntent::Explicit`. `PageView` gives `RefreshIntent::Normal` |
| R35-07 | Mechanical. The changed guards R34-04 to R34-08 and R34-12 pass. The guard of ADR 0075 packet 018 Part A passes with no change. Each test of `HydrateAlbumIdentity` and of `album_identity_backfill_order` passes |

Each test that uses the shared request owner calls `invalidate_hydration` first, as the
existing tests of the file do. The failure message of R35-05 has this form:

```text
ADR 0084 Decision 6, task 035 D1: a hydration after a download must pass
AlbumHydrationTrigger::AfterDownload. A kept Index response would skip the write of
feed_publisher_relationships. Pass the trigger in this call.
```

## Visual Acceptance Criteria

Each criterion here is visual. Only a person can examine it. No test proves it.

- V1: packet 026 V6 again. Select the Haleen group row in Releases "Everything" and click
  "Download". The progress item shows the MusicIndex singles. After each download, its row
  changes in place. The group row stays at the same position, and each new single stays in
  the group.
- V2: open the Haleen singles group page after the download. Each downloaded single shows
  under the singles of the feed, not under "Matched by name".
- V3: download one `MusicIndex` single from its Index release page. Open the singles group page
  of its artist feed with no other navigation. The single shows under the singles of the feed.

## Exclusions

- No change to the reuse window, the key or the capacity of the shared request owner.
- No change to the skip rule for a `PageView` hydration (question Q1).
- No change to the RSS check in `src/rss/check_apply.rs`.
- No change to the sort key of a group row (run record question 1 has its own fix packet).
- No schema change and no migration.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/application/queries/library.rs` (`HydrateAlbumIdentity`, `hydrate_album_identity_facts`,
  the test fixture and `invalidate_hydration`).
- `src/application/queries/search.rs` (`owner_fetch_feed`, `FetchIndexFeedDetail`).
- `src/application/request_reuse.rs` (`RefreshIntent`, `fetch_feed_with_receipts`,
  `REUSED_GENERATION`).
- `src/application/request_profiles.rs` (`INDEX_FEED_DETAIL`, `LIBRARY_ALBUM_HYDRATION_FEED`).
- `src/library/app_impl.rs` (`hydrate_album_identity`, `hydrate_album_identity_by_feed_guid`,
  `hydrate_album_identity_on_view`, `pump_album_identity_backfill`, `run_library_feed_job`,
  `run_library_track_job`).
- `src/library/downloads.rs` (`download_release_plan`, `read_index_release_job`,
  `run_index_release_job`, `run_index_track_job`).
- `tests/architecture_tests.rs` (the guards R34-04 to R34-13 and the guard of ADR 0075 packet
  018 Part A).

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data of its own. A rollback
keeps each row that the fix wrote. The rows have the same shape as before.

## Operator Visual Check

This check needs a Linux desktop session and a connection to `https://api.musicindex.org`. It
needs the Haleen artist feed `64569161` with MusicIndex singles that are not in the Library.

1. Build and start the debug binary:

   ```bash
   cd ~/build/v4vmm
   cargo build --bin v4vmm
   nm -C target/debug/v4vmm | grep -c TestDispatcher
   ./target/debug/v4vmm
   ```

   The `nm` command must give `0`.

2. Repair check. Wait 30 seconds after the app starts. In a second terminal, count the Library
   releases with no link that have the title of a downloaded single:

   ```bash
   sqlite3 -readonly ~/.local/share/v4vmm/v4vmm.sqlite "SELECT f.id, f.title FROM feeds f WHERE f.title IN ('A Greater Call', 'Build on Stone', 'A Phoenix Rising') AND NOT EXISTS (SELECT 1 FROM feed_publisher_relationships r WHERE r.feed_id = f.id AND r.direction = 'music_to_publisher');"
   ```

   **Wrong if:** the query gives a row. The pass at app start did not repair that release.
   Record the row and the time.

3. Count the Library releases with a link to the feed `64569161`:

   ```bash
   sqlite3 -readonly ~/.local/share/v4vmm/v4vmm.sqlite "SELECT count(DISTINCT feed_id) FROM feed_publisher_relationships WHERE direction = 'music_to_publisher' AND remote_feed_guid LIKE '64569161%';"
   ```

   Record the number.

4. V1: in Music, open Releases with the scope "Everything". Note the position of the Haleen
   group row. Click "Select", select the Haleen group row, and click "Download".
   **Wrong if:** a row of a downloaded single does not change in place, the group row moves,
   or a downloaded single leaves the group.

5. When the progress item shows that all downloads are complete, run the query of step 3
   again.
   **Wrong if:** the number did not increase by the number of downloaded singles.

6. V2: open the Haleen singles group page.
   **Wrong if:** a downloaded single shows under "Matched by name".

7. V3: open the Index release page of one more Haleen single that is not in the Library. Click
   "Download". When it is complete, open the singles group page of the feed with no other
   navigation.
   **Wrong if:** the single shows under "Matched by name".

Cleanup: steps 4 and 7 add singles to the Library. If the Library must stay as before the
check, select those singles in Releases with the scope "Library" and remove them. The queries
are read-only and change nothing.

## Orchestrator Answers - 2026-10-24

- Q1: yes, in a separate packet. This packet changes only the download and the pass at app start. The ADR 0084 phase plan records the skip of a `PageView` hydration as an open finding.
