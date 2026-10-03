# ADR 0060 Task 006: Delete The Parked Discover Queries

Status: Implemented - 2026-09-30. Mechanical checks Green. This packet has no new visual gate.

## Goal

Delete the parked query layer of the old Discover surface and its view models.
Also delete the MusicIndex types and client functions that only that layer uses.
ADR 0075 packet 051 then maps each decoded type to the contract.

## Authority

- [ADR 0060](../../adr/0060-workflow-surface-structure.md): Music replaces the Discover surface.
- The working rule in [AGENTS.md](../../../AGENTS.md): "Delete dead code. Code no composition root reaches is removed, not parked."
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

- [Agent rules](../../../AGENTS.md) and the [source map](../../../.github/copilot-instructions.md).
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

## Implementation Result - 2026-09-30

### Measurement Method

I copied `src/lib.rs` to a scratch folder. I changed three top-level module
lines to `pub(crate)`: `api`, `application`, and `view_models`. This is the
method packet 005 records. I ran `cargo check --lib` and read the warning
list.

This method found only the one item that no `expect(dead_code)` marker
already covered: an unused-import warning in `src/view_models/search/mod.rs`.
Every other item in "Recorded Facts" already carries a marker, so the
compiler stays silent about it. An `expect(dead_code)` marker suppresses the
warning on its own item, but the item's body still counts as a real caller
of whatever it calls. A flip-and-check pass cannot see through a marked
item to find what only that item still calls.

I traced each such case by hand with `grep`, across the whole `src` tree,
the same way packet 005 traced its own findings. I confirmed each item
in the list below has no caller outside the marked, dead code. I restored
`src/lib.rs` from the scratch copy after the measurement. `git status`
shows no change to it.

### Unreachable List Before Deletion

The 63 marked sites of "Recorded Facts", each traced to zero callers outside
other marked sites:

- `src/view_models/search/`: all ten files, 4,422 lines. No file outside
  this directory imports from it, once `src/application/queries/search.rs`
  loses its own import of it.
- `src/application/queries/search.rs`: `SharedConnection`,
  `DiscoverSearchResults`, `FetchDiscoverSearchResults` and its `new`,
  `fetch_discover_search_results`, `fetch_search_batch`,
  `PartitionedSearchCursor`, `fetch_partitioned_search_batch`,
  `fetch_typed_search_batch`, `encode_partitioned_search_cursor`,
  `decode_partitioned_search_cursor`, `fetch_local_library_search_rows`,
  `fetch_artist_search_batch`, `search_hit_to_result_row`,
  `enrich_artist_rows`, `fetch_scoped_detail`, its own `fetch_scoped_track`,
  and `bounded_i32_count`.
- `src/application/queries/feed.rs`: `FetchDiscoverRecentFeeds` and its
  `new`, `InspectorDetailData`, `ArtistContextData`, `InspectorDetailResult`,
  `FetchInspectorDetail` and its `new`, `FetchContributors` and its `new`,
  `FetchValueRoutes` and its `new`, `ResolvePodrollFeeds` and its `new`,
  `fetch_inspector_detail`, `fetch_artist_detail`, `artist_feeds_and_image`,
  `artist_feed_for_guid`, `fetch_feed_detail`, `fetch_track_detail`, its own
  `fetch_scoped_track`, `resolve_podroll_feeds`,
  `hydrate_feed_track_play_urls`, `merge_track_play_fields`, its own
  `nonempty_url`, and `bounded_i32_count`.
- `src/application/queries/library.rs`: `LocalTrackContextResult`,
  `FetchLocalTrackContext` and its `new`, `fetch_local_track_context`, and
  its own `nonempty_url`.
- `src/application/queries/images.rs`: `DownloadInspectorImage` and its
  `new`.
- `src/application/commands/metadata.rs`: `LookupRemoteMusicBrainzTrack` and
  its `new`, and `DownloadAndCompareTrack` and its `new`.
- `src/api.rs`:
  - The types `Artist`, `Release`, `Recording`, `ArtistCredit`,
    `ReleaseReference`, and `Source`.
  - The whole `fetch_detail` function. Its only caller was
    `fetch_scoped_detail`, above.
  - The `fetch_contributors` and `fetch_value_routes` methods.
  - The `fuzzy` parameter of `Client::search`.

The trace found four further items with no caller left, once the list
above is gone. None carried a marker. Each stayed reachable only
through a marked item's own body:

- `src/api.rs`: the `Track.artist_credit` field. Its type, `ArtistCredit`,
  is in the deletion list. Its only reader was a test in `src/api.rs`
  itself. No live code reads it.
- `src/api.rs`: the `EntityDetail::Track(Track)` case. Its only
  constructors were `fetch_detail`, `fetch_contributors`,
  `fetch_value_routes`, all deleted above, and the parked query layer.
  `Track` the type stays. Only this wrapper case goes.
- `src/application/request_profiles.rs`: the `INSPECTOR_TRACK_DETAIL_TRACK`
  and `INSPECTOR_TRACK_DETAIL_FEED` profiles. Their only owner was
  `feed::fetch_track_detail`, above. The `L3` and `L4` include-list
  constants those two profiles alone read. The `ScopedOrUnscopedTrack`
  path-shape variant. Its only constructor was
  `INSPECTOR_TRACK_DETAIL_TRACK`.
- `src/views.rs`: `ArtistView::from_api`. Its parameter type is
  `api::Artist`, in the deletion list. No code in the crate called this
  method. The live `ArtistView` constructor is `from_local_rows`.

### Files Deleted Or Changed

Deleted:

| File | Lines |
|---|---|
| `src/view_models/search/` (10 files) | 4,422 |

Changed:

- `src/view_models/mod.rs`: removed `pub mod search;`, and corrected the
  module doc comment's screen list to name `search_results` instead.
- `src/application/queries/search.rs`: 1,770 to 1,273 lines. Deleted the
  unreachable list above. Removed the now-unused imports
  `crate::view_models::search::{...}`, `crate::feed_service`,
  `crate::view_models::workspace::ContentFilter`, and the bare `Client` and
  `PAGE_LIMIT` names from the `crate::api` import (each live call site
  already used the fully qualified path). Moved the test-only `Arc`/`Mutex`
  import into the one test module that uses it, since production code no
  longer does. Updated the two live `Client::search` call sites
  (`fetch_index_feed_result_rows`, `fetch_index_track_result_rows`) to drop
  the removed `fuzzy` argument.
- `src/application/queries/feed.rs`: 1,284 to 577 lines. Deleted the
  unreachable list above. Removed these now-unused imports:
  - `Artist`, `Contributor`, `PaymentRoute`, `RecentFeedsResponse`, and
    `Track`, from the `crate::api` import.
  - The whole `crate::metadata` import.
  - `crate::subscribe_service::enrich_track_context_from_rss`.
  - `crate::view_models::track::TrackVm`.
  - `crate::rss`.
  - `INSPECTOR_TRACK_DETAIL_FEED` and `INSPECTOR_TRACK_DETAIL_TRACK`, from
    the `request_profiles` import.
  - `BTreeMap`.
- `src/application/queries/library.rs`: 3,666 to 3,531 lines. Deleted the
  unreachable list above.
- `src/application/queries/images.rs`: 192 to 128 lines. Deleted the
  unreachable list above, and its now-unused `image_from_bytes` and
  `download_image` imports.
- `src/application/commands/metadata.rs`: 406 to 312 lines. Deleted the
  unreachable list above, and its now-unused `crate::api::Client` import.
- `src/api.rs`: 3,549 to 3,398 lines. Deleted the unreachable list above
  and the four further items. Simplified the two `debug_assert!` path-shape
  checks in `fetch_track_with_profile` and `fetch_feed_track_with_profile`,
  since each now names only one possible path shape.
- `src/application/request_profiles.rs`: 403 to 363 lines. Deleted the two
  profiles, the two include-list constants, and the path-shape variant.
  Corrected the module doc comment's profile count from ten to eight, and
  the `INDEX_FEED_DETAIL` doc comment's call-site list from three to two.
- `src/views.rs`: 1,318 to 1,296 lines. Deleted `ArtistView::from_api`.

### Tests And Guards Deleted Or Changed

Deleted, because each tested only deleted code:

1. `src/application/queries/images.rs`:
   `download_inspector_image_downloads_uncached_image`.
2. `src/application/queries/feed.rs`:
   `adr_0075_request_profile_inspector_feed_detail_sends_l2` and
   `adr_0075_request_profile_inspector_track_detail_sends_l3_then_l4`.
3. `tests/architecture_tests.rs`:
   `adr_0055_search_view_model_is_decomposed_under_module_tree`, with its
   own two helpers `search_vm_sources` and `search_vm_source`. No other
   guard called these two helpers once this one test was gone.

Changed, each keeping its ADR citation:

