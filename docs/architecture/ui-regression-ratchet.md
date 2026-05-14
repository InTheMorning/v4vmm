# UI Regression Ratchet

Status: Active - 2026-05-14.

## Purpose

This project treats user-confirmed UI behavior as a locked invariant. Agents
must not rely on memory or visual intent alone after a bug is fixed.

## Rules

- Every user-confirmed bug fix gets a guard in the same change: unit test,
  architecture test, visual smoke checklist, or documented manual verification.
- Completed ADR behavior is locked unless a new ADR changes it.
- No shell/layout change may land without scroll-chain verification for Search
  results, Recent Feeds, feed detail track lists, Library sidebar/detail,
  playlist detail, and track inspectors.
- Recent Feeds reachability is invariant after any search, filter change,
  selection, or detail navigation.
- Search type filters apply to every visible result section.
- Inspectors must not show raw transport errors for unavailable optional panels.
- Previously fixed flows must not be simplified without a parity guard.
- Subagents get bounded write scopes and must not redesign adjacent surfaces.

## Current Guards

- `tests/architecture_tests.rs::agent_guidelines_lock_user_confirmed_regression_ratchet`
- `tests/architecture_tests.rs::workspace_layout_render_uses_frame_shell_without_screen_internals`
- `tests/architecture_tests.rs::global_search_replaces_screen_local_search_chrome`
- `view_models::search::tests::search_render_snapshot_keeps_recent_feeds_reachable_after_search`
