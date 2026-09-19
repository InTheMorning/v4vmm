# ADR 0039 Task 002: Fixed-Height Reserve And Acceptance

Status: Scheduled - 2026-09-18. Amended the same day: requires task 001's
mechanical handoff only — numeric ratification moved to
[task 003](adr-0039-task-003-type-curve-ratification.md), which follows this
packet. Implementation not started. No visual gate of its own; type output
stays at identity through this packet, so there is nothing new to inspect
from the ramp. ShowCard's single-line summary fix is in scope and is proven
mechanically.

## Goal

Implement ADR 0039's fixed-height reservation at the shared geometry owner,
and guard its live consumers, sized against the ADR's reviewed but unratified
numeric proposal as a working ceiling. Give ShowCard's summary lines the
playlist row's single-line discipline, `whitespace_nowrap()` with
`overflow_hidden()`, correcting a latent wrap-and-clip defect that predates
this ADR. The now-playing row uses `truncate()` instead. That is a different
mechanism, and step 4 records why it is wrong here. Text readable at medium
must not acquire vertical clipping at either extreme, x-small or x-large —
the relative text-to-box pressure this reservation guards against rises at
both ends, not only at x-large.

## Files To Inspect

- `AGENTS.md`, `.github/copilot-instructions.md`, `docs/adr/README.md`.
- `docs/adr/0039-dynamic-type-ramp.md`, its phase plan, task 001 and review
  checklist; `docs/plans/broadcast-chain-delivery-order.md`.
- `docs/adr/0063-show-dashboard-layout.md`,
  `docs/troubleshooting/column-text-truncation.md`.
- `tests/architecture_tests.rs:14668` (`adr_0063_show_card_grid_shell_uses_vm_contract`):
  asserts the literal string `.h(Size::MenuCompact.scaled(cx))` against
  `src/ui/composites/show_card.rs`. Read this guard before touching ShowCard.
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

ADR 0063 text or its existing guard (including
`adr_0063_show_card_grid_shell_uses_vm_contract`, which pins ShowCard's
`.h(Size::MenuCompact.scaled(cx))` literal); configuration/schema; playback or
service commands; chrome coefficients or base dimensions; list paging/drag
algorithms; font roles, typed action semantics, unrelated inherited human
gates or the dated 2026-09-10 reconciliation. Do not fix truncation across the
repository beyond ShowCard's named single-line fix. Numeric ratification is
task 003's; do not promote the ADR's proposed table to a live value here.

## Constraints

Work follows task 001 in a fresh session; task 001 has no visual gate to be
open or closed. This packet's reservation ceiling uses the ADR's reviewed but
unratified numeric proposal as its maximum permitted text extent — it does
not need ratification to size a reservation, because type output stays at
identity through this packet regardless. If task 003 later ratifies numbers
that exceed this proposal, its correction must re-verify this packet's
reservation guard.

`.h(Size::MenuCompact.scaled(cx))` on ShowCard is pinned by the existing ADR
0063 guard `adr_0063_show_card_grid_shell_uses_vm_contract`
(`tests/architecture_tests.rs`). Any ShowCard change must keep that literal
string intact; do not move, edit or broaden that guard to accommodate this
packet's fix.

Line-height prerequisite: RESOLVED, 2026-09-18. GPUI resolves a bare
`div().text_size(...)` with no `.line_height()` override as
`line_height_px = round(font_px * 1.618034)`. `TextStyle::default()` sets
`line_height: phi()` (`gpui-pre-0.3.1/src/style.rs:494`). `phi()` is
`relative(1.618_034)` (`gpui-pre-0.3.1/src/geometry.rs:3708-3711`).

