# ADR 0069: Settings And Presets Review Checklist

## Status And Scope

Task 001 complete - 2026-09-11. Mechanical review Green; V1–V3, saved-value
assertions and all preservation inspections accepted. Fixture cleanup is
confirmed. Later implementation has not started. Review each
packet against [ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md),
the [phase plan](../plans/adr-0069-settings-presets-phase-plan.md) and its diff.

Task 001's [operator procedure](../runbooks/settings-foundation-check.md) remains a regression check.
Later phases require their own packets
and completed dependencies. This checklist supplies review coverage, not a
claim that future test names or controls exist.

## Decision Coverage

| Invariant | Mechanical proof required | Operator proof when delivered |
|---|---|---|
| 1. Inert group navigation | VM/dispatch retains edits and selected group; no saves or resource calls | Task 001 V1/V2: tabs, focus, scroll and unsaved text |
| 2. Explicit component changes | Mode/default and mask tests retain every unselected component; incompatible combinations explain conflicts | Later metadata/preset packet: review compatible and conflicting combinations |
| 3. Draft/saved/running distinction | Recall/save fakes perform no Start/Stop/Attach/audio calls; stale apply rejection and failed-apply state tests | Later editor/resource packet: draft, saved and running effect explanations |
| 4. One guarded writer | Shared Settings/recovery commands; identity/revision, backup, validation and conflict preservation tests | ADR 0066 editor checks plus Settings integration proof |
| 5. Snapshot recall | Save mask, absent/unchecked state, sequential overlap, immutable source semantics and Custom/provenance tests | Later preset packet: assemble components from two named presets |
| 6. Excluded sensitive/session data | Serialization allowlist and fixture inspection exclude secrets, cue/session state, library identity and health | Preset inspection after operator save/recall; no service activation |
| 7. Independent audio scope | Backend-specific destination decoding, missing-resource preservation, ADR 0068 owner and route tests | Deferred: separate program/monitor capture, missing-output behavior |
| 8. Live typed UI owners | Situational ADR 0069 ownership guard; reachable modules, typed actions and tokens | Each delivered surface checked at normal/narrow widths and both themes |

## Task 001 Review

- Verify only General, Library and Diagnostics are introduced. All existing
  settings, report and cached-file actions remain reachable.
- Trace field/action facts to the view model and geometry to shared UI owners.
  Reject renderer-side availability checks or parked future modules.
- Trace Save and Use Defaults back to the existing writer. Both retain their
  existing whole-form scope, visibly described across the editable groups.
  Do not claim the later guarded editor or Cancel/Apply behavior has shipped.
- Check Open report chooses Diagnostics in one action and ordinary Settings
  entry restores the session's selected group. No input loss or implicit save.
- Check no navigation key, TOML table or preset directory has been persisted.
- Check cached-file query ownership and background-runtime failure behavior
  survive composition changes. No synchronous service/device discovery.
- Verify actual M1–M5 tests and guard names, affected owner tests and strict
  repository checks. Record V1–V3 as open until the operator supplies evidence.
- If Settings scroll evidence also satisfies part of ADR 0030, record that
  exact subclaim. Do not close remaining Music scroll or unrelated checks.

## Later Phase Review

- Confirm prerequisite completion before implementation, especially ADR 0066's
  shared writer/transition and full configuration-format gate. No implied waiver.
- Inspect the versioned config/preset schema and existing-config fixtures before
  the first write. Reject unsupported versions without partial application.
- Trace metadata selectors to existing service/registry identities. No tokens
  in TOML, preset content, logs, debug output or command arguments.
- Test component coherence, conflicting cross-component combinations, missing
  resources and repeated recall. Reject automatic mask expansion or defaulting.
- Trace explicit apply effects and failed/stale results through the actual
  owners. Saving cannot activate a new chain or retarget an active player.
- Keep Mixxx audio external. Audio configuration requires independent Show and
  audition ownership; the current shared-player path cannot satisfy that proof.

## Review Result Format

For each implemented packet, record:

1. Pass/fail for the reviewed diff and scope.
2. Required fixes, including architectural drift and missing tests.
3. Optional improvements kept outside the packet.
4. Mechanical evidence and remaining operator criteria, each named separately.
5. Whether the implementation can be merged and whether the next packet's
   actual prerequisites are met. Never equate these two decisions.

Update the packet, phase plan, ADR partial status, delivery index and any
runnable pending-human check together. Existing playback gates remain deferred;
this Settings decision provides no evidence that playback has passed.

## Task 001 Result — 2026-09-11

Mechanical review: Green. The implemented diff provides the three delivered
Settings groups, one shared form/navigation path, typed actions and retained
input entities. Settings navigation produces no edit effect; Open report chooses
Diagnostics before mounting. Existing Save/default behavior and writer guards
remain, with explicit whole-form scope. Cached navigation reuses observations;
mutation and runtime recovery keep their existing refresh paths.

