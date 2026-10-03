# ADR 0069 Task 002: Shared Guarded Settings Editor

Status: Ready - 2026-09-18. Implementation has not started. Task 001 and
ADR 0066 tasks 005–007 are complete with acceptance. Operator criteria below
are prospective. They become runnable after implementation and mechanical checks.
ADR 0066 task 004 retains its separate gate.

## Goal

Give existing Settings fields a retained draft, guarded Save and explicit Cancel.
Use the configuration editor that Settings and recovery already share.
Report saved values separately from values used by running resources.

This packet defines phase 002 of the
[Settings plan](../plans/adr-0069-settings-presets-phase-plan.md).
[ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md) owns the target contract.
[ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md) owns preservation,
repair, write admission and resource checks. No configuration format changes here.

## Files To Inspect

- `AGENTS.md`, `.github/copilot-instructions.md`, `docs/adr/README.md`
- `src/config.rs`, `src/config/correction.rs`
- `src/application/commands/maintenance.rs`, `src/application/session_lifecycle.rs`
- `src/application/capability_recovery.rs`, `src/application/capability_recovery/setup.rs`
- `src/view_models/settings.rs`, `src/view_models/startup/correction.rs`
- `src/view_models/startup/capabilities.rs`, `src/view_models/startup/converter.rs`
- `src/presentation/configuration_editor.rs`, `src/presentation/maintenance_executor.rs`
- `src/app.rs`, `src/app/settings.rs`, `src/app/startup.rs`, `src/app/capabilities.rs`, `src/app/resize.rs`
- `src/ui/composites/settings.rs`, `src/ui/composites/maintenance_forms.rs`, `src/ui/composites/maintenance_page.rs`
- `tests/architecture_tests.rs`
- `docs/runbooks/startup-recovery-fixture.py`, `docs/runbooks/test_startup_recovery_fixture.py`
- `docs/runbooks/startup-recovery-check.md`, `docs/runbooks/settings-foundation-check.md`
- `docs/troubleshooting/column-text-truncation.md`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/adr/archive/0039-dynamic-type-ramp.md`
- `docs/adr/archive/0067-platform-shortcut-modifiers.md`
- `docs/adr/archive/0071-shared-text-selection-and-linux-primary.md`
- `docs/adr/0074-repair-and-diagnostics-pages.md`

## Current Owners And Gaps

| Owner | Current behavior | Work in this packet |
|---|---|---|
| `TopApp::save_settings` | Calls `save_app_settings`, then updates runtime fields | Replace the form save route with the shared editor command. Separate persistence from resource application. |
| `CorrectionSource` and `CorrectionDraft` | Retain source identity and bytes. Validate focused edits. Preserve and atomically replace the file. | Reuse this persistence authority for explicit Settings saves. |
| `CorrectionVm` and `ConfigurationEditor` | Retain repair drafts, command generations, reports and one worker route | Extend the shared transaction for form edits, Cancel and field results. |
| `SettingsVm` and Settings composites | Own group state, typed actions and form geometry | Project the shared draft, field errors and saved/running facts. |
| `CorrectionEvent::Saved` | Refreshes Settings, then queues changed capability checks | Distinguish ordinary Settings save intent from the accepted repair/check workflow. |
| Workspace writers in `app.rs` and `app/resize.rs` | Persist navigation, pane width and list mode independently | Coordinate writes with the editor's retained file revision. Navigation must not cause a conflict. |

The current Library form shows the music directory as read-only. Converter setup
uses the shared editor. Do not restore the former editable music-folder or FLAC
text controls from task 001's historical runbook.

## Files Likely To Change

Use the existing owners listed above. Production edits should remain in those
owners and their direct callers. Extend `tests/architecture_tests.rs` and local
unit tests. Add `docs/runbooks/settings-editor-check.md` for the operator procedure.
Extend the existing startup fixture only for this packet's isolated cases and checks.

Update this packet, the phase plan, review checklist, delivery index, source map,
`docs/README.md` and `AGENTS.md` with actual implementation evidence.
Add this packet to `docs/pending-human-checks.md` when its checks become runnable.

## Do Not Touch

- New TOML keys, configuration migrations, preset documents or workspace key consolidation
- Live Metadata or Audio groups, mode selectors, preset controls or new resource identities
- Database schema/content, music relocation or creation of a replacement library
- Playback drivers, cue/audition separation, external service control or other repositories
- Accepted Diagnostics page composition, type curves or unrelated visual polish
- Status or evidence of another packet's human gate

Never run the app as an agent. Finish this packet before starting another phase.

## Constraints And Implementation Steps

1. **Share the transaction.** Extend the existing correction owner for the retained source, saved baseline, draft and validation.
   A baseline contains the values from the last load or successful save.
   A generation identifies the draft or session that admitted an operation.
   Reject an operation's result if its generation no longer matches.

   Settings owns field presentation and group selection.
   GPUI input entities only display and edit that shared state.
   Do not maintain independently writable form and repair drafts.
   A group change, section change or repair-editor Close retains all pending edits.
   Close remains distinct from Cancel.

2. **Keep the current field scope.** General edits `ui_scale` and `theme_profile`.
   Library edits `musicindex_endpoint`. The existing converter editor owns
   `flac_path`. Core paths remain on the managed repair route.
   Settings Save covers the four existing optional fields across groups.

   If other repair edits are pending, require their existing review/save route.
   Do not silently discard or save those edits through the ordinary form action.

3. **Define Save, Cancel and defaults.** Load a source revision before enabling edits.
   Save validates again and preserves the exact original before publication.
   Failed validation, backup, publication or revision checks retain the draft.
   Successful Save records its receipt and advances the saved baseline.

   Cancel discards the shared unsaved configuration draft and restores its baseline.
   Explain that scope beside Cancel when repair edits are present.
   Cancel reverses appearance previews.
   Cancel does not reload an external edit or undo a completed save.
   Explicit Reload adopts the current file after the operator chooses to discard the draft.

   Use Defaults changes the four optional fields in the draft only.
   Show that its scope includes the converter override.
   Retain core paths and unrelated values.
   Save is still required after defaults.

4. **Preserve scoped repair rules.** Missing, unreadable or malformed documents
   cannot become a default form that overwrites the original.
   Invalid core configuration requires the existing recovery route.
   Invalid optional values pause unrelated saves and automatic persistence.

   An explicit correction of an owned invalid field may use ADR 0066's focused correction rule.
   Preserve invalid siblings. Report their remaining errors.

   Do not serialize displayed fallback values for unedited fields.
   Return typed field errors from the validation owner.
   Display them beside their fields.
   Keep file, backup and conflict failures in the shared report with a useful form notice.

   Use asd-ste100 for repair text.
   Use each action's recorded time in its report.
   Never assign a new time during rendering.

5. **Coordinate workspace persistence.** While the editor retains an editable
   source revision, defer automatic workspace writes to that configuration.
   Retain the latest navigation, pane width and list mode in memory.
   Do not hold `ConfigWriteLease` while waiting for user input.

   After Save or Cancel ends the transaction, check the expected file revision before admitting deferred writes.
   Require the ordinary saveability check too.
   A conflict leaves deferred writes paused until explicit Reload resolves it.
   Never merge an external edit automatically.
   Never replace the retained source while the draft contains unsaved edits.

   Complete admitted preference writes before loading the source for the next edit transaction.
   Navigation must remain usable while writes are deferred.
   Preserve the existing preference keys.
   This coordinates existing writers. It does not consolidate the workspace format.

   On orderly quit, discard unsaved configuration edits as the current editor does.
   Flush deferred workspace preferences only after the same revision and saveability checks.
   Do not lose workspace preferences merely because the operator opened Settings without saving.

6. **Separate saved and running facts.** Appearance may preview and apply locally.
   An ordinary Settings Save does not call `check_saved_capabilities` or install a new endpoint.
   It does not test a converter or retry an original operation.
   Report the saved endpoint separately from the endpoint used by the current session.

   Provide the existing explicit MusicIndex check through typed action state.
   That check validates local configuration. It does not establish endpoint reachability.

   Reuse its capability generation and stale-result checks before installing the saved choice.
   A failed check retains the previous resource and the new saved choice.
   Converter setup retains its explicit Test converters and retry workflow.
   Do not invent a running converter instance where no such owner exists.

   Preserve the accepted repair save/check behavior through explicit command intent.
   Neither save route starts services, attaches events or starts playback.

7. **Protect asynchronous edits.** Use the independent maintenance worker for
   file operations. Keep command admission authoritative below the renderer.
   Prevent overlapping saves. Reject completions from older draft or session generations.

   A capability result must not replace newer form edits or appearance previews.
   A save result must not claim that a resource accepted the saved values.
   Keep the current group, focus, scroll and draft mounted after results arrive.

8. **Remove the displaced route.** Remove `TopApp::save_settings`' direct writer
   and runtime mutation sequence when the shared route reaches all callers.
   Remove obsolete form fields and ordinary-save helpers if no live caller remains.
   Retain workspace writers and their ADR 0066 protections.

   Update existing guards to follow the new owners without dropping their requirements.
   Add the situational ADR 0069 guard below. Replace implementation recipe prose
   with actual owners and proof after implementation.

## Acceptance Criteria

The names below identify required proof, not tests that already exist.

| ID | Mechanical proof owner | Required assertion |
|---|---|---|
| M1 | Shared draft and Settings VM tests | Group/section changes retain edits. Form and repair views share values. Close retains edits. Cancel restores the baseline and appearance. Defaults edits only the declared four fields without writing. |
| M2 | Correction command and persistence tests | Form Save uses the correction writer. Exact original backup, private permissions, atomic publication and identity checks survive. Invalid input, failed backup and external replacement retain the draft and original. |
| M3 | Config/command tests | Missing or malformed files never become defaults. Invalid core fields reject saving. Focused correction preserves unknown values and invalid siblings. Unrelated saves remain paused. |
| M4 | Settings/maintenance dispatch tests | Ordinary Save performs no capability check, endpoint installation, converter execution, service operation, playback or original-operation retry. Repair save keeps its accepted scoped check intent. |
| M5 | Capability and shared editor tests | Explicit endpoint check uses the saved choice. Failed checks preserve the previous resource. Old check/save results cannot overwrite newer draft, resource or session generations. |
| M6 | Workspace persistence coordination tests | Navigation/resize while editing causes no false revision conflict. Deferred preferences persist after checked transaction completion or orderly quit. Quit does not save the draft. External changes remain conflicts, including edits made during deferred-write admission. |
| M7 | Settings VM tests | Actions carry availability and accessibility labels. Field errors and draft/saved/running facts are typed. Converter state claims only existing observations. Reports retain actual result times. |
| M8 | Situational guard `adr_0069_shared_guarded_settings_editor` | Settings and recovery reach one transaction/persistence owner. Screens contain no persistence or validation policy. Existing maintenance worker, shared controls and named tokens remain in use. |
| M9 | Fixture tests | Disposable setup, expected changes, preservation inspection, conflict injection and cleanup reject the wrong directory and unintended mutations. |

V1–V4 below require the operator. Mechanical checks cannot prove visual acceptance.
Preservation inspection and cleanup are additional acceptance requirements.

## Test Commands

Run focused owner tests during implementation. Then run the required checks:

```bash
cargo fmt -- --check
cargo check --locked --offline --quiet
cargo test --locked --offline --quiet
cargo clippy --locked --offline --quiet -- -D warnings
python3 -B -m unittest discover -s docs/runbooks -p 'test_startup_recovery_fixture.py'
cargo build --locked --offline --quiet --bin v4vmm
git diff --check
```

Record exact test names and counts for M1–M9. An empty filtered selection is not proof.
Check relative documentation links. Do not perform unrelated lint cleanup.

## Rollback And Escalation

Revert the implementation as one change if necessary. Preserve configuration
backups and operator fixtures until their inspections pass. No format rollback is needed.

Stop if implementation requires new persisted fields, a second transaction engine,
external edit merging or playback changes.
Report the specific conflict.
Amend the governing ADR before changing those contracts. Do not infer a release
of ADR 0066's configuration-format gate from this packet's readiness.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:

- This packet and every file in Files To Inspect.
- ADR 0069, ADR 0066 and the Settings phase plan.

Goal:

- Deliver shared guarded editing for the existing Settings fields.

Constraints:

- Follow Constraints And Implementation Steps.
- Reuse the correction transaction, worker and capability commands.
- Keep all persistence formats unchanged.
- Keep ordinary Save distinct from explicit resource checks.

Do not touch:

- Any item in Do Not Touch.
- The status of another packet's acceptance gate.

Acceptance criteria:

- Prove M1–M9 mechanically.
- Prepare V1–V4 with preservation inspection and cleanup.
- Keep the visual gate open until the operator supplies evidence.

Test commands:

- Run every command in Test Commands.
- Build the normal desktop binary after the tests.

At the end, report:

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

End the implementation report with Operator visual check and runnable commands.
Record actual evidence in this packet and its review checklist.

## Operator Visual Check

These requirements describe the future implementation. Do not walk them against
the current binary. The implementation must publish `docs/runbooks/settings-editor-check.md`
with exact assertions before opening this packet's human gate.

Prerequisites: Linux desktop, Python 3.11+, the normal debug binary and a new
disposable startup fixture. Use Null playback and local service/Index stubs.
No audio hardware, real publisher or public endpoint is required.

1. Close the normal app.
   Build the normal binary in a desktop terminal.
   Prepare a new fixture with the following commands:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --locked --offline --quiet --bin v4vmm
   settings_editor_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
   python3 docs/runbooks/startup-recovery-fixture.py verify "$settings_editor_fixture"
   python3 docs/runbooks/startup-recovery-fixture.py run "$settings_editor_fixture"
   ```

