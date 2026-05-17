# ADR 0051 — workspace pane width persistence phase plan

## Goal

Persist the `ContentList` / Queue split width across launches without changing
the workspace layout model, `SplitPane`, or visible resize behavior.

## Non-Goals

- No new UI for layout reset.
- No per-monitor, per-window, or per-tab layout persistence.
- No changes to frame navigation, search, queue, or library rendering.
- No persistence inside `SplitPane`; composites remain stateless presentation.

## Current State

`TopApp::content_pane_width` initializes from
`layout::CONTENT_PANE_DEFAULT_WIDTH`. Dragging the workspace divider updates the
field in memory through `src/app/resize.rs`, but quitting the app loses the
value.

Existing config persistence already preserves unrelated TOML values and stores
the ADR 0046 frame layout under `[workspace_layout]`.

## Target State

Add a forward-compatible `[workspace.layout]` TOML section:

```toml
[workspace.layout]
content_pane_width = 1024.0
```

`config::Config` exposes parsed workspace layout preferences. App bootstrap
passes them to `TopApp`, `TopApp` clamps the width to the current
`CONTENT_PANE_MIN_WIDTH..=CONTENT_PANE_MAX_WIDTH` range, and
`end_content_pane_resize` saves the final width once per completed drag.

## Phase

1. Add config structs, parsing, saving, and unit tests for workspace layout
   preferences.
2. Wire bootstrap and `TopApp` initialization to restore the clamped width.
3. Persist on resize end, not resize move.
4. Add an architecture guard for ADR 0051 ownership boundaries.

## Risks

- Malformed nested TOML could break config load if not deserialized
  defensively.
- Writing the full config table could accidentally drop unrelated user keys.
- Persisting during drag move would create noisy disk writes.

## Test Strategy

- Config unit tests for missing, valid, malformed, clamped, and save-preserve
  paths.
- Architecture test for config/app/resize ownership.
- Existing compile, lib, architecture, and clippy gates.

## Rollback

Revert the single ADR 0051 commit. Older configs remain valid because the new
section is optional and additive.
