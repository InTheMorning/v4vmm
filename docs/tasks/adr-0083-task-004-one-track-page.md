# ADR 0083 Task 004: One Track Page

Status: Open - implementation and mechanical checks are complete on 2026-10-07. The operator visual check is pending: [pending check 5](../pending-human-checks.md#5-one-track-page--adr-0083-task-004).

## Goal

Build one track page for a Library track and an Index track. Both origins show the same header, actions, sections and section order.
The actions follow the action hierarchy of ADR 0083. The compare grid, Compare ID3 and MusicBrainz move behind one "Inspect sources" disclosure.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decisions 5, 6, 8 and 10, with the 2026-10-07 amendment of Decision 5.
- [ADR 0037](../adr/0037-same-entity-surface-parity.md) Pass 2: one track page grammar for local and Index origins.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: RSS is the provenance, MusicIndex is a cache.
- The track page rules in the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07), section "Phase 4: Track Page".

## Recorded Facts - 2026-10-07

- The Library page: `render_library_track_detail_core` in `src/ui/shells/library/track_detail.rs` fills `TrackDetailBehaviorSlots` (`src/ui/shells/track.rs`). `render_library_track_detail_metadata` in `src/ui/shells/library/track_detail_metadata.rs` puts the surface in a grid beside the Compare ID3 and MusicBrainz panels, and adds the compare grid below.
- The Library actions come from `render_library_track_detail_actions` in `track_detail_metadata.rs`, in this order: "Download Track" or "Remove Track", "Add to playlist", "Compare ID3", "MusicBrainz", and "Apply" and "Discard" when ID3 edits wait. "Open publisher" is a separate button in `track_detail.rs`.
- `ActionRow` in `src/ui/composites/action_row.rs` draws each `.control()` on its own line. The Library actions use `.control()` five times, so they stack vertically. `.control_group()` draws one row.
- `ActionButtonDisplay` has no tone. "Remove Track" looks like the other buttons. It confirms only when `confirm_library_removal` finds a non-trivial plan.
- No track page has a Play action. Playback waits for ADR 0068.
- No track page shows credits. `ContributorListVm` in `src/view_models/entity_detail.rs` serves the album page only. `TrackView.contributors` holds the credits.
- The Index page: `index_track_detail_slots` in `src/app/search_dispatch.rs` gives only the cover. `render_index_track_detail` in `src/ui/shells/search_results_inspector.rs` adds only the identity links. `TrackView::from_api` already fills the description, the credits, the payment routes and the identity links.
- The Index album page downloads through `download_index_feed` in `src/app/search_dispatch.rs`. An Index track has `feed_guid`.
- `src/ui/primitives/context_menu.rs` exists. `AddToPlaylistPopover` exists in `src/ui/composites/playlist_popover.rs`.
- MusicBrainz and Compare ID3 need a downloaded file.

## Required Changes

1. Add one track page view model in `src/view_models/` that both origins use. It gives the header, the actions, the identity links, the summary rows, the description, the credits and the "Inspect sources" state. It carries no renderer type.
2. The actions follow ADR 0083 Decision 5 and its amendment:

   | Track state | Filled action | Plain actions | "⋯" items |
   |---|---|---|---|
   | Not in the Library (Index) | "Download album" | none | "Copy feed URL" |
   | In the Library, not downloaded | "Download track" | "Add to playlist" | "Copy feed URL", "Remove track…" |
   | Downloaded | "Add to playlist" | none | "Copy feed URL", "MusicBrainz lookup", "Remove track…" |

   - "Remove track…" is the last item, after a divider, in the danger color. It always asks for confirmation.
   - "Apply" and "Discard" of waiting ID3 edits stay, inside "Inspect sources".
   - Each action has typed availability and an accessibility label in the view model.
   - The actions draw in one row.
