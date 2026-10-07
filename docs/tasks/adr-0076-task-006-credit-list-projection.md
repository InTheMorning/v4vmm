# ADR 0076 Task 006: Credit List Projection

Status: Open - implementation and mechanical checks are complete on 2026-09-25. On 2026-10-07 the operator moved its visual check to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07). The packet that rebuilds the surface carries it.
This packet changes the credit rows on the album and track pages. The operator check of V1 is open and paused.

## Goal

Show one credit list for each owner. The list follows the rule of the stored-value projection: the RSS list when a check or a subscribe holds it, else the MusicIndex list.
Both lists stay in storage as evidence.

## Authority

- The operator decision of 2026-09-25: one credit list for each owner, chosen by the projection rule.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: MusicIndex is a cache of RSS, and a screen does not label a value with its provider.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decisions 1 and 5.
- [ADR 0075 packet 020](adr-0075-task-020-stored-value-projection.md): the projection owner `src/application/queries/stored_values.rs`.

## Recorded Facts - 2026-09-25

- `entity_contributors` stores one list for each owner and source. `db::local_contributors` returns the rows of all sources.
- `identity_ingest.rs` writes the MusicIndex list through `replace_local_contributors` with source `musicindex`. The check and the subscribe write the RSS list with source `rss`.
- The packet 002 comparison holds the persons slot. Packet 020 recorded that credit lists show rows from all sources.

## Required Changes

Add the credit list to the projection. For each owner:

1. when a hold exists for the persons slot, the list with source `rss`,
2. else the list with source `musicindex`,
3. else the list with source `rss`, when one exists.

A cleared hold gives an empty list. Each list keeps its order.
`FeedView` and `TrackView` read the credit list from the projection. The MusicIndex writer of the credit list calls the packet 002 gate, like the other compared slots.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_credit_list_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R6-01 | With a persons hold, the view shows only the RSS list, in its order |
| R6-02 | Without a hold, the view shows only the MusicIndex list |
| R6-03 | A cleared hold gives an empty list, although a MusicIndex list exists |
| R6-04 | Both lists stay in `entity_contributors` after a check |
| R6-05 | The MusicIndex credit writer calls the gate. The R2-18 guard lists the site |
| R6-06 | No credit row carries a provider label |

## Visual Acceptance Criteria

For the operator. No test proves them.

- V1: after a check that changed the credits, the track page shows the RSS credits once, with no duplicate rows, in Light and Dark themes.

## Exclusions

- No migration. No change to the stored lists.
- No merge of persons across owners. Person identity stays deferred under ADR 0079.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/application/queries/stored_values.rs`, `src/views.rs`, `src/local_identity.rs`, `src/sources.rs`.
- `src/db.rs`: `local_contributors`, `replace_local_contributors`. `src/identity_ingest.rs`.
- `src/rss/check_apply.rs`: `persons`, `persons_value`.

## Checks

```bash
cargo test --lib adr_0076_credit_list
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Implementation Result - 2026-09-25

### Projection Change

`src/application/queries/stored_values.rs` now projects the credit list of each owner.
`FeedStoredValues.credits` and `TrackStoredValues.credits` hold the list. `feed_credits` gives the feed list alone.

`project_credits` reads the rows of `db::local_contributors` and keeps the `musicindex` rows and the `rss` rows apart.
It calls the one `select` function of the module with these inputs:

1. the hold of the persons slot. A hold with a value gives the `rss` list. A cleared hold gives an empty list.
2. the `musicindex` list, when it has a row.
3. the `rss` list.

Each list keeps its stored order. A row of another source token is evidence only.
Each credit is a `ContributorView`, which has no source field. The projection merges no person across the two lists.
The column step functions `feed_values_from_columns` and `track_values_from_columns` read no row, so their list is empty.

### Converted Readers

| Reader | Before | After |
|---|---|---|
| `views::FeedView::from_local_with_facts` | `LocalIdentityFacts.contributors`: the rows of all sources | `FeedStoredValues.credits` |
| `views::TrackView::from_local_with_facts` | `LocalIdentityFacts.contributors`: the rows of all sources | `TrackStoredValues.credits` |
| `local_identity::facts_for_owner` | Read the credit rows of all sources | Reads links and identifiers only. `LocalIdentityFacts` has no `contributors` field |
| `sources::LocalSource` feed and track views | The rows of all sources, through `local_identity` | The projection, through the two views |
| `ui::shells::library::feed_detail` album page | `album.identity_facts` credits | `album.stored_values` credits, through `FeedView` |
| `library::app_impl::album_thumbnail_urls` | The image of each credit row of all sources | The image of each projected credit |
| `feed_service::track_row_to_track_context_with_local_identity` | `hydrate_feed_identity` and `hydrate_track_identity` read the rows of all sources | The projected lists. The Library track detail, the search results, the tag frames and the tag update scan use this context |
| `application::queries::library::fetch_library_track_context_with_local_fallback` | The fetched `MusicIndex` credit lists replaced the stored lists on the track page | `apply_projected_credits` puts the projected lists of the local context on the page. Finding 2 gives the reason |

