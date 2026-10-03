# ADR 0075 Task 006: Write The Field Rules For Artist Text, Language, Explicit State And Dates

Status: Individual field policies accepted - 2026-09-21. Technical implementation and visual gates remain open.
The parent ADR 0075 is Accepted from 2026-09-19.
The operator released the dispatch of this packet on 2026-09-19.

The operator accepted the artist, language, and explicit-state policies through individual questions on 2026-09-20.
Feed publication-date source priority, removal, conflict, and stale-state rules are accepted.
Feed publication dates can show valid partial dates without invented parts or timezones.
Track publication-date source priority, removal, conflict, and stale-state rules are accepted separately.
Track publication dates also preserve valid partial dates without invented parts or timezones.
An absent track publication date permits a separate "Feed publication date" value.

Feed and track publication timestamp display have separate acceptance.
Show UTC and retain the source timezone and original text in metadata details.
Partial dates and unknown timezones remain unconverted.

Feed release dates require direct release-date evidence. Publication, build, and oldest-item dates remain separate.
Proven feed release dates use the publication-date removal, conflict, and stale-state rules, with original text and evidence retained.
Proven feed release dates prefer fresh supported RSS assertions, then MusicIndex assertions.
Feed release dates preserve year-only and year-month precision without invented date parts.
Feed release timestamps with a known timezone show UTC, with original source details retained.

Track release dates require direct evidence and use the feed release-date source, removal, conflict, and stale-state rules.
Track release dates also preserve year-only and year-month precision without invented date parts.
Track release timestamps with a known timezone show UTC, with original source details retained.
An absent track release date permits a separate "Feed release date" value.
All four date fields require unambiguous formats with known source rules. Other text remains unresolved evidence.

Duration metadata prefers fresh valid RSS iTunes duration, then MusicIndex. Measured file duration stays separate.
Duration uses the accepted removal, conflict, and stale-state rules, with original text and source evidence retained.
Duration retains valid fractional seconds at source precision and accepts explicitly supplied zero.
Negative and malformed durations remain rejected source evidence.

RSS duration accepts seconds, `MM:SS`, and `HH:MM:SS`, with fractional seconds and valid component ranges.
When duration metadata is absent, show available measured file duration separately as "File duration".

Source-format adapters require technical review before implementation.
The [field inventory](../schema/adr-0075-metadata-field-inventory.md) assigns remaining coverage.

The operator's orchestration authorization permits bounded code under accepted rules. Visual checks remain paused.

This packet produces a document. It changes no code.
The deliverable is [adr-0075-field-rules-artist-language-dates.md](../schema/adr-0075-field-rules-artist-language-dates.md).

The correction has no structural finding in changed prose. Its local link check is Green.
Lexical findings remain. The raw STE checker result is not Green.
The [technical review record](../reviews/adr-0075-metadata-contract-review.md#packet-004-to-007-technical-reviews--2026-09-19)
holds the earlier result. Later individual policy decisions are recorded in ADR 0075 and the deliverable.

## Goal

Write the document that states the field rule for artist text, album
artist text, language, explicit state, release date, publication date and
duration.

Ground each rule in ADR 0075
[decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts).
State the owner, the source order, the conflict result and the displayed
value when no source supplies the field.

This packet covers free-text and scalar display fields only. It does not
merge people. It does not claim an artist identity for a track or a feed.

## Files To Inspect

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), decision
  3 and decision 4

