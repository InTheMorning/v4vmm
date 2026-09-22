# ADR 0075 Task 018: Request Reuse, Freshness And Explicit Refresh

Status: Complete on 2026-09-22. Part A completed on 2026-09-21, and Part B on 2026-09-22.
The mechanical checks of both parts are Green. The operator accepted the seven reuse policies
on 2026-09-21, and two more decisions on 2026-09-22.

This packet has two parts. Part A implements the rules that ADR 0075 already decides. It
changes no request count except for concurrent duplicates. Part B ends the repeated fetch
that the baseline measured. Part B applies the accepted policies below.

## Goal

Give the app one shared request identity and one owner that reuses a request. Give the app
one explicit refresh that bypasses reuse. A caller then asks the owner for a resource. The
owner sends a request, joins an active request, or returns a retained response.

Packet 017 named each request. This packet keys those names to a subject and an endpoint, and
it decides when a response is reusable.

## Why This Packet Exists

The [request baseline](../notes/adr-0075-request-and-write-baseline.md) measured the
repetition:

| Measured case | Index requests | RSS requests |
|---|---:|---:|
| First Library track detail | 2 | 1 |
| Repeated Library track detail | 2 | 1 |
| Two Library tracks sharing one feed | 4 | 2 |
| Repeated Library album hydration | 1 | 0 |

The repeated Library track detail fetched the same three resources again. Two tracks of one
feed fetched that feed twice and its RSS document twice. The repeated album hydration deleted
four fact rows and inserted four replacements for values that did not change.

The Index route makes no detail request after a search, because it retains its search result
model. Its requests happen during the search itself.

## What ADR 0075 Already Decides

These rules are accepted. Part A implements them, and they need no new review.

- Reuse a completed or active request with the same endpoint, scoped identity, and request
  profile. ADR 0075 section 6.
- Keep caches isolated by endpoint and source revision. ADR 0075 section 6.
- Define response ordering when the service supplies no reliable source revision. Do not use
  the source observation time as the app's request sequence. ADR 0075 section 6.
- RSS enrichment must report failures. ADR 0075 section 6.
- A failed request keeps the stored facts and exposes the failed refresh state. It does not
  become an empty collection. ADR 0075 section 2.
- Do not resolve a discrepancy because a request failed, or because its evidence expired from
  a cache. ADR 0075 section 4.
- A retained value is not fresh because it exists. ADR 0075 section 4.

## Accepted Policies - 2026-09-21

The operator decided each policy separately on 2026-09-21.

| Number | Accepted policy |
|---|---|
| P18-1 | Reuse a successful Library track detail response for 30 minutes |
| P18-2 | Reuse a successful feed response for 15 minutes, for each distinct include list |
| P18-3 | Reuse a parsed RSS document for 15 minutes, keyed by its feed URL |
| P18-4 | Never reuse a failed request. A failure sends a new request every time |
| P18-5 | Hold at most 64 feed responses, 256 track responses, and 32 RSS documents. Remove the least recently used entry first |
| P18-6 | Hold reused responses in memory only. A restart clears them |
| P18-7 | An explicit refresh removes every entry of the named feed and its tracks, and then sends new requests |
| P18-8 | An Index track detail response gets no reuse window. A concurrent caller still joins an active request. Accepted 2026-09-22 |
| P18-9 | A reused response replays the receipt of the fetch that produced it. It writes no new observation. Accepted 2026-09-22 |

The minute values come from the curator workflow, not from a measurement. The operator chose
the 30-minute track window because the existing check-for-updates control supplies a fresh
value at any time. P18-7 makes that control clear the feed, its RSS document, and its tracks
together.

The 15-minute RSS window can delay the detection of a stale MusicIndex record by 15 minutes
during passive browsing. An explicit refresh removes that delay. Packet 045 owns the
stale report and the podping.me direction.

