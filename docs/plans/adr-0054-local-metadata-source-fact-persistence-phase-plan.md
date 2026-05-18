# ADR 0054 Local Metadata Source-Fact Persistence Phase Plan

## Status

Implemented - 2026-05-18.

## Goal

Create a provenance-preserving local persistence path for feed and track
metadata facts that ADR 0024 could not surface through loading-shape work.

## Non-Goals

- No renderer-only fallback labels.
- No UI detail changes before query/VM hydration exists.
- No artist biography or playlist metadata implementation.
- No source-derived release-kind mapping.
- No migration of historical scalar rows in the first task.

## Current State

- `src/db.rs` stores feed language, description, podcast medium, track pubdate,
  and track explicit state in entity columns.
- ADR 0028 identity helpers persist source links, ids, and contributors.
- ADR 0029 artist helpers persist explicit artist subject facts.
- `src/api.rs` carries MusicIndex feed/track metadata fields that are currently
  only available while remote objects remain in memory.

## Target State

Feed and track metadata facts can be written, replaced by source, and loaded
from local SQLite without involving GPUI renderers or collapsing sources.
Later VM tasks can project selected facts into `FeedView`, `TrackView`, and
detail VMs.

## Affected Modules

- `src/db.rs`
- `src/identity_ingest.rs`
- `src/local_identity.rs` or a new adjacent metadata mapping module
- `src/feed_service.rs`
- `src/subscribe_service.rs`
- `src/views.rs`
- `src/view_models/feed.rs`
- `src/view_models/track_detail.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

1. **Schema and DB helpers.**
   Add `entity_metadata_facts`, typed input/row structs, source-scoped replace
   and owner load helpers, and DB unit tests. No ingest or UI changes.
2. **MusicIndex feed metadata ingest.**
   Persist feed-level MusicIndex facts from `api::Feed`: publisher,
   release kind, release date, language, explicit, description, and preserved
   release description claims where present.
3. **MusicIndex track metadata ingest.**
   Persist track-level MusicIndex facts from `api::Track`: publisher,
   description, pubdate, explicit, and optional language/annotation only when
   product scope is explicitly resolved.
4. **Feed read-model hydration.**
   Hydrate `FeedView::from_local_with_identity` or a new local fact mapper from
   persisted metadata facts. Preserve scalar-column fallbacks only where the
   ADR says they are source facts.
5. **Track read-model hydration.**
   Hydrate local track detail metadata rows from persisted track facts without
   renderer inference.
6. **Readiness guard.**
   Lock table boundaries, fact-key ownership, source replacement semantics, and
   no UI/direct DB coupling with architecture tests.

## Schema And API Implications

The first schema is additive. Existing tables and rows continue to work. No
API response shape changes are required. The MusicIndex fields already exist
on `api::Feed` and `api::Track`; ingest tasks map those fields into typed local
facts.

## Risk Areas

- Accidentally treating scalar columns as provenance-equivalent to source
  facts.
- Collapsing `podcast_medium` into `release_kind`.
- Replacing all sources for an owner when only one source refreshed.
- Letting renderers hide or invent fields.
- Extending ADR 0028 identity tables with non-identity metadata.

## Test Strategy

- DB unit tests for schema creation, owner-shape checks, typed value checks,
  source-scoped replacement, and cascade behavior.
- Unit tests for each ingest mapper.
- VM projection tests before any Library field is rendered.
- Architecture tests that block UI and view-model layers from querying the
  metadata fact table directly.
- Standard gates: `cargo fmt -- --check`, `cargo check --quiet`,
  `cargo test --lib --quiet`, `cargo test --test architecture_tests --quiet`,
  and `cargo clippy --quiet -- -D warnings`.
- Manual visual smoke remains operator-run only.

## Rollback Strategy

Each task is additive and independently revertible. If a fact key or display
policy proves wrong, revert the task that introduced that key or projection
rather than masking the field in a renderer.

## Open Questions

- Which exact feed fact should become the primary displayed release kind:
  MusicIndex `release_kind`, RSS `podcast_medium`, or a source-priority policy?
- Is track language in product scope, or should language remain feed-level?
- Are lyrics and annotations a track metadata field, a transcript field, or a
  future richer text source fact?
- Should existing scalar columns be backfilled into source facts, and under
  which source token?

## References

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/adr/0053-local-detail-source-fact-parity.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
