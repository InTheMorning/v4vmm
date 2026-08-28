# ADR 0017: CLI Debug Contracts

## Status

Accepted - 2026-04-26.

## Context

The project now has a small CLI surface for Phase 2 playback-session work, but
most local library and playlist inspection still requires the desktop UI or
manual SQLite queries.

Before redesigning the UI or replacing the download manager, the project needs
stable non-UI ways to inspect core state. These commands should make backend
behavior visible and testable without binding it to GPUI rendering.

## Decision

Add CLI debug and inspection commands for core local state as services become
available.

Initial targets:

```bash
v4vmm playlists list --json
v4vmm playlist tracks <playlist-id> --json
v4vmm track inspect <track-id> --json
v4vmm library tracks --json
```

These commands must return structured JSON and call the same non-UI service
functions that the UI uses. They are not a separate business-logic layer.

## Consequences

- Future UI work can be checked against CLI output.
- Download-manager work can be developed and debugged without building UI
  controls first.
- JSON output becomes a lightweight local contract for tests and scripts.
- Command shape should stay conservative; avoid adding broad CLI workflows
  before the backing service exists.