3. The album name and the publisher name under the title are links to the album page and the publisher page. Delete the "Open publisher" button. A name with no page target is plain text.
4. The identity links (website, Nostr, RSS) are quiet icon links beside the header, always visible. They keep the owner rules of ADR 0075 task 022: a track shows only its own identities in the header.
5. Add a credits section: the RSS credits one time each, in RSS order, with no MusicIndex credit and no provider label. A track with no credits has no section.
6. Add one "Inspect sources" disclosure, closed by default, at the end of the page. For a Library track it holds the compare grid, Compare ID3, the MusicBrainz panel, and "Apply" and "Discard". An Index track has no "Inspect sources" in this packet. A later packet adds its inspection view.
7. The Index page uses the same view model and the same shell. "Download album" downloads the feed of the track, through the same command as the Index album page.
8. Draw the "⋯" menu with `context_menu.rs`. Each icon is an `IconName`. No label holds an icon character such as "▾".
9. Delete each builder, slot and panel that no path reaches after the change.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R83-41 | The track page view model gives the same section order for a Library track and an Index track |
| R83-42 | For each of the three states, the view model gives exactly one filled action, the plain actions and the "⋯" items of the table |
| R83-43 | "Remove track…" is the last "⋯" item, is marked destructive, and its command asks for confirmation for each plan |
| R83-44 | The view model gives the album name and the publisher name as link targets, and no "Open publisher" action exists |
| R83-45 | The credits section gives the RSS credits in RSS order, each one time, with no MusicIndex credit and no provider label |
| R83-46 | A Library track view model gives "Inspect sources" closed by default. An Index track view model gives no "Inspect sources" |
| R83-47 | The guards of ADR 0075 tasks 022, 048 and 049 stay Green. They cover header identities by owner, track artwork else album artwork, and no derived release date |
| R83-48 | No action label in `src/view_models` or `src/ui` holds an icon character, and the ADR 0083 guards stay Green |

## Visual Acceptance Criteria

These are for the operator. They stay open until a person walks them.

- V83-41: a Library track page and the Index page of the same track show the same header, action row, section order and disclosure. Check in Light and Dark. This carries §2 of the moved rules.
- V83-42: the actions sit in one row. Exactly one button is filled. "⋯" opens a menu, and "Remove track…" is last and red.
- V83-43: the album and publisher names are links. No "Open publisher" button shows.
- V83-44: the moved rules §19, §22, §26 and §27 hold on the new page. Their V-items are in the overhaul plan.
- V83-45: "Inspect sources" is closed when a Library track page opens. Opened, it shows the compare grid, Compare ID3 and MusicBrainz as before.
- V83-46: normal and narrow widths show each element in its place, with no clipped text.

## Exclusions

