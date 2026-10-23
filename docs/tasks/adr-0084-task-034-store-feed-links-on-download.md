# ADR 0084 Task 034: Hydrate A Feed's Identity After Its Download

Status: Implemented - 2026-10-22. Cases R34-01 to R34-15 pass. Mechanical checks are Green. V1 to
V4 are open, for the operator.

## Goal

Run `HydrateAlbumIdentity` for a feed right after its download succeeds, not only after a
person views its album page. Add a once-per-session pass that runs it for each stored release
that still has no stored named feed. A newly downloaded `MusicIndex` single must join its
artist feed's singles group with no page view.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 6, amended 2026-10-22: a release
  with a confirmed named feed joins that feed's singles group.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 7: the feed level of a release comes
  from its stored named feeds.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10: the download queue, its jobs, and
  `present_command` on the ADR 0040 runtime.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 12: the channel image of a named feed,
  read with the same stored rows (packet 024's open finding, below).
- [ADR 0085](../adr/0085-artists-not-publishers.md) Decisions 5 and 6: a named feed, its RSS
  position, and its confirmation.
- [ADR 0077](../adr/0077-publisher-relationship-storage.md) Decision 5: `feed_publisher_relationships`
  stores one row for each publisher relationship entry of a local feed.
- [Packet 032](adr-0084-task-032-singles-group-rows.md): the singles group join rule that this
  task's fix completes.
- [AGENTS.md](../../AGENTS.md): "Current-view state must update in place", renderer portability,
  "Every fix gets a guard", and background work through the ADR 0040 runtime only.
- Orchestrator decisions of 2026-10-22, recorded below.

## Recorded Facts - 2026-10-22

### The defect

- The operator found it on 2026-10-22. A download of a `MusicIndex` single of the Haleen artist
  feed moves the single out of the feed's own singles. On the singles group page, the single
  moves into the "Matched by name" block instead. The row order of that block changes too.
- `src/library/downloads.rs` `finish_download` and `finish_index_download` store no feed link.
  A download writes track and feed rows, never a row of `feed_publisher_relationships`.
- `src/application/queries/library.rs:584-743` `hydrate_album_identity_facts`, the body of the
  `HydrateAlbumIdentity` command (`:160-200`), is the only place that writes
  `feed_publisher_relationships`. It calls `crate::db::publisher_relationships::
  upsert_feed_publisher_relationships` at line 702 and `write_named_feed_positions` at line 711,
  each inside `if feed.publisher.as_deref().is_some_and(|entries| !entries.is_empty())`.
- `src/library/app_impl.rs:2302-2330` `LibraryApp::hydrate_album_identity_on_view` is the only
  caller of `HydrateAlbumIdentity` today. It runs only when a person opens an album page, and
  only when the album's stored identity facts look incomplete.
- A release with no stored row of `feed_publisher_relationships` has an empty `named_feeds`
  list. `src/view_models/library.rs:5645-5646` `AlbumNode::artist_identity` calls
  `ArtistIdentity::resolve(&self.named_feeds, artist_text)`
  (`src/view_models/artist_identity.rs:36-58`). With an empty list, the release gets the name
  level, keyed by its artist text, not the feed level. A later, separate text match then joins
  it to the feed's group under "Matched by name" (packet 032).
- This same empty `named_feeds` list is the stated cause of a second, open result. The Artists
  list shows an artist feed's channel image only after a view of one of its albums (packet 024).
  `src/view_models/library.rs:560-573` `LibraryTree::feed_images` reads `album.named_feeds` of
  every album in the tree.

### Today's hydration call and its result

- `src/application/queries/library.rs:39-48` `AlbumIdentityHydration` carries
  `identity_facts`, `metadata_facts`, `stored_values`, `description`, and
  `observation_receipts`. It carries no named feeds.
- `src/library/app_impl.rs:227-246` `apply_library_hydration_result` writes the hydration's
  fields onto the matching album of `vm.snapshot.tree` (through `LibraryViewModel::
  update_album_identity_facts`, `update_album_metadata_facts`, `update_album_stored_values`,
  `update_album_description`) and onto the open `LibraryDetail::Album`, when it is the same
  feed. None of those four methods (`src/view_models/library.rs:3117-3197`) touches
  `AlbumNode::named_feeds`.
- `src/db/publisher_relationships.rs:330-391` `named_feeds(conn, feed_id)` is the one read of
  the table that gives a `Vec<NamedFeed>`. `src/library/app_impl.rs:2273-2274` reads it when it
  builds an `AlbumNode` straight from the database, outside the cached tree.
- A download already starts a tree reload on success: `run_library_feed_job`
  (`src/library/app_impl.rs:2803-2842`) and `run_library_track_job` (`:2977-3055`) each call
  `start_async_reload_preserving_detail`, and an Index job's completion
  (`src/app/search_dispatch.rs:1593-1740` `finish_index_download`,
  `finish_index_release_download`, `finish_index_track_download`) calls `LibraryApp::refresh`,
  the same reload. `src/library/app_impl.rs:1423-1452` `start_async_reload_with_mode` runs
  `LoadLibraryTracksTree`, which rebuilds the tree from the database and replaces it with
  `vm.replace_tree`. Today this reload finds an empty `named_feeds` list too, because hydration
  never ran.

### The download jobs

- `src/library/app_impl.rs:2803-2842` `run_library_feed_job(identity, feed_id, cx)`: `feed_id`
  is a parameter. Its `feed` local (`crate::api::Feed`) carries `feed_guid: Option<String>`
  before it moves into `SubscribeFeedRequest`.
- `src/library/app_impl.rs:2977-3055` `run_library_track_job(identity, track_id, cx)`: its
  `track` local (`db::TrackRow`) carries `feed_id: i64` and `feed_guid: Option<String>` before
  it moves into `SubscribeTrackRequest::LibraryTrack`.
- `src/library/downloads.rs:338-377` `run_index_release_job(identity, feed_guid, feed_url, feed,
  cx)`: `feed_guid: Option<String>` moves into `DownloadJob::IndexRelease` before
  `present_command`.
- `src/library/downloads.rs:295-336` `run_index_track_job(identity, feed_guid, track_guid,
  context, cx)`: `feed_guid: Option<String>` moves into `DownloadJob::IndexTrack` before
  `present_command`.
- `src/db.rs:794` `find_feed_id_by_guid(conn, feed_guid) -> Result<Option<i64>>` is the stored
  read that `src/app/search_dispatch.rs:1635-1647` `downloaded_index_feed_id` already uses to
  find the Library feed row that a `MusicIndex` download just wrote.

### The guards that name this code today

- `tests/architecture_tests.rs:18382-18474`
  `adr_0075_library_observation_callers_and_consumers_are_guarded` checks that the text of
  `hydrate_album_identity_on_view` (up to `hydrate_detail_from_nav`) contains
  `apply_library_hydration_result(` and `retain_library_query_failure(`, beside its two skip
  checks. Moving those two calls to a shared function, called by
  `hydrate_album_identity_on_view`, moves them out of that checked text.
- `tests/architecture_tests.rs` `adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate`
  lists every file that may call `upsert_feed_publisher_relationships(`:
  `src/db/publisher_relationships.rs` and `src/application/queries/library.rs`. This task adds
  no new call of that function, so the list needs no change.

## Design

### One shared hydration call (fixes D1)

1. In `src/application/queries/library.rs`, add a `named_feeds` field to
   `AlbumIdentityHydration`. Its type is `Vec<crate::view_models::named_feeds::NamedFeed>`. In
   `hydrate_album_identity_facts`, read `db::publisher_relationships::named_feeds(&db,
   feed_id)?` with the existing `identity_facts`/`metadata_facts`/`stored_values`/`description`
   reads. Read it after the write block. A fresh write and a reused response then each give the
   row's current named feeds.
2. In `src/view_models/library.rs`, add `LibraryViewModel::update_album_named_feeds(feed_id,
   named_feeds)`, the same shape as `update_album_identity_facts`. It replaces `named_feeds` on
   each tree album of `feed_id`.
3. In `src/library/app_impl.rs`, extract the command dispatch of
   `hydrate_album_identity_on_view` into a new, shared `fn hydrate_album_identity(&mut self,
   feed_id: i64, feed_guid: String, continue_backfill: bool, cx: &mut Context<Self>)`. It builds
   `HydrateAlbumIdentity`, runs it with `present_command`, and on success calls
   `apply_library_hydration_result` (changed to also call `update_album_named_feeds`, and to set
   `album.named_feeds` on the open album detail); on failure it calls
   `retain_library_query_failure`. `continue_backfill` feeds the once-per-session pass (point 6).
   `hydrate_album_identity_on_view` keeps its two skip checks, then calls
   `self.hydrate_album_identity(feed_id, feed_guid, false, cx)`.
4. Add `fn hydrate_album_identity_by_feed_guid(&mut self, feed_guid: Option<String>, cx: &mut
   Context<Self>)`, `pub(super)`. With `None`, it returns. With `Some`, it reads the Library
   feed id of that GUID with `db::find_feed_id_by_guid`, and, when found, calls
   `self.hydrate_album_identity(feed_id, feed_guid, false, cx)`. With no stored feed id, it
   returns: the once-per-session pass picks up a release this misses.
5. Each success path below calls one of these two functions, before its existing completion
   call, and only when its feed GUID is known:
   - `run_library_feed_job`: read `feed.feed_guid.clone()` before `feed` moves into the command,
     and call `hydrate_album_identity(feed_id, feed_guid, false, cx)` on success.
   - `run_library_track_job`: read `track.feed_id` and `track.feed_guid.clone()` before `track`
     moves into the command, and call `hydrate_album_identity` the same way on success.
   - `run_index_release_job` (`src/library/downloads.rs`): read `feed_guid.clone()` before it
     moves into `DownloadJob::IndexRelease`, and call `hydrate_album_identity_by_feed_guid` with
     it on success.
   - `run_index_track_job` (`src/library/downloads.rs`): the same, before `feed_guid` moves into
     `DownloadJob::IndexTrack`.

### Once-per-session backfill (fixes D2)

6. Add two fields to `LibraryApp` (`src/library.rs`): `album_identity_backfill_started: bool`
   and `album_identity_backfill_queue: VecDeque<(i64, String)>`. Both start empty or `false` in
   `new_with_content_view_mode`.
7. Add a pure function `album_identity_backfill_order(tree: &LibraryTree) -> Vec<(i64, String)>`
   in `src/library/app_impl.rs`: every album with a `feed_id` and a `feed_guid`, sorted by
   `feed_id` descending (the newest Library row first, since `feed_id` is the row's own
   increasing key).
8. Add `db::publisher_relationships::feed_ids_with_no_stored_relationship(conn, candidates) ->
   Result<Vec<i64>>` in `src/db/publisher_relationships.rs`: one query reading every stored
   `feed_id` among `candidates`, then every id of `candidates` that this query did not find, in
   their given order.
9. Add `fn start_album_identity_backfill(&mut self, cx)` in `src/library/app_impl.rs`. It sets
   `album_identity_backfill_started` to `true` first. It builds the ordered list (point 7). It
   reads the ids with no stored row (point 8), with one lock of `self.conn`. It builds
   `album_identity_backfill_queue` from the ordered list, filtered to those ids. It then calls
   `pump_album_identity_backfill` up to `ALBUM_IDENTITY_BACKFILL_CONCURRENCY` (4) times.
10. Add `fn pump_album_identity_backfill(&mut self, cx)`: pops one entry from
    `album_identity_backfill_queue`, and, when one is found, calls `hydrate_album_identity` with
    `continue_backfill: true`. `hydrate_album_identity`'s own success and failure closures call
    `pump_album_identity_backfill` again when `continue_backfill` is `true`. A finished command
    thus starts the next queued one. At most 4 run at a time.
11. In `start_async_reload_with_mode`'s `LoadLibraryTracksTree` success closure
    (`src/library/app_impl.rs:1423-1452`), after `this.vm.replace_tree(result.tree)`, call
    `if !this.album_identity_backfill_started { this.start_album_identity_backfill(cx); }`. The
    first successful load of the session is the only one that starts it, because the flag is
    `true` before that call returns.

## Orchestrator Decisions

- **D1. After each successful download of a release (Library feed job and `MusicIndex`
  release/track job), run `HydrateAlbumIdentity` for that feed (feed id and feed GUID of the
  Library feed row) through the ADR 0040 runtime (`present_command`), and apply the result in
  place (`apply_library_hydration_result` or the equivalent that refreshes the tree), so the
  release joins its feed group without a page view.** This task calls the shared
  `hydrate_album_identity` from the Library feed job and the Library track job, since each job
  already knows its own feed id. It calls `hydrate_album_identity_by_feed_guid` from the Index
  release job and the Index track job, which first finds the feed id that the download wrote.
  "Apply the result in place" now also writes `AlbumNode::named_feeds`, which the singles join
  rule and the Artists list image both read. The four existing update methods did not reach
  that field.
- **D2. Once per app session, after the Library tree first loads, a background pass hydrates
  each Library release that has a feed GUID but no stored row in `feed_publisher_relationships`,
  at most 4 commands at a time, newest first. It sends nothing for a release that already has
  rows. It refreshes the tree in place when results arrive.** This task implements the pass as
  a self-feeding queue that starts 4 commands, and each completion starts the next queued one.
  The pass thus never runs more than 4 at a time, and needs no separate progress state.
- **D3. A failure of a hydration leaves the release as it is and is reported through the
  existing retained failure path, not as a download failure.** Both the download hook and the
  backfill pass share `hydrate_album_identity`'s failure branch, which calls
  `retain_library_query_failure`, the same function `hydrate_album_identity_on_view` already
  uses. Neither hook changes `DownloadOutcome`, so a hydration failure never marks a download as
  failed.
- **D4 (this task's own scoping choice, flagged for the operator).** D1 names "Library feed job
  and `MusicIndex` release/track job", not the Library track job. This task adds the hook to
  the Library track job too. A Library track can be the only track of a single-track feed that
  was never hydrated: the same gap D1 closes for the other three jobs. The operator should
  confirm or correct this reading.
- **D5 (this task's own scoping choice).** "Newest first" has no stated ordering key. This task
  orders a backfill candidate by its Library `feed_id`, descending: a higher id is a more
  recently subscribed feed, and the tree already carries this id with no extra read.

## Required Changes

1. `src/application/queries/library.rs`: add `named_feeds` to `AlbumIdentityHydration`; read it
   in `hydrate_album_identity_facts`.
2. `src/view_models/library.rs`: add `LibraryViewModel::update_album_named_feeds`.
3. `src/library.rs`: add `album_identity_backfill_started` and `album_identity_backfill_queue`
   to `LibraryApp`, and their initial values.
4. `src/library/app_impl.rs`: add `hydrate_album_identity`, `hydrate_album_identity_by_feed_guid`,
   `album_identity_backfill_order`, `start_album_identity_backfill`,
   `pump_album_identity_backfill`, and the constant `ALBUM_IDENTITY_BACKFILL_CONCURRENCY`;
   change `hydrate_album_identity_on_view`, `apply_library_hydration_result`,
   `run_library_feed_job`, `run_library_track_job`, and `start_async_reload_with_mode`.
5. `src/library/downloads.rs`: change `run_index_release_job` and `run_index_track_job`.
6. `src/db/publisher_relationships.rs`: add `feed_ids_with_no_stored_relationship`.
7. `tests/architecture_tests.rs`: update the `hydration_callback` checks of
   `adr_0075_library_observation_callers_and_consumers_are_guarded`. Add the situational guards
   of "Mechanical Acceptance Criteria" R34-04 to R34-08, R34-10 and R34-12. Cite this task in
   each new guard.
8. Add each test of "Mechanical Acceptance Criteria", and the three existing
   `AlbumIdentityHydration { ... }` test literals in `src/library/app_impl.rs` gain a
   `named_feeds` field.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_task_034_` for a new unit test beside its owning code. A guard lives in
`tests/architecture_tests.rs` and carries the same case number in its name.

| Case | Required proof |
|---|---|
| R34-01 | `hydrate_album_identity_facts` gives `named_feeds` from the stored rows of its feed id, after any write. With a stored confirmed named feed, the list is not empty |
| R34-02 | `apply_library_hydration_result` sets `named_feeds` on the matching tree album and, when it is the open detail, on that album too |
| R34-03 | `LibraryViewModel::update_album_named_feeds` replaces the named feeds of each tree album of one feed id |
| R34-04 | Guard: the success closure of `run_library_feed_job` calls `hydrate_album_identity(` |
| R34-05 | Guard: the success closure of `run_library_track_job` calls `hydrate_album_identity(` |
| R34-06 | Guard: the success closure of `run_index_release_job` calls `hydrate_album_identity_by_feed_guid(` |
| R34-07 | Guard: the success closure of `run_index_track_job` calls `hydrate_album_identity_by_feed_guid(` |
| R34-08 | Guard: `hydrate_album_identity_by_feed_guid` returns with no command sent when its argument is `None`, and reads the feed id with `find_feed_id_by_guid` before it calls `hydrate_album_identity` |
| R34-09 | `feed_ids_with_no_stored_relationship` gives every candidate id with no stored row, in their given order. It gives none of a candidate id that has a stored row, of any direction |
| R34-10 | Guard: `start_album_identity_backfill` sets `album_identity_backfill_started` before it builds its queue, and the `LoadLibraryTracksTree` success closure checks that field before it calls `start_album_identity_backfill` |
| R34-11 | `album_identity_backfill_order` gives every tree album with a feed id and a feed GUID, sorted by feed id descending. It gives nothing for an album with no feed GUID |
| R34-12 | Guard: `start_album_identity_backfill` calls `pump_album_identity_backfill` exactly `ALBUM_IDENTITY_BACKFILL_CONCURRENCY` times, and both of `hydrate_album_identity`'s closures call `pump_album_identity_backfill` when `continue_backfill` is true |
| R34-13 | Guard: neither closure of `hydrate_album_identity` changes a `DownloadOutcome`, and its failure closure calls `retain_library_query_failure(` |
| R34-14 | The updated `adr_0075_library_observation_callers_and_consumers_are_guarded` passes, and `adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate` passes with no change to its allow list |
| R34-15 | Each existing test of `HydrateAlbumIdentity`, `apply_library_hydration_result`, and `hydrate_album_identity_on_view` still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: download a `MusicIndex` single of an artist feed that already has a confirmed named feed
  link. Open that feed's singles group page with no other navigation in between. The new single
  shows under the feed's own singles, not under "Matched by name".
- V2: the Artists list row of that same feed shows its channel image right after the download,
  with no need to open an album page first.
- V3: start the app with a Library that has an old release never opened as an album page. Watch
  the Releases list or the Artists list for up to a few seconds after start. A release that
  names a confirmed feed by RSS, but that the app never linked before, settles into its feed's
  group. Its row also gains the feed's image. Neither change needs a page view, and the list
  shows no visible stall.
- V4: Light and Dark themes: no flicker or layout jump when a group row's membership changes
  under V1 or V3.

## Exclusions

- No change to the RSS check (`src/rss/check_apply.rs`), which keeps writing only the compared
  `music_to_publisher` remote pointer, not the full relationship row set.
- No change to the feed page, the Index publisher page, or `name_feed_matches` callers outside
  `src/library/app_impl.rs`.
- No progress indicator or status text for the backfill pass. D2 states no such requirement.
- No change to the download queue's own progress item, its concurrency limit, or its stop/dismiss
  actions. The backfill pass keeps a separate, smaller queue of its own.
- No schema change.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/application/queries/library.rs` (`AlbumIdentityHydration`, `HydrateAlbumIdentity`,
  `hydrate_album_identity_facts`).
- `src/library/app_impl.rs` (`apply_library_hydration_result`, `hydrate_album_identity_on_view`,
  `run_library_feed_job`, `run_library_track_job`, `start_async_reload_with_mode`).
- `src/library/downloads.rs` (`run_index_release_job`, `run_index_track_job`,
  `finish_index_download`).
- `src/library.rs` (the `LibraryApp` struct and its constructor).
- `src/view_models/library.rs` (`AlbumNode`, `update_album_identity_facts` and its neighbors,
  `LibraryTree::feed_images`).
- `src/view_models/artist_identity.rs` (`ArtistIdentity::resolve`).
- `src/db/publisher_relationships.rs` (`named_feeds`, the table, its test module).
- `src/db.rs` (`find_feed_id_by_guid`).
- `tests/architecture_tests.rs`
  (`adr_0075_library_observation_callers_and_consumers_are_guarded`,
  `adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate`).

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

Revert the commit. This task adds no migration and no stored data of its own. It only writes,
earlier and more often, rows that `HydrateAlbumIdentity` already knows how to write.

## Implementation Result - 2026-10-22

### Files Changed

- `src/application/queries/library.rs`: `AlbumIdentityHydration` gains `named_feeds`.
  `hydrate_album_identity_facts` reads it from `db::publisher_relationships::named_feeds` after
  its write block. New test `adr_0084_task_034_hydration_result_carries_named_feeds`.
- `src/view_models/library.rs`: new `LibraryViewModel::update_album_named_feeds`, beside
  `update_album_description`. New test `adr_0084_task_034_update_album_named_feeds_by_feed_id`.
- `src/library.rs`: `LibraryApp` gains `album_identity_backfill_started` and
  `album_identity_backfill_queue`. `VecDeque` joins the module's `std::collections` import.
- `src/library/app_impl.rs`:
  - `apply_library_hydration_result` also calls `update_album_named_feeds`, and sets
    `album.named_feeds` on the open album detail.
  - New shared `hydrate_album_identity`, called by `hydrate_album_identity_on_view` (unchanged
    skip checks), by the Library feed and track job success paths, and by the backfill pump.
  - New `hydrate_album_identity_by_feed_guid`, `pub(super)`, called from
    `src/library/downloads.rs`.
  - New `album_identity_backfill_order` (pure), `start_album_identity_backfill`,
    `pump_album_identity_backfill`, and the constant `ALBUM_IDENTITY_BACKFILL_CONCURRENCY` (4).
  - `start_async_reload_with_mode`'s `LoadLibraryTracksTree` success closure starts the backfill
    pass once, after the first successful load.
  - `run_library_feed_job` and `run_library_track_job` each hydrate their feed on success.
  - Three existing `AlbumIdentityHydration { ... }` test literals gain `named_feeds: Vec::new()`.
    One of them, in `adr_0075_library_observation_comparison_callback_keeps_empty_state_and_album_updates`,
    now carries a populated list, with new assertions for R34-02. New test
    `adr_0084_task_034_backfill_order_is_newest_feed_id_first`.
- `src/library/downloads.rs`: `run_index_release_job` and `run_index_track_job` each call
  `hydrate_album_identity_by_feed_guid` on success, with the feed GUID read before it moves into
  the finished `DownloadJob`.
- `src/db/publisher_relationships.rs`: new `feed_ids_with_no_stored_relationship`. New test
  `adr_0084_task_034_feed_ids_with_no_stored_relationship`.
- `tests/architecture_tests.rs`:
  - `adr_0075_library_observation_callers_and_consumers_are_guarded`'s `hydration_callback`
    check now matches the shared function: it checks `hydrate_album_identity_on_view` calls
    `self.hydrate_album_identity(`, and checks the shared function's own body for
    `apply_library_hydration_result(` and `retain_library_query_failure(`.
  - Four new situational guards, citing this task:
    `adr_0084_task_034_library_download_jobs_hydrate_their_feed`,
    `adr_0084_task_034_index_download_jobs_hydrate_their_feed`,
    `adr_0084_task_034_hydrate_by_feed_guid_resolves_the_feed_id_first`, and
    `adr_0084_task_034_backfill_starts_once_and_hydration_failure_stays_separate`.

### Tests

| Case | Proof |
|---|---|
| R34-01 | `adr_0084_task_034_hydration_result_carries_named_feeds` (`application/queries/library.rs`) |
| R34-02 | New assertions in `adr_0075_library_observation_comparison_callback_keeps_empty_state_and_album_updates` (`app_impl.rs`) |
| R34-03 | `adr_0084_task_034_update_album_named_feeds_by_feed_id` (`view_models/library.rs`) |
| R34-04, R34-05 | Guard `adr_0084_task_034_library_download_jobs_hydrate_their_feed` |
| R34-06, R34-07 | Guard `adr_0084_task_034_index_download_jobs_hydrate_their_feed` |
| R34-08 | Guard `adr_0084_task_034_hydrate_by_feed_guid_resolves_the_feed_id_first` |
| R34-09 | `adr_0084_task_034_feed_ids_with_no_stored_relationship` (`db/publisher_relationships.rs`) |
| R34-10, R34-12, R34-13 | Guard `adr_0084_task_034_backfill_starts_once_and_hydration_failure_stays_separate` |
| R34-11 | `adr_0084_task_034_backfill_order_is_newest_feed_id_first` (`app_impl.rs`) |
| R34-14 | The updated `adr_0075_library_observation_callers_and_consumers_are_guarded`; `adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate` passes with its allow list unchanged |
| R34-15 | `cargo test` is Green, with every prior test kept |

### Checks - 2026-10-22

| Check | Result |
|---|---|
| `cargo test` | Green. 2282 library tests, 380 architecture guards, 0 failed |
| `cargo test --test architecture_tests` | Green, 380 guards |
| `cargo fmt` then `cargo fmt -- --check` | Green |
| `cargo clippy --all-targets -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Behavior Changed

- A download of a Library release, a Library track, a `MusicIndex` release, or a `MusicIndex`
  track now sends one `HydrateAlbumIdentity` command for its feed. The command runs right after
  the download succeeds, when the feed's GUID is known. This writes
  `feed_publisher_relationships` for that feed, the same write a view of its album page already
  triggered.
- The hydration result now also replaces `AlbumNode::named_feeds` on the matching tree album, not
  only its identity facts. The singles join rule, the Artists list image, and the Releases list
  group membership all read this field. Each of them can thus change with no page view.
- Once per app session, after the Library tree first loads, a background pass starts. It
  hydrates each stored release that has a feed GUID but no stored row of
  `feed_publisher_relationships`. It reads newest `feed_id` first, at most 4 at a time.
- A hydration failure, from a download or from the backfill pass, keeps the retained-failure path
  `hydrate_album_identity_on_view` already used. It never marks a download as failed.

### Deviations From The Packet Text

1. This task's own scoping choices, D4 (the Library track job hook) and D5 (the "newest first"
   order), are implemented as the packet's Design and Orchestrator Decisions state. They are
   flagged there for the operator to confirm or correct, not decided silently.
2. `tests/architecture_tests.rs` `adr_0075_library_observation_callers_and_consumers_are_guarded`
   needed a change the packet's Recorded Facts anticipated but did not list under "Required
   Changes" item 7 by its exact assertion text. The change keeps the same two checked facts
   (`apply_library_hydration_result(` and `retain_library_query_failure(` run on every
   hydration), read from the new shared function instead of from `hydrate_album_identity_on_view`
   itself.
3. Cases R34-04 to R34-08, R34-10, R34-12, and R34-13 share four guard functions, not one guard
   for each case. Each shared guard checks facts that sit next to each other in the same
   function body. Splitting them into eight functions would reread the same source text eight
   times, for no added proof.
4. `cargo clippy` asked for `sort_by_key` with `std::cmp::Reverse`, not `sort_by` with a reversed
   comparison, in `album_identity_backfill_order`. The result orders the same way.

### Unresolved Concerns

1. V1 to V4 below need the operator. No test proves a rendered group membership, a channel image,
   or a start-up list with no visible stall.
2. This task's own reading of D1 (Library track job) and "newest first" (D5) are scoping choices,
   not points the operator decided by name. They are recorded in "Orchestrator Decisions" for the
   operator to confirm or correct.
3. **Document change for the orchestrator** (D2 asks for this list item, not an edit by this
   task): add this line to "Throughout: Responsiveness And Background Feedback" in the
   [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md):

   > 2026-10-22: the once-per-session identity backfill pass can start up to 4
   > `HydrateAlbumIdentity` requests right after the first Library tree load. Each request reads
   > one feed from `MusicIndex`. A Library with many never-hydrated releases can keep requests
   > running for some time after start, with no progress state (`start_album_identity_backfill`
   > in `src/library/app_impl.rs`, ADR 0084 task 034).

   Review Correction 2026-10-22: this note describes one session. Across sessions, the fix below
   stops the repeat for a feed with a recorded hydration observation. Only a feed with no such
   observation still sends a request at the next start.

### Review Corrections - 2026-10-22

- HIGH: `start_album_identity_backfill` now also reads `db::provider_observations::
  feed_guids_with_hydration_observation`, and drops a feed with a recorded `success` or `partial`
  hydration observation from its queue. This stops the repeat named above.
- LOW: a failed candidate read now reports through `vm.set_error_status`. The pass still starts,
  with an empty queue for that session.

## Orchestrator Review - 2026-10-23

Pass, after three corrections. The review found that a feed with no publisher in MusicIndex never stores a link, so the pass at start would send its request again at each start. The first correction used each successful hydration observation to leave a feed out. That rule was too wide: a download reads the feed with the same request shape and stores no link, so the pass would leave out the singles that it must repair. The orchestrator narrowed it. A feed is left out only when a successful observation holds an answer with an absent or empty `data.publisher` list. The test `adr_0084_task_034_answer_with_a_publisher_keeps_the_feed_a_candidate` proves it. A read failure of the pass is now reported, not ignored.

## Operator Visual Check

This check needs a Linux desktop session, and a Library with at least one artist feed that has a
confirmed named feed link. It also benefits from an old Library release whose feed was never
opened as an album page, to exercise V3.

1. Build and start the binary:

   ```bash
   cd ~/build/v4vmm
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   ```

2. V1: in Music, find a `MusicIndex` single of an artist feed that already has a confirmed named
   feed link. Download it. With no other navigation in between, open that feed's singles group
   page.
   **Wrong if:** the new single shows under the "Matched by name" heading instead of under the
   feed's own singles.

3. V2: look at the Artists list row of that same feed right after the download in step 2.
   **Wrong if:** the row shows no channel image until an album page of that feed opens.

4. V3: restart the app with the same Library. Watch the Releases list or the Artists list after
   start.
   **Wrong if:** a release that names a confirmed feed by RSS, with no earlier link, still shows
   apart from its feed's group, or the list stalls visibly.

5. V4: repeat steps 2 and 3 in the Dark theme.
   **Wrong if:** a group row's membership change shows a flicker or a layout jump.

Cleanup: this check downloads at most one `MusicIndex` single (V1). Remove it from the Library
afterward if the operator Library should stay as it was.
