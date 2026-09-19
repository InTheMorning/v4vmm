# ADR 0075: Metadata Ownership And Completeness

## Status

Accepted - 2026-09-19. The operator accepted this decision and answered its two open questions.
Packet 001 is complete on 2026-09-19. It preserves contributor claim transport.
The operator holds the dispatch of every remaining packet.

Decision A, field scope. This contract covers every metadata field, not only identity and credit fields.
Each field keeps a separate written rule. A packet cannot run before its field rule exists.

Decision B, identity placement. A track page keeps track identities in its header.
Feed identities and contributor identities go in separate sections with owner labels.
Each row names its owner before the operator activates a link.

Decision C, artwork fallback. Show the track's artwork when it has its own image.
Otherwise, use the feed artwork in the track header without an additional visible owner label.
The stored artwork facts retain their source and owner.

Field-rule review. Document agents propose unresolved source priorities and conflict rules.
The operator reviews those proposals before the dependent code packets run.

The operator prioritised correct metadata handling in v4vmm and MusicIndex.
The operator also paused visual checks. The current acceptance gates remain open.
The [review](../reviews/adr-0075-metadata-contract-review.md) records the evidence.

This ADR covers the app changes. Stophammer must record its own
decision before it changes its parser, API or storage contract.
This ADR reserves no Stophammer ADR number. The deployed Stophammer revision remains unverified.

## Context

The operator requested MoeFactz with three collections in the API `include` parameter. MusicIndex returned
empty track identity lists and three populated contributor claims. Those claims
contain HeyCitizen's Nostr key and the Moe Factz host website.

The app's contributor data transfer object (DTO) omits fields from the response.
These fields are claim ownership, position, source, extraction path, observation
time and normalised role. Storage code cannot recover those fields from the
DTO's serialised JSON.

Separate problems affect extraction, refresh, fallback, requests and presentation.
A change to a missing button would leave those problems unresolved.

ADRs 0028 and 0054 require the app to preserve source data. ADR 0053 keeps track
descriptions distinct from feed descriptions. This decision makes fallback
explicit in the shared code that prepares data for display.

## Terms

| Term | Meaning in this ADR |
|---|---|
| Fact | A recorded source value, with its owner and supporting evidence |
| Subject | The feed, track or contributor that a fact describes |
| Claim | An assertion that a source makes about a subject |
| Provenance | Evidence of a value's owner, source, extraction path and observation time |
| Provider | The service or RSS resource that delivered the facts |
| Collection | A set of related facts in a response |
| Snapshot | A stored copy of a provider's collection from one observation |
| Coverage state | Whether the request returned the complete collection |
| Projection | Shared code that prepares source facts for display |
| DTO | The Rust type that represents an API response during decoding and serialisation |

## Decision

### 1. Preserve The Subject And The Source

Each fact retains its declared subject, field kind, value, source assertion,
extraction path and recorded observation time. Records of fetched data also
identify the provider and resource that delivered the fact.

Keep these parts of the record separate:

| Dimension | Example | Meaning |
|---|---|---|
| Subject | Feed GUID plus track GUID | The publication object described by the fact |
| Contributor occurrence | Subject plus source collection and position | One credit in one source observation. It does not establish a global person identity |
| Assertion source | `podcast_person` | How the publisher expressed the assertion |
| Provider | MusicIndex endpoint or direct RSS URL | How the app received the assertion |
| Observation time | API `observed_at` | The source's recorded observation time |
| Fetch time | Actual completed request time | When this app received the response |

Retain raw evidence with the typed facts. Do not invent missing provenance.
Serialising a cleaned DTO does not recover the original response.
The later ingestion design must identify where it retains evidence before source-text cleaning or fallback.
Older records with incomplete ownership remain explicitly unverified until a
new source observation supplies the missing evidence. Do not infer identity
from a matching name, URL, public key or payment address.

### 2. Make Collection State Explicit

The code that handles responses distinguishes these outcomes before it selects values for display:

| State | Meaning | Refresh effect |
|---|---|---|
| Not requested | This request omitted the collection | Keep the stored snapshot |
| Not returned | The request asked for the collection. The response omitted it or returned null | Keep the snapshot. Record incomplete coverage |
| Returned empty | A complete collection contains zero facts | Replace only that provider's collection with an empty snapshot |
| Returned populated | A complete collection contains facts | Replace only that provider's collection |
| Request failed | No successful observation exists for this request | Keep stored facts. Expose the failed refresh state |

