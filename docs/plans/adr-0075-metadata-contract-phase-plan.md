# ADR 0075 Metadata Contract Phase Plan

## Status

Active plan - 2026-09-19. The source audit and the accepted contract are written.
Phase 001 is complete on 2026-09-19. Its mechanical checks are Green.
The operator paused visual checks and prioritised
metadata handling across v4vmm and MusicIndex.

ADR 0075 is Accepted from 2026-09-19. The operator recorded these decisions:

- Field scope. The contract covers every metadata field. Each field keeps a separate rule.
- Identity placement. The track header keeps track identities. Feed identities and
  contributor identities go in separate sections with owner labels.
- Artwork fallback. The track header uses feed artwork when the track has no artwork of its own.
  This fallback needs no additional visible owner label. Stored facts retain their source and owner.
- Field-rule review. Document agents propose unresolved source priorities and conflict rules.
  The operator reviews those proposals before the dependent code packets run.

The operator released the dispatch of packets 001 to 008 on 2026-09-19.
Packet 001 is complete. Packets 002–008 have corrected deliverables with remaining review gates open.
The operator accepted supported enclosures, fresh RSS priority, and retained discrepancy detection.
The field inventory assigns remaining coverage to packets 031 and 034. Their rules are not yet written.
The operator holds the dispatch of every code packet.

Complete one packet per session.

Phases 002–006 below are outcomes, not instructions. The packet register divides
them into bounded packets. Do not give an agent a whole phase.

## Goal

Preserve metadata meaning from RSS through MusicIndex, local storage and both
app entry routes. Explain missing values, inherited values, conflicts and refresh failures.
Reduce repeated requests after automated checks cover the contract.

## Scope And Non-Goals

Begin with websites, Nostr identifiers, contributor credits and their provenance.
Provenance records a value's owner, source, extraction path and observation time.

The contract then covers every other metadata field. The operator selected that scope.

Before you change the fallback of a field, audit the current rule and write a new rule.
Cover description, artwork, publisher, artist text, language, explicit state, dates,
links, transcripts and enclosures.
The [field inventory](../schema/adr-0075-metadata-field-inventory.md) also assigns titles, numbers, sort text, medium, aggregates, and relationships.
Each field rule states the owner, the source order, the conflict result,
and the displayed value when no source supplies the field.

Distinguish accepted rules from proposed rules. Complete operator review of each proposal before coding its dependent packet.
Keep identity, metadata, payment routes and tag-write policies separate.

Exclude these changes from this work:

- Merging people.
- Redesigning broadcasting.
- Changing configuration formats.
- Rebuilding production data.
- Closing inherited visual gates.

## Assumptions And Decisions

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) is Accepted.
  Its Status section records the field scope and the identity placement.
  A field rule must exist before the packet that changes that field runs.
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
| 001 | Preserve contributor claim fields in the app's type for API data | Complete on 2026-09-19. [First packet](../tasks/adr-0075-task-001-contributor-claim-transport.md). Eleven tests check JSON round trips and compatibility. No schema or UI change |
| 002 | Agree the RSS extraction and Index ownership rules | A Stophammer ADR and a shared list of examples. Tests check parsing, ingestion and queries. Preserve compatibility. Do not crawl live feeds again yet |
| 003 | Store provider snapshots and coverage state with their owners | Write the exact schema and migration packet first. Test transactions, empty refresh, rollback, restart and isolation between providers |
| 004 | Share detail requests and code that prepares data for display | Define request includes, scoped identity, response failure handling and fallback for each field. Test partial responses, request counts and route parity |
| 005 | Display the complete contract through both app routes | Share contributor and entity actions with owner labels. Test the view models. Visual acceptance remains pending during the operator pause |
| 006 | Reconcile old records and deployed Index data | Produce a read-only repair report first. Repair records from source evidence. Define how to crawl or ingest feeds again. Check replicas and preservation |

Write later implementation packets after their prerequisites are complete.
Do not implement all phases in one session.
Phase 001 leaves storage and display preparation for later packets.

## Packet Register

This register divides phases 002-006 into bounded packets.
A packet is the unit of dispatch. A phase is not.
A packet number with a link has a written packet file. A number without a link has none.

A written packet is not a dispatched packet. The operator releases each dispatch.

