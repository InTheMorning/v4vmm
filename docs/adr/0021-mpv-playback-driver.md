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
so the same CLI and session model can drive an actual player. mpv is a strong
first target: it plays every relevant container, it has a stable JSON IPC
protocol, and it ships on every host the project already runs on.

## Decision

Introduce a `PlaybackDriver` trait and an mpv-backed implementation that talks
to a long-lived `mpv --idle` subprocess over a Unix-domain JSON IPC socket.

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

A `NullDriver` keeps the ADR 0020 simulated behavior and remains the default
when no audio backend is configured. The `MpvDriver` becomes opt-in via a
config flag (`playback.driver = "mpv"`) and the existing CLI surface gains
no required arguments.

### IPC over libmpv

The driver spawns mpv with `--idle --no-video --input-ipc-server=<socket>` and
sends `loadfile`, `seek`, `set pause`, `stop`, and `observe_property` commands
as newline-delimited JSON. This avoids a C build dependency, keeps the driver
debuggable from a shell with `socat`, and matches the CLI-first ethos of ADR
0017. libmpv FFI remains a future option if property polling latency becomes a
problem.

### Lifecycle

- The driver is a singleton tied to the default session id.
- mpv spawns lazily on the first `load` call and is reused across tracks.
- The driver owns the subprocess handle and kills mpv on `Drop` so app exit
  does not leave orphans.
- A health check (`player ping` debug command, ADR 0017 style) confirms the
  socket is reachable before the first load.

### Position reconciliation

- During live playback the driver is the source of truth for `position_ms`.
  A poll loop writes observed positions back into `playback_sessions` so
  `now-playing` JSON stays current.
- On app restart the session is the source of truth: the driver re-loads the
  stored track and seeks to `position_ms`.
- On EOF the driver reports `eof = true`; the playback layer calls the
  existing `playback next` path, so playlist advance logic stays in one place.
- Manual `playback position <ms>` CLI calls go through the driver's `seek`
  and the database update happens on the next poll tick.

### Stream URLs

v1 only loads local files resolved through `track_identity`. mpv handles HTTP
natively, but enclosure-URL playback waits for a follow-up ADR so download
gating, caching, and value-block accounting can be designed together.

## Consequences

- The session model from ADR 0014 stays unchanged; mpv is purely an executor.
- Relay tests can keep using `NullDriver` so ADR 0020 behavior is preserved.
- A new `playback_driver` module gives future drivers (libmpv, MPRIS, web
  player) a single seam to implement.
- mpv lifecycle is now part of app shutdown — the TUI must drop the driver
  before exit so the subprocess is reaped.
- Stream playback and gapless transitions are explicitly out of scope and
  tracked for a later ADR.
