# ADR 0075 Task 020: Stored Value Projection

Status: Implemented - 2026-09-24. Mechanical checks are Green. The visual gate is open.
This packet changes which stored value a screen shows. The V1 gate stays open and paused until the operator walks it.

## Goal

Give the app one shared projection of the current value of each field, with its owner.
Views and tag frames read that projection. No caller selects a source at display time.

ADR 0076 reduced this packet on 2026-09-24. It no longer selects a source by freshness.

## Authority

- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decisions 1 and 5.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: provenance is the RSS element, and the provider is transport.
- [ADR 0054](../adr/0054-local-metadata-source-fact-persistence.md): facts persist by source. This packet reads them and changes no row.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability.

## Recorded Facts - 2026-09-24

- `FeedView::from_local_with_facts` and `TrackView::from_local_with_facts` in `src/views.rs` read the MusicIndex fact, then the column.
- `feed_service::track_row_to_track_context` builds the tag frame context from `TrackRow` columns only. It reads no fact.
- `metadata_service::id3_edits_for_track_context` builds the expected ID3 frames from that context.
- ADR 0076 packet 002 adds `rss_field_holds` and writes the RSS value to the slot.

## Required Changes

### One Projection

Add one module in `src/application/queries/`. It returns the current value of each accepted field for a feed or a track. Each value names its owner: channel, item or person.

The order for each field is fixed and written in the module:

1. the held RSS value, when `rss_field_holds` has a row for the owner and field,
2. the MusicIndex fact, when one exists,
3. the column.

A cleared hold (null value) gives no value. The module carries no renderer type and no provider label for display.

The album artist of a track view comes from `feeds.album_artist`, with the channel as its owner. The track column `album_artist_name` is the fallback only when the feed column is null.
The operator decided this feed column on 2026-09-24.

### Readers

`FeedView::from_local_with_facts`, `TrackView::from_local_with_facts` and `track_row_to_track_context` read the projection.
The implementer lists each other reader that combines a fact and a column, and converts it or records it as a finding.

### Accepted Fields Only

The projection covers the compared elements of ADR 0076 packet 002 and the fields with an accepted ADR 0075 rule.
A field with an open policy keeps its current reader and is listed in the result section.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_projection_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R20-01 | With a hold, the projection returns the held value for that field, and the MusicIndex fact for another field |
| R20-02 | Without a hold, the projection returns the MusicIndex fact, then the column |
| R20-03 | A cleared hold returns no value, although the column holds one |
| R20-04 | `FeedView` and `TrackView` show the held value after a packet 002 apply |
| R20-05 | The expected ID3 frames of a track use the held value |
| R20-06 | Each field of the projection names its owner. No projection value carries a provider label |
| R20-08 | A track view shows `feeds.album_artist` as the album artist, owned by the channel. With a null feed column, it shows `tracks.album_artist_name` |
| R20-07 | A guard proves that `src/views.rs` and `src/view_models/` combine no fact and column outside the projection. Its message names ADR 0076 Decision 1 |

## Visual Acceptance Criteria

For the operator. No test proves them.

- V1: after a check applied a title change, the album page and the track page show the new title without navigation, in Light and Dark themes.

## Exclusions

- No new stored value and no migration.
- No source label on a screen.
- No change to the compare table of ADR 0007. It shows sources by design.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/views.rs`, `src/feed_service.rs`, `src/metadata_service.rs`, `src/metadata.rs`.
- `src/db.rs`: `local_metadata_facts`. The packet 002 hold table.

## Checks

```bash
cargo test --lib adr_0075_projection
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Implementation Result - 2026-09-24

### Module And Order

`src/application/queries/stored_values.rs` is the one projection. `select` holds the order, and no other function selects a source:

1. the held RSS value, when `rss_field_holds` has a row for the owner and the field. A cleared hold gives no value.
2. the `musicindex` fact, when one exists.
3. the column.

Three fields have no column: the feed explicit flag, the feed owner name and the track description.
Their `rss` fact row is the column step. Packet 002 used the same rows as its fallback.

`feed_values` and `track_values` read the database. `feed_values_from_columns` and `track_values_from_columns` apply the column step alone, for a caller without a connection.
`db::rss_field_holds::owner_holds` reads the holds of one owner in one query. `db::stored_feed_columns` reads the feed columns, including `feeds.album_artist`.
`local_metadata` now keeps the `musicindex` rows and the `rss` rows apart. A row of another source token is evidence only.

Each value is an `Owned` value with a `ValueOwner` of `Channel` or `Item`. No value carries a provider label or a renderer type.
The enum has no person variant, because no projected scalar value belongs to a person. The `podcast:person` credits stay in the contributor rows. Finding 6 gives the details.

