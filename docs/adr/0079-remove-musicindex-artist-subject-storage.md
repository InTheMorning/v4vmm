# ADR 0079: Remove MusicIndex Artist Subject Storage

## Status

Accepted - 2026-09-24. The operator gave this decision on 2026-09-24, during the review of the ADR 0077 proposals.
Implementation has not started.

This ADR supersedes [ADR 0029](archive/0029-artist-person-identity-persistence.md).
It keeps the ADR 0029 rule for person identity as its own rule.

## Context

ADR 0029 stored MusicIndex artist subjects in three tables: `artist_source_facts`, `artist_source_links` and `artist_source_ids`.
A MusicIndex `artist_id` was the key. The rows hold a name, a sort name, an image, a website, aliases, tags, an area and active years.

Stophammer commit `a16a720` removed the artist records and the artist routes on 2026-04-08.
The rows that v4vmm stored before that date can never refresh.

On 2026-09-24, no path writes new rows:

- `persist_musicindex_artist_facts` in `src/discover/app_impl.rs` writes a row for an artist search result.
  The live `/v1/search` accepts only the `feed` and `track` types, so no artist result arrives.
- `ensure_musicindex_artist_source_fact` in `src/identity_ingest.rs` writes a row for an ADR 0045 binding. ADR 0077 supersedes that binding.

These paths still read the rows:

- `ArtistRef::Musicindex` in `src/views.rs` and `src/sources.rs`,
- the Library artist enrichment in `src/view_models/library.rs`,
- the screen guard `screens_do_not_read_artist_source_facts` in `tests/architecture_tests.rs`.

[ADR 0077](0077-publisher-feed-artist-binding.md) makes the publisher feed GUID the only artist identity.
A publisher page does not use the ADR 0029 rows.

## Decision

### 1. The Artist Subject Storage Is Deleted

One packet deletes these items together:

- the three tables, through an ADR 0016 migration,
- `ArtistRef::Musicindex` and each reader of the rows,
- `persist_musicindex_artist`, `persist_musicindex_artist_facts` and `ensure_musicindex_artist_source_fact`,
- the decode of the MusicIndex artist search entity,
- the guard `screens_do_not_read_artist_source_facts`, because no function that it names remains.

The app keeps no data that no entry point reads.

### 2. The Publisher Feed Is The Only Artist Identity

ADR 0077 owns artist identity. The app has no second artist key.

### 3. Person Identity Stays Deferred

This rule comes from ADR 0029 and stays in force.
A contributor or a `podcast:person` entry never becomes a global person identity.
Its name, role, group and position stay scoped to the source that states them.
A global person identity requires a durable person key from a source and an ADR that states a merge rule.

## Alternatives Considered

- Keep the rows as read-only data. Rejected: the values are from before 2026-04-08, they never refresh, and a page would show them without their date.
- Wait until the publisher pages exist. Rejected: the rows and the publisher pages do not interact, so the result is the same.

## Consequences

Positive:

- v4vmm has one artist identity, the publisher feed.
- Stale artist values from before 2026-04-08 no longer show.
- Three tables, one reference variant and their readers go away.

Negative:

- A Library artist view loses the old aliases, area and active years. No current source supplies these values.
- A migration deletes stored rows. The operator can make a database backup first with the ADR 0066 backup command.

## Invariants

- No artist identity comes from a MusicIndex `artist_id`.
- A contributor never becomes a global person identity.

## Acceptance Criteria

Mechanical:

- The migration removes the three tables. A migration test starts from a database that holds rows in each table.
- No code names `artist_source_facts`, `artist_source_links`, `artist_source_ids` or `ArtistRef::Musicindex`.
- The Library artist view model builds its view without the removed rows.

Visual, for the operator:

- A Library artist view opens and shows its tracks, with no aliases, area or active years.

## Relationship To Other Decisions

- Supersedes ADR 0029.
- Uses ADR 0077 for artist identity.
- Uses ADR 0016 for the migration. ADR 0066 supplies the database backup.
- ADR 0028 keeps contributor facts scoped to their owner. This ADR does not change that.
