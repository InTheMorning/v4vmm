# ADR 0075 Task 017: Named Request Profiles

Status: Complete - 2026-09-21. Implementation, technical review, and mechanical checks
are Green. The orchestrator corrected the recorded inventory on the same day, after a source
inspection.
This packet changes no request, no include list, and no user-visible behavior.
It needs no visual acceptance.

## Goal

Give each metadata request that the Library route and the Index route makes one named,
typed profile. A profile names the path shape, the include list, and the position of one
request in its route.

Preserve every existing request, include string, fallback order, and request count exactly.
Packet 018 needs a stable name for each request before it can hold a cache key.

Under [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md) Decision I, a profile
names what the app asks a cache for. It models no provider ownership, because a provider is
transport evidence and not a source.

## Why This Packet Exists

Include lists are literals at eight sites in five files. Three files hold five more literals
that no packet owns.

In scope:

| Site | Literal | Collections |
|---|---|---|
| `src/feed_service.rs:68` | L1 | 5 |
| `src/feed_service.rs:310` | L1 | 5 |
| `src/application/queries/search.rs:737` | L2, as `INDEX_FEED_DETAIL_INCLUDE` | 7 |
| `src/application/queries/search.rs:752` | L0, the scoped and unscoped track detail | 0 |
| `src/application/queries/feed.rs:312` | L2, through the constant | 7 |
| `src/application/queries/feed.rs:453` | L2, through the constant | 7 |
| `src/application/queries/feed.rs:477` | L3 | 6 |
| `src/application/queries/feed.rs:485` | L4 | 6 |
| `src/application/queries/library.rs:549` | L5 | 4 |

Out of scope. No packet owns these sites, and this packet does not change them:

| Site | Literal |
|---|---|
| `src/subscribe_service.rs:211` | L3 |
| `src/subscribe_service.rs:619` | L3 |
| `src/subscribe_service.rs:626` | `tracks,source_enclosures,source_links,source_ids,source_release_claims` |
| `src/application/commands/payment_routes.rs:28` | `payment_routes`, as `PAYMENT_ROUTES_INCLUDE` |
| `src/application/commands/feed.rs:1007` | A pre-encoded `?include=` query string |

The feed update command encodes its include as a percent-encoded query string rather than a
parameter list. That is a third representation of one concept. Packet 039 completed that
caller, and this packet leaves it unchanged.

## The Recorded Literals

The implementer compares each profile against these strings, character for character.

| Name | String |
|---|---|
| L0 | No `include` parameter |
| L1 | `source_links,source_ids,source_release_claims,source_contributors,payment_routes` |
| L2 | `tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes` |
| L3 | `source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes` |
| L4 | `tracks,source_enclosures,source_links,source_ids,source_release_claims,payment_routes` |
| L5 | `source_links,source_ids,source_release_claims,source_contributors` |

## The Required Profiles

| Profile | Owner | Path shape | Include | Position |
|---|---|---|---|---|
| Library track detail, scoped track | `feed_service::fetch_library_track_detail_with_recorder` | `/v1/feeds/{feed_guid}/tracks/{item_guid}` | L1 | First. Only when the row has a feed GUID |
| Library track detail, unscoped track | The same function | `/v1/tracks/{item_guid}` | L1 | Second. Only when the first request returns no track |
| Library track detail, feed | The same function | `/v1/feeds/{feed_guid}` | L1 | Third |
| Library feed update, feed | `feed_service::apply_feed_updates` | `/v1/feeds/{feed_guid}` | L1 | Only request of that function |
| Library album hydration, feed | `library::hydrate_album_identity_facts` | `/v1/feeds/{feed_guid}` | L5 | Only request |
| Index feed detail | `search::fetch_index_feed_result_rows`, `feed::fetch_recent_feed_result_rows`, `feed::fetch_feed_detail` | `/v1/feeds/{feed_guid}` | L2 | Only request of each call site |
| Index track detail, scoped | `search::fetch_index_track_detail` | `/v1/feeds/{feed_guid}/tracks/{track_guid}` | L0 | Used when the hit has a nonempty feed GUID |
| Index track detail, unscoped | The same function | `/v1/tracks/{track_guid}` | L0 | Used when the hit has no feed GUID |
| Inspector track detail, track | `feed::fetch_track_detail`, through `fetch_scoped_track` | Scoped or unscoped, by the same condition | L3 | First |
| Inspector track detail, feed | `feed::fetch_track_detail` | `/v1/feeds/{feed_guid}` | L4 | Second. A failure returns no feed and stops no request |

