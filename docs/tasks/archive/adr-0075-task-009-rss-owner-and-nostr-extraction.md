# ADR 0075 Task 009: RSS Owner And Nostr Extraction

Status: Complete - 2026-09-20. Implementation and final orchestrator review passed. Mechanical checks are Green.

This packet corrects RSS enrichment under accepted ADR 0075 rules.
It retains evidence in the active app context. Packet 014 owns durable evidence storage.
The packet requires no visual acceptance and changes no production data.

## Goal

Extract feed and track identities only from their direct Podcast Namespace `txt` children with supported purpose `npub`.
Validate NIP-19 encodings before adding active identity facts.
Keep rejected syntax and its actual source location as evidence.
Parse each received enrichment document once without changing unrelated metadata selection.

## Accepted Requirements And Scope

[ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md) supplies the ownership, evidence, and validation requirements.
Decision D supports only the `npub` purpose value.
The [syntax contract](../../schema/adr-0075-identity-syntax-contract.md) distinguishes `npub` from `nprofile` and prohibits contributor promotion.
The [example corpus](../../schema/adr-0075-metadata-example-corpus.md) supplies cases C02, C03, C09, and C10.

This packet implements those requirements in the live enrichment path.
It does not implement provider priority, durable snapshots, contributor display, or complete identity-action validation across existing data.
Decisions F and G retain their separate implementation packets.
Unsupported purpose values do not become accepted through normalization or fallback.

The parser replacement and context carrier below are implementation choices for orchestrator review.
They introduce no new product source preference or identity form.
An ambiguity outside the specified syntax must remain evidence until its own rule exists.

## Behavior Before Implementation

`rss::enrich::fetch_track_enrichment_from_feed` parses a response through `rss::Channel`.
It selects the first item whose GUID or primary enclosure matches the requested track.
It returns no observation when no item matches.

`nostr_from_extension` recursively searches unrelated elements, text, and attributes for an identity prefix.
It can promote a contributor key into a feed or track identity.
It does not validate checksums, decoded lengths, or profile TLVs.
Both encodings receive `nostr_npub` and an invented `purpose=nostr` extraction path.

`enrich_track_context_from_rss` currently discards the enrichment result.
Malformed or unsupported identity text has no live evidence carrier.
Literal extension-prefix lookups also lose alternate namespace prefixes and cannot detect a rebound prefix reliably.

## Files To Inspect

- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md), Decisions 1, 3, 4, and D.
- [Packet 003](../adr-0075-task-003-nostr-syntax-and-purposes.md) and its [syntax contract](../../schema/adr-0075-identity-syntax-contract.md).
- [Packet 007](../adr-0075-task-007-field-rules-links-and-media.md) and its [link rules](../../schema/adr-0075-field-rules-links-and-media.md).
- [Example corpus](../../schema/adr-0075-metadata-example-corpus.md), C02, C03, C09, C10, and checked encoding vectors.
- [enrich.rs](../../../src/rss/enrich.rs), the complete enrichment path and its tests.
- [helpers.rs](../../../src/rss/helpers.rs), text cleaning, artist-role matching, and duration parsing.
- [mod.rs](../../../src/rss/mod.rs), reachable RSS module exports.
- [metadata.rs](../../../src/metadata.rs), `TrackContext`, sanitization, `nostr_from_ids`, and tag export consumers.
- [subscribe_service.rs](../../../src/subscribe_service.rs), enrichment callers and retained subscription context.
- [feed_service.rs](../../../src/feed_service.rs), `merge_track_context_from_detail` and local context construction.
- [feed.rs](../../../src/application/queries/feed.rs), the inspector context query.
- [views.rs](../../../src/views.rs), `nostr_npub_from_ids` and API projections.
- [identity_ingest.rs](../../../src/identity_ingest.rs), existing persistence boundaries, for inspection only.
- [Cargo.toml](../../../Cargo.toml) and [Cargo.lock](../../../Cargo.lock).
- [architecture_tests.rs](../../../tests/architecture_tests.rs), `source_fact_placeholder_and_breadcrumb_regressions_are_guarded`.

