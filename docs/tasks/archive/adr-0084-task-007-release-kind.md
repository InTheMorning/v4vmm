# ADR 0084 Task 007: The Release Kind

Status: Complete - 2026-10-17. The mechanical checks are Green. The operator passed V1 to V6 on 2026-10-17.

## Goal

Each release shows its kind: Album, EP or Single. One pure function gives the kind of a release, for both origins. The release page, the release rows and tiles, and each action label that names a release use it, for example "Download single".

## Authority

- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 5: the kind comes from the value that the feed states, when it names album, EP or single. Without a stated value, a release with one track is a single, and any other release is an album. EP shows only when the feed states it.
- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 4: one page for each item, whatever its origin.
- [ADR 0060](../../adr/0060-workflow-surface-structure.md) Vocabulary: the curator sees "Release". Internal names do not change.
- The orchestrator split the old packet 007 on 2026-10-17. This packet owns Decision 5. Packet 015 owns the singles groups (Decision 6), and packet 016 owns the scopes (Decision 8).

## Recorded Facts - 2026-10-17

- `FeedView::release_kind` (`src/views.rs`) carries the stated value from MusicIndex (`FeedResponse.release_kind`, a free string). On 2026-10-17 the first 100 releases of `/v1/feeds/recent` all stated "unknown": 90 with one track and 10 with more. So the track-count rule decides the kind of most releases.
- `ReleaseDetailVm::summary_facts` (`src/view_models/entity_detail.rs`) shows the fact "Release Kind" with the stated value, or "Unknown". An Index release with one track thus shows "unknown".
- A Library release keeps the stated value in `AlbumNode::metadata_facts.release_kind`, and its tracks in `AlbumNode::tracks`. An Index release has `FeedView::episode_count` and `FeedView::tracks`.
- The content-list rows and tiles show the entity badge "Release" (`ContentListEntityBadgeDisplay::for_kind` in `src/view_models/library.rs`). The Releases destination rows show the release name and the artist (`LibraryViewModel::releases_list`).
- These action labels name a release as "album": `AlbumPageAction::DownloadAlbum` and `RemoveAlbum` (`src/view_models/album_page.rs`), `TrackPageAction::DownloadAlbum` (`src/view_models/track_detail.rs`), and the Index release page action in `index_feed_primary_actions` (`src/app/search_dispatch.rs`). Their accessibility labels say "this album".
- `TrackView` carries neither the stated kind nor the track count of its release. An Index track page thus does not know the kind of its release.

## Design

### One kind function

Add `src/view_models/release_kind.rs`:

```rust
/// The kind of a release (ADR 0084 Decision 5).
pub(crate) enum ReleaseKind { Album, Ep, Single }

impl ReleaseKind {
    /// The stated value wins when it names album, EP or single. Otherwise
    /// one track gives Single, and any other count gives Album.
    pub(crate) fn resolve(stated: Option<&str>, track_count: usize) -> Self;
    /// "Album", "EP", "Single".
    pub(crate) const fn label(self) -> &'static str;
    /// "album", "EP", "single", for an action label.
    pub(crate) const fn noun(self) -> &'static str;
}
```

- Compare the stated value after trim, case-insensitively. Accept "album", "ep" and "single". Each other value, for example "unknown", counts as not stated.
- A track count of 0 gives Album.
- The track count of a Library release is the length of `AlbumNode::tracks`. The track count of an Index release is `FeedView::tracks.len()` when the page has its tracks, else `FeedView::episode_count`.

### Where the kind shows

1. The release page fact "Release Kind" shows `label()`, for both origins.
2. The entity badge of a release row or tile shows `label()` in place of "Release". The badge kind, its color and its icon do not change.
3. The Releases destination rows show the kind after the artist, for example "Haleen · Single". The separator comes from the view model.

### Action labels

- The release page actions use the noun: "Download single", "Download EP", "Download album", "Remove single…" and the matching accessibility labels, for example "Download this single".
- A track page action that names the release of the track uses the kind of that release when the page knows it. A Library track page knows its release in the Library tree. When a page does not know the kind, the label says "release", for example "Download release". An Index track page uses "release" in this packet.
- Each label stays a view-model value. A typed action keeps its availability, its accessibility label and its id. Only the text depends on the kind.