The Library feed update calls the Library track detail requests for each stored track. Those
requests keep their own three profiles.

## Recorded Findings, Not Changes

The routes do not request the same collections. Five different include lists exist for
requests that read the same resources.

No route requests `source_transcripts`. Packet 032 preserved transcript transport for a
collection that no route asks for.

The Index track detail requests send no include parameter. Their responses carry no source
collection.

This packet records these facts. It does not correct them. A change to what the app requests
changes cost, and it needs its own measurement and its own decision.

## Authority And Dependencies

- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md), Decision I and the
  reduced Decision A.
- [Phase plan](../../plans/adr-0075-metadata-contract-phase-plan.md#the-committed-path), the
  committed path and both accepted dependency cuts.
- [Packet 013](../adr-0075-task-013-verified-snapshot-replacement.md), the completeness
  registry and typed local reads.
- [Packet 014](../adr-0075-task-014-provider-observation-retention.md), the shared recorder,
  writer, and receipts.
- [Request baseline](../../notes/adr-0075-request-and-write-baseline.md), the measured request
  counts that this packet must not change.

The operator accepted both dependency cuts on 2026-09-21. This packet does not need packet
008, and it does not need packets 038 to 044. Packets 040 to 044 are deleted, so no packet
plans to convert the remaining caller families.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../../.github/copilot-instructions.md).
- [Feed service](../../../src/feed_service.rs), the Library detail fetch, its fallback order,
  and the feed update fetch.
- [Library queries](../../../src/application/queries/library.rs),
  `hydrate_album_identity_facts` and `compare_library_track`.
- [Search queries](../../../src/application/queries/search.rs), `INDEX_FEED_DETAIL_INCLUDE`,
  `fetch_index_feed_result_rows` and `fetch_index_track_detail`.
- [Feed queries](../../../src/application/queries/feed.rs), the four request sites and
  `fetch_scoped_track`.
- [API client](../../../src/api.rs), `fetch_feed`, `fetch_track`, `fetch_feed_track`, and
  their parameter construction.
- [Observation types](../../../src/provider_observation.rs), `ProviderRequestSpec` and its
  existing profile JSON.
- [Architecture guards](../../../tests/architecture_tests.rs), the existing ADR 0075 guards.

## Files To Change

| File | Permitted change |
| --- | --- |
| New module under `src/application/` | The profile registry, its typed identity, and its tests |
| `src/feed_service.rs` | Read both include values from the registry. Keep the requests and the fallback order |
| `src/application/queries/search.rs` | Replace the literal constant with a registry profile. Name the two track detail requests |
| `src/application/queries/feed.rs` | Use registry profiles at the four request sites |
| `src/application/queries/library.rs` | Use the registry profile in `hydrate_album_identity_facts` |
| `src/api.rs` | Accept a profile where it accepts an include string today. Keep the wire format |
| `src/application/mod.rs` | One line. Register the new module |
| `tests/architecture_tests.rs` | One situational guard, naming ADR 0075. Keep each existing guard on the same rule |

