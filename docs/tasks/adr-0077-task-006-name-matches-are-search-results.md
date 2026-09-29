# ADR 0077 Task 006: Name Matches Are Search Results

Status: Implemented - 2026-09-29. Mechanical checks Green. Visual gate open and paused.

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

## Implementation Result

### Files

New files:

- `src/view_models/name_match_page.rs`: `NameMatchPageFacts` and `NameMatchPageVm`. The view
  model owns the page title, the track rows, the empty message, the failure message, and the
  "more tracks" message.
- `src/app/name_match_dispatch.rs`: `TopApp` owns the fetch and the navigation entry, through the
  ADR 0040 runtime. It follows the stale-result pattern of `publisher_page_result_is_current`.
- `src/ui/shells/name_match_page.rs`: the screen. It composes `NameMatchPageVm` and reuses the
  shared Index result row from `search_result_rows`.

Changed files:

- `src/view_models/workspace/nav.rs`, `breadcrumb.rs`, `tests.rs`: renamed `IndexArtistFeedScope`
  to `IndexNameMatches`. The breadcrumb label is now the quoted name.
- `src/library/app_impl.rs`: renamed the same entry in the Library's own breadcrumb and
  detail-reset match arms.
- `src/app/search_dispatch.rs`: an Index name candidate now opens the name-match page.
- `src/app.rs`: a new frame title ("Tracks matching") and a new content-list render arm for the
  page. It adds a name-match fallback for the shared Index track detail route, and the page's
  mounted state field.
- `src/app/breadcrumb.rs`: the breadcrumb-select handler now also restores the name-match page.
- `src/application/queries/search.rs`: the new `FetchNameMatchTracks` command and its one-request
  fetch function. The Index name candidate's label is now the quoted "Tracks matching" text, with
  an equal accessibility label.
- `src/application/request_profiles.rs`: a new path shape, `TracksByArtistName`, and a new named
  profile, `INDEX_NAME_MATCH_TRACKS`.
- `src/api.rs`: `fetch_tracks_by_artist_with_profile`, the named-profile form of the existing
  call.
- `src/ui/shells/search_results_inspector.rs`, `src/view_models/search_results/mod.rs` and
  `tests.rs`: removed `SearchResultsHeaderMode::Scoped` and `empty_state_for_scope`. See
  Deviations.
- `tests/architecture_tests.rs`: replaced the retired guard with
  `adr_0077_name_matches_index_row_opens_tracks_matching_not_artist_page`, and corrected three
  other guards that named the retired route or the retired header mode.

### Tests

- `cargo test --lib adr_0077_name_matches`: 11 passed. They cover R6-01 (the label and its equal
  accessibility label), R6-03 (the one request, through the named profile), R6-04 (one row for
  each track, with the existing detail's activation id), R6-05 (the empty, failure, and
  "more tracks" states), and R6-06 (the stale-result guard).
- `cargo test`: 1867 library tests and 283 architecture tests passed, 0 failed.
- `cargo test --test architecture_tests`: 283 passed, including the replaced guard (R6-07).
- `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets`: Green, no
  warning.
- `cargo build --bin v4vmm`: Green.

### Behavior

A search for a name shows a row labeled `Tracks matching "<name>"`, in the Artists tab, with its
existing count text and thumbnail. Its accessibility label states the same text. The row opens a
page titled "Tracks matching". Its breadcrumb shows the quoted name. The page lists each track
MusicIndex gives for that exact name. A track row opens the existing Index track detail, which
reaches the album and the album's publisher page.

An exact name with no track shows a message that names the exact name. A failed request shows a
failure message with its detail. A response can hold more tracks than the page lists. The page
states that fact, and this packet adds no paging.

### Deviations

- I named the navigation entry `IndexNameMatches`, the example name the packet gave.
- Packet 017 named ten fixed request profiles. This page's request needs a new path shape
  (`/v1/tracks?artist=`). I added `TracksByArtistName` and `INDEX_NAME_MATCH_TRACKS` to the same
  `request_profiles.rs` registry, the way packet 003 added `INDEX_PUBLISHER_PAGE`.