Resolution runs through `line_height_in_pixels()` (`style.rs:554-556`). That
calls `DefiniteLength::to_pixels` (`geometry.rs:3496-3504`). Its `Fraction`
branch multiplies the element's own font_size, not rem_size. Then it rounds.
gpui is pinned `gpui-pre` `=0.3.1` (`Cargo.toml:25`), source at
`~/.cargo/registry/src/index.crates.io-*/gpui-pre-0.3.1/`. It scales with
font size. It does not vary with font family. No ancestor of the six named
surfaces overrides it: the window root (`src/app/bootstrap.rs:77`) applies
an empty StyleRefinement, and `src/ui/primitives/label.rs:118` and
`src/ui/primitives/button.rs:401,520` call only `.text_size(...)`.

Two options exist for the reservation arithmetic. Read the constant at
runtime — `TextStyle`, `phi()`, `relative()` and `DefiniteLength` are
re-exported at the gpui crate root. Or hardcode it as a documented constant
citing `gpui-pre-0.3.1/src/geometry.rs:3710`. Both are valid. This packet
picks neither.

Also relevant to the arithmetic: box sizing is border-box. Taffy's
`Style::DEFAULT.box_sizing = BoxSizing::BorderBox`
(`taffy-0.13.0/src/style/mod.rs:598-605`). GPUI's `Style::to_taffy`
(`gpui-pre-0.3.1/src/taffy.rs:479-513`) never overrides it. `.h()` sets total
box height. Padding and border subtract from it.

Application source pins two other explicit ratios, corrected count: only
`MultilineText` (`text_size * 1.55`, `src/ui/primitives/multiline_text.rs`
line 120) and `LOG_LINE_HEIGHT` (1.5, `src/ui/tokens.rs:1082` — corrected
from a stale line-779 citation) pin an explicit ratio in the delivery
surfaces this task touches. `src/ui/style.rs:142-150` also defines a
`typography` module of fixed-pixel line heights (`LINE_TIGHT` 14,
`LINE_COMPACT` 15, `LINE_BODY` 16, `LINE_DETAIL` 17, `LINE_TITLE` 20,
`LINE_HEADER` 23), used by `src/ui/shells/discover/*` and
`src/ui/shells/library/{feed_list,feed_detail,track_detail_metadata_values}.rs`
— the legacy pre-Music surfaces ADR 0039's Context calls out as not delivery
targets. None of these three is an ancestor of the six named surfaces.

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
   point size equals line-box height. Resolve the line-height prerequisite
   above before computing capacity.
2. Put the reservation calculation in the shared geometry owner. For each
   variant, derive the maximum permitted text block from the ADR's reviewed
   numeric proposal (unratified, used here only as a sizing ceiling) and the
   resolved line-height policy, then account for surrounding chrome. Use that
   reserved space in the live consumers without changing chrome values or row
   heights.
3. If any reserved block exceeds the existing available height at a permitted
   step, report the measured mismatch and return to the ADR's numeric
   proposal. Do not shrink one screen's font, drop a line, wrap a compact row,
   or enlarge chrome to make a test pass.
4. Give ShowCard's `render_summary_line` (`src/ui/composites/show_card.rs`,
   line 159) the fix determined by investigation:
   `whitespace_nowrap()` + `overflow_hidden()`, NOT `.truncate()`. This
   follows the playlist row's mechanism
   (`src/ui/shells/playlist.rs`, lines 755/765), a silent clip with no
   ellipsis. The now-playing row (`src/ui/shells/queue_now_playing.rs`, lines
   198/206) uses a different mechanism — `.truncate()`, a clip with an
   ellipsis — and is not the pattern to follow here; the packet the two
   rows share is single-line discipline, not an identical mechanism.
   `.truncate()` would fail here: `render_summary_line`'s div chain
   (`show_card.rs:162-166`) has only `.min_h(...)`, `.text_size(...)` and
   `.text_color(...)`, none of which satisfies
   `adr_0063_column_text_does_not_truncate`'s `flex_1()`/`max_w(`/`.w(`
   requirement (`tests/architecture_tests.rs:14517-14550`). Keep
   `.h(Size::MenuCompact.scaled(cx))` on the card (line 83); that is the
   literal `adr_0063_show_card_grid_shell_uses_vm_contract` pins
   (`tests/architecture_tests.rs:14823-14835`). Keep `overflow_hidden()` on
   the card (line 85) too, but note it is not one of that guard's required
   literals — it matters for ADR 0063's decision generally, not as a string
   this particular guard checks. Do not touch that guard.
