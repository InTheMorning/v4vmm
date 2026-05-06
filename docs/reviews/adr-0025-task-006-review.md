# ADR 0025 Task 006 Review: Retire Theme Compatibility Shim

## Reviewed Artifact

- Task packet: `docs/tasks/adr-0025-task-006-retire-theme-shim.md`
- Diff scope: removal of `ui::theme`, replacement fixed-geometry/style module,
  status glyph migration, architecture-test hardening, ADR/plan updates.

## Verdict

Pass.

## Required Fixes

- None.

## Optional Improvements

- `ui::style` still carries no-argument color role helpers for legacy render
  paths. It now follows the installed appearance, but future work should
  continue moving those call sites to direct token, primitive, and composite
  APIs.
- Completed by later ADR 0025 work: fixed geometry moved to `ui::layouts`, and
  `StatusRole` moved out of `ui::style` into typed UI roles exported by
  `ui::composites`.

## Architectural Drift

- None. The deprecated `ui::theme` module is gone, screen files no longer
  import `theme::color`, `theme::badges`, or `theme::glyphs`, and
  architecture-test baselines are tightened to zero.

## Missing Tests

- No visual snapshot tests were added. Coverage is from architecture tests,
  contrast tests, and the full Rust test suite. Manual visual smoke is still
  recommended before a release because this slice touches global styling.

## Merge Recommendation

- Mergeable.
