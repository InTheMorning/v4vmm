# ADR 0066 Task 008: Converter Verification And Setup

Status: Complete - 2026-09-17; mechanical checks Green.
Operator V1–V3, Settings/core-recovery presentation and preservation in both
fixture cases are accepted. Normal-mode restoration and fixture cleanup are
confirmed. This packet has no open acceptance checks.
Task 007 is complete. Task 009 has not started.

## Goal

Let the operator configure and freshly test FLAC/ffmpeg availability without restarting the app, while preserving the actual conversion fallback policy.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 007](adr-0066-task-007-optional-tool-correction-and-retry.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/audio_format.rs — flac_cli_available, ffmpeg_cli_available, transcode_wav_to_flac
- src/track_compare.rs — download_track WAV branch and format_warning
- src/config.rs — flac_path
- src/application/commands/maintenance.rs; src/view_models/startup.rs
- src/ui/composites/maintenance_forms.rs; src/app.rs — existing FLAC setting
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/audio_format.rs
- src/application/commands/maintenance.rs; src/view_models/startup.rs
- src/ui/composites/maintenance_forms.rs; src/app.rs — converter tool composition only
- src/config.rs — focused flac_path correction integration only
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- Staging/materialization and download outcome changes (009)
- New converter config keys, package-manager automation, broad download-format policy changes
- Automatic substitution for an invalid explicitly configured executable
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

The implemented procedure and coding prompt are retired in favor of these live
owners. The situational guard `adr_0066_converter_checks_are_refreshable` in
[architecture_tests.rs](../../tests/architecture_tests.rs) protects the ADR 0066
probe/correction boundary. ADR decisions and invariants remain binding.

| Criterion | Owner and evidence |
|---|---|
| C1: fresh PATH and changed configured paths | `ConverterObservation::refresh` and `ConverterProbe` in [audio_format/probe.rs](../../src/audio_format/probe.rs); `adr_0066_converter_path_checks_refresh_in_one_process` uses real executable files and a per-command PATH, without mutating the test process environment. |
| C2: bounded distinct failures and actual fallback | `bounded_probe`/`observe_child`; `adr_0066_converter_failures_are_distinct_and_output_is_not_retained`, `adr_0066_converter_timeout_reaps_child`, and `adr_0066_converter_preserves_flac_first_and_existing_ffmpeg_fallback`. Real subprocesses cover FLAC first, rejected input, missing explicit path, ffmpeg fallback, permission denial, exit 7, timeout/reaping, and capped output. |
| C3: inert edit, fresh Test, guarded Save | `CorrectionOperation::TestConverter`, `CorrectionSource::converter_path`, and `CorrectionVm`; `adr_0066_converter_edit_test_and_guarded_save_are_separate` and `adr_0066_converter_rejects_stale_tests_and_conflicting_saves` verify invocation counts, no edit/Test writes, exact-byte backup, retained invalid siblings, fresh Test after Save, revision conflict and stale-result rejection. |
| C3: safe recorded report and truthful fallback | [startup/converter.rs](../../src/view_models/startup/converter.rs), `adr_0066_converter_report_preserves_recorded_time_and_fallback_facts`. Backend observations supply actual UTC instants; the VM owns wording. No arbitrary process output is retained in reports or Debug. |
| C4: execution ownership | `adr_0066_converter_checks_are_refreshable` excludes permanent PATH caches and UI-side execution, and guards independent-worker dispatch, bounded child handling and the configured executable. Existing shared correction/presentation guards remain applicable. |

`SettingsAction::OpenConverter` uses task 007's existing correction route.
`configuration_correction` owns the shared input, wrapped guidance and typed
Test/Save geometry in Settings and core recovery. It uses the existing spacing,
typography, button and log-frame tokens. The existing Library converter input
now opens this guarded editor; ordinary Settings persistence retains its existing
saved-value integration. No config key was added.

The bounded helper lives under the existing audio-format backend. It uses a
nonblocking shared output channel with a 16 KiB cap and a five-second timeout
for each executable, terminating/reaping failed probes. Child output is omitted
because a selected executable can print credentials; typed outcomes provide the
safe diagnostic. Probe observations are fresh for every request.

`transcode_observed` retains FLAC-first and ffmpeg-fallback encoding. The narrow
`ensure_taggable_local_path` correction removes silent PATH FLAC substitution
for a failed explicit setting; its existing FLAC prerequisite for reused local
WAV files remains. Download/staging/materialization and retained-track retry
are unchanged and remain task 009's scope. There is no new Retry control.

### Fixture And Review Evidence

[startup-recovery-fixture.py](../runbooks/startup-recovery-fixture.py) adds
`converter-setup` and `converter-recovery`, a stub-only child PATH, live
`converter-tools` modes, version invocation records, and `converter-inspect`.
[Fixture tests](../runbooks/test_startup_recovery_fixture.py) cover search-path
isolation, working/missing/fallback/nonzero/permission/timeout modes, unchanged
configuration during mode changes, and rejection of unrelated edits, un-restored
paths, non-version invocations and non-private backups.

Green: `cargo fmt -- --check`, `cargo check --quiet`, 1,423 unit tests,
254 architecture guards, `cargo clippy --quiet -- -D warnings`, and
`cargo build --quiet --bin v4vmm`. Ten existing documentation examples remain
ignored. Python fixture tests: 18 Green. Backend fixture setup, verification,
all seven tool modes, both cases, preservation inspection, normal-mode/PATH
restoration and cleanup: Green. The normal desktop binary was rebuilt after tests.

The sandbox full-suite attempt failed socket-dependent tests and stalled in an
existing mpv IPC test; it was interrupted and unit tests passed with local socket
access. A converter timeout test initially observed transient
`IoFailure(ExecutableFileBusy)`; its unchanged rerun and full unit suite passed.
The first architecture run found the superseded inline FLAC width assertion;
`settings_form_inputs_fill_scaled_frame_width` now retains the endpoint rule and
checks converter width/scaling at the active shared editor. All 254 guards and
the doc-test pass then completed. No application behavior was changed for these
execution-environment failures.

The agent did not launch the app during implementation. Backend smoke evidence
is in `/tmp/v4vmm-0066-008-fixture-smoke.log`; its temporary fixture was removed.
Operator acceptance uses a separate fixture, recorded below.

## Test Commands

Run the focused owner tests while editing, then the repository gate once:

```bash
cargo fmt -- --check
cargo check --quiet
cargo test --quiet
cargo clippy --quiet -- -D warnings
cargo build --quiet
```

New source assertions must preserve the rules of any guard they replace and name
ADR 0066 as their situational owner. Do not widen the lint gate or add a second
integration-test file.

## Operator Visual Check

Accepted - 2026-09-17. The [task 008 procedure](../runbooks/startup-recovery-check.md#task-008-converter-verification-and-setup)
remains a regression check.

- V1: missing converters become available in the same app, recorded results and
  copied reports update, and installation guidance stays explicit.
- V2: invalid configured FLAC and working PATH ffmpeg are both reported;
  edited paths are inert, Test is fresh, and Save preserves the original without
  converting/downloading. Restore the unset FLAC path afterward.
- V3: timeout, exit, permission and output-limit reports remain distinct while
  Settings responds; narrow controls/report remain reachable. The same checks
  work in core recovery with invalid music configuration.
- Preservation and cleanup: use `converter-inspect` after quitting each case;
  only after it passes restore normal mode and remove the fixture. Launcher PATH
  is child-local, so the operator's shell PATH and real config never change.

### Final Operator Acceptance And Cleanup — 2026-09-17

The operator supplied normal-mode restoration and cleanup output naming
`/tmp/v4vmm-startup-2qpie3af` as removed, then reported "all passed" for the
three remaining confirmations: fixture removal, normal/narrow recovery
presentation and the final Settings Test showing exit 0 for both converters
after the bounded failures. The fixture's final Settings invocations were
recorded at 13:28:37 UTC; the operator confirmation establishes their displayed
success results. No separate cleanup timestamp was supplied.

Together with the reports and both preservation inspections below, this accepts
V1–V3, Settings/core-recovery presentation, preservation and cleanup. The task
008 gate is closed. Task 004 and inherited checks retain their separate gates;
Task 009 has not started and requires a fresh session.

The dated observations below record progress during the walkthrough. This final
acceptance closes every temporary pending state recorded in those observations.

### Operator Evidence — 2026-09-17: V1 Missing PATH Converters

Fixture: `/tmp/v4vmm-startup-2qpie3af`. The supplied repair report records
configuration loading at 10:49:11 UTC and converter checks at 10:49:27 UTC
and 10:55:46 UTC on 2026-09-17. Both checks identify `flac` and `ffmpeg` as
missing PATH executables. Each report preserves the distinction between a
retained WAV with a conversion warning and other usable audio formats, and
states that the version test did not convert, download, save or install.
The separate invalid MusicIndex setting remains visible as expected.

This establishes V1's missing-tool report and supplied copy content. The
live-refresh follow-up is recorded below.

### Operator Evidence — 2026-09-17: V1 Fresh PATH Success

After the instruction to enable working fixture tools without restarting the
app, the operator supplied the same retained repair report with new results at
10:57:12 UTC. Both `flac` and `ffmpeg` succeeded on PATH with exit 0. The report
retains the earlier missing results and identifies FLAC as the first conversion
attempt and ffmpeg as the fallback if FLAC rejects the input. It continues to
state that the version check did not convert, download, save or install.

The missing-to-working refresh result and supplied copy content pass this part
of V1. The remaining V1 confirmations were received in the follow-up below.

### Operator Evidence — 2026-09-17: V1 Accepted And V2 Edit-Only Pass

The operator reported "pass" for the requested before/after configuration and
probe-log hash comparison after entering
`/tmp/v4vmm-startup-2qpie3af/missing-flac`, without Test or Save, and for the
requested Settings responsiveness and visible installation-guidance confirmations.
The edit-only preservation check passes. Together with the supplied missing and
working PATH reports, this accepts V1.

The same fixture remains open with ffmpeg in working fallback mode and the
missing explicit FLAC path in the draft. V2's Test, guarded Save and changed-path
checks remain open, followed by V3, remaining presentation, preservation and
cleanup. This pass does not accept those later operations.

### Operator Evidence — 2026-09-17: V2 Explicit Missing Path And Fallback

The supplied repair report records both version checks at 11:02:45 UTC. FLAC
failed at `/tmp/v4vmm-startup-2qpie3af/missing-flac`, identified as the configured
path. PATH ffmpeg succeeded with exit 0. The explanation correctly allows the
existing WAV download conversion fallback and states that PATH FLAC does not
replace the explicit executable. Earlier observations remain in the supplied
report. This accepts V2's explicit missing-path/fallback Test.

Guarded Save, its backup and no-probe checks, changed-path verification and
restoration remain open. The fixture app is being kept open for these steps;
V3, remaining presentation, preservation and cleanup are also open.

### Operator Evidence — 2026-09-17: V2 Guarded Save And Original Backup

The supplied repair report records Save at 11:05:06 UTC. It names the original
backup `/tmp/v4vmm-startup-2qpie3af/config/v4vmm/.v4vmm-config-237986-0.backup`,
states that Save did not retry an operation, and retains the unrelated invalid
MusicIndex setting and paused ordinary persistence.

The operator supplied these matching SHA-256 values:

| Evidence | SHA-256 |
|---|---|
| Configuration before Save and named original backup after Save | `c1150f0a6f24451afd7f76d943d4d749119ed69c718a72111e465ab898b341c9` |
| Converter invocation log before and after Save | `301bb60ceb71db6a735eb707980ea369b154301b1691f04405c97abe1ff41ebc` |

The original-byte backup and no-probe Save checks pass. Changed-path Test/Save,
restoration, V3, remaining presentation and final preservation/cleanup remain
open. Backup permissions and library/music preservation remain part of the
final fixture inspection; these hashes alone do not establish those facts.

### Operator Evidence — 2026-09-17: V2 Changed Configured Path Test

The supplied report records configuration reload at 11:22:12 UTC and both
version checks at 11:22:55 UTC. FLAC succeeded with exit 0 at
`/tmp/v4vmm-startup-2qpie3af/bin/flac`, identified as the configured path;
ffmpeg succeeded with exit 0 on PATH. The new result identifies the changed
executable instead of reusing the earlier missing-path failure. The report
retains prior observations and the unchanged FLAC-first/ffmpeg-fallback wording.
This accepts the changed-path Test.

Saving/reloading this working path, preserving the preceding saved configuration,
and restoring the unset FLAC path remain open. V3, remaining presentation and
final preservation/cleanup also remain open.

### Operator Evidence — 2026-09-17: V2 Working Path Save And Reload

The supplied report records Save at 11:24:50 UTC, naming
`/tmp/v4vmm-startup-2qpie3af/config/v4vmm/.v4vmm-config-237986-2.backup`,
and configuration reload at 11:25:06 UTC. Save again reports no operation retry,
and the unrelated MusicIndex issue and paused ordinary persistence remain.

The operator also reported "pass" for the requested match between this new
backup and the configuration hash taken before Save, retention of the earlier
backup, and display of `/tmp/v4vmm-startup-2qpie3af/bin/flac` after Reload.
This accepts the changed-path Save/reload and preceding-revision preservation.
Restoring the unset FLAC path remains the final V2 step. V3, remaining
presentation and final preservation/cleanup remain open.

### Operator Evidence — 2026-09-17: V2 Unset Path Restored And Accepted

The operator reported "pass" after clearing the FLAC field, saving, reloading
and selecting Converter setup again. The requested confirmation was a blank
field and the description `unset; use flac on PATH`, with the unrelated
MusicIndex setting unchanged. This accepts unset-path restoration and closes
V2's operator checks. V1 remains accepted.

The same fixture app stays open for V3's bounded failure and responsiveness
checks. V3, remaining Settings/core-recovery presentation, final preservation
and fixture cleanup remain open. Task 009 has not started.

### Operator Evidence — 2026-09-17: V3 Timeout Report

The supplied repair report also records the accepted unset-path restoration
Save at 11:28:08 UTC, with backup `.v4vmm-config-237986-4.backup` in the same
fixture configuration directory, and reload at 11:28:12 UTC.

The timeout test reports PATH FLAC exceeding five seconds at 11:31:15 UTC and
PATH ffmpeg exceeding five seconds at 11:31:20 UTC. Both results name process
termination and reaping, retain the scoped WAV warning, and state that no
conversion, download, configuration save or installation occurred. The supplied
copy contains these results and the preceding observations. Timeout reporting
passes; actual child cleanup remains part of the final fixture inspection.

Timeout interaction confirmation followed with the exit-7 check below. Actual
child cleanup remains part of the final fixture inspection.

### Operator Evidence — 2026-09-17: V3 Exit-7 Report And Interaction Confirmation

The supplied repair report records PATH FLAC and ffmpeg both failing their
version checks with exit 7 at 13:16:21 UTC. It retains the scoped WAV warning and
states that no conversion, download, configuration save or installation
occurred. The supplied copy retains the earlier observations and contains no
fixture process output.

The operator confirmed both requested checks passed: Settings navigation and
resizing responded during the timeout test, and completion left the editor
closed until the operator reopened it; the fixture sentinel was absent from
the app report, copied report and terminal running the app. Timeout interactions,
exit-7 reporting and output privacy pass.

Permission/output-limit cases, remaining Settings/core-recovery presentation,
working-mode recovery and preservation/cleanup remain open. Task 009 has not
started.

### Operator Evidence — 2026-09-17: V3 Permission-Denied Report

The supplied repair report records permission to execute denied for both PATH
FLAC and ffmpeg at 13:22:50 UTC. These results are distinct from the earlier
missing-executable, timeout and exit-7 results. The report retains the scoped
WAV warning and version-only explanation. Permission-denied reporting passes.
The output-limit case, remaining Settings/core-recovery presentation,
working-mode recovery and preservation/cleanup remain open.

### Operator Evidence — 2026-09-17: V3 Output Limit And Settings Presentation

The operator reported pass for the output-limit test and the normal/narrow
Settings window checks. Both converter checks reported the 16 KiB output limit
and process cleanup and finished without hanging. Report text remained readable,
actions did not overlap, and Test/Save controls remained reachable. These checks
pass; no separate observation timestamp was supplied.

Working-mode recovery after the failures, core-recovery access/presentation,
preservation inspection in both cases and fixture cleanup remain open. The
current fixture remains in use; Task 009 has not started.

### Operator Evidence — 2026-09-17: Converter-Setup Preservation

The operator supplied `converter-inspect` output for
`/tmp/v4vmm-startup-2qpie3af` after closing the app. All eight converter checks
passed: original bytes preserved, unedited values preserved, FLAC path restored
to unset, version-only invocations, recorded children reaped, owner-only
backups, no candidate files and an intact original case copy.

Shared inspection also passed: configuration and music preserved, migration
versions 1–11 intact, one playlist with three tracks and three playlist entries,
unchanged `a.wav`/`b.wav`/`c.wav` bindings, and no residual music or database
probes. `config_bytes_unchanged: false` is consistent with the accepted guarded
Saves; original-byte preservation and unchanged unedited values were checked
separately. Converter-setup preservation is accepted.

The fixture was in working mode, with final version invocations recorded at
13:28:37 UTC. Their exit results are not recorded in this fixture output;
operator confirmation of the final success report remains pending.
Core-recovery access/presentation, core-recovery preservation and fixture
cleanup also remain open. Task 009 has not started.

### Operator Evidence — 2026-09-17: Core-Recovery Access And Missing Converters

The supplied recovery report records configuration loading at 13:33:09 UTC,
with the deliberate empty `music_dir` and invalid `musicindex_endpoint` issues.
At 13:33:43 UTC, Test converters reports both PATH FLAC and ffmpeg not found.
It retains the scoped WAV warning and states that no conversion, download,
configuration save or installation occurred. Core-recovery converter access
and missing-converter reporting pass.

Fresh working results in the same recovery window, normal/narrow recovery
presentation, core-recovery preservation and fixture cleanup remain open.
Confirmation of the final working results from converter-setup also remains
pending. Task 009 has not started.

### Operator Evidence — 2026-09-17: Fresh Working Results In Core Recovery

The supplied recovery report retains the missing-converter observations and
records both PATH FLAC and ffmpeg version checks succeeding with exit 0 at
13:35:03 UTC. It names FLAC as the first conversion attempt and ffmpeg as the
existing fallback, with the version-only explanation. Fresh working results
in the same recovery window pass.

Normal/narrow recovery presentation, core-recovery preservation and fixture
cleanup remain open. Confirmation of the final working results from
converter-setup also remains pending. Task 009 has not started.

### Operator Evidence — 2026-09-17: Core-Recovery Preservation

The operator supplied closed-app `converter-inspect` output for the
`converter-recovery` case in `/tmp/v4vmm-startup-2qpie3af`. All eight converter
checks passed, including version-only invocations, reaped children, unchanged
unedited values and restored unset FLAC path. Shared inspection confirms
unchanged configuration bytes, preserved music, migration versions 1–11,
one playlist with three tracks and three playlist entries, unchanged bindings,
and no residual music or database probes. Core-recovery preservation passes;
preservation is now accepted in both fixture cases.

The final converter-setup success report and normal/narrow recovery presentation
still need operator confirmation. Fixture cleanup remains open. Task 009 has
not started.

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

Fresh probing would change conversion policy, a probe cannot be bounded/reaped, or a proposed field requires a new TOML key. Keep verification separate from unapproved policy changes.
Routine placement inside the named owner is authorized. If a boundary needs a
new architectural decision, name the conflict and proposed bounded correction
before widening this packet.