1. `src/application/queries/images.rs`'s
   `image_queries_honor_cancelled_context`: dropped its
   `DownloadInspectorImage` half, kept its `FetchThumbnail` half.
2. `src/application/queries/feed.rs`'s
   `adr_0077_publisher_relationship_index_feed_detail_request_count_is_unchanged`
   (ADR 0077 Decision 5): dropped its `fetch_feed_detail` half, kept its
   `fetch_recent_feed_result_rows` half and the `publisher` assertion.
3. `src/application/queries/library.rs`'s
   `adr_0075_snapshot_live_commands_return_empty_and_retained_failed_refresh`
   (ADR 0075): dropped the middle block that read the same retained state
   twice through `FetchLocalTrackContext`, once from a fresh connection and
   once after a backup-and-reopen. The surrounding assertions, about the
   network-backed retained state itself, are unchanged.
4. `src/application/request_profiles.rs`'s own tests (ADR 0075 packet 017,
   ADR 0077 Decision 5): dropped the `INSPECTOR_TRACK_DETAIL_TRACK`/`FEED`
   rows and the `L3`/`L4` literals from
   `adr_0075_request_profile_include_strings_match_recorded_literals`,
   `adr_0075_request_profile_reports_collection_counts`, and
   `adr_0077_publisher_relationship_profiles_add_publisher_to_two_include_lists`.
   Renamed `adr_0075_request_profile_registry_names_ten_profiles` to
   `adr_0075_request_profile_registry_names_eight_profiles`. Changed its
   `ALL` array from ten entries to eight. The old name and size were false
   once the two profiles were gone.
5. `tests/architecture_tests.rs`'s `global_search_replaces_screen_local_search_chrome`
   (ADR 0043): dropped the two `for required in [...]` blocks that checked
   for `FetchDiscoverSearchResults`, `fetch_local_library_search_rows`, and
   the grouped search view-model contract, all now deleted.
6. `tests/architecture_tests.rs`'s `adr_0047_membership_buttons_use_download_remove_vocabulary`
   (ADR 0047): dropped its loop over `search_vm_sources()`.
7. `tests/architecture_tests.rs`'s `interactive_composites_carry_accessibility_labels`
   (ADR 0038 task 005): dropped the `RecentFeedTileDisplay` row, which named
   the deleted `src/view_models/search/recent.rs`.
8. `tests/architecture_tests.rs`'s
   `adr_0075_library_observation_callers_and_consumers_are_guarded` (ADR
   0075 packet 038): moved the `assembly` boundary's end marker from
   `fn fetch_local_track_context(` to `fn poisoned_lock(`, the function
   that now follows the checked code.
9. `tests/architecture_tests.rs`'s
   `adr_0075_snapshot_registry_transaction_and_local_read_boundaries` (ADR
   0075): dropped the `local` block, which read the deleted
   `fetch_local_track_context`.
10. `tests/architecture_tests.rs`'s `adr_0075_request_reuse_index_routes_ask_the_owner`
    (ADR 0075 packet 018 Part B): dropped the `feed_owner_track` block,
    which read the deleted `fetch_scoped_track`. Moved the
    `feed_owner_feed` boundary's end marker to
    `fn fetch_index_publisher_page_albums(`. Dropped the four-entry `for`
    loop that checked `artist_feed_for_guid`, `fetch_feed_detail`,
    `fetch_track_detail`, and `hydrate_feed_track_play_urls`, all now
    deleted.
11. `tests/architecture_tests.rs`'s `adr_0076_route_readiness_ignores_test_only_files`
    (ADR 0076 Decision 9): dropped its `src/view_models/search/tests.rs`
    entry.
12. `tests/architecture_tests.rs`'s
    `source_fact_placeholder_and_breadcrumb_regressions_are_guarded`: dropped
    its check of `src/application/queries/feed.rs` for
    `sanitize_feed_source_text`, `TrackContext::new(track, feed)`, and
    `sanitize_track_context_source_text`. Each line it looked for lived only
    in the deleted `fetch_track_detail`.
13. `tests/architecture_tests.rs`'s
    `view_models_own_display_fallbacks_for_library_and_search`: renamed to
    `view_models_own_display_fallbacks_for_library`. Changed its assertion
    message from "ADR 0038 Library/Search VM fallback ownership violations"
    to "ADR 0038 Library VM fallback ownership violations". Packet 005 had
    already dropped every `src/discover.rs` row from its table. Every
    remaining row names `src/library.rs` or a Library-adjacent shell file.
    So "and search" was already false.

