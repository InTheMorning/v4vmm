# ADR 0076 Task 007: A Guard Reads Test Files As Test Code

Status: Implemented - 2026-09-30. Mechanical checks Green. This packet has no visual gate.

## Goal

The guard helper that removes test code from a source file also removes each file that only a test build compiles.
A test in such a file then cannot give a false guard result.

## Authority

- [ADR 0076](../../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 9 and its guard `adr_0076_route_readiness_route_frame_writes_read_the_stored_route`.
- The working rules in [AGENTS.md](../../../AGENTS.md): "A guard names its class and its ADR" and "Every fix gets a guard".

## Recorded Facts - 2026-09-30

- `without_unit_test_module` in `tests/architecture_tests.rs` cuts a file at the first line pair `#[cfg(test)]` and `mod ... {`.
- Four files hold only test code. Their parent declares each one with `#[cfg(test)]` and `mod tests;`:
  - `src/discover/tests.rs` (from `src/discover.rs`)
  - `src/view_models/workspace/tests.rs`
  - `src/view_models/search/tests.rs`
  - `src/view_models/search_results/tests.rs`
- The helper reads each of these files as production code, because the file has no `mod tests {` line.
- Incident, ADR 0080 packet 002 on 2026-09-29: a test in `src/discover/tests.rs` wrote a file with `write_id3v24_edits`. The ADR 0076 guard reported it as a route frame write. The implementer moved the test to avoid the false result.

## Required Changes

1. Add one guard helper that tells if a source file only compiles in a test build. A file is test-only when its parent module file declares it with `#[cfg(test)]` on the line before `mod <name>;`.
2. `without_unit_test_module`, or each caller of it, gives an empty production text for a test-only file.
3. Keep every other result of the helper equal.
4. Move the test of ADR 0080 packet 002 (the R82-09 round trip in the `mod tests` block of `src/metadata.rs`) only if it belongs better in `src/discover/tests.rs`. Do not move it for any other reason.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R76-7-01 | The helper marks the four files of "Recorded Facts" as test-only, and marks `src/discover.rs` and `src/metadata.rs` as not test-only |
| R76-7-02 | A sample test-only source with a `write_id3v24_edits(` call gives no ADR 0076 route violation |
| R76-7-03 | A sample production source with the same call still gives one violation |
| R76-7-04 | Each guard in `tests/architecture_tests.rs` passes with the same count as before, 283 or more |

## Exclusions

- No change to any rule that a guard enforces.
- No change to `src/`.

## Files To Inspect

- [Agent rules](../../../AGENTS.md).
- `tests/architecture_tests.rs`: `without_unit_test_module`, `production_source`, `code_only`, `rust_files_under`, `route_source_violations`, and each caller of `without_unit_test_module`.

## Checks

```bash
cargo test --test architecture_tests
cargo test
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`
- This packet: `docs/tasks/archive/adr-0076-task-007-guard-reads-test-files-as-test-code.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": detect a test-only file, and give it no production text.

Constraints:
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- A guard failure message names the owning ADR and the fix.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- Any file in `src/`.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R76-7-01 to R76-7-04 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A guard starts to fail after the change, because it found real production code in a file that it did not read before.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-09-30

### Guard Helper

`is_test_only_source_file(path)` reads the parent module file of `path`. The
file is test-only when the parent declares it with `#[cfg(test)]` on the line
before `mod <name>;`. The helper checks three candidate parents: `mod.rs` and
`lib.rs` in the same directory, and the sibling file `<dir>.rs` adjacent to
that directory. `module_declares_test_only_child` and `line_declares_mod` do
the line-pair check. An optional visibility keyword, such as `pub` or
`pub(crate)`, may come before `mod`.

`without_unit_test_module` takes `path` and `source`. It returns an empty
string for a test-only file, before the line-pair rule runs. Each of its
seven callers passes the file path.

### Files Changed

`tests/architecture_tests.rs` is the only changed file. No file in `src/`
changed.

### New Test

`adr_0076_route_readiness_ignores_test_only_files` proves three mechanical
cases:

- R76-7-01: the four recorded test-only files report test-only.
  `src/discover.rs` and `src/metadata.rs` report not test-only.
- R76-7-02: a sample test-only source with a `write_id3v24_edits(` call gives
  no route violation.
- R76-7-03: the same call in a production source continues to give one
  violation.

R76-7-04 has no test function of its own. It states a suite-wide count. The
full `cargo test --test architecture_tests` run proves it: 284 tests pass, one
more than the recorded baseline of 283.

### Behavior Changed

Each guard that calls `without_unit_test_module` reads an empty production
text for the four test-only files. A test in `src/discover/tests.rs`,
`src/view_models/workspace/tests.rs`, `src/view_models/search/tests.rs`, or
`src/view_models/search_results/tests.rs` can no longer trigger an incorrect
ADR 0076 route violation, tag-write violation, or credit-list violation.

The guard `adr_0079_removed_artist_storage_stays_deleted` keeps its own check
for a file named `tests.rs`. That check skipped the same four files before
this packet. The new general check changes no result for this guard.

### Deviations

Required Change 4 asks to move the R82-09 test from `src/metadata.rs` into
`src/discover/tests.rs`, when it belongs there. This session's rules forbid a
change to a file in `src/`. The test stays in `src/metadata.rs`. Its own
`mod tests { ... }` block gives `without_unit_test_module` an empty
production text for it. This packet closes the risk of an incorrect guard
result from this test.

### Unresolved Concerns

The R82-09 test in `src/metadata.rs` calls functions that
`src/discover/tests.rs` also imports for its own tests, such as
`expand_woar_metadata_rows` and `pending_id3_edits_for_apply`. A subsequent
session that can change `src/` should judge the test's fit for
`src/discover/tests.rs`, by subject. That question is apart from the guard
concern this packet closes.

### Checks - 2026-09-30

| Check | Result |
|---|---|
| `cargo test --test architecture_tests` | Green, 284 tests (283 before this packet) |
| `cargo test` | Green, 1925 library tests, 284 guard tests, 10 documentation examples ignored |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

## Orchestrator Review - 2026-09-30

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,925 unit tests, 284 guards, and no warning.

- The helper reads the parent module file in both layouts: `<dir>.rs`, and `mod.rs` or `lib.rs`.
- Required Change 4 contradicted "Do not touch", which forbids each change in `src/`. That was an error in the packet. The implementer followed "Do not touch", which was correct. The R82-09 test stays in `src/metadata.rs`, where no guard misreads it.
- The file-name skip in `adr_0079_removed_artist_storage_stays_deleted` gives the same result as the new helper. A later guard change can delete it.
