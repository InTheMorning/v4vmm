# ADR 0036 Task 001: Feed Surface Typed Slots

## Goal

Strengthen feed/release surface ownership by replacing free-form shared release
surface slots with typed surface elements and adding architecture guards.

## Files To Inspect

- `docs/adr/0036-feed-visual-and-provenance-surface-consistency.md`
- `docs/plans/adr-0036-feed-visual-and-provenance-consistency-phase-plan.md`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui_entity.rs`
- `src/library.rs`
- `src/ui_feed.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/composites/release_detail_surface.rs`
- `src/ui_entity.rs`
- `src/library.rs`
- `src/ui_feed.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0036-review-checklist.md`

## Do Not Touch

- Backend services, database schema, API clients, RSS/ID3 parsing, playlist
  semantics, playback semantics.
- `src/search.rs` except through existing `ui_feed.rs` call paths.

## Constraints

- Do not redesign the feed page.
- Do not add new feature behavior.
- Screens may provide command-bearing elements, but shared APIs must name the
  slot boundary.
- Keep visual smoke screenshot-based.

## Implementation Steps

1. Add a typed release surface element wrapper in the shared release surface
   module.
2. Change `ReleaseDetailBehaviorSlots`, `ContributorRowSlot`, and
   `ReleaseTrackRowSlot` where appropriate to accept typed release surface
   elements instead of raw `AnyElement`.
3. Wrap existing Library and Discover call-site elements at the boundary.
4. Add architecture tests for typed release surface slots and VM consumption.
5. Run the required checks.

## Acceptance Criteria

- Library and Discover feed detail still route through
  `render_release_detail_shell`.
- Shared release surface behavior slots are typed.
- Architecture tests fail if feed detail bypasses `ReleaseDetailVm`.
- Full code checks are green.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0036-feed-visual-and-provenance-surface-consistency.md`
- `docs/tasks/adr-0036-task-001-feed-surface-typed-slots.md`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui_entity.rs`
- `src/library.rs`
- `src/ui_feed.rs`
- `tests/architecture_tests.rs`

Goal:
- Replace free-form release surface behavior slots with typed surface elements
  and add architecture guards.

Constraints:
- No backend, schema, API, playlist, or playback changes.
- Preserve existing command wiring.
- Do not tune visual spacing in this task.

Do not touch:
- `src/search.rs` unless a compile error proves it is necessary.
- Metadata inference or service modules.

Acceptance criteria:
- `render_release_detail_shell` remains the only Library/Discover feed detail
  shell.
- Release behavior slots use typed wrapper values.
- New architecture tests cover typed slots and VM consumption.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
