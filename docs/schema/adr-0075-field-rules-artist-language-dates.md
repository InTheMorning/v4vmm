# ADR 0075 Field Rules: Artist Text, Language, Explicit State And Dates

> Superseded in part by [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) on 2026-09-24.
> The provider priority, freshness, expiry, stale-label and retained-discrepancy parts of this document are not in force.
> The app stores one value for each field, and the ADR 0076 playlist RSS check replaces it.
> The extraction orders, placeholder rules, date and duration rules, URL action rules, fallback sections and the readable-text comparison stay in force.

## Scope

[ADR 0075 decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts)
requires a separate written rule for each metadata field.
This document states that rule for seven fields.
The fields are artist text, album artist text, language, explicit state,
release date, publication date and duration.

This document covers free-text and scalar display fields only.
It does not merge people. It does not claim an artist identity for a track or a feed.
[ADR 0077](../adr/0077-publisher-feed-artist-binding.md) owns artist identity. It keys an artist page on a publisher feed GUID.

A field-matrix entry records OBSERVED behavior from the original inspection or a PROPOSED policy.
An OBSERVED statement cites a file, a function and the read behavior.
A PROPOSED statement needs operator acceptance before packet 020 can implement it.
Accepted refinements below supersede conflicting proposals and earlier code observations.
Other proposals retain their individual review gates.

## Accepted Track Artist Rules

Accepted on 2026-09-20: prefer fresh RSS author text before the corresponding MusicIndex track artist value.
Keep contributor names in credits. Do not substitute those names for artist text.
This display rule creates no artist identity binding.

Accepted separately: use iTunes author before RSS author.
Apply the description fields' removal, conflict, and stale-state rules. Retain source evidence and selected absence.
Feed fallback, accepted separately: show an available feed artist in a separate "Feed artist" section.
Use this section when the track has no artist text. It does not create a track artist assertion.

Packet 020 implements the accepted selection. Packet 009 preserves existing scalar behavior during its parser correction.

Track placeholders, accepted separately: hide only confirmed generated placeholders and retain literal source assertions.
The text alone cannot prove generated-placeholder status. Keep rejected values and their source evidence.

## Accepted Feed Artist Rules

Accepted on 2026-09-20: prefer fresh RSS iTunes author before the corresponding MusicIndex feed artist value.
Keep contributor and track names separate. They do not supply fallback feed artist text.
This rule covers the feed-owned album artist field without creating an artist identity binding.
Accepted separately: apply the track artist's removal, conflict, and stale-state rules.
Retain source evidence and selected absence.

Accepted separately: hide only confirmed generated placeholders. Retain literal source assertions, including "Unknown Artist".
The text alone cannot prove that a value is a placeholder. Unknown derivation cannot establish generated-placeholder status.
Keep the rejected generated value and its evidence at the source boundary.

## Evidence Base

| Source | Revision | Use |
|---|---|---|
| This repository, working tree | After ADR 0075 packets 001 and 002 | Current app behavior |
| `/home/citizen/build/stophammer` | Commit `a220f44` | Current upstream parser and API behavior |

This document records no live network request. No agent launched the app.

## Field Rule Summary

This table retains the original audit and proposals. Accepted refinements supersede proposals for their named fields.
Other field policies retain their separate review gates.

