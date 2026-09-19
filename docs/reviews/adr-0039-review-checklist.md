# ADR 0039 Review Checklist

## Status

Implementation reviewed - 2026-09-18. Changes requested before task 002's
mechanical handoff: its capacity audit excludes rows whose height grows under
the proposed ramp, and task 001's ownership guard permits bypassing the
per-role resolver. See R1 and R2 below. The current identity outputs are
preserved; the findings concern the required proof and preparation for task
003, not an observed regression from the current scale coefficients.

Reviewed commit `3b40ec1b769d36447792e444098af962973b891d` and the uncommitted
task 002 changes present at review start. Task 003 is not implemented. Its
numeric proposal remains unratified and all twelve visual inspections remain
open. Tasks 001 and 002 retain their packet-specific exemption from a visual
gate. This review changes no implementation or acceptance decision.

## Reviewed Artifacts

- [ADR 0039](../adr/0039-dynamic-type-ramp.md).
- [Phase plan](../plans/adr-0039-dynamic-type-ramp-phase-plan.md).
- [Task 001](../tasks/adr-0039-task-001-scale-domains-and-type-curves.md).
- [Task 002](../tasks/adr-0039-task-002-fixed-height-reserve-and-acceptance.md).
- [Task 003](../tasks/adr-0039-task-003-type-curve-ratification.md).
- [Operator procedure](../runbooks/dynamic-type-ramp-check.md).

## Numerical Decision — Task 003

- [ ] Record the operator's numerical decision in ADR 0039, with date.
- [ ] Cover seven upward and downward endpoints and both intermediate steps.
- [ ] Weigh the operator's x-small-density preference against the proposal's
      downward half before ratifying.
- [ ] Record task 002's fixed-height capacity findings, and re-verify them
      against the final ratified values, before landing them.

The policy is Accepted. None of these checkboxes implies the proposed table
is already ratified or that chrome coefficients are still undecided. This
numerical decision belongs to task 003; tasks 001 and 002 land no proposed
value.

## Task 002 Implementation Step 1 Evidence — 2026-09-18

Investigation evidence for task 002's Implementation Step 1, ahead of
implementation; task 002 stays Scheduled. Computed with the resolved
line-height constant (task 002's Constraints; ADR 0039's Audited Findings)
against the ADR's reviewed but unratified numeric proposal as the sizing
ceiling, at all five steps.

ShowCard (`.h(Size::MenuCompact.scaled(cx))`, 160px base). Reserved =
max(headline_lh, badge_h) + SM gap + 2 body lines + XS gap. Available =
MenuCompact − 2×MD padding − 2×1px border.

| Step | Available | Reserved | Margin | Result |
|---|---:|---:|---:|---|
| XSmall | 113.60 | 73.20 | 40.40 | Fits |
| Small | 123.12 | 76.04 | 47.08 | Fits |
| Medium | 134.00 | 78.00 | 56.00 | Fits |
| Large | 150.32 | 88.44 | 61.88 | Fits |
| XLarge | 168.00 | 99.00 | 69.00 | Fits |

Button (`.h(self.height(cx))`, no `.py()` on the visual div): fits at every
size and step. Tightest case: Sm/Caption at XSmall, 23.80 available vs 19
needed; still fits at 21.80 with the optional 2px border.

Every other named surface — ListRow, TrackRow, the queue row, the playlist
row, content_list's row — is a FLOOR or uncapped, not a cap. The "reserved
block exceeds available height" failure mode does not apply to them
mechanically; M1 must assert something other than a capacity ceiling for
those surfaces.

Review correction, 2026-09-18: the capped ShowCard/button figures above do not
establish unchanged allocation for the other rows. The earlier conclusion of
zero mismatches is withdrawn for the complete inventory. R1 below measures
queue-row growth under the proposal; that conflict must return to the ADR
before task 002 hands off.

## Mechanical Review

| Packet | Evidence required | Result |
|---|---|---|
| 001 M1 | Five exact chrome coefficients and every chrome token/step output unchanged, including the pill exception | Green |
| 001 M2 | Type resolver output bit-identical to the old uniform result, for all seven roles at all five steps — identity, not divergence | Green |
| 001 M3 | Both resolvers used in live paths; situational ADR 0039 ownership guard | Incomplete: live code is connected, but the guard misses disconnection; R2 |
| 001 M4 | Five persisted steps/default unchanged; installed environment path and discrete widget bridge verified | Green: configuration and size-bridge tests; no persistence diff |
| 002 M1 | Fixed row/card/bar capacity inventory and reservation bounds for all variants/steps, sized against the unratified proposal; string length does not alter allocation | Fails unchanged-allocation proof; R1 |
| 002 M2 | Live consumers use the tested shared reservation; situational ADR 0039 guard | Incomplete: the reservation path covers two caps, not the omitted row allocations; R1 |
| 002 M3 | Existing ADR 0034/0063 guards — including `adr_0063_show_card_grid_shell_uses_vm_contract` — and list/drag tests retained; no schema, action or chrome drift | Green: architecture and playlist tests; no prohibited source changes found |
| 002 M4 | ShowCard's `render_summary_line` matches the playlist/now-playing single-line pattern; architecture guard proves it | Green |
| 003 M1 | 35 ratified type outcomes, unchanged medium bases, monotonicity, role ordering and asymmetric growth/shrinkage | Pending |
| 003 M2 | Live type resolver outputs the ratified values, not task 001's identity placeholder; situational ADR 0039 guard | Pending |
| 003 M3 | Task 002's reservation capacity tests stay Green against the ratified values at all five steps | Pending |
| 003 M4 | Existing chrome, configuration, size-bridge and ADR 0034/0063 tests remain Green; no new persisted scale or chrome coefficient change | Pending |

