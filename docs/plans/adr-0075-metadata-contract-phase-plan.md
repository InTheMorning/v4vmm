# ADR 0075 Metadata Contract Phase Plan

## Status

Draft - 2026-09-19. The initial source audit and proposed contract are written.
Implementation has not started. The operator paused visual checks and prioritised
metadata handling across v4vmm and MusicIndex.

Complete one packet per session.

## Goal

Preserve metadata meaning from RSS through MusicIndex, local storage and both
app entry routes. Explain missing values, inherited values, conflicts and refresh failures.
Reduce repeated requests after automated checks cover the contract.

## Scope And Non-Goals

Begin with websites, Nostr identifiers, contributor credits and their provenance.
Provenance records a value's owner, source, extraction path and observation time.

Before changing fallback for individual fields, audit the existing rules.
Cover description, artwork, publisher, artist text, language, explicit state and dates.
Keep identity, metadata, payment routes and tag-write policies separate.

Exclude these changes from this work:

- Merging people.
- Redesigning broadcasting.
- Changing configuration formats.
- Rebuilding production data.
- Closing inherited visual gates.

## Assumptions And Decisions

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) is Proposed.
  Resolve its decisions about fields and presentation before the affected packet runs.
- The inspected backend is Stophammer. The `musicindex` repository publishes
  its generated API contract. The deployed revision remains unverified.
- This workspace permits writes to v4vmm. Changes to Stophammer require a
  decision in that repository and a workspace that permits writes there.
  Writing this plan needs no Stophammer decision or write access to that repository.
- Keep `/tmp/v4vmm-governance.ie6k8TQf` as an unmodified evidence copy for now.
  Its removal is not confirmed. Do not ask for visual checks during this pause.
- Existing ADR 0066 gates for configuration formats remain in force.
  This contract covers source facts about music. It does not cover the deferred
  configuration format for the live producer.

## Affected Owners

| Boundary | Owners |
|---|---|
| RSS extraction | Stophammer `stophammer-parser/src/engine.rs`, `types.rs`, parser regression tests. App `src/rss/subscribe.rs`, `src/rss/enrich.rs` |
| Index ingestion and API | Stophammer `src/api.rs`, `src/db.rs`, `src/query.rs`, `src/openapi.rs`, crawler `src/crawl.rs` |
| Published API | `musicindex/getapi.sh`, generated `musicindex/api.json` |
| App transport | `src/api.rs`, `src/feed_service.rs`, `src/application/queries/search.rs` |
| App storage and refresh | `src/identity_ingest.rs`, `src/local_identity.rs`, `src/local_metadata.rs`, `src/db.rs`, `src/subscribe_service.rs` |
| App code that prepares data for display | `src/views.rs`, `src/metadata.rs`, `src/application/queries/library.rs`, entity and track view models |
| App presentation | `src/app/search_dispatch.rs`, shared entity and track shells, local track metadata composition |

## Proposed Sequence

| Phase | Outcome | Prerequisites and completion evidence |
|---|---|---|
| 001 | Preserve contributor claim fields in the app's type for API data | [First packet](../tasks/adr-0075-task-001-contributor-claim-transport.md). Tests check JSON round trips and compatibility. No schema or UI change |
| 002 | Agree the RSS extraction and Index ownership rules | A Stophammer ADR and a shared list of examples. Tests check parsing, ingestion and queries. Preserve compatibility. Do not crawl live feeds again yet |
| 003 | Store provider snapshots and coverage state with their owners | Write the exact schema and migration packet first. Test transactions, empty refresh, rollback, restart and isolation between providers |
| 004 | Share detail requests and code that prepares data for display | Define request includes, scoped identity, response failure handling and fallback for each field. Test partial responses, request counts and route parity |
| 005 | Display the complete contract through both app routes | Share contributor and entity actions with owner labels. Test the view models. Visual acceptance remains pending during the operator pause |
| 006 | Reconcile old records and deployed Index data | Produce a read-only repair report first. Repair records from source evidence. Define how to crawl or ingest feeds again. Check replicas and preservation |

Write later implementation packets after their prerequisites are complete.
Do not implement all phases in one session.
Phase 001 leaves storage and display preparation for later packets.

## Schema And API Implications

Phase 001 adds optional fields to the data transfer object (DTO).
The DTO is the Rust type that represents API data.
Phase 001 changes neither the HTTP contract nor the schema.
Existing payloads remain readable without invented provenance.

Phase 003 must record the provider separately from the source assertion.
It must also record whether a response contains the complete collection.
Existing `source` columns currently serve several meanings.
The migration must not silently reinterpret them.

Create a database backup before the migration. Preserve rows with uncertain provenance.

The Index currently returns contributors selected through inheritance, with the
original owner on each claim. Preserve that behavior until a Stophammer ADR
defines any additional collections. Generated API documentation must match the
deployed routes and fields. Do not use manual changes to generated JSON as the
source of the API contract.

## Performance Checks

Record request counts before changing request scheduling.
Use a local HTTP server with scripted responses. Count requests for these cases:

1. Search.
2. The first request to open a detail view.
3. A repeated request to open that detail view.
4. Detail views that share feed context.
5. An unavailable endpoint.

Record database writes for each case.

The proposed changes must:

- Bound the number of detail requests.
- Avoid fetching full details for every result during list rendering.
- Share requests that are already active.
- Parse each RSS document once per feed observation.

Cache keys include the provider, scoped identity and request profile.
Do not claim faster operation without results from before and after the change.

## Risk Areas

- The two parsers recognise different Nostr syntax and owner locations.
- A track response from the Index can contain credits that belong to its feed.
- Partial payloads and complete, empty collections require different writes.
- Existing contributor grouping can hide distinct claims when names match.
- Re-ingestion can skip feeds with unchanged hashes and leave old facts in place.
- Older rows cannot always establish which provider delivered them or who owns their values.
- Tag comparison and explicit tag writes share metadata helpers with display.

## Test Strategy

Use example cases that test data through several layers.
Do not write tests that only repeat helper logic. Include these cases:

- The supplied MoeFactz response.
- Identities that belong only to a feed or only to a track.
- Duplicate roles and conflicting names or keys.
- `web_page` links and malformed identifiers.
- Alternate namespace prefixes.
- Requested collections that are absent, null or empty.
- Partial refreshes and endpoint changes.
- Identical track GUIDs under different feeds.

Compare source ownership after decoding, storage, restart and display preparation.
Use injected HTTP responses to test failures and count requests.
When the Stophammer data path changes, add a test from RSS through ingestion to the query response.
Also test signed-event replay for that changed path.
Use each repository's required build, test, lint and format checks.

## Rollback And Preservation

The implementer can revert transport changes without repairing the database.
Before changing the schema, define backup and compatibility procedures for that migration.
Never reverse a production database migration by deleting its migration record.
Re-ingestion and local repair must produce a manifest.
They must retain unresolved data.
Do not change audio bytes, playlists, membership or user configuration during repair.

## Operator Visual Check

The operator paused visual checks on 2026-09-19. This phase requests no app launch.
Retain the existing gates. After the metadata work, compare their criteria with
the accepted contract. Then resume a small visual batch.

## Review

Use the [contract review](../reviews/adr-0075-metadata-contract-review.md).
No implementation packet is complete merely because its parent plan exists.