In the Needs column, a packet number means its completed deliverable and recorded technical review.
A packet file alone does not satisfy that dependency.
Document authors can record unresolved policy proposals as inputs to later document work.
Code authors need operator acceptance of each policy their code implements.

A document packet produces rules, examples or measurements. It changes no code.
A code packet changes code in this repository only.

### Phase 002, Rules And Corrections

| Packet | Kind | Outcome | Needs |
|---|---|---|---|
| [002](../tasks/adr-0075-task-002-shared-example-corpus.md) | Document | A shared corpus of RSS and API examples with an expected owner for each value | ADR 0075 |
| [003](../tasks/adr-0075-task-003-nostr-syntax-and-purposes.md) | Document | The supported Nostr syntax and the supported `podcast:txt` purpose values, with checked examples | ADR 0075 |
| [004](../tasks/adr-0075-task-004-collection-completeness-rules.md) | Document | Completeness and transition rules for each response collection | Packet 002 |
| [005](../tasks/adr-0075-task-005-field-rules-description-artwork-publisher.md) | Document | Field rules for description, artwork and publisher | Packet 002 |
| [006](../tasks/adr-0075-task-006-field-rules-artist-language-dates.md) | Document | Field rules for artist text, language, explicit state and dates | Packet 002 |
| [007](../tasks/adr-0075-task-007-field-rules-links-and-media.md) | Document | Field rules for websites, page links, transcripts and enclosures | Packets 002 and 003 |
| [008](../tasks/adr-0075-task-008-stophammer-decision-request.md) | Document | A decision request for Stophammer, with the app's required parser and API behavior | Packets 002–007 |
| 009 | Code | Correct the RSS owner rule in `src/rss/enrich.rs`. Remove the invented `podcast:txt` extraction path | Packets 003 and 007 |
| 010 | Code | Store an item link as a track `web_page` identity fact in `src/rss/subscribe.rs` | Packet 007 |

### Phase 003, Storage

| Packet | Kind | Outcome | Needs |
|---|---|---|---|
| 011 | Document | Provider snapshots, field coverage, raw evidence, and durable discrepancy records | Packets 004–008, field inventory, and document packets 031, 034, 035 |
| 012 | Code | The migration, with the backup and the rollback procedure | Packet 011 |
| 013 | Code | Atomic replacement of one provider snapshot, with the empty-collection rule | Packets 011 and 012 |
| 014 | Code | Keep the source evidence before the app cleans the text and before it applies fallback | Packet 011 |
| 015 | Code | Tests for provider isolation, restart, rollback and superseded responses | Packets 012, 013 and 014 |

### Phase 004, Requests And Projections

| Packet | Kind | Outcome | Needs |
|---|---|---|---|
| 016 | Document | Measured request counts and database writes for the five scripted cases | ADR 0075 |
| 017 | Code | Named request profiles with scoped identity, summary coverage, and required collections | Packets 008, 016, 030, 032, 033, and applicable packet 034 follow-ups |
| 018 | Code | Cache key, expiry, explicit refresh and response order | Packets 016, 017, and packet 035's freshness interface |
| 019 | Code | RSS enrichment reports a failed request separately from absent data | Packet 004 |
| 020 | Code | One shared projection with accepted field rules and operation-specific enclosure support | Packets 005, 006, 007, 013, 018, 031, 034, and required transport corrections |
| 021 | Code | Route parity tests for facts, selected values, and discrepancy state | Packets 020 and 036 |

### Phase 005, Presentation

| Packet | Kind | Outcome | Needs |
|---|---|---|---|
| 022 | Code | The track header view model, limited to track identities | Packet 020 |
| 023 | Code | The feed section and the contributor section view models, with owner labels | Packet 022 |
| 024 | Code | Shared typed actions for both routes | Packets 022 and 023 |
| 025 | Code | Contributor sections on the Index detail route | Packet 024 |

### Phase 006, Reconciliation

| Packet | Kind | Outcome | Needs |
|---|---|---|---|
| 026 | Document | A read-only repair report of old records, with the unresolved cases | Packet 015 |
| 027 | Code | Repair from source evidence, with a manifest and retained unresolved rows | Packet 026 |
| 028 | Document | Evidence of the deployed Index revision and its generated API contract | Packet 008 |
| 029 | Document | The procedure to crawl or ingest feeds again, with signed-event and replica checks | Packets 008 and 028 |

