# ADR 0066 Task 009: Conversion Retry And Retained Input

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Return from converter setup to the same track, reuse valid downloaded input where possible, and avoid duplicate library materialization.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 008](adr-0066-task-008-converter-verification-and-setup.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/track_compare.rs — DownloadedTrack::finalize, discard, Drop and download_track
- src/subscribe_service.rs — PreparedTrack, subscribe_track_with_config, SearchTrackSubscription and finalization
- src/application/ports/download_manager.rs — DownloadError and ServiceDownloadManager
- src/application/commands/download.rs; src/application/events/download.rs
- src/library/app_impl.rs; src/app/search_dispatch.rs; src/view_models/library.rs; src/view_models/search_results/index_detail.rs
- src/remote_media.rs; src/library_path.rs — existing artifact/path validation
- Task 007 RecoveryIntent and task 008 converter observations
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/track_compare.rs; src/subscribe_service.rs
- src/application/ports/download_manager.rs; src/application/commands/download.rs; src/application/events/download.rs
- src/application/conversion_recovery.rs (new); src/application/mod.rs
- src/view_models/library.rs; src/view_models/search_results/index_detail.rs; src/view_models/startup.rs
- src/library/app_impl.rs; src/app/search_dispatch.rs — outcome/repair/retry adapters only
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- Persistent download queue/schema, feed-wide transaction redesign, or format policy changes
- Source provenance collapse or weakening downloaded-artifact validation
- Automatic retry on saving Settings
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

1. Return structured conversion results instead of requiring the UI to parse format_warning strings. Distinguish conversion succeeded, fallback succeeded, usable WAV retained with a conversion warning, and failed materialization. Keep existing accepted WAV/warning behavior; do not turn every conversion warning into a failed subscription.

2. Add a session-owned recovery entry keyed by original operation/track identity. It owns either retained DownloadedTrack staging input or a validated existing library binding, plus the original request, source enclosure, pending edits and destination context. Move staging ownership out of the automatic Drop cleanup only when a recovery entry has accepted ownership. No clone may independently delete the same directory.

3. On setup/retry, freshly verify converters through 008 and validate retained input using the existing artifact rules, source identity and path containment. Remove failed partial output before another attempt. If input is gone/invalid, offer explicit redownload of that same subject and explain why before dispatch; do not silently reuse untrusted or moved input.

4. Retry the existing subscription/materialization path with retained input as an explicit parameter, rather than creating a second pipeline. For a warning that already produced a usable library entry, revalidate that binding and preserve the existing file until replacement conversion/tagging succeeds; update that entry rather than insert another. Preserve source facts, pending metadata edits and playlist-append semantics.

5. Revalidate original IDs, destination/config generation and current library membership at execution. A changed selection never changes the retry subject. Duplicate clicks have one in-flight operation. Saving converter settings is inert; Retry is explicit.

6. Release owned staging on success, explicit discard and orderly session teardown. Report cleanup failure with its path. Session-only retention does not promise survival across a crash/relaunch; original library files are never staging cleanup targets.

7. Expose the retained operation and truthful outcome through existing download/result surfaces and task 007's remediation route. Extend fixture with a deterministic WAV input, failed conversion, working fallback, and same-process corrected converter.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | track_compare/subscribe_service tests | Actual conversion/fallback/warning outcomes remain distinct. Failed output is removed; valid retained input survives setup and is validated before reuse. |
| C2 | application retry tests | Repair resumes the original track with its edits/destination; stale selection/context and missing/invalid input cannot silently change the subject. Redownload requires the explicit documented path. |
| C3 | materialization tests | Success, duplicate Retry and retry of an already-materialized WAV warning produce one library entry and no repeated playlist append. Original usable file survives failed replacement. |
| C4 | ownership/cleanup tests | Staging has one owner; success/discard/session teardown clean only owned artifacts and report cleanup failures. Existing music never becomes a cleanup target. |
| C5 | new situational guard adr_0066_conversion_retry_uses_existing_materialization | Retry shares the normal validated pipeline and explicit subject contract; Save cannot replay it. |

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

1. Download the fixture WAV with failed converters. Confirm the message says whether the download failed or a usable WAV was kept, and names the track.

2. Open converter setup from that result, correct/test the converter, then Retry. Confirm the same track completes without finding it again, relaunching, or creating a duplicate.

3. Remove the fixture's retained input before another retry. The app must explain the required redownload. Exercise working ffmpeg fallback and compare its successful report. Inspect counts and clean up.

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

Retaining input requires changing the accepted format/provenance policy, a successful warning cannot be retried without risking the original file, or cleanup ownership remains ambiguous. Do not add a parallel materialization path.
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
- Return from converter setup to the same track, reuse valid downloaded input where possible, and avoid duplicate library materialization.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Persistent download queue/schema, feed-wide transaction redesign, or format policy changes
- Source provenance collapse or weakening downloaded-artifact validation
- Automatic retry on saving Settings

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

