# ADR 0069 Task 001: Grouped Settings Foundation

Status: Complete - 2026-09-11. Mechanical checks Green; operator V1–V3,
saved-value assertions and all preservation inspections accepted. Fixture
cleanup is confirmed. The
[operator procedure](../runbooks/settings-foundation-check.md) remains a regression check.

## Goal

Replace the long Settings form with General, Library and Diagnostics group
navigation, using existing controls and behavior. Preserve edits across group
and app-section changes. Give reports a direct Diagnostics destination.

This is the first independent packet under
[ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md) and its
[phase plan](../plans/adr-0069-settings-presets-phase-plan.md). Do not implement
the shared transaction editor, mode/resource selectors, presets or audio here.

## Files To Inspect

- `AGENTS.md`, `.github/copilot-instructions.md`, `docs/adr/README.md`
- `src/app.rs` — Settings inputs, rendering, save/defaults, cached files and screen mount
- `src/app/capabilities.rs`, `src/app/keyboard.rs`, `src/app/menu.rs`
- `src/config.rs`, `src/view_models/mod.rs`, `src/view_models/cached_files.rs`
- `src/view_models/startup/capabilities.rs`, `src/application/capability.rs`
- `src/ui/primitives/`, `src/ui/composites/`, `src/ui/tokens.rs` — inspect actual navigation/form owners before adding one
- `tests/architecture_tests.rs` — existing Settings, token, focus, runtime and report guards
- `docs/runbooks/startup-recovery-fixture.py`, `docs/runbooks/inherited-ui-checks.md`
- `docs/troubleshooting/column-text-truncation.md`

## Files Likely To Change

- `src/app.rs`, `src/app/settings.rs` (new), `src/app/capabilities.rs`
- `src/view_models/settings.rs` (new), `src/view_models/mod.rs`
- `src/config.rs` (save-sequence regression test only)
- Existing navigation/form primitives and tokens only where shared behavior needs extension
- `src/ui/composites/settings.rs` (new only if needed), `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`
- `docs/runbooks/settings-foundation-check.md` (new; operator procedure)
- This packet, its phase plan/review checklist, delivery index, source map,
  `AGENTS.md` and `docs/pending-human-checks.md` when the visual gate is runnable

## Do Not Touch

Configuration keys, ordinary writer semantics, database schema/content, preset
files, service adapters, audio drivers, playback commands, other repositories,
unrelated UI polish or the accepted Event/Producer/Publisher controls in Show.
No new background discovery, report polling, core-path retarget behavior,
reset behavior or runtime apply lifecycle. Never run the app as an agent.

## Constraints And Implementation Steps

1. Add a renderer-independent Settings group model for the three delivered
   groups. General owns theme/scale; Library owns endpoint/music folder/flac;
   Diagnostics owns existing background reports and cached-file maintenance.
   Keep group labels, selected state and action presentation in the view model.
   Render navigation with accessible typed actions and shared tokens.

2. Extract Settings composition to `src/app/settings.rs`. Reuse existing field
   controls, cached-file view model, report composites and commands. Do not copy
   the old form and leave a second route alive. Screens must not invent
   enablement from strings or filesystem checks. Preserve existing command-side
   save guards; presentation state is not authority to bypass them.

3. Keep the selected group and input entities alive for the app session.
   Initial ordinary Settings entry opens General; subsequent ordinary entry
   restores the selected group. Group switching and returning from Music/Show
   preserve unsaved text and existing appearance previews without saving.
   Do not add a persisted group-selection key. Maintain a usable focus target
   when the focused field is unmounted; preserve ADR 0067 app shortcuts.

4. Keep Save and Use Defaults shared across existing editable groups. Label
   their scope so the operator knows they act on General and Library together.
   Preserve their existing validation, application and global-default behavior.
   Do not introduce Cancel/Apply or claim transactional draft behavior here.
   Show existing field/command errors at their relevant surface; do not move
   every useful correction message into Diagnostics.

