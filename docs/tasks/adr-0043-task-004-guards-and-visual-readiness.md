# ADR 0043 Task 004: Toolbar Visual Readiness

Status: Accepted - 2026-09-10.
Implementation recorded; operator visual acceptance remains open.

## Scope After Reconciliation

Normal/narrow toolbar search is the remaining acceptance scope.
The implementation and earlier mechanical evidence are recorded in the
[ADR 0043 review checklist](../reviews/adr-0043-review-checklist.md).
Its retirement table identifies replaced requirements before listing survivors.
This packet no longer instructs an agent to rebuild the earlier screen design.

The 2026-09-10 governance pass changes documentation only. It records no new
visual pass and does not erase the earlier incident evidence.

## Owners And Constraints

- [ADR 0043](../adr/0043-top-toolbar-global-search.md) owns the surviving contract.
- The review checklist names the current shared owners and existing guards.
- ADRs 0047/0048/0060 own shared Music surfaces and frame navigation.
- An agent must not run the app. A person performs the checks below.
- Do not restore a retired screen, toolbar player, global scope enum, or
  inspector-local return control to satisfy an old instruction.
- A failure requires a bounded fix at its shared owner, relevant mechanical
  checks, and another operator inspection of the failed case.

## Acceptance Criteria

Mechanical: existing ownership guards cited in the review remain applicable.
Their recorded implementation results are historical evidence, not a claim
that a new suite was run during reconciliation.

Visual: complete the matching checklist rows in both Light and Dark using
the required populated fixture. An unavailable fixture leaves that row open.

## Operator Visual Check

Follow [the current normal/narrow toolbar search procedure](../runbooks/inherited-ui-checks.md#search-toolbar--adr-0043-task-004),
including preparation and cleanup. Record the fixture, entry route, theme,
result, and any screenshots in the review checklist. Close only this packet's
criteria, even when one walkthrough also supplies another packet's evidence.

## Closure

After operator acceptance and cleanup, update this Status, the review checklist,
the owning ADR when all its gates are closed, and the delivery row. Remove the
corresponding entry from pending human checks in the same change.

No runtime work or visual acceptance is claimed by this reconciliation.