P18-6 keeps this packet away from a storage decision. A durable response store needs its own
migration, its own backup, and its own rollback, which ADR 0075 section 5 requires.

P18-8 covers the one route with no other policy. An Index track detail request sends no
include list, and P18-1 names a Library track detail response only. The operator decided on
2026-09-22 that this route keeps no completed response.

P18-9 keeps one meaning of reuse across all three caches. The feed cache, the track cache,
and the RSS document cache each replay the receipt of the fetch that produced the response.
The replayed receipt keeps the generation of that earlier fetch, because the evidence is
older than the call that reuses it.

## Part A: Request Identity And Sharing

Part A is complete on 2026-09-21. It implements the accepted rules only.

**One request identity.** A key holds the endpoint, the scoped subject, and the packet 017
profile. The storage layer already builds the same identity in
`db::provider_observations::request_identity`. The in-memory key must agree with it, so that
one request has one name in memory and in storage.

**One owner.** A shared owner holds the active requests and the retained responses. A caller
asks the owner, and the owner sends at most one request for one identity at one time. A second
caller with the same identity joins the active request and receives the same result.

**One app request sequence.** The owner orders responses by a monotonic counter that the app
allocates when it starts a request. A response of an older sequence value never replaces a
newer stored response. The owner does not order by a fetch time or by a source time.

**Observations stay with real requests.** A request that reaches the network records its
observation, as packets 014, 038, and 039 require. A caller that joins an active request
receives the receipts of that one request. A caller that receives a retained response receives
no new receipt, and its result names the observation that produced the response.

**Explicit refresh.** A typed refresh intent travels with the request. The intent bypasses
reuse and forces a new request. Part A wires the intent through the existing feed check
command, the existing feed update command, and the ADR 0065 combined workflow. Part A adds no
new control to any screen.

## Part B: Completed Response Reuse

Part B applies the accepted policies in the table above.

Part B adds the freshness test to the owner. A retained response is reusable when its age is
below its accepted window, and when no explicit refresh intent applies. A reusable response
returns without a request.

The owner measures the age of a retained response with a monotonic instant that it records
when the response completes. It does not read a fetch time from storage, and it does not read
a wall-clock time. P18-6 keeps each retained response in memory, so a restart empties the
owner and the first request after a restart always reaches the network. A monotonic instant
also stays correct when the system clock moves.

This replaces the earlier plan to extend `db::provider_observations::read_request_refresh`.
That extension is not necessary, and `src/db/provider_observations.rs` stays unchanged.

Part B also converts the remaining Index request sites in
`src/application/queries/search.rs` and `src/application/queries/feed.rs`. The operator moved
that conversion here on 2026-09-21.

## Authority And Dependencies

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), sections 2, 4, 5, and 6,
  and Decision I.
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md#the-committed-path), the
  committed path.
- [Packet 016](adr-0075-task-016-request-and-write-baseline.md) and its
  [baseline](../notes/adr-0075-request-and-write-baseline.md), the measured counts and the
  retained fixture.
- [Packet 017](adr-0075-task-017-named-request-profiles.md), the ten named profiles.
- [Packet 035](adr-0075-task-035-comparison-and-discrepancy-rules.md), which assigns numeric
  freshness and request scheduling to this packet.
- [Packet 014](adr-0075-task-014-provider-observation-retention.md), the shared recorder and
  its receipts.

Packet 045 owns the stale MusicIndex report and the podping.me direction. This packet does not
report staleness to the operator.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- [Request profiles](../../src/application/request_profiles.rs), the ten profiles.
- [Observation storage](../../src/db/provider_observations.rs), `request_identity`, `begin`,
  and `read_request_refresh`.
- [Observation types](../../src/provider_observation.rs), `ProviderRequestSpec`,
  `RequestRefresh`, `RefreshState`, and the recorder.
- [API client](../../src/api.rs), the three profile methods and `Client`.
- [Feed service](../../src/feed_service.rs), the Library track detail chain and the feed
  update.
