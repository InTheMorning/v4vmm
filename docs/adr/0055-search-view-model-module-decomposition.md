# ADR 0055: Search view-model module decomposition

## Status

Accepted - 2026-05-18.

## Context

`src/view_models/search.rs` has grown into the largest GPUI-free Discover view
model file. It owns result-row projection, recent-feed tiles, search controls,
inspector action labels, track row actions, deferred panels, playlist append
state, and all related unit tests in one module.

ADR 0050 decomposed adjacent app/workspace/search-results modules while
preserving public import paths through module-root re-exports. The same pattern
now applies to `view_models::search`. This is architectural work because it
changes ownership boundaries inside a load-bearing view-model surface used by
parked Discover code and active search shells.

## Decision

Convert `src/view_models/search.rs` into `src/view_models/search/mod.rs` and
extract focused sibling modules under `src/view_models/search/`.

The root `view_models::search` module remains the public import surface for
callers. Submodules are private implementation details and are re-exported from
`mod.rs` where existing callers already depend on the type or function. Callers
must continue to import through `crate::view_models::search::{...}` rather than
deep module paths.

Initial ownership split:

- `results.rs` owns search result rows, display rows, navigation identity, type
  visibility, query normalization, feed title fallbacks, and derived artist rows.
- `recent.rs` owns recent-feed tile display and recent-feed root/list display
  contracts.
- `controls.rs` owns search pane, status, type-filter, section, and render
  snapshot display contracts.
- `actions.rs` owns inspector action-row labels, subscription command messages,
  and playlist append projections.
- `feed_detail.rs` owns publisher, payment-route, feed-list, and feed-inspector
  detail projections.
- `track.rs` owns track row actions and track inspector header/feed-link
  projections.
- `lazy.rs` owns lazy/deferred panel state and deferred-panel display labels.
- `tests.rs` owns the search VM unit tests.

`SearchViewModel` stays in `mod.rs` for this slice because it coordinates state
from multiple focused modules. A later ADR may split state if it has a coherent
consumer boundary.

## Invariants

- File-organization-only refactor. No behavior, strings, element ids, status
  messages, query semantics, Recent Feeds behavior, or parked Discover behavior
  changes.
- No root `src/view_models/search.rs` remains after the split.
- `src/view_models/search/mod.rs` exists and remains GPUI-free.
- Every Rust file under `src/view_models/search/` remains GPUI-free and does
  not import UI, screen, or app modules.
- Existing consumers import through `crate::view_models::search`, not private
  submodule paths.
- Parked Discover code continues to compile through the same
  `view_models::search` surface.
- The split does not revive the retired top-level `src/search.rs` route.

## Non-Goals

- No redesign of Discover/Search state.
- No renderer changes.
- No new view-model display fields.
- No migration of parked Discover into an active UI route.
- No service, database, or application-layer changes.

## Alternatives Considered

- **Rename the old file to `search_legacy.rs`.** Rejected. A legacy bucket keeps
  ownership unclear and fails the decomposition goal.
- **Split `SearchViewModel` immediately by state domain.** Deferred. The state
  object coordinates cross-module display contracts, and splitting it now would
  risk behavior churn without a new consumer boundary.
- **Make submodules public and update callers to deep imports.** Rejected.
  Deep imports expose private ownership details and create churn in UI shells.

## Consequences

Positive:

- Search view-model ownership is easier to review and assign.
- Future changes can target one focused module without reading a 4k-line file.
- Architecture guards can enforce the new tree and GPUI-free boundary.

Negative / risks:

- `git blame` continuity requires following moved code across submodules.
- Root re-export mistakes can break parked Discover callers even when module
  internals compile. The decomposition guard covers this import discipline.

## Follow-Up Work

- Consider a later state-focused ADR if `SearchViewModel` itself grows new
  unrelated responsibilities.
- Keep architecture tests updated to inspect the search module tree instead of a
  retired single file path.

## References

- ADR 0023 - Design system and view-models
- ADR 0038 - Presentation contract enforcement
- ADR 0047 - Library and search unification
- ADR 0050 - Post-ADR-0048 module decomposition
- `docs/plans/adr-0050-module-decomposition-phase-plan.md`