5. Make Open report enter Settings and select Diagnostics in one dispatch.
   Preserve reports, their actual recorded times, copy actions and unavailable
   runtime behavior. General/Library must remain reachable with long reports.
   A tab switch does not start a new cache scan; retain existing query caching
   and explicit refresh semantics. Show no queue, transport or broadcast-status
   panel in Settings. Do not add Live Metadata/Audio tabs or preset controls
   before their real editors exist.

6. Bound the content scroll area beneath the group navigation. Keep labels and
   actions reachable at narrow widths and larger existing UI scales, using
   named dimensions. No column `truncate()` fix, raw glyph or ad hoc per-screen
   style. Extend a shared owner if the same geometry is needed repeatedly.

7. Add focused behavioral tests and the situational ADR 0069 ownership guard
   below. Update still-valid existing guards to the new owner without weakening
   their requirements. Publish the operator procedure and build before opening
   this packet's visual gate in all three indexes.

## Acceptance Criteria

Mechanical; implemented proof owners and exact verification are recorded below.

| ID | Proof owner | Required assertion |
|---|---|---|
| M1 | Settings VM tests | General/Library/Diagnostics expose stable labels, selected state, typed accessible navigation and defined field/report membership |
| M2 | Settings navigation/dispatch tests | Group changes and ordinary Settings re-entry retain group/input values and dispatch no save, defaults, service or playback command; explicit Open report selects Diagnostics |
| M3 | Existing save-command tests and focused dispatch coverage | Shared Save/Use Defaults still address all existing settings once; invalid config cannot bypass current writer guards through a new group route |
| M4 | Report/cached-files owner tests | Group navigation preserves issue identity/results and cannot schedule a second cache query when cached data is valid or one query is in flight |
| M5 | New situational guard `adr_0069_settings_group_ownership` | Settings model has no GPUI dependency; thin app composition reaches one live shared form/navigation path and reuses current save/report owners; no new persistence schema |

Visual V1–V3 below are separate gates. Mechanical tests cannot establish
readability, focus delivery or scrolling in the running app.

## Test Commands

Run focused Settings/dispatch tests during edits, then the required checks:

```bash
cargo fmt -- --check
cargo check --locked --offline --quiet
cargo test --locked --offline --quiet adr_0069_
cargo test --locked --offline --test architecture_tests --quiet
cargo clippy --locked --offline --quiet -- -D warnings
cargo build --locked --offline --quiet
git diff --check
```

Also run affected existing config, capability, cached-file and keyboard tests.
Record exact filters and counts; do not use an empty test selection as evidence.
No unrelated lint cleanup and no agent GUI execution.

## Implementation And Mechanical Evidence — 2026-09-11

- `src/view_models/settings.rs` owns the three groups, field/report membership,
  labels, selected state, accessible actions and navigation/edit effects.
- `src/app/settings.rs` composes the one live Settings route and wires existing
  persistent input entities, appearance previews, focus and commands.
  `src/ui/composites/settings.rs` owns wrapping action rows, selected checks,
  field layout and the bounded scrollbar beneath fixed navigation. It reuses
  the Button primitive, Input control and existing dimension/color/type tokens.
- `src/app.rs` retains the original Save authority. Use Defaults still sets all
  five existing fields and calls that same Save once. Both editable groups state
  this scope. Command errors remain visible in the editable group.
- Open report selects Diagnostics before mounting Settings. Existing report
  projection/copy/time/retry owners remain intact. `CachedFilesVm::enter` reuses
  ready/in-flight observations; mutations and runtime recovery still invalidate
  explicitly. Failed database reads retain the existing re-entry retry.
- No configuration key, schema, writer, actor, service operation or audio owner
  was introduced. There was no phase expansion and no agent GUI execution.