| Field | Declared owner | Sources, in priority order | Conflict result | No-source result | Feed value on a track | Evidence retained | Current code | Required change |
|---|---|---|---|---|---|---|---|---|
| Artist text | Track. A free-text claim, separate from the ADR 0077 artist binding. | RSS ingest chain: iTunes author, RSS author, contributor credit, feed artist.<br>RSS enrichment chain: iTunes author, RSS author, contributor credit. Fills only an empty value.<br>Index display: `track.track_artist` only.<br>Local display: `artist_name`, then `album_artist_name`. | OBSERVED: the first non-empty source in each chain wins. No source tag survives the chain. PROPOSED, pending operator review: keep the first non-empty source and record the discarded candidates as evidence. | No artist text shows. | Yes, through `TrackView::from_api`, not through `track_with_feed_defaults`. It shows `release_artist` when `track_artist` is empty. Upstream, `release_artist` is the feed's own text. No owner label today. PROPOSED label, pending operator review: "Feed artist". | None today. No source tag. No extraction path. No observation time. | `src/rss/subscribe.rs::subscribe_feed`, lines 188 to 198.<br>`src/rss/enrich.rs::fetch_track_enrichment_from_feed`, line 144, and `apply_track_enrichment`, line 195.<br>`src/metadata.rs::source_value_for_metadata_field`, the "Artist" arm, line 1884.<br>`src/views.rs::TrackView::from_api`, line 651.<br>`src/views.rs::TrackView::from_local_with_facts`, line 703.<br>Upstream `src/query.rs`, lines 524 and 542, and `build_track_response`, line 829. | PROPOSED, pending operator review: stop deriving artist text from a contributor credit. Add a source tag and an extraction path to the stored value. |
| Album artist text | Feed. Both the API route and the local route trace this value back to the feed today. | API display: `track.release_artist`, then `feed.release_artist`.<br>Local ingest: `feed_artist` (channel author or a person credit), else the track's own computed artist text.<br>Feed display: the first track's own artist text, a derived value. | OBSERVED: the API route and the local route can disagree, because each reads a different stored column. PROPOSED, pending operator review: the feed's own declared value takes priority. The per-track candidate is a fallback only. | No album artist text shows. | Yes, through `track_with_feed_defaults`. It copies `feed.release_artist` onto `track.release_artist` when the track value is empty. No owner label today. PROPOSED label, pending operator review: "Feed album artist". | None today. No source tag. No extraction path. | `src/api.rs::track_with_feed_defaults`, line 210.<br>`src/metadata.rs::source_value_for_metadata_field`, the "Album artist" arm, lines 1885 to 1889.<br>`src/views.rs::FeedView::from_local_with_facts`, line 585.<br>`src/rss/subscribe.rs::subscribe_feed`, line 200.<br>Upstream `src/query.rs`, lines 524, 542 and 829. | PROPOSED, pending operator review: apply the "Feed album artist" label wherever this copied value reaches a track. |
| Language | Feed. `api::Track` carries no language field today. | MusicIndex fact, key `language`, source `musicindex`, from the top-level `Feed.language` field.<br>Legacy `feeds.language` column, from the RSS `<language>` tag. No later MusicIndex fetch overwrites this column. | OBSERVED: the MusicIndex fact wins over the RSS column, with no rule that states this order as a decision. PROPOSED, pending operator review: accept the MusicIndex fact as the higher-priority source. | No language text shows. | Not applicable today. The app stores no track-level language value. See the open question below about the lost upstream track language field. | For the MusicIndex fact: source label, extraction path `$.language`, raw JSON. For the RSS column: none. | `src/api.rs::Feed`, line 136.<br>`src/api.rs::Track`, line 157, has no `language` field.<br>`src/rss/subscribe.rs::subscribe_feed`, lines 45 and 116.<br>`src/identity_ingest.rs::feed_metadata_facts_by_source`, lines 371 to 379.<br>`src/views.rs::FeedView::from_local_with_facts`, line 592. | Packet 033 must preserve the existing upstream track language field. Packet 011 must design its fact key. PROPOSED, pending operator review: apply the MusicIndex-over-RSS order to that new field too. |
| Explicit state | Feed and track are separate fields in the API data transfer object. | Local track display: MusicIndex fact, key `explicit`, source `musicindex`.<br>Parsed RSS `itunes_explicit` text, through `parse_itunes_explicit`.<br>Index route: `Track.explicit` directly, with a local fallback only when the Index value is empty. | OBSERVED: the MusicIndex fact wins over the parsed RSS text, with no rule that states this order as a decision. PROPOSED, pending operator review: keep this order and treat it as the accepted rule. | No explicit marker shows. | No. `track_with_feed_defaults` does not copy `explicit` from feed to track. | The raw iTunes tag text stays in `tracks.itunes_explicit`. The MusicIndex fact keeps a source label, extraction path `$.explicit`, and raw JSON. | `src/rss/subscribe.rs::subscribe_feed`, line 213.<br>`src/db.rs::parse_itunes_explicit`, line 1276.<br>`src/identity_ingest.rs::feed_metadata_facts_by_source`, lines 380 to 391, and `track_metadata_facts`, lines 451 to 459.<br>`src/views.rs::TrackView::from_local_with_facts`, line 709.<br>`src/application/queries/library.rs::apply_local_track_metadata_defaults`, line 440. | None. PROPOSED, pending operator review: confirm this order as the accepted rule so packet 020 can rely on it. |
| Release date | Feed. | Feed display, API route: `f.release_date` directly, no fallback.<br>Feed display, local route: `metadata_facts.release_date` only.<br>Comparison grid: a release-date claim, then `feed.release_date`, then `feed.oldest_item_at`, then the earliest track publication date. | OBSERVED: the display routes and the comparison-grid chain can select different dates, because they read different fields at different fallback depth. PROPOSED, pending operator review: adopt the comparison grid's longer chain as the accepted display order. | No date shows. | Not applicable to this row. See the Publication date row and the feed-value section below. | A release-date claim keeps its claim type, value, source, extraction path and observation time. The MusicIndex fact keeps a source label, extraction path `$.release_date`, and raw JSON. | `src/views.rs::FeedView::from_api`, line 543.<br>`src/views.rs::FeedView::from_local_with_facts`, line 606.<br>`src/metadata.rs::feed_release_pubdate`, lines 357 to 369.<br>`src/metadata.rs::musicindex_release_date`, lines 375 to 378.<br>`src/identity_ingest.rs::feed_metadata_facts_by_source`, lines 359 to 370. | PROPOSED, pending operator review: adopt one source order for both display routes. |
| Publication date | Track. | Track display, API route: `t.pub_date` directly, no fallback.<br>Track display, local route: `metadata_facts.pub_date`, then `t.pub_date`.<br>Comparison grid: a release-date claim, then `track.pub_date`, then the feed's own release-date chain when the track has no date.<br>Index route with local fallback: fills an empty Index value from the local row. | OBSERVED: the local display route already falls back from the typed fact to the raw column. The comparison grid runs a separate, longer chain that can substitute the feed's date for a missing track date. PROPOSED, pending operator review: keep the MusicIndex fact ahead of the parsed RSS column, and give a feed-substituted date its own owner label. | No date shows on the display routes. | Yes, through `musicindex_release_date`, not through `track_with_feed_defaults`. The feed's own release date can stand in for a missing track date, with no owner label today. PROPOSED label, pending operator review: "Feed release date". | A release-date claim keeps its full evidence set. The MusicIndex fact (track) keeps a source label, extraction path `$.pub_date`, and raw JSON. The raw RSS publication date text stays in the `tracks.pub_date` column before parsing. | `src/views.rs::TrackView::from_api`, line 656.<br>`src/views.rs::TrackView::from_local_with_facts`, line 708.<br>`src/metadata.rs::track_release_pubdate`, lines 352 to 355.<br>`src/metadata.rs::musicindex_release_date`, lines 375 to 378.<br>`src/application/queries/library.rs::apply_local_track_metadata_defaults`, line 437.<br>`src/rss/subscribe.rs::subscribe_feed`, line 184, and `src/db.rs::parse_local_track_pub_date`, line 1266. | PROPOSED, pending operator review, per the conflict result. OPEN QUESTION: reconcile the feed-to-track date fallback with ADR 0054's stated non-goal. See the open questions section. |
| Duration | Track. | Local library route: the raw iTunes duration tag, parsed once at ingest, stored as `tracks.duration_seconds`.<br>Index route: `api::Track.duration_secs`, decoded directly from the API response.<br>Comparison grid: `track.duration_secs` directly, at two separate call sites. | OBSERVED: no rule reconciles these routes. Duration carries no `entity_metadata_facts` fact key today, so no typed fact can outrank a raw column. | No duration shows. | No. `track_with_feed_defaults` does not copy `duration_secs`. `api::Feed` has no duration field of its own. | The raw iTunes duration tag text stays in `tracks.itunes_duration_raw`. No typed fact retains source, extraction path or observation time for duration today. | `src/rss/subscribe.rs::subscribe_feed`, lines 209 to 212.<br>`src/api.rs::Track`, line 164.<br>`src/db.rs::TrackRow`, line 34.<br>`src/local_metadata.rs::track_facts_from_rows`, lines 63 to 83, has no duration key.<br>`src/identity_ingest.rs::track_metadata_facts`, lines 422 to 462, has no duration key.<br>`src/metadata.rs`, the "Duration" arm, lines 1870 and 1920. | OPEN QUESTION for packet 011: decide whether duration needs a fact key and a stated conflict rule. This document records the open question. It does not decide the schema. |

