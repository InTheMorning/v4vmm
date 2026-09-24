# ADR 0075 Metadata Field Inventory

## Status And Scope

Inventory written - 2026-09-19. This inventory assigns fields to rules and packets.
It does not claim that each assigned rule is complete or accepted.
The operator authorized completion orchestration on 2026-09-20. Unaccepted product policies and the visual pause retain their gates.

The inventory covers the inspected feed, track, contributor, artist, and publisher metadata boundaries.
It also identifies upstream fields that the app does not decode.
Unknown extensions remain evidence governed by ADR 0075. An extension remains in scope when a typed model omits it.

## Evidence

| Boundary | Inspected source |
|---|---|
| Direct RSS | [subscribe.rs](../../src/rss/subscribe.rs), `subscribe_feed`. [enrich.rs](../../src/rss/enrich.rs), `fetch_track_enrichment_from_feed` |
| App transport | [api.rs](../../src/api.rs), `Feed`, `Track`, `Contributor`, `Artist`, `Publisher`, and source collection types |
| Local columns and facts | [db.rs](../../src/db.rs), `FeedRow`, `TrackRow`, schema and fact types. [identity_ingest.rs](../../src/identity_ingest.rs) |
| Display fields | [views.rs](../../src/views.rs), `FeedView`, `TrackView`, and contributor fields |
| Upstream parser | Stophammer `stophammer-parser/src/types.rs`, `IngestFeedData`, `IngestTrackData`, and nested types, at commit `a220f44` |
| Upstream API | Stophammer `src/query.rs`, `FeedResponse`, `TrackResponse`, `TrackSummary`, and nested response types, at commit `a220f44` |

The app source is at commit `d3c6ee4`. Documentation changes do not prove deployed behavior.
No live feed or Index request supplied this inventory.

## Implementation Progress — 2026-09-20

The tables below retain the original audit findings. This section records subsequent corrections.

| Packet | Corrected boundary | Remaining boundary |
|---|---|---|
| 030 | Enclosure DTOs retain owner, position, and source observation time. Complete and independently reviewed | Durable collection storage and selection |
| 009 | Direct RSS identities retain their owner and validation result. Rejected evidence remains in the active context. Integrated checks are Green | Durable evidence storage and shared selection |
| 010 | Direct RSS item pages reach track identity storage with raw evidence. Review and integrated checks are Green | Shared action selection |
| 032 | Track DTOs retain all ten transcript claim fields. Review and integrated checks are Green | Requests, storage, and selection |
| 033 | Track DTOs retain language and artist sort text. Review and integrated checks are Green | Storage and field-specific projection |
| 037 | DTOs preserve remaining existing collections and creation times. Review and integrated checks are Green | Requests, storage, and field-specific projection |
| 031, 034, 035 | Proposed field and comparison rules are written | Operator review of new product policies |

