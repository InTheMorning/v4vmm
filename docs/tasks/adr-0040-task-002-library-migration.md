# ADR 0040 Task 002 — `src/library/app_impl.rs` Migration

Status: Completed - 2026-05-18. The current tree had 17 library
command-runner call sites rather than the 18 counted when this packet
was drafted; all 17 were migrated.

## Goal

Migrate the eighteen `GpuiCommandRunner::run(...)` call sites in
`src/library/app_impl.rs` to the presentation bridge introduced by
Task 001. Bulk of the migration work; mechanical once the bridge is
proven.

Plan reference: `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md`.
Prerequisite: Task 001 landed. The bridge exists at
`src/presentation/async_command_presenter.rs` and is consumed in
`src/app/`.

## Files To Inspect

Required:

- `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md` (skim
  Decision + Invariants).
- `src/presentation/async_command_presenter.rs` (the bridge — full
  signature + doc comment).
- `src/app.rs:191` AND one migrated call site under `src/app/` (any of
  lines 792 in `app.rs`, 168 in `playback_bar.rs`, or 747/817/917/938/976
  in `search_dispatch.rs`) — model the diff shape from these.
- `src/library/app_impl.rs:371` — composition root (constructs
  `GpuiCommandRunner` today).
- `src/library/app_impl.rs` call sites: lines **779, 798, 849, 870,
  895, 926, 972, 999, 1309, 1363, 1387, 1622, 1702, 1759, 1963, 2151,
  2231**. Eighteen sites total.

Reference only:

- `src/discover/app_impl.rs` call sites — Task 003 owns this.

## Files Likely To Change

- `src/library/app_impl.rs` — composition root field type change +
  eighteen call-site updates.

Probable but verify:

- `src/library/mod.rs` or callers of `LibraryApp::new` if the
  composition root's constructor signature changes (it should not — the
  field type changes from `GpuiCommandRunner` to `AsyncCommandRunner`
  but the `new` argument list stays the same since both runners take
  `(command_bus, event_bus)`).

## Do Not Touch

- `src/presentation/async_command_presenter.rs` — owned by Task 001.
- `src/app/`, `src/discover/` — owned by Task 001 and Task 003.
- The `async-runtime` feature flag — Task 004.
- `CommandBus` — retained.
- `GpuiCommandRunner` type definition — Task 004 deletes it.

## Constraints

- One-to-one signature replacement at call sites. Match the diff shape
  Task 001 established.
- No behavior change. Every callback that previously fired on the
  GPUI thread continues to do so through the bridge.
- The composition root field type changes from `GpuiCommandRunner` to
  `AsyncCommandRunner`. Any field accessor must continue to compile.
- No `cx.spawn` calls added to library code. All async glue lives
  inside the presenter.
- No new `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read this task, the parent plan summary, and the Task 001 bridge
   plus one migrated call site to confirm the diff pattern.
2. Update `src/library/app_impl.rs:371` (composition root): change the
   `GpuiCommandRunner::new(...)` construction to
   `AsyncCommandRunner::with_vm_bus(...)` (or `AsyncCommandRunner::new`
   if VmBus is not yet plumbed to LibraryApp; check what `src/app.rs`
   does and match). Update the field type accordingly.
3. Walk the 18 call sites in order. For each:
   - Confirm the existing `command_runner.run(...)` shape.
   - Substitute the bridge call (e.g., `present_command(&self.command_runner, ...)`
     or `self.command_runner.present(...)` — match Task 001's call
     pattern).
4. After each batch of ~6 sites, run `cargo build` to catch shape
   mismatches early.
5. Run all five gates at the end.
6. Smoke (if possible): launch the app, exercise a library action that
   maps to one of the migrated sites — e.g., remove a track (line 870
   typically handles this) and confirm UI reflects the change.

## Acceptance Criteria

- `grep -n "GpuiCommandRunner" src/library/app_impl.rs` returns no
  hits.
- `grep -n "command_runner.run(" src/library/app_impl.rs` returns no
  hits.
- All eighteen call sites use the presentation bridge.
- `cargo build` passes with default features.
- `cargo test --lib` passes.
- `cargo clippy -- -D warnings` passes.
- No new `#[allow(...)]` directives.
- Composition root field is now `AsyncCommandRunner`.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded migration task — second of four.

Prerequisite: Task 001 has landed. The presentation bridge exists at
`src/presentation/async_command_presenter.rs` and is in use across
`src/app/`.

Read in order:

1. This task file in full.
2. `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md` (Decision
   + Invariants only).
3. `src/presentation/async_command_presenter.rs` (bridge signature).
4. One migrated call site under `src/app/` (e.g., `src/app.rs:792`) to
   see the post-migration diff shape.
5. `src/library/app_impl.rs:371` (composition root) and the 18 call
   sites listed in this task.

Goal:

Migrate the eighteen `GpuiCommandRunner::run(...)` call sites in
`src/library/app_impl.rs` to the bridge. Change the composition-root
field from `GpuiCommandRunner` to `AsyncCommandRunner`. One-to-one
substitution; no behavior change; no `cx.spawn` outside the bridge.

Constraints:

- Match the diff shape established by Task 001.
- No behavior change.
- Don't touch `src/app/`, `src/discover/`, or the feature flag.
- No `#[allow(...)]`.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. Composition-root field type change (line + type before and after).
2. Eighteen migrated call sites (file:line before → after — terse).
3. Any closure that needed hand-tuning vs purely mechanical
   substitution.
4. Five-gate results.
5. Deviations + unresolved concerns.

## Escalation Triggers

- A call site's closure body references the runner itself (recursive
  pattern, e.g., chaining a follow-up dispatch from the success
  callback). Report the call site; check whether the bridge supports
  this naturally or needs adaptation.
- A call site passes a callback that mutates `LibraryApp` state in a
  way that doesn't survive the `weak.update` re-entry. Report; the
  bridge guarantees `update` re-entry but a missed borrow can still
  surface as a panic.
- Composition root cannot construct `AsyncCommandRunner` without an
  argument that `LibraryApp::new` doesn't currently receive (e.g., a
  `VmBus` if the bridge requires it). Report; propose threading from
  `src/app.rs` rather than stubbing.
- One of the eighteen sites is actually missing or has moved since the
  task was drafted. Confirm against the current file and report the
  delta; do not skip silently.
