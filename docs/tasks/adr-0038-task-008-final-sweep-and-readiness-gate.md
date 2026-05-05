# ADR 0038 Task 008: Final Sweep and Readiness Gate

## Status

Completed on 2026-05-04. Readiness gate decision: **Proceed**.

Sweep outcomes:

- `render_track_row` legacy Discover wrapper inlined; only the
  canonical shared owner at
  `src/ui/shells/track.rs::render_track_row` remains.
- All baseline arrays in `tests/architecture_tests.rs` confirmed at
  zero (`DEPRECATED_VISUAL_HELPER_BASELINES`,
  `DIRECT_COMPONENT_BUTTON_BASELINES`,
  `PROVENANCE_DIFF_HELPER_BASELINES`,
  `SCREEN_LOCAL_PLAYLIST_POPOVER_BASELINES`,
  `RENDER_HELPER_DUPLICATION_BASELINES`).
- Direct `gpui_component::button::Button` usage in
  `src/ui/shells/discover/actions.rs` and
  `src/ui/shells/discover/search_input.rs` carries
  `// CONTROL-COMPAT(reason): ...` allowlist markers; no unmarked
  direct component usage anywhere.
- Screen `unwrap_or_default()` / `unwrap_or("")` audit confirms no
  display-string fallbacks remain in `src/library/app_impl.rs` or
  `src/search/app_impl.rs`; the four remaining sites are non-display
  data fallbacks (Vec default for failed DB reads, candidate-score
  normalization, dedup-key construction).
- Superseded plans `docs/plans/one-owner-per-surface-plan.md` and
  `docs/plans/post-adr-0033-ui-consolidation-plan.md` confirmed as
  no longer driving open work; their items landed under ADR 0038
  tasks 002/003/007 or roll up to the
  deferred-architecture-work index.
- Deferred-architecture-work index reconciled: ADR 0038 added no
  new deferred items.

Final readiness gate captured in
`docs/reviews/adr-0038-review-checklist.md`.

## Goal

Eliminate residual debt, retire the last duplicates, confirm every
baseline is at zero, and decide whether richer playlist/playback UI
work may proceed.

## Sweep Targets

- `render_track_row` duplicate at `src/search.rs:4428` (and the
  shared owner at `src/ui/shells/track.rs::render_track_row`
  post-Task-001). Consolidate; the search call site uses the shared
  owner.
- Any remaining `unwrap_or_default()` / `unwrap_or("")` in screens.
- Any composite still taking a loose `String` not on the explicit
  passthrough allowlist.
- Any guard with a non-zero baseline. Either retire the baseline or
  document why it persists.
- Any HIG surface (light + dark, a11y label) not yet captured in the
  visual smoke ledger.

## Readiness Gate

The gate decides whether the app architecture is ready for richer
playlist/playback feature work. Required answers:

- Every architecture guard is green with baselines at zero.
- Every entity detail surface renders through a PageVm + shell
  helper.
- Every interactive composite carries an a11y label.
- Light + dark visual smoke covers every main surface.
- `library.rs` and `search.rs` are thin entries.
- Task 003's VM consolidation has zero remaining screen-local
  fallback strings.
- The deferred-architecture-work index has been reconciled with
  ADR 0038 outcomes.

If any answer is "no", the gate records "Defer" with the missing
items and a follow-up task pointer. If all are "yes", the gate records
"Proceed" and the next ADR (richer feature work) may begin.

## Files Likely To Change

- `tests/architecture_tests.rs` — final guard cleanup, baseline
  retirement.
- `docs/reviews/adr-0038-review-checklist.md` — final gate decision.
- `docs/plans/deferred-architecture-work-index.md` — reconcile.
- Possibly small fixes scattered across the codebase.

## Constraints

- This task does not introduce new structure. It only retires debt
  and produces the gate decision.
- "Proceed" is binary; if any required answer is uncertain, the gate
  records "Defer" with a concrete follow-up.

## Definition of Done

- All architecture guards green, all baselines at zero.
- Visual smoke ledger fully populated.
- Readiness gate decision recorded in the review checklist with
  evidence.
- The two superseded plans (`one-owner-per-surface-plan.md`,
  `post-adr-0033-ui-consolidation-plan.md`) confirmed as no longer
  driving any open work; if anything remains, file it as a follow-up
  task.

## When To Start

After Task 007 lands. This is the closing task of ADR 0038.
