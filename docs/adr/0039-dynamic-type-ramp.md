# ADR 0039: Dynamic Type Ramp

## Status

Accepted - 2026-09-18.

Amended 2026-09-18: the operator specified the two scale domains, five-step
ceiling, wrapping boundaries, fixed-height reservation and visual gate. This
accepts the policy; the numeric type proposal below is not ratified.
Implementation has not started. The [phase plan](../plans/adr-0039-dynamic-type-ramp-phase-plan.md)
and two packets occupy the slot after ADR 0066 task 013 in the
[delivery order](../plans/broadcast-chain-delivery-order.md#current-delivery-order).
The twelve operator inspections remain open.

Reconciled 2026-09-10: the operator retained this proposal without scheduling
it. The text-scaling requirement has not been withdrawn; age is not a reason
to archive it. A defined policy must precede a phase plan and task packets.

## Context

The current app has Music, Show and Settings under ADR 0060. Music combines
local Library and Index content under ADRs 0047/0062; search is a toolbar
command. ADR 0046 owns frames and navigation. Separate Discover screens and
sidebars from the original May proposal are not the delivery targets.

ADR 0034 made shared UI scale-aware. `FontSize`, `Spacing`, `Radius` and `Size`
currently resolve through the same `ScaleFactor::multiplier()` in
`src/ui/tokens.rs`. Its five coefficients are 0.85, 0.92, 1.0, 1.12 and 1.25.
Multiplying every font role alike makes the smallest text 9.35 px at x-small
and enlarges page titles as much proportionally as small metadata at x-large.
The new ramp must protect small text in both directions.

Shared tokens, primitives and composites remain the scale owners. View models
keep presentation facts and typed actions; screens compose them. The accepted
ADR 0038 contracts remain in force. ADR 0063 already owns column clipping and
its guard; this ADR adds the space reservation needed when type and chrome
growth differ.

## Decision

### Two Live Scale Domains

| Domain | Owner and curve | Pixel effect in this delivery |
|---|---|---|
| Type | `FontSize`, with a named per-role curve | Every non-medium step changes; medium retains the existing seven base sizes |
| Chrome | One named curve for `Spacing`, `Radius` and `Size`, and existing geometry bridges | Values remain bit-identical to the current multiplier at all five steps |

The chrome seam is used by live callers in task 001. An unused helper is not
delivery. The five chrome coefficients are a decided identity with today's
behavior, not placeholder values: x-small 0.85, small 0.92, medium 1.0, large
1.12, x-large 1.25. Preserve multiplication order, base dimensions, rounding,
the unscaled pill-radius exception and the discrete third-party size bridge.
Icons, thumbnails and geometry calculations must not acquire a font curve.
A five-step value test pins the chrome coefficients, with resolved-value
checks for its token consumers.

This delivery does not change chrome density. A future packet that changes
chrome coefficients must schedule its own density inspection; that inspection
is not an open gate on this identity seam.

### Steps And Ceiling

Retain exactly `XSmall`, `Small`, `Medium`, `Large`, `XLarge`, persisted as
`x-small`, `small`, `medium`, `large`, `x-large`. Medium remains the default.
The scale coordinate tops out at 1.25; there is no sixth step or extra
accessibility tier. Per-role font output is a separate curve, so the proposed
Micro output of 1.36 at XL is not a new `UiScale` value.

No configuration key, enum variant, serialization or default changes. There
is no configuration-format migration, so this work does not depend on release
of ADR 0066's configuration-format gate. Its outstanding acceptance remains
independent.

### Type Growth And Shrinkage

At medium, each role retains its current base size. Above medium, smaller
roles gain the most proportionally. Below medium, smaller roles shrink the
least proportionally. Do not extrapolate the upward slope below medium or
take the reciprocal of the upward endpoint. The downward curve has its own
endpoints. At every step, resolved sizes remain positive and ordered
`Micro < Caption < Body < Headline < Title3 < Title2 < Title`.

Each role grows monotonically across the five steps, joins at exactly 1.0 at
medium, and differs from the old uniform result at every non-medium step.
Tokens expose a pure resolver for tests; `.scaled(cx)` uses that same resolver
with the installed environment. No screen computes a role multiplier.

#### Numeric Proposal — Not Ratified

The earlier seven upward multipliers were illustrative, not an operator
decision. The following is a concrete proposal for review, including both
halves. Only Micro 1.36 and Title 1.12 were named in the policy discussion;
the intervening values and downward endpoints here are proposed values.

| Font role | Medium px | Proposed XS factor | Proposed XL factor | Proposed XS px | Proposed XL px |
|---|---:|---:|---:|---:|---:|
| Micro | 11 | 0.98 | 1.36 | 10.78 | 14.96 |
| Caption | 12 | 0.97 | 1.32 | 11.64 | 15.84 |
| Body | 13 | 0.96 | 1.28 | 12.48 | 16.64 |
| Headline | 15 | 0.94 | 1.24 | 14.10 | 18.60 |
| Title3 | 17 | 0.92 | 1.20 | 15.64 | 20.40 |
| Title2 | 20 | 0.90 | 1.16 | 18.00 | 23.20 |
| Title | 24 | 0.88 | 1.12 | 21.12 | 26.88 |

Proposed interpolation uses the existing step coordinate `c`, with XS endpoint
`d` and XL endpoint `u` for a role:

- Below medium: `factor = 1 - ((1 - c) / 0.15) * (1 - d)`.
- Medium: `factor = 1` exactly.
- Above medium: `factor = 1 + ((c - 1) / 0.25) * (u - 1)`.

Thus small uses 8/15 of the downward change and large uses 12/25 of the upward
change. This is a finite five-step mapping, not a public continuous-scale API.
For Micro it proposes about 10.883 px at small and 12.901 px at large; it does
not shrink Micro to 9.35 px. Tests cover all 35 role/step outcomes and relative
shrinkage as well as growth. Record numerical ratification in this ADR before
task 001 lands type changes; a coding model must not silently promote the
proposal to policy. The accepted architecture is already scheduled.

### Wrapping And Fixed-Height Reservation

Detail surfaces, panels and popovers may wrap their explanatory/value text.
Use shared text owners and existing bounded scrolling. This permission does
not make every control multiline or override ADR 0063's unwrapped,
horizontally scrollable log contract.

Compact rows, card summaries and the now-playing bar retain their fixed line
count and fixed-height layout policy; overflowing text clips. Fixed-height
means deterministic geometry for a scale and row variant, not equal pixel
height across the five existing chrome steps. Do not introduce content-driven
row heights or alter pagination, drag placement or list-height assumptions.

[ADR 0063](0063-show-dashboard-layout.md#decision) continues to own the column
rule: use `overflow_hidden()` for stacked column text; do not add `truncate()`
there. Its existing situational guard stays with ADR 0063, unchanged.

**Additional ADR 0039 criterion:** a fixed-height row reserves space for its
largest permitted type step, so text readable at medium does not acquire
vertical clipping at XL. The shared geometry owner accounts for complete
line boxes, line count, gaps, padding, borders and adjacent controls. Test the
reserved block against the maximum permitted text extent and available inner
height at all five chrome steps. Horizontal overflow remains subject to the
existing clip policy; this is not a promise that arbitrary source strings fit
at every width.

Implement this criterion at the shared geometry owner, with a situational
ADR 0039 guard proving fixed-height consumers use it. Do not move or broaden
ADR 0063's guard. Existing chrome outputs remain unchanged: if the proposed
type ramp cannot fit those bounds, bring the measured conflict back to the ADR
before changing coefficients, density or line counts.

### Verification And Acceptance

Mechanical evidence covers chrome identity, all type steps, unchanged config
vocabulary/defaults, live resolver ownership and fixed-height capacity. It
cannot establish legibility or glyph clipping by itself.

The operator gate is exactly three named surfaces, each at x-small and
x-large in Light and Dark: twelve inspections.

1. Compact row: a track row in Music's `Startup fixture playlist` detail.
2. Detail page: the Music track detail opened from that same row.
3. Popover: Add to Playlist on that track detail, including its New Playlist
   input mode, without submitting a change.

The [operator procedure](../runbooks/dynamic-type-ramp-check.md) records all
twelve cells, medium reference observations, failure conditions and cleanup.
Task 002 owns this gate. Task 001 may hand off mechanically to task 002 in a
fresh session, but neither task claims visual acceptance before this gate is
walked. No inherited human check is closed by this policy decision.

## Invariants

- Five existing persisted steps; medium defaults and font bases unchanged.
- Live, separately named type and chrome resolvers; no local font compensation.
- Chrome coefficients and resolved dimensions retain their existing bits.
- Small roles grow most above medium and shrink least below medium; size
  hierarchy and monotonicity hold for all seven roles at every step.
- Fixed-height consumers reserve their text extent and retain list geometry.
- ADR 0063 owns column clipping and its existing guard.
- ADR 0039 stays Accepted until both packets and the twelve inspections pass.

## Non-Goals

No new scale preference, config migration, typography-role rename, theme/color
redesign, chrome-density retuning, variable-height list implementation, playback
change or revival of pre-Music surfaces. No repository-wide truncation cleanup.

## Alternatives Considered

- Keep uniform font scaling: fails the small-text requirement in both directions.
- Apply one role slope on both sides of medium: shrinks small text most when
  the requirement is to protect it.
- Ship an unused chrome helper: leaves the domain split undelivered.
- Retune chrome with type: adds density changes to a delivery whose chrome
  coefficients are explicitly unchanged.
- Re-home the column guard under 0039: splits ownership of a rule ADR 0063
  already decides and verifies.

## Consequences

Non-medium text changes while existing chrome dimensions remain stable. Small
text gains priority without inflating titles at the same rate. The numeric
proposal needs review against real fixed-height bounds before implementation.
The unchanged medium baseline limits default-view change; the extreme-step
checks remain essential, especially x-small readability.

## Follow-Up Work

1. Policy definition: complete on 2026-09-18, including downward scaling.
2. [Task 001](../tasks/adr-0039-task-001-scale-domains-and-type-curves.md): ratify
   the numeric proposal, wire both domains and pin their values/ownership.
3. [Task 002](../tasks/adr-0039-task-002-fixed-height-reserve-and-acceptance.md):
   implement the reservation criterion and its guard; walk the twelve checks.
4. Record evidence in the [review checklist](../reviews/adr-0039-review-checklist.md)
   and reconcile the ADR, packets, delivery order and pending-human index together.
