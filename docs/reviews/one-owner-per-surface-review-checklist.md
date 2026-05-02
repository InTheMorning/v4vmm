# One Owner Per Surface Review Checklist

## Reviewed Artifacts

- `docs/plans/one-owner-per-surface-plan.md`
- `docs/tasks/one-owner-per-surface-task-001-recents-surface-ownership.md`
- `docs/tasks/one-owner-per-surface-task-002-fallback-display-accessors.md`
- `docs/tasks/one-owner-per-surface-task-003-composite-display-contract-audit.md`
- `docs/tasks/one-owner-per-surface-task-004-feature-readiness-gate.md`

## Gate Status

Status: Proceed - 2026-05-02.

Richer playlist/playback feature work may proceed through the structural
owners named in ADR 0033 and this plan. Any new feature surface still needs
to name its composite/primitive owner, VM/display contract, token/icon path,
and regression guard before implementation.

## Structural Review Questions

- Does each changed surface name exactly one composite or primitive owner?
- Does each changed surface name exactly one VM/display contract owner?
- Did the change remove, rather than relocate, screen-local fallback policy?
- Did the change use existing tokens, roles, components, and icon ownership?
- Did the same change add or strengthen a regression guard?
- Did visual smoke cover the affected user-facing surface?

## Task Results

| Task | Status | Required Evidence | Notes |
|---|---|---|---|
| Task 001 recents surface ownership | Pass | `RecentFeedTileDisplay`, `RecentFeedTile`, `discovery_recent_tiles_use_shared_composite`; user screenshot shows real title/subtitle labels in Discovery recents | Recents tile labels now come from the VM display contract and tile chrome lives in one composite. Placeholder ellipsis payloads are treated as absent text, and labels have stable tile-width slots. |
| Task 002 fallback display accessors | Pass | `LibraryTrackRowVm` display accessors, `feed_display_title`, `TrackMetadataGridVm::tag_column_label`, fallback architecture guards, ADR 0033 test-list sync | Screen-local title/artist/album/feed-title/feed-url/tag fallback policy removed from screen files. |
| Task 003 composite display-contract audit | Pass | `composite_loose_string_display_apis_are_allowlisted`; narrow allowlist documents existing generic string APIs | New shared composite string-like public APIs must be display-contract owned or explicitly reviewed. |
| Task 004 feature-readiness gate | Pass | Green checks, user-provided visual smoke for Discovery recents, Library playlist popover, and now-playing chrome | Proceed, with the constraint that future playlist/playback work must enter through the existing shared surface owners and cannot add screen-local chrome. |

## Visual Smoke

- Discovery recents: pass. User screenshot after the final recents fix shows
  visible feed title and artist/publisher labels, not placeholder ellipses.
- Library add-to-playlist popover: pass. User screenshot shows the shared
  popover anchored to the row action with existing playlists and
  `+ New Playlist`.
- Now-playing chrome: pass. User crop shows the compact top chrome with track
  title and transport controls fitting the header band.

## HIG Review Focus

- Popovers remain compact, anchored, and owned by shared popover/composite
  code. Screens only wire command callbacks.
- Buttons have consistent style/content/role and do not reintroduce bare
  leading glyph strings.
- Layout hierarchy uses tokens and shared rows/headers, not local pixel or
  color choices.
- Dense desktop views remain scannable: title, subtitle, metadata, state, and
  actions have predictable placement across Library and Discovery.

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
