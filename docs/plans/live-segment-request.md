# Request: Start A Talk Break Or A Live Set Through The Publisher

## Status

Open - 2026-10-18. `musicindex-live-publisher` makes this request to v4vmm.

This document binds nothing in v4vmm. v4vmm records each decision in its own
ADR.

## Context

The publisher records three decisions:

| Publisher ADR | Status | Content |
|---|---|---|
| 0014 | Accepted 2026-10-18 | The publisher is the only program that assembles and publishes a live value block or a display state. Each producer reports each track that plays, payable or not, through drop file version 3. |
| 0015 | Proposed | v4vmm starts and ends a live segment, a talk break or a live set, through `segment` commands over `ssh`. The split of a segment is set for each show. |
| 0016 | Proposed | One publisher instance for each show and for each event, and the rules of the producer lock. |

Publisher ADR 0015 can be accepted only after v4vmm records its side in an
ADR. That v4vmm ADR is the subject of this request.

Today, when Mixxx stops, the publisher sends the dead block (publisher
ADR 0005). It pays nobody. A boost during talk or during a live act pays
nobody.

The evidence is a recording of a thesplitkit.com show on 2026-10-17:
`~/build/musicindex-research/reports/2026-10-17-pixel-palace-curiohoster-show.md`.
During the live sets, one block paid the show's own recipients. No field told
an app which band played.

## What v4vmm Has Today

The references are to the v4vmm source on 2026-10-18.

- v4vmm makes a live event and writes its broadcaster token to a file in its
  token directory (`src/broadcast/registry.rs:129-131`).
- v4vmm gives the publisher the path of that file, never its content
  (`src/broadcast/publisher_targets.rs:184-195`). The publisher and v4vmm
  share one token file.
- v4vmm controls a remote host through `ssh` (ADR 0059 §Remote Control Uses
  SSH First).
- The relay `AGENTS.md` says that v4vmm "sends no payloads".

## The Requests

1. **The split of a segment for each show.** Add a setting for each show.
   It selects the dead block or the show split. The dead block is the
   default. Record where the routes of the show split come from.
2. **Start and end a segment.** Give the operator a control for a talk break
   and for a live set. The control runs this command through `ssh` on the
   publisher host:

   ```
   musicindex-live-publisher segment start --instance NAME --target NAME < segment.json
   ```

   The JSON object goes through standard input, never through an argument.
   Its fields are in publisher ADR 0015 §Live Segments: `schema`, `kind`,
   `title`, `song_line`, `value_routes` and `max_secs`. `title` can hold the
   name of the band. `segment end` ends the segment. `segment show --json`
   gives its state.
3. **v4vmm makes no block.** Record that v4vmm does not assemble a live value
   block or a display state (publisher ADR 0014). Record that v4vmm does not
   publish to the relay. v4vmm sends requests through the publisher commands only.
4. **The custody of the token.** Select one of these:
   - v4vmm keeps the token file, and a rule says that v4vmm never uses a
     token to publish.
   - v4vmm gives the token file to the publisher at setup and keeps no
     usable copy.

   When a token file is deleted, nobody can get the token again (publisher
   `AGENTS.md` §5). Request 3 of
   `docs/plans/reserved-event-safety-request.md` is related.
5. **One instance for each event.** Publisher ADR 0016 proposes that
   `target add` refuses an event that a different instance holds. Show that
   refusal to the operator with its two instance names.
6. **The show name.** Publisher ADR 0015 open question 2 asks where the show
   name in `line` comes from. v4vmm can send it, or the publisher can read it
   from the target configuration. Give a preference.
7. **A talk show with no Mixxx.** Publisher ADR 0015 open question 1: a
   segment ends when no producer runs. Say if v4vmm needs a segment with no
   producer.

## What The Publisher Gives

After the implementation of publisher ADR 0015:

- `segment start`, `segment end` and `segment show --json`, for each instance
  and target.
- An error that names the field, when the segment object is not correct.
- A segment ends when a track plays, at `max_secs`, when the producer stops,
  and at a publisher restart. `segment show --json` then reports no segment.

## Interim Rules For The Operator

These rules are advisory. They stay until the requests are decided.

- Do not publish to the relay from v4vmm.
- Do not give a token to a second program.

## References

- `musicindex-live-publisher`: `docs/adr/0004-publisher-control-cli.md`,
  `docs/adr/0005-producer-liveness-and-dead-block.md`,
  `docs/adr/0014-one-assembler-and-track-reporting.md`,
  `docs/adr/0015-live-segments-from-v4vmm.md`,
  `docs/adr/0016-instances-events-and-producer-lock.md`
- `musicindex-live-relay`: `docs/adr/0003-display-state-and-artwork.md`,
  `docs/adr/0004-listener-timeline-delay.md`
- v4vmm: ADR 0059, `docs/plans/reserved-event-safety-request.md`,
  `docs/plans/broadcast-chain-delivery-order.md`
