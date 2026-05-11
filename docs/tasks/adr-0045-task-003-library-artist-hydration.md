# ADR 0045 Task 003: Library Artist Hydration

## Goal

Use explicit track-to-artist bindings to enrich Library artist views without
merging artists by display name.

Status: Implemented - 2026-05-11.

## Files to Inspect

- `docs/adr/0045-track-artist-binding.md`
- `docs/tasks/adr-0045-task-001-track-artist-binding-schema.md`
- `docs/tasks/adr-0045-task-002-musicindex-binding-ingest.md`
- `src/sources.rs`
- `src/views.rs`
- `src/view_models/artist.rs`
- `src/view_models/library.rs`
- `src/library/app_impl.rs`

## Files Likely to Change

- `src/sources.rs`
- `src/views.rs`
- `src/view_models/artist.rs`
- `src/view_models/library.rs`
- `src/library/app_impl.rs`

## Do Not Touch

- `src/ui/**` except shell wiring required to consume existing VMs
- `src/db.rs` beyond helpers from Task 001
- Audio tag write paths

## Constraints

- Read models own enrichment and conflict display facts.
- Screens must not infer bindings from names.
- Multiple explicit subjects under one local display name must stay visible as
  separate facts or a conservative conflict state.
- Tracks without bindings keep current behavior.

## Implementation Steps

1. Done: add query/helper usage to collect bindings for tracks in a local
   artist view.
2. Done: project enriched scalar facts only from bound source facts.
3. Done: preserve local name-derived grouping for tracks without bindings.
4. Done: add tests for no-binding, single-binding, and multi-subject cases.

## Acceptance Criteria

- [x] A name-derived Library artist with bound tracks can show stored image,
  website, aliases, area, and active years.
- [x] A name-derived Library artist without bindings is unchanged.
- [x] Multiple bound subjects are not silently merged.
- [x] Renderer code remains binding-policy-free.

## Implementation Notes

- `LocalSource` now enriches `ArtistRef::LocalArtistName` from explicit
  bindings for the already-selected local tracks only.
- `ArtistView::from_local_rows_with_artist_source_facts()` preserves local
  name/count identity and overlays scalar source facts only when exactly one
  explicit subject is present.
- Multiple bound subjects are carried as conservative source-subject facts
  without projecting one subject's scalar fields as canonical.
- Library artist detail consumes the enriched `ArtistView` through the existing
  artist detail shell and VM contracts; no renderer performs binding lookup or
  name matching.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test artist_source_fact
cargo test library_artist
cargo test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0045-track-artist-binding.md`
- `src/sources.rs`
- `src/views.rs`
- `src/view_models/artist.rs`
- `src/view_models/library.rs`
- `src/library/app_impl.rs`

Goal:
- Enrich Library artist views from explicit track-to-artist bindings.

Constraints:
- No name matching.
- No renderer-side binding inference.
- Preserve current behavior for unbound artists.

Do not touch:
- Audio tag write paths
- Unrelated Search/Discover UI

Acceptance criteria:
- Bound artists enrich; unbound artists stay unchanged; multi-subject cases are
  conservative.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test artist_source_fact`
- `cargo test library_artist`
- `cargo test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The UI needs a new conflict disclosure component.
- Multiple bound subjects require product copy not present in existing VMs.
