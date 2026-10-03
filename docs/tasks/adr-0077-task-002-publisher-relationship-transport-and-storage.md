# ADR 0077 Task 002: Publisher Relationship Transport And Storage

Status: Implemented - 2026-09-24. Mechanical checks are Green.
This packet changes no screen. It needs no visual acceptance.

## Goal

Decode each field of the live publisher relationship contract.
Request the `publisher` collection on the Library album hydration and the Index feed detail.
Store each relationship entry of a Library feed in a new table, with its observation time.

Packets 003 and 004 read this data. This packet adds no reader for a screen.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 2 and 5, and its accepted refinements.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I and section 2, collection state.
- [ADR 0016](../adr/archive/0016-schema-migration-discipline.md), the migration registry.
- [Packet 013](adr-0075-task-013-verified-snapshot-replacement.md): MusicIndex collection replacement stays disabled.
- [Packet 017](adr-0075-task-017-named-request-profiles.md): the named request profiles.
- [Packet 018](adr-0075-task-018-request-reuse-and-freshness.md): the shared request owner and its key.
- The live contract at `https://api.musicindex.org/openapi.json`, `PublisherResponse` and `FeedResponse`, read on 2026-09-24.

## The Live Contract - 2026-09-24

`PublisherResponse` has these fields. The first five are required.

| Field | Type | Decoded before this packet |
|---|---|---|
| `publisher_feed_guid` | string | Yes |
| `music_names_publisher` | boolean | No |
| `publisher_lists_music` | boolean | No |
| `publisher_link_resolution` | `guid`, `feed_url` or `unresolved` | No |
| `role_source` | `publisher_rel`, `music_rel`, `default` or `conflict` | No |
| `role` | string or null | No |
| `publisher_rel`, `music_rel` | string or null | No |
| `publisher_link_observed_at` | integer or null | No |
| `direction`, `remote_feed_guid`, `remote_feed_url`, `remote_feed_medium` | string or null | Yes |
| `publisher_feed_url`, `music_feed_guid`, `music_feed_url` | string or null | Yes |
| `reciprocal_declared`, `reciprocal_medium`, `two_way_validated` | legacy values | Yes |

`FeedResponse` adds `publisher_feed_title`, `release_artist_source`, `distinct_release_artist_count` and `distinct_release_artists`.
`api::Feed` decodes none of them before this packet.

On an album feed, each entry has `direction = "music_to_publisher"`. On a publisher feed, each entry has `direction = "publisher_to_music"`.

## Required Changes

### Decode

Add the missing fields to `api::PublisherRelationship` and `api::Feed` in `src/api.rs`.
Decode `publisher_link_resolution` and `role_source` into typed enums. Keep an unknown value as a typed unknown variant with its raw text. Do not fail the response.

Keep the legacy fields. The live contract still sends them.

### Request Profiles

Add `publisher` to two include lists in `src/application/request_profiles.rs`:

| Profile | Before | After |
|---|---|---|
| `LIBRARY_ALBUM_HYDRATION_FEED` | L5, four collections | L5 and `publisher`, five collections |
| `INDEX_FEED_DETAIL` | L2, seven collections | L2 and `publisher`, eight collections |

No request is added, and no request count changes. The response of each request becomes larger.
The packet 017 literal tests change on purpose. Each changed test names ADR 0077 Decision 5.

### Storage

Add schema version 14. It creates `feed_publisher_relationships`:

- `feed_id`, a reference to `feeds(id)`. A feed removal deletes its rows.
- `direction`, `publisher_feed_guid` and `remote_feed_guid`. These three values and `feed_id` are the key.
  The orchestrator added `remote_feed_guid` on 2026-09-24, because each entry of a publisher feed needs its own row.
- One column for each other `PublisherResponse` field.
- `publisher_feed_title`, from the album `FeedResponse`. It is the title of the publisher feed that the album names. Packet 003 uses it as the Library page title.
- `observed_at`, the time of the observation that supplied the row.

The Library album hydration writes the rows inside its existing local write.
A reused response writes nothing. Packet 018 already skips the write for a reused response.

### Replacement Rule

Packet 013 keeps MusicIndex collection replacement disabled. This packet follows that rule:

