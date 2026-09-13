# ADR 0066 Task 006: Configuration Repair And Resumption

Status: Complete - 2026-09-13. Mechanical checks Green. Operator V1–V6,
preservation inspections and fixture cleanup are accepted.
Temporary viewport measurements are removed.
Task 007 has not started; it follows in a fresh session.

The operator's later Close/Reopen editor request is a bounded presentation
follow-up under [ADR 0063 task 005](adr-0063-task-005-shared-log-frames-and-following.md#configuration-editor-close-and-reopen--adr-0066).
That packet's Close/Reopen and Escape follow-ups, visual checks, preservation
and cleanup are complete on 2026-09-13; this packet's accepted repair and
preservation checks remain closed.

## Goal

Repair configuration inside recovery or Settings, preserve the original file, and return to a freshly verified app session.

## Operator Notes — 2026-09-10

During task 002 acceptance, the operator confirmed that the recovery screen
must eventually let them edit broken paths directly. Steps 4–5 below own this
already accepted behavior; task 002 supplies checks and reports only.

The operator also requested an option to create a new setup. Define that
workflow separately before implementing it: selecting existing locations and
creating a new empty library have different preservation and resumption
requirements. This note does not authorize bypassing core checks, overwriting
the broken configuration, or replacing the existing database. The current
packet's path-correction scope remains unchanged.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 005](adr-0066-task-005-session-drain-and-resumption.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.

## Files To Inspect

- src/config.rs — snapshot, guarded writers and field validation
- src/startup.rs; src/view_models/startup.rs; src/app/startup.rs
- src/presentation/maintenance_executor.rs; src/presentation/startup_presenter.rs
- src/ui/composites/startup_report.rs; src/app.rs — render_settings/save_settings
- Session drain and resumption packet — managed transition out of normal work
- docs/troubleshooting/column-text-truncation.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Changed

- New: `src/config/correction.rs`, `src/application/commands/maintenance.rs`,
  `src/view_models/startup/correction.rs`, `src/presentation/configuration_editor.rs`.
- Extended: `src/config.rs`, `src/startup.rs`, `src/startup/storage.rs`,
  `src/application/commands/mod.rs`, `src/presentation/mod.rs`,
  `src/view_models/startup.rs`, `src/view_models/settings.rs`, `src/app.rs`,
  `src/app/startup.rs`, `src/app/settings.rs`, and the lifecycle test in
  `src/app/bootstrap.rs`.
- Shared UI: `src/ui/composites/maintenance_forms.rs`,
  `src/ui/composites/startup_report.rs`, `src/ui/layouts.rs`.
- Guards: `tests/architecture_tests.rs`,
  `docs/runbooks/test_startup_recovery_fixture.py`.
- Documentation: this packet, the existing fixture/runbook, phase plan, review
  checklist, task 001 handoff, ADR 0066 and ADR index, delivery/deferred indexes,
  docs index, pending-human index, AGENTS.md and the source map.

The fixture regression test was added beside its runbook and helper. No Markdown
file or documentation folder was created or moved; existing canonical root
instructions remain in place. Documentation link checks are recorded in the review.

## Do Not Touch

- Database content replacement or music-file moves
- Automatic repair/default reset, configuration key renaming, comment-preserving merge
- New editors independently duplicated in Settings and recovery
- Other repositories or unrelated open human checks.
- Unrelated Clippy debt; `--all-targets` is not this repository's lint gate.

## Constraints

- Preserve ADR 0066's minimum: valid core configuration, usable music storage
  and working SQLite. Optional services/tools never become core requirements.
- Keep typed facts/availability/intent in backend/application/view-model owners.
  Shared composites own geometry and named tokens. Screens only wire them.
- Every new module is called by the packet's live workflow. No parked scaffolding,
  fake healthy values or screen-only execution checks.
- Preserve original data and safe diagnostics; no tokens or credential-bearing
  excerpts in reports, clipboard or Debug output.
- Delete duplicated mechanism prose when its guard lands. Replace it with the
  actual guard symbol and verification artifact in this packet; keep the ADR's
  binding decision/invariants. Task 001 owns the series handoff review.

## Implementation And Proof

The implementation recipe and coding prompt are retired for actual owners and
proof below. ADR 0066 retains the binding decisions and invariants.

| Responsibility | Live owner |
|---|---|
| Original bytes, source/link/target revision, focused TOML edits, backup and replacement | `CorrectionSource`, `CorrectionDraft`, `CorrectionField` in [config/correction.rs](../../src/config/correction.rs) |
| Draft validation, existing-location tests and explicit save | `CorrectionCommand`, `CorrectionAccess`, `CorrectionOperation` in [commands/maintenance.rs](../../src/application/commands/maintenance.rs) |
| Draft retention, typed single-flight actions, generations and UTC receipts | `CorrectionVm` in [startup/correction.rs](../../src/view_models/startup/correction.rs) |
| One input entity, independent worker and completion delivery | [configuration_editor.rs](../../src/presentation/configuration_editor.rs); existing `MaintenanceClient` and `present_startup` |
| Shared geometry, scale and accessible actions | `configuration_correction` in [maintenance_forms.rs](../../src/ui/composites/maintenance_forms.rs); `CONFIGURATION_EDITOR_HEIGHT`, existing spacing, typography, size and control tokens |
| Settings/recovery composition and managed access | [app/startup.rs](../../src/app/startup.rs), [app/settings.rs](../../src/app/settings.rs); task 005's `SessionTransition` and `MaintenanceSession` |
| Ordinary saves and core-path protection | `ConfigWriteLease`, `read_config_for_save`, `save_app_settings` in [config.rs](../../src/config.rs) |

### Mechanical Acceptance

All proof names below are implemented tests. The situational guard is
`adr_0066_shared_guarded_config_repair` in
[architecture_tests.rs](../../tests/architecture_tests.rs), owning ADR 0066
invariants 3–6 and 9. Existing persistence, runtime, session and Settings guards
remain in force. The Settings width/default assertions now follow the core-path
maintenance route under ADR 0066; unrelated field and group rules remain.

| ID | Actual proof |
|---|---|
| C1 | `adr_0066_correction_preserves_exact_original_unknown_values_and_invalid_siblings`; `adr_0066_rejected_and_unreadable_corrections_do_not_write_or_disclose_values`; `adr_0066_failed_backup_and_concurrent_changes_prevent_replacement`; `adr_0066_symlink_correction_preserves_link_and_rejects_link_or_target_changes`; `adr_0066_draft_copy_redacts_nested_credentials_and_malformed_documents` |
| C2 | `adr_0066_ordinary_and_correction_writers_share_nonblocking_admission`; `adr_0066_editing_is_inert_and_save_is_single_flight_without_retry`; `adr_0066_conflict_preserves_draft_and_reload_never_invents_unreadable_contents`; the C1 preservation test also proves clean fresh reads re-enable ordinary saves while invalid siblings keep them paused |
| C3 | `adr_0066_path_correction_tests_existing_locations_and_never_creates_a_database`; `adr_0066_ordinary_settings_preserve_core_paths_and_unedited_values` |
| C4 | Extended `adr_0066_drained_core_reopens_fresh_only_after_successful_checks` now saves both core paths after actual resource release, checks backup bytes, rejects old preparation consent, and verifies the new session's selected resources and larger generation; `adr_0066_validation_generations_reject_old_drafts_and_core_edits_need_drain`; existing task 005 handle-release and single-mount proofs remain |
| C5 | `adr_0066_shared_guarded_config_repair`, paired with the behavioral tests above |

### Mechanical Evidence — 2026-09-11

Green: `cargo fmt -- --check`, `cargo check --quiet`, `cargo test --quiet`,
`cargo clippy --quiet -- -D warnings`, and `cargo build --quiet`.
The full suite passed 1,361 unit tests and 239 architecture tests; ten existing
documentation examples remain ignored. `git diff --check` is Green.

The extended fixture's setup, six repair modes, permission restoration,
concurrent edit, preservation inspection and cleanup commands are Green.
Agent fixtures `/tmp/v4vmm-startup-ju004din` and
`/tmp/v4vmm-startup-_fdn62is` were removed after those backend-only checks. This is not operator acceptance or evidence that the app's UI was run.
The fixture and runbook add no new GUI failure hook or persisted config key.

### Scope And Limits

- Settings keeps General/Library/Diagnostics. Core paths now use the shared
  editor and managed maintenance; the ordinary music-folder control shows the
  current location, and Use Defaults retains core paths. Optional form values
  refresh after a correction so a later ordinary save does not restore an old
  fallback value. Appearance values use the existing live preview setters.
  `ConfigWriteLease` prevents an ordinary writer from overlapping a correction;
  competing Settings Save controls receive typed busy state.
- Raw editing handles malformed TOML; non-UTF-8 or unreadable bytes receive a
  report and reload action. No source is replaced by defaults. Focused fields
  preserve unedited TOML values, including invalid independent optional values.
  Complex fields use a focused TOML `value` assignment. Comments are not merged.
- Copy draft includes the proposed document with credentials redacted. Malformed
  syntax cannot be safely redacted, so copying it returns an explanation and
  leaves the full draft in the editor. Reports and Debug never contain source
  excerpts or rejected credential values.
- Optional adapters retain their current session state until task 007 supplies
  reinitialization and explicit original-operation retry. A Save is not a network
  observation or a retry. No task 007 work or configuration-format change was
  started.
- The backend child module and shared presentation entity are routine placement
  within the packet's assigned owners. No architectural deviation was needed.

## Operator Visual Check

The operator's 2026-09-11 screenshots show recovery naming the malformed file
and its TOML location, with Open app unavailable. After Edit configuration,
the loaded-file report names the resolved destination, but the input collapses
to a tiny empty box. V1 initially stopped at this defect. The shared composite gives the input
an explicit scale-aware viewport and prevents vertical shrinking; the unused
plain-multiline row setting is removed. V1 and V3 explicitly check this defect
class in recovery and Settings before any correction is saved. Operator
inspection below establishes visibility; mechanical checks alone do not.
The next screenshot showed the intended viewport height, but its width was
still approximately the border and padding alone. The operator returned the
app's observation recorded at 2026-09-11 20:16:31.109072072 UTC: the input and
draft both contain 240 matching bytes, input editing is enabled, and the input
measures 18 × 160 pixels while the form's other children are 1,404 pixels wide.
This rules out missing input text and a narrow form; it does not explain why
the widget's percentage width collapsed. A standalone Taffy calculation did
not reproduce the failure.

The shared composite now owns a full-width frame with the named, scaled height
and pins the input to its edges with automatic width. The operator's next
screenshot still shows the narrow, empty input: this frame attempt failed too.
An isolated check using the same compiled GPUI libraries confirms that style
merging preserves absolute positioning, both edge offsets and automatic width;
it does not exercise the running input widget.

The operator returned the deeper observations from 2026-09-11
20:30:10.936905981 UTC and 20:30:10.937181068 UTC: input text still matches,
the widget requests absolute positioning with zero left/right offsets and
automatic width, and it measures 18 × 160 pixels inside a 0 × 160 pixel frame.
The frame's percentage width collapsed. The frame now uses automatic width and
explicit column stretching, matching the neighboring rows' allocation.

On 2026-09-11 the operator reported "pass" after reopening the same fixture and
choosing Edit configuration to inspect the width and loaded document. This
accepts the initial recovery viewport correction. Both temporary frame/widget
measurements are removed. V3's normal/narrow Settings viewport check also passed,
as recorded below.
The V1/V3 manual checks retain the situational ADR 0066 regression guard.

The operator supplied the malformed-draft copy explanation: the app declined
to copy the document because its syntax was invalid and retained the draft.
The next screenshot shows the invalid trailing lines removed, the complete
remaining draft visible, and validation recorded at 2026-09-11 20:50:20 UTC.
The result states that the app validated the draft and tested any changed core
paths, and that no configuration was saved. The startup report retains its
2026-09-11 20:39:50 UTC parse failure and Open app remains unavailable. These
pass the malformed-copy and visible edit/validation checks.

The operator subsequently reported "pass" for the unchanged-file inspection
before Save and the save sequence: a timestamped success report named the
backup, repeated Save became unavailable, and the app remained in recovery.
Backup bytes and permissions are confirmed by the final fixture inspection
recorded below.

The operator then reported "pass" for the separate Check again and Open app
sequence, the same window opening Music with one fixture playlist and three
tracks, and Settings → Diagnostics retaining the correction result and backup
path. The accompanying screenshot confirms the resumed Settings surface and
session 1; its background-runtime report records 2026-09-11 20:56:58 UTC.

The operator supplied the copied repair report and complete corrected TOML.
The copy retains the recorded load and validation times and names the save at
2026-09-11 20:53:57 UTC, with original bytes backed up to
`/tmp/v4vmm-startup-pkqo2pix/config/v4vmm/.v4vmm-config-2475664-0.backup`.
The report states that a fresh read found no configuration issues and ordinary
persistence is permitted again. The copied draft retains both fixture paths,
the loopback MusicIndex endpoint and Null playback, without the invalid trailing
lines. Both copy checks pass.

The operator supplied both final `repair-inspect` reports for `repair-toml`.
Preservation is Green: original bytes are backed up with owner-only permissions,
unedited configuration values are preserved with no changed fields, and music,
bindings, one playlist, three tracks and migration records 1–11 are preserved.
No candidate, music-probe or database-probe files remain. The backup path matches
the copied repair report and its SHA-256 is
`0436a1194d8fe8c40bcfec5572cb8047746ee4c5b6c3cb235568d9b982ea1454`.
The changed configuration bytes are expected after the explicit correction.
The operator then confirmed "width/theme pass; cleanup done." V1 is accepted
on 2026-09-11, including normal/narrow widths in Light/Dark, preservation and
cleanup of `/tmp/v4vmm-startup-pkqo2pix`.

V2 uses `/tmp/v4vmm-startup-6c9aoict`. The operator supplied its startup report
recorded at 2026-09-11 21:09:09 UTC, naming the empty `music_dir` and `db_path`
settings and keeping normal operations closed. The operator then reported
"pass" after entering the fixture's `music` directory and nonexistent
`data/missing.sqlite` database path: switching fields retained the unsaved music
value, and Test and Save rejected nonexistent proposed database paths. The operator then
reported "all pass" for the two absence checks, confirming that neither the
missing database nor a correction backup was created. Replacing `db_path` with
the fixture's existing `data/library.sqlite` allowed Save, Check again and Open
app; Music opened with one fixture playlist and three tracks. The copied report
below records intervening failed validation and save attempts before that
successful correction.

The operator then reported "pass" for the running-session transition: after
reloading the configuration in Settings and proposing the fixture's
`music-choice` directory, Test and Save remained unavailable. End session to
edit core paths reached recovery, completed its check, and retained the proposed
folder in the editor. The operator subsequently reported "pass" for Test and
Save naming a backup while Music stayed closed, separate Check again and Open
app actions, Library displaying the full `music-choice` path, Diagnostics
retaining both session and repair reports with an increased session number,
and the fixture playlist containing three tracks once each. V2's operator
interface checks pass.

The copied V2 repair report records the rejected `missing.sqlite` validation
at 2026-09-11 21:12:36 UTC. The attempts at 21:21:08, 21:21:18 and 21:21:28 UTC
show a line break after `library.sqlite` inside the proposed path; validation
and Save rejected that path too. The operator procedure now specifies entering
each path as one line. The first successful save was recorded at
2026-09-11 21:22:52 UTC. After reload at 21:26:15 UTC, the report records successful
validation at 21:27:44 UTC and the music-folder save at 21:27:53 UTC.

Both final `repair-inspect` reports are Green. Only `music_dir` changed; original
and selected music, unedited configuration values, bindings, one playlist,
three tracks and memberships, and migrations 1–11 are preserved. No candidate,
music-probe or database-probe files remain. The two owner-only backups match the
save reports in `/tmp/v4vmm-startup-6c9aoict/config/v4vmm/`:

- `.v4vmm-config-2483896-0.backup`, SHA-256
  `14d5e039cc06bc6062325ec09110af837fc5f6800ae4046af6b85d2de19ebdd1`.
- `.v4vmm-config-2483896-2.backup`, SHA-256
  `3cad47d10d18c862cca48353b21e1ad37c4e1da473f6ab7fd760399577945937`.

The operator confirmed "cleanup done" for `/tmp/v4vmm-startup-6c9aoict`.
V2 is accepted on 2026-09-11, including preservation inspection and fixture
cleanup.

V3 uses `/tmp/v4vmm-startup-m7fsha8y`. The operator reported "pass" for normal
startup, the Settings editor at normal and narrow widths, and correcting only
`musicindex_endpoint` to the fixture's loopback URL. Its copied report records
the two initial optional issues at 2026-09-11 21:38:35 UTC and a successful save
at 21:39:25 UTC. The original is backed up at
`/tmp/v4vmm-startup-m7fsha8y/config/v4vmm/.v4vmm-config-2491162-0.backup`.
The saved result still names `flac_path` and keeps ordinary persistence paused.
The operator then reported "pass" for the direct check that the file still held
`flac_path = false`, followed by reloading, clearing only that field and saving.
The second save named another backup and permitted ordinary persistence after
a fresh read; the earlier report and backup path remained available. The
operator subsequently reported "pass" after selecting Medium and Dark and
using the ordinary Settings Save control. Returning to Library retained the
corrected loopback endpoint and empty converter input. V3's operator interface
checks pass.

The final copied V3 report records reload at 2026-09-11 21:42:16 UTC with only
`flac_path` still invalid, followed by the second save at 21:42:39 UTC and a clean
fresh read permitting ordinary persistence. Both `repair-inspect` reports are
Green. Only `theme_profile` and `ui_scale` differ from the clean fixture baseline;
the original invalid optional values are backed up, all other configuration
values are preserved, and music, one playlist, three tracks and memberships,
bindings and migrations 1–11 remain intact. No candidate, music-probe or database
probe files remain. The two owner-only backups match the report in
`/tmp/v4vmm-startup-m7fsha8y/config/v4vmm/`:

- `.v4vmm-config-2491162-0.backup`, SHA-256
  `03979f4d5b9700b3590ddfc98e20952547fc39f3600fa9ed79946c8148952fd9`.
- `.v4vmm-config-2491162-2.backup`, SHA-256
  `5c1d259b86233c28bcc337f0b1c02d9b2b7a305fc3f2e1bb7ca38885e4b7e2c2`.

The operator confirmed "cleanup done" for `/tmp/v4vmm-startup-m7fsha8y`.
V3 is accepted on 2026-09-11, including the normal/narrow Settings editor,
independent corrections, ordinary persistence, preservation and fixture cleanup.

During this check the operator noted that Diagnostics logs are drawn directly
on the surrounding chrome in large text. The requested consistent log frame,
matching text size, possible monospace type and automatic following across the
app are recorded under [A11/A12 shared log work](../plans/hig-product-polish-backlog.md#a11---long-log-lines-are-hard-to-inspect).
This is a separate scheduled presentation follow-up, not a failed V3 correction
check or an added task 006 acceptance requirement.

On 2026-09-13 the operator reported "pass" for V4's unreadable-file check:
Edit configuration reported the named read failure with a retry action, without
inventing editable configuration text or offering Save correction. The operator
then reported "pass" after matching the verified fixture path to the recovery
report, restoring only file access, loading the original values, and choosing
Check again then Open app without editing or saving. Music opened with one
fixture playlist and three tracks. V4's operator interface checks pass.

The first V4 inspection reported `original_preserved: false`, no backups and
no changed non-workspace fields. The operator's diff for
`/tmp/v4vmm-startup-fohkgcsc` shows only formatting and the normal
`workspace_layout` save: focused frame 2 with one content-list frame. The
normal shutdown path persists this layout under ADR 0046. The V4 instructions
and repair inspector had incorrectly required exact bytes after normal exit,
omitting the existing startup procedure's workspace-only allowance.

The inspector now shares that existing comparison with ordinary inspection.
Only V4 can use it instead of a saved original, and only with a matching clean
exit, a case copy matching the recorded hash and unchanged non-workspace values.
The report keeps `original_preserved: false` when bytes changed without an app
backup; `normal_workspace_preferences_only` and `config_preserved` state the
separate allowance. Recovery before normal exit still requires unchanged bytes.
The [situational ADR 0066 fixture tests](../runbooks/test_startup_recovery_fixture.py)
reproduced the false failure; all six tests are now Green, including rejection
of changes before normal exit, non-workspace edits and missing preservation in
other correction cases. No app code changed for this checker correction.
The operator supplied both corrected inspection reports. Each reports
`normal_workspace_preferences_only: true` and `config_preserved: true`.
Non-workspace values are unchanged, no backups or candidates exist, and music,
bindings, one playlist, three tracks and memberships, and migrations 1–11 are
preserved without leftover probes. V4 preservation is Green. The operator
confirmed "cleanup done" for `/tmp/v4vmm-startup-fohkgcsc`. V4 is accepted on
2026-09-13, including preservation inspection and fixture cleanup.

For V5, the operator supplied the repair report for
`/tmp/v4vmm-startup-v8d0epb0`. The app loaded the malformed configuration at
2026-09-13 14:04:06 UTC and validated the corrected draft at 14:04:16 UTC without
saving. At 14:04:22 UTC, Save reported permission denied while creating
`.v4vmm-config-2813375-0.backup` and stated that the configuration was not
replaced. The reported failure matches the fixture's denied directory writes.
The operator verified the same fixture and inspected it before restoring access.
Inspection is Green: the original configuration bytes are unchanged, no backups
or candidates exist, and music, bindings, one playlist, three tracks and
memberships, and migrations 1–11 are preserved without leftover probes.
The operator then reported "pass" after restoring directory access, saving the
retained draft without reloading, and choosing Check again then Open app.
This accepts the successful save with a named backup and clean fresh read,
retention of the earlier failure report, and Music opening with one playlist
and three tracks. The operator's final inspection is Green: the original bytes
are preserved in the owner-only `.v4vmm-config-2813375-1.backup`, with SHA-256
`568af65abadc371041b6420a5d4e33bea485fe8851ab3049baeb022c201c8db4`.
Unedited configuration values, music, bindings, one playlist, three tracks and
memberships, and migrations 1–11 are preserved. No candidates or leftover probes
remain. The operator confirmed "cleanup done" for
`/tmp/v4vmm-startup-v8d0epb0`. V5 is accepted on 2026-09-13, including final
preservation inspection and fixture cleanup.

For V6, the operator supplied the repair report for
`/tmp/v4vmm-startup-kwrwaxe9`. The app loaded the configuration at
2026-09-13 14:14:38 UTC and reported invalid endpoint and converter values.
After the fixture's external edit, Save at 14:16:13 UTC reported a changed
configuration, retained the draft, and stated that the file was not replaced
and no automatic merge was available. The operator confirmed the proposed
endpoint remained in the editor and in the copied draft, with `flac_path = false`
retained. The operator's final inspection is Green:
`external_revision_preserved: true` confirms the external file remains intact.
No backups or candidates exist. The reported endpoint and converter differences
are the fixture's intentional values relative to the clean baseline; unedited
configuration values are preserved. Music, bindings, one playlist, three tracks
and memberships, and migrations 1–11 are preserved without leftover probes.
The operator confirmed "cleanup done" for `/tmp/v4vmm-startup-kwrwaxe9` and
the scratch draft/report buffer. V6 is accepted on 2026-09-13, including conflict
handling, copied-draft and final preservation checks, and fixture cleanup.
All six cases are accepted; task 006's acceptance gate is closed.

After the input frame correction: formatting, `cargo check --quiet`, all 239
architecture tests, `cargo clippy --quiet -- -D warnings`, `cargo build --quiet`
and `git diff --check` are Green. All 82 local file links in the four updated
operator/status documents resolve. No documentation files or folders were
created or moved; canonical root documents remain in place.
The standalone layout probe's temporary source and executable were removed;
the operator confirmed cleanup of all six fixtures.

Procedure: [Task 006 checks](../runbooks/startup-recovery-check.md#task-006-configuration-repair-and-resumption).
It retains accepted V1–V6 as regression checks and provides commands, fixture
state, expected results and cleanup for each case:

1. Accepted V1: malformed TOML corrected in recovery, backup, explicit checks/resumption,
   retained reports, narrow/normal widths, Light/Dark and report/draft copy.
2. Accepted V2: two invalid core paths, rejection of a nonexistent database, then an
   existing music-folder change from Settings through session drain and resumption.
3. Accepted V3: two independent optional corrections, retained sibling value, refreshed
   form values and resumed ordinary persistence after a clean fresh read.
4. Accepted V4: unreadable source without an invented editor; permission repair and reload.
5. Accepted V5: backup creation failure preserving the original and draft; explicit retry.
6. Accepted V6: concurrent edit preserving the external revision and proposed draft.

Every case requires its preservation inspection and confirmed fixture cleanup.
A Linux desktop, Python 3.11+, debug binary and unprivileged account are required;
no audio hardware or reachable external service is needed. No agent ran the app.
No task 006 operator check remains open. Task 004 and inherited checks retain
their separate gates; task 007 has not started.

Closure verification - 2026-09-13: all 239 architecture tests, six fixture
regression tests, changed-document file links and `git diff --check` are Green.
The full Rust suite and required compiler, formatting, lint and build checks
remain Green as recorded above; closure changed documentation only.

## Rollback

Revert this packet's code as a coherent change if its gate fails; preserve all
operator configuration, database backups and music. Do not reverse migrations
or delete recovery artifacts as a code rollback. Leave dependent packets pending
and document any observable intermediate limitation.

## Expected Final Report Format

1. Files changed
2. Tests run (Green, or the exact failure)
3. Behavior changed
4. Deviations from task
5. Unresolved concerns
6. Operator visual check, or the explicit reason no new visual check applies

## Escalation Triggers

A focused save cannot preserve unedited values, a path edit would require silently migrating files, or a core correction would reopen beneath existing handles. Keep the draft and report the named conflict.
Routine placement inside the named owner is authorized. If a boundary needs a
new architectural decision, name the conflict and proposed bounded correction
before widening this packet.
