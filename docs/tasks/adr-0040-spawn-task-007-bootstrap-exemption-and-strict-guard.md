# ADR 0040 Spawn Task 007 — Bootstrap Exemption + Strict Guard + ADR Refresh

Status: Completed - 2026-05-18.

## Goal

Final closure slice. After Tasks 001-006 land, the only remaining
`cx.spawn` outside `src/presentation/` and `src/runtime/` is the
window-activation defer at `src/app/bootstrap.rs:135`. That site is
pure GPUI lifecycle (16ms + 100ms refresh after window creation), not
domain work, and the ADR 0040 invariant ("Screens MUST NOT call
`cx.spawn` for *domain work*") does not apply.

This task:

1. Annotates `src/app/bootstrap.rs:135` with a one-line comment naming
   the GPUI quirk it works around (which makes the exemption legible to
   future readers).
2. Replaces the debt-baseline guard
   `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime` with
   a strict allowlist guard
   `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap` that
   fails on any non-allowlisted hit.
3. Updates ADR 0040 Status block to drop the
   "broader screen-local cx.spawn cleanup is not complete" caveat
   sentence.
4. Moves deferred-index item #2
   ("Screen-local `cx.spawn` retirement") to the "Recently Resolved"
   section.

Plan reference: `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.
Prerequisites: Tasks 001-006 landed. The pre-task baseline reads:

| File                          | Sites | Action this task |
|-------------------------------|------:|------------------|
| `src/app.rs`                  | 0     | (already clean)  |
| `src/app/bootstrap.rs`        | 1     | exempt + comment |
| `src/app/search_dispatch.rs`  | 0     | (already clean)  |
| `src/library/app_impl.rs`     | 0     | (already clean)  |
| `src/discover/app_impl.rs`    | 0     | (already clean)  |

If any file still shows a non-zero count outside `bootstrap.rs`, the
prerequisite is not met — the task escalates rather than softening the
guard.

## Files To Inspect

Required:

- `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
  (Decision, Invariants).
- `docs/adr/0040-async-vm-runtime.md` (current Status block; the
  closing sentence about residual debt is the line to remove).
- `docs/plans/deferred-architecture-work-index.md` (item #2 + the
  "Recently Resolved" section).
- `src/app/bootstrap.rs` — read lines **100-156** to see the surrounding
  context for the `cx.spawn` defer.
- `tests/architecture_tests.rs:10511-10555` — current debt-baseline
  guard. This task replaces it.
- The two ADR 0040 Task 004 retirement guards
  (`gpui_command_runner_is_retired`,
  `async_runtime_feature_flag_is_retired`) for style reference.

Grep targets:

```bash
grep -rn "cx\.spawn" src/ | grep -vE 'src/(presentation|runtime|app/bootstrap)'
# Expect: NO hits. If any appear, escalate.

grep -n "cx_spawn_debt_does_not_grow" tests/architecture_tests.rs
# Expect: one match — the guard this task removes.

grep -n "broader screen-local cx.spawn cleanup" docs/adr/0040-async-vm-runtime.md
# Expect: one match — the caveat sentence to remove.
```

## Files Likely To Change

- `src/app/bootstrap.rs` — add one short comment above the `cx.spawn`
  call at line ~135 explaining the GPUI window-activation quirk
  (16ms then 100ms refresh defer) and that this is a presentation
  lifecycle nudge, not domain work. Mention the architecture guard
  by name.
- `tests/architecture_tests.rs`:
  - Remove `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`.
  - Add `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`
    which walks `src/`, fails on any `cx.spawn(` outside
    `src/presentation/`, `src/runtime/`, and `src/app/bootstrap.rs`.
- `docs/adr/0040-async-vm-runtime.md`:
  - Status block: drop the closing sentence that mentions screen-spawn
    debt being pinned by the debt-baseline guard. Replace with a
    sentence noting full retirement landed via Tasks 001-007 of the
    screen-local cx.spawn retirement plan and that the new
    `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`
    guard pins the new shape.
- `docs/plans/deferred-architecture-work-index.md`:
  - Remove item #2 ("Screen-local `cx.spawn` retirement") from the
    "Priority Order" list.
  - Add to "Recently Resolved":
    "Screen-local `cx.spawn` retirement completed YYYY-MM-DD via the
    seven-task screen-local cx.spawn retirement plan. The presentation
    bridge owns one-shot command/result GPUI dispatch; runtime actors
    own polling and saga flows. The only allow-listed exemption is
    `src/app/bootstrap.rs` (window-activation defer). Guard
    `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`
    pins the shape."

## Do Not Touch

- `src/runtime/`, `src/presentation/`, `src/application/`.
- Any file under `src/app/`, `src/library/`, `src/discover/` other
  than the bootstrap comment.
- The retired guards from ADR 0040 Task 004
  (`gpui_command_runner_is_retired`,
  `async_runtime_feature_flag_is_retired`) — keep them.
- The bootstrap behavior itself. Only the comment is new.

## Constraints

- The new guard's path matching must be exact: allow
  `src/presentation/`, `src/runtime/`, and the single file
  `src/app/bootstrap.rs`. Any new file or any spawn in a non-listed
  path fails the guard.
- The new guard must not allowlist anything else. If the user later
  needs another exemption, they update the guard explicitly.
- The comment on `bootstrap.rs:135` is one line + the call-site, not
  a paragraph. Reference the architecture guard by name so future
  readers know where the exemption is enforced.
- The ADR 0040 Status block update should be one-paragraph max — the
  same length budget as the existing closing paragraph.
- No new `#[allow(...)]` anywhere.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Run the grep targets above. Confirm zero non-bootstrap spawns
   outside presentation/runtime. If any remain, stop and escalate.
2. Add a comment above `cx.spawn(` at `src/app/bootstrap.rs:~135`:
   roughly "Window-activation nudge: GPUI sometimes needs a deferred
   refresh after initial window creation. Pure presentation lifecycle,
   not domain work — exempted from
   `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`."
   Keep it short.
3. Edit `tests/architecture_tests.rs`:
   - Delete the `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
     function (the entire `#[test]` block).
   - Add a new `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`
     `#[test]` function that walks `src/`, counts `cx.spawn(`
     occurrences per file, and asserts every hit lives in
     `src/presentation/`, `src/runtime/`, or exactly
     `src/app/bootstrap.rs`. Match the path-walk style of the
     existing retired guards (`gpui_command_runner_is_retired`).
4. Update `docs/adr/0040-async-vm-runtime.md` Status block:
   - Replace the closing sentence about the debt-baseline guard with
     a sentence stating screen-local `cx.spawn` retirement landed
     and pointing to the new guard.
5. Update `docs/plans/deferred-architecture-work-index.md`:
   - Remove item #2 from "Priority Order".
   - Re-number the remaining priority items if needed (item #3 →
     item #2, etc.) OR leave the numbering as-is with a note. Pick
     the style consistent with how prior retirement closures were
     handled (look at the existing entries that were removed for
     ADR 0040 Tasks 001-004).
   - Add a new "Recently Resolved" bullet (one paragraph) per the
     wording in *Files Likely To Change*.
6. Run all five gates.

## Acceptance Criteria

- `grep -n "cx\.spawn(" src/ | grep -vE 'src/(presentation|runtime|app/bootstrap)'`
  returns no hits.
- `tests/architecture_tests.rs` contains
  `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap` and
  does NOT contain `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`.
- The bootstrap.rs spawn has a one-line explanatory comment.
- ADR 0040 Status block does not mention the debt-baseline caveat.
- Deferred-index item #2 is in "Recently Resolved".
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

Plus:

```bash
grep -rn "cx\.spawn" src/ | grep -vE 'src/(presentation|runtime|app/bootstrap)'
# Expect: no hits
cargo test --test architecture_tests cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap
# Expect: pass
```

## Prompt for lower-context coding model

You are implementing the final closure task — seventh of seven in the
screen-local `cx.spawn` retirement plan.

Prerequisites: Tasks 001-006 landed. The only `cx.spawn` outside
`src/presentation/` and `src/runtime/` is the window-activation defer
at `src/app/bootstrap.rs:135`.

Read:

1. This task file in full.
2. `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
   (Decision, Invariants — the new guard name and exemption rule).
3. `docs/adr/0040-async-vm-runtime.md` (Status block).
4. `docs/plans/deferred-architecture-work-index.md` (item #2 + Recently
   Resolved).
5. `src/app/bootstrap.rs` around line 135.
6. `tests/architecture_tests.rs` — the existing debt-baseline guard
   (around line 10511) and the ADR 0040 Task 004 retirement guards
   for style.

Run before editing:

- `grep -rn "cx\.spawn" src/ | grep -vE 'src/(presentation|runtime|app/bootstrap)'`
- Expect zero hits. If non-zero, stop and report — Tasks 001-006 are
  not all landed.

Goal:

1. Add a one-line comment at `src/app/bootstrap.rs:~135` naming the
   GPUI window-activation quirk and citing the architecture guard.
2. Replace `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
   in `tests/architecture_tests.rs` with
   `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`. The
   new guard fails on any `cx.spawn(` outside
   `src/presentation/`, `src/runtime/`, or
   `src/app/bootstrap.rs`. Match the path-walk style of the existing
   retirement guards.
3. Update `docs/adr/0040-async-vm-runtime.md` Status block: replace
   the closing sentence about residual debt with a sentence noting
   full retirement landed via Tasks 001-007 and pointing to the new
   guard.
4. Move deferred-index item #2 ("Screen-local `cx.spawn` retirement")
   from "Priority Order" to "Recently Resolved". Use one paragraph
   summarizing the seven-task closure.

Constraints:

- No new `#[allow(...)]`.
- Only allowed paths in the new guard are `src/presentation/`,
  `src/runtime/`, and `src/app/bootstrap.rs`. No others.
- Bootstrap comment is one line plus the call, not a paragraph.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. The bootstrap.rs comment (verbatim).
2. The new guard name and the assertion it makes.
3. ADR 0040 Status block diff (one paragraph before, one after).
4. Deferred-index "Recently Resolved" bullet text.
5. Five-gate results.
6. Deviations + unresolved concerns.

## Escalation Triggers

- The pre-task grep returns hits outside the expected zero. Report
  which file(s); the task does not soften the guard or relax the
  rule. The user must close the remaining migration (revisit Tasks
  001-006) before this task can land.
- The bootstrap.rs spawn is doing something more than the
  16ms+100ms window-activation nudge. Read the body carefully; if
  the spawn now hides domain work (e.g., a background data fetch),
  that work needs migration first under the same plan. Report and
  escalate; do not exempt domain work.
- The "Recently Resolved" section in deferred-index has a different
  formatting convention from what's described here (e.g., requires a
  date in a different format). Match the existing convention; this
  task's wording is a target, not a literal.
- The `path-walk style` for the new guard requires a helper that
  doesn't exist (`rust_files_under`, `read_source`, `rel_path`).
  Look at how other path-walk guards reference these helpers — they
  should exist in the same test file or a sibling module. If they
  don't, stop and report; this guard's implementation depends on
  shared scaffolding.
