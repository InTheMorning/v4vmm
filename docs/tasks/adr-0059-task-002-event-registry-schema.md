# ADR 0059 Task 002: Event Registry Schema And Token Files

## Goal

Add additive SQLite storage for broadcast events, and a token file writer with
mode `0600`. No service, no CLI, no UI.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0016-schema-migration-discipline.md`
- `docs/architecture/broadcast-chain.md`
- `src/db.rs` (the `MIGRATIONS` registry and `migrate_schema`)
- `src/config.rs` (config directory resolution)
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/db.rs`
- `src/broadcast/mod.rs` (new)
- `src/broadcast/tokens.rs` (new)
- `src/lib.rs` (module declaration)
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/api.rs`
- `src/cli.rs`
- `src/ui/**`, `src/view_models/**`, `src/app/**`
- `src/playback*`
- Any existing migration in the registry

## Constraints

- The new table is additive and goes through the `MIGRATIONS` registry, as ADR
  0016 requires. Do not add an inline `CREATE TABLE` outside the registry.
- **No token text in the database.** The row stores a file path only. This is an
  ADR 0059 invariant.
- The token file has mode `0600`. The parent directory has mode `0700`.
- Write the token file with a temporary file and a rename in the same directory.
- Reject an empty event identifier and an empty endpoint.
- Store timestamps as the existing repository convention for the table you copy.

## Implementation Steps

1. Add `src/broadcast/mod.rs` and declare the module in `src/lib.rs`.
2. Add a migration for `broadcast_events` with these columns:
   - `id` primary key
   - `event_id` text, not null, unique
   - `label` text, nullable, an operator name for the event
   - `endpoint` text, not null
   - `token_path` text, not null
   - `created_at` integer, not null
   - `last_checked_at` integer, nullable
   - `last_status` text, nullable, one of `unknown`, `live`, `dead`
3. Add typed input and row structs beside the existing local source-fact types
   in `src/db.rs`.
4. Add helpers: `insert_broadcast_event`, `broadcast_events`,
   `broadcast_event_by_id`, `delete_broadcast_event`, and
   `update_broadcast_event_status`.
5. Add `src/broadcast/tokens.rs` with `write_token_file` and `read_token_file`.
   `write_token_file` creates the parent directory with mode `0700`, writes the
   temporary file with mode `0600`, then renames it.
6. Resolve the token directory from the existing config directory helper, under
   `broadcast/tokens/`.
7. Add database tests: fresh schema, migrated schema, round trip, unique event
   identifier, delete, and status update.
8. Add token tests: file mode is `0600`, directory mode is `0700`, and a rewrite
   replaces the content.
9. Add an architecture guard: the `broadcast_events` schema text contains no
   column named `token`, and no file under `src/ui/` or `src/view_models/`
   names the table.

## Acceptance Criteria

- The table is created on a fresh database and on an existing database.
- No column stores token text.
- A written token file has mode `0600` on Unix.
- The database tests and the token tests pass.
- The architecture guard blocks a token column and blocks UI access.
- No API, CLI, or UI file changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast --lib --quiet`
- `cargo test db:: --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- The migration registry cannot express an additive table safely.
- The config directory helper does not expose a usable base path.
- File mode assertions are not portable in the existing test setup.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0016-schema-migration-discipline.md`
- `src/db.rs`
- `src/config.rs`

Goal:
- Add an additive `broadcast_events` table and a `0600` token file writer.

Constraints:
- Use the `MIGRATIONS` registry, not an inline `CREATE TABLE`.
- Never store token text in the database. The row holds a file path.
- Token file mode `0600`, parent directory mode `0700`, write with rename.

Do not touch:
- `src/api.rs`, `src/cli.rs`, UI, view models, playback

Acceptance criteria:
- Fresh and migrated schema both create the table.
- Token file mode and directory mode are correct.
- Architecture guard blocks a token column and blocks UI access to the table.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