- An entry in the response inserts or updates its row.
- An entry that a response omits keeps its row.
- An empty `publisher` array deletes no row.
- An absent or null `publisher` value changes no row.

The [open Stophammer requests](../plans/v4vmm-open-requests.md) ask Stophammer for a completeness statement.
A subsequent packet enables replacement through the packet 013 registry after that statement arrives.

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_publisher_relationship_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R2-01 | A recorded album response from 2026-09-24 decodes each `PublisherResponse` field. The test holds the recorded JSON |
| R2-02 | A recorded publisher feed response decodes `distinct_release_artist_count`, `distinct_release_artists` and each `publisher_to_music` entry |
| R2-03 | An unknown `role_source` or `publisher_link_resolution` value decodes to the unknown variant with its raw text |
| R2-04 | `LIBRARY_ALBUM_HYDRATION_FEED` sends L5 and `publisher`. `INDEX_FEED_DETAIL` sends L2 and `publisher`. Each other profile is unchanged |
| R2-05 | The Library album hydration and the Index feed detail send the same number of requests as before this packet |
| R2-06 | A version 13 database migrates to version 14 and has an empty `feed_publisher_relationships` table |
| R2-07 | Hydration stores each entry with each field, `publisher_feed_title` and `observed_at`. `role_source` keeps its value, and a null `role` stays null |
| R2-08 | A second response with a changed entry updates that row. A second response that omits an entry keeps its row |
| R2-09 | An empty array, an absent value and a null value each change no row |
| R2-10 | A reused response writes no row |
| R2-11 | A feed removal deletes the rows of that feed |
| R2-12 | No stored row is written for a track. A guard fails when a track table gains a publisher column, and names ADR 0077 Decision 2 |

## Exclusions

- No page, no view model and no navigation. Packets 003 and 004 own them.
- No MusicIndex collection replacement. Packet 013 keeps it disabled.
- No change to the track routes. The track `include=publisher` view gave an empty array on 2026-09-24.
- No new request.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/api.rs`: `Feed`, `PublisherRelationship`, and the decode tests near line 2340.
- `src/application/request_profiles.rs` and its tests.
- `src/application/request_reuse.rs`: the request key holds the include list.
- `src/application/queries/library.rs`: `hydrate_album_identity_facts`.
- `src/application/queries/search.rs` and `src/application/queries/feed.rs`: the Index feed detail sites.
- `src/provider_observation/contracts.rs` and `src/provider_observation/musicindex.rs`.
- `src/db.rs`: `MIGRATIONS` and the schema contracts.

## Checks

```bash
cargo test --lib adr_0077_publisher_relationship
cargo test --lib adr_0075_request_profile
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree before the first run on a real database.
After a real database migrates to version 14, the new table stays. It is empty or holds relationship rows, and no older code reads it.

## Implementation Result - 2026-09-24

### Decode

`api::PublisherRelationship` in `src/api.rs` decodes each field of the live `PublisherResponse` contract.
The eight new fields are `music_names_publisher`, `publisher_lists_music`, `publisher_link_resolution`, `publisher_link_observed_at`, `publisher_rel`, `music_rel`, `role` and `role_source`.
The legacy fields stay.

`PublisherLinkResolution` and `RoleSource` are typed enums. Each enum has an `Unknown` variant that keeps the raw text.
Serialization writes the raw text again, so an unknown value is not lost. A value that is not a string fails the decode, as the other fields do.

`api::Feed` decodes `publisher_feed_title`, `release_artist_source`, `distinct_release_artist_count` and `distinct_release_artists`.
`release_artist_source` stays a string. The packet asks for typed enums for two fields only.
Each new field is omitted from serialization when it is absent, so the ADR 0075 round-trip tests stay unchanged.

The recorded JSON of R2-01 and R2-02 came from two read-only requests to `https://api.musicindex.org` on 2026-09-24.
The tests keep the album entry and the nine publisher feed entries unchanged.

### Profiles

`src/application/request_profiles.rs` replaces the constants `L2` and `L5` with `L2_PUBLISHER` and `L5_PUBLISHER`.
Each new list adds `publisher` at the end. `LIBRARY_ALBUM_HYDRATION_FEED` requests five collections, and `INDEX_FEED_DETAIL` requests eight collections.
The other eight profiles are unchanged.

These tests changed on purpose. Each changed test names ADR 0077 Decision 5:

