# ADR Triage Task 001: Guard Citations And Archive Of Four ADRs

Status: Implemented - 2026-10-03. Mechanical checks Green.
This task changes no behavior and has no visual gate.

## Goal

Each test that enforces ADR 0011, 0016, 0022 or 0064 names that ADR in its failure message. Then the four ADRs move to `docs/adr/archive/`.

## Authority

- [ADR 0061](../adr/0061-executable-governance.md): a guard names its class and its ADR. A fully guarded ADR archives.
- The operator decided on 2026-10-03 in the [ADR triage](../reviews/adr-triage-2026-10-03.md), group B2.

## Recorded Facts - 2026-10-03

- ADR 0011: tests named for ADRs 0075 and 0080 prove the feed GUID and track GUID TXXX frames.
- ADR 0016: `test_migrations_record_versions_on_fresh_schema` and `test_migrations_update_legacy_schema` in `src/db.rs` prove the migration registry. Two ADR 0066 guards also cite ADR 0016.
- ADR 0022: the guard `core_non_ui_modules_do_not_import_ui_modules` in `tests/architecture_tests.rs` enforces the rule. Its failure message cites ADR 0025.
- ADR 0064: about nine tests in `src/library_path.rs` and `src/db.rs` prove its invariants. One test cites ADR 0064.
- The batch evidence for each ADR is in the triage scratch files. The triage record lists the guards.

## Required Changes

1. For each rule of the four ADRs, find the enforcing test. Add the ADR number to its failure message, and to its doc comment where the test has one. Keep the test logic unchanged.
2. When a test also enforces a rule of another ADR, keep both citations. Read ADR 0025 before you change the ADR 0022 guard message.
3. When a rule of one of the four ADRs has no enforcing test, stop and report it. Do not write a new test, and do not archive that ADR.
4. Move the four ADR files to `docs/adr/archive/` with plain `mv`. Add after the status paragraph: `Archived 2026-10-03: a guard enforces each rule ([ADR triage](../../reviews/adr-triage-2026-10-03.md)).`
5. Update each link to the four paths in `docs/`, `AGENTS.md`, `src/` and `tests/`. Update both ADR indexes. Record the archive in the "Decisions - 2026-10-03" section of the triage record.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| RT-1-01 | Each rule of the four ADRs maps to a test whose failure message names the ADR. The task result lists each mapping |
| RT-1-02 | Each relative Markdown link into `docs/adr` resolves, and no path of a moved ADR stays in `src/` or `tests/` |
| RT-1-03 | `adr_0057_status_headers_are_canonical` passes |

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree.

## Implementation Result - 2026-10-03

### 1. Rule-to-test mapping

ADR 0011 (one rule: map the RSS feed GUID and the RSS track GUID to their own
`TXXX` frames, through the RSS-to-ID3 staging path):

- `metadata_service::tests::adr_0075_track_header_r22_06_tag_edits_stay_equal_without_own_identity`
  and `..._with_own_identity` (`src/metadata_service.rs`). Each test checks
  the two frames against one list of fixed edits.
- `feed_service::tests::library_track_context_preserves_feed_guid_for_id3_provenance`
  (`src/feed_service.rs`). This test checks the feed frame on the library
  read path.

ADR 0016 (two rules):

- The registry records each applied version and runs on a new database:
  `db::tests::test_migrations_record_versions_on_fresh_schema` (`src/db.rs`).
- The registry runs on an existing database and stays idempotent:
  `db::tests::test_migrations_update_legacy_schema` (`src/db.rs`).

ADR 0022 (one rule: a service module does not import `gpui`, `gpui_component`,
`library`, `search`, or `app`):

- `core_non_ui_modules_do_not_import_ui_modules` (`tests/architecture_tests.rs`).
  This guard named ADR 0025 before this change. This change adds the ADR 0022
  citation to it.

ADR 0064 (three rules):

- A stored path is relative to `music_dir`, with no leading separator and no
  parent-directory segment:
  `library_path::tests::from_absolute_accepts_path_under_music_dir`,
  `from_absolute_rejects_path_outside_music_dir`,
  `stored_path_rejects_leading_separator`,
  `from_absolute_rejects_parent_directory_segment`, and
  `resolve_round_trips_with_from_absolute` (`src/library_path.rs`).
- Each read and each write goes through the one resolver in
  `src/library_path.rs`: `adr_0064_local_file_paths_resolve_only_through_library_path`
  (`tests/architecture_tests.rs`). This guard named ADR 0064 before this
  change.
- The repair step meets three rules. It is idempotent. It reports an
  unresolved row before it removes the row. It does nothing when `music_dir`
  is missing, and it does nothing when no row resolves. These tests prove it:
  `db::tests::repair_local_file_paths_does_nothing_when_the_music_folder_is_missing`,
  `repair_local_file_paths_does_nothing_when_no_row_resolves`,
  `repair_local_file_paths_converts_moved_layout`,
  `repair_local_file_paths_records_and_removes_unresolved_row`, and
  `repair_local_file_paths_is_idempotent` (`src/db.rs`).

No rule of the four ADRs lacked an enforcing test.

### 2. Files changed

Code:

