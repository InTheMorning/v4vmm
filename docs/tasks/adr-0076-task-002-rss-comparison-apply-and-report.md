# ADR 0076 Task 002: RSS Comparison, Apply And Report

Status: Open - implementation and mechanical checks are complete on 2026-09-24. On 2026-10-07 the operator moved its visual check to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07). The packet that rebuilds the surface carries it.
This packet changes stored values and adds a difference report. Its visual gate is open and paused.

## Goal

Compare each fetched RSS document with the stored values of its feed and tracks.
Write each RSS value to its stored slot at once. Keep the old value as evidence.
Hold each written field against a later MusicIndex overwrite, until MusicIndex agrees or supplies a newer record.

Store a new track. Mark a removed track. Show one report for each check.

## Authority

- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decisions 3, 4, 5, 6 and 7.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I (ownership), section 7 (a repair report names the old value, the new value, the owner and the source).
- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) packet 002: the stored publisher relationship.
- The [comparison contract](../schema/adr-0075-comparison-and-discrepancy-contract.md#description-comparison-version-1): readable-text comparison of descriptions. It stays in force.
- The [link rules](../schema/adr-0075-field-rules-links-and-media.md): URL normalization of scheme, host and default ports.
- [ADR 0075 Decision E](../adr/0075-metadata-ownership-and-completeness.md): supported enclosure selection.
- [ADR 0016](../adr/archive/0016-schema-migration-discipline.md): the new tables and columns go through the migration registry.

## Release Condition - Decision 5

Decision 5 ends a hold when MusicIndex supplies a record that it updated after the last RSS check.
The live contract at `/openapi.json` gives `FeedResponse.updated_at` as an integer with no description.

Before the first edit, the implementer confirms that `updated_at` records the ingest of the RSS document, from the Stophammer source or from a Stophammer answer.
When it cannot be confirmed, the packet stops and reports. The rest of the packet does not start.

## Recorded Facts - 2026-09-24

- `src/rss/subscribe.rs::subscribe_feed` fetches, parses with the `rss` crate, and upserts `feeds` and `tracks` columns in one function.
  It does not delete a track that RSS no longer lists.
- ADR 0054 facts live in `entity_metadata_facts`, keyed by owner and `source`. Feed keys: `publisher_text`, `musicindex_release_kind`, `release_date`, `language`, `explicit`, `description`. Track keys: `publisher_text`, `description`, `pub_date`, `explicit`.
- `FeedView::from_local_with_facts` and `TrackView::from_local_with_facts` in `src/views.rs` read the MusicIndex fact before the column. Packet 020 changes that read.
- `identity_ingest::persist_musicindex_feed` and `persist_musicindex_track` write MusicIndex facts, links, ids and contributors. `feed_service::apply_feed_updates` also calls `db::set_feed_description`.
- `feeds.musicindex_updated_at` stores the `updated_at` of the last MusicIndex feed response.
- `metadata_discrepancies` and `metadata_discrepancy_transitions` have no production writer. Their `CHECK` limits them to two fields. `metadata_field_selections` has no production writer.
- Schema version 15 is current after packet 001.

## Required Changes

### Parse And Map

Split `subscribe_feed` into a parse step and a persist step. The parse step returns a typed document: the channel values and one item for each RSS item.
The check and the subscribe command use the same parse step and the same column mapping.

### The Compared Elements

The check compares each element below. The stored slot is the column or fact that display and tag frames read.

| Element | Owner | RSS source | Stored slot | Comparison |
|---|---|---|---|---|
| Title | feed, track | `<title>` | `feeds.title`, `tracks.track_title` | Trimmed text |
| Description | feed, track | `<description>`, item description | `feeds.description` and the fact. Track fact `description` | Readable text, per the comparison contract |
| Artwork | feed, track | `podcast:image`, `itunes:image`, `<image><url>` | `feeds.album_image_href`, `tracks.track_image_href` | Normalized URL |
| Link | feed, track | `<link>` | `feeds.link`, `tracks.link` | Normalized URL |
| Audio URL and type | track | the selected enclosure, per Decision E | `tracks.enclosure_url`, `enclosure_type` | Normalized URL, exact type |
| Duration | track | `itunes:duration` | `tracks.duration_seconds`, `itunes_duration_raw` | Seconds |
| Date | track | `pubDate` | `tracks.pub_date` and the fact `pub_date` | Instant |
| Explicit | feed, track | `itunes:explicit` | Feed fact `explicit`. `tracks.itunes_explicit` and the fact | Boolean |
| Language | feed | `<language>` | `feeds.language` and the fact | Lowercase tag |
| Album artist text | feed | channel `itunes:author`, then the accepted chain | `feeds.album_artist` | Trimmed text |
| Artist text | track | item `itunes:author`, then the accepted chain | `tracks.artist_name` | Trimmed text |
| Feed owner | feed | `itunes:owner` name | feed fact `publisher_text` | Trimmed text |
| Persons | feed, track | `podcast:person` | `people_json` and `entity_contributors` with source `rss` | Name, role, group, href, image, in order |
| Nostr | feed, track | `podcast:txt purpose="npub"` | `entity_identity_ids` with scheme `npub` and source `rss` | Exact text |
| Payment routes | feed, track | `podcast:value` | `feeds.podcast_value_json`, `tracks.item_value_json` | Recipient set: type, address, split, fee, custom key and value, name |
| Publisher | feed | `podcast:publisher` remote item | `feed_publisher_relationships`: `remote_feed_guid`, `remote_feed_url` of the `music_to_publisher` row | Exact GUID, normalized URL |

Formatting alone is not a difference. A whitespace change, an HTML change with the same readable text, or a URL with a default port is equal.

The check does not compare a value that MusicIndex derives: the publisher link state and role, the release date from the oldest item, and counts.

### Apply

For each difference, the check writes the RSS value to the stored slot in one transaction for the feed.
A field that the parsed document does not state is cleared. A parse failure or a failed request writes nothing.

For a fact-backed field, the check also writes the fact row with source `rss`. The MusicIndex fact row stays as evidence.

For the publisher element, the check updates `remote_feed_guid` and `remote_feed_url`. The derived columns stay until MusicIndex ingests the feed again.

### Hold

Add `rss_field_holds`: `owner_kind`, `feed_id`, `track_id`, `field`, `rss_value_json` (null for a cleared field), `run_id`, `checked_at_us`.
The key is the owner and the field. Each written difference inserts or updates its hold.

Add one gate function that each MusicIndex writer of a compared field calls before it writes the slot:

- no hold: write.
- hold, and the MusicIndex value equals the held value: delete the hold, write.
- hold, and the response `updated_at` is after `checked_at_us`: delete the hold, write.
- otherwise: skip the slot. The MusicIndex fact row is still written as evidence.

The implementer lists each MusicIndex write site of a compared slot and applies the gate. A guard proves that each listed site calls it.

### New And Removed Tracks

An item whose GUID has no `tracks` row is inserted with the mapping of the subscribe persist step. It gets no file, and `is_in_library` stays 0.
The report lists it with the existing download action.

Add `tracks.removed_from_feed_at` and `tracks.removed_from_feed_confirmed_at` (integer, null).
A stored track whose GUID is absent from a parsed document gets `removed_from_feed_at` set to the check time.
The track, its file, its playlist entries and its session entries stay. A track that returns to RSS gets both columns cleared, and the report lists the return.
Packet 003 uses `removed_from_feed_confirmed_at`.

### Report

Add `rss_check_differences`: `id`, `run_id`, `feed_id`, `track_id` (null), `field`, `kind` (`changed`, `cleared`, `track_added`, `track_removed`, `track_returned`), `old_value_json`, `new_value_json`, `recorded_at_us`.

The playlist page view model exposes the report of the latest run. Each difference names the feed, the track, the field, the old value, the new value and the check time. Each stale feed has a podping.me link.
It exposes the fetch outcomes of packet 001 in the same report. The mounted view updates in place after the check.

### Dead Tables

`metadata_discrepancies`, `metadata_discrepancy_transitions` and `metadata_field_selections` have no production writer. ADR 0076 superseded the decisions that created them.
Migration 16 drops the three tables. The implementer confirms with a search that no production code reads them before the drop, and reports each reader that it finds.

### Album Artist Column

The operator decided on 2026-09-24 that the channel artist text is a feed value in its own column.
Add `feeds.album_artist`, text, null. The subscribe persist step and the check write it.

The migration fills it for each feed whose tracks all hold one equal `album_artist_name`. Other feeds stay null, and the migration result counts them.
`tracks.album_artist_name` stays unchanged. Packet 020 projects the feed column onto a track view with the channel as its owner.

### Migration

Add schema version 16: `rss_field_holds`, `rss_check_differences`, the two `tracks` columns, `feeds.album_artist`, and the three drops.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_rss_comparison_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R2-01 | `subscribe_feed` and the check produce the same column values from one recorded document |
| R2-02 | Two descriptions with equal readable text and different HTML give no difference |
| R2-03 | A URL that differs only by a default port gives no difference |
| R2-04 | Each compared element gives a difference when its value changes, on a recorded document pair. The test covers each row of the element table |
| R2-05 | A changed derived value (publisher role, link state) gives no difference |
| R2-06 | A difference writes the slot, the `rss` fact where one exists, and one hold. The old value is in the difference row |
| R2-07 | A field absent from the document is cleared, with a hold whose value is null |
| R2-08 | A failed request or a parse failure writes no slot, no hold and no difference |
| R2-09 | With a hold, a MusicIndex value that differs and an older `updated_at` does not write the slot. The MusicIndex fact row is written |
| R2-10 | With a hold, an equal MusicIndex value writes the slot and deletes the hold |
| R2-11 | With a hold, a different MusicIndex value with a newer `updated_at` writes the slot and deletes the hold |
| R2-12 | A new item inserts a track with no file and `is_in_library` 0, and a `track_added` difference |
| R2-13 | A missing item sets `removed_from_feed_at`, deletes nothing, and records `track_removed`. Its playlist entries stay |
| R2-14 | A returned item clears both columns and records `track_returned` |
| R2-15 | A changed `podcast:publisher` remote item updates the two remote columns of the relationship row and leaves the derived columns |
| R2-16 | The report view model exposes each difference with the feed, field, old value, new value, time and podping.me link |
| R2-17 | A version 15 database migrates to version 16. The three dead tables are absent, the new tables exist, and each retained row of `tracks` keeps its values |
| R2-18 | A guard proves that each listed MusicIndex write site of a compared slot calls the gate. Its message names ADR 0076 Decision 5 |
| R2-20 | A changed channel artist text writes `feeds.album_artist` and one difference for the feed. No `tracks` column changes |
| R2-21 | The migration fills `feeds.album_artist` for a feed whose tracks share one value, and leaves it null for a feed with two values |
| R2-19 | The check writes no audio tag. A guard proves that the check module does not call `write_id3v24_edits`. Its message names ADR 0076 Decision 8 |

## Visual Acceptance Criteria

For the operator. No test proves them.

- V1: after a check with differences, the playlist page shows the report in place: feed, field, old value, new value, time and the podping.me link, in Light and Dark themes.
- V2: a new track shows with its download action. A removed track shows its mark on the playlist row.
- V3: the report is readable at normal and narrow widths. Stacked text follows the column text rule.

## Exclusions

- No audio tag write. Packet 004 owns the confirmation.
- No readiness change. Packet 003 owns it.
- No projection change in `src/views.rs`. Packet 020 owns it.
- No change to the request pacing of packet 001.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/rss/subscribe.rs`, `src/rss/helpers.rs`, `src/rss/identity.rs`.
- `src/identity_ingest.rs`: the MusicIndex writers. `src/feed_service.rs`: `apply_feed_updates`.
- `src/db.rs`: the feed and track setters, `replace_local_metadata_facts`, `MIGRATIONS`.
- `src/db/publisher_relationships.rs`.
- `src/db/provider_snapshot_schema.rs`: the dead tables.
- `src/runtime/playlist_rss_check.rs` from packet 001.
- `src/view_models/library.rs`: the playlist page view model.
- [Column text truncation](../troubleshooting/column-text-truncation.md).

## Checks

```bash
cargo test --lib adr_0076_rss_comparison
cargo test --lib adr_0076_playlist_check
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree before the first run on a real database.
After migration 16, the three dead tables are gone. A database backup taken before the run restores them.
An applied RSS value stays in its slot. The observation store keeps the earlier MusicIndex response.

## Implementation Result - 2026-09-24

### Release Condition Evidence

The implementer read the local Stophammer checkout `/home/citizen/build/stophammer` at commit `a03c9ea57cf29562c9183abdd19ff2b0e188c3de` (2026-09-24 16:38:19 -0400).
The deployed revision is not confirmed. The implementer changed nothing in that checkout.

`FeedResponse.updated_at` is the `feeds.updated_at` column. `src/query.rs:750-753` reads it, and `src/query.rs:168-215` serves it.

- `src/api.rs:1613` `handle_ingest_feed` is the ingest of one RSS document.
- `src/api.rs:1759` sets `now = db::unix_now()`. `src/db.rs:543-547` gives Unix seconds.
- `src/api.rs:1842-1867` builds the feed row with `updated_at: now`.
- `src/db.rs:5292-5314` (`ingest_transaction`) and `src/db.rs:1320-1346` (`upsert_feed`) write `updated_at = excluded.updated_at`.
- `src/api.rs:1676` returns early when the content hash of the document did not change (`src/verifiers/content_hash.rs:52-54`). That ingest does not change `updated_at`.
- `src/apply.rs:100` applies a `FeedUpserted` event on a replica. The replica copies the `updated_at` value of the primary node.
- No other statement writes `feeds.updated_at`. `src/api.rs:4041` changes `feed_url` only. No trigger writes the column.

Result: `updated_at` changes on each ingest of a changed RSS document of the feed. An unchanged document changes nothing. Thus the release condition holds, and the packet continued.

Two details affect the gate:

- The value is in seconds. The gate multiplies it by 1,000,000 before it compares it with `checked_at_us`.
- A body change that changes no compared value also sets a new `updated_at`. The gate then releases the hold. MusicIndex read the same RSS document, so its value is then current.

### Parse And Persist Split

`src/rss/subscribe.rs` has the parse step `parse_feed_document` and the typed `ParsedFeedDocument`, `ParsedChannel` and `ParsedItem`.
The persist step is `persist_channel`, `persist_items` and `upsert_item_columns`. `subscribe_feed` fetches, parses and persists. The check calls the same parse step and the same `upsert_item_columns`.

The parse step now selects the enclosure by ADR 0075 Decision E. It uses `track_compare::enclosure_supported`. Finding 3 gives the fallback.

### Element Mapping

`src/rss/check_apply.rs::apply_checked_document` compares and applies one document in one transaction for the feed. `src/rss/compare.rs` holds the comparison rules.

| Element | Stored value that the check compares | Write on a difference |
|---|---|---|
| Title | `feeds.title`, `tracks.track_title` | The column |
| Description, feed | The hold, then the `musicindex` fact, then `feeds.description` | `feeds.description` and the `rss` fact |
| Description, track | The hold, then the `musicindex` fact, then the `rss` fact | The `rss` fact |
| Artwork | `feeds.album_image_href`, `tracks.track_image_href` | The column |
| Link | `feeds.link`, `tracks.link` | The column |
| Audio URL and type | `tracks.enclosure_url`, `enclosure_type` | Both columns |
| Duration | `tracks.duration_seconds`, compared in seconds | `duration_seconds` and `itunes_duration_raw` |
| Date | The hold, then the `musicindex` fact `pub_date`, then `tracks.pub_date` | `tracks.pub_date` and the `rss` fact `pub_date` |
| Explicit, feed | The hold, then the `musicindex` fact, then the `rss` fact | The `rss` fact |
| Explicit, track | The hold, then the `musicindex` fact, then `tracks.itunes_explicit` | `tracks.itunes_explicit` and the `rss` fact |
| Language | The hold, then the `musicindex` fact, then `feeds.language` | `feeds.language` and the `rss` fact |
| Album artist text | `feeds.album_artist` | `feeds.album_artist` |
| Artist text | `tracks.artist_name` | The column |
| Feed owner | The hold, then the `musicindex` fact, then the `rss` fact `publisher_text` | The `rss` fact |
| Persons | `entity_contributors` with source `rss` | `people_json` and the `rss` contributor rows |
| Nostr | `entity_identity_ids` with source `rss` and scheme `nostr_npub` or `nostr_nprofile` | Those rows |
| Payment routes | `feeds.podcast_value_json`, `tracks.item_value_json` | The column |
| Publisher | The first `music_to_publisher` row: `remote_feed_guid`, `remote_feed_url` | That row |

A `musicindex` description is plain text. Each other description value is HTML. The readable-text rule of the comparison contract uses the `html5ever` parser for HTML.

Each difference writes its hold and one `rss_check_differences` row with the old value and the new value. A parse failure returns an error before the transaction. A failed request never reaches the apply.

### MusicIndex Gate Sites

`db::rss_field_holds::musicindex_gate` is the one gate. The guard `adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate` proves that each site below calls it before its write.

1. Removed on 2026-09-24 by ADR 0075 packet 020. `subscribe_feed` is not a MusicIndex writer. It records RSS values and holds like a check.
2. `src/feed_service.rs::apply_feed_updates`: `db::set_feed_description`.
3. `src/application/queries/library.rs::hydrate_album_identity_facts`: `db::set_feed_description`.
4. The same function: `upsert_feed_publisher_relationships`. A held publisher keeps its row, and the gate removes the `music_to_publisher` entries from the write.
5. `src/identity_ingest.rs::persist_source_ids`: the `rss` Nostr rows that a MusicIndex response carries. A held value keeps its rows.
6. `src/identity_ingest.rs::persist_feed_metadata_facts`: description, language, explicit and feed owner. A held description keeps its `rss` fact row.
7. `src/identity_ingest.rs::persist_track_metadata_facts`: description, date and explicit.

At sites 6 and 7 the `musicindex` fact rows are evidence, so the app always writes them. The gate call releases a hold when MusicIndex agrees or has a newer record.

No MusicIndex writer changes the title, artwork, link, audio, duration, artist, album artist, persons or payment route slots.

### Migration 16

`MIGRATIONS` in `src/db.rs` has version 16, `rss_field_holds_and_differences`. `CURRENT_VERSION` is 16. `src/db/rss_field_holds.rs` holds the DDL.

- It creates `rss_field_holds` and `rss_check_differences`.
- It adds `tracks.removed_from_feed_at`, `tracks.removed_from_feed_confirmed_at` and `feeds.album_artist`.
- It fills `feeds.album_artist` for each feed whose tracks all hold one equal, non-empty `album_artist_name`. It writes the two counts to the error stream.
- It drops `metadata_discrepancy_transitions`, `metadata_discrepancies` and `metadata_field_selections`.

Migration 16 uses the shared transaction of migrations 12 to 15. After it, the registry verifies the version-16 schema and the retained digest, and it makes sure that the new tables and marks are empty.
`schema_contract(16)` gives the extended `feeds` and `tracks` columns. `upgrades::create_fixture` accepts target 16.

### Dead Tables

A search of `src/` and `tests/` found no production reader or writer of the three tables. It found these test readers:

- `src/db/maintenance/restore.rs`: the current restore test read the two tables. It now proves that a current backup has none of the three tables. It has the name `adr_0075_migration_current_restore_preserves_bodies_and_absence`.
- `src/db/provider_observations.rs`: the evidence-table test listed the three tables. The list no longer names them.
- `src/db.rs`: the migration 13 test reads `metadata_discrepancies` at version 13. It stays, with the fixture rows of `SUPERSEDED_ROWS`.
- `src/db/provider_snapshot_schema.rs`: the migration 12 constraint tests. They stay at version 12.
- `tests/architecture_tests.rs`: `adr_0075_observation_writer_and_library_retention_have_one_owner` did not permit an insert into two of the tables. The tables no longer exist, so the guard no longer names them.

### Report And Removed Marks

The actor snapshot keeps the differences of each run, the removed marks of each loaded playlist, and an applied revision.
`view_models::playlist_rss_check::report` gives one row for each difference. The row gives the feed, the track, the field, the old value, the new value and the check time.
Each stale feed gets a podping.me link. A `track_added` row gets the existing Library track download.

A playlist row with the mark shows "Removed from feed" and the date.
The Library view reloads its stored values when the applied revision changes.

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R2-01 | `adr_0076_rss_comparison_subscribe_and_check_write_equal_columns` | `src/rss/check_apply.rs` |
| R2-02 | `adr_0076_rss_comparison_equal_readable_text_is_equal` | `src/rss/compare.rs` |
| R2-02 | `adr_0076_rss_comparison_description_markup_change_is_equal` | `src/rss/check_apply.rs` |
| R2-03 | `adr_0076_rss_comparison_default_port_url_is_equal` | `src/rss/compare.rs` |
| R2-04 | `adr_0076_rss_comparison_each_element_change_is_a_difference` | `src/rss/check_apply.rs` |
| R2-05 | `adr_0076_rss_comparison_derived_values_are_not_compared` | `src/rss/check_apply.rs` |
| R2-06 | `adr_0076_rss_comparison_difference_writes_slot_fact_and_hold` | `src/rss/check_apply.rs` |
| R2-07 | `adr_0076_rss_comparison_absent_field_is_cleared_with_null_hold` | `src/rss/check_apply.rs` |
| R2-08 | `adr_0076_rss_comparison_parse_failure_writes_nothing` | `src/rss/check_apply.rs` |
| R2-08 | `adr_0076_rss_comparison_failed_request_writes_nothing` | `src/runtime/playlist_rss_check.rs` |
| R2-09 | `adr_0076_rss_comparison_hold_keeps_slot_against_older_musicindex` | `src/db/rss_field_holds.rs` |
| R2-10 | `adr_0076_rss_comparison_equal_musicindex_value_releases_hold` | `src/db/rss_field_holds.rs` |
| R2-11 | `adr_0076_rss_comparison_newer_musicindex_record_releases_hold` | `src/db/rss_field_holds.rs` |
| R2-12 | `adr_0076_rss_comparison_new_item_is_stored_without_file` | `src/rss/check_apply.rs` |
| R2-13, R2-14 | `adr_0076_rss_comparison_removed_item_is_marked_and_returns` | `src/rss/check_apply.rs` |
| R2-15 | `adr_0076_rss_comparison_publisher_remote_updates_relationship` | `src/rss/check_apply.rs` |
| R2-16 | `adr_0076_rss_comparison_report_exposes_differences_and_podping_link` | `src/view_models/playlist_rss_check.rs` |
| R2-17 | `adr_0076_rss_comparison_version_15_migrates_to_16` | `src/db/rss_field_holds.rs` |
| R2-18 | `adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate` | `tests/architecture_tests.rs` |
| R2-19 | `adr_0076_rss_comparison_check_writes_no_audio_tag` | `tests/architecture_tests.rs` |
| R2-20 | `adr_0076_rss_comparison_channel_artist_changes_only_the_feed` | `src/rss/check_apply.rs` |
| R2-21 | `adr_0076_rss_comparison_migration_fills_album_artist` | `src/db/rss_field_holds.rs` |

These tests also pass: `adr_0076_rss_comparison_failed_migration_16_leaves_version_15`, `adr_0076_rss_comparison_hold_and_difference_constraints_hold`, `adr_0076_rss_comparison_gate_skips_publisher_and_nostr_rows` and `adr_0076_rss_comparison_recipient_set_ignores_order_and_formatting`.
The guard messages name ADR 0076 Decisions 5 and 8 and give the fix. No test sends a network request.

### Findings And Deviations

1. `Cargo.toml` has two new direct dependencies: `html5ever = "=0.27.0"` and `markup5ever_rcdom = "=0.3.0"`. The comparison contract requires a standards-based HTML parser. Both crates were already in `Cargo.lock` through `gpui-component`.
2. The Nostr slot uses the existing schemes `nostr_npub` and `nostr_nprofile` of `rss::validate_nostr_identity`. The packet names the scheme `npub`. The parse keeps only valid identities of direct `podcast:txt purpose="npub"` elements.
3. When no enclosure candidate of an item is supported, the parse keeps the direct `<enclosure>`, as the subscribe command did before. Without this fallback, a feed with an unsupported format would lose its audio URL.
4. On a `music_to_publisher` row, the publisher feed GUID is the remote feed GUID. The check thus writes `publisher_feed_guid` with `remote_feed_guid`, so that a later MusicIndex row with the same key updates that row. A feed without such a row gets a new row with no derived value. A document without a `podcast:publisher` remote item deletes the `music_to_publisher` rows. A remote item without `feedGuid` counts as no remote item, because the key column cannot be empty.
5. The fact-backed slots compare the hold, then the `musicindex` fact, then the column or `rss` fact. Packet 020 uses this order for display. The first check of a feed can therefore report a difference for a field that MusicIndex never supplied, such as the feed owner.
6. An invalid URL compares as its trimmed text. The comparison contract calls this result "unknown". The check applies RSS values, so it needs a result.
7. The podping.me link opens `https://podping.me/`. The report text names the feed URL to submit. The implementer found no recorded URL form that takes a feed URL.
8. A difference row shows at most 160 characters of each value. The difference row in the database keeps the full value.
9. The migration fill count goes to the error stream. The migration registry returns no value.
10. The download action of a `track_added` row is available while the track row exists. It does not show whether the download already ran. The existing Library download state reports the progress.
11. `retained_digest` now hashes the named version-11 columns, not `SELECT *`. Migration 16 appends columns to `feeds` and `tracks`, and those columns must not change the digest of a retained row.
12. `db::replace_local_contributors`, `replace_local_identity_links` and `replace_local_identity_ids` now call `write_*` functions that use the transaction of the caller. The check uses these functions inside its one transaction.
13. Two tests used a version-12 fixture for the MusicIndex feed update. They now use `CURRENT_VERSION`, because the gate reads `rss_field_holds`.
14. The guard `source_fact_placeholder_and_breadcrumb_regressions_are_guarded` requires the text `if description.is_some() {`. The gate call is thus inside that block.
15. This packet does not change these documents: `AGENTS.md`, the phase plans, the ADRs and the delivery order. The orchestrator owns them.

### Checks - 2026-09-24

| Check | Result |
|---|---|
| `cargo test --lib adr_0076_rss_comparison` | Green, 24 tests |
| `cargo test --lib adr_0076_playlist_check` | Green, 21 tests |
| `cargo test` | Green, 1741 unit tests and 275 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 275 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.


### Later Corrections

ADR 0076 packet 006 changed two parts of this result on 2026-09-25. A subscribe now holds the persons slot. `identity_ingest::persist_contributors` is a new gate site, and the R2-18 guard lists it.

## Operator Visual Check

Run this check only after the operator resumes visual checks.

This check needs a Linux desktop session, this checkout, the `sqlite3` command and a network connection.
It needs one playlist with tracks from one or more feeds. One of those feeds must have a track that is not in the playlist.

**Each check sends real HTTP requests to the feed hosts of the playlist.** Wavlake throttles crawlers. Do not start many checks in a short time.

**Migration 16 drops three tables when the new build opens the database.** Make the backup with SQLite while the app is closed.
The check also writes RSS values into the stored feed and track values. Only the backup restores the earlier values.

1. Close v4vmm. Find the configured database path:

   ```bash
   cd /home/citizen/build/v4vmm
   db=$(sed -n 's/^db_path = "\(.*\)"$/\1/p' ~/.config/v4vmm/config.toml)
   echo "$db"
   ```

   An empty line is wrong. Set `db` to the `db_path` value from `~/.config/v4vmm/config.toml`.
2. Record the schema version:

   ```bash
   sqlite3 "$db" "SELECT max(version) FROM schema_migrations;"
   ```

   Expect `15`. If the version is 16, the database has already migrated. Use the backup of the first run.
3. Make the backup and verify it:

   ```bash
   backup="$db.before-adr-0076-task-002.sqlite"
   test ! -e "$backup" && sqlite3 "$db" ".backup '$backup'"
   sqlite3 "$backup" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
   ```

   Expect `ok` and `15`. A different result is wrong. Do not continue without a verified backup.
4. Build and open the app once, so that migration 16 runs. Then close it:

   ```bash
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   sqlite3 "$db" "SELECT version, name FROM schema_migrations WHERE version = 16;"
   ```

   Expect `16|rss_field_holds_and_differences`. The terminal shows one sentence with the album artist counts.
5. Find a playlist with two or more tracks of one feed:

   ```bash
   sqlite3 "$db" "SELECT pt.playlist_id, t.feed_id, t.id, t.item_guid, t.track_title FROM playlist_tracks pt JOIN tracks t ON t.id = pt.track_id WHERE t.feed_id IN (SELECT t2.feed_id FROM playlist_tracks p2 JOIN tracks t2 ON t2.id = p2.track_id GROUP BY p2.playlist_id, t2.feed_id HAVING count(*) >= 2) ORDER BY pt.playlist_id, t.feed_id LIMIT 6;"
   ```

   From one playlist and one feed, write down the feed id `F`, two track ids `T1` and `T2`, and the `item_guid` `G` of `T2`.
   If the list is empty, add two tracks of one album to a playlist in the app first.
6. Simulate a feed that changed since the last MusicIndex ingest. Replace `F`, `T1` and `T2` with the values of step 5:

   ```bash
   sqlite3 "$db" "PRAGMA foreign_keys=ON; UPDATE feeds SET title = title || ' (old)' WHERE id = F; UPDATE tracks SET track_title = track_title || ' (old)' WHERE id = T1; UPDATE tracks SET item_guid = item_guid || '-adr-0076-task-002' WHERE id = T2;"
   ```

   The check must then restore both titles. It must mark track `T2` "removed from feed", because RSS does not list its changed GUID.
   It must also store the RSS item `G` again as a new track.

7. Open the app:

   ```bash
   ./target/debug/v4vmm
   ```

8. **V1 - the report in place.** Open **Music**, select the playlist of step 5, and click **Check RSS**. Do not move to another page.
   - The **RSS check** section gives one sentence with the number of RSS values that the check wrote.
   - Each difference gives the field and the feed, the old value with "(old)", the new value from RSS, and "Checked at" with a UTC time.
   - Each stale feed gives one sentence with its feed URL and an **Open podping.me** button. The button opens podping.me in the browser.
   - The album title and the track title in the page change to the RSS values without navigation.
   - Open **Settings → General** with Ctrl+Comma. Select Light, and read the report. Then select Dark, and read it again.

   Each of these results is wrong:
   - a report that shows only after navigation,
   - a row without its old or new value,
   - a missing podping.me button,
   - text that you cannot read in one theme.
9. **V2 - new and removed tracks.**
   - The report has a "Track added" row for the RSS item `G` with an enabled **Download** button. Its tooltip names the track.
   - Clicking **Download** is optional. It downloads a real file. If you click it, the Library download status reports the download.
   - The report has a "Track removed" row for track `T2`.
   - The playlist row of track `T2` shows the badge "Removed from feed" with the check date. The row stays in the playlist.

   A missing row, a disabled **Download** button, a missing badge, or a row that left the playlist is wrong.
10. **V3 - widths.** Make the window narrow, then wide again. Read each report row and the badge at both widths.
    - Long values wrap to more lines. A value never overlaps another row.
    - Stacked text follows the column text rule of ADR 0063: no row text ends in a cut-off word with "…" that the view model did not add.

    Text that overlaps, is clipped, or is unreadable at one width is wrong.
11. Close the app. Confirm the stored results. Replace `T2` with its id:

    ```bash
    sqlite3 "$db" "SELECT kind, field, count(*) FROM rss_check_differences GROUP BY kind, field;"
    sqlite3 "$db" "SELECT removed_from_feed_at IS NOT NULL FROM tracks WHERE id = T2;"
    ```

    Expect `changed|title` rows, one `track_removed|track` row, one `track_added|track` row, and the mark `1`.

### Cleanup And Restore

If you clicked **Download** in V2, remove that file first with the Library remove action of the added track.

Undo the fixture of step 6. Replace `F`, `T2` and `G` with the values of step 5:

```bash
sqlite3 "$db" "PRAGMA foreign_keys=ON; DELETE FROM tracks WHERE feed_id = F AND item_guid = 'G'; UPDATE tracks SET item_guid = 'G', removed_from_feed_at = NULL, removed_from_feed_confirmed_at = NULL WHERE id = T2;"
sqlite3 "$db" "PRAGMA foreign_key_check; PRAGMA integrity_check;"
```

Expect no foreign key row and `ok`. The applied titles are the RSS values, so they need no change.

Keep the backup until the operator accepts this check.

To undo the upgrade and the applied RSS values, close the app and restore the backup:

```bash
cp "$db" "$db.after-adr-0076-task-002.sqlite"
sqlite3 "$db" ".restore '$backup'"
sqlite3 "$db" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
```

Expect `ok` and `15`. Only a build before this packet can open that version-15 database without a new upgrade.

After acceptance, remove the files that this check made:

```bash
rm -i "$backup" "$db.after-adr-0076-task-002.sqlite"
```

The app preservation directory `.v4vmm-upgrade-*` beside the database is an ADR 0066 artifact. Keep it, or remove it with the other upgrade backups.
