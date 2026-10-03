# ADR 0075 Field Rules For Links And Media

> Superseded in part by [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) on 2026-09-24.
> The provider priority, freshness, expiry, stale-label and retained-discrepancy parts of this document are not in force.
> The app stores one value for each field, and the ADR 0076 playlist RSS check replaces it.
> The extraction orders, placeholder rules, date and duration rules, URL action rules, fallback sections and the readable-text comparison stay in force.

## Scope

This document states the field rule for six link and media fields.
The fields are feed `website`, track `web_page`, transcript links, enclosure
links, image links and unknown link kinds.
[ADR 0075, decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules)
names the owner and the source order for each field.
[ADR 0075, decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts)
requires a fallback rule that does not turn a display value into a stored fact.

This document separates current source behavior from proposed field policy.
A statement marked "Current behavior" describes code that this document cites by
file, function and line. A statement marked "Proposed" needs operator review
before a dependent code packet may implement it. This document changes no code.

## Evidence

| Source | Revision | Use |
|---|---|---|
| This repository, working tree | Commit `d3c6ee4` | Current app behavior |
| `/home/citizen/build/stophammer` | Commit `a220f44` | Current upstream parser behavior |
| `rss` crate | Version 2.0.12, pinned by `Cargo.lock` | Current XML extension behavior |
| [Packet 002 corpus](adr-0075-metadata-example-corpus.md) | Cases C07, C08a, C08b | Website and page-link constructed examples |
| [Packet 003 contract](adr-0075-identity-syntax-contract.md) | Supported `podcast:txt` purpose values | Identity syntax boundary |

`cargo tree --offline -e features -i rss` reports the enabled `rss` crate
features: `atom_syndication`, `builders`, `default`, `derive_builder` and
`never`. This command was re-run for this document. The `atom` feature is not
enabled. This document records no live network request and no app launch.

## Field Rule Template

| Field | Declared owner | Sources, in priority order | Conflict result | No-source result | Feed value on a track | Evidence retained | Current code | Required change |
|---|---|---|---|---|---|---|---|---|
| Feed website | Feed | Accepted: fresh direct RSS before MusicIndex. Proposed extraction details appear below | Retain both URLs and discrepancy evidence | No website action | Feed-owned section only | Original URLs, provider resources, paths, source and fetch times | `rss_feed_link_inputs`, `website_from_links`, `website_url_from_links` | Packet 020 applies accepted priority. Packet 035 defines comparison |
| Track web_page | Track | Accepted provider priority. Proposed: item RSS link, then supported Atom alternate link | Keep distinct owners and conflicting candidates | No page action | A feed website cannot become a track page | Owner, provider, kind, path, position, times | RSS stores `tracks.link` only. Local website selector misses `web_page` | Packet 010 stores the item page fact. Packet 020 shares selection |
| Transcript links | Track | Upstream `source_transcripts`, direct RSS transcripts, and legacy transcript links | Source and transcript selection remain proposed rules | No transcript action | No inherited track assertion | Owner, position, URL, MIME type, language, relation, source, path, observation time | Existing legacy paths traced below. Upstream typed collection is lost | Packet 032 transport. Packets 004 and 011 coverage and storage |
| Enclosure links | Track | Accepted: supported primary, supported alternate, supported direct scalar | Apply the selecting operation's capability check at each step | No action for that operation | No feed enclosure inference | Upstream owner, position, and observation time must join the existing enclosure fields | `select_audio_enclosure`, `play_url`, scalar `TrackView` | Packet 030 transport. Packet 020 applies Decision E with named capability owners |
| Image links | Feed, track, or contributor occurrence | Existing scalars and proposed upstream artwork facts | Keep owner evidence even when URLs match | No image from that source | Decision C governs artwork fallback | Source, extraction path, owner, observation | Scalars today. Contributor claim evidence also identifies its source occurrence | Packet 008 requests artwork facts. Packet 011 designs storage |
| Unknown link kinds | Declared subject | Retained source rows | Preserve evidence without inventing an action | No action | No inferred owner | Original row and support state | Known selectors ignore most unknown kinds | Packet 011 records support state. Packet 014 retains raw evidence |

