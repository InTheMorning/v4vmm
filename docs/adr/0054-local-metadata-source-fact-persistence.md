# ADR 0054: Local Metadata Source-Fact Persistence

## Status

Proposed - 2026-05-18.

## Context

ADR 0053 reserved the persistence route for Library / Index parity fields that
cannot be made durable by query-shape changes alone. ADR 0028 already persists
owner-scoped identity facts such as links, ids, and contributors. ADR 0029
persists explicit artist subject facts. Neither ADR stores non-identity
metadata facts such as release publisher, release kind, release date, release
explicit state, track publisher, or track description as auditable local source
facts.

Some source values already exist in local entity columns:

- `feeds.language`, `feeds.description`, and `feeds.podcast_medium`
- `tracks.pub_date` and `tracks.itunes_explicit`

Those columns are useful read-model fields, but they do not preserve source,
extraction path, raw payload, or conflicts. They also cannot represent
MusicIndex facts side by side with RSS facts without collapsing provenance.

## Decision

Add a local metadata source-fact table family for feed and track owners. The
initial owner scope is deliberately limited to local feed and track ids. Artist
biography remains an artist-source-fact extension, and playlist
language/explicit state remains a product-semantics decision outside this ADR.

The first table is `entity_metadata_facts`, with owner discriminator columns
matching ADR 0028's owner checks:

- `owner_kind`: `feed` or `track`
- `feed_id` / `track_id`
- `fact_key`: source-preserving key such as `publisher_text`,
  `musicindex_release_kind`, `rss_podcast_medium`, `release_date`, `explicit`,
  `description`, `language`, or later track annotation keys
- one typed value slot: text, integer, or boolean
- `source`: required source replacement token
- optional `extraction_path`, `observed_at`, and `raw_json`
- `updated_at`

Replacement is source-scoped per owner. A refresh for
`(owner_kind, owner id, source)` replaces that source's metadata rows for the
owner in one transaction and leaves other sources untouched. This mirrors ADR
0028 identity replacement semantics while keeping metadata facts separate from
identity facts.

`podcast:medium` and MusicIndex `release_kind` are preserved as distinct fact
keys. This ADR does not define a lossy mapping between them. A later read-model
task may choose a display policy from preserved facts, but renderers must not
invent or infer one.

## Invariants

- Metadata facts are not stored in identity tables.
- Non-empty source metadata is not hidden, reinterpreted, or overwritten in
  renderers.
- Source-scoped replacement cannot delete facts from unrelated sources.
- Feed delete cascades feed metadata facts; track delete cascades track
  metadata facts.
- At most one typed value slot is populated for a fact row.
- Empty source tokens and empty fact keys are rejected.
- MusicIndex `release_kind` and RSS `podcast_medium` remain distinct until an
  explicit mapping ADR changes that.
- `src/views.rs`, `src/view_models/**`, and UI shells do not query metadata
  source-fact tables directly.

## Non-Goals

- No renderer changes in the schema/helper slice.
- No Library detail field is surfaced until a later query/VM task hydrates it
  from persisted facts.
- No artist biography persistence in this ADR's first slice.
- No playlist source-like metadata model.
- No mapping from track pubdates to feed release dates.
- No migration of existing scalar columns into source facts in the first slice.

## Alternatives Considered

### Reuse ADR 0028 Identity Tables

Rejected. Publisher, release kind, date, explicit state, description, and
annotation are not identity links, ids, or contributors. Storing them in the
identity tables would make the source-fact layer harder to audit.

### Add One Column Per Missing Field

Rejected as the source-fact contract. Scalar columns are useful projections,
but they cannot preserve source conflicts, extraction paths, or raw payloads.

### Store Metadata In `extra_json`

Rejected as the durable contract. `extra_json` can be forensic payload, but it
does not provide typed replacement, source grouping, or queryable fact keys.

### Generic Global Subject Graph

Rejected for the first implementation. Feed and track owner-scoped facts are
the current product need and match existing local ownership boundaries.

## Consequences

Positive:

- Library can later render persisted metadata parity fields without renderer
  inference.
- RSS, MusicIndex, and future source facts can coexist under one local owner.
- Existing source-fact architecture remains split by fact kind: identity,
  artist subject, and metadata.

Negative / risks:

- A later read-model task is required before users see new fields.
- Existing scalar columns still need explicit migration or ingest tasks before
  old library data gains typed metadata facts.
- Fact-key naming becomes part of the data contract and needs architecture
  guards.

## Follow-Up Work

- Task 001: add schema and DB helpers for `entity_metadata_facts`.
- Task 002: persist MusicIndex feed metadata facts during subscribe/hydration
  workflows.
- Task 003: persist MusicIndex track metadata facts during subscribe/hydration
  workflows.
- Task 004: hydrate local feed detail from preserved metadata facts.
- Task 005: hydrate local track detail from preserved metadata facts.
- Separate ADR: artist biography source facts.
- Separate product decision: playlist language / explicit semantics.

## References

- ADR 0024 - Command, query, and event application layer
- ADR 0028 - Local identity source-fact persistence
- ADR 0029 - Artist identity persistence
- ADR 0052 - Library / Index data parity triage
- ADR 0053 - Local detail source-fact parity
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
