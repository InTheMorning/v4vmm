# ADR 0039: Dynamic Type Ramp Phase Plan

## Status

Complete - 2026-09-18. All three packets have mechanical checks Green.
Tasks 001/002 retained uniform type values and had no separate visual gate.
Task 003 introduced the ratified type curves. The operator passed V1–V13
and confirmed fixture removal. The agent inferred preservation and preference
restoration from the conditional cleanup command.

The [review checklist](../reviews/adr-0039-review-checklist.md#operator-visual-check--task-003)
records the source hashes, viewport report and evidence limits.
ADR 0039 is Implemented. No operator gate remains open for this plan.

## Goal

Deliver separate live type and chrome curves, protect small text at both ends
of the five-step scale, and retain fixed-height layout assumptions. Implement
one packet per session under the [delivery order](broadcast-chain-delivery-order.md).

## Non-Goals

No configuration-format change, additional scale step, chrome-density change,
new font role, variable-height list, playback work, or ADR 0063 guard migration.
Do not reopen accepted recovery, log, Settings or text-selection packets.

## Current State And Assumptions

- `src/ui/tokens.rs` resolves fonts through the ratified per-role type curve
  and spacing, radius and size through the unchanged chrome curve.
  Font bases are 11, 12, 13, 15, 17, 20 and 24 px. Existing scale coordinates
  are 0.85, 0.92, 1.0, 1.12 and 1.25.
- Several geometry owners call the multiplier directly; they must retain chrome
  semantics when the new type resolver arrives.
- `src/config.rs` and Settings already expose exactly the five required steps.
  Reuse them. ADR 0066 task 004's independent gate does not block this work.
- ADRs 0046/0047/0060/0062 define the current Music/Show/Settings surfaces.
- Chrome identity is decided and permanent. Task 001 shipped type at
  identity with a per-role resolver shaped but unpopulated. Task 003
  populated it with the endpoints the operator ratified on 2026-09-18.

## Target State And Affected Modules

| Owner | Responsibility |
|---|---|
| `src/ui/tokens.rs` | Named chrome resolver, per-role type resolver, unchanged medium bases, pure value tests |
| `src/ui/layouts.rs`, `src/ui/icons.rs`, `src/ui/sizable_bridge.rs` | Existing geometry bridges use chrome; discrete widget-size mapping stays unchanged |
| `src/ui/primitives/image.rs`, `src/ui/composites/thumbnail.rs` | Image and artwork geometry stays on chrome |
| `src/ui/primitives/label.rs`, `src/ui/primitives/multiline_text.rs`, `src/ui/primitives/button.rs` | Consume type tokens; shared line-box/control geometry |
| `src/ui/composites/list_row.rs`, `src/ui/composites/track_row.rs`, `src/ui/composites/show_card.rs` | Fixed-line compact geometry and reservation |
| `src/ui/layouts.rs` and the shared primitives/composites above | Testable reservation calculation used by live fixed-height consumers |
| `src/ui/shells/playlist.rs`, `src/ui/shells/show.rs`, `src/ui/shells/queue_now_playing.rs` | Consume shared geometry without inventing scale or product policy |
| `src/ui/composites/track_detail_surface.rs`, `src/ui/composites/playlist_popover.rs` | Existing detail/popover composition used by the operator gate |
| `src/view_models/playlist_detail.rs`, `src/view_models/track_detail.rs`, `src/view_models/show.rs` | Retain display facts, line/variant intent and typed action ownership; no GPUI types |
| `tests/architecture_tests.rs` | Situational ADR 0039 ownership/consumer guards; existing ADR 0034/0063 guards retained |

The task packets name the remaining direct geometry call sites. Do not create
an unused abstraction or a second geometry owner.

## Sequence And Stopping Points

Delivery slot: after completed ADR 0066 task 013, before ADR 0069 Settings
follow-through and relay adoption. The real-show priority trigger still applies.

| Phase | Result | Entry condition | State / stopping point |
|---|---|---|---|
| [001: Scale domains and type curves](../tasks/adr-0039-task-001-scale-domains-and-type-curves.md) | Chrome and type resolvers both live, both bit-identical to today's uniform result at all five steps. Type resolver shaped per-role for task 003 | Accepted policy | Complete - 2026-09-18 (`3b40ec1`). Mechanical checks Green. No visual gate — output did not change |
| [002: Fixed-height reserve and acceptance](../tasks/adr-0039-task-002-fixed-height-reserve-and-acceptance.md) | Shared reservation and guard, sized against the reviewed proposal and retargeted to the ratified endpoints by task 003. ShowCard single-line fix | 001 mechanical checks Green. Fresh session | Complete - 2026-09-18. Mechanical checks Green. No visual gate of its own — type output still identity |
| [003: Type curve ratification](../tasks/adr-0039-task-003-type-curve-ratification.md) | The ADR records the numeric decision. The type resolver produces 35 ratified outcomes. The review records thirteen inspections and cleanup. | Task 002 mechanical checks must be Green. Use a fresh session. | Complete - 2026-09-18. Mechanical checks are Green. R2 is resolved. V1–V13 passed. The review records inferred preservation and confirmed fixture removal. |

Each packet may proceed after the preceding packet's mechanical checks pass.
Tasks 001/002 retain uniform type values and have no separate visual gate.
Task 003 owns the type inspections and V13's visual proof for task 002's ShowCard correction.
If operator evidence requires curve adjustments, update the ADR's numeric decision and resolver tests together.
Do not compensate in a screen.

All three packets are accepted. The thirteen cells and final preservation/cleanup
evidence are recorded in task 003 and the review checklist.
Do not chain implementation sessions.

## Schema And API Implications

No persisted format or public service API changes. `UiScale` variants, their
strings and the medium default stay unchanged. New scale/reservation helpers
are internal UI APIs. Remove the old generic multiplier after callers migrate;
if compatibility needs an alias, it delegates to chrome and has a documented
live caller. No dead compatibility layer.

## Risks And Test Strategy

| Risk | Evidence |
|---|---|
| Task 001 quietly diverges from identity | M1/M2 bit-equality tests on both chrome and type resolvers against the exact former `f32` values |
| Upward slope reused downward | All 35 pure type outcomes in task 003; small roles lose the least proportionally below medium |
| Type curve changes geometry | Five exact `f32` chrome coefficients, resolved token bit comparisons and live-owner guard, retained through all three packets |
| Prototype values treated as decided | ADR records the numerical decision in task 003, before that packet lands live type changes |
| Task 002's reservation ceiling stops matching the eventual ratified numbers | Task 003 re-verifies the reservation guard against its final ratified values before claiming the thirteen cells |
| A fixed row cuts glyphs or gains a line | Shared reservation capacity test across roles/variants/steps; task 002 consumer guard; task 003's compact-row visual cells |
| Font hierarchy collapses at XS | Ordered role-size test at every step and XS operator observations in task 003 |
| x-small density regresses under the proposal | Addressed at ratification: the downward half was anchored so Title keeps today's 0.85 and Micro rises 7.1% rather than 15.3%. The shrink-least invariant still holds |
| Details/popovers lose controls | Four detail and four popover inspections with the same data and viewport, in task 003 |
| ShowCard's latent wrap-and-clip defect | Task 002 architecture guard proving `render_summary_line` matches the playlist/now-playing single-line pattern |
| A refactor weakens column clipping or discrete widget sizing | Existing ADR 0063/0034 guards and size-bridge tests remain Green in every packet |

Run focused tests, architecture guards and required check/format/Clippy commands
as specified in each packet. Build the normal desktop binary after tests before
operator handoff. Agents do not launch the app. The
[review checklist](../reviews/adr-0039-review-checklist.md) separates mechanical
results, numerical ratification and visual evidence.

The [operator procedure](../runbooks/dynamic-type-ramp-check.md) uses a new
disposable startup fixture with local tracks, service stubs and Null playback.
It requires no audio device or reachable external service. No fixture is
created by this planning change.

## Rollback Strategy

Before task 002 lands, revert task 001 as one coherent code change if needed.
Before task 003 lands, revert tasks 001/002 together, including the ShowCard correction.
Their type values remain uniform. After task 003
lands, revert all three together or restore the previous type resolver while
preserving a valid reservation contract.

Preserve configuration and library
data; there is no format downgrade. Restore the operator's starting
theme/scale and clean up only the new disposable fixture after inspection.
Reopen failed evidence cells and reconcile status in the same change.

## Remaining Decisions

Tasks 001/002 retained uniform type values and need no further identity decision.
Debug assertions and source guards enforce the reservation.
It does not control rendered dimensions.
Task 003 retargeted the ceiling to the ratified endpoints and increased XS/Small margins.
Other margins and the enforcement mechanism did not change.
Whether the reservation should control rendered dimensions remains a separate design question.

Numerical type ratification completed on 2026-09-18, including all 35 outcomes
and the operator's x-small-density preference. Any future numeric change must
recheck reservation capacity and return to the ADR if it exceeds the bounds.
Chrome retuning requires its own packet and density inspection. The current
completed packets do not implement live layout enforcement.
