# ADR 0084 Task 023: One Track Page Assembly

Status: Ready - 2026-10-20

## Goal

The Library and the Index build the track page through one view model and one shell function.
The origin selects only the data source and the commands. A section shows when its data exists, not because of the origin.

This packet also corrects one defect. The Index track page shows no feed identity section and no "Feed publication date" row. After this packet, it shows them when its feed gives the data.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 4: one page for each item. The origin selects the data source of the page, not its route or its layout.
- [ADR 0083](../adr/0083-design-language.md) Decision 5: the action hierarchy. Decision 8: one "Inspect sources" entry on a track page.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision B and packets 022 and 050: the feed identity section and the "Feed publication date" row.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability. View models carry no renderer type, and screens stay thin.
- The [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md), row 023. The design of packet 021 found the second assembly path on 2026-10-20.
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md): the triage found the feed identity defect. Packet 023 owns the fix.

## Recorded Facts - 2026-10-20

- The track page has one view model type, `TrackDetailVm` (`src/view_models/track_detail.rs`), and one surface builder, `track::build_track_detail_surface` (`src/ui/shells/track.rs`).
- The Library has its own assembly path:
  - `render_library_track_detail` and `render_library_track_window` in `src/ui/shells/library/track_detail.rs` read the `InspectorFrame` and build the action row.
  - `render_library_track_detail_core` in the same file fills `TrackDetailBehaviorSlots` with `TrackDetailSurfaceContext::Library`.
  - `render_library_track_detail_actions`, `render_track_page_button`, `render_track_page_menu` and `run_track_page_action` are in `src/ui/shells/library/track_detail_metadata.rs`.
  - `render_library_track_detail_metadata` in the same file adds the "Inspect sources" disclosure below the page.
- The Index has its own assembly path:
  - `render_index_feed_or_fallback_detail` in `src/app/search_dispatch.rs` calls `index_track_detail_slots`, then `render_index_track_detail`.
  - `index_track_detail_slots`, `index_track_page_actions` and `index_track_description_panel` are in `src/app/search_dispatch.rs`.
  - `render_index_track_detail` in `src/ui/shells/search_results_inspector.rs` builds a second `TrackDetailVm` with `TrackDetailSurfaceContext::Discover` and no named feeds.
- The defect: no Index path calls `TrackDetailVm::with_feed_identity`. Thus `feed_identity_section` and `feed_publication_date_display` give `None` on each Index track page.
- `with_feed_identity` takes an `api::Feed`. The Index track page has no `api::Feed`. `FetchIndexTrackDetail` (`src/application/queries/search.rs`) gives a `TrackView` only.
- `FetchIndexFeedDetail` gives a `FeedView`. `FeedView::from_api` (`src/views.rs`) keeps the feed website and Nostr key in `identity`, and the feed publication date in `published` and `published_source`.
- `owner_fetch_feed` reads each feed through the shared request reuse. "Download album" and "Copy feed URL" on an Index track page read the same feed.
- `TrackDetailSurface` (`src/ui/composites/track_detail_surface.rs`) reads `primary_actions_a11y_label` and then drops it. No element shows it.
- The Library page puts its controls in an `ActionRow` with `control_group`, with the label "Track actions" and a status message. The Index page gives the bare button and the bare menu.
- The Library container has the padding `spacing::LG` (16 px, not scaled). The Index container has `Spacing::MD.scaled(cx)` (12 px, scaled). The release page has `Spacing::LG.scaled(cx)`.
- `track_detail_metadata.rs` has 530 lines. The ADR 0038 ceiling of `src/ui/shells/library` is 500 code lines.
- After packet 022, Index downloads go through the queue (`src/library/downloads.rs`). The queue drops a second job of the same release.
- `TopApp::finish_index_download` calls `show_downloaded_index_feed`. That function calls `apply_track_page_source`, which changes an open track page of the release to the Library in place.
- Guard R83-41, `adr_0083_both_track_page_origins_build_one_shared_surface` in `tests/architecture_tests.rs`, pins both paths. Eleven other guards name a function or a file of the two paths. "Files To Inspect" lists them.

### The Differences Between The Two Contexts

Each use of `TrackDetailSurfaceContext`, and each difference of the two assembly paths:

| # | Difference | Library | Index |
|---|---|---|---|
| 1 | Feed identity section and "Feed publication date" row | Shown when the feed gives the data | Never shown |
| 2 | Album name link target (`TrackDetailVm::name_links`) | `LibraryAlbum(album_feed_id)` | `IndexAlbum { feed_guid }` |
| 3 | `identity_action_prefix` | `library-track` | `discover-track` |
| 4 | `feed_identity_action_prefix` | `library-track-feed` | `discover-track-feed` |
| 5 | `primary_actions_a11y_label` | "Library track actions" | "Discover track actions" |
| 6 | `TrackDetailLabels::release_label` | "Release" | "Release" |
| 7 | Action row | `ActionRow`, label "Track actions", status message | Bare button and bare menu |
| 8 | Action set (`page_actions`) | By the Library state | "Download album" and "Copy feed URL" |
| 9 | "Inspect sources" | Shown | Absent |
| 10 | Description | Trimmed text, Library state store | Trimmed text, Index state store |
| 11 | Title override | The frame title | None |
| 12 | Named feeds | The album feed of the Library | `TrackView.named_feeds` |
| 13 | Cover | The local image cache | The remote image URL |
| 14 | Container padding | `spacing::LG` | `Spacing::MD.scaled(cx)` |
| 15 | Busy state of the filled action | Disabled while busy or while a release download runs | No busy state |
| 16 | Element ids of the actions, the menu, the description and the container | `track-page-*`, `library-track-description` | `index-track-page-*`, `index-track-description:{target}` |

## Design

### The view model

Add `src/view_models/track_page.rs`. It has no renderer type.

- `TrackFeedFacts` is the origin-neutral input of the feed. It holds the feed GUID, the title, the feed URL, the website, the Nostr key and the publication date with its source.
  - `TrackFeedFacts::from_api_feed(&Feed)` reads the Library feed through `feed_website`, `feed_nostr` and `feed_publication_pubdate`.
  - `TrackFeedFacts::from_feed_view(&FeedView)` reads the Index feed from `identity`, `published` and `published_source`.
- `TrackDetailVm::with_feed_identity` takes `Option<&TrackFeedFacts>`. `feed_identity_section`, `feed_publication_date_display` and `feed_url` read it. Each rule of these three functions stays the same.
- `TrackPageFacts` is the origin-neutral input of the page: the `TrackView`, the origin facts, the named feeds, the feed facts, the description state and the playlists.
  - `TrackPageFacts::from_library_track` takes the `TrackContext` and a plain `LibraryTrackPageState`. The host copies the frame fields into that state. The view model names no `InspectorFrame`.
  - `TrackPageFacts::from_index_track` takes the `TrackView`, the activation target and an `Option<&FeedView>`.
- `track_page_vm(&facts)` gives `TrackPageVm`: the `TrackDetailPageVm`, the action row, the description, `inspect_sources` and the container id.
- `TrackPageActionRowVm` holds the filled action, the plain actions and the menu items. It also holds each element id, the menu id, the row label "Track actions", the status message and the playlist popover id.
- `TrackPageCommand` is a typed enum of each command of the page: `Action(TrackPageAction)`, `AddToPlaylist(i64)`, `CreatePlaylist(String)`, `ToggleDescription` and `OpenNameLink(TrackNameLinkTarget)`.

### The shell

Add `src/ui/shells/track_page.rs` with `render_track_page(vm, images, dispatch, inspect_sources, cx)`. It renders the whole track page for both origins.

- It renders the container, the scroll bar, the name links, the identity links and the feed identity section. It also renders the action row, the menu, the playlist popover and the description panel.
- It calls `track::build_track_detail_surface` one time, with one `TrackDetailPageVm`.
- `inspect_sources` is an `Option<AnyElement>` that the host gives. The shell adds it below the surface only when `vm.inspect_sources` is true.
- Each entity stays the host of its data. It gives the facts, the resolved images and one dispatch function built from `cx.entity()`.

### The two dispatchers

- `LibraryApp::run_track_page_command` in `src/library/track_page.rs` replaces `run_track_page_action` and each listener of the Library track page.
- `TopApp::run_index_track_page_command` in `src/app/search_dispatch.rs` replaces the listeners of `index_track_page_actions`, of `index_track_description_panel` and of the name links.
- An Index command reads the open track again at the click, through a function like `index_release_page_feed` of packet 021. It never acts on an old `TrackView`.

### The Index feed read