The [orchestration register](../plans/adr-0075-metadata-contract-phase-plan.md#active-orchestration--2026-09-20) owns current assignments and integrated check results.

## Feed And Track Scalars

Each packet number below refers to the [packet register](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register).
Packets 005, 006, and 007 own their existing field documents.
Packets 031 and 034 must supply the remaining field rules before affected code runs.

| Field | RSS or upstream value | App transport and local representation | Owner and rule assignment |
|---|---|---|---|
| Feed GUID | Channel `podcast:guid`. `FeedResponse.feed_guid` | `Feed.feed_guid`, `Track.feed_guid`. `feeds.feed_guid` and each track's `feed_id` relationship | Feed. ADR 0075 Decisions 1 and 5 |
| Track GUID | Item `guid`. `TrackResponse.track_guid` | `Track.track_guid`. `tracks.item_guid`, scoped by `feed_id` | Track in feed. ADR 0075 Decisions 1 and 5 |
| Feed URL | Requested RSS resource. `FeedResponse.feed_url` | `Feed.feed_url`, `Track.feed_url`. `feeds.feed_url` | Feed resource. Packet 017 provider and request identity |
| Feed title and name alias | Channel `title`. `FeedResponse.title` | `Feed.title`, `Feed.name`. `feeds.title` | Feed. Packet 031 |
| Track title and name alias | Item `title`. `TrackResponse.title` | `Track.title`, `Track.name`. `tracks.track_title` | Track. Packet 031 |
| Feed title on track and album title | Channel title. `TrackResponse.feed_title` | `Track.feed_title`. `tracks.album_title`. `TrackView.album` | Feed value or separately observed tag value. Packet 031 must keep that distinction |
| Track number | Item `podcast:episode`. `TrackResponse.track_number` | `Track.track_number`. `tracks.track_number` | Track. Packet 031 |
| Disc number | App RSS assigns `None`. No inspected track response field | No `Track` field. Local `tracks.disc_number`. `TrackView.disc_number` | Track metadata. Packet 031 must audit the tag source |
| Season | `IngestTrackData.season`. No inspected `TrackResponse` field | No app transport field or dedicated local column | Track. Packet 031 must document the loss without treating season as disc number |
| Description | Channel or item `description`. API scalar and release claims | `Feed.description`, `Track.description`. Feed column and typed facts | Separate feed and track owners. Packet 005, accepted Decision F |
| Artwork URL and media type | Channel image elements. Item image. API image scalar | `Feed.image_url`, `Track.image_url`. Feed and track image columns | Packet 005, Decision C. Upstream track image can contain feed fallback |
| Publisher text | Upstream `publisher_text`. Source extraction needs its own rule | Feed and track scalars. Typed publisher facts | Packet 005. Separate from structured publisher relationships |
| Artist text | RSS authors and person-derived text. `TrackResponse.track_artist` | `Track.track_artist`. `tracks.artist_name` | Packet 006. Free text does not prove an artist binding |
| Album artist text | Feed author data. Upstream `f.release_artist` | Feed and track `release_artist`. `tracks.album_artist_name` | Feed value in inspected API. Packet 006 |
| Raw author and owner text | Parser feed `author_name`, `owner_name`, and track `author_name` | API aliases and derived artist fields do not retain each original assertion | Packet 006 artist rules. Packet 031 must trace raw text separately from selected labels |
| Artist sort text | `FeedResponse.release_artist_sort`, `TrackResponse.track_artist_sort` | Feed field exists. Track field is lost. No dedicated local columns | Packet 031 rules. Packet 033 track transport |
| Raw medium | Channel `podcast:medium`. `FeedResponse.raw_medium` | `Feed.raw_medium`. `feeds.podcast_medium` | Feed. Packet 031 |
| Release kind | `FeedResponse.release_kind` | `Feed.release_kind`. Typed `musicindex_release_kind` fact | Feed. Packet 031. Do not equate raw medium with release kind |
| Language | Channel language and upstream per-track language | Feed field and column exist. `TrackResponse.language` is lost by app decoding | Packet 006 rules. Packet 033 track transport |
| Explicit state | RSS iTunes text. Feed and track API booleans | Separate API fields. `tracks.itunes_explicit`. Typed facts | Packet 006. Preserve unknown separately from false |
| Release date | Upstream scalar and release claims | `Feed.release_date`. Typed feed fact | Feed. Packet 006 |
| Publication date | RSS `pubDate`. `TrackResponse.pub_date`. Parser feed `pub_date` | `Track.pub_date`. Raw `tracks.pub_date`. Typed track fact | Packet 006. Packet 034 must also trace the parser's feed date |
| Duration | RSS iTunes duration. Upstream seconds | `Track.duration_secs`. `tracks.duration_seconds`, `itunes_duration_raw` | Track. Packet 006 conflict rule remains open |
| Episode count | Upstream count and RSS item count | `Feed.episode_count`. No dedicated local scalar column | Feed aggregate. Packet 034 must define coverage and derivation |
| Newest and oldest item times | Upstream `newest_item_at`, `oldest_item_at` | Feed fields exist. No dedicated local scalar columns | Feed aggregates. Packet 034. An item time is not a feed release assertion |
| Creation and update times | Upstream `created_at`, `updated_at` | DTOs retain `updated_at`, not `created_at`. Local observation fields vary | Packet 034. Distinguish source revision, source observation, and fetch time |
| iTunes feed type | Parser `IngestFeedData.itunes_type` | No corresponding inspected app DTO field | Feed. Packet 034 must record its meaning and downstream mapping |

## Source Collections

All collection members retain their owner, provider, source assertion, and available evidence under ADR 0075.
Packet 004 owns completeness. Packet 011 owns storage design. Packet 020 owns display selection.

| Collection and member fields | Current path and loss | Rule or correction packet |
|---|---|---|
| Contributors: `entity_type`, `entity_id`, `position`, `name`, `role`, `role_norm`, `group_name`, `href`, `img`, `npub`, `source`, `extraction_path`, `observed_at` | Packet 001 preserves transport. Typed local storage and display still lose claim fields | Packet 003 syntax. Packets 011–015 storage. Packets 020 and 022–025 display. ADR 0075 occurrence rules |
| Links: `entity_type`, `entity_id`, `position`, `link_type`, `url`, `source`, `extraction_path`, `observed_at` | RSS website and transcript facts exist. Item page link remains only a legacy column | Packet 007 rules. Packet 010 item page storage. Decision F covers website and page display priority |
| IDs: `entity_type`, `entity_id`, `position`, `scheme`, `value`, `source`, `extraction_path`, `observed_at` | API fields exist. RSS ownership and syntax remain incomplete | Packet 003 syntax. Packet 009 extraction. Packet 011 evidence |
| Release claims: `entity_type`, `entity_id`, `position`, `claim_type`, `claim_value`, `source`, `extraction_path`, `observed_at` | API fields exist. Local metadata selection retains only some claim kinds | Packets 005 and 006 named fields. Packet 031 remaining classification claims. Packet 014 unknown evidence |
| Enclosures: owner fields, `position`, `url`, `mime_type`, `bytes`, `rel`, `title`, `is_primary`, `source`, `extraction_path`, `observed_at` | Upstream supplies all fields. App loses owner, position, and observation time | [Packet 030](../tasks/adr-0075-task-030-enclosure-claim-transport.md) transport. Packet 007 supported selection. Packet 011 storage |
| Direct enclosure: `enclosure_url`, `enclosure_type`, `enclosure_bytes` | App DTO fields exist. Local `TrackRow` lacks byte count | Packet 007, Decision E. Packet 011 storage. Keep the chosen URL, MIME type, and byte count together |
| Transcripts: owner fields, `position`, `url`, `mime_type`, `language`, `rel`, `source`, `extraction_path`, `observed_at` | Upstream `source_transcripts` supplies these fields. App has no matching collection. Legacy paths retain only partial data | Packet 007 rules. Packet 032 transport. Packets 004 and 011 coverage and storage |
| Platform claims: `platform_key`, `url`, `owner_name`, `source`, `extraction_path`, `observed_at` | Upstream feed `source_platforms` exists. App drops the collection | Packet 034 rules and a bounded follow-up transport packet before use |
| Remote items: `position`, `medium`, `remote_feed_guid`, `remote_feed_url`, `source` | Feed and track response collections exist upstream. App drops both | Packet 034 relationship rules and a bounded follow-up transport packet before use |
| Publisher relationships: `direction`, remote/publisher/music feed GUIDs and URLs, `remote_feed_medium`, `reciprocal_declared`, `reciprocal_medium`, `two_way_validated` | Upstream `publisher` collection exists. App publisher search summaries represent a different concept | Packet 034 relationship rules. Do not substitute publisher display text for this collection |
| Payment routes: recipient name, type, address, custom key/value, split, fee | RSS value JSON and optional API routes. Upstream inherited routes lack owner markers | Existing payment-route contract remains unchanged. Packet 008 requests ownership evidence only |
| Value time splits: `start_time_secs`, `duration_secs`, remote feed/item GUIDs, `split` | Parser and upstream track response support the collection. App track DTO omits it | Packet 034 records the contract boundary. ADR 0075 changes no payment inheritance or broadcast scheduling |
| General Podcast Namespace evidence: position, scope, GUID, path, tag, attributes, text | Upstream parser retains the snapshot. App has no equivalent general raw snapshot | Packet 014 retention. Packet 034 must identify transport gaps and assign follow-up work |

## Summary Responses And Other Metadata Sources

`FeedResponse.tracks` contains `TrackSummary`, not full track detail.
Its fields are `track_guid`, `title`, `pub_date`, `duration_secs`, `image_url`, `track_number`, and `publisher_text`.
The app decodes these values into `Track`. Missing detail fields do not prove that values are absent or authorize deletion.
Packet 017 must identify summary coverage before packet 013 accepts a replacement observation.

| Boundary | Fields and governing rule |
|---|---|
| Artist subject | `artist_id`, name, sort name, area, begin/end years, URL, aliases, tags, image, update time, feed/track counts. The API sends none of these after 2026-04-08. [ADR 0079](../adr/0079-remove-musicindex-artist-subject-storage.md) deletes the stored subject facts |
| Artist binding | `publisher_feed_guid` of an album feed. [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) owns the binding. No name inference. The API sends no `artist_credit` after 2026-04-08 |
| Publisher search | Text, feed/track counts, and feed/track result lists. Packet 005 owns text. Packet 034 owns aggregates and collection coverage |
| Embedded tags | Format-specific fields and artwork remain separate governed by [ADR 0004](../adr/0004-format-neutral-audio-tag-boundary.md). [ADR 0008](../adr/0008-explicit-id3v24-write-boundary.md) governs writes |
| MusicBrainz | Lookup and release facts remain separate governed by [ADR 0005](../adr/0005-musicbrainz-metadata-lookup.md) and [ADR 0006](../adr/0006-musicbrainz-release-detail-enrichment.md) |
| Live items | Status, start/end time, content link, and shared metadata keep the existing broadcast contract. [ADR 0059](../adr/0059-broadcast-control-surface.md) remains unchanged |
| Local operational state | Database row IDs, membership, subscription state, local file paths, and playback state are not publisher metadata. Existing owners remain unchanged |

## Required Checks Before Field Implementation

1. Match each changed DTO, storage, or display field to an inventory row.
2. Read that row's field rule and acceptance status.
3. Check summary and detail coverage separately.
4. Preserve raw evidence before cleaning, normalization, or fallback.
5. Keep an unknown source field as evidence until its typed rule exists.
6. Update this inventory when a source adds a field.

Packet 031 supplies proposed title, number, sort-text, medium, and release-kind rules.
Packet 034 supplies proposed aggregate, remaining scalar, and relationship rules.
Packet 035 supplies proposed comparison details before packet 011 designs durable discrepancy storage.
These documents do not establish acceptance of new product policies.

## Operator Visual Check

The operator paused visual checks. This inventory requests no app launch and closes no existing gate.
