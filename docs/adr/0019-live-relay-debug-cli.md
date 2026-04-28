# ADR 0019: Live Relay Debug CLI

## Status

Accepted

## Context

The public MusicIndex live relay is deployed at `api.musicindex.org` and
exposes the v1 live item workflow:

- create a live item and receive a one-time broadcaster token
- publish metadata to that event with bearer-token authorization
- read the latest metadata snapshot for public verification

The desktop app already has a canonical `NowPlayingUpdate` JSON contract and a
MusicIndex API client with the live metadata publish path, but testing the relay
still requires ad hoc HTTP commands.

## Decision

Add conservative CLI debug commands under `v4vmm liveitem`:

```bash
v4vmm liveitem health
v4vmm liveitem create --json
v4vmm liveitem latest <event-id> --json
v4vmm liveitem publish <event-id> --metadata-json '<json>' --token <token>
v4vmm liveitem publish-now-playing <event-id> --token <token>
v4vmm liveitem publish-now-playing <event-id> --dry-run
```

The commands use the configured `musicindex_endpoint` by default and accept
`--endpoint <url>` for local relay testing. Publish commands may also read the
broadcaster token from `MUSICINDEX_LIVEITEM_TOKEN`.

`publish-now-playing` does not infer metadata identity. It serializes the
current canonical `NowPlayingUpdate` as the relay metadata payload.

## Consequences

- Operators can verify the public relay without leaving the v4vmm toolchain.
- Relay tests stay aligned with the same API client used by future playback
  publishing work.
- The broadcaster token remains an explicit relay boundary input instead of
  being stored in local playback state.
