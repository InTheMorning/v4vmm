# ADR 0031 Task 001: Contract Types and Projection Tests

## Status

Completed - 2026-05-01.

## Goal

Add the canonical release-detail page contract in the GPUI-free view-model
layer, adapting existing `ReleaseDetailVm` projections rather than creating a
parallel release-detail system.

## Files To Inspect

- `docs/adr/0031-release-detail-presentation-contract.md`
- `docs/plans/adr-0031-release-detail-presentation-contract-phase-plan.md`
- `src/view_models/entity_detail.rs`
- `src/view_models/mod.rs`
- `src/ui_entity.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/entity_detail.rs`
- `docs/tasks/adr-0031-task-001-contract-types-and-projection-tests.md`

## Do Not Touch

- `src/library.rs`
- `src/search.rs`
- `src/ui_feed.rs`
- `src/db.rs`
- `migrations/`
- download, subscription, playlist, playback, MusicBrainz, or metadata service
  code

## Constraints

- Keep `src/view_models/entity_detail.rs` GPUI-free.
- Import `crate::views` facts, not raw API, DB, UI, screen, or service modules.
- Reuse `ReleaseDetailVm`, `EntitySurfaceContext`, `EntityActionVm`,
  `TrackListVm`, and related existing types where practical.
- Do not move click handlers, image handles, popover state, service calls, or
  database reads into view models.
- Do not change current renderer behavior in this task.
- Preserve source facts; this task decides presentation placement only.

## Implementation Steps

1. Add typed page-contract structures in `src/view_models/entity_detail.rs` for
   hero, primary actions, identity actions, summary facts, panels, and tracks.
2. Add a `ReleaseDetailVm::page()` method, or equivalently named method, that
   projects the full contract from existing `FeedView` data and surface
   context.
3. Ensure the hero contains human-readable identity only: artwork, kind, title,
   subtitle, and an optional short supporting line.
4. Ensure raw website URLs, raw Nostr identifiers, long GUIDs, and multi-line
   descriptions do not appear in hero fields.
5. Ensure summary facts are ordered and capped according to ADR 0031.
6. Ensure the description appears in a panel and not in the hero or summary
   facts.
7. Add focused unit tests for the projection invariants and shared structural
   zones across Library and Discovery.
8. Update this task file's status and implementation summary after the bounded
   implementation lands.

## Acceptance Criteria

- [x] A typed release-detail page contract exists in
  `src/view_models/entity_detail.rs`.
- [x] The contract exposes zones for hero, primary actions, identity actions,
  summary facts, panels, and tracks.
- [x] Projection tests prove the hero excludes raw URLs, `npub` values, long
  GUIDs, and multi-line descriptions.
- [x] Projection tests prove summary facts are ordered and capped.
- [x] Projection tests prove the description appears in a panel and not in the
  hero or summary facts.
- [x] Projection tests prove Website, Nostr, and RSS project as identity
  actions, not primary actions.
- [x] Projection tests prove Discovery and Library produce the same structural
  zones for equivalent release data.
- [x] Architecture tests still enforce the GPUI-free view-model boundary.

## Implementation Summary

Task 001 added a GPUI-free `ReleaseDetailPageVm` projection alongside the
legacy renderer-facing methods. `ReleaseDetailVm::page()` now exposes typed
hero, primary action, identity action, summary fact, panel, and track zones
while reusing existing action and `TrackListVm` projections.

The new hero projection keeps only human-readable identity text and filters raw
URLs, `npub` identifiers, long machine identifiers, and multi-line values.
Description text is projected into a single description panel, identity values
are demoted into an identity panel, Website/Nostr/RSS are projected as identity
actions, and summary facts are ordered and capped.

Verification completed:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::entity_detail`
- `cargo test --test architecture_tests`

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
- `docs/plans/adr-0031-release-detail-presentation-contract-phase-plan.md`
- `docs/tasks/adr-0031-task-001-contract-types-and-projection-tests.md`
- `src/view_models/entity_detail.rs`
- `src/view_models/mod.rs`
- `tests/architecture_tests.rs`

Goal:
- Add a GPUI-free release-detail page contract projection and tests.

Constraints:
- Keep `src/view_models/entity_detail.rs` GPUI-free.
- Reuse existing `ReleaseDetailVm`, `EntitySurfaceContext`, action, identity,
  and track-list types where practical.
- Do not change renderer behavior in this task.
- Do not introduce source-fact inference.

Do not touch:
- `src/library.rs`
- `src/search.rs`
- `src/ui_feed.rs`
- `src/db.rs`
- `migrations/`
- service modules for download, subscription, playlist, playback, MusicBrainz,
  or metadata

Acceptance criteria:
- A typed page contract exposes hero, primary actions, identity actions,
  summary facts, panels, and tracks.
- Hero projection excludes raw URLs, `npub` values, long GUIDs, and multi-line
  descriptions.
- Summary facts are ordered and capped.
- Description appears in one panel and not in hero or summary facts.
- Website, Nostr, and RSS project as identity actions.
- Discovery and Library produce the same structural zones.

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

## Escalation Triggers

- The contract appears to require DB, API-client, service, GPUI, or screen
  imports.
- Renderer changes seem necessary to make the projection compile.
- A source fact must be inferred or discarded to satisfy the tests.
