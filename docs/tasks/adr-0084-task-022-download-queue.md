# ADR 0084 Task 022: The Download Queue

Status: Implemented - 2026-10-20. Cases R22-01 to R22-12 pass, with the orchestrator correction. Mechanical checks are Green. V1 to V6 are open, in the final walk of the run.

## Goal

Each download goes through one queue: a Library release, a Library track, an Index track and an Index release. At most three run at the same time, and never two of one release.
One progress item shows the batch on each page, for example "Downloading 3 of 9", with the item name and the failure count.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10, amended on 2026-10-20: the queue rules and the progress item.
- [ADR 0040](../adr/README.md): background work runs on the async command runner. No screen calls `cx.spawn`.
- [AGENTS.md](../../AGENTS.md) "Write reports for the operator": the item names the subject, the action and the object. Each time is a recorded time.
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md): the orchestrator decisions on the owner, the dismissal and the cancel.

## Recorded Facts - 2026-10-20

The line numbers are from before packet 021. Packet 021 changes the release page assembly. Find each call site again before you change it.

- `LibraryApp` has `queued_downloads: VecDeque<QueuedDownload>` with `QueuedDownload::{Feed(i64), Track(i64)}` (`src/library.rs`). `download_singles`, `download_library_tracks` and `start_next_queued_download` in `src/library/app_impl.rs` run one download at a time.
- `download_feed` and `subscribe_track` refuse a new download while one runs (a single-slot guard). `download_rss_added_track` downloads one Library track.
- `LibraryApp` holds `conn`, `application_services`, `command_runner` and `musicindex_endpoint`. An Index job needs no other `TopApp` state when it runs.
- `TopApp` starts each Index download at once: `download_index_track`, `download_index_feed` and `download_index_album_of_track` in `src/app/search_dispatch.rs`. Their success handlers update the open pages in place (`show_downloaded_index_feed`, `apply_release_page_source`, `track_download_updates_open_page` with `apply_track_page_source`) and call `scan_tags_after_search_download`.
- `download_index_album_of_track` reads the feed from MusicIndex first. Nothing shows that read (responsiveness item 6).
- `busy_track: Option<i64>` and `busy_feed: Option<i64>` in `src/view_models/library.rs` hold one busy item each. The status text `vm.status` holds one string.
- `build_live_status_strip` returns `None` when no show is active (`src/app/show.rs`). The live status strip thus cannot hold the item.
- These guards and tests pin the present download shape:
  - `adr_0084_detail_selection_index_track_download_updates_the_open_release_page` in `tests/architecture_tests.rs`.
  - The guard in `tests/architecture_tests.rs` that requires `track_download_updates_open_page(` in `download_index_track` and forbids `replace_current(` there.
  - `adr_0076_follow_up_search_download_starts_one_tag_scan` in `src/app/search_dispatch.rs`.

## Design

### The queue

Add `src/library/download_queue.rs`: a pure scheduler with no GPUI type and no I/O. `LibraryApp` holds one `DownloadQueue` in place of `queued_downloads`.

```rust
enum DownloadJob {
    LibraryFeed { feed_id: i64 },
    LibraryTrack { track_id: i64 },
    IndexTrack { feed_guid: String, track_guid: String, context: Box<TrackContext> },
    IndexRelease { feed_guid: String, feed: Option<Box<api::Feed>> },
}
struct QueuedJob { identity: JobIdentity, feed_key: FeedKey, label: String, job: DownloadJob }
```

- `MAX_IN_FLIGHT` is 3, a named constant.
- `FeedKey` is the release key of the queue. Use the feed GUID first, then the feed URL, then the Library feed id. A Library job and an Index job of one release thus have one key.
- `next_startable()` gives the first queued job whose key is not in flight. A job of a busy release waits, and the jobs after it can start.
- A job whose identity is queued or in flight is dropped. A track job is dropped when a release job of its key is queued or in flight.
- A failure records the item name and its error text. The queue continues.
- `IndexRelease { feed: None }` replaces `download_index_album_of_track`. Its MusicIndex read runs in the slot of the job.
- No cancel in this packet.
- A removal of a release is refused while its key is queued or in flight. The refusal names the release and the reason.
- An Index track with no feed GUID and no feed URL has no key. Refuse it with a report that names the track.

