# ADR 0035 Task 004: Library Track Surface Migration

## Goal

Route Library track rows, inspector-pane track surface, and full-detail track
surface through the `TrackDetailVm` family and shared composites while
preserving Library's advanced metadata workflows.

## Files to Inspect

- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `docs/tasks/adr-0035-task-004-library-track-surface-migration.md`
- `src/library.rs`
- `src/ui_track.rs`
- `src/ui_entity.rs`
- `src/view_models/track_detail.rs`
- `src/ui/composites/track_detail_surface.rs`
- `src/ui/composites/track_inspector_pane.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/file_header.rs`
- `src/ui/composites/musicbrainz_panel.rs`
- `src/ui/composites/track_metadata_grid.rs`

## Files Likely to Change

- `src/library.rs`
- `src/ui_track.rs` only for Library row call sites
- `src/ui_entity.rs` only for Library entity/row adapters
- `src/view_models/track_detail.rs` only for missing display facts
- `tests/architecture_tests.rs` if Library-specific guards are added
- `docs/reviews/adr-0035-review-checklist.md`

## Do Not Touch

- `src/search.rs`
- Backend, schema, services, playlist behavior, playback behavior
- ID3 write semantics or MusicBrainz lookup semantics

## Constraints

- Preserve Library advanced workflows: compare ID3, re-read, re-download,
  MusicBrainz, staged tag edits, apply/discard, metadata grid columns.
- Do not rebuild common track row, track header, inspector pane, summary
  layout, or loading/missing track state in `library.rs`, `src/ui_track.rs`,
  or `src/ui_entity.rs`.
- Screen code may wire command callbacks, resolve artwork, pass the UI-layer
  artwork input, and provide typed slot values.
- Any missing fallback label belongs in `TrackDetailVm`.
- This task migrates *every* Library call site of the legacy `TrackRow`
  constructor introduced in Task 002 to the `TrackRowVm` constructor. Any
  Library call site left on the legacy signature blocks Task 005 from
  deleting it.

## Implementation Steps

1. In Library track row call sites, construct `TrackDetailVm`/`TrackRowVm`
   using the Library context and pass it to `TrackRow`.
2. In Library inspector-pane track surfaces, construct `TrackDetailVm` and pass
   it through `TrackInspectorPane`.
3. In `render_track_window` and `render_track_left_column`, replace the
   screen-local common header/summary composition with `TrackDetailSurface`.
4. Resolve artwork to `Option<Arc<Image>>` in screen code and pass it as the
   composite artwork input, not through the GPUI-free VM slot contract.
5. Pass `library_track_action_row` behavior as typed primary action values.
6. Pass ID3 compare/FileHeader and MusicBrainz panels as advanced panel slot
   values.
7. Keep `TrackMetadataGrid` placement consistent with the shared surface
   design.
8. Preserve pending edit and conflict behavior.
9. Delete inline title/artist/album/tag fallback literals covered by
   `TrackDetailVm` and `TrackDetailLabels`.
10. Add/update architecture tests that forbid Library from reintroducing
    screen-local track row/header/summary/inspector composition.
11. Update the review checklist.

## Acceptance Criteria

- Library track rows use `TrackRowVm` and `TrackRow`.
- Library inspector-pane track surface uses `TrackInspectorPane` and
  `TrackDetailSurface`.
- Library full-detail track surface uses `TrackDetailSurface`.
- All Library metadata workflows remain available.
- Library screen code no longer owns common track row chrome, track summary
  labels, title/artist/album/tag fallbacks, or loading/missing track surface
  rendering.
- Checks pass.
- **Per-task visual smoke**: user-provided screenshots of the Library row
  list, Library inspector pane, Library full-detail track surface, and at
  least one screen with ID3 compare / MusicBrainz / staged-edit panels open
  are captured and recorded in the review checklist before this task is
  marked complete. Regressions caught here belong to this task, not Task
  005. If the user is not available to provide screenshots before the next
  task starts, log explicit residual visual risk in the review checklist.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test track_detail
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `docs/tasks/adr-0035-task-004-library-track-surface-migration.md`
- `src/library.rs`
- `src/ui_track.rs`
- `src/ui_entity.rs`
- `src/view_models/track_detail.rs`
- `src/ui/composites/track_detail_surface.rs`
- `src/ui/composites/track_inspector_pane.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/file_header.rs`
- `src/ui/composites/musicbrainz_panel.rs`
- `src/ui/composites/track_metadata_grid.rs`

Goal:
- Migrate Library track rows, inspector pane, and full-detail track surface to
  the shared track surface family.

Constraints:
- Preserve ID3/MusicBrainz/staged edit workflows.
- Screen code wires callbacks, resolves artwork, passes the UI-layer artwork
  input, and passes typed slots only.
- No Discover changes.
- No backend/service changes.

Do not touch:
- `src/search.rs`
- Backend/schema/service files.
- Playlist/playback behavior.

Acceptance criteria:
- Library uses `TrackRow`, `TrackInspectorPane`, and `TrackDetailSurface`
  through the `TrackDetailVm` family.
- Existing advanced metadata workflows remain visible and wired.
- Library track row chrome, inspector chrome, loading/missing state, fallback
  labels, and artwork display are no longer screen-local.
- Checks pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test track_detail`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If an advanced metadata workflow cannot fit the surface slot model, stop and
  adjust the composite API rather than rebuilding a local surface.
- If staged edit behavior changes, stop and narrow the migration.
