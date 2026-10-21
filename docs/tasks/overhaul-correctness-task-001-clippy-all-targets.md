# Overhaul Correctness Task 001: Clippy For All Targets

Status: Ready - 2026-10-20.

## Goal

`cargo clippy --all-targets -- -D warnings` is Green, and the gate in `AGENTS.md` uses it. The test code then has the same lint discipline as the product code.

## Authority

- [AGENTS.md](../../AGENTS.md) owns the gate: "Every commit passes `cargo check`, `cargo fmt --check`, and `cargo clippy -- -D warnings`". This task changes that line and the "Build, Test, Lint" block.
- The [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md), "Throughout: Code Correctness": add `cargo clippy --all-targets -- -D warnings` to the gate.
- Operator decision of 2026-10-20: one cleanup packet with no behavior change.

## Recorded Facts - 2026-10-20

- `cargo clippy --all-targets -- -D warnings` gives about 95 errors, each in test code or in a test-only path.
- The most frequent lints: `doc_markdown` (13), `items_after_statements` (12), `default_trait_access` (more than 20 in total), `float_cmp` (7), `unused_io_amount` (5), `single_element_loop` (5), `duration_suboptimal_units` (5), `items_after_test_module` (4), `assertions_on_constants` (3).
- The files with the most errors: `src/library/app_impl.rs` (10), `src/app/bootstrap.rs` (10), `src/ui/composites/maintenance_page.rs` (6), `src/rss/check_apply.rs` (6), `src/view_models/startup/capabilities.rs` (5), `src/runtime/playlist_rss_check.rs` (5), `tests/architecture_tests.rs` (4).
- The repository has no CI workflow file. `AGENTS.md` says that CI runs build, test, clippy and format check.

## Design

- Correct each lint at its place. Prefer the fix that clippy proposes. A test that compares `f32` or `f64` uses an explicit tolerance. It can also use `assert_eq!` on a value that is exact by construction, with a comment that says why.
- `#[expect(clippy::...)]` with a `reason` is allowed only when the fix makes a test less clear. Each `expect` names the lint and the reason. No `allow`.
- Change no product behavior and no test assertion meaning.
- Change the gate in `AGENTS.md`: the working rule "Zero warnings" and the "Build, Test, Lint" block use `cargo clippy --all-targets -- -D warnings`. Correct the CI sentence: the repository has no CI workflow file, so the sentence states that the gate runs locally.

## Required Changes

1. Correct each error of `cargo clippy --all-targets -- -D warnings`.
2. Change the gate lines of `AGENTS.md`.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| RC1-01 | `cargo clippy --all-targets -- -D warnings` is Green |
| RC1-02 | `cargo test` gives the same count of passing tests as before the task |
| RC1-03 | `git diff` shows no change to product code outside a `#[cfg(test)]` path, except a lint fix that the report names |

## Visual Acceptance Criteria

None. This task changes no screen.

## Exclusions

- No new lint group, and no change to `#![warn(clippy::pedantic)]`.
- No refactor of a test beyond its lint.

## Checks

```bash
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo build --bin v4vmm
```

## Rollback

Revert the commit.

## Prompt for lower-context coding model

You are implementing one bounded task.

Read:
- `AGENTS.md`
- This packet: `docs/tasks/overhaul-correctness-task-001-clippy-all-targets.md`

Goal:
- Make `cargo clippy --all-targets -- -D warnings` Green with no behavior change, and change the gate lines of `AGENTS.md`.

Constraints:
- Follow the rust-dev skill. Write each comment in ASD-STE100 Simplified Technical English (`~/.agents/skills/asd-ste100/SKILL.md`).
- Record the test count before the first edit, and compare it at the end.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`. Do not run the app.

Do not touch:
- Any ADR, and each document other than this packet and the gate lines of `AGENTS.md`.

At the end, report the lints fixed by kind and by file, and each `#[expect]` with its reason.
Also report each product-code change, and the test counts before and after.

Stop and report the problem, and do not guess, when:
- A lint fix changes the meaning of a test or of product code.
