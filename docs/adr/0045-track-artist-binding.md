# ADR 0045: Track-to-Artist Binding for Library Artist Views

## Status

Proposed - 2026-05-08.

Follows ADR 0029, which persisted explicit artist source facts but
intentionally deferred binding name-derived Library artists to stored artist
subjects.

## Context

Library artist views are currently derived from local track and feed text such
as `artist_name`, `album_artist_name`, and release artist fields. ADR 0029
added `artist_source_facts`, keyed by explicit `(source, source_artist_id)`,
and `LocalSource::fetch_artist(ArtistRef::Musicindex(...))` can hydrate an
`ArtistView` from those stored facts.

The remaining gap is that ordinary Library artist navigation still starts from
`ArtistRef::LocalArtistName`. There is no approved binding between a local
track and an explicit artist subject, so the app cannot safely enrich
name-derived Library artist views with stored source facts.

## Decision

Add an explicit, source-scoped track-to-artist binding table. A binding links a
local `tracks.id` row to one artist source subject:

- `track_id`
- `role`
- `source`
- `source_artist_id`
- `confidence`
- `provenance`
- `observed_at`

Bindings are created only when ingest has an explicit source artist id. The
first supported source is MusicIndex artist ids already persisted under ADR
0029. Name-only matches, fuzzy matching, and tag-derived inference are not
allowed.

Library name-derived artist views may use bindings only as enrichment for the
tracks that already appear in that local artist view. If multiple explicit
subjects are bound under the same display name, the read model must surface the
local track grouping conservatively and avoid collapsing subjects into one
canonical artist.

## Alternatives Considered

- Match stored artist facts to Library artist names. Rejected: this repeats
  the name-only merge problem ADR 0029 avoided.
- Add a direct nullable `tracks.artist_source_fact_id` column. Rejected: a
  track can have multiple artist roles and future source bindings.
- Create a global canonical artist registry now. Rejected: canonical merge and
  conflict policy belongs in a later ADR after explicit bindings are visible.

## Consequences

Positive:

- Library can enrich name-derived artist views when explicit source bindings
  exist.
- Source facts remain provenance-first and raw facts stay intact.
- The binding can support multiple artist roles without changing `tracks`.

Negative:

- Ingest and read-model paths must preserve conservative behavior when a local
  display name maps to multiple source subjects.
- Existing Library artist rows without explicit source ids remain unchanged.

## Invariants

- No name-only, fuzzy, filename, or tag-only binding.
- Bindings require an existing local track id and non-empty `(source,
  source_artist_id)`.
- Stored artist source facts remain source-scoped; bindings do not create a
  canonical artist identity.
- Feed or track removal deletes only local bindings for removed tracks, not the
  artist source facts.
- View-models and screens remain GPUI-free of binding inference.
- Library enrichment is read-model owned, not renderer owned.

## Non-Goals

- No global canonical artist graph.
- No global person identity persistence.
- No artist merge UI.
- No MusicBrainz artist reconciliation.
- No changes to audio tag write semantics.
- No cross-source priority policy beyond explicit per-track bindings.

## Follow-Up Work

- Person/global identity remains deferred until durable person ids and a merge
  policy exist.
- Canonical artist merge/conflict UI remains deferred until explicit bindings
  have enough local data to justify it.
