# ADR 0033 Task 004: File Header Composite

## Goal

Consolidate the duplicated Library/Discover `render_file_header` helpers into
one shared composite fed by a GPUI-free `FileHeaderVm`.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/library.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/file_header.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`
- `docs/tasks/adr-0033-task-004-file-header-composite.md`
- `docs/reviews/adr-0033-task-004-review.md`

## Do Not Touch

- Backend, database, API, service, and command modules.
- MusicBrainz, action-row, metadata-grid, and track-header helpers.
- ADR 0031 release-detail behavior.
- ADR 0032 playlist popovers.

## Constraints

- Preserve the visible file-header behavior: large track thumbnail, tag badge,
  `Re-read`, `Re-download`, title, and two-line path.
- The view-model must not import GPUI or produce `SharedString`/`AnyElement`.
- The composite must stay backend-free and screen-free.
- Image resolution remains screen-owned; pass `Option<Arc<Image>>` into the
  composite separately from the VM.
- Remove the `render_file_header` duplication baseline in the same change.

## Implementation Steps

1. Add `FileHeaderVm` projection from `TagCompareResult`.
2. Add `src/ui/composites/file_header.rs` with action slots supplied by the
   screen.
3. Export `FileHeader` from `src/ui/composites/mod.rs`.
4. Replace Library and Discover helper bodies with direct composite usage and
   delete both local `render_file_header` functions.
5. Remove the `render_file_header` entry from
   `RENDER_HELPER_DUPLICATION_BASELINES`.
6. Run the verification commands.

## Acceptance Criteria

- No `fn render_file_header` remains in `src/library.rs` or `src/search.rs`.
- The architecture duplication gate still passes with the file-header baseline
  removed.
- `FileHeaderVm` has unit coverage for title fallback and embedded-format
  label behavior.
- All verification commands are green.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`

Goal:
- Replace duplicated Library/Discover file-header rendering with one shared
  composite and one shared view-model.

Constraints:
- Keep screen-owned action callbacks and image resolution in the screens.
- Keep display projection in `FileHeaderVm`.
- Keep chrome/layout in `FileHeader`.
- Do not modify unrelated helper families.

Do not touch:
- Backend/service/database/API modules.
- MusicBrainz, action-row, metadata-grid, and track-header tasks.

Acceptance criteria:
- No screen-local `render_file_header` functions remain.
- The render-helper duplication baseline no longer includes
  `render_file_header`.
- Verification commands are green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
