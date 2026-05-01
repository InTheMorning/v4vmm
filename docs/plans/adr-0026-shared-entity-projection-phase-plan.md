# ADR 0026 Shared Entity Projection Phase Plan

## Goal

Make Library and Discover render shared entity content from the same
GPUI-free projections while preserving MusicIndex identity facts for artists,
feeds/items/tracks, and contributors.

## Non-Goals

- Do not redesign the visual style beyond making existing content consistent.
- Do not move service fetching into shared UI code.
- Do not replace ADR 0024 application services.
- Do not implement fuzzy artist/entity identity reconciliation.
- Do not change database schema unless local identity preservation requires it.

## Current State

- `src/views.rs` contains `ArtistView`, `FeedView`, and `TrackView`, but
  identity fields are incomplete and contributor identity is not first-class.
- `src/api.rs` models feed/track `source_links` and `source_ids`, but
  contributor identity fields from the newer MusicIndex API are under-modeled.
- Current view facts expose API-shaped contributor/source facts directly in
  places. ADR 0026 requires a local source-fact contract before shared
  projection adoption.
- Discover feed detail renders through `src/ui_feed.rs` and shared
  `ReleaseDetailSurface`.
- Library album detail also uses `ReleaseDetailSurface`, but still builds
  separate action rows and track-row semantics in `src/library.rs`.
- `src/ui_track.rs` contains a Discover track-row adapter, while Library has a
  separate local track-row path.
- Nostr and website affordances are screen-local and feed/track oriented.

## Target State

- `src/views.rs` preserves identity facts consistently for artists, feeds,
  tracks, and contributors without exposing concrete API row structs as public
  shared fact fields.
- `src/view_models/entity_detail.rs` exposes shared, pure projections for
  entity headers, release detail, track lists, identity links, contributors,
  and action descriptors.
- `src/ui_entity.rs` provides slot-based shells over shared projections without
  importing `SearchApp`, `LibraryApp`, or services.
- Discover and Library render feed/album detail through the same projection
  contract.
- Context differences are expressed as action descriptors and screen adapters,
  not as separate layout branches.
- Architecture tests prevent GPUI/service imports from entering the shared
  projection module.

## Affected Modules

- `src/api.rs`
- `src/views.rs`
- `src/view_models/mod.rs`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/ui_track.rs`
- `src/search.rs`
- `src/library.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

### Phase 1 — Identity Facts

Status: Implemented.

Task: `docs/tasks/adr-0026-task-001-identity-facts.md`

Extend API and view fact types so MusicIndex identity data is deserialized and
preserved. Add local source fact types, `EntityIdentityLinks`, and
`ContributorView`. Feed and track view facts stop exposing `api::Contributor`.
Add extraction tests for Nostr, website, image, and raw source fact
preservation.

### Phase 2 — Shared Projection VMs

Status: Implemented.

Task file to create after Phase 1 review:
`docs/tasks/adr-0026-task-002-shared-projection-vms.md`.

Add `src/view_models/entity_detail.rs` with pure `ReleaseDetailVm`,
`IdentityLinksVm`, `ContributorListVm`, `TrackListVm`, `SharedTrackRowVm`,
`EntityActionVm`, `EntityActionTarget`, and `EntitySurfaceContext`. Add
architecture tests blocking GPUI, UI, API-client, screen, and service imports.

### Phase 3 — Slot-Based UI Shells

Status: Implemented.

Task file to create after Phase 2 review:
`docs/tasks/adr-0026-task-003-slot-based-ui-shells.md`.

Add `src/ui_entity.rs` shell functions that place projected headers, detail
grids, identity slots, contributor sections, and track-row slots using ADR
0023/0025 composites. The shell must not import `search`, `library`, or
services; screen adapters provide action elements through explicit slots or
binder structs.

### Phase 4 — Discover Adoption

Status: Implemented.

Task file to create after Phase 3 review:
`docs/tasks/adr-0026-task-004-discover-projection-adoption.md`.

Route Discover feed and track detail rendering through the shared projections
and slot-based shell without changing behavior. Keep existing action handlers
and async dispatch in `src/search.rs`.

### Phase 5 — Library Adoption

Status: Implemented.

Task file to create after Phase 4 review:
`docs/tasks/adr-0026-task-005-library-projection-adoption.md`.

Route Library album detail through the same release projection. Library keeps
its own handlers but maps shared action descriptors to remove, playlist,
playback, compare, and MusicBrainz affordances. Remove "downloaded" row text
where remove already implies membership.

### Phase 6 — Contributor Identity UI

Task file to create after Phase 5 review:
`docs/tasks/adr-0026-task-006-contributor-identity-ui.md`.

Render contributor images, webpages, and Nostr identity actions through shared
identity-action composites. Use ADR 0025 icon/control roles.

### Phase 7 — Cleanup and Gates

Task file to create after Phase 6 review:
`docs/tasks/adr-0026-task-007-cleanup-and-gates.md`.

Remove obsolete screen-local projection helpers, tighten architecture tests,
and update ADR status if all green criteria are met.

## Schema / API Implications

- API structs must deserialize newer MusicIndex contributor identity fields.
- View fact structs preserve raw `source_links` and `source_ids` through local
  `IdentityLinkFact` and `IdentityIdFact` types rather than public `api::*`
  fields.
- Feed and track view facts expose `ContributorView` collections rather than
  API contributor rows.
- No database migration is expected in the first slice.
- If local Library data cannot preserve an identity fact already in SQLite,
  that gap must be documented before any schema migration is proposed.

## Risk Areas

- Accidentally moving fetch/query logic into shared projection or UI modules.
- Losing raw provenance while populating convenience fields.
- Reintroducing two button/action vocabularies while trying to unify rows.
- Making generic UI functions too tightly coupled to `SearchApp` or
  `LibraryApp`.
- Changing Library removal semantics while visual row changes are in flight.

## Test Strategy

- Unit tests for API deserialization of contributor identity fields.
- Unit tests for `EntityIdentityLinks` extraction and raw fact preservation.
- Unit tests for shared projection labels, summaries, and action descriptors.
- Architecture tests preventing GPUI/service imports from shared projections.
- Architecture tests preventing `src/ui_entity.rs` from importing screen or
  service modules.
- Existing `cargo test` coverage for Library/Discover view-model behavior.
- Manual visual smoke after Discover and Library adoption phases.

Required verification before each implementation commit:

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

Run broader `cargo test` before marking the ADR implemented.

## Rollback Strategy

Each phase is additive until a screen is migrated. If a phase regresses
behavior, revert that phase's commit and keep the prior screen-local rendering
path. Do not partially retain a shared projection call site that silently
changes action semantics.

## Green Criteria

- `ArtistView`, `FeedView`, `TrackView`, and `ContributorView` preserve
  identity facts needed by the current MusicIndex API.
- Shared view fact fields do not expose concrete MusicIndex API row structs as
  public identity/provenance fields.
- Feed and track view facts expose contributor identity through
  `ContributorView`.
- `ReleaseDetailVm` and shared track-row projections are GPUI-free and covered
  by unit tests.
- `src/ui_entity.rs` uses slot-based action binding and imports no screen or
  service modules.
- Discover and Library render the same feed/album content through the same
  projection contract.
- Library no longer displays redundant "downloaded" text for tracks already in
  Library.
- Repeated destructive row actions use quiet ADR 0025 control roles.
- Architecture tests prevent GPUI and service imports in shared projection
  modules.
