# Orchestrated Run Of 2026-10-24

## Status

Open - 2026-10-24. This record is advisory. It states no rule. Each rule stays in its ADR or its packet.

## Operator Decisions - 2026-10-24

- The run implements steps 2 and 3 of the [remaining order](../plans/design-and-cleanup-overhaul-plan.md#remaining-order---2026-10-23): the fix packets of the final walk of the [run of 2026-10-23](orchestrated-run-2026-10-23.md), then the [live value URI plan](../plans/livevalue-uri-is-a-url.md).
- The orchestrator commits each reviewed, Green packet as HeyCitizen, and never pushes.
- The visual checks of the run come last, in one final walk.
- The operator answered the questions of the packets on 2026-10-24. Each packet records its answers.

## Order

1. [ADR 0084 packet 035](../tasks/adr-0084-task-035-download-stores-feed-links.md): a download stores the feed links of its release.
2. [ADR 0087 packet 008](../tasks/adr-0087-task-008-field-list-stays-open.md): the field list stays open for the next term.
3. [ADR 0087 packet 007](../tasks/adr-0087-task-007-live-run-index-rows.md): a live run shows the Index rows.
4. [ADR 0084 packet 036](../tasks/adr-0084-task-036-group-row-at-artist-name.md): a group row at its artist name.
5. [ADR 0084 packet 037](../tasks/adr-0084-task-037-releases-everything-frame-time.md): one cached merge and one virtual list.
6. The live value URI packet. The orchestrator writes it from the plan before it starts.

Each packet starts after the commit of the packet before it. Packets 008 and 007 both change the toolbar search, and packets 036 and 037 both change `src/view_models/releases_everything.rs`.

## Orchestrator Decisions

Each entry names the step, the decision and the reason.

- Packet 008 Q2: the orchestrator decided A, by its recommendation, inside the operator answer of Q1.
- Packet 035 Q1: a later packet changes the skip of a `PageView` hydration. The ADR 0084 phase plan records it as an open finding.
- Packet 007 stop of 2026-10-24: three guards that the packet does not name require the old restore path and the old loading call. The orchestrator accepted the five recommendations of the coding agent. The guards read `restore_content_list_page` and `index_pending_row_count`, with the same meaning. The note of decision D6 is a typed view model value with one test for each reason. The packet records the five items in its Implementation Notes.

## Steps

Each entry names the packet, the result and the commit.

1. ADR 0084 packet 035: implemented and Green on 2026-10-24, with no stop. A hydration after a download and the pass at start send a new feed request (`AlbumHydrationTrigger`). 2519 lib tests and 392 guards pass.
2. ADR 0087 packet 008: implemented and Green on 2026-10-24, with no stop. An empty field name prefix shows the 11 fields in each open state, and a row that goes into the text keeps the list open. One state `Open` replaces `Requested` and `Typing`. 2523 lib tests and 392 guards pass.
3. ADR 0087 packet 007: implemented and Green on 2026-10-24, after one stop (see Orchestrator Decisions). The loading rows follow the Library rows, each toolbar run keeps one generation, and `index_result_step` decides each Index result. A page note names the reason of a stopped result. The cause of the missing Index rows is not proved. The orchestrator accepted one change outside the packet list: `Display` for `RunGeneration` in `src/view_models/live_run.rs`, for the run number of the note. 2534 lib tests and 393 guards pass.

## Final Walk

Open. The orchestrator writes the walk when each packet of the run is committed.
