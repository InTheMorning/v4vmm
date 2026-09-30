# ADR 0075: Metadata Ownership And Completeness

## Status

Accepted - 2026-09-19. The operator accepted this decision and answered its two open questions.
Amended 2026-09-19 with Decisions D to G, and 2026-09-20 with Decision H.

Amended 2026-09-21. The operator recorded Decision I: MusicIndex is a cache of RSS, and
RSS is the only provenance. The operator also reduced the field scope in Decision A and
deferred nine field policies. Provider-ownership display work is superseded.
The app reports a stale MusicIndex record and directs the operator to podping.me.

Amended 2026-09-26: the MusicIndex title refinements no longer name the legacy `name` field. The contract declares no such field.

Amended 2026-09-30: a MusicIndex `release_date` claim with the path `feed.pub_date` now proves a channel publication date. The operator decided this on 2026-09-30.
The MusicIndex oldest-item date shows as the derived fact "First track published", never as a release date.

Amended 2026-09-24: [ADR 0076](0076-playlist-rss-check-for-stale-musicindex-records.md) supersedes Decisions F, G and H.
It also supersedes two Decision I rules. One rule gives fresh RSS the value. The other rule limits MusicIndex to a feed that the app has not fetched.
It supersedes the provider priority, freshness, expiry and stale-label parts of each field refinement. The other parts stay in force.

This section records the decision only.
The [phase plan](../plans/adr-0075-metadata-contract-phase-plan.md) records packets, checks, and open gates.

The operator authorized orchestration of the remaining work on 2026-09-20.
The orchestrator dispatches bounded tasks under accepted rules. Unaccepted policies and visual gates remain open.
The operator requires a separate review of each new field policy, confirmed on 2026-09-20.

Decision A, field scope. This contract covers every metadata field, not only identity and credit fields.
Each field keeps a separate written rule. A packet cannot run before its field rule exists.

Amended 2026-09-21: the operator reduced this scope. The contract covers each field with
an accepted rule. Nine field policies in packets 031 and 034 are deferred. They cover
track and disc numbers, season, embedded album title, artist sort text, medium and release
kind, aggregate counts, iTunes feed type, and relationship evidence. A deferred field
follows the general source rule in Decision I and gets no field-specific refinement.
This contract no longer claims coverage of every metadata field.

Decision B, identity placement. A track page keeps track identities in its header.
Feed identities and contributor identities go in separate sections with owner labels.
Each row names its owner before the operator activates a link.

Decision C, artwork fallback. Show the track's artwork when it has its own image.
Otherwise, use the feed artwork in the track header without an additional visible owner label.
The stored artwork facts retain their source and owner.

Decision D, Nostr purpose values. The app supports the `podcast:txt` purpose value `npub` only.
The app treats `purpose="nostr"` as unsupported syntax. It keeps an unsupported value as evidence.
The operator made this decision on 2026-09-19 and rejected the compatibility proposal in packet 003.

Amended 2026-09-19: added Decision D. The amendment limits the supported purpose value to `npub`.
It tightens this decision and reverses no earlier rule.

Decision E, supported enclosures. Each operation selects only a format that it supports.
Select the first supported primary enclosure, then the first supported enclosure, then a supported direct enclosure.
The priority order is shared. Different operation capabilities can produce different selections.

ADR 0076 supersedes Decision F on 2026-09-24.

Decision F, description and website priority. Prefer a fresh, successful direct RSS observation over the corresponding MusicIndex value.
Compare the same declared owner and field. This rule covers feed and track descriptions, feed websites, and track page links.
Keep both providers' facts. This rule selects display values only.

Other fields retain their separate rules.

ADR 0076 supersedes Decision G on 2026-09-24.

Decision G, retained discrepancies. Preserve a structured discrepancy when comparable RSS and MusicIndex values differ.
Keep both original values, the owner, the field, provider resources, and available source observation times.
Record actual fetch times and whether the discrepancy is active or resolved.
Repeated observations update the same discrepancy. A later matching observation resolves it without deleting its evidence.

Compare descriptions by readable text. HTML formatting and equivalent whitespace alone do not create a discrepancy.
An omitted collection or a failed request is not a conflicting value.
Decision I amends this rule. A discrepancy with a fresh RSS value identifies a stale MusicIndex record.

Retain evidence for a possible future update hook. This decision authorizes no hook implementation or outbound update request.

Amended 2026-09-19: added Decisions E, F, and G after the operator's review.
They define supported enclosure selection, limited RSS priority, and retained discrepancy evidence.
They tighten field handling without changing ownership or releasing code dispatch.