No guard needed a new addition. `adr_0060_discover_surface_stays_deleted`
(packet 005) already fails a return of `src/ui/shells/discover/`. The
deleted `src/view_models/search/` directory needs no separate guard. The
`R60-6-06` repeat measurement is the mechanical proof that it, and the
rest of the query layer, stay gone.

### Deviations

- `ArtistCredit` is one of the six named types, and `Track.artist_credit`
  was its only remaining field reference once `Release` and `Recording`
  were gone. I deleted the field along with the type, since no live code
  read it (Recorded Facts' own Incident section already names this field
  as unread for five months). ADR 0075 packet 051 owns the field-by-field
  contract mapping. This deletion only removes a field that this packet's
  own type deletion made impossible to keep.
- I deleted the `EntityDetail::Track(Track)` variant, though `Track` the
  type is not one of the six named types and stays live elsewhere. Once
  `fetch_detail`, `fetch_contributors`, `fetch_value_routes`, and the
  parked query layer were gone, this variant had no constructor left
  anywhere in the crate. I judged this the same class of case as the
  named types: a `pub` item in `src/api.rs` with no crate caller. `Feed`
  is `EntityDetail`'s only remaining case.
- I deleted `fetch_detail` whole, not only its `release` and `recording`
  arms. Once those two arms and its sole caller (`fetch_scoped_detail`)
  were gone, the function itself had no remaining caller.
- `src/views.rs` is not one of the five files the task names for the
  cascade. I deleted `ArtistView::from_api` there because it took
  `api::Artist` by value and had no caller anywhere in the crate. Keeping
  it would not compile once `Artist` was gone. This is the smallest
  possible repair of a genuine, sanctioned deletion's direct compile
  break, not a wider change to `src/views.rs`.
- I renamed two tests, listed above, whose old names stated a fact
  ("ten profiles", "and search") that the deletion made false.

### Concerns

- `src/view_models/track.rs`'s `TrackVm::play_url` lost its only caller,
  `feed::hydrate_feed_track_play_urls`, in this deletion. The repeat
  measurement (R60-6-06) shows `play_url` and its three private helpers,
  `primary_source_enclosure_url`, `first_source_enclosure_url`, and
  `nonempty_url`, now uncalled outside their own file. `src/views.rs` sits
  outside the five files "Required Changes" names for the cascade, and
  `view_models/track.rs` sits outside it too, so I left this file alone.
  `cargo check --all-targets` stays silent about it, because `TrackVm` is
  `pub` in a `pub mod`. A later packet should measure and delete this
  dead code.
- `subscribe_service::lookup_musicbrainz_track` and
  `subscribe_service::download_and_compare_track` lost their only callers
  in this deletion, the two commands `LookupRemoteMusicBrainzTrack` and
  `DownloadAndCompareTrack`. `src/subscribe_service.rs` sits outside the
  five named cascade files, and both functions are `pub`, so no
  mechanical check flags them. A later packet should decide their fate,
  the same open question packet 005 recorded for `fetch_contributors` and
  `fetch_value_routes`, now resolved for those two.
- Four documents outside this packet still name the renamed guard
  `view_models_own_display_fallbacks_for_library_and_search`: ADR 0038,
  its phase plan, its review checklist, and task 003. "Do not touch" keeps
  this packet from editing them. A later documentation pass should
  reconcile the name.
- I traced each deleted item's callers by `grep` across the whole `src`
  tree, the same method packet 005 used. I did not read every one of the
  roughly 2,200 changed lines one by one.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/archive/adr-0060-task-006-delete-parked-discover-queries.md`
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

## Orchestrator Review - 2026-09-30

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,793 unit tests, 280 guards, and no warning. No marker that names this packet stays in `src/`.

- The diff deletes 6,406 lines and adds 47, in 20 files. `src/view_models/search/` is deleted.
- `src/api.rs` has no `Artist`, `Release`, `Recording`, `ArtistCredit`, `ReleaseReference` or `Source`, no `fetch_detail`, no `fetch_contributors`, no `fetch_value_routes`, no `Track::artist_credit` and no `fuzzy` parameter.
- The orchestrator corrected the renamed guard name in four ADR 0038 documents.

Two parked items stay outside the scope of this packet. A later measurement packet owns them:

- `TrackVm::play_url` and its three private helpers in `src/view_models/track.rs`.
- `lookup_musicbrainz_track` and `download_and_compare_track` in `src/subscribe_service.rs`.
