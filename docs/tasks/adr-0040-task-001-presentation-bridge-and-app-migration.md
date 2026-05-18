# ADR 0040 Task 001 — Presentation Bridge + `src/app/` Migration

Status: Completed - 2026-05-18.

## Goal

Build the presentation-layer bridge that maps
`AsyncCommandRunner::dispatch` outputs onto GPUI entity callbacks, then
migrate the seven `GpuiCommandRunner::run(...)` call sites under
`src/app/` to use it. Proves the migration pattern on the smallest
module before tackling library and discover in later tasks.

Plan reference: `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md`.

## Files To Inspect

Required:

- `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md` (parent
  plan; *Decision*, *Invariants*, *Risk Areas*).
- `docs/adr/0040-async-vm-runtime.md` (governing ADR — invariants,
  especially the `src/presentation/` rule).
- `src/presentation/gpui_command_runner.rs` — full file. The new
  bridge mirrors `GpuiCommandRunner::run`'s signature.
- `src/application/async_command_runner.rs` — focus on
  `AsyncCommandRunner::dispatch` (line 37 onward) and
  `with_vm_bus` to understand the return shape (`oneshot::Receiver`).
- `src/app.rs:191` — composition root for current `GpuiCommandRunner`.
- `src/app.rs:792` — call site #1.
- `src/app/playback_bar.rs:168` — call site #2.
- `src/app/search_dispatch.rs:747,817,917,938,976` — call sites #3–#7.
- `src/runtime/vm_bus.rs` (read for context; bridge does not consume it
  in this task).

Reference only:

- `src/library/app_impl.rs:779,798,...` — sample call sites you'll
  migrate in Task 002 (do not modify here).
- `src/discover/app_impl.rs:1139,...` — sample call sites for Task 003.

## Files Likely To Change

- `src/presentation/mod.rs` — register the new bridge module.
- `src/presentation/async_command_presenter.rs` — NEW. Houses
  `present_command<T, C, OnSuccess, OnError>` (or `AsyncCommandPresenter`
  struct + `present` method — pick whichever reads cleaner at call
  sites).
- `src/app.rs` — replace the `GpuiCommandRunner` construction at line
  191 with `AsyncCommandRunner` + a stored presenter (or just keep the
  `AsyncCommandRunner` and call the free fn at each site). Update the
  one `command_runner.run(...)` site at line 792.
- `src/app/playback_bar.rs` — update one call site.
- `src/app/search_dispatch.rs` — update five call sites.

## Do Not Touch

- `src/library/app_impl.rs` — Task 002 owns this.
- `src/discover/app_impl.rs` — Task 003 owns this.
- `src/presentation/gpui_command_runner.rs` — Task 004 deletes this
  file; in this task it remains for sites that haven't migrated yet.
- The `async-runtime` feature flag in `Cargo.toml` — Task 004 retires
  it; for this task the flag remains on.
- `CommandBus` (`src/application/command_bus.rs` or wherever it lives) —
  retained per ADR 0040.

## Constraints

- The presenter's signature must be a one-to-one replacement for
  `GpuiCommandRunner::run`, except the first argument changes from
  `&self` (on the sync runner) to `&AsyncCommandRunner`. Call sites
  should become a single-token substitution (`command_runner.run(...)` →
  `present_command(&self.command_runner, ...)` or
  `self.command_runner.present(...)`).
- Callbacks MUST execute on the GPUI thread. The presenter accepts a
  GPUI `&mut Context<T>` and uses `cx.spawn` to schedule the callback
  on entity update. The `oneshot::Receiver` is awaited inside the
  spawn closure.
- The presenter must publish nothing on `VmBus` (the `AsyncCommandRunner`
  already does so when constructed via `with_vm_bus`). Do not
  double-publish.
- `src/presentation/` is the only module that may import both `tokio`
  and `gpui`. Do not extend that exception elsewhere.
- All seven migrated call sites must compile and behave identically
  under default features. No behavior change.
- No new `#[allow(...)]` directives.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read the parent plan and all *Files To Inspect — Required* entries.
2. Note the existing `GpuiCommandRunner::run` signature:

   ```rust
   pub fn run<T, C, OnSuccess, OnError>(
       &self,
       command: C,
       context: CommandContext,
       cx: &mut Context<T>,
       on_success: OnSuccess,
       on_error: OnError,
   )
   ```

   The presenter mirrors this. Internally it calls
   `self.dispatch(command, context)` on the supplied
   `AsyncCommandRunner`, then `cx.spawn`s a closure that awaits the
   `oneshot::Receiver` and invokes `on_success` / `on_error` via
   `weak.update(cx, |this, cx| { on_success(this, output, cx) })`.