Field-rule review. Document agents propose unresolved source priorities and conflict rules.
The operator reviews those proposals before the dependent code packets run.

ADR 0076 supersedes Decision H on 2026-09-24.

Decision H, explicit RSS absence. Fresh verified RSS absence hides the retained MusicIndex description or track page value.
This rule covers feed descriptions, track descriptions, feed websites, and track page links.
Retain the earlier values and discrepancy evidence. Unknown coverage or a failed request cannot establish absence.
The operator accepted this rule on 2026-09-20 and separately confirmed the feed-description case.

Decision I, RSS is the only provenance. MusicIndex is a cache of RSS facts, and it is not
an independent source. Provenance names the element in the RSS document that asserted a
value: the channel, the item, or a person.

The provider that delivered a value is transport evidence. It is not provenance, and it is
not a display concern. A screen does not label a value with the provider that carried it.

ADR 0076 supersedes this paragraph on 2026-09-24: A fresh, successful direct RSS observation always supplies the value. MusicIndex supplies a
value only for a feed that the app has not fetched.

A MusicIndex value that disagrees with a fresh RSS value is a stale cache record, and it is
not a competing claim. The app reports that record as stale and names the feed.

The app directs the operator to podping.me to request a refetch. The app sends no podping
and makes no outbound update request. Only a later decision can change that restriction.

The operator accepted this rule on 2026-09-21.

Amended 2026-09-21: added Decision I. It supersedes the treatment of MusicIndex as a
competing source in Decisions F and G. It also deletes the earlier rule that a fresh Index
absence hides a stale RSS value. A cache cannot establish absence in its own source. Decisions F and G keep their retained evidence and their comparison rules, which
Decision I uses to find a stale record.


Description refinements, accepted separately for feed and track fields on 2026-09-20:
Prefer an owner-matching MusicIndex description claim before its plain description field.
When both description sources are stale, retain the last selected description with a stale label.
Use declared source order for multiple sourced descriptions. Report conflicting ties as unresolved.

Feed-website refinement, accepted on 2026-09-20: apply the same absence, source-order, conflict, and stale-label rules.
This field continues to use website claims. The feed's RSS resource URL is not a website fallback.

Track-page refinement, accepted separately on 2026-09-20: apply the same absence, source-order, conflict, and stale-label rules.
Prefer the direct item page link before a supported Atom alternate link.

Track-description placement, accepted on 2026-09-20: show no description when the track has no description of its own.
Do not substitute a feed description or add the proposed fallback "Feed description" section to that track page.

Feed-artwork source order, accepted on 2026-09-20: prefer fresh direct RSS artwork before MusicIndex artwork.
Retain both source facts with their feed owner.
Track-artwork source order, accepted separately: prefer fresh RSS track-owned artwork before MusicIndex track-owned artwork.
Then use the accepted feed-artwork fallback. Unknown artwork ownership remains unknown.

Feed-artwork refinements, accepted on 2026-09-20: honor fresh explicit absence using the description fields' provider priority.
Retain removed artwork as evidence. When both providers expire, retain the last selected field state.
Keep a retained image visible and report its stale state in metadata details. Retained absence stays absent.

Track-artwork refinements, accepted separately on 2026-09-20: apply the same removal and stale-state rules.
When the track's own image is absent, use the accepted feed-artwork fallback.
Retain removed track artwork as evidence. Expiry cannot restore a removed track image.

Feed-artwork extraction order, accepted on 2026-09-20: prefer `podcast:image`, then iTunes artwork, then the RSS image.
Report conflicting ties as unresolved and retain their evidence.
Track-artwork extraction order, accepted separately: prefer `podcast:image`, then iTunes artwork. Report conflicting ties as unresolved.

Legacy track artwork, accepted on 2026-09-20: keep the image for display and report unknown ownership in metadata details.
The compatibility value does not become a track-owned source fact.

Feed publisher text, accepted on 2026-09-20: keep the supplied MusicIndex value and identify its source in metadata details.
This acceptance does not invent the value's derivation evidence or a track-owned publisher assertion.
Feed-publisher removal and expiry, accepted separately: honor verified removals and retain the evidence.
After expiry, retain the last selected state with a stale label. Selected absence stays absent.

Track publisher placement, accepted on 2026-09-20: show an available feed publisher in a separate "Feed publisher" section.
This fallback applies when the track has no proven publisher of its own. It does not create a track assertion.
Proven track publisher text, accepted separately: apply the feed publisher's Index selection, removal, and stale-state rules.
Preserve its evidence and keep a selected absence absent after expiry.

