# ADR 0075 Task 007: Write The Field Rules For Links And Media

Status: Individual field policies accepted - 2026-09-21. Technical implementation and visual gates remain open.
The parent ADR 0075 is Accepted from 2026-09-19.
The operator released the dispatch of this packet on 2026-09-19.

The operator accepted website and page selection refinements through individual questions on 2026-09-20.
Supported enclosure selection is accepted.
Track transcripts prefer fresh direct RSS claims over MusicIndex, with all candidates and source evidence retained.
Transcript refinements use the description fields' removal, source-order, conflict, and stale-state rules, including retained selected absence.

Within MusicIndex, prefer full transcript claims over legacy transcript links. Retain both forms of evidence.
Legacy transcript recognition requires explicit transcript, caption, or subtitle evidence. Filename-only matches remain unresolved evidence.

Transcript language and format alternatives remain available with their declared labels and source evidence.
Legacy transcripts with unknown ownership remain in source details only, without active track transcript actions.
Transcript actions allow only valid HTTP or HTTPS URLs. Retain other URLs as source evidence without a transcript action.

Feed website actions allow only valid HTTP or HTTPS URLs. Other schemes remain source evidence without a website action.
Track page actions also allow only valid HTTP or HTTPS URLs, with other schemes retained as source evidence.
Feed website comparison normalizes scheme, host, and default ports while preserving path, query, and fragment differences.
Track page comparison applies the same normalization while preserving path, query, and fragment differences.

The [field inventory](../schema/adr-0075-metadata-field-inventory.md) assigns remaining coverage.
The operator's orchestration authorization permits bounded code under accepted rules. Visual checks remain paused.

This packet produces a document. It changes no code.
The deliverable is [adr-0075-field-rules-links-and-media.md](../schema/adr-0075-field-rules-links-and-media.md).

The correction has no structural finding in changed prose. Its local link check is Green.
Lexical findings remain. The raw STE checker result is not Green.
The [technical review record](../reviews/adr-0075-metadata-contract-review.md#packet-004-to-007-technical-reviews--2026-09-19)
holds the earlier result. Later individual policy decisions are recorded in ADR 0075 and the deliverable.

## Goal

Write the document that states the field rule for feed `website`, track
`web_page`, transcript links, enclosure links, image links and unknown
link kinds.

Ground each rule in ADR 0075
[decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules)
and
[decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts).
State the owner, the source order, the conflict result and the displayed
value when no source supplies the field.

This packet depends on packet 003, the identity syntax and purpose
contract, per the phase plan's
[Packet Register](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register).
Confirm packet 003's deliverable path before you cite it. Packet 003 did
not exist when this packet was written.

## Files To Inspect

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), decision
  3 and decision 4

- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), the
  [Packet Register](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register)

