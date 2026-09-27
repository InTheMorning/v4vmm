# The liveValue URI Is A URL

Date: 2026-09-26. This plan states no rule. It records a defect that
`stophammer` found during the work on its ADR 0064, and the correction.

## The Defect

`feed_tag_for_event` in `src/view_models/show.rs` writes the event identifier
alone:

```xml
<podcast:liveValue uri="EVENT_ID" protocol="socket.io"/>
```

The tag names no host. A listener app cannot tell which relay holds the event,
so it cannot connect. The runbook `docs/runbooks/broadcast-operations.md` and
ADR 0059 show the same form. No listener app was tested with it.

## The Form Other Tools Write

thesplitkit.com writes a full Socket.IO URL
(`thesplitkit/src/lib/Share/Share.svelte`):

```xml
<podcast:liveValue uri="https://curiohoster.com/event?event_id=GUID" protocol="socket.io"/>
```

`musicindex-live-relay` accepts the same form. It registers the Socket.IO
namespace `/event` and reads `event_id` from the query. On 2026-09-26 the
Socket.IO transport answered at `https://api.musicindex.org/socket.io/`.

## The Correction

- `feed_tag_for_event` writes
  `uri="https://<relay host>/event?event_id=<EVENT_ID>"`. The relay host comes
  from the relay configuration of the event, not from a fixed value.
- The runbook and ADR 0059 show the same form.
- The test in `src/view_models/show/event_report.rs` and the test at the XML
  escape of the identifier expect the full URL.
- An operator replaces each tag that a feed already holds.

## Why Stophammer Needs It

`stophammer` ADR 0064 reads `podcast:liveValue` and gives it with each live
item. A client asks a relay if a stream is on air only in one condition. The
`uri` must name a host that the node lists as a confirming relay. A `uri` with no host names no
relay. The index then shows the item by its scheduled times only.