- [ADR 0045](../adr/archive/0045-track-artist-binding.md), the
  [Decision](../adr/archive/0045-track-artist-binding.md#decision),
  [Invariants](../adr/archive/0045-track-artist-binding.md#invariants) and
  [Non-Goals](../adr/archive/0045-track-artist-binding.md#non-goals) sections

- [ADR 0054](../adr/0054-local-metadata-source-fact-persistence.md), the
  [Decision](../adr/0054-local-metadata-source-fact-persistence.md#decision)
  section

- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), the
  [Packet Register](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register)

- [Task 001](archive/adr-0075-task-001-contributor-claim-transport.md), for this
  packet's tone and level of detail

- `src/api.rs`: the `Feed` and `Track` structs, and `track_with_feed_defaults`

- `src/views.rs`: `FeedView::from_api`, `FeedView::from_local_with_facts`,
  `TrackView::from_api` and `TrackView::from_local_with_facts`

- `src/metadata.rs`: `source_value_for_metadata_field`, the `"Artist"`,
  `"Album artist"`, `"Explicit"`, `"Duration"`, `"Release date"` and
  `"Release year"` match arms. Also `feed_release_pubdate`,
  `track_release_pubdate` and `musicindex_release_date`

- `src/local_metadata.rs`: `feed_facts_from_rows` and `track_facts_from_rows`

- `src/identity_ingest.rs`: `feed_metadata_facts_by_source`,
  `track_metadata_facts` and `persist_track_artist_bindings`

- `src/rss/subscribe.rs`: `subscribe_feed`, the `artist_name` and
  `album_artist_name` derivation, and the raw `pub_date`, `itunes_explicit`
  and `duration_seconds` fields it stores

- `src/db.rs`: `parse_itunes_explicit`, `parse_local_track_pub_date`,
  `FeedRow`, `TrackRow` and `local_metadata_facts`

- `src/application/queries/library.rs`: `apply_local_track_metadata_defaults`

## Deliverable

One document: `docs/schema/adr-0075-field-rules-artist-language-dates.md`.

## Do Not Touch

- Every Rust source file in this repository. This packet writes one
  document.

- The renderer, the view model and the shared projection. Packet 020 owns
  the merge. Packets 022 and 023 own the labeled sections.

- The database schema, a migration, or the `entity_metadata_facts` table
  design. Packet 011 owns the schema.

- The collection completeness rules. Packet 004 owns that document.

- The description, artwork and publisher field rules. Packet 005 owns
  those fields.

- The link and media field rules. Packet 007 owns links and media.

- The ADR 0045 `track_artist_source_bindings` table. State this packet's
  boundary with that table. Do not extend it or change it.

- A canonical person or artist merge design. ADR 0045 and ADR 0075 name
  this as a non-goal.

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

This packet does not merge people. It does not claim an artist identity
for a track or a feed.

Keep the free-text artist fields in this packet distinct from the ADR
0045 `track_artist_source_bindings` table. The binding names an explicit
MusicIndex artist id. The free-text fields in this packet do not.

Cite ADR 0054's stored-fact contract for every field that routes through
`entity_metadata_facts`. Name the fact key for that field.

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

1. An opening paragraph that names ADR 0075 decision 4. State that this
   packet covers artist text, album artist text, language, explicit
   state, release date, publication date and duration.

2. Seven completed Field Rule Template rows: Artist text, Album artist
   text, Language, Explicit state, Release date, Publication date and
   Duration.

3. For Artist text, name the current chain that produces `track.artist_name`.

   - `src/rss/subscribe.rs::subscribe_feed` derives it from the iTunes
     author tag, then the RSS author tag, then a `podcast:person` credit
     with role `artist`, `creator`, `composer` or `performer`, then the
     feed's own artist. It stores the result with no source tag.

   - `src/views.rs::TrackView::from_api` reads `track_artist`, then falls
     back to `release_artist`. `TrackView::from_local_with_facts` reads
     `artist_name`, then falls back to `album_artist_name`.

   - `src/metadata.rs::source_value_for_metadata_field`, the `"Artist"`
     arm, reads only `track.track_artist`. It has no feed fallback.

4. State the ADR 0045 boundary for Artist text. Name
   `src/identity_ingest.rs::persist_track_artist_bindings`, which writes
   an explicit, source-scoped `track_artist_source_bindings` row only
   when `artist_credit.artist_id` is present.

   - State that this binding is a separate structure from the free-text
     `track_artist` and `artist_name` fields this packet rules on.

   - State that this packet's rule must not create, imply or extend a
     binding. It rules on display text only.

5. For Album artist text, name the current sites that produce a value.

   - `api::Feed.release_artist` is the feed's own declared artist.
     The inspected upstream track response selects `f.release_artist`.
     It carries the feed value, not an independent track claim.

   - `src/metadata.rs::source_value_for_metadata_field`, the `"Album
     artist"` arm, falls back from `track.release_artist` to
     `feed.release_artist` for the audio tag comparison grid.

   - `src/views.rs::FeedView::from_local` and `from_local_with_facts` set
     the feed's displayed artist from `tracks.first().artist`. Record
     that this is a derived value from one track, not a stored feed-level
     fact.

   - `src/rss/subscribe.rs::subscribe_feed` sets
     `album_artist_name = feed_artist.or_else(|| artist_name.clone())` at
     ingest time. Record that this collapses two distinct claims into one
     untagged column before storage.

6. For Language, state that `api::Track` carries no language field today.
   Cite the `Track` struct in `src/api.rs`. Record this as a current limit,
   not an invented rule.

   - `src/rss/subscribe.rs::subscribe_feed` writes the RSS `<language>`
     tag into the legacy `feeds.language` column. State that, unlike
     Feed description, no later MusicIndex fetch overwrites this column.

   - `src/identity_ingest.rs::feed_metadata_facts_by_source` writes a
     typed `entity_metadata_facts` row for fact key `language`, source
     `musicindex`, from the top-level `Feed.language` field.

   - `src/views.rs::FeedView::from_local_with_facts` reads
     `metadata_facts.language.or_else(|| nonempty_owned(f.language))`.
     State that this prefers the MusicIndex fact over the RSS column, with
     no rule that states this order as a decision.

7. For Explicit state, name the chain that parses `TrackRow.explicit`.

   - `src/rss/subscribe.rs::subscribe_feed` stores the raw iTunes
     `explicit` tag text in `tracks.itunes_explicit`.

   - `src/db.rs::parse_itunes_explicit` reads that text at query time. It
     maps `explicit`, `yes` and `true` to `Some(true)`, and `clean`, `no`
     and `false` to `Some(false)`. Any other text becomes `None`.

   - `src/identity_ingest.rs::feed_metadata_facts_by_source` and
     `track_metadata_facts` write a typed `entity_metadata_facts` boolean
     row for fact key `explicit`, source `musicindex` only.

   - `src/views.rs::TrackView::from_local_with_facts` reads
     `metadata_facts.explicit.or(t.explicit)`. The MusicIndex fact wins
     over the parsed RSS text when both exist.

8. For Release date and Publication date, name the two separate rules the
   app runs today.

   - `src/views.rs::FeedView::from_api` and `from_local_with_facts` read
     `f.release_date` or `metadata_facts.release_date` with no fallback
     chain. `TrackView::from_api` reads `t.pub_date` directly.
     The local route uses `metadata_facts.pub_date.or(t.pub_date)`.

   - `src/metadata.rs::feed_release_pubdate` runs a different, longer
     chain for the audio tag comparison grid: a release-date claim, then
     `feed.release_date`, then `feed.oldest_item_at`, then the earliest
     track `pub_date`.

   - `src/metadata.rs::musicindex_release_date` falls back from
     `track_release_pubdate` to the feed's own `feed_release_pubdate`
     chain when the track supplies no date. Record this as a feed-to-track
     fallback in the same family the review names for description,
     artwork and publisher, though no audit finding names it by name.

   - `src/rss/subscribe.rs::subscribe_feed` stores the raw RSS `pubDate`
     text. `src/db.rs::parse_local_track_pub_date` parses it with RFC 2822
     at query time into `TrackRow.pub_date`.

9. For Duration, state that `api::Track.duration_secs` and
   `TrackRow.duration_seconds` are the only two current representations.

   - `src/rss/subscribe.rs::subscribe_feed` parses the iTunes `duration`
     tag into `tracks.duration_seconds`.

   - Confirm, by reading `src/local_metadata.rs::track_facts_from_rows`
     and `src/identity_ingest.rs::track_metadata_facts`, that duration has
     no `entity_metadata_facts` fact key today.

   - Record this fact as an open question for the schema packet, not as
     an audit-document citation: duration carries no typed source-fact
     record and no stated conflict rule.

10. State which of this packet's fields `src/api.rs::track_with_feed_defaults`
    copies from a feed onto a track today. Cite its field list by name. State
    that it copies `release_artist`, but not `language`, `explicit`,
    `release_date`, `pub_date`, `duration_secs` or `track_artist`.

11. Propose an owner label for each feed value that appears on a track.
    Include "Feed album artist" as the proposed label for the Album artist row.
    Mark these text-field labels as pending operator review. Decision B governs identity placement.

    State that no other field in this packet has a current feed-to-track
    fallback inside `track_with_feed_defaults` itself.

12. Named evidence, or a named open question, for every rule above.

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

- Required change: the change a later packet must make. Write "None" when
  the current rule already matches this row.

## Additional Assignments

Packet 033 preserves upstream track language and artist sort text in app transport.
Packet 031 owns the remaining title, number, classification, and sort-text rules.
Packet 034 owns remaining scalar and aggregate rules. This packet does not complete those assignments.

## Acceptance Criteria

### Mechanical Criteria

- The file `docs/schema/adr-0075-field-rules-artist-language-dates.md`
  exists.

- The document contains one complete Field Rule Template row for each of
  the seven listed fields. No cell is empty.

- The document states the ADR 0045 boundary. It cites
  `src/identity_ingest.rs::persist_track_artist_bindings` and states that
  this packet does not extend that binding.

- The document states that `api::Track` carries no language field today.
  It cites the `Track` struct in `src/api.rs`.

- The document states that duration has no `entity_metadata_facts` fact
  key today. It cites `src/identity_ingest.rs::track_metadata_facts`.

- The document states which fields `track_with_feed_defaults` copies from
  a feed to a track, and which it does not.

- The document lists every open question it hands to a later packet.

- Every local link in the document resolves.

### Review Criteria

- Binding rules match ADR 0075 decisions 3 and 4. Unresolved field policies appear as proposals for operator review.

- The document adds no person-merge rule and no artist-identity claim.
  This matches ADR 0045's non-goals and ADR 0075's non-goals.

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
  docs/tasks/adr-0075-task-006-field-rules-artist-language-dates.md \
  docs/schema/adr-0075-field-rules-artist-language-dates.md
```

```bash
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" \
  --check --no-heuristics docs/schema/adr-0075-field-rules-artist-language-dates.md
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

- A binding rule, a merge rule or an artist-identity claim of the kind
  ADR 0045 reserves for a later decision.

- A field rule that belongs to packet 005 or packet 007.

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

- Write `docs/schema/adr-0075-field-rules-artist-language-dates.md` with
  the content in this packet's Required Content section.

Constraints:

- Write in ASD-STE100 Simplified Technical English.

- Cite named source evidence for each rule. Record a gap as an open
  question for a later packet when you find no evidence.

- Do not merge people. Do not claim an artist identity.

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
