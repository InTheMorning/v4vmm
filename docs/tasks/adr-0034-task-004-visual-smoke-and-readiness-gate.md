# ADR 0034 Task 004: Visual Smoke and Readiness Gate

## Goal

Verify that scale-aware primitives and playlist popovers improve HIG structure
without making dense app surfaces worse.

## Files to Inspect

- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `docs/reviews/adr-0034-review-checklist.md`
- Changed files from Tasks 001-003

## Files Likely to Change

- `docs/reviews/adr-0034-review-checklist.md`

## Do Not Touch

- Runtime code unless visual smoke exposes a blocking regression that must be
  fixed before the gate can pass.

## Constraints

- Ask the user for screenshots. Do not use `xdotool` or coordinate automation.
- Do not mark the gate as pass without visual evidence or an explicit residual
  risk note.
- Do not proceed to richer playlist/playback feature work if this gate fails.

## Visual Smoke Required

- Library release detail add-to-playlist popover at medium scale.
- Library release detail add-to-playlist popover at a smaller or larger scale.
- Discovery recents grid after primitive scaling.
- Now-playing chrome after primitive scaling.

## Acceptance Criteria

- Popover remains anchored, compact, readable, and includes `+ New Playlist`.
- Button text, icon, padding, and hit target feel coherent at tested scales.
- Discovery recents titles/subtitles remain visible and stable.
- Now-playing chrome still fits the header band.
- Review checklist records pass/fail, screenshots requested, and residual
  risks.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `docs/tasks/adr-0034-task-004-visual-smoke-and-readiness-gate.md`
- `docs/reviews/adr-0034-review-checklist.md`

Goal:
- Complete final verification and record whether the UI scale work is safe for
  richer playlist/playback feature work.

Constraints:
- Ask the user for screenshots; do not use pointer automation.
- Documentation-only unless visual smoke reveals a blocker.
- Run full checks before marking the gate pass.

Do not touch:
- Runtime code unless the gate exposes a blocking regression.

Acceptance criteria:
- Checks are green.
- Required visual surfaces are reviewed from user screenshots.
- Gate status is recorded in the checklist.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If screenshots show cramped, oversized, overlapping, or missing controls,
  stop and create a targeted follow-up task instead of passing the gate.
