# ADR 0075 Field Rules: Titles, Numbers, Author Text And Classification

> Superseded in part by [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) on 2026-09-24.
> The provider priority, freshness, expiry, stale-label and retained-discrepancy parts of this document are not in force.
> The app stores one value for each field, and the ADR 0076 playlist RSS check replaces it.
> The extraction orders, placeholder rules, date and duration rules, URL action rules, fallback sections and the readable-text comparison stay in force.

## Status And Scope

Individual field review in progress - 2026-09-21. Feed and track title rules have individual acceptance. Other policies remain under review.
[Packet 031](../tasks/adr-0075-task-031-title-number-and-classification-rules.md) owns this document.
This document changes no application behavior.

These rules cover the remaining title, number, author, sort, medium, and release-kind rows in the
[field inventory](adr-0075-metadata-field-inventory.md).
[Packet 006](adr-0075-field-rules-artist-language-dates.md) still owns artist display selection.
This document adds the raw evidence contract for that selection.

## Accepted Boundaries

[ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) requires separate owners, providers, raw evidence, and field rules.
A feed title cannot become a track title or a stored album assertion through display fallback.
A contributor occurrence cannot become a global artist identity through a matching name.
An unknown owner remains unknown until source evidence establishes that owner.

Decision F accepts fresh RSS priority for descriptions and website/page links only.
It does not decide title, number, author, sort, medium, or release-kind priority.
Later individual decisions are recorded below. They supersede only the corresponding proposals.

Embedded tags and MusicBrainz remain separate sources.
[ADR 0004](../adr/0004-format-neutral-audio-tag-boundary.md) owns tag reading.
[ADR 0008](../adr/0008-explicit-id3v24-write-boundary.md) owns explicit tag writes.
These proposals change neither tag-write policy nor automatic candidate generation for existing tag workflows.

## Inspected Evidence

The app inspection used commit `a12521e` with packet 030 changes in the working tree.
The upstream inspection used `/home/citizen/build/stophammer` at commit `a220f44`.
No live request or app launch supplied this evidence.

| Boundary | Observed behavior |
|---|---|
| [api.rs](../../src/api.rs), `Feed`, `Track`, and `track_with_feed_defaults` | Both types retain `title` and `name`. Feed defaults copy the feed title into `Track.feed_title`. Author aliases decode into derived artist fields. |
| [views.rs](../../src/views.rs), `FeedView::from_api`, `TrackView::from_api`, and local constructors | API title selection prefers nonempty `title`, then `name`. API album text uses `feed_title`. Local album text prefers `album_title`, then `feed_title`. |
| [entity_detail.rs](../../src/view_models/entity_detail.rs), `ReleaseDetailVm::title` and `SharedTrackRowVm::title` | Missing feed titles use `Unknown Feed`. Missing track titles use the track GUID, then `Untitled`. |
| [subscribe.rs](../../src/rss/subscribe.rs), `subscribe_feed` | RSS titles enter legacy columns. The channel title also enters `tracks.album_title`. Only `podcast:episode` supplies the track number. Disc number is always `None`. |
| [enrich.rs](../../src/rss/enrich.rs), `fetch_track_enrichment_from_feed` and `apply_track_enrichment` | Direct RSS fills missing titles and numbers. Author selection combines several candidates before assignment. |
| [audio_tags.rs](../../src/audio_tags.rs), `AudioTags`, `read_mp3_tags`, and `read_lofty_tags` | The reader retains album text and track numbers. Detailed fields retain format-specific evidence, including supported ID3 frames. |
| [metadata.rs](../../src/metadata.rs), `grouped_id3_frame_labels` and `id3_sort_order_values` | Compare data keeps title, artist, and album sort frames with their corresponding tag fields. |
| [identity_ingest.rs](../../src/identity_ingest.rs), `feed_metadata_facts_by_source` | The typed `musicindex_release_kind` fact uses `$.release_kind`. Only description claims receive additional typed metadata handling here. |
| Upstream `stophammer-parser/src/profile.rs`, `feed_rules` and `track_rules` | Rules decode RSS titles. Number and season rules prefer iTunes values before Podcast Namespace fallback. |
| Upstream `src/api.rs`, `derive_feed_artist_name` and feed construction | Feed artist text can come from author text, a platform URL slug, owner text, or `Unknown Artist`. Ingest sets `release_kind` to `unknown`. |
| Upstream `src/api.rs`, track construction | Track artist text uses item author text or the derived feed artist. Both artist sort fields are initialized as absent. |
| Upstream `src/query.rs`, `FeedResponse`, `TrackResponse`, and `parse_track_row` | Sort fields are exposed. The query selects season, but the row decoder skips that column. The response has no season field. |

