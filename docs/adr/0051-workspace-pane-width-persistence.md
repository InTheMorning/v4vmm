# ADR 0051: Workspace pane width persistence

## Status

Accepted - 2026-05-17. Implemented.

## Context

The ADR-0048 implementation ships fluid resize between the ContentList and
Queue frames via `SplitPane` (`src/ui/composites/split_pane.rs`). The
divider width is held in `TopApp::content_pane_width` and updated through
`begin_content_pane_resize` / `resize_content_pane` /
`end_content_pane_resize` handlers in `src/app.rs:1956-1978`.

The width is **in-memory only**. After the user quits and relaunches the
app, the divider resets to `layout::CONTENT_PANE_DEFAULT_WIDTH`. The user
re-drags every session.

The plan and review both flagged this as a known v1 gap, not a regression.
ADR 0046 + 0048 left persistence to a follow-up. This ADR is that
follow-up placeholder.

## Decision

Persist `content_pane_width` (and any future per-workspace layout prefs)
to `config.toml` under a new section. Restore on app boot.

Proposed schema (treat as starting point; refine when picking up):

```toml
[workspace.layout]
content_pane_width = 1024
# future: queue_pane_width, sidebar_width, etc.
```

The VM layer owns the snapshot; `src/config.rs` (or wherever
`config::Config` lives) gains a `WorkspaceLayoutPrefs` struct. App boot
calls `TopApp::with_content_pane_width(prefs.content_pane_width)`. The
`end_content_pane_resize` handler writes back to disk (debounced or
on-end only — debounce is fine since drags fire many move events).

Width is **clamped** on load to `[CONTENT_PANE_MIN_WIDTH,
CONTENT_PANE_MAX_WIDTH]` to handle config files written by a future
app version with different bounds.

## Invariants

- Persisted width is clamped at load time; out-of-range values do not crash
  the app.
- Resize writes happen on `end_content_pane_resize`, not on every move
  event.
- Missing config section falls back to `CONTENT_PANE_DEFAULT_WIDTH` without
  warning.
- Layout prefs section is forward-compatible: unknown keys do not cause
  parse failure.

## Non-Goals

- No multi-window layout persistence.
- No per-screen layout persistence (e.g., remember different widths per
  monitor size).
- No persisted resize history / undo stack.
- No UI for explicit layout reset (a user wanting default re-drags or
  clears the section by hand).

## Alternatives Considered

- **SQLite `kv_state` table.** Rejected. The user's layout prefs belong
  next to other config values (theme, scale), not in the music-data DB.
- **Per-pane width in `SplitPane` itself, persisted by the composite.**
  Rejected. Composites do not own persistence (ADR 0042). VM owns
  snapshot; app boot wires it.
- **Persist every layout prop (queue width, sidebar width, frame visibility).**
  Out of scope here; start with content pane only. The TOML section is
  forward-compatible so later additions don't need a schema migration.

## Consequences

Positive:
- Operator's preferred layout survives relaunch.
- Section is forward-compatible for the next layout-pref ADR.

Negative / risks:
- Tests must mock config-write path for the resize handler. Mitigation:
  trait the persistence boundary so tests can use an in-memory fake.
- A user with broken config TOML loses just the layout section (clamp +
  fallback to default).

## References

- ADR 0046 — workspace frame architecture
- ADR 0048 — ContentList frame breadcrumb search
- `docs/reviews/adr-0047-0048-0049-implementation-review.md` (P3 finding)
- `src/ui/composites/split_pane.rs`
- `src/app.rs:1956-1978`