### The jobs and the completions

- Each download entry point queues a job: the action bars, the release page buttons, the track page buttons, the playlist page and the singles group page. `TopApp` builds the Index job from its `FeedView` and `TrackView` (`api_feed_from_view`, `api_track_from_view`).
- `LibraryApp::pump_downloads` starts jobs while fewer than three are in flight. Each job runs through `present_command`, as today.
- Each completion calls `finish(identity, outcome)` and then `pump_downloads`.
- An Index completion sends `LibraryAppEvent::IndexDownloadFinished { job, outcome }`. `TopApp` handles it with the present success code: the in-place page updates and `scan_tags_after_search_download`.
- `busy_track` and `busy_feed` become sets that come from the queue. A release or a track shows `Downloading` while its key is queued or in flight.
- Each failure goes to the progress item. One job does not replace the status text of another.

### The progress item

Add `src/view_models/download_progress.rs` with `DownloadProgressDisplay::project(&DownloadBatch) -> Option<Self>`. It is pure.

- The first job queued with no running batch opens a batch. A job queued during a running batch joins it, and the total grows.
- Running: "Downloading 3 of 9", the name of the oldest job in flight, and "1 failed" when a job failed. The number 3 is done plus failed plus one, and it is never more than the total.
- Finished: "Downloaded 8 of 9, 1 failed. Finished 14:02". The time is `finished_at`, which the last completion handler records with `chrono::Local::now()`. The renderer makes no time.
- The accessibility label, for example "Downloading item 3 of 9: Face Pale. 1 download failed."
- A typed "Dismiss" action shows only when the batch is finished. A finished batch stays until the operator dismisses it, or until the next batch opens.
- Add the composite `src/ui/composites/download_status_item.rs`. `TopApp::render` mounts it beside the live status strip, on each section. It uses named tokens and a named icon.

## Required Changes

1. Add `download_queue.rs` and its tests.
2. Replace `queued_downloads`, the single-slot guards and `start_next_queued_download`.
3. Make each download entry point queue a job. Move the Index success code into the completion handler.
4. Make the busy state come from the queue.
5. Add the progress view model, its composite and its mount.
6. Move the three guards to the completion handler with the same meaning. Add the guards of this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_download_queue_` for unit tests beside the owning code. Each new guard cites ADR 0084 Decision 10.

| Case | Required proof |
|---|---|
| R22-01 | `next_startable` gives no job when three are in flight |
| R22-02 | Two jobs of one `FeedKey` are never in flight together. A Library feed id and its Index GUID give the same key |
| R22-03 | The jobs start in their order. A job of a busy key does not stop a later job of another key |
| R22-04 | A job whose identity is queued or in flight is dropped. A track job is dropped when a release job of its key is queued |
| R22-05 | A failure adds to the failed count, and the queue starts the next job |
| R22-06 | A job queued during a running batch adds to the total. The first job after a finished batch opens a new batch |
| R22-07 | `DownloadProgressDisplay` gives the text, the failure text and the accessibility label for a running batch and a finished batch. `finished_at` comes only from its input |
| R22-08 | The display shows a finished batch until "Dismiss". "Dismiss" is absent while the batch runs |
| R22-09 | A release or a track is `Downloading` while its key is queued or in flight |
| R22-10 | A removal is refused while its key is queued or in flight |
| R22-11 | A source guard: no function in `src/app/` calls `SubscribeFeed::new` or `SubscribeTrack::new`. Each Index download queues a job |
| R22-12 | The three moved guards keep their meaning on the completion handler |

## Visual Acceptance Criteria

These are for the operator, in the final walk of the run. No test proves them.

- V1: select nine releases and click "Download". The item shows "Downloading n of 9" and a name. At most three releases show "Downloading" at the same time.
- V2: open another page, then Settings. The item stays.
- V3: a download that fails shows "1 failed", and the other downloads finish.
- V4: a finished batch shows its finish time, and stays until "Dismiss".
- V5: an open Index release page and an open Index track page change to their Library page in place after their download.
- V6: Light and Dark themes, and a narrow width. The item text is not clipped.

## Exclusions

- No cancel.
- No status bar of ADR 0083 Decision 7. A later packet moves the item into it.
- No change to the selection model or to the bar actions.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/library.rs`, `src/library/app_impl.rs`, `src/library/detail_selection.rs`.
- `src/app.rs`, `src/app/search_dispatch.rs`, `src/app/detail_selection.rs`, `src/app/show.rs`.
- `src/view_models/library.rs` (the busy state and the membership), `src/view_models/release_page.rs` (packet 021).
- `src/ui/shells/release_page.rs` (packet 021), `src/ui/shells/library/playlist_detail.rs`, `src/ui/shells/library/singles_group.rs`.
- `src/ui/composites/live_status_strip.rs`, `src/presentation/async_command_presenter.rs`, `src/application/async_command_runner.rs`.
- `tests/architecture_tests.rs` and `src/app/search_dispatch.rs`: the three guards in "Recorded Facts".

