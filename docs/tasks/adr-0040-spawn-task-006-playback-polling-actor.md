# ADR 0040 Spawn Task 006 — Playback Polling Actor

Status: Proposed - 2026-05-18.

## Goal

Retire the 1Hz playback driver polling loop at `src/app.rs:321` by
moving it into a runtime actor that polls the `PlaybackOwner`,
publishes snapshots via `watch::Sender<PlaybackTickSnapshot>`, and
emits `VmEvent::PlaybackTick` (or equivalent) when state advances.

The screen subscribes to the actor's snapshot stream and updates
`settings_status` / triggers `cx.notify()` from the on-change path
through the presentation `watch` bridge (the same bridge introduced
by Task 005, or pre-existing).

Plan reference: `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.
Prerequisites: Tasks 001-005 landed. The only remaining
non-presentation/runtime `cx.spawn` outside `bootstrap.rs` is
`src/app.rs:321`.

## Files To Inspect

Required:

- `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md` (Risk
  Areas — playback polling coupling).
- `docs/adr/0040-async-vm-runtime.md` (Decision — actor rules).
- `src/runtime/actor.rs` (Actor trait — sync `handle`; this task can
  use a custom task shape similar to Task 005 if `Actor` doesn't fit
  the polling cadence).
- `src/runtime/vm_bus.rs` — `VmEvent` enum; may add a
  `PlaybackTick(...)` variant.
- `src/app.rs:300-340` (`maybe_start_playback_polling` site).
- `src/app.rs:508-520` (`poll_playback_owner` body).
- `src/playback_owner.rs` (`PlaybackOwner`, `PollOutcome`, `poll`
  signature — what arguments and locking it requires).
- The presentation `watch` bridge from Task 005 (if added) or
  `src/presentation/` for an existing bridge.
- `tests/architecture_tests.rs` — `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
  baseline; this task removes `src/app.rs` from it.

## Design

The current loop:

```rust
cx.spawn(async move |this: WeakEntity<TopApp>, cx: &mut AsyncApp| loop {
    cx.background_executor().timer(Duration::from_secs(1)).await;
    if this.update(cx, |this, cx| {
        this.poll_playback_owner();
        cx.notify();
    }).is_err() { break; }
});
```

`poll_playback_owner` locks the DB conn + `PlaybackOwner`, calls
`PlaybackOwner::poll`, and either clears `settings_status` or sets it
to an error. It is the only caller.

The actor needs to:

1. Hold (or reach) the `Arc<Mutex<PlaybackOwner>>` and
   `Arc<Mutex<Connection>>`. Both are already `Arc<Mutex<_>>` and Send,
   so the actor can clone them at boot.
2. On a 1Hz tokio interval, call the equivalent of
   `poll_playback_owner` and publish a `PlaybackTickSnapshot` describing
   what happened (idle / advanced / reconciled / error).
3. The screen reduces the snapshot into `settings_status` and notifies
   the entity.

State variants:

```rust
pub struct PlaybackTickSnapshot {
    pub at: Instant,  // for ordering; screen can ignore
    pub outcome: PlaybackTickOutcome,
}
pub enum PlaybackTickOutcome {
    Idle,            // NoSession | Reconciled(None)
    Advanced,        // Reconciled(Some(_)) | Advanced(_) — clear status
    Error(String),   // poll error — set status
}
```

The actor is **always running** once the app boots a live playback
driver. It stops itself when `is_live_driver()` returns false (or stays
quiescent — the existing `maybe_start_playback_polling` only starts if
live; matching behavior is fine).

VmBus integration: a `VmEvent::PlaybackTick` variant lets other actors
observe playback advances if they want. Optional — if no other actor
needs it, the actor publishes only via the snapshot channel and skips
VmBus. Add the VmEvent variant only if needed; otherwise leave VmBus
alone.

## Files Likely To Change

- `src/runtime/playback_polling.rs` — NEW. Actor type + spawn helper
  + `PlaybackTickSnapshot` + `PlaybackTickOutcome` types.
- `src/runtime/mod.rs` — register the module.
- `src/runtime/vm_bus.rs` — if a `PlaybackTick` `VmEvent` is needed
  (probably not), add it. Otherwise unchanged.
- `src/app.rs`:
  - `maybe_start_playback_polling` becomes
    `maybe_start_playback_polling_actor`: it checks
    `playback_owner.driver().is_live_driver()` and spawns the actor
    (storing the snapshot receiver), then subscribes the receiver to
    the entity through the presentation `watch` bridge.
  - The on-change reducer is a new method
    `apply_playback_tick(&mut self, outcome: PlaybackTickOutcome,
    cx: &mut Context<Self>)` that mirrors the current
    `poll_playback_owner` post-`poll` branches.
  - `poll_playback_owner` can move into the actor module (or stay if
    `LibraryApp` reuses it; check first).