The API artist scalars do not prove which original author or owner assertion supplied the value.
The app aliases `owner_name` to `release_artist` and `author_name` to `track_artist`.
Those aliases preserve text but lose the original field name after serialization.
New ingestion must retain the original response before these aliases are applied.

The inspected code does not populate the local disc column from embedded disc tags.
A column named `disc_number` does not prove that the tag value reached storage.

## Common Proposed Selection Contract

The following contract applies only to the individual fields that name it below.
It is not a generic policy for all metadata.

Each candidate retains its subject, field kind, original value, provider, resource, extraction path, and available source observation time.
The app records actual fetch time separately.
Tag candidates also identify the local file observation and original tag field.
Derived values retain their input facts and derivation rule.

A text candidate is selectable when trimming its surrounding whitespace leaves text.
The original value remains unchanged in evidence.
Titles and author text do not receive HTML stripping, case folding, or placeholder guessing through this rule.
Existing boundary validation remains separate.

Selection uses the latest successful applicable observation for each provider, subject, and field.
An incomplete response or failed request retains the preceding observation and its refresh state.
Packet 018 must define freshness and request order before the shared projection uses these rules.
A stale selected value retains its stale state. These fields do not inherit Decision F's RSS freshness exception.

Conflicting candidates remain recoverable even when a priority selects one display value.
Equal candidates can share a display value without merging their evidence records.
Within one priority level, distinct values without defined source order produce an unresolved selection.
Collection iteration order and alphabetical source labels never settle that tie.
These fields do not extend Decision G's durable discrepancy lifecycle beyond descriptions and website/page links.

## Title Rules

### Accepted Feed Title Rules

Accepted on 2026-09-21: prefer fresh direct RSS channel titles over MusicIndex feed titles.
Retain both source assertions and their original evidence.

Refinements, accepted separately: apply the description fields' removal, conflict, and stale-state rules.
Retain original title text, source evidence, and the last selected state, including absence.
Failed or incomplete observations cannot establish removal. Expiry cannot restore a title removed by verified absence.

MusicIndex representation, accepted separately: use `$.title`. Retain the value and its field path.
Corrected 2026-09-26: the MusicIndex contract declares no `$.name`. ADR 0075 records the correction.

Missing-title presentation, accepted separately: keep "Unknown Feed" as a display label when no feed title is selected.
Do not store that generated label as source metadata. The underlying selected absence remains distinct from the presentation label.

Placeholders, accepted separately: exclude only confirmed generated placeholders from feed title selection.
Retain literal publisher-supplied titles such as "Unknown Feed". Preserve the generated value and its derivation evidence in source details.
Unknown provenance does not prove a generated placeholder. A renderer must not guess from the text alone.

### Accepted Track Title Rules

Accepted on 2026-09-21: apply the feed title source priority, removal, conflict, stale-state, and placeholder rules.
Prefer fresh direct item RSS titles over MusicIndex. Within MusicIndex, use `$.title`.
Retain both providers, original text, field paths, and the last selected state, including absence.
Keep the track owner. A feed title cannot become a track title.

Exclude only confirmed generated placeholders from title selection. Retain literal source titles and evidence that identifies generated values.
Missing-title presentation, accepted separately: display the track GUID, then "Untitled" when no GUID exists.
These labels do not create source metadata or replace the underlying selected absence.

### Accepted Feed-Title Reference Rules

