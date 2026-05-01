# ADR 0029: Artist And Person Identity Persistence

## Status

Accepted - 2026-05-01. Tasks 001-002 implemented; remaining phases pending.

## Context

ADR 0026 made artist, release, track, and contributor projections GPUI-free.
ADR 0028 persisted feed, track, and contributor identity source facts locally.
The remaining identity gap is durable artist/person identity.

Today, Library artist views are mostly derived from local track/feed text such
as `artist_name`, `album_artist_name`, and release artist fields. Discover can
receive richer MusicIndex artist records with images, aliases, area, dates,
URLs, source links, and source ids. Contributors can also carry person-like
facts such as website, image, and Nostr identity. Once remote facts become local
Library data, the app needs a provenance-preserving way to persist those facts
without pretending that every matching display name is the same person.

The ideal architecture in `docs/architecture/architecture-diagrams.md` keeps
GPUI thin over source-preserving data, application queries, and pure view
models. Artist/person identity therefore belongs below the UI boundary as local
source facts and read-model projections, not as ad hoc screen inference.

## Decision

Introduce artist/person identity persistence as split source-scoped fact
families. Do not introduce a global canonical artist/person registry in the
first implementation.

The first accepted runtime implementation should persist explicit artist source
facts keyed by `(source, source_artist_id)`, where the source id is an explicit
MusicIndex `artist_id` or an equivalent future source-provided artist key.
These artist facts may include:

- display name and sort name
- image URL
- website URL
- aliases
- area
- active years
- source links and source ids when the source exposes them
- raw source payload for provenance

Contributor/person facts remain owner-scoped under ADR 0028 unless a source
provides an explicit durable person key. MusicIndex contributors and RSS
`podcast:person` rows currently provide useful person-like fields such as
`href`, `img`, and `npub`, but not a durable person id. Their display name,
role, group, and source-order position must not be promoted into a global
person identity.

Local Library artist views may hydrate richer display facts from these
source-scoped records, but they must surface conflicts rather than silently
choosing a canonical identity. If two sources disagree about image, website,
Nostr, aliases, or area, the read model may pick a deterministic display value
for convenience while retaining every raw source fact for inspection.

## Invariants

- No fuzzy matching, name-only merging, or filename/tag inference.
- Raw source facts must survive display convenience fields.
- `src/views.rs` and `src/view_models/*` remain database-free and GPUI-free.
- Screens must not reconstruct artist/person identity from ad hoc JSON.
- Application queries or local read-model helpers own hydration into view
  facts.
- Every schema table introduced for this ADR must define source-scoped
  replacement semantics and cascade behavior.
- Contributor identity remains source-order scoped unless a source provides an
  explicit durable person key.
- MusicIndex `artist_id` is a durable source key; local artist display names are
  not.

## Non-Goals

- No global canonical artist/person graph in the first implementation.
- No automatic merge UI.
- No MusicBrainz artist reconciliation unless a later task defines the contract.
- No changes to audio tag write semantics.
- No Discover API changes unless the inventory proves required include fields
  are missing.

## Consequences

- Library artist views can eventually render offline identity facts that
  Discover already knows, without making the UI depend on MusicIndex API rows.
- The first schema can be narrower than a generic subject graph, but it avoids
  forcing explicit artist records and owner-scoped contributor rows into one
  ambiguous table.
- A later canonical-identity ADR can build on preserved source facts if product
  behavior needs merging, conflict resolution, or operator curation.
- Person-level global persistence remains deferred until a source provides
  durable person keys or a later ADR defines merge and conflict policy.

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
- Task 003 should persist explicit MusicIndex artist records into the artist
  source-fact tables.
- Runtime person/global-identity implementation must wait for durable person
  keys and a merge policy.
