# ADR 0039 Review Checklist

## Status

Accepted - 2026-09-18. Mechanical checks are Green for all three packets.
R1 and R2 are resolved. The operator ratified the numeric decision, passed
V1–V13 and confirmed fixture removal.

The agent inferred preservation and preference restoration from the conditional cleanup command.
The evidence section states the assumption and its limits. ADR 0039 is Implemented.

The original review covered commit `3b40ec1b769d36447792e444098af962973b891d`
and task 002 changes, later committed as `b5349f8`.
The operator inspected task 003 changes in the working tree based on `b5349f8`.
The recorded source hashes identify those files before the wording review.
Tasks 001 and 002 have no separate visual gate. This review does not close inherited human checks.

## Reviewed Artifacts

- [ADR 0039](../adr/0039-dynamic-type-ramp.md).
- [Phase plan](../plans/adr-0039-dynamic-type-ramp-phase-plan.md).
- [Task 001](../tasks/adr-0039-task-001-scale-domains-and-type-curves.md).
- [Task 002](../tasks/adr-0039-task-002-fixed-height-reserve-and-acceptance.md).
- [Task 003](../tasks/adr-0039-task-003-type-curve-ratification.md).
- [Operator procedure](../runbooks/dynamic-type-ramp-check.md).

## Numerical Decision — Task 003

- [x] Recorded in ADR 0039, dated 2026-09-18.
- [x] Seven upward and downward endpoints and both intermediate steps covered.
- [x] The x-small-density preference was weighed. The downward half was
      anchored so Title keeps today's 0.85. Micro rises 7.1%, not 15.3%.
- [x] Task 002's capacity findings re-verified against the ratified values.
      The revised endpoints increased XS/Small margins. Other margins did not change.

Ratified on 2026-09-18. The upward half was accepted as proposed.
At x-large, large roles are smaller than before. Title is 26.88 px instead of 30.00 px.
The decision deliberately reduces these size differences. The operator inspections below confirm readability.

## Task 002 Implementation Step 1 Evidence — 2026-09-18

This evidence covers task 002's Implementation Step 1.
The calculations used the resolved line-height constant and the reviewed proposal as the ceiling at all five steps.
Task 002's Constraints and ADR 0039's Audited Findings document that constant.
Task 003 later retargeted the ceiling to the ratified endpoints.

The 003 M3 row records the revised figures. XS/Small margins increased. Other margins did not change.

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
size and step. The tightest case is Sm/Caption at XSmall. It needs 19 px
within 23.80 px, or 21.80 px with the optional border.

ListRow, TrackRow, queue, playlist and content_list rows have minimum heights or no height limit.
The capacity check for capped surfaces does not apply to those rows.
M1 must check a different property for these surfaces.

Review correction, 2026-09-18: the ShowCard/button figures do not establish unchanged allocation for other rows.
The review withdrew its earlier conclusion of zero mismatches across the full inventory.
R1 recorded that gap and was resolved by narrowing the capacity claim to the two capped surfaces.
Separate tests check string-length independence for TrackRow, the queue row and the playlist row.
They do not establish that property for every uncapped row.

## Mechanical Review

