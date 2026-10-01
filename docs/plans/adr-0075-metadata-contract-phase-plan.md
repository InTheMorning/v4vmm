# ADR 0075 Metadata Contract Phase Plan

## Status

Active plan - 2026-09-19. The source audit and the accepted contract are written.
Phase 001 is complete on 2026-09-19. Its mechanical checks are Green.
The operator paused visual checks and prioritised
metadata handling across v4vmm and MusicIndex.

ADR 0075 is Accepted from 2026-09-19. The operator recorded these decisions:

- Field scope. Reduced on 2026-09-21. The contract covers each field with an accepted rule.
  Nine field policies are deferred. ADR 0075 Decision A, amended.
- Identity placement. The track header keeps track identities. Feed identities and
  contributor identities go in separate sections with owner labels.
- Artwork fallback. The track header uses feed artwork when the track has no artwork of its own.
  This fallback needs no additional visible owner label. Stored facts retain their source and owner.
- Field-rule review. Document agents propose unresolved source priorities and conflict rules.
  The operator reviews those proposals before the dependent code packets run.

The operator released the dispatch of packets 001 to 008 on 2026-09-19.
Packet 001 is complete. Packets 002–008 contain corrected deliverables with remaining review gates open.
The operator accepted supported enclosures, fresh RSS priority, and retained discrepancy detection.
Packets 031 and 034 now contain proposed rules for the remaining fields. Their product-policy review remains open.

The operator authorized orchestration of ADR 0075 completion on 2026-09-20.
The orchestrator can dispatch bounded work that implements accepted rules.
Unaccepted product policies, external deployment, and visual acceptance remain separate gates.
Packet 030 passed independent review. Its mechanical checks are Green.

Each delegated implementation session owns one packet. Review its result before dispatching dependent work.

ADR 0075 is amended on 2026-09-21. Decision I states that MusicIndex is a cache of RSS,
and that RSS is the only provenance. The provider that carried a value is transport
evidence, and no screen labels a value with it. A MusicIndex value that disagrees with a
fresh RSS value is a stale cache record. The app reports it and directs the operator to
podping.me.

Decision I supersedes provider-ownership display work. Document-subject ownership, which
separates the channel, the item, and a person, remains in force.

Register reorganized on 2026-09-21. It holds complete work and a committed path of five
packets. The operator accepted both dependency cuts and deleted sixteen packets on the
same day.

The [API change request](musicindex-api-change-request.md) replaces packet 008 as the
request to Stophammer. It asks for four changes. Three of them need no reingestion.