## Checks

```bash
cargo test --lib adr_0084_download_queue_
cargo test --lib adr_0084_detail_selection_
cargo test --lib adr_0084_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-022-download-queue.md`
- ADR 0084 Decision 10, with its amendment of 2026-10-20
- The archived packets 018 and 021
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the queue, the jobs, the completions, the busy state and the progress item.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each text, count, time and availability. The screen only composes. Use named tokens and named icons.
- Keep each in-place page update and each tag scan of today.
- Tests use recorded values and recorded times. No test sends a request or writes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The selection model, the bar actions and the release page assembly, except each download call.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R22-01 to R22-12 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V6.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- An Index job needs `TopApp` state while it runs.
- A download entry point cannot queue a job without a change to the selection model.
- The present command runner cannot run three commands at the same time.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-20

### Files Changed

- `src/library/download_queue.rs` (new): the pure scheduler `DownloadQueue`, with no GPUI type and no I/O.
  - The types `DownloadJob`, `JobIdentity`, `QueuedJob`, `DownloadRefusal`, `DownloadOutcome` and `EnqueueResult`, and the constant `MAX_IN_FLIGHT` (3).
  - `enqueue`, `refuse`, `next_startable`, `start_next`, `finish`, `rename`, `is_active`, `removal_refusal`, `activity`, `batch` and `dismiss`.
  - Tests R22-01 to R22-06, R22-08 to R22-10 and the refusal of an Index track with no key.
- `src/library/downloads.rs` (new): the `LibraryApp` part of the queue.
  - `download_library_feeds`, `download_library_tracks`, `queue_downloads`, `pump_downloads` and `finish_download`.
  - The Index job runners, the progress item render, "Dismiss" and the removal refusal.
  - The job builders `index_release_job`, `index_release_job_by_guid` and `index_track_job`. `api_feed_from_view` and `api_track_from_view` moved here from `src/app/search_dispatch.rs`.
- `src/view_models/download_progress.rs` (new): `FeedKey`, `DownloadActivity`, `DownloadBatch`, `DownloadFailure` and `DownloadProgressDisplay::project`, with tests R22-02 (the key) and R22-07 to R22-08.
- `src/ui/composites/download_status_item.rs` (new): the composite of the progress item. It uses named tokens and the named icons `Info`, `Check`, `Warning` and `Close`.
- `src/library.rs`: `download_queue: DownloadQueue` replaces `queued_downloads` and `QueuedDownload`. New event `LibraryAppEvent::IndexDownloadFinished { job, outcome }`.
- `src/library/column_view.rs`: new `DownloadStatusView`, a render-only view that observes the Library entity.
- `src/library/app_impl.rs`:
  - `run_library_feed_job` and `run_library_track_job` replace `download_feed` and `subscribe_track`.
  - `download_singles`, `start_next_queued_download` and `download_rss_added_track` are deleted.
  - The track page download queues one job.
  - The removal of a release is refused while a job of its key is queued or in flight.
