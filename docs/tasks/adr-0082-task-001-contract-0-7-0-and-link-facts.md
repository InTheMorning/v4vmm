# ADR 0082 Task 001: Contract 0.7.0 And Link Facts

Status: Implemented - 2026-10-02. Cases R82-1-01 to R82-1-06 pass. Mechanical checks are Green.
This packet has no visual gate.

## Goal

The app decodes and stores the link facts of Stophammer 0.7.0, and accepts a null `role`.
The contract guard compares against the 0.7.0 contract.

## Authority

- [ADR 0082](../adr/0082-publisher-roles-belong-to-each-album-link.md), Context and Decisions 2, 3, 5 and 6.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) section 6, and packet 051: the stored contract copy.
- ADR 0016: the migration registry.

## Recorded Facts - 2026-10-02

- The deployed node is release 0.7.0, at commit `5ba3d1f`. `info.version` of `/openapi.json` is `0.7.0`.
- Compared with the stored 0.2.0 copy, the 0.7.0 contract removes no schema and no field. It adds:
  - `PublisherResponse`: `album_names_as`, `role_agreement`.
  - `FeedResponse`: `agreed_roles`, `co_credited_feeds`, `stated_rels`, `two_way_link_count`.
  - A new schema, `CoCreditedFeedResponse`.
  - Other additions that v4vmm does not read: `FeedCopyResponse`, `FeedRemoteItemResponse` and the live-item view flags.
- `role` is null when no side states one, and null on a conflict. Before 0.6.0 it was `"artist"` for a default.
- `album_names_as` gives `publisher` or null in 0.7.0. The 0.7.0 contract text still names `credit`.
- `api::PublisherRelationship` in `src/api.rs` decodes no `album_names_as` and no `role_agreement`. `api::Feed` decodes no `co_credited_feeds`.
- `feed_publisher_relationships` in `src/db/publisher_relationships.rs` stores each relationship entry of a Library feed, without the two new facts. Schema version 17 is current (AGENTS.md).
- `PublisherPageVm::role_display` in `src/view_models/publisher_page.rs` maps a null `role` with `role_source` `default` to `AlbumRoleDisplay::Unknown`. Its `Assumed` variant needs a non-null `role` with `default`, which 0.7.0 never sends.
- `tests/fixtures/musicindex-openapi-0.2.0.json` is the stored contract. `MUSICINDEX_CONTRACT_FIXTURE` in `tests/architecture_tests.rs` names it.

## Required Changes

### 1. The Contract Copy

- Replace `tests/fixtures/musicindex-openapi-0.2.0.json` with `tests/fixtures/musicindex-openapi-0.7.0.json`, byte for byte as the live `/openapi.json` gives it. Confirm `info.version` is `0.7.0`.
- Update `MUSICINDEX_CONTRACT_FIXTURE`. The orchestrator updates the AGENTS.md line that names the file.
- Map `CoCreditedFeedResponse` in the contract type map.

### 2. Decode

- `api::PublisherRelationship`: add `album_names_as` and `role_agreement` as typed values. Use a `serde` enum with `rename_all = "snake_case"` and an unknown-value fallback, as `RoleSource` does.
- `api::Feed`: add `co_credited_feeds` as a list of a new type with `feed_guid`, `title`, `roles` and `album_count`.
- Accept a null `role` in every reader.

### 3. Store

- Add `album_names_as` and `role_agreement` columns to `feed_publisher_relationships` through the ADR 0016 migration registry, as schema version 18. An existing row gets null.
- Write both values with each stored relationship, and read them back.

### 4. The Unused Display Variant

- `AlbumRoleDisplay::Assumed` has no source in 0.7.0. Delete it, or map it as packet 002 needs. Packet 002 owns the new display. This packet keeps the page output equal for each 0.7.0 shape, and records the choice.

## Mechanical Acceptance Criteria

Use the prefix `adr_0082_link_facts_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R82-1-01 | The contract guard passes against the 0.7.0 copy, and the copy gives `info.version` `0.7.0` |
| R82-1-02 | A recorded 0.7.0 publisher row decodes `album_names_as` `publisher`, a null `role`, `role_source` `default` and a null `role_agreement` |
| R82-1-03 | A recorded row with `role_agreement` `both`, `one_side` or `conflict` decodes each value. An unknown value decodes to the fallback |
| R82-1-04 | A recorded publisher read decodes `co_credited_feeds` with each field |
| R82-1-05 | Migration 18 adds the two columns. A schema 17 database upgrades, and its existing rows read null |
| R82-1-06 | A stored relationship round-trips `album_names_as` and `role_agreement` |

## Exclusions

- No change to the publisher page or the album page output. Packets 002 and 003 own them.
- No decode of a field that v4vmm does not read.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/api.rs`: `PublisherRelationship`, `RoleSource`, `PublisherLinkResolution`, `Feed`.
- `src/db/publisher_relationships.rs` and the ADR 0016 migration registry.
- `src/view_models/publisher_page.rs`: `role_display`, `AlbumRoleDisplay`.
- `src/application/queries/feed.rs` and `src/application/queries/library.rs`: the publisher page facts.
- `tests/architecture_tests.rs`: the contract guard.

