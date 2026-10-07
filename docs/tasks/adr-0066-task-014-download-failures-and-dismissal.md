# ADR 0066 Task 014: Download Failures And Dismissal Of Retained Actions

Status: Open - the operator passed V1 and V3 on 2026-10-07, in Dark and Light. On 2026-10-07 the operator moved V2 to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07), under Phase 4: status bar and reports.

## Goal

A retained action names the failure that occurred. Only a conversion failure offers the converter setting. A later success of the same track supersedes its earlier failure. The operator can dismiss each row in the notice.

## Authority

- [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md):
  - "Issues remain visible until corrected or superseded by an actual successful observation."
  - "Full subjects, repair actions and retained original actions remain reachable within the notice and in Settings."
  - "For a WAV conversion failure, name the track and failed converter operation. Offer converter setup."
- ADR 0066 [task 009](archive/adr-0066-task-009-conversion-retry-and-retained-input.md): retention is session-only.

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
- [ADR 0056 task 005](archive/adr-0056-task-005-remove-enclosure-length-check.md) removes the size check. Other download failures stay.

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

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
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

This procedure covers V1 to V3. It uses a copy of the database and its own configuration. It does not write the real Library.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

### What You Need

- A Linux desktop session (X11) and this checkout at `/home/citizen/build/v4vmm`.
- The `sqlite3` command and `python3` 3.11 or later.
- The `flac` command on `PATH` for V2. Without it, write "not run" for V2.
- Network access to the configured MusicIndex endpoint and to the Delta OG feed host for V2.
- A closed local TCP port for V1. Step 3 finds one.

### Make The Fixture

1. Close v4vmm. The command `pgrep -a v4vmm` must show no output.
2. Build the desktop binary:

   ```bash
   cd /home/citizen/build/v4vmm && cargo build --bin v4vmm
   ```

3. Make the fixture directory, the database copy and the configuration. The configuration sets its own `db_path` and `music_dir`, and no `flac_path`. Corrected on 2026-10-07: the app rejects `flac_path = false` as a setup issue that pauses configuration saves (ADR 0066).

   ```bash
   FX=$(mktemp -d /tmp/v4vmm-task014.XXXXXX)
   CFG="${XDG_CONFIG_HOME:-$HOME/.config}/v4vmm/config.toml"
   mkdir -p "$FX/config/v4vmm" "$FX/data" "$FX/cache" "$FX/music"
   REAL_DB=$(python3 -c 'import os,sys,tomllib; print(os.path.expanduser(tomllib.load(open(sys.argv[1],"rb"))["db_path"]))' "$CFG")
   sqlite3 "$REAL_DB" ".backup '$FX/app.sqlite'"
   python3 - "$CFG" "$FX" <<'EOF'
   import re, sys
   source, fx = sys.argv[1], sys.argv[2]
   keep = [line for line in open(source).read().splitlines()
           if not re.match(r"\s*(music_dir|db_path|flac_path)\s*=", line)]
   head = [f'music_dir = "{fx}/music"', f'db_path = "{fx}/app.sqlite"']
   open(f"{fx}/config/v4vmm/config.toml", "w").write("\n".join(head + keep) + "\n")
   EOF
   PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
   fx_run() { ( export XDG_CONFIG_HOME="$1/config" XDG_DATA_HOME="$1/data" XDG_CACHE_HOME="$1/cache"; shift; /home/citizen/build/v4vmm/target/debug/v4vmm "$@" ) }
   ```

   The Python command closes its socket when it stops. Thus nothing listens on `$PORT`.
4. Select one MP3 Library track in the copy. Point its enclosure to the closed port, and remove its file binding:

   ```bash
   sqlite3 "$FX/app.sqlite" "SELECT t.id, t.track_title FROM tracks t JOIN local_files l ON l.track_id = t.id WHERE t.is_in_library = 1 AND t.enclosure_type = 'audio/mpeg' LIMIT 1;"
   TID=<the id from the output>
   sqlite3 "$FX/app.sqlite" "UPDATE tracks SET enclosure_url = 'http://127.0.0.1:$PORT/closed.mp3' WHERE id = $TID; DELETE FROM local_files WHERE track_id = $TID;"
   ```

   Write down the track title. If the query shows no row, use `audio/mp3` or remove the type condition.

### V1: A Download Failure

1. Start the app on the fixture: `fx_run "$FX"`.
2. Open **Music → Library**. Search for the track title from step 4. Select the track and click **Download**.
3. Look at the notice. Expect the summary "1 retained action".
   Expect the row `Download: <title> (track <TID>)`. The text starts with "App could not download the original enclosure" and ends with "App deleted the downloaded input."
4. Expect the actions **View download actions** and **Dismiss** in the notice row.
   It is wrong if the row shows "Conversion", **Edit converter setting**, or "Available input is retained for this session".
5. Click **View download actions**. Settings opens **Background tools**.
   Expect the same row with **Redownload original track** and **Dismiss**. Expect no **Edit converter setting** and no **Check converter setting** in that row.
6. Make sure that **Redownload original track** is enabled before any check.
   Click it once. The port is closed, so the redownload fails again. Expect the row to stay, with a new time and the same title "Download".
7. Do steps 3 to 6 again in the other theme. Open **Settings → General** with Ctrl+Comma to select Light or Dark.

### V3: Dismiss In The Collapsed Notice

1. Keep the V1 row. Click **Download** on the same track one more time to make a second failure.
   Expect one row for the track. A new failure replaces an earlier failure that keeps no input.
2. In the collapsed notice, click **Dismiss** on the row.
   Expect the row to go away at once, without a change of section. Expect the summary to lose the row.
   It is wrong if the row stays until you open Settings or change to another section.
3. Make the V1 failure again. In **Settings → Background tools**, click **Dismiss**. Expect the same result.

### V2: A Successful Download Supersedes The Failed Conversion

1. Make the conversion fail. Close the app. Add `flac_path = "/nonexistent/v4vmm-no-flac"` to `$FX/config/v4vmm/config.toml`, and start the app again.
   The conversion falls back to `ffmpeg`. If `command -v ffmpeg` prints a path, put a failing `ffmpeg` first on `PATH` for the fixture app only:

   ```bash
   mkdir -p "$FX/bin"; printf '#!/bin/sh\nexit 1\n' > "$FX/bin/ffmpeg"; chmod +x "$FX/bin/ffmpeg"
   ( export PATH="$FX/bin:$PATH" XDG_CONFIG_HOME="$FX/config" XDG_DATA_HOME="$FX/data" XDG_CACHE_HOME="$FX/cache"; /home/citizen/build/v4vmm/target/release/v4vmm )
   ```

   Search the Index for "Delta OG". Download the feed.
   Expect rows `Conversion: <track> (track <id>)` with **Edit converter setting** and **Dismiss**.
   Write down the count in the notice summary.
2. Click **Edit converter setting**. Set `flac_path` to `flac` and save. Run **Check converter setting**. Expect the check to pass.
3. Download the same Delta OG feed from the Index again.
4. Expect no row for the Delta OG tracks in the notice and in **Settings → Background tools**.
   Expect the count to go down by the number of Delta OG rows. It is wrong if a "Conversion" row or a "FLAC converted" row stays for a Delta OG track.
5. Look at Light and Dark again for the empty notice state.

### Cleanup

1. Close the fixture app. The command `pgrep -a v4vmm` must show no output.
2. Delete the fixture: `rm -rf "$FX"`. The real configuration and the real database did not change.
3. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
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

## Implementation Result - 2026-10-03

### 1. Files Changed

