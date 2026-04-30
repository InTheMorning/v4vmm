# ADR 0023 Task 007: Shared Release Detail Surface

## Status

Completed 2026-04-30.

## Task Goal

Make Discover feed detail and Library album detail share one structural detail
surface: header, action bar, detail grid, section header, track rows, and
optional mode-specific panels.

## Files To Inspect

- `src/search.rs`
- `src/library.rs`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/detail_grid.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/composites/action_button.rs`
- `src/view_models/feed.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`

## Files Likely To Change

- `src/search.rs`
- `src/library.rs`
- `src/ui/composites/mod.rs`
- New: `src/ui/composites/detail_pane.rs`,
  `src/ui/composites/release_detail_surface.rs`, or equivalent narrow helper
- `src/view_models/library.rs`
- `src/view_models/search.rs` only if Discover-specific display strings move
- ADR 0023 docs/task status

## Do Not Touch

- `db.rs` schema or migrations.
- MusicBrainz lookup logic.
- Subscription/download service semantics.
- Broad command/query/event architecture.

## Constraints

- The shared surface may accept GPUI children for mode-specific actions and
  panels, but it must own the common layout order.
- Library-specific controls remain available as trailing actions or panels.
- Discover-specific feed controls remain available.
- Do not add visible instructional text to explain the UI.
- Prefer existing tokens/primitives/composites over one-off `div()` chains.

## Implementation Steps

1. Define the minimal shared detail-surface API needed by the two existing
   callers.
2. Convert Discover feed detail to render through the shared surface.
3. Convert Library album detail to render through the shared surface.
4. Keep mode-specific actions passed in as children or action-slot elements.
5. Preserve track-row click handlers and playlist picker behavior.
6. Update docs and review checklist with the new source of truth.

## Acceptance Criteria

- [x] Discover feed detail and Library album detail use the same common surface.
- [x] The layout order is identical in both modes.
- [x] The same album/feed no longer gets a fundamentally different page skeleton
  across tabs.
- [x] Library-only MusicBrainz/compare/remove affordances remain intact.
- [x] No raw screen-level colors or numeric `px(...)` literals are introduced.

## Result

- Added `ReleaseDetailSurface` as the shared composite for release/feed detail
  pages.
- Discover feed detail and Library album detail now pass headers, actions,
  details, panels, and track rows into the same structural surface.
- Discover-only podroll/lazy panels and Library-only playlist picker panels
  remain mode-specific children; the common ordering lives in the composite.
- Track row click, playlist picker, download/remove, and MusicBrainz behavior
  were preserved.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::library`
- `cargo test --lib view_models::search`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A shared surface requires changing service behavior.
- The existing track-row component cannot support one mode without weakening
  the other.
- The task starts growing into a full screen split.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0023-design-system-and-view-models.md`
- `docs/plans/adr-0023-finalization-plan.md`
- `src/search.rs`
- `src/library.rs`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/detail_grid.rs`
- `src/ui/composites/track_row.rs`
- `src/view_models/library.rs`

Goal:
- Make Discover feed detail and Library album detail share one structural
  detail surface.

Constraints:
- Preserve service behavior and click behavior.
- Keep Library-specific actions available.
- Use existing tokens/primitives/composites.
- Do not add broad command bus or screen directory splits.

Do not touch:
- Schema/migrations.
- MusicBrainz lookup behavior.
- Subscription/download semantics.

Acceptance criteria:
- Both modes use the same shared detail-surface helper/component.
- Layout order is identical.
- Mode-specific actions remain intact.
- Verification commands are green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::library`
- `cargo test --lib view_models::search`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
