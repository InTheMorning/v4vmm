# ADR 0059 Task 018: The Service Watch Reads At The Pace Of Its Viewer

Status: Complete - 2026-10-18. Cases R59-18-01 to R59-18-08 pass. Mechanical checks are Green. The operator passed V1 to V3 on 2026-10-18 with the debug build.

## Goal

The broadcast service watch reads the publisher, the producer and the encoder only as often as the operator can see the result.
The app redraws the window only when a shown state changes. The processor load in Music and Settings falls.

## Authority

- [ADR 0059](../../adr/0059-broadcast-control-surface.md), decision "The Service Watch Reads At The Pace Of Its Viewer", and its invariant. The operator decided the pace on 2026-10-18.
- [ADR 0040](../../adr/0040-async-vm-runtime.md): blocking work runs in a runtime actor, and `src/presentation/gpui_vm_bridge.rs` is the only bridge from a watch to GPUI.
- The durable set in [AGENTS.md](../../../AGENTS.md): renderer portability. The pace and the redraw decision live outside the screens.

## Recorded Facts - 2026-10-18

- `BROADCAST_SERVICE_POLL_INTERVAL` is one second (`src/runtime/broadcast_service_watch.rs:20`). The loop reads each unit and the encoder on each tick, and on `BroadcastServiceWatchCommand::RefreshNow`.
- Each read starts `systemctl --user show` for the publisher unit and for the producer unit (`ServiceControl::show` in `src/broadcast/control.rs`), and `butt -S` (`status` in `src/broadcast/encoder.rs`). With an SSH transport, each unit read starts `ssh`.
- On 2026-10-16, in a 41-second profile, the app started the `butt` AppImage 16 times. Those starts used 72% of the processor samples (overhaul plan, responsiveness item 11).
- `maybe_start_broadcast_service_watch` (`src/app/show.rs`) starts the watch at app start, in each section (`src/app/bootstrap.rs`, `src/app/capabilities.rs`).
- `publish_snapshot` sends each read with `watch::Sender::send`. `bridge_watch` calls `cx.notify()` after each snapshot. `BroadcastServiceWatchSnapshot` carries `read_started_at` and `at`, so two snapshots are never equal.
- `apply_publisher_service_snapshot` passes each snapshot to `ShowCommandState::observe`, stores it, projects the Show page again, and can read the publisher logs while Show is open.
- `ShowCommandState::observe` (`src/view_models/show.rs`) keeps a command until its intent is resolved, or until 3 fresh reads after the command returned (`MAX_FRESH_SAMPLES`). A fresh read has a `read_started_at` after `returned_at`.
- Only the Show page and the Show commands read the snapshot. The live status strip of Music does not read it (`broadcast_active` of `LiveStatusProjection` is set only in tests).
- `select_tab` in `src/app.rs` changes the section. `open_broadcast_readiness_in_music` in `src/app/show.rs:1054` also sets `self.tab = AppTab::Music` directly.

## Design

### The pace

Add a typed pace to the watch:

```rust
/// How often the service watch reads (ADR 0059).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BroadcastServiceWatchPace {
    /// Show is open, or a Show command waits for its result.
    Viewed,
    /// Each other condition.
    Background,
}
```

| Pace | Unit read | Encoder read |
|---|---|---|
| `Viewed` | Each 1 second | Each 5 seconds |
| `Background` | Each 10 seconds | Each 10 seconds |

- Named constants hold the three durations. The watch starts with `Background`.
- Add `BroadcastServiceWatchCommand::SetPace(BroadcastServiceWatchPace)` and `BroadcastServiceWatchHandle::set_pace`. A change to `Viewed` reads all services at once.
- The loop reads the units at the unit interval. It reads the encoder only when its interval passed since the last encoder read. Else the snapshot keeps the last encoder value. Use a monotonic clock (`tokio::time::Instant`), so a test can pause time.
- `RefreshNow` reads the units and the encoder.
- The first read at start stays.

### The pace of the app

- One function gives the pace: `Viewed` when `self.tab == AppTab::Show`, or when `ShowCommandState` has a command that waits. Else `Background`.
- Add `ShowCommandState::has_waiting(&self) -> bool`.
- The app sends the pace after each change of section, through `select_tab` and through `open_broadcast_readiness_in_music`. It sends it again after each `observe`, because a command can end outside Show. Send only when the pace changes.

### The redraw

- Add `bridge_watch_with_redraw` to `src/presentation/gpui_vm_bridge.rs`. Its `apply` returns `bool`, and the bridge calls `cx.notify()` only when the value is `true`. Keep `bridge_watch` for each other caller. Update the module comment.
- `apply_publisher_service_snapshot` returns `true` only when one of these changed:
  - the state of a unit, or the encoder snapshot, compared with the stored snapshot, without `read_started_at` and `at`
  - the progress of a command (the result of `observe`)
  - a log read that it started
