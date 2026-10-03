# ADR 0075 Request And Write Baseline

## Status And Scope

Measured and technically reviewed - 2026-09-20. [Packet 016](../tasks/archive/adr-0075-task-016-request-and-write-baseline.md) owns this baseline.
The baseline covers the five scenarios required by the [phase plan](../plans/adr-0075-metadata-contract-phase-plan.md#performance-checks).
It measures application query functions and shared detail projections with constructed data.
It does not measure desktop rendering or claim a performance improvement.

The active Index route loads full result details during search.
Its later detail projections use retained search results and make no additional metadata requests.
The Library track route fetches its track, feed, and RSS again on every measured request.
Repeated Library album hydration replaces stored facts even when the fixture returns unchanged values.

## Source And Capture Record

| Item | Recorded value |
|---|---|
| Baseline revision | `a12521e7510b3d05cd4fc097a370f1f965145aad` |
| Source capture time | `2026-09-20T12:09:54.843538+00:00` |
| Measurement start | `2026-09-20T12:13:51.448000+00:00` |
| Measurement end | `2026-09-20T12:13:51.705000+00:00` |
| Isolated snapshot | `/tmp/v4vmm-adr0075-016-1c_nchth` |
| Dirty application files copied | `src/api.rs`, `src/rss/subscribe.rs` |
| API changes copied | Packet 030 enclosure claims, packet 032 transcripts, and packet 033 language/sort transport |
| RSS changes copied | Packet 010 item-page storage and its tests |
| Measurement-only additions | `src/adr_0075_baseline.rs` and its test-only declaration in the snapshot's `src/lib.rs` |

The capture includes the working source exactly as copied at that time.
Later shared-source changes are outside this baseline.
The manifest lists every copied source hash and the complete working-tree status at capture.
Shared source files were read-only during this task.

| Retained artifact | SHA-256 |
|---|---|
| `baseline-source.patch` | `dd8839152ebb9c9d0ca603ba7e5daa8dcab9cc9791ca29ff5dd6907b2a2f6db8` |
| `src/adr_0075_baseline.rs` | `155992d1b394df9c554e3068c97f73a719afa5ca3369bd0054b690e73b9df180` |
| `baseline-test.log` | `a20c3d120940b20a48a6195f6937aa9020d5f79dd3267419e2d5e4631f761dfc` |
| `baseline-results.json` | `329a72c96825c4b481f19a39eb39bb480135584654656859204cecdf9e8f03fa` |

All artifact paths in this table are relative to the isolated snapshot.
`baseline-manifest.json` holds the source hashes and source-capture record.
The original measurement log remains unchanged during replay.

## Fixture And Counters

The fixture binds an ephemeral localhost port. It supplies one feed, `f1`, and two tracks, `t1` and `t2`.
Both tracks belong to `f1`. The feed URL points to the same fixture's `/feed.xml` resource.
The RSS document contains both item GUIDs.
No response supplies an artwork URL, and no media download or external request occurs.

Feed metadata includes one website claim, a description, language, and release kind.
Other source collections are empty. The fixture returns no pagination cursor.
The unavailable mode returns HTTP 503 for every Index API path.
Fixture setup creates a fresh in-memory database with one feed and two Library tracks.

The server records each received HTTP request path, including query parameters.
Each measurement also reads SQLite `total_changes()` before and after the operation.
A SQLite update hook records affected rows by operation and table.
The hook installation must succeed before measurements start.

The database counts represent row mutations, including replacement deletes and inserts.
They are not SQL statement counts, disk writes, bytes written, or transaction counts.
An update can count a row even when the assigned value is unchanged.
Fixture setup and schema creation occur before counting begins.

The fixture executes cases sequentially in one test.
It retains the shared result model and database when a repeated case requires prior state.
The local server has no redirects, transport failures, or delayed responses.
The measured unavailable service tests HTTP failure. Connection refusal and timeout remain unmeasured.

## Measured Counts

The HTTP columns count metadata requests only.
Index detail rows below measure shared projections from the existing result model.
Library rows measure the commands dispatched by current Library detail code.

| Case | Index HTTP requests | RSS HTTP requests | SQLite row changes | Result |
|---|---:|---:|---:|---|
| Local search for `Baseline` | 0 | 0 | 0 | Two local tracks returned |
| Index search: one feed and two tracks | 5 | 0 | 0 | One feed and two track result models returned |
| First Index track detail after search | 0 | 0 | 0 | Retained track detail returned |
| Repeated Index track detail | 0 | 0 | 0 | The same retained track detail returned |
| Index feed detail and both track details | 0 | 0 | 0 | All three retained detail models returned |
| First Library track detail | 2 | 1 | 0 | Remote context and RSS enrichment returned |
| Repeated Library track detail | 2 | 1 | 0 | The same three resources were fetched again |
| Two Library tracks sharing one feed | 4 | 2 | 0 | Feed and RSS were each fetched twice |
| First Library album hydration command | 1 | 0 | 5 | One feed row and four fact rows changed |
| Repeated Library album hydration command | 1 | 0 | 9 | Old facts were deleted and inserted again |
| Index search with HTTP 503 | 2 | 0 | 0 | Combined search error returned |
| Library track detail with HTTP 503 | 3 | 0 | 0 | Local context returned after remote failures |
| Library album hydration with HTTP 503 | 1 | 0 | 0 | Error returned and existing data retained |

The first album hydration writes one feed row, one identity-link row, and three metadata rows.
The repeated command deletes the four fact rows, inserts four replacements, and updates the feed row.
These totals depend on the constructed payload. They are not universal per-feed write counts.

The repeated album case deliberately invokes the command twice.
The UI can skip its second invocation when identity actions, description, and metadata are already present.
That UI condition is static evidence below. The test does not claim that a second click always writes nine rows.

## Captured Request Sequences

The Index search case sent these five requests:

1. Feed search with `q=Baseline`, `limit=20`, `type=feed`, and `fuzzy=true`.
2. Feed `f1` detail with the current full feed include string.
3. Track search with `q=Baseline`, `limit=20`, `type=track`, and `fuzzy=true`.
4. Scoped detail for track `t1` in feed `f1`, without an include parameter.
5. Scoped detail for track `t2` in feed `f1`, without an include parameter.

The current full feed include string is:

```text
tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes
```

Each successful Library track case sent a scoped track request, a feed request, and an RSS request.
Its track and feed requests used:

```text
source_links,source_ids,source_release_claims,source_contributors,payment_routes
```

The failed Library track case sent three Index requests:

1. `/v1/feeds/f1/tracks/t1`.
2. `/v1/tracks/t1`, after the scoped request failed.
3. `/v1/feeds/f1`.

All three used the Library include string above.
No RSS request followed because neither remote track nor remote feed retrieval succeeded.
The query returned its local context.

Album hydration used one `/v1/feeds/f1` request with:

```text
source_links,source_ids,source_release_claims,source_contributors
```

The JSON artifact retains exact request paths, encoded query strings, and each case's recorded time.

## Static Bounds And Caller Evidence

These conclusions come from source inspection. They are not additional measured cases.

| Source | Static conclusion |
|---|---|
| [search.rs](../../src/application/queries/search.rs), `fetch_index_search_result_rows` | Search executes feed and track searches separately. Every returned hit triggers a detail request. |
| Same file, `fetch_index_feed_result_rows` and `fetch_index_track_result_rows` | The request-call formula is `2 + F + T`, where `F` and `T` are returned hit counts. |
| [api.rs](../../src/api.rs), `PAGE_LIMIT` and `search` | The client requests 20 hits per type. It does not cap the decoded vectors before detail loading. |
| [search_dispatch.rs](../../src/app/search_dispatch.rs), detail navigation | Navigation selects retained Index results. It does not dispatch a new metadata command for those detail projections. |
| [search_results/mod.rs](../../src/view_models/search_results/mod.rs), `index_feed_detail` and `index_track_detail` | Both functions read cached result rows. They have no HTTP client or database parameter. |
| [app_impl.rs](../../src/library/app_impl.rs), `select_track_detail` and `load_track_source_context` | Each Library track selection dispatches `FetchLibraryTrackContext`. There is no completed-request cache in this path. |
| [feed_service.rs](../../src/feed_service.rs), `fetch_library_track_detail` | A failed scoped lookup can trigger an unscoped lookup before the feed request. |
| Same file, `merge_track_context_from_detail` | A successful remote context runs RSS enrichment when a feed URL exists. |
| [rss/enrich.rs](../../src/rss/enrich.rs), `fetch_track_enrichment_from_feed` | Each enrichment call fetches and parses a new RSS document. It does not reuse a previous parse. |
| [library.rs](../../src/application/queries/library.rs), `hydrate_album_identity_facts` | A successful hydration can update the feed description and replace Index fact collections. |
| [app_impl.rs](../../src/library/app_impl.rs), `hydrate_album_identity_on_view` | The UI skips hydration only when identity actions, description, and nonempty metadata already exist. |

For a server that honors both requested limits, one full Index search can issue 42 metadata request calls.
That is a conditional bound, not an enforced client maximum.
An oversized response can exceed it because the loops process every decoded hit.
HTTP redirects and transport retries are outside this formula.

For the scoped Library route, successful fallback can cause three Index calls plus one RSS call.
The measured normal case needed two Index calls plus one RSS call.
The measured HTTP 503 case needed three Index calls and no RSS call.

The inspected upstream `src/query.rs::SearchResponseItem` supplies identifiers, rank, quality score, feed scope, and an optional URL.
It supplies no title or full display metadata at upstream revision `a220f44`.
Reducing detail calls while preserving result labels needs a request-contract decision, retained data, or different summary responses.
This note selects none of those policies.

## Coverage Limits And Next Use

All five planned scenario categories have measurements at their named non-GPUI boundaries.
Packet 016's measurement gate is complete for the constructed fixture.
The orchestrator accepted its query and projection boundaries and stated limits on 2026-09-20.

These routes and effects remain outside the measurement:

- Desktop event dispatch, rendering, artwork fetches, and media playback.
- Concurrent identical requests and cancellation races.
- Connection refusal, DNS failure, timeouts, redirects, and transport retry counts.
- The toolbar's capability rejection before it dispatches an unavailable command.
- Parked Discover search and inspector commands.
- Artist search enrichment and additional pagination pages.
- Real endpoint latency, response bytes, disk writes, and production data.

The parked Discover search has separate per-hit hydration and artist enrichment in `search.rs`.
It must not replace this active Index baseline during later comparisons.
Its request counts remain unmeasured.

Packets 017 and 018 must name their target bounds against these same fixtures before claiming improvement.
They must also add concurrent-request measurements before claiming active-request sharing.
The current baseline establishes no freshness threshold or new source priority.

## Reproduce The Measurement

Use the retained isolated snapshot. It contains the original source, dirty-source patch, manifest, and test harness.
The test requires permission to bind a localhost socket.
It does not start the desktop binary.

```bash
ADR0075_BASELINE_ROOT=/tmp/v4vmm-adr0075-016-1c_nchth
cargo test --manifest-path "$ADR0075_BASELINE_ROOT/Cargo.toml" --target-dir /home/citizen/build/v4vmm/target --locked --offline --lib adr_0075_request_write_baseline -- --nocapture > "$ADR0075_BASELINE_ROOT/baseline-replay.log" 2>&1
python3 - "$ADR0075_BASELINE_ROOT/baseline-replay.log" <<'PY'
import json
import sys
from pathlib import Path

records = [
    json.loads(line.removeprefix("ADR0075_BASELINE "))
    for line in Path(sys.argv[1]).read_text().splitlines()
    if line.startswith("ADR0075_BASELINE ")
]
assert len(records) == 13, f"Expected 13 cases, received {len(records)}"
for record in records:
    print(record["case"], record["request_count"], record["sqlite_total_changes"])
PY
```

Compare request counts and row changes with the measured table.
The ephemeral port and recorded timestamps can change between runs.
The expected logical request paths and row-change totals remain the same for this frozen snapshot.

The snapshot's manifest records source hashes before the measurement module was added.
Its `baseline-source.patch` records application differences from the named Git revision.
The test module and modified snapshot `src/lib.rs` are measurement-only additions.
Do not copy those additions into production source.

The retained temporary snapshot is required by these replay commands.
If it is removed, reconstruct its source from the named revision and saved patch before using the captured harness.
The repository note does not contain a second copy of the source snapshot or harness.

## Checks And Cleanup

The isolated test passed with thirteen measurement records and no compiler warnings.
Local links and diff whitespace are Green. The STE check has lexical findings and no structural findings.
Its server and in-memory database closed when the test returned.

The snapshot remains as evidence. No operator state requires cleanup or restoration.
No desktop binary is handed to the operator by this packet.

## Operator Visual Check

No visual check is required for this baseline. Existing visual gates and the operator's pause remain unchanged.
