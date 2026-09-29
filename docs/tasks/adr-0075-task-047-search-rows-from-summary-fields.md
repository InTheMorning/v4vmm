# ADR 0075 Task 047: Search Rows From Summary Fields

Status: Ready - 2026-09-29. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

The Index search draws each result row from the summary fields of the search response. It sends no detail request for each hit.
A row sends its detail request when the operator opens it.

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) section 6 and Decision C, and packets 017 and 018: named request profiles and one shared request owner.
- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decision 6: feed owner text is not an artist.
- [MusicIndex API change request](../plans/musicindex-api-change-request.md) change 1, live since 2026-09-23.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, and "Current-view state must update in place".

## Recorded Facts - 2026-09-29

- The deployed contract is version `0.2.0`. `SearchResponseItem` declares `entity_type`, `entity_id`, `rank`, `quality_score`, `title`, `feed_guid`, `feed_title`, `feed_image_url`, `track_image_url`, `release_artist`, `release_artist_source`, `track_artist`, `pub_date`, `duration_secs`, `episode_count` and `href`.
- A live hit omits each field whose value is null. A feed hit for "Monster" gave no `feed_guid` and no `href`. Its `entity_id` is the feed GUID.
- `api::SearchResult` decodes only `entity_type`, `entity_id`, `feed_guid` and `quality_score`.
- `fetch_index_feed_result_rows` and `fetch_index_track_result_rows` in `src/application/queries/search.rs` send one search each, then one detail request for each hit.
  With `limit=20`, one search sends a maximum of 42 requests. The [request baseline](../notes/adr-0075-request-and-write-baseline.md) records the sequence.
- `index_feed_display` shows the title, `release_artist`, the track count, `publisher_text` and `image_url`. `index_track_display` shows the title, `track_artist`, `release_artist`, `feed_title` and `image_url`.
- The row keeps the fetched detail. The Index feed and track detail pages read that retained detail (`index_feed_detail` and `index_track_detail` in `src/view_models/search_results/mod.rs`).
- `api::Client::search` sends `fuzzy=true`. The contract declares only `q`, `type`, `limit` and `cursor`.

## Required Changes

### 1. Decode The Summary

- Add each declared summary field to `api::SearchResult` as an optional value. An omitted field decodes as absent.

### 2. Rows From The Summary

- A feed row uses `title`, `release_artist`, `episode_count` and `feed_image_url`. It shows no `publisher_text`, because the summary does not carry it and ADR 0077 Decision 6 does not make it an artist.
- A track row uses `title`, `track_artist`, `release_artist` and `feed_title`. Its artwork is `track_image_url`, else `feed_image_url` (ADR 0075 Decision C).
- A feed hit without `feed_guid` uses `entity_id` as the feed GUID.
- The Index name candidates of packet 006 use `release_artist` and `track_artist` of the summary.
- The search sends no detail request. One search sends two requests: one feed search and one track search.

### 3. Detail On Open

- Opening a feed row or a track row sends its detail request through the shared request owner of packet 018, with the existing named profile.
- The detail page shows a loading state from its view model, then the detail. A failure gives a display state, not a transport error.
- A result for an earlier row does not replace the page of a later row. Follow `publisher_page_result_is_current` in `src/app/publisher_dispatch.rs`.
- A second open of the same row inside the reuse window of packet 018 sends no request.

### 4. The Undeclared Parameter

- Stop sending `fuzzy=true`. The contract does not declare it. Record in the implementation result if a live search gives different rows without it.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_search_summary_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R47-01 | A recorded feed hit and a recorded track hit decode each summary field. An omitted field decodes as absent |
| R47-02 | A search with 20 feed hits and 20 track hits sends two requests. Use the packet 016 measurement form |
| R47-03 | A feed row exposes the title, the artist, the track count and the feed artwork, and no `publisher_text` |
| R47-04 | A track row with `track_image_url` uses it. A track row without it uses `feed_image_url` |
| R47-05 | A feed hit without `feed_guid` opens the feed of its `entity_id` |
| R47-06 | Opening a row sends one detail request with the existing profile. A second open inside the reuse window sends none |
| R47-07 | A detail result for row A that arrives after the page changed to row B does not change the page of row B |
| R47-08 | The search request holds no `fuzzy` parameter |
| R47-09 | Name candidates come from the summary `release_artist` and `track_artist` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an Index search shows feed rows and track rows with titles, artists and artwork, faster than before.
- V2: opening a row shows a loading state, then the detail, in place.
- V3: a fast open of row A and then row B shows the detail of row B.
- V4: normal and narrow widths show each element in its place, in Light and Dark themes, with no clipped text.

## Exclusions

- No change to the Library search.
- No paging of the Index search.
- No change to the detail pages, except that they load on open.
- No stored value. A search result is not stored.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/api.rs`: `SearchResult`, `SearchResponse`, `Client::search`.
- `src/application/queries/search.rs`: `fetch_index_search_result_rows`, `fetch_index_feed_result_rows`, `fetch_index_track_result_rows`, `index_feed_display`, `index_track_display`, the name candidates.
- `src/application/request_profiles.rs` and the packet 018 shared owner.
- `src/view_models/search_results/`: `mod.rs`, `index_detail.rs`.
- `src/app/search_dispatch.rs`, `src/app.rs`: the Index feed and track detail arms.
- `src/app/publisher_dispatch.rs`: the stale-result pattern.
- `docs/notes/adr-0075-request-and-write-baseline.md`: the measurement form.

## Checks

```bash
cargo test --lib adr_0075_search_summary_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V4.
It states the needed state, what counts as wrong, and the cleanup. The check needs network access to `api.musicindex.org`.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0075-task-047-search-rows-from-summary-fields.md`
- ADR 0075 section 6 and Decision C, and the packet 017 and 018 documents
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": decode the summary, draw rows from it, load the detail on open, and stop the undeclared parameter.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use recorded JSON. No test sends a request to `api.musicindex.org`.
- The view model gives each label, loading state and failure state. The screen only composes. A screen uses the scaled tokens of ADR 0039 and existing shared composites.
- Background work uses the ADR 0040 runtime. Never call `cx.spawn` from a screen.
- Treat each MusicIndex response as untrusted input.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The Library search and the Library route.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R47-01 to R47-09 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0075_search_summary_`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo check --all-targets`
- `cargo build --bin v4vmm`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A detail page needs a field that the summary does not carry, and no open-time request can supply it.
- The shared request owner cannot serve an open-time request without a change to another profile.
- A change needs a file in "Do not touch".
