# UI Regression Ratchet

Status: Active - 2026-05-14.

## Purpose

This project treats user-confirmed UI behavior as a locked invariant. Agents
must not rely on memory or visual intent alone after a bug is fixed.

## Rules

- Every user-confirmed bug fix gets a guard in the same change: unit test,
  architecture test, visual smoke checklist, or documented manual verification.
- Completed ADR behavior is locked unless a new ADR changes it.
- Visual presentation, button behavior, and user-workflow changes must pass the
  UI change acceptance gate in `AGENTS.md` and the ownership gate in
  `docs/architecture/ui-backend-boundary.md`.
- Agents must not land isolated renderer tweaks for music presentation,
  buttons, rows, empty states, filters, inspectors, or workflow reachability
  when the same rule belongs in a shared view model, primitive, composite,
  token, or architecture guard.
- HIG product-completeness gaps are a separate polish backlog, not a mandate to
  reopen completed search/sidebar restructuring. Route recent searches,
  sidebar show/hide, Liquid Glass materials, and keyboard shortcuts through
  bounded tasks with the same ownership and proof requirements.
- No shell/layout change may land without scroll-chain verification for Search
  results, Recent Feeds, feed detail track lists, Library sidebar/detail,
  playlist detail, and track inspectors.
- Recent Feeds reachability is invariant after any search, filter change,
  selection, or detail navigation.
- Search type filters apply to every visible result section.
- Inspectors must not show raw transport errors for unavailable optional panels.
- Previously fixed flows must not be simplified without a parity guard.
- Subagents get bounded write scopes and must not redesign adjacent surfaces.

## Agent Acceptance Checklist

Before calling a UI change complete, the agent must be able to answer all of
these with evidence in the diff, tests, or final report:

- Which shared owner changed: GPUI-free view model/projection, primitive,
  composite, token/theme role, screen wiring, or guard?
- What existing duplication or drift was reduced or explicitly kept from
  growing?
- Which user workflow remains reachable after the change?
- Which regression guard blocks the same failure class?
- Which visual surface was inspected, or what residual visual risk remains?

If any answer is missing, the change is not complete. A small tweak is allowed;
a small screen-local shortcut that weakens ownership is not.

## Current Guards

- `tests/architecture_tests.rs::agent_guidelines_lock_user_confirmed_regression_ratchet`
- `tests/architecture_tests.rs::agent_guidelines_require_structural_ui_change_ownership`
- `tests/architecture_tests.rs::hig_product_polish_backlog_stays_separate_from_restructuring`
- `tests/architecture_tests.rs::workspace_layout_render_uses_frame_shell_without_screen_internals`
- `tests/architecture_tests.rs::global_search_replaces_screen_local_search_chrome`
- `view_models::search::tests::search_render_snapshot_keeps_recent_feeds_reachable_after_search`
