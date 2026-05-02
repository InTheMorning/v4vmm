# ADR 0031 Task 002: Renderer Adoption and Slot Retirement

## Status

Planned - 2026-05-01.

## Goal

Render the ADR 0031 page-contract zones through the shared release-detail shell
by consuming `ReleaseDetailPageVm` directly. Retire or narrow
`ReleaseDetailSlots` so slots cannot carry hero, description, summary, or other
placement decisions that belong to the contract.

## Files To Inspect

- `docs/adr/0031-release-detail-presentation-contract.md`
- `docs/plans/adr-0031-release-detail-presentation-contract-phase-plan.md`
- `docs/tasks/adr-0031-task-001-contract-types-and-projection-tests.md`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs` only if a feed-detail call site still constructs release
  detail slots there

## Files Likely To Change

- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/view_models/entity_detail.rs` only for additive projection helpers

## Do Not Touch

- `src/db.rs`
- `migrations/`
- service modules
- navigation, playback, playlist, download, subscription, or MusicBrainz
  semantics

## Constraints

- The renderer consumes the page contract; it does not choose raw metadata
  placement.
- The shell reads only from `ReleaseDetailPageVm`; it does not access
  `FeedView` or source-fact fields directly.
- `ReleaseDetailSlots` must be deleted or reduced to slots that cannot carry
  hero, description, summary, or panel content.
- Screen modules keep action handlers, image resolution, popover state, and
  command dispatch outside the projection layer.
- Do not introduce nested vertical scroll views.
- Do not create a second release-detail shell.
- Identity-detail panels render below summary/action areas; raw identity values
  must not return to the hero.

## Implementation Steps

1. Change `render_release_detail_shell` to consume `ReleaseDetailPageVm`
   directly.
2. Update Library and Discovery call sites in lock-step.
3. Render hero from contract hero fields and screen-provided image handles.
4. Render primary actions separately from identity actions.
5. Render summary facts separately from action rows.
6. Render contract panels below summary without duplicating descriptions.
7. Delete or narrow `ReleaseDetailSlots` so it cannot override contract-owned
   placement.
8. Add focused rendering or structural tests where practical.

## Acceptance Criteria

- [ ] Library album/feed and Discovery feed details render through the same page
  contract.
- [ ] The shell reads only from the contract and not from `FeedView` or raw
  source-fact fields.
- [ ] Existing action handlers remain screen-owned.
- [ ] `ReleaseDetailSlots` is deleted or narrowed so it cannot carry hero,
  description, summary, or panel content.
- [ ] Description is not duplicated by default rendering.
- [ ] Website, Nostr, and RSS render as identity actions, not primary actions.
- [ ] No nested vertical scroll views are introduced.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test view_models::entity_detail
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0031-release-detail-presentation-contract.md`
- `docs/tasks/adr-0031-task-002-renderer-adoption.md`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui_feed.rs`
- `src/library.rs`

Goal:
- Render release detail pages from `ReleaseDetailPageVm`.

Constraints:
- Keep handlers, service calls, image lookup, and popovers screen-owned.
- Do not create a second release-detail shell.
- Do not introduce nested vertical scroll views.
- Delete or narrow `ReleaseDetailSlots`; do not preserve broad override slots
  for hero, description, summary, or panels.
- Render identity rows below the summary/action areas, never in the hero.

Do not touch:
- `src/db.rs`
- `migrations/`
- service modules

Acceptance criteria:
- Shared shell renders contract zones.
- Library and Discovery use the same skeleton.
- Actions still come from slots.
- Description is rendered once.
- Website, Nostr, and RSS render as identity actions.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::entity_detail`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
