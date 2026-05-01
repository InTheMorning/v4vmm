# ADR 0028: Local Identity Source Fact Persistence

## Status

Implemented - 2026-05-01.

## Context

ADR 0026 added GPUI-free identity facts and shared entity projections for
MusicIndex artists, feeds/items/tracks, and contributors. ADR 0027 made
equivalent Library and Discover actions render from shared action-state inputs.

The post-ADR 0026 identity audit found that the projection layer can represent
facts the local Library cannot durably preserve:

- contributor `href`, `img`, and `npub`
- raw `source_links`
- raw `source_ids`
- contributor identity provenance from MusicIndex and RSS source rows

Today, remote Discover detail can render richer identity data than Library
detail because API rows still carry source facts in memory. Local Library rows
mostly rebuild identity from scalar feed/track fields, `people_json`,
`podcast_value_json`, `item_value_json`, and `extra_json`. Those fields are
useful, but they do not provide a normalized local source-fact read model for
ADR 0026 projections.

The ideal architecture in `docs/architecture/architecture-diagrams.md` keeps
GPUI thin over application queries, source-preserving view facts, and shared
view models. This ADR defines the local persistence contract needed for
Library to hydrate the same identity facts as Discover without moving MusicIndex
API structs, database access, or provenance inference into shared projections.

## Decision

Persist identity source facts in local SQLite as first-class local read-model
data for feed, track, and contributor owners.

Add normalized local source-fact storage for three fact families:

1. identity links, compatible with `views::IdentityLinkFact`
2. identity ids, compatible with `views::IdentityIdFact`
3. contributor identity rows, compatible with `views::ContributorView`

The source-fact persistence layer stores facts under a local owner, not as a
new canonical global entity graph. Initial owners are:

- `feed`, identified by `feeds.id`
- `track`, identified by `tracks.id`
- `feed_contributor`, identified by a feed plus contributor position/source row
- `track_contributor`, identified by a track plus contributor position/source row

Contributor position is not a durable person identity. It is the source-order
index inside one owner/source snapshot. Contributor rows and any associated
contributor-scoped facts are replaced as a set for `(owner, source)` during a
refresh. This avoids pretending that `(name, role)` is stable or unique while
also avoiding silent merges when a source reorders its contributor list.

Artist-level local identity remains derived from feed and track facts until a
separate artist persistence ADR creates a durable local artist table. This ADR
does not introduce fuzzy artist reconciliation, canonical MusicBrainz artist
matching, or a global person registry.

The tables must preserve raw provenance fields where available:

- `entity_type`
- `entity_id`
- `position`
- `link_type`
- `url`
- `scheme`
- `value`
- `source`
- `extraction_path`
- `observed_at`
- raw source JSON for rows that cannot be fully represented by scalar columns

Convenience fields such as `EntityIdentityLinks::website_url`,
`EntityIdentityLinks::nostr_npub`, and `ContributorView::image_url` are derived
from persisted source facts when hydrating local views. They must not replace
or delete raw facts.

### Ownership and flow

The intended flow is:

```text
MusicIndex / RSS / tag context
  -> application command or ingest workflow
  -> local source-fact persistence
  -> application query / DB read model
  -> views::{FeedView, TrackView, ContributorView}
  -> shared projections
  -> GPUI screen adapter
```

`src/views.rs` remains GPUI-free and database-free. It may expose constructors
that accept already-loaded local identity facts, but it must not query SQLite.

`src/view_models/entity_detail.rs` remains a pure projection layer. It must not
know whether a fact came from MusicIndex, RSS, audio tags, or SQLite.

`src/application/queries/*` and existing DB/service read paths own hydration
of local source facts into local view inputs. Screens may consume hydrated view
models, but they must not manually reconstruct source facts from ad hoc JSON.

### Schema shape

The implementation should use explicit SQLite tables rather than extending
`extra_json` for this contract. The exact DDL belongs in Task 001, but the
shape should be:

```text
entity_identity_links
  id
  owner_kind              -- feed | track | feed_contributor | track_contributor
  feed_id
  track_id
  contributor_position
  entity_type
  entity_id
  position
  link_type
  url
  source                  -- required replacement token
  extraction_path
  observed_at
  raw_json
  updated_at

entity_identity_ids
  id
  owner_kind
  feed_id
  track_id
  contributor_position
  entity_type
  entity_id
  position
  scheme
  value
  source                  -- required replacement token
  extraction_path
  observed_at
  raw_json
  updated_at

entity_contributors
  id
  owner_kind              -- feed | track
  feed_id
  track_id
  position
  name
  role
  group_name
  href
  image_url
  nostr_npub
  source                  -- required replacement token
  raw_json
  observed_at
  updated_at
```

The schema must be resilient to repeated feed refreshes and MusicIndex
rehydration. Replacement is source-scoped: a refresh replaces rows for one
`(owner_kind, owner id, source)` in one transaction. A feed or track may
therefore retain RSS and MusicIndex facts side by side. Unknown sources must be
stored under an explicit source token such as `unknown`, not with ambiguous
replacement semantics. A refresh must not delete facts from unrelated sources.

The implementation should use single discriminator tables rather than separate
per-owner tables. That keeps query and replacement helpers uniform for
feed/track owners and avoids creating a broad set of near-identical tables such
as `feed_identity_links`, `track_identity_links`, and contributor variants.
Because the discriminator shape has real integrity risk, Task 001 must add
SQLite `CHECK` constraints that tie `owner_kind` to the allowed nullable owner
columns. For example, `feed` rows require `feed_id IS NOT NULL`,
`track_id IS NULL`, and `contributor_position IS NULL`; `track_contributor`
rows require `track_id IS NOT NULL` and `contributor_position IS NOT NULL`.

