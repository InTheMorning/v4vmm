# ADR 0043 Task 004: Guards, Visual Proof, and Readiness Review

Status: Awaiting operator visual recheck after follow-up fixes on 2026-05-14.

## Goal

Add final architecture guards, run full verification, capture visual
proof, and update the ADR 0043 review checklist.

## Files to Inspect

- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `docs/tasks/adr-0043-task-001-app-toolbar-frame.md`
- `docs/tasks/adr-0043-task-002-global-search-contract.md`
- `docs/tasks/adr-0043-task-003-search-workspace-results.md`
- `docs/reviews/adr-0043-review-checklist.md`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0043-review-checklist.md`
- Possibly minor fixes in files touched by Tasks 001-003

## Do Not Touch

- Do not introduce new feature behavior.
- Do not refactor unrelated UI surfaces.
- Do not alter DB schema or playback behavior.

## Constraints

- Every new architecture rule must directly enforce an ADR 0043
  invariant.
- Visual proof must include light and dark themes.
- Check both normal and narrow window widths.
- Any remaining duplicate search chrome must be either removed or
  recorded as a readiness failure.

## Implementation Steps

1. Add guards for single visible toolbar search ownership.
2. Add guards that Library and Search shells do not define duplicate
   visible search input chrome.
3. Add guards that Now Playing remains app-shell-owned and is not a
   single-use composite.
4. Add guards that toolbar scope labels, ids, placeholder, and a11y
   labels come from view-model display contracts.
5. Run required test and lint gates.
6. Pending: capture visual proof for light/dark normal/narrow toolbar states
   after the 2026-05-14 narrow-toolbar fix.
7. Update the ADR 0043 review checklist with pass/fail, evidence, and
   merge recommendation.

## Acceptance Criteria

- All ADR 0043 invariants have either code coverage, architecture-test
  coverage, or explicit review-checklist evidence.
- `cargo fmt -- --check`, `cargo check`, `cargo test`, and
  `cargo clippy -- -D warnings` are green.
- Visual proof confirms toolbar, Now Playing frame, and grouped Search
  results are legible in light and dark themes.
- Review checklist records `Proceed` only if all gates are green.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `docs/reviews/adr-0043-review-checklist.md`
- `tests/architecture_tests.rs`

Goal:
- Add final ADR 0043 architecture guards, run verification, capture
  visual proof, and update the review checklist.

Constraints:
- Add no new feature behavior.
- Guards must map to explicit ADR invariants.
- Visual proof must cover light and dark themes.

Do not touch:
- Unrelated UI surfaces
- Database schema
- Playback behavior

Acceptance criteria:
- Required gates are green.
- Visual proof is recorded.
- Review checklist has a clear readiness decision.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Visual proof shows toolbar overlap, unreadable text, or clipped
  controls at normal or narrow widths.
- Any guard would require broad baselines instead of enforcing the new
  invariant directly.

## Blocker Notes

- `cargo fmt -- --check`, `cargo check`, `cargo test`, and
  `cargo clippy -- -D warnings` were green after Tasks 001-003.
- Follow-up design review fixes added the toolbar search icon, moved the
  toolbar Search button label into `AppToolbarVm`, hid Index-only filters for
  Library scope, and renamed the app-shell tab to Search.
- User screenshots on 2026-05-13 exposed narrow-toolbar clipping risk where
  scope labels could partially appear between Settings and the Now Playing
  frame. The initial mitigation hid scope controls and the submit button at
  named layout-token breakpoints; the later HIG correction below keeps submit
  visible and moves narrow scope switching into a menu.
- User screenshot review on 2026-05-14 still showed narrow dark-toolbar
  clipping, with optional scope/submit controls competing with the global
  search field. The named breakpoints were raised so those optional controls
  collapse earlier and keep the search field and Now Playing frame legible.
- A second 2026-05-14 screenshot still showed the global search field clipped
  by the full-width Now Playing frame. Now Playing now switches to a named
  compact width below the toolbar breakpoint, keeping its frame while
  preserving usable space for the center search field.
- Follow-up HIG review on 2026-05-14 found that hiding scope controls and the
  submit button removed primary toolbar actions at narrow widths. The Search
  submit button remains inline above the compact breakpoint; below it, the
  search field stays visible and Search/scope commands move into the shared
  context-menu primitive with VM-owned ids, labels, and accessibility text.
- Operator screenshot review on 2026-05-14 showed the medium-width correction
  still clipped between Settings and Now Playing. The toolbar now has a second
  compact step: Now Playing shrinks to `MenuRegular`, and global search renders
  only the input plus overflow menu below the compact breakpoint.
- Visual proof could not be captured because `DISPLAY=:0 wmctrl -l`
  failed with `Authorization required, but no authorization protocol specified`
  and `Cannot open display`, including when retried with escalated
  permissions.
- Readiness remains pending until light/dark normal/narrow screenshots can be
  captured and reviewed.
