# ADR 0016: Schema Migration Discipline

## Status

Accepted - 2026-04-26.

## Context

The current database setup is mostly inline schema creation with a small
additive migration helper. That has been sufficient while the app shape was
small, but the planned download manager, playback session state, playlist
expansion, and service extraction will require more deliberate schema changes.

Without migration discipline, developer databases can silently drift from the
expected schema and UI/download behavior can become difficult to reproduce.

## Decision

Future schema changes must be represented as named migrations or an explicit
in-code migration registry with monotonically increasing versions.

The migration system must:

- record applied versions
- run safely on a fresh database
- run safely on an existing developer database
- keep changes idempotent where practical
- include tests for fresh and migrated schema paths

The existing inline `CREATE TABLE IF NOT EXISTS` setup may remain while the
migration system is introduced, but new durable tables and columns should move
through the migration path.

## Consequences

- Download-manager state can be added without guessing which local databases
  have which tables.
- Tests can verify schema expectations directly.
- Schema changes become reviewable planning artifacts instead of incidental
  side effects of opening the app.
- There is some short-term duplication while the current inline schema and the
  migration registry coexist.