2. **V1 — draft, defaults and Cancel.** Enter Settings with Ctrl+Comma.
   Edit the endpoint and appearance. Visit both groups, Music, Show and Configuration repair.
   Check retained values, focus and each group's scroll position.

   Close/Reopen must retain edits. Cancel must restore the baseline and appearance.

   Defaults must remain unsaved until Save. Include the converter override in the review.
   Unexpected writes, hidden reset scope or lost text fail this check.

3. **V2 — save and explicit check.** Save from each editable group using local stub endpoints.
   Check the complete changed-field result and original backup.
   Confirm that saving alone sends no Index request or resource operation.
   Invoke the explicit endpoint check. Check success, failure and retained running values.

   Use invalid or unreadable saved configuration for a failed local check.
   An unreachable host alone must not be reported as a failed local configuration check.
   The runbook must inspect stub requests and service commands for these claims.

4. **V3 — invalid input and conflicts.** Test invalid endpoint input, unavailable
   backup storage and an external file edit after draft loading.
   Check the field message, retained draft, copied report and unchanged external file.
   Navigate to another section during an ordinary draft.
   Resize the pane. Those actions must not cause a conflict.

   Repeat through the shared repair route with an invalid optional sibling.
   The runbook must name exact fixture commands and expected saved keys for each case.

5. **V4 — presentation and shared repair.** Check the new controls and messages at normal/narrow widths and short height.
   Repeat in Light/Dark at M and XL.
   Check Tab, Shift+Tab, Enter, Space and the accepted Escape behavior.
   Text, errors and actions must remain readable and reachable without overlap.
   Check mounted updates, report copy and the existing Configuration repair page.

6. Quit the fixture app. Run the packet's preservation assertions before restoration.
   Inspect exact backups, unedited settings, library records, music, credentials and temporary files.
   Keep failed fixtures. The ordinary `inspect` command alone does not validate intentional Settings saves.

   After the packet's assertions pass, restore the normal fixture baseline.
   Inspect the restored fixture:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py mode "$settings_editor_fixture" normal
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$settings_editor_fixture"
   ```

7. After all required inspections pass, remove only this fixture:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$settings_editor_fixture"
   ```

   Record V1–V4 separately, preservation results and the exact cleanup output.
   No check in another packet closes by implication.
