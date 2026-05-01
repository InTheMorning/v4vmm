# ADR 0024 Task 007: Presentation Cleanup

## Status

In progress.

## Task Goal

After migrated workflows leave presentation code, simplify GPUI modules and
split files only where the application boundary has already made the split
low-risk.

## Orchestration Findings

- First implemented cleanup: top-level playback bar command/query binding moved
  out of `app.rs` into `src/app/playback_bar.rs`. This code already used ADR
  0024 commands and `ApplicationQueryService`, so the move is presentation-only.
- Top-level keyboard routing moved out of `app.rs` into
  `src/app/keyboard.rs`. This is presentation-only tab/search/navigation
  dispatch over existing Library and Search screen methods.
- Top-level application-event bridge/drain handling moved out of `app.rs` into
  `src/app/events.rs`. This remains presentation refresh logic over typed
  application events.
- Keep live-driver polling in `app.rs` for now because it still supervises
  `PlaybackOwner<D>` directly.
- Library cleanup candidates are command-binding adapters and render-only
  sidebar/detail surfaces. Do not move `musicbrainz_feed` fallback/matching,
  ID3 apply/compare, `build_tree`, or filesystem cleanup in this task.
- Search cleanup candidates are render-only helpers and command-dispatch row
  adapters. Do not move remote MusicIndex fetches, remote inspector loads,
  playlist add paths that still query services directly, or metadata grid
  provenance/editing helpers in the first cleanup pass.

## Files To Inspect

- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `src/app.rs`
- `src/app/events.rs`
- `src/app/keyboard.rs`
- `src/app/playback_bar.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/**`
- `src/ui/**`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- Possible new presentation submodules if justified
- `tests/architecture_tests.rs`
- Documentation if paths change

## Do Not Touch

- Application command/query/event semantics.
- Service/domain behavior.
- Database schema.
- Visual redesign, unless required to preserve existing layout during a split.

## Constraints

- Do not split files before migrated service dispatch is gone from the relevant
  workflow.
- Preserve ADR 0023 design-system shape: tokens, primitives, composites,
  view-models, screens.
- GPUI remains presentation-only.
- Avoid opportunistic refactors unrelated to migrated ADR 0024 workflows.

## Implementation Steps

1. Done: identify remaining presentation methods that now only bind view-models,
   dispatch commands, or bridge events.
2. In progress: split or rename presentation modules only where it improves
   comprehension. Top-level playback bar binding has moved to
   `src/app/playback_bar.rs`; top-level keyboard routing has moved to
   `src/app/keyboard.rs`; top-level event drain handling has moved to
   `src/app/events.rs`.
3. Keep view-models and application modules GPUI-free.
4. Update architecture tests and docs for any path changes.
5. Run full verification.

## Acceptance Criteria

- Presentation code is thinner and still behaviorally equivalent.
- No workflow logic moves back into GPUI modules.
- Docs and architecture tests reflect any new paths.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test --test architecture_tests`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A proposed split requires redesigning application commands or view-models.
- Presentation cleanup would change user-visible behavior.
- Architecture tests become too brittle and require a `syn`-based replacement.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- `src/application/**`
- `src/view_models/**`
- `tests/architecture_tests.rs`

Goal:
- Thin and organize presentation code after ADR 0024 workflow migrations.

Constraints:
- No workflow behavior changes.
- No application-layer redesign.
- Preserve ADR 0023 design-system structure.

Do not touch:
- Service/domain behavior.
- Database schema.
- Unrelated UI redesign.

Acceptance criteria:
- Presentation modules are simpler.
- No workflow logic moves back into GPUI.
- Tests and docs reflect final paths.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