- `src/library/release_page.rs`, `src/ui/shells/library/playlist_detail.rs` and `src/ui/shells/library/singles_group.rs`: each download call queues a job.
- `src/app.rs`: `TopApp` holds `download_status_view` and mounts it below the live status strip on each section. It handles `IndexDownloadFinished`.
- `src/app/search_dispatch.rs`: `download_index_feed`, `download_index_track` and `download_index_album_of_track` queue a job.
  The new `finish_index_download`, `finish_index_release_download` and `finish_index_track_download` hold the present success code. The tag scan test reads the new handlers.
- `src/app/detail_selection.rs`: the detail bar queues each Library and Index item. An Index release job names the release with its row title.
- `src/view_models/library.rs`:
  - `BusyItems` replaces `busy_track` and `busy_feed`. `AlbumNode::feed_key` is new.
  - `membership` takes `downloading` and `removing`.
  - `begin_busy_*`, `clear_busy_*`, `finish_feed_download`, `fail_feed_download`, `fail_track_subscribe` and `track_subscribe_begin_status` are deleted. The test of R22-09 is new.
- `src/view_models/release_page.rs` and `src/view_models/singles_group.rs`: the busy state comes from `BusyItems`.
- `src/ui/shells/library/detail.rs` and `src/ui/shells/library/feed_detail.rs`: the `busy_track` argument is deleted.
- `src/view_models/mod.rs` and `src/ui/composites/mod.rs`: the new modules.
- `tests/architecture_tests.rs`: the two moved guards read `finish_index_track_download` and `finish_index_release_download`.
  The ADR 0049 guard reads the Index download requests in `src/library/downloads.rs`. The new guard `adr_0084_download_queue_each_index_download_queues_a_job` is R22-11.

### Behavior Changed

