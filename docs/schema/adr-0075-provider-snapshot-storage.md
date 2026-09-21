# ADR 0075 Provider Snapshot Storage

## Status And Authority

Technical design reviewed - 2026-09-20. Orchestrator review passed after corrections.
[Packet 011](../tasks/adr-0075-task-011-provider-snapshot-schema.md) owns this document.
This document changes no database or application behavior.

[ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) requires separate providers, declared owners, complete collection replacement, and retained raw evidence.
Decisions F and G require separate discrepancy evidence for descriptions and website/page links.
These requirements are accepted. The table design and transaction boundaries below are technical choices for review.

Field selection follows each field's accepted policy, outside this schema contract.
The operator must approve each remaining field policy before its implementation.
The operator accepted selection and absence rules separately for feed descriptions, track descriptions, feed websites, and track page links.
All four fields retain their last selected state when both providers expire, including selected absence.
Storage supports these decisions without accepting other fields' proposed selections.

The operator also accepted feed-artwork and track-artwork selection, explicit absence, and stale-state rules.
Selected artwork absence remains absent after expiry. Track-owned absence permits the accepted feed-artwork fallback without changing the image's owner.

The inspected app registry ends at migration 11, `broadcast_event_selection`.
The proposed migration is 12, `provider_metadata_snapshots`.
The design adds tables. It does not rewrite existing facts or configuration.

## Required Inputs

- [Collection completeness rules](adr-0075-collection-completeness-rules.md).
- [Field inventory](adr-0075-metadata-field-inventory.md).
- [Title, number, and classification rules](adr-0075-field-rules-titles-numbers-classification.md).
- [Aggregate and relationship rules](adr-0075-field-rules-aggregates-and-relationships.md).
- [Comparison and discrepancy contract](adr-0075-comparison-and-discrepancy-contract.md).

Proposed selection rules in these documents do not become accepted through a storage column.
Unknown values, unsupported syntax, and missing ownership evidence remain representable.

## Identity And Time

`ProviderKey` identifies the delivering provider, not a claim's `source` label.
MusicIndex uses its configured endpoint identity. Direct RSS uses its requested feed resource identity.
Use the existing endpoint parser's identity representation. Preserve the original request URI separately.
Do not apply field-level website comparison rules to provider identity.

A changed MusicIndex endpoint creates a different provider.
A redirect preserves the requested provider identity and records the final response URI.
Neither event transfers facts, resolves old discrepancies, or merges providers.

`SubjectKey` is a versioned JSON tuple with exact string components:

```text
[1, "feed", "guid", feed_guid]
[1, "track", "guid", feed_guid, track_guid]
[1, "feed", "resource", requested_feed_uri]
[1, "track", "resource", requested_feed_uri, item_guid]
```

Use the resource form only when a new observation establishes that feed resource as the owner.
It is not a guess about an old database row.
Do not merge resource and GUID subjects automatically when a later response supplies a GUID.
Retain their relationship as evidence until an identity rule authorizes the binding.

An item GUID without its feed scope cannot form a track key.
An uncertain owner has no resolved subject key. Its original owner fields remain in the fact record.

The requested subject and declared fact owner are different fields.
An unscoped track request can have an unresolved requested subject while its response provides a resolved declared owner.
Neither a request URL nor a local row ID can override contradictory declared ownership.

Store actual request start and response completion times as signed UTC microseconds.
Store supplied source times unchanged in JSON, including their original units or syntax.
A decoded source time is optional derived data. Missing source time stays unknown.
Local request generation is a strictly increasing database counter. It is not a source revision or timestamp.

Never infer freshness from that counter. Packet 018 owns freshness and cache ordering.

## Relational Schema

All names below are exact. `INTEGER` identifiers use SQLite integer primary keys unless a composite key is specified.
`TEXT JSON` means `TEXT NOT NULL CHECK(json_valid(column))`.
`TEXT JSON?` permits SQL `NULL`, with the same check for non-null values.

`?` marks a nullable column. Other columns are `NOT NULL`.
Every foreign key uses `ON DELETE RESTRICT`. No deletion cascades into evidence.

Use the connection's existing foreign-key enforcement.
Application validation enforces the cross-table conditions stated below within the same transaction.
Schema checks enforce the listed enum values, nonnegative counters, and unique keys.

Do not use a trigger to infer ownership, coverage, or discrepancy state.

### Provider, Resource, Subject, And Body Tables

| Table | Columns | Keys and checks |
|---|---|---|
| `metadata_providers` | `id INTEGER`, `kind TEXT`, `identity TEXT` | Primary key `id`. Unique `(kind, identity)`. Kind is `musicindex` or `rss`. Identity is nonempty. |
| `metadata_resources` | `id INTEGER`, `provider_id INTEGER`, `request_uri TEXT` | Primary key `id`. Provider FK. Unique `(provider_id, request_uri)`. URI is nonempty. |
| `metadata_subjects` | `id INTEGER`, `subject_key TEXT JSON`, `kind TEXT`, `feed_scope_kind TEXT`, `feed_scope TEXT`, `item_guid TEXT?` | Primary key `id`. Unique `subject_key`. Kind is `feed` or `track`. Nonempty scope uses kind `guid` or `resource`. Feed requires null item GUID. Track requires a nonempty item GUID. |
| `metadata_bodies` | `sha256 TEXT`, `byte_length INTEGER`, `bytes BLOB` | Primary key `sha256`. Hash is 64 lowercase hexadecimal characters. Length is nonnegative and equals `length(bytes)`. |

