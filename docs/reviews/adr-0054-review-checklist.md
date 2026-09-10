# ADR 0054 Review Checklist

## Gate Status

Open - reconciled 2026-09-10. Tasks 004 and 005 have recorded mechanical
completion, but their reviews contain no operator acceptance of the visible
feed/track hydration paths. Task 006 changed guards only; its lack of a visual
criterion does not close those earlier requirements.

ADRs 0047/0048/0060 replace separate Library/Discover screens with local and
Index origins in Music. The persisted-fact display and fallback requirements
survive. Inspect the same facts on those current paths; do not restore old
screens or demand identical values from different sources.

## Scope

Review ADR 0054 implementation slices against:

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- the active task packet

## Required Checks

- [x] Metadata facts are not stored in identity source-fact tables.
  Guard: `metadata_source_fact_table_access_is_owned_by_db`.
- [x] Source-scoped replacement preserves unrelated source rows.
  Guard: existing DB unit tests for `replace_local_metadata_facts`.
- [x] Owner-shape checks distinguish feed and track facts.
  Guard: existing DB unit tests plus `metadata_source_fact_keys_stay_owner_scoped`.
- [x] Exactly one typed value slot is accepted per row.
  Guard: existing DB unit tests for `entity_metadata_facts` row constraints.
- [x] Empty source tokens and empty fact keys are rejected.
  Guard: existing DB unit tests for metadata fact validation.
- [x] Feed and track deletes cascade metadata facts.
  Guard: existing DB unit tests for metadata fact cascade behavior.
- [x] UI, renderer, and view-model layers do not query metadata facts directly.
  Guards: `ui_and_view_models_do_not_access_metadata_source_fact_storage`,
  `metadata_source_fact_storage_helpers_have_explicit_callers`. Coverage
  includes `src/views.rs`, `src/ui/**`, screen files, and `src/view_models/**`.
- [x] `rss_podcast_medium` and `musicindex_release_kind` remain distinct.
  Guard: `metadata_source_fact_release_kind_and_rss_medium_stay_distinct`.
- [x] No renderer hides, reinterprets, or invents source metadata.
  Guard: `ui_and_view_models_do_not_access_metadata_source_fact_storage`.
- [x] Tests are green for the task's required gate list.
  Status: targeted Task 006 gates passed locally.

## Operator Visual Check

Follow [Metadata Hydration](../runbooks/inherited-ui-checks.md#metadata-hydration--adr-0054-tasks-004-and-005).

| Packet | Required proof | Light | Dark |
|---|---|---|---|
| 004 | Known persisted feed facts remain readable through local entry, including when Index is unavailable | Open | Open |
| 005 | Known persisted track facts remain readable through local entry and remote fallback; source claims stay separate | Open | Open |

Record entity identifiers, populated facts, endpoint state, and results.
Fixtures without the source facts leave the relevant criterion open.

## Merge Recommendation

Mechanical implementation remains recorded. Keep ADR 0054 and its phase plan
Accepted until these checks pass and fixture cleanup is confirmed. Reconcile
both task statuses, this checklist, the ADR/index, delivery, and pending checks
when acceptance changes.