Treat a malformed payload as a failed request. Do not treat it as an empty collection.
Record whether cached facts exist separately from the latest request outcome.
An offline view can contain known facts and report that its refresh failed.

A successful HTTP status alone does not establish collection completeness.
The request contract must define completeness for each endpoint, collection and API version.
It must cover pagination, inherited credits and unsupported includes.
An incomplete response cannot authorise deletion of stored facts.

### 3. Use Explicit Field Rules

| Source field | Declared owner | Handling |
|---|---|---|
| Channel RSS `link` | Feed | Preserve as feed `website` |
| Item RSS `link` or supported Atom alternate link | Track | Preserve as track `web_page`. Use it for an action that opens the page |
| `podcast:person` text, role, group, href, img and npub extension | Contributor occurrence | Preserve each credit and its attributes together |
| Direct channel `podcast:txt` with a supported Nostr purpose | Feed | Preserve the entity identity and exact extraction path |
| Direct item `podcast:txt` with a supported Nostr purpose | Track | Preserve the entity identity and exact extraction path |
| Transcript and enclosure links | Their declared subject and link kind | Keep separate from website actions |
| Unknown links, identifiers or extension syntax | Declared subject, when known | Preserve the evidence. Record whether the app supports the syntax |

Do not assign an npub from an arbitrary descendant element to the containing track.
Do not write a `podcast:txt` extraction path for a `podcast:person` value.

`npub` and `nprofile` have different encodings under
[NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md).
The parser must preserve that distinction. A recognised prefix alone does not
validate a value. Preserve invalid values as evidence. Do not treat them as
validated identifiers for identity actions.

The Podcast Namespace lets each service define its `txt` purpose values.
The parser packet must list the exact purpose values that it supports.
It must include checked examples. It must cover existing `purpose="npub"` support.
It must decide compatibility for `purpose="nostr"` before implementation.
It must identify unsupported syntax explicitly.

This decision does not claim that the
[txt specification](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md)
requires either spelling.

### 4. Keep Fallback Out Of Stored Facts

Store the source observation before computing a value for display.
A missing track field does not make a feed link, description, artwork or contributor a track assertion.
Do not store the feed fact as a track assertion.

The [person specification](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md)
makes item credits replace channel credits when item credits are present.
Preserve that rule when selecting credits for display.
Preserve the original owner of each credit. Do not combine feed and track
credits into one list of asserted track credits.

The operator selected separate, labelled sections (Decision B).
A track page keeps track identities in its header.
Feed identities go in a section that names the feed as the owner.
Contributor identities go in a section that names the contributor, the role and the source.
Placement cannot change ownership. A feed website must retain its feed label.
A contributor key must retain the contributor's name and role context.

Artwork follows Decision C. The track header can use feed artwork when the track has no artwork of its own.
This ordinary display fallback needs no additional visible owner label.
It does not make the feed image a track assertion or remove ownership from the stored facts.
Unknown ownership remains unknown until source evidence resolves it.

The operator selected every metadata field (Decision A).
Write a separate rule for each field before you change its fallback.
Cover description, artwork, publisher, artist text, language, explicit state, dates,
links, transcripts and enclosures. Each rule states the owner, the source order,
the conflict result and the displayed value when no source supplies the field.
Document agents propose any rule that an existing decision does not settle.
Mark those rules as proposals until the operator accepts them.
The dependent code packet requires the accepted field rule.
A generic merge helper must not apply one field's rule to another field.
The existing contract still governs payment-route inheritance.

### 5. Replace A Complete Provider Snapshot Atomically

The key for replacement consists of the provider, declared subject and collection kind.
Assertion labels such as `rss_link` are attributes of facts within the snapshot.
They are not the provider key.

The requested subject and each returned fact's declared subject are separate fields.
Before the storage packet, define how a response observation relates to snapshots for each declared owner.
Do not use the request subject as the owner of every returned fact.

A track response can contain feed credits selected through inheritance.
That response does not authorise replacement of the feed snapshot.
Fetch the feed collection separately before replacing it.
Coverage applies to the requested subject and collection.

The phase 002 contract must cover a track changing from its own credits to inherited feed credits, then to no credits.
For each transition, specify which collection is complete and which earlier facts can be replaced.
The storage packet cannot decide these rules during implementation.

