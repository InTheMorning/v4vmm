# ADR 0027 Task 001: Track Row Action State

## Status

Planned.

## Goal

Introduce GPUI-free track-row action-state inputs and projection tests so
Library and Discover can render equivalent download/remove/playlist row actions
from one shared descriptor vocabulary.

## Read

- `docs/adr/0027-shared-entity-action-state.md`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/reviews/post-adr-0026-task-001-visual-smoke-review.md`
- `src/view_models/entity_detail.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/entity_detail.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`
- `docs/tasks/adr-0027-task-001-track-row-action-state.md`
- `docs/reviews/adr-0027-task-001-review.md`

## Do Not Touch

- Do not change database schema or migrations.
- Do not change command implementations.
- Do not change download, playlist, MusicBrainz, or playback service behavior.
- Do not move GPUI imports into `src/views.rs` or shared projection modules.
- Do not implement release-level actions in this task.

## Constraints

- New action-state structs must be plain data.
- Shared projections may emit `EntityActionVm`; they must not emit GPUI
  buttons or closures.
- Screen adapters still own command dispatch and popup state.
- Repeated destructive row actions must use quiet destructive tone.
- Keep the first slice focused on track-row membership and playlist actions.

## Implementation Steps

1. Add a narrow `TrackActionState` input and membership enum in the shared
   projection layer.
2. Update shared track-row projection tests for remote, downloading, in-library,
   and removing states.
3. Adapt Library and Discover row rendering to derive visible labels/tone from
   shared descriptors while keeping existing handlers.
4. Add or update architecture tests for the new action-state boundary.
5. Capture a before/after note in the task review.

## Acceptance Criteria

- Shared projection tests cover track-row action labels, tone, and disabled or
  busy state.
- Library and Discover row actions use the same descriptor vocabulary for
  download/remove/playlist actions.
- Existing command behavior is unchanged.
- Shared projection modules remain GPUI-free.
- Required verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test entity_detail
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

Run broader `cargo test` before marking the task implemented.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0027-shared-entity-action-state.md`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-001-track-row-action-state.md`
- `src/view_models/entity_detail.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

Goal:
- Implement the first ADR 0027 slice for shared track-row action state.

Constraints:
- Keep shared action state plain data and GPUI-free.
- Keep command dispatch and popup state in screen adapters.
- Do not change schema, service behavior, or command semantics.
- Do not implement release-level actions yet.

Do not touch:
- migrations
- database schema
- download, playlist, MusicBrainz, or playback command implementations
- unrelated style/layout code outside the row action binding needed here

Acceptance criteria:
- Projection tests cover the new track action states.
- Library and Discover use shared descriptors for track row action labels and
  tones.
- Architecture tests protect the new boundary.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test entity_detail`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The slice requires changing command behavior.
- Release-level actions are needed to make track-row actions coherent.
- Existing view-model boundaries force GPUI into shared projection code.