- [Review](../reviews/adr-0075-metadata-contract-review.md), the findings
  [Website Rules Differ](../reviews/adr-0075-metadata-contract-review.md#website-rules-differ)
  and
  [Source Order Can Select The Displayed Identity](../reviews/adr-0075-metadata-contract-review.md#source-order-can-select-the-displayed-identity)

- [Task 001](adr-0075-task-001-contributor-claim-transport.md), for this
  packet's tone and level of detail

- `src/api.rs`: `Feed.source_links`, `Track.source_links`,
  `Track.source_enclosures`, `Track.enclosure_url`, `Track.enclosure_type`,
  `Track.enclosure_bytes`, `SourceEntityLink` and `SourceEnclosure`

- `src/views.rs`: `website_url_from_links`,
  `EntityIdentityLinks::from_source_facts` and the transcript URL closure
  inside `TrackView::from_api`

- `src/metadata.rs`: `website_from_links`, `transcript_from_links`,
  `track_website`, `feed_website` and `track_transcript_url`

- `src/db.rs`: `local_identity_links` and `transcript_url_from_extra_json`

- `src/rss/subscribe.rs`: `rss_feed_link_inputs`, `rss_track_link_inputs`,
  `subscribe_feed` and `track_extra_json`. Note where `subscribe_feed`
  stores the raw item `link` value

- `src/rss/enrich.rs`, for context only. Packet 009 owns its correction

- `src/track_compare.rs::select_audio_enclosure`, for the enclosure
  selection rule

- `src/view_models/track.rs`, lines near 128 to 134, for a second
  consumer of `source_enclosures`

Inspect the upstream Stophammer checkout read-only, at
`/home/citizen/build/stophammer`, commit `a220f44`:

- `stophammer-parser/src/engine.rs::extract_links`. Read the match arms
  for `entity_type` values `feed`, `track` and `live_item`, and the Atom
  alternate-link arms

## Deliverable

One document: `docs/schema/adr-0075-field-rules-links-and-media.md`.

## Do Not Touch

- Every Rust source file in this repository. This packet writes one
  document.

- The Stophammer checkout. Read it. Do not write in it.

- The renderer, the view model and the shared projection. Packet 020 owns
  the merge. Packets 022 and 023 own the labeled sections.

- The database schema, a migration, or the `entity_metadata_facts` and
  `entity_identity_links` table design. Packet 011 owns the schema.

- The collection completeness rules. Packet 004 owns that document.

- The description, artwork, publisher, artist text, language, explicit
  state and date field rules. Packets 005 and 006 own those fields.

- The supported Nostr syntax and the supported `podcast:txt` purpose
  values. Packet 003 owns that rule.

- The RSS owner correction in `src/rss/enrich.rs`. Packet 009 owns that
  code change. State the target rule only.

- The storage of an item link as a track `web_page` identity fact. Packet
  010 owns that code change. State the target rule only.

## Constraints

Write every sentence in this deliverable in ASD-STE100 Simplified
Technical English.

Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
Write compliant prose directly. Do not make a draft that requires conversion to STE.

Use the shared checker in Checks. Correct only confirmed errors in the affected prose.
A Green result covers the configured checks. It does not prove full dictionary compliance.


State each rule with its evidence. Name a file path, a function name and
the observed behavior.

Separate current source behavior from the proposed field policy.
When an existing decision does not settle the policy, propose a concrete rule with its reason.
Mark that rule as pending operator review. The dependent code packet requires operator acceptance.
Record an unverified source claim as an evidence gap, not as a proposed fact.

Use the ADR 0075 terms exactly: fact, subject, claim, provenance,
provider, collection, snapshot, coverage state and projection. Do not
change a term's meaning from its ADR 0075 definition.

Apply these shared rules from ADR 0075 decision 4 to every field in this
packet:

- A display value never becomes a stored source assertion.

- A generic merge helper must not apply one field's rule to another
  field.

- The rule must keep the audio tag export separate from the value
  prepared for display.

- ADR 0053 keeps a track description distinct from a feed description.
  This packet does not carry a description field, but the same separation
  of owners applies to every field below.

- The current contract still governs payment-route inheritance. These
  packets do not change it.

## Required Content

The deliverable must contain the items below.

1. An opening paragraph that names ADR 0075 decision 3 and decision 4.
   State that this packet covers feed `website`, track `web_page`,
   transcript links, enclosure links, image links and unknown link kinds.

2. Six completed Field Rule Template rows: Feed website, Track web_page,
   Transcript links, Enclosure links, Image links and Unknown link kinds.

3. For Feed website, name the write side and the read side.

   - `src/rss/subscribe.rs::rss_feed_link_inputs` writes the RSS channel
     `link` as link type `website`, extraction path `channel/link`,
     source `rss`.

   - Cite the upstream rule: `extract_links` in
     `stophammer-parser/src/engine.rs` maps a channel-level RSS `link` to
     link type `website`. This is the source of a MusicIndex
     `Feed.source_links` `website` entry.

   - `src/metadata.rs::website_from_links` and
     `src/views.rs::website_url_from_links` both read link type
     `website` for a feed. State that this matches ADR 0075 decision 3's
     owner split for the feed side.

4. For Track web_page, name the two current risks in the review, in this
   packet's own words.

   - `src/rss/subscribe.rs::subscribe_feed` stores the item RSS `link`
     value in the plain `tracks.link` database column, inside the track
     upsert statement. It does not build an identity link fact for that
     value.

   - Correct one detail against the review's text. The helper named
     `rss_track_link_inputs` only builds the transcript link input. It
     does not process `item.link`.

   - State that `tracks.link` is also not a field on `db::TrackRow`. No
     current read path returns this stored value.

   - State the second review risk. `src/views.rs::website_url_from_links`
     matches only link type `website`. It never matches link type
     `web_page`.

   - State the effect: a track's own `web_page` link can never surface
     through this selector, on the local route.

   - Contrast `src/metadata.rs::website_from_links`, used by
     `track_website` on the API route. It matches link type `website` or
     `web_page`. State that the two routes use different link-type rules
     for the same field.

   - Cite the upstream rule: `extract_links` maps an item or live-item RSS
     `link` to `web_page` with extraction path `entity.link`.
     Atom alternate links use `entity.atom:link[@rel='alternate']`.
     Preserve these separate extraction paths.

   - State the target rule for packet 010: store the item RSS `link` as
     a track `web_page` identity fact, with the same extraction path
     Stophammer uses, not only as a plain database column.

5. For transcript links, distinguish the two legacy storage sites from their read selectors.
   Also inspect upstream `SourceItemTranscriptResponse` and the `source_transcripts` include branch.
   The app drops that existing typed collection. Packet 032 owns its transport correction.

   - `src/rss/subscribe.rs::rss_track_link_inputs` writes an identity
     link fact with link type `transcript`, extraction path
     `podcast:transcript@url`.

   - `src/rss/subscribe.rs::track_extra_json` also writes the same
     transcript URL into the `tracks.extra_json` column.
     `src/db.rs::transcript_url_from_extra_json` reads it back into
     `TrackRow.transcript_url`.

   - `src/metadata.rs::transcript_from_links` reads MusicIndex
     `source_links` for the API route. It matches link type text that
     contains `transcript`, `caption` or `subtitle`, an extraction path
     that contains `transcript`, or a known subtitle file extension.

   - State a fourth site. The transcript closure inside
     `src/views.rs::TrackView::from_api` matches only an exact link type
     of `transcript`. It does not apply the wider match rule that
     `transcript_from_links` applies.

   - Record this as an inconsistency between two selectors for the same
     field across `src/metadata.rs` and `src/views.rs`.

6. For Enclosure links, name the two representations in the API data
   object and the current selection rule.

   - `api::Track` carries a scalar `enclosure_url`, `enclosure_type` and
     `enclosure_bytes`, and a separate typed list, `source_enclosures`.

   - `src/track_compare.rs::select_audio_enclosure` selects the first supported primary enclosure.
     It then selects the first supported enclosure, followed by a supported direct scalar enclosure.
     Cite `selected_source_enclosure` and `classify_enclosure` for the supported-format filter.

   - `src/views.rs::TrackView` reads only the scalar `enclosure_url`,
     `enclosure_type` and `enclosure_bytes` fields, for `audio_url`,
     `mime` and `bytes`. It does not read `source_enclosures`.

   - Record that the display path and the download-selection path use
     different rules for the same conceptual field.

7. For Image links, state a current limit rather than an invented rule.

   - State that no link type `image` exists today. Confirm this against
     `stophammer-parser/src/engine.rs::extract_links` and against every
     app selector in `src/metadata.rs` and `src/views.rs`.

   - State that `Feed.image_url`, `Track.image_url` and `Contributor.img`
     are plain scalar fields. No source-links row records an image link
     with its own extraction path or source.

   - Record this as an open question for packet 003 or packet 008: does
     the field rule need a typed image link kind, or does the scalar
     field stay the rule.

8. For Unknown link kinds, name two link types the app already stores but
   does not select for display: `donate`, seen in
   `src/views.rs`'s own test fixtures, and `self_feed`, seen in the
   upstream Atom `rel="self"` arm of `extract_links`.

   - State that `entity_identity_links` and `source_links` preserve
     these rows. No current code path filters them out at write time.

   - State that no current selector function reads them. `website_from_links`,
     `website_url_from_links` and `transcript_from_links` each match a
     fixed, short list of link types.

   - Cite ADR 0075 decision 3: preserve the evidence, and record whether
     the app supports the syntax. State that no current field records a
     supported or unsupported flag for a link type. Record this as an
     open question for packet 011.

9. A statement of this packet's dependency on packet 003. Cite the phase
   plan's Packet Register row for packet 007. State what packet 003 must
   supply before packet 007's deliverable can rely on it: the supported
   `podcast:txt` purpose values, so a link-kind rule does not conflict
   with an identity-syntax rule for the same source row.

10. Named evidence, or a named open question, for every rule above.

## Field Rule Template

Use this table for every field rule in the deliverable. Add one row for
each field this packet lists. Keep the columns in this order.

| Field | Declared owner | Sources, in priority order | Conflict result | No-source result | Feed value on a track | Evidence retained | Current code | Required change |
|---|---|---|---|---|---|---|---|---|

Column meanings:

- Field: the field name from this packet's field list.

- Declared owner: the subject that owns the value: feed, track or
  contributor.

- Sources, in priority order: each source that can supply the value,
  ranked highest first.

- Conflict result: the value the app keeps when two sources disagree.

- No-source result: the value the app shows when no source supplies the
  field.

- Feed value on a track: state whether a feed value may appear for a
  track, and its exact owner label.

- Evidence retained: the raw evidence the app keeps beside the selected
  value.

- Current code: the file and function that produce today's value. List a
  separate line inside the cell for each route that computes the value in
  a different way.

  Write "None found" when no current code produces the field.

- Required change: the change a later packet must make. Write "None" when
  the current rule already matches this row.

## Accepted Corrections

Decision E preserves the supported-format filter at each selection step.
Use the same priority order with the selecting operation's capability check.
Different operation capabilities can produce different selected files.
Require a case with an unsupported primary enclosure and a supported alternate.

Decision F prefers fresh direct RSS websites and track pages over matching Index values.
Decision G retains discrepancies and their source evidence. Packet 035 defines URL comparison.

Packet 030 preserves the four enclosure claim fields already present upstream.
The artwork request requires separate owner facts while retaining the old scalar API field.

## Acceptance Criteria

### Mechanical Criteria

- The file `docs/schema/adr-0075-field-rules-links-and-media.md` exists.

- The document contains one complete Field Rule Template row for each of
  the six listed fields. No cell is empty.

- The document states both named review risks for Track web_page. It
  cites `src/views.rs::website_url_from_links` and
  `src/db.rs::local_identity_links`.

- The document corrects the exact attribution of the `item.link` storage
  site. It names `subscribe_feed`'s track upsert, not
  `rss_track_link_inputs`, as the site that stores the raw value.

- The document cites `stophammer-parser/src/engine.rs::extract_links` for
  the upstream `website` and `web_page` rule.

- The document states the Image links limit. It states that no current
  code produces a typed image link.

- The document states this packet's dependency on packet 003.

- The document lists every open question it hands to a later packet.

- Every local link in the document resolves.

### Review Criteria

- Binding rules match ADR 0075 decisions 3 and 4. Unresolved field policies appear as proposals for operator review.

- The corrected attribution for `item.link` storage does not weaken the
  review's finding. It states the same risk with a more exact citation.

- After operator acceptance, the rules let packets 009, 010 and 020 implement the fields without choosing product policy.
  A pending proposal keeps its dependent code packet held.

- An unrun review check reports this gate as open. It never reports the
  gate as met.

## Checks

Run these commands from the repository root after writing the deliverable.
The link check tests local files and headings. It returns failure for a missing target.
It does not change the current directory or request external pages.
Use Markdown links for local references. Use fenced code blocks for recorded syntax.

```bash
python3 docs/runbooks/check-markdown-links.py \
  docs/tasks/adr-0075-task-007-field-rules-links-and-media.md \
  docs/schema/adr-0075-field-rules-links-and-media.md
```

```bash
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" \
  --check --no-heuristics docs/schema/adr-0075-field-rules-links-and-media.md
```

Examine the STE findings. Correct confirmed defects in the affected prose.
Record retained technical names and the reasons for retaining them.
Report a checker error as a failed check.
Do not report the raw checker result as Green if findings remain.
No application test applies to this document packet.

## Escalation Triggers

Stop if the work needs any of these changes:

- A code change in this repository or in the Stophammer checkout.

- A change to an accepted decision without a recorded amendment.

- The supported Nostr syntax or `podcast:txt` purpose list. Packet 003
  owns that rule.

- A field rule that belongs to packet 005 or packet 006.

Revise this packet before you continue.

## Expected Report

Name the deliverable's path and its section list. State which rules carry
named source evidence and which are open questions.
List proposed field policies separately. Record their operator review status.

Report the mechanical checks as Green, or name the failure and its cause.
State separately whether the review criteria are met, open or not run.

## Prompt for lower-context coding model

You write one bounded document from a larger plan. Do not write code.

Read:

- This packet and every file in its Files To Inspect section.

Goal:

- Write `docs/schema/adr-0075-field-rules-links-and-media.md` with the
  content in this packet's Required Content section.

Constraints:

- Write in ASD-STE100 Simplified Technical English.

- Cite named source evidence for each rule. Record a gap as an open
  question for a later packet when you find no evidence.

- Do not touch any boundary in this packet's Do Not Touch section.

Acceptance criteria:

- Pass every mechanical criterion listed above.

- Leave every review criterion for a person. Report an unrun review check
  as open, not as met.

Checks:

- Run the commands in this packet's Checks section.

At the end, report:

1. the deliverable's path and section list
2. which rules carry named evidence, and which are open questions
3. mechanical check results
4. deviations from this packet
5. unresolved concerns