## Checks

```bash
cargo test --lib adr_0082_link_facts_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. Migration 18 adds two nullable columns. A database that ran it keeps them, and the earlier code ignores them.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0082-task-001-contract-0-7-0-and-link-facts.md`
- ADR 0082 and the ADR 0075 packet 051 document
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the contract copy, the decode, the storage, and the unused display variant.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Fetch the contract with one read-only GET request. Tests use recorded JSON and send no request.
- Treat each MusicIndex response as untrusted input.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The publisher page and the album page output.
- Any ADR, AGENTS.md, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R82-1-01 to R82-1-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The live `info.version` is not `0.7.0`.
- The contract guard fails for a field that a live screen reads.
- The migration registry needs a change to an existing migration.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-02

### 1. Files Changed

- `tests/fixtures/musicindex-openapi-0.7.0.json` is new. It is a byte-for-byte
  copy of the live `/openapi.json`, fetched on 2026-10-02. `info.version`
  reads `0.7.0`.
- `tests/fixtures/musicindex-openapi-0.2.0.json` is deleted.
- `tests/architecture_tests.rs`: `MUSICINDEX_CONTRACT_FIXTURE` names the
  0.7.0 copy. `CONTRACT_TYPE_MAP` gets a `CoCreditedFeed` entry.
  `CONTRACT_TYPES_WITHOUT_A_FIELD_CHECK` gets `AlbumNamesAs` and
  `RoleAgreement`. The violation message names contract `0.7.0`. One new
  test checks that the stored copy is version `0.7.0`.
- `src/api.rs`: new types `AlbumNamesAs`, `RoleAgreement`, and
  `CoCreditedFeed`. `PublisherRelationship` gets `album_names_as` and
  `role_agreement`. `Feed` gets `co_credited_feeds`. A new test module,
  `adr_0082_link_facts`, holds five tests against a recorded 0.7.0 row.
- `src/view_models/publisher_page.rs`: `AlbumRoleDisplay` drops its
  `Assumed` variant and its one match arm in `role_display`. The R3-07 test
  that checked `Assumed` is replaced with a test of the 0.7.0 shape.
- `src/ui/shells/publisher.rs`: one doc comment drops the word `Assumed`.
  No code changed.
- `src/db/publisher_link_facts.rs` is new. It holds migration 18: the two
  `ALTER TABLE` statements and the read contract that applies after them.
- `src/db.rs`: the module declaration, `CURRENT_VERSION` (18), the
  migration 18 entry, the `schema_contract` chain and its doc comment, and
  the verify-and-retained-digest block for migration 18. Two test lists of
  applied migration versions also change.
- `src/db/upgrades.rs`, `src/db/maintenance.rs`,
  `src/db/maintenance/upgrade.rs`, `src/db/payment_routes.rs`,
  `src/view_models/startup/database.rs`: each test value that named
  version 17 as the current schema version names 18 there. Section 6 gives
  the precedent for this class of edit.
- `src/db/publisher_relationships.rs`: the write statement adds
  `album_names_as` and `role_agreement`. Two new tests cover the version
  17 to 18 upgrade and the stored values of the two columns.
- `src/application/queries/library.rs`: two recorded-row test lists get
  the two new null columns.
- This packet document: the Status line and this section.

### 2. Decode Choices

- `AlbumNamesAs` and `RoleAgreement` follow the `RoleSource` pattern: a
  `from`/`into` string conversion with a named variant for each declared
  value, and an `Unknown(String)` variant for an unrecognized value.
- The 0.7.0 contract text names `credit` and `publisher` for
  `album_names_as`. The live node sends only `publisher` or a null value.
  `AlbumNamesAs::Credit` keeps `credit` as a named variant, because the
  contract names it. A test checks each value. One is `credit`.
- `role` stays `Option<String>`. No reader needed a change to accept a
  null value. The field was optional before this packet.
- `AlbumRoleDisplay::Assumed` needed a stated `role` paired with
  `role_source` `default`. The 0.7.0 contract and the recorded DETOX row
  rule out this pairing.
- Required Change 4 offered two paths: remove the variant, or map it for
  packet 002. This packet removes it. No 0.7.0 response gives this
  pairing, so the page output stays the same for each 0.7.0 shape.
- A row with no stated role reads as `Unknown` today, the value `Assumed`
  gave its own text for. Packet 002 owns the ADR 0082 role display, so a
  removed variant needs no bridge here.
- `cargo clippy --lib -- -D warnings` gave no unread-field warning for
  `co_credited_feeds` or `CoCreditedFeed`. Each type derives `Debug`, and
  that derive reads each field. This packet does not add
  `co_credited_feeds` to `PublisherPageFacts`. The Exclusions line on page
  output stays correct.

### 3. Migration 18 And Its Rollback

- `src/db/publisher_link_facts.rs` adds `album_names_as` and
  `role_agreement`, two nullable text columns, to
  `feed_publisher_relationships`. `publisher_relationships.rs` keeps the
  write and the read of each column of that table.
