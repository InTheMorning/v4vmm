# ADR 0075 Stophammer Decision Request

## Purpose

This document requests a decision in the Stophammer repository.
[ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) governs this app.
It cannot govern Stophammer. Stophammer records its own decision.

This document states the behavior that the app needs from the Index API and the parser.
It adds no new product decision. Each requirement cites an ADR 0075 decision number,
or a named file and function.

This request uses the ADR 0075 terms below.

| Term | Meaning |
|---|---|
| Fact | A recorded source value with its owner and evidence |
| Subject | The feed, track or contributor that a fact describes |
| Claim | An assertion that a source makes about a subject |
| Provenance | Evidence of a value's owner, source, extraction path and observation time |
| Provider | The service or RSS resource that delivered the facts |
| Collection | A set of related facts in a response |
| Snapshot | A stored copy of a provider's collection from one observation |
| Coverage state | Whether a request returned the complete collection |
| Projection | Shared code that prepares source facts for display |

## Limits Of This Request

- The agent must not write in the Stophammer checkout.
- The agent must not reserve a Stophammer ADR number.
- The deployed Stophammer revision remains unverified. Ask the operator for
  that evidence before Stophammer acts on this request.

## Source Documents

This request assembles the output of the ADR 0075 document packets.
It depends on packets 002 through 007 and the corrected field inventory.
Unfinished field rules remain assigned work. Their packet numbers do not establish accepted policy.

| Packet | Document | Use in this request |
|---|---|---|
| 002 | [Metadata example corpus](../schema/adr-0075-metadata-example-corpus.md) | The named cases |
| 003 | [Identity syntax contract](../schema/adr-0075-identity-syntax-contract.md) | The parser requirements |
| 004 | [Collection completeness rules](../schema/adr-0075-collection-completeness-rules.md) | The completeness signal |
| 005 | [Description, artwork and publisher rules](../schema/adr-0075-field-rules-description-artwork-publisher.md) | Field ownership |
| 006 | [Artist, language and date rules](../schema/adr-0075-field-rules-artist-language-dates.md) | Field ownership |
| 007 | [Links and media rules](../schema/adr-0075-field-rules-links-and-media.md) | Link kinds and owners |
| Inventory | [Metadata field inventory](../schema/adr-0075-metadata-field-inventory.md) | Scalars, transport losses, summaries, and remaining assignments |

The inspected Stophammer revision is commit `a220f44`. No agent wrote in that checkout.

Decisions D through G record accepted syntax, supported formats, limited RSS priority, and discrepancy retention.
Other field policies remain proposals. Dependent code requires acceptance of its affected policies.

## 1. Required App Behavior

Each requirement below names the ADR 0075 decision that creates it.

1. **Each claim carries its declared subject.** A contributor claim, a link, an
   identifier and a release claim each state the owner that the source declared.
   ADR 0075 Decision 1.
2. **A contributor occurrence stays separate.** The subject, the source collection and
   the position identify one credit. Two credits with one name stay two credits.
   ADR 0075 Decision 1.
3. **Each claim carries its provenance.** The claim states the assertion source, the
   extraction path and the source observation time. ADR 0075 Decision 1.
4. **An inherited credit keeps its original owner.** A track response can return a feed
   credit. That credit declares `entity_type` `feed`. ADR 0075 Decision 4.
5. **The app separates five collection states.** They are not requested, not
   returned, returned empty, returned populated, and request failed.
   ADR 0075 Decision 2.
6. **A success status does not establish completeness.** The contract states
   completeness for each endpoint, collection and API version. ADR 0075 Decision 2.
7. **A malformed payload is a failed request.** It is not an empty collection.
   ADR 0075 Decision 2.
8. **The parser preserves unsupported syntax as evidence.** It records whether the app
   supports the syntax. ADR 0075 Decision 3.
9. **The parser does not infer an owner.** It takes the owner from the element position
   in the document, and it states that owner in the response. ADR 0075 Decision 3.
10. **The generated API contract matches the deployed routes.** ADR 0075 Decision 6.
11. **A repair states its evidence.** A parser change needs a controlled plan to crawl
    or ingest feeds again. ADR 0075 Decision 7.

## 2. Required Fields And Collections

The app must distinguish upstream evidence from losses during app decoding.
The inspected upstream types are in Stophammer `src/query.rs` at commit `a220f44`.

