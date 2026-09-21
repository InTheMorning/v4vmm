# ADR 0075 Task 014: Provider Observation Retention

Status: Implementation, technical review, and mechanical checks complete - 2026-09-21. The presentation gate remains open and paused.

Assigned implementation agent: `provider_retention_014`. The orchestrator records shared status and reviews the completed diff.

The orchestrator approved the first slice and its Library storage-failure owner.
This packet does not complete observation retention for every caller.

## Goal

Retain original MusicIndex and RSS observations from the live `FetchLibraryTrackContext` command.
Allocate durable request generations before network work. Store response evidence before cleaning, source fallback, or local fallback can discard it.
Return consumed receipts that identify persisted observations and their actual occurrence evidence.

The first slice includes the command's scoped track request, unscoped fallback, feed request, and RSS enrichment request.
The shared writer remains available for later caller conversions.
No collection head, selected field, discrepancy, or legacy source fact changes in this packet.

## Authority And Scope

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), provider ownership, explicit coverage, raw evidence, and separate refresh failure.
- [Storage design](../schema/adr-0075-provider-snapshot-storage.md), exact schema and transaction boundaries.
- [Packet 009](adr-0075-task-009-rss-owner-and-nostr-extraction.md), the existing RSS observation and single document parse.
- [Packet 011](adr-0075-task-011-provider-snapshot-schema.md), reviewed storage design.
- [Packet 012](adr-0075-task-012-provider-snapshot-migration.md), completed schema-12 implementation and technical checks.
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md#phase-003-storage), storage order 012, 014, then 013.

Packet 013 later extends this writer's transaction with verified complete snapshot replacement and local reads.
It owns the minimum collection-contract registry. Packet 017 later adds named request profiles through that registry.
This packet records unknown, partial, and failed coverage. It cannot establish complete coverage through DTO shape or include strings.

The operator requires a separate answer for each field policy.
Raw retention accepts no additional source preference, removal policy, field selection, display fallback, or URL action.
Existing display behavior stays unchanged. A retained transport failure can still use the existing local fallback.