`feed_service::contributor_from_local` had no caller after the change, so it is deleted.

### Readers Left

1. `rss::check_apply::Apply::persons` reads the `rss` rows only. It is the comparison of packet 002, not a display reader.
2. `feed_service::merge_track_context_with_recorder` puts a fetched `MusicIndex` response over the columns. It reads no stored credit row. The Library track detail then applies the projected lists (finding 2). The ADR 0007 tag comparison and `apply_feed_updates` use the response, as packet 020 recorded.
3. The Index track page and the Discover inspector show credits of a `MusicIndex` response. They read no stored row. No composition root constructs `SearchApp`, as packet 005 recorded.
4. The track rows of the album page use `TrackView::from_local`. That path has no connection, so the list is empty, as it was before this packet.
5. `subscribe_service::track_row_to_api_track` and the download tag frames read columns only.

The new guard `adr_0076_credit_list_readers_use_the_projection` keeps this list closed. Only the projection and `rss::check_apply` can call `db::local_contributors` in production code.

### Gate Site

`identity_ingest::persist_contributors` is the one `MusicIndex` credit writer. It now calls `db::rss_field_holds::musicindex_gate` with the persons value of the response and its `updated_at` value.
The `musicindex` list is a separate stored list, so the writer always writes it. The gate call releases a held RSS list when `MusicIndex` agrees or has a newer record. The projection then shows the `musicindex` list.

`persons_value` moved from `rss::check_apply` to `db::rss_field_holds`. The check, the subscribe and the gate claim use this one JSON form.

The R2-18 guard `adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate` lists the site `fn persist_contributors(` with the write `db::replace_local_contributors(`.
It also permits a production call of `replace_local_contributors(` only in `src/db.rs`, `src/identity_ingest.rs` and `src/rss/subscribe.rs`. The subscribe writes the `rss` list.

### Findings And Deviations

1. A subscribe now writes the hold of the persons slot with the RSS value. Before, `record_subscribed_document` deleted that hold, because no `MusicIndex` writer changed the slot. The `MusicIndex` credit writer is now gated, so the old deletion would show an older `MusicIndex` list after a subscribe. The packet goal names this case: "the RSS list when a check or a subscribe holds it". The persons slot left `SUBSCRIBED_CHANNEL_COLUMN_FIELDS` and `SUBSCRIBED_ITEM_COLUMN_FIELDS`, and the test `adr_0076_credit_list_subscribe_holds_the_rss_list` proves the change.
2. After a successful fetch, the Library track page showed the credit lists of the `MusicIndex` response. The command does not store that response. Without a change, V1 fails when `MusicIndex` is stale. `apply_projected_credits` now puts the projected lists of the local context on the page. The other fetched values stay, but this change also affects the "Contributors" row of the ADR 0007 tag comparison on that page: its first column shows the projected list. The operator can reject this at V1.
3. The expected ID3 contributor frames of the tag update scan and the tag frames of a local context now use the projected list. Before, they used the rows of all sources.
4. A track that a check adds (`track_added`) gets the `rss` list and no persons hold. A later `MusicIndex` list then shows, by rule 2. This is the order of the other slots of an added track.
5. The ID3 frame name `TXXX:MusicIndex Contributors` can show in the frame column of the track metadata grid. It is the name of an audio tag frame of ADR 0007, not a provider label on a credit row. This packet does not change it.
6. `view_models::library` and `views` tests built `LocalIdentityFacts` with credits. They now put the credits in the stored values.
7. This packet does not change `AGENTS.md`, the phase plans, the ADRs, the source map or the delivery order. The orchestrator owns them.

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R6-01 | `adr_0076_credit_list_hold_shows_only_the_rss_list_in_order` | `src/application/queries/stored_values.rs` |
| R6-02 | `adr_0076_credit_list_without_hold_shows_only_the_musicindex_list` | `src/application/queries/stored_values.rs` |
| R6-03 | `adr_0076_credit_list_cleared_hold_gives_an_empty_list` | `src/application/queries/stored_values.rs` |
| R6-04 | `adr_0076_credit_list_check_keeps_both_stored_lists` | `src/application/queries/stored_values.rs` |
| R6-05 | `adr_0076_credit_list_musicindex_writer_calls_the_gate` | `src/identity_ingest.rs` |
| R6-05 | `adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate` (R2-18, new site) | `tests/architecture_tests.rs` |
| R6-06 | `adr_0076_credit_list_rows_carry_no_provider_label` | `src/application/queries/stored_values.rs` |
| Finding 1 | `adr_0076_credit_list_subscribe_holds_the_rss_list` | `src/rss/check_apply.rs` |
| Finding 2 | `adr_0076_credit_list_track_page_uses_projected_credits` | `src/application/queries/library.rs` |
| Guard | `adr_0076_credit_list_readers_use_the_projection` | `tests/architecture_tests.rs` |

R6-01 to R6-03 read the views through `LocalSource`. R6-02 also proves the `rss` list without a `musicindex` list, and a row of another source token that the view does not show.
R6-06 reads the projected lists, the views and the track context of the tag frames. No credit has a source value, and no debug text names a provider.