| Packet | Evidence required | Result |
|---|---|---|
| 001 M1 | Five exact chrome coefficients and every chrome token/step output unchanged, including the pill exception | Green |
| 001 M2 | Type resolver output bit-identical to the old uniform result, for all seven roles at all five steps — identity, not divergence | Green |
| 001 M3 | Both resolvers must serve live callers. A situational ADR 0039 guard must enforce ownership. | Green. Task 003 resolved R2. The guard checks `scaled` → `scaled_px` → `type_multiplier` in separate function bodies. Deliberate bypasses failed the corrected guard. |
| 001 M4 | The five persisted steps and default must remain unchanged. Tests must cover environment resolution and the discrete size bridge. | Green. Configuration and size-bridge tests passed. Persistence code did not change. |
| 002 M1 | Check capacity for capped surfaces at every step. Check text-length independence separately for the named rows. | Green under the narrowed R1 scope. `show_card_summary_reservation`, `show_card_available_inner_height` and `button_label_reservation` in `src/ui/layouts.rs` fit at all five steps. Tests: `adr_0039_show_card_reservation_fits_available_at_all_steps`, `adr_0039_button_reservation_fits_available_at_all_steps`, `adr_0039_reservation_does_not_take_text_and_is_deterministic`. Other rows have minimum heights or no height limit. Text-length tests cover TrackRow, the queue row and the playlist row. Tests: `adr_0039_track_row_height_is_independent_of_title_length`, `adr_0039_queue_row_height_is_independent_of_title_length`, `adr_0039_playlist_row_body_height_is_independent_of_title_length`. This does not prove the same property for ListRow or content_list. |
| 002 M2 | Live consumers must use the tested reservation. A situational ADR 0039 guard must enforce those calls. | Green. `adr_0039_capped_surfaces_use_the_shared_reservation` checks ShowCard and Button calls into `src/ui/layouts.rs`. Debug assertions check capacity. These checks do not control rendered dimensions. See task 002 Finding 1. |
| 002 M3 | Retain ADR 0034/0063 guards and list/drag tests. Keep schema, action and chrome behavior unchanged. | Green. Architecture and playlist tests passed. No prohibited source changes were found. The retained guards include `adr_0063_show_card_grid_shell_uses_vm_contract`. |
| 002 M4 | ShowCard summaries must use the specified single-line pattern. An architecture guard must enforce that pattern. | Green. Tests: `adr_0039_show_card_summary_line_matches_single_line_pattern`, `adr_0039_show_card_height_is_independent_of_summary_length`. |
| 003 M1 | Check all 35 type outcomes, Medium bases, monotonic growth, role ordering and asymmetric scaling. | Green. Tests: `adr_0039_type_outcomes_match_ratified_values`, `adr_0039_medium_type_returns_each_role_base_exactly`, `adr_0039_type_grows_monotonically_per_role_across_steps`, `adr_0039_type_role_ordering_holds_at_every_step`, `adr_0039_smaller_roles_grow_more_above_medium`, `adr_0039_smaller_roles_shrink_less_below_medium`. The test `adr_0039_non_medium_type_outcomes_differ_from_uniform_except_title_downward` compares the 28 non-medium outcomes with uniform scaling. Exactly 26 differ. Title at XS/Small retains the former output. |
| 003 M2 | The live type resolver must produce the ratified values. A situational ADR 0039 guard must reject the former identity implementation. | Green. `adr_0039_type_and_chrome_domains_stay_live_and_separate` requires the seven endpoint arms verbatim and rejects the identity helper. Deliberate reversions failed the guard. This also resolves R2. |
| 003 M3 | Capacity tests must pass with the ratified values at all five steps. | Green. XS reserved height changed from 73.20 to 69.20 px, increasing the margin from 40.40 to 44.40 px. Small reserved height changed from 76.04 to 74.04 px, increasing the margin from 47.08 to 49.08 px. Medium, Large and XLarge figures did not change. The tightest button case, Caption at XS, reserves 17 px instead of 19 px within 21.80 px. No component was resized. |
| 003 M4 | Retain chrome, configuration, size-bridge and ADR 0034/0063 tests. Keep persisted scales and chrome coefficients unchanged. | Green. The recorded suite passed 1502 unit tests and 263 architecture guards. Clippy and format checks passed. Chrome value tests did not change. |

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
has no remaining operator action.

At that baseline stage, task 003 and the
two review findings were still open. Their later closure is recorded below.

## Operator Visual Check — Task 003

Accepted - 2026-09-18. The operator completed the
[procedure](../runbooks/dynamic-type-ramp-check.md) in desktop fixture
`/tmp/v4vmm-startup-1nmjz81u`, with the prepared `Élan gyp` title, artist and
description, Null playback and local service stubs.

Evidence:

- `pass, XS,XL,M light,dark`: V1–V4, compact playlist row at XS/XL in both
  themes, with Medium reference observations.
- `all pass`: V5–V6, XS detail in both themes, including readable glyphs,
  the complete expanded Description and reachable controls.
- `XL, XS, M all pass`: V7–V8, XL detail in both themes, including the
  title/metadata hierarchy. XS and Medium were reconfirmed.
- `popover: all four pass`: V9–V12, list and New Playlist draft modes,
  intact text/controls, Back and Escape. The operator was instructed to
  leave the draft unsubmitted and not select a playlist.
- `Show pass`: V13, each card summary retains one clipped line at horizontal overflow.
  It has no ellipsis, wrapping or second line at the bottom edge.
  This cell permits any scale/theme. The exact combination was not supplied.

Build and viewport record:

- Base revision: `b5349f8` (`b5349f8b37cc79ab8ad38a7a4629aa4740a9540c`),
  with task 003 working-tree changes. All three source hashes below matched
  the agent checkout before the wording review. The base revision alone predates task 003.
- Viewport report, retained as supplied: `maximized 1440x900` and
  `~half width`. Exact pane dimensions and whether the latter referred to
  the window or pane were not recorded. No more precise geometry is claimed.
- Starting preferences from the fixture's `case.config`: Dark / Medium.

| Source file | SHA-256 |
|---|---|
| `src/ui/tokens.rs` | `879de5d7adf82e49392d3332c038230330292258ac3fad96956da67ee8337489` |
| `src/ui/layouts.rs` | `1822035a0c4d1a07b7a8dc42e7728442087932444ec0cb7255fc021835884188` |
| `src/ui/composites/show_card.rs` | `237ecb6c3cdef5979e47b25016148e565dba8a40e1314c5ed23a4c36ff410af5` |