The orchestrator approved this smaller slice after inspection found several independent caller families.
The [follow-up table](#required-caller-follow-ups) records the excluded families and their dependencies.

## Current Live Path

`FetchLibraryTrackContext::execute` already has the shared database connection and runs through the existing command worker.
It calls `fetch_library_track_context_with_local_fallback`, which reads local facts and then requests remote context.
That function currently converts every remote error into the local result.

`feed_service::fetch_library_track_detail` makes a scoped track request when a feed GUID exists.
Its `.ok()` calls discard failures before an unscoped fallback and a separate feed request.
The helper returns only DTOs. Their response bytes, response headers, and raw property presence are already unavailable.

`merge_track_context_from_detail` applies local and feed defaults before RSS enrichment and source-text cleaning.
`api::response_json` consumes response text and decodes a DTO. Unknown JSON properties disappear during this decode.
`rss::fetch_track_enrichment_from_feed` retains a successful parsed body, but rejects HTTP and parse failures before returning an observation.

`subscribe_service::enrich_track_context_from_rss` discards the RSS error.
These fallback choices remain available only after the new writer retains the failed request evidence.
Serialization of a cleaned DTO is never an original observation.

## Files To Inspect

Read the authority documents above and these files:

- [AGENTS.md](../../AGENTS.md), current work rules and the visual pause.
- [Source map](../../.github/copilot-instructions.md).
- [Request baseline](../notes/adr-0075-request-and-write-baseline.md), Library request counts and measurement limits.
- [Collection rules](../schema/adr-0075-collection-completeness-rules.md), presence, ownership, and incomplete responses.
- [Field inventory](../schema/adr-0075-metadata-field-inventory.md), known collections and scalar fields.
- [api.rs](../../src/api.rs), `Client`, URL construction, wrapped decoding, `response_json`, and `response_text_with_status`.
- [http_client.rs](../../src/http_client.rs), document timeout ownership and client construction.
- [RSS enrichment](../../src/rss/enrich.rs), fetch, XML decoding, document parsing, matching, and `RssObservation`.
- [RSS module](../../src/rss/mod.rs), current exports.
- [metadata.rs](../../src/metadata.rs), `TrackContext`, its constructor, and sanitization.
- [feed_service.rs](../../src/feed_service.rs), shared fetch, defaults, and enrichment.
- [Library queries](../../src/application/queries/library.rs), `FetchLibraryTrackContext`, local fallback, comparison, and error mapping.
- [Subscription service](../../src/subscribe_service.rs), the RSS URL rule and error-discarding wrapper.
- [Command errors](../../src/application/errors/command.rs), the current string-only query error.
- [Library callbacks](../../src/library/app_impl.rs), `load_track_source_context` and `command_error_detail`.
- [Library view model](../../src/view_models/library.rs), session state and the existing error-status owner.
- [Command presenter](../../src/presentation/async_command_presenter.rs), session admission and error delivery, for inspection only.
- [Database owner](../../src/db.rs) and [schema 12](../../src/db/provider_snapshot_schema.rs).
- [Identity ingestion](../../src/identity_ingest.rs), existing source-label writes, for inspection only.
- [Architecture tests](../../tests/architecture_tests.rs), existing source ownership and service boundaries.
- [Cargo.toml](../../Cargo.toml), [Cargo.lock](../../Cargo.lock), and [crate declarations](../../src/lib.rs).

Inspect the locked dependency source without changing it:

- `reqwest-0.13.2/src/blocking/response.rs`, `bytes`, `text`, and the `Read` implementation.
- `reqwest-0.13.2/src/async_impl/response.rs`, charset selection and text decoding.
- `reqwest-0.13.2/src/blocking/client.rs`, request timeouts and response construction.
- `reqwest-0.13.2/src/async_impl/body.rs`, total response deadlines.
- `encoding_rs-0.8.35`, existing charset aliases and BOM behavior.

## Files To Change

| File | Permitted change |
| --- | --- |
| `src/provider_observation.rs` | New renderer-free types, recorder boundary, retained failures, and receipt handling. |
| `src/provider_observation/http.rs` | New shared capture helper for response bytes, allowed headers, actual times, and partial reads. |
| `src/provider_observation/musicindex.rs` | New extraction adapter from original JSON into coverage and fact evidence. |
| `src/db/provider_observations.rs` | New durable request allocation and atomic observation writer. |
| `src/db.rs` | Declare the writer module. Leave migration 12 and schema inspection unchanged. |
| `src/lib.rs` | Declare the observation module. |
| `src/api.rs` | Share response capture and decoding. Add explicit observation recording at the selected metadata request boundary. |
| `src/http_client.rs` | Expose the existing document deadline through a narrow helper. Do not change its values or configuration. |
| `src/rss/enrich.rs` | Share fetch and parse logic. Capture failed responses and extend evidence during the existing single DOM parse. |
| `src/rss/mod.rs` | Export only the observed enrichment entry point and evidence types reached by this slice. |
| `src/metadata.rs` | Retain receipt summaries separately from API DTO fields and from the successful RSS carrier. |
| `src/feed_service.rs` | Select observation recording for the Library detail command and preserve typed failures through existing fallbacks. |
| `src/subscribe_service.rs` | Share RSS URL selection and enrichment logic with the observed path. Other roots remain unobserved. |
| `src/application/queries/library.rs` | Connect `FetchLibraryTrackContext`, retain its receipts, and reject silent storage-failure fallback. |
| `src/application/errors/command.rs` | Carry an `ObservationWriteFailure` through a typed command error with safe display. |
| `src/view_models/library.rs` | Retain failed response capsules by generation for this Library session. Use the existing error-status owner. |
| `src/library/app_impl.rs` | Connect the existing load error callback and add the exhaustive safe-display arm. No renderer change. |
| `Cargo.toml`, `Cargo.lock` | Add direct dependencies on locked `base64 =0.22.1`, `encoding_rs =0.8.35`, and `mime =0.3.17` only if needed. |
| `tests/architecture_tests.rs` | Add one situational ADR 0075 guard for this writer and its selected live root. |
| This packet | Record implementation evidence, exact covered roots, and remaining limitations. |

All other file additions or scope changes require orchestrator review before implementation.
Do not add a second integration test file or a shared test helper module.

## Types And Module Ownership

Use concrete crate-owned types in `provider_observation.rs`. Keep API data types free of application observation state.
No type below contains GPUI values, a renderer callback, an API access token, or broadcaster credentials.

| Type | Required meaning |
| --- | --- |
| `ProviderRequestSpec` | Provider identity, exact resource, unresolved request parameters, optional scoped subject, exact profile, and actual start time. |
| `ProviderObservationRecorder` | Shared connection and per-command receipt handling at the selected request boundary. No global cache or request scheduler. |
| `RequestToken` | Private durable request key and generation returned after allocation commits. Callers cannot invent a generation. |
| `ProviderObservation` | Shared `Arc<[u8]>` body, stable decoder inputs, allowed occurrence headers, source evidence, coverage, facts, and typed outcome. |
| `ObservationOutcome` | `Success`, `Partial`, or `Failed`. Collection completeness remains separate from transport and decode success. |
| `PropertyPresence` | `Missing`, `Null`, `Empty`, `Populated`, or `Invalid`, measured from the original response. |
| `CoverageEvidence` | Request intent, collection or scalar token, declared target evidence, presence, incomplete state, and reason. |
| `ObservationReceipt` | `#[must_use]` persisted observation ID, request generation, resources, body key, actual times, headers, and every collection outcome. |
| `ObservationRetention` | Explicit unknown, partial, failed, repeated, and superseded-attempt outcomes supported by this slice. No replacement result. |
| `ObservationStorageError` | Typed request-allocation or response-write failure. It never claims retention succeeded. |
| `ObservationWriteFailure` | Failed write operation with its original token, captured observation, and safe error summary. |

Use the schema's versioned keys and signed UTC microseconds.
Store supplied source dates and timestamps unchanged, separate from app times and HTTP occurrence headers.
First and latest occurrence fields follow generation order, even when wall-clock order differs.

The recorder owns the shared database handle. It locks the connection only for request allocation or response persistence.
Do not hold the database mutex during HTTP work, decoding, or source-text cleaning.
Keep SQL in `db/provider_observations.rs`. Keep provider parsing in the provider adapters.

`begin_provider_request` and `record_provider_observation` are the only production writers added here.
The second operation owns one transaction for bodies, observations, coverage, facts, and applicable request-slot updates.
Packet 013 must extend that transaction. It must not append a separately committed replacement operation.
Do not create unused snapshot, selection, discrepancy, or active-projection APIs.

## Capture And Decode Contract

### HTTP Capture

1. Build the same legal request URL and parameters that the existing caller uses.
2. Commit `begin_provider_request` before sending the request.
3. Capture status, final URI, and allowed headers before consuming the response.
4. Read response bytes into one retained buffer, including bytes received before a read failure.
5. Record the actual start, finish, and available response completion times.
6. Decode from the retained bytes through the existing provider decoder behavior.
7. Construct coverage and facts from the original parsed response.
8. Commit the observation before returning DTOs or permitting fallback.

Keep the schema's exact header allowlist and Base64 representation of original header-value bytes.
Preserve missing headers separately from present empty values. Preserve repeated values in received order.
Do not retain cookies, authorization headers, or unrelated response headers.
Do not print bodies, source values, or complete request URIs in errors or `Debug` output.

The retained bytes are the entity body exposed by reqwest, including its active content-decoding behavior.
Do not label decompressed bytes with content codings that no longer apply.
Keep effective decoder inputs separate from raw header spelling and occurrence-only validators.

`Response::bytes()` discards its internal buffer on failure. It cannot satisfy partial-body retention by itself.
The blocking `Read` path also applies its wait per read, unlike one whole-body `bytes()` wait.
Use the existing document deadline from `http_client.rs` during streaming capture.
Do not add another timeout value or let a slow response extend the current document budget.
Test a partial read failure and a slow response that supplies bytes repeatedly before the deadline.

Use `RequestBuilder::timeout` with the existing document duration for observed requests.
The orchestrator approved this stricter combined bound from connection start through body completion.
The existing blocking client can wait separately for response headers and the complete body.
Keep that existing behavior for unobserved callers. Checking time only between blocking reads cannot enforce the observed request deadline.

A failure before any response has no invented HTTP status, response URI, body, or fetch completion time.
A failed body read retains its captured prefix with `body_state = truncated`.
HTTP errors and malformed payloads retain available complete bytes with a failed outcome.
An empty HTTP body remains different from no response body.

### MusicIndex Evidence

Preserve existing charset selection, BOM handling, and decoding behavior before changing the response helper.
Use characterization cases against the locked reqwest decoder, including one declared legacy charset and malformed character input.
Decode the DTO directly from retained decoded text through the existing `serde_json::from_str` behavior.
Parse that same text separately for raw property and member evidence when needed.

Do not decode the DTO from `serde_json::Value`. That intermediate value collapses duplicate object keys before DTO validation.
Duplicate known fields must retain their current decode failure while the original body remains stored.
The single document-parse rule applies to the RSS DOM, not JSON evidence extraction.

Auxiliary evidence extraction must not reject a response that the existing DTO decoder accepts.
An ignored unknown JSON value can exceed the raw-value adapter's numeric representation.
Retain the original body and mark extraction incomplete when this occurs. Preserve the successful DTO result.
Do not enable arbitrary-precision features or change scalar policies to simplify the adapter.

Record the exact existing endpoint shape, include text, request scope, and pagination parameters.
This is a versioned description of the request that happened. It is not a new named request-profile or completeness contract.
Keep an unscoped track request unresolved until the response supplies sufficient declared ownership evidence.

For known collections, retain missing, null, empty, populated, and invalid property states.
Keep original member JSON and every supplied claim field, including unknown member properties.
Preserve contributor, enclosure, transcript, link, identity, release-claim, platform, relationship, and value-split evidence when supplied.
Unknown properties remain recoverable from the original body even when no typed fact adapter exists.

Scalar evidence uses `field:<wire-path>` tokens and retains original values without display cleaning or defaults.
An absent scalar records missing presence, not verified absence.
Unknown ownership remains unresolved. A requested subject cannot override a contradictory member owner.
Without a registered completeness contract, the adapter records unknown or partial coverage and a reason.
No input in this packet can produce complete coverage or a complete-empty collection.

### RSS Evidence

Keep `RssObservation` and its shared body usable by existing consumers.
Extend the existing DOM extraction pass when coverage or fact evidence needs a node location.
Do not parse the document again for persistence or introduce a second parser after an error.
Keep packet 009's XML encoding, namespace, owner, rejection, matching, and scalar compatibility behavior.

Retain direct-owner identity evidence, unsupported syntax, source locations, requested matching inputs, and observed GUIDs.
Keep excluded contributor and nested elements recoverable without promoting them into active identities.
All facts from one response reference its shared body. Do not copy full XML into each track fact.

A parsed feed without the requested item produces `Partial` with a no-match reason.
It cannot assert that the requested track has empty collections.
A match through an enclosure does not prove that a conflicting observed item GUID equals the requested GUID.
Malformed XML produces `Failed` while retaining its body and decoder evidence.
Existing successful RSS context and selected DTO values survive a later failed RSS request.

## Durable Writer Contract

1. Allocate generations through the schema singleton inside the request-allocation transaction.
2. Validate provider/resource agreement and exact scoped subject keys.
3. Compute versioned, length-delimited request, body, and observation identities.
4. Compare identity inputs before reusing a matching key.
5. Insert or reuse body, observation, coverage, and fact rows in one response transaction.
6. Update first/latest occurrence evidence and counts by generation order.
7. Update a request slot only when the token still owns its applicable generation.
8. Return a receipt only after commit succeeds.

Older completions retain their evidence but cannot overwrite a newer request slot's state or failure reference.
This rule does not replace collection-head generation checks, which belong to packet 013.
Occurrence-only header changes must reuse the existing observation and fact rows.
Changed payload source revisions, source timestamps, decoder inputs, or body bytes can establish a different observation.

Body reuse requires an equal digest, byte length, and bytes. A hash collision or inconsistent stored record is a storage error.
Fact and coverage reuse requires equal immutable interpretation evidence.
Do not silently accept a digest match whose stored inputs differ.

Use no garbage collection, legacy backfill, schema change, or hidden source-label replacement.
Seeded collection heads, snapshots, selections, discrepancies, and legacy tables must remain unchanged.
Pending slots survive reopening the database. Opening another connection does not prove that a request lost its owner.
Generations must continue increasing across connections and reopen.

Packet 018 requires a lifecycle contract that proves a request owner is absent before marking its pending slot abandoned.
This packet does not invent process ownership or modify migration 12 to support it.

## Live Caller Integration

Select observation behavior at the existing request boundary.
Share `api::Client` URL construction and response decoding between observed and existing unobserved callers.
Do not create another HTTP client implementation or copy the Library fetch sequence into a parallel function.

Keep the existing public service path available to excluded callers.
Add an explicit recorder argument or constructor at the shared internal boundary used by the selected Library command.
The absence of a recorder must be an explicit compatibility mode, never a fallback after a storage error.

In `fetch_library_track_detail`, replace error-discarding shortcuts only where they would hide a storage failure.
Retained scoped-track transport failure can proceed to the existing unscoped request.
Retained feed failure can preserve the existing local feed defaults.
If all relevant remote requests fail after retention, the existing local context remains available.

Attach receipt summaries to the returned `TrackContext` outside `api::Feed` and `api::Track`.
Keep the actual receipt for every request, including a failed scoped request followed by successful fallback.
Preserve receipts through sanitization, cloning, and the local-fallback result.
If a later operation fails, its typed command error retains earlier committed receipts.
The Library view model retains those receipts even when allocation fails before the next request.
The returned successful RSS carrier and the latest failed RSS receipt remain distinct.

Use the existing RSS URL selection and enrichment implementation with the same recorder.
Record its failure before the best-effort enrichment wrapper keeps current values.
Do not change which RSS URL wins or add a new request when the current path would make none.

### Storage Failure And Retry

A retained transport failure means its observation transaction committed. Existing local fallback remains permitted.
A storage failure means storage could not confirm evidence retention. The command must report that error even when local facts exist.
Do not convert it through `.ok()`, `unwrap_or_else`, or the general local fallback.
Failure to allocate the request must prevent network work.

The writer must return the captured input with a failed response transaction.
A retry of that input uses the original token, body, times, and headers without another HTTP request.
Precommit failure tests must prove complete rollback and successful retry of that same input.
Do not claim rollback from an error alone or blindly repeat a commit with uncertain status.

The current `query_error` converts service errors into `CommandError::Query(String)`.
Add `CommandError::ObservationWriteFailure` carrying the typed failure through this boundary.
Its `Debug` implementation omits response bytes and private source content. Its `Display` reports the failed storage operation safely.
Preserve existing command error equality and clone contracts without duplicating large response buffers.

`LibraryViewModel` owns the failed capsules in a generation-keyed collection for the current Library session.
`load_track_source_context` must call `LibraryViewModel::retain_observation_failure` before updating any selected-frame state.
The current error callback does nothing. Replace that callback with the view-model method.
Do not require the original track frame to remain selected before retaining its failure.
Navigation, successful later loads, and later failures must not discard earlier capsules.

The new method retains the capsule and updates the existing status string through the `LibraryStatusSnapshot` presentation path.
Do not call `set_error_status` for this background failure. That helper also clears loading and cancels Library removal confirmation.
Preserve selection, removal confirmation, unrelated loading, and unrelated busy state.
An allocation failure has no response capsule. Report its safe query error through the same narrow status owner.

Keep source generation, actual response times, and safe failure details in the typed view-model state.
Add an accessor only when a production consumer needs it.
The capsule retains raw data for a later storage-only retry, but the status text contains no raw body or source value.
Use a neutral storage-failure message. Do not classify this result as unavailable MusicIndex or successful local fallback.

The existing presenter rejects callbacks after session teardown. Teardown ends in-memory retention without claiming persistence.

Request tokens share one in-memory write state across clones. The states are `Ready`, `Retry`, `Committed`, and `Uncertain`.
The writer enters `Uncertain` before opening the response transaction.
An explicit successful rollback, followed by SQLite autocommit verification, changes that state to `Retry`.
A retry must use the same observation, including its original bytes, headers, and times.

A successful commit stores the observation and receipt in `Committed`.
An identical replay returns that receipt without database mutations.

Changed input is rejected.
A commit error, failed rollback, or poisoned state lock leaves retry blocked.
The writer makes no rollback claim for these uncertain states.

The writer's existing record operation supports explicit retry of a retained capsule.
A separately bounded packet 019 follow-up owns the visible retry action and capsule removal after verified persistence.
This packet adds no retry screen, global cache, filesystem spool, or automatic retry policy.
The changed error-status presentation requires operator acceptance after the visual pause ends.

## Mechanical Acceptance Criteria

Use behavioral tests beside the owning code with the prefix `adr_0075_observation_`.
Tests use disposable databases and scripted local HTTP responses only.

| Case | Required proof |
| --- | --- |
| O14-01 | The live Library command allocates each generation before the server receives its request. Its returned context retains consumed receipts. |
| O14-02 | Original JSON bytes, unknown member properties, null, empty, missing, invalid, and populated states survive storage and reopen. |
| O14-03 | Original RSS bytes, decoder inputs, GUID scope, rejected identity syntax, and source locations survive storage and reopen. |
| O14-04 | Two feeds with the same item GUID produce distinct resolved subjects. Unscoped or contradictory ownership remains unresolved. |
| O14-05 | HTTP failure, connection failure, malformed JSON, malformed XML, and RSS no-match retain separate typed outcomes before fallback. |
| O14-06 | A failed body read retains its received prefix. Slow repeated bytes cannot extend the existing document timeout budget. |
| O14-07 | A begin-write failure makes zero HTTP requests. A response-write failure reaches the command error despite usable local facts. |
| O14-08 | Injected failures after body, observation, coverage, and fact writes leave no partially committed response. The retained input permits storage-only retry. |
| O14-09 | Repeated identical responses reuse bodies, observations, coverage, and facts. First/latest evidence and occurrence counts still update. |
| O14-10 | Date, ETag, Last-Modified, and cache-header changes update occurrence evidence without new observation or fact rows. |
| O14-11 | Older completions retain evidence without overwriting a newer slot or failure. Wall-clock order does not override generation order. |
| O14-12 | Two connections and a reopen preserve pending requests and continue durable generation allocation. No automatic abandonment occurs. |
| O14-13 | Endpoint changes isolate provider observations. Redirects retain requested identity and record the final response resource. |
| O14-14 | Seeded snapshots, heads, field selections, discrepancy history, and every legacy table remain equal after successful and failed retention. |
| O14-15 | Original JSON and RSS values remain unchanged in evidence after DTO cleaning, local defaults, feed fallback, and context cloning. |
| O14-16 | One received RSS document is parsed once. Identical RSS bytes across track requests occupy one durable body row. |
| O14-17 | Existing unobserved comparison and other excluded roots retain their request and decode behavior. No live-event response enters this recorder. |
| O14-18 | Debug and error output omit response bodies, private header values, and complete resource URIs. Allowed headers remain available as stored evidence. |
| O14-19 | A typed command write failure reaches the Library view model. Navigation and later results preserve every failed generation until session teardown. |
| O14-20 | The view model exposes safe failure status without changing selection, removal confirmation, or unrelated busy/loading state. It makes no persistence claim. |
| O14-21 | Duplicate known JSON fields retain the existing DTO decode failure. Original duplicate-key bytes remain stored as failed evidence. |
| O14-22 | Auxiliary raw-value extraction failure preserves a successful DTO result, original bytes, and incomplete evidence. Characterize ignored unknown numbers with locked serde. |

For O14-14, use an explicit table list from the schema. Do not omit evidence tables to make preservation pass.
The guard names ADR 0075 and its situational class. Behavioral tests prove persistence and fallback behavior.

### Request And Write Bounds

Use packet 016's same two-track local fixture shape, with the current source under test.
This packet changes storage writes and does not claim a performance improvement.
Record received requests separately from SQLite row mutations and transaction counts.

| Scripted case | Required maximum metadata requests |
| --- | --- |
| Successful first Library track detail | Two Index requests and one RSS request. |
| Successful repeated Library track detail | Two Index requests and one RSS request. |
| Two successful tracks sharing one feed | Four Index requests and two RSS requests. |
| Index HTTP 503 for all existing fallback requests | Three Index requests and zero RSS requests. |
| Failed request allocation | Zero requests for that failed allocation. |
| Storage-only retry of captured input | Zero additional HTTP requests. |

Keep the existing include strings, fallback order, request counts, and request scheduling.
For unchanged repeated responses, body, observation, coverage, and fact row counts must remain stable.
Report actual row mutations and expected occurrence updates. Do not describe them as disk writes or bytes written.

## Test Commands

Run focused tests during implementation. Run the full suite after integration.

```bash
cargo test --locked --offline adr_0075_observation_
cargo test --locked --offline adr_0075_rss_
cargo test --locked --offline
cargo check --locked --offline
cargo clippy --locked --offline -- -D warnings
cargo fmt -- --check
cargo build --locked --offline --bin v4vmm
python3 -B docs/runbooks/check-markdown-links.py docs/tasks/adr-0075-task-014-provider-observation-retention.md
python3 -B /home/citizen/.agents/skills/asd-ste100/scripts/ste_lint.py --check --no-heuristics docs/tasks/adr-0075-task-014-provider-observation-retention.md
git diff --check
```

The reviewed incoming suite has 1,593 unit tests and 265 architecture guards.
Packet 012 also passed 42 Python fixture tests and rebuilt the normal desktop binary after tests.
These are prior results, not proof of this packet.
Do not rerun unchanged fixture tests unless a change affects their owner or a failure requires investigation.

## Required Caller Follow-Ups

The phase plan assigns packets 038–043 to the remaining caller families.
Each needs its own reviewed task file before dispatch. This packet does not authorize those caller conversions.

| Follow-up family | Exact current roots | Dependencies and retained boundary |
| --- | --- | --- |
| 038, remaining Library readers | `CompareLibraryTrack`, `compare_library_track`, and `hydrate_album_identity_facts` in `application/queries/library.rs`. | This packet. Add database access to comparison without changing tag comparison. Retain hydration observations before legacy persistence. |
| 039, feed checks and updates | `feed_service::{check_feed_staleness, apply_feed_updates}` and `application/commands/feed.rs`. | This packet and reviewed persistence integration. Preserve existing explicit update and tag-write authorization boundaries. |
| 040, Index search and general inspectors | `application/queries/search.rs` and `application/queries/feed.rs`, including recent feeds, scoped detail, contributors, value-route reads, and podroll resolution. | This packet. Carry evidence through app-owned feed/result wrappers. Payment, publisher, and artist policies remain separate. |
| 041, RSS subscription/import | `rss::subscribe_feed`, `ensure_feed_in_db`, and their callers. | This packet. Retain the original subscription document without a second parse or invented ownership. |
| 042, download and subscription contexts | `subscribe_service`, `SubscribeFeedRequest`, download commands, and retained materialization. | This packet and source result wrappers. Preserve observations before sanitization, enrichment, defaults, and retry. |
| 043, CLI metadata readers | `cli.rs` metadata request roots. | Shared observed callers above. Preserve JSON stdout and use a reviewed database lifecycle. |
| Restart request ownership | Packet 018's request/cache lifecycle. | This packet. Prove owner absence before marking a pending request abandoned. |
| Explicit storage retry | A bounded packet 019 follow-up through the Library view model's retained capsules. | This packet. Dispatch the existing record operation without HTTP work. Remove a capsule only after verified persistence. |

Each caller packet must identify its exact success and failure receipt consumers.
Packet 013 supplies verified replacement inside the same writer when a converted caller has the required completeness contract.
Its local reads must not treat a legacy DTO or source label as a provider snapshot.
Broad ADR 0075 retention stays open until these live paths have their own completed packets.

## Do Not Touch

- Migration SQL, schema 12, production databases, startup repair, restore, or backup behavior.
- Shared ADR, schema, plan, review, pending-check, or status files owned by the orchestrator.
- Source-label grouping and replacement in `identity_ingest.rs`.
- Snapshot heads, selected fields, discrepancy transitions, legacy cleanup, or metadata repair.
- Search, general inspector, album hydration, subscription/import, CLI, and live-event request wiring in this slice.
- Audio files, audio tags, playback, payment behavior, broadcast behavior, or user configuration.
- Renderer composition, layouts, owner labels, field priorities, expiry values, or request scheduling.
- The upstream checkout, deployed services, and generated upstream API documentation.

Do not run the app, request a visual batch, commit changes, or contact MusicIndex for an update hook.
Preserve all existing dirty work.

## Rollback And Escalation

This packet changes no schema. A code revert leaves recorded evidence and schema 12 intact.
Do not delete retained rows, reset generations, or remove migration records during rollback.
Use disposable fixtures for failure injection and remove only fixtures that the test created.

Return to the orchestrator for a new completeness claim, changed field policy, unresolved ownership assumption, or required schema change.
Also return for a different HTTP deadline, broader caller conversion, new storage-retry mechanism, or additional dependency version.
Report any mismatch between the schema contract and the live receipt consumer before implementation continues.
Do not remove an accepted preservation requirement to make the packet smaller.

## Implementation Evidence - 2026-09-21

The live `FetchLibraryTrackContext` command records its scoped track, optional unscoped fallback, feed, and RSS requests.
Other callers retain their unobserved compatibility path. No request include, fallback order, or provider selection changed.
Each request commits its generation before HTTP work. Each response commits its evidence before DTO defaults or local fallback.

The writer stores original bodies, declared owners, property states, raw members, XML locations, decoder inputs, and allowed occurrence headers.
MusicIndex scalar facts retain unresolved ownership. Container identity does not establish ownership for an inherited scalar.
Malformed or empty supplied owners remain unresolved. Invalid publisher shapes remain evidence without becoming complete collections.
All observations remain outside snapshot replacement, field selection, discrepancies, and legacy source-label writes.

The typed command failure retains earlier committed receipts when a subsequent write, allocation, or local fallback fails.
The Library view model retains those receipts and all failed response generations until session teardown.
Its existing status owner reports the storage failure without changing selection, removal confirmation, loading, or busy state.
No unused production accessor or visible retry action was added.

The root approved one compiler-required scope exception: thirteen empty receipt initializers in `src/discover/tests.rs`.
Those initializers preserve each existing test field and behavior.
The three direct dependencies use their existing locked versions: base64 0.22.1, encoding_rs 0.8.35, and mime 0.3.17.

### Verification Mapping

All test names below start with `adr_0075_observation_`.
The files own their tests. No shared test helper or second integration file was added.

| Criteria | Owning regression suffixes |
| --- | --- |
| O14-01, O14-09, O14-15 | `live_library_request_bounds_receipts_and_repeated_mutations` |
| O14-02, O14-12 | `reopen_connections_preserve_pending_and_original_evidence` |
| O14-03, O14-16 | `rss_single_parse_scoped_originals_survive_reopen` |
| O14-04 | `declared_ownership_separates_feeds_and_rejects_contradictions` |
| O14-05, O14-21, O14-22 | `decode_failure_and_auxiliary_failure_keep_original_bytes`, `retained_failures_preserve_local_and_scoped_fallback` |
| O14-06 | `partial_body_and_continuous_stream_share_total_deadline` |
| O14-07, O14-18 | `storage_failures_cross_command_boundary_and_retry_without_http`, `allowed_header_bytes_and_no_response_are_distinct` |
| O14-08 | `atomic_failures_keep_capsules_and_retry_without_network`, `token_replay_is_idempotent_and_uncertain_commit_blocks_retry` |
| O14-10, O14-11 | `repeated_headers_and_generation_order_preserve_identity` |
| O14-13 | `provider_scope_decoder_and_source_revisions_isolate_identity`, `redirect_retains_request_identity_and_final_resource` |
| O14-14 | `preserves_all_legacy_and_snapshot_evidence_tables` |
| O14-17 | `unobserved_service_keeps_compatibility_requests`, `writer_and_library_retention_have_one_owner` |
| O14-19, O14-20 | `failures_survive_navigation_and_preserve_operation_state`, `earlier_receipts_survive_later_write_and_allocation_failures` |
| Decoder compatibility | `charset_matches_locked_reqwest_text_decoder` |
| Collision rejection | `collision_and_interpretation_mismatch_fail_closed` |

The preservation test names all nineteen legacy tables and all applicable provider evidence tables.
It compares each preexisting row in bodies, observations, coverage, facts, snapshots, heads, selections, and discrepancy history.
The response-failure tests verify empty response tables before retry. A commit-rejection test proves that uncertain tokens remain blocked.

### Measured Request And Row Counts

The local fixture contains one feed and two Library tracks. Its RSS document contains both items.
The observation fixture also includes raw placeholder text and an unknown contributor property.
These payloads extend packet 016's fixture shape to test retention.

| Case | Index requests | RSS requests | SQLite row mutations | Committed transactions |
| --- | ---: | ---: | ---: | ---: |
| First successful track detail | 2 | 1 | 139 | 6 |
| Repeated successful track detail | 2 | 1 | 12 | 6 |
| Two successful tracks with one feed | 4 | 2 | Not separately measured | Not separately measured |
| All Index fallback requests return 503 | 3 | 0 | Not separately measured | Not separately measured |
| Failed first request allocation | 0 | 0 | 0 committed | 0 |
| Storage-only retry | 0 additional | 0 additional | Not separately measured | Not separately measured |

The repeated case keeps three bodies, three observations, 95 coverage rows, and 21 facts.
Its twelve mutations update three generation allocations, three request slots twice, and three observation occurrence summaries.
These counts describe rows and transactions. They do not measure disk writes or written bytes.
The original and repeated request includes remain unchanged. No performance improvement is claimed.

### Changed Files And Limits

New modules: `src/provider_observation.rs`, `src/provider_observation/http.rs`, `src/provider_observation/musicindex.rs`, and `src/db/provider_observations.rs`.
Existing source changes follow the permitted-file table, plus the approved Discover test initializers.

| Area | Existing files changed by this packet |
| --- | --- |
| Module and dependency declarations | `src/lib.rs`, `src/db.rs`, `Cargo.toml`, `Cargo.lock` |
| Capture and transport | `src/api.rs`, `src/http_client.rs`, `src/rss/enrich.rs`, `src/rss/mod.rs`, `src/metadata.rs` |
| Caller integration | `src/feed_service.rs`, `src/subscribe_service.rs`, `src/application/queries/library.rs` |
| Typed failure and status | `src/application/errors/command.rs`, `src/view_models/library.rs`, `src/library/app_impl.rs` |
| Regression checks | `tests/architecture_tests.rs`, `src/discover/tests.rs` |

Only this packet changed as implementation documentation.
No document moved, no documentation folder was added, and no root document was created.
Existing root documents and unrelated changes remain unchanged by this agent.

The existing complete RSS carrier survives subsequent failed enrichment. Failed attempts retain separate receipts.
HTTP streaming uses the existing document duration as a request timeout. Unobserved callers keep their existing timeout behavior.
Only one RSS DOM parse runs for each received document. No response body is reconstructed from a cleaned DTO.

The Library presentation gate remains open and paused. The procedure below remains prospective.
Excluded callers, complete collection replacement, visible retry, and process-ownership recovery remain future work.

### Final Mechanical Checks

Each command below sent its output to the listed log. The final integrated run used four test threads.
The normal desktop binary was rebuilt after the final tests.

| Command | Exit | Result | Log |
| --- | ---: | --- | --- |
| `cargo test --locked --offline adr_0075_observation_ -- --nocapture` | 0 | Green: 20 unit tests and one architecture guard. | `/tmp/adr0075-014-focused-final.log` |
| `cargo test --locked --offline adr_0075_rss_` | 0 | Green: 33 unit tests and one architecture guard. | `/tmp/adr0075-014-rss.log` |
| `cargo test --locked --offline -- --test-threads=4` | 0 | Green: 1,613 unit tests and 266 architecture guards. Ten documentation tests remain ignored. | `/tmp/adr0075-014-full-final.log` |
| `cargo check --locked --offline` | 0 | Green. | `/tmp/adr0075-014-check-final.log` |
| `cargo clippy --locked --offline -- -D warnings` | 0 | Green. | `/tmp/adr0075-014-clippy-final.log` |
| `cargo fmt -- --check` | 0 | Green. | `/tmp/adr0075-014-format-final.log` |
| `cargo build --locked --offline --bin v4vmm` | 0 | Green: normal desktop binary rebuilt. | `/tmp/adr0075-014-build-final.log` |

The focused RSS run preceded the final charset-retention correction. The final integrated run includes that correction and all RSS tests.
The prospective desktop server passed a Python syntax check. The server and desktop procedure were not run.
No unchanged startup-fixture tests were repeated.

### Earlier Failure Evidence

The initial `cargo test --locked --offline` run exited 101. Its log is `/tmp/adr0075-014-full.log`.
The Library fallback fixture lacked the new schema. Its request allocation failed before the local fallback check.
That existing fixture now runs `migrate_schema` and checks its three retained receipts.

The same run had one unrelated broadcast-readiness timeout at `src/runtime/broadcast_readiness.rs:214`.
The recorded assertion was `actor should publish before timeout: Elapsed(())`.
No broadcast-readiness code changed.
The isolated `cargo test --locked --offline broadcast_readiness_actor_refreshes_on_command` rerun exited 0 in 0.03 seconds.
Its log is `/tmp/adr0075-014-readiness-rerun.log`. The final integrated run also passed that test.

Two intermediate integrated runs passed all unit tests but found source-guard failures.
Their logs are `/tmp/adr0075-014-full-guards.log` and `/tmp/adr0075-014-full-rerun.log`. Both commands exited 101.

The failed-local-fallback test now uses a SQLite read authorizer. It no longer mutates SQL outside the database owner.
The existing placeholder guard now checks the shared enrichment call and ignores call whitespace and its optional trailing comma.
The final integrated run passed both guards. Their ownership and placeholder requirements remain enforced.

### Final Document Checks

The final document checks use only this packet, except the workspace diff whitespace check.

| Command | Exit | Result | Log |
| --- | ---: | --- | --- |
| `python3 -B docs/runbooks/check-markdown-links.py docs/tasks/adr-0075-task-014-provider-observation-retention.md` | 0 | Green: 31 local links. | `/tmp/adr0075-014-links-final.log` |
| `python3 -B /home/citizen/.agents/skills/asd-ste100/scripts/ste_lint.py --check --no-heuristics --json docs/tasks/adr-0075-task-014-provider-observation-retention.md` | 1 | No structural findings. Lexical findings remain. | `/tmp/adr0075-014-ste-final.json` |
| `git diff --check` | 0 | Green. | `/tmp/adr0075-014-diff-final.log` |

The shared STE checker reports technical terms with dictionary limitations.
These terms retain their source meaning. The raw STE result is not Green and does not prove full standard compliance.
No broken local link was found.

## Document Checks - 2026-09-20

Local links: Green, 31 links in this packet. Diff whitespace: Green.
The shared STE check reports no structural findings. Lexical findings remain for technical terms and dictionary coverage.
The raw STE result is not Green and does not prove full standard compliance.

Only this packet was created. No documentation file moved, no folder was added, and no root document was created.
No application code, database, or shared status document changed during packet preparation.
The orchestrator owns technical review, dispatch, and updates to the plan and pending-check index.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:

- This packet and every entry under Files To Inspect.
- `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.
- `/home/citizen/.agents/skills/rust-skills/rust-dev/SKILL.md`.

Goal:

- Retain provider observations through the shared writer and the selected live Library track-detail command.

Constraints:

- Start only after the orchestrator closes technical review and records dispatch.
- Follow the capture, decode, writer, caller, and storage-failure contracts above.
- Preserve original evidence and existing display policies.
- Keep unknown, partial, and failed collections unchanged.
- Write STE prose directly. Run the shared checker.

Do not touch:

- Every path and behavior under Do Not Touch.

Acceptance criteria:

- Prove O14-01 through O14-22 and the request bounds through named behavioral tests.
- Retain reachable observation receipts and storage-failure input at the reviewed application boundary.
- Leave excluded caller families and paused visual gates open.

Test commands:

- Run every applicable command under Test Commands.
- Rebuild the normal desktop binary after tests.

At the end, report:

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

The Library storage-failure status needs operator acceptance. Its gate remains open and paused.
Do not request these checks until the operator resumes visual checks.
Packet 012's presentation gate and all inherited visual gates remain open.

The implementation report must supply an isolated fixture procedure before this gate can be offered.
Use the completed tests' scripted endpoint and disposable database. No failed production service or hardware is required.
The procedure must include exact setup, desktop launch, failure injection, inspection, and cleanup commands.
Do not expose a production failure-injection setting to provide this fixture.

1. Start the isolated app with a valid local track and scripted Index/RSS responses.
2. Trigger a response-write failure after the HTTP body arrives.
3. Verify that the existing Library status identifies a metadata storage failure.
4. Verify that it does not report MusicIndex unavailability or successful metadata persistence.
5. Navigate to another track before a delayed failed request completes.
6. Verify that the failure remains reported without changing the selected track's metadata.
7. Close the fixture app and remove only its disposable files and server process.

The exact procedure follows. Mechanical results cannot close this presentation gate.

### Isolated Desktop Procedure

Run these steps only after the operator resumes visual checks.
Use a Linux desktop session with Python 3 and the normal debug binary.
The fixture uses Null playback and local HTTP responses. It requires no failed production service or special hardware.

1. Create the disposable fixture from the repository root.

   ```bash
   cargo build --locked --offline --bin v4vmm
   O14_ROOT=$(python3 -B docs/runbooks/startup-recovery-fixture.py setup)
   printf '%s\n' "$O14_ROOT"
   ```

2. Save the isolated response server inside that fixture.

   ```bash
   cat > "$O14_ROOT/observation-server.py" <<'PY'
   import hashlib
   import http.server
   import json
   import sqlite3
   import sys
   import time
   from pathlib import Path
   from urllib.parse import unquote, urlsplit

   root = Path(sys.argv[1]).resolve()
   manifest = json.loads((root / "fixture.json").read_text())
   assert manifest["kind"] == "v4vmm-startup-recovery-v1"
   assert manifest["root"] == str(root)
   database = root / "data/library.sqlite"

   class Handler(http.server.BaseHTTPRequestHandler):
       def log_message(self, *_args):
           pass

       def do_GET(self):
           path = unquote(urlsplit(self.path).path)
           if path == "/feed.xml":
               items = "".join(
                   f"<item><guid>{name}</guid><title>{name}</title></item>"
                   for name in ("a.wav", "b.wav", "c.wav")
               )
               body = (
                   '<rss version="2.0"><channel><title>Observation fixture</title>'
                   + items + '</channel></rss>'
               ).encode()
               content_type = "application/rss+xml"
           else:
               data = {
                   "feed_guid": "fixture-feed",
                   "feed_url": base + "/feed.xml",
                   "title": "Observation fixture",
               }
               if "/tracks/" in path:
                   guid = path.rsplit("/", 1)[1]
                   data.update(track_guid=guid, title=guid)
               body = json.dumps({"data": data}).encode()
               content_type = "application/json"
           arm = root / "observation-arm"
           if arm.exists() and "/tracks/" in path:
               with sqlite3.connect(database) as conn:
                   pending = conn.execute(
                       "SELECT max(s.generation) FROM metadata_request_slots s "
                       "JOIN metadata_resources r ON r.id=s.resource_id "
                       "WHERE s.state='pending' AND r.request_uri=?",
                       (base + self.path,),
                   ).fetchone()[0]
               if pending:
                   try:
                       arm.unlink()
                   except FileNotFoundError:
                       pass
                   else:
                       body += b"\n" + b" " * pending
                       (root / "observation-ready").write_text("Request received.\n")
                       deadline = time.monotonic() + 12
                       while time.monotonic() < deadline:
                           if (root / "observation-release").exists():
                               break
                           time.sleep(0.05)
                       with sqlite3.connect(database) as conn:
                           conn.execute(
                               "CREATE TRIGGER IF NOT EXISTS o14_reject "
                               "AFTER INSERT ON metadata_observations "
                               "BEGIN SELECT RAISE(ABORT,'isolated fixture'); END"
                           )
           self.send_response(200)
           self.send_header("Content-Type", content_type)
           self.send_header("Content-Length", str(len(body)))
           self.end_headers()
           self.wfile.write(body)

   server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
   base = f"http://127.0.0.1:{server.server_port}"
   config = root / "config/v4vmm/config.toml"
   text = config.read_text().replace(
       'musicindex_endpoint = "http://127.0.0.1:9"',
       'musicindex_endpoint = ' + json.dumps(base),
   )
   config.write_text(text)
   (root / "case.config").write_text(text)
   state = json.loads((root / "case.json").read_text())
   state["config_sha256"] = hashlib.sha256(config.read_bytes()).hexdigest()
   (root / "case.json").write_text(json.dumps(state))
   with sqlite3.connect(database) as conn:
       conn.execute("UPDATE feeds SET feed_url=? WHERE id=1", (base + "/feed.xml",))
   (root / "observation-endpoint").write_text(base)
   server.serve_forever()
   PY
   python3 -B "$O14_ROOT/observation-server.py" "$O14_ROOT" > "$O14_ROOT/observation-server.log" 2>&1 &
   O14_SERVER_PID=$!
   printf '%s\n' "$O14_SERVER_PID" > "$O14_ROOT/observation-server.pid"
   ```

3. Wait for `observation-endpoint`, then open the fixture through the existing launcher.

   ```bash
   while [ ! -s "$O14_ROOT/observation-endpoint" ]; do sleep 0.1; done
   python3 -B docs/runbooks/startup-recovery-fixture.py run "$O14_ROOT"
   ```

4. Open Music and select `a.wav` in the Library. Verify that its metadata loads.
5. Open another terminal at the repository root. Set `O14_ROOT` to the printed fixture path.
6. Arm the response-write failure, then select `b.wav` in the Library.

   ```bash
   touch "$O14_ROOT/observation-arm"
   ```

7. Wait for the status. The server delays the response for at most twelve seconds.
8. Verify that the Library status reports unconfirmed metadata retention and retained session input.
9. Treat successful persistence, unavailable MusicIndex, or changed removal confirmation as incorrect.
10. Remove the fixture trigger and markers before the navigation check.

    ```bash
    python3 - "$O14_ROOT" <<'PY'
    import sqlite3
    import sys
    from pathlib import Path
    root = Path(sys.argv[1])
    with sqlite3.connect(root / "data/library.sqlite") as conn:
        conn.execute("DROP TRIGGER IF EXISTS o14_reject")
    for name in ("observation-ready", "observation-release"):
        (root / name).unlink(missing_ok=True)
    PY
    touch "$O14_ROOT/observation-arm"
    ```

11. Select `a.wav`, then select `c.wav` before the twelve-second response delay ends.
12. Verify that the delayed failure is reported while `c.wav` stays selected.
13. Treat a changed selected track, canceled removal confirmation, or cleared unrelated busy state as incorrect.
14. Close the fixture app. Stop only this fixture's server, then remove this fixture.

    ```bash
    kill "$(cat "$O14_ROOT/observation-server.pid")"
    python3 -B docs/runbooks/startup-recovery-fixture.py cleanup "$O14_ROOT"
    unset O14_ROOT O14_SERVER_PID
    ```

The commands create no production failure setting. Cleanup removes the trigger with its disposable database.
The procedure is prospective. No desktop visual result is claimed.
