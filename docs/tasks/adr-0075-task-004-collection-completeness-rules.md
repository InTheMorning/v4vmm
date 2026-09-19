# ADR 0075 Task 004: Write The Collection Completeness And Transition Rules

Status: Ready - 2026-09-19. Work has not started.
The parent ADR 0075 is Accepted from 2026-09-19.
The operator holds the dispatch of every ADR 0075 packet.
This packet produces a document. It changes no code.

## Goal

Write the document that states, for every response collection the app
requests, when a response authorises the app to replace a stored snapshot.
Ground each rule in ADR 0075
[decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit)
and
[decision 5](../adr/0075-metadata-ownership-and-completeness.md#5-replace-a-complete-provider-snapshot-atomically).
This packet does not change storage code or request code.
It defines the rules that packets 011 and 017 must follow.

## Files To Inspect

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), decision 2 and decision 5
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), the
  [Packet Register](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register)
  and
  [Schema And API Implications](../plans/adr-0075-metadata-contract-phase-plan.md#schema-and-api-implications)
- [Review](../reviews/adr-0075-metadata-contract-review.md), the findings
  [Returned Credits Can Belong To The Feed](../reviews/adr-0075-metadata-contract-review.md#returned-credits-can-belong-to-the-feed),
  [An Empty Refresh Can Leave Old Index Facts](../reviews/adr-0075-metadata-contract-review.md#an-empty-refresh-can-leave-old-index-facts)
  and
  [Repeated Requests Can Still Omit Required Facts](../reviews/adr-0075-metadata-contract-review.md#repeated-requests-can-still-omit-required-facts)
- [Task 001](adr-0075-task-001-contributor-claim-transport.md), the field
  contract for `api::Contributor`
- The deliverable of packet 002, the shared example corpus. Confirm its path
  first. Packet 002 did not exist when this packet was written.
- `src/api.rs`: the `Feed`, `Track` and `Contributor` structs. Also
  `fetch_feed`, `fetch_track`, `fetch_feed_track`, `fetch_contributors`,
  `track_with_feed_defaults`, `response_json` and `Pagination`
- `src/application/queries/search.rs`: `INDEX_FEED_DETAIL_INCLUDE`,
  `fetch_index_track_result_rows` and `fetch_index_track_detail`
- `src/identity_ingest.rs`: `persist_source_links`, `persist_source_ids` and
  `persist_contributors`
- `src/db.rs`: `replace_local_identity_links` and `local_identity_links`
- `src/subscribe_service.rs`: `enrich_track_context_from_rss`

Inspect the upstream Stophammer checkout read-only, at
`/home/citizen/build/stophammer`, commit `a220f44`:

- `src/query.rs`: `ListQuery::includes`, `handle_get_feed`, `handle_get_track`
  and `SourceContributorClaimResponse`
- `src/db.rs`: `get_effective_source_contributor_claims_for_track`
- `src/openapi.rs`: the documented `include` values for the feed route and
  the track route
- `src/ingest.rs`: `content_hash` and `force_reingest`

## Deliverable

One document: `docs/schema/adr-0075-collection-completeness-rules.md`.

## Do Not Touch

- Every Rust source file in this repository. This packet writes one document.
- The Stophammer checkout. Read it. Do not write in it.
- The field-level fallback rules for description, artwork, publisher and the
  other fields. Packets 005, 006 and 007 own those rules.
- The supported Nostr syntax and the supported `podcast:txt` purpose values.
  Packet 003 owns that rule.
- A Stophammer ADR number or a Stophammer code proposal. Packet 008 owns the
  decision request.
- The database schema, a migration or a provider-identity design. Packet 011
  owns the schema.

## Constraints

Write each sentence in this deliverable in ASD-STE100 Simplified Technical
English.

Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
Write compliant prose directly. Do not make a draft that requires conversion to STE.

Use the shared checker in Checks. Correct only confirmed errors in the affected prose.
A Green result covers the configured checks. It does not prove full dictionary compliance.

State the evidence for each rule: a file path, a function or
struct name, and the observed behavior. When the current source
cannot prove a rule, write the rule as an open question. Name it for packet
008. Do not invent a completeness signal that the current API does not
supply. Use these ADR 0075 terms: fact, subject, claim, provenance,
provider, collection, snapshot, coverage state and projection. Do not change
a term's meaning from its ADR 0075 definition.

This deliverable depends on packet 002. Use its example corpus for the
worked cases in the transition table.

## Required Content

The deliverable must contain:

1. One row for each endpoint and collection that the app requests. Name the
   collection, the requested subject, and the owners the collection can
   return. Base this row set on the `Feed` and `Track` fields in `src/api.rs`
   and the include values in `src/application/queries/search.rs` and
   `src/subscribe_service.rs`.
2. The five states from ADR 0075 decision 2: not requested, not returned,
   returned empty, returned populated and request failed. State the refresh
   effect of each state.
3. The rule that a malformed payload is a failed request. State that a
   malformed payload is never an empty collection. Cite
   `src/api.rs::response_json`, which returns a decode error for a type
   mismatch. Cite the current gap where
   `src/application/queries/search.rs::fetch_index_track_result_rows` calls
   `.ok()` on a failed fetch and discards that error.
4. The rule that the requested subject and a returned fact's declared
   subject are separate fields. State that a track request does not
   authorise replacement of a feed snapshot. Cite the Stophammer
   `get_effective_source_contributor_claims_for_track` function, which
   returns feed-owned claims when track claims are empty. Cite the app's
   `persist_contributors` function. It currently stores every returned
   contributor under the requested owner, not under each contributor's own
   declared owner.
5. The complete transition table for a track's credits: the track's own
   credits, then inherited feed credits, then no credits. For each
   transition, state which collection is complete. State which earlier facts
   the app may replace.
6. The effect of pagination on each collection. State that the app's
   `Pagination` type wraps only the top-level list and search responses. It
   does not wrap the embedded collections in a feed or track detail response.
7. The effect of an unsupported `include` value. Cite Stophammer's
   `ListQuery::includes`, which matches an exact string and silently ignores
   an unmatched token.
8. The statement that a successful HTTP status alone does not prove
   completeness. Cite the absence of a completeness field in Stophammer's
   `api.rs`, `query.rs` and `openapi.rs`.
9. Named evidence, or a named gap, for every rule above. Record a gap as an
   open question for packet 008 when the current API cannot prove the rule.

## Acceptance Criteria

### Mechanical Criteria

- The file `docs/schema/adr-0075-collection-completeness-rules.md` exists.
- The document contains a row for every collection named in `src/api.rs`'s
  `Feed` and `Track` structs: `tracks`, `source_contributors`,
  `source_links`, `source_ids`, `source_release_claims`,
  `source_enclosures` and `payment_routes`.
- The document contains one row for each of the five collection states from
  ADR 0075 decision 2.
- The document contains the complete three-step credit transition table:
  track credits, then feed credits, then no credits.
- The document states the malformed-payload rule. It cites
  `src/api.rs::response_json`.
- The document states the requested-subject rule. It cites both
  `get_effective_source_contributor_claims_for_track` and
  `persist_contributors`.
- The document lists every open question it hands to packet 008.
- Every local link in the document resolves.

### Review Criteria

- The stated rules match ADR 0075 decision 2 and decision 5. The document
  adds no new product decision.
- The transition table is usable by the packet 011 schema writer without a
  further product decision.
- An unrun review check reports this gate as open. It never reports the gate
  as met.

## Checks

Run these commands from the repository root after writing the deliverable.
The link check tests local files and headings. It returns failure for a missing target.
It does not change the current directory or request external pages.
Use Markdown links for local references. Use fenced code blocks for recorded syntax.

```bash
python3 docs/runbooks/check-markdown-links.py \
  docs/tasks/adr-0075-task-004-collection-completeness-rules.md \
  docs/schema/adr-0075-collection-completeness-rules.md
```

```bash
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" \
  --check --no-heuristics docs/schema/adr-0075-collection-completeness-rules.md
```

Examine the STE findings. Correct confirmed defects in the affected prose.
Record retained technical names and the reasons for retaining them.
Report a checker error as a failed check.
Do not report the raw checker result as Green if findings remain.
No application test applies to this document packet.

## Escalation Triggers

Stop if the work needs any of these changes:

- A code change in this repository or in the Stophammer checkout.
- A new product decision that ADR 0075 does not already state.
- A field-level fallback rule that belongs to packets 005, 006 or 007.
- A Stophammer ADR number or a Stophammer code proposal.

Revise this packet before you continue.

## Expected Report

Name the deliverable's path and its section list. State which rules carry
named source evidence. State which rules are open questions for packet 008.
Report the mechanical checks as Green, or name the failure and its cause.
State separately whether the review criteria are met, open or not run.

## Prompt for lower-context coding model

You are writing one bounded document from a larger plan. Do not write code.

Read:

- This packet and every file in its Files To Inspect section.

Goal:

- Write `docs/schema/adr-0075-collection-completeness-rules.md` with the
  content in this packet's Required Content section.

Constraints:

- Write in ASD-STE100 Simplified Technical English.
- Cite named source evidence for each rule. Record a gap as an open question
  for packet 008 when you find no evidence.
- Do not touch any boundary in this packet's Do Not Touch section.

Acceptance criteria:

- Pass every mechanical criterion listed above.
- Leave every review criterion for a person. Report an unrun review check as
  open, not as met.

Checks:

- Run the commands in this packet's Checks section.

At the end, report:

1. the deliverable's path and section list
2. which rules carry named evidence, and which are open questions
3. mechanical check results
4. deviations from this packet
5. unresolved concerns
