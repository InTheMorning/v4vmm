# ADR 0075 Field Rules For Description, Artwork And Publisher

## Scope

[ADR 0075, decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts)
states the rule for display fallback. The app must store a source observation
before it computes a value for display. A missing track field does not turn a
feed description, a feed image, or a feed publisher into a track assertion.

This document states the field rule for five fields. The fields are feed
description, track description, feed artwork, track artwork, and publisher.
Each rule names the declared owner, the source order, the conflict result, and
the value the app shows when no source supplies the field.

This document separates current source behavior from proposed field policy. A
statement marked "Current behavior" cites a file, a function, and a line. A
statement marked "Proposed" needs operator review before packet 020 may
implement it. [Decision C](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts)
already accepts the artwork fallback rule. This document restates that
accepted rule. It does not propose a new artwork rule.

This document changes no code, no schema, and no renderer.

## Evidence

| Source | Revision | Use |
|---|---|---|
| This repository, working tree | Commit `d3c6ee4` | Current app behavior |
| `/home/citizen/build/stophammer` | Commit `a220f44` | Current upstream query and API behavior |
| [Packet 002 corpus](adr-0075-metadata-example-corpus.md) | Cases C01 and C18a through C18c | Constructed and supplied examples of feed-to-track inheritance |

This document records no live network request and no app launch.

## Feed Description

Two functions write the legacy `feeds.description` column in one subscribe
flow.

- `src/rss/subscribe.rs::subscribe_feed`, lines 79 to 125, first writes the RSS
  channel `<description>` value into that column. Line 47 reads the channel
  text.
- The same function later runs a MusicIndex baseline fetch, lines 144 to 159.
  When that fetch succeeds, line 154 checks the MusicIndex `Feed.description`
  value. Line 155 calls `src/db.rs::set_feed_description` and overwrites the
  column with that value.
- `set_feed_description`, lines 464 to 475, runs a plain `UPDATE` statement
  with no source column.

Current behavior: this legacy column mixes an RSS value and a MusicIndex value
under one untagged name.

`src/identity_ingest.rs::feed_metadata_facts_by_source`, lines 337 to 420,
writes typed facts for feed description into `entity_metadata_facts`. Lines
402 to 417 scan `feed.source_release_claims` for each claim with `claim_type`
`description`. Line 409 assigns each claim its own source token through
`source_token`, lines 510 to 516. An empty or missing claim source becomes the
token `musicindex`. Lines 392 to 400 also write one row for the top-level
`Feed.description` field. That row always carries the source token
`musicindex` and the extraction path `$.description`.

Current behavior: one feed can hold several description facts under different
source tokens.

`src/local_metadata.rs::feed_facts_from_rows`, lines 26 to 61, selects the
description that the app displays. Line 48 finds each row where the source is
`musicindex` and the extraction path is `$.description`. It holds that row
apart as `top_level_description`, line 51. Line 53 keeps the first other
description row it meets, in query order. Line 59 uses `top_level_description`
only when no other row supplied a value.

`src/db.rs::local_metadata_facts`, lines 1685 to 1712, orders rows by `source
COLLATE NOCASE, fact_key COLLATE NOCASE, id`, line 1698.

