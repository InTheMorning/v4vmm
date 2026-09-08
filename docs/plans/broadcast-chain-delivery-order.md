# Broadcast Chain Delivery Order

## Status

Active index - 2026-09-07.

## Purpose

Three repositories hold task packets for one system. Each repository has its own
plan, and no plan can see the others. This document is the only place that
states the order across all three.

Read this before you start a session. Update the Progress table when a packet
lands.

## The Repositories

| Repository | Governing ADR | Packets |
|---|---|---|
| `v4vmm` | ADR 0059, broadcast control surface | 15 |
| `musicindex-live-publisher` | ADR 0003 show log, plus control surface support | 4 |
| `splitkit` | ADR 0001, reserved live items | 5 |

## Hard Dependencies

Two of the three are now satisfied. The third is soft and open.

| Blocked packet | Waits for | Reason |
|---|---|---|
| ~~`v4vmm` 014, attach event to target~~ | ~~`musicindex-live-publisher` control surface 001~~ | Satisfied 2026-09-07 (`a5b434e`). `target add`, `target list`, and `target remove` all shipped |
| ~~`v4vmm` 014~~ | ~~`v4vmm` 010, remote hosts~~ | Satisfied 2026-09-08. Packet 010 shipped the transport |
| `v4vmm` 009, publisher section | `musicindex-live-publisher` control surface 002 | Soft, and still open. Control surface 002 shipped 2026-09-07 (`459854c`), so `--version` and `config show --json` exist. Packet 009 shipped without consuming them, so the section still reports one state less. See Later Work |

`splitkit` blocks nothing and is blocked by nothing.

## Recommended Order

One packet for each session, as the `v4vmm` AGENTS.md mandate requires.

### Stage 1: Unblock the chain

1. `musicindex-live-publisher` control surface 001, target management.
2. `musicindex-live-publisher` control surface 002, machine-readable CLI.

Both are small and both remove a block. Do them first so no `v4vmm` session
stalls later.

### Stage 2: The v4vmm backend

3. `v4vmm` 001, live surface reduction.
4. `v4vmm` 002, event registry schema.
5. `v4vmm` 003, event registry service and CLI.
6. `v4vmm` 004, relay observation actor.

No interface work. At the end of this stage the app can create an event, store
its token, test whether it is alive, and read what the relay serves.

### Stage 3: The Broadcast frame

7. `v4vmm` 005, broadcast page view model.
8. `v4vmm` 006, broadcast workspace frame kind.
9. `v4vmm` 007, broadcast shell and frame adapter.

Packets 005 and 006 do not depend on each other.

### Stage 4: Service control

10. `v4vmm` 008, publisher service control.
11. `v4vmm` 009, publisher section wiring and log panel.
12. `v4vmm` 010, remote hosts over SSH.
13. `v4vmm` 014, attach event to publisher target.

Packet 014 closes the loop. After it, an operator can create an event, attach
it to the publisher, and start the services from one screen.

### Stage 5: Sources and reporting

14. `v4vmm` 011, mpv drop-file producer.
15. `v4vmm` 012, library broadcast readiness report.
16. `v4vmm` 015, stream encoder section.

### Stage 6: Close ADR 0059

17. `v4vmm` 013, final guards and readiness gate.

### Independent track: bank the shows

- `musicindex-live-publisher` show log 001, log writer.
- `musicindex-live-publisher` show log 002, read contract and documentation.

These depend on nothing and block nothing. They need only the drop-file watcher
that already exists.

**Do them early if shows start before the control surface is finished.** A show
that runs without the log can never become an episode. Data that is not
captured cannot be recovered.

### Independent track: relay durability

- `splitkit` reserved live items 001 through 005.

Also independent. Without it, every event dies when the relay restarts or after
24 hours of no activity, and listeners must tune again.

Its urgency is a product question, not a technical one: it matters as soon as a
show repeats or a station runs continuously.

## An Alternative Order

The order above builds the interface before the loop closes. A different
priority reaches a working chain sooner.

Packet 014 depends on `v4vmm` 010 because the attach runs through the transport.
A local-only attach needs no transport. Splitting 014 into a local step and a
remote step makes this order possible:

1. `musicindex-live-publisher` control surface 001.
2. `v4vmm` 001, 002, 003.
3. `v4vmm` 014, local half only.

That reaches a complete chain at the command line, with no interface, in four
sessions. Choose it if proving the chain matters more than showing it.

## Progress

Update this table when a packet lands.