- `adr_0075_request_profile_include_strings_match_recorded_literals` (R17-02),
- `adr_0075_request_profile_reports_collection_counts` (R17-12),
- the messages of the R17-05 and R17-07 call-site tests,
- `adr_0075_library_observation_hydration_repetition_counts_and_reopened_evidence`, which holds the L5 request path as a literal.

The guard `adr_0075_request_profile_registry_owns_literals_and_has_no_provider_field` is unchanged.
Each new list starts with its old list, so the guard still finds a copy of a new list in the five request files.

### Migration 14

`MIGRATIONS` in `src/db.rs` has version 14, `feed_publisher_relationships`. `CURRENT_VERSION` is 14.
The new module `src/db/publisher_relationships.rs` holds the DDL, the column contract and the writer.

The table key is `feed_id`, `direction`, `publisher_feed_guid` and `remote_feed_guid`. `feed_id` refers to `feeds(id)` with `ON DELETE CASCADE`.
On a `music_to_publisher` entry, `remote_feed_guid` is the publisher feed GUID. On a `publisher_to_music` entry, it is the listed feed GUID.
The table has one column for each other `PublisherResponse` field, `publisher_feed_title` and `observed_at`.
The two enum columns keep the wire text, so an unknown value stays in the row.

Migration 14 uses the transaction of migrations 12 and 13. The ledger record is in the same transaction.
After migration 14, the registry verifies the exact version-14 schema. It also compares `retained_digest` before and after, and it makes sure that the new table is empty.

`schema_contract(14)` adds the new table. `inspect_schema` now checks each applied version from 12 to 13 against its own contract.
A version-13 database that matches the version-13 contract is thus "upgrade required".
`upgrades::create_fixture` accepts target 14.

### Storage

`hydrate_album_identity_facts` writes the rows after `persist_musicindex_feed`, in the same branch.
Packet 018 skips that branch for a reused response, so a reused response writes no row.

The writer `upsert_feed_publisher_relationships` follows the replacement rule of this packet:

- An entry inserts or updates its row.
- The writer deletes no row, so an entry that a response omits keeps its row.
- An empty array, an absent value and a null value change no row.
- An entry without `direction`, `publisher_feed_guid` or `remote_feed_guid` has no key, so it writes no row.

`observed_at` is the Unix time in seconds of the fetch that supplied the response.
The value comes from the `fetched_at_us` field of the hydration receipt. The other `observed_at` columns of the local tables also use seconds.
If no receipt gives a fetch time, the hydration reports an error. It does not make a time.

### Guard

The situational guard `adr_0077_publisher_relationship_no_track_table_stores_a_publisher` enforces R2-12. It reads the production part of each Rust file under `src/`.
It reports a publisher column in a `CREATE TABLE` of a track table, an `add_column_if_missing` call or an `ALTER TABLE` statement.
It also reports a track column in `feed_publisher_relationships`. Its report names ADR 0077 Decision 2 and gives the fix.

Four probe lines caused the four expected failures. The session then removed the probes.

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R2-01 | `adr_0077_publisher_relationship_recorded_album_decodes_each_field` | `src/api.rs` |
| R2-02 | `adr_0077_publisher_relationship_recorded_publisher_feed_decodes_entries` | `src/api.rs` |
| R2-03 | `adr_0077_publisher_relationship_unknown_enum_values_keep_raw_text` | `src/api.rs` |
| R2-04 | `adr_0077_publisher_relationship_profiles_add_publisher_to_two_include_lists` | `src/application/request_profiles.rs` |
| R2-05 | `adr_0077_publisher_relationship_album_hydration_sends_one_request` | `src/application/queries/library.rs` |
| R2-05 | `adr_0077_publisher_relationship_index_feed_detail_request_count_is_unchanged` | `src/application/queries/feed.rs` |
| R2-05 | `adr_0077_publisher_relationship_index_feed_rows_request_count_is_unchanged` | `src/application/queries/search.rs` |
| R2-06 | `adr_0077_publisher_relationship_version_13_migrates_to_14_with_empty_table` | `src/db/publisher_relationships.rs` |
| R2-06 | `adr_0077_publisher_relationship_failed_migration_14_leaves_version_13` | `src/db/publisher_relationships.rs` |
| R2-07 | `adr_0077_publisher_relationship_hydration_stores_each_field` | `src/application/queries/library.rs` |
| R2-08 | `adr_0077_publisher_relationship_changed_entry_updates_and_omitted_entry_stays` | `src/application/queries/library.rs` |
| R2-09 | `adr_0077_publisher_relationship_empty_absent_and_null_change_no_row` | `src/application/queries/library.rs` |
| R2-10 | `adr_0077_publisher_relationship_reused_response_writes_no_row` | `src/application/queries/library.rs` |
| R2-11 | `adr_0077_publisher_relationship_feed_removal_deletes_its_rows` | `src/db/publisher_relationships.rs` |
| R2-12 | `adr_0077_publisher_relationship_hydration_writes_no_track_row` | `src/application/queries/library.rs` |
| R2-12 | `adr_0077_publisher_relationship_no_track_table_stores_a_publisher` | `tests/architecture_tests.rs` |

