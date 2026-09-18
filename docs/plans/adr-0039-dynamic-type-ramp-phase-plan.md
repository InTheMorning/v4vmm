# ADR 0039: Dynamic Type Ramp Phase Plan

## Status

Policy accepted and scheduled - 2026-09-18. Implementation not started.
The type coefficient table in [ADR 0039](../adr/0039-dynamic-type-ramp.md#numeric-proposal--not-ratified)
is a proposal, not a ratified value set. All twelve operator inspections are
open; task 002 owns their evidence.

## Goal

Deliver separate live type and chrome curves, protect small text at both ends
of the five-step scale, and retain fixed-height layout assumptions. Implement
one packet per session under the [delivery order](broadcast-chain-delivery-order.md).

## Non-Goals

No configuration-format change, additional scale step, chrome-density change,
new font role, variable-height list, playback work, or ADR 0063 guard migration.
Do not reopen accepted recovery, log, Settings or text-selection packets.

## Current State And Assumptions

- `src/ui/tokens.rs` resolves all four token families through one multiplier.
  Font bases are 11, 12, 13, 15, 17, 20 and 24 px. Existing scale coordinates
  are 0.85, 0.92, 1.0, 1.12 and 1.25.
- Several geometry owners call the multiplier directly; they must retain chrome
  semantics when the new type resolver arrives.
- `src/config.rs` and Settings already expose exactly the five required steps.
  Reuse them. ADR 0066 task 004's independent gate does not block this work.
- ADRs 0046/0047/0060/0062 define the current Music/Show/Settings surfaces.
- Chrome identity is decided. Type numbers need explicit ratification in ADR
  0039 before landing. This is a bounded numeric review, not missing policy.

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
| [001: Scale domains and type curves](../tasks/adr-0039-task-001-scale-domains-and-type-curves.md) | Chrome seam live; reviewed type ramp live; 35 font outcomes and five chrome coefficients pinned | Accepted policy; record numerical ratification before landing type changes | Not started; mechanical handoff only, visual gate remains open |
| [002: Fixed-height reserve and acceptance](../tasks/adr-0039-task-002-fixed-height-reserve-and-acceptance.md) | Shared reservation and guard; twelve operator inspections and cleanup recorded | 001 mechanical checks Green and numerical decision recorded; fresh session | Not started; all visual cells open |

Task 002 may proceed after task 001's mechanical handoff while their shared
visual gate remains open. This is an explicit sequence within this plan, not
acceptance of task 001's presentation. If the curves need adjustment after
operator evidence, update the ADR's numeric decision and resolver tests in the
correction; do not compensate in a screen. Both packets remain unaccepted for
release until the twelve cells pass. Do not chain implementation sessions.

## Schema And API Implications

No persisted format or public service API changes. `UiScale` variants, their
strings and the medium default stay unchanged. New scale/reservation helpers
are internal UI APIs. Remove the old generic multiplier after callers migrate;
if compatibility needs an alias, it delegates to chrome and has a documented
live caller. No dead compatibility layer.

## Risks And Test Strategy

| Risk | Evidence |
|---|---|
| Upward slope reused downward | All 35 pure type outcomes; small roles lose the least proportionally below medium |
| Type curve changes geometry | Five exact `f32` chrome coefficients, resolved token bit comparisons and live-owner guard |
| Prototype values treated as decided | ADR records the numerical decision before task 001 lands |
| A fixed row cuts glyphs or gains a line | Shared reservation capacity test across roles/variants/steps; task 002 consumer guard; compact-row visual cells |
| Font hierarchy collapses at XS | Ordered role-size test at every step and XS operator observations |
| Details/popovers lose controls | Four detail and four popover inspections with the same data and viewport |
| A refactor weakens column clipping or discrete widget sizing | Existing ADR 0063/0034 guards and size-bridge tests remain Green |

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
After both land, revert them together or restore the previous type resolver
while preserving a valid reservation contract. Preserve configuration and
library data; there is no format downgrade. Restore the operator's starting
theme/scale and clean up only the new disposable fixture after inspection.
Reopen failed evidence cells and reconcile status in the same change.

## Remaining Decisions

Only numerical type ratification remains before task 001 can land. The ADR
contains explicit proposed endpoints and interpolation for review. If those
values exceed existing fixed-height capacity, report the measured bounds and
revise that proposal; chrome retuning requires a separately approved packet
with its own density inspection. No unspecified wrapping or truncation policy
remains.
