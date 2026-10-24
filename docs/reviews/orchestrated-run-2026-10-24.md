# Orchestrated Run Of 2026-10-24

## Status

Open - 2026-10-24. This record is advisory. It states no rule. Each rule stays in its ADR or its packet.

## Operator Decisions - 2026-10-24

- The run implements steps 2 and 3 of the [remaining order](../plans/design-and-cleanup-overhaul-plan.md#remaining-order---2026-10-23): the fix packets of the final walk of the [run of 2026-10-23](orchestrated-run-2026-10-23.md), then the [live value URI plan](../plans/archive/livevalue-uri-is-a-url.md).
- The orchestrator commits each reviewed, Green packet as HeyCitizen, and never pushes.
- The visual checks of the run come last, in one final walk.
- The operator answered the questions of the packets on 2026-10-24. Each packet records its answers.

## Order

1. [ADR 0084 packet 035](../tasks/adr-0084-task-035-download-stores-feed-links.md): a download stores the feed links of its release.
2. [ADR 0087 packet 008](../tasks/adr-0087-task-008-field-list-stays-open.md): the field list stays open for the next term.
3. [ADR 0087 packet 007](../tasks/adr-0087-task-007-live-run-index-rows.md): a live run shows the Index rows.
4. [ADR 0084 packet 036](../tasks/adr-0084-task-036-group-row-at-artist-name.md): a group row at its artist name.
5. [ADR 0084 packet 037](../tasks/adr-0084-task-037-releases-everything-frame-time.md): one cached merge and one virtual list.
6. [ADR 0059 packet 019](../tasks/adr-0059-task-019-livevalue-uri-is-a-url.md): the liveValue URI is a URL, from the [live value URI plan](../plans/archive/livevalue-uri-is-a-url.md).

Each packet starts after the commit of the packet before it. Packets 008 and 007 both change the toolbar search, and packets 036 and 037 both change `src/view_models/releases_everything.rs`.

## Orchestrator Decisions

Each entry names the step, the decision and the reason.

- Packet 008 Q2: the orchestrator decided A, by its recommendation, inside the operator answer of Q1.
- Packet 035 Q1: a later packet changes the skip of a `PageView` hydration. The ADR 0084 phase plan records it as an open finding.
- Packet 019 Q1: option A, by its recommendation. An event with no usable relay host gives no tag, and the copy action says why. D2 confirmed: the URI drops the path of the endpoint.
- Packet 019 stop of 2026-10-24: the ADR 0059 broadcast guard forbids each `reqwest::` text under `src/broadcast/`, and the URI builder parses with `reqwest::Url`. The orchestrator chose to narrow the guard: `reqwest::Url` is allowed, and each other `reqwest::` text still fails. The guard stops the build of relay clients, and a URL parser builds no client.
- Packet 007 stop of 2026-10-24: three guards that the packet does not name require the old restore path and the old loading call. The orchestrator accepted the five recommendations of the coding agent. The guards read `restore_content_list_page` and `index_pending_row_count`, with the same meaning. The note of decision D6 is a typed view model value with one test for each reason. The packet records the five items in its Implementation Notes.

## Steps

Each entry names the packet, the result and the commit.

1. ADR 0084 packet 035: implemented and Green on 2026-10-24, with no stop. A hydration after a download and the pass at start send a new feed request (`AlbumHydrationTrigger`). 2519 lib tests and 392 guards pass.
2. ADR 0087 packet 008: implemented and Green on 2026-10-24, with no stop. An empty field name prefix shows the 11 fields in each open state, and a row that goes into the text keeps the list open. One state `Open` replaces `Requested` and `Typing`. 2523 lib tests and 392 guards pass.
3. ADR 0087 packet 007: implemented and Green on 2026-10-24, after one stop (see Orchestrator Decisions). The loading rows follow the Library rows, each toolbar run keeps one generation, and `index_result_step` decides each Index result. A page note names the reason of a stopped result.

   The cause of the missing Index rows is not proved. The orchestrator accepted one change outside the packet list: `Display` for `RunGeneration` in `src/view_models/live_run.rs`, for the run number of the note. 2534 lib tests and 393 guards pass.
4. ADR 0084 packet 036: implemented and Green on 2026-10-24, with no stop. A group row takes the place of its label in both scopes. A group that a later page finds after its label shows at its label at once, the one exception to packet 016 D7. 2540 lib tests and 393 guards pass.
5. ADR 0084 packet 037: implemented and Green on 2026-10-24, with no stop. The Releases list builds its merge one time for each change of its key, and the Releases and Artists lists are gpui virtual lists. A change splices the list, so the top row stays in view. The Releases rows are now narrower by the scrollbar gutter.

   The coding agent gave each spliced item the minimum row height, so that the scrollbar counts the full list (packet Implementation Notes). 2551 lib tests and 394 guards pass. The release binary is built.
6. ADR 0059 packet 019: implemented and Green on 2026-10-24, after one stop (see Orchestrator Decisions). The feed tag is `uri="<scheme>://<host>[:port]/event?event_id=<encoded id>"`, from the endpoint of the event row. An endpoint with no usable host gives no tag, and the copy action says why. The plan moved to `docs/plans/archive/`. 2559 lib tests and 396 guards pass.

## Final Walk

Open - 2026-10-24. Each packet of the run is committed. Section 7 of [pending human checks](../pending-human-checks.md) lists each open check. The "Operator Visual Check" section of each packet gives the steps.

### Setup

The run adds no migration and no configuration key, so no backup is necessary.

1. Open a terminal in a desktop session.
2. Build both binaries: `cd ~/build/v4vmm && cargo build --release && cargo build --bin v4vmm`.
3. Confirm that the debug binary is a normal build: `nm -C target/debug/v4vmm | grep -c TestDispatcher` gives `0`.
4. Start the app with `./target/release/v4vmm` for packet 037, and with either binary for the other packets.

The checks need these states:

- `https://api.musicindex.org` is reachable, except where a step says that it is not.
- The Haleen singles group in Releases "Everything" has MusicIndex singles that are not in the Library.
- For packet 019, the relay fixture of ADR 0059 task 016, as the packet says.

### Order

1. Packet 037 V1 to V7 in the release binary. The speed comes first, because each other check of Releases uses the list.
2. Packet 036 V1 to V3. V1 walks packet 026 V1 again.
3. Packet 035 V1 to V3. V1 walks packet 026 V6 again. Its steps download singles, and its cleanup removes them.
4. Packet 008 V1 to V5. The script scenario `fieldlist2` of `~/.claude/scripts/v4vmm-shots.sh` takes the screenshots. Its click on `single` is at y 161 to 184.
5. Packet 007 V1 to V4. The script scenario `live2` takes the screenshots of V1 to V3. V4 needs MusicIndex not reachable. If V1 fails, write down the text of the note under the rows.
6. Packet 019 V1 to V4. V4 changes a podcast feed outside the app.

### Record

For each check, the operator says pass, fail or skip. The orchestrator records each result in the packet `Status:` line, the phase plan, section 7 of pending human checks and the Walk Log below.

### Walk Log

No check is walked yet.

### Cleanup

Each packet gives its own cleanup. Packet 035 removes the downloaded singles, if the operator does not keep them. Packet 019 restores the relay fixture.
