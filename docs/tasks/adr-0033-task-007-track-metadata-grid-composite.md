# ADR 0033 Task 007: Track Metadata Grid Composite

## Goal

Consolidate the duplicated Library and Discover track metadata-grid shell behind a shared view-model and UI composite while preserving screen-specific cell interactions.

## Files to inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/metadata.rs`
- `src/ui/composites/mod.rs`
- `src/view_models/mod.rs`
- `tests/architecture_tests.rs`

## Files likely to change

- `src/view_models/track_metadata_grid.rs`
- `src/view_models/mod.rs`
- `src/ui/composites/track_metadata_grid.rs`
- `src/ui/composites/mod.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0033-task-007-review.md`

## Do not touch

- Backend services, schema, migrations, ID3 write behavior, MusicBrainz lookup behavior, or playlist behavior.
- Metadata cell drag/drop semantics.
- Metadata value expansion renderers.
- MusicBrainz panel implementation.

## Constraints

- The shared view-model must stay GPUI-free.
- The shared composite must not import screen app types, backend services, `api`, or `db`.
- Screen-specific metadata cells may remain screen-owned because they carry drag/drop and edit callbacks.
- Preserve RSS, tag, and MusicBrainz column visibility and labels.
- Remove `render_track_metadata_grid` from the cross-screen duplication baseline.

## Implementation steps

1. Add `TrackMetadataGridVm` for column headings, column count, and expansion-key lookup.
2. Add `TrackMetadataGrid` composite for heading rendering and grid layout.
3. Replace Library's duplicated metadata-grid shell with the shared VM and composite.
4. Replace Discover's duplicated metadata-grid shell with the shared VM and composite.
5. Keep metadata cell rendering local to each screen.
6. Remove the `render_track_metadata_grid` duplication baseline.
7. Add focused view-model coverage for heading and expansion-key behavior.

## Acceptance criteria

- `src/library.rs` and `src/search.rs` no longer define `render_track_metadata_grid`.
- Grid headings and column count are projected by `TrackMetadataGridVm`.
- Grid layout and heading rendering live in `src/ui/composites/track_metadata_grid.rs`.
- Architecture tests still prevent reintroducing the duplicated helper.
- Formatting, compile, architecture tests, full tests, clippy, and diff whitespace checks are green.

## Test commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test
cargo clippy --lib --tests -- -D warnings
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/metadata.rs`
- `src/ui/composites/mod.rs`
- `src/view_models/mod.rs`
- `tests/architecture_tests.rs`

Goal:
- Replace duplicated track metadata-grid shell rendering with shared `TrackMetadataGridVm` and `TrackMetadataGrid`.

Constraints:
- Do not move drag/drop, edit callbacks, or screen app types into shared UI.
- Preserve current column visibility, column labels, expansion behavior, and cell interactions.
- Use tokenized spacing and semantic colors in the shared composite.

Do not touch:
- Backend services, migrations, ID3 write logic, MusicBrainz lookup logic, playlist code, or unrelated screen helpers.

Acceptance criteria:
- No duplicated `render_track_metadata_grid` helper remains.
- Grid shell and headings are shared.
- Architecture baseline no longer contains `render_track_metadata_grid`.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation triggers

- The shared composite needs command callbacks or screen-specific app types.
- Preserving cell behavior requires moving backend/service calls into shared UI.
- Removing the baseline exposes unrelated duplicated helpers.