Accepted on 2026-09-21: allow `$.feed_title` from a track response when no separately selected feed title exists.
Present that value as a labeled feed reference. Retain the feed owner identified by `feed_guid` and the original response evidence.
Keep the reference separate from the track title and embedded album assertions.

Removal, accepted separately: verified feed-title removal hides the reference while retaining its evidence.
A reference cannot restore a title removed by verified absence. Failed or incomplete observations cannot establish that removal.
Refinements, accepted separately: apply the feed title's conflict, stale-state, and generated-placeholder rules.
Retain unresolved conflicts and the last selected state, including absence. Identify retained expired values as stale.
Exclude only confirmed generated placeholders. Retain literal publisher-supplied text and the evidence that identifies generated values.

### Original Title Proposals

The accepted feed, track, and feed-title reference rules supersede the corresponding proposals below.
Unaccepted details and other fields remain proposals.

| Field | Owner and evidence | Source order | Conflict and missing result |
|---|---|---|---|
| Feed title | Feed. Preserve channel `title`, API `$.title`, and compatibility `$.name` separately. | MusicIndex `title`, direct RSS channel `title`, then MusicIndex `name`. | Keep alternatives. Select the first usable candidate. With none, return no title and typed missing state. |
| Track title | Track within its feed. Preserve item `title`, API `$.title`, and compatibility `$.name`. | MusicIndex `title`, direct RSS item `title`, then MusicIndex `name`. | Keep alternatives. Never substitute a feed title. With none, return no title and typed missing state. |
| Feed title delivered with a track | Feed identified by `feed_guid`. Preserve API `$.feed_title` as a feed reference value. | A separate feed observation, then the track response's feed reference value. | A feed observation selects its title through the feed rule above. Without either source, return no feed title. |
| Album title | Embedded tag observation for the bound local track. Preserve `TALB` or the format-specific album field. | Embedded album text in the tag source section only. MusicBrainz release text remains in its separate source section. | Different album and feed titles remain different facts. No tag album means no album assertion. |

The `name` fields are compatibility fallbacks, not proof of publisher-declared title aliases.
If `title` and `name` differ, retain both values and their API paths.
Do not populate an artist alias collection from either field.

For a missing title, retain the existing shared detail fallback text.
The feed displays `Unknown Feed`. The track displays its GUID, or `Untitled` when the GUID is absent.
These labels remain presentation values. They never become title facts.

Keep feed title and album title as separate projection fields.
The track detail can show its feed title even when an embedded album title differs.
The existing combined `Album/Feed` compare row and its tag-write behavior remain unchanged by this packet.
A later presentation packet must label separate source facts without changing their owners.

Legacy `tracks.album_title` rows from RSS remain feed-derived or unverified evidence.
Do not declare them embedded album assertions without a file observation.

## Proposed Number Rules

| Field | Owner and evidence | Source order | Conflict and missing result |
|---|---|---|---|
| Index track number | Track within its feed. Preserve API `$.track_number` and any supplied extraction evidence. | Select the MusicIndex scalar for the existing track-number projection. If absent, use the RSS episode rule below. | Keep competing episode and tag numbers separately. Without a usable value, return no track number. |
| RSS episode number | Track. Preserve direct item `itunes:episode` and `podcast:episode` as distinct source assertions. | Valid `itunes:episode`, then valid `podcast:episode`, for the integer projection. | Keep both when they disagree. The fallback records that an episode assertion supplied the displayed number. |
| Disc number | Bound local track's tag observation. Preserve `TPOS` or the format-specific disc field. | A validated embedded disc value only. No RSS or Index field inspected here supplies disc number. | Conflicting tag values remain unresolved. Missing disc stays absent. Never assume disc 1. |
| Season | Track within its feed. Preserve direct item `itunes:season` and `podcast:season`. | Valid `itunes:season`, then valid `podcast:season`. | Keep conflicts. Missing season stays absent. Season never supplies disc number. |

For the existing integer projections, a valid value is a positive integer within the target type's range.
Reject truncation, wrapping, and rounding during projection.
Retain zero, negative, fractional, overflowing, and malformed source values with their validation state.
This definition limits an integer projection. It does not claim that every source syntax requires integers.

