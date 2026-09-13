# ADR 0063 Task 005: Shared Log Frames And Following

Status: Complete - 2026-09-13. Mechanical checks Green. Operator V1–V3,
ADR 0070 narrow Show layout/header/footer checks, Settings/recovery editor
Close/Reopen and Escape follow-ups, final preservation inspection and all
fixture cleanup are accepted. No acceptance gates remain for this packet.
ADRs 0063 and 0070 are Implemented. ADR 0066 task 006 remains complete;
task 007 follows in a fresh session and has not started.

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
- ADR 0070 gives the open Show log priority over card height, with scrolling
  cards and an independent sidebar. Preserve the preferred log height through
  temporary window constraints.

Do not change configuration format, report retention/redaction/timestamps,
service lifecycle or playback. Compact card density, full-width docking,
card-title readability, inactive transport and cross-repository UTC corrections
remain separate. No agent runs the GUI.

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

## Operator Resize Finding — 2026-09-13

During V1, the operator resized startup recovery in fixture
`/tmp/v4vmm-startup-dmizjdrm` (startup report recorded at 15:03:35 UTC).
The supplied wide and narrow screenshots show scrollbar thumbs outside the log
frame, overlapping the configuration-repair explanation. The remaining V1
checks are not accepted.

The shared composite placed the scrollbar directly after the viewport. The
pinned component supplies absolute positioning and full size but no edge
insets. The correction anchors an enclosing layer to all four viewport edges,
matching the component's own scrollable wrapper. The measurement canvas also
has an explicit top-left anchor. Every mounted log uses this shared owner.

The situational ADR 0063 manual regression is V1 step 1 and V2 step 4 of the
[operator procedure](../runbooks/log-frame-check.md): resize repeatedly, inspect
track/thumb containment and footer clearance, and drag both axes afterward.
The operator reported pass after relaunching the same fixture and repeating the
recovery-details wide/narrow resize and scrollbar-drag check on 2026-09-13.
That correction's recovery-details visual check is accepted. Editor/history,
following, copying, transitions and other log families remain open. Retain the
current fixture through the remaining V1 checks.

Correction checks Green: cargo check, build, fmt check, strict Clippy, six
ADR 0063 unit tests and all 240 architecture tests. The debug binary is rebuilt.
All 73 local file links in the four updated documents resolve. No documents or
folders were created or moved; canonical root documents remain in place.
These checks do not establish visual acceptance.

## Operator Wheel Finding — 2026-09-13

During the remaining V1 check, the operator reported that the mouse wheel moved
the whole page even when aimed at a log. The operator subsequently reported pass
for log-only wheel scrolling through both limits, ordinary page scrolling
outside, pause and Go to latest on 2026-09-13. Keep fixture
`/tmp/v4vmm-startup-dmizjdrm` for the retest.

The pinned GPUI viewport handler updates its scroll offset without stopping
event propagation. The shared LogFrame now consumes the wheel event after
GPUI's handler, observes the updated position for following state, and notifies
the view. Ancestor pages do not receive that event, including at a log's scroll
limits. Pointer hit testing leaves scrolling outside the viewport unchanged.

The situational ADR 0063 manual guard is V1 step 2 and V2 step 4 in the
[operator procedure](../runbooks/log-frame-check.md). It checks log-only wheel
movement at middle/top/bottom and with short text, ordinary page scrolling
outside, and follow/pause/latest behavior. Its recovery retest is accepted;
other log families and the remaining V1 checks still need operator acceptance.

Wheel correction checks Green: cargo check, build, fmt check, strict Clippy,
six ADR 0063 unit tests and all 240 architecture tests. The debug binary is
rebuilt. All 84 local file links across the five updated documents resolve;
no documents or folders were created or moved. The overall visual gate remains open.

## Operator Editor Observation — 2026-09-13

With the wheel retest passed, the operator added blank lines to the configuration
draft. The closer screenshot establishes that the editor's scrollbar exists but
is mostly obscured by the containing page's scrollbar. The earlier claim that
the editor failed to keep its scrollbar visible was incorrect. This finding is
about overlapping tracks; editor typography and editing behavior are unchanged.

