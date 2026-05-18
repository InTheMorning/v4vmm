# ADR 0054 Task 003 Review: MusicIndex Track Metadata Ingest

Date: 2026-05-18

## Reviewed Artifacts

- `docs/tasks/adr-0054-task-003-musicindex-track-metadata-ingest.md`
- `src/identity_ingest.rs`
- `src/subscribe_service.rs`

## Result

Pass.

The implementation persists MusicIndex track metadata facts for the approved
Task003 keys and keeps feed-default copied metadata out of track-owned facts.
The post-implementation review found one public search-track fallback risk:
callers could pass already defaulted display/download contexts. The final patch
keeps the fix inside `subscribe_service`: explicit non-defaulted payloads are
used directly, public search-track callers fetch an authoritative MusicIndex
track by GUID before persistence, and unavailable authoritative payloads skip
track persistence instead of writing ambiguous context metadata.

## Architectural Drift

None observed.

- No UI, view-model, Discover, RSS, or schema files changed.
- Track fact persistence remains behind `identity_ingest`.
- `subscribe_service` owns the boundary between display/download enrichment and
  source-fact persistence.
- The lower-level persistence helper remains source-scoped through
  `replace_local_metadata_facts`.

## Regression Guards

- Track-level fact persistence covers `publisher_text`, `description`,
  `pub_date`, and `explicit`.
- Empty text facts are skipped and `explicit = false` is preserved.
- MusicIndex replacement preserves RSS/other-source metadata rows.
- Feed-default publisher/description are not written as track facts.
- Public search-track fallback does not persist defaulted context metadata when
  authoritative MusicIndex fetch fails.
- Fetched authoritative tracks merge only track matching fields from the
  display/download context, not metadata fact fields.

## Verification

- `git diff --check`
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo build --quiet`
- `cargo test identity_ingest::tests::musicindex_track_metadata --lib --quiet`
- `cargo test subscribe_service::tests::authoritative_persistence --lib --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Manual Smoke

Not run. This task is a source-fact persistence slice and does not change UI
rendering; GUI smoke remains operator-owned.

## Merge Recommendation

Merge as ADR 0054 Task003.
