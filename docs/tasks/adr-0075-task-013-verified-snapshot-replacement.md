# ADR 0075 Task 013: Verified Snapshot Replacement

Status: Implementation, technical review, and mechanical checks complete - 2026-09-21.
Visual gates remain open and paused. Do not request a visual batch.

Packet 012 passed technical and mechanical review. Its presentation gate remains open.
Packet 014 completed implementation, technical review, and mechanical checks on 2026-09-21. Its presentation gate remains open and paused.

Root approved the corrected direct RSS completeness contract for this bounded implementation on 2026-09-21.
Packet 004's applicable rules preserve incomplete evidence and permit only verified, provider-scoped replacement.
Its wider document review gate remains open. This technical approval accepts no new field policy or visual result.

## Goal

Extend the existing observation transaction with verified provider snapshot replacement.
Add local reads for accepted snapshots and separate refresh evidence.
Connect those reads to both Library track-context commands and the mounted Library frame.

The first production contract covers direct RSS `source_ids` for proven feed and track owners.
MusicIndex replacement remains disabled because the inspected evidence does not establish its deployed completeness contract.
This packet does not complete active identity display, field selection, or observation retention for every caller.

## Authority And Evidence

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), decisions 1, 2, 5, and 6.
- [Packet 004](adr-0075-task-004-collection-completeness-rules.md) and its [collection rules](../schema/adr-0075-collection-completeness-rules.md).
- [Storage design](../schema/adr-0075-provider-snapshot-storage.md), coverage, heads, transaction boundaries, and local reads.
- [Packet 009](adr-0075-task-009-rss-owner-and-nostr-extraction.md) and the [identity syntax contract](../schema/adr-0075-identity-syntax-contract.md).
- [Packet 012](adr-0075-task-012-provider-snapshot-migration.md), completed schema 12 and preservation.
- [Packet 014](adr-0075-task-014-provider-observation-retention.md), the existing writer and selected Library caller.
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md#phase-003-storage), storage order and later projections.

Inspection used app revision `a12521e7510b3d05cd4fc097a370f1f965145aad` plus the current uncommitted ADR 0075 implementation.
The packet depends on packet 014's final working-tree implementation, not that revision alone.
The inspected Stophammer revision is `a220f441d57640912eed9b190d7c194284f87a38`.
No deployed service was inspected. Local source does not establish the deployed revision.

The operator requires a separate review for each field policy.
This packet defines technical admission and storage boundaries. It accepts no new field policy.

## Files To Inspect

Read the authority documents and these exact owners before editing:

- [AGENTS.md](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- [Observation types](../../src/provider_observation.rs), request tokens, retained failures, recorder, and receipts.
- [Observation writer](../../src/db/provider_observations.rs), `begin`, `record_once`, `record`, and `write_evidence`.
- [MusicIndex adapter](../../src/provider_observation/musicindex.rs), property presence and owner resolution.
- [HTTP capture](../../src/provider_observation/http.rs), complete bodies, failures, and decoder evidence.
- [Schema 12](../../src/db/provider_snapshot_schema.rs) and [database owner](../../src/db.rs).
- [RSS enrichment](../../src/rss/enrich.rs), the single DOM parse, matching, owner evidence, and `retain_dom_evidence`.
- [RSS identity decoder](../../src/rss/identity.rs) and [RSS exports](../../src/rss/mod.rs).
- [Library queries](../../src/application/queries/library.rs), both track-context commands and local fallback.
- [Feed service](../../src/feed_service.rs), observed detail fetch and local identity hydration.
- [Subscription service](../../src/subscribe_service.rs), existing RSS resource selection and matching inputs.
- [Track context](../../src/metadata.rs), constructors, cloning, and sanitization.
- [Library callback](../../src/library/app_impl.rs), `load_track_source_context` and local command result handling.
- [Library state](../../src/view_models/library.rs), mounted context and retained failures.
- [Local identity reads](../../src/local_identity.rs) and [legacy ingestion](../../src/identity_ingest.rs), for exclusion checks.
- [Architecture tests](../../tests/architecture_tests.rs), packet 014's live-root guard and source boundaries.

For the remaining Index gaps, inspect Stophammer read-only:

- `/home/citizen/build/stophammer/src/query.rs`: `ListQuery::includes`, `build_feed_response`, and `build_track_response`.
- `/home/citizen/build/stophammer/src/db.rs`: `get_effective_source_contributor_claims_for_track`.
- `/home/citizen/build/stophammer/src/openapi.rs`: nested pagination documentation.

## Files To Change

| File | Permitted change |
| --- | --- |
| `src/provider_observation.rs` | Verified coverage types, collection outcomes, typed local-read results, and consumed recorder results. |
| `src/provider_observation/contracts.rs` | New minimum registry and private proof construction. |
| `src/db/provider_observations.rs` | Extend the existing response transaction. Add snapshot reuse, head updates, and local reads. |
| `src/rss/enrich.rs` | Group direct identity evidence and establish coverage during the existing DOM parse. |
| `src/metadata.rs` | Carry typed provider collections separately from API DTOs and current RSS observations. |
| `src/application/queries/library.rs` | Load provider state in both track-context commands, including remote failure and local-only results. |
| `src/feed_service.rs` | Share exact local subject and resource lookup where both commands need it. |
| `src/subscribe_service.rs` | Expose existing RSS request inputs for exact read lookup, only if needed. |
| `src/view_models/library.rs` | Consume typed results in existing Library state without adding display selection or presentation. |
| `src/library/app_impl.rs` | Preserve typed state through existing command callbacks, only if the current context assignment is insufficient. |
| `src/discover/tests.rs` | Add empty provider-state initializers only when new `TrackContext` fields require them. |
| `tests/architecture_tests.rs` | Extend the situational ADR 0075 guard for the registry, transaction, and live readers. |
| This packet | Record implementation evidence, limits, and remaining gates. |

Keep unit tests beside their owners. Existing `TrackContext` literals in permitted files must retain their other fields.
Report any additional required file before editing it.

## Minimum Completeness Registry

Use one registry shared by admission and storage validation.
Only its private constructor can issue proof that permits replacement.
A caller-supplied boolean, contract name, include token, or DTO vector cannot create that proof.

Each entry names provider kind, operation, collection, decoder version, owner rules, coverage rules, and pagination assumptions.
Retain its versioned identifier and concrete proof in the observation and coverage records.
Bind proof to the captured response, requested resource, declared subject, and exact coverage scope.
The writer must reject mismatched proof before any replacement commits.

| Candidate | Evidence and required result |
| --- | --- |
| Direct RSS `source_ids` | Reviewed technical entry `rss-direct-source-ids-v1`. The app controls complete document decoding and direct-child enumeration. Apply the requirements below. |
| MusicIndex detail collections | No enabled production entry. Upstream nested pagination documentation conflicts with the inspected handler behavior. No verified deployment or response completeness signal resolves this. |
| MusicIndex inherited contributors | Retain declared feed owners. The observed fallback branch does not establish a deployed versioned contract. Do not clear track or feed snapshots. |
| RSS scalar fields and other collections | Retain current unknown or partial evidence. This packet registers no scalar, contributor, enclosure, transcript, relationship, or payment contract. |
| Unknown provider, operation, collection, or contract version | Retain evidence and a typed reason. Preserve all accepted heads. |

No allowlist entry may infer an Index deployment from its endpoint string.
Do not add a configuration override that declares an unverified server complete.
Packet 017 will reuse this registry. It owns named request profiles and expanded request coverage.

### Direct RSS Identity Contract

The contract describes the direct identity assertions present in one complete RSS document.
It makes no claim about complete feed membership, historical items, scalar fields, or other resources.
The accepted syntax and namespace rules remain those from packet 009.

1. Require successful HTTP capture, complete body bytes, successful decoding, and the existing supported RSS root.
2. Reuse the existing DOM. Do not fetch again or parse XML again to establish proof.
3. Require one unambiguous channel owner. Repeated channel owners prevent complete coverage.
4. Count raw feed GUID declarations before scalar-text helpers discard empty or malformed values.
5. Resolve the feed from its exact observed GUID only when one structurally valid, nonblank declaration exists.
6. Use the exact requested resource as the feed scope only when no feed GUID declaration exists.
7. Keep empty, whitespace-only, structurally malformed, and repeated feed GUID declarations partial and unresolved.
8. Do not merge a resource subject with a later GUID subject.
9. Limit complete track coverage to the matched item with an exact, nonempty observed item GUID.
10. Keep empty, whitespace-only, structurally malformed, or repeated item GUID declarations partial and unresolved for that track.
11. Require exact agreement when the request supplies an item GUID. Enclosure matching cannot override a conflicting GUID.
12. Treat duplicate items with that GUID as ambiguous for that track subject.
13. Keep ambiguous scopes partial. Preserve their evidence and prior snapshots.
14. Enumerate all direct `podcast:txt` candidates for each eligible owner through the existing namespace and syntax checks.
15. Create one grouped `source_ids` coverage row for each eligible owner, including owners with zero candidates.
16. Keep each member's original owner, source text, validation, order, attributes, extraction path, and body locator.

An unusable declaration is different from an absent declaration. Do not create resource-scoped ownership from a cleaned-away GUID.
Do not use trimming to rekey a nonblank GUID. Retain the original declaration and exact subject components.
Unresolved feed ownership also prevents complete coverage for its tracks.

Other items remain retained evidence. A requested item that is absent does not acquire complete-empty coverage.
The existing no-match result remains partial. This slice does not accept separate feed completeness during that failed item match.
Conflicting request and observed owner evidence cannot authorize the requested subject's replacement.

Zero direct candidates under a proven owner produce complete-empty coverage.
A malformed supported identity produces partial coverage, never complete empty, even when every candidate is malformed.
Unsupported candidates also remain partial in this minimum contract. Preserve their bytes and validation without enabling identity actions.
Valid candidates can coexist in retained evidence with rejected candidates. Such a collection preserves its earlier accepted snapshot.

Contributor identities and arbitrary nested elements remain outside the direct-owner collection.
Keep their locations and raw evidence without promoting them into feed or track ownership.
Foreign namespaces remain outside the registered syntax. Do not change packet 009's parser acceptance to simplify coverage.

An empty collection means no accepted direct assertion was present under the contract's complete enumeration.
It does not mean no person, key, or identity exists elsewhere in the document or outside that document.

## Atomic Writer Extension

Extend `record_once` and `record` inside the existing response transaction.
Do not call a separately committed replacement operation after recording the observation.
Keep request allocation as the existing earlier transaction.

1. Validate proof, provider/resource agreement, exact subjects, and each intended replacement key.
2. Reject duplicate authorized coverage scopes for the same provider, subject, and collection.
3. Insert or reuse the body, observation, coverage, and fact rows through the existing writer.
4. Check both request-slot supersession and each target head's accepted generation.
5. Create or reuse each complete snapshot, including snapshots with zero members.
6. Update accepted heads and their exact occurrence evidence inside this transaction.
7. Update attempt state only when the attempt generation permits it.
8. Commit all planned replacements and the observation together.
9. Return the receipt only after commit succeeds.

Use the reviewed `(provider, declared subject, collection)` key.
Assertion-source labels do not partition replacement. Replacing a collection changes its head across all its source labels.
Retain old snapshots, facts, bodies, selections, and discrepancies. No evidence deletion or garbage collection belongs here.

Validate head, snapshot, coverage, provider, subject, and collection agreement before reuse or update.
The member count must equal the ordered member rows. Every member must have the snapshot's proven owner.
Compare immutable inputs after a content-key match. A digest collision or inconsistent stored row is a storage error.

Compute the content key from ordered member evidence, ownership basis, collection, and verified contract.
Exclude local generation, fetch time, row IDs, and unrelated response fields.
Reuse an identical snapshot while advancing its head to the newer accepted occurrence.
Preserve its first coverage reference and the new occurrence's exact fetch time and metadata.

Set the new RSS extraction version `rss-dom-v2` for each observed RSS result before capture and parsing.
Use it for complete, malformed, unsupported, ambiguous, no-match, and failed results, including results without completeness proof.
Packet 014 used `rss-dom-v1`, null contract IDs, and different coverage grouping.
A contract ID attached only to complete coverage cannot distinguish the other changed interpretations.

Keep request descriptors, request-key inputs, and profiles unchanged for this version change.
The extraction version changes observation identity, not request identity.
Retain old and new observation rows together. Never change packet 014's immutable interpretation rows in place.
Never report their former partial coverage as complete.

An older completion cannot replace a newer accepted head, including through a different request profile or resource.
A request token superseded in its own slot cannot replace a head.
Retain the response and its rejected outcome. Keep superseded counters consistent without counting one occurrence twice.

Never overwrite a newer failure with an older completion.
Keep pending slots across restart. Packet 018 owns proof of abandoned request ownership.

Use typed per-collection outcomes: `KeptUnknown`, `KeptPartial`, `KeptFailed`, `RejectedSuperseded`, `ReplacedEmpty`, `ReplacedPopulated`, and `Repeated`.
Each outcome identifies its provider, subject when resolved, collection, and coverage scope.
A bare collection name is insufficient when one RSS document contains multiple owners.
Keep response retention separate from collection replacement outcomes.

Inject failures after evidence writes, after the first planned head change, and before commit.
Each failure must roll back every response write and head change while preserving the allocated request generation.
Retain packet 014's response capsule and verified-rollback retry behavior.
An uncertain commit must not offer an unverified retry or report successful replacement.

Preserve the committed-token replay check before opening a response transaction.
An identical replay through a cloned committed token returns its stored receipt, even after a newer generation advances the head.
It changes no head, slot, occurrence count, or superseded count.
Changed-input replay and replay after an uncertain commit remain blocked.
Do not reinterpret a committed replay as a new superseded completion.

## Live Local Reads

Add a typed `read_provider_collection` boundary for an exact provider, subject, and collection.
Return `NoSnapshot`, `CompleteEmpty`, or `CompletePopulated` with separate refresh evidence.
Preserve member order, validation, raw evidence references, accepted generation, and accepted occurrence metadata through reopen.
A head without a snapshot remains `NoSnapshot`. Zero members in an accepted snapshot mean `CompleteEmpty`.
Do not derive these states from legacy rows or DTO array shape.

Both `FetchLibraryTrackContext` and `FetchLocalTrackContext` must load these results.
The remote command must read again after its recorder finishes, including retained transport failures.
Its initial local read cannot represent a newly committed empty snapshot or failed refresh.
The local-only command must read durable state without HTTP requests or database writes.

Perform reads after recording inside the existing typed result assembly that retains receipts.
If a read fails, retain all committed receipts and any preceding `ObservationWriteFailure` capsule in the command error.
Do not let an early `?` or generic `Query` conversion discard this evidence.
The Library view model must receive and retain that typed failure through its existing callback.

Carry provider state in `TrackContext` outside `api::Track` and `api::Feed`.
The existing mounted Library frame must retain that state through its command callback.
Test the actual command result and frame/state consumer. A database helper used only by tests is insufficient.

Keep cloning, sanitization, successful local fallback, and navigation retention intact.
Propagate read errors through the typed result assembly described above. An error is not `NoSnapshot`.

Use exact provider resource and subject keys from the selected local context and retained request evidence.
Do not search by item GUID alone or choose an arbitrary recent provider.
Do not merge resource subjects into GUID subjects or bind legacy ownership from a matching source label.
If an exact binding is unavailable, retain the unresolved state and identify that limitation in the result.

RSS failures before parsing currently have no declared subject or coverage scope.
Keep those failures in their existing durable request slot.
Resolve request refresh evidence using the full existing request key.
Its inputs include provider kind, provider identity, requested resource, requested-subject descriptor, request parameters, and the separate profile.
Reuse request-key construction or compare each input against the stored slot.
Preserve exact track GUID and enclosure inputs, including null versus empty values.

Return that request evidence separately when no proven collection scope exists.
Do not invent a declared owner to attach a failure to a head.
Do not attach another track's failure merely because both requests used the same feed URL.

Accepted collection state and the latest failed request must remain available together.
Neither failure nor incomplete coverage restores older collection members after complete-empty replacement.
The reader must not expose malformed or unsupported evidence as an active `SourceEntityId`.

Legacy DTO fields, existing scalar defaults, and field selection remain unchanged.
Packet 020 owns shared display and action projection. Carrying provider state does not complete that projection.
Packet 036 owns discrepancies. This packet does not create or resolve them.

## Mechanical Acceptance Criteria

Use behavioral tests with the `adr_0075_snapshot_` prefix beside the owning code.

| Case | Required proof |
| --- | --- |
| R13-01 | A live observed Library RSS request produces grouped complete feed and matched-track identity snapshots under the registered contract. |
| R13-02 | Complete populated to complete empty replaces only that provider's exact owner and collection. Reopen returns `CompleteEmpty`. |
| R13-03 | Missing, null, invalid, omitted, summary, unsupported, partial, failed, and unverified Index responses preserve seeded heads. Raw presence remains distinct. |
| R13-04 | Malformed supported identities and unsupported candidates preserve prior snapshots. Neither becomes false complete empty or an active identity after reopen. |
| R13-05 | Duplicate channels, repeated owner GUIDs, duplicate item GUIDs, GUID conflicts, enclosure-only ambiguity, and no-match responses cannot clear affected snapshots. |
| R13-06 | Two feeds with the same item GUID stay separate. Resource and GUID subjects stay separate. Redirects retain the requested provider. |
| R13-07 | Nested and contributor identities remain evidence with their original owners. Unrelated items cannot populate the requested track snapshot. |
| R13-08 | Replacement spans all assertion-source labels within one provider collection. Other providers, legacy rows, selected fields, and discrepancies remain equal. |
| R13-09 | An older completion loses to a newer accepted head across distinct profiles and resources. It also preserves a newer failure report. |
| R13-10 | Each injected response-write failure rolls back evidence, snapshots, and all planned heads. Verified retry commits once without another request generation. |
| R13-11 | Repeated content reuses snapshots and members. Occurrence-only header changes keep body, observation, fact, and snapshot counts stable. |
| R13-12 | Changed contracts cannot reinterpret old observations in place. Equal-key input mismatches fail without partial writes. |
| R13-13 | Both real Library commands return typed provider state. Local-only reads survive reopen without network work or writes. |
| R13-14 | The mounted Library frame retains the command's typed state through clone and sanitization. Legacy DTO fields remain equal. |
| R13-15 | Failed refresh and accepted populated or empty state remain distinct. Different GUID/enclosure descriptors, changed RSS URLs, and unresolved subjects cannot share failure state accidentally. |
| R13-16 | Local read failures propagate. Missing bindings, missing heads, and complete-empty heads remain distinguishable without guessed provenance. |
| R13-17 | Existing packet 014 request counts and response-retention tests remain Green. Snapshot work adds no HTTP call or response commit. |
| R13-18 | Unknown contract IDs and forged or mismatched proof cannot authorize replacement. Production MusicIndex completeness remains disabled. |
| R13-19 | Seed packet 014 observations, then record identical inputs under the new extraction version for each observed RSS outcome. Old interpretations remain unchanged and new interpretations coexist, including results without proof. Request descriptors and profiles remain equal. |
| R13-20 | Seed GUID-scoped and resource-scoped heads. Empty, whitespace-only, malformed, and repeated feed GUID declarations cannot replace them or establish resource ownership. Empty item GUIDs cannot clear track heads. |
| R13-21 | Commit a snapshot, then inject a local-read failure through the Library command and view model. Preserve the committed receipt and separately test retention of a preceding write capsule. |
| R13-22 | Identical resources and subject descriptors with different profiles retain separate refresh states. Null and empty request inputs remain distinct. |
| R13-23 | Replay a committed cloned token after a newer generation advances its head. Return the original receipt with unchanged heads, slots, occurrence counts, and superseded counts. Changed-input and uncertain replay remain blocked. |

Use synthetic database fixtures to test provider isolation, multiple source labels, and cross-profile ordering.
Any test-only completeness constructor must remain under `cfg(test)`. It must not enable an Index production contract.
Synthetic complete Index data does not prove deployed completeness or close the inherited-credit contract gap.
Keep storage case S11-08's production Index transition proof open until that upstream contract is established.

## Test Commands

Run focused tests during implementation. Root coordinates the combined suite after the final diff stabilizes.

```bash
cargo test --locked --offline adr_0075_snapshot_
cargo test --locked --offline adr_0075_observation_
cargo test --locked --offline adr_0075_rss_
cargo test --locked --offline --test architecture_tests
cargo check --locked --offline
cargo clippy --locked --offline -- -D warnings
cargo fmt -- --check
cargo test --locked --offline
cargo build --locked --offline --bin v4vmm
python3 -B docs/runbooks/check-markdown-links.py docs/tasks/adr-0075-task-013-verified-snapshot-replacement.md
python3 -B "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" --check --no-heuristics docs/tasks/adr-0075-task-013-verified-snapshot-replacement.md
```

Use disposable local HTTP fixtures. Do not contact MusicIndex or launch the app.
The normal binary build follows tests. A test binary cannot serve as the desktop handoff.

## Excluded Scope And Remaining Proof

- No migration, schema alteration, restore change, production data repair, or legacy backfill.
- No scalar selection, source priority, freshness duration, fallback change, discrepancy comparison, or identity display completion.
- No conversion of caller families assigned to packets 038 through 043.
- No named request profiles, caching, pending-request abandonment, or visible storage-retry action.
- No audio file, tag, playback, payment, broadcast, configuration, renderer, or layout change.
- No upstream edit, deployment, update hook, new dependency, commit, or shared status-document edit.

Three proof limits remain explicit.
Index needs a versioned endpoint completeness contract, pagination guarantees, and a verified deployment or supported response proof.
Inherited credits also need guaranteed owner-partition behavior before the track-owned collection can become complete empty.
Legacy and resource/GUID subject bindings need evidence before shared projections can combine those records.

These limits do not prevent the bounded RSS storage and typed-read work.
They prevent claims that all provider replacement, all inherited-credit transitions, or active identity display are complete.

## Rollback And Escalation

This packet changes no schema. A code revert leaves retained evidence and schema 12 intact.
Do not delete snapshots, rewind generations, or remove migration records during rollback.

Return to root for unsupported completeness assumptions, new field policy, broader caller conversion, or a required schema change.
Also return if exact live-read binding needs an identity rule absent from the accepted contract.
Preserve the current dirty work. Do not weaken a coverage requirement to produce a passing test.

## Expected Report

Report changed files, registry entries, live writer and reader roots, and tests with their results.
Map R13-01 through R13-23 to concrete behavioral tests.
State the remaining Index, owner-binding, display, and visual limits separately.
Report the shared STE check accurately. Retain technical terms when replacement would change their meaning.

## Implementation Evidence — 2026-09-21

Implementation, root technical review, and mechanical checks are complete.
No app was launched. No commit, upstream change, production-data write, or shared status edit was made.

The registry enables `rss-direct-source-ids-v1` only.
A private decoded-body value binds the existing DOM to the retained response bytes.
The registry checks the supported root, raw owner declarations, exact matching, complete enumeration, and structured values against the DOM.
Proof binds the request, response, owner, collection, and coverage scope. Proof and decoded-body Debug output remain redacted.
MusicIndex has no enabled production completeness contract.

The existing response transaction now stores or reuses complete snapshots and changes accepted collection heads.
It retains rejected, partial, unknown, failed, and superseded evidence.
Snapshot reads check provider, owner, contract, first coverage, ordered members, content key, and accepted coverage content.
Committed-token replay returns the original receipt without another transaction or state change.

Both Library commands load typed provider state. The remote command reads after response recording.
The local command uses the exact stored feed URL through `feed_service::local_provider_request`.
This lookup does not add the URL to legacy display fields.
The existing Library callback retains provider state in the mounted frame.
Read failures preserve committed receipts and preceding write capsules through the command result and view model.

### Changed Owners And Approved Scope Additions

| Owner | Change |
| --- | --- |
| [Observation types](../../src/provider_observation.rs) | Private coverage proof, scoped outcomes, typed collection reads, refresh states, and read-failure evidence. |
| [Completeness registry](../../src/provider_observation/contracts.rs) | New direct RSS registry, decoded-body binding, exact owner resolution, and DOM value checks. |
| [Observation writer](../../src/db/provider_observations.rs) | Atomic snapshot replacement, content reuse, consistency checks, and exact local reads. |
| [RSS enrichment](../../src/rss/enrich.rs) | Grouped identity coverage, complete-empty scopes, versioned extraction, and behavioral tests. |
| [Track context](../../src/metadata.rs) | Provider state remains separate from API DTOs. |
| [Library queries](../../src/application/queries/library.rs) | Live remote and local reads, receipt-preserving errors, and fixture tests. |
| [Feed service](../../src/feed_service.rs) | Exact local feed-resource lookup without changing legacy DTO fields. |
| [Subscription service](../../src/subscribe_service.rs) | Shared construction of unchanged RSS request inputs. |
| [Library callback](../../src/library/app_impl.rs) | Shared frame assignment and a mounted-state regression test. |
| [Library view model](../../src/view_models/library.rs) | Read-error initializer and test inspection of retained evidence. |
| [Discover tests](../../src/discover/tests.rs) | Empty provider-state initializers for existing context fixtures. |
| [Architecture guards](../../tests/architecture_tests.rs) | Registry, transaction, reader, callback, and single-DOM boundaries. |

Root approved two narrow additions to the original file list on 2026-09-21:

- [MusicIndex adapter](../../src/provider_observation/musicindex.rs): initialize absent proof without changing extraction or completeness behavior.
- [RSS exports](../../src/rss/mod.rs): expose the existing identity validator within the crate for registry validation.

This packet is the only changed document for this implementation session.
No document or folder was created or moved by the implementation agent.

### Behavioral Evidence Map

Each snapshot test name below starts with `adr_0075_snapshot_`.
Tests remain beside the production owner. Existing observation and RSS tests remain part of the verification boundary.

| Cases | Concrete tests |
| --- | --- |
| R13-01, R13-13, R13-15, R13-17 | `live_commands_return_empty_and_retained_failed_refresh`. Both real commands return provider state. Reopened local reads make no request or write. The observed command retains three HTTP requests and six transactions. |
| R13-02, R13-06 | `populated_empty_reopen_and_provider_subject_isolation`. Provider, feed, item, GUID, and resource subjects remain separate. Empty snapshots survive reopen. |
| R13-03 | `unverified_index_presence_matrix_preserves_seeded_heads`. Missing, null, empty, invalid, populated, failed, and omitted-profile inputs preserve synthetic Index heads. |
| R13-04, R13-20 | `rejected_identity_and_owner_declarations_preserve_heads`. Malformed identities, unsupported purposes, and unusable GUID declarations cannot clear accepted heads. |
| R13-05, R13-07 | `ambiguous_matches_and_nested_evidence_cannot_replace_track`. Duplicate channels, duplicate item GUIDs, conflicts, enclosure ambiguity, missing items, and nested identities remain conservative. |
| R13-08 | `source_labels_replace_together_and_uncertain_commit_blocks_replay`. Replacement spans both retained assertion-source labels. Existing `adr_0075_observation_preserves_all_legacy_and_snapshot_evidence_tables` checks unrelated evidence preservation. |
| R13-09 | `repeated_content_replay_and_generation_order` and `cross_resource_order_keeps_newer_head_and_failure`. Older completions cannot replace newer heads or request failures. |
| R13-10 | `transaction_failures_rollback_and_verified_retry_once` and `source_labels_replace_together_and_uncertain_commit_blocks_replay`. Injected failures preserve generations, roll back response writes, and distinguish verified retry from uncertain commit. |
| R13-11, R13-23 | `repeated_content_replay_and_generation_order`. Identical content reuses snapshots. Changed occurrence headers preserve row counts. Committed replay returns its original receipt without mutations. |
| R13-12 | `proof_mismatch_and_decoder_versions_preserve_old_interpretations` and `read_rejects_cross_provider_member_and_head_content_substitution`. Mismatched proof, substituted members, and inconsistent accepted coverage fail without partial replacement. |
| R13-14 | `mounted_frame_preserves_typed_state_and_legacy_fields`. The actual callback helper retains cloned and sanitized provider state. Legacy fields remain equal. |
| R13-16 | `missing_binding_missing_head_and_empty_are_distinct`. Unavailable binding, request-only binding, missing heads, unaccepted heads, and complete-empty state remain distinct. |
| R13-18 | `registry_rejects_unrelated_dom_and_conflicting_values`, `proof_mismatch_and_decoder_versions_preserve_old_interpretations`, and `unverified_index_presence_matrix_preserves_seeded_heads`. Forged evidence cannot enable replacement. |
| R13-19 | `live_v2_capture_preserves_actual_v1_per_node_and_failed_rows`. The fixture seeds packet 014's per-node rows, then invokes live v2 capture. Original rows remain equal. Failed responses receive v2 without proof. |
| R13-21 | `committed_read_failure_reaches_command_and_library_state` and `read_failure_also_preserves_preceding_write_capsule`. The command and view model retain committed receipts and prior write evidence. |
| R13-22 | `exact_refresh_keys_keep_profiles_null_and_empty_separate`. Profiles, GUIDs, enclosure inputs, and resource changes retain distinct request states. |

Cross-resource ordering uses a test-only proof within one synthetic provider.
The Index preservation matrix uses synthetic seeded heads. Neither test enables a production Index contract or proves deployed completeness.
Storage case S11-08's production inherited-credit transition proof remains open.

### Mechanical Check Record

| Check | Result |
| --- | --- |
| `cargo test --locked --offline adr_0075_snapshot_` | Green. 18 unit tests and one architecture guard. |
| `cargo test --locked --offline adr_0075_observation_` | Green. 20 unit tests and one architecture guard. |
| `cargo test --locked --offline adr_0075_rss_` | Green. 33 unit tests and one architecture guard. |
| `cargo test --locked --offline --test architecture_tests` | Green. 267 architecture guards. |
| `cargo check --locked --offline` | Green. |
| `cargo clippy --locked --offline -- -D warnings` | Green. |
| `cargo fmt -- --check` | Green. |
| `cargo test --locked --offline --quiet -- --test-threads=4` | Green. 1,631 unit tests and 267 architecture guards. Ten documentation tests remain ignored. |
| `cargo build --locked --offline --bin v4vmm` | Green after the full suite. |
| Markdown links | Green. 43 local links. |
| `git diff --check` | Green. |
| Shared STE checker | Exit 1. Lexical findings and one unchanged `must have` possession-form finding remain. No confirmed structural defect. |

Cargo results are retained in the command tool output. The final suite used direct commands with the approved Cargo test prefix.
The language-check result is `/tmp/adr0075-013-ste-final.json`. Technical terms and source meanings remain unchanged.

The first localhost test attempt was blocked by the sandbox. The direct approved Cargo test command supplied fixture access.
Initial failures found one read-helper borrow lifetime, outdated source guards, three large tuple types, and a fixture commit-count assertion.
The local-command regression also found an omitted resource lookup. The feed-service lookup corrects that production gap.
Each correction retains its focused regression or architecture guard.

## Operator Visual Check

Visual gates remain open and paused. This packet supplies no new display selection or layout.
Do not request an operator session or launch the app.

Packet 014 retains its storage-failure presentation procedure. Packet 012 retains its repair-report procedure.
Packet 020 must define a disposable fixture and terminal procedure before provider state changes visible identity or metadata selection.
Mechanical command and frame tests cannot close those visual gates.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:

- This packet, its authority documents, and all Files To Inspect.
- The shared STE and Rust development skills before prose or code changes.

Goal:

- Register proven direct RSS identity completeness and replace snapshots inside the existing observation transaction.
- Connect typed local reads to both Library commands and their existing mounted state.

Constraints:

- Wait for root dispatch after this packet's technical review and dependency release.
- Preserve original owners, rejected evidence, complete-empty state, generation ordering, and retained refresh failures.
- Keep production Index replacement disabled and every field policy separate.

Do not touch:

- Anything excluded by Files To Change or Excluded Scope And Remaining Proof.
- Shared status documents, upstream repositories, operator data, or display selection.

Acceptance criteria:

- Prove R13-01 through R13-23 through the named production boundaries and scoped behavioral tests.
- Keep unsupported production contracts and paused visual gates open.

Test commands:

- Run the focused commands in Test Commands. Coordinate combined Cargo checks with root.
- Run the Markdown link checker and shared STE checker for changed prose.

At the end, report:

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