Proof mapping: M1 uses `adr_0069_group_contract_has_stable_membership_and_accessible_actions`.
M2 uses `adr_0069_navigation_and_reentry_emit_no_edit_effect` plus the ownership
guard, which pins session-owned input entities and forbids field writes in
navigation dispatch/re-entry. M3 uses
`adr_0069_edit_actions_have_one_shared_whole_form_route`, the same guard's Save
and defaults call/field assertions, and existing guarded config-save tests.
M4 uses `adr_0069_cache_entry_reuses_valid_and_in_flight_observations` and
`adr_0069_report_navigation_preserves_issues_times_and_retry_result` with the
existing runtime/report guards. M5 is `adr_0069_settings_group_ownership`.
The former form-width, report and runtime guards now follow the new owners;
the screen is registered with the durable architecture backstop.

| Exact verification | Result |
|---|---|
| `cargo test --locked --offline --quiet adr_0069_` | Green: 6 unit tests and 2 architecture guards |
| `cargo test --locked --offline --test architecture_tests --quiet` | Green: 237 guards |
| `cargo test --locked --offline --lib --quiet config::` | Green: 50 tests, including ordinary-save rejection/preservation and subsequent workspace saves |
| `cargo test --locked --offline --lib --quiet capability` | Green: 2 tests |
| `cargo test --locked --offline --lib --quiet capabilities` | Green: 4 tests |
| `cargo test --locked --offline --lib --quiet cached` | Green: 19 tests |
| `cargo test --locked --offline --lib --quiet ui::primitives::button` | Green: 9 tests after the focus correction |
| `cargo test --locked --offline --lib --quiet app::keyboard` | Green: 9 tests |
| `cargo test --locked --offline --lib --quiet app::menu` | Green: 4 tests |
| `cargo check --locked --offline --quiet` | Green |
| `cargo clippy --locked --offline --quiet -- -D warnings` | Green |
| `cargo fmt -- --check` | Green |
| `cargo build --locked --offline --quiet` | Green |
| `git diff --check` | Green |

These selections overlap; their counts are not a unique-test total. The empty
binary test target is not evidence. Mechanical dispatch/ownership proof does
not establish delivered focus, text retention on screen or visual readability;
operator V1–V3 acceptance and fixture cleanup are recorded below.

Fixture procedure verification: Green for `startup-recovery-fixture.py setup`,
then `mode` and `inspect` for `normal`, `endpoint-and-player-unavailable` and
`runtime-and-cache-unavailable`, followed by `cleanup` on the newly created
agent fixture. The three runbook Python assertion snippets parse. These CLI
checks establish fixture preparation/inspection only; no GUI was launched and
those CLI checks alone accepted no operator gate.

Documentation: added the operator runbook; updated this packet, the ADR/status
index, phase plan, review checklist, delivery order, pending-human index,
documentation index, source map and AGENTS.md. No files moved and no folders
created. Existing canonical root documentation stays in place.
All 214 relative file links in the 11 changed Markdown files resolve; no broken file
links were found.

## Operator Visual Check

Operator evidence - 2026-09-11: initial Ctrl+Comma entry to General, mouse
navigation through General/Library/Diagnostics, expected group contents and
selected-group indication passed. After the corrections below, the operator
confirmed visible group focus, retained focus after Enter/Space activation,
and forward Tab and reverse Shift+Tab traversal. Light/M, Dark/M, Light/XL and
Dark/XL passed at normal and narrow widths: all three groups, fixed navigation,
scrolling, reachable labels and controls without clipping or overlap, and
visible focus on appearance choices and Save/Use Defaults. Those last two
buttons were not activated. Dark/M, Light/XL and Dark/XL also passed keyboard
traversal and group activation. The operator confirmed that clicking General
from a focused Library endpoint field transfers focus to General, and the next
Tab reaches Library. V1 passed. V2, V3, preservation and fixture cleanup results
follow below.

V2 navigation evidence - 2026-09-11: the unsaved endpoint suffix
`/unsaved-settings-check` survived General/Library switching, Music with
Ctrl+1 followed by Ctrl+Comma, and Show with Ctrl+2 followed by Ctrl+3.
Both section returns restored Library and its edited text. General retained
the Dark/XL preview. No failures were reported for the subsequent field-focused
shortcut and Settings operational-panel walkthrough when the operator returned
the requested inspection output. Save and global-default results are recorded below.

