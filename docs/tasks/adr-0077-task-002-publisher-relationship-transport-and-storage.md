# ADR 0077 Task 002: Publisher Relationship Transport And Storage

Status: Ready - 2026-09-24. Implementation has not started. Start after packet 001.
This packet changes no screen. It needs no visual acceptance.

## Goal

Decode each field of the live publisher relationship contract.
Request the `publisher` collection on the Library album hydration and the Index feed detail.
Store each relationship entry of a Library feed in a new table, with its observation time.

Packets 003 and 004 read this data. This packet adds no reader for a screen.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 2 and 5, and its accepted refinements.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I and section 2, collection state.
- [ADR 0016](../adr/0016-schema-migration-discipline.md), the migration registry.
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
- `direction` and `publisher_feed_guid`. These two values and `feed_id` are the key.
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

The [publisher album summary request](../plans/stophammer-publisher-album-summary-request.md) asks Stophammer for a completeness statement.
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

## Operator Visual Check

None. This packet changes no screen.
