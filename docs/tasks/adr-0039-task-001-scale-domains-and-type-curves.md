# ADR 0039 Task 001: Scale Domains And Type Curves

Status: Complete - 2026-09-18 (`3b40ec1`). Mechanical checks are Green.
Both resolvers initially produced the former uniform values.
This packet has no visual gate.

Task 003 introduced the ratified type curves
and resolved review finding R2. Its review records thirteen visual passes,
inferred preservation and confirmed fixture removal.
[ADR 0039](../adr/0039-dynamic-type-ramp.md) is Implemented.

## Goal

Wire separate live chrome and type resolvers. Preserve existing chrome bits,
medium font bases and configuration. Both resolvers resolve bit-identically
to today's uniform result at all five steps; nothing changes on screen. Shape
the type resolver per-role so [task 003](adr-0039-task-003-type-curve-ratification.md)
can change its numbers in one place without touching this seam again.

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

The ADR's type table is a proposal, owned by task 003. This packet does not
ratify it, does not present it for numerical decision, and does not land any
non-identity type value. Wire the per-role resolver's shape only: it must
resolve every role to its exact medium-based value at every step, bit-equal
to the old uniform multiplier's result. No screen-specific adjustment, unused
chrome seam, new persisted scale or speculative compatibility helper.

Keep `Radius::Full` unscaled. Preserve the arithmetic order and float values
of existing chrome calculations. The pixel-size branch of `SizableScaled`
uses chrome; its discrete tier branch keeps its current behavior.

## Implementation Steps

1. Inspect current callers with `rg -n 'multiplier\(\)|ScaleFactor|FontSize'`
   in the listed files. Classify direct multiplier calls as geometry. Confirm
   the count and file set against this packet's list; report any mismatch
   rather than silently expanding scope.
2. Introduce a named chrome curve and a pure per-role type resolver in
   `src/ui/tokens.rs`. The type resolver's per-role shape exists so task 003
   can populate it later without a second migration, but every role at every
   step must resolve to the same `f32` bits as today's uniform multiplier.
   Keep `.scaled(cx)` as the shared rendering entry point. Route `FontSize` to
   type; route `Spacing`, `Radius` and `Size` to chrome.
3. Migrate the listed geometry consumers to chrome without changing their
   output or discrete mappings. Remove the old multiplier if no live caller
   needs it; any retained compatibility alias delegates to chrome.
4. Add value tests and a situational ADR 0039 guard for live domain ownership.
   Preserve existing ADR 0034 and ADR 0063 guards. Keep constants in one owner.
5. Run the checks below. Record a mechanical handoff and state plainly there
   is no visual gate to open or close. End the session before task 002 starts.

## Mechanical Acceptance Criteria

- M1: a five-step pure value test compares chrome coefficients with the exact
  former `f32` values using bit equality, not rounded display strings.
  Resolved `Spacing`, `Radius` and `Size` tests compare every variant/step with
  the old arithmetic, including the pill exception; no chrome base changes.
- M2: a five-step pure value test compares the per-role type resolver's
  output with the exact former uniform multiplier's `f32` result, using bit
  equality, for all seven roles at all five steps. This is bit-identity with
  the old uniform result, not divergence from it — task 001 ships no ratified
  numbers, so there is nothing to diverge yet. Monotonic role growth, role
  ordering and asymmetric growth/shrinkage are task 003's M2, tested once the
  ratified curves land.
- M3: the situational ADR 0039 guard proves `FontSize::scaled` uses type while
  chrome tokens and direct geometry bridges use chrome. Removing either live
  connection must fail; defining unused helpers must not satisfy the guard.
- M4: existing configuration parsing/default and size-bridge tests remain
  Green; review proves no new `UiScale` variant/string/key/default or discrete
  widget mapping. Test the environment-backed resolver path, not only a table.

## Visual Acceptance

None. Both resolvers land at identity, so resolved output does not change at
any step, on any surface. There is nothing to inspect. This packet's
mechanical handoff allows task 002 to proceed in a fresh session; it does not
claim any presentation, because none changed.
[Task 003](adr-0039-task-003-type-curve-ratification.md) owns the thirteen
inspections, accepted on 2026-09-18 after landing the ratified per-role curves. There is no
chrome-density inspection for unchanged coefficients, in this packet or any
other in this ADR.

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
Return to the ADR for a capacity conflict, any chrome pixel change, new
persistence, or a need to alter the discrete size bridge. This packet lands
no numeric proposal, so there is no ratification question to escalate here;
that belongs to task 003. Do not widen the packet into density or row-layout
work.

## Expected Final Report

List files changed, checks (Green or error), confirmation that resolved
output is bit-identical to the old uniform result at every step, deviations
and unresolved concerns. Keep the Status line, phase plan, review checklist,
delivery row and pending-human entry consistent. State plainly that there is
no visual gate for this packet and name task 003 as the next packet. Do not
run the app.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet's Files To Inspect and Files Likely To Change.
- `docs/adr/0039-dynamic-type-ramp.md` and its phase plan.

Goal:
- Deliver live type/chrome resolution, both bit-identical to today's uniform
  result at every step. Shape the type resolver per-role for task 003.

Constraints:
- Meet M1–M4 above; preserve exact chrome output and all five persisted steps.
- Land no numeric proposal value. Task 003 ratifies and lands those numbers.

Do not touch:
- Everything in this packet's Do Not Touch section; task 002 owns reservation
  and task 003 owns numeric ratification.

Acceptance criteria:
- M1–M4 Green; no visual gate — state plainly there is nothing to inspect.

Test commands:
- Run this packet's Test Commands in order.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
