# ADR 0039 Task 002: Fixed-Height Reserve And Acceptance

Status: Scheduled - 2026-09-18. Implementation not started; requires task 001
mechanical handoff and numeric ratification. Twelve operator inspections open.

## Goal

Implement ADR 0039's fixed-height reservation at the shared geometry owner,
guard its live consumers, and record the twelve extreme-scale inspections.
Text readable at medium must not acquire vertical clipping at XL.

## Files To Inspect

- `AGENTS.md`, `.github/copilot-instructions.md`, `docs/adr/README.md`.
- `docs/adr/0039-dynamic-type-ramp.md`, its phase plan, task 001 and review
  checklist; `docs/plans/broadcast-chain-delivery-order.md`.
- `docs/adr/0063-show-dashboard-layout.md`,
  `docs/troubleshooting/column-text-truncation.md`.
- `src/ui/tokens.rs`, `src/ui/layouts.rs`,
  `src/ui/primitives/label.rs`, `src/ui/primitives/multiline_text.rs`,
  `src/ui/primitives/button.rs`, `src/ui/composites/list_row.rs`,
  `src/ui/composites/track_row.rs`, `src/ui/composites/show_card.rs`.
- `src/ui/shells/playlist.rs`, `src/ui/shells/library/content_list.rs`,
  `src/ui/shells/show.rs`, `src/ui/shells/queue_now_playing.rs`.
- `src/ui/composites/track_detail_surface.rs`,
  `src/ui/composites/playlist_popover.rs`, `src/ui/primitives/popover.rs`.
- `src/view_models/playlist_detail.rs`, `src/view_models/track_detail.rs`,
  `src/view_models/show.rs`, `src/view_models/queue_now_playing.rs`,
  `tests/architecture_tests.rs`, `docs/runbooks/dynamic-type-ramp-check.md`.

## Files Likely To Change

- `src/ui/layouts.rs`: pure named reservation geometry using task 001's
  resolver; shared primitives/composites from the inspection list as needed
  to own line boxes and consume that geometry.
- `src/ui/shells/playlist.rs`, `src/ui/shells/library/content_list.rs`,
  `src/ui/shells/show.rs`, `src/ui/shells/queue_now_playing.rs`: consume shared
  geometry only where their fixed-height composition requires it.
- `tests/architecture_tests.rs`: new situational ADR 0039 reservation guard.
- This packet, ADR 0039, phase plan, review checklist, delivery order,
  pending-human index and operator procedure: evidence/status reconciliation.

## Do Not Touch

ADR 0063 text or its existing guard; configuration/schema; playback or service
commands; chrome coefficients or base dimensions; list paging/drag algorithms;
font roles, typed action semantics, unrelated inherited human gates or the
dated 2026-09-10 reconciliation. Do not fix truncation across the repository.

## Constraints

Work follows task 001 in a fresh session. Its shared visual gate may still be
open. Verify that ADR 0039 actually records numerical ratification.

Geometry belongs in existing shared layout/primitive/composite owners. View
models retain renderer-free presentation intent. Shells use the owner; they
do not compute ad hoc font sizes, heights, or action availability. Fixed row
variants have fixed line counts; long text clips without making a row taller.
Preserve the existing geometry consumed by list layout and drag hit testing.

Detail/panel/popover wrapping permission does not override the unwrapped log
contract or require wrapping button labels. Column clipping remains ADR 0063's
rule; cite and run its guard without re-homing or broadening it.

## Implementation Steps

1. Inventory each fixed-height consumer's permitted font roles, line count,
   padding, gaps, border and control/artwork extent, including optional row
   states. Record actual capacity in the review checklist. Do not assume font
   point size equals line-box height.
2. Put the reservation calculation in the shared geometry owner. For each
   variant, derive the maximum permitted text block from the ratified ramp and
   line-height policy, then account for surrounding chrome. Use that reserved
   space in the live consumers without changing chrome values or row heights.
3. If any reserved block exceeds the existing available height at a permitted
   step, report the measured mismatch and return to the ADR's numeric decision.
   Do not shrink one screen's font, drop a line, wrap a compact row, or enlarge
   chrome to make a test pass.
