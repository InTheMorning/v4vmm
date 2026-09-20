# ADR 0075 Task 030: Preserve Enclosure Claim Transport

Status: Draft - 2026-09-19. Dispatch is held. Implementation has not started.
The packet needs technical review and explicit release before implementation.
The operator's authorization to correct documents does not release this code packet.

## Goal

Preserve the four enclosure claim fields that the inspected Index already supplies.
Keep older payloads readable. Do not invent provenance for locally constructed enclosures.

## Inputs And Evidence

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), Decisions 1 and 3.
- [Field inventory](../schema/adr-0075-metadata-field-inventory.md), source collections.
- [Packet 001](adr-0075-task-001-contributor-claim-transport.md), the equivalent contributor transport boundary.
- Upstream `src/query.rs::SourceItemEnclosureResponse`, at Stophammer commit `a220f44`.
- The corrected [Stophammer request](../plans/adr-0075-stophammer-decision-request.md#2-required-fields-and-collections).

The upstream type already contains `entity_type`, `entity_id`, `position`, and `observed_at`.
The app's `SourceEnclosure` omits them. No upstream API addition is needed for this packet.
The deployed revision remains unverified. This packet makes no claim about live payloads.

## Files To Inspect

- `AGENTS.md`
- `src/api.rs`, `SourceEnclosure`, `Track`, and transport tests.
- `src/track_compare.rs`, enclosure selection and its fixtures.
- `src/view_models/track.rs`, enclosure actions and their fixtures.
- Each `SourceEnclosure` constructor found with `rg`.

## Files Likely To Change

- `src/api.rs`, the DTO and its unit tests.
- Existing constructor sites, only if the new fields require initialization.
- This packet's implementation result after authorized implementation.

## Field Contract

| Field | Rust type | JSON input | JSON output |
|---|---|---|---|
| `entity_type` | `Option<String>` | String, missing, or null | Preserve a supplied string. Omit an absent value |
| `entity_id` | `Option<String>` | String, missing, or null | Preserve a supplied string. Omit an absent value |
| `position` | `Option<i64>` | Integer, missing, or null | Preserve a supplied integer. Omit an absent value |
| `observed_at` | `Option<i64>` | Integer, missing, or null | Preserve a supplied integer. Omit an absent value |

Retain `#[serde(default)]`. Use `skip_serializing_if = "Option::is_none"` for each new field.
Preserve each existing field's serialization behavior.
Keep an unknown `entity_type` string unchanged. This transport type does not validate ownership.

## Constructed Fixture

This fixture represents the inspected wire type. It is not a captured live response.

```json
{
  "entity_type": "track",
  "entity_id": "track-a",
  "position": 7,
  "url": "https://example.test/audio.mp3",
  "mime_type": "audio/mpeg",
  "bytes": 12345,
  "rel": null,
  "title": "Primary audio",
  "is_primary": true,
  "source": "rss_enclosure",
  "extraction_path": "item.enclosure",
  "observed_at": 1779240280
}
```

The source and extraction path are synthetic fixture strings. Preserve them without interpreting them.

## Do Not Touch

- Database schema, migrations, persistence, or repair.
- Enclosure selection, supported formats, playback, download, or conversion behavior.
- Request profiles, caching, or response completeness rules.
- UI fields, actions, or presentation.
- Stophammer code, its API contract, or generated API files.

## Implementation Steps

1. Add the four optional fields to the existing DTO.
2. Set unknown provenance to `None` at required local constructor sites.
3. Add tests for the field contract and constructed fixture.
4. Run the checks below.
5. Report transport preservation separately from storage and display preservation.

## Acceptance Criteria

### Mechanical Criteria

- A test decodes and serializes the fixture without losing supplied fields.
- A test preserves positions `7`, `2`, and `19` independently of array order.
- A test preserves an unknown owner type without substituting the request owner.
- Tests decode older payloads and explicit nulls without adding provenance fields to output.
- Tests reject string values for integer fields.
- Existing selection tests remain Green without changed selection expectations.
- The required build, test, lint, and format checks are Green.

### Review Criteria

The reviewer checks the four types against the inspected upstream response.
The reviewer confirms that constructor changes invent no owner, position, or observation time.
No source-preservation claim extends beyond transport in this packet.

## Checks

After code dispatch, run these commands from the repository root:

```bash
cargo test --locked --offline --lib adr_0075_enclosure_transport
cargo test --locked --offline
cargo check --locked --offline
cargo fmt -- --check
cargo clippy --locked --offline -- -D warnings
cargo build --locked --offline --bin v4vmm
```

For this document, run the shared link and language checks:

```bash
python3 docs/runbooks/check-markdown-links.py docs/tasks/adr-0075-task-030-enclosure-claim-transport.md
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" --check --no-heuristics docs/tasks/adr-0075-task-030-enclosure-claim-transport.md
```

## Escalation Triggers

Stop if the transport fix requires storage, selection, UI, or upstream contract changes.
Revise the packet before implementation continues.

## Rollback

Revert the DTO additions with their required constructor changes. No database migration belongs to this packet.

## Expected Report

Report changed files, checks, deviations, and remaining storage or display losses.
Keep missing deployment evidence separate from local transport test results.

## Operator Visual Check

This transport packet requires no visual acceptance. The existing visual pause and inherited gates remain in force.
