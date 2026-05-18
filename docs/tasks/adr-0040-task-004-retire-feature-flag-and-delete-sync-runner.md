# ADR 0040 Task 004 — Retire `async-runtime` Feature Flag and Delete `GpuiCommandRunner`

Status: Completed - 2026-05-18. The retired command-runner and feature
flag surfaces were removed. The requested strict `cx.spawn` guard was
converted to `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
because unrelated pre-existing screen spawns remain outside the legacy
command-runner scope.

## Goal

Final retirement slice. Remove the `async-runtime` Cargo feature flag,
strip every `#[cfg(feature = "async-runtime")]` directive in `src/`,
delete `GpuiCommandRunner` and its scaffolding, add three regression
guards, bump ADR 0040 to "Implemented including retirement", and move
the deferred-work-index entry to "Recently Resolved".

Plan reference: `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md`.
Prerequisite: Tasks 001, 002, and 003 landed.
`grep -r "GpuiCommandRunner" src/` returns no hits in `src/app/`,
`src/library/`, or `src/discover/` (only the type definition in
`src/presentation/gpui_command_runner.rs` remains).

## Files To Inspect

Required:

- `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md` (Decision,
  Invariants — especially the "additions to ADR 0040" list, and
  Verification).
- `docs/adr/0040-async-vm-runtime.md` (current status block + references
  to retirement).