Track artist text, accepted on 2026-09-20: prefer fresh RSS author text, then the corresponding MusicIndex value.
Keep contributor names in credits. Do not substitute those names for artist text.

Track-artist refinements, accepted separately: use iTunes author before RSS author.
Apply the accepted removal, conflict, and stale-state rules. Retain source evidence and selected absence.

Track-artist fallback, accepted separately: show an available feed artist in a separate "Feed artist" section.
Use this section when the track has no artist text. It does not create a track artist assertion.

Feed artist text, accepted separately: prefer fresh RSS iTunes author, then MusicIndex.
Keep contributor and track names separate. They do not supply fallback feed artist text.
Feed-artist refinements, accepted separately: apply the track artist's removal, conflict, and stale-state rules.
Retain source evidence and selected absence.

Feed-artist placeholders, accepted separately: hide only confirmed generated placeholders.
Retain literal source assertions, including "Unknown Artist". The text alone cannot prove that a value is a placeholder.
Track-artist placeholders, accepted separately: apply the same rule. Retain literal source assertions and all rejected-value evidence.

Feed language, accepted on 2026-09-20: prefer fresh RSS before MusicIndex.
Apply the accepted removal, conflict, and stale-state rules. Retain source evidence and selected absence.
Proven track language, accepted separately: prefer fresh item RSS before MusicIndex, with those same rules.

Track-language fallback, accepted separately: show the available feed value as a separate "Feed language" value when track language is absent.
Legacy track language, accepted separately: retain values with unknown ownership in source details only.
Do not present those values as track-owned assertions.

Feed explicit state, accepted on 2026-09-20: prefer fresh valid RSS markers and keep unknown distinct from clean.
Missing or unsupported markers cannot establish clean content. Retain raw values and their validation evidence.
Accepted separately: apply the description fields' removal, conflict, and stale-state rules.
Retain source evidence and the last selected field state, including absence. Unknown remains distinct from clean.

Proven track explicit state, accepted separately: prefer fresh valid item RSS markers.
Apply the feed explicit-state removal, conflict, unknown, and stale-state rules. Retain source evidence and selected absence.
Feed fallback, accepted separately: show a known feed state as a separate "Feed explicit state" value when track state is unknown.
This fallback does not create a track-owned assertion.

Legacy feed explicit state, accepted separately: MusicIndex `false` without evidence of a valid clean marker stays unknown.
Retain the original boolean and its source evidence. That scalar alone cannot establish clean content or verified absence.

Legacy track explicit state, accepted separately: retain MusicIndex booleans with unknown ownership in source details only.
Do not present those values as track-owned explicit-state assertions.

Feed publication-date source priority, accepted on 2026-09-20: prefer valid fresh RSS channel `pubDate`.
Then use MusicIndex claims that prove an actual channel publication date.
A MusicIndex `release_date` claim with the path `feed.pub_date` proves a channel publication date, accepted on 2026-09-30.
Stophammer stopped the `lastBuildDate` substitution on 2026-09-23, and its refresh pass of 2026-09-24 read each feed again.
A claim with the path `oldest_item.pub_date` never proves a publication date or a release date.
Oldest-item date presentation, accepted on 2026-09-30: show it as the derived fact "First track published", with MusicIndex as its source.

Feed publication-date refinements, accepted separately: apply the description fields' removal, conflict, and stale-state rules.
Retain original date text and source evidence. Retain the last selected field state after expiry, including selected absence.

Feed publication-date precision, accepted separately: show valid partial dates at their supplied precision.
Do not invent missing date parts or timezones. Retain original text and validation evidence.

Feed publication timestamp display, accepted separately: show UTC and retain the source timezone and original text in metadata details.
Convert only a known instant. Partial dates and unknown timezones remain unconverted.

Track publication-date source priority, accepted separately: prefer valid fresh item RSS `pubDate`.
Then use MusicIndex claims that prove the track's publication date.

Track publication-date refinements, accepted on 2026-09-21: apply the feed date's removal, conflict, and stale-state rules.
Retain original date text and source evidence. Retain the last selected field state after expiry, including selected absence.

Track publication-date precision, accepted separately: show valid partial dates at their supplied precision.
Do not invent missing date parts or timezones. Retain original text and validation evidence, as required for feed publication dates.

