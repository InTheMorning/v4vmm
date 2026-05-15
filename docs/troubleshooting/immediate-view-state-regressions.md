# Immediate View-State Regressions

## Purpose

Prevent UI state from becoming correct only after leaving and returning to a
view.

## Prohibited Fix

Do not rely on navigation, tab changes, playlist switches, or full actor
respawns to make state changes visible. If a mutation changes the current
surface, the visible view model or actor cache must be invalidated or primed in
the same command-success path.

## Required Mitigation

- Identify the cache that owns the stale display row: page actor, detail frame,
  sidebar tree, queue projection, or search result snapshot.
- Publish or send the narrowest invalidation that reaches the currently mounted
  owner.
- If preserving a warm actor, prime it with fresh rows before the async refresh
  so the current viewport updates without a placeholder-only detour.
- Add a regression guard that exercises the same-view mutation path.

## Current Guard

`PagedTrackListMsg::PrimeRows` lets Library reselect or refresh a playlist
without replacing the actor while still replacing stale cached row bodies. The
regression test `prime_rows_replaces_cached_body_for_same_playlist_refresh`
guards the case where a track becomes unavailable while its playlist remains
mounted.