- When an Index track page with a feed GUID opens, `TopApp` gets the `FeedView` of that feed.
- If `index_feed_detail_state` holds that feed GUID as `Loaded`, the page uses it and sends no request.
- Else `TopApp` sends one `FetchIndexFeedDetail` and keeps the result in a new field `index_track_feed_state`, keyed by the feed GUID.
- A result for a feed GUID that the open track page does not name is dropped, as `index_feed_detail_result_is_current` does for the release page.
- A failed read gives no feed section and no "Feed publication date" row. The page stays usable. The status text names the feed GUID and the MusicIndex error.
- The read starts in the `TrackPageSource::Index` branch of `apply_track_page_source`, so the name match page path also gets it.

### Keep

- The stale result checks of the Index track fetch (`index_track_detail_result_is_current`).
- The in-place update after a download: `finish_index_download`, `show_downloaded_index_feed` and `apply_track_page_source`.
- The "Inspect sources" body: `render_library_track_detail_metadata`, `render_inspect_sources_body` and `render_inspect_sources_controls`.
- `download_index_album_of_track` and `copy_index_feed_url_of_track`. The new Index dispatcher calls them.
- The two description state stores.
- `TrackDetailSurfaceContext`, for the element id prefixes only. The release page rows use it too.

### Delete

- `render_library_track_detail_core`, `render_library_track_detail_actions`, `render_track_page_button`, `render_track_page_menu` and `run_track_page_action`, when no caller remains.
- `render_index_track_detail`, `index_track_detail_slots`, `index_track_page_actions` and `index_track_description_panel`, when no caller remains.
- `TrackDetailVm::primary_actions_a11y_label` and the field `primary_actions_a11y_label` of `TrackDetailSurface`.
- The `context` field of `TrackDetailLabels`.
- Each helper of `src/ui/shells/track.rs` and each field of `TrackDetailBehaviorSlots` that no caller reads after the change.

## Orchestrator Decisions

The orchestrator decided each difference on 2026-10-20 under ADR 0084 Decision 4. The numbers are the rows of the difference table.

1. Feed identity section and "Feed publication date" row: a data rule. Each origin shows them when its feed facts give the data. The Index page gains them. This is the defect fix.
2. Album name link target: a data rule. A Library album feed id gives `LibraryAlbum`. Else the feed GUID gives `IndexAlbum`. The context does not select it. The target of each page stays the same.
3. and 4. Identity action prefixes: they stay origin facts. They change only element ids, not what the operator sees. Packet 021 kept its id prefixes for the same reason.
5. `primary_actions_a11y_label`: delete it. No element reads it. The `ActionRow` label "Track actions" names the action row of both origins.
6. `release_label`: the two values are equal. The labels take no context.
7. Action row: both origins use one `ActionRow` with `control_group`. The status message shows when a message exists. Only the Library has a message source today.
8. Action set: it stays a data rule of `page_actions`, selected by the Library state. This packet changes no action.
9. "Inspect sources": a data rule. It shows when the page has Library inspection facts: a stored track, the file tag comparison and the waiting ID3 edits. An Index track has no file and no stored track, so its page has no "Inspect sources", as today.
   - An Index inspection view of RSS and MusicIndex values (ADR 0083 Decision 8) stays an open finding. No packet owns it.
10. Description: one text rule and one projection. The two state stores stay, as in packet 021.
11. Title override: a data rule. The page uses the frame title when the Library gives one.
12. and 13. Named feeds and cover: data sources. Each host gives its own values.
14. Container padding: both origins use `Spacing::LG.scaled(cx)`, the value of the release page. **Operator: the Index track page gets 4 px more padding. The Library track page padding now follows the interface scale.**
15. Busy state: no change in this packet. The queue drops a second job of one release, so a second click on "Download album" does no harm. The orchestrator records the missing busy state of the Index page as an open finding in the phase plan.
16. Element ids: each id stays the same, so GPUI keeps each popover, menu and hover state.

No decision removes an element that the operator sees today.

## Size And Split

The size is like packet 021: two new files, two dispatchers, one new state field, deletions and about twelve guard updates. The orchestrator can run it in one session.

If the session context runs short, do the packet in two parts, in this order:

- Part A, the defect fix: `TrackFeedFacts`, the new `with_feed_identity` input, the Index feed read and the old Index path with the feed facts. Cases R23-01 to R23-05. Part A is a complete change with its own checks.
- Part B, the one assembly: the view model, the shell, the two dispatchers, the deletions and the guard updates. Cases R23-06 to R23-16.

