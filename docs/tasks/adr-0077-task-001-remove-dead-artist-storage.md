# ADR 0077 Task 001: Remove Dead Artist Storage

Status: Open - implementation and mechanical checks are complete on 2026-09-24. On 2026-10-07 the operator moved its visual check to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07). The packet that rebuilds the surface carries it.
This packet changes one user-visible result: a Library artist view no longer shows stored aliases, area or active years.
The operator visual check below is pending. Visual checks stay paused until the operator resumes them.

## Goal

Delete the ADR 0045 track artist binding and the ADR 0029 artist subject storage in one change.
Add schema version 13, which drops the four tables.

No current source writes these tables. Stophammer removed `artist_credit` and the artist routes on 2026-04-08.
The stored rows can never refresh.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decision 7 deletes the ADR 0045 binding.
- [ADR 0079](../adr/0079-remove-musicindex-artist-subject-storage.md) Decision 1 deletes the artist subject storage.
- [ADR 0016](../adr/archive/0016-schema-migration-discipline.md) owns the migration registry.
- [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md) owns the startup schema check, the backup and the repair of an interrupted upgrade.

## Why One Packet

The two storage families share readers. `src/sources.rs` reads a binding and then the artist fact that it names.
One migration drops the four tables together. Two packets would each leave a half-deleted read path.

## Recorded Inventory - 2026-09-24

The implementer confirms each count before the first edit. A count that differs is a finding for the report.

| Symbol | Files that name it |
|---|---|
| `track_artist_source_bindings` | `src/db.rs` (39), `src/identity_ingest.rs` (6), `src/sources.rs` (3), `src/db/provider_observations.rs` (1) |
| `TrackArtistSourceBinding` | `src/db.rs` (19), `src/identity_ingest.rs` (3), `src/sources.rs` (3) |
| `artist_source_fact` | `src/db.rs` (66), `src/identity_ingest.rs` (22), `src/sources.rs` (10), `src/views.rs` (8), `src/discover/tests.rs` (2), `src/view_models/library.rs` (1), `src/db/provider_observations.rs` (1) |
| `artist_source_links`, `artist_source_ids` | `src/db.rs`, `src/db/provider_observations.rs` |
| `ArtistRef::Musicindex` | `src/sources.rs` (6), `src/views.rs` (3) |
| `persist_musicindex_artist` | `src/identity_ingest.rs` (7), `src/discover/app_impl.rs` (3), `src/discover.rs` (1), `src/discover/tests.rs` (2) |
| `source_subjects` | `src/views.rs` (5), `src/view_models/library.rs` (4), `src/sources.rs` (2) |

The SQL fixtures in `src/db/fixtures/` also create the tables. They record schema version 11, and they stay unchanged.

## Required Changes

### Code To Delete

- `persist_track_artist_bindings`, `explicit_artist_credit_id`, `ensure_musicindex_artist_source_fact` and `persist_musicindex_artist` in `src/identity_ingest.rs`, with their tests.
- `persist_musicindex_artist_facts` in `src/discover/app_impl.rs`, its call site and its tests.
- The `"artist"` arm of `Client::fetch_detail` in `src/api.rs`. It requests `/v1/artists/{id}`, which the live contract does not have.
- Each binding and artist fact function and row type in `src/db.rs`, with its tests.
- `ArtistRef::Musicindex`, `ArtistView::from_artist_source_fact`, the source fact half of `from_local_rows_with_artist_source_facts`, and `source_subjects` in `src/views.rs`.
- The binding and fact reads in `LocalSource::fetch_artist` and `local_artist_view_from_tracks` in `src/sources.rs`.
- `push_artist_source_rows` and the `source_subjects` rows in `src/view_models/library.rs`.
- The guard `screens_do_not_read_artist_source_facts` in `tests/architecture_tests.rs`. No function that it names remains.

### Code To Keep

