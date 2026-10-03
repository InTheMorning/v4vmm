# ADR 0075 Collection Completeness And Transition Rules

## Scope

This document states the completeness and transition rules for each response
collection that the app requests.
[ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) governs each
rule. The primary sources are
[decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit)
and
[decision 5](../adr/0075-metadata-ownership-and-completeness.md#5-replace-a-complete-provider-snapshot-atomically).
Packet 011 must use these rules to design the schema. Packet 017 must use
these rules to define request profiles.

Each rule below carries one of three labels.

- OBSERVED. A statement about current code, with a file, a function and a
  line.
- REQUIRED. A rule that decision 2 or decision 5 already states, or a direct
  consequence of that stated rule. A REQUIRED rule is not a new product
  decision. No current code implements it yet.
- OPEN. A named question for packet 008. Current evidence cannot answer it.

This document changes no code, no schema and no acceptance gate. It does not
propose a Stophammer ADR number or a Stophammer code change. Packet 008 owns
that request. It does not state a field-level fallback rule for description,
artwork, publisher or any other field.

Packets 005, 006 and 007 own those field-level rules. This document does
not state supported Nostr syntax or `podcast:txt` purposes. Packet 003 owns
that rule. It does not state a database schema, a migration or a
provider-identity design. Packet 011 owns that design.

This document does not state a projection rule. A projection is the shared
code that prepares stored facts for display, per ADR 0075's Terms table.
Decision 6 owns projection rules.

The Review Criteria for this document require a person's review. That review
has not run. Report this document's own gate as open, never as met.

## Evidence

| Source | Revision | Use |
|---|---|---|
| This repository, working tree | After ADR 0075 packet 001 | Current app behavior |
| `/home/citizen/build/stophammer` | Commit `a220f44` | Current upstream API behavior |
| [Metadata example corpus](adr-0075-metadata-example-corpus.md) | Packet 002 deliverable | Worked cases C11 through C18c |

This document cites no live network request. No agent launched the app.

## Endpoint And Collection Inventory

The app's `src/api.rs::Feed` (line 124) and `src/api.rs::Track` (line 157)
together expose seven optional collection kinds. Upstream supplies additional collections that the app does not decode.
The [field inventory](adr-0075-metadata-field-inventory.md#source-collections) assigns those losses to packets.
`src/api.rs::fetch_feed` (line 501),
`fetch_track` (line 509) and `fetch_feed_track` (line 517) each build that
parameter from a caller-supplied value.

| Collection | Declared in | Requested by | Requested subject | Owners the collection can return | Inheritance observed |
|---|---|---|---|---|---|
| `tracks` | `Feed`, line 143 | `INDEX_FEED_DETAIL_INCLUDE` (search.rs:737) | Feed | Track facts scoped to the requested feed. Collection membership belongs to that feed | None. Upstream `build_feed_response` (stophammer query.rs:661) queries `tracks WHERE feed_guid = ?1` (line 701) only |
| `source_contributors` | `Feed`, line 145. `Track`, line 179 | `INDEX_FEED_DETAIL_INCLUDE`, `fetch_contributors` (api.rs:550), and `MUSICINDEX_TRACK_PERSISTENCE_INCLUDE` (subscribe_service.rs:26). `fetch_index_track_detail` (search.rs:744) requests no collection | Feed or Track | The declared owner of each entry, from its own `entity_type` and `entity_id` (task 001 field contract) | Yes, for a track request only. See [the transition table](#the-complete-track-credit-transition-table) |
| `source_links` | `Feed`, line 147. `Track`, line 181 | Same profiles as `source_contributors`, minus `fetch_contributors` | Feed or Track | The declared owner of each entry (`SourceEntityLink`, api.rs:286-295) | None observed. Upstream reads `get_source_entity_links_for_feed_entity` (query.rs:900-906) with no feed fallback |
| `source_ids` | `Feed`, line 149. `Track`, line 183 | Same profiles as `source_links` | Feed or Track | The declared owner of each entry (`SourceEntityId`, api.rs:299-308) | None observed. Upstream reads `get_source_entity_ids_for_feed_entity` (query.rs:909-915) |
| `source_release_claims` | `Feed`, line 150. `Track`, line 184 | Same profiles as `source_links` | Feed or Track | The declared owner of each entry (`SourceReleaseClaim`, api.rs:312-321) | None observed. Upstream reads `get_source_release_claims_for_feed_entity` (query.rs:928-934) |
| `source_enclosures` | `Track` only, line 185. `Feed` holds no such field | `MUSICINDEX_TRACK_PERSISTENCE_INCLUDE`. `INDEX_FEED_DETAIL_INCLUDE` also names this token for a feed request | Track | Upstream `SourceItemEnclosureResponse` declares the owner. App `SourceEnclosure` drops both owner fields | None observed. Upstream reads `get_source_item_enclosures_for_feed_entity` (query.rs:937-944), scoped by feed and track |
| `payment_routes` | `Feed`, line 151. `Track`, line 186 | `INDEX_FEED_DETAIL_INCLUDE`, `fetch_value_routes` (api.rs:570), and `MUSICINDEX_TRACK_PERSISTENCE_INCLUDE` | Feed or Track | Unknown per entry. `PaymentRoute` (api.rs:274-282) carries no `entity_type` or `entity_id` field | Yes, for a track request. Upstream substitutes the feed's routes when the track has none (query.rs:859-882). The substituted rows carry no owner marker |

The `INDEX_FEED_DETAIL_INCLUDE` row for `source_enclosures` is a named
example of [an unsupported include value](#effect-of-an-unsupported-include-value).
Upstream enclosure rows already declare their owner. The app DTO loses that declaration.
[Packet 030](../tasks/archive/adr-0075-task-030-enclosure-claim-transport.md) corrects that transport loss.
Payment routes have a different gap: the inspected upstream rows also lack owner fields.
Packet 008 requests that upstream decision without changing payment inheritance.

### Upstream Collections Missing From App Transport

| Collection | Requested subject and owner evidence | Current app coverage | Assigned work |
|---|---|---|---|
| `source_transcripts` | Track. `SourceItemTranscriptResponse`, query.rs:413, supplies owner, position, URL, MIME type, language, relation, source, path, and observation time | No matching DTO field or include profile | Packet 032 transport, packet 017 requests, packet 011 storage |
| `source_platforms` | Feed. `SourcePlatformClaimResponse` supplies platform, URL, owner text, source, path, and observation time | No matching DTO field | Packet 034 rules and bounded transport follow-up |
| `remote_items` | Feed or track. The response context and remote GUID/URL fields describe relationships | No matching DTO field | Packet 034 rules and bounded transport follow-up |
| `publisher` | Feed or track. Relationship rows include remote, publisher, and music feed references | App publisher search data represents a different contract | Packet 034 rules and bounded transport follow-up |
| `value_time_splits` | Track. Rows carry timing, remote feed/item references, and split | No matching track DTO field | Packet 034 contract boundary. Existing payment and broadcast rules remain in force |

Packet 017 cannot request these collections for durable use before their transport and coverage rules exist.
Their current absence in the app does not prove that the source supplies no facts.

### Summary Coverage

Upstream `FeedResponse.tracks` contains `TrackSummary` values with fewer fields than `TrackResponse`.
The app decodes both into `Track`. That shared type does not make their coverage equal.
Missing summary fields cannot replace detail observations or establish absent metadata.
Packet 017 must declare summary coverage. Packet 013 must reject deletion based on unsupported coverage.

## The Five Collection States

Decision 2 defines five collection states. The table below restates each
state's refresh effect and cites the current app behavior.

| State | ADR meaning | Refresh effect | Current app behavior | Case |
|---|---|---|---|---|
| Not requested | This request did not ask for the collection | Keep the stored snapshot | `fetch_index_track_detail` (search.rs:744) passes `None` for `include`, in both branches (lines 751 and 753). `persist_source_ids` (identity_ingest.rs:224) returns early on `None` and keeps the snapshot | [C11](adr-0075-metadata-example-corpus.md#c11) |
| Not returned | The app asked for the collection. The response omitted the field or returned `null` | Keep the snapshot. Record incomplete coverage | `Feed` and `Track` (api.rs:124, 157) decode a missing field and an explicit `null` to the same `None` value, through `#[serde(default)]`. `persist_source_ids` returns early on `None` here too, with no recorded coverage gap | [C12](adr-0075-metadata-example-corpus.md#c12) |
| Returned empty | A complete collection holds zero facts | Replace only that provider's collection with an empty snapshot | `persist_source_ids` (line 224) pre-seeds its group map with the key `musicindex` and an empty list. `replace_local_identity_ids` (db.rs:1435) deletes rows by owner and source only. It replaces only the `musicindex` group. Rows under another source label, such as `rss`, stay in place | [C13](adr-0075-metadata-example-corpus.md#c13), [C18c](adr-0075-metadata-example-corpus.md#c18c) |
| Returned populated | A complete collection holds facts | Replace only that provider's collection | `persist_source_ids` groups entries by each entry's own `source` field (`source_token`, identity_ingest.rs:510), not by the delivering provider. A provider that reports several `source` labels, such as `podcast_txt` and `podcast_person`, is replaced across several separate SQL calls, one call for each label | [C14](adr-0075-metadata-example-corpus.md#c14) |
| Request failed | No successful observation exists for this request | Keep the stored facts. Expose the failed refresh state | `response_text_with_status` (api.rs:755) returns an error for a status that is not success. `fetch_index_track_result_rows` (search.rs:659) calls `.ok()` on the detail result and discards that error. `enrich_track_context_from_rss` (subscribe_service.rs:552) discards its own result the same way, at line 562. No code records a failed request today | [C15](adr-0075-metadata-example-corpus.md#c15), [C16](adr-0075-metadata-example-corpus.md#c16) |

### Distinguish Not Requested From Not Returned

The [example corpus](adr-0075-metadata-example-corpus.md#unresolved-results)
assigns this open point to this packet. C11 and C12 decode to the identical
Rust value today. This section closes that point.

REQUIRED. Decision 2 separates these two states by request intent, not by
response shape. The app builds its own `include` string before it sends a
request. `fetch_feed`, `fetch_track` and `fetch_feed_track` (api.rs:501, 509,
517) each accept an `include: Option<&str>` value that the caller supplies.
A coverage record can compare that same value against each decoded `None`
field, with no API change:

1. When the sent `include` value omits the collection's token, or when
   `include` was `None`, record "not requested."
2. When the sent `include` value contains the collection's token, and the
   decoded field is `None`, record "not returned."

This rule needs no operator decision. It restates decision 2's own
definitions. No current code path performs this comparison.
`persist_source_ids`, `persist_source_links` and `persist_contributors`
(identity_ingest.rs:224, 190, 258) all return early on `None`, with no
distinction. Packets 011 and 017 must implement this comparison before a
schema can record five distinct states.

## The Malformed-Payload Rule

A malformed payload is a failed request. It is never an empty collection.

`response_json` (api.rs:738) returns a decoding error when the body does not
parse as valid JSON (line 747). A body such as `{ "track_guid": "track-1",`
fails this way.

A body that does parse as a JSON object, but with unexpected or missing
keys, decodes without an error. `Feed` and `Track` apply `#[serde(default)]`
to every field (api.rs:123, 156). An unexpected key is ignored. A missing
key becomes `None`. This second, malformed-but-parseable form looks the same
as an empty response today.

GAP. `fetch_index_track_result_rows` (search.rs:643) calls `.ok()` on the
detail result from `fetch_index_track_detail`, at line 659. It discards the
error. A failed request and an absent collection look the same at this call
site. `enrich_track_context_from_rss` (subscribe_service.rs:552) discards
its own result the same way, at line 562.

REQUIRED. Treat every decoding error, and every non-success HTTP status, as
"request failed." Keep the stored facts. Report the failed refresh. Do not
report an empty collection for either kind of failure. See
[C15](adr-0075-metadata-example-corpus.md#c15) and
[C16](adr-0075-metadata-example-corpus.md#c16).

## The Requested-Subject And Declared-Subject Rule

The requested subject of a fetch, and a returned fact's declared subject,
are separate fields. A track request does not authorize replacement of a
feed snapshot.

Upstream `get_effective_source_contributor_claims_for_track`
(stophammer db.rs:5556) reads the track's own claims first, at line 5561.
When the track has none, it returns the feed's claims instead, at lines
5563 and 5564, calling `get_source_contributor_claims_for_feed_entity`
(db.rs:5570). Each returned claim keeps its own `entity_type` and
`entity_id` (`SourceContributorClaimResponse`, query.rs:350-364). A track
request can therefore return entries that declare `entity_type` `feed`.

GAP. The app's `persist_contributors` (identity_ingest.rs:258) ignores each
entry's own declared owner. It stores every returned contributor under the
single `owner` value that the caller supplies for the whole request, at
line 283: `db::replace_local_contributors(conn, owner, MUSICINDEX_SOURCE,
&rows)`. `persist_musicindex_track` (identity_ingest.rs:65) always passes
`LocalEntityOwner::Track(track_id)` as that owner, at line 82. It does this
regardless of what any contributor entry declares.

`LocalEntityOwner` (db.rs:150) holds only `Feed` and `Track` variants, each
with one caller-supplied identifier. It has no per-row declared-owner field.
See [C18b](adr-0075-metadata-example-corpus.md#c18b).

REQUIRED. Do not use the request subject as the owner of every returned
fact. Read each entry's own `entity_type` and `entity_id` before choosing
which stored snapshot it may replace. The next section states this rule as
a transition table.

## The Complete Track Credit Transition Table

Apply this table only after the endpoint contract establishes complete coverage for the requested track and collection.
Array shape alone does not prove that prerequisite. Unknown or partial coverage preserves the earlier snapshot.

This table covers the sequence that decision 5 requires: a track's own
credits, then inherited feed credits, then no credits. Each step cites the
same upstream function, `get_effective_source_contributor_claims_for_track`
(db.rs:5556-5567), and its matching corpus case.

| Step | Case | Response shape | Complete collection | Facts the app may replace |
|---|---|---|---|---|
| 1. Track has its own credits | [C18a](adr-0075-metadata-example-corpus.md#c18a) | Every entry declares `entity_type` `track`. The track's own claim list is non-empty, so the upstream function returns it directly (line 5566) | The track's own `musicindex` `source_contributors` collection, for this track | The track's stored `musicindex` contributor rows. This response says nothing about the feed's own snapshot |
| 2. Track has no own credits. The feed has credits | [C18b](adr-0075-metadata-example-corpus.md#c18b) | Every entry declares `entity_type` `feed`. The track's own claim list is empty, so the upstream function substitutes the feed's claims (lines 5563-5564) | Zero entries declare `entity_type` `track`. This is a complete, empty observation of the track's own `musicindex` collection, counted by declared owner | The track's stored `musicindex` contributor rows, replaced with an empty snapshot. The feed-declared entries may not replace the feed's own stored snapshot. That needs its own feed-scoped fetch, per decision 5's separate-fetch rule |
| 3. Track has no credits. The feed has no credits | [C18c](adr-0075-metadata-example-corpus.md#c18c) | The array holds zero entries, of any declared owner. Both the track query and the feed-fallback query return empty results | A complete, empty observation of the track's own `musicindex` collection | The track's stored `musicindex` contributor rows, replaced with an empty snapshot. This step is silent about the feed's own snapshot, for the same reason as step 2 |

REQUIRED. Partition each response by each entry's own declared subject, not
by the request's target. Only track-declared entries may populate the track's own snapshot.
Verified complete absence can replace that snapshot with an empty one. Only a feed-declared entry, from a feed-scoped
fetch, may replace the feed's own stored snapshot. This restates decision
5's own instruction: "Do not use the request subject as the owner of every
returned fact."

OPEN. The read upstream code never returns entries with two different
declared owners in one `source_contributors` array for a track request. The
`if` and `else` branches at db.rs:5561-5567 each return one list or the
other. No read specification states this as a guaranteed contract for a
future or different deployed revision. Ask packet 008 to confirm this rule
before packet 013 assumes a response can never mix declared owners.

## Pagination Effect On Each Collection

`Pagination` (api.rs, lines 64-69) wraps only four types:
`SearchResponse`, `PublisherSearchResponse`, `TrackListResponse` and
`RecentFeedsResponse` (api.rs:29, 45, 52, 59). `Feed` and `Track` hold none
of the seven collections behind a `Pagination` value. Each collection is a
plain `Option<Vec<T>>` field.

OPEN. Upstream `openapi.rs` documents a `cursor` and a `limit` query
parameter for "included nested collections" on the feed and track detail
routes (openapi.rs:594-595, 669-671). `build_feed_response` (query.rs:661)
and `build_track_response` (query.rs:801) never read `params.cursor` or
`params.capped_limit()` inside any of their include branches (read lines
698-966). Each include branch runs one SQL query with no `LIMIT` clause and
no cursor filter.

At this revision, each returned collection is the complete set that SQL
query selects, when the response returns it at all. The documented cursor
parameter suggests a future or different deployed revision could still
truncate a large collection. Ask packet 008 to confirm whether that can
happen, and whether the app would receive any signal of it.

## Effect Of An Unsupported Include Value

Upstream `ListQuery::includes` (query.rs:95-99) checks an exact string
match, after a trim, against a comma-separated list. An unmatched token
produces no error. It simply leaves the matching response field unset.

Named example. The app's own `INDEX_FEED_DETAIL_INCLUDE` (search.rs:737)
requests the token `source_enclosures` on a feed detail fetch.
`FeedResponse` (query.rs:165-203) has no `source_enclosures` field.
`build_feed_response` (query.rs:661-696) has no matching
`if params.includes("source_enclosures")` branch.

Upstream's own feed-route documentation (openapi.rs:596) also omits
`source_enclosures` from its supported list. The request succeeds.
The token is silently ignored. No
error reaches the app.

REQUIRED. A caller must not infer, from a token's presence in a sent
`include` string, that the server returned that collection.
The decoded field's presence establishes only that some collection data returned.
Request intent distinguishes omitted requests. The endpoint contract must establish completeness before replacement.

## No Completeness Signal In A Success Status

A successful HTTP status does not, by itself, establish that a response is
complete. At commit `a220f44`, Stophammer's `src/api.rs`, `src/query.rs` and
`src/openapi.rs` define no completeness or coverage field. `TrackResponse`
(query.rs:228-269) and `FeedResponse` (query.rs:165-203) carry no such
field. `SourceContributorClaimResponse` (query.rs:350-364) and its sibling
response types carry no such field either.

REQUIRED. A 200 response tells the app only that the request succeeded. It
does not tell the app that every requested collection returned all its
facts. Decision 2 already states this.

The app derives request intent and returned shape from its request record and response.
It also needs the endpoint's completeness contract before authorizing replacement.
Unknown completeness keeps the stored snapshot. The HTTP status and array shape cannot prove complete coverage alone.

## Open Questions For Packet 008

| Question | Why it is open | Related case |
|---|---|---|
| Can a `source_contributors` array from a track request ever mix `entity_type` `track` and `entity_type` `feed` entries in one response? | Current code returns one list or the other, never both (db.rs:5556-5567). No specification confirms this for a future revision | [C18a](adr-0075-metadata-example-corpus.md#c18a), [C18b](adr-0075-metadata-example-corpus.md#c18b) |
| Can a nested collection in a detail response arrive truncated? The `cursor` and `limit` parameters are documented and unused today | `openapi.rs` documents the parameters (lines 594-595, 669-671). The handler code never reads them for a nested collection (query.rs:661, 801) | Not applicable |
| How should payment routes declare the owner of inherited rows? | Upstream payment rows have no owner marker. Enclosure ownership already exists upstream and needs app transport correction | Not applicable |
| How does the endpoint contract establish complete collection coverage? | No completeness field exists in the read Stophammer source | [C11](adr-0075-metadata-example-corpus.md#c11), [C12](adr-0075-metadata-example-corpus.md#c12) |

## Evidence Summary

| Rule | Evidence status |
|---|---|
| Endpoint and collection inventory | Named evidence for every collection |
| The five collection states | Named evidence for four states. The not-requested and not-returned distinction states a required mechanism. No code implements it yet |
| A malformed payload is a failed request | Named evidence |
| Requested subject and declared subject are separate fields | Named evidence |
| The track credit transition table | Named evidence for the response shape. The by-declared-subject replacement rule is required. No code implements it yet |
| Pagination effect on each collection | Named evidence. One open question sent to packet 008 |
| Effect of an unsupported include value | Named evidence, with a concrete example from the app's own code |
| No completeness signal in a success status | Named evidence |

## Retained Terms

The shared STE checker reports lexical findings for these technical names.
The [example corpus](adr-0075-metadata-example-corpus.md#retained-terms)
already retains several shared terms. This document adds the terms below.

| Retained term | Meaning in this document |
|---|---|
| include | The API query parameter that names each requested collection |
| token | One comma-separated value inside an `include` string |
| coverage | Short form of "coverage state," the ADR 0075 term for whether a request returned the complete collection |
| cursor | An opaque value that would select one page of a paginated collection |
| endpoint | One named HTTP route that the app calls |
| partition | To group entries by one shared field, such as a declared owner |
| gap | A named place where current code does not yet meet a required rule |

Do not replace a retained term with an unrelated dictionary alternative.

## Checks

Run the link check and the STE check from the repository root:

```bash
python3 docs/runbooks/check-markdown-links.py \
  docs/tasks/adr-0075-task-004-collection-completeness-rules.md \
  docs/schema/adr-0075-collection-completeness-rules.md
```

```bash
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" \
  --check --no-heuristics docs/schema/adr-0075-collection-completeness-rules.md
```

A person has not reviewed the rules above against the cited functions.
That review gate stays open. This document's mechanical checks do not
close it.