## Artist Text: The Current Chain

`src/rss/subscribe.rs::subscribe_feed`, lines 188 to 198, computes `artist_name`
for each item in this order:

1. The iTunes item author tag.
2. The RSS item author tag.
3. A `podcast:person` credit with role `artist`, `creator`, `composer` or `performer`.
4. The feed's own artist text.

It stores the result in the local `tracks.artist_name` column with no source tag.

`src/rss/enrich.rs::fetch_track_enrichment_from_feed`, line 144, runs a close but
separate chain for the Index route. The chain is the iTunes item author, then
the item author, then the same four-role credit search. `apply_track_enrichment`,
line 195, applies that result through `set_text_if_missing`. It fills
`track.track_artist` only when the field is empty.

`src/views.rs::TrackView::from_api`, line 651, reads `track_artist`, then falls
back to `release_artist`. `TrackView::from_local_with_facts`, line 703, reads
`artist_name`, then falls back to `album_artist_name`.

`src/metadata.rs::source_value_for_metadata_field`, the `"Artist"` arm, line 1884,
reads only `track.track_artist` for the audio tag comparison grid. It has no feed
fallback.

Both the RSS ingest chain and the RSS enrichment chain can select a contributor
credit as the track's artist text. A contributor credit is an occurrence in a
source collection under ADR 0075. Using it to fill a free-text artist field
blends two different field kinds. This document records that blend as a
candidate defect against ADR 0075's ownership rules. It does not fix the blend.