| Repository | Packet | State |
|---|---|---|
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
| `v4vmm` | 010 remote hosts | implemented - 2026-09-08; visual proof blocked by GPUI X11 initialization in the agent session |
| `v4vmm` | 014 attach event | implemented - 2026-09-08; visual proof blocked by GPUI X11 initialization in the agent session |
| `v4vmm` | 011 mpv producer | implemented - 2026-09-08 |
| `v4vmm` | 012 readiness report | implemented - 2026-09-08; one visual gate open: readiness count legibility and the action that reaches the filtered list |
| `v4vmm` | 015 stream encoder | implemented - 2026-09-08; visual proof blocked by GPUI X11 initialization in the agent session |
| `v4vmm` | 013 final guards | ready |
| `v4vmm` | 0063 001 card contract | implemented - 2026-09-08; no visual criteria |
| `v4vmm` | 0063 002 card grid shell | mechanical implemented - 2026-09-08; operator visual check open |
| `v4vmm` | 0063 003 detail panel | ready |
| `splitkit` | reserved 001 store boundary | not started |
| `splitkit` | reserved 002 reserved class | not started |
| `splitkit` | reserved 003 restore and TTL | not started |
| `splitkit` | reserved 004 list and delete | not started |
| `splitkit` | reserved 005 guards and review | not started |

## Current Blocker

ADR 0060 replaced the surface design on 2026-09-07. Every UI packet of ADR 0059
is superseded or blocked until it is reconciled with that ADR.

The restructure runs first. It needs ADR 0060 only. Cache, audition, and play
history are features inside `Music`, not the structure of it, so their ADRs do
not gate the structural work.

| Step | Packet | State |
|---|---|---|
| 1 | `docs/tasks/adr-0060-task-001-remove-broadcast-frame.md` | complete - 2026-09-07 |
| 2 | `docs/tasks/adr-0060-task-002-show-screen-mount.md` | implemented - 2026-09-08; visual acceptance met |
| 3 | `docs/tasks/adr-0060-task-003-music-surface.md` | implemented - 2026-09-08; visual acceptance met |
| 4 | `docs/tasks/adr-0060-task-004-live-status-strip.md` | implemented - 2026-09-08; visual acceptance met |

`Show` comes before `Music`. The queue moves into `Show`, so `Show` must exist
before the curation surface gives it up. The reverse order leaves playback with
no home.

ADR 0062 then rebuilds what `Music` shows:

| Step | Packet | State |
|---|---|---|
| 1 | `docs/tasks/adr-0062-task-001-mixed-entity-row-contract.md` | complete - 2026-09-07 |
| 2 | `docs/tasks/adr-0062-task-002-music-default-view.md` | complete - 2026-09-07 |
| 3 | `docs/tasks/adr-0062-task-003-library-tri-state-control.md` | complete - 2026-09-07 |
| 4 | `docs/tasks/adr-0062-task-004-tile-and-list-modes.md` | complete - 2026-09-07 |
| 5 | `docs/tasks/adr-0062-task-005-retire-recent-feeds-destination.md` | complete - 2026-09-07 |

The ADR 0059 packets were revised against ADR 0060 on 2026-09-08 and are no
longer blocked. Packet 009 establishes how a section composes into the `Show`
screen mount, so it runs before 012, 014, and 015.

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

ADR 0063 rebuilds the `Show` layout. It does not block the packets above, and
they do not block it. Packet 013 checks the layout that ADR 0063 leaves.

| Order | Packet | Depends on |
|---|---|---|
| 1 | `docs/tasks/adr-0063-task-001-card-contract-and-width-class.md` | nothing |
| 2 | `docs/tasks/adr-0063-task-002-card-grid-shell.md` | 001 |
| 3 | `docs/tasks/adr-0063-task-003-collapsible-detail-panel.md` | 002 |

After that:

1. Write ADR 0063, cache and dump policy. `Dump` needs it.
2. Write ADR 0064, audition, and ADR 0065, play history.

`v4vmm` packets 001 through 004 shipped and are unaffected. They are backend
work that ADR 0060 does not touch.

## Later Work

These have no packets and are not scheduled. They are listed so the order above
is not mistaken for the whole plan.

- A seventh `ServiceState` in `v4vmm`, for a publisher that is installed and
  not configured. `musicindex-live-publisher` control-surface task 002 supplies
  the two facts that separate it, through `--version` and `config show --json`.
  Landing that packet does not change `v4vmm` on its own. Needs a `v4vmm` packet.
- Episode generation in `v4vmm`, from the show log. Needs a future ADR.
- Post-processing tools in `v4vmm`. Gates publisher-side backup recording.
- Broadcaster identity and quotas in `splitkit`. Options recorded, no decision.
- A remote control API in `musicindex-live-publisher`, for liquidsoap.
- Liquidsoap as a source.

## References

- `docs/architecture/broadcast-chain.md`
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
- `docs/research/broadcast-recording-and-feed-publishing.md`
- `musicindex-live-publisher`: `docs/README.md`
- `splitkit`: `docs/README.md`
