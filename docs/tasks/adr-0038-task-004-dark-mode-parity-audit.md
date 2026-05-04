# ADR 0038 Task 004: HIG Dark-Mode Parity Audit

## Status

Completed (2026-05-04). Token resolution for `src/ui/style.rs` is
complete and guarded. Light and dark surface verification was completed
with transient `/tmp` captures after operator navigation; no screenshot
artifacts are retained or committed.

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

## Changed Files

- `src/ui/style.rs` — ID3 frame colors now resolve through semantic
  tokens.
- `src/ui/tokens.rs` and `src/ui/theme_profiles.rs` — semantic ID3
  frame color roles added for light, dark, high-contrast-light, and
  high-contrast-dark profiles.
- `tests/architecture_tests.rs` — raw-color guard now includes
  `src/ui/style.rs`.
- `src/view_models/search.rs` — Discover now owns the reset-to-recents
  pane state transition and the `Recent Feeds` command display.
- `src/search.rs` — Discover wires the VM-owned `Recent Feeds` command
  so recent-feed tiles are reachable again after a search.
- `docs/reviews/adr-0038-review-checklist.md` — visual smoke ledger
  records transient verification evidence without screenshot artifacts.

## Resolved Questions

1. **Is `src/ui/style.rs` legitimate token-layer code?** Yes. It now
   resolves through `SemanticColor` roles and is guarded against raw
   `rgb(...)` literals.
2. **Surface coverage for visual smoke.** The HIG audit covered:
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

   Captures were transient and operator-navigated per review policy.
3. **Material/elevation parity.** No task-blocking material or elevation
   divergence was found in the inspected light/dark states.
4. **Coordination with ADR 0034 (scale-aware tokens).** Tokens resolve
   through scale; this task kept theme resolution orthogonal.

## Constraints

- No palette redesign. Whatever colors exist today must work in both
  themes; if they don't, file a follow-up. This task is audit + raw-rgb
  cleanup, not design.
- Visual smoke is required for every surface in the inventory in both
  themes. Per operator instruction, proof is recorded in the review
  ledger and transient `/tmp` captures are not retained in git.
- Don't touch `screens_do_not_add_unapproved_hardcoded_dark_defaults`
  baselines without a separate task.

## Definition of Done

- [x] `src/ui/style.rs` resolves all colors through the token layer;
      raw `rgb(...)` literals are gone. Module-level docs declare the
      file token-resolved.
- [x] The raw-color guard covers `src/ui/style.rs`
      (`ui_style_resolves_colors_through_token_layer`).
- [x] Light + dark visual verification completed for every surface in
      the inventory via operator-navigated, transient `/tmp` captures.
      No screenshot files were retained or committed.
- [x] A coverage table lives in the review checklist.

## When To Start

After Task 002 lands (composite contracts settled, so theme resolution
is unambiguous per composite). Can run in parallel with Task 005.