Stop after Part A and report, if the session cannot complete Part B.

## Required Changes

1. Add `TrackFeedFacts` and change `with_feed_identity` to take it. Change the Library caller to `TrackFeedFacts::from_api_feed`.
2. Add the Index feed read and `index_track_feed_state`. Give the feed facts to the Index track page.
3. Add the view model file and its tests.
4. Add the shell file. Make both origins use it.
5. Add the two dispatchers. Delete the replaced functions and fields.
6. Update each guard that names a deleted function or a moved string. Add the guards of this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_track_page_` for unit tests beside the owning code. Each new guard cites ADR 0084 Decision 4.

| Case | Required proof |
|---|---|
| R23-01 | For one recorded `api::Feed` with a website, a Nostr key, a channel `pubDate` and a feed URL, `TrackFeedFacts::from_api_feed(&feed)` equals `TrackFeedFacts::from_feed_view(&FeedView::from_api(feed))` |
| R23-02 | The defect fix. `track_page_vm` for an Index track with feed facts gives a `feed_identity_section`. It names the feed title and holds the feed website and the feed Nostr key |
| R23-03 | The defect fix. For an Index track with no date of its own and a feed with a publication date, the summary rows hold "Feed publication date". A track with its own date has no such row |
| R23-04 | An Index track with no feed facts, while the read runs or after it fails, gives no feed section and no "Feed publication date" row. The other sections are the same as with feed facts |
| R23-05 | `index_track_feed_result_is_current` is false for a result whose feed GUID the open track page does not name. The read sends no request when `index_feed_detail_state` holds the same feed GUID as `Loaded` |
| R23-06 | Take a Library track and the same track as an Index track, with the same feed. `track_page_vm` gives an equal title, name link labels, identity links, summary rows, credits, feed section and description text. Only the actions, the ids, the link targets and `inspect_sources` differ |
| R23-07 | For each `LibraryTrackActionState`, a Library `TrackPageActionRowVm` gives the filled action, the plain actions, the menu, each element id and the status message of today |
| R23-08 | An Index `TrackPageActionRowVm` gives the filled "Download album", no plain action, no message and the ids `index-track-page-action:DownloadAlbum` and `index-track-page-menu-{feed_guid}`. With no feed GUID, the filled action is unavailable and the menu is empty |
| R23-09 | `inspect_sources` is true for Library facts and false for Index facts |
| R23-10 | The album name link gives `LibraryAlbum` when the facts hold a Library album feed id, and `IndexAlbum` with the feed GUID when they do not. The test uses each context with each fact |
| R23-11 | A source guard: in `src/view_models/track_detail.rs`, `self.context` is read only in `identity_action_prefix` and `feed_identity_action_prefix`. No file in `src/` names `primary_actions_a11y_label` |
| R23-12 | A source guard: `src/` has one definition of `render_track_page`, and no caller of `build_track_detail_surface` outside `src/ui/shells/track_page.rs` |
| R23-13 | A source guard: no file in `src/` names `render_library_track_detail_core`, `render_library_track_detail_actions`, `run_track_page_action`, `render_index_track_detail`, `index_track_detail_slots`, `index_track_page_actions` or `index_track_description_panel` |
| R23-14 | A source guard: `src/ui/shells/track_page.rs` names no `LibraryApp`, no `TopApp` and no `db::` |
| R23-15 | The tests of packet 004 of ADR 0083 (R83-41 to R83-48), of ADR 0075 packets 022 and 050, and of the in-place update of packet 022 pass. Their assertions do not change. Only the file names of a retargeted guard change |
| R23-16 | Each file in `src/ui/shells/library` stays at or below the ADR 0038 ceiling of 500 code lines (`surface_modules_under_500_loc`) |

## Visual Acceptance Criteria

These are for the operator. No test proves them. They stay open until a person walks them.

- V1: a Library track page looks as before. Examine the header, the name links, the identity links, the action row, the "⋯" menu and the playlist popover. Also examine the feed section, the description, the summary, the credits and "Inspect sources". The padding can change with the interface scale (Decision 14).
- V2: an Index track page looks as before, with three changes. It shows the feed section below the header when the feed has a website or a Nostr key. It shows "Feed publication date" for a track with no date of its own. Its padding is 4 px larger (Decision 14).
- V3: on an Index track page, the feed section does not repeat a website or a Nostr key of the track. This walks §22 V1 and V2 of the overhaul plan on the Index page.
- V4: on an Index track page, "Download album" downloads the release. The page then changes to the Library track page in place, with no navigation. The feed section stays. Back goes to the page before the track page.
- V5: at narrow width, each element of V1 and V2 stays in its position, and no text is clipped.

## Exclusions

- The hover actions of a track row. [ADR 0083 task 006](adr-0083-task-006-row-hover-actions.md) owns them.
- No Index "Inspect sources" view.
- No busy state for the Index "Download album" (Decision 15).
- No change to the download queue or to the download functions. Packet 022 owns them.
- No change to the release page. Packet 021 owns it.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- This packet, the archived packet [ADR 0083 task 004](archive/adr-0083-task-004-one-track-page.md) and the twin packet [ADR 0084 task 021](adr-0084-task-021-one-release-page-assembly.md).
- `src/ui/shells/library/track_detail.rs`, `src/ui/shells/library/track_detail_metadata.rs`, `src/ui/shells/library/detail.rs`.
- `src/ui/shells/track.rs`, `src/ui/shells/search_results_inspector.rs`, `src/ui/shells/release_page.rs`.
- `src/ui/composites/track_detail_surface.rs`, `src/ui/composites/action_row.rs`, `src/ui/composites/playlist_popover.rs`, `src/ui/primitives/context_menu.rs`.
- `src/view_models/track_detail.rs`, `src/view_models/release_page.rs`, `src/view_models/library.rs` (`LibraryTrackActionVm`, `display_description_text`), `src/views.rs` (`FeedView::from_api`).
- `src/app.rs` (`index_track_detail_state`), `src/app/search_dispatch.rs`, `src/application/queries/search.rs` (`FetchIndexFeedDetail`, `FetchIndexTrackDetail`).
- `src/library.rs` (`InspectorFrame`), `src/library/app_impl.rs`, `src/library/release_page.rs`, `src/library/downloads.rs`.
- `src/metadata.rs`: `feed_website`, `feed_nostr`, `feed_publication_pubdate`. `src/api.rs`: the comment of `track_with_feed_defaults`.
- `tests/architecture_tests.rs`:
  - `adr_0083_both_track_page_origins_build_one_shared_surface` (R83-41) and `adr_0024_index_track_detail_uses_rich_track_view_path`.
  - `adr_0047_phase_c_inspector_rewire_uses_vm_state_and_shared_disclosures`, `adr_0047_description_rendering_must_not_infer_placeholder_metadata` and `adr_0047_membership_buttons_use_download_remove_vocabulary`.
  - `track_identity_links_use_shared_renderer`, `entity_detail_pages_render_through_shell_helper_and_page_vm` and `library_release_detail_playlist_popovers_use_shared_composite`.
  - `adr_0042_composite_call_site_reconciliation_is_current`, `source_fact_placeholder_and_breadcrumb_regressions_are_guarded` and `adr_0084_scrollbars_r14_01_music_columns_use_the_shared_helper`.
  - `PLAYLIST_POPOVER_CALLSITE_FILES`, `LIBRARY_SCREEN_SURFACE_FILES` and `surface_modules_under_500_loc`.

## Checks

```bash
cargo test --lib adr_0084_track_page_
cargo test --lib adr_0083_r83_4
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-023-one-track-page-assembly.md`
- ADR 0084 Decision 4, ADR 0083 Decisions 5 and 8, and the packets 021 and archived ADR 0083 task 004
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the feed facts, the Index feed read, one view model, one shell function, two dispatchers, and the deletions.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each item of "Orchestrator Decisions". Make no other behavior change. Keep each element id.
- When a guard names a deleted function, change the guard to the new owner and keep its rule. Delete a guard only when its rule has no meaning after this packet, and record it.
- Tests use recorded values. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The release page assembly of packet 021.
- The download queue and the download functions.
- The "Inspect sources" body and the compare grid.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R23-01 to R23-16 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V5.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- R23-01 fails: the two feed sources give different values for one feed.
- A guard rule conflicts with one shell function for both origins, and its ADR is current.
- A file in `src/ui/shells/library` stays above the ADR 0038 line ceiling after the move.
- One render path needs a `TopApp` observation of `LibraryApp`.
- A change needs a file in "Do not touch".
- The session context runs short before Part B. Complete Part A, then stop and report.