- [Library queries](../../src/application/queries/library.rs), the track context command and
  the album hydration.
- [Search queries](../../src/application/queries/search.rs), the Index search request loops.
- [RSS enrichment](../../src/rss/enrich.rs), which parses a new document for each call.
- [Runtime](../../src/runtime/), the ADR 0040 actors, and
  [playback polling](../../src/runtime/playback_polling.rs) as the reference actor.
- [Library screen](../../src/library/app_impl.rs), the composition root that builds the track
  context command.

## Files To Change

| File | Permitted change |
| --- | --- |
| New module under `src/application/` | The request key, the owner, its typed states, and its tests |
| `src/application/request_profiles.rs` | Nothing in Part A. Part B may add an accepted window to a profile |
| `src/db/provider_observations.rs` | Not needed. The owner records its own completion instant under P18-6 |
| `src/feed_service.rs` | Ask the owner, not the client. Keep the requests and the fallback order |
| `src/application/queries/library.rs` | Ask the owner. Carry the refresh intent |
| `src/application/queries/search.rs` | Part B only. Ask the owner. Carry the refresh intent |
| `src/application/queries/feed.rs` | Part B only. Ask the owner. Carry the refresh intent |
| `src/rss/enrich.rs` | Part B only. Hold a parsed document under P18-3 |
| `src/application/mod.rs` | One line. Register each new module |
| `src/library/app_impl.rs` | Not needed. The owner is one process-wide value, as `src/remote_media.rs` and `src/ui/icons.rs` already do |
| `tests/architecture_tests.rs` | Situational guards, naming ADR 0075 |

Do not change the include lists in `src/subscribe_service.rs`,
`src/application/commands/payment_routes.rs`, or `src/application/commands/feed.rs`. Packet
017 records why those lists stay outside. Part B may change the receipt assembly in
`src/application/commands/feed.rs` when R18B-11 needs it.

## Mechanical Acceptance Criteria, Part A

Use the prefix `adr_0075_request_reuse_` for behavioral tests beside the owning code.

| Case | Required proof |
| --- | --- |
| R18A-01 | A request key holds the endpoint, the scoped subject, and the profile. Two requests with different endpoints have different keys |
| R18A-02 | The in-memory key and `request_identity` agree. A test builds both from one `ProviderRequestSpec` and compares them |
| R18A-03 | Two concurrent callers with one identity produce one HTTP request. Both receive the same result |
| R18A-04 | Two concurrent callers with different identities produce two HTTP requests |
| R18A-05 | The joining caller receives the receipts of the one request. The test counts the receipts |
| R18A-06 | A response of an older sequence value does not replace a newer stored response |
| R18A-07 | The owner allocates its sequence value when the request starts. The test proves it is not a fetch time and not a source time |
| R18A-08 | A failed request records its failure and returns the typed failed state. It stores no value |
| R18A-09 | An explicit refresh intent sends a new request, and it does not join an active request that started without the intent |
| R18A-10 | The feed check command, the feed update command, and the ADR 0065 workflow each carry the refresh intent |
| R18A-11 | Every measured request count of the baseline is unchanged for sequential callers. The test names the counts |
| R18A-12 | A guard fails when a metadata request reaches `Client` without the owner, in the converted routes. The guard names ADR 0075 section 6 |

## Mechanical Acceptance Criteria, Part B

