# ADR 0030 Task 004 Review: Discovery Compare Actions

## Reviewed Artifact

- `src/view_models/entity_detail.rs`
- `src/search.rs`
- `src/library.rs`
- `docs/tasks/adr-0030-task-004-discovery-compare-actions.md`

## Pass/Fail

Pass.

## Required Fixes

None.

## Optional Improvements

- Manual smoke should open a Discovery track and a Library track to verify the
  visible action rows match the projection behavior.

## Architectural Drift

None. The change uses the existing `EntitySurfaceContext` vocabulary and gates
the projection layer instead of hiding buttons only in screen code.

## Missing Tests

None for this bounded task. The new view-model test proves Discover context
projects no Compare ID3 or MusicBrainz actions, while the existing Library test
continues to prove those actions render for Library/local-file context.

## Merge Recommendation

Merge Task 004. Command gates passed on 2026-05-01.
