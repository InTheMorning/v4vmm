# ADR 0059 Broadcast Control Surface Phase Plan

## Status

Accepted - 2026-09-06.

## Goal

Make this app the control surface and the status display for an external
broadcast chain. The chain sends live payment data to listeners. The chain must
operate when this app is closed.

## Non-Goals

- No metadata sends from this app to the relay. The publisher is the only
  sender.
- No install of `musicindex-live-publisher`. That package installs separately.
- No changes to the drop-file contract. That contract belongs to the publisher.
- No liquidsoap control. Liquidsoap needs the deferred publisher control API.
- No Icecast control.
- No changes to the `QueueNowPlaying` frame or to local `mpv` transport.

## Current State

- `src/api.rs:555` creates live items. `src/api.rs:564` reads the latest
  metadata. `src/api.rs:581` and `src/api.rs:590` send metadata in the wrapped
  form that listener apps cannot read for payment splits.
- `src/cli.rs:17` to `src/cli.rs:171` give the `v4vmm liveitem` commands.
- `src/metadata.rs:3428` writes `TXXX:MusicIndex Value Routes` into downloads.
- `src/playback.rs:14` holds `NowPlayingUpdate` for local `mpv` playback.
- `src/view_models/workspace/frame.rs:45` holds four frame kinds.
- `src/runtime/playback_polling.rs` shows the runtime actor pattern for work
  that blocks.
- `src/http_client.rs` owns blocking HTTP client construction.
- The app has no record of live items, no service control, and no display of
  the external publisher.

## Target State

- A `Broadcast` frame with three sections: `Source`, `Publisher`, and `Event`.
- A local event registry with token files.
- A relay observation actor that shows listener truth for local hosts and
  remote hosts.
- Service control for a local host and for a remote host over SSH.
- A drop-file producer for the `mpv` source only.
- A library readiness report for payment route coverage.

## Affected Modules

- `src/api.rs`
- `src/cli.rs`
- `src/db.rs`
- `src/config.rs`
- `src/runtime/`
- `src/application/`
- `src/view_models/workspace/frame.rs`
- `src/view_models/` (new broadcast view models)
- `src/ui/shells/` (new broadcast shell)
- `src/app/` (new frame adapter)
- `tests/architecture_tests.rs`
- `docs/runbooks/workflows.md`

Each task confirms this module layout before it writes code:

- `src/broadcast_service.rs` or `src/broadcast/` for registry, control, and
  observation services. All GPUI-free.

## Proposed Sequence

Do one phase in each session. Make sure the phase is correct, then start a new
session.

1. **Live surface reduction.**
   Remove `publish_live_metadata`, `publish_live_metadata_with_token`, and the
   `v4vmm liveitem publish` commands. Keep create, health, and read. Set ADR
   0018 and ADR 0019 to `Superseded by ADR 0059`. Update
   `docs/runbooks/workflows.md`. No new behavior.

2. **Event registry.**
   Add the schema for events, as ADR 0016 requires. Write tokens to files with
   mode `0600`. Add a GPUI-free registry service for create, forget, resume,
   and a liveness test. Add CLI commands first, as ADR 0017 requires.

3. **Relay observation actor.**
   Read the latest relay payload for the selected event, one time each second,
   in a runtime actor. ADR 0040 gives the pattern. Project the result into a
   GPUI-free view model. No interface work.

4. **Broadcast frame.**
   Add the fifth workspace frame kind and its search scope. Build the three
   sections with shared primitives and typed action state. Add empty states for
   "no event" and "publisher not reachable". Add the architecture guards.

5. **Local publisher control.**
   Add a service that runs `systemctl --user` for the publisher unit and the
   producer unit. Report `active`, `inactive`, and `failed` as separate states.
   Add start, stop, and reset. Add a log panel with a button.

6. **Remote hosts.**
   Add a host list with a local entry and SSH entries. Run the same control
   commands through `ssh`. Show reachability as its own state. Read the drop
   file and the unit state through the same transport.

7. **The `mpv` source producer.**
   Write `musicindex.nowplaying/1` drop files for local `mpv` playback. Read
   the payment routes from the tags of the audio file, as `mixxx-now-playing`
   does. Treat pause as stopped and show the pause state in the panel. Remove
   the drop file when the app closes and warn the operator first.

8. **Library readiness report.**
   Report the tracks that carry no payment routes. Show the count in the
   `Source` section and the list in the content frame.

## Risks

| Risk | Mitigation |
|---|---|
| Loss of a token file kills an event permanently | Write mode `0600` files, show the path in the panel, and document a backup step in a runbook |
| A relay restart kills every event | Show dead events as dead. Add long-lived events to `splitkit` as follow-up work |
| The panel reports success while listeners get nothing | Show listener truth from the relay next to local rig state, and show the difference |
| A failed unit makes `Start` appear to do nothing | Report `failed` as its own state and offer a reset command |
| SSH is not configured on the operator machine | Show reachability as its own state with a clear empty state |
| The `mpv` producer and `mixxx-now-playing` write the same directory | One instance directory for each producer, as the publisher units define |
| Frame name confusion with `QueueNowPlaying` | Separate names, separate modules, and an architecture guard |
| Token text reaches a log file | Hide tokens in every log statement and in `Debug` output |

## Test Strategy

- Unit tests for the registry service: create, forget, resume, and dead-event
  detection.
- Unit tests for drop-file output against the `musicindex.nowplaying/1`
  examples in the publisher repository.
- Unit tests for the control service against recorded `systemctl` output for
  `active`, `inactive`, and `failed`.
- View-model tests for each of the three sections, with no GPUI imports.
- Architecture tests for the new frame kind, the source-kind rule, the token
  storage rule, and the no-send rule.
- Manual verification with a local relay from `~/build/splitkit` and a local
  publisher instance.

## Rollback Strategy

Each phase is additive except phase 1. Phase 1 deletes an untested path, and
`git revert` restores it.

The panel reads state and runs commands that the operator can also type. If the
panel fails, the publisher and the relay continue without it.

## Open Questions

- Which runbook holds the token backup procedure?
- Does the operator need Icecast state in the same panel?
- Which host does the readiness report use when the library is on one machine
  and the publisher is on another?

## References

- `docs/adr/0059-broadcast-control-surface.md`
- ADR 0016, ADR 0017, ADR 0040, ADR 0046, ADR 0057, ADR 0058
- `musicindex-live-publisher` ADR 0002 and its deployment runbook
