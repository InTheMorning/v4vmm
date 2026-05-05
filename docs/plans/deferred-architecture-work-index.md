# Deferred Architecture Work Index

## Status

Active index - 2026-05-01.

## Purpose

Keep completed ADRs closed while making the remaining deferred work visible,
prioritized, and routed to the right governance artifact.

## Priority Order

1. Track-to-artist binding for name-derived Library artist views.
   - Status: deferred from ADR 0029.
   - Route: future ADR before adding `tracks` artist-subject bindings,
     source-priority rules, or conflict surfacing.
2. Person/global identity persistence.
   - Status: deferred from ADR 0029.
   - Route: future ADR only after durable person ids and merge policy exist.
3. Library/Discover data parity for release date, language, explicit state,
   description, and related local detail fields.
   - Status: needs triage after ADR 0028 contributor visibility.
   - Route: ADR 0024 query/read-model work if the fix is loading shape;
     source-fact ADR work if the fix is persistence.
4. ADR 0024 query/service thinning for remote-only Discover reads and remote
   inspector lazy panels.
   - Status: deferred from ADR 0024 final review.
   - Route: new ADR 0024 follow-up plan and bounded vertical slices.
5. Staged metadata durability.
   - Status: product/storage decision required.
   - Route: future ADR before schema or command behavior changes.
6. Non-URL artwork rendering.
   - Status: audit completed; no producer/resolver contract yet.
   - Route: future ADR only when cache, storage, or public artwork contracts
     change.
7. Playback volume and playback-driver supervision.
   - Status: isolated playback-boundary follow-up.
   - Route: ADR 0021/0024 follow-up after the driver contract is clear.
8. Visual-system polish and lower-priority product improvements.
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

## Execution Rule

Execute one deferred item at a time. If the work changes schema, persistence,
matching, application query ownership, or public projection contracts, create
or revise an ADR before changing runtime code.