The packet records [exact commands, counts and M1–M5 proof owners](../tasks/adr-0069-task-001-grouped-settings-foundation.md#implementation-and-mechanical-evidence--2026-09-11).
No architectural drift was found in mechanical review. The operator's V2 Save
replay passed both visible-value and saved-file checks below. No new persistence
schema, service/audio owner or unrelated polish was added. Optional later editor,
metadata and preset work stays in its phases.

Task 001 is accepted, including the operator procedure, preservation inspections
and fixture cleanup recorded below. Phase 002 is not ready: ADR 0066 tasks
005–007 remain prerequisites. No later phase was started.

Operator V1 update - 2026-09-11: initial entry/grouping passed, but keyboard focus
was not visible outside the search field. The shared Button primitive now adds
a scaled, theme-aware outline with stable geometry. A situational ADR 0069 guard
pins that shared path. The operator's group keyboard retest passed after this
correction and the activation follow-up below. V1–V3 are accepted.

The activation follow-up removes the group handler's toolbar focus reset. The
existing ownership guard now requires focus retention, and the VM test checks
stable group IDs. GPUI mouse-down focus remains available when hiding a focused
Library field. The operator confirmed group focus visibility, Enter/Space focus
retention and Tab/Shift+Tab traversal. Light/M, Dark/M, Light/XL and Dark/XL
passed at normal/narrow widths: all groups, fixed navigation, scrolling,
reachable unclipped controls and focus on appearance choices and Save/Use
Defaults. Dark/M, Light/XL and Dark/XL also passed keyboard traversal and group
activation. Mouse focus transfer from a Library endpoint field to General,
followed by Tab reaching Library, passed. V1 is accepted - 2026-09-11.

Operator V2 update - 2026-09-11: unsaved endpoint text survived group changes
and round trips through Music and Show. Both section returns restored Library,
and General retained the Dark/XL preview. No field-shortcut or Settings panel
failure was reported with the requested normal-case inspection. That inspection
passed: only permitted workspace preferences changed, all preservation flags
were true, and no music or database probes remained. Subsequent Save/default
and preservation results follow below. Final preservation and fixture cleanup
follow V3.

Operator V2 Save failure - 2026-09-11: the saved configuration retained the
original endpoint and omitted FLAC, while Light/L persisted. Whether the
Library values disappeared before or during Save is not yet established.
The exact-value writer/workspace-save regression test is Green, as are all
50 configuration tests; those tests alone did not reproduce the operator failure.
No production behavior changed. Runbook step 4 now checks visible Library
values before and after Save; verified replays follow below.

Operator pre-save replay - 2026-09-11: both exact Library values survived the
Dark/XL to Light/L appearance change and remained visible on return to Library.
The operator subsequently reported the post-Save check passed, with both exact
values retained after saving from General. The message was not transcribed.
Saved-file verification after quitting then returned Green: all five requested
values matched, unrelated settings were preserved and the General-save snapshot
was created. Save from General is accepted on the verified replay. The earlier
discrepancy was not reproduced, and its cause remains unknown. The reported triple-click
and primary-selection paste gaps are
tracked in the [Linux text-input backlog](../plans/hig-product-polish-backlog.md#11-linux-text-selection-and-primary-paste);
the subsequent Save/default results follow below.

Operator Save-from-Library result - 2026-09-11: the saved-file assertion returned
Green after selecting Dark/XL in General and saving from Library. Library values
and unrelated settings matched the General-save snapshot; only the intended
appearance values and permitted workspace preferences changed. The Library-save
snapshot was created. Save from Library is accepted.

Operator Defaults screenshot - 2026-09-11: Library reset values are visible,
along with an applied-settings notice reporting failure to prepare the default
artists directory. The fixture did not create `home/V4Vmusic`, and the existing
save path only creates its `artists` child. Runbook step 6 now declares and
prepares this directory prerequisite; it also explains how to finish value
verification when that setup was omitted. No production change was made.
The screenshot alone did not establish General's Dark/M selection or saved-file
contents. The operator subsequently returned Green after the General check and
defaults assertion: the four expected default values matched, FLAC was absent,
unrelated settings were preserved and `settings-defaults.toml` was created.
Global defaults are accepted. The operator then restored the normal baseline
and supplied an inspection with unchanged configuration bytes, every preservation
flag true, one playlist, three tracks and memberships, the original bindings
and migrations, and no probe leftovers. V2 is accepted - 2026-09-11.
V3, final preservation inspection and fixture cleanup results follow below.

Operator V3 screenshot - 2026-09-11: Diagnostics displays the configured-player
preparation failure and invalid endpoint issue, both with recorded time
`2026-09-11 16:53:52 UTC` and the fixture configuration location. Runtime and
thumbnail observations and Copy report are visible; Cached files starts below
the visible report. The operator then confirmed direct Open-report routing,
report/time retention through group changes, complete copied text at normal
and roughly 560-pixel widths, and scrolling to Cached files while group
navigation stayed visible. These checks pass for the paired endpoint/player
case. The operator then returned the requested paired-case inspection with no
refusal-error failure reported for Library Save, General Save or General Use
Defaults. Configuration bytes were unchanged, all preservation flags were true,
original playlist/track/binding/migration facts remained, and there were no
probe leftovers. The paired report/guarded-save case is accepted - 2026-09-11.
The operator then confirmed all visual checks in `runtime-and-cache-unavailable`:
direct Diagnostics routing, both complete failure reports in copied text,
General/Library access, report/time retention and unavailable cached-file
status. V3 visual checks passed. The operator's final-case inspection then
confirmed that only permitted workspace preferences changed, every preservation
flag was true, original library/binding/migration facts remained and no probes
were left. V3 is accepted. The operator then confirmed cleanup with
`Removed fixture: /tmp/v4vmm-startup-duaionfq`. Task 001 is complete, including
removal of its fixture and saved config evidence. Inherited recovery, playback
and Music gates retain their separate requirements.