Inspect these cached dependency sources without changing them:

- `rss-2.0.12/src/channel.rs`, `Channel::read_from` and `Channel::from_xml`.
- `rss-2.0.12/src/util.rs`, `element_text`.
- `rss-2.0.12/src/extension/itunes/mod.rs`, namespace and repeated-value handling.
- `roxmltree-0.21.1/src/lib.rs` and `src/parse.rs`, resolved namespaces and parsing options.
- `quick-xml-0.37.5/src/reader/mod.rs`, declaration decoding.

## Files To Change

| File | Permitted change |
|---|---|
| `src/rss/enrich.rs` | One DOM parse, field extraction, observation creation, and context mutation |
| `src/rss/identity.rs` | New private module for typed identity evidence and the narrow NIP-19 decoder |
| `src/rss/mod.rs` | Declare the new module and export only the types used by live contexts |
| `src/metadata.rs` | Add the app observation field and a context constructor. Preserve it through sanitization |
| `src/subscribe_service.rs` | Build contexts before enrichment. Preserve incoming observation ownership through subscriptions |
| `src/feed_service.rs` | Build the merged context before enrichment and preserve the returned observation |
| `src/application/queries/feed.rs` | Build the inspector context before enrichment |
| `Cargo.toml`, `Cargo.lock` | Add direct dependencies on already locked parser versions without unrelated upgrades |
| `tests/architecture_tests.rs` | Update two affected call-shape assertions and add the situational ADR 0075 boundary guard |
| This packet | Record actual implementation checks and remaining limits |

Mechanical `TrackContext` constructor changes are also permitted in these files:

- `src/app/search_dispatch.rs`.
- `src/discover/app_impl.rs`.
- `src/discover/tests.rs`.
- `src/library/app_impl.rs`.
- `src/application/commands/download.rs`.
- `src/application/commands/payment_routes.rs`.
- `src/application/ports/download_manager.rs`.
- `src/subscribe_service/materialization.rs`.

Those mechanical changes must not alter commands, payment logic, renderers, or test expectations unrelated to context construction.
`src/musicbrainz.rs` has a different private `TrackContext`. Leave that type unchanged.

## One Parsed Enrichment Observation

Use `roxmltree` version `=0.21.1`, already present in `Cargo.lock`, as a direct dependency.
Use `quick-xml` version `=0.37.5` with its existing `encoding` feature for declaration decoding only.
Both versions are already cached. Do not change their versions or other dependency pins.

A bounded prolog read may identify the declared encoding before decoding the response.
Do not read the complete document through `quick-xml` and then parse it again.
Keep the original response bytes before decoding. Reject decoding errors without lossy substitution.

Use one full `roxmltree::Document` parse for the enrichment observation.
Reject DTD processing and external entities. Do not add a fallback full parser after a DOM error.

Extract ordinary metadata and identity evidence from this same document.
The HTML parser in the lockfile is unnecessary here. Readable-description comparison belongs to packet 035.
Do not strip HTML from descriptions or reinterpret their content in this packet.

Keep `fetch_feed_podroll` and RSS subscription parsing unchanged.
Their separate requests are not a second parse of this enrichment observation.
Delete the obsolete recursive identity scan and prefix-only extraction functions after replacing their live calls.
Keep shared helpers that subscription code still uses.

### Compatibility During The Parser Replacement

Preserve the current enrichment values for supported existing input:

| Value | Existing behavior to preserve |
|---|---|
| Item selection | First item with matching GUID or primary enclosure URL. Comparisons remain exact |
| Feed title and description | Direct channel fields, followed by existing `clean_text` handling |
| Feed artist | iTunes channel author only |
| Feed image | iTunes channel image, then channel image URL |
| Feed episode count | Number of parsed items, converted with checked `i32` conversion |
| Track title and description | Direct item fields, followed by existing `clean_text` handling |
| Track artist | iTunes author, item author, then the first supported contributor-role match |
| Track image | iTunes item image |
| Track number | Direct Podcast Namespace episode text parsed as `i32` |
| Duration | Existing `parse_itunes_duration`, followed by checked `i32` conversion |
| Publication date | Existing RFC 2822 parser |
| Transcript | First direct Podcast Namespace transcript URL and media type |
| Applying values | Existing missing-value checks. Keep populated scalar values and existing placeholder handling |

