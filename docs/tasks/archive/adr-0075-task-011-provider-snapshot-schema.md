# ADR 0075 Task 011: Provider Snapshot Schema

Status: Complete - 2026-09-20. The technical deliverable passed orchestrator review after corrections.
This is one bounded document task. It authorizes no database migration or production code change.

## Goal

Define durable provider observations, collection snapshots, and discrepancy history under accepted ADR 0075 requirements.
Define migration 12 without breaking interrupted migration 11 recognition, preservation, or repair.
Keep every unaccepted field policy open.

The deliverable is [Provider Snapshot Storage](../../schema/adr-0075-provider-snapshot-storage.md).
The operator authorized orchestration. That authorization does not accept proposed field selections.

## Files To Inspect

- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md).
- [Phase plan](../../plans/adr-0075-metadata-contract-phase-plan.md).
- [Collection completeness rules](../../schema/adr-0075-collection-completeness-rules.md).
- [Field inventory](../../schema/adr-0075-metadata-field-inventory.md).
- [Packet 031 rules](../../schema/adr-0075-field-rules-titles-numbers-classification.md).
- [Packet 034 rules](../../schema/adr-0075-field-rules-aggregates-and-relationships.md).
- [Packet 035 contract](../../schema/adr-0075-comparison-and-discrepancy-contract.md).
- [Packet 009](adr-0075-task-009-rss-owner-and-nostr-extraction.md).
- [db.rs](../../../src/db.rs), migration registry, schema inspection, existing facts, and `open_db`.
- [identity_ingest.rs](../../../src/identity_ingest.rs), current provider transport and replacement calls.
- [startup.rs](../../../src/db/startup.rs), readiness and preparation.
- [upgrades.rs](../../../src/db/upgrades.rs), migration-11 recognition and interruption fixtures.
- [maintenance.rs](../../../src/db/maintenance.rs), verified SQLite snapshots and bounded work.
- [preservation.rs](../../../src/db/maintenance/preservation.rs), exclusive access and original-file preservation.
- [restore.rs](../../../src/db/maintenance/restore.rs), candidate upgrade, preservation comparison, installation, and rollback verification.
- `src/application/commands/maintenance.rs` and `src/view_models/startup/database.rs`, maintenance outcomes and readiness reports.
- [cli.rs](../../../src/cli.rs), `open_configured_db`.
- `src/metadata.rs`, `src/rss/subscribe.rs`, `src/rss/enrich.rs`, `src/subscribe_service.rs`, and `src/feed_service.rs`.
- `src/application/queries/feed.rs` and `src/application/queries/library.rs`.

## Files To Change

- This packet.
- `docs/schema/adr-0075-provider-snapshot-storage.md`.

## Do Not Touch

- Rust source, Cargo files, migrations, databases, configuration, audio files, or secrets.
- Shared ADR, plan, inventory, review, or status documents owned by the coordinator.
- The upstream checkout.
- Field selection, payment behavior, broadcast behavior, or product UI.

## Constraints

Apply the shared STE, repository documentation, and feature orchestration skills.
Separate accepted requirements from technical design choices and open product policies.
Keep raw evidence before decoding or cleaning. Share large bodies across observations.
Separate stable decoding inputs from per-fetch headers. Occurrence-only header changes cannot create duplicate observations or discrepancy history.

Keep requested subjects separate from declared owners. Use feed-scoped track keys.

Keep historical provider and ownership uncertainty unresolved.
Do not infer complete coverage from `Option<Vec<_>>` shape.
Do not infer a provider snapshot from a legacy assertion-source label.

Do not put `comparison_version` into stable discrepancy identity.
Preserve accepted absence rules and selected field states. Do not choose remaining display policies.

Include feed-artwork and track-artwork selected states. Preserve the feed owner when track artwork uses feed fallback.
Include both provider resources in stable discrepancy identity. Artwork selection does not extend discrepancy comparison fields.
Do not launch the app, request a visual batch, or commit changes.

## Document Steps