| ID | Surface | Scale | Theme | Result / evidence |
|---|---|---|---|---|
| V1 | Music compact playlist track row | x-small | Light | Pass — 2026-09-18 operator response above |
| V2 | Music compact playlist track row | x-small | Dark | Pass — 2026-09-18 operator response above |
| V3 | Music compact playlist track row | x-large | Light | Pass — 2026-09-18 operator response above |
| V4 | Music compact playlist track row | x-large | Dark | Pass — 2026-09-18 operator response above |
| V5 | Music track detail page | x-small | Light | Pass — 2026-09-18 operator reported `all pass` for the XS detail batch |
| V6 | Music track detail page | x-small | Dark | Pass — 2026-09-18 operator reported `all pass` for the XS detail batch |
| V7 | Music track detail page | x-large | Light | Pass — 2026-09-18 operator reported `XL, XS, M all pass` for the detail batch, including hierarchy |
| V8 | Music track detail page | x-large | Dark | Pass — 2026-09-18 operator reported `XL, XS, M all pass` for the detail batch, including hierarchy |
| V9 | Track detail Add to Playlist popover | x-small | Light | Pass — 2026-09-18 operator reported `popover: all four pass`, including list/input modes, Back and Escape |
| V10 | Track detail Add to Playlist popover | x-small | Dark | Pass — 2026-09-18 operator reported `popover: all four pass`, including list/input modes, Back and Escape |
| V11 | Track detail Add to Playlist popover | x-large | Light | Pass — 2026-09-18 operator reported `popover: all four pass`, including list/input modes, Back and Escape |
| V12 | Track detail Add to Playlist popover | x-large | Dark | Pass — 2026-09-18 operator reported `popover: all four pass`, including list/input modes, Back and Escape |
| V13 | Show card summary lines | any, not recorded | any, not recorded | Pass — 2026-09-18 operator reported `Show pass` after the summary overflow check |

Preservation and cleanup evidence:

The operator confirmed fixture removal with `confirmed removed, my bad`.
The supplied command runs `cleanup` only if `inspect` returns success.
The agent accepted preservation by inference from this condition and the removal confirmation.
This inference assumes that the operator used the supplied command.

The conversation does not contain the original inspection JSON.
The agent did not inspect the removed data.

The inspection checks configuration, audio, migrations, bindings and library records.
It requires one playlist, three tracks, three memberships and no residual probes.
It permits changes to workspace preferences.
It rejects changes to the original Dark/Medium theme and scale.
Under the stated assumption, successful inspection also supports preference restoration and the unchanged playlist count.

The later `locate` result was `/tmp/v4vmm-startup-2oo1teip`.
It contained the normal case, no recorded app exit and the unprepared `a.wav` title.
It supplied no evidence for this walkthrough. The agent left it unchanged.

The agent also created `/tmp/v4vmm-startup-b2xn42ks` but did not open the app with it.
The desktop terminal could not access that fixture.
The agent inspected and removed it separately.
Its result did not replace evidence for the operator's fixture.

- [x] The agent accepted preservation of Dark/Medium preferences and unrelated configuration by the stated inference.
- [x] The agent accepted preservation of the library, audio, bindings, migrations and playlist count by the same inference.
- [x] The operator confirmed removal of the inspected fixture.
- [x] The agent removed its unused fixture and left other fixtures unchanged.

V13 supplies the visual proof for task 002's ShowCard single-line correction.
These are type/reservation checks. Unchanged chrome creates no density gate.

## Review Outcome

Accepted - 2026-09-18. R1 and R2 are resolved as recorded below. Numerical
ratification, all three packets' mechanical evidence, V1–V13, preservation
and cleanup are recorded. ADR 0039 is Implemented.

The reservation still uses
a debug assertion plus architecture guards. Possible live layout enforcement
remains a separate design question. This walkthrough did not change the enforcement mechanism.

The original findings and their resolutions follow for traceability.

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

At the time of this probe, the checkout retained uniform type values. These measurements are
evidence of a proposal/reservation conflict, not an already-shipped type
regression or operator visual acceptance. No desktop app was launched.

Required fix: include minimum-height and uncapped row variants in the shared
geometry inventory and five-step consumer checks. Check the full allocated
row, its optional content and controls against the baseline and proposed
ceiling. Report the measured conflict in ADR 0039 before numerical
ratification; resolve the conflicting constraints there rather than silently
allowing larger rows or changing chrome. Do not hand the present zero-conflict
capacity conclusion to task 003.

