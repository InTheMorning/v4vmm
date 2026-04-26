# Pre-UI And Download Manager Preparation Plan

## Purpose

Before redesigning the UI or replacing the download manager, stabilize the
non-UI framework that both efforts will depend on.

The target is not a broad rewrite. The target is a set of narrow service
boundaries that make the existing behavior callable from CLI, UI, tests, and
future player/download adapters without duplicating business logic in GPUI
event handlers.

## Current Constraints

- Playlists are already largely functional.
- Phase 2 now-playing state has a first CLI-backed `PlaybackSession` scaffold.
- Discover and Library still contain substantial workflow logic inside UI
  modules.
- Schema setup still relies mostly on inline `CREATE TABLE IF NOT EXISTS`
  statements plus lightweight additive helpers.
- Download behavior exists, but the future download manager needs a stable
  track identity and local file contract first.

## Non-Goals

- Do not redesign the UI in this plan.
- Do not build the new download manager in this plan.
- Do not add player adapters, relay transport, Icecast push, or VTS resolution.
- Do not infer metadata identity from filenames, titles, or fuzzy matching.

## Guiding Principle

Every future UI or download action should call a tested non-UI service that
operates on explicit local database facts.

The first-class identity path remains:

```text
track_id
→ local file
→ feed_guid
→ item_guid
→ source value / extra JSON
```

## Phase A — Track Identity Service

### Goal

Create a single read path for the canonical facts needed by playback, VTS,
download, playlist, and UI workflows.

### Deliverables

- Add a `track_identity` or `library_service` module.
- Return a typed record for:
  - local track id
  - feed id
  - feed GUID
  - item GUID
  - title, artist, album, artwork
  - duration
  - local file path
  - item/feed value JSON
  - raw extra JSON
- Reuse this service from now-playing assembly instead of duplicating the SQL
  shape there.
- Add unit tests for missing local file, missing feed GUID, malformed JSON, and
  happy path.

### Acceptance

- `NowPlayingUpdate` still emits the same JSON for valid tracks.
- Source facts are preserved as raw JSON.
- Invalid identity facts fail before any session/download state is mutated.

## Phase B — Workflow Service Extraction

### Goal

Move business behavior out of UI event handlers into reusable service modules.

### Initial Services

- `playlist_service`
  - list playlists
  - inspect playlist tracks
  - append/remove/reorder tracks
  - preview playlist row as track identity
- `library_service`
  - inspect local track
  - mark/unmark track in library
  - resolve local file state
- `subscription_service`
  - subscribe feed
  - subscribe track
  - unsubscribe feed/track
  - report skipped/failure details as structured results

### Acceptance

- UI modules call services for the moved behavior.
- CLI commands can call the same services without GPUI dependencies.
- Tests target service functions directly.

## Phase C — Schema And Migration Discipline

### Goal

Make schema changes predictable before the download manager adds more state.

### Deliverables

- Define an internal migration registry in code or a `migrations/` directory.
- Record applied migration versions.
- Keep migrations idempotent for existing developer databases.
- Add a schema test that opens an empty DB, applies migrations, and verifies
  expected tables/indexes.

### Acceptance

- New tables are introduced through named migrations.
- Existing inline schema remains compatible until it can be safely retired.
- `cargo test` covers a fresh DB and a migrated DB path.

## Phase D — CLI Debug Contracts

### Goal

Provide stable non-UI inspection commands that make future UI and download work
observable.

### Initial Commands

```bash
v4vmm playlists list --json
v4vmm playlist tracks <playlist-id> --json
v4vmm track inspect <track-id> --json
v4vmm library tracks --json
```

### Acceptance

- Commands return structured JSON.
- Commands use the same services as the UI.
- Commands are documented in `docs/workflows.md`.

## Phase E — Download Manager Readiness

### Goal

Prepare the contract for a later download manager without implementing the
manager yet.

### Deliverables

- Document the intended download state model.
- Identify where current download code writes files, detects format, upgrades
  WAV to FLAC, and marks `local_files`.
- Define the future queue/status fields before adding UI.

### Acceptance

- The planned download manager can be built as a service over stable track
  identity and schema primitives.
- The current UI remains functional during the preparation work.

## Execution Order

1. Phase A: Track identity service.
2. Phase C: Migration discipline.
3. Phase D: CLI debug contracts.
4. Phase B: Extract one workflow service at a time.
5. Phase E: Download manager readiness document.

Do not batch these phases into one implementation session. Each phase should
land with tests and green CI before the next starts.
