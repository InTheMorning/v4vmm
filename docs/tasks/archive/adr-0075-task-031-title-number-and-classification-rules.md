# ADR 0075 Task 031: Title, Number And Classification Rules

Status: Individual field review in progress - 2026-09-21. Feed and track title rules have individual acceptance.
The operator approved orchestration to complete ADR 0075 on 2026-09-20.
That instruction releases this document work. It does not accept new field policies.

The operator separately accepted fresh direct RSS priority for feed titles on 2026-09-21.
Retain RSS and MusicIndex assertions with their original evidence. Other policy proposals retain individual review gates.
Feed titles also use the accepted removal, conflict, and stale-state rules, with original text and selected absence retained.
Within MusicIndex, prefer `title`, then legacy `name`. Retain both values and their field paths.

When no feed title is selected, keep "Unknown Feed" as a display label only.
Do not store that generated label as source metadata.
Hide only confirmed generated feed title placeholders. Retain literal publisher-supplied titles and the evidence that identifies generated values.

Track titles use the feed title source priority, title/name order, removal, conflict, stale-state, and placeholder rules.
Prefer fresh direct item RSS titles over MusicIndex. Retain original text, source evidence, and selected absence under the track owner.
When no track title is selected, display its GUID, then "Untitled" when no GUID exists. These labels do not create source metadata.

Feed-title reference fallback is accepted. Allow a track response's `feed_title` as a labeled feed reference without a separately selected feed title.
Retain the feed owner and source evidence. Do not create a track title or embedded album assertion.
Verified feed-title removal hides that reference while retaining its evidence.
Apply the feed title's accepted conflict, stale-state, and generated-placeholder rules to the reference.

## Goal

Complete the missing field rules assigned to packet 031 in the
[inventory](../../schema/adr-0075-metadata-field-inventory.md).
The [deliverable](../../schema/adr-0075-field-rules-titles-numbers-classification.md) covers titles, numbers, raw author text, artist sort text, medium, and release kind.
Application implementation remains a separate task.

## Files To Inspect

- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md), accepted ownership and evidence rules.
- [Phase plan](../../plans/adr-0075-metadata-contract-phase-plan.md), packets 011, 020, 031, and 033.
- [Field inventory](../../schema/adr-0075-metadata-field-inventory.md), assigned scalar fields.
- [Packet 005 rules](../../schema/adr-0075-field-rules-description-artwork-publisher.md), publisher boundaries.
- [Packet 006 rules](../../schema/adr-0075-field-rules-artist-language-dates.md), accepted artist field policies.
- [Packet 007 rules](../../schema/adr-0075-field-rules-links-and-media.md), source priority scope.
- [ADR 0004](../../adr/0004-format-neutral-audio-tag-boundary.md) and [ADR 0008](../../adr/0008-explicit-id3v24-write-boundary.md), tag boundaries.
- [api.rs](../../../src/api.rs), `Feed`, `Track`, and `track_with_feed_defaults`.
- [views.rs](../../../src/views.rs), feed and track constructors.
- [entity_detail.rs](../../../src/view_models/entity_detail.rs), `ReleaseDetailVm::title` and `SharedTrackRowVm::title`.
- [subscribe.rs](../../../src/rss/subscribe.rs), `subscribe_feed`.
- [enrich.rs](../../../src/rss/enrich.rs), `fetch_track_enrichment_from_feed` and `apply_track_enrichment`.
- [audio_tags.rs](../../../src/audio_tags.rs), `AudioTags` and readers.
- [metadata.rs](../../../src/metadata.rs), `grouped_id3_frame_labels` and `id3_sort_order_values`.
- [identity_ingest.rs](../../../src/identity_ingest.rs), `feed_metadata_facts_by_source`.
- [db.rs](../../../src/db.rs), `TrackRow`, `FeedRow`, and current scalar storage.
- Upstream `/home/citizen/build/stophammer/stophammer-parser/src/profile.rs` and `types.rs`, read-only.
- Upstream `/home/citizen/build/stophammer/src/api.rs`, `ingest.rs`, and `query.rs`, read-only.