Validate that subject columns exactly reproduce the versioned subject key.
Compare existing body bytes before reusing a matching digest. A collision or inconsistent length is a storage error.
Keep response bytes before character decoding, JSON parsing, XML parsing, or source-text cleaning.
The body is the received entity body exposed by the HTTP boundary, not a reconstructed DTO.

Preserve its content-type and encoding metadata with the observation.

A single RSS body can serve many track observations and snapshots through the same body key.
Do not put a complete RSS document into each track fact's JSON.
Unknown JSON keys, namespace elements, attributes, and rejected syntax remain recoverable from the shared body.

### Request And Observation Tables

| Table | Columns | Keys and checks |
|---|---|---|
| `metadata_generation` | `singleton INTEGER`, `last_generation INTEGER` | Primary key `singleton`, equal to 1. Generation is nonnegative. Migration inserts `(1, 0)`. |
| `metadata_request_slots` | `request_key TEXT`, `provider_id INTEGER`, `resource_id INTEGER`, `requested_subject_id INTEGER?`, `requested_subject_json TEXT JSON`, `profile_json TEXT JSON`, `generation INTEGER`, `started_at_us INTEGER`, `state TEXT`, `latest_observation_id INTEGER?`, `latest_failure_id INTEGER?` | Primary key `request_key`. Provider, resource, subject, and observation FKs. Generation is positive. State is `pending`, `success`, `partial`, `failed`, or `abandoned`. |
| `metadata_observations` | The immutable and occurrence columns below. | Primary key `id`. Unique `observation_key`. Provider, resource, subject, and body FKs. Outcome is `success`, `partial`, or `failed`. Generations are positive. Counts are nonnegative, with occurrence count positive. |

Immutable observation columns:

```text
id INTEGER
observation_key TEXT
provider_id INTEGER
resource_id INTEGER
requested_subject_id INTEGER?
requested_subject_json TEXT JSON
profile_json TEXT JSON
body_sha256 TEXT?
http_status INTEGER?
response_uri TEXT?
interpretation_metadata_json TEXT JSON
source_revision_json TEXT JSON?
source_times_json TEXT JSON
contract_id TEXT?
decoder_version TEXT
outcome TEXT
failure_json TEXT JSON?
```

Occurrence columns:

```text
first_generation INTEGER
last_generation INTEGER
first_started_at_us INTEGER
last_started_at_us INTEGER
first_finished_at_us INTEGER
last_finished_at_us INTEGER
first_fetched_at_us INTEGER?
last_fetched_at_us INTEGER?
first_occurrence_metadata_json TEXT JSON
last_occurrence_metadata_json TEXT JSON
occurrence_count INTEGER
superseded_count INTEGER
```

`request_key` is a versioned digest of provider, resource, requested subject descriptor, and exact request profile.
The descriptor retains unresolved request parameters. Null subject IDs do not collapse different requests into one key.
`profile_json` retains endpoint shape, requested includes, pagination parameters, and the profile version.
The resource's provider must equal the request or observation provider.

`observation_key` adds response status, body key, final URI, stable interpretation metadata, contract, decoder, and outcome.
It also includes payload source revisions, payload source times, and structured failure evidence.
Exclude local generation, start time, finish time, fetch time, and occurrence counters from this digest.
Exclude all occurrence metadata, including HTTP dates, cache headers, ETags, and Last-Modified values.

Use a versioned, length-delimited canonical encoding. Do not concatenate strings without boundaries.
The writer compares all identity inputs before reusing a matching observation key.

Immutable observation fields describe the response and its interpretation.
Only the occurrence columns can change after insertion.
First and last refer to generation order, not wall-clock order.
Increment `superseded_count` once when an occurrence contains any rejected superseded collection, regardless of collection count.

Record a fetch time when a response completes. A failure without a response has no invented fetch time.
The failed request still retains its actual start and finish times.
Do not put local occurrence times inside the immutable failure identity.

### Stable Interpretation And Occurrence Metadata

`interpretation_metadata_json` has exactly these version-1 members:

| Member | Meaning |
|---|---|
| `version` | Integer 1. |
| `media_type` | Effective lowercase media type used by the decoder, or null. |
| `charset` | Effective canonical encoding name used for the retained bytes, or null. |
| `retained_body_codings` | Ordered lowercase content-coding tokens still applicable to the retained bytes. An empty array means none. |
| `body_state` | `complete`, `truncated`, or `absent`. |
| `effective_base_uri` | URI actually used to resolve relative source references, or null when no resolution occurs. |

These members describe decoder inputs, not a copy of all response headers.
Decoder version and contract ID already have separate columns in observation identity.
Derive this metadata through those existing decoder contracts. Do not add a new encoding-selection policy here.
A consumed header that changes interpretation must change a named decoder input or the versioned contract.
Header spelling, parameter spacing, and raw charset aliases remain occurrence evidence when they produce identical decoder inputs.

Each occurrence metadata JSON has version 1 and a `headers` object.
The object permits only the following lowercase header names:

```text
content-type, content-encoding, content-length, content-range,
content-location, location, date, age, cache-control, expires,
etag, last-modified, vary, warning, retry-after
```

An absent header has no object member. A present empty header retains an empty value.
Each member stores an ordered array of original header-value bytes encoded as Base64.
Do not parse a raw date or validator into another meaning during retention.
Do not copy authorization headers, cookies, or broadcaster secrets into this record.
Any additional contract header needs an explicit retention mapping before a decoder relies on it.

