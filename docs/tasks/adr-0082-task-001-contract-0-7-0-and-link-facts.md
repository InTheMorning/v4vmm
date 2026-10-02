# ADR 0082 Task 001: Contract 0.7.0 And Link Facts

Status: Ready - 2026-10-02. Implementation has not started. This packet has no visual gate.

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
