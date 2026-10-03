# ADR 0037 Task 001: Feed Identity Action Parity

Status: Complete - 2026-09-19.
Implementation recorded. Operator visual acceptance and fixture cleanup are
confirmed. Task 002 retains its separate gate. ADR 0037 remains Accepted.

Fixture preparation is complete on 2026-09-19. The operator's read-only query
confirmed stored Website, Nostr and RSS facts for `The Heycitizen Experience`,
local feed `2`. The [fixture reference](../../reviews/adr-0037-review-checklist.md#task-001-fixture-reference--2026-09-19)
records the GUID, values and sources. The operator confirmed fixture removal
on 2026-09-19.

The operator accepted Light local-feed control visibility and readability,
Nostr copy, Website and RSS actions on 2026-09-19. The operator also accepted
Light Index-feed control visibility/readability, Nostr copy, Website and RSS
actions for the same feed. Dark local-feed control visibility/readability,
Nostr copy, Website and RSS actions also passed. Dark Index-feed control
visibility/readability, Nostr copy, Website and RSS actions passed.
All visual checks and fixture cleanup are accepted.

The earlier fixture launch selected Chromium while the desktop selected Firefox.
The corrected launch restores desktop lookup paths while retaining private app
directories. The operator accepted the corrected launch and Firefox opening
the expected Website and RSS URLs. The review records these passes and the
handler diagnosis.

## Scope After Reconciliation

Feed identity and hydration was this packet's acceptance scope.
The implementation and earlier mechanical evidence are recorded in the
[ADR 0037 review checklist](../../reviews/adr-0037-review-checklist.md).
Its retirement table identifies replaced requirements before listing survivors.
This packet no longer instructs an agent to rebuild the earlier screen design.

The 2026-09-10 governance pass changes documentation only. It records no new
visual pass and does not erase the earlier incident evidence.

## Owners And Constraints

- [ADR 0037](../../adr/0037-same-entity-surface-parity.md) owns the surviving contract.
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

Accepted - 2026-09-19. This procedure remains a regression check.

Follow [the current feed identity and hydration procedure](../../runbooks/inherited-ui-checks.md#identity-and-detail-parity--adr-0037-tasks-001-and-002),
including preparation and cleanup. Record the fixture, entry route, theme,
result, and any screenshots in the review checklist. Close only this packet's
criteria, even when one walkthrough also supplies another packet's evidence.

## Closure

The operator accepted both entry routes in Light and Dark and confirmed removal
of `/tmp/v4vmm-governance.JN95oN81`. This packet has no remaining gate.
The review, ADR status, phase plan and delivery row record closure. The pending
index retains task 002. No runtime code changed during this acceptance pass.