Recovery and Settings now mount their page contents through
[`page_scroll_content`](../../src/ui/composites/page_scroll_content.rs). It
reserves the pinned component's full scrollbar hit width plus scaled spacing
inside the scrolling area. The page's scrollbar stays outside that content
gutter, separating it from all nested editors and logs. The shared layout token
`OVERLAY_SCROLLBAR_WIDTH` records the component's fixed hit width; the existing
view models and input entity retain their state and behavior.

The situational ADR 0063 architecture guard
`adr_0063_nested_scrollbars_share_page_content_owner` checks that both page
composites use this owner. V1 step 2 of the
[operator procedure](../runbooks/log-frame-check.md) guards the visible defect:
overflow the editor and page, resize, and drag each scrollbar independently.
The operator reported pass for the recovery retest on 2026-09-13, including
wide/narrow resizing and independent editor/page scrollbar dragging. Repeat in
Settings when the remaining V1 workflow reaches it. Keep fixture
`/tmp/v4vmm-startup-dmizjdrm` for the remaining V1 checks.

Gutter correction checks Green: cargo check, build, fmt check, strict Clippy,
six ADR 0063 unit tests and all 241 architecture tests. The debug binary is
rebuilt. All 86 local file links in the six updated documents resolve. One
shared composite source file was added; no documentation files or folders were
created or moved, and canonical root documents remain in place. Mechanical
checks do not close the remaining Settings visual gate.

## Operator Copy Evidence — 2026-09-13

The operator reported pass for V1 step 3: Ctrl+C and context-menu Copy preserve
selected text across line breaks and full paths, an appended validation result
preserves the selected passage and paused reading position, and Copy repair
report includes entries outside the viewport. The supplied complete report
starts with the configuration load at 15:36:07 UTC and contains validation
results from 15:39:15 through 15:39:40 UTC. It states that no configuration was
saved. Subsequent save/resumption, Settings reading-state, preservation and
cleanup evidence for this fixture is recorded below.

## Operator Settings Width And Page Position — 2026-09-13

The operator confirmed that log windows retain their reading positions after
saving the correction, opening the app, and switching Settings groups. The
supplied screenshots show the resumed app's background observations recorded at
15:42:58 UTC and the earlier correction report. They also expose a fixed-width
Settings column on a wide window and one page offset reused across Library
and Diagnostics. Log reading-state acceptance does not accept page scrolling.

The shared Settings frame now uses available width inside the existing gutter.
The obsolete 720-pixel column token is removed; input controls retain their
scaled sizing and ability to fill the allocated width. `SettingsScrollHandles`
in the shared Settings composite owns a distinct scroll handle for each group,
and TopApp retains them while groups or Settings are hidden. The selected group
still comes from SettingsVm; page positions never enter configuration. The
visible page and its scrollbar use the same selected handle. Each group's own
content bounds clamp its restored offset.

The ADR 0069 amendment records this behavior. The unit test
`adr_0069_settings_pages_retain_independent_scroll_offsets` exercises the actual
handles without a window. The existing situational ADR 0069 form/ownership
guards now protect available-width inputs and the retained handle route.
V1 step 4 of the operator procedure checks wide/narrow allocation, separate
Library/Diagnostics positions, visiting General, and leaving/reopening Settings.
The operator reported pass for that retest on 2026-09-13: wide Settings logs,
clear scrollbar gutter, independent Library/Diagnostics/General page positions,
and restoration after visiting Music. Keep the current fixture for the
remaining preservation and fixture checks.

Settings correction checks Green: cargo check, build, fmt check, strict Clippy,
seven ADR 0069 unit tests and all 241 architecture tests. The composite token
guard now checks production source, preserving ADR 0034's existing exception
for unit-test coordinates. The debug binary is rebuilt. All 94 local file links
in the seven updated documents resolve; no documentation files or folders were
created or moved. Operator width and page-restoration acceptance is recorded
above; the remaining packet gates stay open.