For tag text such as `2/4`, preserve both the number and total with the original string.
The current track-number projection uses the validated number component only.
An absent total stays absent. Neither a feed item count nor array position supplies a missing total.
A total smaller than the number makes that pair invalid for projection.

An Index scalar with no extraction path remains an Index track-number assertion.
Do not invent an iTunes or Podcast Namespace extraction path for it.
Do not reorder an existing playlist when a number changes.
Library sorting remains with its existing owner until a separate packet changes that behavior.

Season requires direct RSS retention before display can use this rule.
An upstream season response would need a separate API change and transport packet.
The current omission cannot establish that the publisher supplied no season.

## Proposed Raw Author And Owner Rules

| Field | Owner and evidence | Source order | Conflict and missing result |
|---|---|---|---|
| Channel author text | Feed assertion. Preserve each direct author element with its namespace and path. | Separate source entries. Use the latest applicable observation for each exact path. | Do not combine distinct paths. Missing author text remains absent. |
| Item author text | Track assertion. Preserve direct item `itunes:author` and RSS `author` separately. | Separate source entries. Use the latest applicable observation for each exact path. | Keep different values. No feed-author substitution into item evidence. Missing item author remains absent. |
| Channel owner text | Feed assertion at `itunes:owner/itunes:name`. It is not verified artist identity. | Direct owner text, or an upstream claim with the same explicit source path. | Keep provider alternatives. Without that evidence, return no raw owner value. |
| API artist text | Declared API field owner. Preserve `$.release_artist`, `$.track_artist`, and any original alias key. | Packet 006 records the accepted display artist order. This rule adds no author-to-artist priority. | Mark scalar provenance as API-level when derivation is unavailable. Do not reconstruct missing raw author or owner text. |

Raw author and owner facts identify what a source said about its publication object.
They do not establish a person, contributor occurrence, publisher relationship, or artist binding.
Owner text alone cannot supply a verified publisher relationship.
[ADR 0077](../adr/0077-publisher-feed-artist-binding.md) owns artist binding. It uses the publisher feed GUID and never name text.

Packet 006's accepted artist selection remains separate: select declared artist text before any labeled feed fallback.
Do not derive new artist identities from owner text, contributor names, or platform URL slugs in v4vmm.
Unknown upstream derivation remains unknown. Retaining its scalar does not certify that derivation.

## Proposed Artist Sort Rules

Feed `release_artist_sort` belongs to the feed's artist-text assertion.
Track `track_artist_sort` belongs to the track's artist-text assertion.
Neither value proves a global artist identity or supplies a missing display name.

Preserve each API sort value with its exact field path and provider observation.
Select the explicit sort text paired with the selected artist assertion from the same provider and subject.
If that observation supplies no sort text, use the displayed artist text as a derived sort key only.
Do not borrow another provider's sort text or a feed sort value for a track artist.
If the selected display value is a labeled feed fallback, its sort key also retains the feed owner.

Conflicting sort values stay attached to their respective artist assertions.
If no artist text is selected, return no artist sort key.
Embedded `TSOP` remains in the tag source section. It does not overwrite an API sort field.
Packet 033 preserves the track scalar. Packet 011 must retain the relationship between sort text and its source observation.

## Proposed Medium And Release Kind Rules

| Field | Owner and evidence | Source order | Conflict and missing result |
|---|---|---|---|
| Raw medium | Feed. Preserve direct `podcast:medium` and API `$.raw_medium`. | MusicIndex `raw_medium`, then direct RSS channel `podcast:medium`. | Retain alternatives and unknown tokens. With none, return no medium. Never infer it from filenames or enclosures. |
| Release kind | Feed. Preserve API `$.release_kind` and explicitly typed release-kind claims. | A documented explicit release-kind claim, then the API scalar. Unsupported claim syntax remains evidence only. | Distinct claims without source order remain unresolved. Missing and literal `unknown` mean unknown classification, with different retained evidence. |

Raw medium and release kind remain separate fields even when their strings match.
The inspected ingest assigns `unknown` to release kind. That value does not prove a publisher classification.
Keep unknown API tokens visible as source evidence. Do not silently map them to a known release kind.
Do not infer an album, single, or EP from item count, medium, duration, or feed title.

