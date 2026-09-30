# ADR 0060 Task 006: Delete The Parked Discover Queries

Status: Ready - 2026-09-30. [Packet 005](adr-0060-task-005-delete-parked-discover-code.md) is complete in the working tree. Implementation has not started.
This packet has no new visual gate. Packet 005 owns the visual check of the live screens.

## Goal

Delete the parked query layer of the old Discover surface and its view models.
Also delete the MusicIndex types and client functions that only that layer uses.
ADR 0075 packet 051 then maps each decoded type to the contract.

## Authority

- [ADR 0060](../adr/0060-workflow-surface-structure.md): Music replaces the Discover surface.
- The working rule in [AGENTS.md](../../AGENTS.md): "Delete dead code. Code no composition root reaches is removed, not parked."
- [ADR 0075 packet 051](adr-0075-task-051-contract-field-guard.md), section "Dispatch Stop": the six decoded types that block the contract guard.

## Recorded Facts - 2026-09-30

The packet 005 measurement records these items. Packet 005 deletes each of their callers.

- `src/view_models/search/`: the view models of the parked Discover search, for example `SearchViewModel`, `LazyPanel` and `ResultRow`.
  The live search uses `src/view_models/search_results/`, a different module.
- `src/application/queries/search.rs`: `DiscoverSearchResults`, `FetchDiscoverSearchResults`, `fetch_discover_search_results`, `fetch_search_batch`, `PartitionedSearchCursor`, `fetch_partitioned_search_batch`, `fetch_typed_search_batch`, `encode_partitioned_search_cursor`, `decode_partitioned_search_cursor`, `fetch_local_library_search_rows`, `fetch_artist_search_batch`, `search_hit_to_result_row`, `enrich_artist_rows`, `fetch_scoped_detail` and `fetch_scoped_track`. About 580 lines.
- `src/application/queries/feed.rs`: `FetchDiscoverRecentFeeds`, `InspectorDetailData`, `ArtistContextData`, `InspectorDetailResult`, `FetchInspectorDetail`, `FetchContributors`, `FetchValueRoutes`, `ResolvePodrollFeeds`, and the functions `fetch_inspector_detail`, `fetch_artist_detail`, `artist_feeds_and_image`, `artist_feed_for_guid`, `fetch_feed_detail`, `fetch_track_detail`, `fetch_scoped_track`, `hydrate_feed_track_play_urls`, `merge_track_play_fields` and `resolve_podroll_feeds`. About 400 to 500 lines, between live functions.
- Packet 005 marked each item that lost its last caller with `expect(dead_code)`, with the reason "ADR 0060 packet 006 deletes this parked query layer". It marked 63 sites in `src/view_models/search/`, `src/application/queries/search.rs`, `feed.rs`, `library.rs` and `images.rs`, and `src/application/commands/metadata.rs`.
- Packet 005 also found dead content in `src/application/request_profiles.rs` that traces to the same deletion.
- `owner_fetch_feed` in `src/application/queries/feed.rs` stays live. `fetch_index_publisher_page_albums` calls it.
- `src/api.rs`:
  - The types `Artist`, `Release`, `Recording`, `ArtistCredit`, `ReleaseReference` and `Source`. MusicIndex contract `0.2.0` declares no schema for them.
  - The `fetch_detail` arms for `/v1/releases/{id}`, `/v1/recordings/{id}` and `/v1/artists/{id}`. The contract declares none of these paths.
  - The client methods `fetch_contributors` and `fetch_value_routes`. Their only callers are `FetchContributors` and `FetchValueRoutes`.
  - The `fuzzy` parameter of `Client::search`. Only the parked search path passes `true`.
- Some parked functions carry doc comments of ADR 0075 packet 018, dated 2026-09-22. `SearchApp` had no caller at that date. The operator confirmed the deletion on 2026-09-30.

## Required Changes

1. Measure again after packet 005, with the method that packet 005 records. Record the unreachable list of the query layer in this packet before any deletion.
2. Delete each unreachable item of "Recorded Facts" and of the new measurement, in `src/view_models/search/`, `src/application/queries/`, `src/application/commands/metadata.rs`, `src/application/request_profiles.rs` and `src/api.rs`.
3. Delete the `fuzzy` parameter of `Client::search`.
4. Delete each test that tests only deleted code, and each guard that names only deleted code. A guard that also covers live code is changed, and keeps its ADR citation. Record each one.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R60-6-01 | The packet records the measurement method and the unreachable list before deletion |
| R60-6-02 | `src/view_models/search/` does not exist, or the packet names the live reader of each item that stays |
| R60-6-03 | `src/api.rs` has no type and no client function for `/v1/releases`, `/v1/recordings` or `/v1/artists` |
| R60-6-04 | `Client::search` has no `fuzzy` parameter |
| R60-6-05 | `src/api.rs` has no `fetch_contributors` and no `fetch_value_routes`, or the packet names a live caller |
| R60-6-06 | A repeat of the measurement lists no unreachable item in the query layer |
| R60-6-07 | No `expect(dead_code)` marker that names ADR 0060 packet 006 stays in `src/` |

## Exclusions

- No change of behavior on a live screen.
- No change to the six other files with `allow(dead_code)`. A later packet measures them.
- No change to `src/view_models/search_results/`.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- The packet 005 document: its measurement method and list.
- `src/view_models/search/`, `src/application/queries/search.rs`, `src/application/queries/feed.rs`, `src/api.rs`.
- `tests/architecture_tests.rs`: each guard that names a deleted item.

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0060-task-006-delete-parked-discover-queries.md`
- The packet 005 document, for the measurement method
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": measure, delete the parked query layer, and delete the unused MusicIndex types and client functions.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Record the unreachable list in this packet before you delete an item.
- Restore each file that the measurement changes. The final diff holds only the deletions and their direct repairs.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The behavior of a live screen.
- `src/view_models/search_results/` and the six other files with `allow(dead_code)`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R60-6-01 to R60-6-07 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- Packet 005 is not complete in the working tree or in the last commit.
- A live screen reads an item of the deletion list.
- A change needs a file in "Do not touch".
