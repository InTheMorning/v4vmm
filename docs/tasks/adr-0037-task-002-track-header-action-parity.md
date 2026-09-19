# ADR 0037 Task 002: Track Header and Action Parity

Status: Accepted - 2026-09-10.
Implementation recorded; operator visual acceptance remains open.

The operator confirmed the private database/audio copy at
`/tmp/v4vmm-governance.ie6k8TQf` on 2026-09-19. Track selection and source-fact
checks remain open. The first candidate query returned `[]`. A diagnostic query
confirmed 79 library tracks with feed GUIDs and copied audio, no stored track
Website facts and nine tracks with Nostr facts. All 79 scoped Index lookups
succeeded. The check counted no Website or Nostr candidates. A `website`-only
check can miss the Index's `web_page` links. The Index returned empty track
identity lists for MoeFactz. Its parent-feed facts do not qualify as track
facts. A follow-up request confirmed three contributor claims, including
HeyCitizen's Nostr key and the Moe Factz host website. These facts belong to
the contributors. Source inspection found app request and presentation gaps
for those facts. The [review](../reviews/adr-0037-review-checklist.md#task-002-contributor-source-check--2026-09-19)
records the evidence. No application code changed. No visual check or fixture
cleanup is accepted yet.

## Scope After Reconciliation

Track identity and detail parity is the remaining acceptance scope.
The implementation and earlier mechanical evidence are recorded in the
[ADR 0037 review checklist](../reviews/adr-0037-review-checklist.md).
Its retirement table identifies replaced requirements before listing survivors.
This packet no longer instructs an agent to rebuild the earlier screen design.

The 2026-09-10 governance pass changes documentation only. It records no new
visual pass and does not erase the earlier incident evidence.

## Owners And Constraints

- [ADR 0037](../adr/0037-same-entity-surface-parity.md) owns the surviving contract.
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

Follow [the current track identity and detail parity procedure](../runbooks/inherited-ui-checks.md#identity-and-detail-parity--adr-0037-tasks-001-and-002),
including preparation and cleanup. Record the fixture, entry route, theme,
result, and any screenshots in the review checklist. Close only this packet's
criteria, even when one walkthrough also supplies another packet's evidence.

## Closure

After operator acceptance and cleanup, update this Status, the review checklist,
the owning ADR when all its gates are closed, and the delivery row. Remove the
corresponding entry from pending human checks in the same change.

No runtime work or visual acceptance is claimed by this reconciliation.
