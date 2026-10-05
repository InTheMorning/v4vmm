# Request: Keep A Reserved Live Event Safe

## Status

Open - 2026-10-04. `musicindex-live-publisher` makes this request to v4vmm.

This document binds nothing in v4vmm. v4vmm records each decision in its own
ADR.

## Context

Relay ADR 0001 adds reserved live events to the relay. A reserved event
survives a relay restart and the idle TTL. Only the relay operator makes one,
with the admin token. The relay gives its broadcaster token one time.

On 2026-10-04 the operator reserved an event for the Mixxx target and put it
in the publisher configuration by hand. The reserved event is not in the
v4vmm registry. The registry still holds the earlier ephemeral event of that
target.

A review on 2026-10-04 read the v4vmm source. Nothing in v4vmm changes the
setup by itself: not at startup, not on a timer, and not on a `404`. But one
click or one command can change it. The publisher side has its own plan:
`docs/plans/reserved-event-safety.md` in `musicindex-live-publisher`.

## Findings

The references are to the v4vmm source on 2026-10-04.

| # | Path | Trigger | Effect |
|---|---|---|---|
| 1 | `src/cli.rs:277-288`, `src/broadcast/registry.rs:70-85` | `v4vmm broadcast events forget` | It deletes the token file of the registry row. If a publisher target uses that file, the publisher loses its token. |
| 2 | `src/app/show.rs:1597-1604`, `src/broadcast/publisher_targets.rs:183-198` | Attach, and Retry in the recovery panel | It runs `target add --replace` for the target, then restarts the publisher. Attach is enabled when the target does not carry the selected event. Nothing shows that the target carries a different event. |
| 3 | `src/cli.rs:307-325` | `v4vmm broadcast targets attach` | The same as row 2, with no check that the event is live. |
| 4 | `src/api.rs:1010-1011`, `src/broadcast/registry.rs:190-207` | Each Check | Each `404` from the metadata route marks the event Dead. That enables Replace, and Replace makes Attach possible. |

About row 4: the relay gives `404 metadata_not_found` for an event that exists
and has no snapshot. A reserved event has no snapshot after a relay restart
until its next publish. Only `404 event_not_found` means that the event does
not exist (relay `docs/runbooks/reserved-live-items.md`).

About row 2: `target add --replace` also removes `display_dir` and the stream
delay from the target. Publisher task 002 of the reserved event safety plan
stops this.

ADR 0059 says that v4vmm runs `provision` and `setup-mixxx-musicindex`. The
source calls neither.

## The Requests

1. **Attach does not replace a different event without a question.** When
   the target carries a different event, Attach shows both event IDs and
   asks before it changes the target. The CLI attach command needs an
   explicit option for the same case.
2. **Only `event_not_found` means Dead.** A `404 metadata_not_found` means
   that the event exists and has no snapshot.
3. **Forget does not delete a token file that a target uses.** Before it
   deletes a token file, forget reads `target list --json`. If a target names
   the file, forget keeps the file and says why.
4. **A reserved event can enter the registry.** The operator gives an event
   ID and the path of its token file. v4vmm checks the event at the relay and
   stores the row as reserved. v4vmm offers no Replace for a reserved event,
   and forget never deletes its token file. The Later Work entry for a
   reserve packet in `docs/plans/broadcast-chain-delivery-order.md` can
   cover this.

## What The Publisher Gives

After publisher task 002 of the reserved event safety plan:

- `target add --replace` changes only `event_id`, `token_file`, and
  `stream_delay_secs` when the flag is given. It keeps each other line of the
  stanza.
- `config show --json` and `target list --json` give `display_dir` for each
  target.

After publisher task 001, `provision` refuses a token file that exists.

## Interim Rules For The Operator

These rules are advisory. They stay until the requests are decided.

- Do not use Attach, Retry or Replace for the Mixxx target.
- Do not run `broadcast events forget` or `broadcast targets attach` for the
  Mixxx target.

## References

- `musicindex-live-relay`: `docs/adr/0001-reserved-live-items.md`,
  `docs/runbooks/reserved-live-items.md`
- `musicindex-live-publisher`: `docs/adr/0004-publisher-control-cli.md`,
  `docs/plans/reserved-event-safety.md`
- v4vmm: ADR 0059, `docs/plans/broadcast-chain-delivery-order.md`
