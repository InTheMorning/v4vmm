# ADR 0066 Task 009: Conversion Retry And Retained Input

Status: Complete - 2026-09-17. Mechanical checks Green; operator V1–V3,
normal/narrow presentation, configuration restoration and preservation accepted.
Fixture cleanup confirmed.

## Goal And Scope

Return from converter setup to the original track, reuse validated downloaded
input, and replace its existing library binding without repeating playlist append.
Tasks 008 and 009 are complete. This session covered only task 009; task 010 has
not started and requires a fresh session.

[ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md), the
[phase plan](../plans/adr-0066-startup-recovery-phase-plan.md) and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md)
remain binding. No schema, configuration format, persistent download queue,
source-provenance policy or feed-wide transaction redesign is introduced.

## Implementation And Proof

The implemented mechanism steps and coding prompt are retired in favor of the
live owners and situational guard
`adr_0066_conversion_retry_uses_existing_materialization` in
[architecture_tests.rs](../../tests/architecture_tests.rs).

| Contract | Live owner and verification |
|---|---|
| C1: typed FLAC, fallback, usable-WAV and failed-materialization results; failed output removed | `audio_format::ConversionOutcome`, `encode_observed`, `track_compare::DownloadedTrack`; `adr_0066_conversion_actual_fallback_cleans_partial_output_and_keeps_input`, the task 008 converter process tests, and `adr_0066_invalid_converter_preserves_downloads_and_retains_wav` |
| C2: immutable original request, source, edits and destination; revalidated context; explicit redownload | `application::conversion_recovery::ConversionRecovery`, `subscribe_service::materialization::Materialization`; `adr_0066_conversion_missing_input_requires_explicit_redownload_of_original_enclosure`, `adr_0066_conversion_retry_rejects_changed_binding_context_and_input`, `adr_0066_conversion_controls_preserve_subject_and_explain_explicit_redownload` |
| C3: one binding, one in-flight attempt, no repeated playlist append; original preserved on failure | Shared `Materialization::run`, transactional binding replacement and publication rollback; `adr_0066_conversion_retry_keeps_edits_one_binding_and_original_wav`, `adr_0066_conversion_single_flight_save_is_inert_and_playlist_append_is_not_repeated`, `adr_0066_failed_replacement_keeps_file_binding_and_retained_staging`, `adr_0066_database_rejection_restores_staging_and_original_binding` |
| C4: one staging owner; validation and cleanup never target existing music | `track_compare::retained::{RetainedArtifact, OwnedStaging}`, `SessionTransition::close`; `adr_0066_retained_artifact_checks_size_bytes_identity_and_containment`, `adr_0066_staging_cleanup_refuses_replaced_directory_and_reports_its_path`, `adr_0066_conversion_discard_and_session_close_keep_existing_music` |
| C5: normal and retry share validation/materialization; Save cannot replay | `subscribe_track_retaining`, `subscribe_feed_retaining`, `RetryConversion` through task 007's `RetryCommand`; the named architecture guard and inert-Save/VM tests above |

The session owns retained requests and staging through the existing shared
application services. Track, feed and playlist downloads use that owner.
Screens adapt watch/command completions; `CapabilityReportVm` supplies the
original subject, explanation, typed actions and accessibility labels. Existing
startup-report composites, log frames, controls and named tokens supply geometry.
Cleanup failure after a committed conversion stays a successful conversion
with a path-bearing warning and an owned cleanup entry; it cannot enable
another conversion. The original usable WAV remains on disk after successful replacement; its old
binding is removed transactionally, leaving one active FLAC binding. Retention
is session-only and makes no crash/relaunch persistence promise.

The normal download-result contract also carries the typed conversion outcome.
Actual conversion reports appear through the existing retained-action notice and
Background tools in Settings. A tag-comparison read failure after successful
materialization is reported separately instead of claiming the saved file failed.

## Mechanical Evidence — 2026-09-17

Green: `cargo check --quiet`, `cargo fmt -- --check`, full `cargo test --quiet`
(1,434 unit tests, 255 architecture guards; ten existing documentation examples
ignored), `cargo clippy --quiet -- -D warnings`, and 20 fixture tests.
Green: normal `cargo build --quiet --bin v4vmm` after the test run.
Final focused verification is Green: 136 ADR 0066 unit tests and 18 guards,
including the final retained-filename and cleanup-report corrections.

The sandboxed full-suite attempt hit local HTTP/Unix socket restrictions and
was stopped. The unrestricted run exposed the direct-download staging-root
prerequisite; its existing creation behavior was restored. The new source guard
was corrected to tolerate formatted line breaks. A minimal invalid FLAC test
sample was replaced with a real encoded silent sample before the Green run.
No app was launched and these checks establish no visual acceptance.

