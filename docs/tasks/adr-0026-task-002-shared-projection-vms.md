# ADR 0026 Task 002: Shared Projection VMs

## Status

Implemented.

## Goal

Add the GPUI-free shared entity-detail projection module that formats
source-normalized `views` facts into display-ready headers, rows, track-list
summaries, contributor groups, identity actions, and semantic action
descriptors.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-001-identity-facts.md`
- `src/views.rs`
- `src/view_models/mod.rs`
- `src/view_models/feed.rs`
- `src/view_models/track.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/mod.rs`
- `src/view_models/entity_detail.rs`
- `src/views.rs` only if projection targets need trait derives
- `tests/architecture_tests.rs`
- ADR 0026 task/review docs

## Do Not Touch

- Do not migrate Discover or Library rendering.
- Do not create `src/ui_entity.rs`.
- Do not import GPUI, UI modules, screen modules, service modules, or API
  client row types into `view_models::entity_detail`.
- Do not change playlist, download, MusicBrainz, playback, or database
  behavior.

## Constraints

- `src/view_models/entity_detail.rs` must stay pure and GPUI-free.
- Projections consume `crate::views` facts, not raw `crate::api` rows.
- Action descriptors contain kind, target, enabled state, and tone; they do
  not contain GPUI handlers or elements.
- Context differences must be expressed by `EntitySurfaceContext`.
- Keep all display formatting covered by focused unit tests.

## Implementation Summary

- Added `view_models::entity_detail` with shared header, release detail,
  identity-link, contributor-list, track-list, track-row, and action
  projections.
- Added semantic `EntityActionKind`, `EntityActionTarget`,
  `EntityActionTone`, and `EntitySurfaceContext` types.
- Added unit tests for release headers, metadata rows, identity actions,
  contributor grouping, track summaries, and context-specific row actions.
- Added an architecture test that rejects GPUI, UI, screen, service, and API
  imports in `src/view_models/entity_detail.rs`.

## Acceptance Criteria

- [x] `src/view_models/entity_detail.rs` exists and is exported from
  `view_models::mod`.
- [x] The module imports `crate::views` facts and no GPUI/UI/screen/service/API
  row modules.
- [x] `ReleaseDetailVm`, `IdentityLinksVm`, `ContributorListVm`,
  `TrackListVm`, `SharedTrackRowVm`, `EntityActionVm`,
  `EntityActionTarget`, and `EntitySurfaceContext` exist.
- [x] Projection tests cover headers, summaries, actions, contributors, and
  empty identity states.
- [x] Architecture tests enforce the module boundary.
- [x] Existing Discover and Library behavior is unchanged.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test view_models::entity_detail
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-002-shared-projection-vms.md`
- `src/views.rs`
- `src/view_models/mod.rs`
- `tests/architecture_tests.rs`

Goal:
- Add pure shared entity-detail projection VMs.

Constraints:
- No GPUI, UI, screen, service, or API client imports in
  `src/view_models/entity_detail.rs`.
- No screen rendering migration.
- No service/database behavior changes.

Do not touch:
- `src/library.rs`
- `src/search.rs`
- `src/ui_feed.rs`
- `src/ui_track.rs`
- playlist/download/MusicBrainz/playback behavior

Acceptance criteria:
- Shared projection types exist and are unit-tested.
- Architecture tests enforce the boundary.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::entity_detail`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A projection needs raw `api` rows rather than `views` facts.
- A projection needs GPUI state, loaded image handles, or click handlers.
- Discover or Library must be migrated to make the module compile.
