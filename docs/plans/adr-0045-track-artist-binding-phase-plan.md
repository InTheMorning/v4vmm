# ADR 0045 Track-to-Artist Binding Phase Plan

## Goal

Complete the top deferred architecture item by adding explicit, provenance-first
track-to-artist bindings so Library artist views can use stored artist facts
without name-only merging.

## Non-Goals

- Do not implement person/global identity persistence.
- Do not infer artist subjects from names, filenames, or tags.
- Do not create a canonical artist registry.
- Do not change audio tag writes.
- Do not refactor unrelated Library or Discover UI.

## Assumptions

- ADR 0029 artist source-fact tables and helpers remain the source of explicit
  artist facts.
- MusicIndex artist ids are the first supported binding source.
- Local Library artist grouping by display name remains valid for tracks with
  no explicit binding.

## Affected Modules

- `src/db.rs`
- `src/identity_ingest.rs`
- `src/sources.rs`
- `src/views.rs`
- `src/view_models/artist.rs`
- `src/view_models/library.rs`
- `src/library/app_impl.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

1. `adr-0045-task-001-track-artist-binding-schema`
   - Add additive binding schema and DB helper tests.
2. `adr-0045-task-002-musicindex-binding-ingest`
   - Persist bindings only from explicit MusicIndex artist ids.
3. `adr-0045-task-003-library-artist-hydration`
   - Enrich Library artist read models from explicit bindings.
4. `adr-0045-task-004-guards-and-readiness`
   - Add architecture guards, run full checks, and update the review
     checklist.

## Schema/API Implications

The schema change must be additive. A binding table is preferred over a
`tracks` column because one track can have multiple artist roles and future
source-specific subjects.

No public API change is required for the first pass. MusicIndex ingest may only
write a binding when an explicit artist id already appears in the response.

## Risk Areas

- Accidentally merging distinct artists that share a display name.
- Treating a contributor/person row as a durable artist subject.
- Making renderer code decide binding or conflict behavior.
- Breaking feed/track removal by cascading artist source facts instead of only
  local binding rows.

## Test Strategy

- DB migration/helper tests for binding insert, replace, delete, and required
  explicit keys.
- Ingest tests proving name-only artists do not create bindings.
- Local source and `ArtistView` tests for enriched and split-subject cases.
- Architecture tests preventing screen-side binding inference.
- Final gate:

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

## Rollback Strategy

Because the schema is additive, rollback can disable ingest writes and ignore
binding rows in local read models. Existing artist source facts and Library
name-derived artist views continue to work.