- `docs/plans/deferred-architecture-work-index.md` (item #3).
- `Cargo.toml` (`[features]` table and `tokio` / `tokio-util`
  dependency declarations).
- `src/presentation/gpui_command_runner.rs` (file to delete).
- `src/presentation/mod.rs` (remove the module registration).
- `src/lib.rs` (line 27-28: the `#[cfg(feature = "async-runtime")]
  pub mod runtime;` block).
- `src/view_models/mod.rs:80` (the `pub mod search_results;` with cfg
  gate, if still gated).
- `tests/architecture_tests.rs` (read enough of the file to understand
  the existing guard style; the new guards must match).

Grep targets to enumerate before editing:

```bash
grep -rn 'cfg(feature = "async-runtime")' src/
grep -rn "async-runtime" Cargo.toml
grep -rn "GpuiCommandRunner" src/ tests/
grep -rn 'cx\.spawn' src/
```

## Files Likely To Change

- `Cargo.toml` — remove `async-runtime` from `[features]`; drop
  `optional = true` from `tokio` / `tokio-util` declarations; ensure
  both are unconditional dependencies.
- `src/lib.rs` — strip the `#[cfg(feature = "async-runtime")]` directive
  preceding `pub mod runtime;` and any other gated module declarations.
- `src/view_models/mod.rs` — strip the gate from `pub mod search_results;`.
- `src/view_models/search_results/mod.rs` — remove the
  `#![cfg(feature = "async-runtime")]` inner attribute at line 8.
- `src/view_models/recent_feeds.rs` — remove any `#[cfg(feature = "async-runtime")]`
  if present.
- Every other file that contains `#[cfg(feature = "async-runtime")]`
  or `#[cfg(not(feature = "async-runtime"))]` (37 hits total per the
  pre-task audit — enumerate with grep and strip each).
- `src/presentation/mod.rs` — remove the `pub mod gpui_command_runner;`
  declaration and any re-exports.
- `src/presentation/gpui_command_runner.rs` — DELETE (entire file).
- `tests/architecture_tests.rs` — add three new guards (see *Guards*
  below).
- `docs/adr/0040-async-vm-runtime.md` — update Status block to
  "Implemented including retirement - 2026-05-18 (or current date)" and
  add a closing sentence about the retired path.
- `docs/plans/deferred-architecture-work-index.md` — move item #3 to
  "Recently Resolved" with the closure line.

## Do Not Touch

- The presentation bridge (`src/presentation/async_command_presenter.rs`).
- `AsyncCommandRunner` — the replacement.
- `CommandBus` — explicitly retained per ADR 0040 invariants.
- Any non-retirement changes to ADR 0040 invariants (additions only —
  no rewording of existing invariants).

## Constraints

- The retirement is atomic per file: a file that previously had
  `#[cfg(feature = "async-runtime")] pub mod foo;` becomes
  `pub mod foo;`. Do not leave half-gated files.
- The Cargo `[features]` table either has no `async-runtime` entry at
  all, or, if Cargo requires a `default = []` entry, the entry is
  removed cleanly.
- `tokio` and `tokio-util` dependencies must no longer be `optional`.
  If they had separate feature-gated entries (e.g., `tokio` with
  feature-list), consolidate into one unconditional dependency line.
- The three new guards (see below) must pass after this task and fail
  if any retired surface is reintroduced.
- No `#[allow(...)]` directives. No `#[allow(dead_code)]` left behind.
- Never skip hooks. Don't commit unless explicitly asked.

## Guards to add

In `tests/architecture_tests.rs`, mirror the path-walk style used by
existing guards (e.g., `nav_top_drives_content_list_body_switch`):

1. **`gpui_command_runner_is_retired`** — walks `src/` and asserts no
   file contains the string `GpuiCommandRunner` (excluding comments
   that intentionally reference the historical name in module docs;
   the implementer decides whether to allow such comments — safer to
   disallow entirely and rephrase any historical references).
2. **`async_runtime_feature_flag_is_retired`** — walks `src/` and
   asserts no file contains `cfg(feature = "async-runtime")` or
   `cfg(not(feature = "async-runtime"))`. Also asserts
   `Cargo.toml` does not contain `async-runtime` (load the manifest
   and grep the parsed `[features]` table or string-search the file).
3. **`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`** —
   walks `src/`, allows the pre-existing non-presentation/runtime
   `cx.spawn(` baseline, and fails if that debt grows. Full screen-spawn
   retirement is separate from the legacy command-runner deletion.

## Implementation Steps

1. Read this task, the parent plan, ADR 0040, and the deferred-work
   index entry.
2. Run the grep targets in *Files To Inspect* to confirm the current
   state. Confirm Tasks 001/002/003 left no `GpuiCommandRunner`
   references in `src/app/`, `src/library/`, or `src/discover/`.
3. Edit `Cargo.toml`:
   - Remove the `async-runtime` entry from `[features]` (and from
     `default = [...]`).
   - Drop `optional = true` from `tokio` and `tokio-util`.
   - Confirm both deps are unconditional, with the same feature lists
     (`rt-multi-thread`, `sync`, `time`, `macros`, `rt`) they had
     when activated.
4. Strip every `#[cfg(feature = "async-runtime")]` and
   `#[cfg(not(feature = "async-runtime"))]` directive across `src/`.
   For inner attributes (`#![cfg(...)]`), strip them as well.
   This includes:
   - `src/lib.rs:27-28` (runtime module gate).
   - `src/view_models/mod.rs:80` (search_results module gate).
   - `src/view_models/search_results/mod.rs:8` (inner attribute).
   - All other 30+ hits enumerated via grep.
5. If any code was previously inside `#[cfg(not(feature = "async-runtime"))]`
   (the legacy synchronous path), DELETE that code. The synchronous
   path is retired; there is no replacement needed because Tasks
   001–003 already migrated every call site to the async path.
6. Remove the `pub mod gpui_command_runner;` declaration from
   `src/presentation/mod.rs` and any re-exports.
7. Delete `src/presentation/gpui_command_runner.rs`.
8. Build and check that nothing references the deleted file.
9. Add the three regression guards to `tests/architecture_tests.rs`.
   Verify each fails when its invariant is broken (manually toggle a
   string in `src/` to confirm) and passes on the current tree.
10. Run the five gates.
11. Update `docs/adr/0040-async-vm-runtime.md`:
    - Status block: `Implemented including retirement - 2026-05-18`
      (or current date).
    - Add a paragraph at the end of the Status block noting that
      `GpuiCommandRunner` and the `--no-default-features` build path
      have been retired, with pointer to this task and the regression
      guards.
12. Update `docs/plans/deferred-architecture-work-index.md`:
    - Remove item #3 (ADR 0040 legacy synchronous scheduling
      retirement) from the "Priority Order" list.
    - Add a bullet under "Recently Resolved" noting: "ADR 0040 legacy
      scheduling retirement completed YYYY-MM-DD via Tasks 001–004.
      `GpuiCommandRunner`, `--no-default-features` build path, and
      the `async-runtime` Cargo feature are all retired. Guards
      `gpui_command_runner_is_retired`,
      `async_runtime_feature_flag_is_retired`, and
      `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
      prevent retired-surface regression and pin unrelated screen-spawn
      debt."

## Acceptance Criteria

- `Cargo.toml` has no `async-runtime` feature.
- `tokio` and `tokio-util` are unconditional dependencies.
- `grep -r 'cfg(feature = "async-runtime")' src/` returns no hits.
- `grep -r "GpuiCommandRunner" src/` returns no hits.
- `src/presentation/gpui_command_runner.rs` does not exist.
- `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime` passes
  and pins the remaining pre-existing non-presentation/runtime
  `cx.spawn` baseline.
- The three new guards exist in `tests/architecture_tests.rs` and
  pass.
- ADR 0040 status reflects retirement.
- Deferred-work-index item #3 is in "Recently Resolved".
- All five gates pass.
- No new `#[allow(...)]`.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