| Collection | Upstream evidence | App gap and assigned work |
|---|---|---|
| `source_contributors` | `SourceContributorClaimResponse` supplies owner, position, credit attributes, source, path, and observation time | Packet 001 preserves transport. Typed storage and display remain incomplete |
| `source_links` | `SourceEntityLinkResponse` supplies owner and provenance | App fields exist. Packet 007 defines selection |
| `source_ids` | `SourceEntityIdResponse` supplies owner and provenance | App fields exist. Packet 003 defines supported syntax |
| `source_release_claims` | `SourceReleaseClaimResponse` supplies owner and provenance | App fields exist. Field-specific selection remains incomplete |
| `source_enclosures` | `SourceItemEnclosureResponse`, line 427, supplies owner, position, and observation time | App `SourceEnclosure` drops `entity_type`, `entity_id`, `position`, and `observed_at`. Packet 030 owns this correction |
| `source_transcripts` | `SourceItemTranscriptResponse`, line 413, supplies owner, position, URL, MIME type, language, relation, source, path, and observation time | App drops the complete collection. Packet 032 owns transport |
| `payment_routes` | `RouteResponse` supplies payment attributes without an owner marker | Upstream ownership evidence needs a decision. Existing inheritance remains unchanged |

The contributor fields are `entity_type`, `entity_id`, `position`, `name`, `role`, `role_norm`, `group_name`, `href`, `img`, and `npub`.
They also include `source`, `extraction_path`, and `observed_at`.
[Task 001](../tasks/adr-0075-task-001-contributor-claim-transport.md) records their transport contract.

The four enclosure claim fields already exist upstream. They need no new upstream field.
`TrackResponse.language` and `track_artist_sort` are also lost by the app. Packet 033 owns that transport correction.

**Request to Stophammer.** Define how payment-route responses identify the declared owner of inherited routes.
Keep the existing payment inheritance contract. The app needs ownership evidence, not a new payment-selection policy.

### Required Artwork Ownership

`get_track_rows_by_guid` and `get_track_row_for_feed` select `COALESCE(t.image_url, f.image_url)` in Stophammer `src/query.rs`.
The returned `Track.image_url` does not identify which owner supplied the image.
Matching URLs cannot recover that evidence.

**Request to Stophammer.** Return separate artwork facts with declared owners, source evidence, and complete/absent coverage.
Keep the existing `image_url` display field for older clients.
The new facts must distinguish a track image from feed fallback, even when both URLs match.
Include a case with no track image and a case where both owners assert the same URL.
This supplies ADR 0075 Decisions 1 and C without changing the accepted artwork display order.

### Remaining Scalar And Relationship Evidence

The field inventory identifies upstream platform, remote-item, publisher, and time-split collections absent from the app DTOs.
Packets 031 and 034 must complete their rules and assign bounded transport work before dependent use.
Their absence in the app is not a request to invent new upstream collections.

The inspected track `release_artist` comes from `f.release_artist`. Document that feed ownership in the API contract.
The feed artist helper can return "Unknown Artist". Clarify whether this is derived fallback text or a publisher assertion.
Retain that distinction in any new source-fact contract. This request does not claim that a live response currently contains that text.

## 3. Required Completeness Signal

Packet 004 separates request intent, response shape, and completeness.
The app knows the include list that it sent.
An omitted include means "not requested". A requested collection decoded as missing or null means "not returned".
No API addition is needed for that distinction. The app must retain request intent with the response.

At commit `a220f44`, `TrackResponse` and `FeedResponse` supply no collection completeness field.
The app requests `source_enclosures` on the feed route, which ignores that unsupported token.
The app can record "not returned", but it cannot determine why the server omitted the collection.

**Request to Stophammer.** Define how each endpoint establishes completeness for each returned collection and API version.
Specify pagination, unsupported includes, inherited credits, and explicit empty results.
Decide whether an explicit completeness signal supplies that evidence.
Until completeness is established, a returned array cannot authorize deletion of earlier facts.

Also document the limited field coverage of `TrackSummary` inside `FeedResponse.tracks`.
Missing summary fields must not establish absence in full track detail. ADR 0075 Decision 2 requires this distinction.

## 4. Required Parser Behavior

Packet 003 states the identity syntax that this app supports.

**Supported identity syntax.** The app supports the `podcast:txt` purpose value `npub`.
The operator decided this on 2026-09-19. ADR 0075 records it as Decision D.
The app treats `purpose="nostr"` as unsupported syntax and keeps the value as evidence.

`stophammer-parser/src/engine.rs::extract_entity_ids`, line 802, accepts only
`purpose="npub"` today. Line 814 drops every other purpose value from the typed list.
That behavior agrees with Decision D.

**The owner of a `podcast:person` key.** `extract_persons`, line 779, reads direct
`person` children of the node that the caller passes. The caller passes the channel node
or the item node. The parser extracts occurrence attributes and position from that location.
`src/api.rs::build_source_contributor_claims`, line 624, assigns the typed owner, source, extraction path, and observation time.
No attribute on the `podcast:person` element states that owner.

An `npub` attribute on a credit is a contributor identity. It is not a feed identity and
it is not a track identity. ADR 0075 Decision 1.

**Valid positions for a `podcast:txt` value.** `extract_entity_ids`, line 802, reads only
direct children of the passed node. A channel `podcast:txt` value belongs to the feed.
An item `podcast:txt` value belongs to the track. A nested element is not read.