### Owners Of Track Values

- The album artist of a track is `feeds.album_artist`, owned by the channel. The track column `album_artist_name` is the fallback only when the feed has no hold and a null column. That fallback is owned by the item, because the subscribe step can copy the item author into it.
- The album title of a track is the channel title, with its hold. The track copy `album_title` is the fallback only when the feed title is null. Finding 1 explains this deviation.
- The artwork of a track is the item artwork. The channel artwork is the fallback when the item has none (ADR 0075 Decision C).
- A track view without its own artist shows the album artist. This is a display fallback between two projected values.

### Packet 002 Comparison

`check_apply::Apply::fact_backed` now calls `stored_values::compared_slot`. The comparison had its own copy of the order, and that copy is deleted.
`compared_slot` reads the hold, the decoded `musicindex` fact and the column in the same order as the display. It reports if the value came from `MusicIndex`, because a `MusicIndex` description is plain text.

The `musicindex` description is now the decoded fact of the display: a description claim, then the top-level description. Before, the comparison used the newest row. Both give the claim when one exists. All 24 `adr_0076_rss_comparison` tests stay Green.

### Subscribe Writes RSS Values - Orchestrator Decision Of 2026-09-24

The orchestrator decided that findings 2 and 3 are defects of packets 002 and 020 together, and that packet 020 corrects them.
The rule is a technical reading of ADR 0076 Decision 5. A subscribe reads the RSS document, so its values of the compared slots are RSS values, as the values of a check are.

- `rss::subscribe::persist_subscribed_document` persists the channel and the items. Then it calls `check_apply::record_subscribed_document`. `subscribe_feed` calls this persist step.
- `record_subscribed_document` writes the `rss` fact row and the hold of each fact-backed slot with the fresh RSS value. A field that the document does not state gets a cleared hold. The hold has no run, and the function records no difference.
- The check and the subscribe use the same slot builders, `channel_fact_slots` and `item_fact_slots`. The check apply code now uses these builders for its RSS values and its `rss` fact rows.
- The subscribe deletes an earlier hold of each column-backed slot that the persist step writes, so an older held value cannot hide the new column. No `MusicIndex` writer changes these slots. The Nostr and publisher slots keep their holds, because the subscribe does not write them.
- `subscribe_feed` no longer writes a `MusicIndex` feed description. The gate call is removed. The fetch of `MusicIndex` stays for the baseline `updated_at` value only.
- The R2-18 guard no longer lists `subscribe_feed` as a gate site. Its message says that the subscribe is not a `MusicIndex` writer.
- The guard `source_fact_placeholder_and_breadcrumb_regressions_are_guarded` required the placeholder test of the removed write. It now requires that `src/rss/subscribe.rs` has no `set_feed_description(` call.

A subscribe of a feed without `MusicIndex` data also writes the holds. A later `MusicIndex` write then changes a slot only when it agrees or sends a newer `updated_at` value.
The packet 002 result still lists `subscribe_feed` as gate site 1. This packet does not edit that document.

### Converted Readers

| Reader | Before | After |
|---|---|---|
| `views::FeedView::from_local_with_facts` | The fact, then the column | Takes `FeedStoredValues` |
| `views::TrackView::from_local_with_facts` | The fact, then the column | Takes `TrackStoredValues`. `TrackView` has a new `album_artist` field |
| `feed_service::track_row_to_track_context` | The columns only | The column step of the projection |
| `feed_service::track_row_to_track_context_with_local_identity` | `hydrate_track_metadata`: each fact replaced its column | The full projection. `hydrate_track_metadata` is deleted |
| `sources::LocalSource` feed and track views | The facts of all sources, then the column | `feed_values` and `track_values` |
| `application::queries::library::build_tree` | The album description came from the subscribed feed column | `AlbumNode.stored_values` and the projected description |
| `application::queries::library` album hydration | The album description was the description of the `MusicIndex` response. It ignored a hold | The projected description after the writes. `AlbumIdentityHydration.stored_values` carries the values |
| `library::app_impl` album node for a feed | The `MusicIndex` facts only | Also `stored_values` |
| `ui::shells::library::feed_detail` album page | Passed the facts to `FeedView` | Passes `album.stored_values`. An album without a feed row uses its columns |
| `rss::check_apply` fact-backed slots | Its own order | `stored_values::compared_slot` |
| `rss::subscribe::subscribe_feed` | A gated `MusicIndex` description write. No hold | The RSS value and the hold of each fact-backed slot |

`AlbumNode.metadata_facts` stays. It now holds the `musicindex` facts only, and the Library uses it to find out if an album has had its hydration.