Track publication timestamp display, accepted separately: show UTC and retain the source timezone and original text in metadata details.
Convert only a known instant. Partial dates and unknown timezones remain unconverted.

Track publication-date fallback, accepted separately: show the available feed date as a separate "Feed publication date" value.
Use this value when the track publication date is absent. This presentation does not create a track-owned date assertion.

Feed release-date evidence, accepted on 2026-09-21: require direct release-date evidence.
Keep publication, build, and oldest-item dates separate. Those dates cannot supply a missing feed release date.
Retain derived values and their derivation evidence. A `release_date` field name or claim type alone cannot prove release-date meaning.

Feed release-date refinements, accepted separately: apply the publication-date removal, conflict, and stale-state rules.
Retain original date text, source evidence, and the last selected field state, including absence.

Feed release-date source priority, accepted separately: prefer fresh supported RSS assertions, then MusicIndex assertions.
Both sources must prove an actual release date.

Feed release-date precision, accepted separately: preserve year-only and year-month precision.
Do not invent a missing day or time. Keep the original date text and source precision.

Feed release timestamp display, accepted separately: show UTC when the source timestamp has a known timezone.
Metadata details retain the source timezone and original text. Partial calendar dates remain unchanged.

Track release dates, accepted separately: require direct evidence and apply the feed release-date source, removal, conflict, and stale-state rules.
Retain original date text, source evidence, and the last selected field state, including absence.
Keep track publication dates separate.

Track release-date precision, accepted separately: preserve year-only and year-month precision without invented date parts.
Keep the original date text and source precision, as required for feed release dates.

Track release timestamp display, accepted separately: show UTC when the source timestamp has a known timezone.
Metadata details retain the source timezone and original text. Partial calendar dates remain unchanged.

Track release-date fallback, accepted separately: show the available feed release date as a separate "Feed release date" value.
Use this value when the track release date is absent. This presentation does not create a track-owned release-date assertion.

Date format interpretation, accepted separately for all four date fields: require unambiguous formats with known source rules.
Retain other text as unresolved evidence. Do not guess date order, epoch units, missing date parts, or timezones.

Track duration metadata, accepted separately: prefer fresh valid RSS iTunes duration, then MusicIndex.
Keep measured file duration separate.
Apply the accepted removal, conflict, and stale-state rules. Retain original text, source evidence, and selected absence.

Track duration precision, accepted separately: retain valid fractional seconds at source precision without rounding stored values to whole seconds.
Track duration validation, accepted separately: accept explicitly supplied zero. Reject negative or malformed durations while retaining their source evidence.

RSS duration formats, accepted separately: accept seconds, `MM:SS`, and `HH:MM:SS`, with fractional seconds and valid component ranges.
The leading component can exceed 59. Each subsequent minute or second component must remain below 60 and nonnegative.

Measured duration presentation, accepted separately: when duration metadata is absent, show available measured file duration separately as "File duration".
This presentation does not create an RSS or MusicIndex duration assertion.

Track transcript source priority, accepted separately: prefer fresh direct RSS claims over MusicIndex.
Retain all transcript candidates and their source evidence.
Track transcript refinements, accepted separately: apply the description fields' removal, source-order, conflict, and stale-state rules.
Retain the last selected state, including absence, and the underlying evidence.

MusicIndex transcript representation, accepted separately: prefer full transcript claims over legacy transcript links.
Retain both forms of evidence. This rule does not change provider priority.

Legacy transcript recognition, accepted separately: require explicit transcript, caption, or subtitle evidence.
Filename-only matches remain unresolved evidence. A file extension alone cannot establish a transcript action.

Transcript alternatives, accepted separately: offer available language and format alternatives with their declared labels.
Preserve each alternative's source evidence. Distinct language or format alternatives are not conflicting claims merely because their URLs differ.

Legacy transcript ownership, accepted separately: retain unknown ownership in source details only.
Do not present these links as track-owned transcripts or active track transcript actions.

Transcript URL actions, accepted separately: allow only valid HTTP or HTTPS URLs.
Retain other URLs as source evidence without a transcript action.

Feed website actions, accepted separately: allow only valid HTTP or HTTPS URLs.
Retain other schemes as source evidence without a website action.

Track page actions, accepted separately: allow only valid HTTP or HTTPS URLs.
Retain other schemes as source evidence without a page action.

Feed website comparison, accepted separately: normalize scheme, host, and default ports.
Preserve path, query, and fragment differences. Retain the original URL and comparison evidence.

Track page comparison, accepted separately: use the same scheme, host, and default-port normalization as feed websites.
Preserve path, query, and fragment differences. Retain the original URL and comparison evidence.