Preserve direct text and CDATA behavior. Do not concatenate descendant markup into a new scalar value.
Preserve existing repeated-element selection for ordinary RSS and iTunes fields.
Use characterization fixtures against the cached `rss` behavior when that selection is unclear.

Keep RSS 2.0 channel items and accepted RSS/RDF root-level item forms.
`Channel::read_from` accepts RDF input and appends root-level items after channel items.
Preserve that matching sequence. Do not confuse a namespace-qualified item with an unrelated extension element.
Test UTF-8 and one declared legacy encoding already accepted by the current parser.
Report an unsupported legacy shape before reducing parser coverage.

## Namespace And Ownership Rules

Compare resolved namespace URIs and local names. Never accept a value because its prefix spells `podcast`.
Recognize these Podcast Namespace URIs, matching the inspected upstream namespace helper:

- `https://podcastindex.org/namespace/1.0`.
- `https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/1.0.md`.

Keep URI comparison exact. Namespace URIs are not interchangeable through case folding or URL normalization.
Alternate prefixes and a default namespace on the `txt` element must behave identically.
A prefix rebound to another URI must not supply a Podcast Namespace identity.

For each channel or item, inspect only its direct `txt` children in a recognized namespace for entity claims.
Read the unqualified `purpose` attribute. Preserve its original value.
Apply the existing upstream trim and ASCII lowercase treatment before comparing the purpose to `npub`.
Missing, blank, `nostr`, and other purpose values remain unsupported evidence.

Read the complete direct text value as one candidate, after trimming surrounding whitespace for validation.
Do not search arbitrary substrings, URLs, descendant elements, or attributes for an encoding.
Do not remove a `nostr:` wrapper or extract a key from a web URL.
Mixed element content remains malformed identity evidence rather than an identity candidate assembled from fragments.

A `podcast:person` key remains contributor evidence in the raw observation.
It must never enter feed or track `source_ids` through this path.
Nested `txt` elements, transcript attributes, and payment descendants also cannot supply an entity identity.

Use observed item and channel GUIDs when they exist. Do not claim that a requested GUID appeared in the response.
Keep feed resource scope and requested identity separately in the observation.
If an enclosure match selects another declared track GUID, retain the observation without promoting that item's identity into the requested track.
A known disagreement between requested and observed feed GUIDs likewise prevents identity promotion.
These checks do not change the existing scalar item-matching behavior in this packet.

## NIP-19 Decoder Contract

Implement a narrow decoder in `src/rss/identity.rs`. No Bech32 crate is currently present in `Cargo.lock`.
The decoder accepts only the complete `npub` and `nprofile` encodings covered by the syntax contract.
It performs no signing, key generation, network access, or relay connection.

The decoder must check:

1. ASCII Bech32 characters and the separator.
2. Uniform lowercase or uppercase. Reject mixed case.
3. The Bech32 checksum constant `1`. Reject Bech32m.
4. Conversion from five-bit groups to bytes without excess or nonzero padding.
5. Exactly 32 decoded bytes for `npub`.
6. Complete type-length-value headers and values for `nprofile`.
7. Exactly one type-0 public key of 32 bytes for `nprofile`.
8. ASCII relay text for type-1 fields, with repeated relay hints preserved.
9. Unknown TLV fields retained in their original order and byte representation.

Do not apply the original 90-character Bech32 address limit to NIP-19 profiles.
The checked profile vector in the syntax contract exceeds that limit.
Use checked indexing and bounded integer operations for all untrusted lengths.
Malformed data returns a typed validation result. It must not panic.