The test `adr_0077_publisher_relationship_unkeyed_entries_write_no_row` also proves that an entry without a key writes no row.
It also proves that the foreign key refuses a row for a missing feed.

The test `adr_0077_publisher_relationship_recorded_publisher_feed_stores_one_row_per_entry` stores the recorded R2-02 response through the writer.
It gets one row for each of the nine entries.

### Findings And Deviations

1. The first key of this packet could not hold each entry of a publisher feed.
   On 2026-09-24, the nine `publisher_to_music` entries of feed `bcbe7207-9338-474e-ba18-09e6b1b69979` had the same `direction` and `publisher_feed_guid`.
   With that key, the last entry replaced the others in one row.
   On 2026-09-24, the orchestrator added `remote_feed_guid` to the key. An entry without it writes no row.
   No real database has migration 14, so the migration 14 DDL changed in place.
2. The `publisher` presence rule in `src/provider_observation/musicindex.rs` came from commit `82c3c06` without a documented rule.
   It gave each live array the presence `invalid`. The live contract declares `FeedResponse.publisher` and `TrackResponse.publisher` as an array of `PublisherResponse` or null.
   On the orchestrator decision of 2026-09-24, `publisher` now uses the collection rule of `PropertyPresence::from_json`, and the special branch is deleted.
   The existing test proves that an array with entries is `populated`, an empty array is `empty`, and an object or `false` is `invalid`.
   A search found no other test or guard that holds the object shape.
3. The Library album hydration has no transaction around its local writes. The relationship write uses its own savepoint after `persist_musicindex_feed`.
   A failure in the relationship write rolls back all relationship rows of that response. It does not roll back the earlier writes of the hydration.
4. The test fixture of `src/application/queries/library.rs` now creates a database at `CURRENT_VERSION`, not at version 12.
   Before this change, the fixture database had no relationship table.
5. These tests expected version 13. They now expect version 14 or `MIGRATIONS.len()`:
   - in `src/db.rs`, `test_migrations_record_versions_on_fresh_schema`, `migration_cleanup_placeholder_source_text_nulls_only_placeholder_payloads`, and the packet 001 tests R1-01 and R1-03,
   - in `src/db/upgrades.rs`, the two tests that name `current: 13`,
   - in `src/db/maintenance.rs`, the inspection test,
   - in `src/db/maintenance/upgrade.rs`, three tests. The boundary test also stops migration 14 at each boundary. Each stop gives a verified rollback to version 11,
   - in `src/view_models/startup/database.rs`, the repair report test.
6. The fallback preparation failure text now reads "Apply migrations 12 to 14 and verify retained records".
7. This packet does not change these documents. The orchestrator owns them:
   - the status of ADR 0077, the ADR index, and `AGENTS.md`,
   - the ADR 0077 phase plan and `docs/plans/broadcast-chain-delivery-order.md`.

### Checks - 2026-09-24

| Check | Result |
|---|---|
| `cargo test --lib adr_0077_publisher_relationship` | Green, 17 tests |
| `cargo test --lib adr_0075_request_profile` | Green, 17 tests |
| `cargo test` | Green, 1696 unit tests and 273 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 273 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.
`cargo clippy --all-targets -- -D warnings` still reports the earlier errors. No error is in a file that this packet adds.

## Operator Visual Check

None. This packet changes no screen.