Fixture/runbook owners are
[startup-recovery-fixture.py](../runbooks/startup-recovery-fixture.py), its unit
tests, the small shared [FLAC sample](../runbooks/fixtures/conversion.flac), and
[task 009's operator procedure](../runbooks/startup-recovery-check.md#task-009-conversion-retry-and-retained-input).
The fixture's CLI seed extends the existing debug-only `startup::fixture` owner.

Backend fixture smoke: Green — fresh seed, loopback WAV response, isolated
failed/working/fallback encoding, unchanged configuration, original audio,
original bindings and migration records. The smoke fixture
`/tmp/v4vmm-startup-rvo3fb5a` and its owned server were removed successfully.
This was backend verification only and accepts none of V1–V3.


## Review And Limits

No deviation from the packet's functional scope. The existing owners gained two
backend child modules to keep artifact ownership and shared materialization out
of screen adapters. Source identities and pending edits remain separate from
presentation text. The phase plan, delivery order and pending-human index close
this packet's operator gate. Task 004's separate gate and inherited checks stay
unchanged.

## Operator Visual Check

Follow [V1–V3, preservation and cleanup](../runbooks/startup-recovery-check.md#task-009-conversion-retry-and-retained-input)
in a Linux desktop terminal. The procedure supplies copyable setup/run commands,
failed and working converters, exact original subjects and enclosure request
counts, expected results, failure indicators and cleanup. It needs no audio
hardware or installed conversion tool.

1. Confirm **Conversion retry** reports a usable WAV and exposes converter setup.
2. Test/save the corrected converter without automatic replay, then explicitly
   retry that original track; confirm one updated binding and no second fetch.
3. Move **Conversion redownload** input with the fixture helper; confirm Retry
   explains redownload before any request, then explicitly redownload with
   working ffmpeg fallback. Inspect the new controls at normal/narrow widths and
   copy the safe report. Close the app, inspect preservation and clean up.

### Operator Evidence — 2026-09-17

The operator supplied screenshots, copied reports and fixture inspection output
from `/tmp/v4vmm-startup-4psemojh` in the same desktop session:

- V1: at 15:01:17 UTC, **Conversion retry** retained a usable WAV and exposed
  **Edit converter setting**. The mounted Music view showed four library tracks.
  Inspection showed one `/conversion-4.wav` request, four bindings, five playlist
  rows and empty staging.
- V2: both fresh converter probes succeeded at 15:05:35 UTC. The next inspection
  retained the WAV binding and original counts. A separate check at 15:15:14 UTC
  enabled explicit Retry. At 16:09:26 UTC, the report named **Conversion retry**
  and FLAC success; subsequent inspection confirmed its sole binding changed to
  FLAC with no additional request or playlist row. The operator accepted normal
  and narrow widths, readable/reachable controls and Dismiss without Retry.
- V3: **Conversion redownload** retained a usable WAV at 16:46:21 UTC. After the
  fixture moved its input aside, Retry reported the missing original file at
  16:49:11 UTC and offered explicit redownload. Inspection still showed only one
  `/conversion-5.wav` request. Explicit redownload reported ffmpeg fallback
  success at 16:50:11 UTC. Inspection confirmed both FLAC bindings, request counts
  of one and two respectively, five bindings, five playlist rows and empty staging.

### Final Operator Acceptance And Cleanup — 2026-09-17

V1–V3 and normal/narrow presentation are accepted. The copied reports name the
original tracks, conversion outcomes, paths and recorded UTC times without the
converter-output secret. The repair report confirms the initial correction at
15:07:30 UTC preserved `.v4vmm-config-359868-0.backup`. Restoration to the original
unset FLAC path at 16:54:20 UTC preserved `.v4vmm-config-359868-2.backup`. Both
backups are under the fixture's `config/v4vmm` directory. Both saves report no
automatic retry, and the final request counts remain one and two.

After closing the app, the operator supplied all fifteen preservation flags as
true: one binding per track; original tracks and bindings; both converted
bindings; one reuse and one explicit redownload; released staging; preserved
moved WAV; restored configuration and original revision; private backups; no
candidate/probe files; preserved migrations/database and unrelated audio;
preserved original usable WAV; and reaped converter children.

The operator then ran the fixture cleanup and supplied both
`Removed fixture: /tmp/v4vmm-startup-4psemojh` and `Fixture removed`, confirming
owned-server/directory cleanup and the final absence check. Task 009 has no open
operator gate. Task 004 and inherited checks remain separate; task 010 has not
started. The procedure above remains a regression check.

## Rollback

Revert this packet's code coherently if its gate fails. Preserve operator
configuration, backups, original music and any reported recovery artifacts.
Do not reverse migrations or delete operator data as a code rollback. Keep
successor packets pending until the failure is resolved.