- Each download goes through one queue: a Library release, a Library track, an Index track and an Index release. The entry points are the action bars, the release page, the track page, the playlist page and the singles group page.
- At most three downloads run at the same time. Two jobs of one release never run at the same time.
- A job of a busy release waits, and the jobs after it can start.
- A Library release and its Index copy have one key: the feed GUID, then the feed URL, then the Library feed id.
- A second job of the same item is dropped. A track job is dropped while a release job of its key is queued or running.
- "Download album" of an Index track page, and an Index release of the detail bar, read the feed from MusicIndex in the slot of the job. The progress item shows this read.
- One progress item shows below the live status strip on Music, Show and Settings. While the batch runs, it shows "Downloading 3 of 9", the name of the oldest running job and "1 failed".
- Each failed job adds one line to the item: "The app could not download Face Pale: connection refused".
- A finished batch shows, for example, "Downloaded 8 of 9, 1 failed. Finished 14:02". It stays until the operator clicks "Dismiss", or until a new batch opens.
- A release shows `Downloading` while a job of its key is queued or running. A track row shows busy while its own job, or a release job of its key, is queued or running.
- A removal of a release with a queued or running download is refused. The status line names the release and the reason. A selection removal records the refusal in its report.
- A download no longer writes the status line of the Library or of Settings. Only a format warning of a Library track also goes to the status line.
- The open Index release page and the open Index track page change to their Library page in place, as before. Each successful Index download starts one tag scan, as before.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_download_queue_` | Green, 17 tests |
| `cargo test --lib adr_0084_detail_selection_` | Green, 9 tests |
| `cargo test --lib adr_0084_selection_` | Green, 29 tests |
| `cargo test` | Green, 2010 unit tests and 334 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 334 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred. No test sends a request or writes a file.

### Deviations From The Task

1. `FeedKey`, `DownloadActivity` and `DownloadBatch` are in `src/view_models/download_progress.rs`, not in `src/library/download_queue.rs`. The view model guard forbids a `crate::library::` import, and the busy state of the view models reads these types.
2. `IndexTrack` and `IndexRelease` hold `feed_guid: Option<String>`, not `String`. An Index item with no feed GUID but with a feed URL has a key. `IndexRelease` also holds `feed_url`, so the completion can find the Library feed.
3. `api_feed_from_view` and `api_track_from_view` moved to `src/library/downloads.rs`. A job with `feed: None` converts the feed that it reads, and that code runs in `LibraryApp`.
4. A refused job, for example an Index track with no key, is a failure of the batch. The progress item names it. The task did not give a place for this report.
5. `membership` takes two values, `downloading` and `removing`. Before, a busy release with Library tracks showed `Removing`. A queued track download would then show `Removing`.
6. The singles group main action is not available while a single of its group is busy. Before, any busy download made it not available.
7. A Library track job reads its track row from the database when it is queued and again when it starts. The job then downloads the stored row of that time.
8. The track page completion now runs for each Library track job of the open track. A bar or a release page can also queue that job.
9. The track page stays busy only while a job of its track is queued or running. When the queue drops the track for a release job, the page shows the usual button.
10. The guard R22-11 also reads `src/app.rs`, not only `src/app/`. `SubscribeThenAppendToPlaylist` stays in `TopApp`. It adds Index tracks to a playlist, and the task names no queue job for it.
11. `next_startable` is used only by tests. `start_next` starts the job that it gives.

### Unresolved Concerns

1. V1 says "At most three releases show Downloading at the same time". R22-09 says that a release shows `Downloading` while its key is queued or in flight. This packet follows R22-09, so nine queued releases all show `Downloading`. The operator must decide which rule is correct.
2. The progress item has no stop. A batch of many downloads runs to its end. The run record asks the operator about "Stop queued".
3. The source map does not list the four new files. Responsiveness item 6 of the overhaul plan is now shown by the progress item, but the plan still lists it. The task names only this packet as a document to edit.
4. The new checks V1 to V6 are not in `docs/pending-human-checks.md`, for the same reason.
5. An Index track with no track GUID has an empty track GUID in its identity. Two such tracks of one release are one identity, so the queue drops the second.
6. A removal of a release asks for its confirmation first. The refusal comes when the operator confirms.

## Operator Visual Check

This check needs a Linux desktop session, this checkout and a network connection to the MusicIndex endpoint and to the RSS hosts. It also needs these items:

- Nine or more Library releases that are not downloaded. Use releases that you can remove after the check.
- An Index release that is not in the Library, with an Index track page.
- A feed page with one Library release that is not downloaded and one Index release that is not in the Library.

The steps download audio files. The cleanup removes them.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. Open Music, then Releases, in the sidebar.

### V1: Nine releases in one batch

3. Click "Select". Check nine releases that are not downloaded. Click "Download 9 releases".
4. Look at the item below the live status strip. It must read "Downloading n of 9" and name one release.
5. Look at the release pages and the release rows during the batch. Count the releases that show "Downloading" and the releases that show "Queued".

**Wrong if:** the item does not show, or n is more than 9. Also wrong: more than three releases show "Downloading" at the same time, or a release that waits does not show "Queued". Also wrong: more than three downloads write files at the same time.

### V2: The item on each section

6. While the batch runs, open a different release page. The item must stay.
7. Open Show, then Settings. The item must show on each section.

**Wrong if:** the item goes, or its count goes back to a lower number.

### V3: A download that fails

This step makes the MusicIndex endpoint unreachable. Port 9 on the local host refuses the connection.

8. Wait until the batch of V1 shows its finish time. Click "Dismiss".
9. Open the feed page with the Library release and the Index release.
10. Open Settings. Record the present "MusicIndex endpoint" value. Change it to `http://127.0.0.1:9`, then save the settings.
11. Open Music. The feed page must show again. If the page shows an error, open it again from the Back history.
12. Click "Select". Check the Library release and the Index release. Click "Download 2 releases".
13. Look at the item. The Index release must fail, because the app cannot read it from MusicIndex. The Library release must download from its RSS feed.
14. The item must read "Downloaded 1 of 2, 1 failed. Finished hh:mm". A red line must name the Index release and the error.

**Wrong if:** the failure stops the Library release, or the item shows no failure line. Also wrong: the failure line does not name the release.

### V4: The finish time and "Dismiss"

