# ADR 0064 Task 002: Repair Report Surface

Status: Ready - 2026-09-08. Do after task 001, which writes the
`local_path_repairs` rows that this task shows.

## Goal

Show the operator which downloaded files the path repair could not find, so the
files can be moved into `music_dir` and downloaded again.

## Files To Inspect

- `docs/adr/0064-local-file-addressing.md`
- `docs/tasks/adr-0064-task-001-relative-local-paths.md`
- `src/library_path.rs`, for `repair_local_file_paths` and the repair row shape
- `src/application/queries/broadcast.rs`, for the readiness report precedent
- `src/view_models/show.rs`, for the `Source` card and its readiness rows
- `src/view_models/library.rs`, for the `Music` list that readiness opens
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/application/queries/library.rs`
- `src/view_models/show.rs`
- `src/view_models/library.rs`
- `src/cli.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/library_path.rs`. Task 001 owns the repair.
- `src/db.rs` schema beyond reading `local_path_repairs`
- `src/broadcast/**`, `src/runtime/**`
- The card contract from ADR 0063

## Constraints

- **The repair report is library content, not broadcast content.** ADR 0062 made
  `Music` the surface for library content, and a file that needs moving is a
  library fact. The `Source` card of `Show` does not grow a second list.
- The `Source` readiness detail already names the reasons a track is not ready.
  A track whose file was removed by the repair reads as not downloaded, which is
  correct and needs no new readiness state.
- The report names the **old** path, because that is where the operator looks
  for the file. It also names the track, so the operator knows what is lost.
- A repair row is history. Removing a row is an operator action, and the app
  never removes one on its own.
- Never offer to move a file. This app does not move an operator's files, and
  ADR 0064 says a download lives under `music_dir`.
- Display-ready text only in the view model. No `PathBuf` in a display contract.

## Implementation Steps

1. Add a query that reads `local_path_repairs` into a display-ready report: the
   track title, the artist, the old path, and the date the repair ran.
2. Add a view model for the report, following the readiness report contract in
   `src/application/queries/broadcast.rs`.
3. Show the report in `Music`, reached the same way the readiness list is
   reached. Follow the route that ADR 0062 and ADR 0059 task 012 established.
4. Add a row action that removes one repair row, and one that removes them all.
   Both are operator actions. Nothing removes a row on its own.
5. Add `v4vmm library repairs list --json` and
   `v4vmm library repairs clear` to the CLI.
6. Show a count in `Settings`, beside the music folder, so an operator who
   changes the folder sees the result of that change.
7. Add view-model tests:
   - an empty report renders an empty state, not a zero-length list
   - a report row carries the old path and the track title
   - removing a row leaves the others
8. Add a guard: the repair report renders in `Music` and in `Settings`, and no
   list of repair rows renders inside `Show`.

## Acceptance Criteria

Mechanical, proved by a test:

- The report view model carries display-ready text and holds no `PathBuf`.
- An empty report has an explicit empty state.
- A row carries the old path and the track title.
- Removing one row leaves the others, and nothing removes a row without an
  operator action.
- `v4vmm library repairs list --json` prints the rows.
- The guard blocks a repair list inside `Show`.

Visual, operator only:

- The count in `Settings` is legible beside the music folder.
- The old path is readable in full, or reachable without guessing. A path is
  long, and `docs/troubleshooting/column-text-truncation.md` records what
  happens to stacked text that truncates.
- The list explains what to do: move the file under the music folder, then
  download it again.

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

- The report needs a fact that `local_path_repairs` does not hold. Report which
  fact, before you add a column that task 001 did not plan.
- The `Music` route cannot carry a second report without a change to the ADR
  0062 row contract. Report it. Do not add a list inside `Show`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0064-local-file-addressing.md`
- `docs/tasks/adr-0064-task-001-relative-local-paths.md`
- `src/application/queries/broadcast.rs` for the readiness report precedent

Goal:
- Show the `local_path_repairs` rows in `Music` and a count in `Settings`, so an
  operator learns which files to move.

Constraints:
- The report is library content. It renders in `Music`, never inside `Show`.
- The report names the old path and the track.
- Removing a repair row is an operator action only.
- Never offer to move a file.
- Display-ready text only. No `PathBuf` in a display contract.

Do not touch:
- `src/library_path.rs`, `src/broadcast/**`, `src/runtime/**`

Acceptance criteria:
- An empty report has an explicit empty state.
- A row carries the old path and the track title.
- A guard blocks a repair list inside `Show`.

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