### Additional Packets From The Correction

These numbers preserve the existing register. Their phase assignment determines dependencies, not their numeric order.
A row without a linked packet file remains planned work.

| Packet | Phase | Kind | Outcome | Needs |
|---|---|---|---|---|
| [030](../tasks/adr-0075-task-030-enclosure-claim-transport.md) | 002 | Code | Preserve enclosure owner, position, and observation time in the app DTO | Accepted ADR 0075 and the inspected existing wire contract. Draft, dispatch held |
| 031 | 002 | Document | Rules for titles, album/feed titles, track/disc/season numbers, artist sort text, medium, and release kind | Field inventory and packets 005–007 |
| 032 | 002 | Code | Preserve the existing upstream transcript collection and its claim fields | Packet 007, field inventory, and a reviewed bounded transport packet |
| 033 | 002 | Code | Preserve upstream track language and artist sort text | Packet 006, field inventory, and a reviewed bounded transport packet |
| 034 | 002 | Document | Rules for aggregates, remaining scalar fields, platform claims, remote items, and publisher relationships | Field inventory and packets 004–008. Assign bounded transport follow-ups for uncovered collections |
| 035 | 002 | Document | Description and URL comparison, freshness interface, discrepancy identity, and lifecycle cases | Accepted Decisions F and G, packets 005 and 007 |
| 036 | 004 | Code | Detect discrepancies and retain active/resolved evidence through restart and snapshot replacement | Packets 011, 013, 014, 018, 020, and accepted packet 035 rules |

Packet 035 defines what comparison requires from freshness state. Packet 018 supplies the numeric expiry and refresh policy.
Packet 011 designs retention before packet 036 writes evidence. The schema must support replacement without erasing discrepancy history.

Packet 020 exposes operation-specific supported-format selection. Deferred playback work keeps its separate gate.
No packet implements a MusicIndex update hook or sends an update signal.

Packets 002 to 008 need no further decision to write their documents and proposals. Write them first.
Proposed field rules require operator acceptance before the dependent code packets run.
Packets 009 and later need the outputs of the document packets above them.
Do not write a packet before its inputs exist.

## Requirements Before Dispatch

ADR 0057 requires an Accepted decision before implementation. ADR 0075 is Accepted.
Packet 001 is complete. Document corrections do not establish full field-rule coverage or acceptance.
The operator's dispatch hold still applies to every code packet, including packets 030, 032, 033, and 036.

The following prerequisites apply to individual packets, not whole phases.
Document work can produce a decision request before that decision exists.
Schema design can precede its migration, backup procedure, and tests.
Evidence documents can precede the operations that use their evidence.
Do not give an agent one task that changes both repositories.
Review each completed packet before you dispatch its dependent packet.

| Packet | Required input before dispatch |
|---|---|
| 001 | Accepted ADR 0075 and the Ready packet's field contract and supplied response |
| 002, 003 | Accepted ADR 0075 and access to their cited evidence. No completed corpus or Stophammer decision is required |
| 004, 005, 006 | Completed packet 002 and its technical review |
| 007 | Completed packets 002 and 003 and their technical reviews |
| 008 | Completed packets 002–007 and their technical reviews. The output requests the Stophammer decision |
| 011 | Corrected packets 004–008, the field inventory, and document packets 031, 034, 035. Record unresolved policies explicitly |
| 031, 034, 035 | The written inputs listed above and operator release of their document dispatch |
| 030, 032, 033, 036 | A reviewed implementation packet, accepted affected rules, completed dependencies, and operator release |
| 016 | Accepted ADR 0075 and the current request paths. The output measures the baseline before request changes |
| 026 | Completed packet 015. Read-only access to the records under inspection |
| 028 | Completed packet 008. Read-only access to deployment evidence. This packet establishes the deployed revision |
| 029 | Completed packets 008 and 028. The accepted upstream decision and relevant event and replica contracts |
| Code packets 009–025 and 027 | Completed dependencies from the register, accepted affected policies, and a reviewed Ready implementation packet |

Code that relies on changed Stophammer behavior also needs its accepted decision and compatible implementation evidence.
Packet 030 preserves fields already supplied upstream. It needs no upstream API addition.
The same distinction applies to the existing transcript and track scalar transport gaps.
A written decision request does not establish either result.
A live rollout also needs evidence that the configured endpoint supports the required contract.
Existing-contract code does not need unrelated upstream changes.

