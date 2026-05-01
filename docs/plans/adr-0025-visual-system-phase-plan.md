# ADR 0025 Visual-System Phase Plan

## Goal

Make theme, icon, and reusable control-style changes local to the design-system
layer instead of requiring screen edits. Preserve the current product behavior
and the tokens -> primitives -> composites -> screens architecture from ADR
0023.

## Non-goals

- No visual redesign.
- No GPUI replacement.
- No database, service, command, query, or event changes.
- No arbitrary user theme editor.
- No broad screen-directory split.

## Current State

- `tokens.rs` owns semantic dimensions and light/dark color resolution.
- `theme_bridge.rs` installs those values into `gpui_component`.
- `theme.rs` is still used as a dark-only compatibility shim.
- Screens still use direct `Button::new(...)` plus style chains in many
  reusable contexts.
- `ui::primitives::Button` exists but is dormant; ADR 0025 makes
  `ControlStyle` its screen-facing role layer instead of creating another
  button vocabulary.
- RSS/Nostr/playback/status iconography is split between inline SVG helpers,
  string glyphs, and badge emoji.
- `TagBadge`, `EntityKind`, and `ProvenanceRole` cover screen entity and
  metadata diff roles. `theme::badges` still exists only as a compatibility
  shim inside `src/ui/theme.rs`.

## Target State

- Named theme profiles resolve appearances and visual roles.
- `theme_bridge::install_theme` accepts `ThemeProfile`.
- Screens do not add new deprecated theme helper usage.
- Icons are requested by semantic role.
- Reusable buttons/actions are styled through named control roles mapped to
  `ui::primitives::Button`.
- Entity/status badges are typed.
- Runtime theme/profile changes flow through `theme_bridge` and repaint without
  screen-specific code.

## Assumptions

- The app remains GPUI-based for this plan.
- The current dark profile remains the default until runtime profile selection
  is ready.
- Existing behavior and layout should be preserved unless a task explicitly
  states otherwise.
- The first slices may leave old helpers in place while architecture tests
  prevent new usage.

## Affected Modules

- `src/ui/tokens.rs`
- `src/ui/theme.rs`
- `src/ui/theme_bridge.rs`
- `src/ui/theme_profile.rs`
- `src/ui/control_styles.rs`
- `src/ui/primitives/button.rs`
- `src/ui/composites/action_button.rs`
- `src/ui/composites/tag_badge.rs`
- `src/ui/mod.rs`
- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

### Phase 1 - theme profile contract and gates

Add the theme-profile type and architecture tests that prevent new screen-level
use of deprecated theme helpers. Change `install_theme` to take
`ThemeProfile`, update all theme-install call sites, add high-contrast contrast
tests, and ratchet the existing `theme::color`, `theme::badges`, and
`theme::glyphs` debt. Do not migrate all existing call sites yet.

### Phase 2 - icon catalog

Introduce semantic icon roles and move RSS/Nostr/playback/status icon helpers
behind the catalog.

### Phase 3 - control style roles

Introduce reusable control style roles as the public face of
`ui::primitives::Button`. Migrate `ActionButton`, add pure role mapping tests,
and define the `CONTROL-COMPAT(reason): ...` marker plus architecture-test
support for direct `gpui_component::Button` compatibility exceptions.

### Phase 3b - screen button style sweep

Absorb the repeated screen style-chain sweep in `app.rs`, `library.rs`, and
`search.rs`. Any remaining direct `gpui_component::Button` styling must be
marked with `CONTROL-COMPAT(reason): ...` and reported in the final inventory.

### Phase 4 - typed badge roles

Replace remaining `theme::badges` screen usage with typed
entity/status/provenance roles.

Status: implemented by
`docs/tasks/adr-0025-task-004-badge-role-migration.md`.

### Phase 5 - runtime profile selection

Persist selected theme profile and route changes through `theme_bridge`.
Expose settings only after major render paths no longer depend on dark-only
helpers.

### Phase 6 - retire compatibility shims

Remove or sharply narrow `theme.rs`, update docs, and harden architecture
tests. This phase has its own task packet and a measurable gate: migrated
screen files must have zero `theme::color::*`, `theme::badges`, and
`theme::glyphs` call sites before the deprecated namespace ban becomes
unconditional.

## Schema/API Implications

No database schema changes are expected. Phase 5 may extend `Config` with a
theme-profile setting. That is a config-file compatibility change, not a SQLite
migration.

## Risk Areas

- Migrating style helpers can accidentally change contrast or button affordance.
- Icon replacement can change hit target size or alignment.
- Architecture tests can become too broad if they ban legitimate compatibility
  code before replacements exist.
- `ControlStyle` can become an unreviewed collection of one-off styles. New
  roles require at least two unrelated call sites or a state/contrast rule that
  a generic chain cannot express.
- Runtime profile selection can expose incomplete light/high-contrast paths too
  early.

## Test Strategy

- `cargo test --test architecture_tests`
- existing contrast tests through `cargo test`
- focused tests for pure theme/icon/control metadata where practical
- `cargo fmt -- --check`
- `cargo check`
- `cargo clippy --lib --tests -- -D warnings`

For visual-sensitive phases, run the app and manually compare Library,
Discover, Settings, and playback controls in the current dark profile.

## Dependencies

- Task 003 must land before Task 003b because the screen sweep depends on
  `ControlStyle` and `CONTROL-COMPAT` architecture-test support.
- Task 002 should land before Task 004 if provenance/diff roles need new icon
  roles.
- Task 005 must wait until major render paths no longer depend on dark-only
  helpers.
- Task 006 must wait until Phases 1-5 are complete or explicitly deferred.

## Rollback Strategy

Each phase should preserve existing helper behavior until its call sites are
migrated. If a phase regresses UI behavior, revert the specific new design
system wrapper or relax the new architecture gate while keeping completed,
working migrations.

## Open Questions

- When should `ThemeProfile::System` start following OS appearance? Until that
  platform integration exists, it may resolve internally to the default profile
  but must not be exposed as a user-visible no-op setting.
- Should custom accent color be allowed only from a fixed palette or from an
  arbitrary color picker after contrast validation exists?
- Should icon assets remain inline SVG strings inside `ui::icons`, or should
  they move to asset files once the catalog exists?