Record consumer, roles, line count, line-height rule, maximum text extent and
available inner height in the implementation review. Record test names and
results; do not infer glyph fit from a numeric font size alone.

## Operator Baseline Before Task 003

Walkthrough complete, including preservation, preference restoration and
fixture cleanup confirmed by the operator. Type still uses the identity
coefficients. These observations establish the current baseline; they do not
close task 003's V1–V12 or either mechanical review finding.

The operator created and verified `/tmp/v4vmm-startup-6l4pw6r3` in the desktop
session. The walkthrough used track 1 with the long `Élan gyp` title, artist
and description from the runbook. Operator revision and exact viewport
dimensions were not recorded.

| Surface | Scale | Theme | Result / evidence |
|---|---|---|---|
| Music playlist track row | Medium | Light | Pass — operator reported `pass` after the row inspection and route to track detail |
| Music track detail | Medium | Light | Pass by screenshot inspection — the operator's second detail screenshot shows the full expanded Description paragraph through `next field or its actions.`, clean wrapping, and intact title, artist and actions. The earlier metadata-table excerpt is a separate compact preview. |
| Add to Playlist popover, including New Playlist input | Medium | Light | Pass — operator reported `pass` after list/input inspection, Back and Escape; instructed to leave the draft unsubmitted and not select an existing playlist |
| Music playlist row, track detail and Add to Playlist popover | XSmall | Light | Pass — operator reported `pass` for the three-surface check; no separate density preference or numeric ratification was stated |
| Music playlist row, track detail and Add to Playlist popover | XSmall | Dark | Pass — operator reported `pass` for the three-surface check |
| Music playlist row, track detail and Add to Playlist popover | XLarge | Dark | Pass — operator reported `pass` for the three-surface check |
| Music playlist row, track detail and Add to Playlist popover | XLarge | Light | Pass — operator reported `pass` for the three-surface check |
| Show card summaries, normal/narrow widths and card selection | XLarge | Light | Pass — operator reported `pass`; no separate observation established which summary reached horizontal overflow |
| Show card summaries, normal/narrow widths and card selection | XSmall | Light | Pass — operator reported `pass` for the remaining three Show combinations |
| Show card summaries, normal/narrow widths and card selection | XSmall | Dark | Pass — operator reported `pass` for the remaining three Show combinations |
| Show card summaries, normal/narrow widths and card selection | XLarge | Dark | Pass — operator reported `pass` for the remaining three Show combinations |

Music and Show XS/XL comparisons in both themes are complete for the identity
baseline. Final preservation inspection: Green, from the operator's pasted
JSON. Configuration changes are limited to normal workspace preferences;
configuration, music, migrations, bindings, library and tool blockers are
preserved. The fixture retains three tracks, three playlist memberships and
one playlist, with no music or database probes. The operator confirmed starting
theme/scale restoration and the cleanup command's
`Removed fixture: /tmp/v4vmm-startup-6l4pw6r3` message. This baseline walkthrough
has no remaining operator action. Task 003's checks remain open for its future
ratified type curves; the two mechanical review findings remain open.

## Operator Visual Check — Task 003

Run the [procedure](../runbooks/dynamic-type-ramp-check.md) after all three
packets; the first two land no presentation to inspect. Each cell requires an
observation or screenshot reference, inspected revision, viewport and result.
The same row/track/popover is used throughout.

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

Changes requested. The resolver split preserves the existing arithmetic, the
ShowCard correction uses the specified single-line clipping policy, and the
existing ADR 0034/0063 guards pass. The following two proof gaps prevent a
complete mechanical handoff. No application source was edited during review.

### R1 — P2: Include minimum-height rows in the unchanged-allocation proof

Locations: `src/ui/shells/queue_now_playing.rs:348-405`,
`src/ui/layouts.rs:194-223`, and the capacity inventory above.

The new row tests compare short and long strings only at the default medium
scale. They do not render the proposed ceiling or assert unchanged allocation
against the identity baseline at all five steps. The audit exempts these
consumers because they use `.min_h(...)` rather than `.h(...)`, but that means
they can grow when type diverges. It does not satisfy task 002 step 2's
requirement to preserve row heights.

An isolated copy of the current queue consumer was rendered in GPUI's unit
test context with the same 320 px container, title `A`, artist `Artist`, no
duration and no now-playing state. A test installed each `ScaleFactor`, drew
the view and read `debug_bounds("queue-now-playing-row")`. Repeating it with
only the temporary copy's type resolver changed to the ADR proposal produced:

