# ADR 0075 Task 051: Contract Field Guard

Status: Implemented - 2026-10-01. Mechanical checks Green. This packet has no visual gate.

## Goal

A guard compares each field that `src/api.rs` decodes with a stored copy of the MusicIndex contract.
A decoded field that the contract does not declare fails the guard. A removed or renamed upstream field then gives a failed check, not five months of silent `None`.

## Operator Decision

The operator accepted each recommendation below on 2026-09-30.

| Detail | Recommendation |
|---|---|
| Accept the guard | Yes. The incident below names the time that the rule broke |
| The stored contract | A copy of `/openapi.json` at `tests/fixtures/musicindex-openapi-0.2.0.json`. Its file name holds the contract version |
| The update of the copy | A person or an agent replaces the copy after each Stophammer release, in one change with the decode changes. The guard names the fix |
| A declared field that the app does not decode | Not a failure. The app reads only what it needs |

## Authority

- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md) section 6: the app decodes only declared fields and treats each response as untrusted input.
- Stophammer ADR 0044: the contract declares each field, and a `v1` field keeps its meaning. Stophammer ADR 0066: a version number tells what to upgrade.
- The working rules in [AGENTS.md](../../../AGENTS.md): "Every fix gets a guard" and "A guard names its class and its ADR". This guard is situational. It cites ADR 0075.

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

- [Agent rules](../../../AGENTS.md).
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
- This packet: `docs/tasks/archive/adr-0075-task-051-contract-field-guard.md`
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

## Dispatch Stop - 2026-09-30

The first implementer stopped before any change, under the stop conditions of this packet. The stop was correct.

- `Artist`, `Release`, `Recording`, `ArtistCredit`, `ReleaseReference` and `Source` have readers. Each reader serves only the parked `discover::SearchApp`, which no entry point constructs.
- `Client::fetch_detail` decodes `Release` and `Recording` from `/v1/releases/{id}` and `/v1/recordings/{id}`. Contract `0.2.0` declares neither path.
- Their deletion needs files outside the edit scope of this packet.
- `LiveItemCreateResponse` and `LiveMetadataSnapshot` decode `/v1/liveitems` and `/v1/liveitems/{event_id}/metadata` of the live relay (archived ADR 0018). They map to another service.

[ADR 0060 packet 005](../adr-0060-task-005-delete-parked-discover-code.md) and [packet 006](adr-0060-task-006-delete-parked-discover-queries.md) delete the parked code and these six types first. This packet then runs with the six types gone.

## Implementation Result - 2026-10-01

### 1. Files Changed

- `tests/fixtures/musicindex-openapi-0.2.0.json` is new. It is a byte-for-byte
  copy of the live `/openapi.json`, fetched on 2026-10-01. `info.version` reads
  `0.2.0`.
- `tests/architecture_tests.rs` adds the contract field guard: the type map,
  the recorded-result list, the struct-reading helpers, and five tests.
- This packet document: the Status line and this section.
- `src/api.rs` has no change. Each field the app decodes today is a declared
  property of its mapped schema.

### 2. The Type Map

Each decoded MusicIndex type maps to a contract schema. A type with no schema
states the cause in the table below.

| Decoded type | Contract schema | Recorded cause (no per-field check) |
|---|---|---|
| `SearchResponse` | the `GET /v1/search` `200` response schema (an inline schema, not a named one) | |
| `SearchResult` | `SearchResponseItem` | |
| `TrackListResponse` | the `GET /v1/tracks` `200` response schema (inline) | |
| `RecentFeedsResponse` | the `GET /v1/feeds/recent` `200` response schema (inline) | |
| `Pagination` | `Pagination` | |
| `DetailResponse` | the `GET /v1/feeds/{guid}` `200` response schema (an inline schema that each single-item path shares) | |
| `Feed` | `FeedResponse` | |
| `Track` | `TrackResponse` | |
| `Contributor` | `SourceContributorClaimResponse` | |
| `PaymentRoute` | `RouteResponse` | |
| `SourceEntityLink` | `SourceEntityLinkResponse` | |
| `SourceEntityId` | `SourceEntityIdResponse` | |
| `SourceReleaseClaim` | `SourceReleaseClaimResponse` | |
| `SourceEnclosure` | `SourceItemEnclosureResponse` | |
| `SourceTranscript` | `SourceItemTranscriptResponse` | |
| `SourcePlatformClaim` | `SourcePlatformClaimResponse` | |
| `RemoteItem` | `FeedRemoteItemResponse` or `TrackRemoteItemResponse` (one Rust type decodes two endpoint shapes, and a field passes when one schema names it) | |
| `PublisherRelationship` | `PublisherResponse` | |
| `ValueTimeSplit` | `VtsResponse` | |
| `PublisherLinkResolution` | | A plain string value, not an object. MusicIndex sends it as the text of `PublisherRelationship.publisher_link_resolution`, a field the map checks. It declares no properties of its own. |
| `RoleSource` | | A plain string value, not an object, the same as `PublisherLinkResolution`. The map checks `PublisherRelationship.role_source` with the same rule. |
| `LiveItemCreateResponse` | | Maps to a different service. It decodes `POST /v1/liveitems` of the live relay (archived ADR 0018), not the MusicIndex contract. |
| `LiveMetadataSnapshot` | | Maps to a different service. It decodes `GET /v1/liveitems/{event_id}/metadata` of the live relay (archived ADR 0018), not the MusicIndex contract. |
| `EntityDetail` | | Wraps the mapped `Feed` type in Rust code. The app builds it from a decoded `Feed`. It does not decode `EntityDetail` from a wire response. |

No field was deleted. Each field that `src/api.rs` decodes today is a
declared property of its mapped schema. The Deviations section below records
one more check of this result, with a sample edit made and then reverted.

### 3. Tests Run

Each command ran at the repository root.

- `cargo test --test architecture_tests`: 286 passed. Six tests carry the
  `adr_0075_task_051_contract_field_guard` prefix. None showed an error.
- `cargo test`: 1793 lib tests, 286 architecture tests, and 10 doc tests. All
  lib and architecture tests passed. The doc tests are ignored by design. None
  showed an error.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green. No warning appeared.
- `cargo build --bin v4vmm`: Green.

### 4. Behavior Changed

None. The app decodes the same fields from the same endpoints as before. The
new guard runs only in `cargo test`. It adds no live check of the contract.
The Exclusions section asks for none.

### 5. Deviations From Task

- The orchestrator reviewed this packet on 2026-10-01 and asked for one
  change. The census must examine `src/api.rs` for each `Deserialize`
  derive, not read a fixed list. A new sample test must prove that an
  unmapped type fails. This document and `tests/architecture_tests.rs`
  reflect that change.
- The Required Changes section names two causes for a type with no per-field
  check. A type maps to a different service, or it has no reader and this
  packet deletes it. Three types need a third cause, which the Type Map table
  states for each one.
- `PublisherLinkResolution` and `RoleSource` are plain string values, not
  objects, so the guard finds no properties to examine. `EntityDetail` wraps
  a mapped type in Rust code. It does not decode the wire response itself.
  The operator should look at this selection.
- `SearchResponse`, `TrackListResponse`, `RecentFeedsResponse`, and
  `DetailResponse` decode a list or single-item envelope (`data` and `pagination`).
  The contract declares each envelope's shape inline in its path's response,
  not as a named schema. The guard reads the inline schema at one path for
  each of these four types, rather than treating them as schema-less.
- Before the Checks list ran, this session edited a saved copy of
  `src/api.rs` to add one more, undeclared field on `Track`. It ran the new
  guard test alone and confirmed the test reported the field by name, with
  the ADR 0075 section 6 message. It then restored the saved copy. `git diff`
  shows no change to `src/api.rs`. This check adds to the four accepted
  proofs (R51-01 to R51-04). It gives a sign that the guard works on an
  actual regression, not only on its own sample text.

### 6. Unresolved Concerns

- The guard finds each decoded type directly in `src/api.rs`. A new
  `Deserialize` struct or enum with no map entry and no recorded cause fails
  this test.
- `decoded_field_names` reads one `#[serde(...)]` attribute for each line.
  This matches each mapped struct's current layout. It panics with a named
  cause if it meets `#[serde(flatten)]`, because no mapped struct uses it
  today. A struct that spreads one attribute across more than one line, or
  that flattens a nested type, needs this helper extended first.

## Orchestrator Review - 2026-10-01

The orchestrator reviewed the diff two times and ran each check. Each check is Green: 1,793 unit tests, 286 guards, and no warning.

- The stored contract gives `info.version` `0.2.0`.
- Each decoded field of the present code is a property of its mapped schema. `src/api.rs` did not change.
- The first version checked only a fixed list of types. On the orchestrator's request, the guard now finds each `Deserialize` type in `src/api.rs`, and a type with no map entry and no recorded cause fails.
- After each Stophammer release, replace the stored contract in the same change as the decode changes. The file name holds the version.