## Files Likely To Change

- This task packet.
- `docs/schema/adr-0075-field-rules-titles-numbers-classification.md`.

## Do Not Touch

- Rust source, tests, dependencies, database files, migrations, or app configuration.
- Shared ADR, phase plan, inventory, review, or status files during parallel document work.
- Other field-rule documents or the upstream checkout.
- Existing tag-write policies, artist bindings, payment routes, or playback behavior.

## Constraints

Use the shared STE skill at `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.
State observed behavior separately from accepted invariants and proposed policies.
Give each field an owner, provenance, source order, conflict result, and missing-value result.

Do not extend Decision F's accepted priority to unrelated fields without a marked proposal.
Do not infer a raw source path from an API alias or derived scalar.
Keep field-rule choices concrete. Do not delegate unresolved policy design to a coding agent.

## Implementation Steps

1. Trace the assigned fields through direct RSS, Index transport, legacy storage, and current views.
2. Trace album, number, and sort fields through the tag reader without changing tag-write policy.
3. Record upstream derivation and transport losses with exact source owners.
4. Write each proposed rule and its missing-value behavior.
5. Add constructed cases for ownership, conflicts, aliases, number validation, and failure retention.
6. Identify product decisions that require operator acceptance.
7. Run the document checks below.

## Acceptance Criteria

These are document review criteria. No application behavior is claimed by this packet.

- Each assigned inventory field has a completed rule, checked against the source functions in the evidence table.
- Feed titles, track titles, compatibility names, and embedded album titles remain distinct in the documented examples.
- Disc and season evidence remains separate in case T31-07.
- Cases T31-05, T31-06, and T31-08 specify number priority and failed numeric conversion.
- Cases T31-09 through T31-11 specify author ownership and sort association.
- Cases T31-12 and T31-14 specify classification conflicts without inferred release kinds.
- Case T31-13 preserves earlier observations after request failure.
- The document names pending decisions without declaring them accepted.
- Local links pass the repository checker. Confirmed language defects are corrected after the shared STE check.

## Checks

```bash
python3 docs/runbooks/check-markdown-links.py docs/tasks/archive/adr-0075-task-031-title-number-and-classification-rules.md docs/schema/adr-0075-field-rules-titles-numbers-classification.md
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" --check --no-heuristics docs/tasks/archive/adr-0075-task-031-title-number-and-classification-rules.md docs/schema/adr-0075-field-rules-titles-numbers-classification.md
git diff --check
```

The STE checker covers configured rules only. Its result does not prove full standard compliance.
Application tests are unnecessary for this document-only packet.

Check result - 2026-09-20: 34 local links and diff whitespace are Green.
The shared STE check reports lexical findings. No structural finding remains after correction.
Retained technical terms include provenance, source evidence, medium, release kind, classification, and operator review.
The required coding-model prompt also retains the skill's exact wording.

## Rollback

Remove or amend only this packet's new documents before adoption.
Preserve unrelated working-tree changes. This packet changes no application or database state.

## Escalation Triggers

- A field needs a policy that contradicts an accepted ADR.
- An upstream source cannot establish the claimed owner or extraction path.
- A proposal would change tag writes, artist binding, payment inheritance, or playlist ordering.

## Expected Final Report

Report the two documents, check results, source evidence limits, and substantive decisions requiring operator review.
Distinguish completed document work from unimplemented application behavior.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet and every file under Files To Inspect.
- `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.

Goal:
- Complete the document rules assigned to packet 031.

Constraints:
- This is a document-only task.
- Preserve accepted ownership and source boundaries.
- Keep all new selection policies proposed until the operator accepts them.

Do not touch:
- Any file outside Files Likely To Change.
- Application state or the upstream checkout.

Acceptance criteria:
- Meet every criterion in Acceptance Criteria.
- Include the constructed regression cases and exact current source evidence.

Test commands:
- Run all commands under Checks.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

No visual check is required for this document task. The operator's visual pause remains in force.
