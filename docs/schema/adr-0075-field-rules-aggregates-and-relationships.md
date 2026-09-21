# ADR 0075 Field Rules: Aggregates And Relationships

## Status And Scope

Individual field review in progress - 2026-09-20. Packet 034 supplies the rules assigned by the [field inventory](adr-0075-metadata-field-inventory.md).
The document review remains open. This document releases no implementation packet.

[ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) already requires source separation, declared ownership, raw evidence, and explicit coverage.
Those requirements are accepted. Feed publication-date source priority, removal, conflict, and stale-state rules have individual acceptance.
Other selections and additional checks remain proposals unless explicitly accepted below.
Decision F does not give RSS priority for these fields.
Decision G does not extend discrepancy detection to these fields.

This document covers aggregate counts, item time extrema, recorded times, feed publication dates, and iTunes feed type.
It also covers platform claims, remote items, publisher relationships, value time splits, and general namespace evidence.
It adds no product screen, identity action, payment rule, or broadcast operation.

## Source Evidence

The app inspection uses [api.rs](../../src/api.rs), [db.rs](../../src/db.rs), and [identity_ingest.rs](../../src/identity_ingest.rs).
RSS evidence comes from [subscribe.rs](../../src/rss/subscribe.rs) and [enrich.rs](../../src/rss/enrich.rs).
Upstream evidence comes from `/home/citizen/build/stophammer`, commit `a220f441d57640912eed9b190d7c194284f87a38`.
No live request supplied this evidence. The deployed upstream revision remains unverified.

| Inspected upstream owner | Observed behavior |
|---|---|
| `src/query.rs`, `FeedResponse` and `TrackResponse` | Both responses contain `created_at` and `updated_at`. Optional includes control separate source collections |
| `src/api.rs`, feed construction near line 1988 | `episode_count` counts parsed tracks. Item extrema also include ended live items that have enclosures |
| `stophammer-parser/src/profile.rs`, feed date rules | The parser can substitute channel `lastBuildDate` for `pubDate` before ingestion |
| `src/api.rs`, release claims near line 2092 | The claim distinguishes the parser's feed date from the oldest item date. `feed.pub_date` can conceal build-date substitution |
| `src/api.rs`, release claims near line 2152 | The `itunes_type` claim preserves the parsed feed value with path `feed.itunes_type` |
| `src/db.rs`, `upsert_feed` and `upsert_track` | Inserts use supplied creation times. Conflict updates retain creation times and replace update times |
| `src/query.rs`, `handle_publisher_search` | Feed counts group matching music feeds. Track counts use exact publisher text against tracks |
| `src/query.rs`, `handle_publisher_detail` | Feed and track arrays each have a limit. The response still reports `has_more: false` |
| `src/query.rs`, `load_publisher` and `load_track_publisher` | Publisher relationships derive from remote items and reciprocal references known to that Index |
| `stophammer-parser/src/types.rs`, `IngestPodcastNamespaceSnapshot` | The parser preserves structured namespace elements. No corresponding field exists in the inspected public detail responses |

The app keeps several aggregate scalars in DTOs but has no corresponding `FeedRow` columns.
`rss::enrich` currently fills a missing episode count from the received RSS item count.
That count describes the received document. It does not prove the feed's complete publication history.

## Accepted Rules Applied To Every Row

Keep the requested subject separate from each fact's declared owner.
Keep the provider, resource, extraction path, source time, and actual fetch time when available.
Do not invent absent member provenance from the request metadata.
The [collection rules](adr-0075-collection-completeness-rules.md) govern replacement and failed refreshes.

An endpoint contract can establish collection ownership when every member belongs to the requested subject.
Record that ownership basis separately from an owner field supplied inside the member.
Unknown endpoint coverage does not establish a complete collection.
Equal URLs, names, GUID strings, or counts do not merge independently observed facts.

## Accepted Feed Publication Source Priority

Accepted on 2026-09-20: prefer valid fresh RSS channel `pubDate`.
Then use MusicIndex claims that prove an actual channel publication date.
Preserve both observations. The scalar and `feed.pub_date` path alone cannot prove channel publication.
Build time and oldest item time retain their own meanings. Without an eligible claim or retained selection, the field remains unknown.