Current behavior: this alphabetical source order, not a stated priority,
selects the description when two non-top-level sources both supply one. The
[review](../reviews/adr-0075-metadata-contract-review.md#source-order-can-select-the-displayed-identity)
names the same alphabetical order risk for `local_identity_links`. See
[Proposed Field Policies](#proposed-field-policies-pending-operator-review)
for the source-priority proposal this finding requires.

## Track Description

`src/api.rs::track_with_feed_defaults`, lines 190 to 230, copies
`feed.description` into `track.description`, lines 207 to 209, only when the
track's own field is `None`. Current behavior: this function can place a feed
description inside a track's API data object, with no marker that the value
came from the feed. The
[review](../reviews/adr-0075-metadata-contract-review.md#feed-defaults-can-appear-as-track-facts)
names this same risk.

Two functions build a display-only `TrackContext` with this fallback already
applied.

- `src/feed_service.rs::merge_track_context_from_detail`, lines 81 to 106,
  calls `track_with_feed_defaults` at line 92, for a track fetched from the
  MusicIndex API.
- `src/feed_service.rs::track_row_to_track_context`, lines 344 to 356, calls
  the same function at line 346, for a track read from the local database.

Both functions pass the merged track on to further code with no feed-owner
label attached.

`src/ui/shells/library/track_detail.rs::render_library_track_window`, line 66,
reads `track_context.track.description`. It uses that value directly for the
Library track detail panel. Current behavior: a local track can have no
persisted track description fact. This panel then shows the feed's description
as the track's own description, with no owner label.

`src/feed_service.rs::track_row_to_track_context_with_local_identity`, lines
358 to 371, calls `hydrate_track_metadata`, lines 419 to 434, after the
fallback above already ran. That function replaces `track.description` only
when a stored track fact exists, line 424. When no stored track fact exists,
the feed-defaulted value from the earlier step stays in place.

Three production call sites reach
`src/identity_ingest.rs::persist_track_metadata_facts`, lines 300 to 308. Each
one uses a track value that never passed through `track_with_feed_defaults`.

- `src/subscribe_service.rs::subscribe_feed_retaining`, lines 224 to 229,
  persists `track_for_persistence` before line 231 calls
  `track_with_feed_defaults`.
- `src/subscribe_service.rs::subscribe_track_from_search_internal`, lines 449
  to 454, persists `track_for_persistence`. That value comes from
  `authoritative_track_for_persistence`, lines 718 to 739. The feed-default
  fallback never reaches that function, even though line 416 calls
  `track_with_feed_defaults` earlier in the same enclosing function.
- `src/feed_service.rs::apply_feed_updates`, lines 313 to 315, persists the
  raw `fetched_track` value. Line 307 passes a clone of that same value to
  `merge_track_context_from_detail`. The fallback runs on the clone only.

Current behavior: at these three sites, the typed `musicindex` track
description fact in `entity_metadata_facts` reflects the track's own API
value. This document resolved every production call site it found for this
question. It excludes two call sites inside `#[cfg(test)]` modules at
`src/identity_ingest.rs`, lines 800 and 939.

The feed-default value can also reach the audio tag comparison grid and the
written audio tag.

- `src/subscribe_service.rs::subscribe_feed_retaining`, line 231, calls
  `track_with_feed_defaults` before line 233 calls
  `enrich_track_context_from_rss`.
- `src/rss/enrich.rs::apply_track_enrichment`, line 194, calls
  `set_text_if_missing` for `track.description`. That helper fills the field
  only when it is still `None`.
- Current behavior: when the feed default already filled `track.description`,
  this RSS enrichment step cannot supply the track's own RSS item description
  instead.
- `src/feed_service.rs::apply_feed_updates`, line 317, calls
  `id3_edits_for_track_context` with the fallback-merged context. A written
  ID3 description tag can therefore also carry the feed's description text.
- `src/metadata.rs::source_value_for_metadata_field`, lines 1924 to 1926,
  computes the tag-comparison grid value for `Description` with its own
  separate fallback to `feed.description`. This fallback runs independent of
  the two fallbacks named above.

See [Proposed Field Policies](#proposed-field-policies-pending-operator-review)
for the placement and label proposal this section requires.

## Feed Artwork

`feeds.album_image_href` has one writer. `src/rss/subscribe.rs`, lines 58 to
70, reads the RSS channel `podcast:image` attribute, then the iTunes channel
image, then the RSS `<image><url>` element, in that order. Lines 79 to 125
write the selected value. Current behavior: no later step overwrites this
column. It has no MusicIndex-baseline second writer, unlike the description
column above.

`src/identity_ingest.rs::feed_metadata_facts_by_source`, lines 337 to 420,
writes no artwork fact key. This document read the full function body.
Current behavior: the feed's artwork carries no source tag and no extraction
path in `entity_metadata_facts`.

`src/views.rs::FeedView::from_local_with_facts`, line 604, and `from_api`,
line 541, read the stored scalar value through `artwork_from_url`, lines 353
to 355.

## Track Artwork

Three sites in this repository compute the feed-to-track artwork fallback.

- `src/metadata.rs::artwork_url`, lines 301 to 306, falls back from
  `track.image_url` to `feed.image_url`, for a `TrackContext` built at any
  layer.
- `src/api.rs::track_with_feed_defaults`, lines 201 to 203, computes the same
  fallback again, on its own, inside the API data object, before any caller
  runs `artwork_url`.
- `src/views.rs::TrackView::from_local_with_facts`, line 690, reads
  `t.track_image_href.or(t.album_image_href)` from two separate local
  database columns.

This third site runs the fallback in the Rust view projection. It is not a
SQL-level fallback. `src/db.rs::library_tracks_for_feed` and its sibling
queries, lines 480 to 483, select `t.track_image_href` and `f.album_image_href`
as two separate columns. The `.or()` expression at `src/views.rs:690` chooses
between the two after the query already ran. An earlier accuracy review of
this packet's own instructions required this correction.

A fourth, upstream site computes a true SQL-level artwork fallback, before the
app decodes the API response.

- `/home/citizen/build/stophammer/src/query.rs::get_track_rows_by_guid` and
  `get_track_row_for_feed`, lines 524 and 542, select `COALESCE(t.image_url,
  f.image_url)`.
- `build_track_response`, line 818, copies that selected value into the API
  `Track.image_url` field with no change.

Current behavior: the API response can already carry the feed's image inside
`Track.image_url`, before `track_with_feed_defaults` or `artwork_url` ever
run. The app cannot tell, from the response alone, which owner supplied that
value. This document does not infer an owner from a matching URL. This
evidence gap needs a Stophammer decision. See
[Open Questions](#open-questions-for-later-packets).

[Decision C](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts)
is accepted. Show the track's own artwork when it has one. Otherwise, show the
feed's artwork in the track header. This fallback needs no additional visible
owner label in the track header. The stored artwork facts keep their source
and their owner, where such evidence exists.

This document does not propose a new artwork rule.
It restates the accepted rule so the Field Rule Table can cite it.

## Publisher

Three sites in this repository handle the feed-to-track publisher fallback.

- `src/api.rs::track_with_feed_defaults`, lines 204 to 206, copies
  `feed.publisher_text` into `track.publisher_text`, only when the track's own
  field is `None`.
- `src/metadata.rs::source_value_for_metadata_field`, the `Publisher` and
  `Label` match arm, lines 1903 to 1906, applies the fallback a second time.
  It reads `track.publisher_text`, then `feed.publisher_text`, for the audio
  tag comparison grid.
- `src/views.rs::TrackView::from_local_with_facts`, line 717, reads
  `publisher_text` only from `metadata_facts.publisher_text`. It uses no
  legacy column and no feed fallback at that layer.

Current behavior: the local route's displayed publisher value never inherits
a feed value directly. The API route's `Track.publisher_text` field, and the
audio tag comparison grid, can each independently show an inherited feed
value.

`src/identity_ingest.rs::track_metadata_facts`, lines 422 to 462, writes a
`publisher_text` fact from `track.publisher_text`, lines 426 to 433. This is
the same function that writes the `description` fact discussed above. The
three production call sites resolved for track description, in the Track
Description section, apply to this fact by the same evidence. Current
behavior: the typed `musicindex` track publisher fact reflects the track's own
API value at those three sites.

See [Proposed Field Policies](#proposed-field-policies-pending-operator-review)
for the placement and label proposal this section requires.

## Generic Merge Helper Risk

`src/api.rs::track_with_feed_defaults` applies one pattern to many fields.
Lines 192 to 227 all repeat the same shape: `if track.field.is_none() {
track.field = feed.field.clone(); }`. The function applies this one pattern to
`feed_url`, `feed_guid`, `feed_title`, `image_url`, `publisher_text`,
`description`, `release_artist`, `source_contributors`, `source_links`,
`source_ids`, `source_release_claims`, and `payment_routes`.

This document's rule governs `image_url`, `publisher_text`, and `description`
only. A later packet must not reuse this one function, or copy its pattern, as
if one fallback rule fit every field it touches. Each field keeps its own
rule, its own conflict result, and its own owner label, under
[ADR 0075, decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts).

`src/application/commands/payment_routes.rs::fetch_musicindex_payment_routes`,
line 478, also calls `track_with_feed_defaults`, for payment routes. The
current contract still governs payment-route inheritance. This document does
not change that contract or trace that call site further.

## Field Rule Template

| Field | Declared owner | Sources, in priority order | Conflict result | No-source result | Feed value on a track | Evidence retained | Current code | Required change |
|---|---|---|---|---|---|---|---|---|
| Feed description | Feed | Accepted: fresh direct RSS before MusicIndex. Proposed details appear below | Keep both observations and detect readable-text differences. Within-provider ties remain unresolved | No description | Only in a proposed feed-owned section | Original values, provider resources, paths, source times, fetch times, discrepancy state | RSS writer, typed facts, and local selector traced above | Packets 011, 018, 020, 035, and 036 |
| Track description | Track | Accepted: fresh direct item RSS before the matching Index track value. Proposed details appear below | Keep source alternatives. Never compare feed fallback as a track assertion | No track description | Proposed separate "Feed description" section | Both original values and retained discrepancy evidence | The Track Description section traces display fallback and raw persistence separately | Packet 020 applies accepted display rules. Existing tag-write policy stays separate |
| Feed artwork | Feed | Current RSS extraction: podcast image, iTunes image, RSS image. Cross-provider priority remains proposed work | Preserve separate artwork assertions | No artwork | Accepted Decision C fallback in the track header | Separate facts must retain the source and declared owner | The Feed Artwork section traces the scalar column | Packet 008 requests upstream artwork facts. Packet 011 designs their storage |
| Track artwork | Track | Accepted: track artwork, then feed artwork | Same URLs do not prove the same owner | No artwork | Accepted, with no additional visible owner label | Separate track and feed facts, including absent coverage | App projections and upstream COALESCE traced above | Preserve the old API display field and add separate upstream artwork facts |
| Publisher | Separate feed and track assertions | Current: own value, then unlabeled feed fallback. Proposed: keep own value and disclose feed publisher separately | Keep distinct owners. Placement proposal remains open | No publisher text | Proposed "Feed publisher" section | Source, owner, path, and observation time | Publisher and Generic Merge Helper Risk sections above | Packet 020 needs accepted placement. No tag-write policy change |

## Proposed Field Policies Pending Operator Review

These proposals need operator acceptance before packet 020 may implement
them. [ADR 0075, decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts)
requires this review for every field policy that an existing decision does
not settle. [Decision B](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts)
selects separate, labeled sections for identities. It does not select the
placement or the label for an inherited description or an inherited
publisher.

1. **Selection from one provider.** Use the direct channel or item `description` for the corresponding RSS owner.
   In one Index observation, prefer description claims in supplied position order, then the top-level description.
   A claim retains an extraction path, which supports this proposed priority.
   Conflicting claims with missing or tied positions remain unresolved alternatives. Do not select by source label.

2. **Inherited track description placement and label.** Keep the track's own description empty when it has no assertion.
   Show an available feed description in a separate section labeled "Feed description".
   This preserves the owner and makes the related description available.

3. **Inherited publisher placement and label.** Keep the track's own publisher empty when it has no assertion.
   Show an available feed publisher in a separate section labeled "Feed publisher".
   This preserves the owner without treating the feed value as a track assertion.

A proposal above does not become an accepted field rule until the operator's
field-rule review accepts it. Packet 020 stays held for a field until its
policy is accepted.

## Accepted Description Priority And Discrepancies

The operator accepted ADR 0075 Decisions F and G on 2026-09-19.
For the same owner, prefer a fresh direct RSS description over the corresponding MusicIndex description.
This rule applies separately to feeds and tracks. It does not authorize inherited description placement.

Keep both source observations before preparing display text.
Compare their readable text, not their raw HTML representation.
Equivalent markup and whitespace alone do not create a discrepancy. Preserve both original strings even when their readable text agrees.

An active discrepancy retains the owner, field, provider resources, extraction paths, source times, and actual fetch times.
Repeated observations update the same record. Matching comparable observations resolve it without deleting the evidence.
An unavailable provider or an omitted field cannot resolve the discrepancy.

Packet 035 must specify and test normalization, comparison identity, and discrepancy transitions before implementation.
Its examples must include equivalent HTML, entity decoding, block boundaries, whitespace, and changed readable text.
Packet 018 must define freshness. Cached RSS is not automatically fresh.
Packets 011 and 036 must preserve evidence through restart and later snapshot replacement.

A possible MusicIndex update hook remains deferred. No current packet sends an update request.

Proposed fallback when no fresh RSS description is available: use a current Index description for the same owner.
If neither provider has a current usable value, retain the last selected value with its stale or failed-refresh state.
When no source has a value, show no description.
These fallback details and the within-provider proposal above still need operator acceptance.
They do not change audio-tag comparison or tag-write policy.

## Open Questions For Later Packets

- **Upstream artwork ownership.** The API's coalesced `Track.image_url` value
  carries no owner marker. This document cannot resolve which owner supplied
  a given track's image from the response alone. Packet 008 now requests separate artwork facts with declared owners and source evidence.
  The existing scalar display field remains for compatibility.
- **Typed artwork facts.** Neither feed artwork nor track artwork has a typed
  source fact today. Packet 011 must define their fact keys and durable owner evidence.
- **Legacy description column source tag.** `feeds.description` has two
  writers and no source column. Packet 011 must decide whether to add a
  source tag to that column, or retire it once the typed facts fully cover
  feed description.
- **Feed description source priority.** See proposal 1 above. This document
  records accepted provider priority and a concrete within-provider proposal.
  The within-provider and stale-value details still need operator review.
- **Inherited description and publisher placement.** See proposals 2 and 3
  above. Packets 022 and 023 need an accepted placement and label before they
  build the labeled sections.

## Retained Terms

The shared STE checker reports lexical findings for these technical names.

| Retained term | Meaning in this document |
|---|---|
| declared owner | The subject that a field rule states as the owner of a value |
| source token | The string that labels which source wrote a fact row |
| fact key | The string that names which field a fact row records |
| extraction path | The recorded location that a value came from in its source |
| typed fact | One row in `entity_metadata_facts`, with a source and a fact key |
| owner label | Visible text that names the owner of a displayed value |
| unlabeled | Shown with no owner label attached |
| coalesced | Combined by the SQL `COALESCE` function into one selected value |
| provenance | Evidence of a value's owner, source, extraction path, and time |

Do not replace a retained term with an unrelated dictionary alternative.

## Checks

The [packet report](../tasks/adr-0075-task-005-field-rules-description-artwork-publisher.md)
records the link check and the language check results for this document.
