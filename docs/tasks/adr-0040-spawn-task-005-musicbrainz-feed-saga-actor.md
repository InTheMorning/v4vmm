# ADR 0040 Spawn Task 005 — MusicBrainz Feed Saga Actor

Status: Proposed - 2026-05-18.

## Goal

Retire the multi-step MusicBrainz album-then-per-track lookup saga at
`src/library/app_impl.rs:2279`. Replace the inline `cx.spawn(...)` block
with a saga actor in `src/runtime/` that:

- Receives a `StartFeedLookup { feed_id, downloadable_tracks, ... }`
  message.
- Internally drives the album-level release search, then loops over
  each track (matching candidates → staging or per-track fallback),
  publishing progress snapshots after every stage transition.
- Falls back to the per-track recording search if the album-level
  search fails or returns no candidates.

The screen (`LibraryApp`) subscribes to the actor's `watch::Receiver<
SagaState>` and reduces the snapshots into its existing
`musicbrainz_album_*` VM methods.

Plan reference: `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.
Prerequisites: Tasks 001-004 landed. The library spawn baseline shows
only this one site remaining.

## Files To Inspect

Required:

- `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
  (Risk Areas — saga refactor).
- `docs/adr/0040-async-vm-runtime.md` (Decision section, Actor rules).
- `src/runtime/actor.rs` (full — note `Actor` trait is sync; a saga
  needs a custom task shape).
- `src/runtime/paged_list_vm.rs` — example of a runtime actor in use.
- `src/runtime/vm_bus.rs` — `VmEvent` variants; this task may need to
  add one (`MusicBrainzFeedSagaProgress` or similar) OR use a dedicated
  `watch` channel without VmBus.
- `src/library/app_impl.rs:2279-2431` — the existing saga in full.
- `src/library/app_impl.rs` VM methods called from the saga:
  `vm.begin_musicbrainz_album_track_stage`,
  `vm.finish_musicbrainz_album_track_stage`,
  `vm.fail_musicbrainz_album_lookup_with_fallback`,
  `vm.fallback_empty_musicbrainz_album_lookup`,
  `vm.finish_musicbrainz_album_lookup`,
  `stage_musicbrainz_lookup_for_track`.
- `src/library/app_impl.rs:2435-2540` — the `lookup_musicbrainz_stage_for_track`,
  `match_candidate_to_track`, `stage_candidate_for_track` helpers
  (currently `#[allow(dead_code)]` — they're called only from the
  saga; if so, this task touches that dead-code allow).
- `src/application/commands/metadata.rs` — `LookupMusicBrainzAlbumReleases`
  (already used by the saga), `StageMusicBrainzTrack`.
- `tests/architecture_tests.rs` — baseline `("src/library/app_impl.rs", 1)`
  expected before this task; should become 0 after.

## Design

The saga is genuinely async (multiple sequential awaits with branching).
The existing `Actor` trait's `fn handle` is synchronous and not the right
fit. Two viable shapes:

**Shape A — Custom saga task with `Actor`-like ergonomics.**

Define a `MusicBrainzFeedSaga` type that mirrors `ActorHandle` (`mpsc`
inbox + `watch::Receiver<SagaState>` snapshot) but drives the saga in
its own `tokio::spawn` loop. The loop awaits a `StartFeedLookup`
message, runs the saga body (which is async, with internal awaits),
and publishes intermediate `SagaState` snapshots after each step.

State variants:

```rust
pub enum MusicBrainzFeedSagaState {
    Idle,
    AlbumSearchInFlight { feed_id: i64 },
    AlbumSearchFailed { feed_id: i64, error: String },
    AlbumSearchEmpty { feed_id: i64 },
    PerTrackInFlight { feed_id: i64, progress: usize, total: usize, track_id: i64 },
    TrackDone { feed_id: i64, track_id: i64, edit_count: usize },
    TrackSkipped { feed_id: i64, track_id: i64, reason: String },
    Completed { feed_id: i64, total_edits: usize, processed: usize },
}
```

The screen holds the `watch::Receiver` and reduces it into the VM via
the existing `vm.begin_*`, `vm.finish_*`, `vm.fallback_*` methods.

**Shape B — Decompose into chained `present_command` calls.**

Replace the saga with a state machine driven from `LibraryApp`. Each
step is a `present_command` whose on-success closure dispatches the
next. The screen keeps orchestration state (which track we're on,
running edit count, etc.).

Pros: zero new runtime concepts; minimal code churn.
Cons: orchestration lives in the screen, which is exactly what ADR 0040
says should NOT happen for multi-step domain flows. The same screen-level
state machine is more fragile and harder to test in isolation.

