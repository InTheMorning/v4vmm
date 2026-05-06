# ADR 0035 Task 002: Track Surface Composites

## Goal

Add or bind the shared composites that own track full-detail layout,
inspector-pane layout, and row layout. They consume the `TrackDetailVm`
family and accept explicit typed slots for surface-specific actions and panels.

## Files to Inspect

- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `src/view_models/track_detail.rs`
- `src/ui/composites/track_header.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/detail_grid.rs`
- `src/ui/composites/action_row.rs`
- `src/ui/composites/mod.rs`

## Files Likely to Change

- `src/ui/composites/track_detail_surface.rs`
- `src/ui/composites/track_inspector_pane.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs` only for early shared-UI boundary coverage
- `docs/reviews/adr-0035-review-checklist.md`

## Do Not Touch

- `src/library.rs`
- `src/search.rs`
- Backend, schema, services, playback, playlist behavior

## Constraints

- The composite must be backend-free and screen-free.
- The composites accept `TrackDetailVm`, `TrackRowVm`, load state, artwork,
  and typed slot values.
- Do not import `db`, `api`, `feed_service`, `playlist_service`, `library`,
  or `search`.
- Use existing primitives/composites and scaled tokens.
- Keep slots explicit: primary actions, external links, contributors, value
  routes, summary grid, description, lazy sections, advanced panels, and back
  navigation.
- Do not expose slot setters typed as `AnyElement` or `impl IntoElement`; the
  composite can render GPUI elements internally after receiving typed values.
- Screens resolve artwork to `Option<Arc<Image>>`; composites own artwork
  display and fallback rendering.
- Loading, missing, and failed states are rendered by the composite from
  `TrackDetailLoadState`.
- **Do not break the build.** `TrackRow` already has callers in
  `src/library.rs`, `src/search.rs`, `src/ui_track.rs`, and `src/ui_entity.rs`.
  This task adds the `TrackRowVm`-based constructor (e.g. `TrackRow::from_vm`
  or a new `TrackRow::new(vm: TrackRowVm)`) *additively* and keeps every
  existing constructor signature compiling. Tasks 003 and 004 migrate call
  sites to the VM constructor; Task 005 deletes the legacy constructors once
  every caller has moved. Same rule for `TrackInspectorPane`: introduce as a
  new composite without altering whatever the screens call today.
- **Slot-typing guard lands here.** The
  `track_surface_slots_are_typed` test from ADR 0035 is added in this task,
  not deferred to Task 005. The composite API is the only thing this guard
  inspects, so it can land at baseline zero as soon as the composites exist.

## Implementation Steps

1. Create `src/ui/composites/track_detail_surface.rs`.
2. Create `src/ui/composites/track_inspector_pane.rs` as the shared inspector
   frame around `TrackDetailSurface`. Note in the module-level doc comment
   that `src/ui_track.rs` (a `KNOWN_SHARED_UI_SHELL_FILES` entry) currently
   owns inspector chrome and will be drained by Tasks 003/004 then revisited
   in Task 005 — until then, the new composite and the legacy shell coexist.
3. Add a `TrackRowVm`-based constructor on `src/ui/composites/track_row.rs`
   *alongside* existing constructors. Do not remove or change the legacy
   signatures in this task.
4. Build the detail layout from `TrackHeader`, `DetailGrid`, existing
   primitives, and typed slot renderers.
5. Add builder methods for:
   - artwork
   - load state
   - back navigation
   - primary actions
   - external links
   - contributors
   - value routes
   - description
   - lazy sections
   - advanced panels
6. Export the composites from `src/ui/composites/mod.rs`.
7. Add `track_surface_slots_are_typed` to `tests/architecture_tests.rs`. The
   test inspects the public builder/constructor signatures of
   `TrackDetailSurface`, `TrackInspectorPane`, and `TrackRow` and fails when
   any slot setter is typed as `AnyElement`, `impl IntoElement`, or any
   `gpui` element trait. Baseline zero.
8. Add pure builder tests where possible.
9. Update the review checklist.

## Acceptance Criteria

- `TrackDetailSurface` compiles and is exported.
- `TrackInspectorPane` compiles and is exported.
- `TrackRow` accepts `TrackRowVm` via a new constructor *and* still compiles
  for every existing call site. `cargo build` is green for the whole repo
  after this task — no broken intermediate state.
- `track_surface_slots_are_typed` passes at baseline zero.
- Shared UI boundary tests still pass.
- No screen has adopted the new constructor or composites yet; the old call
  paths still render unchanged.
- Composites own layout, loading/empty rendering, artwork display, and fallback
  rendering, not command behavior.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test track_detail_surface
cargo test --test architecture_tests
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `docs/tasks/adr-0035-task-002-track-detail-surface-composite.md`
- `src/view_models/track_detail.rs`
- `src/ui/composites/track_header.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/track_inspector_pane.rs` if it already exists
- `src/ui/composites/detail_grid.rs`
- `src/ui/composites/action_row.rs`
- `src/ui/composites/mod.rs`

Goal:
- Add or bind the shared `TrackDetailSurface`, `TrackInspectorPane`, and
  `TrackRow` composites.

Constraints:
- Shared UI stays backend-free and screen-free.
- Use existing tokens, primitives, and composites.
- No Library or Discover migration in this task.
- Typed slots only; no `AnyElement` callback bag API.

Do not touch:
- `src/library.rs`
- `src/search.rs`
- Backend/schema/service files.

Acceptance criteria:
- Composites are exported and tests pass.
- `TrackRow` has an additive `TrackRowVm` constructor while legacy callers
  still compile.
- `track_surface_slots_are_typed` passes at baseline zero.
- No command behavior changes.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test track_detail_surface`
- `cargo test --test architecture_tests`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If a composite needs screen-specific command state, stop and add a typed
  display contract or command slot instead of importing screen modules.
- If the VM contract lacks a required display fact, stop and update Task 001
  rather than adding fallback policy in the composite.
