# Deferred Architecture Work Index

## Status

Active index - 2026-05-18.

## Purpose

Keep completed ADRs closed while making the remaining deferred work visible,
prioritized, and routed to the right governance artifact.

## Priority Order

1. Person/global identity persistence.
   - Status: deferred from ADR 0029.
   - Route: future ADR only after durable person ids and merge policy exist.
2. Screen-local `cx.spawn` retirement.
   - Status: pinned at current baseline by
     `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
     (added by ADR 0040 Task 004).
   - Route: ADR 0040 follow-up; migrate each remaining spawn to a
     presentation-bridge.
3. Staged metadata durability.
   - Status: product/storage decision required.
   - Route: future ADR before schema or command behavior changes.
4. Non-URL artwork rendering.
   - Status: audit completed; no producer/resolver contract yet.
   - Route: future ADR only when cache, storage, or public artwork contracts
     change.
5. Playback volume and playback-driver supervision.
   - Status: isolated playback-boundary follow-up.
   - Route: ADR 0021/0024 follow-up after the driver contract is clear.
6. Visual-system polish and lower-priority product improvements.
   - Status: use bounded ADR 0025 tasks only when the change affects tokens,
     primitives, composites, or theme contracts.
   - HIG product-completeness gaps are tracked separately in
     `docs/plans/hig-product-polish-backlog.md`. They are not permission to
     reopen completed search/sidebar restructuring.

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
- ADR 0040 legacy synchronous scheduling retirement completed on
  2026-05-18 via Tasks 001-004. The GPUI-coupled command runner,
  `--no-default-features` desktop compatibility path, and async-runtime
  Cargo feature are retired. Guards
  `gpui_command_runner_is_retired`,
  `async_runtime_feature_flag_is_retired`, and
  `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime` prevent
  retired-surface regression and pin unrelated screen-spawn debt until a
  future ADR 0040 cleanup.
- ADR 0042 layer-consolidation status was reconciled on 2026-05-08.
  The confirmed single-use composites were inlined, the retained
  composites have real multi-site use, and the remaining index items
  below still require their named future ADR routes.
- ADR 0045 planning artifacts were created on 2026-05-08 for the top
  deferred item: track-to-artist binding for name-derived Library artist
  views. Tasks 001-004 completed on 2026-05-11; name-derived Library artist
  views now enrich from explicit bindings without name-only merging.
- Deferred item #2, Library/Discover data parity, completed on 2026-05-18.
  ADR 0052 routed the work; ADR 0024 follow-up runtime delivery shipped the six
  loading-shape slices (`6e61d4f`, `f9bff8d`, `d7d0220`, `e8c1aaa`,
  `8f701d2`, `de934bb`); ADR 0053 accepted the parent source-fact parity
  contract; ADR 0054 implemented the concrete feed/track metadata source-fact
  slice.

## Execution Rule

Execute one deferred item at a time. If the work changes schema, persistence,
matching, application query ownership, or public projection contracts, create
or revise an ADR before changing runtime code.

## Status Hygiene Rule

Any change that completes an ADR phase, task packet, review remediation, or
deferred-index item must reconcile status in the same commit:

- the governing ADR status,
- the phase plan or task packet status,
- the review checklist or smoke checklist,
- the deferred-work index entry when applicable.

Do not leave `Proposed`, `Implementation pending`, unchecked checklist boxes, or
active deferred-index entries behind for work that has shipped and been
verified. If a runtime slice ships but visual proof remains operator-owned, mark
the runtime status complete and name the visual gate explicitly instead of
leaving the whole artifact ambiguous.