- It keeps each present effect: `observe`, the stored snapshot, the projection of the Show page, and the log read.

## Required Changes

1. Add `BroadcastServiceWatchPace`, its constants, `SetPace`, `set_pace` and the encoder interval to the watch. Delete `BROADCAST_SERVICE_POLL_INTERVAL` when no caller remains.
2. Add `ShowCommandState::has_waiting` and the pace function, and send the pace from each section change and after each `observe`.
3. Add `bridge_watch_with_redraw`, and use it for the service watch only.
4. Change `apply_publisher_service_snapshot` to return the redraw decision.
5. Rewrite each comment that names the one-second poll.

## Mechanical Acceptance Criteria

Use the prefix `adr_0059_watch_pace_` for behavioral tests beside the owning code. The watch tests use a counting fake `BroadcastStatusReader` and `tokio::time::pause`.

| Case | Required proof |
|---|---|
| R59-18-01 | With `Background`, 30 seconds give 3 unit reads and 3 encoder reads after the first read, at most |
| R59-18-02 | With `Viewed`, 10 seconds give 10 unit reads and 2 encoder reads after the first read, plus or minus one |
| R59-18-03 | `set_pace(Viewed)` reads the units and the encoder at once |
| R59-18-04 | `refresh_now` reads the units and the encoder, also before the encoder interval ends |
| R59-18-05 | A snapshot between two encoder reads keeps the last encoder value |
| R59-18-06 | The pace function gives `Viewed` for Show, `Viewed` for Music with a waiting command, and `Background` for Music and Settings with no waiting command |
| R59-18-07 | `apply_publisher_service_snapshot` returns `false` when only `read_started_at` and `at` differ. A changed unit state, encoder status or command progress gives `true` |
| R59-18-08 | A command that waits for fresh reads still ends after 3 fresh reads, at the `Viewed` pace, when the state does not change |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: in Music and Settings, the processor load of the app is lower than before, and the fan stays slow. The app starts `butt -S` at most each 10 seconds.
- V2: when Show opens, the Live Metadata and Stream sections show the current state at once. A started or stopped publisher shows within about 1 second, and a connected or disconnected `butt` within about 5 seconds.
- V3: "Start publisher" and "Stop publisher" in Show still show their progress and their result. A command that the operator starts and then leaves Show still ends with its result.

## Exclusions

- No change to the playback poll (`src/runtime/playback_polling.rs`). Responsiveness item 12 keeps it.
- No change to the commands or to what the Show page shows.
- No D-Bus subscription. The watch keeps its command reads.
- No schema change.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../architecture/source-map.md).
- `src/runtime/broadcast_service_watch.rs`, `src/broadcast/control.rs`, `src/broadcast/encoder.rs`.
- `src/presentation/gpui_vm_bridge.rs`.
- `src/app/show.rs` (`maybe_start_broadcast_service_watch`, `apply_publisher_service_snapshot`, `open_broadcast_readiness_in_music`), `src/app.rs` (`select_tab`), `src/app/capabilities.rs`, `src/app/bootstrap.rs`.
- `src/view_models/show.rs` (`ShowCommandState`).
- `tests/architecture_tests.rs`: the guards that name the watch or the bridge.

## Checks