The new guard is situational and names ADR 0076 packet 006 and the fix. Packet 020 finding 6 is the incident: a track page listed the `rss` credits and the `musicindex` credits together.

### Checks - 2026-09-25

| Check | Result |
|---|---|
| `cargo test --lib adr_0076_credit_list` | Green, 8 tests |
| `cargo test --lib adr_0075_projection` | Green, 10 tests |
| `cargo test --lib adr_0076_rss_comparison` | Green, 24 tests |
| `cargo test` | First run: 1807 passed, 1 failed. Second run: Green, 1808 unit tests and 280 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 280 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

The failed test of the first full run is `runtime::broadcast_readiness::tests::broadcast_readiness_actor_refreshes_on_command`. It waits one second for an actor report. It passed alone and in the second full run. This packet does not change that module.

No application launch and no production-data change occurred. This packet adds no migration. Schema version 17 stays current.

## Operator Visual Check

Run this check only after the operator resumes visual checks.

This check needs a Linux desktop session, this checkout, the `sqlite3` command and a network connection.
It needs the schema version 17 of packet 003. It needs one playlist track whose RSS item has `podcast:person` credits.

This packet adds no migration, so it needs no backup.
The fixture changes the stored credit rows of one track, and the cleanup removes the fixture row.
**The RSS check sends real HTTP requests to the feed hosts of the playlist.** Wavlake throttles crawlers. Do not start many checks in a short time.
The check writes RSS values into the stored values, as each check does.

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

   Expect `17`. A different value is wrong. Stop, and complete the check of packet 003 first.
3. Find a playlist track with RSS credits:

   ```bash
   sqlite3 "$db" "SELECT pt.playlist_id, t.feed_id, t.id, t.track_title, count(c.id) FROM playlist_tracks pt JOIN tracks t ON t.id = pt.track_id JOIN entity_contributors c ON c.owner_kind = 'track' AND c.track_id = t.id AND c.source = 'rss' GROUP BY pt.playlist_id, t.id LIMIT 5;"
   ```

   Select one row. Set its values in the shell:

   ```bash
   P=<playlist_id>
   T=<track id>
   ```

   An empty list means that no playlist track has RSS credits. Stop. This check then cannot run.
4. Record the RSS credits of the track:

   ```bash
   sqlite3 "$db" "SELECT position, name, role FROM entity_contributors WHERE owner_kind = 'track' AND track_id = $T AND source = 'rss' ORDER BY position;"
   ```

   Write down each name and the order.
5. Apply the fixture. It makes the RSS credits old, and it adds one MusicIndex credit:

   ```bash
   sqlite3 "$db" "UPDATE entity_contributors SET name = name || ' (old)' WHERE owner_kind = 'track' AND track_id = $T AND source = 'rss'; INSERT INTO entity_contributors(owner_kind, track_id, position, name, role, source) VALUES ('track', $T, 999, 'Fixture Index Credit', 'guest', 'musicindex');"
   ```

   An error is wrong. Stop, and run the cleanup.
6. Build the binary and open the app:

   ```bash
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   ```

7. Open **Music**, and select the playlist `P`. Click the row of track `T` to open its track page.
   - Read the **Contributors** row in the **People** group. It shows the names with "(old)", or it shows a list with "Fixture Index Credit".
   - A list with both "(old)" names and "Fixture Index Credit" is wrong.
8. Go back to the playlist `P`, and click **Check RSS**. Wait until the **RSS check** section says that the check finished.
   - The report has a difference for the credits of track `T`, with the "(old)" names as the old value.
9. **V1 - track page.** Click the row of track `T` again to open its track page.
   - The **Contributors** row shows the names of step 4, in the order of step 4.
   - Each name shows one time.
   - No name ends in "(old)". "Fixture Index Credit" does not show.
   - No credit shows a provider label, such as "RSS" or "MusicIndex".
10. Open **Settings → General** with Ctrl+Comma. Select Light, and do step 9 again. Then select Dark, and do it again.
11. Close the app.

Each of these results is wrong:

- a name that shows two times,
- a name with "(old)" after the check,
- "Fixture Index Credit" on the track page after the check,
- a new list that shows only after you restart the app,
- a credit with a provider label,
- text that you cannot read in one theme.

### Cleanup

Remove the fixture credit. The check of step 8 normally restores the RSS names. The second command also restores them when the check failed:

```bash
sqlite3 "$db" "DELETE FROM entity_contributors WHERE owner_kind = 'track' AND track_id = $T AND source = 'musicindex' AND name = 'Fixture Index Credit';"
sqlite3 "$db" "UPDATE entity_contributors SET name = replace(name, ' (old)', '') WHERE owner_kind = 'track' AND track_id = $T AND source = 'rss';"
sqlite3 "$db" "PRAGMA integrity_check;"
```

Expect `ok`. Restore the theme that you used before step 10 in **Settings → General**.
This check makes no file.

The gate stays open in this `Status:` line and in [pending human checks](../pending-human-checks.md) until the operator walks it.
