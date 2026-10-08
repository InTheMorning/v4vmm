# ADR 0084 Task 003: One Route For Each Track

Status: Ready - written on 2026-10-08. It runs after [task 002](adr-0084-task-002-one-release-route.md). Before the session starts, read the Implementation Result of task 002, and correct this packet where task 002 changed a name or a function.

## Goal

Each track has one navigation entry, keyed by its identity, whatever its origin. A Library track row in the content list opens its track page.
The origin selects only the data source of the page. A download changes the data source of the open page in place.

This packet fixes finding 2 of the [Music flow map](../architecture/music-flow-map.md).

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 4 and its Verification section. A track is identified by its item GUID and its feed GUID.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 9 and [task 001](archive/adr-0084-task-001-one-navigation-stack.md).
- [Task 002](adr-0084-task-002-one-release-route.md): `ReleaseKey` and the resolver method.
- The durable set in [AGENTS.md](../../AGENTS.md).

## Recorded Facts - 2026-10-08

- `FrameNavigationEntry` has two track entries: `TrackDetail(i64)`, keyed by the Library track id, and `IndexTrackDetail { id, label }`. The Index `id` is `<feed_guid>:<item_guid>`, or `<item_guid>` alone when the result has no feed GUID (`split_index_track_target` in `src/app/search_dispatch.rs`).
- The track page has one view model for both origins: `TrackDetailVm` in `src/view_models/track_detail.rs`, with `TrackDetailSurfaceContext::Library` or `Discover`. Two assembly paths stay:
  - Library: `render_library_track_detail` in `src/ui/shells/library/track_detail.rs`, from `LibraryDetail::Track`.
  - Index: `index_track_detail_slots` and `render_index_track_detail`, from `TopApp.index_track_detail_state`.
- The body switch renders `IndexTrackDetail` only when a search or a name-match page is open. Otherwise the entry falls to the Library mount, and `hydrate_detail_from_nav` clears the Library detail. The operator then sees the content list, not the track.
- A Library content-list row is a track. Its row id is the Library track id as text. `open_content_list_row` in `src/library/app_impl.rs` finds only Index release rows, so a click on a Library track row does nothing.
- `download_index_track` refreshes the Library and changes no page. `download_index_album_of_track` downloads the release of an Index track.
- `db::TrackRow` has `item_guid: String` and `feed_guid: Option<String>`. `tracks` has `UNIQUE(feed_id, item_guid)`. `db::find_track_id` keys on the feed URL, not the feed GUID. No query finds a track by its feed GUID and item GUID.
- These places push or match a track entry:
  - the sidebar track rows, the album track rows (`feed_detail.rs`) and the playlist track rows (`playlist_detail.rs`)
  - the Library track search results, the Index track search results and the name-match rows
  - `hydrate_detail_from_nav`
- These guards name the two entries: `adr_0049_inspector_source_ownership_is_guarded`, `adr_0024_index_track_detail_uses_rich_track_view_path`, `adr_0024_loading_shape_readiness_gate_is_locked` and `nav_top_drives_content_list_body_switch`.

## Design

```rust
/// The identity of one track (ADR 0084 Decision 4).
pub(crate) struct TrackKey {
    /// The release of the track. `None` only for an Index result with no
    /// feed GUID.
    release: Option<ReleaseKey>,
    /// The RSS item GUID.
    item_guid: String,
}

/// The data source of the open track page.
pub(crate) enum TrackPageSource {
    Library { track_id: i64 },
    Index { target: String },
}
```

- `FrameNavigationEntry::Track(TrackKey)` replaces `TrackDetail(i64)` and `IndexTrackDetail { id, label }`.
- The resolver gives the Library source when the Library holds the track: by feed GUID and item GUID for `FeedGuid`, and by feed id and item GUID for `LibraryFeed`. Otherwise it gives the Index source. A key with no release is always the Index source.
- The top app keeps the source of the open track page, with the method of task 002 for releases.
- The breadcrumb label is the track title: the Library title, the loaded Index title, or the label that the opening row gave. Without a label, the breadcrumb shows "Track".

## Required Changes

