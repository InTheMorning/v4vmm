# ADR 0075 Task 001: Preserve Contributor Claim Transport Fields

Status: Draft - 2026-09-19. Implementation has not started.
The parent ADR is Proposed. Execute this packet after its transport decision becomes binding.
This packet preserves contributor fields when the app decodes and serialises API data.

## Goal

Preserve every documented field in MusicIndex contributor claims when decoding and serialising `api::Contributor`.
Keep older payloads readable. This packet does not complete provenance handling
in storage or display code. Provenance records a value's owner, source,
extraction path and observation time.

## Files To Inspect

- `AGENTS.md`, `.github/copilot-instructions.md`
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md)
- [Plan](../plans/adr-0075-metadata-contract-phase-plan.md)
- [Review](../reviews/adr-0075-metadata-contract-review.md)
- `src/api.rs`, especially `Contributor` and API fixture tests
- `src/views.rs`, `src/identity_ingest.rs`, `src/local_identity.rs`
- `src/feed_service.rs`, `src/rss/subscribe.rs`, `src/metadata.rs`
- Stophammer `src/query.rs::SourceContributorClaimResponse`, read-only

## Files Likely To Change

`src/api.rs` owns the data transfer object (DTO) and its focused tests.
The DTO is the Rust type that represents API data.
Update `Contributor` literals in direct callers only where compilation requires a change.
Record those files in the implementation report.
Keep unknown provenance unchanged when local code constructs a contributor.

Update this packet, its review and plan with the actual test results.

## Do Not Touch

- SQLite schema, migrations or production databases
- HTTP include profiles, request scheduling or source selection
- Contributor grouping, renderers, action availability or label placement
- RSS extraction and existing record repair
- Audio tags, playback, broadcast services or configuration
- Stophammer or generated MusicIndex API files

## Constraints

Retain the existing name, role, group, href, image and npub fields.
Add optional `entity_type`, `entity_id`, `position`, `role_norm`, `source`,
`extraction_path` and `observed_at` fields using their documented wire names.
Use `Option<i64>` for position and observation time. Fields missing from older
payloads remain `None`. Do not add invented values for absent fields when serialising payloads.

Do not add a duplicate contributor DTO. Do not infer the owner from the request URL.
Do not replace a supplied position with the enumeration index.
Do not replace a supplied observation time with the current time.

Later packets must address limitations in `ContributorView` and the local schema.
Do not expand this task to resolve those limitations while correcting compilation errors.

## Implementation Steps

1. Check that the parent's transport decision is binding.
2. Read the current API type.
3. Add the seven optional fields to the existing DTO with compatible Serde behavior.
4. Update affected literals without changing their existing values or meaning.
5. Add tests using the operator's supplied response with three credits as a fixture.
6. Add tests for older payloads and contributors owned by a feed to prevent ownership inference.
7. Run the focused tests.
8. Run the required repository checks.
9. Record the fields that later layers still lose in the review.

## Acceptance Criteria

Unit tests beside `api::Contributor` must check these results:

- Deserialisation retains all documented claim fields from the supplied response.
- Serialisation and a second decode preserve those values and the order of the three credits.
- Two HeyCitizen credits retain their different roles and original positions.
- A claim owned by a feed keeps `entity_type: feed` when a track response returns it.
- An older contributor payload with six fields decodes with unknown provenance.
- Missing optional fields do not become fabricated values during serialisation.
- Existing `href`, `img` and `npub` values remain unchanged.

This packet changes transport only. It requires no visual acceptance.
It closes no gate under ADR 0037 or ADR 0054. Visual checks remain paused.

## Test Commands

```bash
cargo test --locked --offline --lib adr_0075_contributor_transport
cargo test --locked --offline
cargo check --locked --offline
cargo fmt -- --check
cargo clippy --locked --offline -- -D warnings
cargo build --locked --offline --bin v4vmm
```

Give the new tests the `adr_0075_contributor_transport` prefix. Follow repository escalation rules
if a required check encounters a sandbox restriction. Do not launch the app.

## Rollback

Revert this packet's DTO changes together with its required constructor changes.
This packet includes no database migration or fixture change.

## Escalation Triggers

Stop if the implementation needs any of these changes:

- A schema change.
- A new field in the upstream API.
- A rule for source selection.
- A presentation change.

Revise this packet before continuing. Missing API provenance remains unknown.
Do not invent values to replace it.

## Expected Report

Name files changed, tests run, behavior changed, deviations and unresolved work.
State separately whether transport, storage and display preserve the fields.
Report passing checks as Green. Record failures with their cause and consequence.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:

- This packet and every file in its Files To Inspect section.

Goal:

- Preserve the documented contributor claim fields in the existing API DTO.

Constraints:

- Apply this packet's Constraints and Implementation Steps.
- Preserve unknown provenance for older and local payloads.

Do not touch:

- Every boundary listed in this packet's Do Not Touch section.

Acceptance criteria:

- Pass every mechanical criterion listed above.
- Leave storage, display preparation and visual acceptance open.

Test commands:

- Run the commands in this packet's Test Commands section.

At the end, report:

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