| Case | Required proof |
| --- | --- |
| R18B-01 | A retained response inside its accepted window returns without an HTTP request |
| R18B-02 | A retained response outside its window sends a new request |
| R18B-03 | The 30-minute, 15-minute, and 15-minute windows each appear once in the code. A test reads each value from that one owner |
| R18B-04 | A failed request is never reused. The next ask sends a new request |
| R18B-05 | An explicit refresh removes the entries that P18-7 names, and then sends new requests |
| R18B-06 | The owner holds at most the P18-5 limits. It removes the least recently used entry first |
| R18B-07 | A reused response carries the identifier of the observation that produced it. It creates no new observation |
| R18B-08 | A reused response cannot create or resolve a discrepancy. A guard names ADR 0075 section 4 |
| R18B-09 | The repeated Library track detail case sends no Index request and no RSS request inside the windows |
| R18B-10 | Two Library tracks of one feed send one feed request and one RSS request inside the windows |
| R18B-11 | A caller that joins a `check_feed_staleness` or `apply_feed_updates` request receives the receipts of that request. This closes the recorded limit of R18A-05 |
| R18B-12 | Each converted Index request site asks the owner. The guard counts the owner calls in `search.rs` and `feed.rs` |

## Measurement

The baseline requires a target bound before this packet claims an improvement. It also
requires concurrent measurement before this packet claims active-request sharing.

Measure with the fixtures of packet 016, in the same isolated way, without an app launch:

| Case | Current | Target after Part B |
|---|---:|---:|
| Repeated Library track detail | 2 Index, 1 RSS | 0 Index, 0 RSS |
| Two Library tracks sharing one feed | 4 Index, 2 RSS | 3 Index, 1 RSS |
| Repeated Library album hydration | 1 Index, 9 row changes | 0 Index, 0 row changes |
| Two concurrent Library track details of one track | Not measured | 1 request set. Part A measured this at the owner. Part B measures it against the packet 016 fixtures |

Record the measured results beside the targets. Report a target that the result does not
reach. Do not change a target to match a result.

## Exclusions

- No stale MusicIndex report, and no podping.me direction. Packet 045 owns those.
- No shared projection and no display change. Packet 020 owns those.
- No new screen control, and no new visual gate.
- No durable response store, under P18-6.
- No field policy, and no comparison rule.
- No change to the packet 017 include lists or request paths.

## Checks

