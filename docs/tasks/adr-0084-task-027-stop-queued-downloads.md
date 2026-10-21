# ADR 0084 Task 027: Stop The Queued Downloads

Status: Implemented - 2026-10-20. Cases R27-01 to R27-06 pass. Mechanical checks are Green. V1 to V3 are open, in the final walk of the run.

## Goal

The progress item of the download queue gets a "Stop queued" action. It removes each job that waits, and the running jobs finish.
The finished item then names the stopped jobs, for example "Downloaded 3 of 9, 6 stopped".

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10, amended on 2026-10-20: one download queue and one progress item.
- Operator decision of 2026-10-20: add "Stop queued". A running download is not stopped, because a stop during a write leaves partial files.
- [AGENTS.md](../../AGENTS.md) "Write reports for the operator", and the durable rule of typed action state.
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md).

## Recorded Facts - 2026-10-20

- `DownloadQueue` in `src/library/download_queue.rs` holds the queued jobs and the jobs in flight. It gives `enqueue`, `start_next`, `finish`, `activity`, `batch` and `dismiss`.
- `DownloadProgressDisplay` in `src/view_models/download_progress.rs` gives the text, the failure text, the accessibility label and the typed "Dismiss" action (`DownloadProgressDismissAction`). "Dismiss" shows only for a finished batch.
- `LibraryApp::dismiss_download_report` and `render_download_status` are in `src/library/downloads.rs`. The composite is `src/ui/composites/download_status_item.rs`.
- A queued release and a queued track show "Queued" (packet 022 correction C1). The busy state comes from `DownloadActivity`.

## Design

- `DownloadQueue::stop_queued()` removes each queued job, adds their count to a new `stopped` count of the batch, and changes no job in flight. It returns the count.
- A batch with no job in flight after the stop is finished at once, with `finished_at` from the caller. A batch with running jobs finishes when its last job ends.
- `DownloadProgressDisplay` gives a typed "Stop queued" action only while the batch runs and one or more jobs wait. Its accessibility label names the count, for example "Stop 6 queued downloads".
- The text of a finished batch names the stopped jobs: "Downloaded 3 of 9, 6 stopped. Finished 14:02". With failures: "Downloaded 2 of 9, 1 failed, 6 stopped. Finished 14:02".
- A stopped release or track goes back to its earlier state, for example "Download", with no "Queued" state.
- The action asks for no confirmation, because it deletes no file and the operator can queue the jobs again.

## Required Changes

1. Add `stop_queued` and the `stopped` count, with tests.
2. Add the typed action and the new texts to `DownloadProgressDisplay`, with tests.
3. Add the button to the composite and its handler to `LibraryApp`. Refresh the open page in place.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_stop_queued_` for unit tests beside the owning code.

| Case | Required proof |
|---|---|
| R27-01 | `stop_queued` removes each queued job and changes no job in flight. It returns the count |
| R27-02 | A batch with no job in flight after the stop is finished, with the time that the caller gives |
| R27-03 | "Stop queued" shows only while the batch runs with one or more queued jobs. Its accessibility label names the count |
| R27-04 | The finished text names the stopped count, with and without failures |
| R27-05 | After the stop, `DownloadActivity` gives no `Queued` phase for a stopped key |
| R27-06 | Each packet 022 test still passes |

## Visual Acceptance Criteria

These are for the operator, in the final walk of the run. No test proves them.

- V1: queue nine releases. While jobs wait, the item shows "Stop queued". Click it. The waiting releases go back to "Download", and the running ones continue.
- V2: when the running jobs end, the item reads "Downloaded n of 9, m stopped. Finished hh:mm".
- V3: Light and Dark themes, and a narrow width. The button and the text are not clipped.

## Exclusions

- No stop of a running download.
- No stop of one queued job. The row menu of ADR 0083 task 006 can add it later.

## Files To Inspect

- `src/library/download_queue.rs`, `src/library/downloads.rs`, `src/view_models/download_progress.rs`, `src/ui/composites/download_status_item.rs`.
- `docs/tasks/adr-0084-task-022-download-queue.md`.

## Checks

```bash
cargo test --lib adr_0084_stop_queued_
cargo test --lib adr_0084_download_queue_
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
- This packet: `docs/tasks/adr-0084-task-027-stop-queued-downloads.md`
- Packet 022: `docs/tasks/adr-0084-task-022-download-queue.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each text, count and availability. The screen only composes. Use named tokens.
- Tests use recorded values and recorded times.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app.

Do not touch:
- The selection model and the release page assembly.
- The database schema.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R27-01 to R27-06 has a passing test.
- Each command in "Checks" is Green.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V3.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns.

Stop and report the problem, and do not guess, when:
- A stop needs a change to a job in flight.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-20

### Files Changed

- `src/library/download_queue.rs`: `BatchState` gets a new `stopped` count.
  `DownloadQueue::stop_queued` removes each queued job and adds their
  count to `stopped`. It finishes the batch at the given time when no job
  stays in flight. Tests for R27-01, R27-02 and R27-05.
- `src/view_models/download_progress.rs`:
  - `DownloadBatch` gets a `stopped` count and a `queued` count. The new
    type `DownloadProgressStopQueuedAction` holds the typed "Stop
    queued" action. `DownloadProgressDisplay` gets a `stop_queued`
    field.
  - `DownloadProgressDisplay::project` shows "Stop queued" only while
    the batch runs with one or more queued jobs. It also names the
    stopped count in the finished text.
  - New functions `stopped_downloads` and `stop_queued_a11y_label` build
    the count text. Tests for R27-03 and R27-04.
- `src/ui/composites/download_status_item.rs`: the composite takes a new
  `on_stop_queued` callback. It renders a "Stop queued" button with the
  `Stop` icon, adjacent to the "Dismiss" button. The shared handler type
  is renamed from `DismissHandler` to `ClickHandler`, because both
  buttons use it.
- `src/library/downloads.rs`: the new method
  `LibraryApp::stop_queued_downloads` calls `DownloadQueue::stop_queued`.
  It then refreshes the busy state and notifies the open page when the
  call removes one or more jobs. `render_download_status` wires the new
  button to this method.

### Behavior Changed

- The download progress item shows "Stop queued" while the batch runs
  and one or more jobs wait. The button names the count in its
  accessibility label, for example "Stop 6 queued downloads".
- A click on "Stop queued" removes each job that waits. A running job
  keeps writing its file, and it is not stopped.
- A stopped release or a stopped track goes back to its earlier state,
  for example "Download". The queue no longer holds its job.
- A batch with no job in flight after the stop finishes at once, with
  the time of the click. A batch with running jobs finishes when its
  last job ends.
- The finished text names the stopped count. For example, "Downloaded 3
  of 9, 6 stopped. Finished 14:02". With failures: "Downloaded 2 of 9, 1
  failed, 6 stopped. Finished 14:02".
- "Stop queued" shows no confirmation dialog, because it deletes no file
  and the operator can queue the jobs again.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_stop_queued_` | Green, 6 tests |
