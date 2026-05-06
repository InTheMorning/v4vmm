# ADR 0029: Artist Identity Persistence (Person Deferred)

## Status

Accepted - 2026-05-01. Tasks 001-005 complete. ADR runtime scope closed;
follow-up ADRs remain for name-derived artist binding and person identity.

## Context

ADR 0026 made artist, release, track, and contributor projections GPUI-free.
ADR 0028 persisted feed, track, and contributor identity source facts locally.
The remaining identity gap is durable artist identity. Person identity remains
owner-scoped under ADR 0028 in this ADR; a future ADR may address it once a
source provides a durable person key and a merge policy is defined.

Today, Library artist views are mostly derived from local track/feed text such
as `artist_name`, `album_artist_name`, and release artist fields. Discover can
receive richer MusicIndex artist records with images, aliases, area, dates,
URLs, source links, and source ids. Contributors can also carry person-like
facts such as website, image, and Nostr identity. Once remote facts become local
Library data, the app needs a provenance-preserving way to persist those facts
without pretending that every matching display name is the same person.

The ideal architecture in `docs/architecture/architecture-diagrams.md` keeps
GPUI thin over source-preserving data, application queries, and pure view
models. Artist identity therefore belongs below the UI boundary as local source
facts and read-model projections, not as ad hoc screen inference.

## Decision

Persist explicit artist source facts locally as a new source-scoped table
family. Do not introduce a global canonical artist or person registry, and do
not persist person identity in this ADR.

Artist source facts are keyed by `(source, source_artist_id)`, where
`source_artist_id` is an explicit MusicIndex `artist_id` or an equivalent
future source-provided artist key. Each source row persists:

- display name and sort name
- image URL
- website URL
- aliases
- tags
- area
- active years (`begin_year`, `end_year`)
- source links and source ids when the source exposes them
- raw source payload for provenance

Contributor/person facts remain owner-scoped under ADR 0028 unless both a
durable source-provided person key exists and a future ADR defines a merge
policy. MusicIndex contributors and RSS `podcast:person` rows currently provide
useful person-like fields such as `href`, `img`, and `npub`, but not a durable
person id. Their display name, role, group, and source-order position must not
be promoted into a global person identity.

### Storage shape

Artist source facts live in dedicated tables, not in ADR 0028's
`entity_identity_*` tables. ADR 0028 owners are local feeds, tracks, and
contributor positions; artist subjects have no such local owner and are keyed
by `(source, source_artist_id)` instead. Task 002 implemented:

- `artist_source_facts` — one row per `(source, source_artist_id)` carrying
  scalar display fields (name, sort_name, image_url, website_url, area,
  begin_year, end_year), `aliases_json` and `tags_json` JSON-array columns for
  multi-valued fields, `observed_at`, and `raw_json` for full provenance.
- `artist_source_links` — child rows for source links, foreign-keyed to the
  parent fact id.
- `artist_source_ids` — child rows for source ids, foreign-keyed to the parent
  fact id.

Aliases and tags are stored as JSON arrays on the parent row rather than as
relational child tables, because they are display-only and never queried by
value. Source links and source ids stay relational because consumers iterate
them and surface them as identity facts.

### Lifecycle

Artist source facts are upserted by `(source, source_artist_id)`. Each upsert
fully replaces the child `artist_source_links` and `artist_source_ids` rows
for that subject in one transaction; scalar fields and JSON arrays are
overwritten. Artist subjects do not cascade from feeds or tracks: unsubscribing
a feed does not delete artist subjects that other tracks or releases may still
reference. A future ADR may introduce a curation surface to purge unused artist
subjects.

### Track-to-artist binding

This ADR does **not** add a column linking local `tracks` rows to a stored
artist subject. Phase 4 Library hydration is therefore scoped to local read
paths that already have an explicit `source_artist_id`, such as a future local
lookup by `ArtistRef::Musicindex`. A name-derived
`ArtistRef::LocalArtistName` artist continues to render only the facts
available from local tracks today.

Adding a `(source, source_artist_id)` reference on `tracks` is left to a
follow-up ADR so that ingest, conflict, and unbinding policy can be designed
once. Until then, this ADR's value to Library is limited to artist views that
already hold an explicit source id; that limitation is intentional.

### Conflict resolution