### Readers Recorded And Not Converted

1. `feed_service::merge_track_context_with_recorder` puts a fetched `MusicIndex` response over the columns. The Library track detail command, the tag comparison of ADR 0007 and `apply_feed_updates` use it. It reads no stored fact, and the compare table shows sources by design. Finding 4 gives its effect on tag writes.
2. `view_models::library::LibraryTrackRowVm::display_artist` and `display_album` group the Library tree by the track copies `album_artist_name` and `album_title`. They read no fact. A view model cannot read the projection without a query change.
3. The album page builds each track row with `TrackView::from_local`. That path has no connection, so it uses the column step. The track title column holds the same value as the hold after a check.
4. `subscribe_service::subscribe_library_track_internal` builds the tag frames of a download from `track_row_to_api_track`. It reads columns only.
5. `track_identity.rs` and `app/queue_now_playing.rs` show the artist column, then the album artist column. They read no fact.
6. The contributor, Nostr and link rows: `local_identity::facts_for_owner` lists the rows of all sources. The check compares the `rss` rows only.
7. Payment routes: the local views show no route. Packet 003 owns the stored route.

### Fields With An Open Policy

Packet 031 and packet 034 hold these fields. Each keeps its current reader:

- Track and disc numbers: the views read `track_number` and `disc_number`.
- Release kind and medium: the projection carries the `musicindex_release_kind` fact unchanged, with no hold and no column. The view could not keep its own fact read under the R20-07 guard. The value is the same as before.
- Aggregate counts: `episode_count` stays the track count of the view.
- Season, embedded album title, artist sort text, iTunes feed type and relationship evidence: no view or tag frame of this packet reads them.

The audio URL, the audio type and the duration have no fact. The views read their columns, and ADR 0024 guards keep those readers.

### Findings And Deviations

1. The album title of a track now comes from the channel. Before, a track view showed the track copy `album_title` first. The subscribe step writes that copy, and the playlist check does not change it. Without this change, V1 fails on the track page after a channel title change.
   The rule follows ADR 0076 Decision 1, "A feed value never becomes a track value", and the accepted feed-title reference of packet 031. The operator can reject it at V1.
2. Corrected. A released hold could leave an older `musicindex` fact in place. `subscribe_feed` called the gate for the feed description and wrote the column, but it wrote no `musicindex` fact. The subscribe now writes the RSS value and the hold, and it has no gate call. R20-09 proves the correction.
3. Corrected. A subscribe after a check did not change a hold, so a hold could keep an older RSS value than the new column. The subscribe now replaces each fact-backed hold and deletes each column-backed hold that it writes. R20-09 and R20-10 prove the correction.
4. Open follow-up. `feed_service::apply_feed_updates` writes audio tags from a fetched `MusicIndex` response. It does not read the projection, so it does not write a held RSS value. ADR 0076 Decision 8 gives tag writes to packet 004.
5. The feed description no longer prefers an `rss` description claim of a `MusicIndex` response over the top-level `MusicIndex` description. Such a claim uses the `rss` source token. The projection reads it only as the column step of a field without a column. ADR 0076 supersedes the source priority of ADR 0075 Decision F.
6. Open follow-up. The contributor lists show the rows of all sources. After a check, a track can list the `rss` credits and the `musicindex` credits together. A projection of these lists needs a product decision.
7. The Library album description now comes from the projection for each album with a feed row. Before, only a subscribed feed gave a description before the hydration.

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R20-01 | `adr_0075_projection_hold_wins_for_its_field_only` | `src/application/queries/stored_values.rs` |
| R20-02 | `adr_0075_projection_musicindex_fact_then_column` | `src/application/queries/stored_values.rs` |
| R20-03 | `adr_0075_projection_cleared_hold_gives_no_value` | `src/application/queries/stored_values.rs` |
| R20-04 | `adr_0075_projection_views_show_held_values_after_apply` | `src/application/queries/stored_values.rs` |
| R20-05 | `adr_0075_projection_tag_frames_use_held_value` | `src/application/queries/stored_values.rs` |
| R20-06 | `adr_0075_projection_values_name_owner_without_provider_label` | `src/application/queries/stored_values.rs` |
| R20-07 | `adr_0075_projection_views_combine_no_fact_and_column` | `tests/architecture_tests.rs` |
| R20-08 | `adr_0075_projection_album_artist_is_the_channel_value` | `src/application/queries/stored_values.rs` |
| R20-09 | `adr_0075_projection_resubscribe_replaces_check_hold` | `src/rss/check_apply.rs` |
| R20-10 | `adr_0075_projection_resubscribe_without_field_gives_no_value` | `src/rss/check_apply.rs` |
| R20-11 | `adr_0075_projection_subscribe_hold_gates_musicindex` | `src/rss/check_apply.rs` |

