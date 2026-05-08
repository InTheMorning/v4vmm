# ADR 0044 Playlist Drag Handle Reordering Phase Plan

## Goal

Replace playlist row up/down reorder arrows with handle-based
drag-and-drop reordering, while preserving menu-based Move Up/Move Down
fallback actions.

## Non-Goals

- No schema migration.
- No cross-playlist drag/drop.
- No multi-row drag.
- No live row shifting while dragging.
- No unrelated playlist layout redesign.

## Assumptions

- V1 starts dragging only from the handle.
- V1 commits on drop and shows an insertion line while dragging.
- Row Actions menu carries Move Up and Move Down fallback commands.
- Existing `ReorderPlaylistTrack` remains the persistence path.
- Pending paged rows can be drop-adjacent targets but are not draggable.

## Affected Modules

- `src/view_models/library.rs` for playlist row controls/display
  projection.
- `src/ui/shells/playlist.rs` for row chrome, drag handle, context
  menu, and insertion-line rendering.
- `src/ui/shells/library/playlist_detail.rs` for wiring drag/drop and
  menu fallback callbacks to `LibraryApp::move_playlist_track`.
- `src/ui/icons.rs` for a semantic drag-handle icon.
- `tests/architecture_tests.rs` and VM tests for ownership guards.

## Proposed Sequence

1. View-model contract migration.
   - Replace arrow-specific display fields with drag-handle display and
     reorder menu item display.
   - Keep existing play/remove display fields.
   - Update VM tests around boundary availability.

2. Shell drag/drop and menu fallback.
   - Render the drag handle.
   - Add same-playlist drag payload and insertion-line drop targets.
   - Add Move Up and Move Down row Actions menu items.
   - Keep pending rows non-draggable.

3. Wiring, guards, and visual proof.
   - Wire eager and paged playlist rows to existing reorder callbacks.
   - Update architecture tests from arrow ownership to handle/menu
     ownership.
   - Verify light/dark visuals and drag insertion feedback.

## Schema and API Implications

- No schema changes.
- No external API changes.
- No new application command is required.
- Internal UI slots change from separate `on_move_up` and
  `on_move_down` row buttons to drag/drop reorder and menu fallback
  callbacks.

## Risk Areas

- Drag/drop may conflict with existing row selection if attached outside
  the handle.
- Paged playlist rows may have pending slots where no track payload is
  available.
- Existing architecture guards explicitly reference arrow ids and glyphs.
- The insertion index must be converted to `from -> to` without
  off-by-one errors.

## Test Strategy

- VM tests for drag handle ids, a11y labels, menu item ids, labels, and
  boundary disabled states.
- Unit or integration tests for target-index conversion.
- Existing DB/application reorder tests remain the persistence proof.
- Architecture tests ensure no playlist up/down arrow glyphs or ids are
  rendered by the shell.
- Visual smoke in light and dark themes for handle, Actions menu,
  unavailable row, and insertion line.

## Rollback Strategy

- Task 001 is a pure contract change and can be reverted before shell
  adoption.
- Task 002 is the main UI behavior change; reverting it restores the
  previous arrow UI if Task 001 is also reverted.
- Task 003 contains guards and visual review updates and can be adjusted
  independently if guard wording is too broad.