```bash
cargo test --lib adr_0075_request_reuse
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Part A adds one module and changes the call sites to ask the owner. Revert the working tree to
remove it. Part B adds the freshness test and one storage read. Revert the working tree to
remove it. No migration, no stored data, and no configuration changes.

## Implementation Result, Part A - 2026-09-21

`src/application/request_reuse.rs` holds the request key, the owner, and its typed states.

`RequestKey` holds the provider identity, the subject, and the include list. `RequestSubject`
separates a feed, a scoped track, and an unscoped track. `MetadataRequestOwner` holds three
typed registries and one sequence counter. `request_reuse::shared` returns the one
process-wide owner. A test builds its own owner, so one test cannot reach another test's
active requests.

`RefreshIntent` has the values `Normal` and `Explicit`. `SharedFetchError` holds an
`Arc<anyhow::Error>`, because `anyhow::Error` is not `Clone`. Its `into_anyhow` restores an
`ObservationWriteFailure` or an `ObservationStorageError` to its own type, so
`propagate_storage_failure` still classifies a storage failure after the round trip.

### The Sequence Counter

The owner holds an `AtomicI64` and increments it once for each request start. It does not use
the storage `metadata_generation` allocation. That allocation is reachable only through
`begin_provider_request`, which runs inside the private `Client::get_observed_json`. A call to
it before the join decision would also write to the database on every check. The in-memory
counter orders the owner's own entries for one run of the process, which matches P18-6.

### The Lock And The Request

`single_flight` holds the registry lock only to find or insert a slot. It releases the lock
before a joined caller waits, and before the winning caller sends its request. It takes the
lock again after the request returns. It removes the slot only while that slot is still the
registry entry for its identity. A superseded slot that finishes late cannot remove a
newer one.

### An Abandoned Request

The orchestrator added `SlotCompletion` during review. A panic in the request closure left the
slot `Pending`, so each joined caller waited without end, and the identity stayed in the
registry. The guard now fails the slot, wakes each joined caller, and removes the identity. A
later caller then starts a new request. `adr_0075_request_reuse_abandoned_request_releases_a_joined_caller`
proves it.

### Converted Call Sites And Their Intents

| File | Function | Intent |
|---|---|---|
| `src/feed_service.rs` | `fetch_library_track_detail_with_recorder` | From its caller |
| `src/feed_service.rs` | `fetch_library_track_context` | `Normal` |
| `src/feed_service.rs` | `check_feed_staleness` | `Explicit` |
| `src/feed_service.rs` | `apply_feed_updates`, and its track loop | `Explicit` |
| `src/application/queries/library.rs` | `fetch_library_track_context_with_local_fallback` | `Normal` |
| `src/application/queries/library.rs` | `compare_library_track` | `Explicit` |
| `src/application/queries/library.rs` | `hydrate_album_identity_facts` | `Normal` |

The ADR 0065 combined workflow calls `check_feed_staleness` and `apply_feed_updates`, so it
carries the explicit intent without code of its own.

The operator accepted the explicit intent for `compare_library_track` on 2026-09-21. A
comparison against the source always sends a new request. It never joins another caller's
request, and Part B never reuses a retained response for it. That keeps the two independent
observations that packet 038 accepted for a concurrent detail read and comparison.

### Two Recorded Limits

R18A-02 asks for a test that builds the in-memory key and the storage identity from one
`ProviderRequestSpec`. `request_identity` is private, and `src/db/provider_observations.rs` is
a Part B file. The test instead drives the storage identity through the public
`begin_provider_request`. It then proves that both agree on which requests are the same, and
on which requests are different, for each pair it checks. It proves agreement as a relation, not by one call
on one input.

R18A-05 holds for `hydrate_album_identity_facts`, which shares its receipts with a joined
caller. A caller that joins a `check_feed_staleness` or `apply_feed_updates` request receives
the data without a receipt of its own. Those two functions send an explicit intent, so they
never join another request, and only a `Normal` caller of the same identity can join theirs.
The sequential behavior is unchanged. R18B-11 closes this gap.

### The Index Routes Are Part B

`src/application/queries/search.rs` and `src/application/queries/feed.rs` keep their direct
client calls. Six request sites remain. The operator moved this conversion to Part B on
2026-09-21. The Index search loops are sequential, and Part A changes behavior only for two
callers that ask at the same time. The reuse windows of Part B make the conversion
useful, and they touch the same lines.

### Guards

The new guard is `adr_0075_request_reuse_converted_routes_ask_the_owner`. It counts the owner
calls in each converted `feed_service` function. Its failure message names ADR 0075 section 6
and the three owner methods.

Two existing guards moved with the code. Each one checks the same rule as before:

- `adr_0075_feed_observation_roots_and_consumers_are_guarded`, for the two explicit feed
  requests.
- `adr_0075_library_observation_callers_and_consumers_are_guarded`, for the album hydration
  call, which `cargo fmt` now wraps across lines.

### Checks - 2026-09-21

| Check | Result |
|---|---|
| `cargo test --lib adr_0075_request_reuse` | Green. 11 tests passed |
| `cargo test` | Green. 1,676 unit tests and 271 architecture tests passed. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green. 271 passed |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

The full suite used four test threads. No application launch and no production-data change
occurred.

## Implementation Result, Part B - 2026-09-22

Part B ran as two sessions, a core and a follow-up. The orchestrator reviewed each result and
ran the integrated checks.

### Reuse, Freshness And Capacity

`RetainedCache` holds a completed response beside the `std::time::Instant` of its completion.
The owner compares that instant against the accepted window. It reads no fetch time from
storage and no wall-clock time, so a system clock change cannot make a retained response look
fresh. `src/db/provider_observations.rs` is unchanged.

Each window value appears once in the code. The track and feed windows live in
`src/application/request_reuse.rs`. The RSS document window lives in `src/rss/enrich.rs`,
because P18-3 keys that document by feed URL and not by a request identity.

A failure is never retained. The caches hold at most 64 feed responses, 256 track responses,
and 32 RSS documents, and each one removes its least recently used entry first.

### Feed-Wide Invalidation

The registry key holds the provider identity, the subject, and the include list. One feed
therefore has several entries, and a scoped track names its feed in its own subject. The owner keeps a
second index from a feed to its keys. `invalidate_feed` removes each indexed key from both
retained caches, and `rss::invalidate_feed_document` removes that feed's RSS document. An
unscoped track carries no feed identity, so P18-7 cannot reach it by feed. Its own window
still bounds it.

### Evidence Of A Reused Response

The feed and track registries hold each response beside the receipts of the fetch that
produced it. A retained hit and a joining caller both receive those receipts. This closed the
limit that Part A recorded against R18A-05.

`ProviderObservationRecorder::record` now returns the receipt it wrote, and `replay` adds an
earlier receipt without a second write. The RSS document cache keeps the receipt of its fetch
and replays it. An unobserved fetch retains no receipt, so a later observed call fetches
again rather than report evidence it cannot name.

The first Part B session recorded a new observation for each RSS reuse instead, to keep
receipt order stable. The operator rejected that on 2026-09-22. Only tests depended on that order, and a reused
response is genuinely older evidence. A new observation for each cache hit would also grow
`metadata_observations` without bound.

### Active RSS Requests Are Shared

`src/rss/enrich.rs` calls the same generic `single_flight` that Part A built, keyed by feed
URL. It is not a second copy of that machinery, so the abandonment guard still applies. Two
concurrent callers for one feed URL send one request.

### The Index Routes

The six Index request sites in `src/application/queries/search.rs` and
`src/application/queries/feed.rs` ask the owner. Neither file holds an observation recorder.
Their requests carry no receipts. An Index feed detail response falls under P18-2 and
keeps its 15-minute window. An Index track detail response falls under P18-8:
`fetch_track_shared` joins an active request and never reads or writes the retained cache.

### Measurement - 2026-09-22

| Case | Target | Measured |
|---|---|---|
| Repeated Library track detail | 0 Index, 0 RSS | 0 Index, 0 RSS |
| Two Library tracks sharing one feed | 3 Index, 1 RSS | 3 Index, 1 RSS |
| Repeated Library album hydration | 0 Index, 0 row changes | 0 Index, 0 row changes |
| Two concurrent Library track details of one track | 1 request set | 3 requests, all shared |

Each target is met. The concurrent case reached its target only after the RSS single-flight,
which the first Part B session reported as a miss at four requests.

### A Recorded Limit

The owner serves the whole process, and its capacity is shared by every test in the binary.
A test that stores more than the capacity between another test's two calls could evict an
entry and make that test fetch again. Key isolation holds. Each fixture binds its own port,
and that port is part of the provider identity. Six full suite runs were stable. A later
test that stores many entries could still reach this limit.

### Guards

`adr_0075_request_reuse_retention_never_reaches_comparison_code` proves R18B-08 structurally:
retention code must not reach the comparison types. A second guard counts the owner calls in
the two Index query files for R18B-12. Four existing guards moved with the code, and each one
checks the same rule as before.

### Checks - 2026-09-22

| Check | Result |
|---|---|
| `cargo test --lib adr_0075_request_reuse` | Green. 25 tests passed |
| `cargo test` | Green. 1,695 unit tests and 273 architecture tests passed. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green. 273 passed |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

The full suite used four test threads. No application launch and no production-data change
occurred.

## Operator Visual Check

None. Part A and Part B add no control and change no layout. A reused response can show an
older value inside its window, and R18B-01 and R18B-02 prove that behavior mechanically.

The inherited presentation gates for packets 012, 014, 038, and 039 stay open and paused. Do
not request a visual batch for this packet.
