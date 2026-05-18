# Deferred Architecture Work Index

## Status

Active index - 2026-05-08.

## Purpose

Keep completed ADRs closed while making the remaining deferred work visible,
prioritized, and routed to the right governance artifact.

## Priority Order

1. Person/global identity persistence.
   - Status: deferred from ADR 0029.
   - Route: future ADR only after durable person ids and merge policy exist.
2. ADR 0024 query/service thinning for remote-only Discover reads and remote
   inspector lazy panels.
   - Status: deferred from ADR 0024 final review.
   - Route: new ADR 0024 follow-up plan and bounded vertical slices.
3. ADR 0040 legacy synchronous scheduling retirement.
   - Status: Phase F default-on `async-runtime` flip has shipped, but
     `GpuiCommandRunner` and `--no-default-features` compatibility paths
     still exist.
   - Route: ADR 0040 follow-up after the remaining screen/runtime swaps
     are complete; do not remove legacy paths as part of unrelated UI
     feature work.
4. Staged metadata durability.
   - Status: product/storage decision required.
   - Route: future ADR before schema or command behavior changes.
5. Non-URL artwork rendering.
   - Status: audit completed; no producer/resolver contract yet.
   - Route: future ADR only when cache, storage, or public artwork contracts
     change.
6. Playback volume and playback-driver supervision.
   - Status: isolated playback-boundary follow-up.
   - Route: ADR 0021/0024 follow-up after the driver contract is clear.
7. Visual-system polish and lower-priority product improvements.
   - Status: use bounded ADR 0025 tasks only when the change affects tokens,
     primitives, composites, or theme contracts.

## Recently Resolved

- ADR 0038 presentation-contract enforcement closed on 2026-05-04 with
  readiness gate `Proceed`. Layer relocation, composite display
  contracts, VM consolidation, dark-mode parity, accessibility labels,
  PageVm generalization, screen decomposition, and final sweep are all
  complete. ADR 0038 added no new deferred items.
- ADR 0029 explicit artist identity persistence is complete for its runtime
  scope. It persists explicit MusicIndex artist source facts and hydrates
  `ArtistRef::Musicindex` locally without name matching.
- Library contributor identity visibility is no longer deferred. It was
  completed by `docs/tasks/post-adr-0028-task-001-library-contributor-panel.md`.
- ADR 0027 action-state parity is implemented and should not be reopened for
  unrelated data or service-boundary work.
- ADR 0040 and ADR 0041 status text was reconciled on 2026-05-08. The
  default-on async-runtime flip has landed; remaining legacy scheduling
  cleanup is now explicit deferred follow-up work instead of an ambiguous
  phase blocker.
- ADR 0042 layer-consolidation status was reconciled on 2026-05-08.
  The confirmed single-use composites were inlined, the retained
  composites have real multi-site use, and the remaining index items
  below still require their named future ADR routes.
- ADR 0045 planning artifacts were created on 2026-05-08 for the top
  deferred item: track-to-artist binding for name-derived Library artist
  views. Tasks 001-004 completed on 2026-05-11; name-derived Library artist
  views now enrich from explicit bindings without name-only merging.
- Library/Discover data parity triage completed on 2026-05-17 under ADR 0052.
  Runtime fixes remain routed, not implemented: loading-shape gaps go to
  `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`, and
  persistence/source-fact gaps go to
  `docs/adr/0053-local-detail-source-fact-parity.md`.

## Execution Rule

Execute one deferred item at a time. If the work changes schema, persistence,
matching, application query ownership, or public projection contracts, create
or revise an ADR before changing runtime code.