ETag and Last-Modified are transport revision and time evidence within the occurrence metadata.
Keep payload-supplied source revisions and source times in the immutable observation columns.
Header values cannot enter observation identity through `failure_json`, `source_revision_json`, or `source_times_json`.
An ETag does not prove revision order. A Date header is not the app's fetch time or a publisher assertion time.
An occurrence-only header change updates evidence without changing observation identity, even when its validator changes.

A truncated or rejected body cannot authorize collection replacement.

Retain the first and latest occurrence metadata by generation, alongside their actual occurrence times.
Count intermediate identical observations without appending an occurrence row.
This bounded summary is not a complete HTTP header log for every refresh.
Evidence that supports an accepted head, selected field state, or discrepancy transition freezes that occurrence's metadata separately.
Later updates to an observation summary cannot change those frozen copies.

Retain a failed response body when bytes exist, including malformed XML, malformed JSON, and non-success HTTP responses.
`latest_failure_id` remains separate from the latest successful collection head.
A later success can clear the active failure state without deleting the previous failure observation.

### Coverage And Fact Tables

| Table | Columns | Keys and checks |
|---|---|---|
| `metadata_coverage` | `observation_id INTEGER`, `scope_ordinal INTEGER`, `collection TEXT`, `target_subject_id INTEGER?`, `target_owner_json TEXT JSON`, `request_intent TEXT`, `presence TEXT`, `completeness TEXT`, `contract_id TEXT?`, `basis_json TEXT JSON` | Primary key `(observation_id, scope_ordinal)`. Observation and subject FKs. Ordinal is nonnegative. Intent is `requested`, `implicit`, or `not_requested`. Presence is `missing`, `null`, `empty`, `populated`, or `invalid`. Completeness is `complete`, `partial`, `unknown`, or `failed`. |
| `metadata_facts` | `id INTEGER`, `observation_id INTEGER`, `scope_ordinal INTEGER`, `transport_ordinal INTEGER`, `declared_subject_id INTEGER?`, `declared_owner_json TEXT JSON`, `owner_basis_json TEXT JSON`, `fact_kind TEXT`, `assertion_source TEXT?`, `source_position INTEGER?`, `extraction_path TEXT?`, `source_observed_json TEXT JSON?`, `representation TEXT`, `validation TEXT`, `value_json TEXT JSON`, `raw_member_json TEXT JSON?`, `body_locator_json TEXT JSON` | Primary key `id`. Composite coverage FK and subject FK. Unique `(observation_id, scope_ordinal, transport_ordinal)`. Ordinal is nonnegative. Representation is `plain_text`, `html`, `structured`, or `unknown`. Validation is `valid`, `unsupported`, `malformed`, or `unresolved`. |

Use separate coverage rows for distinct declared subjects within one parsed RSS feed.
An unknown target remains evidence only. It cannot be a replacement target.
Collection tokens identify actual collections, or one scalar field through `field:<wire-path>`.
Unknown collection tokens are retained but cannot authorize replacement without a registered contract.
Summary membership coverage never proves complete track-detail coverage.

`basis_json` records the verified response contract, pagination evidence, and any owner-partition rule used for completeness.
It also records why coverage is partial, unknown, or failed.
Only a verified contract can produce `complete` coverage.

An include token and `Some(Vec::new())` are insufficient evidence by themselves.
Distinguish a missing property from explicit null in retained presence, even when both prevent replacement.

`transport_ordinal` records location within this response. It does not replace a supplied `position` claim.
`declared_owner_json` retains original `entity_type` and `entity_id`, including missing or malformed values.
`owner_basis_json` records explicit row ownership, a verified envelope contract, or unresolved ownership.
Never manufacture per-member owner fields that the wire response omitted.

Contributor, enclosure, transcript, link, and identity facts retain every supplied claim field.
Preserve source, extraction path, source observation time, supplied position, declared type, and declared ID separately from local transport position.
Keep contributor `role_norm` in its structured value with its original `role`.
Keep transcript MIME type, language, relation, and URL without substituting an enclosure contract.

Unknown member properties remain in `raw_member_json` for JSON responses.
Take that member from the original parsed JSON, never from a DTO serialized after unknown properties were discarded.
RSS facts use body locators and typed values. Raw XML remains in `metadata_bodies`.

Unsupported or malformed `podcast:txt` stays a fact with its validation result.
It cannot enter an active `SourceEntityId` projection through deserialization, restart, or fallback.
General namespace evidence, platform claims, remote items, publisher relationships, and value time splits use structured fact values.
Their retention does not change actions, payment routes, or broadcast behavior.
Field representations remain explicit. Storage never reparses Index plain text as HTML.

### Snapshot And Head Tables