## Operator Theme, Scale And Library Evidence — 2026-09-13

The operator reported pass for Light and Dark at XS, S, M, L and XL, normal and
narrow Diagnostics widths, readable log text, separate usable scrollbars and
unobscured Go to latest. The procedure previews these settings without Save and
restores Dark/M. Music still shows one playlist and three tracks. Fixture
preservation inspection, cleanup and the separate startup-details disclosure
check are accepted below. The remaining Show/session checks remain open.

## Operator Preservation Evidence — 2026-09-13

The supplied repair-inspect reports for `/tmp/v4vmm-startup-dmizjdrm` are Green.
The original configuration is preserved in the owner-only backup
`config/v4vmm/.v4vmm-config-2851814-0.backup`, SHA-256
`beef07f5ddbae4bc5dd1c8df30385501d3ac7cced8f84d1e42580e5f66f54b73`.
Unedited configuration values are preserved, with no changed fields or residual
candidates. Music, bindings, the library, migrations 1–11, one playlist, three
tracks and three playlist memberships are preserved. No music/database probes
remain, and tool blockers are preserved. The configuration bytes changed as
expected for the saved syntax correction.

Preservation is accepted. The operator confirmed cleanup of
`/tmp/v4vmm-startup-dmizjdrm` on 2026-09-13. The startup-details hide/reopen
check used a separate read-only recovery fixture, as recorded below.

## Operator Startup Disclosure Evidence — 2026-09-13

The operator accepted the separate startup-details hide/reopen check. After
scrolling the log, hiding and reopening details twice retained its reading
position and following state. The supplied repair-inspect reports confirm
unchanged configuration bytes, preserved unedited values and selected music,
and no backups or residual candidates. Migrations 1–11, one playlist, three
tracks and playlist bindings (`a.wav`, `b.wav`, `c.wav`) are preserved; there
are no database or residual music probes, and tool blockers are preserved.
No backup was needed because the check saved no correction. Preservation is
accepted, and the operator confirmed disclosure fixture cleanup on 2026-09-13.
This closes V1; V2–V3 remain open.

## Operator Show Log Evidence — 2026-09-13

The operator accepted initial Producer and Publisher log presentation in the
isolated shared-log fixture. Both use the recovery frame and compact monospace
text, initially show entry `000080` with following enabled, and name the correct
service: `mixxx-now-playing.service` or
`musicindex-live-publisher@log-frames-fixture.service`.

The operator also accepted automatic refresh and following after journal
appends. Publisher followed through entry `000092`; Producer followed through
entry `000104`, without reopening either visible log.

Paused Producer reading stayed anchored during the next append. Producer and
Publisher retained separate positions and paused states across source switches,
closing/reopening each pane and visiting Settings then Show, including refreshed
contents. Tab then Enter on Producer's Go to latest reached entry `000116` and
resumed following while Publisher retained its paused position. These checks
are accepted.

The operator also accepted exact long-line copying with keyboard/context-menu
Copy, viewport scrollbar containment and dragging, and wheel isolation. The
accompanying narrow screenshots expose a separate allocation failure: at one
card column the source header consumes almost all of the remaining log pane.

## Open Log Height Correction — ADR 0070

The operator requested usable logs at the expense of cards while keeping
sidebar Logs actions reachable. [ADR 0070](../adr/0070-show-log-space-priority.md)
records the exception to simultaneous card visibility before implementation.
The Show view model now budgets the log before card space, retains a preferred
height across temporary constraints, and bounds both by the available region.
The Show log composite wraps the cards in a scrolling viewport with shared
scrollbar clearance. The sidebar retains its own allocation. The shared split
measurement is anchored to its actual container.

