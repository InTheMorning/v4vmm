# ADR 0075 Task 008: Write The Stophammer Decision Request

Status: Ready - 2026-09-19. Work has not started.
The parent ADR 0075 is Accepted from 2026-09-19.
The operator holds the dispatch of every ADR 0075 packet.
This packet produces a document. It changes no code.

## Goal

Write the request that the operator hands to the Stophammer repository.
Stophammer is a separate repository with its own decision record. This app
cannot change Stophammer. State the app behavior that ADR 0075 requires from
the Index API and the parser. Put that behavior in one document a
Stophammer maintainer can act on. This packet does not decide Stophammer's
answer. It states the app's requirement and names each open question.

## Files To Inspect

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), the
  Context section, decisions 1, 2, 3, 5, 6 and 7, and the References section
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), the
  [Affected Owners](../plans/adr-0075-metadata-contract-phase-plan.md#affected-owners)
  table, the
  [Schema And API Implications](../plans/adr-0075-metadata-contract-phase-plan.md#schema-and-api-implications)
  section, the
  [Risk Areas](../plans/adr-0075-metadata-contract-phase-plan.md#risk-areas)
  list and the
  [Rollback And Preservation](../plans/adr-0075-metadata-contract-phase-plan.md#rollback-and-preservation)
  section
- [Review](../reviews/adr-0075-metadata-contract-review.md), the findings
  [RSS Extraction Can Assign The Wrong Owner](../reviews/adr-0075-metadata-contract-review.md#rss-extraction-can-assign-the-wrong-owner)
  and
  [Website Rules Differ](../reviews/adr-0075-metadata-contract-review.md#website-rules-differ)
- [Task 001](adr-0075-task-001-contributor-claim-transport.md), the field
  contract for `api::Contributor`
- [Task 004](adr-0075-task-004-collection-completeness-rules.md), the
  collection completeness rules
- The deliverable of packet 002, the shared example corpus, and the
  deliverable of packet 003, the supported Nostr syntax and `podcast:txt`
  purpose values. Confirm both paths first. Packets 002 and 003 did not
  exist when this packet was written
- `src/api.rs`: the `Feed`, `Track`, `Contributor`, `SourceEntityLink`,
  `SourceEntityId`, `SourceReleaseClaim`, `SourceEnclosure` and
  `PaymentRoute` structs
- `src/application/queries/search.rs`, `src/identity_ingest.rs`, `src/db.rs`,
  `src/subscribe_service.rs` and `src/rss/enrich.rs`, for the app-side
  behavior a Stophammer change would affect

Inspect the upstream Stophammer checkout read-only, at
`/home/citizen/build/stophammer`, commit `a220f44`:

- `src/query.rs`: `SourceContributorClaimResponse`, `ListQuery::includes`
  and `CapabilitiesResponse`
- `src/db.rs`: `get_effective_source_contributor_claims_for_track`
- `src/api.rs` and `src/openapi.rs`: the documented routes and `include`
  values
- `src/ingest.rs`: `content_hash` and `force_reingest`
- `stophammer-parser/src/engine.rs`: `extract_persons`, `extract_entity_ids`
  and `extract_links`
- Stophammer's own `docs/adr/0039-feed-scoped-track-identity-routes.md`,
  `docs/adr/0040-store-track-identity-as-feed-scoped.md` and
  `docs/adr/0041-contributor-npub-source-evidence.md`

## Deliverable

One document: `docs/plans/adr-0075-stophammer-decision-request.md`.

## Do Not Touch

- Every file in the Stophammer checkout. Read it. Do not write in it.
- A Stophammer ADR number. This packet requests a decision. It does not
  reserve a number for that decision.
- Every Rust source file in this repository. This packet writes one
  document.
- The field-level fallback rules, the Nostr syntax rules and the
  completeness rules themselves. Packets 003, 004, 005, 006 and 007 state
  those rules. This packet assembles their output into one request.
- The database schema, a migration or a provider-identity design. Packet
  011 owns the schema.

## Constraints

Write each sentence in this deliverable in ASD-STE100 Simplified Technical
English.

Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
Write compliant prose directly. Do not make a draft that requires conversion to STE.

Use the shared checker in Checks. Correct only confirmed errors in the affected prose.
A Green result covers the configured checks. It does not prove full dictionary compliance.

State the source for each requirement: an ADR 0075 decision
number, or a named file and function. Use these ADR 0075 terms: fact, subject, claim, provenance, provider, collection, snapshot,
coverage state and projection.

State these three limits in the deliverable, in their own section:

- The agent must not write in the Stophammer checkout.
- The agent must not reserve a Stophammer ADR number.
- The deployed Stophammer revision remains unverified. Ask the operator for
  that evidence before Stophammer acts on this request.

This deliverable depends on packets 002, 003 and 004. Pull the shared
examples from packet 002, the supported Nostr syntax from packet 003, and
the completeness signal requirement from packet 004.

## Required Content

The deliverable must contain:

1. The app behavior ADR 0075 requires from the Index API, as a numbered
   list of requirements. Cite the ADR 0075 decision number for each
   requirement.
2. The exact fields and collections the app needs, and the owner each field
   declares. Use the `SourceContributorClaimResponse` fields from task 001
   and the `entity_type`/`entity_id` fields already present on
   `SourceEntityLink`, `SourceEntityId`, `SourceReleaseClaim` and
   `SourceEnclosure` in `src/api.rs`.
3. The completeness signal the app needs for each collection. Pull this
   requirement from packet 004's deliverable. State that Stophammer's
   `api.rs`, `query.rs` and `openapi.rs` define no such signal today.
4. The parser behavior the app needs, from packet 003: the supported
   identity syntax, the owner of a `podcast:person` key, and the valid
   positions of a `podcast:txt` value. Cite
   `stophammer-parser/src/engine.rs::extract_persons`, which assigns a
   credit's owner from the feed node or the item node around it, not from
   an attribute on the `podcast:person` element. Cite
   `extract_entity_ids`, which today accepts only `purpose="npub"` and
   drops every other purpose value.
5. The open questions the app cannot answer alone. Include whether
   `purpose="nostr"` is compatible with `purpose="npub"`, and whether
   Stophammer will add a completeness signal to the Index API.
6. The compatibility requirement. An older payload must stay readable
   after a Stophammer change. The generated API contract in `openapi.rs`
   must match the deployed routes and fields.
7. The recrawl requirement. Cite `src/ingest.rs::content_hash` and
   `force_reingest`. State that an unchanged feed hash can cause the
   ingestion to skip a feed. State that a parser deployment alone does not
   prove that stored facts changed.
8. The signed-event and replica checks a data path change needs. Cite ADR
   0075 decision 7 and the phase plan's requirement to test signed-event
   replay for a changed Stophammer data path.

## Acceptance Criteria

### Mechanical Criteria

- The file `docs/plans/adr-0075-stophammer-decision-request.md` exists.
- The document contains a numbered list of app behavior requirements. Each
  requirement cites an ADR 0075 decision number.
- The document contains a field and collection table with an owner column.
- The document states the three limits from this packet's Constraints
  section, word for word.
- The document states the recrawl requirement and cites `content_hash` and
  `force_reingest`.
- The document states the signed-event and replica check requirement.
- The document lists its open questions in their own section.
- The document states its dependency on packets 002, 003 and 004.
- Every local link in the document resolves.

### Review Criteria

- The requirements match ADR 0075. The document adds no new product
  decision for Stophammer.
- A Stophammer maintainer can open a Stophammer ADR from this request
  without further app-side research.
- An unrun review check reports this gate as open. It never reports the
  gate as met.

## Checks

Run this command after you write the packet file, and again after you write
the deliverable, from the repository root:

```bash
cd docs/tasks && grep -oE '\]\([^)]+\)' adr-0075-task-008-stophammer-decision-request.md \
  | sed -E 's/^\]\((.*)\)$/\1/' | grep -v '^http' | cut -d'#' -f1 | sort -u \
  | while read -r f; do test -e "$f" && echo "OK $f" || echo "MISSING $f"; done
```

```bash
cd docs/plans && grep -oE '\]\([^)]+\)' adr-0075-stophammer-decision-request.md \
  | sed -E 's/^\]\((.*)\)$/\1/' | grep -v '^http' | cut -d'#' -f1 | sort -u \
  | while read -r f; do test -e "$f" && echo "OK $f" || echo "MISSING $f"; done
```

```bash
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" \
  --check --no-heuristics docs/plans/adr-0075-stophammer-decision-request.md
```

No application test applies to this document packet.

## Escalation Triggers

Stop if the work needs any of these changes:

- A code change in this repository or in the Stophammer checkout.
- A reserved Stophammer ADR number.
- A write of any kind in the Stophammer checkout.
- A new product decision that ADR 0075 does not already state.

Revise this packet before you continue.

## Expected Report

Name the deliverable's path and its section list. State which requirements
carry named source evidence. State which items are open questions for the
operator. Report the mechanical checks as Green, or name the failure and
its cause. State separately whether the review criteria are met, open or
not run. Confirm that the three stated limits appear in the deliverable.

## Prompt for lower-context model

You are writing one bounded document from a larger plan. Do not write code.
Do not write in the Stophammer checkout.

Read:

- This packet and every file in its Files To Inspect section.

Goal:

- Write `docs/plans/adr-0075-stophammer-decision-request.md` with the
  content in this packet's Required Content section.

Constraints:

- Write in ASD-STE100 Simplified Technical English.
- Cite named source evidence for each requirement.
- State the three limits from this packet's Constraints section, word for
  word, in their own section.
- Do not touch any boundary in this packet's Do Not Touch section.

Acceptance criteria:

- Pass every mechanical criterion listed above.
- Leave every review criterion for a person. Report an unrun review check
  as open, not as met.

Checks:

- Run the commands in this packet's Checks section.

At the end, report:

1. the deliverable's path and section list
2. which requirements carry named evidence, and which are open questions
3. mechanical check results
4. deviations from this packet
5. unresolved concerns
