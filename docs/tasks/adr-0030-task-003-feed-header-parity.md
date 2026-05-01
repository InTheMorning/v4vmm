# ADR 0030 Task 003: Feed Header Parity

## Status

Pending.

## Goal

Make Library and Discovery feed detail headers render the same data structure
while keeping actions in explicit action slots.

## Files To Inspect

- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/plans/discovery-library-ui-fixes.md`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/search.rs`
- `src/library.rs`
- `src/view_models/entity_detail.rs`

## Files Likely To Change

- `src/ui/composites/detail_header.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/search.rs`
- `src/library.rs`
- `src/view_models/entity_detail.rs`

## Do Not Touch

- Database schema or migrations.
- Service, download, playlist, playback, and MusicBrainz command code.
- Contributor metadata tree formatting from Task 005.

## Constraints

- Extend `DetailHeader` additively.
- Do not create a second feed-header composite.
- Keep action buttons in `ReleaseDetailSlots.action_row` or identity-action
  slots.
- Preserve GPUI-free view models.

## Implementation Steps

1. Add optional data slots to `DetailHeader`.
2. Render publisher, description, npub, and website as data, not actions.
3. Feed the same fields from Library and Discovery detail call sites.
4. Keep existing action elements in action slots.
5. Add focused projection or rendering tests where practical.

## Acceptance Criteria

- Library and Discovery feed headers use one structure.
- Data fields do not appear interleaved with action buttons.
- Existing feed actions still work from screen-owned handlers.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test view_models::entity_detail
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/tasks/adr-0030-task-003-feed-header-parity.md`
- `src/ui/composites/detail_header.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/search.rs`
- `src/library.rs`
- `src/view_models/entity_detail.rs`

Goal:
- Unify feed header data placement between Library and Discovery.

Constraints:
- Extend `DetailHeader` additively.
- Do not move command handlers into shared code.
- Do not add a parallel `FeedHeader`.

Do not touch:
- `src/db.rs`
- `migrations/`
- download/playback/playlist/MusicBrainz service code

Acceptance criteria:
- Same header data order in Library and Discovery.
- Actions remain below or outside data fields through existing slots.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- focused projection/rendering tests
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