Accepted separately: apply the description fields' removal, conflict, and stale-state rules.
Fresh verified RSS absence hides the retained MusicIndex value. Fresh verified MusicIndex absence hides a stale RSS value.
Retain original date text and source evidence. Invalid input, unknown coverage, and failed requests cannot establish absence.

Use declared source order for eligible claims. Report conflicting ties as unresolved.
When both sources expire, retain the last selected field state with a stale label, including selected absence.
Expiry cannot restore a removed date. Retain the earlier values as evidence.

This decision does not accept release-date fallbacks or change the track publication-date policy.

## Proposed Scalar Rules

Each row states its owner, source selection, conflict handling, and absent result.
Selections apply only to valid observations with known coverage for the selected field.
Packet 018 supplies freshness and response ordering. Packet 011 supplies storage.

| Field | Owner and source selection | Conflict and absent result |
|---|---|---|
| Feed episode count | Feed aggregate from one identified Index observation. Keep an RSS document item count as a separate derived value | Do not replace an Index total with a page length or local library count. Preserve differing scopes. No count means unknown |
| Artist feed and track counts | Artist aggregate from the identified provider and query scope | Keep explicit provider counts separate from received list lengths. Missing values remain unknown |
| Publisher feed and track counts | Aggregate for exact publisher text and the recorded query scope | A publisher search count does not establish an identity relationship. Missing counts remain unknown |
| Newest and oldest item times | Feed aggregates from one provider observation and its recorded derivation scope | Do not combine extrema from different observations. Missing extrema remain unknown. They do not supply a feed publication assertion |
| Index creation time | Feed or feed-scoped track, from that Index's `created_at` scalar | Retain provider time. It is neither release time nor local library creation time. Missing means unknown |
| Index update time | Feed or feed-scoped track, from that Index's `updated_at` scalar | Retain provider time. Do not use it as collection observation time or app request sequence. Missing means unknown |
| Source observation time | The source fact or collection that supplied `observed_at` | Never substitute `updated_at`, publication time, or fetch time. Missing means unknown |
| App fetch time | The completed app request | Record the actual completion time independently. A missing historical time remains unknown |
| iTunes feed type | Explicit channel `itunes:type` from fresh RSS, then the feed's `itunes_type` release claim | Retain both original strings. Unknown text remains unsupported evidence. No eligible value means unknown |

For iTunes feed type, stale observations remain evidence but do not supply a current selected value.
This proposed freshness rule does not add a visible field or change the existing track date policy.
The [date rules](adr-0075-field-rules-artist-language-dates.md) continue to govern release dates and track publication dates.

Counts accept nonnegative integers. Preserve negative or out-of-range input as invalid evidence.
Do not clamp counts, convert them to zero, or silently narrow an upstream `i64`.
The current app uses `Option<i32>` for several counts. A separate compatibility change must address that narrower transport range.

Timestamp zero is a value, not a missing-value marker.
Keep a timestamp outside the display library's range as evidence without a formatted display value.

Recognize the exact iTunes type tokens `episodic` and `serial` for typed classification.
Keep the raw value before trimming surrounding whitespace for that classification.
Other spellings remain unsupported. Do not infer a default type, release kind, track order, or season from this field.

## Proposed Collection Rules

These fields retain all occurrences. They do not select one provider's collection as universal truth.
No new display field or action is proposed for these collections.
Absent, omitted, empty, failed, and unsupported collections keep the states defined by packet 004.

| Collection | Owner and proposed interpretation | Conflict and absent result |
|---|---|---|
| Feed `source_platforms` | Feed claim under the feed endpoint contract. Preserve every platform key, URL, owner name, and supplied provenance field | Keep conflicting claims separately. A platform URL is not automatically the feed website. Absence gives no platform classification |
| Feed `remote_items` | Reference occurrence declared by the requested feed | Preserve position and target. Do not merge equal targets. Absence creates no relationship |
| Track `remote_items` | Reference occurrence declared by the feed-scoped track | Preserve it separately from feed references and value time splits. Absence does not inherit feed references |
| Feed or track `publisher` | Index interpretation for the exact requested subject. Preserve direction, targets, and supplied reciprocal flags | Retain conflicting relationships. Flags describe the Index result, not independently verified ownership. Absence creates no publisher binding |
| Track `value_time_splits` | Payment evidence attached to the feed-scoped track | Preserve every supplied member. Do not infer missing durations or change payment routing. Absence changes no existing payment behavior |
| General namespace evidence | Element occurrence with source scope, available GUID, path, tag, attributes, text, and document position | Preserve unknown extensions. Unknown scope or identity remains unresolved. Absence of this transport field does not prove absence of namespace elements |

