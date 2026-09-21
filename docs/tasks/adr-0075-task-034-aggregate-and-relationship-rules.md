# ADR 0075 Task 034: Aggregate And Relationship Rules

Status: Deliverable written - 2026-09-20. The document review remains open.
Feed publication-date source priority, removal, conflict, and stale-state rules have individual acceptance.
The other proposed field policies stay open.

The operator requested orchestration of ADR 0075 completion. This packet changes documentation only.
Its [field rules](../schema/adr-0075-field-rules-aggregates-and-relationships.md) separate accepted invariants from proposed selections.
No implementation packet is released by this document.

## Goal

Complete the field inventory's aggregate, recorded-time, remaining feed scalar, and relationship rules.
Identify app transport omissions separately from missing upstream evidence.
Keep payment behavior and broadcast operations unchanged.

## Files To Inspect

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md).
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md).
- [Field inventory](../schema/adr-0075-metadata-field-inventory.md).
- [Collection rules](../schema/adr-0075-collection-completeness-rules.md).
- [Date rules](../schema/adr-0075-field-rules-artist-language-dates.md).
- [Upstream request](../plans/adr-0075-stophammer-decision-request.md).
- [api.rs](../../src/api.rs), DTOs and summary response decoding.
- [db.rs](../../src/db.rs), `FeedRow`, `TrackRow`, and metadata facts.
- [identity_ingest.rs](../../src/identity_ingest.rs), retained source claims.
- [enrich.rs](../../src/rss/enrich.rs), RSS episode count and publication date handling.
- `/home/citizen/build/stophammer/src/query.rs`, response types, includes, and publisher queries.
- `/home/citizen/build/stophammer/src/api.rs`, feed aggregates and release claim construction.
- `/home/citizen/build/stophammer/src/db.rs`, `upsert_feed` and `upsert_track`.
- `/home/citizen/build/stophammer/stophammer-parser/src/types.rs`, parsed feed and namespace types.

## Files To Change

- This packet.
- `docs/schema/adr-0075-field-rules-aggregates-and-relationships.md`.

## Do Not Touch

- Rust source, dependencies, database schemas, and migrations.
- Shared ADR, plan, inventory, status, and review documents owned by the coordinator.
- The upstream checkout.
- Payment routes, broadcast scheduling, tag export, and product UI.

## Constraints

Apply the shared STE, documentation, and orchestration skills.
Read upstream code without writing to its checkout.
Record the inspected revision and each evidence boundary.
Mark new source selections as proposals until the operator accepts them.

Do not infer completeness from a successful request or a summary array.
Do not reserve follow-up packet numbers. Give the coordinator bounded transport requirements.
Do not launch the app or request a visual batch.

## Implementation Steps

1. Trace each assigned field from parser to API and app storage.
2. Separate observed code behavior from accepted rules and proposed selections.
3. State ownership, source selection, conflict handling, and the absent result for each field.
4. Define exact existing API fields for bounded app transport work.
5. Record summary limits and unknown coverage.
6. Assign mechanical regression cases to later implementation packets.
7. Run the document checks.

## Acceptance Criteria

Document review verifies each criterion below. It does not establish implementation acceptance.

1. Each assigned field has a concrete rule and a stated acceptance status.
2. Counts retain their derivation scope and integer range.
3. Creation, update, source observation, fetch, and publication times remain separate.
4. Feed publication evidence does not use an oldest-item fallback as a channel assertion.
5. iTunes type uses its existing release claim transport.
6. Platform, remote item, and publisher facts keep their distinct owners and meanings.
7. Value time split preservation changes no payment behavior.
8. Namespace evidence distinguishes a parsed snapshot from original XML.
9. Summary responses cannot erase omitted detail fields.
10. Existing API omissions have exact field contracts for a bounded follow-up.
11. Local links pass. Confirmed STE defects are corrected without changing technical meaning.

## Test Commands

Run from the repository root:

```bash
python3 docs/runbooks/check-markdown-links.py docs/tasks/adr-0075-task-034-aggregate-and-relationship-rules.md docs/schema/adr-0075-field-rules-aggregates-and-relationships.md
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" --check --no-heuristics docs/tasks/adr-0075-task-034-aggregate-and-relationship-rules.md docs/schema/adr-0075-field-rules-aggregates-and-relationships.md
git diff --check
```

These checks validate documents only. No application test is required for this packet.

## Recorded Checks

Checked on 2026-09-20. Local links: Green, 20 links in two files.
Diff whitespace: Green. The shared STE check found no structural defects after correction.

Lexical findings remain, including technical uses of review, request, evidence, and value time split.
The raw STE result is not Green. The document review remains open.

## Rollback And Escalation

Rollback removes only this packet's document edits. It changes no application state.
Escalate an unassigned field or a conflict with an accepted ADR rule to the coordinator.
Escalate an upstream contract gap before dependent app code assumes the missing evidence exists.
Do not broaden the packet to repair an observed upstream defect.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet and every entry under Files To Inspect.
- `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.

Goal:
- Write the document rules for packet 034. Make no application code change.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve source ownership, evidence gaps, and proposed decision status.
- Write STE prose directly. Run the shared checker.

Do not touch:
- Every path and behavior listed under Do Not Touch.

Acceptance criteria:
- Satisfy every document review criterion above.
- Keep the operator review open until acceptance is recorded.

Test commands:
- Run every command under Test Commands.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

This packet changes no presentation. The existing visual pause remains in force.
No app launch or fixture cleanup is required.