## Artist Text: The ADR 0077 Boundary

[ADR 0077](../adr/0077-publisher-feed-artist-binding.md) binds an album feed to
the publisher feed that the album names. The binding uses the publisher feed GUID.
It is a separate structure from the free-text `track_artist` and `artist_name`
fields that this document rules on. The free-text fields name no identity.

This document's rule for Artist text and Album artist text creates no artist
binding. It rules on display text only.
[ADR 0077 Decision 1](../adr/0077-publisher-feed-artist-binding.md#1-a-publisher-feed-guid-identifies-an-artist-page)
forbids an artist identity from name text. This document adds no identity rule.

The superseded ADR 0045 binding in `src/identity_ingest.rs` receives no identifier
after 2026-04-08. ADR 0077 Decision 7 deletes it.

## Album Artist Text: The Current Sites

`api::Feed.release_artist` (`src/api.rs`, line 130) is the feed's own declared
artist field, with `#[serde(alias = "owner_name")]`. `api::Track.release_artist`
(`src/api.rs`, line 175) was assumed to be a separate, per-track claim.

That assumption does not hold for the inspected upstream response. Stophammer
`src/query.rs`, lines 524 and 542, both select `f.release_artist` from the
joined feed row, in the same query that reads the track. Neither query selects a
per-track `release_artist` column. `build_track_response`, line 829, copies that
selected value straight into the track response's `release_artist` field. The
API's `Track.release_artist` therefore already carries the feed's own value, not
an independent track-level claim, in the inspected upstream code.

`src/metadata.rs::source_value_for_metadata_field`, the `"Album artist"` arm,
lines 1885 to 1889, falls back from `track.release_artist` to
`feed.release_artist` for the audio tag comparison grid. Given the upstream
finding above, this fallback rarely changes the selected value today, because
the two fields already tend to hold the same text.

`src/views.rs::FeedView::from_local_with_facts`, line 585, sets the feed's
displayed artist from `tracks.first().artist`. `FeedView::from_local` reaches
this same line through delegation. This is a derived value from one track, not
a stored feed-level fact.

`src/rss/subscribe.rs::subscribe_feed`, line 200, sets
`album_artist_name = feed_artist.or_else(|| artist_name.clone())` at ingest
time. This collapses two distinct claims, the feed's artist and the track's own
computed artist text, into one untagged column before storage.

Upstream `derive_feed_artist_name` (`src/api.rs`, line 678, commit `a220f44`)
can return the literal text "Unknown Artist" when no author, slug or owner name
resolves, confirmed by its test at line 1195. `src/metadata.rs::source_text_is_placeholder`,
line 501, checks for ellipsis, repeated dots and markup entities. It does not
recognize a plain literal word string as a placeholder. This document records
this gap as an open question. It does not confirm that a live MusicIndex
response returns this literal today.

## Language

Feed language, accepted on 2026-09-20: prefer fresh RSS before MusicIndex.
Apply the accepted removal, conflict, and stale-state rules. Retain source evidence and selected absence.
Proven track language, accepted separately: prefer fresh item RSS before MusicIndex, with those same rules.

Feed fallback, accepted separately: show the available feed value as a separate "Feed language" value when track language is absent.
Legacy track language, accepted separately: retain values with unknown ownership in source details only.
Do not present those values as track-owned assertions.

Packet 033 now preserves the existing track language scalar in the app DTO.
Upstream inspection at revision `a220f44` found feed inheritance before that scalar reaches the API.
The parser copies feed language when track language is missing. The ingest path also supplies that fallback.
The API scalar does not prove an independent track language assertion.
Its ownership and absence evidence need the upstream contract described in packet 008.

Feed language has a feed-owned release claim. Track language has no corresponding claim that can recover its original owner.

The following audit records the transport and display paths before packet 033.

`api::Track` (`src/api.rs`, line 157) carries no language field today. This is a
current limit of the app's data transfer object, not an invented rule.

`src/rss/subscribe.rs::subscribe_feed`, lines 45 and 116, writes the RSS
`<language>` tag into the legacy `feeds.language` column. Unlike the feed
description, no later MusicIndex fetch overwrites this column. This repository
holds no `set_feed_language` function.

`src/identity_ingest.rs::feed_metadata_facts_by_source`, lines 371 to 379,
writes a typed `entity_metadata_facts` row for fact key `language`, source
`musicindex`, from the top-level `Feed.language` field.

`src/views.rs::FeedView::from_local_with_facts`, line 592, reads
`metadata_facts.language.or_else(|| nonempty_owned(f.language))`. This prefers
the MusicIndex fact over the RSS column, with no rule that states this order as
a decision today.

Upstream `TrackResponse` (`src/query.rs`, line 238, commit `a220f44`) does carry
a per-track `language` field, and `get_track_rows_by_guid` selects `t.language`
directly per track. The app's `api::Track` has no field to receive that value,
so the app loses it during decoding today. This document records that loss as
an open question for a transport-correction packet. It does not add the field.

## Explicit State

Feed explicit state, accepted on 2026-09-20: prefer fresh valid RSS markers and keep unknown distinct from clean.
Missing or unsupported markers cannot establish clean content. Retain raw values and their validation evidence.
Verified source absence and unknown validation remain different observation states, even when neither supplies an explicit or clean assertion.

Feed refinements, accepted separately: apply the description fields' removal, conflict, and stale-state rules.
Retain source evidence and the last selected field state, including absence. Unknown remains distinct from clean.

Proven track explicit state, accepted separately: prefer fresh valid item RSS markers.
Apply the feed explicit-state removal, conflict, unknown, and stale-state rules. Retain source evidence and selected absence.
Feed fallback, accepted separately: show a known feed state as a separate "Feed explicit state" value when track state is unknown.
This fallback does not create a track-owned assertion.

Legacy feed explicit state, accepted separately: MusicIndex `false` without evidence of a valid clean marker stays unknown.
Retain the original boolean and its source evidence. That scalar alone cannot establish clean content or verified absence.

Legacy track explicit state, accepted separately: retain MusicIndex booleans with unknown ownership in source details only.
Do not present those values as track-owned explicit-state assertions.

Upstream inspection on 2026-09-20 found three distinct inputs that can produce the same boolean at revision `a220f44`.
Feed and track builders default to `false`. A missing track marker can inherit a supplied feed marker.
The parser's `ExplicitBool` transform maps only `yes` and `true` to true.
Every other nonempty token becomes false, including malformed input.

The parser's transient `explicit_set` flag does not survive in its output.
Neither feed nor track explicit state has a release claim that restores presence, validity, or original ownership.
The API boolean preserves this computed result. It does not prove an explicit clean marker in the source.
These findings require an upstream evidence contract before a scalar alone can establish verified explicit-state coverage.
See `stophammer-parser/src/engine.rs::extract_track_fields` and `stophammer-parser/src/transform.rs::apply_transform`.

`src/rss/subscribe.rs::subscribe_feed`, line 213, stores the raw iTunes
`explicit` tag text in `tracks.itunes_explicit`.

`src/db.rs::parse_itunes_explicit`, line 1276, reads that text at query time.
It maps `explicit`, `yes` and `true` to `Some(true)`. It maps `clean`, `no` and
`false` to `Some(false)`. Any other text becomes `None`.

`src/identity_ingest.rs::feed_metadata_facts_by_source`, lines 380 to 391, and
`track_metadata_facts`, lines 451 to 459, each write a typed `entity_metadata_facts`
boolean row. The fact key is `explicit`, and the source is `musicindex` only.

`src/views.rs::TrackView::from_local_with_facts`, line 709, reads
`metadata_facts.explicit.or(t.explicit)`. The MusicIndex fact wins over the
parsed RSS text when both exist.

`src/application/queries/library.rs::apply_local_track_metadata_defaults`,
line 440, fills an Index route track's empty `explicit` value from the matching
local `TrackRow.explicit` value. It changes nothing when the Index value is
already present.

## Release Date And Publication Date

### Accepted Feed Release-Date Evidence

Accepted on 2026-09-21: feed release dates require direct release-date evidence.
Keep publication, build, and oldest-item dates separate. Those dates cannot supply a missing feed release date.
Retain derived values and their derivation evidence. A `release_date` field name or claim type alone cannot prove release-date meaning.

This rule preserves ADR 0054's boundary against mapping track publication dates to feed release dates.

Accepted separately: apply the publication-date removal, conflict, and stale-state rules to proven feed release dates.
Retain original date text, source evidence, and the last selected field state, including absence.

Source priority, accepted separately: prefer fresh supported RSS assertions, then MusicIndex assertions.
Both sources must prove an actual release date. This rule does not add support for an unrecognized RSS date field.

Precision, accepted separately: preserve year-only and year-month feed release dates with their source precision.
Do not invent a missing day or time. Keep the original date text and precision in the retained evidence.

Timestamp display, accepted separately: show feed release timestamps in UTC when the source timezone is known.
Metadata details retain the source timezone and original text. Partial calendar dates remain unchanged.

### Accepted Track Release-Date Rules

Accepted on 2026-09-21: track release dates require direct release-date evidence.
Apply the feed release-date source priority, removal, conflict, and stale-state rules.
Retain original date text, source evidence, and the last selected field state, including absence.
Keep track publication dates separate. A track publication date cannot supply a missing track release date.

Precision, accepted separately: apply the feed release-date precision rule to track release dates.
Preserve year-only and year-month precision without invented date parts. Keep the original date text and source precision.

Timestamp display, accepted separately: show track release timestamps in UTC when the source timezone is known.
Metadata details retain the source timezone and original text. Partial calendar dates remain unchanged.

Feed fallback, accepted separately: show the available feed release date as a separate "Feed release date" value.
Use this value when the track release date is absent. Keep its feed ownership and source evidence.
This presentation does not create a track-owned release-date assertion.

### Accepted Feed Publication Source Priority

Accepted on 2026-09-20: prefer valid fresh RSS channel `pubDate`, then MusicIndex claims that prove an actual channel publication date.
The `feed.pub_date` claim path alone cannot prove that assertion because the upstream parser can substitute `lastBuildDate`.
The [aggregate rules](adr-0075-field-rules-aggregates-and-relationships.md#accepted-feed-publication-source-priority) record this field separately from release dates.

Accepted separately: apply the description fields' removal, conflict, and stale-state rules.
Retain original date text, source evidence, and the last selected field state, including absence.

Precision, accepted separately: show valid partial feed publication dates at their supplied precision.
Do not invent missing date parts or timezones. Preserve original text and validation evidence.
This rule does not make malformed source syntax valid or turn a partial date into a timestamp.

Timestamp display, accepted separately: show feed publication timestamps in UTC.
Metadata details retain the source timezone and original text. Convert only a known instant.
Partial dates and unknown timezones remain unconverted.

### Accepted Track Publication Source Priority

Accepted on 2026-09-20: prefer valid fresh item RSS `pubDate`, then MusicIndex claims that prove the track's publication date.
This rule selects track-owned publication assertions. It does not substitute a feed date for a missing track date.

Accepted on 2026-09-21: apply the feed publication date's removal, conflict, and stale-state rules.
Retain original date text, source evidence, and the last selected field state, including absence.

Precision, accepted separately: show valid partial track publication dates at their supplied precision.
Do not invent missing date parts or timezones. Preserve original text and validation evidence.
Apply the feed publication rule's distinction between valid partial dates, malformed source syntax, and timestamps.

Timestamp display, accepted separately: show track publication timestamps in UTC.
Metadata details retain the source timezone and original text. Convert only a known instant.
Partial dates and unknown timezones remain unconverted.

Feed fallback, accepted separately: show the available feed date as a separate "Feed publication date" value.
Use this value when the track publication date is absent. Keep its feed ownership and source evidence.
This presentation does not create a track-owned date assertion or select a feed release date.

### Accepted Date Format Interpretation

Accepted on 2026-09-21 for feed and track publication and release dates: require unambiguous formats with known source rules.
Retain other text as unresolved evidence. Do not guess date order, epoch units, missing date parts, or timezones.
The source format must distinguish a calendar year from an epoch value. Retain the original text and interpretation evidence.
Packet 020 must document and test each supported source format before its adapter selects a date.

### Observed Date Handling

Upstream inspection at revision `a220f44` found two separate feed-date fallbacks.
`stophammer-parser/src/profile.rs` substitutes channel `lastBuildDate` when `pubDate` is absent or invalid.
`src/api.rs` later substitutes the oldest item date when the parser supplies no date.
That calculation includes ended live items with enclosures.

The release claim distinguishes `oldest_item.pub_date` from `feed.pub_date`, but the latter can conceal the build-date substitution.
It does not prove an original channel publication-date assertion. The oldest-item claim identifies no contributing item GUID.
These derivations need distinct evidence before display selection can treat them as publication or release assertions.

Track publication parsing reads item `pubDate` without a feed-date fallback.
It accepts RFC 2822, RFC 3339, and signed integers, then discards the original date text.
Missing, empty, and invalid inputs all become `None`.
The track release claim preserves the normalized value and declared track owner, but not those distinctions.

The app runs two separate rules today, one for the feed's release date and one
for the track's publication date.

`src/views.rs::FeedView::from_api`, line 543, and `FeedView::from_local_with_facts`,
line 606, read `f.release_date` or `metadata_facts.release_date` with no
fallback chain in either function. `TrackView::from_api`, line 656, reads
`t.pub_date` with no fallback. `TrackView::from_local_with_facts`, line 708,
reads `metadata_facts.pub_date.or(t.pub_date)`. This local track route already
falls back from the typed fact to the raw column.

`src/metadata.rs::feed_release_pubdate`, lines 357 to 369, runs a longer,
separate chain for the audio tag comparison grid:

1. A release-date claim.
2. `feed.release_date`.
3. `feed.oldest_item_at`.
4. The earliest track `pub_date`.

`src/metadata.rs::musicindex_release_date`, lines 375 to 378, falls back from
`track_release_pubdate` to the feed's own `feed_release_pubdate` chain when the
track supplies no date. This is a feed-to-track fallback in the same family as
the [review's](../reviews/adr-0075-metadata-contract-review.md) finding about
feed defaults appearing as track facts. No finding in that review names this
date fallback by name.

[ADR 0054's Decision](../adr/0054-local-metadata-source-fact-persistence.md#decision)
lists "No mapping from track pubdates to feed release dates" as a non-goal of
its schema slice. `musicindex_release_date` already runs a mapping in the
opposite direction: it can show the feed's release date as the track's date
when the track has none. This document records the tension as an open
question. It does not resolve it.

`src/rss/subscribe.rs::subscribe_feed`, line 184, stores the raw RSS `pubDate`
text. `src/db.rs::parse_local_track_pub_date`, line 1266, parses it with RFC
2822 at query time into `TrackRow.pub_date`.

`src/application/queries/library.rs::apply_local_track_metadata_defaults`,
line 437, fills an Index route track's empty `pub_date` value from the matching
local `TrackRow.pub_date` value.

## Duration

### Accepted Track Duration Rules

Accepted on 2026-09-21: prefer fresh valid RSS iTunes duration, then MusicIndex, for track duration metadata.
Keep measured file duration separate.

Accepted separately: apply the description fields' removal, conflict, and stale-state rules.
Retain original duration text, source evidence, and the last selected field state, including absence.

Precision, accepted separately: retain valid fractional seconds at source precision.
Do not round the stored value to whole seconds. Keep original text and precision evidence.

Validation, accepted separately: accept explicitly supplied zero as a valid duration.
Reject negative and malformed durations while retaining their source evidence. Missing values do not become zero.

RSS formats, accepted separately: accept seconds, `MM:SS`, and `HH:MM:SS`, with fractional seconds and valid component ranges.
Only the seconds component can contain a fraction. The leading component can exceed 59.
In colon formats, the seconds component must be less than 60. The middle component of `HH:MM:SS` must be less than 60.

All components must be nonnegative. Reject malformed suffixes, signs on components, and extra components without discarding their original text.

Measured duration presentation, accepted separately: when duration metadata is absent, show available measured file duration separately as "File duration".
Keep the measurement's file ownership and source evidence. This presentation does not create an RSS or MusicIndex duration assertion.

### Observed Duration Handling

At upstream revision `a220f44`, `stophammer-parser/src/duration.rs` converts item duration text without a measured-audio or feed fallback.
It truncates fractional seconds and can accept negative components or malformed fractional suffixes.
For example, `300.bad` and `300.7` follow the same integer conversion.
The API preserves the resulting integer. It has no duration release claim that recovers the discarded syntax or validation evidence.

`api::Track.duration_secs` (`src/api.rs`, line 164) and `TrackRow.duration_seconds`
(`src/db.rs`, line 34) are the only two current representations.

`src/rss/subscribe.rs::subscribe_feed`, lines 209 to 212, parses the iTunes
`duration` tag into `tracks.duration_seconds`, and stores the untouched tag text
in `tracks.itunes_duration_raw`.

`src/local_metadata.rs::track_facts_from_rows`, lines 63 to 83, and
`src/identity_ingest.rs::track_metadata_facts`, lines 422 to 462, confirm that
duration has no `entity_metadata_facts` fact key today. Neither function reads
or writes a `duration` fact key.

This document records duration's missing fact key as an open question for the
schema packet. It is not an audit-document citation. Duration carries no typed
source-fact record and no stated conflict rule today.

## Feed Values Copied Onto A Track

`src/api.rs::track_with_feed_defaults`, lines 190 to 229, copies these feed
values onto a track only when the track's own value is empty:

- `feed_url`
- `feed_guid`
- `feed_title`
- `image_url`
- `publisher_text`
- `description`
- `release_artist`
- `source_contributors`
- `source_links`
- `source_ids`
- `source_release_claims`
- `payment_routes`

Of this document's seven fields, `track_with_feed_defaults` copies only
`release_artist`, the Album artist text field. It does not copy `language`,
`explicit`, `release_date`, `pub_date`, `duration_secs` or `track_artist`.

## Proposed Owner Labels For A Feed Value On A Track

This document proposes an owner label for each feed value that
`track_with_feed_defaults` places on a track.

- Album artist text: PROPOSED label "Feed album artist", pending operator
  review. [Decision B](../adr/0075-metadata-ownership-and-completeness.md#status)
  governs identity placement and applies the same owner-label principle here.

No other field in this document has a current feed-to-track fallback inside
`track_with_feed_defaults` itself. The Artist text row and the Publication date
row each show a separate feed-to-track substitution. Each one runs through a
different function: `TrackView::from_api` and `musicindex_release_date`. Each of
those gets its own proposed label in the Field Rule Summary above.

## Open Questions

1. Artist text can come from a contributor credit through the RSS ingest chain
   and the RSS enrichment chain. This document records that as a candidate
   defect against ADR 0075's ownership rules. No packet in the current register
   is assigned to correct it. Packet 020 needs an accepted rule before it can
   apply one.

2. Upstream `derive_feed_artist_name` can return the literal text
   "Unknown Artist". The app's placeholder filter does not catch a plain
   literal word string. This document could not confirm whether a live
   MusicIndex response returns this literal today. That fact stays unverified.

3. Upstream `TrackResponse.language` supplies a track language value that the app's `api::Track` cannot receive.
   Packet 033 must preserve this existing field. Packet 011 must design its storage.
   The [field inventory](adr-0075-metadata-field-inventory.md) records this app transport loss.

4. Duration carries no `entity_metadata_facts` fact key today. This is an open
   question for packet 011.

5. `musicindex_release_date` can show a feed's release date as a track's
   publication date. This tension with ADR 0054's stated non-goal is an open
   question. No packet in the current register is assigned to resolve it.

6. Every source-priority and conflict-result rule marked PROPOSED in the Field
   Rule Summary needs operator acceptance before packet 020 can implement it.
   A proposal in this document is not an accepted rule.

## Retained Terms

The shared STE checker reports lexical findings for these technical names.

| Retained term | Meaning in this document |
|---|---|
| declared owner | The subject that a source states for a value |
| conflict result | The value the app keeps when two sources disagree |
| no-source result | The value the app shows when no source supplies a field |
| evidence retained | The raw evidence the app keeps beside a selected value |
| owner label | Text on a screen naming the subject a value belongs to |
| source order | The ranked list of sources the app reads for one field |
| provenance | Evidence of a value's owner, source, extraction path and observation time |
| fact key | The named column that groups one kind of stored metadata fact |
| binding | The ADR 0077 link from an album feed to the publisher feed that it names |
| chain | The ordered set of fallback steps the code runs for one field |
| placeholder | Source text that replaces an absent value. It does not name a real value |

Do not replace a retained term with an unrelated dictionary alternative.

## Checks

The link check and the STE check ran on this document.
Run them from the repository root:

```bash
python3 docs/runbooks/check-markdown-links.py \
  docs/tasks/adr-0075-task-006-field-rules-artist-language-dates.md \
  docs/schema/adr-0075-field-rules-artist-language-dates.md
```

```bash
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" \
  --check --no-heuristics docs/schema/adr-0075-field-rules-artist-language-dates.md
```

The operator accepted the artist policies recorded above through individual field questions.
The remaining proposed policies retain their individual review gates.
