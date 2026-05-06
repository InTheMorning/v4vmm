# ADR 0033 Task 008: MusicBrainz Panel Composite

## Goal

Consolidate the duplicated Library and Discover MusicBrainz lookup panel render
helpers into one shared view-model and one shared composite.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/metadata.rs`
- `src/ui/composites/mod.rs`
- `src/view_models/mod.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/musicbrainz_panel.rs`
- `src/ui/composites/musicbrainz_panel.rs`
- `src/view_models/mod.rs`
- `src/ui/composites/mod.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend services, database schema, API clients, and MusicBrainz lookup logic.
- Playlist popover primitives or already-completed ADR 0033 composites.
- Unrelated screen rendering helpers.

## Constraints

- View-model code must stay GPUI-free and expose plain Rust display data.
- Shared UI must take pre-resolved images and screen-agnostic callbacks.
- The composite owns the picker, selected-result header, and empty-state
  presentation.
- Preserve the canonical behavior from the plan: Search-style empty state,
  disabled trigger when no candidates exist, `SM` trigger spacing, and
  screen-owned candidate selection dispatch.
- Remove the MusicBrainz render-helper duplication baseline when the duplicate
  helpers are gone.

## Implementation Steps

1. Add `MusicBrainzPanelVm` to project lookup candidates into trigger text,
   selected title/subtitle, option labels, and selected option state.
2. Add `MusicBrainzPanel` under `src/ui/composites/`.
3. Export the VM/composite from their module roots.
4. Replace the Library inspector MusicBrainz section with the composite.
5. Delete the duplicated Library and Discover MusicBrainz render helpers.
6. Remove the MusicBrainz entries from `RENDER_HELPER_DUPLICATION_BASELINES`.
7. Add focused VM tests for empty, selected, and invalid selection states.

## Acceptance Criteria

- No `render_musicbrainz_*` helper appears in both screen files.
- The architecture duplication baseline is empty.
- The Library panel uses the shared composite and still dispatches selection
  through `LibraryApp::select_musicbrainz_candidate`.
- The view-model has unit coverage for candidate projection and fallback.
- Verification is green.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/metadata.rs`
- `tests/architecture_tests.rs`

Goal:
- Consolidate the duplicated MusicBrainz lookup panel presentation into
  `MusicBrainzPanelVm` and `MusicBrainzPanel`.

Constraints:
- Keep view-models GPUI-free.
- Keep image lookup and app-specific callbacks in screen code.
- Use the canonical behavior listed in the plan.
- Do not change backend lookup behavior.

Do not touch:
- Service modules.
- Database schema or migrations.
- Existing playlist popover implementation.
- Unrelated render helpers.

Acceptance criteria:
- Library uses the shared composite.
- Duplicated `render_musicbrainz_*` helpers are removed.
- The MusicBrainz render-helper duplication baseline is empty.
- Unit tests cover the VM selection cases.

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

## Escalation Triggers

- A needed behavior is not representable without adding GPUI to the VM.
- The composite requires backend or screen-specific types.
- The architecture tests need a new baseline instead of removing one.