15. Compare the time in the item with the clock of the desktop. The time must be the time when the last download ended.
16. Open a different page and wait one minute. The item must stay, with the same time.
17. Click "Dismiss". The item must go.

**Wrong if:** the time changes, or the item goes before "Dismiss".

### V5: Pages that change in place

18. Open Settings. Set the "MusicIndex endpoint" to the value of step 10, then save the settings.
19. Open the Index release page that is not in the Library. Click "Download". Stay on the page.
20. When the download ends, the page must change to the Library release page, with no navigation.
21. Open an Index track page of a different Index release. Click "Download album". Stay on the page.
22. When the download ends, the page must change to the Library track page, with no navigation.

**Wrong if:** a page stays an Index page after its download, or Back history gets a new entry.

### V6: Themes and a narrow width

23. Start a batch of two or more releases. Look at the item in the Light theme and in the Dark theme.
24. Make the window narrow. Look at the title, the release name and each failure line.

**Wrong if:** a text is clipped, or text overlaps the "Dismiss" button. Also wrong: a text has too little contrast in one theme.

### Cleanup

25. Make sure that the "MusicIndex endpoint" in Settings has the value of step 10.
26. Open Music, then Releases. Click "Select" and check each release that the check downloaded. Click "Remove n releases…", then confirm.
27. Close the app.

## Orchestrator Correction - 2026-10-20

The orchestrator made three corrections on 2026-10-20. The Status line does not change.

### C1: A queued job and a running job show different states

`TrackMembershipState` and `ReleaseMembershipState` have the new state `Queued`.

- A key with a job that waits, and with no job in flight, gives `Queued`. The state word is "Queued", and no download action is available.
- A key with a job in flight gives `Downloading`.
- `DownloadActivity` holds the jobs in flight in separate sets. `DownloadPhase` gives `Queued` or `InFlight` for a key, a release job and a track.
- A Library track row gets its state from the new `TrackRowWork`. It replaces the busy flag of the row.

Why: nine queued releases all showed "Downloading". Six of them did not run, so the word stated a false state.

Tests:

- `adr_0084_download_queue_r22_09_activity_holds_queued_and_running_jobs`
- `adr_0084_download_queue_r22_09_release_and_track_busy_follow_the_queue`
- `adr_0084_download_queue_nine_releases_three_downloading_six_queued`
- `album_detail_vm_membership_follows_library_tracks_and_busy_state`
- `track_action_state_projects_busy_and_disabled_membership_actions`
- `adr_0084_release_page_r21_03_library_row_matches_library_track_row_vm`

### C2: The identity of an Index track with no track GUID

The job identity of an Index track uses the track GUID first. With no track GUID, it uses the enclosure URL. With no track GUID and no enclosure URL, the job gets a unique identity, and the queue drops no such job as a duplicate.

Why: two tracks of one release with no track GUID had one identity. The queue dropped the second track.

Test: `adr_0084_download_queue_guidless_tracks_of_one_release_are_two_jobs`.

### C3: "Remove" is absent while a download of the key is active

While a job of the feed key is queued or in flight, "Remove" of the release and of its tracks is absent:

- The release page: the "Remove" menu item is absent, as before. Each track row of the release in the Library shows "Queued" or "Downloading..." in place of "Remove".
- The track page: "Remove track…" is absent from the menu.
- The Releases bar, the Tracks bar and the detail column bar: "Remove" does not count the item. The removal list of the confirmation does not hold the item.

No confirmation opens for an item that the removal cannot act on. The refusal in the removal command path of a release stays as a second guard.

Why: the confirmation opened for an action that could not act.

Tests:

- `adr_0084_download_queue_remove_is_absent_while_the_key_is_active`
- `adr_0084_download_queue_track_page_has_no_remove_while_release_downloads`
- `adr_0084_download_queue_r22_10_removal_refused_while_active`, the second guard

### Notes

- A release page shows no release state word. The state word shows on each track row of the release.
- The removal command path has no refusal for a track. Only the view model makes "Remove" of a track absent.
- V1 in "Operator Visual Check" now expects at most three releases with "Downloading". The other releases of the batch show "Queued". This change closes unresolved concern 1.
