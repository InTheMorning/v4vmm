# ADR 0077 Task 001: Remove Dead Artist Storage

Status: Ready - 2026-09-24. Implementation has not started.
This packet changes one user-visible result: a Library artist view no longer shows stored aliases, area or active years.
Its visual gate opens when the implementation is complete.

## Goal

Delete the ADR 0045 track artist binding and the ADR 0029 artist subject storage in one change.
Add schema version 13, which drops the four tables.

No current source writes these tables. Stophammer removed `artist_credit` and the artist routes on 2026-04-08.
The stored rows can never refresh.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decision 7 deletes the ADR 0045 binding.
- [ADR 0079](../adr/0079-remove-musicindex-artist-subject-storage.md) Decision 1 deletes the artist subject storage.
- [ADR 0016](../adr/0016-schema-migration-discipline.md) owns the migration registry.
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

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
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

## Operator Visual Check

The implementer writes this section at completion. It must contain these items:

1. The command that makes a database backup before the first run, and the command that restores it.
2. The steps that open a Library artist view for an artist with local tracks.
3. What to look at: the view shows its tracks and albums. It shows no aliases, area or active years.
4. What is wrong: a missing track, an empty view, or an error report.
5. The cleanup that restores the backup, if the operator used a fixture database.