When two source rows disagree on a scalar display fact for the same artist
(for example, the same MusicIndex artist re-fetched with a different image),
each `(source, source_artist_id)` row keeps its own values; nothing is merged
across keys. When a single key is upserted, the new row replaces the old:
last-observed wins per source key. Cross-source display priority is not defined
in this ADR because there is no cross-source artist binding yet. A future
binding ADR must define source priority, tie-breaking, and conflict surfacing
before any read model combines multiple source rows into one logical artist.
Raw source rows must remain readable; any future selection is purely for
display.

## Invariants

- No fuzzy matching, name-only merging, or filename/tag inference.
- Raw source facts must survive display convenience fields.
- `src/views.rs` and `src/view_models/*` remain database-free and GPUI-free.
- Screens must not reconstruct artist identity or owner-scoped person facts
  from ad hoc JSON.
- Application queries or local read-model helpers own hydration into view
  facts.
- Every schema table introduced for this ADR must define source-scoped
  replacement semantics and cascade behavior.
- Contributor identity remains source-order scoped unless both a durable
  source-provided person key exists and a future ADR defines a merge policy.
- MusicIndex `artist_id` is a durable source key; local artist display names are
  not.
- Artist source facts are not deleted by feed or track unsubscription.
- Local `tracks` rows do not gain a foreign reference to artist subjects in
  this ADR.

## Non-Goals

- No global canonical artist graph in this ADR.
- No global canonical person graph in this ADR.
- No automatic merge UI.
- No MusicBrainz artist reconciliation unless a later task defines the contract.
- No changes to audio tag write semantics.
- No Discover API changes unless the inventory proves required include fields
  are missing.
- No track-to-artist-subject binding column on `tracks`; deferred to a
  follow-up ADR.

## Consequences

- Library artist views opened through a local read path with an explicit
  `ArtistRef::Musicindex` can render scalar artist facts (image, website,
  aliases, area, active years) offline once Tasks 003-004 land. Source links
  and source ids become offline-available only after the MusicIndex API exposes
  them on artist detail; today's `api::Artist` does not.
- Library artist views opened with `ArtistRef::LocalArtistName` (name-derived)
  do not benefit from this ADR until a follow-up ADR binds local tracks to
  artist subjects.
- The first schema is narrower than a generic subject graph but avoids forcing
  explicit artist records and owner-scoped contributor rows into one ambiguous
  table.
- A later canonical-identity ADR can build on preserved source facts if product
  behavior needs merging, conflict resolution, or operator curation.
- Person-level global persistence remains deferred until a source provides
  durable person keys and a later ADR defines merge and conflict policy.

### Done when

For an artist with a stored `(source, source_artist_id)` row, a local
`ArtistView` lookup by explicit source id renders, from local data only:

- `image_url`
- `website_url`
- `aliases`
- `area`
- `begin_year` / `end_year`

Library `ArtistView` for a name-derived `ArtistRef::LocalArtistName` continues
to behave as it does today; this ADR does not regress that path.

Completed by Tasks 003-004 and verified by Task 005 on 2026-05-01.

## Alternatives Considered

### Name-Based Artist Table

Rejected. It would collapse unrelated artists or contributors that share a
display name and would violate the provenance-first rule.

### Global Canonical Person Registry Now

Rejected for the first implementation. Canonical identity requires merge rules,
conflict policy, and probably operator review. ADR 0029 preserves facts first
so that a registry can be added later without data loss.

### Keep Artist Identity Remote-Only

Rejected as the long-term architecture. It preserves current behavior but keeps
Library artist views poorer than Discover and blocks offline identity parity.

## Follow-Up Work

- Task 001 inventoried source artist/person fields and recommended split schema
  tracks.
- Task 002 added artist source-fact schema and DB helpers for explicit artist
  subjects only.
- Task 003 persists explicit MusicIndex artist records into the artist
  source-fact tables.
- Task 004 hydrates local `ArtistView` from explicit source facts for
  `ArtistRef::Musicindex`.
- Task 005 verified the full gate and closed ADR 0029's runtime scope.
- A follow-up ADR must define track-to-artist-subject binding before Library
  can hydrate artist views for name-derived artists.
- Runtime person/global-identity implementation must wait for durable person
  keys and a merge policy.
