# ADR 0075 Task 037: Preserve Remaining Existing Collections

Status: Complete - 2026-09-20. Technical review passed. Integrated checks and the normal binary build are Green.
Five focused transport tests, the format check, and `cargo check` are Green.

## Goal

Preserve existing upstream timestamps, platform claims, remote items, publisher relationships, and value time splits in app DTOs.
The [packet 034 inventory](../../schema/adr-0075-field-rules-aggregates-and-relationships.md#existing-api-fields-requiring-app-transport) specifies every member field.
Stophammer commit `a220f44`, `src/query.rs`, supplies these fields already.
This transport correction implements accepted evidence preservation. It selects no display source or payment behavior.

## Exact Contract

Add these optional fields with `#[serde(skip_serializing_if = "Option::is_none")]`:

| Parent | Field | Type |
|---|---|---|
| Feed and Track | `created_at` | `Option<i64>` |
| Feed | `source_platforms` | `Option<Vec<SourcePlatformClaim>>` |
| Feed and Track | `remote_items` | `Option<Vec<RemoteItem>>` |
| Feed and Track | `publisher` | `Option<Vec<PublisherRelationship>>` |
| Track | `value_time_splits` | `Option<Vec<ValueTimeSplit>>` |

Each member DTO derives `Debug`, `Clone`, `Serialize`, `Deserialize`, and `Default`, with `#[serde(default)]`.
Use the exact member names and types in packet 034's inventory.
Wrap every scalar in `Option`, including required upstream strings, integers, and booleans.
Omit absent member fields on serialization. Missing flags stay unknown.

Keep missing and null collections absent. Preserve an explicit empty array.
Preserve supplied order, duplicate occurrences, signed integers, false flags, unknown tokens, and empty strings.
Reject wrong JSON types. Do not coerce strings into integers or booleans.

Do not invent owner, path, or time fields absent from the upstream member contract.
Do not inherit these fields through `track_with_feed_defaults`.
Keep `PublisherRelationship` separate from the existing publisher search DTO.

## Scope And Guard

Change `src/api.rs` and required constructor sites only.
Preserve all previous transport changes. Do not change requests, storage, projections, payment code, or upstream files.
Do not add actions or interpret reciprocal flags as local verification.
Do not commit.

Tests must prove complete parent-object round trips for every added collection and scalar.
Also test missing, null, empty, repeated members, false versus missing flags, and wrong JSON types.
Use integers beyond `i32::MAX`, unknown strings, negative split values, and missing versus zero duration.
A feed-default test must prove these collections and timestamps do not become track assertions.

```bash
cargo test --locked --offline --lib adr_0075_remaining_transport
cargo fmt -- --check
cargo check --locked --offline
```

The orchestrator runs full integration checks. Rollback removes only these DTO additions and their required constructor changes.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, ADR 0075, this packet, and packet 034's inventory.
- `src/api.rs` and upstream `src/query.rs` at the inspected revision.
- The shared Rust and ASD-STE100 skills.

Goal:
- Preserve the existing wire fields listed above.

Constraints:
- Follow the exact contract. Preserve other agents' work.

Do not touch:
- Storage, requests, display, payment behavior, upstream code, and shared status documents.

Acceptance criteria:
- Every stated transport and no-inheritance test passes.

Test commands:
- Run the commands under Scope And Guard.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

This transport correction needs no visual acceptance. Existing visual gates remain paused.
