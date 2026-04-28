# ADR 0021: mpv Playback Driver

## Status

Proposed

## Context

ADR 0014 declared `PlaybackSession` the authoritative now-playing state and
explicitly reserved space for player adapters that "report playback facts such
as position, pause state, and local file progress" without owning canonical
metadata. ADR 0020 then accepted a simulated transport so relay smoke tests
could exercise the session without an audio backend.

With Phase 2 playlist playback in place, the next step is a real audio driver
so the same session model can drive an actual player. mpv is a strong first
target: it plays every relevant container, it has a stable JSON IPC protocol,
and it ships on every host the project already runs on.

## Decision

Introduce a `PlaybackDriver` trait and an mpv-backed implementation for a
long-running playback owner. The owner is the desktop/TUI app, or a future
playback daemon, not a one-shot CLI command.

One-shot CLI commands continue to operate in session-only mode unless a later
ADR defines daemon/RPC control. In that mode `playlist play`, `playback next`,
`playback previous`, `playback stop`, and `playback position` preserve ADR 0020:
they update `playback_sessions` synchronously, return JSON from the database
write, and do not start or control an audio process.

Live-driver mode exists only inside the long-running owner. Commands routed to
that owner call the driver, then reconcile observed driver state back into
`playback_sessions`. `playback position <ms>` sends `seek` to the driver and
persists the accepted target position immediately; later polls may correct
drift. `playback next` and EOF both use the same playlist advancement service.
`playback stop` stops the driver when live, then marks the session stopped.

### Trait

```rust
pub trait PlaybackDriver: Send + Sync {
    fn load(&self, path: &Path, start_ms: u64) -> Result<()>;
    fn seek(&self, position_ms: u64) -> Result<()>;
    fn pause(&self, paused: bool) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn poll(&self) -> Result<DriverStatus>;
}

pub struct DriverStatus {
    pub position_ms: u64,
    pub paused: bool,
    pub eof: bool,
    pub error: Option<String>,
}
```

A `NullDriver` keeps the ADR 0020 simulated behavior and remains the default.
The `MpvDriver` is opt-in for the long-running owner only, and the existing CLI
surface gains no required arguments.

### Config

Playback config is represented as:

```toml
[playback]
driver = "null" # "null" or "mpv"
mpv_path = "mpv" # optional; defaults to PATH
```

Missing `[playback]` defaults to `driver = "null"`. Unknown drivers are config
errors. `mpv_path` is optional, and mpv availability is checked lazily when live
playback starts, not while reading config. Generated default config should
include commented playback settings.

### IPC over libmpv

The driver spawns mpv with `--idle --no-video --input-ipc-server=<socket>` and
sends `loadfile`, `seek`, `set pause`, `stop`, and `observe_property` commands
as newline-delimited JSON. This avoids a C build dependency, keeps the driver
debuggable from a shell with `socat`, and matches the CLI-first ethos of ADR
0017. libmpv FFI remains a future option if property polling latency becomes a
problem.

### IPC contract

- Every mpv command uses a monotonically increasing `request_id`.
- Socket writes are serialized through a mutex or single actor.
- A reader loop drains all mpv messages and routes replies and events by type.
- Command calls wait for the matching `request_id` response with a bounded
  timeout; timeout or socket disconnect returns an explicit driver error.
- Async property events update cached driver status, and `poll()` returns that
  cache rather than consuming arbitrary socket messages.
- Observed mpv properties include `time-pos` and `pause`; position is normalized
  to milliseconds at the driver boundary.
- EOF is detected from mpv events or EOF-relevant properties and is
  edge-triggered so one observation advances the playlist at most once.

### Lifecycle

- The driver singleton is owned by the long-running playback process and tied
  to the default session id.
- mpv spawns lazily on the first `load` call and is reused across tracks.
- One-shot CLI invocations do not create a reusable driver or imply mpv
  ownership across process boundaries.
- The driver owns the subprocess handle. Normal shutdown uses an explicit
  shutdown path; `Drop` performs best-effort cleanup.
- A health check (`player ping` debug command, ADR 0017 style) confirms the
  socket is reachable before the first load.

### Socket and process hygiene

- The IPC socket lives under a private per-user runtime or temp directory.
- Socket names include the process id or a random suffix.
- Directory permissions are owner-only where the platform allows it.
- Stale socket paths are removed before startup only after checking that they
  are not active.
- Startup waits for socket readiness with a bounded timeout.
- Shutdown first asks mpv to terminate, then kills the child after timeout.
- Cleanup removes the socket path on normal shutdown and best-effort `Drop`.

### Position reconciliation

- During live playback the driver is the source of truth for `position_ms`.
  A poll loop writes observed positions back into `playback_sessions` so
  `now-playing` JSON stays current.
- On app restart the session is the source of truth: the driver re-loads the
  stored track and seeks to `position_ms`.
- On EOF the driver reports `eof = true`; the playback layer calls the
  existing `playback next` path, so playlist advance logic stays in one place.
- Manual live `playback position <ms>` calls go through the driver's `seek` and
  persist the accepted target immediately; later polls correct drift.

### Pause state

`playback_sessions.state` supports `playing`, `paused`, and `stopped`.
`DriverStatus.paused = true` reconciles the session to `paused`.
`DriverStatus.paused = false` reconciles the session to `playing` unless EOF or
stop handling has taken precedence. Stopped sessions still produce no
`now-playing --json` result.

Pause is persisted as session state, but metadata identity remains owned by
`PlaybackSession` and local database facts. Adding `paused` directly to
`NowPlayingUpdate` is deferred until a consumer needs it.

### Stream URLs

v1 only loads local files resolved through `track_identity`. mpv handles HTTP
natively, but enclosure-URL playback waits for a follow-up ADR so download
gating, caching, and value-block accounting can be designed together.

## Consequences

- The session model from ADR 0014 stays unchanged; mpv is purely an executor.
- Relay tests can keep using `NullDriver` so ADR 0020 behavior is preserved.
- A new `playback_driver` module gives future drivers (libmpv, MPRIS, web
  player) a single seam to implement.
- mpv lifecycle is part of long-running owner shutdown, not one-shot CLI exit.
- Stream playback and gapless transitions are explicitly out of scope and
  tracked for a later ADR.