- Deleting the old scoped Index feed-results route left `SearchResultsHeaderMode::Scoped` and
  `SearchResultsInspectorPageVm::empty_state_for_scope` with no caller. I deleted both, under
  "Delete Dead Things." This also needed two small corrections in `tests/architecture_tests.rs`.
  Two guards, for ADR 0049 and ADR 0066, named the deleted mode or counted its wiring. I corrected
  each guard to its new, true count or requirement. I changed no rule of either ADR.
- The name-match page reads its own cached track row for the shared `IndexTrackDetail` route,
  in addition to the existing search-results cache. This keeps "opens the existing Index track detail"
  true when the operator did not reach the track from a search flow.

### Concerns

- V1's "does not look like an artist identity" and the page's visual weight need the operator's
  judgment. The page heading uses `SectionHeader`, not an entity header, so it carries no
  identity-like badge.
- The two corrected ADR 0049 and ADR 0066 guards sit outside this packet's named files. I
  corrected them because this packet's required deletion made their assertions false, not to
  change either ADR's rule.

## Operator Visual Check

**Setup**

1. Build and open the desktop binary:

   ```bash
   cargo build --bin v4vmm && target/debug/v4vmm
   ```

   Run this command first. A prior `cargo test` run can leave a GPUI test-support binary at
   `target/debug/v4vmm`. This step is the only one that starts the app.
2. Open Settings. Confirm the MusicIndex endpoint field holds a working endpoint.
3. In the Music tab, type a name into the toolbar search field that MusicIndex knows by name
   text, for example "Survival Guide" or "DETOX". Submit the search.

**V1 — the row states a search result, not an artist**

4. Open the Artists tab of the search results. Find the row for the typed name.
5. Read the row.
   - This result is wrong: the row states the bare name, with no quotes and no
     "Tracks matching" text.
   - This result is wrong: the row looks like an artist identity row, for example a
     person-shaped thumbnail placeholder shown nowhere else in this tab.

**V2 — the row opens the track list, and a track opens its album's publisher page**

6. Select the row. Read the page title and its breadcrumb.
   - This result is wrong: the page title or the breadcrumb states "Artist".
   - This result is wrong: the page shows a role, a page type, or another artist identity fact.
7. Read the page body. It lists each track MusicIndex gives for the typed name.
8. Select a track row. It opens the existing Index track detail page for that track.
9. From the track detail, open its album, then the album's publisher page.
   - This result is wrong: a step in this chain fails, or opens the wrong page.

**V3 — an exact name with no track**

10. Type a name that matches no track by its exact text, for example a real name with one
    misspelled letter.
11. Select its "Tracks matching" row.
    - This result is wrong: the page shows a blank area, with no message.
    - This result is wrong: the message does not name the exact typed name.

**V4 — normal and narrow widths, Light and Dark themes**

12. Repeat step 6 through step 7 at the normal window width, then at a narrow width. Pull the
    window edge until the Library sidebar collapses, or resize below the narrow-layout width
    named in the sidebar and toolbar runbooks.
13. Repeat step 12 in Light theme, then in Dark theme (Settings > Appearance).
    - This result is wrong: the page title, a track row, or the "more tracks" message (when
      shown) clips its text at either width or in either theme.
    - Color alone is not a valid difference between a correct result and an incorrect one.

**Cleanup**

14. Close the app window. This packet writes no file and stores no new row. No step undoes state.
    Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

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

## Orchestrator Review - 2026-09-29

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,867 unit tests, 283 guards, and no warning.

- The screen uses scaled tokens and the shared result row. The view model owns each text.
- A result for an earlier name does not change the page. `name_match_page_result_is_current` follows the pattern of packet 004.
- The new profile `INDEX_NAME_MATCH_TRACKS` is additive. It changes no other profile.
- The deletion of `SearchResultsHeaderMode::Scoped` is correct. No entry point reached it after this packet. Two guards lost only a requirement that named the deleted code.