Represent valid values with separate public-key and profile variants.
The profile variant retains its public key, relay hints, and complete TLV evidence.
Do not convert a profile into an `npub` value.
Keep the original text beside any lowercase comparison or output form.

Only a validated bare public key can produce scheme `nostr_npub`.
Only a validated profile can produce scheme `nostr_nprofile`.
The current display selectors recognize only `nostr_npub`. Do not disguise a profile to activate those selectors.
Later identity-action work owns profile actions and validation of existing Index or stored facts.

## Live Evidence Carrier

Add an app-owned `RssObservation` type, separate from API DTOs.
Add `rss_observation: Option<Arc<RssObservation>>` to `metadata::TrackContext`.
Use `Arc<[u8]>` for the original response bytes. Context clones must share this evidence.
Add a `TrackContext::new(track, feed)` constructor that initializes the observation to `None`.
Do not add app observation fields to `api::Feed`, `api::Track`, or `SourceEntityId`.

Each successful parsed observation must contain:

- Requested RSS URL and final response URL.
- Actual fetch completion time, distinct from unknown publisher observation time.
- Original response bytes and the decoder's character encoding.
- Requested track GUID and enclosure URL.
- Observed feed GUID, when present.
- A typed item-match outcome, including no matching item.
- Typed evidence for direct owner `txt` occurrences, including unsupported and malformed values.

Each typed `txt` evidence entry must contain:

- Feed or item ownership, document occurrence, and available source GUIDs.
- Original purpose, direct text, element XML, and resolved namespace URI.
- Source position and an extraction path that identifies the actual element.
- A typed result: validated public key, validated profile, unsupported purpose, unsupported encoding, or malformed encoding.

Keep missing values separate from empty strings in the evidence.
The full response also retains contributor, nested-element, and unknown-namespace evidence excluded from active identity extraction.
Decoded element XML is character evidence. The original bytes remain the authoritative response copy for legacy encodings.
Do not print raw response bodies or candidate values in errors or `Debug` output.

Use one-based element ordinals in extraction paths and zero-based occurrence positions in fact fields.
For example, a direct RSS 2.0 item claim can use `rss/channel/item[3]/podcast:txt[2]/text()`.
The `podcast` path component identifies the resolved recognized namespace. The raw element retains its published prefix.
Adapt the root and owner path to the actual RSS/RDF structure.
Never invent a `purpose=nostr` path or a `txt` path for an attribute read from another element.

### Fetch Result And Mutation

Replace the fetch function's outer `Option` with a successful result that always contains the observation.
That result contains optional matched-item `RssTrackEnrichment` data.
No matching item means no scalar enrichment, while the successful observation remains available.
Malformed XML remains an error. It must not become an empty successful observation.

Change `enrich_track_from_feed_rss` to accept a mutable `TrackContext` and the feed URL.
Attach the successful observation before applying its optional scalar enrichment and eligible identity facts.
Change `enrich_track_context_from_rss` to accept a mutable `TrackContext`.
Resolve its RSS URL using the existing track/feed rule, then call the enrichment entry point.
This mutation API prevents a caller from ignoring a returned evidence object.

On no match, preserve the observation and leave existing track and feed values unchanged.
On a failed request or parse, preserve any prior successful observation and existing facts.
No packet owns the separate typed RSS refresh-failure state. The operator deleted packet 019 on 2026-09-21. This packet must not claim that failure reporting is complete.
Packet 018 owns cache ordering and freshness.

Generate active `SourceEntityId` values only from the validated variants of eligible direct-owner evidence.
Set the actual owner, source position, scheme, value, and extraction path.
Keep source observation time unknown when RSS supplied none. Fetch time must not fill `observed_at`.
Keep repeated source occurrences separate, even when their values match.
Repeated application of the same observation must not add identical copies of one occurrence.

Deduplication must include owner, source, position, scheme, value, and extraction path.
An equal Index value must not suppress independent RSS evidence.

## Construction And Consumer Map

