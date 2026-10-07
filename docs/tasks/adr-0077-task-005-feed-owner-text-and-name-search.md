# ADR 0077 Task 005: Feed Owner Text And Name Search

Status: Open - implementation and mechanical checks are complete on 2026-09-26. On 2026-10-07 the operator moved its visual check to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07). The packet that rebuilds the surface carries it.

## Goal

Show `publisher_text` as feed owner text that opens no page. Remove the `publisher_text` inspector.
Change the Index artist rows that come from name text into a search. Label the Library name grouping.

The orchestrator divided the earlier packet 004 into packets 004 and 005 on 2026-09-26.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 1, 2 and 6, and its accepted refinements.
- The publisher relationship request records that `/v1/publishers` groups feeds by `itunes:owner`. On 2026-09-24, "Wavlake" held 7,352 feeds.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, typed action state and "Delete dead code".

## Recorded Facts - 2026-09-26

The `publisher_text` inspector has these code paths:

- `api::Client::fetch_publisher` and the `"publisher"` arm of the entity detail fetch in `src/api.rs`.
- The `"publisher"` arm of the inspector detail query in `src/application/queries/feed.rs`.
- `EntityDetail::Publisher` in `src/view_models/search/results.rs`, `src/discover/app_impl.rs` and their callers.
- The `"publisher"` skeleton in `src/ui/shells/discover/feed_inspector.rs`.

`artist_rows_from_result_rows` in `src/view_models/search/results.rs` builds artist rows from name text.

## Required Changes

### 1. Feed Owner Text

`publisher_text` shows as "Feed owner" text on the album page. It opens no page.

### 2. Remove The Inspector

List each entry point that opens the `/v1/publishers/{publisher_text}` inspector. Remove each one.
Then delete each code path that no entry point reaches.
This includes `api::Publisher` and `fetch_publisher` when nothing else uses them.

### 3. Name Search

Each Index row that `artist_rows_from_result_rows` builds opens a search result with the title "Tracks matching" and the quoted name.
The row has no artist identity, no role and no page type. Each result row links to its album, and through the album to its publisher page.

### 4. Name Grouping

An album without an owned publisher relationship keeps the Library name grouping. The grouping title shows "Grouped by name".

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_feed_owner_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R5-01 | `publisher_text` is exposed as feed owner text with no action |
| R5-02 | No entry point opens the `/v1/publishers/{publisher_text}` inspector. A guard names ADR 0077 Decision 6 and the fix |
| R5-03 | A name-built Index row exposes a search result title with the quoted name, and no artist identity |
| R5-04 | A Library name grouping exposes the "Grouped by name" label |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: `publisher_text` shows as feed owner text and opens nothing, in Light and Dark themes.
- V2: a name search result shows "Tracks matching" and the name, and opens no artist page.
- V3: the Library name grouping shows "Grouped by name".
- V4: normal and narrow window widths show each element in its defined place, with no clipped text.

## Exclusions

- No change to the publisher page or its navigation. Packet 004 owns them.
- No new request and no stored data.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- Each file in "Recorded Facts".
- `src/view_models/search/feed_detail.rs` and the album page view model.
- `src/library/app_impl.rs`: the Library artist grouping.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0077_feed_owner
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Implementation Result - 2026-09-26

### Feed Owner Text (R5-01)

`ReleaseHeroVm::display` in `src/view_models/entity_detail.rs` labels the `publisher_text` row
"Feed owner". This change replaces the label "Publisher" with "Feed owner". The row shows a label
and a value only. It has no click handler, and it opens no page.

This view model backs the Library album page and the Index album page. The fix reaches each
surface.

### Inspector Removal (R5-02)

The search for a click that opens the `/v1/publishers/{publisher_text}` inspector found none. No
code in the repository sends `entity_type = "publisher"` to `push_inspector` or `load_inspector`.
The surviving code was dead plumbing only. This packet removes it:

- `api::Client::fetch_publisher`, the `"publisher"` arm of `Client::fetch_detail`, `api::Publisher`,
  `api::PublisherSearchResponse`, and `Client::search_publishers` in `src/api.rs`. Nothing called
  `search_publishers` before this change. This packet removes it too, because it depends on
  `api::Publisher`.
- `Client::fetch_wrapped`. `fetch_publisher` was its only caller.
- The `"publisher"` arm of `fetch_inspector_detail` and `InspectorDetailData::Publisher` in
  `src/application/queries/feed.rs`.
- `InspectorDetail::Publisher` in `src/discover.rs`, and each match arm that named it in
  `src/discover/app_impl.rs`.
- `render_publisher_inspector` and its `PublisherInspectorVm`, `DetailHeader`, and `DetailGrid`
  wiring in `src/ui/shells/discover/feed_lists.rs`.
