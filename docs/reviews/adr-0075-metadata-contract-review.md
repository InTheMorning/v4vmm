# ADR 0075 Metadata Contract Review

## Status

Initial audit recorded - 2026-09-19. At that audit, ADR 0075 was Proposed and the first packet was Draft.
Implementation had not started then. No migration, deployment or new visual acceptance is recorded.

Pre-dispatch review recorded on 2026-09-19. Packet 001's instructions are corrected.

Operator decisions recorded on 2026-09-19. ADR 0075 is Accepted. Packet 001 became Ready.
Document packets 002–008 exist. Their deliverables remain incomplete.
The operator's dispatch hold applied to every packet at that time.

The plan now divides phases 002–006 into bounded packets.

Accuracy review recorded on 2026-09-19. The first-packet corrections below resolve R4, R5, R7, and R8.
R1, R2, and R6 remain open for later packets.
These corrections do not change the accepted ADR or release a packet.

Packet 001 implementation recorded on 2026-09-19. The operator released its dispatch.
The transport change is complete and its mechanical checks are Green.
Storage and display preparation remain open. The operator holds every remaining dispatch.

## Evidence And Limits

At the audit, the app checkout was clean at commit `7b95b25`.
The Stophammer checkout was clean at commit `a220f44`.
The agent did not inspect the deployed MusicIndex revision or current RSS contents.
The agent did not launch the app or a browser.

