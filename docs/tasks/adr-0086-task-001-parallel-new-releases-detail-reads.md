# ADR 0086 Task 001: Parallel New Releases Detail Reads

Status: Implemented - 2026-10-23. Cases R86-1-01 to R86-1-06 pass. Mechanical checks are Green. The operator passed V3 on 2026-10-23. V1, V2 and V4 are open, in the final walk of the run of 2026-10-23.

## Goal

The New releases page currently reads the detail of each of the 20 feeds on a page one after the other. The fetch takes about 400 ms total. This packet reduces the delay by running at most 4 detail reads at the same time. The rows keep the order of the list response.

## Authority

- [ADR 0086](../adr/0086-new-releases-singles-window.md) Decision 5: at most 4 feed detail reads of a page run at the same time, as in ADR 0084 packet 026.
- [ADR 0084 packet 026](adr-0084-task-026-index-singles-groups.md) decision D4 and the pattern of [`FEED_SINGLES_PARALLEL`](../../src/application/queries/name_lists.rs:50) with `std::thread::scope` in `src/application/queries/name_lists.rs`.
- [Design and cleanup overhaul plan](../plans/design-and-cleanup-overhaul-plan.md) responsiveness item 10 (2026-10-17): the 400 ms fetch of each page.
- [AGENTS.md](../../AGENTS.md): "Every fix gets a guard", the guard pattern, and the conventions.

## Recorded Facts - 2026-10-23

- `src/application/queries/feed.rs:112-147` `fetch_recent_feed_result_rows` reads the list response, then sends one `fetch_feed_with_profile` request for each of the 20 feeds, one after the other. The loop `enumerate().map(|(index, feed)| { ... })` collects the results.
- `src/application/queries/name_lists.rs:50` `FEED_SINGLES_PARALLEL` is the constant 4.
- `src/application/queries/name_lists.rs:134-210` the pattern of `FetchReleaseNamePage::execute` reads each pending feed detail through `std::thread::scope` and a `Mutex<Vec<...>>` result list, keeping the order.
- The rows built in `fetch_recent_feed_result_rows` carry `feed_guid` and `index_feed_display`, which come from the response feed and its detail. The list response feed is a fallback when the detail request fails or returns nothing.
- `src/api.rs:208` `Feed::tracks: Option<Vec<Track>>`. The detail request includes `tracks` when a request profile asks for it. The list response does not. A detail failure falls back to the list response, which has no tracks.

## Design

### One shared constant

The detail-read limit is 4. One constant `PARALLEL_DETAIL_READS` in `src/application/queries/mod.rs` holds it. `feed.rs` and `name_lists.rs` both use it (D1).

### Parallel reads with scope

Replace the `.map()` loop in `fetch_recent_feed_result_rows` with `std::thread::scope`, similar to the pattern in `FetchReleaseNamePage::execute` (line 134-210 of `name_lists.rs`). The scope:

1. Creates a `Mutex<Vec<Option<FeedDisplayRow>>>` to hold the results in order.
2. Starts `PARALLEL_DETAIL_READS.min(feeds.len())` workers. Each worker takes the next index from a shared counter and sends that detail request, as the workers of `name_lists.rs` do. No other limit is necessary.
3. Keeps the order: each thread writes its result at the correct index.
4. Falls back to the list response when the detail request fails.

### No change to the fallback rule

When `fetch_feed_with_profile` fails or returns no detail, the row uses the list response feed, the same as today. The row's visual presentation does not change.

## Orchestrator Decisions

- **D1. One shared constant.** Both `name_lists.rs` and `feed.rs` import it from `src/application/queries/mod.rs`. The comment in `feed.rs` names ADR 0084 packet 026 as the shared pattern.

## Required Changes

1. `src/application/queries/mod.rs`: add the constant `PARALLEL_DETAIL_READS` with the value 4. Its `///` comment names ADR 0086 Decision 5 and ADR 0084 packet 026.
2. `src/application/queries/feed.rs`: import the constant. Rewrite `fetch_recent_feed_result_rows` to use `std::thread::scope` and the parallel reads pattern, keeping the row order.
3. `src/application/queries/name_lists.rs`: delete `FEED_SINGLES_PARALLEL` and use `PARALLEL_DETAIL_READS` in each place, also in the tests.
4. `tests/architecture_tests.rs`: add the guard of R86-1-06.

## Mechanical Acceptance Criteria

Use the prefix `adr_0086_parallel_reads_` for each test.

| Case | Required proof |
|---|---|
| R86-1-01 | With a local test server and a list of 20 feeds, the query gives 20 rows. Each row holds the detail feed or the list feed |
| R86-1-02 | The peak count of detail requests at one time is at most `PARALLEL_DETAIL_READS` and more than 1. Use the counter pattern of R26-08 in `name_lists.rs` |
| R86-1-03 | The rows keep the order of the list response, also when the server answers the detail requests in a different order |
| R86-1-04 | A failed detail request gives the row of the list feed, the same as before this packet |
| R86-1-05 | A list feed with no feed GUID sends no detail request and gives the row of the list feed |
| R86-1-06 | A situational guard in `tests/architecture_tests.rs` fails when `fetch_recent_feed_result_rows` does not use `PARALLEL_DETAIL_READS`. Its message names ADR 0086 Decision 5 and the fix: send the detail reads through the shared limit |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: open New releases in Tiles mode. The tiles show as placeholders while the page loads. After the page shows, each tile displays its cover, title and date span. Light and Dark themes.
- V2: open New releases in List mode. The rows show as placeholders while the page loads. Light and Dark themes.
- V3: normal width and narrow width. No tile clips, no row clips.
- V4: in the release build, the next page shows sooner than before. The overhaul plan recorded about 400 ms on 2026-10-17. When the rows arrive, the loading state ends, and the tiles or rows show together, not one at a time. Wrong: a page with tiles in a different order from the previous build.

## Exclusions

- No change to the row order, the pagination or the page limit.
- No change to the list response fallback rule.
- No visual change to the tiles or rows themselves.
- No new request profile or change to `INDEX_FEED_DETAIL`.
- No schema change, and no new database query.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md), the rust-dev skill and `docs/architecture/source-map.md`.
- `src/application/queries/feed.rs` and `name_lists.rs`.
- `src/api.rs` (`Feed` struct, `tracks` field).
- `src/application/request_profiles.rs` (`INDEX_FEED_DETAIL`).

## Checks

```bash
cargo test --lib adr_0086_parallel_reads_
cargo test --lib feed::tests
cargo test --lib name_lists::tests
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0086-task-001-parallel-new-releases-detail-reads.md`
- ADR 0086 Decision 5 and the overhaul plan responsiveness item 10
- ADR 0084 packet 026 decision D4 and its implementation in `src/application/queries/name_lists.rs`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- The view model gives each text, image and availability. The renderer only composes. Use named tokens. Column text does not call `.truncate()`.
- Tests use recorded values. No test sends a request to a live service.
- Background work runs through the ADR 0040 runtime. Do not call `cx.spawn` from a screen.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The database schema and the migration registry.
- ADRs and documents other than this packet.

Acceptance criteria:
- Each case R86-1-01 to R86-1-06 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns, and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
