# ADR 0025 Review Checklist

## Reviewed Artifact

Use this checklist for ADR 0025 implementation diffs and final review.

## Pass / Fail

- Status: Reviewed through Task 009.
- Reviewer: Codex
- Date: 2026-05-01
- Review: ADR 0025 implementation slices 001-009 pass automated verification.

## Architectural Invariants

- [x] Screens do not introduce raw colors, inline icon SVG, or string glyphs
      for reusable visual roles.
- [x] New screen code does not call `theme::color::*`, `theme::badges`, or
      `theme::glyphs`.
- [x] Semantic icons are requested through `ui::icons`.
- [x] Brand/protocol icon colors live inside the icon catalog.
- [x] Reusable button/action styling flows through named control style roles.
- [x] `ControlStyle` maps to `ui::primitives::Button`; it does not create a
      third button vocabulary beside the native primitive and
      `gpui_component::Button`.
- [x] Remaining direct `gpui_component::Button` styling in screens is explicitly
      marked with `CONTROL-COMPAT(reason): ...` compatibility debt.
- [x] Architecture tests reject unmarked direct screen-level
      `gpui_component::Button` usage.
- [x] `ActionButton` uses the shared control-style boundary.
- [x] Entity/status/provenance badges use typed roles, not string-keyed color
      maps.
- [x] General status message color and glyph semantics resolve through
      `StatusRole` exported by `ui::composites`, not through `ui::style`.
- [x] Color is not the sole indicator for destructive, success, warning, diff,
      disabled, or pending states.
- [x] Runtime visual changes flow through `theme_bridge` / `Environment`.
- [x] `theme_bridge::install_theme` takes `ThemeProfile`.
- [x] Design-system primitives and composites resolve default colors through
      the active `ThemeProfile`.
- [x] `theme::glyphs` does not exist.
- [x] Fixed layout geometry lives in `ui::layouts`, not `ui::style::layout`.
- [x] View-models, application, config, and core service/domain modules do not
      import UI modules.

## Slice-Specific Checks

- [x] Theme-profile contract preserves current dark default behavior.
- [x] High-contrast profile tests exist before high contrast is exposed.
- [x] High-contrast profiles are visually distinct from base Dark/Light.
- [x] Architecture gates ratchet deprecated helper usage without false
      positives that block planned migrations.
- [x] Brand/protocol icon colors have non-text contrast coverage.
- [x] Icon migration preserves size, alignment, colors, and click targets.
- [x] Control-style migration preserves labels, behavior, disabled states, and
      focus affordances.
- [x] Control-style roles satisfy the admission rule: at least two unrelated
      call sites, or a state/contrast requirement that generic chains cannot
      express.
- [x] Screen button sweep final report includes the inventory and disposition of
      direct `gpui_component::Button` chains.
- [x] Badge migration preserves label meaning and contrast.
- [x] Badge roles cover all current `EntityKind` variants: feed, track, artist,
      publisher, release, recording, playlist, and generic.
- [x] Provenance/diff roles resolve color plus non-color cue together.
- [x] Runtime profile selection exposes only tested profiles.
- [x] Runtime profile selection exposes high-contrast profiles only after
      automated contrast tests and manual visual smoke.
- [x] `ThemeProfile::System` follows GPUI window appearance before it is
      exposed.
- [x] Phase 6 reduces `theme.rs` to documented layout constants only or removes
      it.
- [x] Layout-boundary architecture tests reject reintroducing
      `ui::style::layout`.
- [x] Status-role architecture tests reject reintroducing `StatusRole` or
      `style::color::status_*` in `ui::style`.

## Tests And Verification

- [x] `cargo fmt -- --check` passed.
- [x] `cargo check` passed.
- [x] `cargo test --test architecture_tests` passed.
- [x] relevant contrast tests passed.
- [x] high-contrast profile contrast tests passed before exposure.
- [x] relevant focused unit tests passed.
- [x] pure control role mapping tests passed.
- [x] `cargo clippy --lib --tests -- -D warnings` passed.
- [x] Manual visual smoke completed for Library, Discover, Settings, and
      playback controls when the slice changes visible UI.
- [x] Manual visual smoke completed for High Contrast Dark and High Contrast
      Light before exposing them in Settings.

## Required Fixes

- None recorded.

## Manual Visual Smoke

- Completed: 2026-05-01.
- Library dark profile: tab selection, search controls, playlist/library tree,
  empty detail pane, and playback controls rendered with consistent contrast.
- Library split pane: sidebar divider dragged wider and the layout remained
  stable.
- Discover dark profile: recent-feed grid, filter pills, search controls, and
  empty-state pane rendered without obvious contrast or overlap regressions.
- Settings dark profile: input rows, scale controls, theme controls, save
  action, and cached-files section rendered consistently.
- Settings light profile: runtime Light toggle repainted the window through
  `theme_bridge`; controls remained readable. Dark was restored without saving.
- Playback controls stayed visible and readable in both dark and light smoke
  passes.
- High Contrast Dark and High Contrast Light: Library, Discover, Settings, and
  playback controls rendered without blocking contrast, layout, text-overlap,
  or repaint issues.
- Settings selector: System, Dark, Light, High Contrast Dark, and High Contrast
  Light fit in the existing theme control; switching from High Contrast Dark to
  High Contrast Light repainted through `theme_bridge`.

## Optional Improvements

- Decide whether `src/media` is presentation infrastructure or a reusable
  non-UI media layer before adding it to the core non-UI architecture audit.

## Merge Recommendation

- Mergeable through Task 007.
