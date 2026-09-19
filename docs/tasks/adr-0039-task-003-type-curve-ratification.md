# ADR 0039 Task 003: Type Curve Ratification

Status: Complete - 2026-09-18. Mechanical checks are Green.
The operator ratified the numeric decision, passed V1–V13 and confirmed fixture removal.
The agent inferred preservation and preference restoration from the conditional cleanup command.
The conversation does not contain the original inspection JSON.

The [review checklist](../reviews/adr-0039-review-checklist.md#operator-visual-check--task-003)
records the observations, source hashes, viewport report and evidence limits.
ADR 0039 is Implemented. No operator gate remains open in this packet.

## Goal

Ratify ADR 0039's numeric type proposal, land the per-role curves in the live
type resolver task 001 shaped, and walk the thirteen operator inspections. This
is the first ADR 0039 packet that changes type values.
Tasks 001/002 retain uniform type values. Task 002's ShowCard correction receives visual acceptance in this packet's V13.

## Files To Inspect

- `AGENTS.md`, `.github/copilot-instructions.md`, `docs/adr/README.md`.
- `docs/adr/0039-dynamic-type-ramp.md`,
  `docs/plans/adr-0039-dynamic-type-ramp-phase-plan.md`, task 001, task 002,
  `docs/plans/broadcast-chain-delivery-order.md`.
- `src/ui/tokens.rs`: the per-role type resolver task 001 shaped at identity.
- `src/ui/layouts.rs` and the fixed-height reservation task 002 added; confirm
  its reservation ceiling still covers the ratified numbers before claiming
  any visual cell.
- `src/ui/composites/show_card.rs`, `src/ui/shells/playlist.rs`,
  `src/ui/shells/queue_now_playing.rs`: task 002's single-line fix must stay
  intact under real non-medium type.
- `src/ui/composites/track_detail_surface.rs`, `src/ui/composites/playlist_popover.rs`.
- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`,
  `docs/adr/0063-show-dashboard-layout.md`,
  `docs/troubleshooting/column-text-truncation.md`.
- `tests/architecture_tests.rs`, `docs/runbooks/dynamic-type-ramp-check.md`,
  `docs/reviews/adr-0039-review-checklist.md`.

## Files Likely To Change

- `docs/adr/0039-dynamic-type-ramp.md`: record the operator's final numeric
  decision, with date, covering upward endpoints, downward endpoints and
  intermediate-step interpolation. This packet, the phase plan, review
  checklist, delivery order and pending-human index: status/evidence only.
- `src/ui/tokens.rs`: populate the per-role type resolver's existing shape
  with the ratified values; no change to its identity-delivery structure.
- `tests/architecture_tests.rs`: extend the situational ADR 0039 guard, or add
  one, proving the live resolver outputs the ratified values, not the
  identity placeholder.
- `docs/runbooks/dynamic-type-ramp-check.md`: this packet's procedure.

## Do Not Touch

Configuration formats/defaults, database/schema, playback/runtime behavior,
services, theme colors, chrome coefficients or base dimensions, list
paging/drag algorithms, font-role names, ADR 0063 or its guard. Do not edit
the dated 2026-09-10 reconciliation. Do not change the discrete third-party
widget-size mapping in `src/ui/sizable_bridge.rs`. Do not reopen task 002's
ShowCard fix beyond what re-verifying its reservation ceiling requires.

## Constraints

Work follows tasks 001/002 in a fresh session. They retain uniform type values and have no separate visual gate.
This packet opens the first visual gate in this ADR, including V13 for task 002's ShowCard correction.
Verify that task 002's reservation ceiling covers this proposal.
If the ratified values exceed that ceiling, report the measured mismatch.
Correct the reservation at its shared owner before introducing those values.

Present the concrete endpoints, intermediate results and task 002's
fixed-height capacity findings for the operator's numerical decision before
changing the live resolver's output. Do not copy the proposed table into
production as an accepted decision without that recorded ratification. No
screen-specific adjustment, chrome coefficient change, new persisted scale,
or additional accessibility tier.

## Implementation Steps

1. Record the operator's numeric decision in ADR 0039. The review must cover
   upward endpoints, downward endpoints and intermediate-step interpolation,
   and must explicitly weigh the operator's x-small-density preference
   against the proposal's downward half. Do not silently promote the
   proposal to policy.
2. Populate the type resolver's existing per-role shape with the ratified
   values. `.scaled(cx)` remains the shared rendering entry point; no screen
   computes a role multiplier.
3. Confirm task 002's reservation still holds for every fixed-height consumer
   under the ratified values, at all five steps. Correct the reservation at
   its shared owner if a variant is short; do not shrink a font, drop a line,
   or wrap a compact row to make it fit.
4. Add tests for all 35 ratified type outcomes.
   Check exact Medium bases, monotonic growth and role ordering at every step.
   Check that smaller roles grow more proportionally above Medium and shrink less proportionally below Medium.
   Compare each non-medium outcome with the former uniform result.
   Require differences for 26 outcomes and equality for Title at XS/Small.
   Extend the situational ADR 0039 guard to require the ratified values in the live resolver.
5. Run the checks.
   Rebuild the normal desktop binary.
   Give the operator the linked procedure.
   Record each of the thirteen visual checks separately.
   Correct defects at shared owners with regression proof.
   Repeat affected visual checks after a correction.
6. Keep ADR 0039 short of Implemented until numeric ratification, all mechanical checks, thirteen visual checks, preservation and cleanup pass.
   Update all current status records in the same change.

## Mechanical Acceptance Criteria

- M1: tests cover all 35 ratified type outcomes.
  They check exact Medium bases, monotonic role growth and role ordering at every step.
  They check that smaller roles grow more proportionally above Medium and shrink less proportionally below Medium.
  Of the 28 non-medium outcomes, 26 differ from the old uniform
  result. Title at x-small and at small are asserted as equal to it, because
  anchoring Title at today's 0.85 makes them so by construction. See the ADR's
  amended difference criterion.
- M2: the situational ADR 0039 guard proves the live type resolver outputs
  the ratified values recorded in the ADR, not the task 001 identity
  placeholder. Reverting to identity, or to any unratified value, must fail
  the guard.
- M3: task 002's reservation capacity tests stay Green against the ratified
  values at all five steps, for every fixed-height consumer, including
  ShowCard. A reservation shortfall is reported and corrected at its shared
  owner, not absorbed by shrinking type or wrapping a compact row.
- M4: existing chrome, configuration, size-bridge and ADR 0034/0063 tests
  remain Green; review proves no new `UiScale` variant/string/key/default,
  discrete widget mapping, or chrome coefficient change.

M1–M2 are the new ratification requirements; M3–M4 prove integration and
retain existing requirements. Mechanical evidence does not prove legibility
or glyph clipping by itself.

## Visual Acceptance

Accepted - 2026-09-18. V1–V13 passed. The review records inferred preservation
and confirmed fixture removal. Three surfaces at XS/XL in both themes produce
twelve type checks. V13 checks ShowCard summaries. Task 003 owns the only
visual gate in ADR 0039.

1. Compact row: a track row in Music's `Startup fixture playlist` detail.
2. Detail page: the Music track detail opened from that same row.
3. Popover: Add to Playlist on that track detail, including its New Playlist
   input mode, without submitting a change.
4. Show cards, V13: each summary renders as one clipped line, with no ellipsis and no second row.
   Folded in on 2026-09-18 as the
   visual proof owed for task 002's user-visible single-line fix. It is
   scale-independent, so one observation closes it.

Use the [operator procedure](../runbooks/dynamic-type-ramp-check.md); record
the same viewport/data, a medium reference, readable small text, uncropped
glyphs, fixed row lines, accessible controls, detail/popover wrapping and
cleanup. All thirteen cells are listed in the
[review checklist](../reviews/adr-0039-review-checklist.md). Do not add a
chrome-density re-walk while chrome coefficients remain unchanged.

Operator evidence — 2026-09-18: the operator passed the prepared playlist
row, detail and popover at XS/XL in Light/Dark, with Medium references, and
ShowCard's single-line summaries. The
[review checklist](../reviews/adr-0039-review-checklist.md#operator-visual-check--task-003)
records each response, base revision `b5349f8`, matching working-tree source
hashes and the reported `maximized 1440x900` / `~half width` viewport coverage.
Exact pane geometry was not recorded. Starting preferences were Dark/Medium.

The operator confirmed the inspected fixture `/tmp/v4vmm-startup-1nmjz81u`
was already removed. The agent inferred preservation and preference restoration from the supplied command.
That command permits cleanup only after successful inspection.
The inference assumes that the operator used that command.

The conversation does not contain the original inspection JSON. The subsequent missing-path error
was not a failed preservation result. The unrelated located fixture was left
untouched. All acceptance for this packet is complete.

## Test Commands

```bash
cargo fmt -- --check
cargo check --locked --offline
cargo test --locked --offline adr_0039_
cargo test --locked --offline ui::tokens::tests
cargo test --locked --offline ui::shells::playlist::tests
cargo test --locked --offline ui::composites::show_card::tests
cargo test --locked --offline --test architecture_tests
cargo clippy --locked --offline -- -D warnings
cargo build --locked --offline --bin v4vmm
git diff --check
```

Name new ratification tests with `adr_0039_`; confirm the filter executes
them. Run additional affected consumer tests if a variant's behavior changes.

## Rollback And Escalation

Revert this packet coherently; no configuration migration is required.
Return to the ADR for a capacity conflict, a request to change chrome, or a
need to alter the discrete size bridge. If ratified numbers do not fit an
existing reservation, report the measured bounds and revise the proposal;
chrome retuning requires a separately approved packet with its own density
inspection. An unwalked visual cell stays open.

## Expected Final Report

List files changed, the recorded numerical decision and its evidence, checks
(Green or error), behavior changed, visual cell evidence, deviations and
unresolved concerns. Keep the Status line, phase plan, review checklist,
delivery row and pending-human entry consistent. End with
`Operator visual check`, linking the procedure. Do not run the app.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet's Files To Inspect and Files Likely To Change.
- `docs/adr/0039-dynamic-type-ramp.md`, its phase plan, task 001 and task 002.

Goal:
- Ratify the numeric proposal, land it in the live type resolver, and walk
  the thirteen operator inspections.

Constraints:
- Meet M1–M4 above; preserve exact chrome output and all five persisted
  steps.
- Obtain the numeric decision, recorded in the ADR, before landing proposed
  type values.
- Re-verify task 002's reservation ceiling against the final ratified values.

Do not touch:
- Everything in this packet's Do Not Touch section.

Acceptance criteria:
- M1–M4 Green. Thirteen separately recorded operator results and cleanup.

Test commands:
- Run this packet's Test Commands in order, then the applicable consumer
  tests.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