Smoke (post-retirement):

```bash
grep -rn 'cfg(feature = "async-runtime")' src/   # expect: no hits
grep -rn "GpuiCommandRunner" src/                # expect: no hits
cargo test --test architecture_tests cx_spawn_debt_does_not_grow
                                                  # expect: pass
```

## Prompt for lower-context coding model

You are implementing the final retirement task — fourth of four.

Prerequisites: Tasks 001, 002, and 003 have landed. No call site of
`GpuiCommandRunner::run` remains under `src/app/`, `src/library/`, or
`src/discover/`. The presentation bridge at
`src/presentation/async_command_presenter.rs` owns the GPUI bridge.

Read:

1. This task file in full.
2. `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md` (Decision,
   "Invariants additions", Verification).
3. `docs/adr/0040-async-vm-runtime.md` (status block + Invariants).
4. `docs/plans/deferred-architecture-work-index.md` item #3.
5. `Cargo.toml`.
6. `src/presentation/gpui_command_runner.rs` and
   `src/presentation/mod.rs`.
7. `tests/architecture_tests.rs` (skim — match guard style for new
   guards).

Run these greps before editing to enumerate the surface:

- `grep -rn 'cfg(feature = "async-runtime")' src/`
- `grep -rn "async-runtime" Cargo.toml`
- `grep -rn "GpuiCommandRunner" src/`

Goal:

1. Remove `async-runtime` from `Cargo.toml [features]`. Make `tokio`
   and `tokio-util` unconditional (no `optional = true`).
2. Strip every `#[cfg(feature = "async-runtime")]` and
   `#[cfg(not(feature = "async-runtime"))]` directive across `src/`.
   Delete any code inside `#[cfg(not(feature = "async-runtime"))]`
   blocks — that was the legacy synchronous path, fully replaced by
   the bridge.
3. Delete `src/presentation/gpui_command_runner.rs` and remove its
   declaration from `src/presentation/mod.rs`.
4. Add three guards to `tests/architecture_tests.rs`:
   - `gpui_command_runner_is_retired`
   - `async_runtime_feature_flag_is_retired`
   - `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
5. Update `docs/adr/0040-async-vm-runtime.md` status to
   "Implemented including retirement - <date>" with a closing
   paragraph.
6. Move deferred-index item #3 to "Recently Resolved".

Constraints:

- No `#[allow(...)]` directives.
- No `#[allow(dead_code)]`.
- Never skip hooks. Don't commit.
- Keep `CommandBus` and `AsyncCommandRunner` untouched.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. `Cargo.toml` diff (features + deps).
2. Count of `#[cfg(feature = "async-runtime")]` directives stripped.
3. Count of `#[cfg(not(feature = "async-runtime"))]` blocks removed
   (with line counts of deleted code per file).
4. Confirmation that `src/presentation/gpui_command_runner.rs` is
   deleted.
5. Three new guard names + the assertions each makes.
6. ADR 0040 status line after edit.
7. Deferred-index closure line.
8. Five-gate results.
9. Deviations + unresolved concerns.

## Escalation Triggers

- A `#[cfg(not(feature = "async-runtime"))]` block contains code
  that's NOT a legacy synchronous fallback — i.e., some other
  semantically distinct path. Report; do not delete blindly. The
  expected case is "no such blocks exist or all such blocks are
  legacy fallbacks fully replaced by the bridge".
- After removing the feature flag, a Cargo dependency line that
  previously had `optional = true` is referenced by a non-deleted
  feature entry (e.g., another feature in `[features]` depended on
  `async-runtime`). Report the dependency chain; the user may need to
  decide whether to retire the dependent feature as well.
- The composition site `src/app.rs:191` (or library/discover
  equivalents) has somehow retained a reference to
  `GpuiCommandRunner` that earlier tasks missed. Report; do not delete
  `gpui_command_runner.rs` until the call site is also gone.
- A new architecture guard fails on its first run for a reason other
  than the retirement surface (e.g., the path-walk helper used in the
  guard has a different argument shape than expected). Inspect
  existing guards in `tests/architecture_tests.rs` and match style;
  do not skip the guard.