| Step | Identity height, px | Proposed type height, px |
|---|---:|---:|
| XSmall | 40 | 44.5 |
| Small | 43 | 45 |
| Medium | 47 | 47 |
| Large | 52.5 | 54 |
| XLarge | 58.5 | 61 |

The production checkout retains identity type values. These measurements are
evidence of a proposal/reservation conflict, not an already-shipped type
regression or operator visual acceptance. No desktop app was launched.

Required fix: include minimum-height and uncapped row variants in the shared
geometry inventory and five-step consumer checks. Check the full allocated
row, its optional content and controls against the baseline and proposed
ceiling. Report the measured conflict in ADR 0039 before numerical
ratification; resolve the conflicting constraints there rather than silently
allowing larger rows or changing chrome. Do not hand the present zero-conflict
capacity conclusion to task 003.

### R2 — P2: Guard the call from `scaled_px` to `type_multiplier`

Location: `tests/architecture_tests.rs:18065-18076`.

The guard requires the two method declarations and a `self.scaled_px(` call,
but it never requires `scaled_px` to call `self.type_multiplier(scale)`.
In an isolated copy, this single replacement left all 19 `adr_0039_` tests
Green, including the ownership guard:

```rust
// Existing implementation:
px(f32::from(self.px()) * self.type_multiplier(scale))
// Undetected bypass used only for the review probe:
px(f32::from(self.px()) * Self::identity_step(scale))
```

The current implementation is correctly connected. The missing guard matters
because task 001 M3 explicitly requires removal of that connection to fail;
identity value tests cannot distinguish the two expressions. A bypass would
also make later edits to the per-role curve ineffective.

Required fix: scope the source check to the executable `scaled_px` body and
require its call to `type_multiplier`; keep the `scaled` → `scaled_px` check
separate. Verify that disconnecting either connection fails the guard, and
exercise `FontSize::scaled(cx)` with all five installed scales.

### Documentation and scope follow-through

The ADR, tasks 001/002, phase plan, delivery order and pending-human index
still describe implementation as not started. Reconcile them with the
committed task 001 code and task 002 work in progress when recording the
corrected handoff. Keep task 003's numerical decision and twelve cells open;
this review does not close an inherited human gate. No optional code cleanup
is requested.

### Verification

All final checks used the production checkout with its identity type resolver.

| Check | Result |
|---|---|
| `cargo fmt -- --check` | Green |
| `cargo check --locked --offline` | Green |
| `cargo test --locked --offline adr_0039_` | Green: 15 unit tests and 4 guards |
| `cargo test --locked --offline ui::tokens::tests` | Green: 13 tests |
| `cargo test --locked --offline ui::sizable_bridge::tests` | Green: 9 tests |
| `cargo test --locked --offline config::tests` | Green: 51 tests |
| `cargo test --locked --offline ui::shells::playlist::tests` | Green: 6 tests, including drag checks |
| `cargo test --locked --offline --test architecture_tests` | Green: 263 guards |
| `cargo clippy --locked --offline -- -D warnings` | Green |
| `cargo build --locked --offline --bin v4vmm` after tests | Green |
| `git diff --check` and review-file link targets | Green |

The full unit suite was not run. Temporary probe sources were removed; review
logs remain under `/tmp/v4vmm-adr0039-review-*.log`. The desktop app was not
launched and no operator acceptance was inferred from test-context rendering.
Only this existing review document was edited by the review. No documentation
files or folders were created or moved; root documents were left in place.
No broken review-file link targets were found.

ADR 0039 becomes Implemented only when numerical ratification, all three
packets, all twelve cells and cleanup are recorded. Reconcile packet Status
lines, phase plan, ADR index, delivery order and pending-human index
together.

## Operator visual check

Task 003's twelve cells remain open. Its numerical decision and implementation
must precede the ramp inspection; tasks 001/002 have no separate visual gate.

1. After task 003 is ready, use a Linux desktop terminal and the
   [dynamic type procedure](../runbooks/dynamic-type-ramp-check.md). It names
   the setup commands, data preparation and all twelve observations. Start
   with `cargo build --locked --offline --bin v4vmm`, then
   `type_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)`.
   The fixture uses Null playback and local service stubs; no audio device or
   live service is required.
2. Follow the procedure's data preparation, then run
   `python3 docs/runbooks/startup-recovery-fixture.py run "$type_fixture"`.
   Inspect its Music playlist row, track detail and Add to Playlist popover
   at XS/XL in Light/Dark against the medium reference. New vertical clipping,
   unexpected row growth, wrapping compact rows or unreachable controls fail
   the relevant cell.
3. Restore the starting theme/scale and close the app. Run
   `python3 docs/runbooks/startup-recovery-fixture.py inspect "$type_fixture"`.
   If preservation passes, run
   `python3 docs/runbooks/startup-recovery-fixture.py cleanup "$type_fixture"`
   and `unset type_fixture`. Record each result before closing any gate.
