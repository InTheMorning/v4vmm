# ADR 0034 Review Checklist

## Reviewed Artifacts

- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `docs/tasks/adr-0034-task-001-scale-shared-primitives.md`
- `docs/tasks/adr-0034-task-002-scale-playlist-popover-layout.md`
- `docs/tasks/adr-0034-task-003-scale-regression-guards.md`
- `docs/tasks/adr-0034-task-004-visual-smoke-and-readiness-gate.md`

## Gate Status

Status: Proceed - 2026-05-02.

Tasks 001-004 are implemented. Richer playlist/playback feature work that
depends on popover, button, label, icon, or surface scaling may proceed
through the ADR 0033/0034 shared primitive, composite, token, and regression
guard boundaries.

## Structural Review Questions

- Do shared primitives resolve user-facing dimensions through scaled tokens?
- Did the implementation avoid screen-local scale compensation?
- Does `AddToPlaylistPopover` remain the single owner of playlist popover
  layout and behavior?
- Does `+ New Playlist` remain present wherever create mode is wired?
- Are all remaining unscaled shared UI dimensions allowlisted with a specific
  non-user-facing reason?
- Did visual smoke use user-provided screenshots rather than pointer
  automation?

## Task Results

| Task | Status | Required Evidence | Notes |
|---|---|---|---|
| Task 001 scale shared primitives | Pass | `Surface`, `Button`, `Label`, `MultilineText`, `SectionHeader`, `DetailHeader`, and `Icon` render through scaled token paths; checks green | Added button height tokens so control height scales through the token layer instead of hidden fixed pixels. |
| Task 002 scale playlist popover layout | Pass | `AddToPlaylistPopover` local width, max height, gaps, divider margins, empty-state padding, caption text, and create-mode wrappers use scaled tokens; playlist popover ownership tests green | Popover semantics and `+ New Playlist` wiring are unchanged. |
| Task 003 scale regression guards | Pass | `shared_ui_render_paths_use_scale_aware_tokens`, `shared_header_badges_use_intrinsic_flex_rows`; ADR 0033 enforcing-test list updated; ADR 0034 consequences updated | The only allowlisted unscaled token `.px()` is the base `IconSize::Transport` value used by `IconSize::scaled`. Header badges are guarded against block-width wrappers. |
| Task 004 visual smoke and readiness gate | Pass | Full checks are green; user screenshots reviewed for playlist popovers, Discovery recents, now-playing chrome, and Library/Discover track headers | Initial screenshot review found a stretched track badge blocker; fixed with intrinsic shared header badge rows and verified by follow-up screenshots. |

## Visual Smoke

- Library playlist popover at medium scale: pass. User screenshot shows the
  popover anchored to the release-level trigger with `bob`, `darcy`, and
  `+ New Playlist`.
- Library/Discovery track playlist popover at alternate scale: pass for
  popover contents. User screenshots show `bob`, `darcy`, and
  `+ New Playlist`; they also exposed a stretched `track` badge in the shared
  header.
- Discovery recents grid: pass. User screenshot shows visible feed titles and
  publishers with stable grid spacing.
- Now-playing chrome: pass. User screenshot shows title and transport controls
  fitting the header band.
- Shared track/detail header badge: pass. Initial screenshots showed the
  `track` badge stretching into a full-width bar; fixed by wrapping header
  badges in intrinsic flex rows, guarded by
  `shared_header_badges_use_intrinsic_flex_rows`, and verified by follow-up
  Library and Discover screenshots.

## Verification

- `cargo fmt -- --check`: Green.
- `cargo check`: Green.
- `cargo test --test architecture_tests`: Green, 39 passed.
- `cargo test playlist_popover`: Green, 3 relevant architecture tests passed.
- `cargo test`: Green, 490 unit tests, 39 architecture tests, and doc tests
  passed.
- `cargo clippy -- -D warnings`: Green.
- `git diff --check`: Green.

## Merge Recommendation Template

Use this for each task review:

```text
Status: Pass / Fail

Required fixes:
- ...

Optional improvements:
- ...

Architectural drift:
- ...

Missing tests or visual proof:
- ...

Feature-readiness impact:
- Proceed / Proceed with constraints / Do not proceed
```