| Table | Columns | Keys and checks |
|---|---|---|
| `metadata_snapshots` | `id INTEGER`, `provider_id INTEGER`, `subject_id INTEGER`, `collection TEXT`, `content_key TEXT`, `first_observation_id INTEGER`, `first_scope_ordinal INTEGER`, `member_count INTEGER` | Primary key `id`. Provider and subject FKs. Composite coverage FK. Unique `(provider_id, subject_id, collection, content_key)`. Count is nonnegative. |
| `metadata_snapshot_members` | `snapshot_id INTEGER`, `member_ordinal INTEGER`, `fact_id INTEGER` | Primary key `(snapshot_id, member_ordinal)`. Snapshot and fact FKs. Unique `(snapshot_id, fact_id)`. Ordinal is nonnegative. |
| `metadata_collection_heads` | `provider_id INTEGER`, `subject_id INTEGER`, `collection TEXT`, `snapshot_id INTEGER?`, `accepted_generation INTEGER`, `accepted_observation_id INTEGER?`, `accepted_scope_ordinal INTEGER?`, `accepted_fetched_at_us INTEGER?`, `accepted_occurrence_metadata_json TEXT JSON?`, `last_attempt_generation INTEGER`, `last_attempt_state TEXT`, `last_attempt_observation_id INTEGER?`, `latest_failure_id INTEGER?` | Primary key `(provider_id, subject_id, collection)`. Provider, subject, snapshot, observation, and composite coverage FKs. Generations are nonnegative. Attempt state is `none`, `pending`, `success`, `partial`, `failed`, `superseded`, or `abandoned`. |

A head with no snapshot has no accepted complete observation.
An empty snapshot has zero members and complete coverage. It is different from a missing head or null snapshot.
Require accepted observation and scope together whenever a snapshot exists.
Its provider, subject, collection, and coverage must match the head.

Require the accepted occurrence metadata when a snapshot exists. Otherwise, that metadata remains null.

The snapshot's member count must match its member rows.
Every member needs the same proven declared owner as the snapshot.

`content_key` covers the complete ordered member evidence, ownership basis, collection, and verified coverage contract.
It excludes local fetch time, generation, observation row IDs, and unrelated response fields.
A repeated complete collection can reuse its snapshot while its head references a newer successful observation.
The first coverage reference preserves the original snapshot evidence.

An assertion's `source` does not appear in the head key.
Replace all source labels supplied by one provider's complete collection together.
The replacement cannot delete another provider's facts.

### Selected Field State

| Table | Columns | Keys and checks |
|---|---|---|
| `metadata_field_selections` | `subject_id INTEGER`, `field TEXT`, `context_key TEXT`, `context_json TEXT JSON`, `selected_provider_id INTEGER`, `selected_observation_id INTEGER`, `selected_scope_ordinal INTEGER`, `selected_snapshot_id INTEGER`, `selection_state TEXT`, `value_json TEXT JSON?`, `evidence_json TEXT JSON`, `policy_version TEXT`, `selected_at_us INTEGER` | Primary key `(subject_id, field, context_key)`. Subject, provider, snapshot, and composite coverage FKs. State is `value` or `absent`. Field is a nonempty registered field key. Value requires non-null value JSON. Absence requires SQL null. |

The selected-state schema has no fixed field-name enum.
Initial selector keys include `description`, `website_page`, and `artwork`, each with its feed or track subject.
Feed and proven track publisher text also have accepted retention rules, using `publisher_text` with the corresponding subject.
The typed policy registry rejects unregistered fields and unsupported owner combinations before a write.
A future accepted selector can add a key without changing this table's field constraint.
This extensibility does not accept that future field's policy.

`context_key` is the versioned canonical key for the considered provider identities and resources.
`context_json` preserves that context, including unavailable providers. A changed endpoint cannot reuse the old selection accidentally.
`selected_provider_id` identifies the provider whose value or verified absence supplied the selected state.
The selected observation, coverage, snapshot, and subject must agree.
`evidence_json` freezes its actual occurrence, response metadata, ownership, coverage proof, and source evidence.

This table preserves the last selected field state. An absent state is not a missing row.
Expiry can mark a retained selection stale without changing an absent state into a value.
The shared projection derives freshness from actual recorded observations under the accepted field policy.
It does not replace selected absence with an older value merely because both providers expired.
No stored fresh flag can bypass restart freshness checks.

Only a registered, accepted field policy can update this table.
Validate its input heads and write the selected state in one transaction.
The table does not authorize a source priority, claim order, website action, or policy for another field.
Unsupported fields stay outside this selector until their own policies pass review.

For track artwork, retain the track's own selected state separately from the feed's selected artwork state.
A track-owned `absent` row must not point at a feed-owned snapshot or store the feed image as its value.
The shared projection reads the separate feed artwork selection when the accepted fallback applies.
Its typed result retains the feed owner and the track's own absence evidence.
Retained track absence cannot become an older track image after expiry or restart.
The feed fallback retains its own freshness and absence evidence.

### Discrepancy Tables

| Table | Columns | Keys and checks |
|---|---|---|
| `metadata_discrepancies` | `id INTEGER`, `subject_id INTEGER`, `field TEXT`, `rss_provider_id INTEGER`, `rss_resource_id INTEGER`, `index_provider_id INTEGER`, `index_resource_id INTEGER`, `state TEXT`, `first_recorded_at_us INTEGER`, `last_recorded_at_us INTEGER`, `last_compared_json TEXT JSON`, `latest_transition_id INTEGER?` | Primary key `id`. Subject, provider, resource, and transition FKs. Unique `(subject_id, field, rss_provider_id, rss_resource_id, index_provider_id, index_resource_id)`. State is `active` or `resolved`. Field is `description` or `website_page`. |
| `metadata_discrepancy_transitions` | `id INTEGER`, `discrepancy_id INTEGER`, `sequence INTEGER`, `previous_state TEXT?`, `state TEXT`, `reason TEXT`, `comparison_version TEXT`, `pair_content_key TEXT`, `rss_observation_id INTEGER`, `index_observation_id INTEGER`, `rss_body_sha256 TEXT?`, `index_body_sha256 TEXT?`, `rss_evidence_json TEXT JSON`, `index_evidence_json TEXT JSON`, `recorded_at_us INTEGER` | Primary key `id`. Discrepancy, observation, and body FKs. Unique `(discrepancy_id, sequence)`. Sequence is positive. States are `active` or `resolved`. Reason is `mismatch`, `changed_mismatch`, `resolved`, or `reactivated`. |

