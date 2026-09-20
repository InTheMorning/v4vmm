# ADR 0075 Task 005: Write The Field Rules For Description, Artwork And Publisher

Status: Deliverable corrected - 2026-09-19. Remaining review gates are open.
The parent ADR 0075 is Accepted from 2026-09-19.
The operator released the dispatch of this packet on 2026-09-19.

Correction scope: the operator accepted ADR 0075 Decisions E, F, and G.
Only those decisions and earlier accepted rules are approved. Other policy proposals remain open.
The [field inventory](../schema/adr-0075-metadata-field-inventory.md) assigns remaining coverage.
This correction releases no code packet and closes no visual gate.

This packet produces a document. It changes no code.
The deliverable is [adr-0075-field-rules-description-artwork-publisher.md](../schema/adr-0075-field-rules-description-artwork-publisher.md).
The correction has no structural finding in changed prose. Its local link check is Green.
Lexical findings remain. The raw STE checker result is not Green.
The [technical review record](../reviews/adr-0075-metadata-contract-review.md#packet-004-to-007-technical-reviews--2026-09-19)
holds the result. A person has not walked the review gate.

## Goal

Write the document that states the field rule for feed description, track
description, feed artwork, track artwork and publisher.

Ground each rule in ADR 0075
[decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts).
State the owner, the source order, the conflict result and the displayed
value when no source supplies the field.

This packet does not change display code, storage code or a merge helper.
It defines the rule that packet 020 must apply.

## Files To Inspect

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), decision 3
  and decision 4

