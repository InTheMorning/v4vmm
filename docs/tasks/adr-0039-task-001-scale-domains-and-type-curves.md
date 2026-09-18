# ADR 0039 Task 001: Scale Domains And Type Curves

Status: Scheduled - 2026-09-18. Implementation not started; numeric type
proposal unratified; shared twelve-cell visual gate open under task 002.

## Goal

Wire separate live chrome and type resolvers. Preserve existing chrome bits,
medium font bases and configuration while changing non-medium font results
according to a numerically ratified ADR 0039 ramp.

## Files To Inspect

- `AGENTS.md`, `.github/copilot-instructions.md`, `docs/adr/README.md`.
- `docs/adr/0039-dynamic-type-ramp.md`,
  `docs/plans/adr-0039-dynamic-type-ramp-phase-plan.md`,
  `docs/plans/broadcast-chain-delivery-order.md`.
- `src/ui/tokens.rs`, `src/config.rs`, `src/app/settings.rs`,
  `src/view_models/settings.rs` and `src/ui/composites/settings.rs`.
- All geometry consumers named in the following section, plus
  `src/ui/primitives/label.rs`, `src/ui/primitives/multiline_text.rs`,
  `src/ui/primitives/button.rs`, `tests/architecture_tests.rs`.
- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`,
  `docs/adr/0063-show-dashboard-layout.md`,
  `docs/troubleshooting/column-text-truncation.md`.

## Files Likely To Change

- `src/ui/tokens.rs`: pure type/chrome resolution and value tests.
- `src/ui/layouts.rs`, `src/ui/icons.rs`, `src/ui/sizable_bridge.rs`,
  `src/ui/primitives/image.rs`, `src/ui/composites/thumbnail.rs`.
- `src/ui/composites/detail_grid.rs`, `src/ui/composites/log_frame.rs`,
  `src/ui/composites/show_log_pane.rs`,
  `src/ui/composites/maintenance_page.rs`, `src/app/show.rs`: existing direct
  multiplier calls become chrome calls, with no change to their calculations.
- `tests/architecture_tests.rs`: situational ADR 0039 domain-ownership guard.
- ADR 0039, this packet, the phase plan and review checklist, delivery order,
  and pending-human index: numerical decision and status/evidence only.

## Do Not Touch

Configuration formats/defaults, database/schema, playback/runtime behavior,
services, theme colors, chrome base dimensions, list heights, row reservations,
font-role names, ADR 0063 or its guard. Task 002 owns reservation work.
Do not edit the dated 2026-09-10 reconciliation. Do not change the discrete
third-party widget-size mapping in `src/ui/sizable_bridge.rs`.

## Constraints

The ADR's type table is a proposal. Acceptance of the architecture is not
ratification of those numbers. Present the concrete endpoints, intermediate
results and existing fixed-height capacity findings for the numerical decision
before landing type changes. No screen-specific adjustment, unused chrome
seam, new persisted scale or speculative compatibility helper.

Keep `Radius::Full` unscaled. Preserve the arithmetic order and float values
of existing chrome calculations. The pixel-size branch of `SizableScaled`
uses chrome; its discrete tier branch keeps its current behavior.

## Implementation Steps

1. Inspect current callers with `rg -n 'multiplier\(\)|ScaleFactor|FontSize'`
   in the listed files. Classify direct multiplier calls as geometry. Audit
   compact row/control capacity before the numerical decision; report any
   conflict rather than quietly changing chrome.
2. Record the operator's numeric decision in ADR 0039. The review must cover
   upward endpoints, downward endpoints and intermediate-step interpolation.
   Do not copy a proposed table into production as an accepted decision.
3. Introduce a named chrome curve and a pure per-role type resolver in
   `src/ui/tokens.rs`. Keep `.scaled(cx)` as the shared rendering entry point.
   Route `FontSize` to type; route `Spacing`, `Radius` and `Size` to chrome.
4. Migrate the listed geometry consumers to chrome without changing their
   output or discrete mappings. Remove the old multiplier if no live caller
   needs it; any retained compatibility alias delegates to chrome.
5. Add value tests and a situational ADR 0039 guard for live domain ownership.
   Preserve existing ADR 0034 and ADR 0063 guards. Keep constants in one owner.
6. Run the checks below. Record a mechanical handoff, the numerical decision,
   and the open shared visual gate. End the session before task 002 starts.

## Mechanical Acceptance Criteria

- M1: a five-step pure value test compares chrome coefficients with the exact
  former `f32` values using bit equality, not rounded display strings.
  Resolved `Spacing`, `Radius` and `Size` tests compare every variant/step with
  the old arithmetic, including the pill exception; no chrome base changes.
- M2: tests cover all 35 ratified type outcomes. They prove exact medium base
  preservation, change from the old uniform result at each non-medium step,
  monotonic role growth, role ordering at every step, greater proportional
  growth for smaller roles above medium and smaller proportional loss below it.
- M3: the situational ADR 0039 guard proves `FontSize::scaled` uses type while
  chrome tokens and direct geometry bridges use chrome. Removing either live
  connection must fail; defining unused helpers must not satisfy the guard.
- M4: existing configuration parsing/default and size-bridge tests remain
  Green; review proves no new `UiScale` variant/string/key/default or discrete
  widget mapping. Test the environment-backed resolver path, not only a table.

## Visual Acceptance

Open. [Task 002](adr-0039-task-002-fixed-height-reserve-and-acceptance.md) owns
the shared twelve inspections. This packet's mechanical handoff allows that
packet in a fresh session; it does not claim that presentation is accepted.
There is no additional chrome-density inspection for unchanged coefficients.

## Test Commands

```bash
cargo fmt -- --check
cargo check --locked --offline
cargo test --locked --offline adr_0039_
cargo test --locked --offline ui::tokens::tests
cargo test --locked --offline ui::sizable_bridge::tests
cargo test --locked --offline config::tests
cargo test --locked --offline --test architecture_tests
cargo clippy --locked --offline -- -D warnings
cargo build --locked --offline --bin v4vmm
git diff --check
```

Name new focused tests with `adr_0039_`; confirm the filter executes them.
Run additional affected unit tests if a consumer changes beyond a method name.

## Rollback And Escalation

Revert this packet coherently; no configuration migration is required.
Return to the ADR for an unratified value set, capacity conflict, any chrome
pixel change, new persistence, or a need to alter the discrete size bridge.
Do not widen the packet into density or row-layout work.

## Expected Final Report

List files changed, numerical decision/evidence, checks (Green or error),
behavior changed, deviations and unresolved concerns. Keep the Status line,
phase plan, review checklist, delivery row and pending-human entry consistent.
End with `Operator visual check`, linking the procedure and naming the open
task 002 gate. Do not run the app.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet's Files To Inspect and Files Likely To Change.
- `docs/adr/0039-dynamic-type-ramp.md` and its phase plan.

Goal:
- Deliver live type/chrome resolution under the recorded numeric decision.

Constraints:
- Meet M1–M4 above; preserve exact chrome output and all five persisted steps.
- Obtain the numeric decision before landing proposed type values.

Do not touch:
- Everything in this packet's Do Not Touch section; task 002 owns reservation.

Acceptance criteria:
- M1–M4 Green; shared visual gate explicitly open.

Test commands:
- Run this packet's Test Commands in order.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