The `owner_name` in a platform claim remains source text.
It does not prove a person identity, publisher relationship, or payment owner.
The publisher relationship collection is distinct from `publisher_text` and publisher search results.
`two_way_validated` is a provider assertion. The app must not rename it as local verification.

Remote references do not trigger subscription, fetching, payment, or navigation by themselves.
Unknown `direction`, `medium`, `platform_key`, and `source` values remain recoverable strings.
An absent relationship flag is unknown. It must not become `false` through a default.
The app must not inherit these collections through `track_with_feed_defaults`.

Value time split integers retain their signed `i64` representation at transport.
Preserve a missing duration separately from zero.
Do not normalize weights, resolve overlaps, infer recipients, or schedule broadcast actions in metadata ingestion.
The existing payment contract retains authority over payment interpretation.

## Namespace Evidence Boundary

The inspected parser snapshot contains these exact member fields:

| Field | Existing parser type |
|---|---|
| `position` | `i64` |
| `entity_scope` | `String` |
| `entity_guid` | `Option<String>` |
| `path` and `tag` | `String` |
| `attributes` | `HashMap<String, String>` |
| `text` | `Option<String>` |

The snapshot contains canonicalized paths and parsed text. It is not a byte-for-byte copy of the RSS response.
Packet 014 must preserve the response evidence before cleaning or fallback.
It must retain the namespace snapshot's actual fidelity when a snapshot is available.
It must not reconstruct missing source markup from cleaned DTO fields.

The public API has no inspected include for this snapshot.
A new app field alone cannot recover it. The upstream decision request must define delivery before Index ingestion can use it.
Direct RSS retention can preserve received XML independently. Unknown syntax must not produce identity actions.
This document adds no upstream API proposal beyond that evidence requirement.

## Summary And Detail Coverage

`FeedResponse.tracks` contains track summaries with seven fields.
Those fields are `track_guid`, `title`, `pub_date`, `duration_secs`, `image_url`, `track_number`, and `publisher_text`.
They do not establish full track metadata coverage.

Publisher detail has separate feed and track summary shapes.
Its feed members contain `feed_guid`, `feed_url`, `title`, `image_url`, `episode_count`, and `raw_medium`.
Its track members contain `track_guid`, `feed_guid`, `title`, `image_url`, `duration_secs`, and `track_number`.
Missing full-detail fields in these summaries cannot erase stored detail facts.

The publisher detail endpoint defaults to 50 entries per array and caps the limit at 200.
Its `has_more: false` does not prove complete membership because each query still applies its limit.
Even a full detail response needs an endpoint and version contract before collection replacement.
Keep scalar field coverage separate from collection membership coverage.

The API's current summary shapes and omission behavior are observations, not a deployed completeness guarantee.
Packet 017 must carry the request profile and response shape into ingestion.
Packet 013 must reject deletion when that coverage remains unknown.

## Existing API Fields Requiring App Transport

The coordinator should prepare one bounded transport packet for the fields below.
It changes only DTOs and tests in `src/api.rs`.
It adds no request include, database write, source selection, renderer, or payment consumer.
The packet needs no upstream API change.

Use optional client fields with defaults and omit absent added fields during serialization.
Preserve non-null values with their supplied types. Reject a wrong JSON type without numeric string coercion.
Use separate collection DTO names that cannot be confused with the existing publisher search DTO.