V2 unsaved-file inspection - 2026-09-11: the operator's normal-case result
reported `config_bytes_unchanged: false`, with
`normal_workspace_preferences_only: true` and `config_preserved: true`.
Only permitted workspace preferences changed; the Settings edits were not
persisted. Music, migration records, bindings, library and tool blockers were
preserved. The fixture retained one playlist, three tracks, three playlist
memberships, bindings `a.wav`/`b.wav`/`c.wav` and migrations 1–11. No residual
music probes or database probes remained. Save/default, V3, final preservation
and fixture cleanup results follow below.

V2 Save-from-General failure - 2026-09-11: the operator's value assertion
failed. The saved file contained `theme_profile = "light"` and
`ui_scale = "large"`, but retained endpoint `http://127.0.0.1:9` and omitted
`flac_path`. The music directory was still the fixture's `music` directory.
This does not establish whether the Library edits were lost before or during
Save. The failed assertion ran before creating `settings-save-general.toml`,
so the dependent Library-save check waited for the verified replay below.

V2 pre-save replay - 2026-09-11: the operator re-entered the exact endpoint
`http://127.0.0.1:9/settings-foundation` and FLAC path `/usr/bin/flac`, changed
General from Dark/XL to Light/L, and returned to Library without saving.
Both values remained visible. The operator then reported the post-Save check
passed: Save from General completed and both exact Library values remained
visible on return. The result message was not transcribed. After quitting, the
operator returned Green for the saved-file assertion: endpoint, music directory,
FLAC, Light and L all matched the requested values; every other setting except
workspace preferences matched the baseline. The assertion created
`settings-save-general.toml`. Save from General is accepted on this verified
replay. The earlier discrepancy was not reproduced; its cause is not established.

V2 Save-from-Library evidence - 2026-09-11: the operator selected Dark/XL in
General, confirmed the Library fields remained correct and saved from Library.
After quitting, the saved-file assertion returned Green: the document matched
`settings-save-general.toml` with only the intended Dark/XL changes and permitted
workspace preferences. Library values and unrelated settings were preserved.
The assertion created `settings-save-library.toml`. Save from Library is accepted.

V2 Defaults screenshot - 2026-09-11: Library shows the default endpoint,
the fixture's `home/V4Vmusic` path and the empty FLAC field's placeholder.
The app reports that it applied the saved settings but could not prepare
`home/V4Vmusic/artists` because the path does not exist. Inspection found a
runbook prerequisite omission: fixture setup creates `music` and `home`, but
not `home/V4Vmusic`; the existing save path creates only the `artists` child.
No production change is needed to explain this result. Step 6 now prepares
the default root for future runs and documents this observed failure if that
setup was omitted. The screenshot alone did not establish General's Dark/M
selection or saved-file contents; the follow-up verification below does.

V2 Defaults verification - 2026-09-11: the operator returned Green after the
General Dark/M check and saved-defaults assertion. The document contained the
default endpoint, fixture `home/V4Vmusic` directory, Dark and M, with `flac_path`
absent. Unrelated settings matched the baseline except permitted workspace
preferences. The assertion created `settings-defaults.toml`. Global defaults
are accepted for both groups. Baseline restoration and preservation results
follow below.

V2 preservation evidence - 2026-09-11: after restoring the normal fixture
baseline, the operator supplied an inspection with `config_bytes_unchanged`
and every preservation flag true. One playlist, three tracks, three playlist
memberships, bindings `a.wav`/`b.wav`/`c.wav` and migrations 1–11 remained.
Music and database probe counts were zero. Intentional Save/default values
were verified and recorded in their snapshots before restoration. V2 is
accepted. V3, its final preservation inspection and fixture cleanup follow below.

