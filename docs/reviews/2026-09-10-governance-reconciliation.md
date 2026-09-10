# Governance Reconciliation — 2026-09-10

## Status

Documentation reconciliation complete. Inherited visual acceptance remains
open for the five groups below. No app behavior changed and no new operator
pass was recorded.

## Scope And Method

Read the status sections of all 63 current numbered ADRs and gate statements
in all 26 review checklists. Search review prose for pending, outstanding, awaiting,
unverified, missing proof, manual verification, and conditional merge language.
Follow each candidate into its task review, later acceptance evidence, and
superseding ADR. Checkbox syntax alone is insufficient: ADR 0037 had no empty
boxes, and ADR 0024's old unchecked template coexists with a final passed review.

Classify each candidate as a surviving check, a retired requirement, recorded
acceptance, deferred implementation, or a reusable/future review instruction.
Only surviving unverified requirements enter pending human checks. A source
guard proves its named ownership contract; it does not prove visual behavior.

## Surviving Checks

| ADR / packets | Why the check survives | Disposition |
|---|---|---|
| 0030 / 006 | The scroll review expressly leaves manual verification residual; bounded scrolling still applies to Music details and Settings | Retire Discovery/Recent Feeds paths under 0047/0048/0060/0062; retain current scroll check |
| 0037 / 001 and 002 | Feed-identity hydration recheck and populated track-identity proof were never recorded as passed | Compare local/Index origins in Music; retire separate screen and old helper-file requirements |
| 0043 / 004 | Normal/narrow Light/Dark toolbar readability remains unverified | Retire the player, global scopes, Search workspace, and Recent Feeds root; retain input/focus/submission/readability |
| 0044 / 003 | Final handle/menu/insertion and mounted-row update inspection remains open | Retain reorder contract; replace inspector-owned return controls with frame navigation |
| 0054 / 004 and 005 | Both task reviews retain visual verification; task 006 is guard-only and provides no closure | Correct ADR and phase plan from Implemented to Accepted, with mechanical implementation recorded and visual gates open |

Each current checklist names the superseding ADR before listing survivors.
[Pending human checks](../pending-human-checks.md) groups seven task gates
under these five ADRs. The [operator runbook](../runbooks/inherited-ui-checks.md)
names current entry routes, fixture requirements, failure observations, and
cleanup. A shared walkthrough may supply multiple results, but acceptance is
recorded per packet and theme.

## Checklist Sweep Dispositions

This inventory covers every review checklist present at the start of the pass.
It is a review record, not another live gate index.

| Checklist | Gate disposition |
|---|---|
| active-frame-search-dispatch-review-checklist | Explicitly superseded by ADR 0048; do not relist old search structures |
| adr-0023-review-checklist | Finalization recorded in adr-0023-final-implementation-review and the migration plan; no outstanding gate statement found |
| adr-0024-review-checklist | Final review passed; unchecked reusable items do not override that recorded disposition |
| adr-0025-review-checklist | Manual theme/UI passes recorded; remaining implementation phases are not an open operator check |
| adr-0026-review-checklist | Post-ADR 0026 visual-smoke review passed with follow-ups routed to later owners |
| adr-0027-review-checklist | Task 005 final visual-smoke review records acceptance |
| adr-0029-review-checklist | Runtime scope closed; person/global identity is deferred implementation |
| adr-0030-review-checklist | Task 006 residual scroll check survives; separate recents paths are retired |
| adr-0031-review-checklist | Visual-smoke review explicitly accepted residual fixture gaps; do not turn that qualified pass into new acceptance |
| adr-0032-review-checklist | Popover task 001 records visual proof; future-change guidance is not an outstanding gate |
| adr-0034-review-checklist | Proceed, with tasks complete |
| adr-0035-review-checklist | Proceed with recorded user screenshots |
| adr-0036-review-checklist | Tasks 001-003 complete with recorded user screenshots |
| adr-0037-review-checklist | Two surviving task checks restored |
| adr-0038-review-checklist | Completed; operator-navigated visual verification explicitly recorded |
| adr-0043-review-checklist | Surviving toolbar check restored after retirement of replaced requirements |
| adr-0044-review-checklist | Surviving playlist check restored after navigation reconciliation |
| adr-0045-review-checklist | Completed; optional future smoke is not an open gate |
| adr-0046-review-checklist | Visible command deferral accepted; model-only work has no visual criterion |
| adr-0047-review-checklist | Operator closed remaining visual smoke on 2026-05-18; ADR 0048 owns current search routing |
| adr-0051-review-checklist | Mechanical persistence checklist; no outstanding operator gate statement |
| adr-0054-review-checklist | Task-review prose reveals two unclosed hydration checks; restored |
| deferred-work-integration-review-checklist | Completed documentation gate |
| inspector-source-ownership-review-checklist | Operator visual smoke passed 2026-05-18 |
| library-discover-parity-triage-review-checklist | ADR 0052 triage completed; implementation/hydration follow-ups have separate owners |
| one-owner-per-surface-review-checklist | Proceed with recorded evidence; ADR 0038 absorbed the planning artifact |

Earlier task reviews that say a later task must perform visual proof are read
with that later task's result. Hypothetical future-smoke recommendations and
historical agent display failures do not create additional gates. The explicit
0030/0054 residuals have no later closure evidence in the reviewed corpus.

## Other Reconciliations

- AGENTS.md now names the inherited open checks instead of claiming repository-
  wide closure. Its gate-audit method requires reading prose and resolving
  supersession before indexing checks.
- ADR 0039 remains Proposed and explicitly unscheduled. Text-scale tiers,
  maximum scale, wrapping, and truncation policy are unspecified. The operator
  has not withdrawn the requirement.
- ADRs 0059 and 0063 remain Implemented for their accepted scope. Task 017's
  fixture cleanup and acceptance remain closed. Their no-open-check language
  now names that scope.
- The delivery plan records the operator's chain-first order, the real-show
  trigger for publisher show-log task 001, relay adoption, and explicit 004/005
  follow-through. The config ADR is next; this pass does not decide recovery
  behavior for each startup failure or implement it.
- Playback scope is resolved in the narrow-layout proposal: hide the bar when
  all typed actions are unavailable and retain working controls. Its ADR 0063
  amendment and implementation packet remain scheduled work.
- A11 precedes A12's scroll-anchor design. A12 and UTC are designed together
  and delivered in bounded packets. Wider Clippy cleanup and Show view-model
  decomposition remain unscheduled.

## Verification

Green on 2026-09-10:

- 269 local links and anchors, including incoming links to changed files.
- Seven open packet statuses agree with five pending groups and delivery rows;
  ADRs 0059/0063 remain Implemented for their accepted scope.
- Every guard symbol cited by the three rewritten checklists exists.
- All operator shell blocks and embedded Python parse.
- Synthetic config/database/audio preparation preserves the original bytes
  after fixture writes, dereferences copied audio symlinks, and rejects an
  invalid cleanup directory.
- `git diff --check`.

No Rust source changed, so no Rust build or full test run is claimed for this
pass. Existing mechanical evidence remains dated in the original reviews.

## Operator Visual Check

This documentation change has no new visual criterion. The five inherited
groups remain open. Use the [runbook](../runbooks/inherited-ui-checks.md) one
check at a time when that acceptance work is selected; it does not interrupt
the approved next phase of configuration failure behavior.