Situational ADR 0070 guards are
`adr_0070_log_height_is_bounded_and_survives_close`,
`adr_0070_log_space_precedes_cards_and_restores_after_resize`, and
`adr_0070_show_log_budget_and_card_scroll_have_shared_owners`. Mechanical checks
are Green: cargo check, debug build, format check, strict Clippy, two ADR 0070
height tests, five existing Show log tests, three shared split tests and all
242 architecture guards. The first new architecture assertion failed because
formatting split its source expression across lines; it now uses the existing
whitespace-normalization helper, and the complete guard suite passes.
The operator accepted the initial narrow allocation retest: readable logs at
one card column, scrolling all cards, independent reachable sidebar actions,
divider resizing and restoration of preferred height. The screenshot then
identified excessive header height from a publisher unit wrapping into six
lines. The shared header now clips the selectable source to one line, with full
identity on hover and exact Ctrl+A/Copy through the existing selection owner.
Its second row contains the line count and view-model service state; the longer
service explanation is available on hover. Event sources clear that service
state. No source text is abbreviated or rewritten for copying.

`adr_0070_show_log_header_is_compact_and_preserves_identity` guards the shared
header route, with state assertions in existing Show source/selection tests.
Header mechanical checks are Green: cargo check, debug build, format check,
strict Clippy, all 55 Show view-model tests and all 243 architecture guards.
The header text has a focused helper within the shared Show log composite.
The operator accepted the compact-header retest on 2026-09-13 for both Producer
and Publisher: two compact text rows at narrow width, readable count/state,
reachable Close, full source on hover, and complete unit identity copied with
Ctrl+A/Ctrl+C and right-click Copy. The Show Light/Dark XS–XL sweep is also
accepted on 2026-09-13. Event framing/source isolation is accepted in the
evidence below; V3 and fixture preservation/cleanup remain open.

## XL Narrow Footer And Count Hover — ADR 0070

The operator's XL screenshot showed the full following label wrapping one
character per line and consuming the log body. The clipped count exposed only
the service explanation on hover. This failed the theme/scale check.

The shared footer now stays on one row. `LogFooterLayout` chooses compact
presentation from the measured viewport width divided by UI scale. Compact
states are Following, Paused and History changed, with the full explanation on
hover; Go to latest uses its named icon, full tooltip, accessible label and
existing typed availability/activation. Wider frames keep the full labels.
The Show header count has a separate tooltip containing its complete text.

The situational ADR 0070 guards are
`adr_0070_compact_footer_keeps_follow_pause_and_anchor_loss_explicit` and
`adr_0070_log_footer_bounds_text_and_retains_the_follow_action`; the existing
header guard also checks count hover. Mechanical checks are Green: cargo check,
debug build, format check, strict Clippy, all four log-reading view-model tests
and all 244 architecture guards. The operator accepted the retest on 2026-09-13:
the XL narrow footer stays on one row with visible log text; the state and count
expose complete hover text; the compact Go to latest control works with mouse
and Tab/Enter; widening restores full labels. The narrow Diagnostics regression
also passed. The operator subsequently accepted the broader Show Light/Dark
XS–XL sweep on 2026-09-13 for both Producer and Publisher: log text remains
visible through narrow/wide resizing, headers and footers stay compact,
scrollbars stay contained, all three cards can be reached by scrolling, and
sidebar Logs actions stay reachable. Dark/M was restored and the fixture
retained. V3 and fixture preservation/cleanup remain open.

## Event Log Evidence — 2026-09-13

The operator accepted the isolated fixture's Event log check. Event · No event
uses the same monospace frame and usable footer at narrow and wide widths.
Appending service journal entries while the Event report is open does not
replace it with service text. Switching back to Producer restores its paused
position and noted entry; reopening Event restores the Event report. This
acceptance covers the existing no-event snapshot and source switching; no real
event or external service was created. The fixture remains open for V3 and
preservation/cleanup.

## Reading Anchor Evidence — 2026-09-13

The operator accepted the Producer journal trimming check. After pausing within
the newest 30 entries and trimming the fixture journals to those entries, the
next snapshot reports 30 lines, retains the noted entry at the same position,
and stays paused without a history-changed warning.

The operator also accepted complete journal replacement on 2026-09-13. The
next snapshot reports 80 lines and shows the oldest replacement entry while
remaining paused. The footer explains that earlier text is unavailable, using
History changed with a full hover explanation in compact mode. Go to latest
reaches the newest entry, resumes following and clears the history-change
message. This completes V2 visual checks; V3 and fixture preservation/cleanup
remain open.