The operator supplied the [MoeFactz API evidence](adr-0037-review-checklist.md#task-002-contributor-source-check--2026-09-19).
MusicIndex supplies three contributor credits attached to the track. Two carry
HeyCitizen's npub. The host credit carries `https://www.moefactz.com/`.
Those facts do not establish a website or Nostr identity owned by the track itself.
The origin of nine stored local track Nostr facts remains unverified.

## Findings From Source Inspection

### Contributor Fields Are Lost

The app's `src/api.rs::Contributor` omitted fields that Stophammer's
`src/query.rs::SourceContributorClaimResponse` supplies. These include the owner,
position, normalized role, source, extraction path and observation time.
Packet 001 corrected the transport layer on 2026-09-19. Tests in `src/api.rs` now
enforce that result. The [implementation record](#packet-001-implementation--2026-09-19)
holds the remaining losses.

Storage has a separate problem. `src/identity_ingest.rs::persist_contributors`
assigns positions from list order and sets observation time to `None`.
The types in `src/local_identity.rs` do not retain typed claim ownership.
The storage packet must preserve the supplied evidence.

### Returned Credits Can Belong To The Feed

Stophammer's `src/db.rs::get_effective_source_contributor_claims_for_track`
can return feed credits for a track. Each credit must retain its declared owner.
Contract tests must cover this case.

### RSS Extraction Can Assign The Wrong Owner

The app's `src/rss/enrich.rs::nostr_from_extension` scans arbitrary nested values.
`enrich_track_from_feed_rss` can assign a person's npub to the track with an
invented `podcast:txt` extraction path. The parser packet must correct this rule.

### Website Rules Differ

Stophammer's parser `extract_links` emits an item link as `web_page`.
The app writes `item.link` to the `tracks.link` column in `src/rss/subscribe.rs`.
The app writes no page identity fact for that link.
`rss_track_link_inputs` builds only the transcript link.
The shared field rules must also cover `src/views.rs::website_url_from_links`.
The packet work on 2026-09-19 corrected this attribution.

### Feed Defaults Can Appear As Track Facts

The app's `src/api.rs::track_with_feed_defaults` can place feed identity,
credits, description and image in the track's API data object.
The refactor must keep stored source facts separate from values selected for display.

### An Empty Refresh Can Leave Old Index Facts

The app's `src/identity_ingest.rs::persist_source_links` and `persist_source_ids`
use source labels to select replacement groups.
`src/db.rs::replace_local_identity_links` removes only the `musicindex` group
when an Index collection is empty. Earlier `rss_link` and `podcast_txt` groups can remain.
The storage packet must replace the complete collection from that provider.

### Source Order Can Select The Displayed Identity

The app's `src/db.rs::local_identity_links` orders facts by source label.
The selectors `src/views.rs::website_url_from_links` and `nostr_npub_from_ids`
can use that alphabetical order to select an identity.
The shared display rules must define source selection and conflict handling explicitly.

### Repeated Requests Can Still Omit Required Facts

The app's `src/application/queries/search.rs::fetch_index_track_result_rows`
requests full details one track at a time.
Its `fetch_index_track_detail` calls do not request the identity collections.
The request packet must define required collections and measure request counts.

### Index Details Omit Contributor Sections

The app's `src/app/search_dispatch.rs::index_track_detail_slots` omits contributor sections.
Preserving API fields alone will not add contributor actions.
The later presentation packet must provide shared actions with owner labels.

### The Tag Summary Is Incomplete For Display

The app's `src/metadata.rs::musicindex_contributors_id3_value` keeps contributor names and roles only.
That tag summary cannot represent complete contributor facts.
The refactor must keep tag export separate from data prepared for display.

### RSS Errors Are Ignored

The app's `src/subscribe_service.rs::enrich_track_context_from_rss` ignores enrichment errors.
Missing data and failed requests can therefore appear alike.
The request result must distinguish those states.

These findings come from source inspection. They do not establish corruption
in production data or the contents of every Index record.

## Pre-Dispatch Review — 2026-09-19

Reviewed the ADR, phase plan and packet 001 against app commit `fb1972a` and
Stophammer commit `a220f44`. The app code still has the six-field contributor type.
The agent changed documentation only.

### Packet 001 Corrections

| Finding | Correction | Result |
|---|---|---|
| The packet depended on conversation history for its example | Embedded the complete selected response supplied by the operator | An agent can construct the test fixture without a live request |
| New string types and absent-field serialisation were unspecified | Added exact types, null handling and omission rules for the seven new fields | Older payloads retain their existing six-field output shape |
| Sequential positions could hide accidental renumbering | Added a derived fixture with positions `7`, `2`, `19` | Tests must preserve positions independently of array order |
| Normalised roles matched original roles in the supplied example | Added a test with different `role` and `role_norm` values | An agent cannot substitute one field for the other |
| The upstream checkout was an implicit requirement | Recorded its inspected revision and copied the field contract into the packet | The implementation can run in the app repository alone |
| The packet understated its existing JSON storage effect | Documented that `persist_contributors` serialises the expanded DTO into `raw_json` | Typed columns and display remain explicitly incomplete |
| A partially binding parent decision was ambiguous | Required Accepted ADR 0075 and Ready packet status before dispatch | The packet follows ADR 0057's status vocabulary |

Packet 001's content review is Green. It does not require a screen-placement decision.
Keep its scope limited to the transport fields and required constructor changes.
No implementation exists to test or merge yet.

Documentation checks are Green: local links and anchors, embedded JSON, unchanged test commands, status consistency and diff whitespace.
The agent did not run application tests for these documentation changes.

### Requirements Before Later Packets

1. **Define completeness before replacement.** A track response can contain feed credits.
   A populated list therefore does not establish a complete collection for every returned owner.
   Phase 002 must define the transitions from track credits to feed credits, then to no credits.
   Phase 003 must implement those rules without replacing an unrelated owner's facts.

2. **Define evidence retention before cleaning.** `sanitize_source_contributors` can remove values and contributor rows.
   `persist_contributors` serialises the DTO that it receives.
   That JSON is not necessarily the original source payload.
   The later ingestion design must retain source evidence before cleaning or fallback.
   The app must continue to exclude placeholder text from presentation.

3. **Define cache freshness and response ordering.** Reusing a completed request without an expiry rule can preserve stale facts indefinitely.
   Phase 004 must define explicit refresh behavior and ordering when source revisions are unavailable.
   Its packet must set numeric request bounds before implementation.
   Fewer requests alone do not establish lower latency.

4. **Record the limited replacement of earlier ADR rules.** ADRs 0028 and 0054 use source-scoped replacement.
   ADR 0075 proposes a separate provider key.
   At acceptance, reconcile that change without reopening unrelated completed work or closing existing visual gates.

5. **Split later phases before delegation.** Phase 002 spans parser rules, ingestion and API behavior in separate repositories.
   Phase 006 spans reporting, repair, deployment and replica checks.
   Neither phase is one bounded coding packet.

The plan now records these requirements before dispatch.
They are future design requirements, not new claims of production corruption.

The technical references support the proposal's ownership distinctions.
The [person specification](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md)
uses item credits in place of channel credits and defines default roles and groups.
Phase 002 must distinguish those interpreted defaults from supplied attributes.
The [txt specification](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md)
leaves purpose values service-specific.
[NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md) distinguishes a public key from a profile encoding that can contain relay data.

### Operator Decisions Requested

Both questions now have an answer. See [Operator Decisions](#operator-decisions--2026-09-19).

- **Field scope:** Start with identity and contributor fields, or include all metadata fields in separately bounded packets.
- **Identity placement:** Keep the track header limited to track identities, or include related identities in that header with owner labels.

## Operator Decisions — 2026-09-19

The operator answered both questions and accepted ADR 0075.

| Question | Operator answer | Effect |
|---|---|---|
| Field scope | Every metadata field, in separately bounded packets | Phases 002 to 005 grow. Each field needs a written rule before its packet runs. The first merge comes later |
| Identity placement | Separate, labelled sections | The track header keeps track identities. Feed identities and contributor identities go in their own sections with owner labels |
| ADR status | Accept ADR 0075, hold every dispatch | ADR 0075 is Accepted from 2026-09-19. Packet 001 became Ready and held. Write the document packets first |

The operator selected the recommendation for placement. The operator selected the
wider option for field scope. The agent recommended the narrower option.
The wider scope adds the field rule packets 005, 006 and 007 to the plan.

The plan now holds a [packet register](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register).
It divides phases 002 to 006 into 28 bounded packets.
The register records the inputs of each packet. Packets 002 to 008 need no further decision.

The acceptance records cover these documents:

- ADR 0075, from Proposed to Accepted, with both decisions in its Status section.
- ADR 0028 and ADR 0054, with a dated amendment for the replaced replacement key.
- The ADR index rows for 0028, 0054 and 0075.
- The phase plan status, scope, dispatch rules and packet register.
- This review.

No code changed. No test ran for these documentation changes.

## Accuracy Review Before Dispatch — 2026-09-19

Disposition: Changes required before dispatch of the document packets.
Packet 001's narrow transport contract still matches the inspected upstream type.
Its existing dispatch hold remains in force.

### Scope And Evidence

This review covers the working copies of ADR 0075, its plan, packets 001–008,
this review and the changed documentation indexes and ADR amendments.
The app base is `fb1972a`. The Stophammer checkout is `a220f441d57640912eed9b190d7c194284f87a38`.
The findings concern the inspected source. They do not establish the deployed API revision.

The seven packet files 002–008 exist. Their seven deliverables do not yet exist.
Writing an instruction packet does not complete its field rules or examples.
No implementation, deployment, database repair or visual check ran during this review.

### R1 — Correct Upstream Ownership Before Defining Scalar Rules

[Packet 006](../tasks/adr-0075-task-006-field-rules-artist-language-dates.md),
Required Content 5, calls `api::Track.release_artist` a separate track claim.
That statement is false for the inspected Stophammer response.

Stophammer `src/query.rs:524` and `src/query.rs:542` select `f.release_artist`
from the joined feed row. `build_track_response` copies that value into the track response.
Both queries also select `COALESCE(t.image_url, f.image_url)`.
The API can therefore supply feed artwork in `Track.image_url` before any app fallback runs.

[Packet 005](../tasks/adr-0075-task-005-field-rules-description-artwork-publisher.md)
audits three app artwork paths but omits this upstream fallback.
It also calls the Rust expression in `src/views.rs:682` a SQL-level fallback.
The expression runs in the view projection. The upstream query contains the SQL fallback.

Required correction: trace scalar ownership through the upstream query and the app DTO.
Record the feed ownership of the inspected `release_artist` value.
Record that the current scalar artwork response does not identify which owner supplied the image.
Do not infer that owner from equal URLs. Send the missing artwork provenance requirement to packet 008.
An app-only fallback change cannot restore evidence that the API response does not carry.

### R2 — Complete The Field And Transport Inventory

The accepted scope covers every metadata field. Packets 005–007 do not yet cover that scope.
They omit rules for existing fields such as title/name, album/feed title, track/disc number,
release kind, raw medium and artist sort text.
These fields appear in `src/api.rs:124`, `src/api.rs:157` and `src/views.rs:154`.
The plan assigns packet 020 the field rules from packets 005–007 only.

The transport audit also needs these corrections:

| Packet claim or omission | Inspected evidence | Required correction |
|---|---|---|
| Packet 008 says `SourceEnclosure` already carries `entity_type` and `entity_id` | App `src/api.rs:306` has neither field. Upstream `src/query.rs:427` also supplies position and observation time that the app omits | Record all four missing fields. Assign a bounded transport correction |
| Packet 006 records no track language field in the app, but stops at that limit | Upstream `TrackResponse`, `src/query.rs:238`, supplies `language` | Audit the lost track language separately from feed language |
| Packet 007 describes transcripts through `source_links` only | Upstream `src/query.rs:264` and `src/query.rs:413` define `source_transcripts` with ownership and provenance | Include this collection in the API audit and completeness rules |

Required correction: create a field inventory across RSS, upstream responses, app DTOs and local storage.
Assign each field a rule packet or an explicit reference to an unchanged governing rule.
Include scalar coverage in summary and detail responses.
For example, upstream `Feed.tracks` contains `TrackSummary` values with fewer fields than full track details.
The app decodes both into `Track`. Missing summary fields must not prove absence in a complete track observation.

Do not expand packet 001 while making this inventory. Its contributor-only boundary remains useful.

### R3 — Separate Policy Proposals From Observed Behavior

At this review, packets 005–007 required a source priority, a conflict result and a no-source result for every field.
They also prohibited new product decisions and required results usable without another product decision.
ADR 0075 required those rules without choosing each field's source priority.
Current source order is evidence about current behavior. It is not approval of the future rule.

Packet 005 also extended Decision B to descriptions, artwork and publishers.
Required Content 9 mandated their placement in the feed section, never in the track header.
Decision B explicitly selects placement for identities. It does not select artwork fallback placement.

Required correction: distinguish existing binding rules, current behavior and proposed field policies.
Name the review that accepts each proposed policy before implementation.
Keep a dependent packet held while its required policy remains unresolved.
Do not assign policy selection implicitly to the coding agent for packet 020.

Operator response recorded on 2026-09-19:

1. Accepted. Document agents propose field priorities and conflict rules for a separate operator review before coding.
   A proposal does not become an accepted field rule until that review accepts it.
2. Accepted after explanation. Use track artwork when the track has its own image.
   Otherwise, use feed artwork in the track header without an additional visible owner label.
   The operator expects this familiar display fallback. Stored facts retain their source and owner.

R3 documentation correction: Green. ADR 0075 records Decision C and the field-rule review process.
The plan and packets 005–007 now distinguish proposals from accepted policy.
Packet 005 records the artwork exception. Other field placement proposals require operator review.
The remaining accuracy findings stay open. No field-rule deliverable or UI implementation is complete.

The existing dispatch hold remains unchanged. This response does not dispatch a packet.

### R4 — Give Corpus Cases Separate Current And Required Results

[Packet 002](../tasks/adr-0075-task-002-shared-example-corpus.md) defines expected
storage and display results under ADR 0075.
Its Review Criteria then require those results to match the current source function.
Those requirements conflict where the ADR corrects current behavior.

For example, C08a requires a track page link. The current RSS helper stores no corresponding identity fact.
C09 must retain malformed evidence without enabling an identity action.
The current Nostr extraction checks prefixes without validating the full encoding.

Required correction: add distinct current-result and required-result fields.
Check current results against source. Check required results against the accepted contract.
Allow a documented difference to identify the implementation work that the case must guard.

Also exempt C01 from the universal fenced-source-block requirement.
Its specific instruction requires a link to the supplied fixture instead of copied JSON.

### R5 — Remove Circular And Missing Packet Prerequisites

The plan's Requirements Before Dispatch table requires an accepted Stophammer decision
and completed rules before any phase 002 work.
Phase 002 includes the packets that write those rules and the request for that decision.
The literal dispatch rule therefore prevents its own prerequisites from being produced.
The same pattern affects the schema design in phase 003 and the evidence documents in phase 006.

The packet register also omits dependencies that the packet text needs:

- Packet 006 sends the duration storage question to packet 011. Packet 011 does not require the field-rule outputs.
- Packet 008 says it assembles outputs from packets 005–007. Its dependencies and reading list omit those outputs.
- App code that depends on packet 008 also needs the Stophammer decision before it can rely on changed API behavior.
  Writing the request alone does not supply that result.

Required correction: state prerequisites per packet and distinguish document work from code or deployment work.
Add the field-rule dependencies to the schema and upstream request.
Name the external decision and implementation evidence that dependent app work needs.
Distinguish the existence of packet files from completion and review of their deliverables.

### R6 — Correct The Remaining Source Assertions

These instructions prescribe inaccurate facts to the next author:

| Location | Correction and evidence |
|---|---|
| Packet 007, Required Content 4 | Upstream Atom alternate links use `entity.atom:link[@rel='alternate']`, not `entity.link`. See `stophammer-parser/src/engine.rs:826` |
| Packet 007, Required Content 6 | `select_audio_enclosure` selects the first supported primary enclosure, then the first supported enclosure, then a supported scalar enclosure. Invalid or unsupported entries are skipped. See app `src/track_compare.rs:209` and `src/track_compare.rs:493` |
| Packet 006, Required Content 8 | The local track date does have a fallback: `metadata_facts.pub_date.or(t.pub_date)`. See app `src/views.rs:700` |
| Packet 008, Required Content 4 | `extract_persons` extracts attributes and position. Stophammer `src/api.rs:624`, `build_source_contributor_claims`, assigns typed ownership, source, extraction path and observation time |
| Packet 008, Required Content 7 | `src/ingest.rs` declares the request fields. The hash skip and forced-ingestion behavior live in `src/verifiers/content_hash.rs:30`. Cite that behavior as well |
| Packet 007, Required Content 5 | The transcript selectors reside in two files, `src/metadata.rs` and `src/views.rs`. One is a read selector, not a third storage site |

Replace each inaccurate statement before asking an agent to reproduce it as required content.
Keep the supported-media filter in the enclosure rule and its eventual regression cases.

### R7 — Make The Document Checks Executable And Accurate

Language checker correction: Green on 2026-09-19.
Packets 002–008 give the installed Python command for the shared checker.
The checker and skill reside in `~/.agents/skills/asd-ste100`.

Claude and Codex use that source. Their existing skill names remain aliases.
The `/ste-check` alias is a Claude command. It is not a shell executable.

The shared skill requires direct STE composition and correction of confirmed errors in changed prose.
The Claude hook checks changed prose blocks with their Markdown context.
The installed checker and hook pass 45 regression tests.
The skill and plugin validation are Green.

The checker covers 13 rule groups with an Issue 8 dictionary extract.
That extract lacks many words and all meaning definitions.
A Green result does not prove full Issue 9 compliance.
The official Issue 9 dictionary remains necessary for a full dictionary audit.

The link commands discard anchors and can return success after printing a missing file.
They therefore do not prove their stated acceptance criterion.
Some command blocks also change the current directory before later blocks assume the repository root.
Use a check that validates files and anchors, returns failure on errors and preserves its working directory.

Packet 003 requires reading external specification pages but prohibits every live network request.
Narrow the prohibition to live service or feed requests, or provide pinned local copies of the specifications.

Packet 002 gives `docs/tasks/` as the link base for a deliverable in `docs/schema/`.
Correct that base. Sibling directory links can happen to work from either base, but task-file links cannot.

### R8 — Reconcile The Current Status Documents

`AGENTS.md:16` and the broadcast delivery order still call ADR 0075 Proposed.
The delivery order also calls packet 001 Draft.
The ADR and packet now say Accepted and Ready, with dispatch held.
The review's earlier claim that the acceptance changes moved in one commit is not true of this checkout.
The acceptance changes remain uncommitted here.

Required correction: update the current status owners together.
Use Accepted, Ready and held consistently. Preserve every existing visual gate.
Replace the unverified commit statement with the actual working-tree state or the eventual commit evidence.

### Checks And Limits

Documentation checks before this review update: Green.
The check inspected 16 changed or new Markdown files and 289 local links, including heading targets.
Embedded JSON parsed successfully. Packet 002 lists 21 distinct case identifiers, as claimed.
`git diff --check HEAD` was Green.

The primary specifications support the distinction between npub and nprofile,
service-defined txt purposes, and item credits replacing channel credits.
Sources: [NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md),
[Podcast txt](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md),
and [Podcast person](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md).
These checks do not resolve the field policies identified in R3.

The initial accuracy review changed this review file only.
The later operator responses prompted the R3 documentation correction recorded above.
No application test was needed for this documentation review.
The existing Operator Visual Check section remains in force. No app launch is requested.

## Contract Cases Required Before Completion

- [ ] Identity facts retain their feed, track or contributor owner.
- [ ] Item credits replace feed credits for display without deleting feed facts.
- [ ] Matching names or keys do not merge separate credits from a source.
- [ ] `website`, `web_page`, transcript and enclosure links keep distinct kinds.
- [ ] Valid npub, nprofile, invalid values and unsupported syntax stay distinguishable.
- [ ] Namespace prefixes do not determine RSS semantics.
- [ ] Not requested, absent/null, empty, populated and failed responses stay distinct.
- [ ] An empty refresh replaces only the matching collection from that provider.
- [ ] Older request completion cannot replace a newer snapshot.
- [ ] Persisted facts retain meaning after restart and endpoint changes.
- [ ] Feed-scoped track IDs prevent cross-feed collisions.
- [ ] Each field has an explicit fallback rule that preserves stored ownership.
- [ ] Same facts produce the same actions through local and Index routes.
- [ ] Request counts establish the claimed performance improvement.
- [ ] Repair reports preserve unresolved records and do not modify audio files.
- [ ] Upstream parser changes include recrawl, signed-event and replica verification.
- [ ] Completeness rules cover track credits changing to feed credits, then to no credits.
- [ ] Source evidence survives text cleaning and display fallback.
- [ ] Cache expiry, explicit refresh and response ordering have named tests.

## Open Product Decisions

The operator answered the original scope and identity-placement questions on 2026-09-19.
The operator accepted the proposal-and-review process for field policies in R3.
The operator accepted artwork fallback without an additional visible owner label, as recorded in Decision C.
Field-specific priority and conflict proposals still require operator review before dependent code runs.
Packet 001 keeps its transport scope. It selects no placement and no extra field.

Work that the decisions create:

- Packet 003 must list the supported Nostr purpose values with checked examples.
- Packets 005, 006 and 007 must write a rule for every metadata field.
- Packets 022 to 025 must build the labelled sections that Decision B selects.
- Packet 026 must report the repair of old records before any repair runs.

## First-Packet Corrections — 2026-09-19

The operator requested corrections to the first-packet instructions, prerequisites, and checks.
This change resolves these findings:

| Finding | Correction |
|---|---|
| R4 | Packet 002 separates current and required storage and display results. C01 links to the supplied response |
| R5 | Dispatch prerequisites apply per packet. Documents can produce missing rules. Packets 008 and 011 now require field-rule documents |
| R7 | Packets 002–008 use one link check that tests files and headings. Packet 003 permits specification retrieval. Packet 002 uses the deliverable's link base |
| R8 | AGENTS.md and the delivery index now state Accepted, Ready, and held. The unsupported commit claim is removed |

Packet 003 also separates proposed syntax support from current parser behavior.
The operator must accept proposed policies before dependent code implements them.
Packet 001 retains its contributor transport scope.

### Verification

The [link checker](../runbooks/check-markdown-links.py) is a situational check for ADR 0075 document packets.
It tests local file paths, Markdown headings, explicit HTML anchors, and reference links.
It skips external URLs and fenced code. It makes no network requests.
The packet commands do not change the current directory.

Temporary fixtures checked these results:

- Valid links pass, including duplicate headings, Unicode, HTML anchors, and references.
- Nested list links are checked.
- Links in fenced code and inline code are excluded.
- A missing file returns failure.
- A missing heading returns failure.
- A missing reference definition returns failure.
- An unreadable input returns failure.

These checks are Green. The temporary fixtures were removed.
Application tests do not apply to the document and checker changes.

The shared STE checker identified sentence-length, paragraph-length, and American-spelling defects in the first packets.
Those confirmed defects are corrected.
The full checker still reports lexical findings. Its raw result is not Green.
The following retained terms need their technical meanings:

| Retained term | Meaning |
|---|---|
| Ready | The repository's packet status before dispatch |
| case | A named input and its expected test result |
| field kind | A data classification named by ADR 0075 |
| key | A map lookup value or identity key, as specified by context |
| request | An HTTP operation or a recorded task request |
| review | A recorded assessment of evidence, instructions, or implementation |
| evidence | Source material that supports a recorded claim |
| source form | The RSS or JSON syntax supplied to a test |
| acceptance | The repository's decision that a named requirement is satisfied |
| may | A retained qualification when the evidence does not establish certainty |

Do not replace these terms with unrelated dictionary alternatives.
Do not change a governing status or remove uncertainty to suppress a finding.
Packet checks now require review of lexical findings and correction of confirmed defects.
They distinguish that review from a raw Green checker result.

### Remaining Scope

Packets 002 and 003 are ready for document work when the operator releases their dispatch hold.
Their deliverables and policy reviews are not complete.
R1, R2, and R6 still require corrections before the affected later packets run.
This change does not implement metadata handling or resume visual checks.

## Packet 001 Implementation — 2026-09-19

The operator released the dispatch of packet 001 on 2026-09-19. The implementation is complete.

`src/api.rs::Contributor` now holds `entity_type`, `entity_id`, `position`, `role_norm`,
`source`, `extraction_path` and `observed_at`. Each new field is optional.
Serialization writes a new field only when the app holds a value.
The six existing fields keep their previous serialization.
`src/views.rs` and `src/feed_service.rs` set the seven fields to `None` when local code
constructs a credit. One test literal in `src/views.rs` now uses `..Default::default()`.

Eleven tests with the `adr_0075_contributor_transport` prefix enforce this result.
They use the operator's supplied response as the base fixture. They cover these cases:

- The round trip and the order of the three credits.
- The two HeyCitizen roles and their supplied positions.
- A feed-owned credit in a track.
- An older payload and explicit null values.
- An unknown `entity_type` value.
- A decoding error for a string in `position` or `observed_at`.

These checks are Green: the focused tests, `cargo test --locked --offline`,
`cargo check --locked --offline`, `cargo fmt -- --check`,
`cargo clippy --locked --offline -- -D warnings`, and
`cargo build --locked --offline --bin v4vmm`.

### Fields That Later Layers Still Lose

| Layer | Owner | Result |
|---|---|---|
| Storage, raw evidence | `src/identity_ingest.rs::persist_contributors` | The `raw_json` column of a newly fetched record now holds the seven fields. Old rows keep their old JSON |
| Storage, typed columns | `src/db.rs::LocalContributorRow` | Six fields remain. The position comes from the list index, not from the supplied value. The observation time stays `None`. The declared owner, normalized role, assertion source and extraction path have no column |
| Display preparation | `src/views.rs::ContributorView` | Six fields remain. All seven claim fields are lost |
| Local route | `src/feed_service.rs::contributor_from_local` | The local row supplies no provenance, so all seven fields stay `None` |

The storage packets and the projection packet must correct these losses.
Packet 001 does not establish complete storage preservation.

## Review Disposition

The audit establishes that the app must correct how it handles metadata.
The operator accepted ADR 0075 on 2026-09-19. The operator holds every remaining dispatch.
Packet 001 is complete. It only preserves fields in the app's type for API data.

The acceptance of the ADR closes no gate. Every open gate stays open.
The instructions and checks for packets 002 and 003 are corrected.
Correct R1, R2, and R6 before dispatching the affected later packets.

## Operator Visual Check

The operator paused visual checks on 2026-09-19. Do not request another visual batch until
the metadata work is ready and the operator resumes those checks.
The current fixture is `/tmp/v4vmm-governance.ie6k8TQf`. Cleanup is unconfirmed.

## References

- [Accepted ADR](../adr/0075-metadata-ownership-and-completeness.md)
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md)
- [First packet](../tasks/adr-0075-task-001-contributor-claim-transport.md)
- [Podcast person semantics](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md)
- [Podcast txt semantics](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md)
- [NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md)
