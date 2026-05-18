# ADR 0040 Task 003 — `src/discover/app_impl.rs` Migration

Status: Completed - 2026-05-18.

## Goal

Migrate the eight `GpuiCommandRunner::run(...)` call sites inside the
parked Discover module (`src/discover/app_impl.rs`) to the presentation
bridge. The Discover module is documented as "compiled but unreachable"
in `docs/notes/2026-05-discover-module-parked.md`; this task migrates
its command-runner usage in place without unparking it.

Plan reference: `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md`.
Prerequisite: Tasks 001 and 002 landed. The presentation bridge exists
at `src/presentation/async_command_presenter.rs` and `src/app/` +
`src/library/app_impl.rs` already use it.

## Files To Inspect

Required:

- `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md` (Decision
  + Invariants only).
- `docs/notes/2026-05-discover-module-parked.md` (full — important
  context about why Discover survives).
- `src/presentation/async_command_presenter.rs` (bridge).
- `src/library/app_impl.rs:371` and one post-Task-002 call site as
  reference for the diff shape.
- `src/discover/app_impl.rs:76` — composition root.
- `src/discover/app_impl.rs` call sites: lines **1139, 1244, 1300,
  1502, 1630, 1654, 1678, 1757**. Eight sites total.

Reference only:

- `tests/architecture_tests.rs` — `discover_module_public_surface_is_pinned`
  guard (or similar). Confirm the migration does not require changing
  pinned public symbols.

## Files Likely To Change

- `src/discover/app_impl.rs` — composition-root field type +
  eight call-site updates.

Should NOT change:

- The Discover module's public surface (per the parked-module guard).
- Any file under `src/ui/shells/discover/`.
- The parked-module note itself (the migration is in-scope per the note's
  "compiled" status).

## Do Not Touch

- `src/presentation/`, `src/app/`, `src/library/` — owned by earlier
  tasks.
- `src/ui/shells/discover/**`.
- The `async-runtime` feature flag — Task 004.
- The parked-module note unless its call-graph snapshot needs a one-line
  update to reflect the migration (only if the note explicitly lists
  `GpuiCommandRunner`).

## Constraints

- One-to-one signature replacement at call sites (same diff shape as
  Task 002).
- No behavior change. Discover remains compiled and unreachable.
- The migration does not unpark Discover. No new render path is added;
  no symbol becomes publicly visible that wasn't already.
- If the discover composition root's `GpuiCommandRunner::new` call at
  line 76 takes arguments that `AsyncCommandRunner::new` /
  `with_vm_bus` doesn't, plumb the missing arg from wherever
  `DiscoverApp::new` is called — but only if there is still a live
  caller. The parked-module note says Discover has no render path
  from the composition root, so `DiscoverApp::new` may be dead code.
  Check before plumbing.
- No new `#[allow(...)]`.
- The architecture guard `discover_module_public_surface_is_pinned`
  must continue to pass. If the field type change tightens or loosens
  visibility in a way the guard catches, update the guard's fixture in
  the same task and document the delta.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read this task, the parked-module note, the parent plan summary, and
   the bridge + one Task 002 call site.
2. Check whether `DiscoverApp::new` (the function containing the
   composition root) has any live callers. `grep -n "DiscoverApp::new"
   src/`. If all callers are themselves in dead code, document that in
   the report — the migration is still required to remove
   `GpuiCommandRunner` references, but the composition site is dead.
3. Update `src/discover/app_impl.rs:76`: change `GpuiCommandRunner::new(...)`
   to the same `AsyncCommandRunner` shape Task 002 used. Update the
   field type.
4. Walk the eight call sites (1139, 1244, 1300, 1502, 1630, 1654, 1678,
   1757). For each, substitute the bridge call to match the Task 002
   diff pattern.
5. After every ~4 sites, `cargo build` to catch shape mismatches.
6. Confirm the `discover_module_public_surface_is_pinned` guard still
   passes. If it fails, inspect the fixture and update only if the
   surface change is intended (it shouldn't be — only the private
   field type changes).
7. Run the five gates.

## Acceptance Criteria

- `grep -n "GpuiCommandRunner" src/discover/app_impl.rs` returns no
  hits.
- `grep -n "command_runner.run(" src/discover/app_impl.rs` returns no
  hits.
- All eight call sites use the presentation bridge.
- `cargo build` passes with default features.
- `cargo test --lib` passes.
- `cargo test --test architecture_tests` passes (including
  discover-surface guard).
- `cargo clippy -- -D warnings` passes.
- Discover remains compiled-but-unreachable. No new public symbol; no
  new render path from the composition root.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded migration task — third of four.

Prerequisite: Tasks 001 and 002 have landed. The presentation bridge
exists at `src/presentation/async_command_presenter.rs` and is in use
across `src/app/` and `src/library/app_impl.rs`.

Important context: `src/discover/` is a **parked** module per
`docs/notes/2026-05-discover-module-parked.md`. It is compiled but not
reachable from the composition root. The migration is required to
remove `GpuiCommandRunner` references but must not unpark the module.

Read in order:

1. This task file.
2. `docs/notes/2026-05-discover-module-parked.md` (full).
3. `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md` (Decision
   + Invariants).
4. `src/presentation/async_command_presenter.rs` (bridge signature).
5. One migrated call site under `src/library/app_impl.rs` to see the
   diff shape.
6. `src/discover/app_impl.rs:76` and the eight call sites listed.

Goal:

Migrate the eight `src/discover/app_impl.rs` call sites and the
composition-root field type using the same diff shape as Task 002.
Discover stays compiled and unreachable; no new public symbols, no new
render path. The `discover_module_public_surface_is_pinned` guard must
still pass.

Constraints:

- One-to-one substitution matching Task 002's pattern.
- No behavior change.
- No unpark — public surface unchanged.
- No `#[allow(...)]`.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. Composition-root field change.
2. Eight migrated call sites (file:line, terse before/after notation).
3. Whether `DiscoverApp::new` has any live callers (check with grep
   and report finding — it may be all-dead-code).
4. Whether the `discover_module_public_surface_is_pinned` guard
   needed any update; if yes, what and why.
5. Five-gate results.
6. Deviations + unresolved concerns.

## Escalation Triggers

- `DiscoverApp::new` has zero live callers AND the discover-surface
  guard fails after the migration. Report; do not soften the guard
  to make it pass. The right move may be removing the dead
  composition site entirely, but that decision should go to the user.
- A call site uses a closure pattern not seen in Task 002 (for example,
  a callback that re-enters `DiscoverApp::start_search` or similar).
  Report the pattern; do not silently special-case.
- The composition root needs a `VmBus` argument that
  `DiscoverApp::new` does not receive. Check whether any live caller
  could supply it; if not, the discover composition root is dead and
  the field can be `AsyncCommandRunner::new` (no VmBus) without losing
  anything. Report the situation.
