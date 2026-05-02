# ADR 0031 Release Detail Presentation Contract Phase Plan

## Goal

Make Library and Discovery release-like detail pages render from one typed,
GPUI-free presentation contract that separates hero identity, actions, summary
facts, optional panels, and tracks.

## Non-Goals

- No navigation redesign.
- No database, migration, or API shape change.
- No metadata persistence or identity-ingest change.
- No change to MusicBrainz, download, playlist, playback, or subscription
  semantics.
- No generalized visual redesign outside release-like detail pages.

## Assumptions

- ADR 0031 is the architecture source of truth.
- Existing `ReleaseDetailVm`, `ReleaseDetailSlots`, `TrackListVm`, and
  `EntitySurfaceContext` should be adapted before introducing parallel systems.
- Screen modules continue to own GPUI handlers, image handles, popovers, async
  service calls, and command dispatch.
- Source facts must remain available even when demoted out of the hero.

## Affected Modules

- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/release_detail_surface.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

### Phase 1 - Contract Types and Projection Tests

Status: Completed.

Task: `docs/tasks/adr-0031-task-001-contract-types-and-projection-tests.md`

Adapt `src/view_models/entity_detail.rs` so `ReleaseDetailVm` exposes a
canonical page contract containing hero, primary actions, identity actions,
summary facts, panels, and track section data. Add focused unit tests proving
hero exclusions, summary ordering/capping, single description placement, and
shared structural zones across Library and Discovery.

### Phase 2 - Renderer Adoption

Status: Planned.

Task: `docs/tasks/adr-0031-task-002-renderer-adoption.md`

Update the shared release-detail shell to consume `ReleaseDetailPageVm`
directly. Retire or narrow `ReleaseDetailSlots` so slots cannot carry hero,
description, summary, or other placement decisions that belong to the contract;
screen modules may still inject callbacks, image handles, and action elements.

### Phase 3 - Track Section Parity

Status: Planned.

Task: `docs/tasks/adr-0031-task-003-track-section-parity.md`

Normalize the visual row template of the track section across Library and
Discovery. Scope is row geometry only; Task 002 owns the section structure.

### Phase 4 - Visual Smoke and Cleanup

Status: Planned.

Task: `docs/tasks/adr-0031-task-004-visual-smoke-and-cleanup.md`

Run representative Library and Discovery visual smoke against the ADR fixture
list, attach or reference screenshots from a review document, verify
screen-owned behavior still triggers, and remove obsolete screen-local
composition paths introduced by earlier fixes.

## Schema / API Implications

No schema or API change is expected. If a task appears to require one, stop and
write a follow-up ADR before implementation.

## Risk Areas

- Reintroducing raw website, Nostr, GUID, or description values into the hero.
- Rendering Website, Nostr, or RSS as primary actions instead of identity
  actions.
- Moving command dispatch, service calls, or image-cache lookup into the
  GPUI-free projection layer.
- Creating a parallel release-detail system instead of adapting existing
  projections and slots.
- Letting Library and Discovery diverge through local renderer overrides.
- Keeping `ReleaseDetailSlots` broad enough to override hero, description, or
  summary placement after Task 002.
- Hiding source facts rather than demoting them into panels or copy/open
  actions.
- Introducing nested vertical scroll views in the detail surface.

## Execution Guidance From ADR 0031

- Treat `ReleaseDetailPageVm` as the product contract, not as a convenience
  helper.
- Renderers bind contract zones; they do not classify raw metadata fields.
- Slot APIs may inject behavior and already-resolved UI assets, but they must
  not decide hero, summary, description, or panel placement.
- Website, Nostr, and RSS are identity actions. They are never primary release
  actions and never hero metadata rows.
- Source facts are preserved by demotion into panels or copy/open actions, not
  by first-viewport display.
- Library and Discovery may differ by surface policy and action availability
  only; they share the page skeleton.
- Any need for schema, API, service, or metadata-ingest changes stops this ADR
  and requires a follow-up decision.

## Test Strategy

- `cargo test view_models::entity_detail` for projection invariants.
- `cargo test --test architecture_tests` for layer boundary enforcement.
- `cargo check` for compile verification after each bounded task.
- `cargo clippy --lib --tests -- -D warnings` before merge-ready completion.
- Manual visual smoke after renderer adoption and track-section parity.
- Visual smoke fixture coverage for releases with Website/Nostr/description,
  empty description, zero tracks, 100+ tracks, podcast/RSS-only identity, and
  local-file metadata.

## Rollback Strategy

- Phase 1 can be rolled back by reverting the additive page-contract types and
  tests while retaining the existing `ReleaseDetailVm` methods.
- Phase 2 can be rolled back by restoring `ui_entity` default rendering to the
  pre-contract `header`, `detail_rows`, and `track_list` methods.
- Phase 3 can be rolled back by restoring the previous track-row slot adapter
  behavior.
- Phase 4 cleanup should only remove code after screenshots and tests prove the
  contract path is active.