- `src/metadata_service.rs`, `src/feed_service.rs`, `src/db.rs`,
  `src/library_path.rs`, `tests/architecture_tests.rs`: added the ADR number
  to the `assert!` or `assert_eq!` message of each test named above. It also
  added the ADR number to each doc comment that named the test's rule before
  this change. No test logic changed.

Archive move and notice:

- `docs/adr/0011-musicindex-guid-id3-tags.md`,
  `docs/adr/0016-schema-migration-discipline.md`,
  `docs/adr/0022-ui-agnostic-core-extraction.md`, and
  `docs/adr/0064-local-file-addressing.md` moved to `docs/adr/archive/`, each
  with the archive notice after its status paragraph.

ADR indexes:

- `docs/adr/README.md`: removed the four rows.
- `docs/adr/archive/README.md`: added the four rows, each marked "Fully
  guarded".

Link repair, in `docs/adr` and `docs/tasks`:

- `docs/adr/0066-configuration-and-startup-failure-recovery.md`,
  `docs/adr/0068-show-cue-and-audition-isolation.md`,
  `docs/tasks/adr-0059-task-002-event-registry-schema.md`,
  `docs/tasks/adr-0064-task-001-relative-local-paths.md`,
  `docs/tasks/adr-0064-task-002-repair-report-surface.md`,
  `docs/tasks/adr-0066-task-002-core-checks-and-startup-reports.md`,
  `docs/tasks/adr-0066-task-010-database-check-and-backup.md`,
  `docs/tasks/adr-0066-task-013-interrupted-upgrade-repair.md`,
  `docs/tasks/adr-0075-task-012-provider-snapshot-migration.md`,
  `docs/tasks/adr-0076-task-001-playlist-rss-document-check.md`,
  `docs/tasks/adr-0076-task-002-rss-comparison-apply-and-report.md`,
  `docs/tasks/adr-0076-task-003-stored-payment-route-and-readiness.md`,
  `docs/tasks/adr-0077-task-001-remove-dead-artist-storage.md`, and
  `docs/tasks/adr-0077-task-002-publisher-relationship-transport-and-storage.md`:
  each link or named path to one of the four files points at its `archive/`
  location after this change.

### 3. Tests run

- `cargo test`: Green. The orchestrator ran it again: 1815 library tests and 289 architecture tests pass.
- `cargo test --test architecture_tests`: Green, 289 passed.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green.
- `cargo build --bin v4vmm`: Green.
- `adr_0057_status_headers_are_canonical`: Green on its own, so the new
  archive notices keep the canonical status format.
- A script checked each relative Markdown link in `docs/` and `AGENTS.md`
  that points into `docs/adr`. It found 465 such links across 652 files.
  Each one resolves.

### 4. Deviations

- The task names "each link" in `docs/`, `AGENTS.md`, `src/`, and `tests/`.
  `src/` and `tests/` name the four ADRs only in prose, for example "ADR
  0016's schema authority". `src/` and `tests/` name no file path for the
  four ADRs, so these two directories needed no change.
- The task's word "link" could name a markdown link alone. This change takes
  it to also cover a plain, backtick file path in a historical task
  document. One example is `` `docs/adr/0016-schema-migration-discipline.md` ``.
  This change corrects that type of path when it names a moved ADR. The
  prior archive commit (433e46a) corrected the same type of path.
- A plain prose citation of an archived ADR has no link and no file path. One
  example is "ADR 0016 - Schema migration discipline" in a References list.
  This citation stays as it is. An archived ADR keeps its number, so these
  citations read correctly after the move.
- The task's Recorded Facts name two ADR 0066 guards:
  `adr_0066_database_checks_and_snapshots_have_one_owner` and
  `adr_0066_upgrade_repair_uses_normal_migration_authority`. Each guard's doc
  comment names ADR 0016 before this change. The `assert!` message of each
  guard stays about ADR 0066, because these are ADR 0066's own guards. ADR
  0016 is their second citation only.
- The two migration tests named above are the primary tests for each ADR
  0016 rule. The task's rule-to-test requirement for ADR 0016 is complete
  with these two tests.
- A test with no doc comment before this change has no new doc comment. The
  task asks for a doc comment update only "where the test has one." The
  `assert!` or `assert_eq!` message of each of these tests names the ADR.

### 5. ADRs not archived

None. This change archives all four ADRs: 0011, 0016, 0022, and 0064. Each
rule of each ADR has a test that proves it.

## Prompt for lower-context coding model

You are implementing one bounded task.

Read:
- `AGENTS.md`
- This task: `docs/tasks/adr-triage-task-001-guard-citations-and-archive.md`
- ADRs 0011, 0016, 0022, 0025, 0061 and 0064
- `docs/reviews/adr-triage-2026-10-03.md`

Goal:
- Make each change in "Required Changes".

Constraints:
- Change only failure messages, doc comments, documents and file locations. Change no test logic and no production behavior.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git commit` or `git mv`.
- Do not run the app.

At the end, report:
1. the rule-to-test mapping for each ADR
2. files changed
3. tests run
4. deviations
5. each ADR that you did not archive, and why

Stop and report, and do not guess, when a rule has no enforcing test.
