# ADR 0054 Review Checklist

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

## Merge Recommendation Template

- Pass/fail:
- Required fixes:
- Optional improvements:
- Architectural drift:
- Missing tests:
- Safe to merge:
- Next task packet adjustments:
