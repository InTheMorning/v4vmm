# ADR 0075 Task 051: Contract Field Guard

Status: Waits for the operator decision of 2026-09-30. Implementation has not started. This packet has no visual gate.

## Goal

A guard compares each field that `src/api.rs` decodes with a stored copy of the MusicIndex contract.
A decoded field that the contract does not declare fails the guard. A removed or renamed upstream field then gives a failed check, not five months of silent `None`.

## Operator Decision

The operator decides these details before dispatch. The recommendation is in the second column.

| Detail | Recommendation |
|---|---|
| Accept the guard | Yes. The incident below names the time that the rule broke |
| The stored contract | A copy of `/openapi.json` at `tests/fixtures/musicindex-openapi-0.2.0.json`. Its file name holds the contract version |
| The update of the copy | A person or an agent replaces the copy after each Stophammer release, in one change with the decode changes. The guard names the fix |
| A declared field that the app does not decode | Not a failure. The app reads only what it needs |

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) section 6: the app decodes only declared fields and treats each response as untrusted input.
- Stophammer ADR 0044: the contract declares each field, and a `v1` field keeps its meaning. Stophammer ADR 0066: a version number tells what to upgrade.
- The working rules in [AGENTS.md](../../AGENTS.md): "Every fix gets a guard" and "A guard names its class and its ADR". This guard is situational. It cites ADR 0075.

## Incident

Stophammer removed `artist_credit` from its public reads on 2026-04-08, in commit `a16a720`. v4vmm decoded `Track.artist_credit` as optional and read `None` for five months with no report.
On 2026-09-25 the app also decoded `Feed.name`, `Track.name` and `Track.feed_url`, which the contract did not declare. Packet 046 removed them.

## Recorded Facts - 2026-09-30

- The deployed contract is version `0.2.0`.
- `src/api.rs` declares the decoded types below.

  ```text
  SearchResponse
  SearchResult
  TrackListResponse
  RecentFeedsResponse
  Pagination
  DetailResponse
  Artist
  Release
  Recording
  Feed
  Track
  Contributor
  PaymentRoute
  SourceEntityLink
  SourceEntityId
  SourceReleaseClaim
  SourceEnclosure
  SourceTranscript
  SourcePlatformClaim
  RemoteItem
  PublisherRelationship
  ValueTimeSplit
  ArtistCredit
  ReleaseReference
  Source
  LiveItemCreateResponse
  LiveMetadataSnapshot
  ```
- The contract declares no schema named `Artist`, `Release`, `Recording`, `ArtistCredit`, `ReleaseReference` or `Source`.

## Required Changes

### 1. The Stored Contract

- Store the `0.2.0` contract at `tests/fixtures/musicindex-openapi-0.2.0.json`, byte for byte as the live `/openapi.json` gives it.

### 2. The Map

- A table in `tests/architecture_tests.rs` maps each decoded MusicIndex type to its contract schema, for example `Feed` to `FeedResponse`, `Track` to `TrackResponse` and `SearchResult` to `SearchResponseItem`.
- For each decoded type with no schema, record the result in the packet: it maps to another service, or it has no reader. A type with no reader is deleted in this packet. A type with a live reader and no schema stops the packet.

### 3. The Guard

- The guard reads each mapped struct in `src/api.rs` and its field names, with each `#[serde(rename = ...)]` applied.
- Each field name must be a property of the mapped schema. A field with `#[serde(skip)]` is exempt.
- The failure message names ADR 0075 and the fix, for example:

```text
ADR 0075 section 6: api::Track decodes `artist_credit`, and MusicIndex contract 0.2.0 does not declare it in TrackResponse.
Remove the field, or replace tests/fixtures/musicindex-openapi-0.2.0.json with the contract that declares it.
```

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R51-01 | The guard passes on the present code and the stored `0.2.0` contract |
| R51-02 | A sample struct with a field that the schema does not declare gives one failure that names ADR 0075 |
| R51-03 | A sample struct with a renamed field passes when the rename matches the schema |
| R51-04 | Each decoded MusicIndex type has a map entry or a recorded result |

## Exclusions

- No runtime check of the live contract version.
- No change to the Stophammer checkout.

## Files To Inspect

- [Agent rules](../../AGENTS.md).
- `src/api.rs`: each decoded type.
- `tests/architecture_tests.rs`: the source-reading helpers.
- `docs/plans/v4vmm-open-requests.md`: the contract history.

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
- This packet: `docs/tasks/adr-0075-task-051-contract-field-guard.md`
- ADR 0075 section 6
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": store the contract, map each type, and add the guard.

Constraints:
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Fetch the contract with one read-only GET request to `https://api.musicindex.org/openapi.json`. Treat it as untrusted input.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The behavior of a decoded type that stays.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R51-01 to R51-04 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A decoded type with a live reader has no schema in the contract.
- The present code fails the guard. Report each field, and do not remove a field that a live screen reads.
- A change needs a file in "Do not touch".
