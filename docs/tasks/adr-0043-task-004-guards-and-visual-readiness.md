# ADR 0043 Task 004: Toolbar Visual Readiness

Status: Complete - 2026-09-18.
Implementation recorded. Operator visual checks and fixture cleanup are accepted.

Light at normal width passed Ctrl+F,
Enter, the clear control, Search-button submission and toolbar layout.
Query `heycitizen` returned both Library and Index results.
Light at narrow width passed layout, compact Search and the detail toolbar check.
Dark at normal width passed keyboard search, clear/Search-button operation and layout.
Dark at narrow width passed layout, compact Search and the detail toolbar check.
The [review checklist](../reviews/adr-0043-review-checklist.md#operator-batches--2026-09-18) records the fixture and evidence.

## Scope After Reconciliation

Normal/narrow toolbar search was the surviving acceptance scope.
The implementation and earlier mechanical evidence are recorded in the
[ADR 0043 review checklist](../reviews/adr-0043-review-checklist.md).
Its retirement table identifies replaced requirements before listing survivors.
This packet no longer instructs an agent to rebuild the earlier screen design.

The 2026-09-10 governance pass changes documentation only. It records no new
visual pass and does not erase the earlier incident evidence.

## Owners And Constraints

- [ADR 0043](../adr/archive/0043-top-toolbar-global-search.md) owns the surviving contract.
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

Visual: the operator passed all four theme/width rows using a populated fixture.
The review records each result separately.

## Operator Visual Check

Follow [the current normal/narrow toolbar search procedure](../runbooks/inherited-ui-checks.md#search-toolbar--adr-0043-task-004),
including preparation and cleanup, for future regression checks.
This packet's visual checks are accepted. Other packets retain their own criteria.

## Closure

The operator confirmed cleanup on 2026-09-18 with:

```text
Removed fixture: /tmp/v4vmm-governance.jTg6NGQf
```

Task 004 is complete. ADR 0043 is Implemented.
The pending-check index no longer lists this packet.
This acceptance pass changed documentation only. No new runtime test result is claimed.
