# ADR 0066 Task 008: Converter Verification And Setup

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

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

## Implementation Steps

1. Replace permanent negative PATH observations in flac_cli_available and ffmpeg_cli_available with a refreshable observation owner. Explicit Test/Retry always runs a fresh probe in the same process. Return typed results naming executable, source (configured path/PATH), recorded time, exit outcome and safe diagnostic.

2. Keep transcode_wav_to_flac's actual fallback policy: FLAC first where available and ffmpeg where already supported. Distinguish an invalid explicit FLAC setting from an absent optional converter, and describe any permitted fallback honestly. Do not claim that every WAV operation failed just because FLAC is absent.

3. Add a Converter tool to the shared Settings/recovery maintenance UI. Show the configured path and observed FLAC/ffmpeg results. Offer Choose executable/path input, Test, guarded Save, and explicit installation guidance. Guidance does not run a package manager and should direct the operator to their distribution's package tools without guessing a distribution command.

4. Run version probes through a bounded backend process helper on the maintenance worker. Set a named five-second timeout and output cap; reap timed-out children. Surface permission, missing executable, nonzero exit and timeout distinctly. No blocking Command::status in a renderer.

5. Use task 007's correction intent. This packet verifies/configures the tool; task 009 owns resuming track conversion. Do not add a Retry control that cannot yet execute its represented operation.

6. Extend the fixture with controlled FLAC/ffmpeg executables and modes for missing, working, nonzero exit and timeout. Restore PATH and configured path after the operator check.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | audio_format.rs process tests | A missing PATH executable made available later is detected by an explicit fresh check in the same process. A configured path change does not reuse the old result. |
| C2 | process/fallback tests | Working fallback remains available; explicit bad-path, permission, nonzero exit and timeout have accurate reports. Children are reaped and output is bounded. |
| C3 | VM/command tests | Choose/edit has no write/probe side effect; Test is fresh, Save uses guarded correction, and neither installs software nor starts a download. |
| C4 | new situational guard adr_0066_converter_checks_are_refreshable | Repair/retry cannot depend on immutable OnceLock PATH observations or run probes in UI code. |

Documentation proof: remove this packet's duplicate mechanism prose as its guards
land; record actual symbols and fixture/runbook anchors. Keep its Status,
the plan, ADR partial line, delivery row and pending-human-check index truthful.
An unwalked visual check cannot pass through a green mechanical suite.

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

The implementation must extend `docs/runbooks/startup-recovery-fixture.py` and
`docs/runbooks/startup-recovery-check.md` with a section for this packet.
Those files are implementation deliverables, not commands available at packet
authoring time. Follow the phase plan's fixture contract. Supply unindented
copyable commands, named fixture state, purpose, expected result and cleanup.
Never ask an operator to repeat an already accepted check without a changed
owner or an unresolved failure.

1. Open Converter setup with both fixture tools missing. Make the FLAC stub available and press Test. It must update without closing the app.

2. Use a bad configured FLAC path and a working ffmpeg fallback. Confirm the report identifies both facts rather than claiming a universal download failure.

3. Test the fixture's timeout and failed-exit modes. Settings must remain responsive and name what failed. Restore fixture mode/PATH and clean up.

Do not run the app as an agent. When the binary/fixture are ready, open this
packet's gate in its Status, the delivery row and pending-human-check index.
Record the operator's actual result before closing it.

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

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`
- `docs/adr/0066-configuration-and-startup-failure-recovery.md`
- `docs/plans/adr-0066-startup-recovery-phase-plan.md`
- This packet in full, including Files To Inspect, implementation steps and criteria.

Goal:
- Let the operator configure and freshly test FLAC/ffmpeg availability without restarting the app, while preserving the actual conversion fallback policy.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Staging/materialization and download outcome changes (009)
- New converter config keys, package-manager automation, broad download-format policy changes
- Automatic substitution for an invalid explicitly configured executable

Acceptance criteria:
- Prove every mechanical row in this packet at its named owner.
- Record actual guard references and keep trackers consistent.
- Supply the specified fixture/runbook check; keep its human gate open until walked.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo build --quiet`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