4. Add one mechanical capacity criterion and its tests across all relevant
   roles/variants and five steps. Add a situational ADR 0039 guard proving that
   fixed-height consumers use the tested owner. Include the owner in any
   geometry-dependent list/drag checks; do not create a parallel formula.
5. Run checks, rebuild the normal binary and hand the operator the linked
   procedure. Record each of the twelve cells separately. Corrections stay at
   shared owners and carry regression proof; repeat affected cells after edits.
6. Keep both packets and ADR 0039 short of accepted implementation until all
   cells, preservation and cleanup pass. Reconcile all live status records in
   the same change; inherited gates remain separate.

## Mechanical Acceptance Criteria

- M1, reservation: the pure geometry test proves the reserved block contains
  the maximum permitted line-box extent for every fixed row/card/bar variant
  and fits its available inner height at all five steps, including padding,
  gaps, borders and neighboring controls. Fixed geometry and line count do not
  depend on string length. Assert bounds and unchanged allocation, not only a
  helper's literal result. Use long/short source strings and optional states
  in consumer tests to protect this class of regression.
- M2, ownership: a situational ADR 0039 architecture guard proves the actual
  fixed-height consumers use the tested reservation path. Removing that use
  must fail the guard. It neither replaces nor edits
  `adr_0063_column_text_does_not_truncate`.
- M3, preservation: task 001's type/chrome tests, existing ADR 0034/0063 guards
  and affected playlist/list geometry tests stay Green. Diff review proves
  unchanged persistence, action intent and list-height/drag calculations.

M1 is the new reservation requirement; M2/M3 prove its integration and retain
existing requirements. Mechanical evidence does not prove glyph legibility.

## Visual Acceptance

Open: compact playlist track row, that track's detail page, and its Add to
Playlist popover at XS/XL in Light/Dark. All twelve cells are listed in the
[review checklist](../reviews/adr-0039-review-checklist.md). Use the
[operator procedure](../runbooks/dynamic-type-ramp-check.md); record the same
viewport/data, a medium reference, readable small text, uncropped glyphs,
fixed row lines, accessible controls, detail/popover wrapping and cleanup.
Do not add a chrome-density re-walk while its coefficients remain unchanged.

## Test Commands

```bash
cargo fmt -- --check
cargo check --locked --offline
cargo test --locked --offline adr_0039_
cargo test --locked --offline ui::tokens::tests
cargo test --locked --offline ui::shells::playlist::tests
cargo test --locked --offline --test architecture_tests
cargo clippy --locked --offline -- -D warnings
cargo build --locked --offline --bin v4vmm
git diff --check
```

Name new reservation tests with `adr_0039_` and confirm the filter executes
them. Run the existing affected composite tests as well if their behavior changes.

## Rollback And Escalation

Rollback follows the phase plan; do not leave larger type with a broken
reservation. Escalate capacity failure with numeric evidence, any needed chrome
or line-count change, a variable-height list proposal, or inability to reach
one of the named surfaces. An unwalked visual cell stays open. Playback
acceptance and real audio are not prerequisites for this packet.

## Expected Final Report

List files, checks (Green or error), capacity proof, visual cell evidence,
behavior changes, deviations, concerns and cleanup. End with
`Operator visual check`, the procedure and remaining cells. Agents never run
the app. Keep packet Status, ADR, plan, review, delivery row and pending-human
entry consistent; mark Implemented only after every named gate passes.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet's Files To Inspect and Files Likely To Change.
- Task 001's numerical decision and mechanical handoff.

Goal:
- Deliver shared fixed-height reservation, its guard and operator handoff.

Constraints:
- Meet M1–M3; preserve chrome values and list geometry.
- Leave all unwalked visual cells open.

Do not touch:
- Everything in this packet's Do Not Touch section.

Acceptance criteria:
- M1–M3 Green; twelve separately recorded operator results and cleanup.

Test commands:
- Run this packet's Test Commands in order, then the applicable consumer tests.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