## Feed Website

### Write Side

`src/rss/subscribe.rs::rss_feed_link_inputs`, line 415, builds one identity
link input from the RSS channel `link` element. It sets `link_type` to
`website`, `entity_type` to `feed`, `position` to `0`, and `extraction_path`
to `channel/link`. `src/rss/subscribe.rs::persist_rss_feed_identity`, line
346, calls `replace_local_identity_links` with the source label `rss`.

Upstream `stophammer-parser/src/engine.rs::extract_links`, line 826, matches a
non-namespaced `link` element under a feed node and returns `link_type`
`website` with `extraction_path` `feed.link` (line 835). This produces the
`website` entry in a MusicIndex `Feed.source_links` collection.

The app writes `channel/link`. Upstream writes `feed.link`. Both values name
the same source element. This document records both extraction path strings
with their separate sources. Neither string is invented.

### Read Side

`src/metadata.rs::website_from_links`, line 308, and
`src/views.rs::website_url_from_links`, line 343, both read a `SourceEntityLink`
row with link type `website` for a feed. This matches ADR 0075 decision 3's
owner split for the feed side. The channel `link` value stays a feed fact
through both the write path and the read path.

## Track web_page

### Two Risks From The Review

The [review](../reviews/adr-0075-metadata-contract-review.md#website-rules-differ)
records two risks for this field. This section restates them with exact
citations and one correction.

**Risk one: the raw item link becomes a column value, not an identity fact.**
`src/rss/subscribe.rs`, line 183, reads `item.link()` into the local variable
`item_link`. The track upsert statement, with its `ON CONFLICT(feed_id,
item_guid)` clause at line 258, writes `item_link` to the `tracks.link`
column (see the parameter list at lines 278 to 299).

The review's text wrongly names `rss_track_link_inputs` as the site of this
write. `rss_track_link_inputs`, line 435, does not read `item.link` at all.
It builds only a transcript link input from the
`podcast:transcript@url` element. The site of the `tracks.link` write is the
track upsert statement inside `subscribe_feed`, at line 233, not
`rss_track_link_inputs`. The risk itself is the same: the app stores no
`web_page` identity fact for the item link. This document restates the same
finding with the exact function and line that write the column.

`db::TrackRow`, defined at `src/db.rs` lines 23 to 45, has no `link` field.
No current read path returns the stored `tracks.link` column value to display
code.

**Risk two: the local selector matches the wrong link type.**
`src/views.rs::website_url_from_links`, line 343, matches only a row whose
`link_type` equals `website`. It never matches `link_type` `web_page`. This
function backs `src/db.rs::local_identity_links`, line 1538, on the local
route. A track's own `web_page` link, if one existed, could never surface
through this selector on the local route.

Contrast `src/metadata.rs::website_from_links`, line 308, used by
`track_website` on the API route. It matches link type `website` or
`web_page` (line 311). The local route and the API route apply different
link-type rules for what this document treats as the same field.

### Upstream Rule

Upstream `extract_links`, line 826, maps a non-namespaced `link` under a
track or `live_item` node to `link_type` `web_page` with `extraction_path`
`entity.link` (line 836). It maps the Atom `rel="alternate"` link under a
track or `live_item` node to `link_type` `web_page` with `extraction_path`
`entity.atom:link[@rel='alternate']` (lines 851 to 864). Either match
produces the `web_page` entry in a MusicIndex `Track.source_links`
collection.

### Target Rule (Proposed, Packet 010)

Packet 010 must store the item RSS `link` as a track `web_page` identity
fact. It must use the extraction path `entity.link`, matching the string
Stophammer uses for this source. This target rule needs operator acceptance
before packet 010 runs. Today the app keeps only a plain database column for
this value.

## Transcript Links

### Accepted Transcript Rules

Accepted on 2026-09-21: prefer fresh direct RSS track transcript claims over MusicIndex.
Retain all transcript candidates and their source evidence. Keep each claim's declared owner, URL, MIME type, language, relation, and position.

Accepted separately: apply the description fields' verified-removal, declared-source-order, unresolved-conflict, and stale-state rules.
Retain the last selected state, including absence, and the underlying evidence. Failed or incomplete refreshes cannot prove removal.

MusicIndex representation, accepted separately: prefer full `source_transcripts` claims over legacy transcript links.
Retain both forms of evidence. This rule applies within MusicIndex and does not change the accepted provider priority.

Legacy recognition, accepted separately: require explicit transcript, caption, or subtitle evidence.
An explicit link kind or a recognized extraction path can supply that evidence. A filename extension alone cannot supply it.
Retain filename-only matches as unresolved evidence without a transcript action.
Packet 020 must define and test the recognized markers at the shared selection boundary.

Alternatives, accepted separately: offer transcripts in different languages or formats with their declared labels.
Retain each alternative's language, format, relation, URL, and source evidence.
Distinct alternatives are not conflicting claims merely because their URLs differ. Apply source priority to corresponding candidates without erasing separate alternatives.
Do not invent missing labels or infer a transcript's language from the track or feed.

Legacy ownership, accepted separately: retain transcripts with unknown ownership in source details only.
Do not present them as track-owned transcripts or active track transcript actions.
An explicit transcript marker does not prove ownership. A verified source contract or source assertion must establish the owner.

URL actions, accepted separately: allow only valid HTTP or HTTPS transcript URLs.
Retain other URLs as source evidence without a transcript action.
This check does not replace the accepted owner, transcript-purpose, and selection requirements.

### Existing Upstream Transcript Collection

Stophammer `src/query.rs::SourceItemTranscriptResponse`, line 413, supplies a separate `source_transcripts` collection.
Each row contains `entity_type`, `entity_id`, `position`, `url`, `mime_type`, `language`, `rel`, `source`, `extraction_path`, and `observed_at`.
The track response include branch reads scoped transcript rows. It does not inherit feed transcripts.

The original audit found no matching app DTO field. Packet 032 now preserves this collection in transport.
This correction is separate from the legacy URL selectors below. Coverage, durable selection, and presentation remain later work.


### Three Storage Sites Named In The Packet, Plus One More

`src/rss/subscribe.rs::rss_track_link_inputs`, line 435, writes an identity
link fact with `link_type` `transcript`, `entity_type` `track`, and
`extraction_path` `podcast:transcript@url`.

`src/rss/subscribe.rs::track_extra_json`, line 316, writes the same
transcript URL into a JSON object that becomes the `tracks.extra_json`
column value. `src/db.rs::transcript_url_from_extra_json`, line 1300, reads
that column back and returns the `transcript_url` field into
`db::TrackRow.transcript_url`.

`src/metadata.rs::transcript_from_links`, line 319, reads MusicIndex
`source_links` for the API route. It matches a link whose `link_type`
contains `transcript`, `caption` or `subtitle`. It also matches a link whose
`extraction_path` contains `transcript`, or whose URL ends in a known
subtitle file extension (line 336, calling the helper defined at lines 345
to 350).

The review's own text calls these three sites. A fourth site exists in the
same file the review cites for the selector. The transcript closure inside
`src/views.rs::TrackView::from_api`, lines 632 to 639, matches only a link
whose `link_type` equals exactly `transcript`. It does not apply the wider
match rule that `transcript_from_links` applies.

The review corrects this exact point: the transcript selectors live in two
files, `src/metadata.rs` and `src/views.rs`. `src/views.rs` holds a read
selector, not a third storage site. This document records that same
correction. One conceptual value has two storage sites, `entity_identity_links`
and `tracks.extra_json`, and three read selectors that apply two different
match rules across `src/metadata.rs` and `src/views.rs`.

### Shared Selection Boundary

The accepted recognition rule replaces the original proposal to copy `transcript_from_links` into `TrackView::from_api`.
That helper accepts filename-only matches, which the operator rejected as sufficient evidence on 2026-09-21.
Packet 020 must apply the accepted rule through one shared selection owner for API and local routes.
The renderer must not infer transcript meaning from a filename.

## Enclosure Links

### Two Representations

`api::Track`, in `src/api.rs`, carries scalar fields `enclosure_url`,
`enclosure_type` and `enclosure_bytes` (lines 169 to 171), and a separate
typed list, `source_enclosures` (line 185). `api::SourceEnclosure`, line 325,
holds `url`, `mime_type`, `bytes`, `rel`, `title`, `is_primary`, `source` and
`extraction_path`. It has neither `entity_type` nor `entity_id`.

### Three Different Selection Orders

`src/track_compare.rs::select_audio_enclosure`, line 209, selects the first supported primary enclosure.
It then tries the first supported enclosure, followed by a supported scalar enclosure.
`selected_source_enclosure`, line 493, requires a nonempty URL and a recognized audio format.
An unsupported primary entry does not prevent selection of a later supported entry.

`src/view_models/track.rs::play_url`, line 123, uses a different order. It
checks the scalar `enclosure_url` first (when non-empty), then the first
`source_enclosures` entry marked `is_primary` (line 188), then the first
`source_enclosures` entry with any URL (line 195). This function is a second
consumer of `source_enclosures`, and its order is not the same as
`select_audio_enclosure`'s order.

`src/views.rs::TrackView`, lines 662 to 664 and 714 to 716, reads only the
scalar `enclosure_url`, `enclosure_type` and `enclosure_bytes` fields for its
`audio_url`, `mime` and `bytes` fields. It does not read `source_enclosures`
at all.

This document treats the enclosure as one conceptual field. The display path
(`TrackView`), the download-selection path (`select_audio_enclosure`) and the
playback path (`play_url`) use three different rules for that one field.

### Accepted Enclosure Rule

The operator accepted ADR 0075 Decision E on 2026-09-19.
Each operation selects only formats that it supports, in this order:

1. The first supported primary enclosure.
2. The first supported enclosure.
3. A supported enclosure from the direct scalar fields.

Use one priority order with an explicit capability check for each operation.
Playback and download can select different files when their supported formats differ.
Keep the selected URL, MIME type, and byte count from the same enclosure.
No supported candidate means no available action for that operation.
The implementation packet must name each capability owner and test the unsupported-primary case.
This document does not define new playback capabilities or release deferred playback work.

### Enclosure Transport Correction

Stophammer `src/query.rs::SourceItemEnclosureResponse`, line 427, already supplies owner fields, position, and observation time.
The app drops `entity_type`, `entity_id`, `position`, and `observed_at` during decoding.
[Packet 030](../tasks/archive/adr-0075-task-030-enclosure-claim-transport.md) assigns that app transport correction.
It changes no format selection or upstream contract.

## Image Links

No link type `image` exists today. `stophammer-parser/src/engine.rs::extract_links`
lists no `image` link type in any of its match arms. This document also
checked every selector in `src/metadata.rs` and `src/views.rs`. None of them
read a link type named `image`.

`Feed.image_url`, `Track.image_url` and `Contributor.img` are plain scalar
fields in `src/api.rs`. No `source_links` row records an image link with its
own extraction path or source. An image value today carries no evidence of
which element supplied it.

Packet 008 now requests separate artwork facts with declared owners and source evidence.
The current scalar image field remains for compatibility. Packet 011 must design storage for the separate assertions.
This correction does not require that artwork use the generic identity-link collection.

## Unknown Link Kinds

The app already stores rows with two or more link types that no selector
reads for display. `src/views.rs`, line 768, holds a test fixture with
`link_type` `donate`. Upstream `extract_links`, lines 837 to 843, maps the
Atom `rel="self"` link under a feed to `link_type` `self_feed`.

`entity_identity_links` and `source_links` preserve these rows. No code path
this document inspected filters them out at write time. No current selector
function reads them. `website_from_links`, `website_url_from_links` and
`transcript_from_links` each match a fixed, short list of link types, and
none of those lists includes `donate` or `self_feed`.

ADR 0075 decision 3 states two duties for an unknown link kind: preserve the
evidence, and record whether the app supports the syntax. No current field
records a supported or unsupported flag for a link type. This is an open
question for packet 011, the schema design packet. It must decide where that
flag lives before a code packet can read it.

<a id="source-order-is-a-proposed-rule"></a>

## Accepted Source Priority

The [review](../reviews/adr-0075-metadata-contract-review.md#source-order-can-select-the-displayed-identity)
records that `src/db.rs::local_identity_links`, line 1538, orders rows by
`source COLLATE NOCASE, position, id`. `src/views.rs::website_url_from_links`,
line 343, returns the first `website` link in that row order. An alphabetical
source label can therefore decide the displayed feed website when two
sources both supply one. [Case C07](adr-0075-metadata-example-corpus.md#c07)
and [the corpus's unresolved-results table](adr-0075-metadata-example-corpus.md#unresolved-results)
name this same gap and assign it to this packet.

The operator accepted ADR 0075 Decision F on 2026-09-19.
Prefer a fresh direct RSS value over the corresponding MusicIndex value for the same owner and link kind.
This rule covers feed `website` and track `web_page`. It does not cover transcript, image, or enclosure selection.
Store both providers' values and preserve a discrepancy under Decision G.

The record keeps both URLs, resources, extraction paths, source times, and actual fetch times.
Do not infer that a changed URL proves the Index is stale.

Selection details, accepted separately for feed websites and track pages on 2026-09-20:

1. For RSS feed websites, select the channel `link`.
2. For RSS track pages, select the item `link`, then a supported Atom alternate link.
3. In one Index observation, select a matching owner and link kind by supplied position.
4. Keep conflicting rows with missing or tied positions as unresolved alternatives.
5. Fresh verified RSS absence hides retained Index values. Fresh verified Index absence hides stale RSS values.
6. If neither observation is current, retain the last selected field state with its stale label, including selected absence.

Keep removed values and discrepancy evidence. Unknown coverage and failed requests cannot establish absence.
Expiry cannot restore a value removed by verified absence.

This order follows source structure and preserves provider evidence. It never ranks a value by its source label's spelling.
When no source supplies a usable value, show no website or page action.
Packet 018 must define freshness. Packet 035 must define URL comparison without discarding meaningful path or query differences.

Failed requests and omitted collections cannot create or resolve a value discrepancy.
The future Index update hook remains deferred.

### Accepted Website URL Actions

Accepted separately on 2026-09-21: feed website actions allow only valid HTTP or HTTPS URLs.
Retain other schemes as source evidence without a website action.
Track page actions, accepted separately on 2026-09-21: allow only valid HTTP or HTTPS URLs.
Retain other schemes as source evidence without a page action.

Feed website comparison, accepted separately: normalize scheme, host, and default ports while preserving path, query, and fragment differences.
Retain the original URL. The [comparison contract](adr-0075-comparison-and-discrepancy-contract.md#website-and-page-comparison-version-1) defines the technical details.
Track page comparison, accepted separately: apply the same scheme, host, and default-port normalization.
Preserve path, query, and fragment differences and the original URL.

## Corpus Agreement

[Case C07](adr-0075-metadata-example-corpus.md#c07) records the feed
`website` current result and cites the same extraction path values,
`channel/link` and `feed.link`, that this document cites. This document
agrees with C07.

[Case C08a](adr-0075-metadata-example-corpus.md#c08a) records the item
`link` current result. The app stores the value only in the `tracks.link`
column. `rss_track_link_inputs` builds only a transcript link. This document
agrees with C08a and applies the same correction to the review's
attribution.

[Case C08b](adr-0075-metadata-example-corpus.md#c08b) records the Atom
alternate link current result. The `atom` feature is not enabled, so the
element is stored under the extension key `atom`. No app function reads it.
This document agrees with C08b.

## Dependency On Packet 003

The [phase plan's packet register](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register)
lists packet 007 with the input "Packets 002 and 003." Packet 003 supplies
the [identity syntax and purpose contract](adr-0075-identity-syntax-contract.md).
That contract names the supported `podcast:txt` purpose value, `npub`.

The identity syntax contract document, as written, also lists a proposed
compatible value, `nostr`. The operator later decided this point.
[ADR 0075, Decision D](../adr/0075-metadata-ownership-and-completeness.md#status)
and the [review's operator decision record](../reviews/adr-0075-metadata-contract-review.md#operator-decision-on-nostr-purpose-values--2026-09-19),
both dated 2026-09-19, state that the operator rejected the `nostr`
compatibility proposal. The supported `podcast:txt` purpose value is `npub`
only. A value with `purpose="nostr"` is unsupported syntax.

This document's link-kind rules do not name a `podcast:txt` purpose value and
do not change the Nostr identity syntax that packet 003 owns. The two
documents cover separate row kinds: an identity row carries a `scheme`, and a
link row carries a `link_type`. Neither field rule in this document
reassigns a row from one kind to the other. This document confirms no
conflict with packet 003's supported purpose values before relying on that
contract.

## Open Questions For Later Packets

| Open question | Packet that must decide |
|---|---|
| Within-provider order, stale-value fallback, and URL comparison details under accepted Decision F | Packets 035 and 020, after acceptance of remaining details |
| Whether transcript matching should use one shared rule across the API route and the local route | Packet 020, after operator acceptance |
| The capability owner and supported formats for each operation under accepted Decision E | Packet 020 implementation packet. Existing playback gates remain in force |
| The wire and storage representation for separate artwork facts | Stophammer decision and packet 011 |
| Where the app records whether it supports a stored but unread link type such as `donate` or `self_feed` | Packet 011, the schema design |

## Retained Terms

| Retained term | Meaning in this document |
|---|---|
| declared owner | The subject that a source states for a value |
| extraction path | The string that names the source element a fact came from |
| identity link | A row in `entity_identity_links` or `source_links` with a `link_type` |
| selector | A function that reads stored links or enclosures and returns one value |
| provider | The service or RSS resource that delivered the facts |
| proposed | A rule that needs operator acceptance before a code packet implements it |
| order | The sequence in which a selector checks several rows or sources |
| priority | The rank that decides which source a selector checks first |
| conflict result | The value the app keeps when two sources disagree on one field |
| route | The API (Index) path or the local (on-disk) path through the app |
| acceptance | The operator's recorded decision that a rule may guide dependent code |
| review | A recorded assessment of evidence, instructions or implementation |
| purpose | The `podcast:txt` attribute that names the free-form use of the element |
| kind | A named classification for one field, such as a link kind |
| action | A typed control that the view model exposes, such as a page or audio action |
| evidence | Source material that supports a recorded claim |
| alternate | The Atom `rel="alternate"` link relation |
| case | A named example and its expected result, from the shared corpus |

Do not replace these terms with unrelated dictionary alternatives.

## References

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md)
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md)
- [Review](../reviews/adr-0075-metadata-contract-review.md)
- [Task 007 packet](../tasks/adr-0075-task-007-field-rules-links-and-media.md)
- [Example corpus, cases C07, C08a and C08b](adr-0075-metadata-example-corpus.md)
- [Identity syntax and purpose contract](adr-0075-identity-syntax-contract.md)

## Checks

The link check and the STE check ran on this document. Their results are in
the [task 007 report](../tasks/adr-0075-task-007-field-rules-links-and-media.md).
A person has not reviewed the current results against the cited functions.
That review gate stays open.