- The `"publisher"` skeleton arm in `src/ui/shells/discover/feed_inspector.rs`.
- `PublisherInspectorVm` in `src/view_models/search/feed_detail.rs`.

The regression test `adr_0077_feed_owner_publisher_text_inspector_stays_removed`, in
`tests/architecture_tests.rs`, is the R5-02 guard. It names ADR 0077 Decision 6 and the fix. It
fails if a removed identifier reappears in the source tree.

### Name Search (R5-03)

`ResultRowVm::artist_display`, in `src/view_models/search/results.rs`, titles a name-built row
`Tracks matching "<name>"`. The underlying `Artist` record can carry an area and active years. The
row shows no such fact. `ResultRow::inspector_title()` reuses this same title.

A pushed inspector frame reads `Tracks matching "<name>"` in its breadcrumb. The row keeps its
existing call, `fetch_tracks_by_artist` (`/v1/tracks?artist=<name>`). This change adds no new
call.

**Result found during the search, in files this packet does not name.**
`artist_rows_from_result_rows` belongs to `src/discover.rs` (`SearchApp`). That module carries
`#![allow(dead_code)]`, in ADR 0023. It has no entry point from `src/app.rs`'s `TopApp`. The search
confirmed this fact: nothing in the repository constructs `SearchApp`. No click handler sends
`entity_type = "artist"`, built from this function, to a live screen.

The fix is correct, and it matches the packet's own named target. It gives no benefit to a person
who opens the built app today.

The live Index name-search feature is a different, unaffected code path.
`FrameNavigationEntry::IndexArtistFeedScope`, in `src/view_models/workspace/nav.rs`, comes from
`IndexArtistCandidate` in `src/application/queries/search.rs`. Today this feature shows the plain
name as an "Artist" frame title, in `src/app.rs`'s `content_list_frame_title`.

When the search result cache holds no data, the app shows this frame through the Library artist
screen (`src/library/app_impl.rs`, `hydrate_detail_from_nav`). That is the same surface an owned
artist identity opens through. This conflicts with ADR 0077 Decision 1, and it needs its own
packet.

This packet's "Files To Inspect" list does not name `src/app.rs`,
`src/application/queries/search.rs`, `src/view_models/search_results/`, or
`src/view_models/workspace/nav.rs`. This change leaves them as they are, and does not widen the
task.

### Library Name Grouping (R5-04)

`LibraryArtistDetailVm::page()`, in `src/view_models/library.rs`, sets the page subtitle to
"Grouped by name". `src/ui/shells/library/feed_list.rs` renders this subtitle through the existing
`DetailHeader` contract. That file needed no change.

### Files Changed

- `src/api.rs`
- `src/application/queries/feed.rs`
- `src/discover.rs`
- `src/discover/app_impl.rs`
- `src/discover/tests.rs`
- `src/ui/shells/discover/feed_inspector.rs`
- `src/ui/shells/discover/feed_lists.rs`
- `src/view_models/entity_detail.rs`
- `src/view_models/library.rs`
- `src/view_models/search/feed_detail.rs`
- `src/view_models/search/mod.rs`
- `src/view_models/search/results.rs`
- `src/view_models/search/tests.rs`
- `tests/architecture_tests.rs`
- This packet document.

### Tests

Each command in "Checks" is Green.

- `cargo test --lib adr_0077_feed_owner`: 3 tests pass.
- `cargo test`: 1,848 library tests pass, 283 architecture-guard tests pass, and 10 doc-tests stay
  ignored. All Green.
- `cargo test --test architecture_tests`: 283 tests pass. The new R5-02 guard is in that set.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green, with no warning.
- `cargo build --bin v4vmm`: Green.

### Deviations From The Task

The packet's "Files To Inspect" list does not name `src/view_models/artist.rs` or
`src/ui/shells/artist.rs`. These files own the body of the page a name-built row opens: the
`ArtistVm` title, the "Feeds with tracks by this artist" subtitle, and the "Sort Name", "Area",
and "Active" rows. This change leaves them as they are.

Each of those three rows stays missing from the rendered page. A name-built row sets no
`sort_name`, `area`, `begin_year`, or `end_year`, so the page has no value to show in them. Only
the page's own title text (the plain name, with no "Tracks matching" wording) stays different from
the corrected breadcrumb above it. Closing that difference needs a change to `ArtistVm::title()`
and `ArtistVm::subtitle()`, in a file this packet does not name.

### Unresolved Concerns

- The result found during the search, above: the live `IndexArtistFeedScope` flow shows a
  name-built result as an "Artist" title, and can show it through the Library artist screen. It
  needs its own packet.
- The `ArtistVm`/`render_artist_view` body, in the corrected but unreachable `SearchApp`
  inspector, shows the plain name. See "Deviations" above.
