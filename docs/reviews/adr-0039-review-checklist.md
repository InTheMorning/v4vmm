# ADR 0039 Review Checklist

## Status

Planning records prepared - 2026-09-18. Implementation review not started.
Numeric type proposal unratified; mechanical gates pending; twelve visual
inspections open. No implementation merge recommendation is made.

## Reviewed Artifacts

- [ADR 0039](../adr/0039-dynamic-type-ramp.md).
- [Phase plan](../plans/adr-0039-dynamic-type-ramp-phase-plan.md).
- [Task 001](../tasks/adr-0039-task-001-scale-domains-and-type-curves.md).
- [Task 002](../tasks/adr-0039-task-002-fixed-height-reserve-and-acceptance.md).
- [Operator procedure](../runbooks/dynamic-type-ramp-check.md).

## Numerical Decision

- [ ] Record the operator's numerical decision in ADR 0039, with date.
- [ ] Cover seven upward and downward endpoints and both intermediate steps.
- [ ] Record fixed-height capacity findings before landing the proposed values.

The policy is Accepted. None of these checkboxes implies the proposed table
is already ratified or that chrome coefficients are still undecided.

## Mechanical Review

| Packet | Evidence required | Result |
|---|---|---|
| 001 M1 | Five exact chrome coefficients and every chrome token/step output unchanged, including the pill exception | Pending |
| 001 M2 | 35 type outcomes, unchanged medium bases, monotonicity, role ordering and asymmetric growth/shrinkage | Pending |
| 001 M3 | Both resolvers used in live paths; situational ADR 0039 ownership guard | Pending |
| 001 M4 | Five persisted steps/default unchanged; installed environment path and discrete widget bridge verified | Pending |
| 002 M1 | Fixed row/card/bar capacity inventory and reservation bounds for all variants/steps; string length does not alter allocation | Pending |
| 002 M2 | Live consumers use the tested shared reservation; situational ADR 0039 guard | Pending |
| 002 M3 | Existing ADR 0034/0063 and list/drag tests retained; no schema, action or chrome drift | Pending |

Record consumer, roles, line count, line-height rule, maximum text extent and
available inner height in the implementation review. Record test names and
results; do not infer glyph fit from a numeric font size alone.

## Operator Visual Check

Run the [procedure](../runbooks/dynamic-type-ramp-check.md) after both packets.
Each cell requires an observation or screenshot reference, inspected revision,
viewport and result. The same row/track/popover is used throughout.

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

ADR 0039 becomes Implemented only when numerical ratification, both packets,
all twelve cells and cleanup are recorded. Reconcile packet Status lines,
phase plan, ADR index, delivery order and pending-human index together.