- `ArtistRef::LocalArtistName` and the Library name grouping. ADR 0077 keeps it for an album without a publisher.
- `EntityDetail::Artist` and `api::Artist`. The Index search builds artist rows from feed and track name text. Packet 004 changes those rows.
- `Track.artist_credit` and `ArtistCredit` in `src/api.rs`, and their use in `src/musicbrainz.rs`. MusicBrainz lookup reads the credit text, not a binding.
- Migration versions 4 and 5. A registry entry stays, because an existing database records it.

### Migration

Add schema version 13 to `MIGRATIONS` in `src/db.rs`, and set `CURRENT_VERSION` to 13.
The migration drops `track_artist_source_bindings`, `artist_source_ids`, `artist_source_links` and `artist_source_facts`, in that order.
It uses the same transaction and ledger behavior as migration 12. The implementer confirms that behavior in `src/db.rs`.

Keep migrations 4 and 5. A fresh database runs 4, 5 and 13 in sequence and ends with no artist table.

Update each schema contract that names a dropped table. `VERSION_11_COLUMNS` describes version 11 and stays unchanged.
The contract for the current version must not name a dropped table.

The retention test in `src/db/provider_observations.rs` lists the tables that the observation migration keeps.
Remove the four names from its list of current tables. Do not weaken its other assertions.

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_remove_artist_storage_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R1-01 | A version 12 database with rows in each of the four tables migrates to version 13. The four tables are absent after the migration |
| R1-02 | The same migration keeps each row of `feeds`, `tracks`, `local_files`, `playlists`, `playlist_tracks` and the ADR 0075 observation tables |
| R1-03 | A fresh database migrates to version 13 and has none of the four tables |
| R1-04 | The version 11 fixtures still migrate to the current version. The ADR 0066 interrupted-upgrade fixture still repairs |
| R1-05 | `LocalSource::fetch_artist(ArtistRef::LocalArtistName(..))` returns the name-grouped tracks, with no source subject and no alias, area or year |
| R1-06 | The Library artist view model builds its rows without a source subject row |
| R1-07 | No code under `src/` names a dropped table, `ArtistRef::Musicindex`, `persist_musicindex_artist` or `source_subjects`. The test reports each site |
| R1-08 | The Index search still gives name-built artist rows. Their count and order equal the result before this packet |
| R1-09 | MusicBrainz lookup still reads `artist_credit` text. Its existing tests pass unchanged |

R1-07 is a situational guard. It names ADR 0079 and ADR 0077 Decision 7 in its failure message, with the fix:

```text
ADR 0079: MusicIndex artist subject storage is deleted. Use ArtistRef::LocalArtistName for a name grouping, or the ADR 0077 publisher feed GUID for an artist identity.
```

## Exclusions

