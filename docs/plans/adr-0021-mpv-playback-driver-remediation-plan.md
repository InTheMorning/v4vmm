# ADR 0021 Remediation Plan: mpv Playback Driver

## Goal

Address the review findings against ADR 0021 before implementing the mpv
driver. The revised ADR must make runtime ownership, command semantics,
pause state, IPC behavior, socket/process hygiene, and configuration explicit
enough that implementation cannot accidentally violate ADR 0014 or ADR 0020.

## 1. Clarify Runtime Ownership

Update ADR 0021 to make mpv playback owned by a long-running process rather
than one-shot CLI commands.

Required decisions:

- `MpvDriver` is owned by the desktop/TUI app or a future playback daemon.
- One-shot CLI commands continue to operate on `PlaybackSession` only unless
  they explicitly talk to a long-running playback owner.
- `playlist play`, `playback next`, `playback previous`, `playback stop`, and
  `playback position` remain valid simulated/session commands when no live
  playback owner is running.
- ADR 0021 must not imply that a one-shot CLI can spawn, poll, reuse, and
  cleanly drop mpv across separate commands.

Acceptance criteria:

- ADR 0021 states who owns the driver singleton.
- ADR 0021 states what happens in CLI-only mode.
- Driver lifecycle language includes a process boundary.

## 2. Define Command Semantics

Split playback behavior into two explicit modes.

Session-only mode:

- Preserves ADR 0020 behavior.
- Commands update `playback_sessions` synchronously.
- JSON output reflects the database write immediately.
- No audio process is started or controlled.

Live-driver mode:

- Commands are routed through the long-running playback owner.
- The owner calls the driver and reconciles observed state back into
  `playback_sessions`.
- `playback position <ms>` sends `seek` to the driver and persists the
  accepted target position immediately, with later polls correcting drift.
- `playback next` and EOF both call the same playlist advancement service.
- `playback stop` stops the driver if live, then marks the session stopped.

Acceptance criteria:

- No command depends on a future poll tick that may not exist.
- CLI-only behavior remains deterministic.
- Live-driver behavior has one owner responsible for driver calls and
  reconciliation.

## 3. Add Pause State Contract

Decide whether pause is persisted as canonical playback state or remains
driver-local. The preferred incremental policy is to persist pause in the
session state without changing now-playing metadata identity.

Required ADR updates:

- `playback_sessions.state` allowed values are `playing`, `paused`, and
  `stopped`.
- `DriverStatus.paused = true` reconciles the session to `paused`.
- `DriverStatus.paused = false` reconciles the session to `playing`, unless
  EOF or stop handling has taken precedence.
- Stopped sessions still produce no `now-playing --json` result.
- Adding `paused` directly to `NowPlayingUpdate` is deferred unless a consumer
  needs it.

Acceptance criteria:

- Pause behavior is specified at the database/session boundary.
- `DriverStatus.paused` has a documented effect.
- Metadata identity remains owned by `PlaybackSession` and local database
  facts, not by mpv.

## 4. Specify The mpv IPC Contract

Add an IPC contract section to ADR 0021.

Required rules:

- Every mpv command uses a monotonically increasing `request_id`.
- Socket writes are serialized behind a mutex or single actor.
- A reader loop drains all mpv messages and routes replies/events by type.
- Command calls wait for the matching `request_id` response with a bounded
  timeout.
- Async property events update cached driver status.
- `poll()` returns cached status rather than opportunistically consuming
  arbitrary socket messages.
- EOF is detected from mpv events.
- EOF is edge-triggered so one EOF observation causes at most one playlist
  advance.
- Position units are normalized to milliseconds at the driver boundary.
- Property names are explicitly listed, including at least `time-pos`,
  `pause`, and EOF-relevant events/properties.

Acceptance criteria:

- Replies cannot be confused with async events.
- Duplicate EOF events cannot advance multiple tracks.
- Timeout and disconnected-socket behavior is specified.

## 5. Define Socket And Process Hygiene

Add a security and cleanup subsection to ADR 0021.

Required rules:

- The IPC socket lives under a private per-user runtime or temp directory.
- Socket names include a process id or random suffix.
- Directory permissions are owner-only where the platform allows it.
- Stale socket paths are removed before startup only after checking that they
  are not active.
- Startup waits for socket readiness with a bounded timeout.
- Shutdown first attempts graceful mpv termination, then kills the child after
  timeout.
- `Drop` performs best-effort cleanup, but normal app shutdown should use an
  explicit shutdown path.

Acceptance criteria:

- Local IPC exposure is limited to the current user.
- App exit does not intentionally leave orphaned mpv processes.
- Stale socket handling is deterministic.

## 6. Define Config Schema

Add a concrete playback config shape to ADR 0021.

Suggested TOML:

```toml
[playback]
driver = "null" # "null" or "mpv"
mpv_path = "mpv" # optional; defaults to PATH
```

Required behavior:

- Missing `[playback]` defaults to `driver = "null"`.
- Unknown drivers are config errors.
- `mpv_path` is optional.
- mpv availability is checked lazily when live playback starts, not while
  reading config.
- The generated default config includes commented playback settings.

Acceptance criteria:

- `Config` can represent playback settings without breaking existing configs.
- Default config preserves ADR 0020 simulated behavior.
- Invalid driver names fail clearly.

## 7. Testing Plan

Add focused tests with implementation.

Required tests:

- Config parsing defaults and invalid driver names.
- `NullDriver` preserving ADR 0020 simulated behavior.
- Fake mpv IPC peer tests for request IDs, response routing, timeout handling,
  event draining, pause, seek, stop, and EOF edge-triggering.
- Playback reconciliation tests for position and paused state.
- CLI tests confirming session-only commands still update DB synchronously.
- Optional ignored/manual test for real mpv availability and basic playback.

Acceptance criteria:

- Tests do not require mpv for normal CI.
- Driver protocol behavior is testable without a real audio backend.
- CLI regressions are covered.

## 8. Implementation Order

Recommended sequence:

1. Revise ADR 0021 with runtime ownership, command semantics, pause policy,
   IPC contract, socket hygiene, and config schema.
2. Update `docs/runbooks/workflows.md` to distinguish session-only playback from
   live-driver playback.
3. Add playback config structs and defaults.
4. Add the playback owner/reconciliation abstraction without mpv.
5. Add fake-driver reconciliation tests.
6. Implement the mpv IPC actor/driver.
7. Wire the driver only into the long-running app path.
8. Keep one-shot CLI commands session-only unless a daemon/RPC ADR exists.

## 9. Green Criteria

The remediation is complete when:

- ADR 0021 is internally consistent with ADR 0014 and ADR 0020.
- One-shot CLI behavior remains deterministic.
- There is a clear owner for mpv lifetime and polling.
- Pause behavior is documented and tested.
- IPC uses request IDs, timeouts, and explicit event handling.
- Socket path and process cleanup rules are specified.
- `cargo check` is green.
- `cargo fmt -- --check` is green.
