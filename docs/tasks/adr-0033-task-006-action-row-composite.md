# ADR 0033 Task 006: Action Row Composite

## Goal

Consolidate the duplicated Library and Discover inspector action-row presentation into one shared composite while preserving screen-owned command wiring.

## Files to inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/ui/composites/action_button.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`

## Files likely to change

- `src/ui/composites/action_row.rs`
- `src/ui/composites/mod.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0033-task-006-review.md`

## Do not touch

- Backend services, schema, migrations, playlist mutation behavior, or metadata write behavior.
- Playlist popover internals; this task only places the existing popover inside shared action-row chrome.
- Track metadata grid and MusicBrainz panel implementations.

## Constraints

- Keep command callbacks, target resolution, and backend IDs in screen code.
- Shared UI must not import `api`, `db`, service modules, `LibraryApp`, or `SearchApp`.
- Preserve action labels, disabled states, status text, and playlist target behavior.
- Use tokenized colors and spacing for the shared shell and messages.
- Remove `render_action_row` from the cross-screen duplication baseline.

## Implementation steps

1. Add `ActionRow` and `ActionRowMessage` under `src/ui/composites/`.
2. Export the composite from `src/ui/composites/mod.rs`.
3. Replace Library's track action row helper with the shared composite plus screen-owned controls.
4. Replace Discover's inspector action row helper with the shared composite plus screen-owned controls.
5. Remove the `render_action_row` duplication baseline.
6. Add focused builder/default tests for the composite's message contract.

## Acceptance criteria

- `src/library.rs` and `src/search.rs` no longer define `render_action_row`.
- Shared action-row spacing and message tone styling live in `src/ui/composites/action_row.rs`.
- Shared UI has no backend or screen imports.
- Playlist popover behavior still uses the existing shared `AddToPlaylistPopover`.
- Architecture duplication tests remain green.

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
- `src/ui/composites/action_button.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`

Goal:
- Replace duplicated inspector action-row presentation with a shared `ActionRow` composite while leaving command wiring in the screens.

Constraints:
- Do not move backend IDs, command callbacks, or playlist target resolution into shared UI.
- Preserve labels, disabled states, status messages, and add-to-playlist behavior.
- Use tokenized spacing and semantic color roles.

Do not touch:
- Backend services, migrations, playlist popover internals, metadata grid, MusicBrainz panel, or unrelated screen helpers.

Acceptance criteria:
- No duplicated `render_action_row` helper remains.
- `ActionRow` owns the repeated action-row stack and message styling.
- Architecture baseline no longer contains `render_action_row`.

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

- The shared composite needs direct access to `db`, `api`, services, or screen app types.
- Preserving playlist target behavior requires changing command semantics.
- Removing the baseline exposes unrelated duplicated helpers.