- No publisher relationship decode, storage or page. Packets 002 to 004 own them.
- No change to the Index artist search rows. Packet 004 owns them.
- No change to `artist_credit` decode or MusicBrainz lookup.
- No change to a request, an include list or a request profile.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/db.rs`: `MIGRATIONS`, `CURRENT_VERSION`, the schema contracts, and the artist and binding functions.
- `src/db/upgrades.rs` and `src/db/startup.rs`: the ADR 0066 schema check and upgrade repair.
- `src/db/provider_observations.rs`: the retention test near line 1390.
- `src/identity_ingest.rs`, `src/sources.rs`, `src/views.rs`, `src/view_models/library.rs`.
- `src/discover/app_impl.rs` and `src/discover/tests.rs`.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0077_remove_artist_storage
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree before the first run on a real database.
After a real database migrates to version 13, the dropped rows are gone. A database backup taken before the run restores them.

## Implementation Result - 2026-09-24

The implementation session deleted the ADR 0045 track artist binding and the ADR 0029 artist subject storage in one change.
Schema version 13 drops the four tables. The code that the packet keeps is unchanged.

### Inventory Confirmation

Each count in the recorded inventory is correct for the files that the table names.
Two more sources name the symbols. The table does not list them, but they are not defects:

- The two SQL fixtures in `src/db/fixtures/` name the tables. The packet records them, and they stay unchanged.
- `tests/architecture_tests.rs` named `artist_source_fact` twice, in the guard that this packet deletes.

### Deleted Code

| File | Deleted items |
|---|---|
| `src/identity_ingest.rs` | `persist_musicindex_artist`, `persist_track_artist_bindings`, `ensure_musicindex_artist_source_fact`, `explicit_artist_credit_id`, the call in `persist_musicindex_track`, and six tests |
| `src/discover/app_impl.rs` | `persist_musicindex_artist_facts` and its call in the search result callback |
| `src/discover/tests.rs` | The test of `persist_musicindex_artist_facts` |
| `src/api.rs` | The `"artist"` arm of `Client::fetch_detail` |
| `src/db.rs` | The four artist row types, the binding and fact functions, their SQL row readers, `parse_string_array`, and nine tests |
| `src/views.rs` | `ArtistRef::Musicindex`, `ArtistSourceSubjectView`, `source_subjects`, `from_artist_source_fact`, `from_local_rows_with_artist_source_facts`, `apply_single_artist_source_fact`, `checked_year`, and two tests |
| `src/sources.rs` | `local_artist_view_from_tracks`, `artist_source_facts_for_tracks`, the `ArtistRef::Musicindex` arms, and five tests |
| `src/view_models/library.rs` | `push_artist_source_rows`, `push_string_row`, `artist_active_years`, the `view` field, `LibraryArtistDetailVm::with_view`, and two tests |
| `tests/architecture_tests.rs` | The guard `screens_do_not_read_artist_source_facts` |

`ApiSource::fetch_artist` now returns an error, because MusicIndex has no artist route.
`ArtistView::from_api` sets no artist reference. `LibraryApp::select_artist` builds its view with `ArtistView::from_local_rows`.

### Migration 13

`MIGRATIONS` in `src/db.rs` has version 13, `drop_artist_storage`. `CURRENT_VERSION` is 13.
The migration drops `track_artist_source_bindings`, `artist_source_ids`, `artist_source_links` and `artist_source_facts`, in that order.
The constant `ARTIST_STORAGE_TABLES` holds that order.

Migration 12 used a transaction that holds the migration and its ledger record. Migration 13 uses the same behavior.
When migrations 12 and 13 are both pending, they share one transaction. A failure in migration 13 thus rolls back migration 12 too.
The ADR 0066 preparation then verifies the version-11 rollback as before.

After migration 13, the registry verifies the exact version-13 schema. It also compares a digest of the retained version-11 rows before and after the drop.

`VERSION_11_COLUMNS` and the version-11 SQL fixtures are unchanged. The new function `schema_contract(version)` gives the read contract of each version.
The contract for version 13 names no dropped table. The startup schema check, `verify_target` and the ADR 0066 contract test use this function.

### Findings And Deviations

1. The startup schema check refused each version-12 database. It treated the ADR 0075 metadata tables as an unknown partial upgrade.
   The check now accepts a version-12 database that matches the version-12 contract. It reports the database as an upgrade to version 13.
   Without this change, no existing database could reach version 13.
2. `legacy_digest` hashes all version-11 tables. It cannot read a version-13 database. The new `retained_digest` hashes only the retained version-11 tables.
   Tests that compare a version-11 original with a version-13 result use `retained_digest`.
3. Sixteen ADR 0066 and ADR 0075 tests expected version 12 as the current version. They now expect version 13 or `CURRENT_VERSION`.
   Three test names contained `12` for the current version. Their names now contain `current`.
   The frozen version-11 test of migration 12 now stops at version 12, so it still proves migration 12 alone.
4. The fallback preparation failure text named only migration 12. It now reads "Apply migrations 12 and 13 and verify retained records".
5. After the source subject rows were deleted, `LibraryArtistDetailVm` did not read its `view` field. The field and `with_view` are deleted.
   The Library screen calls `LibraryArtistDetailVm::new`. The guard `entity_detail_pages_render_through_shell_helper_and_page_vm` now names that constructor. It checks the same rule.
6. R1-09 assumes that `src/musicbrainz.rs` reads `Track.artist_credit`. It does not. It reads the `artist-credit` names of the MusicBrainz response.
   After this packet, `api::Track.artist_credit` has no reader except its decode test and the provider observation field list. The packet keeps it.
7. ADR 0079 Decision 1 deletes "the decode of the MusicIndex artist search entity". The packet keeps `api::Artist` and `EntityDetail::Artist` for packet 004.
   This implementation follows the packet. The orchestrator must resolve the difference in ADR 0079.
8. `ApiSource` has no constructor outside `src/sources.rs`. This packet does not remove it.
9. This packet does not change these documents. The orchestrator owns them:
   - the status of ADR 0077 and ADR 0079, the ADR index, and `AGENTS.md`,
   - a row for this gate in `docs/plans/broadcast-chain-delivery-order.md`,
   - the `ArtistRef::Musicindex` reference in `docs/plans/deferred-architecture-work-index.md`.

### Guards

The guard `screens_do_not_read_artist_source_facts` is deleted. No function that it named remains.

The situational guard `adr_0079_removed_artist_storage_stays_deleted` enforces R1-07. It reads each Rust file under `src/`.
It reports each line that names a dropped table, `ArtistRef::Musicindex`, `persist_musicindex_artist` or `source_subjects`.
Its report names ADR 0079 and ADR 0077 Decision 7, and it gives the fix from this packet.

The guard does not read four schema history sections of `src/db.rs`. These are the migration registry, `VERSION_11_COLUMNS` with `ARTIST_STORAGE_TABLES`, the migration functions of versions 4, 5 and 13, and the DDL of migrations 4 and 5.
It also does not read unit test modules. A probe line in `src/sources.rs` caused the expected failure. The session then removed the probe.

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R1-01 | `adr_0077_remove_artist_storage_version_12_rows_migrate_to_13_without_artist_tables` | `src/db.rs` |
| R1-02 | `adr_0077_remove_artist_storage_migration_13_keeps_retained_rows` | `src/db.rs` |
| R1-03 | `adr_0077_remove_artist_storage_fresh_database_has_no_artist_tables` | `src/db.rs` |
| R1-04 | `adr_0077_remove_artist_storage_version_11_fixture_migrates_to_current` | `src/db/upgrades.rs` |
| R1-04 | `adr_0077_remove_artist_storage_interrupted_11_repairs_then_prepares_current` | `src/db/maintenance/restore.rs` |
| R1-05 | `adr_0077_remove_artist_storage_local_artist_name_groups_tracks_only` | `src/sources.rs` |
| R1-06 | `adr_0077_remove_artist_storage_library_artist_rows_have_no_source_subject` | `src/view_models/library.rs` |
| R1-07 | `adr_0079_removed_artist_storage_stays_deleted` | `tests/architecture_tests.rs` |
| R1-08 | `adr_0077_remove_artist_storage_index_artist_rows_keep_count_and_order` | `src/view_models/search/tests.rs` |
| R1-09 | `adr_0077_remove_artist_storage_musicbrainz_reads_artist_credit_text` | `src/musicbrainz.rs` |

R1-02 compares each row of each retained table, and each ledger row of versions 1 to 12.
The fixture holds rows in `feeds`, `tracks`, `local_files`, `playlists`, `playlist_tracks` and the ADR 0075 observation tables.

The R1-04 upgrade test also stops migration 13 at each boundary. Each stop leaves the exact version-11 schema and records.
The ADR 0066 preparation test also stops migration 13 at each boundary. Each stop gives a verified rollback to version 11.

The session also ran the R1-08 test on a copy of the source before this packet. It passed there with the same expected rows.
The existing MusicBrainz tests pass without change.

### Checks - 2026-09-24

| Check | Result |
|---|---|
| `cargo test --lib adr_0077_remove_artist_storage` | Green, 9 tests |
| `cargo test` | Green, 1679 unit tests and 272 guards |
| `cargo test --test architecture_tests` | Green, 272 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

## Operator Visual Check

This check needs a Linux desktop session, this checkout, the `sqlite3` command and a library with local tracks.
It needs no network service and no audio hardware.

The new binary upgrades the configured database to version 13 when it opens. Migration 13 deletes the artist rows.
The in-app ADR 0066 **Back up database** action runs only after that upgrade. Thus make the backup with SQLite while the app is closed.

1. Close v4vmm. Find the configured database path:

   ```bash
   cd /home/citizen/build/v4vmm
   db=$(sed -n 's/^db_path = "\(.*\)"$/\1/p' ~/.config/v4vmm/config.toml)
   echo "$db"
   ```

   An empty line is wrong. Set `db` to the `db_path` value from `~/.config/v4vmm/config.toml`.
2. Record the schema version and the artist rows:

   ```bash
   sqlite3 "$db" "SELECT max(version) FROM schema_migrations;"
   sqlite3 "$db" "SELECT count(*) FROM artist_source_facts; SELECT count(*) FROM track_artist_source_bindings;"
   ```

   Expect version 12. If the version is 13, the database has already migrated. Stop, and restore an earlier backup.
3. Make the backup and verify it:

   ```bash
   backup="$db.before-adr-0079.sqlite"
   test ! -e "$backup" && sqlite3 "$db" ".backup '$backup'"
   sqlite3 "$backup" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
   ```

   Expect `ok` and `12`. A different result is wrong. Do not continue without a verified backup.
4. Find artists that had a binding:

   ```bash
   sqlite3 "$db" "SELECT DISTINCT coalesce(t.album_artist_name, t.artist_name) FROM track_artist_source_bindings b JOIN tracks t ON t.id = b.track_id WHERE t.is_in_library = 1 LIMIT 5;"
   ```

   Write down one name. If the list is empty, use any artist in the Library.
5. Build and open the app:

   ```bash
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   ```

6. Open **Music**. In the Library artist list, click the name from step 4.
7. Look at the artist view:
   - The view shows the artist name, its albums and its tracks.
   - The detail rows are **Albums**, **Tracks** and, if a track is downloaded, **Downloaded**.
   - The view shows no **Aliases**, **Area**, **Active**, **Sort Name**, **Website** or **Source Subjects** row.
8. Open two more Library artists, and open an album from one of them.
9. Each of these results is wrong:
   - a track or an album that the artist had before the upgrade is missing,
   - an artist view is empty,
   - the app shows an error report or a database recovery page.
10. Close the app. Confirm the migration:

    ```bash
    sqlite3 "$db" "SELECT version, name FROM schema_migrations WHERE version = 13;"
    sqlite3 "$db" "SELECT name FROM sqlite_schema WHERE name LIKE '%artist_source%';"
    ls -d "$(dirname "$db")"/.v4vmm-upgrade-*
    ```

    Expect `13|drop_artist_storage`, no table name, and one app preservation directory. The app wrote that directory before the upgrade.

### Cleanup And Restore

Keep the backup until the operator accepts this check.

To undo the upgrade, close the app and restore the backup:

```bash
cp "$db" "$db.after-adr-0079.sqlite"
sqlite3 "$db" ".restore '$backup'"
sqlite3 "$db" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
```

Expect `ok` and `12`. Only a build before this packet can open that version-12 database without a new upgrade.

After acceptance, remove the files that this check made:

```bash
rm -i "$backup" "$db.after-adr-0079.sqlite"
```

The app preservation directory `.v4vmm-upgrade-*` beside the database is an ADR 0066 artifact. Keep it, or remove it with the other upgrade backups.