- A version 17 database upgrades to 18 with a null value in each new
  column of its existing row. The test
  `adr_0082_link_facts_version_17_migrates_to_18_with_null_link_facts`
  checks this.
- Rollback: revert the working tree. A database that ran migration 18
  keeps the two columns. The earlier code does not read or write them.

### 4. Tests Run

A terminal at the repository root ran each command below.

- `cargo test --lib adr_0082_link_facts_`: 8 passed. None showed an error.
- `cargo test --lib`: 1785 passed. None showed an error.
- `cargo test --test architecture_tests`: 288 passed. One test,
  `adr_0057_status_headers_are_canonical`, showed an error. This packet
  did not add that test or cause its error. Section 6 gives the cause.
- `cargo test`: the same result as the two commands above, together.
- `cargo fmt -- --check`: Green, after one format pass on `src/api.rs`.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green, with no warning.
- `grep -rn "dead_code" src`: no match.
- `cargo build --bin v4vmm`: Green.

### 5. Behavior Changed

- The app decodes `album_names_as`, `role_agreement`, and
  `co_credited_feeds` from a MusicIndex response. It stores the first two
  values with each publisher relationship row. No screen reads the three
  values today, so no screen output changes.
- A publisher-page album row with a stored `role` and `role_source`
  `default` reads as `Unknown`, not `Assumed`. A live 0.7.0 response sends
  no such pairing, so a person watching the deployed node sees no change.
  Section 2 gives the cause.

### 6. Deviations From Task

- `cargo test --test architecture_tests` shows one error this packet did
  not add. This packet cannot correct the error. The correction needs a
  change to `docs/adr/0078-publisher-page-type-from-stated-role.md`, a
  file not in this packet's edit scope.
- `adr_0057_status_headers_are_canonical` rejects that file's status
  header. The header reads "Superseded - 2026-10-02 by [ADR 0082](...)."
  ADR 0057 wants "Superseded by ADR 0082 - 2026-10-02."
- `git diff` shows this session made no change to that file. Commit
  `b10e4da`, the commit that accepted ADR 0082, wrote this header text,
  before this packet began. "Do not touch: Any ADR" keeps this packet
  from a correction. The Status line above names this open item.
- The ADR 0016 migration registry carries some test values that name "17"
  for "the current version" or "all migrations so far." Each such value
  needed an update to 18. Each test then kept passing. The files are
  `src/db/upgrades.rs` (the fixture range and two compatibility checks),
  `src/db/maintenance.rs` (one compatibility check),
  `src/db/maintenance/upgrade.rs` (an error message, a target check, a
  report check, three boundary cases, a verify call, and a row count),
  `src/db/payment_routes.rs` (the version check and the post-migration
  compatibility check), and `src/view_models/startup/database.rs` (one
  report check).
- The command `git log -p` on `src/db/upgrades.rs` and on
  `src/db/maintenance/upgrade.rs` shows the same class of update at each
  earlier migration. This packet follows that pattern. This packet does
  not count the pattern as a change to an existing migration.
- `src/db.rs` gets a new `if let (18, ...)` verify block, placed with the
  blocks for versions 12 through 17. Its retained-digest range grows from
  `13..=17` to `13..=18`. The command `git log -p` on `src/db.rs` shows
  this same pair of edits added for each earlier migration. Migration 18
  does not touch a version-11 era table. This check only proves that it
  leaves each retained row the same.
- `tests/fixtures/musicindex-openapi-0.2.0.json` is deleted with `rm`, as
  the fixture-rename note asks, not with a Git rename command.

### 7. Unresolved Concerns

- `adr_0057_status_headers_are_canonical` stays red until
  `docs/adr/0078-publisher-page-type-from-stated-role.md` gets a corrected
  status header. ADR 0082's own text says ADR 0078 moves to the archive in
  the commit of packet 002. That packet, or an earlier small correction,
  should correct the header text at the same time.
- `co_credited_feeds` decodes, but no page fact and no screen reads it.
  Packet 002 or 003 should make sure it decodes correctly when the
  "Shares Albums With" display reads it.
- A subsequent packet must read `album_names_as` and `role_agreement` out
  of the database, into `PublisherPageFacts`. That packet should compare
  each stored value with the wire value this packet tested.

## Orchestrator Review - 2026-10-02

- The orchestrator read the diff of each changed file. The changes stay inside the packet scope.
- The failed status guard came from the ADR 0078 header that the orchestrator wrote in the acceptance commit. The orchestrator corrected that header to the ADR 0057 form. The implementer did not edit an ADR.
- `AlbumNamesAs` keeps `Credit` because the 0.7.0 contract text still declares it. The open request to Stophammer asks for a correction of that text. Remove the variant in the change that stores the corrected contract.
- The orchestrator changed the contract fixture name and the schema version in `AGENTS.md`.
- Gate: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets` with no warning, `cargo test` (1785 library tests and 289 guards), and `cargo build --bin v4vmm` are Green. No `dead_code` attribute is in `src/`.