No inspected release-claim syntax establishes an additional selectable release kind today.
Packet 011 must preserve such claims before a later contract can validate and select them.

## Constructed Regression Cases

These cases describe required mechanical tests for later code packets.
They do not claim that the application currently passes those tests.

| Case | Inputs | Required result |
|---|---|---|
| T31-01 | Fresh Index title `Night Harbor`, fresh direct RSS title `Harbor`, Index name `Archive` | Select `Harbor` under the accepted RSS priority. Retain all three values with their distinct paths. |
| T31-02 | Missing Index title, RSS title `Harbor`, Index name `Archive` | Select `Harbor`. A nonempty compatibility name does not override the direct title. |
| T31-03 | Track title absent, feed title `Channel` | No track title. Feed title remains available under its feed owner. |
| T31-04 | Feed title `Channel`, embedded `TALB=Album`, legacy album column `Channel` | Preserve feed and embedded album separately. Do not certify the legacy value as embedded evidence. |
| T31-05 | Index track number 7, RSS iTunes episode 8, Podcast Namespace episode 9, embedded `TRCK=3/12` | Select Index number 7 for the existing number field. Retain three other assertions and the embedded total. |
| T31-06 | Missing Index number, RSS iTunes episode 8, Podcast Namespace episode 9 | Select 8 with its iTunes path. Retain 9 as a conflicting source value. |
| T31-07 | RSS season 2, no disc evidence | Season is 2. Disc remains absent. |
| T31-08 | Number text `3.5`, `-1`, or `999999999999999999999`. Tag pair `5/3`. | Retain every raw value. Return validation failures without truncation or invented numbers. |
| T31-09 | Channel owner `Host Company`, item author `Performer`, Index artist `Other Performer` | Retain three distinct assertions. No person identity or publisher relationship follows from the strings. |
| T31-10 | Selected Index artist `The Group`, paired sort `Group, The`, RSS author `Different Group` | Use `Group, The` for the selected artist. Do not attach it to the RSS author. |
| T31-11 | Selected RSS artist text, Index sort text present | Derive the sort key from the selected RSS text. Retain the unmatched Index sort evidence separately. |
| T31-12 | Raw medium `music`, release kind `unknown`, one returned item | Preserve medium and unknown kind. Do not infer single or album. |
| T31-13 | A later Index request fails after an earlier title observation | Keep the earlier title and failed refresh state. Do not record an empty title observation. |
| T31-14 | Equal-priority release-kind claims disagree and provide no order | Return an unresolved classification. Keep both claims and their evidence. |
| T31-15 | No separately selected feed title, track response supplies `feed_title=Channel` and its feed owner | Offer `Channel` as a labeled feed reference. Preserve response evidence without creating a track title or embedded album assertion. |
| T31-16 | Verified feed-title removal, retained track response supplies `feed_title=Channel` | Hide the reference. Retain the removal and the earlier reference evidence. |
| T31-17 | The last selected feed-title reference expires | Retain its selected state with stale metadata. A retained absence cannot restore an earlier value. |
| T31-18 | Literal publisher reference `Unknown Feed` and a separately confirmed generated placeholder | Retain the literal reference. Exclude only the confirmed placeholder and retain its derivation evidence. |

## Operator Decisions Required

1. Review Index track-number priority and RSS episode fallback separately from the accepted title rules.
2. Review separate embedded-album presentation.
3. Accept iTunes-first RSS number and season selection, with validated integers for existing numeric projections.
4. Accept artist sort text only with its matching artist assertion and source observation.
5. Accept Index-first raw medium selection and explicit classification without inferred release kinds.

Raw evidence retention, owner separation, and unchanged tag-write boundaries already follow accepted ADR 0075 rules.
The decisions above do not reopen those invariants.
Packet 006's artist field policies have individual acceptance. These remaining decisions do not reopen them.

## Operator Visual Check

The operator paused visual checks. This document requires no app launch and closes no visual gate.
Later presentation packets must describe checks for separate feed, album, author, and number evidence.
