# ADR 0059 Task 018: The Service Watch Reads At The Pace Of Its Viewer

Status: Ready - 2026-10-18. Implementation has not started.

## Goal

The broadcast service watch reads the publisher, the producer and the encoder only as often as the operator can see the result.
The app redraws the window only when a shown state changes. The processor load in Music and Settings falls.

## Authority

- [ADR 0059](../adr/0059-broadcast-control-surface.md), decision "The Service Watch Reads At The Pace Of Its Viewer", and its invariant. The operator decided the pace on 2026-10-18.
- [ADR 0040](../adr/0040-async-vm-runtime.md): blocking work runs in a runtime actor, and `src/presentation/gpui_vm_bridge.rs` is the only bridge from a watch to GPUI.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability. The pace and the redraw decision live outside the screens.

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

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
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

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V3.

V1 measures the processor load with `top -b -d 5 -n 6 -p "$(pgrep -n v4vmm)"` in Music, before and after the change, with the same binary type. It also counts `butt -S` starts over 60 seconds, for example with `pgrep -f 'butt -S'` in a loop, or with the process accounting tool that the implementer names.

V2 and V3 need the configured publisher unit, the producer unit and `butt`. The cleanup returns each service to its state before the check.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0059-task-018-service-watch-pace.md`
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
