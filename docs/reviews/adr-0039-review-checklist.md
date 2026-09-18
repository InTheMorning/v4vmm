# ADR 0039 Review Checklist

## Status

Planning records prepared - 2026-09-18. Amended the same day: three packets,
not two. Task 001 ships both live resolvers at identity; task 002 adds
fixed-height reservation and ShowCard's single-line fix; neither has a visual
gate. Task 003 ratifies the numeric proposal, lands the per-role curves and
owns the twelve visual inspections. Implementation review not started.
Numeric type proposal unratified; mechanical gates pending; twelve visual
inspections open. No implementation merge recommendation is made.

## Reviewed Artifacts

- [ADR 0039](../adr/0039-dynamic-type-ramp.md).
- [Phase plan](../plans/adr-0039-dynamic-type-ramp-phase-plan.md).
- [Task 001](../tasks/adr-0039-task-001-scale-domains-and-type-curves.md).
- [Task 002](../tasks/adr-0039-task-002-fixed-height-reserve-and-acceptance.md).
- [Task 003](../tasks/adr-0039-task-003-type-curve-ratification.md).
- [Operator procedure](../runbooks/dynamic-type-ramp-check.md).

## Numerical Decision — Task 003

- [ ] Record the operator's numerical decision in ADR 0039, with date.
- [ ] Cover seven upward and downward endpoints and both intermediate steps.
- [ ] Weigh the operator's x-small-density preference against the proposal's
      downward half before ratifying.
- [ ] Record task 002's fixed-height capacity findings, and re-verify them
      against the final ratified values, before landing them.

The policy is Accepted. None of these checkboxes implies the proposed table
is already ratified or that chrome coefficients are still undecided. This
numerical decision belongs to task 003; tasks 001 and 002 land no proposed
value.

## Mechanical Review

| Packet | Evidence required | Result |
|---|---|---|
| 001 M1 | Five exact chrome coefficients and every chrome token/step output unchanged, including the pill exception | Pending |
| 001 M2 | Type resolver output bit-identical to the old uniform result, for all seven roles at all five steps — identity, not divergence | Pending |
| 001 M3 | Both resolvers used in live paths; situational ADR 0039 ownership guard | Pending |
| 001 M4 | Five persisted steps/default unchanged; installed environment path and discrete widget bridge verified | Pending |
| 002 M1 | Fixed row/card/bar capacity inventory and reservation bounds for all variants/steps, sized against the unratified proposal; string length does not alter allocation | Pending |
| 002 M2 | Live consumers use the tested shared reservation; situational ADR 0039 guard | Pending |
| 002 M3 | Existing ADR 0034/0063 guards — including `adr_0063_show_card_grid_shell_uses_vm_contract` — and list/drag tests retained; no schema, action or chrome drift | Pending |
| 002 M4 | ShowCard's `render_summary_line` matches the playlist/now-playing single-line pattern; architecture guard proves it | Pending |
| 003 M1 | 35 ratified type outcomes, unchanged medium bases, monotonicity, role ordering and asymmetric growth/shrinkage | Pending |
| 003 M2 | Live type resolver outputs the ratified values, not task 001's identity placeholder; situational ADR 0039 guard | Pending |
| 003 M3 | Task 002's reservation capacity tests stay Green against the ratified values at all five steps | Pending |
| 003 M4 | Existing chrome, configuration, size-bridge and ADR 0034/0063 tests remain Green; no new persisted scale or chrome coefficient change | Pending |

Record consumer, roles, line count, line-height rule, maximum text extent and
available inner height in the implementation review. Record test names and
results; do not infer glyph fit from a numeric font size alone.

## Operator Visual Check — Task 003

Run the [procedure](../runbooks/dynamic-type-ramp-check.md) after all three
packets; the first two land no presentation to inspect. Each cell requires an
observation or screenshot reference, inspected revision, viewport and result.
The same row/track/popover is used throughout.

| ID | Surface | Scale | Theme | Result / evidence |
|---|---|---|---|---|
| V1 | Music compact playlist track row | x-small | Light | Open |
| V2 | Music compact playlist track row | x-small | Dark | Open |
| V3 | Music compact playlist track row | x-large | Light | Open |
| V4 | Music compact playlist track row | x-large | Dark | Open |
| V5 | Music track detail page | x-small | Light | Open |
| V6 | Music track detail page | x-small | Dark | Open |
| V7 | Music track detail page | x-large | Light | Open |
| V8 | Music track detail page | x-large | Dark | Open |
| V9 | Track detail Add to Playlist popover | x-small | Light | Open |
| V10 | Track detail Add to Playlist popover | x-small | Dark | Open |
| V11 | Track detail Add to Playlist popover | x-large | Light | Open |
| V12 | Track detail Add to Playlist popover | x-large | Dark | Open |

- [ ] Starting theme/scale restored; unrelated configuration preserved.
- [ ] Fixture library/audio/bindings/migrations preserved; no playlist submitted.
- [ ] New fixture cleanup confirmed; earlier retained fixtures untouched.

These are type/reservation checks. Chrome coefficient changes would require a
future density packet; unchanged chrome creates no additional density gate.

## Review Outcome

Complete after implementation: Pass/Fail, required fixes, optional improvements,
architectural drift, missing mechanical/visual proof, merge recommendation and
whether the next packet needs adjustment. Review against the final diff.

ADR 0039 becomes Implemented only when numerical ratification, all three
packets, all twelve cells and cleanup are recorded. Reconcile packet Status
lines, phase plan, ADR index, delivery order and pending-human index
together.
