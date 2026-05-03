# ADR 0038 Task 004: HIG Dark-Mode Parity Audit (Stub)

## Status

Stub. May run in parallel with Task 005 after Task 002 lands.

## Goal

Confirm every composite resolves through the theme bridge in both
themes. Eliminate raw `rgb(0x…)`/`Rgba` literals outside the token
layer. Capture light + dark visual smoke for every main surface.

## Inventory

Verified 2026-05-03:

- `src/ui/style.rs:105-114` contains raw `gpui::rgb(0x…)` literals.
  Either legitimize as token-layer code, fold into `tokens.rs`, or
  delete if unused.
- Existing guard
  `screens_do_not_reintroduce_raw_color_or_numeric_px_literals` covers
  `SCREEN_FILES` only — `src/ui/style.rs` is exempt today.
- Existing guard `ui_components_do_not_bypass_theme_profile_resolution`
  exists; verify scope.

## Files Likely To Change

- `src/ui/style.rs` — clean up or absorb into `tokens.rs`/
  `theme_profiles.rs`.
- `src/ui/tokens.rs` — possibly new semantic colors if `style.rs`
  defines values not yet tokenized.
- `tests/architecture_tests.rs` — tighten the raw-color guard to
  include `src/ui/style.rs` (or remove the file and skip).
- `docs/reviews/screenshots/` — light + dark pairs for every main
  surface.

## Open Questions

1. **Is `src/ui/style.rs` legitimate token-layer code?** If so, document
   it in the file's module-level comment and add it to the
   raw-color-allowed set. If not, delete and migrate callers to
   `tokens::SemanticColor`.
2. **Surface coverage for visual smoke.** The full main-surface list
   for HIG audit:
   - Library list
   - Library inspector
   - Library track detail
   - Discover list
   - Discover inspector
   - Discover track detail
   - Release detail (Library + Discover)
   - Playlist popover (Library)
   - Now-playing bar
   - Search results
   - Recent feed tiles
   - Sidebar / selection rows

   Confirm the list before capturing.
3. **Material/elevation parity.** HIG materials (translucency, vibrancy)
   may differ between themes. Note any divergence rather than fixing
   in this task.
4. **Coordination with ADR 0034 (scale-aware tokens).** Tokens already
   resolve through scale; theme resolution is orthogonal. Don't
   conflate.

## Constraints

- No palette redesign. Whatever colors exist today must work in both
  themes; if they don't, file a follow-up. This task is audit + raw-rgb
  cleanup, not design.
- Visual smoke is required for every surface in the inventory. Both
  themes. File at
  `docs/reviews/screenshots/adr-0038-{surface}-{light,dark}.png`.
- Don't touch `screens_do_not_add_unapproved_hardcoded_dark_defaults`
  baselines without a separate task.

## Definition of Done

- `src/ui/style.rs` is either gone, absorbed, or documented as a
  token-layer file allowed to hold raw colors.
- The raw-color guard covers `src/ui/style.rs` (one way or another).
- Light + dark screenshot pairs filed for every surface in the
  inventory.
- A coverage table lives in the review checklist.

## When To Start

After Task 002 lands (composite contracts settled, so theme resolution
is unambiguous per composite). Can run in parallel with Task 005.
