# ADR 0075 Task 001: Preserve Contributor Claim Transport Fields

Status: Complete - 2026-09-19. All mechanical checks are Green.
The parent ADR 0075 is Accepted from 2026-09-19. This packet's content review is Green.
The operator released the dispatch on 2026-09-19. Document packets 002 to 008 exist.

This packet changes transport only. It needs no operator visual acceptance.
The [implementation result](#implementation-result) records the changed files and the remaining losses.

This packet preserves contributor fields when the app decodes and serializes API data.

## Goal

Preserve every documented field in MusicIndex contributor claims when decoding and serializing `api::Contributor`.
Keep older payloads readable. This packet does not complete provenance handling
in storage or display code. Provenance records a value's owner, source,
extraction path and observation time.

## Files To Inspect

- `AGENTS.md`, `docs/architecture/source-map.md`
- [ADR status rules](../../adr/0057-adr-status-vocabulary-and-amendment-policy.md)
- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md)
- [Plan](../../plans/adr-0075-metadata-contract-phase-plan.md)
- [Review](../../reviews/adr-0075-metadata-contract-review.md)
- `src/api.rs`, especially `Contributor` and API fixture tests
- `src/views.rs`, `src/identity_ingest.rs`, `src/local_identity.rs`
- `src/feed_service.rs`, `src/rss/subscribe.rs`, `src/metadata.rs`
- The [supplied response](#supplied-response) and [field contract](#field-contract) in this packet

The upstream reference is `/home/citizen/build/stophammer/src/query.rs::SourceContributorClaimResponse`
at commit `a220f44`. Inspect it read-only when available.
The field contract below lets an agent work without that checkout or a live endpoint.

## Files Likely To Change

`src/api.rs` owns the data transfer object (DTO) and its focused tests.
The DTO is the Rust type that represents API data.
The current production literals that need new defaults are:

- `src/views.rs::From<ContributorView> for api::Contributor`.
- `src/feed_service.rs::contributor_from_local`.

Update other `Contributor` literals only where compilation requires a change.
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
Add the seven fields in the field contract below.
Retain the existing struct derives and `#[serde(default)]` behavior.
Apply `#[serde(skip_serializing_if = "Option::is_none")]` to each new field only.
Do not change serialization of the six existing fields.

Do not add a duplicate contributor DTO. Do not infer the owner from the request URL.
Do not replace a supplied position with the enumeration index.
Do not replace a supplied observation time with the current time.
Do not trim, normalize, validate identity syntax or infer missing fields in this DTO.
Do not add an unknown-field map or new dependencies.

The existing `persist_contributors` helper serializes this DTO into `raw_json`.
Newly fetched records can therefore retain the added fields in that JSON without a schema change.
This packet does not change typed database columns or recover fields from old JSON.
Do not describe this change as complete storage preservation.

Later packets must address limitations in `ContributorView` and the local schema.
Do not expand this task to resolve those limitations while correcting compilation errors.

## Field Contract

These fields match `SourceContributorClaimResponse` at the inspected upstream revision.
The app uses optional fields to accept older payloads.

| JSON field | App type | Meaning |
|---|---|---|
| `entity_type` | `Option<String>` | Declared owner of the credit, such as `feed` or `track` |
| `entity_id` | `Option<String>` | Declared owner's identifier. It is not a global contributor identifier |
| `position` | `Option<i64>` | Supplied position within the source collection |
| `role_norm` | `Option<String>` | Supplied normalized role, separate from the original `role` |
| `source` | `Option<String>` | Supplied assertion source |
| `extraction_path` | `Option<String>` | Supplied source path |
| `observed_at` | `Option<i64>` | Supplied observation time. Preserve the integer unchanged |

Missing fields and explicit JSON `null` decode as `None`.
Serialization omits new fields whose value is `None`.
This task does not preserve the distinction between an absent field and an explicit null in one contributor.
Collection coverage remains a separate later task.
Unknown string values remain unchanged. A value with the wrong JSON type remains a decoding error.

## Supplied Response

The operator supplied this selected response on 2026-09-19.
It is not the full HTTP response. Decode its `source_contributors` array as `Vec<Contributor>` in the tests.
Keep the three credits and their values unchanged in the base fixture.

```json
{
  "title": "MoeFactz",
  "source_links": [],
  "source_ids": [],
  "source_contributors": [
    {
      "entity_type": "track",
      "entity_id": "d489101a-4e62-492f-812e-9fe51def9423",
      "position": 0,
      "name": "HeyCitizen",
      "role": "musician",
      "role_norm": "musician",
      "group_name": "music",
      "href": null,
      "img": "https://files.heycitizen.xyz/Songs/HeyCitizen.jpg",
      "npub": "npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck",
      "source": "podcast_person",
      "extraction_path": "track.podcast:person",
      "observed_at": 1779240280
    },
    {
      "entity_type": "track",
      "entity_id": "d489101a-4e62-492f-812e-9fe51def9423",
      "position": 1,
      "name": "HeyCitizen",
      "role": "audio engineer",
      "role_norm": "audio engineer",
      "group_name": "audio-production",
      "href": null,
      "img": "https://files.heycitizen.xyz/Songs/HeyCitizen.jpg",
      "npub": "npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck",
      "source": "podcast_person",
      "extraction_path": "track.podcast:person",
      "observed_at": 1779240280
    },
    {
      "entity_type": "track",
      "entity_id": "d489101a-4e62-492f-812e-9fe51def9423",
      "position": 2,
      "name": "Moe Factz",
      "role": "host",
      "role_norm": "host",
      "group_name": "cast",
      "href": "https://www.moefactz.com/",
      "img": null,
      "npub": null,
      "source": "podcast_person",
      "extraction_path": "track.podcast:person",
      "observed_at": 1779240280
    }
  ]
}
```

## Implementation Steps

1. Check that ADR 0075 is Accepted and this packet is Ready.
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

- Deserialization retains all thirteen fields in each supplied contributor object.
- Serialization matches each original contributor object as a JSON value, independent of object-key order.
- A second decode preserves those values and the order of the three credits.
- Two HeyCitizen credits retain their different roles and original positions.
- A derived `Track` fixture keeps a feed credit's declared `entity_type` and `entity_id` through a round trip.
- A derived fixture with positions `7`, `2`, `19` retains those positions and its input order.
- An older payload with six fields decodes with all seven new fields set to `None`.
- Serialising that older payload does not add any of the seven new keys.
- Explicit null values in the seven new fields decode as `None` and are omitted during serialization.
- A derived fixture retains an unknown `entity_type` string and different `role` and `role_norm` values.
- A string in `position` or `observed_at` produces a decoding error.
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
The focused command must execute tests. A successful command that matches zero tests is not Green.

## Implementation Result

Completed on 2026-09-19.

### Files Changed

| File | Change |
|---|---|
| `src/api.rs` | `Contributor` holds the seven claim fields. The file holds eleven new tests with the `adr_0075_contributor_transport` prefix and the supplied response as a fixture |
| `src/views.rs` | `From<ContributorView> for api::Contributor` sets the seven fields to `None`. One existing test literal uses `..Default::default()` |
| `src/feed_service.rs` | `contributor_from_local` sets the seven fields to `None` |

The compiler required the change in the `src/views.rs` test literal. No other literal required a change.
The other literals already use `..Default::default()`.

### Behavior Changed

Deserialization keeps `entity_type`, `entity_id`, `position`, `role_norm`, `source`,
`extraction_path` and `observed_at` from a MusicIndex contributor claim.
Serialization writes each of these fields only when the app holds a value.
The six existing fields keep their previous serialization.
An older payload and a locally constructed credit keep unknown provenance.

### Preservation By Layer

| Layer | Result |
|---|---|
| Transport | The DTO preserves all thirteen fields. Tests check the round trip |
| Storage | `src/identity_ingest.rs::persist_contributors` serializes the expanded DTO, so the `raw_json` column of a newly fetched record holds the seven fields. The typed columns in `src/db.rs::LocalContributorRow` hold six fields, a position from the list index, and no supplied observation time. Old rows keep their old JSON |
| Display | `src/views.rs::ContributorView` holds six fields. `contributor_from_local` returns `None` provenance. The display route loses all seven fields |

Storage and display preparation remain open for later packets. This packet does not complete them.

### Checks

| Command | Result |
|---|---|
| `cargo test --locked --offline --lib adr_0075_contributor_transport` | Green. 11 tests passed |
| `cargo test --locked --offline` | Green. 1513 unit tests, 263 architecture tests |
| `cargo check --locked --offline` | Green |
| `cargo fmt -- --check` | Green |
| `cargo clippy --locked --offline -- -D warnings` | Green |
| `cargo build --locked --offline --bin v4vmm` | Green |

`cargo clippy --all-targets` reports 55 existing errors in test code of other modules.
This packet changed no file in that list. The repository does not require that command.

### Deviations

None. The packet triggered no escalation. No schema, request, or presentation change was necessary.

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