| `cargo test --lib adr_0084_download_queue_` | Green, 21 tests |
| `cargo test` | Green, 2070 unit tests and 344 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 344 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy --all-targets -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred. No test
sends a request or writes a file.

### Deviations From The Task

1. The accessibility label of "Stop queued" also gives the singular
   word for one queued job, "Stop 1 queued download". The packet names
   only the plural example, "Stop 6 queued downloads". The singular word
   follows the same pattern as the existing failed-download count text.

### Unresolved Concerns

1. AGENTS.md asks each fix to leave its gate open in
   `docs/pending-human-checks.md` too. This task names only the packet
   file as an editable document, so checks V1 to V3 are not yet in that
   index. The operator should add them there.
2. Packet 022's unresolved concern about a missing "Stop queued" action
   is now resolved. Its other concerns, for example the source map not
   listing the new queue files, stay open and are outside this packet.

## Operator Visual Check

This check needs a Linux desktop session, this checkout, and nine or
more Library releases that are not downloaded. Use releases that you
can remove after the check.

The steps download audio files. The cleanup removes them.

1. Build the app: `cargo build --bin v4vmm`. Then start
   `target/debug/v4vmm` from a desktop session.
2. Open Music, then Releases, in the sidebar.

### V1: "Stop queued" removes the waiting releases

3. Click "Select". Check nine releases that are not downloaded. Click
   "Download 9 releases".
4. Look at the item below the live status strip. It must show the
   button "Stop queued".
5. Click "Stop queued".
6. Look at the release rows. The releases that waited must go back to
   "Download". The releases that ran must keep "Downloading" and
   continue.

**Wrong if:** "Stop queued" is absent while jobs wait. Also wrong: a
running release stops, or a release that waited keeps "Queued" after
the click.

### V2: The finished text names the stopped count

7. Wait until each running release ends.
8. Look at the item. It must show "Downloaded n of 9, m stopped.
   Finished hh:mm". Here, n is the releases that finished, and m is the
   releases that this check stopped.
9. Click "Dismiss".

**Wrong if:** the text does not name the stopped count, or the item
does not show a finish time.

### V3: Themes and a narrow width

10. Repeat steps 3 to 5 with the app in the Light theme. Look at the
    item and the "Stop queued" button.
11. Repeat steps 3 to 5 with the app in the Dark theme.
12. Make the window narrow. Look at the item title, the item name and
    the "Stop queued" button.

**Wrong if:** a text is clipped, the button label overlaps another
element, or a text has too little contrast in one theme.

### Cleanup

13. Open Music, then Releases. Click "Select" and check each release
    that this check downloaded.
14. Click "Remove n releases…", then confirm.
15. Close the app.