Insert a discrepancy and its first transition in one transaction, then set the head's transition reference.
Validate provider kinds and transition ownership before commit.
Require each resource's provider to equal its corresponding provider ID in the discrepancy key.
The compared observations must match both providers and both resources in that key.
Different Index resources under one endpoint produce separate discrepancies. One resource's agreement cannot resolve another resource's mismatch.

The field's feed or track meaning comes from its subject key.
`comparison_version` is never part of stable discrepancy identity.

Artwork selected-state storage does not add artwork to discrepancy comparison.
The accepted description and website/page discrepancy fields remain unchanged.

Each evidence JSON has a versioned structure with these required members:

```text
coverage: known_value | known_absent
subject_key, provider_key, request_resource, response_resource
observation_id, generation, requested_subject, declared_owner, owner_basis
original_value: JSON value | null
representation, assertion_source, extraction_path
source_observed, source_revision, source_times
fetched_at_us, occurrence_metadata, comparison_value, coverage_basis
```

`original_value` is null only for explicit absence or an original JSON null whose contract defines absence.
Unknown coverage cannot create a transition.
Source fields can contain explicit unknown values. Do not insert fabricated source times or paths.

`source_revision` and `source_times` contain payload assertions. HTTP validators remain in `occurrence_metadata`.
The evidence freezes exact occurrence times and HTTP metadata, including validators, before its observation summary receives later updates.
Body references and immutable evidence survive snapshot replacement.

`last_compared_json` retains the most recent comparable pair's version, generations, actual times, and both occurrence metadata objects.
Identical content updates this summary without another transition.
`pair_content_key` includes normalized results, original values, ownership, assertion evidence, coverage, and comparison version.
It excludes local occurrence times and all occurrence metadata. A Date, ETag, or Last-Modified change alone cannot add a transition.

Check the current pair when suppressing repetition. Do not make this key globally unique.
A previous mismatch can recur after resolution and requires a new transition.

A comparison-version change alone causes no transition.
A new successful comparable pair can change the existing discrepancy under the new version.
Unknown coverage, stale inputs, or failed comparison cannot create or resolve a discrepancy.
The caller must establish freshness under packet 018 before requesting a transition.
The transaction must verify both accepted heads against the compared resources, observation IDs, snapshots, and accepted generations.
An observation ID alone cannot identify an occurrence because identical responses share that ID.

## Typed Application Boundary

Keep API DTOs free of application observation state.
Use an application-owned `ProviderObservation` with shared `Arc<[u8]>` body bytes and typed evidence.
Packet 009's RSS carrier remains usable. Packet 014 supplies its durable storage adapter.
The exact Rust module split belongs to implementation packets, but these boundary operations are required:

| Operation | Required input and result |
|---|---|
| `begin_provider_request` | Provider, resource, requested subject descriptor, profile, and actual start time. Returns a durable `RequestToken` with generation. |
| `record_provider_observation` | Consumes the token and a typed success, partial, or failure observation. Returns an `ObservationReceipt` and per-collection application outcomes. |
| `read_provider_collection` | Provider, resolved subject, and collection. Returns no snapshot, complete empty, or complete populated, plus separate refresh evidence. |
| `record_field_selection` | Accepted field policy, current input heads, provider context, and selected value or absence. Updates the retained state atomically. |
| `read_field_selection` | Subject, field, and provider context. Returns no selection, selected value, or selected absence, with recorded evidence. |
| `record_discrepancy_comparison` | Current comparable observations, frozen value/absence evidence, comparison version, and result. Returns retained, created, changed, resolved, or reactivated. |

`ObservationReceipt` is `#[must_use]` and contains the persisted observation ID and every collection outcome.
It also retains that occurrence's generation, resources, body key, actual times, and response metadata.
Build pinned evidence from that receipt or the accepted head, not a later first/latest observation summary.

A dropped transport result must not silently mean empty coverage.
Consumers explicitly handle `KeptUnknown`, `KeptPartial`, `KeptFailed`, `RejectedSuperseded`, `ReplacedEmpty`, `ReplacedPopulated`, and `Repeated`.
Storage errors return failure. They cannot report successful replacement.

A failed observation write returns its request token and observation in a typed failure.
The Library command retains that failure through the command error channel before converting its message for display.
The Library view model keeps failed capsules by request generation for the current session, including failures from previously selected tracks.
Navigation and later failures do not discard earlier capsules. Session teardown ends this in-memory retention without claiming persistence.

Debug output and status text must not include response bodies or raw provider values.
The existing Library error status reports the storage failure. Packet 019 supplies the separate explicit storage-retry action.
Retry uses the retained token and observation. It does not refetch the response or allocate a new request generation.

The observation input retains request intent, raw property presence, contract proof, and typed validation separately.
Constructing a complete observation requires a verified collection contract, not merely a DTO vector.
A successful RSS parse without the requested item records `partial` with a no-match reason.
It preserves the feed body and matching evidence without declaring the requested track's collections empty.
Any separate complete feed coverage needs its own explicit declared-owner proof.

