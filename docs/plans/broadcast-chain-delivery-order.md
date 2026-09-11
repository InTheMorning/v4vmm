# Broadcast Chain Delivery Order

## Status

Active index - 2026-09-10. Operator-approved order; one implementation packet
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

## Current Delivery Order

Approved by the operator on 2026-09-10. Complete one phase or packet per
session. Restored human checks remain visible in the Progress table and
[pending human checks](../pending-human-checks.md); listing them does not
require walking them before the independent chain work.

| Order | Work | Completion point |
|---|---|---|
| 1 | Governance reconciliation | Sweep ADR statuses and review gate prose; retire replaced requirements before indexing survivors; correct AGENTS.md; retain ADR 0039 as Proposed and unscheduled |
| 2, keyboard correction | [Platform shortcuts — ADR 0067](../tasks/adr-0067-task-001-platform-shortcuts.md) | Complete - 2026-09-11; Ctrl shortcuts, focus handling and Settings responsiveness accepted; fixture cleanup confirmed |
| 2 | [Configuration and startup failure recovery — ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md) | Accepted - 2026-09-10; tasks 001–003 complete; [task 004 ready](../tasks/adr-0066-task-004-optional-tool-isolation.md), tasks 004–013 not started in the [phase plan](adr-0066-startup-recovery-phase-plan.md); finish implementation and acceptance before config format changes |
| 3 | Relay durability through adoption | splitkit reserved 001 → 002 → 003; deploy, reserve an event, configure the publisher to use it, then implement the v4vmm reservation packet |
| 3, follow-through | splitkit reserved 004 → 005 | List/delete, final guards, and delivery reconciliation; explicitly scheduled after adoption, with interim command-line reservation allowing these before the v4vmm packet if needed |
| 4 | Narrow Show layout and A10 | ADR 0063 amendment and packet: compact cards, full-width log docking, card-title readability, and hiding transport only when every typed action is unavailable; one combined visual gate |
| 5 | A11, then A12 and UTC | Decide long-line wrapping/horizontal navigation first; design per-log following and the timestamp contract together, then deliver separate bounded packets |
| 6 | Steady state | v4vmm ADR 0064 task 002; publisher show-log 001 → 002; installed-but-unconfigured publisher state |

**Priority trigger:** when a real show is scheduled, publisher show-log task
001 becomes the next packet ahead of this order and must be operational before
that show if its timeline is to be captured. Verify that logging is enabled
and writing entries. Timeline data that was never recorded cannot be recovered.
This trigger does not combine two implementation phases into one session.

### Dependencies And Adoption

- Configuration failure behavior precedes deferred item 7's workspace-config
  format migration and any other config format change.
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
- A11's display treatment informs A12's reading anchors. A12 must define fresh
  entry delivery, source identity, and replaced/trimmed-anchor behavior.
  Its app work does not wait for timestamp corrections in other repositories.
- The publisher's machine-readable configuration facts already exist. A new
  v4vmm packet must consume them to distinguish installed-but-unconfigured
  from the current service states.

Show view-model decomposition and wider all-target Clippy cleanup remain
unscheduled. Cache/dump policy, audition, and play history remain separate
future decisions, outside this execution order.

## Progress

Update this table when a packet lands.

| Repository | Packet | State |
|---|---|---|
| `v4vmm` | [governance reconciliation](../reviews/2026-09-10-governance-reconciliation.md) | complete - 2026-09-10; documentation only; surviving gates indexed below |
| `v4vmm` | [0066 configuration/startup failure recovery](adr-0066-startup-recovery-phase-plan.md) | ADR Accepted; tasks 001–003 complete, including [003 operator acceptance, preservation and fixture cleanup](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md#operator-evidence--2026-09-11) - 2026-09-11; mechanical gate Green; task 004 ready; 004–013 not started |
| `v4vmm` | [0067 platform shortcuts 001](../tasks/adr-0067-task-001-platform-shortcuts.md) | Complete - 2026-09-11; mechanical gate Green; all shortcut checks, preservation and fixture cleanup accepted; ADR Implemented |
| `v4vmm` | [0030 006 scroll containers](../tasks/adr-0030-task-006-scroll-containers.md) | implementation recorded; current Music/Settings visual check open |
| `v4vmm` | [0037 001 feed identity](../tasks/adr-0037-task-001-feed-identity-action-parity.md) | implementation recorded; local/Index identity visual check open |
| `v4vmm` | [0037 002 track detail parity](../tasks/adr-0037-task-002-track-header-action-parity.md) | implementation recorded; local/Index track visual check open |
| `v4vmm` | [0043 004 toolbar readiness](../tasks/adr-0043-task-004-guards-and-visual-readiness.md) | implementation recorded; surviving normal/narrow Light/Dark check open |
| `v4vmm` | [0044 003 playlist reorder](../tasks/adr-0044-task-003-playlist-reorder-guards-visual.md) | implementation recorded; current handle/menu/insertion visual check open |
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
2. Write the audition decision, and the play-history decision.

These carry no number until somebody writes them. A number reserved in prose
collided once already, when ADR 0063 became the `Show` dashboard layout.

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

Scheduled for design with A12 after A11 - 2026-09-10. The operator requires
consistent UTC timestamps across all logs, including producers outside `v4vmm` where
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