Every source-fact table must cascade with the local owner. Feed-owned rows use
`feeds(id) ON DELETE CASCADE`; track-owned rows use
`tracks(id) ON DELETE CASCADE`. Contributor-scoped rows cascade through the
same feed or track owner and are also deleted by source-scoped contributor-list
replacement.

Contributor scalar columns such as `href`, `image_url`, and `nostr_npub` are
stored source-row fields, not a substitute for raw provenance. If a source also
provides generic contributor `source_links` or `source_ids`, the ingest task
must write both the contributor scalar row and the corresponding raw link/id
rows in the same transaction. Loaders may use the scalar columns to hydrate
`ContributorView`, but raw source facts must remain persisted for audit and
future projection work.

`raw_json` is forensic source payload. Writers should store it whenever the
original source row is available. It may be null only for facts constructed from
typed local values where no source row exists. Read-model queries trust typed
scalar columns for current display; they do not reconcile display fields from
`raw_json` at read time.

### Ingest rules

MusicIndex API rows should persist:

- feed and track `source_links`
- feed and track `source_ids`
- feed and track `source_contributors`
- contributor `href`, `img`, and `npub`

RSS ingestion should persist contributor/source facts that are already captured
in `people_json` where they can be mapped without inference. If an RSS person
row lacks a field, the row remains absent rather than guessed from names or
publisher text.

ID3/audio tag writes remain a presentation/export concern for selected
metadata. They are not the durable source-fact store and should not be treated
as a complete provenance backup.

### Query rules

Local feed and track queries should return enough identity source facts for
`FeedView::from_local` and `TrackView::from_local` replacements to populate:

- `EntityIdentityLinks::source_links`
- `EntityIdentityLinks::source_ids`
- convenience `website_url`, `nostr_npub`, and `image_url`
- contributor rows with `href`, `image_url`, and `nostr_npub`

The measurable local hydration contract after Task 003 is:

- local feed detail can render persisted `website_url` and `nostr_npub`
  identity actions when those facts were fetched or ingested
- local feed detail can render contributor `href`, `image_url`, and
  `nostr_npub` when those contributor facts were fetched or ingested
- local track detail can expose persisted `source_links` and `source_ids` to
  metadata/provenance projections without reparsing screen-local JSON
- raw source-link and source-id vectors survive a local reload even when
  convenience fields are also populated

Library and Discover should then differ only because a source fact is genuinely
unavailable locally, not because Library dropped already-known facts during
persistence.

## Consequences

- Library can render identity affordances while offline when facts were
  previously fetched or ingested.
- Source facts become inspectable local data instead of transient API context.
- The database schema grows, but the shared projection layer stays stable and
  GPUI-free.
- Feed refresh and subscription workflows need transactional source-fact
  replacement semantics.
- Artist-level Library identity remains incomplete until a later artist
  persistence ADR adds a durable local artist identity model.
- Tests must cover preservation and non-inference, not only display labels.

## Invariants

- Do not infer identity from names, titles, filenames, publisher text, or fuzzy
  matching.
- Do not expose concrete `api::*` identity row types from shared view facts.
- Do not make `src/views.rs` or `src/view_models/entity_detail.rs` query
  SQLite, call services, import GPUI, or import screen modules.
- Do not collapse multiple source facts into one guessed canonical value.
- Do not use ID3/audio tags as the local provenance store.
- Unsupported artist/person reconciliation remains explicit future work.

## Non-Goals

- No global artist table.
- No global contributor/person table.
- No fuzzy identity reconciliation.
- No MusicBrainz artist/person merge logic.
- No non-URL artwork resolver changes.
- No visual redesign beyond surfacing facts through existing ADR 0026/0027
  projections.

## Alternatives Considered

### Keep facts only in existing JSON columns

Rejected. `extra_json`, `people_json`, and value JSON are source-specific and
not enough for a stable local read model. They also push every consumer toward
ad hoc parsing.

### Store only convenience fields

Rejected. Storing only `website_url`, `nostr_npub`, or image URL would discard
provenance and conflict information, violating ADR 0026's source-preserving
identity model.

### Introduce global artist and contributor entities now

Rejected for this ADR. The proven gap is local preservation of known facts, not
global identity reconciliation. A global entity graph would require separate
matching rules and conflict policy.

### Use separate per-owner tables

Rejected for the first implementation. Separate tables would make SQLite
foreign keys more obvious but would multiply the schema and helper surface for
the same read-model behavior. The discriminator-table approach is acceptable
only with explicit `CHECK` constraints, source-scoped replacement helpers, and
tests that cover invalid owner shapes. If those constraints become awkward in
Task 001, the task should stop and revise this ADR before implementation.

## Follow-Up Work

- Task 001: schema and DB read/write helpers.
- Task 002: MusicIndex and RSS ingest persistence.
- Task 003: local view hydration and projection tests.
- Task 004: Library/Discover identity visual smoke.
- Task 005: cleanup and architecture gates.
- Post-ADR 0028 Task 001 surfaced already-hydrated local contributor facts in
  Library release detail through the shared contributor panel.
- A later ADR may introduce global artist/person identity if product behavior
  requires reconciliation across feeds.
