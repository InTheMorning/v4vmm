# ADR 0029 Task 001 Source Inventory Review

## Result

Pass with split schema direction - 2026-05-01.

## Scope

- Reviewed `../musicindex/api.json`.
- Audited `src/api.rs`, `src/views.rs`, `src/sources.rs`,
  `src/local_identity.rs`, `src/db.rs`, `src/identity_ingest.rs`,
  `src/rss/subscribe.rs`, and `src/view_models/artist.rs`.
- Compared MusicIndex artist records, MusicIndex contributor rows, RSS
  `podcast:person` rows, local Library artist derivation, and `ArtistView`.

## Preservation Matrix

| Fact | MusicIndex artist | MusicIndex contributor | RSS person | Local Library artist rows | `ArtistView` |
|---|---|---|---|---|---|
| Explicit durable key | `artist_id` exists and maps to `ArtistRef::Musicindex`. | No contributor/person id in `api::Contributor`; only owner row position during ingest. | No durable id; local persistence uses owner/source position. | `ArtistRef::LocalArtistName` is display-name derived. | Supports `ArtistRef::Musicindex` and `ArtistRef::LocalArtistName`. |
| Display name | `name`, `sort_name`. | `name`. | Text value from `podcast:person`. | Derived from `artist_name`, `album_artist_name`, or release artist text. | Supports `name` and `sort_name`. |
| Image | `image_url`. | `img`, persisted to `entity_contributors.image_url` by ADR 0028. | `img`, persisted to `entity_contributors.image_url`. | First album image from matching tracks. | Supports `image_url`, `artwork`, and identity image. |
| Website | `url`; `ArtistView::from_api` also maps it to `identity.website_url`. | `href`, persisted to `entity_contributors.href`. | `href`, persisted to `entity_contributors.href`. | Not persisted as artist identity; `ArtistView::from_local_rows` returns `url: None`. | Supports `url` and `identity.website_url`. |
| Nostr | Not modeled as a direct artist field in `api::Artist`; possible only if future source ids expose it. | `npub`, persisted to `entity_contributors.nostr_npub`. | `npub`, persisted to `entity_contributors.nostr_npub`. | Not persisted as artist identity. | Supports `identity.nostr_npub` through `EntityIdentityLinks`. |
| Aliases | `aliases`. | None. | None. | Lost. | Supports `aliases`. |
| Area | `area`. | None. | None. | Lost. | Supports `area`. |
| Active years | `begin_year`, `end_year`. | None. | None. | Lost. | Supports `begin_year`, `end_year`; `ArtistVm` renders `Active`. |
| Source links | No `api::Artist.source_links` field currently exists. | No generic link rows beyond scalar `href`. | No generic link rows beyond scalar `href`. | None. | Supports `EntityIdentityLinks::source_links`, but `ArtistView::from_api` does not receive artist source links today. |
| Source ids | No `api::Artist.source_ids` field currently exists. | No generic id rows beyond scalar `npub`. | No generic id rows beyond scalar `npub`. | None. | Supports `EntityIdentityLinks::source_ids`, but local/API artist paths do not hydrate them today. |
| Raw provenance | API struct has typed fields, no raw JSON in views. | ADR 0028 stores contributor raw JSON during MusicIndex ingest. | ADR 0028 stores `podcast:person` raw JSON. | Existing feed/track tables retain some raw RSS JSON, but artist rows are not normalized. | Does not carry raw JSON; projections consume typed facts. |

## Local Flow Findings

- `ApiSource::fetch_artist` fetches MusicIndex artist detail and maps it with
  `ArtistView::from_api`.
- `LocalSource::fetch_artist` filters local library tracks by exact
  `album_artist_name` or `artist_name`, then calls
  `ArtistView::from_local_rows`.
- `ArtistView::from_local_rows` produces counts and first album image only. It
  does not hydrate website, Nostr, aliases, area, active years, source links,
  or source ids.
- ADR 0028 source-fact tables cover feed, track, feed-contributor, and
  track-contributor owners. They do not model durable artist subjects.
- RSS `podcast:person` and MusicIndex contributor rows are already persisted as
  owner-scoped contributor facts. Their row position is source-order, not a
  durable person identity.

## Durable Keys vs Display Names

Explicit durable keys currently available:

- MusicIndex `Artist.artist_id`.
- Feed and track ids already covered by ADR 0028.
- Nostr public keys for contributors/RSS persons, when present, are durable ids
  for an identity claim but not proof that two display names are the same person
  without an explicit merge policy.

Display-only or weak keys:

- `Artist.name`, `sort_name`.
- `Contributor.name`, `role`, `group_name`.
- RSS `podcast:person` text, `role`, `group`.
- Local `artist_name` and `album_artist_name`.

## Recommended Schema Direction

Use separate schema tracks for artists and persons in the first runtime phase.

1. Add artist source-fact storage for explicit artist subjects keyed by
   `(source, source_artist_id)` where `source_artist_id` is MusicIndex
   `artist_id` or another future explicit artist id.
2. Keep contributor/RSS person facts owner-scoped under ADR 0028 until a source
   provides an explicit durable person id. Do not promote contributor position,
   display name, or role into global person identity.
3. If person persistence is needed before durable person ids exist, model it as
   owner-scoped person facts linked to the existing contributor owner/position,
   not as global people.
4. Reuse `EntityIdentityLinks`/`IdentityLinkFact`/`IdentityIdFact` for hydrated
   view facts, but keep raw source JSON in DB-layer tables only.

This split avoids forcing MusicIndex artist records and RSS/MusicIndex
contributor rows into one polymorphic subject table when their identity keys
have different strength.

## Risks For Task 002

- A single `source_subjects` table would need nullable key columns and
  application-level rules to prevent name-only person merging.
- Artist facts may need source links/ids before the MusicIndex API exposes
  `source_links` or `source_ids` on artist detail.
- Local artist views can improve only for artists with explicit stored source
  subjects; name-derived Library artists must remain conservative.
- Nostr-based grouping should be treated as a displayed shared id until a later
  ADR defines merge/canonicalization rules.

## Suggested Task 002 Scope

- Revise ADR 0029 to commit to split artist/person schema direction.
- Add an additive artist-source-fact schema and DB helpers for explicit artist
  subjects only.
- Add tests proving local name-only artists do not receive source facts unless
  an explicit source artist id is available.
- Defer global person persistence unless a durable source person key appears.

## Verification

Documentation-only task.

Green on 2026-05-01:

- `cargo fmt -- --check`
- `cargo check`
