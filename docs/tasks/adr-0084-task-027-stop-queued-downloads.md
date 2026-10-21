# ADR 0084 Task 027: Stop The Queued Downloads

Status: Ready - 2026-10-20.

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
