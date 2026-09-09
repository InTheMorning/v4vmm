# ADR 0064 Task 001: Relative Local Paths

Status: Ready - 2026-09-08. Large and atomic. The column changes meaning, so a
half-converted tree has a broken library. Do it in one change.

## Goal

Store `local_files.path` relative to `music_dir`, resolve it through one
function, and repair the rows that hold an absolute path today.

## Files To Inspect

- `docs/adr/0064-local-file-addressing.md`
- `src/db.rs`, for `local_files`, `upsert_local_file`, `mark_track_downloaded`,
  the five queries that select `lf.path`, and `migrate_schema`
- `src/config.rs`, for `music_dir`
- `src/app/bootstrap.rs`, for the point after the configuration loads
- `src/playback_owner.rs`, `src/application/queries/broadcast.rs`,
  `src/subscribe_service.rs`, `src/track_compare.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/library_path.rs` (new)
- `src/db.rs`
- `src/lib.rs`
- `src/app/bootstrap.rs`
- Every file that reads `TrackRow::local_path`. The compiler names them.
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/**`, `src/runtime/**`, `src/ui/**`
- The `tracks` table and the feed schema
- `music_dir` itself. This task never moves a file.

## Constraints

- **Make the compiler find every site.** `TrackRow::local_path` becomes a
  newtype, not a `String`. There are 44 read sites across 20 files, and a search
  will miss one.
- One resolver and one path-maker. No caller joins `music_dir` by hand, and no
  caller strips a prefix by hand. ADR 0058 states the same rule for the HTTP
  client.
- A stored path is relative, holds no leading separator, and holds no `..`.
  Reject all three at the writer.
- **No absolute path survives the repair.** A row the repair cannot resolve is
  removed, and its old path is recorded first. Leaving it would put two shapes in
  one column, which ADR 0064 rejected.
- A write of a path outside `music_dir` is an error. Report it and store
  nothing. Do not silently store an absolute path.
- **The repair step belongs in this task.** Without it, an existing database
  reads every absolute string as a relative one and the whole library breaks.
- The repair step runs after the configuration loads, not in `migrate_schema`.
  `migrate_schema` takes a `Connection` and cannot see `music_dir`.
- The repair step is idempotent. It runs on every start and does nothing when no
  absolute row is left.
- **The repair step never deletes a file.** It changes rows only.

## Implementation Steps

1. Add `src/library_path.rs`:
   - `LibraryRelativePath(String)`, a stored path relative to `music_dir`
   - `from_absolute(music_dir, absolute) -> Result<Self>`, which rejects a path
     outside `music_dir`, a leading separator, and any `..`
   - `resolve(&self, music_dir) -> PathBuf`
   - `as_stored(&self) -> &str`, for the database layer only
2. Change `TrackRow::local_path` to `Option<LibraryRelativePath>`. Follow the
   compiler through every read site. At each one, resolve against `music_dir`.
   Do not add a second join.
3. Change `upsert_local_file` and both `mark_track_downloaded` functions to take
   `&LibraryRelativePath`, and their callers to build one first.
4. Add `repair_local_file_paths(conn, music_dir) -> Result<LocalPathRepair>`:
   - select every row whose `path` is absolute
   - for each, take the longest trailing part of the path that names an existing
     file under `music_dir`, and store that part
   - a row with no match is **removed**, and its old path is recorded in a new
     `local_path_repairs` table, so the track reads as not downloaded and the
     operator still learns which file to move. ADR 0064 forbids leaving an
     absolute path in the column, because two shapes in one column is the thing
     it rejected.
   - return counts of repaired and removed rows
5. Call the repair after the configuration loads in `src/app/bootstrap.rs`, and
   log the counts. Task 002 gives the unresolved rows a surface.
6. Add a CLI command `v4vmm library repair-paths --json` that runs the same
   function and prints the counts and the recorded paths.
7. Add unit tests:
   - `from_absolute` accepts a path under `music_dir` and rejects one outside it
   - `from_absolute` rejects a leading separator and a `..` segment
   - `resolve` round-trips with `from_absolute`
   - the repair converts a row whose layout moved with the folder
   - the repair records and removes an unresolved row, and the track then reads
     as not downloaded
   - the repair is idempotent across two runs
8. Add a guard: no file outside `src/library_path.rs` joins `music_dir` to a
   stored path, and no file outside it strips a `music_dir` prefix.

## Acceptance Criteria

Mechanical, proved by a test:

- `TrackRow::local_path` is a newtype, so a missed read site does not compile.
- `from_absolute` rejects a path outside `music_dir`, a leading separator, and a
  `..` segment.
- The repair converts a row whose folder moved with its layout intact.
- The repair records an unresolved row in `local_path_repairs`, removes the
  `local_files` row, and leaves the track readable as not downloaded.
- No absolute path is left in `local_files.path` after the repair.
- Two repair runs give the same result as one.
- The guard blocks a hand-written join or prefix strip outside the resolver.
- `v4vmm library repair-paths --json` prints counts and the recorded paths.

Visual, operator only:

- The readiness report counts the repaired tracks as ready or as missing routes,
  and no longer as missing files.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

Do not run the app. Write the operator visual check instead, as AGENTS.md
requires.

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns
6. Operator visual check

## Escalation Triggers

- A read site needs the absolute path before the configuration is available.
  Report the site. Do not cache a resolved path in the row.
- A writer receives a path outside `music_dir` in normal operation. Report which
  writer, because ADR 0064 says that case does not exist.
- The trailing-part match finds the wrong file for a row. Report it before you
  add a checksum, which ADR 0064 rejected for now.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0064-local-file-addressing.md`
- `src/db.rs`, `src/config.rs`, `src/app/bootstrap.rs`

Goal:
- Store `local_files.path` relative to `music_dir`, resolve through one
  function, and repair existing absolute rows at start.

Constraints:
- `TrackRow::local_path` becomes a newtype, so the compiler finds all 44 read
  sites across 20 files.
- One resolver and one path-maker. No hand-written join and no hand-written
  prefix strip.
- A write outside `music_dir` fails and stores nothing.
- The repair runs after the config loads, not in `migrate_schema`, and it is
  idempotent. It never deletes a file.

Do not touch:
- `src/broadcast/**`, `src/runtime/**`, `src/ui/**`, the `tracks` table

Acceptance criteria:
- A missed read site does not compile.
- The repair converts a moved layout, records and removes what it can not
  resolve, and is idempotent.
- A guard blocks a join or a prefix strip outside the resolver.

Test commands:
- `cargo fmt -- --check`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
6. operator visual check