**Recommendation:** Shape A. Sagas belong in runtime actors per ADR
0040's Decision section ("Every long-lived concern ... runs as a tokio
task with an mpsc::Sender<Cmd> inbox"). The implementer should confirm
Shape A and only fall back to Shape B if the actor scaffolding lands
on a problem that can't be resolved in-scope (e.g., the saga needs to
re-enter GPUI state mid-flow, which it shouldn't).

## Files Likely To Change

- `src/runtime/musicbrainz_feed_saga.rs` — NEW. Saga actor type +
  `spawn()` helper + `SagaState` enum.
- `src/runtime/mod.rs` — register the new module.
- `src/library/app_impl.rs`:
  - Composition root: store the saga's `ActorHandle`-style handle
    (or, if reusing `ActorHandle<M, S>`, just store that).
  - The site at `:2279`: replace the inline saga with a single
    `saga.send(StartFeedLookup { ... })`. Add a separate code path
    that subscribes to the saga's snapshot stream and applies it
    to the VM through the existing methods. (Subscription can be a
    `cx.observe(...)` over a `watch::Receiver` adapter — there's
    likely an existing helper; look in
    `src/presentation/` or `src/view_models/` for `watch` → GPUI
    bridging patterns.)
- `src/library/app_impl.rs:2435-2540` — if the helpers
  (`lookup_musicbrainz_stage_for_track`, `match_candidate_to_track`,
  `stage_candidate_for_track`) move into the saga module, remove from
  the file. Otherwise leave with `pub(crate)` visibility so the saga
  module can call them.
- `tests/architecture_tests.rs` — baseline:
  remove `src/library/app_impl.rs` entirely from the baseline (it's
  zero after this task).
- New unit tests under `src/runtime/musicbrainz_feed_saga.rs` (in-file
  `#[cfg(test)] mod tests`) covering: empty album → fallback, failed
  album search → fallback, happy path → completion, per-track failure
  → skipped status.

## Do Not Touch

- `src/app/`, `src/discover/`, `src/presentation/`.
- Existing `Actor` trait — do not modify (the saga is a peer pattern,
  not a re-implementation).
- `LookupMusicBrainzAlbumReleases`, `StageMusicBrainzTrack` — reuse.
- `CommandBus` / `AsyncCommandRunner`.

## Constraints

- The saga runs domain logic. No GPUI imports inside
  `src/runtime/musicbrainz_feed_saga.rs`. Snapshots are plain data.
- Each saga state transition publishes a snapshot. The screen receives
  the new state via `watch::Receiver::changed()` (or a GPUI bridge)
  and applies it to the VM in `LibraryApp::handle_saga_snapshot`
  (new method).
- The screen-side subscription must NOT use `cx.spawn`. If a `watch`
  → GPUI bridge doesn't already exist, this task creates one in
  `src/presentation/` (the only layer allowed to import `tokio` +
  `gpui`).
- The saga reuses existing helpers (`match_candidate_to_track`,
  `stage_candidate_for_track`, `lookup_musicbrainz_stage_for_track`)
  to preserve behavior. If those helpers move into the saga module,
  delete the `#[allow(dead_code)]` they carry; they're now reachable.
- The saga uses existing commands (`LookupMusicBrainzAlbumReleases`,
  `StageMusicBrainzTrack`) for the calls that go through `CommandBus`.
  `execute` runs on the saga's tokio task.
- No new `#[allow(...)]` directives.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read this task, the parent plan, ADR 0040 Decision section, the
   existing `Actor` shape, and the inline saga at `:2279` through
   `:2431`.
2. Confirm Shape A with the user, or proceed and document the choice
   in the final report.
3. Create `src/runtime/musicbrainz_feed_saga.rs`:
   - Define `SagaState` enum (variants per *Design* above).
   - Define `StartFeedLookup` message struct (carries `feed_id`,
     `feed_title`, `downloadable_tracks: Vec<LibraryDownloadable>`,
     and the `Arc<Mutex<Connection>>`).
   - `pub fn spawn(bus: VmBus) -> SagaHandle` returns a handle with
     `inbox: mpsc::Sender<StartFeedLookup>` and `snapshot:
     watch::Receiver<SagaState>`.
   - Inside the spawned loop, on each `StartFeedLookup`: run the
     existing saga body, publishing a snapshot after every UI-visible
     stage transition. Use the existing `LookupMusicBrainzAlbumReleases`
     command (`CommandBus::execute`) for the album search and
     `StageMusicBrainzTrack` for staging.
4. Register the new module in `src/runtime/mod.rs`.
5. Add a `watch::Receiver` → GPUI bridge in `src/presentation/` if
   one doesn't already exist. Name it
   `present_watch_snapshot<T, S, OnChange>` mirroring the existing
   `present_command` style. It subscribes the receiver to the GPUI
   entity via a `cx.spawn` inside the bridge (allowed; presentation
   layer is exempt).