## Required Changes

1. Add `ReleaseKind` and its tests.
2. Give the release page fact, the badge and the Releases rows the kind, as "Where the kind shows" gives.
3. Give the action labels the kind, as "Action labels" gives. Change each label function that needs the kind to take it.
4. Update each test that asserts the old text "Release", "Unknown", "Download album" or "Remove album…". Add the tests of the mechanical criteria.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_release_kind_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R7-01 | `ReleaseKind::resolve`: the stated values "Album", "ep" and " Single " win over the count. "unknown" with one track gives Single, and with 5 tracks gives Album. No stated value with 0 tracks gives Album. A count never gives EP |
| R7-02 | `ReleaseDetailVm::summary_facts` gives "Single" for an Index feed that states "unknown" and has one track, and "Album" for a Library album with 5 tracks |
| R7-03 | A release row in the content list has the badge label "Single" for a one-track release, and the badge kind stays the release kind |
| R7-04 | `releases_list` gives the secondary text "artist · Single" for a one-track Library release |
| R7-05 | The release page download action label is "Download single" for a single and "Download EP" for a stated EP. Its accessibility label names the same kind |
| R7-06 | A track page download action label is "Download release" when the kind is not known, and "Download single" when the page gives a single |
| R7-07 | A guard names ADR 0084 Decision 5: no "Download album" literal and no "Remove album" literal outside `src/view_models/release_kind.rs` and tests |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an Index release with one track shows "Release Kind: Single" on its page, and its download action says "Download single".
- V2: an Index release with more tracks shows "Album" and "Download album".
- V3: in "New releases", each row and tile badge says "Single" or "Album".
- V4: the Releases list shows the kind after each artist.
- V5: a Library single page shows "Remove single…" in its menu.
- V6: Light and Dark, and a larger scale, show no clipped badge or label.

## Exclusions

- No singles groups and no group page. Packet 015 owns them.
- No scope control. Packet 016 owns it.
- No new MusicIndex request. The Index track page uses "release".
- No change to the stored data and no schema change.
- No change to an internal name, such as `AlbumNode` or `DownloadAlbum`.

## Files To Inspect