```bash
cargo test --lib adr_0059_watch_pace_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Implementation Result - 2026-10-18

Files changed:

- `src/runtime/broadcast_service_watch.rs`: added `BroadcastServiceWatchPace`, its three named
  interval constants, `BroadcastServiceWatchCommand::SetPace`, `BroadcastServiceWatchHandle::set_pace`,
  and the pace-aware read loop. Removed `BROADCAST_SERVICE_POLL_INTERVAL`. The loop tracks the
  last encoder read time with `tokio::time::Instant` and carries the last encoder snapshot over a
  unit-only tick. Added the `adr_0059_watch_pace_` tests and a `CountingBroadcastStatusReader` fake.
- `src/view_models/show.rs`: `ShowCommandState::observe` returns `true` when a waiting command ends.
  Added `ShowCommandState::has_waiting`, the pace function `broadcast_service_watch_pace`, and the
  redraw function `broadcast_service_snapshot_redraw`. Added `adr_0059_watch_pace_` tests for the
  pace function, the redraw function, and the three-fresh-read release at the `Viewed` pace.
- `src/presentation/gpui_vm_bridge.rs` and `src/presentation/mod.rs`: added `bridge_watch_with_redraw`,
  which calls `cx.notify()` only when `apply` returns `true`. `bridge_watch` calls it with an
  `apply` that always returns `true`, so each other caller keeps its initial behavior.
- `src/app.rs`: added the `broadcast_service_watch_pace` field, its start value, and the
  `BroadcastServiceWatchPace` import. `select_tab` sends the pace after it sets the tab and focuses it.
- `src/app/show.rs`: `apply_publisher_service_snapshot` returns the redraw decision and calls
  `sync_broadcast_service_watch_pace` after `observe`. Added `sync_broadcast_service_watch_pace`,
  which sends the watch a changed pace only. `maybe_start_broadcast_service_watch` uses
  `bridge_watch_with_redraw`, and sends the pace again after it starts the watch.
  `open_broadcast_readiness_in_music` sends the pace after it sets the tab to Music.
- `src/runtime/mod.rs`: exported `BroadcastServiceWatchPace`.
- `Cargo.toml`: added a `tokio` dev-dependency with the `test-util` feature, so a test can pause and
  read `tokio::time::Instant`. The production binary does not include this feature.
- `docs/tasks/archive/adr-0059-task-018-service-watch-pace.md`: this packet. The Status line, this section,
  and the Operator Visual Check section below.

Behavior changed:

- Outside Show, with no Show command waiting, the watch reads each unit and the encoder each 10
  seconds (`Background` pace). Before this change, it read each one each second.
- In Show, or with a Show command waiting, the watch reads each unit each second and the encoder
  each 5 seconds (`Viewed` pace).
- A change of pace to `Viewed` reads all services at once, so Show never shows a value older than
  its new pace.
- `refresh_now` still reads the units and the encoder at once, at either pace.
- The app redraws only when a unit's state, the encoder's status, or a command's progress changes.
  A tick that only changes the read time no longer redraws.

Checks:

| Check | Result |
|---|---|
| `cargo test --lib adr_0059_watch_pace_` | Green. 8 passed. |
| `cargo test` | Green. 1937 lib tests, 317 architecture tests, 10 ignored doc tests. |
| `cargo test --test architecture_tests` | Green. 317 passed. |
| `cargo fmt -- --check` | Green after one `cargo fmt` pass fixed three wrapped lines its own formatter had not yet seen. |
| `cargo clippy -- -D warnings` | Green. No warning. |
| `cargo check --all-targets` | Green. No warning. |
| `cargo build --bin v4vmm` | Green. See its own result below. |

Deviations from the task:

- Added the `tokio` `test-util` dev-dependency to `Cargo.toml`. The packet does not name this file.
  The watch tests need `tokio::time::pause` and `#[tokio::test(start_paused = true)]`. Without this
  feature, Tokio removes each one from the compiled test code. This dependency change applies to
  tests only. The production binary does not include it.
- `maybe_start_broadcast_service_watch` sends the pace again after it starts the watch. The Design
  section names only `select_tab`, `open_broadcast_readiness_in_music`, and each `observe` as senders.
  A config change can restart the watch while Show is open, or while a command waits. The restarted
  watch always starts at `Background`. Without this added send, that one condition would stay at
  `Background` until the next section change or `observe`.
- `ShowCommandState::observe` and `apply_publisher_service_snapshot` return the changed boolean
  values the packet describes, so each caller gets the fact it needs. R59-18-07 is proved against
  the extracted `broadcast_service_snapshot_redraw` function, not against
  `apply_publisher_service_snapshot` directly. That method takes a GPUI `Context<TopApp>`, and no
  fixture in this project builds a full `TopApp` in a test. `apply_publisher_service_snapshot` is a
  thin wrapper around the tested function, the stored snapshot, the reprojection, and the log read,
  in the sequence the packet names.

Unresolved concerns:

- The V1 `butt -S` count in the Operator Visual Check below polls each 0.05 seconds. A `butt -S`
  invocation shorter than that can be missed. The section names a more accurate alternative.
- No person walked the Operator Visual Check. The visual gate stays open.

## Operator Visual Check

V1, V2, and V3 are open. No test proves them. Follow each numbered list at a terminal in a desktop
session. Do not run `cargo run` or any other display attempt. That rule stands.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

### V1: processor load and `butt` starts in Music

Needed state: the configured publisher unit, the producer unit, and `butt` are all reachable, as in
normal use. Pick one binary type (debug or release) and use that same type for both runs.

1. Build the binary: `cargo build --bin v4vmm` (debug) or `cargo build --release --bin v4vmm`
   (release).
