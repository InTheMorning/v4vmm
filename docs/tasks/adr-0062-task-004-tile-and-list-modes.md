# ADR 0062 Task 004: Tile And List Modes

Status: Ready - 2026-09-07. Do after task 003.

## Goal

Give the `Music` content region a tile mode and a list mode over the same rows.
Promote the existing view mode rather than inventing one.

## Files To Inspect

- `docs/adr/0062-music-content-surface.md`
- `src/view_models/recent_feeds.rs`, for `RecentFeedsViewMode`
- `src/view_models/library.rs`, for `ContentListPageVm`
- `src/ui/shells/workspace.rs`
- `src/config.rs`, for preference persistence
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/library.rs`
- `src/ui/composites/` for the tile renderer and the mode control
- `src/ui/shells/workspace.rs`
- `src/config.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- The row contract from task 001. Both modes render the same rows.
- The filter and the sort. Neither changes with the mode.
- `src/view_models/recent_feeds.rs` internals. Task 005 moves ownership.

## Constraints

- **`RecentFeedsViewMode` already exists** with `Tiles` and `List`, a default of
  `Tiles`, and label and accessibility label helpers. Move or generalize it.
  Do not define a second view mode enum.
- **Both modes render the same rows and the same badges.** A row that is
  visible in list mode is visible in tile mode. The mode changes presentation
  only.
- Tile mode is a second renderer over one contract. It must read the row
  contract and add nothing of its own. ADR 0062 records renderer drift as the
  risk here.
- The entity badge and the library badge appear in both modes.
- Mode selection is a view-model fact with a typed control, not a renderer
  toggle.
- Tokens own tile size, spacing, and grid geometry. No raw literals.

## Implementation Steps

1. Move `RecentFeedsViewMode` to a shared location, or generalize it so the
   content list owns it. Keep the existing labels and accessibility labels.
2. Add the mode to `ContentListPageVm` as display state with a typed control.
3. Add the tile renderer. It reads the same rows as the list renderer.
4. Render the mode control in the frame chrome beside the filter.
5. Persist the mode in configuration. An older `config.toml` without the key
   loads and uses the default.
6. Add guards, situational, citing ADR 0062:
   - one view mode enum exists, not two
   - both renderers read the same row contract
   - tile geometry comes from tokens
7. Add view-model tests: default mode, mode change, and identical visible rows
   in both modes for the same filter.

## Acceptance Criteria

Mechanical:

- One view mode enum exists in the codebase.
- `visible_rows()` returns the same rows in both modes for the same filter, and
  a test asserts it.
- The mode is a view-model fact with a typed control.
- The mode persists, and a `config.toml` without the key still loads.
- Tile geometry uses tokens, and a guard proves no raw literal.

Visual proof, operator only:

- Tile mode shows artwork clearly and remains readable at a normal width.
- Both modes show the entity badge and the library badge.
- Switching modes preserves scroll position or returns to the top predictably.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test library --lib --quiet`
- `cargo test config --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo run` for the visual check

## Expected Final Report Format

1. Files changed
2. Tests run
3. Where the view mode enum now lives
4. Screenshots captured, or the visual gate reported open
5. Deviations from task
6. Unresolved concerns

## Escalation Triggers

- A row kind has no artwork to show in tile mode. Report the fallback rather
  than inventing a placeholder that looks like real artwork.
- Moving `RecentFeedsViewMode` breaks a caller that task 005 has not yet moved.
- Scroll position cannot be preserved across a mode change without a change to
  the paging contract.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0062-music-content-surface.md`
- `src/view_models/recent_feeds.rs`, `src/view_models/library.rs`

Goal:
- Tile and list modes over the same rows, promoting the existing
  `RecentFeedsViewMode`.

Constraints:
- One view mode enum. Do not define a second.
- Both modes render the same rows and the same badges.
- Mode is a view-model fact with a typed control.
- Tile geometry from tokens only.
- The mode persists and an older config still loads.

Do not touch:
- the row contract, the filter, the sort, `recent_feeds.rs` internals

Acceptance criteria are split into mechanical and visual. Report the visual
gate as open if no window can be opened.

Test commands:
- `cargo fmt -- --check`
- `cargo test library --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. where the view mode enum now lives
4. screenshots captured or the visual gate reported open
5. deviations from task
6. unresolved concerns