V3 Diagnostics screenshot - 2026-09-11: Diagnostics is selected and shows the
configured-player preparation failure and invalid `musicindex_endpoint` issue.
Both name the fixture's `config/v4vmm/config.toml` and carry the recorded time
`2026-09-11 16:53:52 UTC`. Runtime-start and thumbnail-cleanup observations
are also visible. The endpoint report explains that ordinary configuration
saves remain paused. Copy report is visible, with Cached files beginning below
the visible report. The operator subsequently confirmed that Open report went
directly to Diagnostics, report identity/times survived group changes, and
Copy report retained both issues, timestamps and complete paths at normal and
roughly 560-pixel widths. Cached files was reachable by scrolling while group
navigation remained visible; deletion controls were not used. These V3 report
checks pass for the paired endpoint/player failure case. Rejected-save,
preservation and unavailable-runtime results follow below.

V3 guarded-save/preservation evidence - 2026-09-11: the operator returned the
requested inspection without reporting any failure of Library Save, General
Save or General Use Defaults to show the expected refusal error in the editable
group. In `endpoint-and-player-unavailable`, `config_bytes_unchanged` and every
preservation flag were true. One playlist, three tracks and memberships,
original bindings and migrations 1–11 remained; music/database probes were
absent, and the runtime blocker was preserved. The V3 paired-case report,
guarded-save and preservation checks are accepted.

V3 unavailable-runtime evidence - 2026-09-11: the operator confirmed all checks
passed in `runtime-and-cache-unavailable`. Open report went directly to
Diagnostics with both runtime and cache-cleanup failures; Copy report retained
both failures in full. General/Library remained reachable, and returning to
Diagnostics preserved the reports and recorded times. Cached files explained
that background tools were unavailable. V3 visual checks passed.

V3 final preservation evidence - 2026-09-11: after quitting the
`runtime-and-cache-unavailable` case, the operator supplied an inspection with
`normal_workspace_preferences_only: true`, `config_preserved: true` and every
other preservation flag true. Configuration bytes differed only in permitted
workspace preferences. Music, one playlist, three tracks and memberships,
bindings `a.wav`/`b.wav`/`c.wav` and migrations 1–11 remained; both probe checks
were empty. V3 is accepted.

Fixture cleanup evidence - 2026-09-11: the operator returned
`Removed fixture: /tmp/v4vmm-startup-duaionfq` after the runbook cleanup command.
This removes the Settings fixture and its saved config evidence. Task 001 is
complete; no inherited recovery, playback or Music gate is closed by this result.