- [Agent rules](../../../AGENTS.md), the [source map](../../architecture/source-map.md) and the [phase plan](../../plans/adr-0084-music-column-browser-phase-plan.md).
- `src/views.rs` (`FeedView`, `TrackView`), `src/view_models/entity_detail.rs`, `src/view_models/feed.rs`.
- `src/view_models/library.rs` (`AlbumNode`, `ContentListEntityBadgeDisplay`, `releases_list`, the content rows from recent feeds).
- `src/view_models/album_page.rs`, `src/view_models/track_detail.rs`.
- `src/ui/shells/library/feed_detail.rs`, `src/ui/shells/library/track_detail.rs`, `src/ui/shells/library/content_list.rs`, `src/ui/shells/library/destination_lists.rs`.
- `src/app/search_dispatch.rs` (`index_feed_primary_actions`, `index_track_page_actions`).
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test adr_0084_release_kind_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last, after every test run. A test run can leave a GPUI test-support binary at `target/debug/v4vmm` (AGENTS.md, "Build, Test, Lint").

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-007-release-kind.md`
- ADR 0084 Decisions 4 and 5, and the Vocabulary section of ADR 0060
- Each file in "Files To Inspect"

Goal:
- One `ReleaseKind` function, and the kind on the release page, the release badges, the Releases rows and the action labels, as "Design" gives.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- View models carry no GPUI type. Each label and each separator is a view-model value. The renderer decides no text.
- Do not rename an internal type, function or action variant.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The navigation model in `src/view_models/workspace/`.
- The scroll helpers of task 014 and the description display of task 013.
- The search results view and its chips.
- The Settings and Show screens.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R7-01 to R7-07 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V6. The check gives numbered steps, the expected result, an incorrect result, and the cleanup. It names releases from "New releases" for V1 and V2, or tells the operator how to find them.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A label that names a release has no view-model owner, and only a renderer gives it.
- A page needs the kind but has neither the stated value nor a track count.
- A stored or exported value, for example a file tag or a database field, would change.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-17

Each Required Change is done.

### Files Changed

- `src/view_models/release_kind.rs` (new file): Adds `ReleaseKind`, its `resolve`, `label` and `noun`
  methods, and the R7-01 tests.
- `src/view_models/mod.rs`: Adds the `release_kind` module.
- `src/views.rs`: Adds `FeedView::track_count`. The method reads the loaded track list first. With
  no loaded list, it reads the stated episode count.
- `src/view_models/entity_detail.rs`: `ReleaseDetailVm::summary_facts` now names the resolved kind,
  for both origins, in place of the raw stated value or "Unknown". Adds the R7-02 test. Fixes two
  existing tests that named the old raw text.
- `src/view_models/library.rs`:
  - Adds `AlbumNode::release_kind`.
  - `releases_list` now names the kind after the artist, for example "Haleen · Single".
  - `ContentListEntityBadgeDisplay::for_kind` now reads the row kind, not only the entity kind. A
    release row's badge label names its resolved kind. The badge kind, its color and its icon stay
    the row's entity type.
  - Adds the private function `release_kind_of_feed_result` and the method
    `ContentListRowKind::badge_label`.
  - Adds the R7-03 and R7-04 tests. Fixes two existing tests that named the old text.
- `src/view_models/album_page.rs`: `AlbumPageAction::label` and `a11y_label` now take the release
  kind, and each builds its text from `ReleaseKind::noun`. Both methods change from `pub` to
  `pub(crate)` (Deviation 1). Adds the R7-05 test. Fixes one existing test.
- `src/view_models/track_detail.rs`: `TrackPageAction::label` and `a11y_label` now take an optional
  release kind. With no known kind, the label says "Download release". Both methods change from
  `pub` to `pub(crate)` (Deviation 1). Adds the R7-06 test. Fixes one existing test.
- `src/ui/shells/library/feed_detail.rs`: The Library album page passes the album's resolved kind
  to each action label and each accessibility label.
- `src/ui/shells/library/track_detail_metadata.rs`: The Library track page passes no known release
  kind to each track action label (Deviation 2).
- `src/app/search_dispatch.rs`: `index_feed_primary_actions` passes the Index release's resolved
  kind to each action label. `index_track_page_actions` passes no known kind, since `TrackView`
  carries no stated kind and no track count of its release.
- `tests/architecture_tests.rs`: Adds the R7-07 guard. It forbids the literal `"Download album"`
  and `"Remove album…"` in production code outside `release_kind.rs`.

### Behavior Changed

- A release page names its kind, Album, EP or Single, in place of the raw stated value or
  "Unknown".
- A release row or tile badge names the kind, in place of "Release". The badge color and icon stay
  the same.
- The Releases list names the kind after each artist.
- A release page's download and removal actions name the kind, for example "Download single" and
  "Remove single…".
- A track page's "Download …" action names the release kind when the page knows it. An Index
  track page still says "Download release", since it has no known kind.

### Checks

| Command | Result |
|---|---|
| `cargo test adr_0084_release_kind_` | Green, 8 tests |
| `cargo test` | Green: 1,882 unit tests, 315 guards, 10 ignored doc tests |
| `cargo test --test architecture_tests` | Green, 315 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. `AlbumPageAction::label`/`a11y_label` and `TrackPageAction::label`/`a11y_label` change from
   `pub` to `pub(crate)`. `ReleaseKind` is `pub(crate)` (the "Design" section names this). A `pub`
   method with a `pub(crate)` parameter type gives a `private_interfaces` warning, which the
   "Zero warnings" rule forbids.
2. `src/ui/shells/library/track_detail_metadata.rs` passes `None` for the release kind at each
   call of `TrackPageAction::label`/`a11y_label`. The Library track page's own code gives
   `TrackPageAction::DownloadAlbum` only when `page_actions` has no Library state, which never
   happens on this page: `render_library_track_window` always calls `with_library_state`. A
   Library track that is not downloaded shows "Download track" instead, and a downloaded track
   shows "Add to playlist". So this page never renders "Download album" or "Download release" for
   a track action, and the kind passed here changes no visible text today.
3. This packet does not record the open V1 to V6 gate in `docs/pending-human-checks.md` or in
   `docs/plans/broadcast-chain-delivery-order.md`. The "Do not touch" list names "each document
   other than this packet". Packets 004 to 006 and 013 recorded the same scope limit, in their own
   deviations.

### Unresolved Concerns

- The two files named in Deviation 3 still need the open V1 to V6 gate recorded, outside this
  packet's own editable scope.
- No person walked V1 to V6 yet. A green mechanical suite gives no evidence about them.

## Operator Visual Check

This check reads the app only. It needs a Linux desktop session, this checkout, and a reachable
MusicIndex endpoint. V2 needs one Index release with more than one track. V5 needs one Library
release with one track.

On 2026-10-17 the first rows of "New releases" held "My Love" (NeilLAmRobot, one track) and
"Catena Aurea" (Contra, seven tracks). "Chatti und Claude" (one track) is in the operator's
Library. Use other releases with the same track counts when these have moved down.

1. Close v4vmm. Make the binary.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app.

   ```bash
   target/debug/v4vmm
   ```

**V1 - an Index release with one track names itself a Single**

3. Open Discover. Open "My Love", or another Index release with one track.
4. Look at its "Release Kind" fact.
   - Correct: it says "Single".
   - Incorrect: it says "Album", "Unknown" or the raw stated value.
5. Look at its download action.
   - Correct: the button says "Download single".
   - Incorrect: the button says "Download album".

**V2 - an Index release with more tracks names itself an Album**

6. Open "Catena Aurea", or another Index release with more than one track.
7. Look at its "Release Kind" fact and its download action.
   - Correct: the fact says "Album", and the button says "Download album".
   - Incorrect: either one says "Single" or names a different word.

**V3 - "New releases" rows and tiles name each release kind**

8. Return to "New releases". Look at the badge of each row or tile.
   - Correct: each badge says "Single" or "Album", matching the track count of V1 and V2.
   - Incorrect: a badge says "Release".

**V4 - the Releases list names the kind after the artist**

9. Open the Releases destination. Look at the row of a release you downloaded or already had.
   - Correct: the row's second line reads "artist · kind", for example "Haleen · Single".
   - Incorrect: the row shows the artist alone, with no kind.

**V5 - a Library single page shows "Remove single…" in its menu**

10. Open Releases, then "Chatti und Claude", or another Library release with one track.
11. Open the "⋯" menu of its page.
    - Correct: the menu's last item reads "Remove single…".
    - Incorrect: the item reads "Remove album…".

**V6 - Light, Dark and a larger scale show no clipped badge or label**

12. Open Settings, then General (`Ctrl+Comma`). Select Light, then Dark, then a larger scale.
    Return to the release pages of V1 and V2 after each change.
    - Correct: each badge, fact and button stays fully visible, with no clipped text in any
      combination.
    - Incorrect: a badge, fact or button is clipped, cut off, or overlaps another element.
13. Close the app.

**Clean-up**

None. This check reads the app only. It writes no file and changes no stored data. Set the theme
and the scale of V6 back to your usual values.

## Orchestrator Review - 2026-10-17

The orchestrator reviewed the diff against ADR 0084 Decisions 4 and 5, this packet and the [review checklist](../../reviews/adr-0084-review-checklist.md). The orchestrator also ran each check.

- `ReleaseKind`, the release page fact, the badge, the Releases rows and the action labels follow "Design". The badge keeps its kind, its color and its icon.
- Deviation 1, the change of two label functions from `pub` to `pub(crate)`, prevents a compiler warning. It changes no caller.
- Deviation 2 is correct. A Library track page never shows "Download album", because a Library track has a Library state. So the kind of its release changes no visible text, and the page passes `None`.
- The "New releases" rows carry the full feed detail of each release, so their badge has the track count. `fetch_recent_feed_result_rows` in `src/application/queries/feed.rs` reads that detail with one request for each release, one after the other. Responsiveness item 10 records this cost.
- The visual check now names "My Love", "Catena Aurea" and "Chatti und Claude", so V5 needs no download, and the clean-up changes no data.
- The orchestrator recorded the open gate in the pending checks and in the delivery order.

Merge recommendation: pass. Packet 015, the singles groups, comes after the visual check.

## Operator Check - 2026-10-17

The operator walked V1 to V6 with a normal debug binary, and each step passed.
