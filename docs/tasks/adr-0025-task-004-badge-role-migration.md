# ADR 0025 Task 004: Typed Badge Role Migration

## Status

Implemented.

## Task Goal

Replace remaining screen-level `theme::badges` usage with typed
entity/status/provenance roles.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `src/ui/composites/tag_badge.rs`
- `src/ui/icons.rs`
- `src/ui/theme.rs`
- `src/search.rs`
- `src/library.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/composites/tag_badge.rs`
- `src/ui/icons.rs`
- `src/ui/theme.rs`
- `src/search.rs`
- `src/library.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- application commands/queries/events
- service modules
- database migrations
- unrelated row layout

## Constraints

- Preserve current badge labels and behavior.
- Replace string-keyed visual roles with typed roles.
- Do not remove compatibility helpers until all call sites are migrated.
- State must not rely on color alone.
- Entity roles must match the current `EntityKind` surface: feed, track,
  artist, publisher, release, recording, playlist, and generic.
- Provenance/diff roles (`match`, `different`, `missing`) are owned here as
  typed visual roles. They may consume icons from `ui::icons`, but color,
  icon/glyph, label, and accessibility text must resolve together.

## Implementation Steps

1. Identify remaining `theme::badges` call sites.
2. Align ADR 0025 badge roles with all current `EntityKind` variants:
   feed, track, artist, publisher, release, recording, playlist, and generic.
3. Extend `TagBadge` / `EntityKind` or add status/provenance role types where
   entity roles are insufficient.
4. Move provenance/diff display off `theme::color::diff_*` plus loose glyphs
   and into typed visual roles.
5. Migrate one coherent set of call sites at a time.
6. Tighten architecture tests once screen-level usage is gone.
7. Leave `theme::badges` only if non-screen compatibility code still needs it.

## Acceptance Criteria

- [x] Screen-level `theme::badges` usage is removed or explicitly allowlisted
      with a reason.
- [x] Badge roles are typed and cover all current `EntityKind` variants.
- [x] Provenance/diff roles are typed and resolve color plus non-color cue
      together.
- [x] Existing label/color intent is preserved.
- [x] Architecture tests prevent regression.

## Implementation Notes

- Added typed `EntityKind` color accessors for compatibility surfaces that
  still need raw colors while the native button primitive is catching up.
- Added `ProvenanceRole` for metadata comparison states so match, different,
  and missing color plus glyph resolve from one typed role.
- Removed remaining screen-level `theme::badges` imports from `src/library.rs`
  and `src/search.rs`.
- Tightened architecture tests to hold `theme::badges` at zero screen usage
  and prevent loose provenance/diff helper growth.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A remaining badge use is not an entity/status/provenance role.
- Preserving contrast requires changing token values.
- A badge migration would require changing workflow behavior.
- Provenance/diff rendering requires icon catalog functionality not available
  after Task 002.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0025-theme-icon-style-boundary.md`
- `src/ui/composites/tag_badge.rs`
- `src/ui/theme.rs`
- `src/search.rs`
- `src/library.rs`
- `tests/architecture_tests.rs`

Goal:
- Replace screen-level `theme::badges` usage with typed badge roles.

Constraints:
- Preserve current badge labels and behavior.
- Do not remove compatibility helpers until call sites are migrated.
- State must not rely on color alone.
- Cover all current `EntityKind` variants.
- Own provenance/diff visual roles in this migration.

Do not touch:
- `src/application/**`
- service modules
- database migrations
- unrelated row layout

Acceptance criteria:
- Screen-level badge color lookup is gone or explicitly allowlisted.
- Typed entity/status/provenance roles are used.
- Entity roles cover feed, track, artist, publisher, release, recording,
  playlist, and generic.
- Architecture tests prevent regression.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
