# ADR 0075 Task 052: Full Include Lists

Status: Complete - 2026-10-07. Mechanical checks Green. This packet has no visual gate. ADR 0083 task 004 V83-41 shows its result on Index track pages.

## Goal

Make each MusicIndex request ask for each collection that its endpoint supports.
An Index track page then gets its payment routes, identity links, credits and publisher, as a Library track page does.

## Authority

- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md), amendment of 2026-10-07: full include lists.
- [ADR 0075 packet 017](adr-0075-task-017-named-request-profiles.md): the request profile registry. This packet replaces its include lists.
- [ADR 0083 task 004](../adr-0083-task-004-one-track-page.md): one track page for both origins.

## Recorded Facts - 2026-10-07

- `src/application/request_profiles.rs` holds the profiles. `INDEX_TRACK_DETAIL_SCOPED` and `INDEX_TRACK_DETAIL_UNSCOPED` send no `include`. The track "The Arbiter" then gives 0 payment routes. With includes it gives 4.
- The stored contract `tests/fixtures/musicindex-openapi-0.7.0.json` lists the supported collections:
  - `/v1/feeds/{guid}`: `tracks`, `payment_routes`, `source_links`, `source_ids`, `source_contributors`, `source_platforms`, `source_release_claims`, `remote_items`, `publisher`.
  - `/v1/feeds/{guid}/tracks/{track_guid}` and `/v1/tracks/{guid}`: `payment_routes`, `value_time_splits`, `source_links`, `source_ids`, `source_contributors`, `source_release_claims`, `source_enclosures`, `source_transcripts`, `remote_items`, `publisher`.
  - `/v1/tracks` and `/v1/search` take no `include`.
- Today's feed lists also send `source_enclosures`, which the feed contract does not list. Keep it in the feed list, because the nested tracks can carry it.
- Measured on 2026-10-07: one track with includes, 2,144 bytes in 0.059 s. The 471-track feed with every collection, 200,251 bytes in 0.096 s.
- No decoder in `src/api.rs` rejects unknown fields.
- Code that reads the tracks of a fetched feed: `src/subscribe_service.rs` (subscribe) and the "Total tracks" fallback in `src/metadata.rs` (`feed.tracks.len()`). The feed update reads its tracks from the database.
- The packet 018 cache keys on the include string. Profiles with one full list share cache entries for one feed.

## Required Changes

1. Replace the include lists with two full lists: one for the feed path shape, and one for both track path shapes.
   Keep `INDEX_NAME_MATCH_TRACKS` without an include, because `/v1/tracks` takes none.
2. Update the R17-02 literal test and the registry comments to the two full lists.
3. Make the "Total tracks" fallback in `src/metadata.rs` give the same value as before for a Library track page. A feed response that now carries `tracks` must not add a track total that the RSS or the stored values do not state. Add a test.
4. Check each caller in the list of "Recorded Facts" with the larger responses. Record each caller and its result.
5. Show the Index track data on its page through the existing `TrackView` fields: payment routes, identity links, credits and publisher. Add no new section.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R52-01 | Each feed profile sends the full feed list, and each track profile sends the full track list |
| R52-02 | `INDEX_NAME_MATCH_TRACKS` sends no include |
| R52-03 | A Library track page gives the same "Total tracks" value as before when the feed response carries `tracks` |
| R52-04 | An Index track decoded from a full response gives its payment routes, identity links, credits and publisher feed GUID in `TrackView` |
| R52-05 | The full suite and the architecture guards stay Green |

## Exclusions

- No new page section and no layout change.
- No database change.

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
cargo build --release --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Result - 2026-10-07

- `src/application/request_profiles.rs` has two lists. `FEED_FULL` holds the nine feed collections and `source_enclosures`. `TRACK_FULL` holds the ten track collections. Each feed profile sends `FEED_FULL`, and each track profile sends `TRACK_FULL`. `INDEX_NAME_MATCH_TRACKS` sends no include.
- `feed_service::library_feed_from_response` drops the `tracks` of a fetched feed before the Library track merge. The Library track total stays the stored count.
- Callers checked with the larger responses:
  - The subscribe reads the `tracks` of the Index feed detail. That request carried `tracks` before, so nothing changes.
  - The feed update reads its tracks from the database, so nothing changes.
  - The "Total tracks" fallback reads no MusicIndex track list on a Library page, because of `library_feed_from_response`.
  - Profiles with one path shape now share one packet 018 cache entry and one evidence resource. The Library album hydration and the Index feed detail are one exact request.
- Tests that asserted the replaced lists were updated or deleted:
  - Deleted: R17-12 (collection counts), R17-13 (no `source_transcripts`), the ADR 0077 test that only two profiles ask for `publisher`, and R3-01 (`publisher` only).
  - Updated: R17-02 became `adr_0075_r52_profiles_send_full_include_lists`. The L0 test now uses the name-match profile. Four request-string tests expect the full lists.
  - `adr_0075_library_observation_interleaved_detail_comparison_and_hydration_share_exact_slots` now expects one evidence state and 6 requests, because the hydration and the comparison feed request are one exact request.
- Proof:
  - R52-01 and R52-02: `adr_0075_r52_profiles_send_full_include_lists`.
  - R52-03: `adr_0075_r52_03_library_feed_takes_no_musicindex_track_list`.
  - R52-04: `adr_0075_r52_04_full_track_response_fills_the_index_track_view` and `from_api_track_keeps_the_named_publisher_feed_guid`.
  - R52-05: the full suite and the architecture guards are Green.
