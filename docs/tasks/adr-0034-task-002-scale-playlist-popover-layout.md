# ADR 0034 Task 002: Scale Playlist Popover Layout

## Goal

Make `AddToPlaylistPopover` local layout scale coherently after shared
primitives are scale-aware.

## Files to Inspect

- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `src/ui/composites/playlist_popover.rs`
- `src/ui/primitives/popover.rs`
- `src/ui/primitives/surface.rs`
- `src/ui/tokens.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/ui/composites/playlist_popover.rs`
- Focused tests if playlist popover behavior has existing test coverage
- `docs/reviews/adr-0034-review-checklist.md`

## Do Not Touch

- Playlist service behavior
- Library/Search call-site semantics beyond compile-required adjustments
- Backend, database, schema, playback, MusicBrainz, metadata write paths

## Constraints

- Keep `AddToPlaylistPopover` as the only playlist popover owner.
- Do not add screen-local padding, width, or scale overrides.
- Preserve `+ New Playlist` in every add-to-playlist popover.
- Keep the popover compact enough to satisfy HIG's "small amount of related
  functionality" guidance.
- Use scaled tokens for local gaps, padding wrappers, caption text, menu width,
  and max height unless a fixed value is explicitly justified.

## Implementation Steps

1. Convert `Size::MenuRegular.px()` and `Size::ColumnRegular.px()` in
   `playlist_popover.rs` to scaled values.
2. Convert local gaps, divider margins, empty-state padding, caption text, and
   create-mode input/button wrapper padding to scaled token values.
3. Confirm `Popover::surface_padding(Spacing::XS)` now scales through the
   `Surface` primitive from Task 001.
4. Run architecture tests to confirm shared popover ownership gates still pass.
5. Record task status in the review checklist.

## Acceptance Criteria

- Playlist popover outer padding, inner spacing, width, max height, and text
  respond to `ui_scale`.
- No screen-local playlist popover layout is introduced.
- All call sites still use `AddToPlaylistPopover`.
- `+ New Playlist` remains available wherever create mode is wired.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test playlist_popover
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `docs/tasks/adr-0034-task-002-scale-playlist-popover-layout.md`
- `src/ui/composites/playlist_popover.rs`
- `src/ui/primitives/popover.rs`
- `src/ui/tokens.rs`
- `tests/architecture_tests.rs`

Goal:
- Make `AddToPlaylistPopover` local layout scale coherently with `ui_scale`.

Constraints:
- Keep one popover owner.
- No screen-local popover padding/width workarounds.
- Preserve `+ New Playlist`.
- Do not change playlist behavior.

Do not touch:
- Backend/database/service/schema files.
- Playback behavior.
- MusicBrainz or metadata write paths.

Acceptance criteria:
- Popover-local dimensions use scaled token accessors.
- Existing playlist popover architecture tests pass.
- Behavior is layout-only.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test playlist_popover`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If scaling menu width/max height makes the popover cover essential content,
  stop and document the visual issue before choosing a clamp.
- If any call site lacks `+ New Playlist`, stop and treat it as an ADR 0033
  ownership regression, not a local layout issue.