**Resolved, 2026-09-18.** Task 002 added per-row string-length-independence
tests at the default scale
(`adr_0039_track_row_height_is_independent_of_title_length`,
`adr_0039_queue_row_height_is_independent_of_title_length`,
`adr_0039_playlist_row_body_height_is_independent_of_title_length`) and the
situational guard `adr_0039_fixed_height_rows_keep_single_line_text`. The
capacity claim was narrowed to capped surfaces.
The ADR's capacity audit records minimum heights or no height limit for ListRow, TrackRow, queue, playlist and content_list rows.
The capacity check for capped surfaces does not apply to those rows.

The measured queue-row growth follows its minimum-height behavior as the text line height increases.
The review therefore did not treat that growth as a capacity conflict requiring escalation.
This resolution narrows the finding's original required fix. It does not perform that fix as originally specified.

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

**Resolved, 2026-09-18.** Task 003 scoped
`adr_0039_type_and_chrome_domains_stay_live_and_separate` to each function's
own body: `scaled_px` must call `self.type_multiplier(`, and `scaled` must
call `self.scaled_px(`, as two separate links. The same guard now also
requires the seven ratified `type_endpoints` arms verbatim and forbids
reintroducing the identity placeholder.

The review's own bypass was reproduced against the corrected guard, and it
fails:

```text
src/ui/tokens.rs: ADR 0039 (review R2) `scaled_px` must call
`self.type_multiplier(scale)` in its own body; a bypass (e.g. resolving via
the step-coordinate table directly) would silently revert every non-medium
TYPE step to uniform output with no test failing. Fix: ...
```

At identity, the bypass produced the same values and passed the original ownership guard.
With the ratified curves, that bypass changes type values.
The corrected ownership guard rejects it. The current value tests also detect a return to uniform scaling.
The quoted probe output above records the former diagnostic text.

### Documentation and scope follow-through

The ADR, all three task statuses, phase plan, review, runbook, indices,
AGENTS.md and delivery order record completion. ADR 0039's entry is removed
from the pending-human index. The dated 2026-09-10 reconciliation remains
historical. No inherited gate is closed and no application source changed
in this operator walkthrough.

### Verification

The original review checks below used the production checkout with its
identity type resolver. Task 003's later mechanical evidence is recorded in
the Mechanical Review table above. The walkthrough did not rerun Rust tests.

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

The full unit suite was not run at review time. Temporary probe sources were
removed. Review logs remain under `/tmp/v4vmm-adr0039-review-*.log`. The
desktop app was not launched and no operator acceptance was inferred from
test-context rendering.

Only this existing review document was edited by the
review. No documentation files or folders were created or moved. Root
documents were left in place. No broken review-file link targets were found.

Full-suite verification, 2026-09-18: `cargo test`, full suite, 1499 unit
tests, 0 failed. `cargo test --test architecture_tests`, 263 guards, 0
failed, up from 260 after task 001. Re-run and confirmed as part of the
documentation reconciliation that closed R1 above.

ADR 0039 is Implemented with the evidence recorded above. Final documentation
diff and local link checks are Green. No files or folders were created or
moved during the acceptance-record update.

### Wording Review — 2026-09-18

Approved for commit under the `asd-ste100` skill's structural rules and plain-word guidance.
This approval covers the revised ADR 0039 documentation, Rust comments and guard diagnostics.
Procedures and diagnostics use Strict mode. Explanatory prose uses STE-flavored mode.
The review did not check ASD's official dictionary and does not certify full ASD-STE100 compliance.

The revised wording separates operator observations from inferred preservation.
It states the inference's assumption, retains the viewport limits and assigns ShowCard's visual proof to V13.
It also corrects the stated endpoint direction, capacity coverage and resolver-guard evidence.
Code snippets, recorded operator responses and the historical probe output remain unchanged.

Source edits during this wording review changed comments and diagnostic messages only.
No calculation, font value, layout rule or test assertion changed.
The source hashes above identify the files inspected before these wording edits.
No repeat visual inspection is required for this wording review.

Green: formatting, `cargo check`, Clippy with warnings denied and the focused `adr_0039_` tests.
The focused tests passed 18 unit tests and 4 architecture guards.
The normal desktop binary was rebuilt after the tests. Build: Green.

Green: documentation whitespace checks and all 335 local links in the changed Markdown files.
No documentation file or folder was created or moved. This review made no commit.

## Operator visual check

Complete - 2026-09-18. No further operator step is due for this packet.
The following steps and linked procedure remain available for regression.
Tasks 001/002 have no separate visual gate.

1. For a future regression check, use a Linux desktop terminal and the
   [dynamic type procedure](../runbooks/dynamic-type-ramp-check.md). It names
   the setup commands, data preparation and all thirteen observations. Start
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
