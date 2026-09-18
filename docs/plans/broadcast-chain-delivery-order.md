# Broadcast Chain Delivery Order

## Status

Active index - 2026-09-13. Operator-approved order; one implementation packet
per session.

## Purpose

Three repositories hold task packets for one system. Each repository has its own
plan, and no plan can see the others. This document is the only place that
states the order across all three.

Read this before you start a session. Update the Progress table when a packet
lands.

## The Repositories

| Repository | Governing ADR | Packets |
|---|---|---|
| `v4vmm` | ADR 0059, broadcast control surface | 17 |
| `musicindex-live-publisher` | ADR 0003 show log, plus control surface support | 4 |
| `splitkit` | ADR 0001, reserved live items | 5 |

Task 013 presentation correction — 2026-09-18: the operator rejected the inline
repair/diagnostics layout. [ADR 0074](../adr/0074-repair-and-diagnostics-pages.md)
records separate pages, action placement and report viewports within task 013.
V1–V3 behavior, replacement presentation and both final inspections are accepted.
Fixture cleanup is confirmed. Task 013 is complete and ADR 0074 is Implemented.
The page choices and text size passed. The corrected Database Check page passed
short-window scrolling, text selection and complete report copy.
The operator accepted the restored Settings title and single view toggle inside
the content container, including switching in both directions. The theme and
scale check exposed an empty Background tools page. Its tool-list correction
passed normal/narrow scrolling and view switching. Settings passed normal/narrow
and short-height checks at XL scale in both themes, including menu input and
preference restoration. After a normal/recovery window mix-up, the latest
screenshot confirms the unsupported fixture's startup check 1 and disabled
Open app. The operator then accepted explicit Check again with the unsupported
schema still reported and Open app disabled. Recovery Startup, Configuration
and Database then passed normal/narrow and short-height layout, scrolling,
view switching and report copy. The absent repair action is confirmed. The
operator reported both final preservation checks as passed and confirmed cleanup
of both fixtures. No task 013 gate remains open. Task 004 retains its separate gate.
This correction does not start another delivery phase.

## Current Delivery Order

Approved by the operator on 2026-09-10; amended on 2026-09-11 to prioritize
the ADR 0069 existing-field Settings foundation while playback is deferred.
Amended at the operator's request on 2026-09-13: shared log framing and following
now precede ADR 0066 task 007; the remaining recovery packets wait for its acceptance.
The operator's 2026-09-13 single-column finding adds
[ADR 0070 log-height priority](../adr/0070-show-log-space-priority.md) to that
packet's corrections. Cards may scroll while logs are open; compact density,
full-width docking, title readability and inactive transport stay in order 4.
Amended 2026-09-13 at the operator's request: the independent ADR 0071 shared
text-selection packet follows the completed log packet in a fresh session,
before recovery 007. It retains the log packet's closed acceptance.
Complete one phase or packet per session. Restored human checks remain visible in the Progress table and
[pending human checks](../pending-human-checks.md); listing them does not
require walking them before the independent chain work.