- `tests/architecture_tests.rs` — baseline: remove `src/app.rs`
  entirely from the `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
  baseline.

## Do Not Touch

- `src/app/bootstrap.rs` — Task 007 (window lifecycle exemption).
- `src/library/`, `src/discover/`, `src/presentation/`
  (except adding the `watch` bridge if Task 005 didn't).
- `PlaybackOwner` API — the actor calls existing methods.
- `CommandBus` / `AsyncCommandRunner`.

## Constraints

- No GPUI imports inside `src/runtime/playback_polling.rs`. The
  actor returns plain `PlaybackTickSnapshot` values.
- The actor uses `tokio::time::interval(Duration::from_secs(1))` (or
  `tokio::time::sleep` in a loop) for the 1Hz cadence. Do not use
  `cx.background_executor().timer` — that's GPUI's executor and is
  not what the runtime uses.
- The snapshot subscription path on the screen MUST NOT use
  `cx.spawn`. It goes through the presentation `watch` bridge.
- Behavior preservation: the screen only acts on `Advanced` (clears
  status) and `Error` (sets status). `Idle` is no-op. Match the
  existing branch shape exactly.
- The actor must shut down cleanly when the app exits (drop the
  `mpsc::Sender` will cause `inbox_rx.recv()` to return `None` — same
  shape as `Actor` trait). If the actor doesn't take inbox messages
  (it's a pure poller), provide a shutdown signal via a `oneshot`
  channel or drop of the snapshot sender.
- No new `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read this task, the parent plan, Task 005's actor shape, and the
   current `maybe_start_playback_polling` + `poll_playback_owner` body.
2. Create `src/runtime/playback_polling.rs`:
   - Define `PlaybackTickSnapshot` + `PlaybackTickOutcome` types.
   - `pub fn spawn(playback_owner: Arc<Mutex<PlaybackOwner<...>>>,
     conn: Arc<Mutex<Connection>>) -> PlaybackPollingHandle` that
     spawns a tokio task and returns `{ snapshot: watch::Receiver,
     shutdown: oneshot::Sender<()> }`.
   - Inside the spawned task: `tokio::select! { biased; _ = interval.tick() => poll();
     _ = shutdown_rx => break; }`. After each poll, send a snapshot
     describing the outcome.
3. Register the module in `src/runtime/mod.rs`.
4. Update `src/app.rs`:
   - Replace `cx.spawn(... loop { timer; poll })` at `:321` with
     `let handle = playback_polling::spawn(...)`; store the snapshot
     receiver in the entity.
   - Subscribe the snapshot through the presentation `watch` bridge
     to an on-change closure that calls
     `self.apply_playback_tick(outcome, cx)`.
   - Define `apply_playback_tick` that branches on the outcome and
     mirrors the existing post-poll behavior.
   - Drop or move `poll_playback_owner` into the actor module
     (preferred — the actor owns it).
5. Verify no other caller relies on `poll_playback_owner` being on
   `TopApp`. Grep first.
6. Update the architecture-test baseline: remove `src/app.rs` from
   the baseline.
7. Run all five gates.
8. Smoke: start playback (via the library or recent feeds), confirm
   the now-playing state advances every second and that the
   `settings_status` clears as before.

## Acceptance Criteria

- `grep -n "cx\.spawn(" src/app.rs` returns no hits.
- `src/runtime/playback_polling.rs` exists with the actor + types.
- No GPUI imports in `src/runtime/playback_polling.rs`.
- The architecture baseline drops `src/app.rs`.
- All five gates pass.
- Playback advancement and status clearing behave identically.
- No new `#[allow(...)]`.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one engineering task — sixth of seven in the
screen-local `cx.spawn` retirement plan.

Prerequisites: Tasks 001-005 landed. The only `cx.spawn` site in
`src/app.rs` is line 321 (1Hz playback driver polling). The
presentation `watch` → GPUI bridge from Task 005 exists; reuse it
here.

Read in order:

1. This task file in full.
2. `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
   (Risk Areas — polling coupling).
3. `src/runtime/actor.rs` (for pattern reference).
4. `src/app.rs:300-340` and `:508-520`.
5. `src/playback_owner.rs` (`PlaybackOwner::poll` signature).
6. The Task 005 `watch` bridge in `src/presentation/`.

Goal:

Build a polling actor at `src/runtime/playback_polling.rs` that
ticks once per second, calls the equivalent of `poll_playback_owner`,
and publishes a `PlaybackTickSnapshot` via `watch::Sender`. Replace
`src/app.rs:321` with a `playback_polling::spawn(...)` call and a
snapshot subscription through the presentation `watch` bridge. The
screen reducer (`apply_playback_tick`) updates `settings_status` per
the existing branches.

Remove `src/app.rs` from the
`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
baseline.

Constraints:

- No GPUI imports in `src/runtime/playback_polling.rs`.
- Use `tokio::time::interval` for cadence; not GPUI's
  `background_executor().timer`.
- No new `#[allow(...)]`.
- No behavior change.
- Clean shutdown via `oneshot` or sender drop.
- Don't touch `src/app/bootstrap.rs` (Task 007).
- Don't touch other modules.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. New module path + types.
2. Composition-root changes in `src/app.rs` (where the actor is
   spawned, where the snapshot receiver is stored).
3. `apply_playback_tick` body.
4. Whether `VmEvent::PlaybackTick` was added (and why or why not).
5. Baseline diff.
6. Five-gate results.
7. Deviations + unresolved concerns.

## Escalation Triggers

- `PlaybackOwner::poll` requires a borrow that can't cross the actor
  boundary (e.g., a non-Send pointer). Report; the right path is
  usually narrowing what the poll returns to plain data.
- The 1Hz cadence interacts with another timer the task didn't
  anticipate (e.g., a position-tick UI animation). Report; the actor
  cadence stays 1Hz unless the user approves otherwise.
- The presentation `watch` bridge from Task 005 doesn't exist
  (Task 005 chose Shape B, or didn't add it). Report; this task may
  add a bridge if needed.
- A behavior diff appears at smoke (e.g., the status takes longer to
  clear than before). Report; do not increase cadence to mask the
  issue.