**Namespace prefix.** `is_podcast_namespace`, line 535, compares the resolved namespace
URI. It accepts `https://podcastindex.org/namespace/1.0` and the legacy URI. The prefix
does not change the owner. The app's own RSS route does not match this behavior, because
the `rss` crate keys an extension by the literal prefix. Packet 009 owns that app defect.

**Request to Stophammer.** Confirm that the parser keeps the owner from the element
position. Confirm that it keeps an unsupported purpose value as evidence. Decide whether
a response reports which syntax the parser supports.

## 5. Compatibility Requirement

An older payload must stay readable after a Stophammer change.
The app decodes each collection field as optional. A removed field decodes to `None`,
which the app reads as "not returned". A renamed field therefore loses data silently.

The generated API contract in `openapi.rs` must match the deployed routes and fields.
Do not use a manual change to generated JSON as the source of the API contract.
ADR 0075 Decision 6.

The deployed Stophammer revision remains unverified. Packet 028 must establish it.

## 6. Recrawl Requirement

A parser deployment alone does not change stored facts.

`src/ingest.rs`, line 28, carries the `content_hash` field on an ingest request.
`src/verify.rs` registers `verifiers::content_hash::ContentHashVerifier`, line 308.
That verifier short-circuits an unchanged feed, as `src/verify.rs`, line 40, records.
An unchanged feed hash therefore causes the ingestion to skip the feed.

`src/ingest.rs`, line 32, carries the `force_reingest` field.
`src/verifiers/content_hash.rs::ContentHashVerifier::verify`, line 30, bypasses this check when that flag is true.
Other ingestion checks still apply. An unchanged hash otherwise returns the `NO_CHANGE` sentinel for ordinary music feeds.

**Request to Stophammer.** Write the procedure that crawls or ingests feeds again after
a parser change. State which feeds need `force_reingest`. State how an operator confirms
that the stored facts changed. ADR 0075 Decision 7.

## 7. Signed-Event And Replica Checks

ADR 0075 Decision 7 requires compatibility with signed events and replicas.
The [phase plan](adr-0075-metadata-contract-phase-plan.md) requires a test of
signed-event replay for a changed Stophammer data path.

**Request to Stophammer.** For each changed data path, add a test from RSS through
ingestion to the query response. Add a signed-event replay test for that path.
State how a replica receives the repaired facts. State the expected replica state after
a recrawl.

## 8. Open Questions

The app cannot answer these questions alone.

| Question | Why it is open | Needed by |
|---|---|---|
| Will the upstream contract preserve unsupported purpose values as accessible evidence? | The app supports `npub` only. Raw parser evidence exists, but its API contract needs confirmation | Packets 009 and 014 |
| How will the API establish completeness for each collection? | No explicit signal exists in the inspected response | Packets 011 and 017 |
| How will payment routes identify their declared owner? | Upstream route rows have no owner marker. Enclosure rows already have one | Packets 011 and 020 |
| How will separate artwork facts identify track and feed assertions? | The existing track scalar already combines both owners | Packets 011 and 020 |
| Will the API document feed ownership of track `release_artist` and identify derived artist fallback text? | The query joins the feed value. The helper can synthesize "Unknown Artist" | Packets 006 and 020 |
| Can one response mix track-owned and feed-owned contributor claims? | The inspected helper returns one set or the other. No contract states the future shape | Packets 013 and 020 |
| Can a nested collection arrive truncated? | `openapi.rs` documents cursor and limit. The handler does not use them for nested collections | Packet 017 |
| What is the deployed Stophammer revision? | Only a local checkout was inspected | Packet 028 |

## 9. Discrepancy Evidence And Future Updates

The operator accepted fresh direct RSS priority for descriptions and website links.
The app must retain differing Index values with their owners, resources, source times, and actual fetch times.
Description comparison uses readable text. Equivalent formatting alone does not create a discrepancy.
Repeated observations update one discrepancy. Later comparable agreement resolves it without deleting its evidence.

This behavior needs stable subject identity and retained source provenance from existing API data.
It does not require an upstream update endpoint now.
A possible hook that signals MusicIndex to update remains deferred. No current packet sends that signal.

## Retained Terms

| Retained term | Meaning in this request |
|---|---|
| coverage state | Whether a request returned the complete collection |
| include | The API request parameter that names requested collections |
| purpose | The `podcast:txt` attribute that names an identifier kind |
| recrawl | A repeated crawl or ingestion of a feed |
| replay | A repeated application of a stored signed event |
| replica | A copy of the Index data on another host |
| route | One API endpoint |
| proposal | A rule that needs operator acceptance |

## Checks

The link check and the language check ran on this document.
The [packet](../tasks/adr-0075-task-008-stophammer-decision-request.md) records the results.

No person walked the review gate of this document. That gate is open.
A Stophammer maintainer did not read this request yet.
No new Stophammer decision exists for this request.
Code that needs a changed upstream contract waits for that decision and compatible implementation evidence.
Existing-contract transport corrections need no unrelated upstream change. The operator's separate code dispatch hold still applies.