R20-04 applies two recorded documents with the packet 002 apply and reads the views through `LocalSource`.
R20-09 also proves that a subscribe records no difference.
R20-05 reads the `TIT2`, `TALB` and `TPE2` edits of `metadata_service::id3_edits_for_track_context`.

The R20-07 guard is situational and names ADR 0076 Decision 1 and the fix. It fails when:

- a line of `src/views.rs` or `src/view_models/` reads a metadata fact field,
- a local view constructor does not take `FeedStoredValues` or `TrackStoredValues`,
- `stored_values.rs` does not have exactly one `select`,
- the packet 002 check reads a fact without `compared_slot`.

Three tests in `src/views.rs` tested the old fact order in the view. They now test that the view shows the projected values.

### Checks - 2026-09-24

| Check | Result |
|---|---|
| `cargo test --lib adr_0075_projection` | Green, 10 tests |
| `cargo test --lib adr_0076_rss_comparison` | Green, 24 tests |
| `cargo test` | Green, 1751 unit tests and 276 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 276 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred. This packet adds no migration. Schema version 16 stays current.

## Operator Visual Check

Run this check only after the operator resumes visual checks.

This check needs a Linux desktop session, this checkout, the `sqlite3` command and a network connection.
It needs one playlist with a track from a feed that is still available by RSS.

This packet adds no migration, so it needs no new backup.
The fixture changes two stored titles, and the RSS check writes the RSS titles back.
**The RSS check sends real HTTP requests to the feed hosts of the playlist.** Do not start many checks in a short time.

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

   Expect `16`. If the result is `15`, do steps 1 to 4 of the [packet 002 procedure](adr-0076-task-002-rss-comparison-apply-and-report.md#operator-visual-check) first. They make the backup and run migration 16.
3. Build the app:

   ```bash
   cargo build --bin v4vmm
   ```

4. Find a playlist track and its feed:

   ```bash
   sqlite3 "$db" "SELECT pt.playlist_id, t.feed_id, t.id, f.title, t.track_title FROM playlist_tracks pt JOIN tracks t ON t.id = pt.track_id JOIN feeds f ON f.id = t.feed_id LIMIT 5;"
   ```

   From one row, write down the playlist id `P`, the feed id `F` and the track id `T`.
   Also write down the album title and the track title.
5. Make the two stored titles old. Replace `F` and `T` with the values of step 4:

   ```bash
   sqlite3 "$db" "UPDATE feeds SET title = title || ' (old)' WHERE id = F; UPDATE tracks SET track_title = track_title || ' (old)' WHERE id = T;"
   ```

6. Open the app:

   ```bash
   ./target/debug/v4vmm
   ```

7. Open **Music**. Open the album page of feed `F`, then the track page of track `T`. Both titles end in "(old)".
8. Select the playlist `P`, and click **Check RSS**. Wait until the report shows the changed titles.
9. **V1 - album page.** Open the album page of feed `F` again.
   - The album title is the RSS title, without "(old)".
   - The title row of track `T` is the RSS track title, without "(old)".
10. **V1 - track page.** Open the track page of track `T`.
    - The track title is the RSS track title, without "(old)".
    - The album name on the page is the RSS album title, without "(old)".
    - The album artist in the metadata grid is the album artist of the feed.
11. Open **Settings → General** with Ctrl+Comma. Select Light, and do steps 9 and 10 again. Then select Dark, and do them again.

Each of these results is wrong:

- a title with "(old)" after the check,
- a new title that shows only after you restart the app,
- a new title that shows only after you open another page and come back two times,
- an album name on the track page that differs from the album title on the album page,
- text that you cannot read in one theme.

### Cleanup

The check wrote the RSS titles into the stored values. The fixture of step 5 thus needs no undo.
Confirm the stored titles. Replace `F` and `T` with the values of step 4:

```bash
sqlite3 "$db" "SELECT title FROM feeds WHERE id = F; SELECT track_title FROM tracks WHERE id = T;"
```

Expect two titles without "(old)". If a title still ends in "(old)", the check did not reach that feed. Remove the suffix:

```bash
sqlite3 "$db" "UPDATE feeds SET title = replace(title, ' (old)', '') WHERE id = F; UPDATE tracks SET track_title = replace(track_title, ' (old)', '') WHERE id = T;"
```

Restore the theme that you used before step 11 in **Settings → General**.
This check makes no file. The packet 002 procedure owns its backup and the restore of that backup.