| Current site | Required integration |
|---|---|
| `feed_service::merge_track_context_from_detail` | Construct the merged context first, enrich it, then sanitize it |
| `feed_service::track_row_to_track_context` | Construct a local context with no invented RSS observation |
| `application::queries::feed::fetch_track_detail` | Construct the inspector context before enrichment. Return the same context with its observation |
| `subscribe_service::subscribe_feed_retaining` | Enrich the constructed per-track context. Preserve its observation when constructing the subscription request |
| `subscribe_service::subscribe_track_from_search_internal` | Preserve the incoming context observation while applying existing feed defaults and any subsequent enrichment |
| `subscribe_service::download_and_compare_track` | Construct and enrich the context before comparison or download processing |
| `subscribe_service::subscribe_library_track_internal` | Construct a local context with no source observation unless one was actually fetched |
| `subscribe_service::materialization::Materialization` | Its owned context keeps the shared observation through retry. Do not refetch during retry |
| Metadata sanitization and tag comparison | Leave observation bytes and typed evidence unchanged. Only active identity facts can feed existing identity selectors |
| `views::nostr_npub_from_ids` and `metadata::nostr_from_ids` | They receive only eligible newly extracted `nostr_npub` values from this path. They must never read rejected evidence |
| Existing identity persistence | No new persistence behavior. Packet 014 must consume the app observation carrier before claiming durable preservation |

Confirm the current function names before editing. Do not change unrelated code when a surrounding function moved.
Replace the remaining context literals mechanically with `TrackContext::new` or explicitly preserve an existing observation.
Do not reconstruct an observed context from its track and feed while dropping its observation.

## Required Regression Tests

Use unit tests beside the owning code. Do not add another integration test file.
Use the checked public vectors from the syntax contract, including the expected decoded keys and relay hints.
Do not use placeholder-looking positive encodings from older UI tests.

| Test class | Required evidence |
|---|---|
| Direct ownership | Channel and item keys remain separate. Contributor-only keys create no entity IDs |
| Position | Repeated equal direct claims retain their source positions. Nested claims create no active entity ID |
| Purpose | `npub` succeeds. Missing, blank, `nostr`, and unrelated purposes remain evidence only |
| Namespace | Canonical, alternate, default, and inherited prefixes agree. Rebound and unrelated namespaces produce no active identity |
| Exact candidate | Surrounding whitespace is handled. Embedded URLs, wrappers, multiple values, and mixed element content do not produce substring matches |
| Public key decoder | Checked vectors decode correctly. Wrong checksum, Bech32m, mixed case, bad alphabet, padding, and key length fail |
| Profile decoder | Checked profile retains relays. Missing, duplicate, short, or long required keys fail |
| TLV boundaries | Truncated headers and values fail. Unknown complete TLVs remain in the valid profile evidence |
| Scoped match | An enclosure match with a different declared item GUID cannot promote that item's identity into the requested track |
| No item match | The successful observation remains on `TrackContext`. Existing values remain unchanged |
| Failure | A request or XML failure does not clear prior facts or a prior successful observation |
| Evidence reachability | Unsupported and malformed text survive the live enrichment entry point, context cloning, and sanitization |
| Identity isolation | Rejected evidence produces neither `source_ids` nor a Nostr value through the existing metadata and view selectors |
| Profile typing | A valid profile remains `nostr_nprofile` and cannot appear as `nostr_npub` |
| Existing metadata | Titles, descriptions, artist priority, images, counts, number, duration, date, and transcript match characterization fixtures |
| Existing document support | RSS 2.0, RDF items, CDATA, entities, and declared legacy encoding retain their supported values |
| One observation | One HTTP response supplies one full DOM parse and the recorded raw bytes. No second full `Channel` parse remains |

For wrong payload lengths and padding, use checksum-valid negative fixtures.
Otherwise, checksum failure alone would leave those validation branches untested.
Test unknown TLVs without a required public key separately from valid profiles with unknown TLVs.

