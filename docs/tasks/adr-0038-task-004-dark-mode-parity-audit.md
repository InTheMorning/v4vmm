# ADR 0038 Task 004: HIG Dark-Mode Parity Audit

## Status

In progress (2026-05-04). Token resolution for `src/ui/style.rs` is
complete and guarded; visual smoke capture is the remaining blocker.

## Goal

Confirm every composite resolves through the theme bridge in both
themes. Eliminate raw `rgb(0x…)`/`Rgba` literals outside the token
layer. Capture light + dark visual smoke for every main surface.

## Inventory

Verified 2026-05-04:

- `src/ui/style.rs` previously held four raw `gpui::rgb(0x…)` literals
  for ID3 frame chips. Folded into the token layer: new
  `SemanticColor::Id3FrameV22 / V23Only / V24Only / Unknown` tokens
  with light, dark, high-contrast-light, and high-contrast-dark
  values. The four `color::id3_frame_*` helpers now resolve through
  `role(SemanticColor::…)`, identical to the rest of the file.
- New architecture test
  `ui_style_resolves_colors_through_token_layer` scans `src/ui/style.rs`
  for `gpui::rgb(` or `rgb(0x` literals and rejects them, replacing
  the prior `SCREEN_FILES`-only coverage gap for this file.
- `ui_components_do_not_bypass_theme_profile_resolution` continues to
  guard primitives and composites against `Appearance::current(cx)`
  shortcuts.
- Brand colors in `src/ui/icons.rs` (`Rss`, `Nostr`) intentionally use
  raw `gpui::rgb(...)` literals: brand identity is appearance-invariant
  and so the token system is not the right home. Out of scope for this
  task.

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

- [x] `src/ui/style.rs` resolves all colors through the token layer;
      raw `rgb(...)` literals are gone. Module-level docs declare the
      file token-resolved.
- [x] The raw-color guard covers `src/ui/style.rs`
      (`ui_style_resolves_colors_through_token_layer`).
- [ ] Light + dark screenshot pairs filed for every surface in the
      inventory. Blocked: capturing app screenshots from the CLI
      sandbox is not currently feasible. Filed as a deterministic
      capture follow-up alongside the Task 001 visual-proof caveat.
- [x] A coverage table lives in the review checklist.

## When To Start

After Task 002 lands (composite contracts settled, so theme resolution
is unambiguous per composite). Can run in parallel with Task 005.