Each storage code packet must define its applicable schema, backup, rollback, and verification requirements before dispatch.
Each request code packet must define freshness, response order, and measured request limits before dispatch.
Each presentation packet must define typed actions and handling of unresolved ownership before dispatch.
The operator's dispatch hold applies after these prerequisites are met.

The operator answered the original scope and identity-placement questions on 2026-09-19.
ADR 0075 also records the accepted artwork fallback and field-rule review process.
The [review](../reviews/adr-0075-metadata-contract-review.md#operator-decisions--2026-09-19) records the answers.

## Schema And API Implications

Phase 001 added optional fields to the data transfer object (DTO).
The DTO is the Rust type that represents API data.
Phase 001 changed neither the HTTP contract nor the schema.
Existing payloads remain readable without invented provenance.

Existing JSON storage includes newly supplied fields, because it serialises the expanded DTO.
Typed storage and display remain incomplete after phase 001.
The [review](../reviews/adr-0075-metadata-contract-review.md#fields-that-later-layers-still-lose)
lists the fields that each later layer loses.

Phase 003 must record the provider separately from the source assertion.
It must also record whether a response contains the complete collection.
Discrepancy records retain both provider observations and actual recorded times, including after restart or snapshot replacement.
Packet 035 must define comparison and resolution before the schema fixes their durable representation.
Existing `source` columns currently serve several meanings.
The migration must not silently reinterpret them.
Define response completeness before writing replacement code.
An HTTP success, populated list or empty list alone does not prove complete owner coverage.

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
The packet must set numeric request bounds for the scripted cases before implementation.
Count requests on success and failure paths, including fallback and retries.
Record response bytes and latency if the packet claims improvements in either measure.
Do not claim faster operation without results from before and after the change.

## Risk Areas

- The two parsers recognize different Nostr syntax and owner locations.
- A track response from the Index can contain credits that belong to its feed.
- Partial payloads and complete, empty collections require different writes.
- Existing contributor grouping can hide distinct claims when names match.
- Re-ingestion can skip feeds with unchanged hashes and leave old facts in place.
- Older rows cannot always establish which provider delivered them or who owns their values.
- Tag comparison and explicit tag writes share metadata helpers with display.
- Source-text cleaning can remove values before a later helper serialises its DTO as raw evidence.
- A cached response can remain stale without an explicit expiry and refresh rule.

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
- Track credits changing to inherited feed credits, then to no credits.
- An older response arriving after a newer request, including when source timestamps match.
- Invalid or placeholder source text remaining available as evidence without becoming an active identity.
- A rejected primary enclosure followed by a supported alternate enclosure.
- Different operation capabilities with the same enclosure priority order.
- RSS and Index descriptions that differ only in HTML or equivalent whitespace.
- A meaningful description change, repeated mismatch, later agreement, and failed refresh during an active discrepancy.
- Discrepancy evidence surviving restart, endpoint changes, and replacement of its source snapshots.
- Track summaries that omit fields supplied by full detail.
- Artwork facts with no track image, and separate owners that supply the same image URL.

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
the accepted contract. Resume a small visual batch only when the operator resumes those checks.

## Document Correction Checks

Checked on 2026-09-19:

- Local links: Green. The checker resolved 530 links in 24 files.
- Identity examples: Green. The guard checked 13 valid examples, two intentional invalid examples, and three decoded vectors.
- Guard controls: Green. Known-key, uppercase, unknown-TLV, and eight rejection controls passed.
- Diff whitespace: Green.
- Shared STE check: no structural findings in changed Markdown prose or guard docstrings. Lexical findings remain.

The raw STE result is not Green. Technical terms retain their source and contract meanings.
The checker cannot prove full standard compliance.

Source inspection used app commit `d3c6ee4` and Stophammer commit `a220f44`.
The field inventory records the inspected boundaries and assigns missing rules.
This correction changes documents and their example guard. It changes no application behavior.
Application tests were not rerun for these changes. Document checks do not close the remaining operator gates.

## Review

Use the [contract review](../reviews/adr-0075-metadata-contract-review.md).
No implementation packet is complete merely because its parent plan exists.