| Order | Work | Completion point |
|---|---|---|
| 1 | Governance reconciliation | Sweep ADR statuses and review gate prose; retire replaced requirements before indexing survivors; correct AGENTS.md; retain ADR 0039 as Proposed and unscheduled |
| 2, keyboard correction | [Platform shortcuts — ADR 0067](../tasks/adr-0067-task-001-platform-shortcuts.md) | Complete - 2026-09-11; Ctrl shortcuts, focus handling and Settings responsiveness accepted; fixture cleanup confirmed |
| 2, Settings foundation | [Grouped Settings — ADR 0069 task 001](../tasks/adr-0069-task-001-grouped-settings-foundation.md) | Complete - 2026-09-11; mechanical checks Green; V1–V3 and all preservation inspections accepted; fixture cleanup confirmed; no configuration-format or playback changes |
| 2, before recovery 007 | [Shared log frames and following — ADR 0063 task 005](../tasks/adr-0063-task-005-shared-log-frames-and-following.md) | Complete - 2026-09-13; mechanical checks Green; V1–V3, ADR 0070 narrow layout/header/footer, editor Close/Reopen and corrected Escape behavior accepted; final preservation and all fixture cleanup confirmed; ADRs 0063/0070 Implemented; ADR 0066 task 007 completion is recorded below |
| 2, text selection | [Shared text selection — ADR 0071 task 001](../tasks/adr-0071-task-001-shared-text-selection.md) | Complete - 2026-09-15; migration and pinned ADR 0072 correction verified; mechanical checks Green; available X11 operator checks, preservation and fixture/scratch cleanup accepted; IME composition and Wayland untested; completed log packet remains closed |
| 2 | [Configuration and startup failure recovery — ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md) | Accepted - 2026-09-10; tasks 001–003 and 005–006 complete; task 004 implemented with remaining operator gate open; task 006 complete on 2026-09-13 with mechanical checks Green and operator V1–V6, preservation and cleanup accepted; task 007 complete on 2026-09-16 with mechanical checks Green, V1–V3 and preservation accepted, Library/Show follow-ups accepted with preservation and cleanup, and no remaining startup fixtures in the checked temporary directories; task 008 complete on 2026-09-17 with operator V1–V3, presentation, preservation and cleanup accepted; task 009 complete on 2026-09-17 with mechanical checks Green and operator V1–V3, presentation, configuration restoration, preservation and cleanup accepted; task 010 complete on 2026-09-17 with operator acceptance, preservation and cleanup confirmed; task 011 complete, mechanical checks Green, operator V1–V3/preservation/cleanup accepted; task 012 is complete on 2026-09-17 with mechanical checks Green, operator V1–V3, presentation, preservation and cleanup accepted; task 013 complete on 2026-09-18 with operator V1–V3, presentation, preservation and cleanup accepted in the [phase plan](adr-0066-startup-recovery-phase-plan.md); finish implementation and acceptance before config format changes |
| 2, Settings follow-through | [ADR 0069 editor, metadata and preset phases](adr-0069-settings-presets-phase-plan.md#sequence-and-stopping-points) | Planned; shared editor waits for 0066 task 007, with 005–006 complete; new persisted mode/resource/preset formats wait for the full 0066 gate; author bounded packets before implementation; audio stays deferred |
| 3 | Relay durability through adoption | splitkit reserved 001 → 002 → 003; deploy, reserve an event, configure the publisher to use it, then implement the v4vmm reservation packet |
| 3, follow-through | splitkit reserved 004 → 005 | List/delete, final guards, and delivery reconciliation; explicitly scheduled after adoption, with interim command-line reservation allowing these before the v4vmm packet if needed |
| 4 | Remaining narrow Show layout and A10 | Future decision and packet: compact cards, full-width log docking, card-title readability, and hiding transport only when every typed action is unavailable; one combined visual gate. ADR 0070 log-height priority is already in the shared-log packet |
| 5 | Remaining UTC emitter/adapter work | Shared app log framing and following moved before recovery 007; agree the common UTC contract and correct external emitters in bounded packets after the earlier chain work |
| 6 | Steady state | v4vmm ADR 0064 task 002; publisher show-log 001 → 002; installed-but-unconfigured publisher state |

**Priority trigger:** when a real show is scheduled, publisher show-log task
001 becomes the next packet ahead of this order and must be operational before
that show if its timeline is to be captured. Verify that logging is enabled
and writing entries. Timeline data that was never recorded cannot be recovered.
This trigger does not combine two implementation phases into one session.

### Dependencies And Adoption

- Configuration failure behavior precedes deferred item 7's workspace-config
  format migration and any other config format change.
- ADR 0069 task 001 is independent of the remaining ADR 0066 acceptance gate:
  it changes grouping and report navigation using current fields and writers.
  Its acceptance does not waive the configuration-format gate. The later
  explicit task 005 scheduling exception is recorded below.
  Later Settings phases reuse the ADR 0066 editor/transition owners. PulseAudio
  and JACK selectors wait for ADR 0068 isolation; native PipeWire is later.
- splitkit 002 adds the reservation route; 003 adds restoration and idle-TTL
  exemption. The v4vmm reservation packet consumes 002's contract and cannot
  claim operational durability without 003 deployed.
- Ordinary events remain ephemeral. Their default expiry is after 24 hours
  without activity; a restart also loses them. Existing events are not
  automatically converted to reserved events.
- Adoption means the publisher uses a newly reserved identity and its saved
  token. Verify identity/token survival across relay restart and idle expiry,
  and resume publication after the restored event initially serves an empty
  payload. Preserve the old configuration for rollback during deployment.
- The interim reservation uses the documented relay API from an operator
  terminal. v4vmm's future reservation packet must define credential handling,
  selection, and publisher configuration without treating an ordinary Create
  as a durable reservation.
- A11/A12's shared frame, compact monospace text, long-line scrolling and
  per-source following now belong to ADR 0063 task 005 before recovery 007.
  Its gate includes Diagnostics and recovery. External timestamp corrections
  retain their separate slot and do not block the app viewport work.

- The publisher's machine-readable configuration facts already exist. A new
  v4vmm packet must consume them to distinguish installed-but-unconfigured
  from the current service states.

Show view-model decomposition and wider all-target Clippy cleanup remain
unscheduled. Cache/dump policy and play history remain separate future decisions.
[ADR 0068: Show cue and audition isolation](../adr/0068-show-cue-and-audition-isolation.md)
is Proposed - 2026-09-11; its implementation packets are not yet scheduled.

On 2026-09-11, the operator clarified that a playlist loads into the Show cue
before Show starts playback; other playback buttons use a separate audition
audio path. Current Music Play shares the Show session and producer. Task 004's
playback-dependent acceptance checks are paused pending acceptance and
implementation of the proposed separation; its producer screenshot also contains
an unresolved mpv IPC read error. See [the correction and remaining gate](../tasks/adr-0066-task-004-optional-tool-isolation.md#playback-workflow-correction--2026-09-11).
This records a prerequisite to those checks, without starting another packet or
changing the approved cross-repository order.

The operator deferred playback work on 2026-09-11 and requested independent
backlog closure. The [pending-work reconciliation](../reviews/2026-09-11-pending-work-reconciliation.md)
records that bounded pass. Non-playback report/path-repair and inherited Music
checks can proceed while audio/publication acceptance remains paused. Task 004's remaining gate stays open; the later explicit task 005 scheduling
exception is recorded below.

The operator then accepted [ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md)
on 2026-09-11. Its existing-field foundation is complete: mechanical checks are Green,
operator V1–V3 and preservation passed, and fixture cleanup is confirmed.
This closes task 001 without accepting any open recovery or playback check.
Preset recall edits a draft; configuring a setup must not activate the
chain. Future Settings phases remain subject to the dependencies above.

The operator then requested ADR 0066 task 005 on 2026-09-11. Its session
drain/resumption implementation and mechanical checks are complete, using task
004's delivered owners while retaining task 004's open acceptance gate.
Task 005's [operator check](../runbooks/startup-recovery-check.md#task-005-session-drain-and-resumption),
preservation inspection and fixture cleanup passed on 2026-09-11. Task 006 is
complete on 2026-09-13. V1–V3 are accepted on 2026-09-11,
including preservation inspection and confirmed fixture cleanup.
V4 is accepted on 2026-09-13, including corrected preservation inspection and
confirmed fixture cleanup. The inspector's omitted normal-workspace allowance
is corrected, with six fixture tests Green. V5 is accepted on 2026-09-13,
including final preservation inspection and confirmed fixture cleanup.
V6 is accepted on 2026-09-13, including conflict handling, copied-draft and final
preservation checks, and confirmed fixture cleanup. Task 006's gate is closed.
Temporary viewport measurements are removed.
Task 007 is complete on 2026-09-16 with mechanical checks Green. V1–V3 and
preservation are accepted; the Library/Show follow-ups are accepted with
preservation and cleanup. No startup fixtures remain in the checked temporary
directories. The packet corrects an unsupported extra gate inferred from a
port-only log. ADR 0073 is Implemented. Task 008 is complete on 2026-09-17 with
mechanical checks Green; operator V1–V3, Settings/core-recovery presentation,
preservation in both cases and fixture cleanup are accepted.
Task 009 is complete on 2026-09-17 with mechanical checks Green; operator V1–V3,
normal/narrow presentation, configuration restoration and all preservation checks
are accepted. The operator confirmed cleanup of `/tmp/v4vmm-startup-4psemojh`.
Task 010 is complete on 2026-09-17 with operator acceptance, preservation and cleanup confirmed. Task 011 is complete on 2026-09-17 with mechanical checks Green; operator acceptance, preservation, normal restoration and cleanup are confirmed.
This explicit scheduling exception accepts no playback check or configuration-format change.

## Progress

Update this table when a packet lands.

| Repository | Packet | State |
|---|---|---|
| `v4vmm` | [governance reconciliation](../reviews/2026-09-10-governance-reconciliation.md) | complete - 2026-09-10; documentation only; surviving gates indexed below |
| `v4vmm` | [pending-work reconciliation](../reviews/2026-09-11-pending-work-reconciliation.md) | Complete - 2026-09-11; ADR header-format debt and corpus guard closed; stale startup backlog and shortcut instructions corrected; 235 architecture tests and required checks Green; human gates remain open |
| `v4vmm` | [0069 grouped Settings foundation 001](../tasks/adr-0069-task-001-grouped-settings-foundation.md) | Complete - 2026-09-11; mechanical checks Green; V1–V3 and all preservation inspections accepted; fixture cleanup confirmed; ADR remains Accepted for later phases |
| `v4vmm` | [0066 configuration/startup failure recovery](adr-0066-startup-recovery-phase-plan.md) | ADR Accepted; tasks 001–003 complete, including [003 operator acceptance, preservation and fixture cleanup](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md#operator-evidence--2026-09-11) - 2026-09-11; task 004 implementation and mechanical gate Green; presentation case and preservation accepted - 2026-09-11; playback checks paused for cue/audition separation and observed mpv IPC error; remaining operator checks and final fixture cleanup open; task 005 complete as recorded below; task 006 complete on 2026-09-13 with mechanical checks Green and operator V1–V6, preservation and cleanup accepted; 007 complete on 2026-09-16 with mechanical checks Green, V1–V3 and preservation accepted, Library/Show follow-ups accepted with preservation and cleanup, and no remaining startup fixtures in the checked temporary directories; 008 complete on 2026-09-17 with operator V1–V3, presentation, preservation and cleanup accepted; 009 complete on 2026-09-17 with mechanical checks Green and operator acceptance, preservation and cleanup confirmed; 010 complete with operator acceptance, preservation and cleanup confirmed; 011 complete with operator acceptance, preservation and cleanup confirmed; task 012 is complete on 2026-09-17 with mechanical checks Green, operator V1–V3, presentation, preservation and cleanup accepted; task 013 complete on 2026-09-18 with operator V1–V3, presentation, preservation and cleanup accepted |
| `v4vmm` | [0066 session drain and resumption 005](../tasks/adr-0066-task-005-session-drain-and-resumption.md) | Complete - 2026-09-11; mechanical checks Green; [operator V1–V3 and preservation](../tasks/adr-0066-task-005-session-drain-and-resumption.md#operator-evidence--2026-09-11) accepted; fixture cleanup confirmed; task 006 completion recorded below |
| `v4vmm` | [0063 shared log frames and following 005](../tasks/adr-0063-task-005-shared-log-frames-and-following.md) | Complete - 2026-09-13; mechanical checks Green; V1–V3, ADR 0070 narrow layout/header/footer, editor Close/Reopen and corrected Escape behavior accepted; final preservation and all fixture cleanup confirmed; ADRs 0063/0070 Implemented; ADR 0066 task 007 completion is recorded below |
| `v4vmm` | [0066 configuration repair and resumption 006](../tasks/adr-0066-task-006-configuration-repair-and-resumption.md) | Complete - 2026-09-13; mechanical checks Green; [operator V1–V6](../runbooks/startup-recovery-check.md#task-006-configuration-repair-and-resumption), preservation inspection and fixture cleanup accepted; task 007 completion is recorded below |
| `v4vmm` | [0066 optional-tool correction and retry 007](../tasks/adr-0066-task-007-optional-tool-correction-and-retry.md) | Complete - 2026-09-16; mechanical checks Green; V1–V3 behavior, presentation and preservation accepted; narrow Library, drag responsiveness and [ADR 0073 Show card overflow](../adr/0073-show-card-overflow-scrolling.md) follow-ups accepted; V2/V3 and Library/Show follow-up cleanup confirmed; no earlier startup fixtures remain in the checked temporary directories; packet reconciles the unsupported port-only fixture gate; [operator procedure](../runbooks/startup-recovery-check.md#task-007-optional-tool-correction-and-retry) retained for regression; task 008 complete on 2026-09-17 with operator V1–V3, presentation, preservation and cleanup accepted; task 009 complete on 2026-09-17 with mechanical checks Green and operator V1–V3, presentation, configuration restoration, preservation and cleanup accepted; task 010 complete on 2026-09-17 with operator acceptance, preservation and cleanup confirmed; task 011 complete, mechanical checks Green, operator V1–V3/preservation/cleanup accepted; task 012 is complete on 2026-09-17 with mechanical checks Green, operator V1–V3, presentation, preservation and cleanup accepted; task 013 complete on 2026-09-18 with operator V1–V3, presentation, preservation and cleanup accepted; task 004 retains its separate gate |
| `v4vmm` | [0066 converter verification and setup 008](../tasks/adr-0066-task-008-converter-verification-and-setup.md) | Complete - 2026-09-17; mechanical checks Green; operator V1–V3, Settings/core-recovery presentation and preservation in both cases accepted; normal-mode restoration and fixture cleanup confirmed; [operator procedure](../runbooks/startup-recovery-check.md#task-008-converter-verification-and-setup) retained for regression; task 009 complete on 2026-09-17 with operator V1–V3, presentation, configuration restoration, preservation and cleanup accepted |
| `v4vmm` | [0066 conversion retry and retained input 009](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md) | Complete - 2026-09-17; mechanical checks Green; [operator V1–V3](../runbooks/startup-recovery-check.md#task-009-conversion-retry-and-retained-input), normal/narrow presentation, configuration restoration and preservation accepted; fixture cleanup confirmed; task 010 complete on 2026-09-17 with operator acceptance, preservation and cleanup confirmed; task 011 complete with operator acceptance, preservation and cleanup confirmed |
| `v4vmm` | [0066 database check and backup 010](../tasks/adr-0066-task-010-database-check-and-backup.md) | Complete - 2026-09-17; mechanical checks Green; [operator V1–V3](../runbooks/startup-recovery-check.md#task-010-database-check-and-backup), Settings/recovery normal/narrow presentation, report copy, responsiveness and preservation in both cases accepted; normal-mode restoration and cleanup confirmed; task 011 complete with operator acceptance, preservation and cleanup confirmed |
| `v4vmm` | [0066 database maintenance and preservation 011](../tasks/adr-0066-task-011-database-maintenance-and-preservation.md) | Complete - 2026-09-17; mechanical checks Green; [operator V1–V3](../runbooks/startup-recovery-check.md#task-011-database-maintenance-and-preservation), manifest contents, reopening, report retention, normal/narrow presentation, both-fixture preservation, normal restoration and cleanup accepted; task 012 completion is recorded below |
| `v4vmm` | [0066 database restore 012](../tasks/adr-0066-task-012-database-restore.md) | Complete - 2026-09-17; mechanical checks Green; [operator V1–V3](../runbooks/startup-recovery-check.md#task-012-database-restore), Settings/recovery normal/narrow presentation, report copy, input-change protection, same-window resumption, both preservation inspections and all three fixture cleanups accepted; task 013 complete on 2026-09-18 with operator V1–V3, presentation, preservation and cleanup accepted |
| `v4vmm` | [0066 interrupted upgrade repair 013](../tasks/adr-0066-task-013-interrupted-upgrade-repair.md) | Complete — 2026-09-18; mechanical checks Green; V1–V3 behavior, [ADR 0074 presentation](../runbooks/startup-recovery-check.md#repair-and-diagnostics-pages--adr-0074) and both final preservation inspections accepted; cleanup confirmed; task 004 retains its separate gate; ADR stays Accepted |
| `v4vmm` | [0071 shared text selection 001](../tasks/adr-0071-task-001-shared-text-selection.md) | Complete - 2026-09-15; migration and pinned ADR 0072 correction verified; mechanical checks Green; available X11 operator checks, preservation and fixture/scratch cleanup accepted; IME composition and Wayland untested; completed log packet remains closed |
| `v4vmm` | [0067 platform shortcuts 001](../tasks/adr-0067-task-001-platform-shortcuts.md) | Complete - 2026-09-11; mechanical gate Green; all shortcut checks, preservation and fixture cleanup accepted; ADR Implemented |
| `v4vmm` | [0068 Show cue and audition isolation](../adr/0068-show-cue-and-audition-isolation.md) | Proposed - 2026-09-11; requested ADR drafted; implementation and its visual/audio checks not started; does not close 0066 task 004's paused playback gate |
| `v4vmm` | [0030 006 scroll containers](../tasks/adr-0030-task-006-scroll-containers.md) | implementation recorded; current Music/Settings visual check open |
| `v4vmm` | [0037 001 feed identity](../tasks/adr-0037-task-001-feed-identity-action-parity.md) | implementation recorded; local/Index identity visual check open |
| `v4vmm` | [0037 002 track detail parity](../tasks/adr-0037-task-002-track-header-action-parity.md) | implementation recorded; local/Index track visual check open |
| `v4vmm` | [0043 004 toolbar readiness](../tasks/adr-0043-task-004-guards-and-visual-readiness.md) | implementation recorded; surviving normal/narrow Light/Dark check open |
| `v4vmm` | [0044 003 playlist reorder](../tasks/adr-0044-task-003-playlist-reorder-guards-visual.md) | implementation recorded; shared handle/text-selection conflict guarded; raw profile recovery identified GPUI synchronous test drawing in the desktop binary; fixture launcher rebuilds normally before opening; 16 fixture tests and 251 guards Green; operator restart/reorder/out-of-bounds drag responsiveness and post-check preservation accepted on 2026-09-16; broader Light/Dark handle/menu/insertion and mounted-row visual gate open |
| `v4vmm` | [0054 004 feed hydration](../tasks/adr-0054-task-004-feed-read-model-hydration.md) | implementation recorded; stored metadata visual check open |
| `v4vmm` | [0054 005 track hydration](../tasks/adr-0054-task-005-track-read-model-hydration.md) | implementation recorded; stored metadata/fallback visual check open |
| `musicindex-live-publisher` | control surface 001 | complete - 2026-09-07 (`a5b434e`) |
| `musicindex-live-publisher` | control surface 002 | complete - 2026-09-07 (`459854c`) |
| `musicindex-live-publisher` | show log 001 | not started |
| `musicindex-live-publisher` | show log 002 | not started |
| `v4vmm` | 001 live surface reduction | complete - 2026-09-07 |
| `v4vmm` | 002 event registry schema | complete - 2026-09-07 |
| `v4vmm` | 003 event registry service | complete - 2026-09-07 |
| `v4vmm` | 004 relay observation actor | complete - 2026-09-07 |
| `v4vmm` | 005 broadcast page VM | removed by ADR 0060 / task 001 |
| `v4vmm` | 006 broadcast frame kind | removed by ADR 0060 / task 001 |
| `v4vmm` | 007 broadcast shell | removed by ADR 0060 / task 001 |
| `v4vmm` | 008 service control | complete - 2026-09-08 |
| `v4vmm` | 009 publisher section | implemented - 2026-09-08; visual acceptance met, one layout defect found and fixed (detail line now always present) |
| `v4vmm` | 010 remote hosts | implemented - 2026-09-08; visual acceptance met with `Broken` SSH host |
| `v4vmm` | 014 attach event | implemented - 2026-09-08; visual acceptance met with default target attach and detach |
| `v4vmm` | 011 mpv producer | implemented - 2026-09-08 |
| `v4vmm` | 012 readiness report | implemented - 2026-09-08; visual acceptance met with readiness CLI count |
| `v4vmm` | 015 stream encoder | implemented - 2026-09-08; visual acceptance met with no encoder and `butt`; command-button disappearance fixed and accepted in action feedback 001 on 2026-09-10 |
| `v4vmm` | 013 final guards | implemented - 2026-09-09; ADR 0059 review and final guard coverage complete |
| `v4vmm` | 0063 001 card contract | implemented - 2026-09-08; no visual criteria |
| `v4vmm` | 0063 002 card grid shell | implemented - 2026-09-08; visual acceptance met after the 2026-09-09 fix retest |
| `v4vmm` | 0063 003 detail panel | implemented - 2026-09-08; visual acceptance met after the 2026-09-09 fix retest; log placement moved to 004 |
| `v4vmm` | 0063 004 log bottom pane | implemented - 2026-09-09; mechanical and operator visual acceptance met, including selectable text and right-click Copy |
| `v4vmm` | [show action feedback 001 (A7, A8, A9)](../tasks/show-action-feedback-task-001-command-state-and-result.md) | implemented - 2026-09-10; mechanical acceptance Green; [operator visual acceptance passed](../tasks/show-action-feedback-task-001-command-state-and-result.md#operator-visual-check) for A7, A8, and A9 |
| `v4vmm` | 016 event row in Live Metadata | implemented - 2026-09-09; mechanical acceptance Green; [all operator event recovery checks passed](../runbooks/broadcast-event-recovery-check.md), including Copy, service readiness, resizing, and registry/token preservation; visual acceptance met |
| `v4vmm` | [017 compact event controls and badges](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md) | implemented - 2026-09-10; mechanical checks Green; [operator acceptance and fixture cleanup complete](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check), including preservation and confirmation of three intentional registrations; narrow log-body limitation tracked in the proposal; ADRs 0059/0063 reconciled |
| `v4vmm` | 0064 001 relative local paths | implemented - 2026-09-09; visual acceptance met after path repair converted moved library |
| `v4vmm` | 0064 002 repair report surface | ready |
| `v4vmm` | 0065 001 tag repair service | implemented - 2026-09-09; no visual criteria |
| `v4vmm` | 0065 002 check-all-feeds repair and list actions | implemented - 2026-09-09; visual acceptance met; result-row readability fixed and accepted in action feedback 001 on 2026-09-10 |
| `splitkit` | reserved 001 store boundary | ready |
| `splitkit` | reserved 002 reserved class | ready; response contract pinned against `LiveItemCreateResponse` |
| `splitkit` | reserved 003 restore and TTL | ready; restores reserved identities and exempts them from idle expiry; ephemeral behavior remains unchanged |
| `splitkit` | reserved 004 list and delete | ready; scheduled after adoption, or before the v4vmm reservation packet while command-line reservation is the interim; list envelope pinned |
| `splitkit` | reserved 005 guards and review | ready; follows 004; final relay review also reconciles this plan |

## Surface Rewrite — Complete

ADR 0060 replaced the surface design on 2026-09-07. The ADR 0059 UI packets
were reconciled against it and are complete, including task 017's operator
acceptance on 2026-09-10. The surface rewrite no longer blocks broadcast work.

The restructure ran first under ADR 0060. Cache, audition, and play history
are separate features inside `Music`; they did not gate the structural work.

| Step | Packet | State |
|---|---|---|
| 1 | `docs/tasks/adr-0060-task-001-remove-broadcast-frame.md` | complete - 2026-09-07 |
| 2 | `docs/tasks/adr-0060-task-002-show-screen-mount.md` | implemented - 2026-09-08; visual acceptance met |
| 3 | `docs/tasks/adr-0060-task-003-music-surface.md` | implemented - 2026-09-08; visual acceptance met |
| 4 | `docs/tasks/adr-0060-task-004-live-status-strip.md` | implemented - 2026-09-08; visual acceptance met |

`Show` preceded `Music` so it could own the queue before the curation surface
gave it up.

ADR 0062 rebuilt what `Music` shows:

| Step | Packet | State |
|---|---|---|
| 1 | `docs/tasks/adr-0062-task-001-mixed-entity-row-contract.md` | complete - 2026-09-07 |
| 2 | `docs/tasks/adr-0062-task-002-music-default-view.md` | complete - 2026-09-07 |
| 3 | `docs/tasks/adr-0062-task-003-library-tri-state-control.md` | complete - 2026-09-07 |
| 4 | `docs/tasks/adr-0062-task-004-tile-and-list-modes.md` | complete - 2026-09-07 |
| 5 | `docs/tasks/adr-0062-task-005-retire-recent-feeds-destination.md` | complete - 2026-09-07 |

The ADR 0059 packets were revised against ADR 0060 on 2026-09-08 and are no
longer blocked. Packet 009 established how a section composes into the `Show`
screen mount, so it ran before 012, 014, and 015. These packets are complete;
their dependency order is retained below.

| Order | Packet | Depends on |
|---|---|---|
| 1 | 008 service control | nothing |
| 2 | 009 publisher section and logs | 008 |
| 3 | 010 remote hosts | 009 |
| 4 | 015 stream encoder section | 009 |
| 5 | 011 mpv drop-file producer | nothing |
| 6 | 012 library readiness report | 009, and ADR 0062 for the `Music` list |
| 7 | 014 attach event to publisher target | 009, 010, and publisher task 001 |
| 8 | 013 final guards and readiness | everything above |

ADR 0063 rebuilt the `Show` layout. Its four dashboard packets are complete,
and task 017's compact-item amendment has passed operator acceptance.

| Order | Packet | Depends on |
|---|---|---|
| 1 | `docs/tasks/adr-0063-task-001-card-contract-and-width-class.md` | nothing |
| 2 | `docs/tasks/adr-0063-task-002-card-grid-shell.md` | 001 |
| 3 | `docs/tasks/adr-0063-task-003-collapsible-detail-panel.md` | 002 |
| 4 | `docs/tasks/adr-0063-task-004-log-bottom-pane.md` | 003 |

ADR 0064 repairs the local-file addressing. It blocks nothing above, and the
readiness report of `v4vmm` 012 reads correctly only after task 001 lands.

| Order | Packet | Depends on |
|---|---|---|
| 1 | `docs/tasks/adr-0064-task-001-relative-local-paths.md` | nothing |
| 2 | `docs/tasks/adr-0064-task-002-repair-report-surface.md` | 001 |

ADR 0065 repairs the payment-route tag. The readiness report reads correctly
only after task 001, and the list becomes usable after task 002.

| Order | Packet | Depends on |
|---|---|---|
| 1 | `docs/tasks/adr-0065-task-001-tag-repair-service.md` | nothing |
| 2 | `docs/tasks/adr-0065-task-002-readiness-list-actions.md` | 001 |

ADR 0065 was amended on 2026-09-08: `Check all feeds` performs the repair, so
an operator does not need to know a second action exists.

Unscheduled follow-ups:

1. Write the cache and dump policy decision. `Dump` needs it.
2. Review [proposed ADR 0068](../adr/0068-show-cue-and-audition-isolation.md) and
   schedule its implementation with explicit reconciliation of task 004's
   paused playback checks. Writing the ADR does not establish acceptance.
3. Write the play-history decision.

Unwritten decisions carry no number until somebody writes them. A number
reserved in prose collided once already, when ADR 0063 became the `Show`
dashboard layout.

`v4vmm` packets 001 through 004 shipped and are unaffected. They are backend
work that ADR 0060 does not touch.

## Later Work

These entries include work awaiting packets and unscheduled work. The Current
Delivery Order above determines priority; an entry here is not a second order.

- A `v4vmm` packet for reserving a durable live item. `splitkit` reserved live
  items 002 adds the route, and nothing in `v4vmm` calls it. After 002/003 are
  deployed, command-line reservation and publisher configuration provide the
  interim adoption path. This does not automatically add the new event to the
  app registry. The app packet completes reservation and selection in v4vmm.
- A seventh `ServiceState` in `v4vmm`, for a publisher that is installed and
  not configured. `musicindex-live-publisher` control-surface task 002 supplies
  the two facts that separate it, through `--version` and `config show --json`.
  Landing that packet does not change `v4vmm` on its own. Needs a `v4vmm` packet.
- Episode generation in `v4vmm`, from the show log. Needs a future ADR.
- Post-processing tools in `v4vmm`. Gates publisher-side backup recording.
- Broadcaster identity and quotas in `splitkit`. Options recorded, no decision.
- A remote control API in `musicindex-live-publisher`, for liquidsoap.
- Liquidsoap as a source.
- [Show narrow-layout proposal](show-narrow-layout-proposal.md): automatic/manual
  compact cards and full-width logs, the overlap alternative, and requested
  hiding of the playback bar when all typed actions are unavailable. The
  operator resolved that scope on 2026-09-10; working controls remain.

### Consistent UTC Log Timestamps

Scheduled for separate emitter/adapter design - 2026-09-10. The app framing
and following portion moved before recovery 007 on 2026-09-13. The operator
requires consistent UTC timestamps across all logs, including producers outside `v4vmm` where
changes are needed. This requirement accompanies
[per-log following and reading positions](hig-product-polish-backlog.md#a12---follow-latest-logs-and-remember-each-reading-position).
It is not implemented or an additional gate on task 017.

Recorded UTC is already required for
[ADR 0063 Event reports](../adr/0063-show-dashboard-layout.md#event-diagnostics-reuses-the-bottom-pane).
[ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md#recovery-reports-explain-the-failure-and-the-next-action)
was accepted on 2026-09-10 and extends that rule to startup and maintenance
reports. The common contract must preserve these scoped decisions; it
defines their shared precision and broader emitter/adapter behavior rather
than reopening canonical time or implying that all current logs already comply.

Agree a common timestamp contract in the owning ADRs before assigning packets:
record the actual event time, use an unambiguous UTC date/time and zone marker,
and specify consistent precision. Audit Event reports, local/remote service
journals, fixture output, and affected publisher/producer/relay log emitters.
`ServiceControl::logs` in `src/broadcast/control.rs` currently supplies no
explicit UTC or timestamp-output format. The fixture journal also needs an
explicit UTC marker. Other repositories have not yet been audited for this work.

Make necessary corrections at the emitter or journal/transport boundary.
Do not append a UTC label to an unknown local timestamp or assign the display
time to an older event. Preserve source instants and uncertainty where an input
does not identify its timezone.

Future preferences may display local time and follow the user's locale.
Those are presentation preferences over canonical UTC instants; they must not
change the recorded event time. Their format and copy behavior need their own
acceptance criteria when implemented.

Delivery: decide the contract, update affected emitters/adapters in their owning
repositories, then integrate the app presentation and fixtures. Verify the
same instants under different local/remote timezone settings, with explicit
UTC output by default. Add mechanical and operator checks to those future
packets; current fixture acceptance does not prove upstream timestamp behavior.

## References

- `docs/architecture/broadcast-chain.md`
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
- `docs/research/broadcast-recording-and-feed-publishing.md`
- `musicindex-live-publisher`: `docs/README.md`
- `splitkit`: `docs/README.md`