- [ADR 0053](../adr/0053-local-detail-source-fact-parity.md), the
  [Decision](../adr/0053-local-detail-source-fact-parity.md#decision) and
  [Invariants](../adr/0053-local-detail-source-fact-parity.md#invariants)
  sections

- [ADR 0054](../adr/0054-local-metadata-source-fact-persistence.md), the
  [Decision](../adr/0054-local-metadata-source-fact-persistence.md#decision)
  section

- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), the
  [Packet Register](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register)

- [Review](../reviews/adr-0075-metadata-contract-review.md), the findings
  [Feed Defaults Can Appear As Track Facts](../reviews/adr-0075-metadata-contract-review.md#feed-defaults-can-appear-as-track-facts)
  and
  [Source Order Can Select The Displayed Identity](../reviews/adr-0075-metadata-contract-review.md#source-order-can-select-the-displayed-identity)

- [Task 001](adr-0075-task-001-contributor-claim-transport.md), for this
  packet's tone and level of detail

- `src/api.rs`: the `Feed` and `Track` structs, and `track_with_feed_defaults`

- `src/views.rs`: `FeedView::from_api`, `FeedView::from_local_with_facts`,
  `TrackView::from_api`, `TrackView::from_local_with_facts`,
  `description_from_release_claims` and `artwork_from_url`

- `src/metadata.rs`: `artwork_url`, `track_artwork_url` and
  `source_value_for_metadata_field`

- `src/local_metadata.rs`: `feed_facts_from_rows` and `track_facts_from_rows`

- `src/identity_ingest.rs`: `feed_metadata_facts_by_source`,
  `track_metadata_facts`, `persist_feed_metadata_facts` and
  `persist_track_metadata_facts`

- `src/rss/subscribe.rs`: `subscribe_feed`, the feed row upsert, and the
  MusicIndex baseline block that calls `db::set_feed_description`

- `src/db.rs`: `set_feed_description`, `local_metadata_facts` and `FeedRow`

- `src/application/queries/library.rs`: `build_tree`,
  `fetch_library_track_context_with_local_fallback`,
  `apply_local_track_metadata_defaults` and `hydrate_album_identity_facts`

- `src/feed_service.rs`: `merge_track_context_from_detail`, `track_defaults`
  and `track_row_to_track_context`

- `src/subscribe_service.rs`: the call order around
  `identity_ingest::persist_musicindex_context_by_feed_url` and
  `track_with_feed_defaults` in the feed-download loop

## Deliverable

One document:
`docs/schema/adr-0075-field-rules-description-artwork-publisher.md`.

## Do Not Touch

- Every Rust source file in this repository. This packet writes one
  document.

- The renderer, the view model and the shared projection. Packet 020 owns
  the merge. Packets 022 and 023 own the labeled sections.

- The database schema, a migration, or the `entity_metadata_facts` table
  design. Packet 011 owns the schema.

- The collection completeness rules. Packet 004 owns that document.

- The link and media field rules. Packet 007 owns links and media.

- The artist text, language, explicit state and date field rules. Packet
  006 owns those fields.

- The behavior of the audio tag comparison in
  `src/metadata.rs::source_value_for_metadata_field`. State its current
  behavior. Do not change it.

## Constraints

Write every sentence in this deliverable in ASD-STE100 Simplified Technical
English.

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

Apply these shared rules from ADR 0075 decision 4 to every field in this
packet:

- A display value never becomes a stored source assertion.

- A generic merge helper must not apply one field's rule to another field.

- The rule must keep the audio tag export separate from the value prepared
  for display.

- ADR 0053 keeps a track description distinct from a feed description. Do
  not reverse that rule.

- The current contract still governs payment-route inheritance. These
  packets do not change it.

## Required Content

The deliverable must contain the items below.

1. An opening paragraph that names ADR 0075 decision 4. State that a
   missing track field does not make a feed description, a feed image or a
   feed publisher a track assertion.

2. Five completed Field Rule Template rows: Feed description, Track
   description, Feed artwork, Track artwork and Publisher.

3. For Feed description, name the two sites that write the legacy
   `feeds.description` column.

   - `src/rss/subscribe.rs::subscribe_feed` first writes the RSS channel
     `<description>` into the `feeds.description` column.

   - The same function later overwrites that column. It uses the
     MusicIndex `Feed.description` value, through
     `src/db.rs::set_feed_description`, after a baseline MusicIndex fetch
     succeeds.

   - `set_feed_description` carries no source tag. Record that this column
     currently mixes two sources under one untagged name.

4. For Feed description, also cite
   `src/identity_ingest.rs::feed_metadata_facts_by_source`. State that it
   writes a typed `entity_metadata_facts` row for fact key `description`
   for each source token drawn from `source_release_claims`, plus one row
   from the top-level `Feed.description` field under source `musicindex`.

5. Cite the read side for Feed description.

   - `src/local_metadata.rs::feed_facts_from_rows` keeps the first
     non-top-level description row it meets. It falls back to the
     top-level MusicIndex row only when it finds no other row.

   - Row order comes from `src/db.rs::local_metadata_facts`, which orders
     rows by `source COLLATE NOCASE`. Name this as the same alphabetical
     source-label order that the review names for
     [`local_identity_links`](../reviews/adr-0075-metadata-contract-review.md#source-order-can-select-the-displayed-identity).

   - State that this order, not a stated priority, currently selects the
     description when two sources both supply one.

6. For Track description, cite `src/api.rs::track_with_feed_defaults`. It
   copies `feed.description` into `track.description` when the track's own
   field is `None`.

   - State the review's named risk in this packet's own words:
     `track_with_feed_defaults` can place a feed description inside a
     track's API data object.

   - Trace the path of the `Track` value that this fallback produces.
     `src/feed_service.rs::merge_track_context_from_detail` and
     `track_row_to_track_context` use it to build a display-only
     `TrackContext`.

   - For every call site you can find, state whether that path reaches
     `src/identity_ingest.rs::persist_track_metadata_facts` before or
     after `track_with_feed_defaults` runs. Record any call site you do
     not resolve as an open question.

7. For Feed artwork and Track artwork, name the three sites of the
   feed-to-track fallback.

   - `src/metadata.rs::artwork_url` falls back from `track.image_url` to
     `feed.image_url`.

   - `src/api.rs::track_with_feed_defaults` computes the same fallback
     again, on its own, inside the API data object.

   - `src/views.rs::TrackView::from_local_with_facts` reads
     `t.track_image_href.or(t.album_image_href)` from the database row. It
     is a third site in the Rust projection, after separate database columns are read.

   - Inspect upstream `get_track_rows_by_guid` and `get_track_row_for_feed`.
     They apply `COALESCE(t.image_url, f.image_url)` before the API response.
     Request separate artwork facts with declared owners and source evidence in packet 008.

8. For Publisher, name the three sites of the feed-to-track fallback.

   - `src/api.rs::track_with_feed_defaults` copies `feed.publisher_text`
     into `track.publisher_text`.

   - The `"Publisher"` and `"Label"` match arm in
     `src/metadata.rs::source_value_for_metadata_field` falls back from
     `track.publisher_text` to `feed.publisher_text`, on its own, for the
     audio tag comparison grid.

   - `src/views.rs::TrackView::from_local_with_facts` reads
     `publisher_text` only from `entity_metadata_facts`. It uses no legacy
     column and no feed fallback at that layer.

9. Apply ADR 0075 Decision C to artwork. Show the track's artwork when it has its own image.
   Otherwise, use feed artwork in the track header without an additional visible owner label.
   Preserve the source and owner in stored facts. Do not store the fallback as a track assertion.

   Propose the placement and owner labels for inherited descriptions and publisher text as separate field rules.
   Decision B governs identity placement. It does not settle those other fields.

10. State the generic-merge-helper risk. `track_with_feed_defaults` applies
    one pattern, `if track.field.is_none() { track.field =
    feed.field.clone() }`, to `image_url`, `publisher_text`, `description`,
    `release_artist` and other fields inside one function.

    State that this packet's rule must stop a later packet from the reuse
    of that one function for every field, as if one fallback rule fit all
    fields.

11. Named evidence, or a named open question, for every rule above.

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

- Feed value on a track: state whether a feed value may appear for a track.
  State any visible owner label. For artwork fallback, write "No additional visible label, per Decision C".

- Evidence retained: the raw evidence the app keeps beside the selected
  value.

- Current code: the file and function that produce today's value. List a
  separate line inside the cell for each route that computes the value in
  a different way.

- Required change: the change a later packet must make. Write "None" when
  the current rule already matches this row.

## Accepted Corrections

Apply Decision F to feed and track descriptions for matching owners.
Prefer fresh direct RSS over the corresponding Index value. Retain both observations.
Apply Decision G to readable-text differences and retained discrepancy evidence.
Formatting alone must not create a discrepancy. Failed refresh must not resolve one.

Propose within-provider order and stale-value handling separately. Keep those proposals open until accepted.
Existing audio-tag comparison and write policies remain separate.

## Acceptance Criteria

### Mechanical Criteria

- The file
  `docs/schema/adr-0075-field-rules-description-artwork-publisher.md`
  exists.

- The document contains one complete Field Rule Template row for each of
  the five listed fields: Feed description, Track description, Feed
  artwork, Track artwork and Publisher. No cell is empty.

- The document states the `track_with_feed_defaults` risk. It cites
  `src/api.rs::track_with_feed_defaults` by name.

- The document states the alphabetical source-order risk for Feed
  description. It cites `src/db.rs::local_metadata_facts`.

- The document names, for each of the three fallback sites found for
  artwork and the three found for publisher, the exact file and function.

- The document lists every open question it hands to a later packet.

- Every local link in the document resolves.

- The artwork rule states track artwork first, then feed artwork, with no additional visible owner label for the fallback.
  It retains ownership in stored facts.

### Review Criteria

- Binding rules match ADR 0075 decisions 3 and 4. Unresolved field policies appear as proposals for operator review.

- Identity labels follow Decision B. Artwork fallback follows Decision C without an additional visible owner label.

- After operator acceptance, the rules let the packet 020 writer implement the fields without choosing product policy.
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
  docs/tasks/adr-0075-task-005-field-rules-description-artwork-publisher.md \
  docs/schema/adr-0075-field-rules-description-artwork-publisher.md
```

```bash
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" \
  --check --no-heuristics docs/schema/adr-0075-field-rules-description-artwork-publisher.md
```

Examine the STE findings. Correct confirmed defects in the affected prose.
Record retained technical names and the reasons for retaining them.
Report a checker error as a failed check.
Do not report the raw checker result as Green if findings remain.
No application test applies to this document packet.

## Escalation Triggers

Stop if the work needs any of these changes:

- A code change in this repository.

- A change to an accepted decision without a recorded amendment.

- A field rule that belongs to packet 006 or packet 007.

- A schema, migration or projection change that belongs to packet 011 or
  packet 020.

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

- Write `docs/schema/adr-0075-field-rules-description-artwork-publisher.md`
  with the content in this packet's Required Content section.

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
