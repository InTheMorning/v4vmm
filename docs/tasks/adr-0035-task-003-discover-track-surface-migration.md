# ADR 0035 Task 003: Discover Track Surface Migration

## Goal

Route Discover track rows, inspector-pane track surface, and full-detail track
surface through the `TrackDetailVm` family and shared composites while
preserving Discover-specific links, actions, lazy sections, and search
navigation.

## Files to Inspect

- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `docs/tasks/adr-0035-task-003-discover-track-surface-migration.md`
- `src/search.rs`
- `src/ui_track.rs`
- `src/ui_entity.rs`
- `src/view_models/track_detail.rs`
- `src/ui/composites/track_detail_surface.rs`
- `src/ui/composites/track_inspector_pane.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/track_header.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/search.rs`
- `src/ui_track.rs` only for Discover row call sites
- `src/ui_entity.rs` only for Discover entity/row adapters
- `src/view_models/track_detail.rs` only for missing display facts
- `tests/architecture_tests.rs` if a Discover-specific guard is added
- `docs/reviews/adr-0035-review-checklist.md`

## Do Not Touch

- `src/library.rs`
- Backend, schema, services, playlist behavior, playback behavior
- MusicBrainz or ID3 write logic

## Constraints

- Do not remove Discover-specific affordances.
- Do not rebuild the track row, track detail header, inspector pane, or summary
  grid in `search.rs`, `src/ui_track.rs`, or `src/ui_entity.rs`.
- Screen code may wire callbacks, resolve artwork, pass the UI-layer artwork
  input, and provide typed slot values only.
- Any missing fallback label belongs in `TrackDetailVm`.
- Keep `render_track_header_subtitle` only if it becomes a slot builder; it
  must not own common header layout.
- Discover must pass `TrackDetailLoadState` rather than drawing its own track
  loading or missing state.
- This task migrates *every* Discover call site of the legacy `TrackRow`
  constructor introduced in Task 002 to the `TrackRowVm` constructor. Any
  Discover call site left on the legacy signature blocks Task 005 from
  deleting it.

## Implementation Steps

1. In Discover track row call sites, construct `TrackDetailVm`/`TrackRowVm`
   using the Discover context and pass it to `TrackRow`.
2. In `render_discover_track_inspector`, construct `TrackDetailVm` using the
   Discover context and pass it through `TrackInspectorPane`.
3. Route any Discover full-detail track surface through `TrackDetailSurface`.
4. Resolve artwork to `Option<Arc<Image>>` in screen code and pass it as the
   composite artwork input, not through the GPUI-free VM slot contract.
5. Build Discover-specific feed/audio/Nostr/RSS links as `ExternalLinkItem`
   slot values.
6. Pass existing actions as typed primary action values.
7. Pass description, contributors, value routes, back navigation, and lazy
   sections through typed surface slots.
8. Delete or shrink screen-local row/header/summary/loading construction.
9. Add/update tests that prove Discover no longer rebuilds track surface labels
   or row chrome locally.
10. Update the review checklist.

## Acceptance Criteria

- Discover track rows use `TrackRowVm` and `TrackRow`.
- Discover track inspector uses `TrackInspectorPane` and `TrackDetailSurface`.
- Discover full-detail track surface, if present, uses `TrackDetailSurface`.
- Discover still shows feed link, play action, Nostr/RSS affordances when
  available, action row, description, contributors, and value routes.
- Discover screen code no longer owns common track row chrome, track summary
  labels, or loading/missing track surface rendering.
- Architecture tests pass.
- **Per-task visual smoke**: user-provided screenshots of the Discover row
  list, Discover inspector pane, and (if present) Discover full-detail track
  surface are captured and recorded in the review checklist before this task
  is marked complete. Regressions caught here belong to this task, not
  Task 005. If the user is not available to provide screenshots before the
  next task starts, log explicit residual visual risk in the review checklist
  rather than implicitly deferring to Task 005.

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
- `docs/tasks/adr-0035-task-003-discover-track-surface-migration.md`
- `src/search.rs`
- `src/ui_track.rs`
- `src/ui_entity.rs`
- `src/view_models/track_detail.rs`
- `src/ui/composites/track_detail_surface.rs`
- `src/ui/composites/track_inspector_pane.rs`
- `src/ui/composites/track_row.rs`

Goal:
- Migrate Discover track rows, inspector pane, and full-detail track surface to
  the shared track surface family.

Constraints:
- Preserve Discover-specific behavior.
- Screen code wires callbacks, resolves artwork, passes the UI-layer artwork
  input, and passes typed slots only.
- No Library changes.
- No backend/service changes.

Do not touch:
- `src/library.rs`
- Backend/schema/service files.
- Playlist/playback behavior.

Acceptance criteria:
- Discover uses `TrackRow`, `TrackInspectorPane`, and `TrackDetailSurface`
  through the `TrackDetailVm` family.
- Common track detail summary labels are VM-owned.
- Discover track row chrome, inspector chrome, loading/missing state, and
  artwork display are no longer screen-local.
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

- If Discover loses a visible link/action/section, stop and add a typed slot
  rather than dropping the feature.
- If a shared label needs a different context-specific name, update the VM
  context contract instead of hardcoding it in `search.rs`.
