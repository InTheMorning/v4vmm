# ADR 0028 Task 005 Review

## Result

Pass - 2026-05-01.

## Scope

- Removed duplicated source-fact row mapping between `src/sources.rs` and
  `src/library.rs`.
- Added `src/local_identity.rs` as the shared local SQLite-to-view-fact mapper.
- Added the new mapper to the non-UI core architecture-test path list.
- Marked ADR 0028 and the phase plan implemented.

## Findings

| Area | Observation | Result |
|---|---|---|
| Mapping ownership | `LocalSource` and Library album snapshots now use the same mapper for persisted identity links, ids, and contributors. | Pass |
| Projection boundary | `src/views.rs` and `src/view_models/entity_detail.rs` remain database-free and GPUI-free. | Pass |
| Screen boundary | Library still builds screen-owned album snapshots from DB data, but no longer duplicates source-fact row mapping. | Pass |
| Architecture gates | `src/local_identity.rs` is scanned as non-UI core code, so future UI imports fail architecture tests. | Pass |
| Deferred work | Library contributor visibility was completed by Post-ADR 0028 Task 001. Artist/person reconciliation remains a later ADR. | Accepted |

## Verification

Passed:

```bash
cargo fmt -- --check
cargo check
cargo test sources::tests::local_source_fetch_feed_hydrates_feed_and_track_identity_facts
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Merge Recommendation

Mergeable.
