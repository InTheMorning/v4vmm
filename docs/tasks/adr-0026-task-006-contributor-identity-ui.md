# ADR 0026 Task 006: Contributor Identity UI

## Status

Implemented.

## Goal

Render contributor images, website links, and Nostr identities through the ADR
0026 local contributor projection while preserving screen-owned click behavior.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `src/views.rs`
- `src/view_models/entity_detail.rs`
- `src/search.rs`

## Files Changed

- `src/search.rs`
- `src/ui/composites/identity_action.rs`
- `src/ui/composites/mod.rs`
- `src/view_models/search.rs`
- `src/view_models/entity_detail.rs`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-006-contributor-identity-ui.md`
- `docs/reviews/adr-0026-task-006-review.md`

## Do Not Touch

- Do not change MusicIndex API fetching behavior.
- Do not move network requests into shared projection or UI shell modules.
- Do not change feed, track, playlist, Library, or download behavior.
- Do not introduce another button vocabulary outside ADR 0025 control roles.

## Constraints

- Store lazy contributor rows as local `ContributorView` values after fetch.
- Use shared contributor row projections from `entity_detail`.
- Keep image resolution and click handlers in `src/search.rs`.
- Render actions with existing design-system control roles and semantic icons
  where available.
- Route website and Nostr contributor affordances through a shared
  identity-action composite.
- Preserve raw contributor identity fields already modeled by Phase 1.

## Implementation Summary

- Converted the Discover inspector contributor lazy panel from
  `api::Contributor` rows to local `ContributorView` rows.
- Removed the duplicate search-local `ContributorVm` projection.
- Rendered contributor rows with thumbnail fallbacks, website buttons, and
  Nostr copy buttons.
- Reused `ContributorListVm` and `ContributorRowVm` for grouping and labels.
- Added `identity_action_button` for shared Website/Nostr identity affordance
  styling.

## Acceptance Criteria

- [x] Contributor rows render image thumbnails when `img` is present.
- [x] Contributor rows expose website actions when `href` is present.
- [x] Contributor rows expose Nostr copy actions when `npub` is present.
- [x] The contributor lazy panel no longer stores API contributor rows.
- [x] Shared projections remain GPUI-free.
- [x] Verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test contributor
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/tasks/adr-0026-task-006-contributor-identity-ui.md`
- `src/views.rs`
- `src/view_models/entity_detail.rs`
- `src/search.rs`

Goal:
- Render contributor images, website links, and Nostr identities through local
  contributor projections.

Constraints:
- Keep fetch behavior unchanged.
- Keep click behavior screen-owned.
- Use ADR 0025 control roles for contributor actions.
- Do not import GPUI into `view_models/entity_detail.rs`.

Do not touch:
- Library rendering
- download/playback/playlist behavior
- MusicBrainz lookup behavior

Acceptance criteria:
- Contributor rows show image fallbacks or fetched thumbnails.
- Contributor website and Nostr affordances are visible only when present.
- The lazy contributor panel stores `ContributorView`, not `api::Contributor`.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test contributor`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Rendering contributor images requires moving image-cache access into shared
  projection code.
- Contributor identity data is unavailable from the MusicIndex API response.