## Session Drain Evidence — 2026-09-13

The operator accepted V3's drain-frame checks using
`/tmp/v4vmm-startup-kqcrv2ir`. The copied report records session 1 finishing at
17:47:37 UTC and opening fresh session 2 at 17:48:52 UTC. Session 2 started
ending at 17:49:02 UTC; six drain observations from 17:49:07 through 17:49:42 UTC retained
the named remaining work, `FixtureSessionCommand: 1`, with maintenance
unavailable. This is the fixture's intended held-command state.

The operator's pass confirms the shared frame, controls outside its viewport,
automatic following during retries, stable paused reading on another retry,
keyboard Go to latest and whole-report copying. The subsequent operator pass
accepts paused reading through session resumption: Diagnostics retains the noted
entry and paused state, and Go to latest exposes the fresh-session opening
entry. The next session was armed for the following-state check. The operator
accepted that check on 2026-09-13: with following enabled, release/drain/check/open
retains following and automatically displays the fresh-session opening entry
in Diagnostics. Narrow/wide report inspection in Light/Dark at XS–XL is also
accepted; Dark/M was restored. This completes V1–V3 visual checks. Editor
Close/Reopen inspection and fixture preservation/cleanup remain open.

## Configuration Editor Close And Reopen — ADR 0066

During V3 acceptance the operator reported that the configuration editor lacks
a clear exit. The ADR 0066 disclosure amendment adds Close editor beside the
shared repair heading, with Reopen editor when collapsed. Closing hides the
editing controls and retains the source revision, selected field, all draft
values and input entity. The repair log stays visible. Reopening does not
reread the file, save or discard edits. Explicit Reload file still discards
the draft and reads a new revision. Completion of admitted work leaves the
editor closed if the operator closed it.

Owners: `CorrectionVm` supplies disclosure state, labels and availability;
`ConfigurationEditor` routes the action without work submission or input
synchronization; `configuration_correction` supplies the shared header and
collapsed entry action in Settings and recovery. Existing typography, spacing
and Secondary control tokens own appearance. No configuration key is added.
This follow-up is part of the current packet; task 006's original repair gate
remains closed.

The situational ADR 0066 behavioral guards are
`adr_0066_closing_retains_raw_and_field_drafts_until_explicit_reload`,
`adr_0066_closed_editor_accepts_admitted_validation_and_save_without_reopening`
and `adr_0066_load_completion_keeps_closed_editor_closed_on_success_or_failure`.
They cover concurrent external revision changes, hidden-input callbacks, retained
raw/field drafts, worker suspension, and completion without forced reopening.
The existing shared-repair architecture guard remains binding. Mechanical
checks are Green: cargo check, debug build, format check, strict Clippy, all
1,373 unit tests and all 244 architecture guards. The ten existing documentation
examples remain ignored. The operator accepted the Settings regression on
2026-09-13: Close hides the editing controls while retaining the log without a
new load/save entry; Library/Diagnostics switches keep it closed; Reopen restores
the selected field and unsaved value, including after selecting another field.
Mouse and Tab/Enter controls remain reachable without overlap at narrow XL.
Explicit Reload restores the original value. The app remains at XL for the
recovery regression; that check and fixture preservation/cleanup remain open.

## Recovery Keyboard Activation And Preservation — 2026-09-13

The operator reported that Tab followed by Enter or Space closes the editor
and immediately reopens it. That fails the recovery keyboard check and requires
a focused Settings regression after the shared primitive changes. Returning to
the normal app automatically restored Dark/M; manual reselection was not needed.