## Request, Replacement, And Restart Transitions

1. Allocate the next generation and update the request slot in one transaction before network work starts.
2. Record the response body and observation before any cleaning or fallback can discard evidence.
3. Validate each coverage row and declared owner against the registered contract.
4. Begin one database transaction for the response's snapshot changes and receipt.
5. Keep incomplete, failed, and unresolved collections unchanged.
6. Reject a collection result whose generation is older than its accepted generation.
7. Create or reuse each complete snapshot, including a zero-member snapshot.
8. Replace its head and accepted occurrence fields atomically.
9. Update refresh state only when the attempt generation equals or exceeds the recorded attempt generation.
10. Commit all accepted collection changes together. Return every retained or rejected outcome.

Steps 2 through 10 form one durable transaction when an observation changes stored state.
No committed observation can falsely claim that only part of its planned replacement succeeded.
On failure, SQLite rolls back the observation writes and all head changes.
The caller retains the in-memory response for an explicit storage retry.

A newer failed request does not delete the last successful snapshot.
Generation ordering is independent of source timestamp order.
Packet 018 can reject additional cache results before this boundary, but cannot permit accepted-generation regression.
Older results retain their observation evidence and increment the superseded occurrence count.
They cannot replace a newer accepted head or overwrite a newer failure report.

On restart, read the persisted generation before issuing requests.
Mark a retained `pending` slot as `abandoned` only after proving that its process no longer owns the request.
Opening another connection or process does not supply that proof. The desktop app and CLI can use the same database.
Keep their previous success and failure references. Do not infer an empty response.

Packet 014 retains pending slots across reopen without automatic abandonment.
Packet 018 requires a reviewed process-ownership contract before implementing restart abandonment.
Until that proof exists, a new request can advance its slot generation without reclassifying other pending requests.
A restored backup starts a new process session with no old callbacks.
Any running session must drain before database restoration under the existing maintenance contract.

### Complete Track Credit Example

| Response for track `T` in feed `F` | Stored effect |
|---|---|
| Complete track-owned contributors | Replace only `(provider, F/T, source_contributors)` with those facts. |
| Complete response containing only inherited feed-owned contributors | Store the feed-owned observations. Replace the track's own collection with complete empty only under the verified fallback contract. |
| Complete empty contributors | Keep a complete empty track-owned snapshot. Do not clear the feed snapshot. |
| Partial, unknown, mixed-owner, omitted, failed, or unsupported response | Preserve prior track and feed snapshots. Retain coverage and evidence. |

The inherited response cannot replace the feed's authoritative snapshot.
A separate complete feed request is required for that operation.
Feed-owned facts remain reachable through the observation receipt without being promoted into track ownership.
Payment routes with unknown inherited ownership remain evidence under their existing behavior.

## Retention And Repeated Observations

Identical response bytes reuse one body, even across many requested tracks.
Identical observation identity reuses one observation and its fact rows.
Identical complete collection content reuses one snapshot and its members.
Identical consecutive discrepancy content updates its latest occurrence summary without adding history.
Tests must prove stable row counts after repeated identical refreshes.
Actual first/last times and occurrence counters still update.

Date, Age, cache metadata, validators, and other occurrence-only header changes preserve these row-count guarantees.
The guarantee excludes changed payload source times, changed response bytes, changed stable decoder inputs, or changed interpretation contracts.
Those changes can produce different evidence and new observation rows.
No retention limit or history expiry is introduced here.
The initial implementation performs no garbage collection of committed observations, snapshots, or discrepancy transitions.

If a later maintenance rule adds garbage collection, it can delete only data unreachable from all retained evidence roots.
Roots include collection heads, retained field selections, request success/failure references, retained snapshots, and every discrepancy transition.
Body deletion additionally requires zero observation and transition references within the same transaction.
Deleting old discrepancy evidence or retaining only the current mismatch requires a separate accepted rule.

## Migration 12 And Preservation

Migration 12 creates the tables, indexes, and generation singleton in one transaction with its ledger row.
It does not copy legacy facts into provider snapshots.
Legacy `source = rss` does not prove delivery by direct RSS rather than MusicIndex.
Legacy local ownership also does not prove the original declared owner.
Keep those rows unchanged and expose their provenance as unresolved when later readers inspect them.

Do not remove identity records, contributor rows, metadata rows, or ledger entries to prepare this migration.
Legacy cleanup needs its own evidence report and accepted repair rules.
Migration 12 does not select field values or repopulate legacy scalar columns.

### Existing Migration Couplings

The current [upgrade recognizer](../../src/db/upgrades.rs) creates the latest complete schema to recognize interrupted migration 11.
Adding migration 12 would make a valid interrupted-11 database fail that comparison.
The current [restore owner](../../src/db/maintenance/restore.rs) also calls `recognize_migration_11` to validate every upgraded candidate.
Its interrupted-11 preservation digest omits ledger row 11 but still hashes all schema objects.
Applying migration 12 inside that repair would fail the existing preservation comparison.

Use one normal registry with an explicit maximum target version.
Build expected version 11 through that registry, without migration 12.
Keep migration 12's tables out of `init_schema` so the base initializer cannot contaminate the version-11 authority.
The initializer also currently creates migration 11's selection table before the registry runs.
Move that creation to its existing migration-11 registry entry so version-10 interruption fixtures can use the same authority.

Keep version-11 column requirements separate from the latest column requirements.
Do not copy migration SQL into a second recognizer or fixture authority.