| App field | Inspected upstream member fields and types |
|---|---|
| `Feed.created_at`, `Track.created_at` | `created_at: i64` |
| `Feed.source_platforms` | `platform_key: String`, `url: Option<String>`, `owner_name: Option<String>`, `source: String`, `extraction_path: String`, `observed_at: i64` |
| `Feed.remote_items`, `Track.remote_items` | `position: i64`, `medium: Option<String>`, `remote_feed_guid: String`, `remote_feed_url: Option<String>`, `source: String` |
| `Feed.publisher`, `Track.publisher` | `direction: String`, `remote_feed_guid: String`, `publisher_feed_guid: String`, `music_feed_guid: String` |
| The same publisher member | `remote_feed_url: Option<String>`, `remote_feed_medium: Option<String>`, `publisher_feed_url: Option<String>`, `music_feed_url: Option<String>` |
| The same publisher member | `reciprocal_declared: bool`, `reciprocal_medium: Option<String>`, `two_way_validated: bool` |
| `Track.value_time_splits` | `start_time_secs: i64`, `duration_secs: Option<i64>`, `remote_feed_guid: String`, `remote_item_guid: String`, `split: i64` |

For client DTOs, wrap each upstream required scalar in `Option` to retain missing-field evidence from older responses.
Keep each collection as `Option<Vec<T>>`. Missing or null differs from an empty array at this boundary.
Request intent and completeness still require separate ingestion metadata.
There is no member owner, extraction path, or observation time in remote items, publisher relationships, or value time splits.
Do not fabricate those missing fields.

Transcript transport belongs to packet 032. Track language and artist sort text belong to packet 033.
iTunes type already uses the existing `SourceReleaseClaim` transport.
Feed publication evidence needs proof of the original channel publication assertion.
The inspected `feed.pub_date` path alone is insufficient because the parser can substitute `lastBuildDate`.
The general namespace snapshot needs an upstream delivery contract before its Index transport packet exists.

The coordinator should prepare a separate count compatibility packet after tracing each consumer.
That packet should preserve upstream `i64` counts and test values greater than `i32::MAX`.
It must not change display selection or substitute a count for unknown coverage.

## Review Decisions And Regression Cases

The operator must review these proposed selections before dependent policy code runs:

1. Keep Index counts separate from RSS document counts and local counts.
2. Select iTunes feed type values using the proposed source order.
3. Keep relationships, platform claims, and value time splits as evidence without new product actions.
4. Preserve provider relationship flags without treating them as local verification.

The later implementation packets must prove these cases mechanically:

| Case | Required result |
|---|---|
| RSS document has 10 items. Index count is 200 | Keep both values with their distinct scopes |
| Ended live item supplies the earliest date | Preserve the Index extremum without changing the feed publication date |
| `release_date` claim uses `oldest_item.pub_date` | Do not create a channel publication assertion |
| `release_date` claim uses `feed.pub_date` without original-date evidence | Retain the claim. Do not infer channel publication from a path that can conceal build time |
| Fresh verified RSS removes the feed publication date | Hide the retained MusicIndex date. Retain original date text and source evidence |
| Fresh verified MusicIndex absence follows a stale RSS date | Hide the stale RSS date and retain its evidence |
| Both feed publication sources expire | Retain the last selected value or absence with a stale label. Do not restore a removed date |
| Feed publication claims have conflicting source-order ties | Report an unresolved conflict and retain each claim |
| Feed publication input is invalid or collection coverage is unknown | Retain evidence without declaring verified absence |
| iTunes type is omitted, unknown, or valid | Keep unknown, unsupported evidence, and recognized type distinct |
| Publisher detail returns its requested limit | Do not infer complete membership from `has_more: false` |
| A summary omits relationships or timestamps | Keep existing detail facts |
| Remote references repeat a target at different positions | Preserve both occurrences |
| Reciprocal flag is missing, false, or true | Keep all three states distinct |
| Value time split duration is missing or zero | Preserve the difference without changing payment behavior |
| Namespace evidence lacks an item GUID | Retain the occurrence without inventing a track owner |
| Provider time differs from fetch time | Preserve both values under their correct meanings |
| Count exceeds `i32::MAX` | Preserve the integer through the separately scoped compatibility change |

## Operator Visual Check

No visual change belongs to this document packet. The visual pause remains in force.
No app launch or fixture cleanup is required.