The pinned GPUI 0.2.2 code emits a keyboard ClickEvent on key-up for a focused
clickable element. The shared Button also invokes its activation handler on
key-down, so one physical press can dispatch twice. The ADR 0069 amendment
keeps keyboard activation in the key-down route and admits only mouse events
to the activation click route. Editor actions and draft ownership do not change.
The situational guard
`adr_0069_keyboard_press_and_synthesized_release_activate_once` checks press,
held-repeat and synthesized-release sequences for Enter/Space. The existing
`adr_0069_keyboard_buttons_show_focus_without_layout_shift` architecture guard
now requires the click filter at the shared dispatch site. Mechanical checks
are Green: cargo check, debug build, strict Clippy, all 1,374 unit tests and
244 architecture guards. The operator accepted the Settings/recovery retest on
2026-09-13, except for the new focused-input Escape finding below.

The supplied `session-held-command` preservation inspection is Green:
configuration differences are normal workspace preferences only; music, one
playlist, three tracks and memberships, bindings a.wav/b.wav/c.wav and migration
records 1–11 are preserved. No music or database probes remain, and tool
blockers are preserved. Keep `/tmp/v4vmm-startup-kqcrv2ir` for the keyboard retest,
then repeat inspection before cleanup.

## Editor Escape And Focus Return — 2026-09-13

The operator passed the initial Escape retest, then clarified that Escape must
leave the text input and focus Close editor while keeping the editor open.
The ADR 0066 disclosure amendment now routes the input's existing bubbled
Escape action only to the Close editor focus handle. The input handles its own
menus first; Enter and Tab retain their text-editing behavior while it is
focused. Visible help explains that Escape focuses Close editor, then Enter or
Space activates that button.

`CorrectionVm` owns separate, idempotent CloseEditor/ReopenEditor intents.
`ConfigurationEditor` keeps the draft and returns focus to the disclosure
control when Close editor is activated. The shared correction composite moves
focus without changing the editor, draft or report when the input's Escape
action bubbles, in both Settings and recovery. The shared Button accepts the
presenter's retained focus handle and registers it as a tab stop. Existing
control, typography and spacing tokens own the appearance; no global shortcut
is added.

The existing draft-retention behavioral guard now also checks repeated Close
and Reopen and their typed availability. The situational guard
`adr_0066_editor_escape_focuses_close_without_closing` requires
the shared input-action route to only move focus, without closing, changing
the draft or submitting work, and requires the button tab stop. Mechanical
checks are Green: cargo check, debug build, format check, strict Clippy, all
1,374 unit tests and 245 architecture guards. The ten existing documentation
examples remain ignored. The operator accepted the
[operator procedure](../runbooks/log-frame-check.md#configuration-editor-close-and-reopen--adr-0066)
on 2026-09-13: Escape from the focused input focuses Close editor while the
editor stays open; activating that button closes it. The field, draft and log
survive both steps in Settings and recovery. This completes the packet's visual
checks. Final preservation is recorded below. Task 007 has not started.

## Final Preservation — 2026-09-13

The operator supplied the final `session-held-command` inspection after the
accepted Escape retest. Preservation is Green: configuration differences are
normal workspace preferences only; music, one playlist, three tracks and their
three playlist memberships, bindings a.wav/b.wav/c.wav, migration records 1–11
and tool blockers are preserved. No music probes or database probes remain.
The final preservation gate is accepted. The operator confirmed cleanup of
`/tmp/v4vmm-startup-kqcrv2ir` on 2026-09-13; both V1 fixtures were already cleaned.
This closes the packet's final gate. All mechanical, visual, preservation and
cleanup checks for this packet are complete. ADRs 0063 and 0070 are Implemented.
ADR 0066 task 007 follows in a fresh session and has not started.

## Scope Limits And Rollback

Log-source selection and report retention are unchanged. Read-only service
snapshots refresh at the existing observation cadence; no playback, service
restart or optional-tool reinitialization was added. External UTC emitters,
compact cards, full-width log docking and inactive transport remain separate
scheduled work. ADR 0070's log-height priority is included in this correction.
Reading state lasts for the window and is not persisted in configuration.

Revert this packet's presentation and reading-state changes together if its
operator gate fails. Preserve fixture evidence, configuration, backups and music.
ADR 0066 task 007's prerequisite from this packet is met; it requires a fresh session.
