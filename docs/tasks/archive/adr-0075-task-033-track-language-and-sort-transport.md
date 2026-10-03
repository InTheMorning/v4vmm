# ADR 0075 Task 033: Preserve Track Language And Artist Sort Text

Status: Complete - 2026-09-20. Technical review passed. Integrated checks and the normal binary build are Green.
Five focused transport tests, the format check, and `cargo check` are Green.
The operator authorized orchestration of ADR 0075 completion.

## Goal

Preserve the upstream `TrackResponse.language` and `track_artist_sort` fields in the app's `Track` type.
These fields exist in Stophammer `src/query.rs`, commit `a220f44`.
Their transport needs no new source-selection policy.

## Contract And Scope

Add `language: Option<String>` and `track_artist_sort: Option<String>` to `Track` in `src/api.rs`.
Keep `#[serde(default)]`. Omit each new field on serialization when its value is `None`.
Preserve supplied text exactly, including empty strings and unknown language codes.

Do not copy feed language or `release_artist_sort` into either field.
Keep missing and null values unknown. Update constructors only when compilation requires it.

Inspect [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md), the [inventory](../../schema/adr-0075-metadata-field-inventory.md), and `src/api.rs`.
Change the DTO and its tests only, plus required constructor sites.
Do not change storage, normalization, sorting, display, requests, or upstream code.
Preserve packets 030 and 032. Do not commit.

## Acceptance And Checks

- Tests preserve both supplied strings through parent `Track` serialization and a second decode.
- Tests keep older, missing, and null fields absent on serialization.
- Tests preserve empty and unknown text without inventing provenance.
- Tests show that feed defaults do not populate these track fields.

```bash
cargo test --locked --offline --lib adr_0075_track_scalar_transport
cargo fmt -- --check
cargo check --locked --offline
```

The orchestrator runs full checks after integration.
Rollback removes only these transport additions and required constructor changes.
Stop if storage or display policy is needed to complete the transport correction.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, ADR 0075, this packet, and `src/api.rs`.
- The shared Rust and ASD-STE100 skills.

Goal:
- Preserve existing upstream track language and artist sort text.

Constraints:
- Follow the field contract above. Preserve existing work.

Do not touch:
- Storage, requests, display, upstream code, and shared status documents.

Acceptance criteria:
- The transport and no-inheritance tests above pass.

Test commands:
- Run the commands in Acceptance And Checks.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

This transport change needs no visual acceptance. Existing visual gates remain paused.
