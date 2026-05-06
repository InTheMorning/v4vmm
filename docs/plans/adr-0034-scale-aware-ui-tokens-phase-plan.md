# ADR 0034 Scale-Aware UI Tokens - Phase Plan

## Status

Proposed - 2026-05-02.

## Goal

Make the app's shared UI layer obey the `ui_scale` setting consistently and in
the Apple HIG spirit: readable text, coherent spacing, usable controls, compact
but comfortable popovers, and regression guards that prevent unscaled shared UI
from returning.

## Non-Goals

- No new playlist or playback feature behavior.
- No palette, theme-profile, or accent redesign.
- No broad screen rewrite beyond call sites needed to remove local scale
  compensation.
- No automated GUI pointer scripting for visual verification; ask for
  screenshots.

## Current State

The token system already exposes scale-aware accessors:

- `Spacing::scaled(cx)`
- `Radius::scaled(cx)`
- `FontSize::scaled(cx)`
- `Size::scaled(cx)`

But several shared render paths use fixed base values:

- `src/ui/primitives/surface.rs`: padding and radius use `.px()`.
- `src/ui/primitives/button.rs`: height, padding, radius, font size, and gap
  use fixed values.
- `src/ui/primitives/label.rs`: text size uses `.px()`.
- `src/ui/primitives/multiline_text.rs`: text size and line height are fixed.
- `src/ui/icons.rs`: icon sizes use fixed values.
- `src/ui/composites/playlist_popover.rs`: menu width, max height, gaps,
  divider margins, empty-state padding, and caption text use fixed values.

## Target State

- Shared primitives scale themselves. Callers do not need to know whether a
  control's internal padding, text, icon, radius, or height needs adjustment.
- Playlist popovers inherit scaled surface/button/label behavior and use
  scaled local menu dimensions and gaps where those dimensions are
  user-facing.
- Architecture tests prevent new unscaled token usage in shared UI render
  paths unless explicitly allowlisted.
- The review checklist records screenshot-based visual smoke for the affected
  surfaces at multiple UI scale settings.

## Affected Modules

- `src/ui/tokens.rs`
- `src/ui/icons.rs`
- `src/ui/primitives/surface.rs`
- `src/ui/primitives/button.rs`
- `src/ui/primitives/label.rs`
- `src/ui/primitives/multiline_text.rs`
- `src/ui/primitives/popover.rs`
- `src/ui/composites/playlist_popover.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0034-review-checklist.md`

## Proposed Sequence

### Task 001 - Scale Shared Primitives

Convert `Surface`, `Button`, `Label`, `MultilineText`, and `Icon` so they
resolve user-facing dimensions through `.scaled(cx)` inside render methods.
Keep legitimate fixed dimensions documented and minimal.

This task must land first because every later popover/layout fix should inherit
primitive behavior rather than patching around it.

### Task 002 - Scale Playlist Popover Local Layout

Update `AddToPlaylistPopover` local dimensions that are not owned by the
primitive layer: menu width, max height, list gap, divider margin, empty-state
padding, input/button inset wrappers, and caption text.

Do not alter playlist semantics. This is a layout/scale task only.

### Task 003 - Add Scale Regression Guards

Add architecture tests that scan shared UI render paths for token `.px()` calls
on `Spacing`, `Radius`, `FontSize`, `Size`, and icon-size roles. Use a narrow
allowlist for legitimate fixed cases with comments that explain why they do
not represent user-facing adaptable layout.

This task should also update ADR 0033's enforcing-test list if the new guard
is a permanent governance rule.

### Task 004 - Visual Smoke and Readiness Gate

Run checks and ask the user for screenshots instead of using pointer
automation. Required surfaces:

- Library release detail add-to-playlist popover at medium scale.
- The same popover at a smaller or larger scale.
- Discovery recents grid after primitive scaling, to ensure titles/subtitles
  still fit.
- Now-playing chrome after primitive scaling, to ensure transport controls and
  title still fit the header band.

Record pass/fail in `docs/reviews/adr-0034-review-checklist.md`.

## Schema/API Implications

None.

## Risk Areas

- Scaling button height and padding can affect dense row alignment.
- Scaling menu width can cause popovers to cover more content at large scale.
- Scaling icons may reveal mismatches between icon roles and surrounding text
  roles.
- Changing line height in `MultilineText` can alter metadata panel density.
- Architecture tests can be too broad if they do not distinguish token
  definitions/tests from render paths.

## Test Strategy

Each implementation task must run:

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
git diff --check
```

Task-specific focused tests should be added where possible. The final gate
must run full `cargo test`.

Visual verification is screenshot-based. Ask the user for screenshots of the
named surfaces; do not use `xdotool` or coordinate automation.

## Rollback Strategy

Each task is independently reversible:

- Task 001 can revert primitive scaling if it causes broad layout breakage.
- Task 002 can revert popover-local scaling without undoing primitive scaling.
- Task 003 can temporarily relax allowlist entries if a legitimate fixed
  dimension needs a follow-up design decision.
- Task 004 is documentation-only and can be amended with corrected visual
  evidence.

Do not ship richer playlist/playback feature work on top of a failed Task 004
gate.
