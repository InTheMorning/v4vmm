# ADR 0066 Task 014: Download Failures And Dismissal Of Retained Actions

Status: Ready - 2026-10-03. Implementation has not started.
Its visual gate opens when the implementation is complete.

## Goal

A retained action names the failure that occurred. Only a conversion failure offers the converter setting. A later success of the same track supersedes its earlier failure. The operator can dismiss each row in the notice.

## Authority

- [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md):
  - "Issues remain visible until corrected or superseded by an actual successful observation."
  - "Full subjects, repair actions and retained original actions remain reachable within the notice and in Settings."
  - "For a WAV conversion failure, name the track and failed converter operation. Offer converter setup."
- ADR 0066 [task 009](adr-0066-task-009-conversion-retry-and-retained-input.md): retention is session-only.

## Recorded Facts - 2026-10-03

- The operator downloaded the Delta OG feed with an empty `flac_path`. The WAV conversion failed. The operator set `flac_path` to `flac`, and a second download of the feed completed.
  The notice then showed "12 retained actions". The failed rows offered only "Edit converter setting", and the operator could not dismiss them.
- The HeyCitizen download then gave two rows "Conversion: MoeFactz (track 2). App could not finish materializing this track: downloaded enclosure size mismatch ... Available input is retained for this session."
  The files are MP3 files. No conversion occurred. The download had already deleted the staged file.
- `ConversionRecovery::record_entry` in `src/application/conversion_recovery.rs` keeps an entry for each `Err` and for each `WavRetained` outcome. `report()` maps each `Err` to `ConversionState::Failed`.
- `sync_conversions` in `src/application/capability_recovery.rs` gives each report `Dependency::Converter`. Thus each failure, for example a network error, an HTTP status, a database error or "RSS did not identify the requested track", shows as "Conversion" with "Edit converter setting".
- `subscribe_feed` in `conversion_recovery.rs` does not find an earlier entry of the same track. Only the single-track path does. A later success adds a report, and the earlier failure stays.
- The collapsed notice gives `Dismiss` only to a completed row (`rows(false)` in `src/view_models/startup/capabilities.rs`). Settings → Background tools gives `Dismiss` to each row.
- The report text "Available input is retained for this session" shows also when the staging directory is gone.
- `ConversionState::RedownloadRequired` and the action "Redownload original track" exist for a retained input that cannot be used again.
- [ADR 0056 task 005](adr-0056-task-005-remove-enclosure-length-check.md) removes the size check. Other download failures stay.

## Required Changes

1. A retained report records the class of its failure: download, conversion or storage. Classify at the point of the error, in the materialization path, and not from the error text.
2. Only a conversion failure gets `Dependency::Converter`, the title "Conversion", and the converter repair action.
   A download failure gets the title "Download", and the action "Redownload original track". A storage failure names the database or file operation, and offers no converter action.
3. A success of the same track key, from the single-track path or the feed path, supersedes each earlier entry of that key. A superseded entry leaves the notice and the count.
4. The collapsed notice gives `Dismiss` to each row, as Settings does.
5. The report says "Available input is retained for this session" only when the retained input exists. Otherwise it states that the app deleted the downloaded input.
6. Retention stays session-only.

## Mechanical Acceptance Criteria

Use the prefix `adr_0066_download_failure_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R66-14-01 | A download error (HTTP status or transport) gives a report of class download, with no `Dependency::Converter` and the action "Redownload original track" |
| R66-14-02 | A WAV conversion failure gives class conversion, `Dependency::Converter`, and the converter repair action, as before |
| R66-14-03 | A database error during materialization gives class storage, with no converter action |
| R66-14-04 | A failed feed download and a later successful feed download of the same track leave no entry for that track. The count goes down by the superseded entries |
| R66-14-05 | The collapsed notice view model gives a `Dismiss` action to a failed row and to a completed row |
| R66-14-06 | A failure that deleted the staging directory gives no "Available input is retained" text |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: a download failure shows "Download: <track>" with "Redownload original track" and "Dismiss", and no converter action. Check in Light and Dark themes.
- V2: after a failed conversion and a successful retry of the same feed, the notice shows no row for those tracks.
- V3: each row of the collapsed notice has "Dismiss", and a click removes the row in place.

## Exclusions

- No size check. ADR 0056 task 005 removes it.
- No change to persistence across a restart.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/application/conversion_recovery.rs`, `src/application/capability_recovery.rs`, `src/application/materialization.rs`.
- `src/subscribe_service.rs`: `subscribe_feed_retaining`.
- `src/view_models/startup/capabilities.rs`, `src/ui/composites/startup_report.rs`, `src/app/capabilities.rs`.
- ADR 0066 tasks 007 and 009.

## Checks

```bash
cargo test --lib adr_0066_download_failure_
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

The implementer writes this section at completion. It gives numbered steps for V1 to V3.
It uses an isolated configuration and database copy, and a local or unreachable enclosure URL for the download failure. It never writes the real Library.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0066-task-014-download-failures-and-dismissal.md`
- ADR 0066, and its tasks 007 and 009
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the failure class, the supersede on success, the dismissal in the notice, and the true retained-input text.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment, each report text and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, action and availability. The screen only composes.
- Tests use local fixtures. No test sends a request to a remote host.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The download size check in `src/track_compare.rs`. ADR 0056 task 005 owns it.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R66-14-01 to R66-14-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V3.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The materialization path cannot give a failure class without a change to a file in "Do not touch".
- A superseded entry holds an operation that is still running.