6. Update `src/library/app_impl.rs`:
   - Composition root: spawn the saga at app boot (or lazily on first
     library entry); store the handle.
   - At `:2279`: replace the spawn with
     `self.saga.send(StartFeedLookup { ... })`.
   - Subscribe to the saga's snapshot via the new presentation
     bridge; in the on-change closure, dispatch to a new
     `LibraryApp::apply_musicbrainz_saga_snapshot(state, cx)` method
     that translates each `SagaState` into the appropriate VM call.
7. Move helper fns (`lookup_musicbrainz_stage_for_track`,
   `match_candidate_to_track`, `stage_candidate_for_track`) into the
   saga module if they have no other callers. Remove the
   `#[allow(dead_code)]` since they become live in the saga module.
8. Add unit tests for `MusicBrainzFeedSaga` covering empty / failed /
   happy / partial-skip paths. Use a `MockCommandBus` if needed.
9. Update the architecture-test baseline: remove
   `("src/library/app_impl.rs", ...)` from the baseline (count = 0).
10. Run all five gates.
11. Smoke: trigger a feed-level MusicBrainz lookup on a multi-track
    album in the library. Each per-track stage should show progress
    identically; final completion banner should match the pre-task
    UI.

## Acceptance Criteria

- `grep -n "cx\.spawn(" src/library/app_impl.rs` returns no hits.
- `src/runtime/musicbrainz_feed_saga.rs` exists with the saga actor.
- No GPUI imports inside `src/runtime/musicbrainz_feed_saga.rs`.
- The presentation `watch` bridge exists (or the existing one is
  reused).
- The architecture baseline drops `src/library/app_impl.rs`.
- All five gates pass.
- New unit tests for the saga pass.
- No new `#[allow(...)]`.
- No behavior change in the user-visible MusicBrainz album lookup
  flow.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one engineering task — fifth of seven in the
screen-local `cx.spawn` retirement plan.

Prerequisites: Tasks 001-004 landed. The only `cx.spawn` site in
`src/library/app_impl.rs` is line 2279 (the MusicBrainz album saga).

Read in order:

1. This task file in full.
2. `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
   (Risk Areas — saga).
3. `docs/adr/0040-async-vm-runtime.md` (Decision — actor rules).
4. `src/runtime/actor.rs` and `src/runtime/vm_bus.rs`.
5. `src/library/app_impl.rs:2279-2540` (saga + helpers).

Goal:

Build a saga actor at `src/runtime/musicbrainz_feed_saga.rs` that runs
the existing album-then-per-track MusicBrainz flow as a tokio task
with `mpsc` inbox + `watch::Sender<SagaState>` snapshot output. Add a
`watch` → GPUI presentation bridge if missing. Replace the inline
spawn at `src/library/app_impl.rs:2279` with `saga.send(...)` + a
snapshot-subscription that drives the existing VM stage methods
through a new `apply_musicbrainz_saga_snapshot` reducer. Move helpers
into the saga module if their callers go with the saga.

Remove `src/library/app_impl.rs` from the
`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
baseline.

Constraints:

- No GPUI imports inside `src/runtime/`.
- No new `#[allow(...)]`.
- No behavior change at the user-visible level.
- Don't touch `src/app/`, `src/discover/`, or other unrelated areas.
- Reuse existing commands (`LookupMusicBrainzAlbumReleases`,
  `StageMusicBrainzTrack`).
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. Chosen shape (A or B) and rationale.
2. Saga module path + state enum variants.
3. Whether a new `watch`→GPUI presentation bridge was added (and its
   signature) or an existing one was reused.
4. Helpers relocated.
5. Removed `#[allow(dead_code)]` directives.
6. Library composition-root changes.
7. Baseline diff.
8. Five-gate results.
9. Deviations + unresolved concerns.

## Escalation Triggers

- The existing presentation layer has no `watch::Receiver` → GPUI
  bridge AND adding one ripples into shells / view-models. Report;
  this task is allowed to add the bridge but it should be a small
  presentation-only file.
- The saga needs information that's currently captured only inside
  `LibraryApp` state (e.g., per-stage UI thresholds). The right path
  is to surface that information at the `StartFeedLookup` boundary,
  not to give the actor a `LibraryApp` reference.
- `LookupMusicBrainzAlbumReleases` or `StageMusicBrainzTrack`'s
  `execute` requires inputs the saga can't easily provide. Report the
  shape; the right move is usually copying small `Vec`s or `Arc`s into
  the saga module at boundary.
- A behavior diff appears at smoke (e.g., progress UI shows different
  text). Report the diff; do not paper over with a guard.