The operator sent that request on 2026-09-22, and its fixes are live on 2026-09-23.
Changes 1, 2, and 3 are
[verified](musicindex-api-change-request.md#verification-against-the-deployed-api) against the
deployed API. The deployed revision stays unconfirmed.

Packets 047, 048 and 049 implement the three landed changes on 2026-09-29. The
[answer table](musicindex-api-change-request.md#what-each-answer-changes-here) records the
work that each one releases. The two upstream questions are answered.

Phases 002–006 below are outcomes, not instructions. The packet register divides
them into bounded packets. Do not give an agent a whole phase.

## Active Orchestration — 2026-09-20

| Work | Dispatch and evidence |
|---|---|
| 030 enclosure transport | Independent review passed. Six focused tests, full tests, check, Clippy, format, and normal build are Green |
| 010 item page storage | Complete and reviewed. Focused and integrated checks are Green |
| 032 transcript transport | Complete and independently reviewed. Focused and integrated checks are Green |
| 033 track language and sort text | Complete and independently reviewed. Focused and integrated checks are Green |
| 037 remaining existing collections | Complete and independently reviewed. Focused and integrated checks are Green |
| 031 title, number, and classification rules | Feed and track title rules have individual acceptance on 2026-09-21. Other policy proposals retain individual review gates |
| 034 aggregate and relationship rules | Feed publication-date source priority, removal, conflict, and stale-state rules accepted. Other proposals retain individual review gates |
| 035 comparison and discrepancy contract | Technical review passed. Individual URL comparison and action policies accepted on 2026-09-21. Implementation remains open |
| 009 RSS owner correction | Complete after compatibility corrections and independent review. Focused and integrated checks are Green |
| 016 request baseline | Complete and reviewed. Thirteen isolated measurement cases are Green. No app launch |
| 011 provider snapshot design | Complete. Technical review passed after corrections |
| 012 provider snapshot migration | Implementation, technical review, and mechanical checks complete. The presentation gate remains open and paused |
| 014 provider observation retention | Implementation, technical review, and mechanical checks complete on 2026-09-21. Shared writer and Library track-detail slice. Presentation gate open and paused |
| [013 verified snapshot replacement](../tasks/adr-0075-task-013-verified-snapshot-replacement.md) | Implementation, technical review, and mechanical checks complete on 2026-09-21. Bounded RSS identity contracts and typed local reads. Inherited presentation gates open and paused |
| [038 Library reader retention](../tasks/adr-0075-task-038-library-reader-observation-retention.md) | Implementation, technical review, and mechanical checks complete on 2026-09-21. Presentation gate open and paused |
| [039 feed checks and updates](../tasks/adr-0075-task-039-feed-check-and-update-observation-retention.md) | Implementation, technical review, and mechanical checks complete on 2026-09-21. Three converted roots, one dead command deleted, receipts kept across route repair. Presentation gate open and paused |
| [017 named request profiles](../tasks/adr-0075-task-017-named-request-profiles.md) | Implementation, technical review, and mechanical checks complete on 2026-09-21. Ten named profiles, eight converted request sites, one new guard. No visual gate |
| [018 Part A, request identity and sharing](../tasks/adr-0075-task-018-request-reuse-and-freshness.md#implementation-result-part-a---2026-09-21) | Implementation, technical review, and mechanical checks complete on 2026-09-21. One shared owner, single-flight sharing, an app sequence counter, and the explicit refresh intent. The orchestrator added an abandoned-request guard. No visual gate |
| [018 Part B, completed response reuse](../tasks/adr-0075-task-018-request-reuse-and-freshness.md#implementation-result-part-b---2026-09-22) | Implementation, technical review, and mechanical checks complete on 2026-09-22. The accepted windows, capacity, feed-wide invalidation, RSS sharing, and the converted Index routes. Each of the four measurement targets is met. No visual gate |

Code agents preserve the existing packet 030 working changes. They do not commit or run the app.
The orchestrator reviews each diff and runs the integrated checks.
Reviews go directly to the operator. Existing status documents record completion without a new review document.

Integrated verification on 2026-09-22: 1,695 unit tests and 273 architecture tests are Green. Ten documentation examples remain ignored.
The final full suite used four test threads. Packets 013, 014, and 038 record earlier failures and their corrections.
Packet 039 adds eight unit tests and one guard. Root corrected seven defects in its new test code before acceptance.

Packet 017 adds fifteen unit tests and one guard. It also repaired a guard that the Decision I amendment broke.
That guard asserted one exact sentence of `AGENTS.md`, which the amendment rewrapped. It now compares collapsed whitespace.

Packet 018 Part A adds eleven unit tests and one guard. The orchestrator added the abandoned-request guard and its test during review.
A panic in a request closure had left each joined caller waiting without end.

Format, compile, and strict Clippy checks are Green. The normal desktop binary build is Green after testing.
These results cover the combined code changes through packets 009, 010, 012, 013, 014, 017, 030, 032, 033, 037, 038, and 039. They also cover both parts of packet 018.

Packet 012 also passed 42 Python fixture tests and isolated CLI verification.
No application launch, production-data change, or visual acceptance occurred.

## Goal

Preserve metadata meaning from RSS through MusicIndex, local storage and both
app entry routes. Explain missing values, inherited values, conflicts and refresh failures.
Reduce repeated requests after automated checks cover the contract.

## Scope And Non-Goals

Begin with websites, Nostr identifiers, contributor credits and their provenance.
Provenance is the element in the RSS document that asserted a value. ADR 0075 Decision I.

The contract then covers each field with an accepted rule. ADR 0075 Decision A, amended on 2026-09-21, sets that scope.

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
| 005 | Display the accepted contract through both app routes | Share contributor and entity actions with document-subject owner labels. Test the view models. Visual acceptance remains pending during the operator pause |
| 006 | Reconcile old records and deployed Index data | Dropped on 2026-09-21. Its four packets are deleted. ADR 0075 completes at the reduced scope |

Phases 001, 002, and 003 are complete. Phase 006 is dropped.
Phases 004 and 005 continue only through the committed path in the register.
Write each implementation packet after its prerequisites are complete.
Do not implement more than one packet in one session.

## Packet Register

A packet is the unit of dispatch. A phase is not.
This register holds two groups: complete work and the committed path.
The committed path is what an agent may dispatch now.
Deleted work is not planned work, and no agent may dispatch it.

Reorganized on 2026-09-21. This grouping replaces the earlier phase-ordered register.
Git history holds the earlier phase tables and the dependency notes of each deleted packet.
A later need for deleted work starts with a new decision record that states its reason.

### Complete

Code packets. Technical review and integrated checks are Green.

| Packet | Outcome |
|---|---|
| 001 | Contributor claim transport |
| 009 | Direct RSS identity ownership, with retained rejected evidence |
| 010 | Item page stored as a track `web_page` identity fact |
| 012 | Provider snapshot migration to schema 12 |
| 013 | Verified snapshot replacement and typed local reads |
| 014 | Shared observation writer and Library track-detail retention |
| 030 | Enclosure claim transport |
| 032 | Transcript claim transport |
| 033 | Track language and artist sort transport |
| 037 | Remaining collection transport |
| 038 | Library reader observation retention |
| 039 | Feed check and feed update observation retention |

Document packets. Their deliverables exist. Some field policies stay open.

| Packet | Deliverable |
|---|---|
| 002 | Shared example corpus |
| 003 | Nostr syntax and supported purpose values |
| 004 | Collection completeness rules |
| 005 | Description, artwork and publisher rules |
| 006 | Artist, language and date rules |
| 007 | Links and media rules |
| 008 | Upstream decision request. The [API change request](musicindex-api-change-request.md) replaces it as the request |
| 011 | Provider snapshot schema |
| 016 | Request and write baseline |
| 031 | Title, number and classification rules. Five policies stay open |
| 034 | Aggregate and relationship rules. Four policies stay open |
| 035 | Comparison and discrepancy rules |

### The Committed Path

These five packets carry ADR 0075 to a visible result. Dispatch them in this sequence.

| Packet | Kind | Outcome | Needs |
|---|---|---|---|
| [017](../tasks/adr-0075-task-017-named-request-profiles.md) | Code | Name the requests that the Library route and the Index route make. Complete on 2026-09-21 | 013, 014, 016, and both accepted cuts below |
| [018](../tasks/adr-0075-task-018-request-reuse-and-freshness.md) | Code | Request identity, sharing, expiry, explicit refresh, and response order. Complete on 2026-09-22 | 016, 017, and nine policies that the operator accepted on 2026-09-21 and 2026-09-22 |
| 045 | Code | Replaced on 2026-09-24 by the ADR 0076 packets. The [ADR 0076 phase plan](adr-0076-playlist-rss-check-phase-plan.md) registers them | 018 and ADR 0076 |
| [020](../tasks/adr-0075-task-020-stored-value-projection.md) | Code | One shared projection of the stored values with their owners. Implemented on 2026-09-24. Visual gate open and paused | ADR 0076 packet 002 |
| [022](../tasks/adr-0075-task-022-track-header-identities.md) | Code | The track header view model, limited to track identities. Implemented on 2026-09-28. Mechanical checks Green. Visual gate open and paused | 020 |

Packet 017 is complete. It named ten requests and models no provider profile, because a
provider is transport and not a source. Packet 018 is complete. ADR 0076 is Accepted on
2026-09-24. Its packets replace packet 045. The ADR 0076 phase plan registers them and packet 020.

Part B of packet 018 ends the repeated fetch that the baseline measured. Part A implements the accepted ADR 0075 rules and changes no sequential request count.
Packet 020 applies the rules that the operator accepted. Packet 022 puts them on screen.

Packet 020 covers accepted fields only. A field with an open policy stays unattributed.
That keeps packets 031 and 034 off the critical path.

### Deleted Work

The operator deleted sixteen packets on 2026-09-21: 015, 019, 021, 023 to 029, 036,
and 040 to 044. They are not planned work, and they carry no obligation.

Git history holds each deleted row, its outcome, and its dependencies. ADR 0075 completes
at the reduced scope in Decision A, amended. A later need for any of this work starts with
a new decision record that states its reason.

### Accepted Dependency Cuts

The operator accepted both cuts on 2026-09-21.

1. Packet 017 does not need packet 008. Packet 008 asks for an upstream decision that does
   not exist. Packet 017 names the requests against the contract that exists today.
2. Packet 017 does not need packets 038 to 044. Packet 017 covers the Library and Index
   detail routes. Other callers keep their current behavior, and packets 040 to 044 are
   deleted.

### Unassigned Work

| Item | Owner |
|---|---|
| The evidence store has no retention limit and no history expiry | No packet |
| Five `Option<i32>` count fields narrow an upstream `i64` in `src/api.rs` | No packet |
| The retained discrepancy tables of ADR 0075 Decision G. Deleted packet 036 implemented them. ADR 0076 supersedes Decision G on 2026-09-24 | No packet. The ADR 0076 comparison packet uses the tables or deletes them |
| A visible retry action for the capsules that packet 014 retains after a storage failure. Deleted packet 019 owned it | No packet. The storage-failure presentation gate of packet 014 depends on it |
| A field rule for the new upstream `last_build_date` claim type | Closed. ADR 0076, amended on 2026-09-26: the app ignores the field and never uses `lastBuildDate` as a date or a change signal |
| `Feed.name`, `Track.name`, and `Track.feed_url` no longer arrive from the deployed API | [Packet 046](../tasks/adr-0075-task-046-remove-undeclared-api-fields.md), Implemented on 2026-09-26. Mechanical checks Green. No visual gate. `TrackContext::feed_url` gives the feed address |
| Stophammer removed its public artist credits on 2026-04-08, in commit `a16a720` | Closed. ADR 0077 packet 001 deleted the ADR 0045 binding on 2026-09-24 |
| Search rows from the new upstream summary fields, without a detail request for each hit | [Packet 047](../tasks/adr-0075-task-047-search-rows-from-summary-fields.md), Implemented on 2026-09-29. Mechanical checks Green. Visual gate open and paused |
| The separate track and feed artwork fields of MusicIndex change 2 | [Packet 048](../tasks/adr-0075-task-048-separate-track-artwork.md), Implemented on 2026-09-29. Mechanical checks Green. Visual gate open and paused |
| `feed_release_pubdate` gives an oldest-item date as a release date, against the accepted feed release-date evidence rule. `TDRC` gets it | [Packet 049](../tasks/adr-0075-task-049-no-derived-release-date.md), Implemented on 2026-09-29. Mechanical checks Green. Visual gate open and paused |
| `TrackView.artwork` has no reader. `TrackView::display_artwork_url` gives the track artwork | No packet. Packet 048 recorded it on 2026-09-29 |
| A feed artwork fallback to the first track image puts a track value on a feed (`index_feed_artwork_url`) | No packet. Packet 048 recorded it on 2026-09-29 |
| The app reads no RSS channel `pubDate`. The accepted feed publication-date rules have no source. The album page shows the oldest-item date as "Release Date" | [Packet 050](../tasks/adr-0075-task-050-feed-dates-by-owner.md), Ready on 2026-09-30. The operator decided both details |
| A guard that compares the fields that the app decodes with a stored copy of the MusicIndex contract. The incident: Stophammer removed `artist_credit` on 2026-04-08, and the app read `None` for five months without a report | [Packet 051](../tasks/adr-0075-task-051-contract-field-guard.md), Ready on 2026-09-30. The operator accepted the design. ADR 0060 packets 005 and 006 deleted the six schema-less types |
| A typed RSS refresh-failure state. ADR 0075 §2 and §6 require the app to report a failed refresh. Deleted packet 019 owned it | No packet |
| Combined isolation, restart, rollback, and superseded-response tests for provider snapshots. Deleted packet 015 owned them | No packet |

## Requirements Before Dispatch

ADR 0057 requires an Accepted decision before implementation. ADR 0075 is Accepted.
Packet 001 is complete. Document corrections do not establish full field-rule coverage or acceptance.

The 2026-09-20 completion request releases orchestration of accepted work.
Each dependent policy still needs acceptance before its code runs.
Packet 030 is complete and passed independent review.

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
| 031, 034, 035 | The written inputs listed above and the completion orchestration authorization |
| 030, 032, 033 | A reviewed implementation packet, accepted affected rules, completed dependencies, and recorded orchestrator dispatch |
| 016 | Accepted ADR 0075 and the current request paths. The output measures the baseline before request changes |
| Committed-path code packets 017, 018, 045, 020, 022 | Completed dependencies from the register, accepted affected policies, and a reviewed Ready implementation packet |

Code that relies on changed Stophammer behavior also needs its accepted decision and compatible implementation evidence.
Packet 030 preserves fields already supplied upstream. It needs no upstream API addition.
The same distinction applies to the existing transcript and track scalar transport gaps.

A written decision request does not establish either result.
A live rollout also needs evidence that the configured endpoint supports the required contract.
Existing-contract code does not need unrelated upstream changes.

Each storage code packet must define its applicable schema, backup, rollback, and verification requirements before dispatch.
Each request code packet must define freshness, response order, and measured request limits before dispatch.
Each presentation packet must define typed actions and handling of unresolved ownership before dispatch.
ADR 0075 Decision I removes provider ownership from every presentation packet.
Dispatch follows the completion authorization after these prerequisites are met.

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
