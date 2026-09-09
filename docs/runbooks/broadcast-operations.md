# Broadcast Operations

## Purpose

Use this runbook when you operate the ADR 0059 live payment chain. The app is a
control surface. The publisher, producer, encoder, and relay must continue to
run when the app is closed.

## Prerequisites

- An installed `v4vmm` binary.
- A configured `musicindex-live-publisher` instance.
- A producer such as `mixxx-now-playing`, or the opt-in `mpv` drop-file
  producer.
- A user `systemd` session for local service control.
- SSH access for each configured remote host.
- `butt` installed when the Stream section controls the encoder.

## Create An Event

Create the relay event from a terminal:

```bash
v4vmm broadcast events create --json --label "show-name"
```

The JSON output includes an `event_id` and a `token_path`. It does not print the
token text. Confirm the registry row when needed:

```bash
v4vmm broadcast events list --json
v4vmm broadcast events check EVENT_ID --json
```

## Publish The RSS Live Tag

Listener apps find a live event only through the `podcast:liveValue` tag in the
RSS feed of the show. Paste the tag from the Event section, or build it from the
event identifier:

```xml
<podcast:liveValue uri="EVENT_ID" protocol="socket.io"/>
```

The local chain can report success without this tag. In that case the publisher
sends payloads to the relay, but listener apps do not discover the event and no
listener receives anything.

## Back Up The Token File

The relay returns the broadcaster token one time. Nobody can replace it. If the
file is lost, create a new event and update the RSS tag.

Back up the token path from the event output:

```bash
install -m 700 -d "$HOME/.config/v4vmm/broadcast/token-backups"
cp -p TOKEN_PATH "$HOME/.config/v4vmm/broadcast/token-backups/"
```

For a remote publisher, copy the token file to the same path on the remote host,
or attach the target to a token path that already exists on that host. Do not
send token text as a command argument.

## Attach A Publisher Target

Read the target list:

```bash
v4vmm broadcast targets list --json
```

Attach the selected event to a publisher target:

```bash
v4vmm broadcast targets attach EVENT_ID --target default
```

The attach command runs the publisher `target add --replace` command and then
restarts the publisher unit. This app does not write the publisher configuration
file directly.

## Start And Stop Local Services

Use the Show surface when the desktop app is running. The equivalent terminal
commands are:

```bash
systemctl --user start mixxx-now-playing.service
systemctl --user start musicindex-live-publisher@mixxx.service
systemctl --user stop musicindex-live-publisher@mixxx.service
systemctl --user stop mixxx-now-playing.service
```

Replace `mixxx` with the configured publisher instance name.

## Start And Stop Remote Services

A remote host must be configured in `~/.config/v4vmm/config.toml` and must accept
non-interactive SSH. Check the remote unit state:

```bash
ssh HOST systemctl --user show musicindex-live-publisher@mixxx.service --property=LoadState,ActiveState,SubState,Result
```

Start and stop the remote publisher:

```bash
ssh HOST systemctl --user start musicindex-live-publisher@mixxx.service
ssh HOST systemctl --user stop musicindex-live-publisher@mixxx.service
```

The Show surface reports an unreachable SSH host as host reachability, not as a
failed service.

## Read Logs For A Failed Unit

Open the service log panel in Show, or read the journal from a terminal:

```bash
journalctl --user -u musicindex-live-publisher@mixxx.service -n 50 --no-pager
```

For a remote host:

```bash
ssh HOST journalctl --user -u musicindex-live-publisher@mixxx.service -n 50 --no-pager
```

If the unit reports `Result=start-limit-hit`, reset the failed state before you
start it again:

```bash
systemctl --user reset-failed musicindex-live-publisher@mixxx.service
systemctl --user start musicindex-live-publisher@mixxx.service
```

## Recover After Relay Restart Or Event Expiry

The relay stores live events in memory and removes idle events. A relay restart
or idle expiry makes the event dead.

Recovery steps:

1. Create a new event.
2. Back up the new token file.
3. Update the RSS `podcast:liveValue` tag with the new `EVENT_ID`.
4. Attach the publisher target to the new event.
5. Restart the producer and publisher if they do not pick up the new target.
6. Publish the updated feed, then send the normal feed announcement.

The old event cannot be recovered. A listener must discover the new event
identifier from the updated feed.

## References

- [ADR 0059](../adr/0059-broadcast-control-surface.md)
- [Broadcast chain architecture](../architecture/broadcast-chain.md)
- [ADR 0059 implementation review](../reviews/adr-0059-implementation-review.md)