1. Inspect current collection, ingestion, migration, startup, and maintenance boundaries.
2. Specify exact table columns, keys, constraints, and typed application operations.
3. Specify complete, partial, failed, superseded, and restart transitions.
4. Specify immutable discrepancy evidence and repeated-observation behavior.
5. Trace version-11 recognition and repair through the proposed version-12 registry.
6. Define preservation, backup, migration rollback, and fixture checks through existing database owners.
7. Name the live composition roots for later implementation packets.
8. Mark each unresolved field policy separately from technical review.
9. Check local links, whitespace, and shared STE findings.
10. Report the design and remaining risks directly to the coordinator.

## Acceptance Criteria

These criteria are document checks. They do not claim implementation or runtime acceptance.

1. The schema defines provider, resource, requested subject, declared owner, raw body, coverage, and source-time boundaries.
2. Track keys include feed scope. Uncertain ownership cannot authorize replacement.
3. Contributors, enclosures, transcripts, and unknown source properties remain recoverable.
4. One shared body supports many tracks. Identical observations do not add duplicate facts or history.
   Occurrence-only header changes preserve that guarantee and retained transition evidence.
5. Failed response evidence remains separate from successful snapshots.
6. Atomic replacement covers all assertion-source labels within one provider collection.
7. Own-track, inherited-feed, and empty credits cannot erase an unrelated feed snapshot.
8. Generations survive restart and prevent accepted-generation regression without becoming source timestamps.
9. Discrepancy history survives replacement and comparison-version changes.
10. Known absence remains distinct from unknown coverage. Selected absence survives expiry and restart without accepting other field policies.
    Artwork state preserves the track's own absence and the separate feed fallback.
11. Migration 12 preserves legacy rows and does not invent provider snapshots.
12. Version-11 recognition, repair, candidate validation, and fixture construction remain version-specific.
13. Startup and CLI cannot migrate an existing database before verified preservation and backup.
14. The rollback procedure preserves ledger integrity and uses existing database tools.
15. The design names live callers and mechanical regression cases for later implementation.
16. Local links and whitespace pass. Confirmed STE defects are corrected.
17. Different Index resources under one endpoint cannot share or resolve each other's discrepancy.

## Test Commands

Run from the repository root:

```bash
python3 docs/runbooks/check-markdown-links.py docs/tasks/archive/adr-0075-task-011-provider-snapshot-schema.md docs/schema/adr-0075-provider-snapshot-storage.md
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" --check --no-heuristics docs/tasks/archive/adr-0075-task-011-provider-snapshot-schema.md docs/schema/adr-0075-provider-snapshot-storage.md
git diff --check
```

No application tests or database operations are required for this document task.
Future code packets must implement the schema document's S11 regression cases with disposable fixtures.
Their normal checks include `cargo check`, focused tests, `cargo fmt -- --check`, and `cargo clippy -- -D warnings`.

## Recorded Checks

Checked on 2026-09-20. Local links: Green, 30 links in two files. Diff whitespace: Green.
The shared STE check found no structural defects after correction.

Lexical findings remain for technical terms, including provider, snapshot, observation, and discrepancy.
The raw STE result is not Green. Orchestrator technical review passed after the storage corrections.

## Rollback And Escalation

Rollback removes only this packet's document edits.
Escalate any conflict with an accepted ADR or an unsupported claim of complete coverage.
Escalate migration behavior that bypasses preservation, weakens version-11 recognition, or rewrites historical facts.
Refer each new field policy to the operator. Do not infer acceptance from orchestration authorization.
Do not dispatch migration implementation before root technical review passes.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet and every entry under Files To Inspect.
- `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.

Goal:
- Complete the packet 011 storage design documents. Make no application code change.

Constraints:
- Follow this packet's Constraints and Document Steps.
- Preserve accepted source boundaries and unresolved product policy status.
- Write STE prose directly. Run the shared checker.

Do not touch:
- Every path and behavior under Do Not Touch.

Acceptance criteria:
- Satisfy every document criterion above.
- Leave root technical review open until its result is recorded.

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
No app launch, production database operation, or operator fixture cleanup is required.
