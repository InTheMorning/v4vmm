# ADR 0013: Shared Discover Track Row Module

## Status

Accepted - 2026-04-25.

## Context

The staged plan in `docs/plans/unify-discover-library-views.md` extracts shared view models and shared feed/artist renderers before reusing the same track-row UI in Discover and Library. Stage 4 requires moving the Discover track row into a dedicated module without changing current behavior.

## Decision

Create `src/ui_track.rs` as the shared track-row renderer module. Keep `search.rs` as the Discover integration layer by dispatching its existing `render_track_row` function into `ui_track::render_track_row` with an explicit `TrackRowMode::Discover`.

Expose only the helper functions and `SearchApp` methods that the shared row needs for current Discover behavior: inspector navigation, per-track download/remove actions, add-to-playlist popup rendering, and play-button rendering.

## Consequences

- Discover track rows move behind a shared renderer without changing operator-visible behavior.
- Future Library reuse can extend `TrackRowMode` instead of duplicating the row structure again.
- `search.rs` remains the owner of Discover-specific state mutations and action wiring.