Do not change `src/subscribe_service.rs`, `src/application/commands/payment_routes.rs`, or
`src/application/commands/feed.rs`. They stay outside this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_request_profile_` for behavioral tests beside the owning code.

| Case | Required proof |
| --- | --- |
| R17-01 | The registry exposes one named profile for each row of the profile table. Each profile names its path shape and its include list. A doc comment records its position, and R17-04 to R17-07 prove that position at its call site |
| R17-02 | Each profile include string equals its recorded literal, character for character. The test compares against the literal table above |
| R17-03 | An L0 profile sends no `include` query parameter. The recorded request path equals the path before this packet |
| R17-04 | The Library track detail route issues the same three requests, in the same order, with the same fallback, as the baseline records |
| R17-05 | The Index feed detail profile serves its three call sites, and each one sends L2 |
| R17-06 | The inspector track detail route sends L3 for its track request and L4 for its feed request, in that order |
| R17-07 | The Library album hydration request sends L5, and the Library feed update request sends L1 |
| R17-08 | No profile type carries a provider ownership field. A guard fails when one appears, and names ADR 0075 Decision I |
| R17-09 | Each profile exposes a stable identity. Equal requests produce equal identities, and different include lists produce different identities |
| R17-10 | `ProviderRequestSpec` values and their profile JSON equal the values recorded before this packet. Retained evidence is unchanged |
| R17-11 | A guard fails when a new include literal appears in the five in-scope files. The guard names the three out-of-scope files as permanent exceptions, until a later decision changes them |
| R17-12 | The registry reports the collection count of each profile. The test names 5, 7, 6, 6, 4, and 0 for L1 to L5 and L0 |
| R17-13 | No profile requests `source_transcripts`. A later addition must change this test deliberately |

R17-09 exists for packet 018. A profile identity is the value that a cache key will hold.
This packet adds no cache and no expiry.

## Exclusions

- No change to any include list, request, path, fallback, or request count.
- No cache, no expiry, no refresh command. Packet 018 owns those.
- No field policy, no projection, and no display change.
- No change to the three out-of-scope files.
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

The packet touches one new module and the request sites in five files. Revert the working
tree to remove it. No migration, no stored data, and no configuration changes.

## Implementation Result - 2026-09-21

`src/application/request_profiles.rs` holds the ten named profiles. A profile carries its
path shape and its include list. It carries no provider ownership field.

A profile value is its own identity. The type is `Copy`, `Eq`, and `Hash`, so packet 018 can
put a profile value in a cache key. Two profiles that request the same path shape with the
same include list are one identity. The Library track detail feed request and the Library
feed update request are one identity for that reason.

`src/api.rs` gained three wrapper methods: `fetch_feed_with_profile`,
`fetch_track_with_profile`, and `fetch_feed_track_with_profile`. Each one reads the include
list from the profile and calls the existing method. The three existing methods keep their
signatures, and their other callers stay unchanged. Each wrapper asserts the path shape of
its profile in a debug build.

### Converted Request Sites

| File | Function | Profile |
|---|---|---|
| `src/feed_service.rs` | `fetch_library_track_detail_with_recorder` | The three Library track detail profiles |
| `src/feed_service.rs` | `apply_feed_updates` | `LIBRARY_FEED_UPDATE_FEED` |
| `src/application/queries/search.rs` | `fetch_index_feed_result_rows` | `INDEX_FEED_DETAIL` |
| `src/application/queries/search.rs` | `fetch_index_track_detail` | The two Index track detail profiles |
| `src/application/queries/feed.rs` | `fetch_recent_feed_result_rows` | `INDEX_FEED_DETAIL` |
| `src/application/queries/feed.rs` | `fetch_feed_detail` | `INDEX_FEED_DETAIL` |
| `src/application/queries/feed.rs` | `fetch_track_detail` | The two inspector track detail profiles |
| `src/application/queries/library.rs` | `hydrate_album_identity_facts` | `LIBRARY_ALBUM_HYDRATION_FEED` |

The `INDEX_FEED_DETAIL_INCLUDE` constant is deleted. The `fetch_scoped_track` helper keeps
its signature, because an out-of-scope caller also uses it.

### Position Is A Doc Comment, Not A Field

The registry records each position in a doc comment. A stored position field has no
production reader today, and a value that only a test reads is dead code under strict Clippy.
R17-04 to R17-07 prove the order and the fallback at each owning call site. That layer owns
the fact.

### Guards That Moved With The Code

Four existing guards matched the exact text of a converted call site. Each one now matches
the new text and checks the same rule as before:

- `adr_0049_inspector_source_ownership_is_guarded`
- `source_fact_placeholder_and_breadcrumb_regressions_are_guarded`
- `adr_0075_feed_observation_roots_and_consumers_are_guarded`
- `adr_0075_library_observation_callers_and_consumers_are_guarded`

### A Repaired Guard, Found During Verification

`source_fact_placeholder_and_breadcrumb_regressions_are_guarded` also failed on a clean tree
before this packet started. It asserted one exact sentence of `AGENTS.md`, and the ADR 0075
Decision I amendment rewrapped that sentence across two lines. The guard now collapses the
whitespace of `AGENTS.md` before it compares, so a later rewrap cannot break it again. Its
failure message names Decision I and the repair. This defect belongs to the amendment, not to
packet 017. The orchestrator repaired it because it held `cargo test` red.

### Checks - 2026-09-21

| Check | Result |
|---|---|
| `cargo test --lib adr_0075_request_profile` | Green. 15 tests passed |
| `cargo test` | Green. 1,665 unit tests and 270 architecture tests passed. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green. 270 passed |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

The full suite used four test threads. No include list, request, path, fallback order, or
request count changed. No application launch and no production-data change occurred.

`cargo clippy --all-targets -- -D warnings` reports 68 errors. Every one is in a file that
this packet does not change, and no changed file reports one. That debt is outside packet 017
and outside the repository's required checks.

## Operator Visual Check

None. This packet changes no user-visible behavior, and it requires no visual acceptance.

The inherited presentation gates for packets 012, 014, 038, and 039 stay open and paused.
Do not request a visual batch for this packet.
