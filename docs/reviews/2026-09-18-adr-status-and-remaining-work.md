# ADR Status And Remaining Work — 2026-09-18

## Status And Scope

Documentation correction complete. This review began against v4vmm commit
`4c9b7e5`. The follow-up corrected the active records in this workspace and the
publisher's show-log documents. It changes no application behavior or operator
acceptance result.

The review used repo-docs-organizer and asd-ste100. It checked current ADR
statuses, task headers, delivery dependencies and gate prose. The
[2026-09-10 reconciliation](2026-09-10-governance-reconciliation.md) supplies
older checklist dispositions. Dated observations retain their historical meaning.
This review does not certify every historical document or deployed service.

## Findings And Corrections

| Finding | Correction and evidence |
|---|---|
| The Settings plan called recovery 007–013 unstarted. | The [plan](../plans/adr-0069-settings-presets-phase-plan.md) and delivery index now record their completion. Phase 002 prerequisites are met. Its packet remains unwritten. |
| Completed packets still advertised unstarted successors. | Current status paragraphs now identify completed successors or refer to the phase plan. Historical observations use past tense or their existing dates. |
| Six completed structural packets said Ready. | ADR 0060 task 001 and ADR 0062 tasks 001–005 now match the [delivery record](../plans/broadcast-chain-delivery-order.md#surface-rewrite--complete). No new visual acceptance was inferred. |
| ADR 0025 claimed seven of eleven phases delivered. | All eleven packets record implementation. The [review](adr-0025-review-checklist.md) records checks through task 010 and visual passes. ADR 0025 is now Implemented. |
| ADR 0055's shipped decomposition still said Accepted. | The module review, existing guard and 97 focused tests support Implemented. Verification below records the limits. |
| ADR 0060 assigned occupied numbers to future decisions. | Cache/dump and play history now have no assigned ADR number. ADR 0060 retains its unfinished audition requirement. |
| ADR 0062 listed delivered controls and view-mode persistence as future work. | Its status separates the five completed packets from broader search/expansion evidence that still needs review. |
| The deferred index still described independent blocking HTTP client construction. | It now names ADR 0058's shared construction owner. Broader document-fetch policy remains conditional. |
| The polish backlog repeated platform shortcuts as unfinished work. | It now credits ADR 0067 and limits future work to additional usable commands. |
| The pending-check index mixed current gates with a long drag-fix history. | All six groups now precede the method section. The owning playlist review retains the detailed history. |
| Service-fixture cleanup removed whole drop-in directories and restored an unrelated config backup. | Cleanup now names only the test file for the method used. Setup forbids overwriting an existing file. No service commands ran. |
| Publisher show-log task 002 used delivery time for episode ranges. | The packet now filters and sorts on `observed_at`, as its amended ADR requires. Future tests must distinguish producer and delivery times. |
| Publisher documents described the unimplemented show log as available. | The ADR, index, boundary document and packets now name the missing implementation and timestamp prerequisite. |

The publisher source check found no producer timestamp in version 1 drop files.
The scheduler holds `queued_at` and `due_at` as monotonic `Instant` values.
Those values do not prove a wall-clock producer time. The writer packet now
requires a source decision before implementation. No replacement source or
schema change was invented in this correction.

## Remaining Evidence And Decisions

- ADR 0069 phase 002 can use the completed recovery owners after its packet
  defines the scope and checks. Phase 003 still requires full ADR 0066 acceptance.
- ADR 0066 task 004 retains that gate. Its playback checks wait for proposed
  ADR 0068 and diagnosis of the mpv IPC error. No scheduling exception is inferred.
- ADR 0062's broader mixed-row search/expansion scope still needs an evidence
  review. The five completed packets remain closed.
- Legacy ADRs 0003 and 0013 still need the index's supersede-or-keep review.
- Publisher writer 001 needs the producer timestamp source and failure-time
  representation specified in its owning contract. Reader 002 follows that work.

## ADR Status Snapshot

There are 74 numbered ADRs: 72 current and two archived.

| Recorded status | Count | Interpretation |
|---|---:|---|
| Implemented | 40 | Recorded completion for the ADR's accepted scope |
| Accepted | 31 | Binding decision, with implementation incomplete or completion not fully verified |
| Proposed | 1 | ADR 0068, Show cue and audition isolation |
| Superseded, archived | 2 | ADRs 0018 and 0019, replaced by ADR 0059 |

These are header counts, not 31 unstarted projects. Several Accepted records
are longstanding contracts. Others retain the evidence gaps or unfinished scope
named above. The [ADR index](../adr/README.md) remains the current decision inventory.

Recent completed scope stays closed:

- ADR 0059 tasks 001–017 and Show action feedback.
- ADR 0063 dashboard and shared-log work, including ADR 0070.
- ADR 0067 platform shortcuts.
- ADRs 0071/0072 shared selection and the pinned correction. IME and Wayland
  remain coverage limits, not newly inferred acceptance gates.
- ADRs 0073/0074 card overflow and repair/diagnostics pages.
- ADR 0039's three type-ramp packets and V1–V13. Preservation and preference
  restoration were inferred from conditional cleanup, as its review records.
- ADR 0066 tasks 001–003 and 005–013: twelve of thirteen packets complete.
- ADR 0069 task 001, grouped Settings. Later Settings phases remain unstarted.

## Open Human Gates

The [pending-human index](../pending-human-checks.md) records six groups covering
eight task packets. The owning prose and later evidence support retaining all
six groups. This review supplies no new operator acceptance.

| Group | Packet owners | Remaining proof |
|---|---|---|
| Scrolling | ADR 0030 task 006 | Overflowing current Music details and Settings, with wheel, scrollbar and supported keyboard behavior in both themes |
| Identity/detail parity | ADR 0037 tasks 001–002 | Populated feed and track identity through local and Index origins, with separate results in both themes |
| Toolbar search | ADR 0043 task 004 | Focus, submission and readability at normal/narrow widths in Light/Dark |
| Playlist reordering | ADR 0044 task 003 | Remaining theme-specific handle, menu, insertion, cancellation and mounted-row behavior. The September 16 drag-pause correction is already accepted. |
| Stored metadata | ADR 0054 tasks 004–005 | Feed/track hydration, local/Index comparison and unavailable-endpoint fallback with known persisted facts |
| Optional-tool isolation | ADR 0066 task 004 | Remaining Index/player, producer, publisher and partial-path-repair observations, report copy, outstanding preservation and cleanup evidence |

Task 004's playback-dependent portions remain paused for Show/audition
separation and the observed mpv IPC error. Its report/path-repair checks and
the inherited Music checks can proceed independently. The presentation pass
and producer preservation pass stay accepted.

Later fixture cleanup reports do not by themselves close task 004's remaining
behavior or case-specific preservation requirements. Reconcile its recorded
fixture ownership during closure rather than assuming an old directory still
exists. Keep the configuration-format gate until its owning records release it.

## Existing Packets To Complete

These retain the [approved cross-repository order](../plans/broadcast-chain-delivery-order.md).

| Work | Packet state | Dependency or stopping point |
|---|---|---|
| Eight packets in the human-gate table | Implementation recorded, acceptance incomplete | Walk surviving criteria and record each result under its owner |
| [ADR 0064 task 002](../tasks/adr-0064-task-002-repair-report-surface.md), repair history | Written, Ready | Scheduled in steady state. Task 001 is delivered. Refresh integration against current Music and Settings owners before execution. |
| [splitkit reserved 001–005](../../../splitkit/docs/plans/adr-0001-reserved-live-items-phase-plan.md) | All five written and Ready in local records | Execute 001 → 002 → 003, deploy and adopt a reserved identity, then 004 → 005 follow-through. Operational durability requires restoration and TTL exemption, not merely the create route. |
| Publisher show-log [001](../../../musicindex-live-publisher/docs/tasks/show-log-task-001-log-writer.md) → [002](../../../musicindex-live-publisher/docs/tasks/show-log-task-002-read-contract-and-docs.md) | Both written, not started | Resolve the writer timestamp source before implementation. A real scheduled show makes writer 001 the next packet, with logging enabled and verified before the show. |

Publisher control-surface tasks 001/002 are already complete. Their CLI facts
do not automatically implement a seventh service state in v4vmm. These local
document checks do not establish what is deployed on any host.

## Packets And Decisions Still To Write

| Work | Needed artifact | Prerequisite |
|---|---|---|
| ADR 0069 phase 002: shared guarded editor | Bounded implementation packet, review and operator procedure | Named recovery prerequisites are now complete. This is the next packet-authoring step in the approved order. |
| ADR 0069 phase 003: live metadata setup | Exact schema/resource/apply contracts and bounded packets | Phase 002 acceptance and full configuration-format gate release |
| ADR 0069 phase 004: selective presets | Versioned persistence/merge/recall design and packets, split model/UI if needed | Phase 003 and guarded persistence |
| ADR 0069 phase 005: independent audio Settings | Concrete adapter packets and audio verification | ADR 0068 acceptance and verified independent owners. PulseAudio/JACK first, native PipeWire later. |
| ADR 0068: Show cue and audition isolation | Accept or revise the proposal, then packets for cue persistence/commands, audition/output ownership and UI/verification | Explicit scheduling reconciliation with task 004. Diagnose the mpv IPC error and add regression proof before audio acceptance. |
| v4vmm reserved-event adoption | Reservation, credential storage, selection and publisher-configuration packet | Relay 002/003 delivered and deployed. Command-line reservation is the interim path. |
| Remaining narrow Show layout/A10 | Owning ADR amendment or decision, then a bounded packet and combined visual gate | Preserve completed 0070/0073 behavior. Remaining scope is compact cards, full-width log docking, title readability and hiding wholly unavailable transport. |
| Consistent UTC logs | Common timestamp contract in owning ADRs, emitter/adapter audit, per-repository packets | Actual source times, explicit UTC and common precision. Shared app frames/following are already complete. |
| Installed-but-unconfigured publisher | v4vmm service-state/VM/UI packet | Consume the already delivered publisher CLI facts |
| Workspace configuration consolidation | Dedicated migration ADR and packet | Full ADR 0066 gate release |
| Cache/dump policy, play history, episode generation | Separate future ADRs, then scoped packets | No reserved ADR numbers. Episode generation consumes the publisher show log. Post-processing precedes publisher-side backup recording. |

## Deferred And Conditional Work

These are follow-ups, not additional acceptance gates on completed packets:

- [Deferred index](../plans/deferred-architecture-work-index.md): person/global
  identity, staged metadata durability, non-URL artwork, playback volume and
  driver supervision, and bounded visual-system improvements.
- [ADR 0053](../adr/0053-local-detail-source-fact-parity.md): remaining source-fact
  product decisions, including artist biography, track language/annotation,
  playlist metadata, medium/kind mapping and date policy. New required
  persistence needs its own concrete schema decision.
- [Product polish](../plans/hig-product-polish-backlog.md): recent/suggested
  searches, sidebar disclosure/customization and material roles. Its keyboard
  item now credits completed ADR 0067. Future commands need their own checks.
- ADR 0039 live layout enforcement is a separate design question. Its current
  debug assertions and source guards do not control rendered dimensions.
- Visible multi-frame commands and detach/dock need fresh scope if selected.
  [ADR 0046 task 013](../tasks/adr-0046-task-013-multi-frame-commands-ux.md)
  records an accepted command deferral, not an unwalked visual gate.
- Show view-model decomposition and wider all-target Clippy cleanup remain
  unscheduled. Conditional enclosure diagnostics, additional image formats and
  document-fetch consolidation retain their triggers in the deferred index.
- Cross-repository later work includes broadcaster identity/quotas, publisher
  remote control and Liquidsoap, recording post-processing and backup capture.
- ADR 0072 upstream correction/replacement and IME/Wayland coverage remain
  follow-up opportunities without reopening the accepted X11 packet.

## Verification And Disposition

The ADR 0055 review checked private module declarations, root re-exports and
consumer imports. Commit `4be09d8` records the original split. The existing
situational guard checks the retired path, module wiring, renderer boundary
and prohibition on deep consumer imports. No source files changed.

Green in this session:

- `cargo test --locked --offline --lib view_models::search::tests --quiet`: 97 tests.
- `cargo test --locked --offline --test architecture_tests adr_0055 --quiet`: one guard.
- `cargo test --locked --offline --test architecture_tests adr_0057 --quiet`: two status tests.
- `cargo build --locked --offline --bin v4vmm --quiet`, after the tests.
- `cargo check --locked --offline --quiet`.
- `cargo fmt -- --check`.
- `cargo clippy --locked --offline --quiet -- -D warnings`.
- 636 local links and incoming anchors across 35 documents.
- Ten shell blocks passed syntax checks. Both repositories passed whitespace checks.
- ADR/index agreement, six pending groups and the six corrected packet headers.

The focused tests establish current projection and boundary behavior. They do
not prove every application behavior or create a new visual pass.
The normal desktop build completed after the tests. No app or service command ran.
The full runtime test suite was not repeated for this documentation change.

The next scheduled feature packet to author remains ADR 0069 phase 002.
The real-show trigger still promotes publisher writer 001. Its timestamp
prerequisite must be resolved before implementation. No open human gate closes
through this documentation change.

## Status Maintenance Check

Use this manual check when a packet changes status. It guards against stale
successor claims and unsupported completion under ADRs 0057 and 0061.

1. Read the owning packet's status and evidence.
2. Compare its ADR, review, phase plan, delivery row and deferred entry.
3. Check gate prose against later acceptance and superseding decisions.
4. Preserve any surviving criterion in the pending-human index.
5. Remove obsolete current claims. Keep dated evidence clearly historical.
6. Check changed links, shell syntax and ADR header tests.
7. Require evidence before using Implemented. File existence alone is insufficient.

For service-fixture instructions, check that setup refuses an existing test file.
Check that cleanup removes only the named file created by that method.
Do not execute the service commands during a documentation review.

## Changed Documents

This conversation created this review and updated the 34 existing documents below.
No files moved. No repository folders were created. Canonical `AGENTS.md` and
`README.md` files remain at each repository root. The link check found no broken
links in the changed files or incoming links to their anchors.

| Repository | Updated document |
|---|---|
| v4vmm | [AGENTS.md](../../AGENTS.md) |
| v4vmm | [docs/README.md](../README.md) |
| v4vmm | [docs/adr/0025-theme-icon-style-boundary.md](../adr/0025-theme-icon-style-boundary.md) |
| v4vmm | [docs/adr/0055-search-view-model-module-decomposition.md](../adr/0055-search-view-model-module-decomposition.md) |
| v4vmm | [docs/adr/0060-workflow-surface-structure.md](../adr/0060-workflow-surface-structure.md) |
| v4vmm | [docs/adr/0062-music-content-surface.md](../adr/0062-music-content-surface.md) |
| v4vmm | [docs/adr/0068-show-cue-and-audition-isolation.md](../adr/0068-show-cue-and-audition-isolation.md) |
| v4vmm | [docs/adr/README.md](../adr/README.md) |
| v4vmm | [docs/pending-human-checks.md](../pending-human-checks.md) |
| v4vmm | [docs/plans/adr-0025-visual-system-phase-plan.md](../plans/adr-0025-visual-system-phase-plan.md) |
| v4vmm | [docs/plans/adr-0069-settings-presets-phase-plan.md](../plans/adr-0069-settings-presets-phase-plan.md) |
| v4vmm | [docs/plans/broadcast-chain-delivery-order.md](../plans/broadcast-chain-delivery-order.md) |
| v4vmm | [docs/plans/deferred-architecture-work-index.md](../plans/deferred-architecture-work-index.md) |
| v4vmm | [docs/plans/hig-product-polish-backlog.md](../plans/hig-product-polish-backlog.md) |
| v4vmm | [docs/reviews/adr-0025-review-checklist.md](adr-0025-review-checklist.md) |
| v4vmm | [docs/tasks/adr-0060-task-001-remove-broadcast-frame.md](../tasks/adr-0060-task-001-remove-broadcast-frame.md) |
| v4vmm | [docs/tasks/adr-0062-task-001-mixed-entity-row-contract.md](../tasks/adr-0062-task-001-mixed-entity-row-contract.md) |
| v4vmm | [docs/tasks/adr-0062-task-002-music-default-view.md](../tasks/adr-0062-task-002-music-default-view.md) |
| v4vmm | [docs/tasks/adr-0062-task-003-library-tri-state-control.md](../tasks/adr-0062-task-003-library-tri-state-control.md) |
| v4vmm | [docs/tasks/adr-0062-task-004-tile-and-list-modes.md](../tasks/adr-0062-task-004-tile-and-list-modes.md) |
| v4vmm | [docs/tasks/adr-0062-task-005-retire-recent-feeds-destination.md](../tasks/adr-0062-task-005-retire-recent-feeds-destination.md) |
| v4vmm | [docs/tasks/adr-0063-task-005-shared-log-frames-and-following.md](../tasks/adr-0063-task-005-shared-log-frames-and-following.md) |
| v4vmm | [docs/tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md) |
| v4vmm | [docs/tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md) |
| v4vmm | [docs/tasks/adr-0066-task-004-optional-tool-isolation.md](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| v4vmm | [docs/tasks/adr-0066-task-006-configuration-repair-and-resumption.md](../tasks/adr-0066-task-006-configuration-repair-and-resumption.md) |
| v4vmm | [docs/tasks/adr-0066-task-008-converter-verification-and-setup.md](../tasks/adr-0066-task-008-converter-verification-and-setup.md) |
| v4vmm | [docs/tasks/adr-0066-task-009-conversion-retry-and-retained-input.md](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md) |
| v4vmm | [docs/tasks/adr-0067-task-001-platform-shortcuts.md](../tasks/adr-0067-task-001-platform-shortcuts.md) |
| musicindex-live-publisher | [docs/README.md](../../../musicindex-live-publisher/docs/README.md) |
| musicindex-live-publisher | [docs/adr/0003-show-log-contract.md](../../../musicindex-live-publisher/docs/adr/0003-show-log-contract.md) |
| musicindex-live-publisher | [docs/architecture/broadcast-chain-boundaries.md](../../../musicindex-live-publisher/docs/architecture/broadcast-chain-boundaries.md) |
| musicindex-live-publisher | [docs/tasks/show-log-task-001-log-writer.md](../../../musicindex-live-publisher/docs/tasks/show-log-task-001-log-writer.md) |
| musicindex-live-publisher | [docs/tasks/show-log-task-002-read-contract-and-docs.md](../../../musicindex-live-publisher/docs/tasks/show-log-task-002-read-contract-and-docs.md) |

## Operator Visual Check

This review changes no interface and creates no fixture. No new visual check
is due. Existing checks stay open in their packet status, delivery row and
pending-human index.

When an inherited check is selected, follow the numbered preparation,
inspection and cleanup commands in the
[inherited UI runbook](../runbooks/inherited-ui-checks.md). For task 004, use
the [optional-tool procedure](../runbooks/startup-recovery-check.md#task-004-optional-tool-isolation)
and retain the playback pause. Those procedures specify the required desktop,
data, tool/service state, failure observations and cleanup. Do not infer passes
from the newer Settings or type-ramp acceptance.
