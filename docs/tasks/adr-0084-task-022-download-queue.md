# ADR 0084 Task 022: The Download Queue

Status: Ready - 2026-10-20.

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