`recognize_migration_11` must compare against the version-11 schema only.
Retain its existing exact-object checks, allowed legacy column order, ledger-prefix checks, and selection-row validation.
A separate current-schema validator must compare candidates against the registry's requested target version.
`restore::migrate_candidate` must use that validator for its requested target version.

Carry the target version through `ValidatedRestore` and `verify_installed`.
That installation check currently requires the latest `Current` schema and `DatabaseReadiness::Ready`.
For a repair targeting 11, validate exact version 11 and a usable connection without claiming version-12 readiness.
Keep ordinary current-version restore checks unchanged.

Interrupted-11 repair must apply and verify only migration 11 first.
Its preservation digest remains valid because no migration-12 object enters that repair.
Under a version-12 binary, successful repair then yields `UpgradeRequired`, not normal-session readiness.
The normal preparation path must perform the separately backed-up migration-12 step before installing a normal session.
Repair reports must distinguish repaired version 11 from a database ready for the current application.
Update `src/view_models/startup/database.rs` and its command result so `InstallState::Verified` does not imply current-version readiness by itself.

For migration 12, apply DDL and insert its ledger row inside the same transaction.
Place failure-injection boundaries before DDL, after DDL, and after ledger insertion but before commit.
Verify the target schema, integrity, foreign keys, and version-11 record digest before committing that transaction.
Every injected failure leaves version 11 without any migration-12 table or ledger row.
An uncommitted version-12 transaction must also roll back after process interruption.
A version-12 table without its ledger row is unknown schema, not a new automatic repair case.

The current debug interruption fixture removes migration-11 objects and its ledger row.
Replace that setup with a disposable database built through migration 10 and stopped at the real migration-11 boundary.
Do not modify production data or delete a ledger entry to simulate an older version.

### Backup Before Existing-Database Upgrade

The current [startup preparation](../../src/db/startup.rs) and `db::open_db` migrate without a preservation snapshot.
The [CLI database opener](../../src/cli.rs) also reaches `db::open_db`.
Both routes must use one guarded existing-database preparation owner before migration 12 can ship.
Secondary actor connections through `open_existing` remain non-migrating.

Use [database maintenance](../../src/db/maintenance.rs) and its [exclusive preservation owner](../../src/db/maintenance/preservation.rs).
Do not make a raw copy of a live SQLite main file without its existing preservation contract.
No new configuration key or independent backup engine is needed.

1. Close the preparation probe connection before acquiring exclusive database access.
2. Acquire the existing bounded exclusive access while the normal session is absent or drained.
3. Inspect schema, integrity, and foreign keys before selecting the supported upgrade path.
4. Create a private, uniquely named preservation directory beside the database.
5. Preserve the original files and manifest through `ExclusiveDatabase::preserve`.
6. Create and verify a standalone SQLite snapshot through the existing snapshot builder.
7. Sync the snapshot and preservation directory before any migration DDL.
8. Apply the registry's required migrations while exclusive access remains held.
9. Verify the committed target and recorded preservation result before releasing access.
10. Reopen through startup validation. Return readiness only for the current schema.

The shared owner must return the preservation paths and actual recorded times for the existing preparation report.
Preservation, disk-space, cancellation, lock, and validation failures stop migration before DDL.
A new empty database needs no original-data backup.
Earlier supported database versions also receive preservation before any remaining migration runs.
An interrupted-11 database keeps its existing explicit repair route.

Earlier migrations keep their existing approved transformations. Capture the unchanged-record baseline after version 11 and before migration 12.

Existing direct `open_db` callers must not bypass this owner through `init_schema` on a populated database.
Tests can build disposable versioned fixtures through the internal registry without production preparation or backup files.
That fixture API is test/debug scoped and cannot become an application migration bypass.

### Rollback And Fixture Preservation

For a migration-12 failure before commit, verify the original ledger, schema, and legacy record digest.
Do not claim rollback from an error alone.
Keep preservation artifacts when verification fails or remains incomplete.
The existing restore installer already distinguishes verified rollback from failed verification.

Verification after commit cannot claim transaction rollback. Preserve the database and report that restoration remains necessary.

A completed version-12 upgrade is not undone by deleting ledger row 12 or dropping new tables.
Use the verified pre-upgrade snapshot and the existing reviewed restore workflow with a compatible version-11 binary.
Preserve the version-12 database first. Drain all database users before replacement.
The current binary's `upgrade_backup` action upgrades an old backup again. It is not a downgrade operation.

This design adds no automatic downgrade command and makes no claim that the current restore screen accepts older schemas.

Fixture tests must preserve every existing table, row identity, and value across migration 12.
Include contributor and enclosure claim JSON, transcripts where retained, unresolved identity rows, and metadata facts.
Also include local file paths and checksums, playlists, playback sessions, broadcasts, and `broadcast_event_selection`.
Include revision 7 for the selected event to detect accidental initialization.
Configuration, audio files, tags, and broadcaster secret files remain outside migration writes.

Use disposable DELETE-mode and WAL-mode databases for preservation and failure tests.
The verified backup must contain the original rows and the version-11 ledger.
Test a failed preservation step, failed migration step, interrupted transaction, failed verification, and verified rollback.
Fixture cleanup removes only paths created by the test. No production database is a test input.

## Reachable Implementation Owners

