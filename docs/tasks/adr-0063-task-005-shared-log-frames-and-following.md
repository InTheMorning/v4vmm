# ADR 0063 Task 005: Shared Log Frames And Following

Status: Implemented - 2026-09-13. Mechanical checks Green. Operator V1–V3,
preservation inspection and fixture cleanup remain open.
This packet precedes ADR 0066 task 007 at the operator's explicit
request. Task 006 of ADR 0066 remains complete; task 007 has not started.

## Goal And Owners

Give every app log the same framed, compact monospace viewport and independent
following behavior, including startup and recovery. The
[ADR 0063 amendment](../adr/0063-show-dashboard-layout.md#shared-log-frames-and-following)
owns presentation; ADR 0066 keeps report, repair and session semantics.

Inspect and extend the shared selectable-text and Show log composites, startup
report and maintenance composites, renderer-free log reading state, Show source
identity, and the existing service observation/command route. Named tokens own
font, height, spacing and frame treatment. The app roots retain renderer handles.

## Scope

- Frame Show service/Event logs, Diagnostics/background reports, startup details,
  configuration repair history, session drain and previous-session reports.
- Follow the bottom initially and on new entries. Pause on scrolling up;
  resume at the bottom or with keyboard-accessible Go to latest outside the text.
- Restore each source independently after switching or hiding. Retain logical
  reading anchors through updates; explain when replacement removes an anchor.
- Keep long lines reachable using visible horizontal scrolling and exact
  selection/copy. Refresh visible service snapshots through existing owners.

Do not change configuration format, report retention/redaction/timestamps,
service lifecycle or playback. Narrow Show card geometry and cross-repository
UTC corrections remain separate. No agent runs the GUI.

## Acceptance

Mechanical: model tests cover initial following, manual pause, paused updates,
return to latest, independent source keys, trimmed/replaced text and Unicode.
Service tests cover single-flight refresh and stale/closed results. A situational
ADR 0063 architecture guard enforces the shared viewport route, typed actions,
named tokens and renderer-free reading model. Existing copy/selection guards stay.

Run cargo check, build, fmt check, strict Clippy, relevant tests and architecture
guards. Record actual results here. The operator must inspect all log families,
normal/narrow widths, Light/Dark, supported scaling, long-line copy, follow and
pause/resume, source switching, disclosure and session transitions. Keep this
gate in pending human checks and the delivery order until accepted.

## Operator Visual Check

Follow the [operator procedure](../runbooks/log-frame-check.md) in a desktop
session. V1 covers recovery, correction and Diagnostics; V2 covers Show source
switching, long-line copy, live snapshots and reading anchors; V3 covers held
session draining and retained reports. It includes exact setup, observation,
preservation and cleanup commands. No agent ran the GUI.

## Implementation And Verification

| Responsibility | Owner and proof |
|---|---|
| Follow/pause/latest and logical-line anchors | [LogReadingVm](../../src/view_models/log_view.rs); `adr_0063_follow_pause_append_and_return_are_explicit`, `adr_0063_trim_preserves_surviving_unicode_anchor_and_explains_loss`, `adr_0063_invalid_geometry_does_not_change_following` |
| Per-source retained view and viewport | [LogFrame and LogFrames](../../src/ui/composites/log_frame.rs); operator V1–V3 and `adr_0063_logs_share_frame_following_and_renderer_free_state` |
| Service and event identity; single-flight fresh reads | [Show model](../../src/view_models/show.rs) and existing [Show adapter](../../src/app/show.rs); `adr_0063_journal_identity_uses_transport_instance_and_unit`, `adr_0063_visible_journal_refresh_is_single_flight_and_does_not_reopen`, extended Event identity test |
| Exact selection during appends | [TextSelection](../../src/view_models/text_selection.rs), existing selectable-text composite; `adr_0063_log_append_keeps_selection_and_replacement_clears_it` and existing Copy guards |
| Frame/typography/gutter tokens | `LOG_FRAME_HEIGHT`, `LOG_FRAME_BORDER`, `LOG_SCROLLBAR_GUTTER` in layouts; `LOG_TEXT_SIZE`, `LOG_LINE_HEIGHT`, `log_font_family` in tokens |
| All mounting paths | Show log pane/shell, startup report, maintenance forms, ConfigurationEditor and app roots share the viewport; Settings/recovery retain the same collection |

Green: cargo check, build, fmt check, `cargo clippy -- -D warnings`, and the full
suite of 1,367 unit tests and 240 architecture tests. Ten existing documentation
examples remain ignored. The shared-log architecture guard is situational,
owned by ADR 0063; existing exact-copy, renderer-boundary and session guards stay.

The new [fixture helper](../runbooks/log-frame-fixture.py) reuses the prepared
startup library and its verified isolation/cleanup boundary. Backend checks
passed setup, service observation, Unicode/long journal output, append, trim,
replacement, preservation and cleanup. Agent fixture
`/tmp/v4vmm-startup-1heid9jr` was removed. This is not visual proof.

New documentation is this packet and the operator runbook, with a fixture helper
beside the existing fixtures. No folders or Markdown files were moved; canonical
root instructions stay in place. Source-map and current status indexes are updated.
All 316 local file links across the twelve changed/new Markdown files resolve;
formatting and `git diff --check` are Green.

## Scope Limits And Rollback

Log-source selection and report retention are unchanged. Read-only service
snapshots refresh at the existing observation cadence; no playback, service
restart or optional-tool reinitialization was added. External UTC emitters and
the existing narrow Show card/log allocation remain separate scheduled work.
Reading state lasts for the window and is not persisted in configuration.

Revert this packet's presentation and reading-state changes together if its
operator gate fails. Preserve fixture evidence, configuration, backups and music.
ADR 0066 task 007 waits for this packet's acceptance and a fresh session.