A complete, empty Index response must remove that Index snapshot's earlier
`rss_link` or `podcast_txt` facts. It must not delete direct RSS observations,
tag values, MusicBrainz facts or user-authored data. Commit facts and coverage
state together. Failed or superseded requests cannot replace a newer snapshot.

Before changing the database, the storage packet must define:

- The migration.
- The provider identity.
- The checks of source revisions.
- The backup procedure.
- The rollback procedure.

Do not guess a provider for old rows whose provenance cannot establish it.
Preserve those rows for review.

### 6. Share Requests And Read Projections

Library and Index detail routes use the same rules to fetch facts and prepare
data for display. Those rules do not depend on a renderer.
Each named request profile specifies the required collections.
A missing include does not establish that a publisher supplied no metadata.

Search lists must not fetch every full detail merely to render a result page.
Measure existing request counts first. Fetch detail when the view needs it.
Reuse a completed or active request with the same endpoint, scoped identity and request profile.
Keep caches isolated by endpoint and source revision. RSS enrichment must report failures.

Before changing requests, define cache freshness, expiry and explicit refresh behavior.
Define response ordering when the service supplies no reliable source revision.
Do not use the source observation time as the app's request sequence.

Both routes receive the same typed actions for the same observed facts.
The shared model selects sources and determines availability. The UI displays
those decisions. It does not infer ownership or select a preferred source.

### 7. Repair Existing Data With Evidence

Before repairing stored facts, produce a report with:

- The old value.
- The proposed value.
- The declared owner.
- The supporting source.
- The unresolved cases.

Fetch source data again when necessary.
Do not delete the nine local Nostr records as a group based on the candidate count.
Do not write audio tags as a side effect of metadata refresh.

Changes to the Stophammer parser require a controlled plan to crawl or ingest feeds again.
An unchanged feed hash can cause ingestion to skip a feed.
Deploying a parser therefore does not prove that stored facts changed.
Preserve compatibility with signed events and replicas.

## Invariants

- Feed, track and contributor ownership survives every boundary.
- A track identity always includes its feed scope where the API supplies it.
- Multiple roles and conflicting assertions remain recoverable.
- A missing include, an empty collection and a failed request remain distinct.
- A source refresh cannot erase another provider's facts.
- Display fallback cannot become a new source assertion.
- Unknown provenance stays unknown until evidence resolves it.
- Raw values and recorded times remain available beside derived values.
- Accept a presentation or performance claim only after its named check passes.

## Non-Goals

- Canonical person merging, artist claiming or payment-owner inference.
- Broad metadata edits, automatic audio-tag repair or a production database rebuild.
- New configuration keys, presets, playback changes or release of ADR 0066's gate.
- Declaring all metadata complete from one feed, one API response or static review.

## Alternatives Considered

- Adding missing buttons leaves extraction and storage errors unresolved.
- Copying parent facts into track fields hides ownership.
- Treating MusicIndex as the sole source removes independent RSS and offline evidence.
- A shared parser crate may reduce differences between parsers. Assess that dependency separately before adopting it.

Start with one contract and common example cases.

## Consequences

The app can explain an absent field and preserve facts during failed refreshes.
The refactor needs storage and compatibility work in addition to UI work.
Earlier stored provenance may remain unresolved after the first repair pass.
The [phase plan](../plans/adr-0075-metadata-contract-phase-plan.md) separates these changes.

## Relationship To Existing Decisions

ADRs 0028 and 0054 remain binding, with one limited replacement.
This ADR replaces their use of assertion source as the replacement key for affected collections.
The replacement key is now the provider, the declared subject and the collection kind.
This ADR retains their separation of identity facts and metadata facts.
ADRs 0028 and 0054 and the ADR index record this limited replacement.
Their unrelated decisions and their open acceptance gates remain in force.

## References

- [ADR 0028](0028-local-identity-source-fact-persistence.md)
- [ADR 0053](0053-local-detail-source-fact-parity.md)
- [ADR 0054](0054-local-metadata-source-fact-persistence.md)
- [MoeFactz evidence](../reviews/adr-0037-review-checklist.md#task-002-contributor-source-check--2026-09-19)
- Stophammer ADRs 0039/0040: feed-scoped track identity.
- Stophammer ADR 0041: contributor npub source evidence.