2. Start v4vmm and open Music. Leave it open and idle.
3. Find its process ID: `pgrep -n v4vmm`.
4. Measure processor load for 30 seconds: `top -b -d 5 -n 6 -p "$(pgrep -n v4vmm)"`.
5. Save this script as `count_butt_starts.sh` and run it with `bash count_butt_starts.sh`. It counts
   each new `butt -S` process over 60 seconds:

   ```bash
   #!/usr/bin/env bash
   rm -f /tmp/butt-s-starts.seen
   touch /tmp/butt-s-starts.seen
   end=$((SECONDS + 60))
   while [ "$SECONDS" -lt "$end" ]; do
     for pid in $(pgrep -f "butt -S" 2>/dev/null); do
       if ! grep -qx "$pid" /tmp/butt-s-starts.seen 2>/dev/null; then
         echo "$pid" >> /tmp/butt-s-starts.seen
       fi
     done
     sleep 0.05
   done
   echo "butt -S starts in 60 seconds: $(wc -l < /tmp/butt-s-starts.seen)"
   ```

   For a more exact count, use a process accounting tool such as `execsnoop-bpfcc` instead, filtered
   to `butt`.
6. Repeat steps 3 to 5 while Show is open. Compare the two results.

What would count as wrong: the processor load in Music stays as high as the 2026-10-16 profile (72%
of samples busy over 41 seconds). The script reports more than about 6 `butt -S` starts in Music over
60 seconds (more than roughly once each 10 seconds). The fan runs as fast in Music as in Show.

Cleanup: the script stops on its own after 60 seconds. Remove its temporary file:
`rm -f /tmp/butt-s-starts.seen count_butt_starts.sh`. Close v4vmm.

### V2: Show shows the current state at once

Needed state: the configured publisher unit, the producer unit, and `butt` are all reachable. You
can start, stop, or connect each one from outside the app, for example with `systemctl --user` and a
`butt -S` or connect command.

1. Start v4vmm. Open Music or Settings.
2. Leave it open for 15 seconds or more. The watch then settles at the `Background` pace.
3. Outside the app, change one service: stop or start the publisher unit, or connect or disconnect
   `butt`.
4. In the app, select Show. Record the time of this step.
5. Watch the Live Metadata section's publisher and producer rows, and the Stream section.

What would count as wrong: the Live Metadata section still shows the state from before step 3.
This is wrong only if it stays for more than about 1 second after you open Show. The Stream section
still shows the old connection state for more than about 5 seconds after you open Show.

Cleanup: return each service to the state it had before step 3. Close v4vmm.

### V3: a command started in Show still ends with its result

Needed state: the configured publisher unit is reachable, and is in a state where "Start publisher"
or "Stop publisher" has a real effect (for example, stopped, so Start changes it).

1. Start v4vmm and open Show.
2. Press "Start publisher" (or "Stop publisher" if the unit is already active).
3. Confirm the button shows a working state right after the press.
4. Before the command finishes, switch to Music or Settings.
5. Wait about 10 seconds.
6. Switch back to Show.
7. When Settings shows a publisher recovery action, for example "Start publisher", stay in Settings and press it.
8. Check that its result shows within a few seconds.

What would count as wrong: the button still shows "working" long after the unit actually started or
stopped. The Live Metadata section shows a state that disagrees with the unit's real state, checked
with `systemctl --user status`. The command's result (success or failure) never reaches Show. The Settings action of step 7 shows no result for about 10 seconds or more.

Cleanup: return the publisher unit to the state it had before step 2.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/archive/adr-0059-task-018-service-watch-pace.md`
- ADR 0059, decision "The Service Watch Reads At The Pace Of Its Viewer", and ADR 0040
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes" 1 to 5: the pace of the watch, the pace of the app, and the redraw decision.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Blocking reads stay in the actor, on the blocking pool. No screen decides the pace or the redraw.
- Tests use a fake reader and paused time. No test starts `systemctl`, `ssh` or `butt`.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `src/runtime/playback_polling.rs` and each other caller of `bridge_watch`.
- The Show commands, and what the Show page shows.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R59-18-01 to R59-18-08 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V3.

Test commands:
- The commands in "Checks", with `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A screen other than Show reads the service snapshot.
- `observe` needs a read that the `Background` pace does not give.
- A change needs a file in "Do not touch".

## Orchestrator Review - 2026-10-18

Pass, after one fix. The orchestrator made the fix in the same change.

- A command that began outside Show, for example from a Settings recovery action, kept the `Background` pace until the next read. That read could come 10 seconds later, so the command waited longer for its first fresh read.
  `dispatch_show_command` and the event target path of `src/app/show.rs` now send the pace when the command begins.
  No test reaches `TopApp`, so the manual check V3 covers it: a command started from Settings shows its result within a few seconds.
- V2 named step 2 for the change of state. The change happens in step 3.
- The `tokio` dev-dependency with `test-util` reaches only the test build. `cargo tree -e normal,build,features -i tokio` shows no `test-util`.
- The rest agrees with the packet: the pace constants, `SetPace`, the encoder interval, the redraw decision, and the pace at each section change and after each watch restart.