Feed title source priority, accepted separately: prefer fresh direct RSS titles over MusicIndex.
Retain both source assertions and their original evidence.
Feed title refinements, accepted separately: apply the accepted removal, conflict, and stale-state rules.
Retain original title text, source evidence, and the last selected state, including absence.

MusicIndex feed title representation, accepted separately: use `title`.
Retain the value and its field path.

Amended 2026-09-26: the MusicIndex contract declares no `name` field, and the live responses of 2026-09-25 send none.
The rule no longer names the legacy `name` fallback. Packet 046 deletes the readers. This corrects a fact and changes no decision.

Missing feed title presentation, accepted separately: keep "Unknown Feed" as a display label only.
Do not store that generated label as source metadata.

Feed title placeholders, accepted separately: hide only confirmed generated placeholders from title selection.
Retain literal publisher-supplied titles such as "Unknown Feed". Preserve evidence that identifies a value as generated.

Track title rules, accepted separately: apply the feed title source priority, removal, conflict, stale-state, and placeholder rules.
Prefer fresh direct item RSS titles over MusicIndex. Retain original title text, source evidence, and selected absence.
Keep the track owner. A feed title cannot become a track title.

Missing track title presentation, accepted separately: display the track GUID, then "Untitled" when no GUID exists.
These display labels do not create source metadata.

Feed-title references, accepted separately: allow a track response's `feed_title` when no separately selected feed title exists.
Present it as a labeled feed reference. Retain its feed owner, original value, response field, and source evidence.
Do not turn that reference into a track title or an embedded album assertion.

Feed-title reference removal, accepted separately: verified feed-title removal hides the reference while retaining its evidence.
The reference cannot restore a title removed by verified absence.
Feed-title reference refinements, accepted separately: apply the feed title's conflict, stale-state, and generated-placeholder rules.

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
| Provenance | The element in the RSS document that asserted a value: the channel, the item, or a person. Decision I |
| Provider | The service or RSS resource that delivered the facts. This is transport evidence, not provenance |
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
Decision D rejects compatibility for `purpose="nostr"`.
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

The operator reduced the field scope on 2026-09-21 (Decision A, amended).
Write a separate rule for a covered field before you change its fallback.
Cover description, artwork, publisher, artist text, language, explicit state, dates,
links, transcripts and enclosures. Each rule states the owner, the source order,
the conflict result and the displayed value when no source supplies the field.
The [field inventory](../schema/adr-0075-metadata-field-inventory.md) assigns every inspected metadata field to a rule or packet.
An assigned packet does not establish a completed or accepted field rule.
Document agents propose any rule that an existing decision does not settle.
Mark those rules as proposals until the operator accepts them.
The dependent code packet requires the accepted field rule.
A generic merge helper must not apply one field's rule to another field.
The existing contract still governs payment-route inheritance.

Decisions E and F govern enclosure support and limited source priority.
The [description rules](../schema/adr-0075-field-rules-description-artwork-publisher.md#accepted-description-priority-and-discrepancies)
and [link rules](../schema/adr-0075-field-rules-links-and-media.md#accepted-source-priority)
define their scope and proposed details. Unaccepted details remain held.

### 4a. Keep Discrepancy Evidence Separate From Display Selection

Superseded by ADR 0076 on 2026-09-24. ADR 0076 Decision 4 keeps the original responses as evidence.

Decision G applies to comparable values covered by Decision F.
Match the feed scope, track scope when applicable, field, and provider pair before comparison.
Retain the source resource and extraction path for each value.
Missing source timestamps remain unknown. Fetch time does not replace source observation time.

The schema packet must preserve discrepancy evidence through restart and later snapshot replacement.
The comparison packet must specify readable-text normalization and comparison-version changes before implementation.
It must distinguish a known absent value from incomplete coverage, a failed refresh, or unknown ownership.
Only comparable successful observations can establish or resolve a discrepancy.
Do not resolve a discrepancy because a request failed or its evidence expired from a cache.

Define freshness and expiry before implementing Decision F. A retained RSS value is not fresh merely because it exists.
Do not compare a feed fallback against a track assertion as if both describe the same field owner.
The future hook remains separate work. Current packets preserve evidence and expose typed discrepancy state only.

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
- Display priority cannot erase discrepancy evidence from another provider.
- Description formatting alone cannot create a discrepancy between equivalent readable values.
- Enclosure priority never bypasses the selecting operation's supported-format check.
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
