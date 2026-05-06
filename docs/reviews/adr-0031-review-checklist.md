# ADR 0031 Review Checklist

## Reviewed Artifact

- ADR: `docs/adr/0031-release-detail-presentation-contract.md`
- Plan: `docs/plans/adr-0031-release-detail-presentation-contract-phase-plan.md`
- Tasks:
  - `docs/tasks/adr-0031-task-001-contract-types-and-projection-tests.md`
  - `docs/tasks/adr-0031-task-002-renderer-adoption.md`
  - `docs/tasks/adr-0031-task-003-track-section-parity.md`
  - `docs/tasks/adr-0031-task-004-visual-smoke-and-cleanup.md`

## Required Checks

- Does the hero contain only human-readable identity?
- Are raw URLs, `npub` values, GUIDs, and long machine IDs absent from the hero?
- Are Website, Nostr, and RSS identity affordances outside `primary_actions`?
- Are identity detail rows rendered below summary/action areas?
- Does the description render exactly once?
- Are summary facts capped and ordered?
- Do Library and Discovery share the same page skeleton?
- Are differences limited to surface policy and action availability?
- Are command dispatch and services still screen-owned?
- Are projection tests GPUI-free?
- Does manual smoke show the track section starting in the first viewport when
  content exists?

## Architectural Drift Checks

- No GPUI, UI, screen, DB, API-client, or service imports in
  `src/view_models/entity_detail.rs`.
- No service or command dispatch moved into `src/ui_entity.rs`.
- No schema migration or metadata persistence change.
- No source-fact inference introduced.
- No parallel release-detail system created.
- No broad `ReleaseDetailSlots` fields remain that can carry hero,
  description, summary, or panel content.
- No nested vertical scroll views introduced.

## Merge Recommendation Template

Pass/fail:

Required fixes:

Optional improvements:

Architectural drift:

Missing tests:

Merge recommendation:

Next task adjustment:
