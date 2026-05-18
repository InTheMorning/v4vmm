# ADR 0040 Legacy Scheduling Retirement Plan

## Status

Proposed - 2026-05-18.

## Goal

Intentionally retire the `--no-default-features` build path and the
synchronous `GpuiCommandRunner` (the GPUI-coupled command-runner wrapper).
Complete ADR 0040's deferred "legacy synchronous scheduling retirement"
deferred-work-index item #3 by moving the entire app onto the
default-on async runtime.

## Context

The `async-runtime` Cargo feature is in the default set and gates the
entire `src/runtime/` module (`RuntimeHost`, `AsyncCommandRunner`,
`VmBus`, `Actor`, `ActorHandle`, `PagedListVm`) as well as the
`src/view_models/search_results/` module family. The post-ADR-0048
Recent Feeds + search-result inspector work shipped without
re-gating its consumers behind the feature flag, so today:

- `cargo check --no-default-features` is **broken** with 6 unconditional
  imports of feature-gated modules (`src/app.rs:38`,
  `src/app/search_dispatch.rs:36`,
  `src/ui/shells/recent_feeds.rs:34`,
  `src/ui/shells/search_result_rows.rs:22`,
  `src/ui/shells/search_results_inspector.rs:16,35`,
  `src/view_models/recent_feeds.rs:10`).
- No `.github/workflows/` exists, so no CI verifies the
  `--no-default-features` path. The feature lives only as a Cargo-level
  scaffolding contract; nothing enforces it.
- `GpuiCommandRunner` has 32 unconditional call sites across five
  modules (`src/app.rs` ×1, `src/library/app_impl.rs` ×18,
  `src/discover/app_impl.rs` ×8, `src/app/playback_bar.rs` ×1,
  `src/app/search_dispatch.rs` ×5).
- `AsyncCommandRunner` exists at
  `src/application/async_command_runner.rs:37` with `dispatch()` plus
  `with_vm_bus()` plumbing, but has **zero production call sites** today.
- No shared `CommandRunner` trait: sync `.run(&mut Context<T>, ...
  on_success, on_error)` and async `.dispatch(...) -> oneshot::Receiver`
  have incompatible signatures and cannot share a trait without crossing
  ADR 0040's layer rules.