- No new inspection view. The compare grid content does not change.
- No payment split bar. A later packet adds it for the track page and the album page.
- No Play action.
- No album page change.
- No database change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/ui/shells/library/track_detail.rs`, `track_detail_metadata.rs`, `track_detail_metadata_grid.rs`, `track_detail_metadata_values.rs`.
- `src/ui/shells/track.rs`, `src/ui/composites/track_detail_surface.rs`, `track_header.rs`, `action_row.rs`, `action_button.rs`, `playlist_popover.rs`, `disclosure_group.rs`.
- `src/ui/primitives/context_menu.rs`, `src/ui/icons.rs`.
- `src/view_models/track_detail.rs`, `entity_detail.rs`, `library.rs` (`LibraryTrackActionVm`).
- `src/app/search_dispatch.rs`: `index_track_detail_slots`, `download_index_feed`. `src/ui/shells/search_results_inspector.rs`: `render_index_track_detail`.
- `src/library/app_impl.rs`: `toggle_local_subscription`, `request_library_removal`, `open_publisher_page`.
- `tests/architecture_tests.rs`: the ADR 0075 and ADR 0083 guards.

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
cargo build --release --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Result - 2026-10-07

The orchestrator implemented this packet in the main session, after four subagent runs stopped at the view model.

- View model, `src/view_models/track_detail.rs`:
  - `TrackPageAction`, `TrackPageActionDisplay` and `TrackPageActions` give the filled action, the plain actions and the "⋯" items. `LibraryTrackActionState` (downloaded, busy, `MusicBrainz` available) selects them.
  - `name_links()` gives `TrackNameLinkVm` values with a typed `TrackNameLinkTarget`: a Library album, an Index album or a publisher.
  - `credits()` gives `TrackCreditVm` values in source order, each name and role one time. The Library list is the projected list of ADR 0076 task 006.
  - `publisher_action()` is deleted. The publisher name link replaces it.
- Shared owners:
  - `ContextMenu` draws a divider before a destructive item.
  - `AddToPlaylistPopover::trigger_style` lets "Add to playlist" be the filled button.
  - `TrackDetailSurface` draws the name links under the title and a "Credits" box after the description.
  - `track::render_track_name_links` draws the links for both origins.
- Library page: `render_library_track_detail_actions` draws one row from `page_actions()`. "Inspect sources" (`InspectorPanelKind::Sources`, closed by default) holds Compare ID3, Apply and Discard, the `MusicBrainz` panel and the compare grid. "MusicBrainz lookup" in the menu opens "Inspect sources" and starts the lookup. A disabled lookup shows the ADR 0047 reason as its menu description.
- Removal: `RemovalConfirmation::Always` makes "Remove track…" confirm each plan. Other callers keep `WhenReferenced`.
- Navigation: `LibraryAppEvent::OpenAlbumPage` and `TopApp::open_library_album_page` open the Library album page. The search result path uses the same function.
- Index page: `index_track_detail_slots` adds the name links, the filled "Download album" and the "⋯" menu.
  - "Download album" reads the feed with `FetchIndexFeedDetail`, then calls `download_index_feed`, as the Index album page does.
  - "Copy feed URL" reads the same feed and copies its `feed_url`. `TrackResponse` has no feed URL, and `FeedResponse` has it.
  - `TrackView.publisher_feed_guid` keeps the publisher GUID that the `publisher` relationships of `TrackResponse` name. The publisher name links to the Index publisher page.
- Deleted: the old action builder, the "Open publisher" button, `LibraryTrackActionVm::subscription_button_label` and its two unread fields.

Follow-up: on 2026-10-07 the operator decided that each MusicIndex request asks for full data. [ADR 0075 packet 052](archive/adr-0075-task-052-full-include-lists.md) implemented it, so an Index track page gets its payment routes, identity links, credits and publisher.

Deviations:

- "Remove track…" shows only on a downloaded track. Today's toggle offers removal only in that state.
- The `adr_0076_route_readiness_removal_actions_reuse_existing_flows` guard now expects the confirmation policy argument in `remove_track`. The rule is the same: one ADR 0044 removal flow.

Proof:

- R83-41: `adr_0083_both_track_page_origins_build_one_shared_surface`.
- R83-42: `adr_0083_r83_42_index_track_actions`, `adr_0083_r83_42_library_track_not_downloaded_actions`, `adr_0083_r83_42_r83_43_downloaded_track_actions`.
- R83-43: `adr_0083_r83_42_r83_43_downloaded_track_actions` and `always_confirmation_defers_an_unreferenced_track_removal`.
- R83-44: `adr_0083_r83_44_name_links_open_the_album_and_the_publisher`, `adr_0083_r83_44_index_album_name_links_to_the_index_album` and `from_api_track_keeps_the_named_publisher_feed_guid`.
- R83-45: `adr_0083_r83_45_credits_keep_source_order_without_duplicates`.
- R83-46: `adr_0083_r83_46_inspect_sources_starts_closed_and_toggles`.
- R83-47 and R83-48: the ADR 0075 and ADR 0083 guards stay Green.
- The project gate is Green, and the release build is Green.

## Operator Visual Check Procedure

No step needs a mouse wheel. Use Page Down to scroll a page.

1. Run `cargo build --release --bin v4vmm`, then `./target/release/v4vmm`.
2. Downloaded track: open a Library album in the sidebar, then a downloaded track.
   - Expected: under the title, the album name and the publisher name as links. One row: a filled "Add to playlist" and a "⋯" button. No "Open publisher" button.
   - Click "⋯". Expected: "Copy feed URL", "MusicBrainz lookup", a divider, then "Remove track…" in red.
   - Click "Remove track…". Expected: a confirmation dialog, also for a track in no playlist. Click Cancel.
3. On the same page, press Page Down. Expected: "Credits" (if the track has credits) and a closed "Inspect sources". Click "Inspect sources". Expected: Compare ID3, then the compare grid as before.
4. Click "⋯", then "MusicBrainz lookup". Expected: "Inspect sources" opens and the lookup panel shows.
5. Click the album name link. Expected: the album page. Click Back, then the publisher name link. Expected: the publisher page.
6. Index track: search `mellow cassette` in the toolbar and open the track "The Arbiter".
   - Expected: the same header, the album name and the publisher name (when MusicIndex names one) as links, a filled "Download album" and a "⋯" button.
   - Click "⋯", then "Copy feed URL". Paste into a terminal. Expected: the RSS URL of the album "Pilot".
   - Expected: the same section order as the Library page, with no "Inspect sources".
   - Do not click "Download album" unless you want the album in the Library.
7. Switch to Light (Settings, General) and repeat steps 2 and 6 once. Make the window narrow. Expected: no clipped text. Switch back.

Cleanup: none, unless step 6 downloaded an album. Then remove it from the album page.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0083-task-004-one-track-page.md`
- ADR 0083, ADR 0037, and the "Phase 4: Track Page" rules of the overhaul plan
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": one track page view model and shell for both origins, the action hierarchy, name links, credits, and the "Inspect sources" disclosure.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`, especially renderer portability and typed action state.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Each color, size and icon comes from a token or an `IconName`.
- Column text does not call `truncate()` (ADR 0063).
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git mv` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The compare grid content and the MusicBrainz lookup logic.
- The album page, the playlist pages and the Show section.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R83-41 to R83-48 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed and files deleted
2. tests run
3. behavior changed, for a Library track and for an Index track
4. the view model name and its fields
5. deviations from task
6. unresolved concerns