Keep the existing placeholder and populated-value enrichment tests.
Update only the changed call shapes in `source_fact_placeholder_and_breadcrumb_regressions_are_guarded`.
Keep its placeholder and breadcrumb requirements intact.
Add a situational ADR 0075 guard for the live parser boundary and the absence of the recursive identity scan.
Delete that situational guard when ADR 0075 is superseded.

## Do Not Touch

- API DTO definitions and Index response selection.
- Database schemas, production data, migrations, repair operations, and upstream files.
- `src/rss/subscribe.rs` and the podroll parser.
- Payment routing, audio tags, broadcast scheduling, and playback behavior.
- View layout, owner labels, profile actions, and existing visual acceptance gates.
- Shared plans and status files owned by the orchestrator.

## Implementation Steps

1. Read the accepted rules and the inspected parser behavior.
2. Add the live observation carrier and update context construction.
3. Implement the narrow decoder with checked vectors and rejection tests.
4. Replace enrichment parsing with one namespace-aware DOM parse.
5. Preserve existing scalar extraction and add direct-owner identity evidence.
6. Connect successful and no-match observations to the active context.
7. Remove the obsolete recursive scan and update its guards.
8. Run the checks and record their actual results.

## Acceptance Criteria

Mechanical acceptance requires every regression class above to pass at its owning layer.
The orchestrator must inspect the live call chain and the complete diff before marking this packet complete.
No rejected candidate may reach an active identity fact through the new extraction path.
No successful parsed observation may lose its evidence merely because no requested item matched.
No ordinary enrichment field may change source priority during this correction.
No code may exist only for an unimplemented future consumer.

This packet makes no claim about persistent evidence after restart.
Existing invalid Index or contributor actions remain outside this packet's repair scope.
Those limits remain assigned to their separate packets.

## Test Commands

Run from the repository root after implementation:

```bash
cargo test --offline adr_0075_rss_
cargo test --offline rss_enrichment_
cargo test --offline --test architecture_tests
cargo check --offline
cargo fmt -- --check
cargo clippy --offline -- -D warnings
cargo test --offline
cargo build --offline --bin v4vmm
python3 docs/runbooks/check-adr0075-identity-examples.py
python3 docs/runbooks/check-markdown-links.py docs/tasks/archive/adr-0075-task-009-rss-owner-and-nostr-extraction.md
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" --check --no-heuristics docs/tasks/archive/adr-0075-task-009-rss-owner-and-nostr-extraction.md
git diff --check
```

The first build may update only the root dependency edges in `Cargo.lock`.
Inspect that diff before accepting the dependency change. Do not update unrelated locked packages.
After the lockfile update, repeat required Cargo checks with `--locked` when applicable.
Do not run the desktop application.

## Rollback And Escalation

Rollback restores only this packet's code, dependency edges, and task report.
It changes no schema or production data.
Escalate an unsupported existing RSS encoding or shape before reducing compatibility.
Escalate any proposed URI alias, identity wrapper, or new purpose. Do not add support implicitly.

Escalate any construction path that cannot preserve the observation without unrelated behavior changes.
Do not implement a storage workaround to close packet 014's separate requirement.

## Implementation Results

Completed on 2026-09-20. The final orchestrator production review passed.

The enrichment request now creates one DOM observation and retains it on `TrackContext`.
The observation keeps original response bytes, resolved namespaces, source positions, owner GUIDs, request identity, and actual fetch time.
Direct owner claims with supported purpose `npub` receive NIP-19 validation.
Rejected claims remain evidence. Contributor keys and nested claims cannot become entity identities.

Validated public keys use `nostr_npub`. Validated profiles use `nostr_nprofile` and retain their relay hints and ordered TLVs.
Known item or feed GUID disagreements prevent identity promotion.
Repeated equal RSS occurrences retain separate positions. An equal Index fact does not suppress RSS evidence.
Applying the same observation again does not duplicate its active facts.

The parser preserves the measured `rss` 2.0.12 scalar behavior.
Ordinary fields use the last occurrence. Channel titles and descriptions retain the last XML-nonempty value before placeholder handling.

