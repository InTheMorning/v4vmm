# ADR 0039: Dynamic Type Ramp

## Status

Implemented - 2026-09-18.

Amended 2026-09-18. The operator specified both scale domains, five steps,
wrapping rules, the reservation and visual checks. The operator then ratified
the numeric decision below.

The [phase plan](../plans/adr-0039-dynamic-type-ramp-phase-plan.md) defines three
packets after recovery task 013. The
[delivery order](../plans/broadcast-chain-delivery-order.md#current-delivery-order)
places them before Settings follow-through and relay adoption. All three packets are complete.

- Task 001 introduced separate type and chrome resolvers. Both initially
  produced values identical to the former uniform resolver, including every bit.
  This packet had no visual gate.
- Task 002 added the shared reservation and ShowCard's single-line correction.
  Type values remained uniform. This packet had no separate visual gate.
- Task 003 introduced the ratified type curves. It owned all thirteen visual
  checks, including V13 for ShowCard's correction.

Amended 2026-09-18 after operator acceptance. The operator passed all thirteen
visual checks and confirmed fixture removal.
The agent inferred preservation and preference restoration from the conditional cleanup command.
The conversation does not contain the original inspection JSON.
The [review checklist](../reviews/adr-0039-review-checklist.md#operator-visual-check--task-003)
records each observation, source hash, viewport report and evidence limit.
No ADR 0039 operator gate remains open.

Reconciled 2026-09-10: the operator retained this proposal without scheduling
it. The text-scaling requirement has not been withdrawn; age is not a reason
to archive it. A defined policy must precede a phase plan and task packets.

## Context

The current app has Music, Show and Settings under ADR 0060. Music combines
local Library and Index content under ADRs 0047/0062; search is a toolbar
command. ADR 0046 owns frames and navigation. Separate Discover screens and
sidebars from the original May proposal are not the delivery targets.

ADR 0034 made shared UI scale-aware. `FontSize`, `Spacing`, `Radius` and `Size`
previously resolved through the same `ScaleFactor::multiplier()` in
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

| Domain | Owner and curve | Pixel effect in task 001 |
|---|---|---|
| Type | `FontSize`, with a named per-role curve, shaped now so task 003 changes numbers in one place | Bit-identical to today's uniform result at all five steps; medium retains the existing seven base sizes |
| Chrome | One named curve for `Spacing`, `Radius` and `Size`, and existing geometry bridges | Bit-identical to today's uniform result at all five steps |

Here, identity means values identical to the former uniform resolver, including every bit.
Task 001 delivered both resolvers at identity.
Chrome retains those values throughout this ADR.
Type retained identity through tasks 001 and 002.
Task 003 changed the type coefficients after operator ratification.

Both resolvers must serve live callers. An unused helper does not satisfy this requirement.
The chrome coefficients are x-small 0.85, small 0.92, medium 1.0, large 1.12 and x-large 1.25.
These values are final for this ADR.
Preserve multiplication order, base dimensions, rounding, the unscaled pill-radius exception and the discrete size bridge.
Icons, thumbnails and geometry calculations must not use a font curve.

Tests check the chrome coefficients and their resolved token values at all five steps.
Task 001 separately checked the type resolver's identity output.

Task 001 changed neither chrome density nor type density.
Task 003 changed type values under its own numeric decision and visual gate.
Any future change to chrome coefficients requires a separate density inspection.
Unchanged chrome coefficients create no additional visual gate for these packets.

### Steps And Ceiling

Retain exactly `XSmall`, `Small`, `Medium`, `Large`, `XLarge`, persisted as
`x-small`, `small`, `medium`, `large`, `x-large`. Medium remains the default.
The scale coordinate tops out at 1.25; there is no sixth step or extra
accessibility tier. Per-role font output is a separate curve, so the ratified
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

Task 003 landed this ramp. Each role grows monotonically across the five
steps and joins at exactly 1.0 at medium. Of the 28 non-medium outcomes, 26
differ from the old uniform result. Title at x-small and at small are the two
anchored exceptions recorded under the amended difference criterion below.
Tokens expose a pure resolver for tests, and `.scaled(cx)` uses that same
resolver with the installed environment. No screen computes a role multiplier.

Task 001 shipped this shape at identity, bit-identical to the old uniform
result at every step. Task 003 replaced those placeholder values with the
ratified ones.

#### Ratified Numeric Decision — 2026-09-18

The operator ratified the per-role curve on 2026-09-18. These are the decided
values, and
[task 003](../tasks/adr-0039-task-003-type-curve-ratification.md) landed them
in the live type resolver.

The downward half is anchored. Title retains the former uniform factor of 0.85 at x-small.
Only the smaller roles shrink less than before.
Its endpoints step by a uniform 0.01 per role. The upward half is the reviewed
proposal, ratified unchanged, stepping by 0.04 per role.

| Font role | Medium px | XS factor | XL factor | XS px | XL px |
|---|---:|---:|---:|---:|---:|
| Micro | 11 | 0.91 | 1.36 | 10.01 | 14.96 |
| Caption | 12 | 0.90 | 1.32 | 10.80 | 15.84 |
| Body | 13 | 0.89 | 1.28 | 11.57 | 16.64 |
| Headline | 15 | 0.88 | 1.24 | 13.20 | 18.60 |
| Title3 | 17 | 0.87 | 1.20 | 14.79 | 20.40 |
| Title2 | 20 | 0.86 | 1.16 | 17.20 | 23.20 |
| Title | 24 | 0.85 | 1.12 | 20.40 | 26.88 |

Interpolation uses the existing step coordinate `c`, with XS endpoint `d` and
XL endpoint `u` for a role:

- Below medium: `factor = 1 - ((1 - c) / 0.15) * (1 - d)`.
- Medium: `factor = 1` exactly.
- Above medium: `factor = 1 + ((c - 1) / 0.25) * (u - 1)`.

Small uses 8/15 of the downward change and large uses 12/25 of the upward
change. This is a finite five-step mapping, not a public continuous-scale API.
All 35 resolved values:

| Role | XSmall | Small | Medium | Large | XLarge |
|---|---:|---:|---:|---:|---:|
| Micro | 10.01 | 10.472 | 11.00 | 12.9008 | 14.96 |
| Caption | 10.80 | 11.36 | 12.00 | 13.8432 | 15.84 |
| Body | 11.57 | 12.2373 | 13.00 | 14.7472 | 16.64 |
| Headline | 13.20 | 14.04 | 15.00 | 16.728 | 18.60 |
| Title3 | 14.79 | 15.8213 | 17.00 | 18.632 | 20.40 |
| Title2 | 17.20 | 18.5067 | 20.00 | 21.536 | 23.20 |
| Title | 20.40 | 22.08 | 24.00 | 25.3824 | 26.88 |

##### Why The Downward Half Is Anchored

The operator values x-small density. The reviewed proposal made every role larger and reduced that density.
Micro would have moved from 9.35 px to 10.78 px, a 15.3% increase.
Retaining the Title endpoint limits this increase.
Micro is 10.01 px, a 7.1% increase. Title remains 20.40 px.

The accepted invariant still holds. Micro loses 9% below medium and Title loses
15%, so smaller roles shrink least.

##### The Upward Half Reduces Large Text Relative To Uniform Scaling

At x-large, the ratified curve renders large roles smaller than the former uniform curve.
Title is 26.88 px instead of 30.00 px, a 10.4% reduction. Title2 is 7.2%
lower, Title3 4.0% lower and Headline 0.8% lower. Micro, Caption and Body grow.
The decision deliberately reduces these differences in size.
The twelve type checks require a person to inspect the resulting hierarchy.

##### Amended Difference Criterion

An earlier form of this decision required every non-medium outcome to differ
from the old uniform result. Anchoring Title at the former 0.85 makes its downward half bit-identical to uniform.
The formula resolves x-small to that endpoint and small to exactly 0.92.

Of the 28 non-medium outcomes, 26 differ from the uniform result. Title at
x-small and Title at small are deliberate exceptions, and the tests assert them
as equal. The seven medium outcomes are identical under both curves, because
both force `factor = 1` there. Base preservation covers those separately.

This criterion was amended to state a ratified decision. It was not weakened to
make a test pass.

The text-to-box divergence this ADR addresses is not an x-large-only concern.
At x-small the chrome box shrinks 15% while Body shrinks only 11%, so the
relative fit tightens even though the absolute text is smaller. At x-large
the box grows 25% while Body grows 28%, so the fit tightens in the other
direction. Relative pressure rises at both ends. The former uniform curve is
scale-invariant by comparison. Task 002's reservation and task 003's numeric
decision both need this both-ends framing, not an x-large-only one.

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
vertical clipping at either extreme, x-small or x-large. The relative-fit
pressure this guards against rises at both ends, not only at x-large — see
the text-to-box divergence finding above. The shared geometry owner accounts
for complete line boxes, line count, gaps, padding, borders and adjacent
controls. Test the reserved block against the maximum permitted text extent
and available inner height at all five chrome steps. Horizontal overflow
remains subject to the existing clip policy; this is not a promise that
arbitrary source strings fit at every width.

Implement this criterion at the shared geometry owner, with a situational
ADR 0039 guard proving fixed-height consumers use it. Do not move or broaden
ADR 0063's guard. Existing chrome outputs remain unchanged: if the proposed
type ramp cannot fit those bounds, bring the measured conflict back to the ADR
before changing coefficients, density or line counts.

### Audited Findings — 2026-09-18

Verified against the code, ahead of task 001:

- The 11 direct `ScaleFactor::multiplier()` call sites are all chrome/geometry
  call sites, and they match task 001's "Files Likely To Change" list exactly.
  The remaining token `.scaled()` call sites reclassify automatically at the
  `FontSize`/chrome enum boundary; no per-caller migration is needed beyond
  those 11.
- `render_summary_line` in `src/ui/composites/show_card.rs` (line 159) sets
  neither `whitespace_nowrap()` nor `truncate()`. It only sets a one-line
  `min_h`, inside a hard `.h(Size::MenuCompact.scaled(cx))` cap with
  `overflow_hidden()` (lines 83 and 85). A wrapped second line is silently
  clipped with no ellipsis. This is a latent defect today, at every scale —
  it is not introduced by this ADR. A future divergent type ramp would only
  add scale-dependence to an existing bug. Task 002 folds in the fix.
- ShowCard's fixed height is pinned by the existing ADR 0063 architecture
  guard `adr_0063_show_card_grid_shell_uses_vm_contract`
  (`tests/architecture_tests.rs`), which asserts the literal string
  `.h(Size::MenuCompact.scaled(cx))` against `src/ui/composites/show_card.rs`.
  Task 002 cannot change that surface without continuing to satisfy that
  guard. ADR 0063 keeps ownership of its guard; task 002 does not move or
  broaden it.
- The playlist track row (`src/ui/shells/playlist.rs`, lines 755 and 765) and
  the now-playing row (`src/ui/shells/queue_now_playing.rs`, lines 198 and
  206) are already structurally immune to this defect: they force one line
  and clip horizontally.
- Line-height prerequisite for task 002: RESOLVED, 2026-09-18. GPUI resolves
  a bare `div().text_size(...)` with no `.line_height()` override as
  `line_height_px = round(font_px * 1.618034)`. `TextStyle::default()` sets
  `line_height: phi()` (`gpui-pre-0.3.1/src/style.rs:494`). `phi()` is
  `relative(1.618_034)` (`gpui-pre-0.3.1/src/geometry.rs:3708-3711`).
  Resolution runs through `line_height_in_pixels()` (`style.rs:554-556`),
  which calls `DefiniteLength::to_pixels` (`geometry.rs:3496-3504`); its
  `Fraction` branch multiplies the element's own font_size, not rem_size,
  then rounds. gpui is pinned `gpui-pre` `=0.3.1` (`Cargo.toml:25`). It
  scales with font size, not font family. No ancestor of the six named
  surfaces overrides it: the window root (`src/app/bootstrap.rs:77`) applies
  an empty StyleRefinement, and `src/ui/primitives/label.rs:118` and
  `src/ui/primitives/button.rs:401,520` call only `.text_size(...)`. Task 002
  may read it at runtime — `TextStyle`, `phi()`, `relative()` and
  `DefiniteLength` are re-exported at the gpui crate root — or hardcode it as
  a documented constant citing `gpui-pre-0.3.1/src/geometry.rs:3710`; this
  finding records both options and picks neither. Also relevant to the
  arithmetic: box sizing is border-box. Taffy's `Style::DEFAULT.box_sizing =
  BoxSizing::BorderBox` (`taffy-0.13.0/src/style/mod.rs:598-605`) and GPUI's
  `Style::to_taffy` (`gpui-pre-0.3.1/src/taffy.rs:479-513`) never overrides
  it, so `.h()` sets total box height and padding/border subtract from it.
  Application source pins two other explicit ratios, corrected count: only
  `MultilineText` (`text_size * 1.55`, `src/ui/primitives/multiline_text.rs`
  line 120) and `LOG_LINE_HEIGHT` (1.5, `src/ui/tokens.rs:1082` — corrected
  from a stale line-779 citation). `src/ui/style.rs:142-150` also defines a
  `typography` module of fixed-pixel line heights (`LINE_TIGHT` 14,
  `LINE_COMPACT` 15, `LINE_BODY` 16, `LINE_DETAIL` 17, `LINE_TITLE` 20,
  `LINE_HEADER` 23), used by the legacy pre-Music `discover` and `library`
  feed/track-detail surfaces this ADR's Context excludes from delivery. None
  of the three is an ancestor of the six named surfaces.
- Pre-existing, not caused by this ADR: `src/ui/composites/track_row.rs`
  (lines 185 and 221) and `src/ui/composites/skeleton_track_row.rs` (line
  102) consume `layouts::TRACK_NUMBER_WIDTH` (24px) and
  `layouts::MIN_HIT_TARGET` (44px) as raw unscaled constants, with no
  `.scaled()` call, so a 44px hit-target floor never responds to scale. This
  is a finding, not scheduled work.
- Capacity audit, 2026-09-18: first computed against the reviewed proposal as
  the sizing ceiling, and retargeted to the ratified endpoints when task 003
  landed them. Margins increased at XS/Small and remained unchanged elsewhere.
  No component was resized to fit.
  With the resolved line height, ShowCard and Button fit at all five steps.

  ListRow, TrackRow, queue, playlist and content_list rows have minimum heights or no height limit.
  The capacity check for capped surfaces does not apply to those rows.
  No capacity conflict requires escalation to this ADR. Full figures are recorded in the
  [review checklist](../reviews/adr-0039-review-checklist.md#task-002-implementation-step-1-evidence--2026-09-18).
- Reservation enforcement, 2026-09-18: task 002's reservation is a debug
  assertion (`ShowCard::render`,
  `Button::debug_assert_label_reservation_fits`) plus source-grep
  architecture guards (`adr_0039_capped_surfaces_use_the_shared_reservation`
  in `tests/architecture_tests.rs`), not a live layout constraint. It is
  deliberately not used as a rendered dimension.
  Task 002 sized the ceiling against the proposal, which exceeded the uniform output at non-medium steps.
  Using that ceiling as a layout dimension would change the appearance before ratification.
  Task 002 prohibited that change. Release builds perform no runtime reservation check.

  Task 003 retained this mechanism.
  Whether the reservation should control rendered dimensions remains a separate design question.
- `ListRow` (`src/ui/composites/list_row.rs`), 2026-09-18: received no new task 002 test.
  It renders opaque `AnyElement` children and owns no text field.
  This observation does not prove that child geometry is independent of text length.
- Guard methodology, 2026-09-18: the first draft of
  `adr_0039_show_card_summary_line_matches_single_line_pattern`
  (`tests/architecture_tests.rs`) passed even with the fix removed.
  It matched method names in an explanatory comment in `render_summary_line`.
  The corrected guard uses `code_only()` to remove `//` lines before searching for calls.
  It also requires a dot before each method name.
  The comment no longer contains the searched call syntax.
  A source guard must not accept a match found only in a comment.
- Out of scope, recorded only, 2026-09-18:
  `src/ui/shells/library/content_list.rs:391-406`
  (`render_content_list_row_text`) has the same undisciplined shape ShowCard's
  summary line had before task 002's fix — no `.truncate()`/`.whitespace_nowrap()`
  on row title or secondary text. That row has no `.h()` cap, so wrapping
  grows the row instead of clipping it silently. Task 002's Do Not Touch bars
  fixing truncation beyond ShowCard's named single-line fix; no work is
  scheduled here.
- Out of scope, recorded only, 2026-09-18: `queue_now_playing.rs`'s
  `.truncate()` sites sit in text divs whose own chain lacks
  `flex_1()`/`max_w(`/`.w(` — that `flex_1()` is on their ancestor, at lines
  186-187. The file is not one of `adr_0063_column_text_does_not_truncate`'s
  three checked files, so nothing fails today, but the guard's own doc
  comment (`tests/architecture_tests.rs:14508-14511`) asserts that usage
  "sits in a flex row that gives the element a width" without mechanically
  verifying this file. A latent inconsistency in an existing guard's
  reasoning; the guard, its doc comment and the file are unchanged.

### Verification And Acceptance

Mechanical evidence covers chrome identity, all type steps, unchanged config
vocabulary/defaults, live resolver ownership and fixed-height capacity. It
cannot establish legibility or glyph clipping by itself.

The operator gate is three named surfaces, each at x-small and x-large in
Light and Dark, for twelve type cells, plus a thirteenth for ShowCard.

1. Compact row: a track row in Music's `Startup fixture playlist` detail.
2. Detail page: the Music track detail opened from that same row.
3. Popover: Add to Playlist on that track detail, including its New Playlist
   input mode, without submitting a change.
4. Show cards: each summary must render as exactly one clipped line.
   Folded in on 2026-09-18 as the visual proof owed for task 002's
   user-visible single-line fix. It is scale-independent, so one observation
   closes it.

The [operator procedure](../runbooks/dynamic-type-ramp-check.md) records all
thirteen cells, medium reference observations, failure conditions and cleanup.
[Task 003](../tasks/adr-0039-task-003-type-curve-ratification.md) owns this gate.
It ratifies and introduces the per-role values, producing the first type-size change.
Task 001 has no visual gate. Task 002 has no separate visual gate.
Task 002 checks its reservation and ShowCard fix mechanically while type values remain uniform.
V13 supplies the visual proof for that ShowCard fix.

Tasks 001/002 may hand off after mechanical checks pass, with each subsequent packet starting in a fresh session.
Visual acceptance requires all thirteen inspections. This policy decision closes no inherited human check.

## Invariants

- Five existing persisted steps; medium defaults and font bases unchanged.
- Live, separately named type and chrome resolvers; no local font compensation.
- Chrome coefficients and resolved dimensions retain their existing bits, in
  every packet.
- Task 001's type resolver is bit-identical to today's uniform result at
  every step; only task 003 may change that.
- Under the ratified curves, small roles grow most above Medium and shrink least below Medium.
  All seven roles retain their size order at every step and grow monotonically across steps.
  Ratification reconsidered this invariant against the operator's x-small-density preference and anchored the downward half.
- Fixed-height consumers reserve their text extent and retain list geometry.
- ADR 0063 owns column clipping and its existing guard.
- Implementation acceptance requires all three packets, all thirteen
  inspections, preservation and cleanup. That evidence is recorded in the review.

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

Task 001 changed neither scale domain's output.
Task 002 added reservation checks without changing rendered dimensions.
Its ShowCard correction prevents a long summary from wrapping beneath the card boundary.
V13 in task 003 supplied visual acceptance for that correction.

Task 003 changed non-medium type values after the operator reviewed the numbers and the reservation bounds.
The operator's x-small-density preference informed that decision.
Chrome dimensions remain unchanged at each step.
Medium retains the former font sizes.
Checks at both extremes remain necessary because text and chrome scale differently in both directions.

## Follow-Up Work

1. Policy definition: complete on 2026-09-18, including downward scaling.
2. [Task 001](../tasks/adr-0039-task-001-scale-domains-and-type-curves.md):
   complete on 2026-09-18 (`3b40ec1`). Both live resolvers shipped at identity.
   No visual gate.
3. [Task 002](../tasks/adr-0039-task-002-fixed-height-reserve-and-acceptance.md):
   complete on 2026-09-18 with mechanical checks Green — the reservation
   criterion, its guards, and ShowCard's single-line fix. No visual gate of
   its own. Its reservation is a debug assertion today, not a live layout
   constraint. Task 003 retained that mechanism. Possible live enforcement
   remains a separate design question in the phase plan.
4. [Task 003](../tasks/adr-0039-task-003-type-curve-ratification.md): numeric
   decision ratified and the per-role curves landed on 2026-09-18, with
   mechanical checks Green. V1–V13, preservation and cleanup are accepted.
   The packet is complete.
5. Acceptance evidence is recorded in the
   [review checklist](../reviews/adr-0039-review-checklist.md). ADR, packets,
   delivery order and pending-human index are reconciled.
