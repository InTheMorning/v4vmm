# ADR 0075 Task 032: Preserve Transcript Claim Transport

Status: Complete - 2026-09-20. Technical review passed. Integrated checks and the normal binary build are Green.
Seven focused transport tests, the format check, and `cargo check` are Green.
This packet implements the existing wire contract. Source selection and storage remain separate work.

## Goal And Contract

Preserve `TrackResponse.source_transcripts` from Stophammer `src/query.rs`, commit `a220f44`.
Add `Track.source_transcripts: Option<Vec<SourceTranscript>>` in `src/api.rs`.
Keep the collection absent on serialization when its value is `None`.
An empty collection remains an explicit empty array.

`SourceTranscript` derives `Debug`, `Clone`, `Serialize`, `Deserialize`, and `Default`.
Use `#[serde(default)]`. Its ten optional fields follow this contract:

| Fields | Type |
|---|---|
| `entity_type`, `entity_id`, `url`, `mime_type`, `language`, `rel`, `source`, `extraction_path` | `Option<String>` |
| `position`, `observed_at` | `Option<i64>` |

Skip absent owner, position, and observation fields on serialization, as `SourceEnclosure` does.
Keep normal serialization for the other six fields, including explicit null values.
Preserve unknown owner strings and supplied positions. Do not infer the owner from the request.
Do not copy feed data into this collection.

## Files And Constraints

Inspect `src/api.rs`, the upstream response type, and the [field inventory](../../schema/adr-0075-metadata-field-inventory.md).
The owning contract is [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md), Decisions 1–3.
Change `src/api.rs` and required existing `Track` constructor sites only.
Preserve all uncommitted packet 030 changes.
Do not change requests, database storage, selection, playback, UI, or upstream files.

## Implementation And Acceptance

1. Add the collection and its member type.
2. Update constructors only when compilation requires it. Use unknown values, not invented facts.
3. Add tests in the existing test module, named with `adr_0075_transcript_transport`.
4. Check all ten fields through `Track` decoding, serialization, and a second decode.
5. Test missing, null, and empty collections separately.
6. Test unknown owners, repeated URLs, and nonsequential positions without merging occurrences.
7. Test older members without provenance and reject strings for integer fields.
8. Confirm that `track_with_feed_defaults` adds no transcript facts.

## Checks And Rollback

```bash
cargo test --locked --offline --lib adr_0075_transcript_transport
cargo fmt -- --check
cargo check --locked --offline
```

The orchestrator runs the full suite, strict Clippy, and the normal binary build after integration.
Rollback removes these DTO additions and their constructor changes. No migration belongs to this packet.
Stop if this transport correction requires a new source-selection or storage policy.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, ADR 0075, this packet, and `src/api.rs`.
- `/home/citizen/.agents/skills/rust-skills/rust-dev/SKILL.md`.
- `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.

Goal:
- Preserve the existing transcript wire contract described above.

Constraints:
- Retain unknown provenance and collection presence correctly.
- Preserve existing packet 030 work. Do not commit.

Do not touch:
- Storage, requests, selection, UI, playback, and upstream files.
- Shared status documents, which the orchestrator owns.

Acceptance criteria:
- The tests above prove the transport contract through the parent `Track` type.

Test commands:
- Run the commands in Checks And Rollback.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

This transport change needs no visual acceptance. Existing visual gates remain paused.
