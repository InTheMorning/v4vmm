# ADR 0075 Task 017: Named Request Profiles

Status: Ready - 2026-09-21. Implementation has not started.
This packet changes no request, no include list, and no user-visible behavior.
It needs no visual acceptance.

## Goal

Give each metadata request that the Library track detail route and the Index detail route
makes one named, typed profile. A profile names the path, the include list, and the
fallback position of one request.

Preserve every existing request, include string, fallback order, and request count exactly.
Packet 018 needs a stable name for each request before it can hold a cache key.

Under [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I, a profile
names what the app asks a cache for. It models no provider ownership, because a provider is
transport evidence and not a source.

## Why This Packet Exists

Include lists are literals in six owners today, and two owners repeat the same string:

| Owner | Value | In scope |
|---|---|---|
| `src/feed_service.rs:67` | `source_links,source_ids,source_release_claims,source_contributors,payment_routes` | Yes |
| `src/feed_service.rs:309` | The same string, repeated | Yes |
| `src/application/queries/search.rs:737` | `tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes` | Yes |
| `src/subscribe_service.rs:26` | `source_enclosures,source_links,source_ids,source_contributors,payment_routes` | No. No packet owns it |
| `src/application/commands/payment_routes.rs:28` | `payment_routes` | No. No packet owns it |
| `src/application/commands/feed.rs:1006` | A pre-encoded `?include=` query string | No. Packet 039 completed that caller |

The feed update owner encodes its include as a percent-encoded query string rather than a
parameter list. That is a third representation of one concept.

## Recorded Finding, Not A Change

The two routes in scope do not request the same collections.

| Route | Collections requested |
|---|---|
| Library track detail | Five. `source_links`, `source_ids`, `source_release_claims`, `source_contributors`, `payment_routes` |
| Index feed detail | Seven. The five above, plus `tracks` and `source_enclosures` |

No route requests `source_transcripts`. Packet 032 preserved transcript transport for a
collection that no route asks for.

This packet records these facts. It does not correct them. A change to what the app
requests changes cost, and it needs its own measurement and its own decision.

## Authority And Dependencies

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), Decision I and the
  reduced Decision A.
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md#the-committed-path), the
  committed path and both accepted dependency cuts.
- [Packet 013](adr-0075-task-013-verified-snapshot-replacement.md), the completeness
  registry and typed local reads.
- [Packet 014](adr-0075-task-014-provider-observation-retention.md), the shared recorder,
  writer, and receipts.
- [Request baseline](../notes/adr-0075-request-and-write-baseline.md), the measured request
  counts that this packet must not change.

The operator accepted both dependency cuts on 2026-09-21. This packet does not need packet
008, and it does not need packets 038 to 044. Packets 040 to 044 are deleted, so no packet
plans to convert the remaining caller families.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- [Feed service](../../src/feed_service.rs), the Library detail fetch, its fallback order,
  and the staleness fetch.
- [Library queries](../../src/application/queries/library.rs),
  `fetch_library_track_context_with_local_fallback` and its request assembly.
- [Search queries](../../src/application/queries/search.rs), `INDEX_FEED_DETAIL_INCLUDE`
  and its call sites.
- [Feed queries](../../src/application/queries/feed.rs), the two Index detail call sites.
- [API client](../../src/api.rs), `fetch_feed`, `fetch_track`, `fetch_feed_track`, and
  their parameter construction.
- [Observation types](../../src/provider_observation.rs), `ProviderRequestSpec` and its
  existing profile JSON.
- [Architecture guards](../../tests/architecture_tests.rs), the existing ADR 0075 guards.

## Files To Change

| File | Permitted change |
| --- | --- |
| New module under `src/application/` | The profile registry, its typed identity, and its tests |
| `src/feed_service.rs` | Read both include values from the registry. Keep the requests and the fallback order |
| `src/application/queries/search.rs` | Replace the literal constant with a registry profile |
| `src/application/queries/feed.rs` | Use the registry profile at both call sites |
| `src/api.rs` | Accept a profile where it accepts an include string today. Keep the wire format |
| `tests/architecture_tests.rs` | One situational guard, naming ADR 0075 |

Do not change `src/subscribe_service.rs`, `src/application/commands/payment_routes.rs`, or
the feed update constant. They stay outside this packet, and no packet owns changing them.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_request_profile_` for behavioral tests beside the owning code.

| Case | Required proof |
| --- | --- |
| R17-01 | The registry exposes one named profile for each request that the two routes make. Each profile names its path, its include list, and its fallback position |
| R17-02 | Each profile's include string equals the literal it replaced, character for character. The test compares against the recorded literal in this packet |
| R17-03 | The Library track detail route issues the same three requests, in the same order, with the same fallback, as the baseline records |
| R17-04 | The Index feed detail route issues the same request with the same include list at both call sites |
| R17-05 | No profile type carries a provider ownership field. A guard fails when one appears, and names ADR 0075 Decision I |
| R17-06 | Each profile exposes a stable identity. Equal requests produce equal identities, and different include lists produce different identities |
| R17-07 | `ProviderRequestSpec` values and their profile JSON equal the values recorded before this packet. Retained evidence is unchanged |
| R17-08 | A guard fails when a new literal include string appears in the two in-scope routes. The guard names the three out-of-scope literals as permanent exceptions, until a later decision changes them |
| R17-09 | The registry reports five collections for the Library route and seven for the Index route. The test names them |
| R17-10 | No profile requests `source_transcripts`. A later addition must change this test deliberately |

R17-06 exists for packet 018. A profile identity is the value that a cache key will hold.
This packet adds no cache and no expiry.

## Exclusions

- No change to any include list, request, path, fallback, or request count.
- No cache, no expiry, no refresh command. Packet 018 owns those.
- No field policy, no projection, and no display change.
- No change to the three out-of-scope include owners.
- No new product policy. This packet proposes none.

## Checks

```bash
cargo test --lib adr_0075_request_profile
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

The packet touches one new module and five call sites. Revert the working tree to remove
it. No migration, no stored data, and no configuration changes.

## Operator Visual Check

None. This packet changes no user-visible behavior, and it requires no visual acceptance.

The inherited presentation gates for packets 012, 014, 038, and 039 stay open and paused.
Do not request a visual batch for this packet.