The operator also reported no double/triple-click select-all and no middle-click
selection paste. The pinned input component provides double-click word selection
and Ctrl+A, but has no triple-click select-all or middle-button paste handler.
These existing input behaviors are recorded in the
[Linux text-input backlog](../plans/hig-product-polish-backlog.md#11-linux-text-selection-and-primary-paste).
No input behavior change or new acceptance gate is added to this packet.

Save diagnosis: the shared handler reads all three retained Library input
entities; both General and Library route Save to this handler. The new
situational ADR 0069 regression test
`adr_0069_saved_settings_survive_subsequent_workspace_saves` supplies the exact
requested values to the existing writer, then saves workspace layout and
preferences and verifies the complete non-workspace document. It is Green;
the configuration suite is Green (50 tests), as are all 237 architecture guards,
`cargo check --locked --offline --quiet`,
`cargo clippy --locked --offline --quiet -- -D warnings`, `cargo fmt -- --check`
and `git diff --check`. No production behavior was changed during this diagnosis.
The runbook now checks visible Library values after appearance changes and after
Save, followed by the exact saved-file assertion. That complete replay passed
as recorded above. V1 acceptance was retained; later checks and fixture cleanup
also completed.

The initial keyboard check failed: the operator could see focus entering/leaving
search, but could not identify the focused group button. The shared Button
primitive registered Tab stops without a focus style. The correction adds a
rounded outline on enabled `on_activate` buttons, using `SemanticColor::Focus`
and scaled `CONTROL_FOCUS_RING_WIDTH`. A transparent border reserves the same
space when unfocused. Selected-group checks remain separate from keyboard focus.
The situational ADR 0069 guard
`adr_0069_keyboard_buttons_show_focus_without_layout_shift` covers the shared
focus/activation path, token use, stable geometry and disabled controls.
Correction verification is Green: check, Clippy, format, debug build, 237
architecture guards, 9 Button tests and 9 app keyboard tests. No GUI was run
by the agent. The operator's group keyboard retest passed as recorded above.
No new documentation files or folders were needed for this correction; the
packet, runbook, review and gate indexes were updated.

During the keyboard walkthrough, the operator asked whether Enter/Space should
reset focus. Inspection confirmed that group activation called
`focus_active_tab`, returning focus to the app toolbar. The group handler now
leaves focus on the activated button. Group IDs remain stable across selection
changes; GPUI's existing mouse-down behavior focuses a clicked group before
its previous input is unmounted. The ownership guard replaces its incorrect
requirement for a toolbar focus reset with retention and mouse-focus assertions.
The group VM test also checks stable IDs across every selection. The operator
subsequently confirmed activation-focus retention and Shift+Tab traversal.
Follow-up verification is Green using the table's check, Clippy, format, build and diff checks,
`adr_0069_` (then 5 unit tests and 2 guards), the full 237-guard suite, and
`app::keyboard` (9 tests).
Final verification of `adr_0069_` is Green with 6 unit tests and 2 guards after
the Save/workspace-save regression test was added.

Accepted - 2026-09-11. [Grouped Settings Foundation Check](../runbooks/settings-foundation-check.md)
retains exact setup, launch, inspection and cleanup commands for regression checks. It reuses the startup
fixture contract with a fresh disposable fixture. Do not clean up or repurpose
the operator's retained ADR 0066 task 004 fixture.

Prerequisites: Linux desktop, rebuilt debug binary, isolated fixture with local
tracks and existing invalid-optional configuration cases. No audio hardware,
reachable MusicIndex endpoint, running broadcast service or playback is needed.

1. V1: Open Settings with Ctrl+Comma. Walk the three groups with mouse and
   keyboard in Light/Dark, normal/narrow widths and an existing larger scale.
   Inspect labels, selected state, scroll bounds and reachable controls. Clipped
   tabs, inaccessible actions or reports displacing group navigation fail.
2. V2: Edit disposable endpoint text without saving, switch groups and visit
   Music, then return. Text and selected group remain. Check Ctrl+F and section
   shortcuts from a field. Verify the shared Save/defaults scope using the
   disposable config and inspection; hidden-group values must not reset.
3. V3: Use an invalid-optional fixture and Open report from its notice. It must
   reach Diagnostics immediately, retain all issues and permit navigation back
   to editable groups. Check report copying, scrolling and cached-file access.
   Close the app and inspect configuration/music preservation before cleanup.

The runbook must name any intended config edits separately from preservation
assertions, and clean up only its own fixture. Acceptance is recorded in the
packet Status and delivery Progress row; this completed gate is removed from the
pending-human index. Unrelated inherited checks retain their separate gates.

## Final Report, Escalation And Rollback

Report changed owners, behavior, exact verification and deviations. Say Green
for passed checks. End with Operator visual check commands or their runbook
link, prerequisites and cleanup; leave acceptance open until walked.

Stop this packet's expansion and document a plan correction if it requires a
new config key, writer, background actor, service operation or audio owner.
Routine composition choices do not need renewed permission. Roll back code as
one change if needed; preserve all operator files and outstanding evidence.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This entire packet and its Files To Inspect.
- ADR 0069, its phase plan/review checklist and the delivery order.

Goal:
- Group existing Settings under General, Library and Diagnostics.

Constraints:
- Follow steps 1–7 and shared owner, token, action and focus contracts.
- Preserve current saves, global defaults and preview behavior in this packet.

Do not touch:
- Configuration schema, preset persistence, service/audio behavior or other repositories.

Acceptance criteria:
- Prove M1–M5 at their named owners; publish V1–V3 without claiming operator acceptance.

Test commands:
- Run the Test Commands section and affected owner tests; never run the app.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
