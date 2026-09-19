# ADR 0075: Metadata Ownership And Completeness

## Status

Proposed - 2026-09-19.

The operator prioritised correct metadata handling in v4vmm and MusicIndex.
The operator also paused visual checks. This proposal defines the contract for
that work. Implementation has not started. Existing acceptance gates remain open.
The [review](../reviews/adr-0075-metadata-contract-review.md) records the evidence.

This ADR covers the proposed app changes. Stophammer must record its own
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
descriptions distinct from feed descriptions. This proposal makes fallback
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

This proposal does not claim that the
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

The current proposal shows related feed and contributor identities with explicit
owner labels. The operator's placement preference remains open. Placement cannot
change ownership. A feed website must retain its feed label. A contributor key
must retain the contributor's name and role context.

Before changing fallback for individual fields, document a separate rule for each field.
Cover description, artwork, publisher, artist text, language, explicit state and dates.
A generic merge helper must not apply one field's rule to another field.
The existing contract still governs payment-route inheritance.

### 5. Replace A Complete Provider Snapshot Atomically

The key for replacement consists of the provider, declared subject and collection kind.
Assertion labels such as `rss_link` are attributes of facts within the snapshot.
They are not the provider key.

A track response can contain feed credits selected through inheritance.
That response does not authorise replacement of the feed snapshot.
Fetch the feed collection separately before replacing it.
Coverage applies to the requested subject and collection.

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

## References

- [ADR 0028](0028-local-identity-source-fact-persistence.md)
- [ADR 0053](0053-local-detail-source-fact-parity.md)
- [ADR 0054](0054-local-metadata-source-fact-persistence.md)
- [MoeFactz evidence](../reviews/adr-0037-review-checklist.md#task-002-contributor-source-check--2026-09-19)
- Stophammer ADRs 0039/0040: feed-scoped track identity.
- Stophammer ADR 0041: contributor npub source evidence.
