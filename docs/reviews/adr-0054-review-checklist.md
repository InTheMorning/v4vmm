# ADR 0054 Review Checklist

## Scope

Review ADR 0054 implementation slices against:

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- the active task packet

## Required Checks

- [ ] Metadata facts are not stored in identity source-fact tables.
- [ ] Source-scoped replacement preserves unrelated source rows.
- [ ] Owner-shape checks distinguish feed and track facts.
- [ ] Exactly one typed value slot is accepted per row.
- [ ] Empty source tokens and empty fact keys are rejected.
- [ ] Feed and track deletes cascade metadata facts.
- [ ] UI, renderer, and view-model layers do not query metadata facts directly.
- [ ] `podcast_medium` and `release_kind` remain distinct.
- [ ] No renderer hides, reinterprets, or invents source metadata.
- [ ] Tests are green for the task's required gate list.

## Merge Recommendation Template

- Pass/fail:
- Required fixes:
- Optional improvements:
- Architectural drift:
- Missing tests:
- Safe to merge:
- Next task packet adjustments:
