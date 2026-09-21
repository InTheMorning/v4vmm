# ADR 0075 Task 016: Request And Write Baseline

Status: Complete - 2026-09-20. Measurements and orchestrator technical review are Green.
The operator's completion request released this bounded document task.
The task changes no shared application source or operator data.

## Goal

Measure current metadata requests and database row changes before request scheduling changes.
Cover search, first detail, repeated detail, shared feed context, and an unavailable Index endpoint.
Keep measured results separate from static bounds and untested UI dispatch behavior.

The [baseline note](../notes/adr-0075-request-and-write-baseline.md) contains the measurements, source scope, evidence paths, and reproduction commands.

## Files To Inspect

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), shared requests and source observations.
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md#performance-checks), the five required baseline cases.
- [search.rs](../../src/application/queries/search.rs), current and parked search commands.
- [library.rs](../../src/application/queries/library.rs), `FetchLibraryTrackContext` and `HydrateAlbumIdentity`.
- [feed.rs](../../src/application/queries/feed.rs), parked Discover detail commands.
- [search_dispatch.rs](../../src/app/search_dispatch.rs), active search and Index detail navigation.
- [app_impl.rs](../../src/library/app_impl.rs), Library detail dispatch and album hydration conditions.
- [search_results/mod.rs](../../src/view_models/search_results/mod.rs), retained Index results and detail projections.
- [feed_service.rs](../../src/feed_service.rs), scoped requests, fallback requests, and RSS enrichment.
- [subscribe_service.rs](../../src/subscribe_service.rs), `enrich_track_context_from_rss`.
- [rss/enrich.rs](../../src/rss/enrich.rs), direct RSS fetch and parse.
- [api.rs](../../src/api.rs) and [http_client.rs](../../src/http_client.rs), request construction.
- [identity_ingest.rs](../../src/identity_ingest.rs) and [db.rs](../../src/db.rs), hydration writes and test schema.

## Files Likely To Change

- This task packet.
- `docs/notes/adr-0075-request-and-write-baseline.md`.
- Temporary fixture and source files under a dedicated `/tmp/v4vmm-adr0075-016-*` directory.

## Do Not Touch

- Shared Rust source, integration tests, dependencies, configuration, or operator databases.
- Other task packets, shared status files, the ADR, or its phase plan.
- The upstream checkout or a live HTTP service.
- The desktop app, a display server, or a headless GPUI session.

## Constraints

Use the shared STE skill at `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.
Use production query functions through test-only calls in an isolated source snapshot.
Inject a localhost endpoint through existing constructor parameters.
Count received HTTP requests separately from database row mutations.
Record the source revision and copied dirty-source scope before adding the test harness.

Use only a fixture database and constructed feed data.
Keep capture times, hashes, and command output unchanged.
Do not claim latency, byte-volume, UI, or production performance results without those measurements.
Do not declare an unmeasured route complete.

## Implementation Steps

1. Identify the active search, detail, and hydration callers.
2. Capture a source snapshot and its hashes under `/tmp`.
3. Add an isolated localhost server and in-memory SQLite database to that snapshot.
4. Execute each baseline case through current query functions or shared detail projections.
5. Record HTTP paths, SQLite change counts, and table-level row operations.
6. Document static request formulas and unmeasured coverage separately.
7. Run the checks below.

## Acceptance Criteria

- The note identifies all five required scenarios and measured route boundaries.
- Thirteen fixture cases produce captured request and row-change records.
- The fixture proves first and repeated Library requests do not share feed or RSS reads.
- The note separates repeated hydration commands from the UI condition that can skip those commands.
- The unavailable-service case returns HTTP 503 and records its fallback requests.
- The note identifies connection refusal and timeout behavior as unmeasured.
- All requests use localhost. SQLite uses memory only. Fixture setup is excluded from row-change totals.
- The source snapshot records its revision, dirty-source patch, and source hashes.
- Exact reproduction commands and evidence paths are present.
- Local links and diff whitespace pass their checks. Confirmed STE defects are corrected.

## Checks

```bash
cargo test --manifest-path /tmp/v4vmm-adr0075-016-1c_nchth/Cargo.toml --target-dir /home/citizen/build/v4vmm/target --locked --offline --lib adr_0075_request_write_baseline -- --nocapture
python3 docs/runbooks/check-markdown-links.py docs/tasks/adr-0075-task-016-request-and-write-baseline.md docs/notes/adr-0075-request-and-write-baseline.md
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" --check --no-heuristics docs/tasks/adr-0075-task-016-request-and-write-baseline.md docs/notes/adr-0075-request-and-write-baseline.md
git diff --check
```

The localhost fixture required execution outside the sandbox after the sandbox rejected its listener.
The approved fixture run is Green. It ran one test with thirteen measurement cases.
The orchestrator accepted the named query and projection boundaries and their recorded limits on 2026-09-20.

Document checks are Green for 27 local links and diff whitespace.
The shared STE check reports lexical findings and no structural findings.
Retained technical terms include request, detail, source evidence, active route, and operator review.
The required coding-model prompt retains the skill's exact wording.

The STE checker covers configured rules only. Its result does not prove full standard compliance.

## Rollback And Cleanup

The fixture closes its server and in-memory database after the test.
The isolated source snapshot remains as measurement evidence.
No operator files, settings, or database state need restoration.
Remove only the dedicated snapshot directory when its evidence is no longer required.

## Escalation Triggers

- A required route cannot run without launching GPUI.
- A measurement requires operator data or a live service.
- Sandbox restrictions prevent the local listener from starting.
- The copied source changes during capture or cannot be identified precisely.

## Expected Final Report

Report request counts, row-change counts, reproduction paths, source scope, and unmeasured cases directly to the orchestrator.
Distinguish a document baseline from a scheduling improvement.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This packet and the files listed under Files To Inspect.
- `/home/citizen/.agents/skills/asd-ste100/SKILL.md`.

Goal:
- Measure the current request and database-write baseline.

Constraints:
- Change shared documentation only.
- Add measurement code only to an isolated temporary snapshot.
- Report measured cases separately from static analysis.

Do not touch:
- Shared application source, operator data, or live services.
- Other task packets or shared status documents.

Acceptance criteria:
- Meet every criterion under Acceptance Criteria.
- Record every required case or keep its measurement gate open.

Test commands:
- Run the commands under Checks.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

No visual check is required for these query measurements. The operator's visual pause remains in force.