ADR 0040 invariants explicitly retain the synchronous `CommandBus` for
CLI and test use ("The synchronous `CommandBus` continues to exist for
CLI and tests where the runtime is not desired"). This plan retires
`GpuiCommandRunner` — the GPUI-coupled wrapper — not `CommandBus`.

## Decision

Migrate all 32 sync `GpuiCommandRunner::run(...)` call sites to
`AsyncCommandRunner::dispatch(...)` via a small new presentation-layer
bridge that preserves the existing on-success / on-error callback
ergonomics. Delete `GpuiCommandRunner` and its three composition sites.
Remove the `async-runtime` Cargo feature flag, strip every
`#[cfg(feature = "async-runtime")]` directive (37 hits across `src/`),
and make `tokio` / `tokio-util` unconditional dependencies. Update
ADR 0040 status and move the deferred-index entry to "Recently
Resolved".

The presentation bridge lives in `src/presentation/` (the only ADR 0040
layer permitted to import both `tokio` and `gpui`) and mirrors the
existing `GpuiCommandRunner::run` argument shape so the per-call-site
diff is a single token replacement.

## Invariants (additions to ADR 0040)

After Task 004 lands:

- `src/presentation/gpui_command_runner.rs` does not exist.
- No `#[cfg(feature = "async-runtime")]` directive appears anywhere in
  `src/`.
- The Cargo `[features]` table has no `async-runtime` entry, and
  `tokio` / `tokio-util` are unconditional dependencies (no
  `optional = true`).
- `src/presentation::present_command` (or equivalent — implementer may
  choose `AsyncCommandPresenter::present`) is the sole bridge between
  `AsyncCommandRunner` outputs and GPUI entity updates. Screens never
  call `cx.spawn` for domain work (ADR 0040 invariant preserved).
- `CommandBus` continues to exist for non-GPUI consumers
  (CLI / tests).
- The architecture-guard set gains three new guards that pin the retired
  surface: no `GpuiCommandRunner` references, no `async-runtime` cfg, no
  `cx.spawn` calls outside `src/presentation/` and `src/runtime/`.

## Non-Goals

- No reorganization of `ApplicationCommand` variants.
- No new actor types beyond those required to host previously-sync
  flows (and none are anticipated).
- No retirement of `CommandBus` (separate concern; explicitly retained
  per ADR 0040).
- No CI workflow creation (separate concern; absence is noted in the
  Context section but fixing it is out of scope).
- No `src/discover/` unparking. The discover module migrates in place
  while remaining parked per
  `docs/notes/2026-05-discover-module-parked.md`.
- No behavior change for any user-visible flow. Migration is mechanical
  and behavior-preserving.
- No ADR 0041 paged-list migrations beyond what is already shipped.

## Proposed Sequence

Four sequential bounded slices. Each slice ships green under the five
gates before the next starts.

1. **Task 001 — Presentation bridge + `src/app/` migration.** Build
   `crate::presentation::present_command` (mirroring the existing
   `GpuiCommandRunner::run` signature). Migrate the seven call sites in
   `src/app.rs` (×1), `src/app/playback_bar.rs` (×1), and
   `src/app/search_dispatch.rs` (×5). Proves the pattern on the smallest
   surface.
2. **Task 002 — `src/library/app_impl.rs` migration.** Eighteen call
   sites. Bulk of the work; mechanical once the bridge is proven.
3. **Task 003 — `src/discover/app_impl.rs` migration.** Eight call sites
   inside the parked Discover module. Migrate in place; do not unpark.
4. **Task 004 — Retire feature flag + delete `GpuiCommandRunner`.**
   Remove the `async-runtime` feature from `Cargo.toml`, strip every
   `#[cfg(feature = "async-runtime")]` directive, drop
   `optional = true` from `tokio` / `tokio-util`, delete the
   `GpuiCommandRunner` type and its three composition sites
   (`src/app.rs:191`, `src/discover/app_impl.rs:76`,
   `src/library/app_impl.rs:371`), add three regression guards, update
   ADR 0040 status to "Implemented including retirement", and move the
   deferred-index entry.

Tasks 001 → 004 must land in order. Each task file is a self-contained
packet sized for a single sonnet subagent run.

## Risk Areas

- **Behavior change risk.** Sync `.run(...)` invokes its on-success /
  on-error callback synchronously on the GPUI thread via the
  `background_executor`. Async `.dispatch(...)` returns a
  `oneshot::Receiver` resolved by the tokio runtime; the GPUI callback
  is then re-attached via `cx.spawn`. Subtle ordering differences are
  possible. Mitigation: the presentation bridge must execute the
  callback on the GPUI thread, not the tokio thread, matching the
  existing contract. The bridge is the place to centralize this.
- **VmBus invalidation overlap.** `AsyncCommandRunner::with_vm_bus()`
  publishes coarse `VmEvent` invalidations on every command outcome.
  Some screens listen to VmBus separately. Re-publishing invalidations
  via VmBus could double-fire callbacks. Mitigation: the bridge wires
  the callback off the `Receiver` only, not the bus.
- **Discover migration ratchets parked-module LOC.** Discover is
  compiled-but-unreachable. Migrating its 8 call sites adds to the
  module's complexity. Defensible because the parked-module note
  already commits to "compiled, not deleted"; the migration is required
  to delete `GpuiCommandRunner`.
- **Discovery of non-mechanical call sites.** A small fraction of the
  32 sites might pass closures that capture GPUI-specific state in a
  way that doesn't translate one-to-one. The bridge signature must be
  permissive enough to handle the existing closure shape; per-site
  escalation is allowed if a call site needs hand-tuning.
- **Architecture-guard introduction order.** The new guards in Task 004
  must not be added before Task 003 finishes — they will fail.

## Verification

End-to-end after Task 004 lands:

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

Plus:

- `grep -r "GpuiCommandRunner" src/` → no hits.
- `grep -r 'cfg(feature = "async-runtime")' src/` → no hits.
- `grep -r "async-runtime" Cargo.toml` → no hits.
- `grep -rn "cx.spawn" src/ | grep -v "src/presentation/\|src/runtime/"` → no hits.
- Smoke: launch the app, exercise search submit, Recent Feeds toolbar
  button, playlist track add/remove, playback play/pause, library
  download — each was a former GpuiCommandRunner call site; each
  should behave identically.

## References

- ADR 0040 — `docs/adr/0040-async-vm-runtime.md`
- ADR 0041 — `docs/adr/0041-paged-list-vm.md`
- `docs/plans/deferred-architecture-work-index.md` item #3
- `src/presentation/gpui_command_runner.rs:14`
- `src/application/async_command_runner.rs:37`
- `src/runtime/vm_bus.rs:34`
- `src/runtime/actor.rs:48`
- `Cargo.toml` `[features]`