| Later work | Existing composition roots and required integration |
|---|---|
| Migration and preparation | `src/db.rs::{open_db, inspect_schema, migrate_schema_with}`, `src/db/startup.rs::prepare_database`, and `src/cli.rs::open_configured_db`. Wire the guarded preparation owner into both startup and CLI. |
| Versioned repair and restore | `src/db/upgrades.rs::{recognize_migration_11, interrupt_fixture}` and `src/db/maintenance/restore.rs::{repair_interrupted_upgrade, migrate_candidate, verify_installed}`. Carry the target through `src/application/commands/maintenance.rs` and `src/view_models/startup/database.rs`. |
| Provider snapshot writes | `src/identity_ingest.rs::{persist_musicindex_context_by_feed_url, persist_musicindex_feed, persist_musicindex_track}`. Replace source-label grouping only when observed request envelopes reach these calls. |
| Direct RSS observation writes | `src/rss/subscribe.rs::{subscribe_feed, persist_rss_feed_identity, persist_rss_track_identity}` and `src/rss/enrich.rs::fetch_track_enrichment_from_feed`. Reuse the parsed observation and shared body. |
| Request propagation | `src/subscribe_service.rs`, `src/feed_service.rs`, `src/application/queries/feed.rs`, and `src/application/queries/library.rs`. Carry observations through `TrackContext` and handle receipts at persistence calls. |
| Durable reads and comparison | Packet 020's shared metadata projection and packet 036's discrepancy owner consume current heads plus retained evidence. A renderer must not reconstruct provider ownership. |

Packets 012, 013, and 014 separate migration, replacement, and raw observation integration.
Delivery order is 012, 014, then 013.
Packet 014 connects observation retention to the Library track-detail caller without replacing collection heads.
Packets 038–043 extend that writer to the remaining caller families in the phase plan.

Packet 013 extends that same writer with verified complete-collection replacement inside the observation transaction.
It supplies the minimum completeness contracts and connects local reads. Packet 017 reuses those contracts for named request profiles.

Packets 017 and 018 supply named request profiles and freshness.
No packet is complete with an unused storage abstraction or tests as its only caller.
Migration code is reached through database preparation. Writers and readers require the live roots listed above.

## Required Mechanical Cases

| Case | Required proof |
|---|---|
| S11-01 | Migration 12 preserves all version-11 rows and adds only its schema and ledger entry. |
| S11-02 | Valid interrupted migration 11 remains recognized after 12 enters the registry. Unsupported schema remains rejected without writes. |
| S11-03 | Interrupted-11 repair verifies version 11 first. Preparation then performs a separately backed-up upgrade to 12. |
| S11-04 | Failure at each migration-12 boundary leaves no partial schema or ledger entry. |
| S11-05 | Backup or preservation failure prevents migration. SQLite snapshots include committed WAL data. |
| S11-06 | Contributor, enclosure, and transcript claim fields survive write, restart, and read. Unknown member fields remain recoverable. |
| S11-07 | Two feeds with the same item GUID retain different track subjects. Unknown legacy ownership remains unresolved. |
| S11-08 | Complete own-track, inherited-feed, and empty-credit responses follow the transition table without deleting a feed snapshot. |
| S11-09 | Missing, null, summary, partial, malformed, failed, and unsupported responses cannot clear complete snapshots. |
| S11-10 | A provider response replaces its complete collection across assertion-source labels in one transaction. |
| S11-11 | Older generations cannot replace newer accepted heads. Reopening retains pending requests and advances generations. Abandonment requires process-ownership proof. |
| S11-12 | Endpoint changes isolate new snapshots and discrepancies while retaining the old provider evidence. |
| S11-13 | One RSS body referenced by many tracks occupies one body row. Identical refreshes keep body, fact, snapshot, and history row counts stable. |
| S11-14 | Failed response bytes remain reachable independently from the last successful snapshot. |
| S11-15 | Changed mismatch, resolution, reactivation, and repeated mismatch preserve the required evidence after snapshot replacement. |
| S11-16 | Comparison-version changes retain one discrepancy identity and cannot change state without a new comparable pair. |
| S11-17 | Known absence round-trips distinctly from unknown coverage without accepting an unreviewed display fallback. |
| S11-18 | Unsupported RSS identity syntax remains evidence after restart and never becomes an active entity ID. |
| S11-19 | Selected absence survives expiry and restart for all four accepted fields. Older values cannot return through stale fallback. |
| S11-20 | Feed-artwork and track-artwork value and absence states survive restart. Track absence uses separate feed-owned fallback evidence without restoring removed track artwork. |
| S11-21 | Repeated bodies with changed Date, Age, ETag, Last-Modified, and cache headers keep observation and history row counts stable. First/latest occurrence evidence updates. |
| S11-22 | A retained discrepancy transition keeps its original headers and occurrence times after later identical fetches update observation summaries. |
| S11-23 | Two Index resources under one endpoint retain different discrepancy keys. Agreement for one resource cannot resolve the other's mismatch. |

## Open Product Policies And Technical Review

Open product policies include each unaccepted field selection, remaining absence behavior, and packet 018's freshness limits.
Relationship actions, expanded discrepancy fields, evidence expiry, and any MusicIndex update hook are outside this design.
None is required to preserve source observations accurately.

Root technical review must confirm the schema, canonical identity encoding, transaction scope, versioned migration authority, and preservation integration.
No migration packet may run before that review passes.

## Operator Visual Check

This document changes no presentation and requests no app launch.
The existing visual pause remains in force. Later presentation changes keep their separate acceptance gates.