- Cleanup of the evidence fixture `/tmp/v4vmm-governance.ie6k8TQf` stays unconfirmed. This is
  unrelated to this packet. The standing note in `AGENTS.md` records this fact.

## Operator Visual Check

V1 and V3 use the open app. V2 has no path to walk: its code has no entry point from the open app.
See the result found during the search, above. Report V2's gate as open, not as met, until a next
packet reaches the live `IndexArtistFeedScope` flow.

**Setup**

1. Make and open the desktop binary:

   ```bash
   cargo build --bin v4vmm && target/debug/v4vmm
   ```

   Do this command first. A prior `cargo test` run can keep a GPUI test-support binary at
   `target/debug/v4vmm`. This is the only step that starts the app. Each next step happens in the
   open app.
2. Open Settings. Make sure the MusicIndex endpoint field is set, and the service answers.
3. Make sure one feed in the Library carries an `itunes:owner` value. If not, open an Index album
   with a feed that carries one. A Wavlake feed always carries one.

**V1 — Feed owner text opens no page**

4. Open a Library album with an owner name on its feed. Or open an Index album through toolbar
   search. Do this in Light theme, then in Dark theme (Settings > Appearance).
5. Look at the row below the album title. It reads "Feed owner", with the owner name as its
   value.
   - This result is incorrect: the row keeps the label "Publisher".
   - This result is incorrect: a click on the row's text does anything, such as opening a page,
     showing a pointing-hand cursor, or a hover highlight.
6. Repeat step 4 at a narrow window width. Pull the window edge until the Library sidebar
   collapses, or resize below the narrow-layout width in the sidebar and toolbar runbooks. The
   "Feed owner" row shows in full, with no clipped text, in each theme.

**V2 — Name search: record why the gate stays open**

7. This criterion names `src/view_models/search/results.rs`'s function
   `artist_rows_from_result_rows`. The app reaches that function only through `src/discover.rs`'s
   `SearchApp`. Make sure the built app has no menu, tab, or button that opens that screen.
   - When a next change wires that screen into the open app, walk this gate then.
   - Type a query that matches only by name text.
   - Read the result title as `Tracks matching "<name>"`.
   - Make sure it opens no artist identity page.
   - Until that walk happens, report this gate as open, not as met.

**V3 — Library name grouping shows its label**

8. In the Library, find or add an artist name group with no publisher relationship. A local artist
   name with no linked publisher feed meets this condition. Most feeds added before ADR 0077 meet
   it. Select that name from the Library sidebar tree.
9. Look below the artist name, at the top of the page. It reads "Grouped by name".
   - This result is incorrect: no subtitle shows below the name.
   - This result is incorrect: the subtitle reads anything else, such as a caption from before
     this change.
10. Repeat steps 8 and 9 in Dark theme, and again at the narrow window width from step 6. The
    "Grouped by name" subtitle stays on one line, with no clipped text, in each theme and each
    width.

**Cleanup**

11. Close the app window, or press `Ctrl+C` in the terminal. This check reads existing Library and
    Index data. It writes no configuration and no stored file. No fixture and no generated file
    needs removal.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0077-task-005-feed-owner-text-and-name-search.md`
- `docs/adr/0077-publisher-feed-artist-binding.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": feed owner text, the inspector removal, the name search, and the name grouping label.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model decides each label and action availability. The screen only composes them.
- Delete code that no entry point reaches. Do not park it.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The publisher page, its navigation and `src/view_models/publisher_page.rs`. Packet 004 owns them.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R5-01 to R5-04 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0077_feed_owner`
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
- Another feature uses `api::Publisher` or `fetch_publisher`.
- A change needs a file in "Do not touch".
- The name search needs a new request.

## Orchestrator Review - 2026-09-26

The orchestrator reviewed the diff and ran the checks. R5-01, R5-02 and R5-04 reach the live app, and each check is Green.

R5-03 does not reach the live app. The orchestrator confirmed the implementer's finding:

- `artist_rows_from_result_rows` has callers only in `fetch_search_batch`, `fetch_partitioned_search_batch` and `fetch_artist_search_batch`.
- Only `FetchDiscoverSearchResults` reaches them, and only `SearchApp` uses that command. No code outside `src/discover` constructs `SearchApp`, and `src/discover.rs` carries `#![allow(dead_code)]`.
- The live Index search, `FetchIndexSearchResults`, builds name-based artist candidates (`IndexArtistCandidate`). They open `FrameNavigationEntry::IndexArtistFeedScope`, a page keyed by name text.

This packet named the wrong function. That was an error in the packet, not in the implementation.
ADR 0077 packet 006 owns the live name-keyed artist page. V2 of this packet cannot be walked, and packet 006 replaces it.
The parked `SearchApp` module is a separate finding in the phase plan.