- `src/subscribe_service/materialization.rs`: the types `FailureClass` and `StorageOperation`. Each step of `Materialization::run` sets the class before it starts. The accessors are `failure()` and `has_retained_input()`.
- `src/application/conversion_recovery.rs`: each `ConversionReport` has a `failure` field. The report text names the failed operation. It states the retained input only when that input exists. A success supersedes the earlier reports of its track key. `subscribe_feed_at` lets a test give its own configuration path.
- `src/application/capability_recovery.rs`: `RecoveryAction::Conversion` has a `failure` field. Only the class conversion gets `Dependency::Converter`. A download or storage action uses `Dependency::LibraryPaths` and needs no setup check (`requires_check`).
- `src/view_models/startup/capabilities.rs`: the titles "Download", "Conversion" and "Storage". Each collapsed row has **Dismiss**. A download or storage row has no converter action. The new label is "Retry original track".

### 2. Tests Run

- `cargo test --lib adr_0066_download_failure_`: 7 tests, Green.
  - R66-14-01: `adr_0066_download_failure_gives_download_class_and_redownload_action`. It uses an HTTP 500 response and a closed port.
  - R66-14-02: `adr_0066_download_failure_conversion_keeps_converter_action`.
  - R66-14-03: `adr_0066_download_failure_database_error_gives_storage_class`.
  - R66-14-04: `adr_0066_download_failure_feed_success_supersedes_failure`. A second test, `adr_0066_download_failure_single_track_success_supersedes_failure`, covers the single-track path.
  - R66-14-05: `adr_0066_download_failure_collapsed_notice_dismisses_each_row`.
  - R66-14-06: `adr_0066_download_failure_deleted_staging_gives_no_retained_text`.
- Each test uses a local HTTP fixture on `127.0.0.1`. No test sends a request to a remote host.
- A temporary change disabled each fix in turn. With that change, the tests for R66-14-01, 03, 04, 05 and 06 failed. The single-track test also failed. R66-14-02 passed before and after the change, because it proves that the conversion behavior stays the same.
- `cargo test`: 1800 library tests and 289 architecture tests, Green.
- `cargo test --test architecture_tests`, `cargo fmt -- --check`, `cargo clippy -- -D warnings` and `cargo check --all-targets`: Green, with no warning.
- `cargo build --bin v4vmm`: Green.

### 3. Behavior Changed

- A retained report records its class: download, conversion or storage. The failed step sets the class. The app does not read the error text for the class.
- A download failure shows "Download: <track>". Its action is **Redownload original track**, and it needs no converter check. A storage failure shows "Storage: <track>". Its text names the library database operation or the music file operation.
- A failure with no retained input gets the state `RedownloadRequired` at once. Its text says "App deleted the downloaded input." for a download failure, and "App has no retained input for this track." for other failures.
- A success of a track key removes each earlier report of that key, from the single-track path, the feed path or a retry. The app does not remove a running report. A new failure removes only earlier reports of the key that keep no input.
- A successful new download that keeps no staging records no report. Before this change, a FLAC conversion of a new download added a "Completed" row.
- The single-track path refuses a new download only when an earlier entry of the key keeps input. An entry with no input no longer blocks a new download.
- Each row of the collapsed notice has **Dismiss**. A row that needs a check also has **Edit** or **View**. A download or storage row has **View download actions** or **View storage actions**. The notice does not run an action.
- Retention stays session-only.

### 4. Deviations From Task

- A download or storage action skips the setup check before its retry. No check applies to the network or to a storage failure, so a check gate would leave a dead control. The retry still compares the database path, the music directory and the track subject.
- The collapsed notice shows **View download actions** for a download row. **Redownload original track** is in **Settings → Background tools**. The notice keeps its rule that it does not run an action.
- The new rule for a successful new download (no report) and the relaxed single-track refusal are necessary for R66-14-04 and V2.

### 5. Unresolved Concerns

- The feed path does not use the single-flight set of the single-track path. A feed download can run while a retry of the same key runs. The supersede step does not remove that running report. When the retry ends, its report comes back.
- An error in the `download_track` function always gets the class download, also when the app cannot make the staging directory. A finer class needs a change in `src/track_compare.rs`. This packet does not change that file.
- The visual gate is open. No person has done V1 to V3.
