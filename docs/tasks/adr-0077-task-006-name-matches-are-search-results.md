# ADR 0077 Task 006: Name Matches Are Search Results

Status: Ready - 2026-09-28. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

The live Index search shows a name match as a search result, not as an artist. The result title is
`Tracks matching "<name>"`. Its page lists the tracks that MusicIndex gives for that name.
The result has no artist identity, no role and no page type. ADR 0077 Decision 1 and its accepted
refinement "Index artist page by name".

This packet replaces the name search of packet 005 (R5-03), which reached only parked code.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decision 1 and the accepted refinement "Index artist page by name".
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) packet 017: each MusicIndex request uses a named request profile.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, element hierarchy and token discipline.

## Recorded Facts - 2026-09-28

- `FetchIndexSearchResults` in `src/application/queries/search.rs` builds `IndexArtistCandidate` rows.
  A candidate comes from `release_artist` or `track_artist` of a search hit, when the name contains each query term.
  `merge_index_artist_candidates` merges candidates by lowercase name and adds their counts.
- `IndexArtistCandidate::into_display` gives an `ArtistResultDisplay` with the id `index-artist:<name>`, in the Artists tab.
- `handle_index_artist_result_selected` in `src/app/search_dispatch.rs` pushes `FrameNavigationEntry::IndexArtistFeedScope(name)`.
- `src/app.rs` renders that entry as the Index feed results of the active search query.
  The name selects no content. The page title is "Artist", and the breadcrumb shows the name.
- `api::Client::fetch_tracks_by_artist` sends `GET /v1/tracks?artist=<name>`.
  On 2026-09-28, `artist=Survival Guide` and `artist=survival guide` gave equal tracks. The match is the full name, without case.
  `artist=DETOX` gave no track, because the tracks state "Official DETOX Music".
- The live callers of `fetch_tracks_by_artist` are none. Its callers are in parked code: `enrich_artist_rows`,
  `fetch_artist_detail` for the `SearchApp` inspector, and `ApiSource` in `src/sources.rs`.
- The guard `index_artist_activation_is_scoped_feed_route_not_detail_page` in `tests/architecture_tests.rs`
  asserts the scoped feed route. It names no ADR.

## Required Changes

### 1. The Result Row

- An Index name candidate gives a row with the label `Tracks matching "<name>"`. Its accessibility label states the same text.
- The row keeps its secondary count text and its thumbnail.
- The view model owns the label text. The screen composes it.
- The row stays in its present tab. The Library name groupings in that tab do not change.

### 2. The Navigation Entry

- Rename `FrameNavigationEntry::IndexArtistFeedScope(String)` to a name that states a name match, for example `IndexNameMatches(String)`.
- The frame title is "Tracks matching". It is never "Artist".
- The breadcrumb shows the quoted name.

### 3. The Page

- The page sends one request: `GET /v1/tracks?artist=<name>`, through a named request profile of packet 017.
- It lists each returned track. A track row opens the existing Index track detail. That detail reaches the album, and the album reaches its publisher page (packet 004).
- It uses the ADR 0040 runtime. A result for an earlier name does not replace the page of a later name.
- When the response gives no track, the page states that MusicIndex gave no track for this exact name.
- When the request fails, the page gives a display state from the view model, not a transport error.
- When `pagination.has_more` is true, the page states that more tracks exist. This packet adds no paging.

### 4. The Old Route

- Delete the scoped feed-results rendering of the old entry.
- Replace the guard `index_artist_activation_is_scoped_feed_route_not_detail_page`. The new guard names ADR 0077 packet 006.
  It fails when a name-built Index row opens a page with the title "Artist".
- Keep the check that the retired name `IndexArtistDetail` does not return.

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_name_matches_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R6-01 | An Index name candidate exposes the label `Tracks matching "Survival Guide"` and an equal accessibility label |
| R6-02 | Activation of that row gives the renamed navigation entry with the name, and its frame title is "Tracks matching" |
| R6-03 | The page command requests `/v1/tracks` with `artist=<name>` through a named profile, and no other route |
| R6-04 | The page view model exposes one row for each returned track, and each row opens the Index track detail of its track |
| R6-05 | The page view model exposes an empty state for no track, a failure display state, and a "more tracks exist" state |
| R6-06 | A result for name A that arrives after the page changed to name B does not change the page of name B |
| R6-07 | The replaced guard names ADR 0077 packet 006 and fails for the old scoped feed route |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: a search for "Survival Guide" shows a row `Tracks matching "Survival Guide"`. The row does not look like an artist identity.
- V2: the row opens a page titled "Tracks matching" that lists the tracks of that name. A track opens its detail, and the album opens its publisher page.
- V3: a name with no exact match shows the empty state text.
- V4: normal and narrow window widths show each element in its defined place, in Light and Dark themes, with no clipped text.

## Exclusions

- No change to the Library "Grouped by name" view.
- No change to the tab labels or the tab order.
- No paging of the track list.
- No deletion of the parked `SearchApp` code. The phase plan records that finding apart.
- No change to the name matcher or to the candidate merge.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/application/queries/search.rs`: `IndexArtistCandidate`, `FetchIndexSearchResults`.
- `src/app/search_dispatch.rs`: `handle_index_artist_result_selected`, `sync_search_results_detail_with_nav`.
- `src/app.rs`: the content body switch and the frame title.
- `src/view_models/workspace/nav.rs`, `breadcrumb.rs` and `tests.rs`.
- `src/library/app_impl.rs`: the navigation entry arms.
- `src/view_models/search_results/`: `results.rs`, `index_detail.rs`.
- `src/api.rs`: `fetch_tracks_by_artist`. `src/application/request_profiles.rs`: the packet 017 profiles.
- `src/app/publisher_dispatch.rs`: the stale-result check pattern of packet 004.
- `tests/architecture_tests.rs`: `index_artist_activation_is_scoped_feed_route_not_detail_page`, `nav_top_drives_content_list_body_switch`, `adr_0049_inspector_source_ownership_is_guarded`.

## Checks

```bash
cargo test --lib adr_0077_name_matches
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

The implementer writes this section at completion, with numbered steps for V1 to V4, the needed state, what counts as wrong, and the cleanup.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0077-task-006-name-matches-are-search-results.md`
- ADR 0077 Decision 1 and its accepted refinements
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the row label, the renamed navigation entry, the track page and the replaced guard.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, title, empty state and failure state. The screen only composes them.
- No raw display literal and no glyph string in a renderer.
- Background work uses the ADR 0040 runtime. Never call `cx.spawn` from a screen.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The Library name grouping, the tab labels and the tab order.
- The parked `SearchApp` code in `src/discover.rs` and `src/discover/`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R6-01 to R6-07 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0077_name_matches`
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
- The packet 017 profile owner cannot name a profile for `/v1/tracks?artist=` without a change to another profile.
- A live caller other than the Index name row uses `IndexArtistFeedScope`.
- A change needs a file in "Do not touch".