3. Create `src/presentation/async_command_presenter.rs` with the
   bridge. Document the contract at the top of the file (one short
   doc comment referencing ADR 0040 and this task).
4. Register the module in `src/presentation/mod.rs`. Decide visibility:
   `pub(crate) mod async_command_presenter;` plus a `pub(crate) use`
   re-export of `present_command` (or `AsyncCommandPresenter`).
5. In `src/app.rs`:
   - Replace the `GpuiCommandRunner::new` construction at line 191 with
     `AsyncCommandRunner::with_vm_bus(...)` (already imported by the
     runtime). Store it in the same field; the field's type changes
     from `GpuiCommandRunner` to `AsyncCommandRunner`.
   - Update the single `command_runner.run(...)` call site at line 792
     to call the presenter.
6. In `src/app/playback_bar.rs`: update the one call site (line 168).
7. In `src/app/search_dispatch.rs`: update the five call sites (lines
   747, 817, 917, 938, 976). These five sites are inside the same
   method/file family; expect uniform patterns.
8. Run the five gates. If any test fails for a reason unrelated to
   this task (e.g., the broken `--no-default-features` build path),
   note it in the report but do not fix here.
9. Smoke: build, launch the app if possible, exercise the playback bar
   play/pause control (call site #2) and submit one toolbar search
   (call site #1 — opens search results).

## Acceptance Criteria

- `src/presentation/async_command_presenter.rs` exists with the bridge.
- All seven `src/app/` call sites migrated; `grep "GpuiCommandRunner"
  src/app.rs src/app/playback_bar.rs src/app/search_dispatch.rs`
  returns no hits.
- `grep "GpuiCommandRunner" src/library/ src/discover/`
  still returns hits (Task 002/003 will handle).
- `cargo build` passes with default features.
- `cargo test --lib` passes.
- `cargo clippy -- -D warnings` passes.
- No new `#[allow(...)]` directives.
- The presenter executes callbacks on the GPUI thread (verified by
  reading the bridge code).

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded migration task — first of four.

Read in order:

1. This task file in full.
2. `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md` (focus
   on Decision + Invariants + Risk Areas).
3. `docs/adr/0040-async-vm-runtime.md` (status block + Invariants
   section).
4. `src/presentation/gpui_command_runner.rs` (full).
5. `src/application/async_command_runner.rs` lines 30–110.
6. Each of the seven listed call sites.

Goal:

Build `src/presentation/async_command_presenter.rs` exporting
`present_command<T, C, OnSuccess, OnError>` (or
`AsyncCommandPresenter::present`) that mirrors
`GpuiCommandRunner::run`'s argument shape, internally dispatches via
`AsyncCommandRunner`, awaits the resulting `oneshot::Receiver` inside
`cx.spawn`, and invokes the success/error callback on the GPUI thread
through `weak.update`. Migrate the seven `src/app/` call sites
(`src/app.rs:191,792`, `src/app/playback_bar.rs:168`,
`src/app/search_dispatch.rs:747,817,917,938,976`) to the bridge.

Constraints:

- One-to-one signature replacement at call sites.
- Callbacks execute on the GPUI thread.
- No `VmBus::publish` from the bridge; the `AsyncCommandRunner` already
  emits invalidations.
- `src/presentation/` is the only module allowed to import both `tokio`
  and `gpui`.
- Do NOT touch `src/library/` or `src/discover/` — later tasks own
  those.
- No `#[allow(...)]`.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. New file path + bridge signature.
2. Composition-root change in `src/app.rs` (line + field type before
   and after).
3. The seven migrated call sites (file:line before → after).
4. Any closure that needed hand-tuning vs purely mechanical
   substitution.
5. Five-gate results.
6. Deviations + unresolved concerns.

## Escalation Triggers

- A call site passes a closure that captures GPUI-specific state in a
  way the bridge signature can't express. Report the closure shape;
  propose an extended bridge signature (e.g., an additional generic
  parameter) rather than special-casing the call site.
- `AsyncCommandRunner::dispatch` signature does not match what the
  user's description implies (e.g., it takes additional arguments).
  Report the actual signature; adapt accordingly.
- The composition root currently constructs `GpuiCommandRunner` from
  arguments that don't exist on `AsyncCommandRunner` (e.g., a missing
  `RuntimeHost`). Report what's missing and where it would be plumbed
  from; do not stub.
- The `search_dispatch.rs` call sites take different ownership shapes
  (e.g., some pass `Weak<Entity<T>>`, others `&mut Context<T>`).
  Report; the bridge must support both, or each call site needs its
  own minor adaptation.
