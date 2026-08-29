# ADR 0044: Playlist Drag Handle Reordering

## Status

Accepted - 2026-05-08. Implementation partial: follow-up fixes landed.
Operator visual recheck outstanding. See
`docs/reviews/adr-0044-review-checklist.md`.

## Context

Playlist detail rows currently expose visible up and down arrow buttons
for reordering tracks. The underlying application layer already supports
arbitrary `from -> to` playlist reordering through `ReorderPlaylistTrack`,
but the visible UI is a stepwise control pair.

That shape is functional but not aligned with the app's current
human-interface direction:

- playlist rows are list items, and reordering list items is more direct
  when the item itself can be moved.
- visible arrow pairs add repeated row chrome and compete with play and
  remove actions.
- the row display contract already owns reorder ids and labels, so this
  should be a structural VM/shell change, not a local button swap.

Apple HIG drag-and-drop guidance also expects alternative ways to
complete drag operations. This ADR therefore removes visible arrows, but
does not make drag the only way to reorder.

## Decision

Replace visible playlist row up/down arrow buttons with a drag handle.
Dragging starts only from the handle. Reordering uses same-container move
semantics and commits on drop.

The playlist shell shows an insertion line between rows while dragging a
playlist row over a valid target. It does not live-shift rows before
drop in v1.

Accessible alternatives remain available through row Actions menu items:

- Move Up
- Move Down
- Remove

Move Up and Move Down are disabled at the first and last positions. The
alternative commands use the existing reorder command path.

The data flow stays the same:

- VM projects drag handle ids, labels, and menu item display.
- Shell owns drag/drop chrome and insertion-line rendering.
- Library screen wires callbacks to existing `move_playlist_track`.
- Application/database reorder behavior remains unchanged.

## Alternatives Considered

- Keep arrow buttons and add drag as an extra feature. Rejected because
  it keeps duplicated reorder chrome in every row.
- Whole-row dragging. Rejected because the row body already owns
  selection/open behavior. A handle avoids gesture conflict.
- Drag-only reordering. Rejected because HIG drag-and-drop guidance asks
  for alternate ways to complete tasks.
- Live row shifting during drag. Deferred because insertion-line
  feedback is lower risk with paged playlist rows.

## Consequences

Positive:

- Reordering becomes direct and visually quieter.
- Reorder display policy remains in playlist row view models.
- Existing backend/application reorder code is reused.
- Accessibility is stronger than drag-only because menu alternatives remain.

Negative:

- Playlist shell needs drag/drop state and insertion-line rendering.
- Architecture guards that currently name arrow ids/glyphs must be
  updated to the new handle/menu contract.
- Visual smoke must cover drag feedback, which may require manual or
  scripted pointer verification.

## Invariants

- No visible up/down arrow buttons remain in playlist track rows.
- Drag starts from a handle, not the whole row body.
- Reorder ids, labels, availability, and menu item display live in
  `PlaylistTrackRowVm`.
- Reorder persistence continues through `ReorderPlaylistTrack`.
- Pending paged rows are not draggable.
- Drag feedback is an insertion line.
- Boundary moves are no-ops or disabled before dispatch.
- Visual proof is required in light and dark themes.

## Non-Goals

- No database schema changes.
- No playlist multi-select drag.
- No cross-playlist or cross-app drag/drop.
- No live row shifting during drag.
- No new global keyboard shortcuts in v1.

## Follow-Up Work

- Consider full keyboard reorder shortcuts after focus behavior for
  playlist rows is explicitly modeled.
- Consider richer drag previews after the insertion-line behavior is
  stable.