5. Add one mechanical capacity criterion and its tests across all relevant
   roles/variants and five steps. Add a situational ADR 0039 guard proving
   that fixed-height consumers, including ShowCard, use the tested owner and
   the single-line pattern. Include the owner in any geometry-dependent
   list/drag checks; do not create a parallel formula.
6. Run checks and rebuild the normal binary. State plainly that this packet
   has no visual gate of its own; hand off mechanically to task 003.
7. Keep all three packets and ADR 0039 short of accepted implementation until
   the twelve cells, preservation and cleanup pass in task 003. Reconcile all
   live status records in the same change; inherited gates remain separate.

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
- M3, preservation: task 001's type/chrome tests, existing ADR 0034/0063
  guards — including `adr_0063_show_card_grid_shell_uses_vm_contract` — and
  affected playlist/list geometry tests stay Green. Diff review proves
  unchanged persistence, action intent and list-height/drag calculations.
- M4, ShowCard single-line fix: an architecture guard proves
  `render_summary_line` matches the playlist/now-playing single-line pattern
  (no wrapped, silently clipped second line). This is a mechanical criterion,
  not a visual one — the defect and its correction are independent of scale.

M1 is the new reservation requirement; M2–M4 prove its integration, ShowCard's
correction and retained requirements. Mechanical evidence does not prove
glyph legibility.

## Visual Acceptance

None of its own. Type output stays at identity through this packet, so the
ramp produces nothing new to inspect. The twelve compact-row/detail/popover
cells at XS/XL in Light/Dark belong to
[task 003](adr-0039-task-003-type-curve-ratification.md), which lands the
ratified numbers that make them meaningful; they are listed in the
[review checklist](../reviews/adr-0039-review-checklist.md) under that
packet. ShowCard's single-line fix is proven by M4 above, not by a new
operator cell — its on-screen effect, where a summary line was already long
enough to wrap, is real but is not part of the type-ramp gate.

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

Name new reservation tests with `adr_0039_` and confirm the filter executes
them. Run the existing affected composite tests as well if their behavior changes.

## Rollback And Escalation

Rollback follows the phase plan; do not leave a broken reservation for task
003 to inherit. Escalate capacity failure with numeric evidence, any needed
chrome or line-count change, a variable-height list proposal, or inability to
reach one of the named surfaces. This packet has no visual cell of its own to
leave open. Playback acceptance and real audio are not prerequisites for this
packet.

## Expected Final Report

List files, checks (Green or error), capacity proof, the ShowCard fix and its
guard, behavior changes, deviations and concerns. State plainly that this
packet has no visual gate of its own and name task 003 as the next packet.
Agents never run the app. Keep packet Status, ADR, plan, review, delivery row
and pending-human entry consistent; mark Implemented only after every named
gate passes in task 003.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet's Files To Inspect and Files Likely To Change.
- Task 001's mechanical handoff. Task 001 lands no numeric values.

Goal:
- Deliver shared fixed-height reservation, its guard, and ShowCard's
  single-line fix. Type output stays at identity throughout.

Constraints:
- Meet M1–M4; preserve chrome values and list geometry.
- Do not touch `adr_0063_show_card_grid_shell_uses_vm_contract` or its
  pinned `.h(Size::MenuCompact.scaled(cx))` literal.
- Resolve the line-height prerequisite before computing reservation capacity.

Do not touch:
- Everything in this packet's Do Not Touch section.

Acceptance criteria:
- M1–M4 Green; no visual gate — hand off mechanically to task 003.

Test commands:
- Run this packet's Test Commands in order, then the applicable consumer tests.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