iTunes fields use the first occurrence. GUID matching retains the previous trimming behavior.
Scalar text joins direct text and CDATA while skipping nested markup.
Mixed identity markup remains malformed evidence, with its direct text and element XML retained.

RSS and RDF root items follow channel items. Root artwork retains its previous priority over ordinary channel artwork.
The parser accepts UTF-8 and the checked Windows-1252 fixture without changing original response bytes.
The unapproved HTTPS iTunes namespace alias was removed. The two specified Podcast Namespace URIs remain supported.
Invalid XML and DTD declarations fail without exposing response text through parser errors.

Context construction, subscription refresh, cloning, sanitization, and materialization retain the observation.
No-match responses retain their observation without changing existing facts.
Failed requests and failed parses preserve prior facts and the prior successful observation.
The retry test converts retained WAV input while preserving the same observation and response-byte allocations.
The architecture guard prohibits another RSS enrichment request during materialization retry.

Both complete original enrichment tests were restored from the baseline revision.

### Verification

Focused checks are Green:

| Check | Result |
|---|---|
| `cargo test --locked --offline adr_0075_rss_` | 33 unit tests and one architecture guard passed |
| `cargo test --locked --offline rss_enrichment_` | Both original enrichment tests passed |
| `cargo test --locked --offline --test architecture_tests` | 264 guards passed |
| Identity example checker | 13 valid examples, two intentional invalid examples, and three decoded vectors passed |

The focused suite covers every regression class listed in this packet.
Sixteen independently encoded negative fixtures reach checksum, padding, key-length, relay-text, and TLV rejection branches.
Separate tests cover invalid characters, separators, mixed case, uppercase values, unknown encodings, and candidate truncation.
Valid profiles retain unknown TLVs and repeated relay hints in their original order.

Local HTTP fixtures exercise the live enrichment entry point and the service wrapper.
They check rejected-evidence retention, profile isolation, no-match results, failed refreshes, redirects, and raw response preservation.
The first restricted run could not bind loopback ports. The authorized rerun passed outside that sandbox.

The orchestrator also verified the combined working tree:

| Check | Result |
|---|---|
| Formatting and `cargo check` | Green |
| `cargo clippy -- -D warnings` | Green |
| Complete test suite | 1,571 unit tests and 264 architecture tests passed |
| Documentation examples | Ten examples remained ignored |
| Normal desktop binary build after tests | Green |
| Final production review | Passed |

The lockfile changes add only the two root dependency edges for already locked parser versions.
No dependency version changed. No application launch or production-data mutation occurred.

Local links and diff whitespace are Green. The shared STE checker found no structural defects after correction.
Lexical findings remain for technical wording. The raw STE result is not Green.

### Remaining Boundaries

This packet preserves evidence in memory. Packet 014 owns durable evidence after restart.
Packet 018 owns cache ordering and freshness. No packet owns typed RSS refresh-failure state.

Existing invalid Index or stored identity facts remain outside this packet's repair scope.
The current metadata and view selectors still exclude profiles from `nostr_npub` output.
No profile actions, source-priority policy, payment behavior, or audio-tag policy changed.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet and all files under Files To Inspect.
- `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.
- `/home/citizen/.agents/skills/rust-skills/rust-dev/SKILL.md`.

Goal:
- Correct direct RSS identity extraction and preserve its observation in the active context.

Constraints:
- Follow the specified parser, decoder, evidence, and consumer contracts.
- Preserve unrelated enrichment behavior and existing dependency versions.
- Keep unsupported and malformed candidates outside active identity facts.

Do not touch:
- All paths and behaviors listed under Do Not Touch.

Acceptance criteria:
- Satisfy every mechanical criterion and regression class in this packet.
- Keep durable storage and existing-data validation limits explicit.

Test commands:
- Run all commands under Test Commands after implementation.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

This packet changes extraction and in-memory evidence. It changes no layout or presentation contract.
No new visual batch is requested. The existing visual pause and inherited gates remain open.
No operator fixture or cleanup is required.