1. Add `TrackKey` and `FrameNavigationEntry::Track(TrackKey)`. Delete `TrackDetail` and `IndexTrackDetail`. Update `display_label` and the breadcrumb element id.
2. Add `TrackPageSource` and the resolver in `src/view_models/`. Add `TrackKey` constructors for a `TrackRow` and for an Index track target.
3. Add two read queries in `src/db.rs`. One finds a Library track id by feed GUID and item GUID. The other finds it by feed id and item GUID. Add no migration.
4. Each place in Recorded Facts pushes `Track(key)`.
5. A Library track row in the content list opens its track page in the current path. The content-list view model gives the open target of each row: a `ReleaseKey` for a release row and a `TrackKey` for a track row.
6. `restore_content_list_nav_entry` restores a track through the resolver. Delete the "search or name-match page is open" condition of the body switch. Keep the stale-result check of `index_track_detail_result_is_current`, and keep the name-match cache.
7. After `download_index_track` and `download_index_album_of_track` complete, the top app resolves the source of the open track page again. The page changes to the Library state in place. No navigation entry changes.
8. Update the guards in Recorded Facts. Delete the text that requires the two old entries. Extend the task 002 guard to the track entries.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_track_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R3-01 | The `TrackKey` of a Library track with a feed GUID equals the `TrackKey` of an Index target with the same feed GUID and item GUID |
| R3-02 | The `TrackKey` of a Library track in a feed with no feed GUID carries `ReleaseKey::LibraryFeed` |
| R3-03 | The resolver gives the Library source when the lookup finds the track, and the Index source when it does not. A key with no release gives the Index source |
| R3-04 | The content-list view model gives a `TrackKey` open target for a Library track row and a `ReleaseKey` open target for an Index release row |
| R3-05 | The two new read queries find a track by feed GUID and item GUID, and by feed id and item GUID, in an in-memory database |
| R3-06 | A guard blocks `TrackDetail(` and `IndexTrackDetail` in `src/`, blocks a navigation change in the two download completions, and names ADR 0084 Decision 4 |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: a click on a Library track row in the content list opens its track page.
- V2: one track opened from a playlist and from an Index search shows the same page layout, with its Library state.
- V3: "Download album" on an Index track page changes the page to the Library state in place. Back goes to the page before the track.
- V4: Back to an Index track page after the search closes shows that track, not the content list.
- V5: normal and narrow widths, Light and Dark.

## Exclusions

- No merge of the two page assembly paths into one renderer.
- No change to the name-match page and its rows, except the entry that a row pushes.
- No schema change and no new MusicIndex request.

## Files To Inspect

- [Agent rules](../../AGENTS.md), the [source map](../architecture/source-map.md), the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md) and the Implementation Result of [task 002](adr-0084-task-002-one-release-route.md).
- `src/view_models/workspace/nav.rs`, `breadcrumb.rs`, `tests.rs`.
- `src/view_models/track_detail.rs`, `src/view_models/library.rs` (`ContentListRowDisplay`, `index_feed_selection`), `src/view_models/name_match_page.rs`.
- `src/app.rs`, `src/app/breadcrumb.rs`, `src/app/search_dispatch.rs`, `src/app/name_match_dispatch.rs`.
- `src/library.rs`, `src/library/app_impl.rs`.
- `src/ui/shells/library/sidebar.rs`, `feed_detail.rs`, `playlist_detail.rs`, `content_list.rs`.
- `src/db.rs` (`find_track_id`, `track_row_by_id`).
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_track_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-003-one-track-route.md`
- The Implementation Result of `docs/tasks/adr-0084-task-002-one-release-route.md`
- ADR 0084 Decisions 4 and 9
- Each file in "Files To Inspect"

Goal:
- Replace `TrackDetail` and `IndexTrackDetail` with one `Track(TrackKey)` navigation entry and one data-source resolver, as "Design" and "Required Changes" give. A Library track row in the content list opens its track page.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Use the `ReleaseKey` and the resolver method of task 002. Do not make a second method.
- View models carry no GPUI type. The resolver is a pure function. The top app does the database lookup, never the render path.
- Keep each stale-result check of the Index fetch, and the name-match cache.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The release route of task 002, except to share its key and its method.
- The database schema and the migration registry.
- Any ADR.
- Each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R3-01 to R3-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V5. The check gives numbered steps, the expected result, an incorrect result, and the cleanup.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A track opens from a place that has no item GUID.
- Two Library tracks match one key.
- The Index page needs data that only the old `label` field carried.
- A change needs a file in "Do not touch".
