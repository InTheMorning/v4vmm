# ADR 0076 Task 003: Stored Payment Route And Readiness

Status: Implemented - 2026-09-24. Mechanical checks are Green. The visual gate is open.

## Goal

Make the stored payment route the source of each route write to a file.
Ask MusicIndex for a route only when the database has none.
Report a track as not ready when its file route differs from the stored route. Report it as not ready when it has the "removed from feed" mark.

## Authority

- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decisions 7 and 9.
- [ADR 0065](../adr/archive/0065-payment-route-tag-repair.md), amended on 2026-09-24: the repair uses the stored route.
- [ADR 0059](../adr/0059-broadcast-control-surface.md), amended on 2026-09-24: the two new not-ready conditions.
- [ADR 0004](../adr/0004-format-neutral-audio-tag-boundary.md) and [ADR 0008](../adr/0008-explicit-id3v24-write-boundary.md): the tag write boundary.
- [ADR 0016](../adr/0016-schema-migration-discipline.md): the new columns go through the migration registry.
- The operator decided the route comparison on 2026-09-24: every field counts.

## Recorded Facts - 2026-09-24

- The file frame `TXXX:MusicIndex Value Routes` holds a JSON array of `api::PaymentRoute`: `recipient_name`, `route_type`, `split`, `fee`, `address`, `custom_key`, `custom_value`.
- The database stores no route in that shape. `tracks.item_value_json` and `feeds.podcast_value_json` hold the raw `podcast:value` block from RSS.
- `application::commands::payment_routes::repair_loaded_payment_routes_track` fetches the route from MusicIndex and writes the frame. It stores only the "absent" fact.
- The download materialization in `src/subscribe_service.rs` writes the frame from the MusicIndex track response of the download.
- `application::queries::broadcast::BroadcastReadinessState` has `Ready`, `NoRouteTag`, `NoRoutesUpstream`, `NotDownloaded` and `FileMissing`. `Ready` means that the file carries a non-empty route array.
- `SourceReadinessDisplay` in `src/view_models/show.rs` reduces the report to the Source card. `from_broadcast_readiness_track` in `src/view_models/library.rs` builds the readiness rows.
- ADR 0076 packet 002 adds `tracks.removed_from_feed_at` and `removed_from_feed_confirmed_at`.
- Schema version 16 is current after packet 002.

## Required Changes

### Canonical Route

Add a converter from a raw `podcast:value` block to `Vec<PaymentRoute>`. It reads each `podcast:valueRecipient`: `name`, `type`, `address`, `split`, `fee`, `customKey`, `customValue`.
A block without a recipient converts to an empty list.

Add schema version 17: `tracks.payment_routes_json` and `feeds.payment_routes_json`, text, null.
The migration fills each column from the raw block with the converter. A row that does not convert stays null, and the migration result counts it.

The stored route of a track is `tracks.payment_routes_json`, else `feeds.payment_routes_json` of its feed.

The RSS check of packet 002 refreshes the canonical column when it writes a raw block. The subscribe persist step does the same.

### Route Source For Each Write

Each write of the route frame reads the stored route:

- the download materialization in `src/subscribe_service.rs`,
- `repair_loaded_payment_routes_track`,
- the feed update in `feed_service::apply_feed_updates`,
- the tag update of packet 004.

When the stored route is null, the repair asks MusicIndex, stores the response in `payment_routes_json`, and then writes the frame from the stored value.
A MusicIndex response never replaces a stored route. The "absent" fact keeps its meaning for a track with no route anywhere.

A guard proves that no site builds the route frame from an API response directly. Its message names ADR 0076 Decision 9.

### Route Comparison

A file route differs from the stored route when the recipient sets differ in any field: `recipient_name`, `route_type`, `address`, `split`, `fee`, `custom_key`, `custom_value`.
The operator decided on 2026-09-24 that a changed recipient name also makes the track not ready.

### Readiness States

Add two states to `BroadcastReadinessState`, with labels and reasons:

- `RouteOutOfDate`: the file carries a route array that differs from the stored route. Reason: "Payment route in file is out of date."
- `RemovedFromFeed`: `removed_from_feed_at` is set and `removed_from_feed_confirmed_at` is null. Reason: "Removed from feed on {time}. Confirm to play it, or remove it from the playlist."

`RemovedFromFeed` takes priority over the route states. The summary, `problem_count`, the Source card detail and the readiness rows include both states.

The readiness row of a removed track exposes two typed actions: confirm, and remove from playlist.
Confirm sets `removed_from_feed_confirmed_at`. The row then shows its route state.
The readiness row of a `RouteOutOfDate` track exposes no repair action. Packet 004 owns the file write. The row names the "Update n file(s)" button as the fix.

### Removal Actions - Operator Decision 2026-09-24

The operator decided the removal actions on 2026-09-24:

- The readiness row of a removed track has two actions: Confirm, and "Remove from library".
  "Remove from library" uses the existing ADR 0044 Library removal flow and its confirmation.
- Each playlist row of a removed track shows the removed-from-feed message as a row error.
- That playlist row has two actions: "Remove from playlist" and "Remove from all playlists".
  "Remove from playlist" removes the track from that playlist at that position.
  "Remove from all playlists" asks for confirmation. The confirmation names each playlist that holds the track.

Each action carries typed availability and an accessibility label from the view model.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_route_readiness_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R3-01 | A recorded `podcast:value` block converts to the expected `PaymentRoute` list. A block without a recipient converts to an empty list |
| R3-02 | A version 16 database with raw blocks migrates to version 17 with filled canonical columns. A row that does not convert stays null and is counted |
| R3-03 | The download write, the repair write and the feed update write read `payment_routes_json`. A test injects a MusicIndex response with a different route and proves that the file gets the stored route |
| R3-04 | With a null stored route, the repair asks MusicIndex once, stores the response, and writes the stored value |
| R3-05 | With a stored route, the repair sends no MusicIndex request |
| R3-06 | A file whose route differs in `split` is `RouteOutOfDate`. A file that differs only in `recipient_name` is also `RouteOutOfDate`. A file with an equal route in a different recipient order is `Ready` |
| R3-07 | A track with `removed_from_feed_at` set and no confirmation is `RemovedFromFeed`, before any route state |
| R3-08 | A confirmed removed track shows its route state |
| R3-09 | The summary and `problem_count` include both new states. The Source card detail names them |
| R3-10 | The readiness row of a removed track exposes confirm and remove actions with accessibility labels. The row of a `RouteOutOfDate` track exposes no repair action |
| R3-12 | The readiness row of a removed track exposes Confirm and "Remove from library". "Remove from library" dispatches the ADR 0044 removal flow |
| R3-13 | A playlist row of a removed track exposes the row error and the two playlist actions. "Remove from playlist" removes only that entry |
| R3-14 | "Remove from all playlists" names each playlist in its confirmation, and removes the track from each one after confirm |
| R3-11 | The guard for the route source fails on a site that builds the frame from an API response |

## Visual Acceptance Criteria

For the operator. No test proves them.

- V1: the Music readiness list shows a "Payment route in file is out of date" row and a "Removed from feed" row with their actions, in Light and Dark themes.
- V2: the Show Source card counts both states as not ready and names them in its detail.
- V3: confirm on a removed track changes the row in place.
- V4: a playlist row of a removed track shows its row error and the two playlist actions. Each action changes the playlist in place. The implementer added this item on 2026-09-24 for the removal decision.

## Exclusions

- No file write for an out-of-date route. Packet 004 owns it.
- No change to the readiness poll interval.
- No route comparison of a file that the show plays. The readiness scan reads tags as it does today.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/application/commands/payment_routes.rs`, `src/application/queries/broadcast.rs`.
- `src/subscribe_service.rs` and `src/subscribe_service/materialization.rs`.
- `src/feed_service.rs`: `apply_feed_updates`. `src/metadata.rs`: `MUSICINDEX_VALUE_ROUTES_FRAME`, `value_routes_json_is_ready`.
- `src/rss/helpers.rs`: `value_block_json`, `ext_to_json`.
- `src/view_models/show.rs`: `SourceReadinessDisplay`, `readiness_detail_label`. `src/view_models/library.rs`: `from_broadcast_readiness_track`.
- `tests/architecture_tests.rs`: `adr_0065_payment_route_repair_stays_in_command_boundary`, `adr_0065_readiness_rows_keep_state_labels_separate_from_actions`.

## Checks

```bash
cargo test --lib adr_0076_route_readiness
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree before the first run on a real database. After migration 17, the two columns stay filled. No older code reads them.

## Implementation Result - 2026-09-24

### Converter

`src/rss/value_routes.rs` holds the one converter, `payment_routes_from_value_block`. It reads the JSON form of `helpers::value_block_json`.

- Each `valueRecipient` gives one `PaymentRoute`: `name`, `type`, `address`, `split`, `fee`, `customKey` and `customValue`.
- The converter trims each text value and removes an empty value. It writes `type` in lower case.
- A recipient without `fee` gets `false`, the namespace default.
- A block without a recipient gives an empty list.
- These blocks do not convert: a block that is not a JSON object, and a block with a recipient list that is not an array.
- A `split` that is not a number stops the conversion of its block.
- A `fee` that is not `true` or `false` also stops the conversion.

`canonical_routes_json` gives the column value. A missing block or a block that does not convert gives `NULL`.

### Migration 17 And The Stored Route

`MIGRATIONS` in `src/db.rs` has version 17, `stored_payment_routes`. `CURRENT_VERSION` is 17. `src/db/payment_routes.rs` holds the migration and the stored route functions.

- The migration adds `feeds.payment_routes_json` and `tracks.payment_routes_json`, text, null.
- It fills each column from the raw block of its row. A row whose block does not convert stays null.
- It writes one sentence with four counts to the error stream: the feeds and tracks that it filled, and the feeds and tracks whose block did not convert. The migration registry returns no value, as for migration 16.
- Migration 17 uses the shared transaction of migrations 12 to 16. After it, the registry verifies the version-17 schema and the retained digest.
- `schema_contract(17)` gives the extended `feeds` and `tracks` columns. `inspect_schema` reports a version-16 database as "upgrade required". `upgrades::create_fixture` accepts target 17.

`stored_route` gives the track column, else the feed column. `None` means that the database has no route.

`route_for_write` reads the stored route. When it is `None`, the function stores a non-empty `MusicIndex` route in the track column, and it reads the stored value again. The update has the condition that the track column and the feed column are null. Thus a `MusicIndex` route never replaces a stored route.

### Canonical Column Refresh

Each write of a raw block also writes the canonical column:

| Path | Raw block write | Canonical write |
|---|---|---|
| Check, changed feed block | `set_feed_column(... "podcast_value_json" ...)` | `refresh_feed_route` in the same closure |
| Check, changed item block | `set_track_column(... "item_value_json" ...)` | `refresh_track_route` in the same closure |
| Check, added item, and subscribe items | `upsert_item_columns` | The same statement writes `payment_routes_json` |
| Subscribe channel | `persist_channel` | `refresh_feed_route` after the upsert |

The check and the subscribe use the same `upsert_item_columns`. When the row has no raw block before and after the write, the upsert keeps the canonical column. That column can hold a `MusicIndex` route.
`refresh_feed_route` clears the track columns of the feed that have no raw block, when the feed route converts. Such a column can hold only a `MusicIndex` route, and the RSS feed route then applies.

### Route Write Sites

`metadata_service::with_stored_route_frame` is the one function for a route frame write. It removes each route frame edit, calls `route_for_write`, and adds one edit with the stored route. `RouteFrameWrite::Always` always sets the frame. `RouteFrameWrite::WhenSelected` changes the frame only when the edit list already has one.

| Site | Change |
|---|---|
| Download: `subscribe_service::materialization::Materialization::run` | Replaces the route frame of the retained edits before each write. This covers the feed download, the search download and the Library download. `Always` |
| Repair: `repair_loaded_payment_routes_track` | Reads the stored route first. A stored route gives the write with no request. With no stored route, the repair asks `MusicIndex`, stores the response, and writes the stored value. `Always` |
| Feed update: `feed_service::apply_feed_updates` | Packet 004 removed this tag write (ADR 0076 Decision 8). The feed update writes no route frame |
| Library tag apply: `commands::metadata::ApplyTrackId3Edits` | The Library track page passes the track id. A selected route frame edit gets the stored route. `WhenSelected` |
| Packet 004: `commands::tag_update` | Calls `with_stored_route_frame` |

The repair keeps the ADR 0065 behavior for a track with no stored route. The batch run trusts the recorded "absent" fact. A targeted repair asks again.
A stored route with no recipient means that RSS has no route. The repair then returns `NoRoutesUpstream` with the reason "RSS has no payment recipients for this track or feed." It sends no request.

The repair-all batch skips `RemovedFromFeed` and `RouteOutOfDate` tracks.

### Route Comparison

`metadata::payment_routes_equal` compares two recipient sets. Every field counts: `recipient_name`, `route_type`, `address`, `split`, `fee`, `custom_key` and `custom_value`. The recipient order does not count.

The comparison trims text, compares `route_type` without case, compares `split` as a number, and reads a missing `fee` as `false`.

### Readiness States

`BroadcastReadinessState` has two new states:

| State | Label | Reason | Row action |
|---|---|---|---|
| `RouteOutOfDate` | "Route out of date" | "Payment route in file is out of date." | None. The row text names the "Update n file(s)" button |
| `RemovedFromFeed` | "Removed from feed" | "Removed from feed on {time}. Confirm to play it, or remove it from the playlist." | "Confirm" |

`{time}` is the recorded check time of the mark, in the form `2026-09-24 10:00 UTC`.

- `RemovedFromFeed` comes first. It also comes before `NotDownloaded` and `FileMissing`.
- `RouteOutOfDate` needs a stored route and a file route array that parses. Without a stored route, a file with a route stays `Ready`.
- A non-empty stored route makes an earlier "absent" fact of `MusicIndex` irrelevant. A stored route with no recipient gives `NoRoutesUpstream` with the RSS reason.
- `BroadcastReadinessSummary` has `route_out_of_date` and `removed_from_feed`. `problem_count` adds both.
- The Source card counts both as not ready. Its detail names "n removed from feed" and "n with an out-of-date payment route".

### Confirm Action

`ContentListRowActionKind::ConfirmRemovedFromFeed { track_id }` is the typed action. Its label is "Confirm". Its accessibility label is "Confirm that the show plays {title}, which was removed from its feed".

`LibraryApp` runs `commands::playlist::ConfirmRemovedTrack` through the command runner. The command calls `db::rss_field_holds::confirm_removed_track`. It sets `removed_from_feed_confirmed_at` to the time of the confirmation, in microseconds, only for a track with an unconfirmed mark.
After the command, the mounted readiness list reads the report again. The row then shows its route state, or it leaves the list when the track is ready.

### Removal Actions - Operator Decision 2026-09-24

The readiness row of a removed track has two typed actions:

| Action | Kind | Label | Accessibility label | Flow |
|---|---|---|---|---|
| Confirm | `ConfirmRemovedFromFeed { track_id }` | "Confirm" | "Confirm that the show plays {title}, which was removed from its feed" | `ConfirmRemovedTrack` |
| Remove from library | `RemoveFromLibrary { track_id }` | "Remove from library" | "Remove {title} from the library" | `LibraryApp::remove_track`, the ADR 0044 removal plan and its confirmation |

- `ContentListRowDisplay.action` is now `actions: Vec<ContentListRowActionDisplay>`. The row renderer in `src/ui/shells/library/content_list.rs` renders each action from its view model display, in list and tile views. The content row has no composite in `src/ui/composites`. The view model and this one renderer are the owners of the row actions.
- `run_content_list_row_action` now receives the window, because the Library removal opens its confirmation dialog.
- The Library removal flow adds no step for this action. After a removal, it now also reads the mounted readiness list again, so the removed track leaves the list in place.

Each playlist row of a track with an unconfirmed mark shows a row error and two actions. `view_models::playlist_rss_check::removed_from_feed_row` builds them into `PlaylistTrackRowDisplay.removed_from_feed`:

- The row error is "Removed from feed on {time}. Remove it from the playlist, or confirm it in the readiness list." `{time}` is the recorded check time, in UTC. It replaces the packet 002 badge.
- "Remove from playlist", kind `RemoveFromPlaylist { playlist_id, position }`, accessibility label "Remove {title} from this playlist". It uses the `on_remove` slot of the row, which runs the existing `RemovePlaylistTrackAt` for that position.
- "Remove from all playlists", kind `RemoveFromAllPlaylists { track_id }`, accessibility label "Remove {title} from all playlists". It opens a confirmation. The confirmation lists each playlist that holds the track through the item list of the shared confirmation composite. After the confirm, `commands::playlist::RemoveTrackFromAllPlaylists` removes each entry of the track in one transaction. The positions of each playlist stay gapless, and the track stays in the library.
- Each action has the typed availability `PlaylistRemovedTrackActionAvailability`.
- The shared playlist row shell `src/ui/shells/playlist.rs` renders the error and both buttons below the row controls, at the full row width. The error text wraps and is not truncated (ADR 0063).
- After "Remove from all playlists", the app reads the playlists again. It restores the mounted playlist page when that playlist changed, and it reads the mounted readiness list again.

New code: `db::playlists_holding_track`, `db::playlist_remove_track_everywhere`, the two `playlist_service` wrappers, the command, and the presenter `src/ui/shells/playlist_removal_confirmation.rs`.

`db::rss_field_holds::playlist_removed_marks` now gives only unconfirmed marks. A confirmed track is ready for the show, so its playlist row shows no error.

### Guard Changes

- New situational guard `adr_0076_route_readiness_route_frame_writes_read_the_stored_route` (R3-11). Each production function that calls `write_id3v24_edits` or `apply_id3_edits_nonfatal` must call `with_stored_route_frame` before the write. The tag writer and its non-fatal wrapper are the exceptions. The guard first proves that it fails on a synthetic site that builds the frame from an API response. It also fails on a site that reads the stored route after the write. Its message names ADR 0076 Decision 9 and the fix.
- Changed `adr_0065_payment_route_repair_stays_in_command_boundary`. It required `id3_edits_for_track_context` in the repair owner. That requirement encoded the superseded rule that the repair builds the frame from a `MusicIndex` track. The guard now requires `db::payment_routes::stored_route(` and `with_stored_route_frame(` in the production part of the owner, and it rejects `id3_edits_for_track_context` there. The message names ADR 0076 Decision 9. The other checks of the guard are unchanged.
- Changed `adr_0065_readiness_rows_keep_state_labels_separate_from_actions` for the action list of the removal decision. It now requires `pub(crate) actions: Vec<ContentListRowActionDisplay>`, the section marker `for action in &row.actions`, and `this.run_content_list_row_action(kind, window, cx)`. Its rules are unchanged: state labels carry no click handler, and each row action renders from the view model.
- New situational guard `adr_0076_route_readiness_removal_actions_reuse_existing_flows` (R3-12). "Remove from library" must call `self.remove_track(track_id, window, cx)`, and `remove_track` must request the ADR 0044 removal plan. The playlist row block must use the `on_remove` slot and the new slot, and each button must render from its view model display. Its message names ADR 0076 Decision 7 and the fix.

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R3-01 | `adr_0076_route_readiness_value_block_converts_to_payment_routes` | `src/rss/value_routes.rs` |
| R3-02 | `adr_0076_route_readiness_version_16_migrates_to_17_with_routes` | `src/db/payment_routes.rs` |
| R3-03, download | `adr_0076_route_readiness_download_writes_the_stored_route` | `src/subscribe_service/materialization.rs` |
| R3-03, repair | `adr_0076_route_readiness_repair_writes_stored_route_without_request` | `src/application/commands/payment_routes.rs` |
| R3-03, feed update | Packet 004 deleted this test with the feed update tag write. `adr_0076_tag_update_feed_update_writes_no_tag` replaces it | `src/feed_service.rs` |
| R3-04 | `adr_0076_route_readiness_repair_stores_musicindex_route_before_write` | `src/application/commands/payment_routes.rs` |
| R3-05 | `adr_0076_route_readiness_repair_writes_stored_route_without_request` | `src/application/commands/payment_routes.rs` |
| R3-06 | `adr_0076_route_readiness_route_comparison_counts_every_field_and_ignores_order` | `src/application/queries/broadcast.rs` |
| R3-07, R3-08 | `adr_0076_route_readiness_removed_track_comes_before_route_state` | `src/application/queries/broadcast.rs` |
| R3-09 | `adr_0076_route_readiness_summary_counts_both_new_states` | `src/application/queries/broadcast.rs` |
| R3-09 | `adr_0076_route_readiness_source_card_names_both_new_states` | `src/view_models/show.rs` |
| R3-10 | `adr_0076_route_readiness_rows_expose_confirm_and_no_repair` | `src/view_models/library.rs` |
| R3-11 | `adr_0076_route_readiness_route_frame_writes_read_the_stored_route` | `tests/architecture_tests.rs` |
| R3-12 | `adr_0076_route_readiness_rows_expose_confirm_and_no_repair` | `src/view_models/library.rs` |
| R3-12 | `adr_0076_route_readiness_removal_actions_reuse_existing_flows` | `tests/architecture_tests.rs` |
| R3-13 | `adr_0076_route_readiness_playlist_row_exposes_error_and_two_actions` | `src/view_models/playlist_rss_check.rs` |
| R3-13 | `adr_0076_route_readiness_remove_from_playlist_removes_only_that_entry` | `src/application/commands/playlist.rs` |
| R3-14 | `adr_0076_route_readiness_remove_everywhere_confirmation_names_each_playlist` | `src/view_models/playlist_rss_check.rs` |
| R3-14 | `adr_0076_route_readiness_remove_everywhere_dialog_lists_each_playlist` | `src/ui/shells/playlist_removal_confirmation.rs` |
| R3-14 | `adr_0076_route_readiness_remove_from_all_playlists_removes_each_entry` | `src/application/commands/playlist.rs` |

These tests also pass:

- `adr_0076_route_readiness_invalid_value_block_does_not_convert`
- `adr_0076_route_readiness_fill_counts_rows_before_migration_17_records`
- `adr_0076_route_readiness_failed_migration_17_leaves_version_16`
- `adr_0076_route_readiness_musicindex_route_never_replaces_a_stored_route`
- `adr_0076_route_readiness_feed_route_replaces_musicindex_track_route`
- `adr_0076_route_readiness_subscribe_and_check_refresh_stored_route`
- `adr_0076_route_readiness_repair_of_empty_stored_route_asks_nobody`
- `adr_0076_route_readiness_empty_stored_route_needs_publisher_routes`
- `adr_0076_route_readiness_confirmed_track_has_no_playlist_row_error`

The feed update test uses the loopback server of the packet 017 tests. No test sends a request to another host.

### Findings And Deviations

1. The operator decided the removal actions on 2026-09-24. The section "Removal Actions - Operator Decision 2026-09-24" above gives the result. No removal finding stays open.
2. A fifth route frame write site exists: the Library track page applies selected tag edits through `ApplyTrackId3Edits`. The auto-populated edits can include a route frame from the `MusicIndex` context. This packet converted it with `RouteFrameWrite::WhenSelected`. The comparison row of that page still shows the `MusicIndex` route. The file gets the stored route.
3. The Discover tag comparison applies edits to a file that can have no track row. The database then has no route for it, and the command carries no stored route. It keeps its behavior. The R3-11 guard accepts it, because the function contains the stored route call for the Library case.
4. Packet 004 removed the tag write of `feed_service::apply_feed_updates`. The feed update thus writes no route frame.
5. Stophammer at commit `a03c9ea` serializes `route_type` as a lower-case enum (`node`, `wallet`, `keysend`, `lnaddress`) and `split` as an integer. The converter and the comparison normalize these forms. A file that a `MusicIndex` route wrote shows `RouteOutOfDate` when RSS differs in any field, the recipient name included. The number of such files in a real library is not known before the operator check.
6. A stored route with no recipient is a known RSS answer. The readiness row then shows "No upstream routes" and the publisher detail, as for the `MusicIndex` "absent" fact.
7. `RemovedFromFeed` comes before each state, not only before the route states. A removed track without a file thus shows "Removed from feed" first.
8. The confirm action has no "working" state. The list reads the report again after the command. The Show Source card changes at the next readiness scan. The scan interval of 5 minutes is unchanged.
9. The `RouteOutOfDate` row names the "Update n file(s)" button of packet 004.
10. These tests expected version 16. They now expect version 17 or `MIGRATIONS.len()`:
    - in `src/db.rs`, `test_migrations_record_versions_on_fresh_schema` and `migration_cleanup_placeholder_source_text_nulls_only_placeholder_payloads`,
    - in `src/db/upgrades.rs`, the three places that name `current: 16`,
    - in `src/db/maintenance.rs`, the inspection test,
    - in `src/db/maintenance/upgrade.rs`, three tests. The boundary test also stops migration 17 at each boundary,
    - in `src/db/rss_field_holds.rs`, R2-17. A version-16 database is now "upgrade required",
    - in `src/view_models/startup/database.rs`, the repair report test.
11. The fallback preparation failure text now reads "Apply migrations 12 to 17 and verify retained records".
12. The playlist RSS check actor loads the marks of a playlist once in a session. A confirmation in the readiness list thus changes the playlist row after the next check of that playlist or the next app start.
13. "Remove from all playlists" asks for confirmation, and "Remove from playlist" does not. The existing playlist entry removal has no confirmation, and this packet does not add one.
14. This packet does not change these documents: `AGENTS.md`, the phase plans, the ADRs, the source map and `docs/plans/broadcast-chain-delivery-order.md`. The orchestrator owns them.

### Checks - 2026-09-24

| Check | Result |
|---|---|
| `cargo test --lib adr_0076_route_readiness` | Green, 24 tests |
| `cargo test --lib adr_0076_tag_update` | Green, 16 tests |
| `cargo test` | Green, 1791 unit tests and 279 guards. Ten documentation examples stay ignored |

The first full run had one failure in a packet 004 test: `runtime::tag_update::tests::adr_0076_tag_update_actor_scans_writes_and_sends_track_changed`, at `src/runtime/tag_update.rs:248`. The same test passed in the filtered run and in the second full run. This packet does not change that actor. The failure is probably a timing condition of the actor test in a parallel run.
| `cargo test --test architecture_tests` | Green, 279 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

## Operator Visual Check

Run this check only after the operator resumes visual checks.

This check needs a Linux desktop session, this checkout and the `sqlite3` command.
It needs two downloaded library tracks whose files carry a payment route tag.

**Migration 17 adds two columns and fills them when the new build opens the database.** Make the backup with SQLite while the app is closed.

The fixture changes no audio file. It changes the stored route of track `T1`, so the route in its file is old. It sets the "removed from feed" mark on tracks `T2` and `T3`.
It also adds two fixture playlists. Track `T3` is two times in the first playlist and one time in the second playlist.

**"Remove from library" deletes the audio file.** This check opens its confirmation and then cancels it. Do not confirm it.

1. Close v4vmm. Find the configured database path:

   ```bash
   cd /home/citizen/build/v4vmm
   db=$(sed -n 's/^db_path = "\(.*\)"$/\1/p' ~/.config/v4vmm/config.toml)
   echo "$db"
   ```

   An empty line is wrong. Set `db` to the `db_path` value from `~/.config/v4vmm/config.toml`.
2. Record the schema version:

   ```bash
   sqlite3 "$db" "SELECT max(version) FROM schema_migrations;"
   ```

   Expect `16`. If the version is 17, the database has already migrated. Use the backup of the first run.
3. Make the backup and verify it:

   ```bash
   backup="$db.before-adr-0076-task-003.sqlite"
   test ! -e "$backup" && sqlite3 "$db" ".backup '$backup'"
   sqlite3 "$backup" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
   ```

   Expect `ok` and `16`. A different result is wrong. Do not continue without a verified backup.
4. Build and open the app once, so that migration 17 runs. Then close it:

   ```bash
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   sqlite3 "$db" "SELECT version, name FROM schema_migrations WHERE version = 17;"
   ```

   Expect `17|stored_payment_routes`. The terminal shows one sentence with the counts of stored routes and of value blocks that did not convert.
5. Record the readiness state before the fixture:

   ```bash
   ./target/debug/v4vmm broadcast readiness --json > /tmp/adr-0076-task-003-before.json
   python3 -c 'import collections, json, sys; print(collections.Counter(t["state"] for t in json.load(open(sys.argv[1]))["tracks"]))' /tmp/adr-0076-task-003-before.json
   ```

   The command prints JSON and opens no window. Tracks with `route_out_of_date` before the fixture are real results. Write down their number.
6. List the ready tracks that have a stored route and no "removed from feed" mark:

   ```bash
   python3 -c 'import json, sys; print(",".join(str(t["track_id"]) for t in json.load(open(sys.argv[1]))["tracks"] if t["state"] == "ready"))' /tmp/adr-0076-task-003-before.json > /tmp/adr-0076-task-003-ready.txt
   sqlite3 "$db" "SELECT t.id, t.track_title FROM tracks t WHERE t.id IN ($(cat /tmp/adr-0076-task-003-ready.txt)) AND t.removed_from_feed_at IS NULL AND coalesce(t.payment_routes_json, (SELECT f.payment_routes_json FROM feeds f WHERE f.id = t.feed_id)) IS NOT NULL LIMIT 10;"
   ```

   An empty list or an SQL error is wrong for this check. The library then has no ready track with a stored route. Stop, and report the result.
   Select three ids from the list. Set them in the shell:

   ```bash
   T1=<first id>
   T2=<second id>
   T3=<third id>
   ```

7. Keep the stored route of `T1`, and then apply the fixture:

   ```bash
   sqlite3 "$db" "SELECT quote(payment_routes_json) FROM tracks WHERE id = $T1;" > "$db.adr-0076-task-003-route.txt"
   sqlite3 "$db" "UPDATE tracks SET payment_routes_json = '[{\"recipient_name\":\"ADR 0076 fixture\",\"route_type\":\"node\",\"split\":100.0,\"fee\":false,\"address\":\"03fixture\"}]' WHERE id = $T1; UPDATE tracks SET removed_from_feed_at = $(date +%s)000000, removed_from_feed_confirmed_at = NULL WHERE id IN ($T2, $T3);"
   sqlite3 "$db" "PRAGMA foreign_keys=ON; INSERT INTO playlists(name) VALUES ('ADR 0076 fixture A'), ('ADR 0076 fixture B');"
   PA=$(sqlite3 "$db" "SELECT id FROM playlists WHERE name = 'ADR 0076 fixture A';")
   PB=$(sqlite3 "$db" "SELECT id FROM playlists WHERE name = 'ADR 0076 fixture B';")
   sqlite3 "$db" "PRAGMA foreign_keys=ON; INSERT INTO playlist_tracks(playlist_id, track_id, position) VALUES ($PA, $T3, 0), ($PA, $T2, 1), ($PA, $T3, 2), ($PB, $T3, 0);"
   ```

   An error is wrong. Stop, and run the cleanup.
8. Open the app:

   ```bash
   ./target/debug/v4vmm
   ```

9. **V1 - the readiness rows.** Open **Show**. On the **Source** card, click **Open**. The Music readiness list opens.
   - The row of `T1` has the state "Route out of date". Its text says "Payment route in file is out of date." and names the "Update n file(s)" button. The row has no button.
   - The rows of `T2` and `T3` have the state "Removed from feed". Their text gives the UTC time of step 7 and says "Confirm to play it, or remove it from the playlist."
   - Each of these two rows has a **Confirm** button and a **Remove from library** button.
   - Click **Remove from library** on the row of `T2`. The Library removal confirmation opens, because `T2` is in fixture playlist A. Click **Cancel**. The row of `T2` stays.
   - Open **Settings → General** with Ctrl+Comma. Select Light, and read the three rows. Then select Dark, and read them again.

   Each of these results is wrong:
   - a **Fix routes** button on the row of `T1`,
   - a missing **Confirm** or **Remove from library** button on the row of `T2` or `T3`,
   - a removal with no confirmation, or a change after **Cancel**,
   - a time that is not the time of step 7,
   - text that you cannot read in one theme.
10. **V2 - the Source card.** Go back to **Show**, and read the **Source** card.
    - The count "n tracks not ready" includes `T1`, `T2` and `T3`.
    - The detail names "2 removed from feed" and the number of tracks "with an out-of-date payment route". That number is the number of step 5 plus 1.

    A count without `T1`, `T2` or `T3`, or a detail without the two phrases, is wrong.
11. **V3 - confirm in place.** Open the readiness list again, and click **Confirm** on the row of `T2`. Do not move to another page.
    - The row of `T2` changes without navigation. It leaves the list when its file route is equal to the stored route. Otherwise it shows its route state.

    A row that changes only after navigation, or a row that still shows "Removed from feed", is wrong.
    The Source card changes at its next readiness scan, in 5 minutes or less. This delay is not part of V3.
12. **V4 - the playlist row error and actions.** Open **Music**, and select the playlist "ADR 0076 fixture A".
    - The two rows of `T3` each show a red error text below the row controls. The text starts with "Removed from feed on" and gives the UTC time of step 7.
    - Each of these rows has a **Remove from playlist** button and a **Remove from all playlists** button.
    - The error text wraps at a narrow window width. It does not end with a cut-off word and "…".
    - Check the rows in Light and Dark themes.
    - Click **Remove from playlist** on the second row of `T3`. That row leaves the playlist without navigation. The first row of `T3` stays.
    - Click **Remove from all playlists** on the first row of `T3`. The confirmation lists "ADR 0076 fixture A" and "ADR 0076 fixture B". Click **Remove**.
    - The row of `T3` leaves the playlist without navigation. Select "ADR 0076 fixture B". It has no track.

    Each of these results is wrong:
    - a row of `T3` without the error or without the two buttons,
    - a removal of more than one row by **Remove from playlist**,
    - a confirmation that does not name both playlists,
    - a row that changes only after navigation.
13. Close the app. Confirm the stored results:

    ```bash
    sqlite3 "$db" "SELECT removed_from_feed_confirmed_at IS NOT NULL FROM tracks WHERE id = $T2;"
    sqlite3 "$db" "SELECT count(*) FROM playlist_tracks WHERE track_id = $T3;"
    sqlite3 "$db" "SELECT count(*) FROM tracks t JOIN local_files lf ON lf.track_id = t.id WHERE t.id = $T3 AND t.is_in_library = 1;"
    ```

    Expect `1`, `0` and `1`. The last value shows that `T3` stays in the library.

### Cleanup And Restore

Undo the fixture of step 7. The deletion of the two fixture playlists also deletes their remaining entries:

```bash
sqlite3 "$db" "PRAGMA foreign_keys=ON; UPDATE tracks SET payment_routes_json = $(cat "$db.adr-0076-task-003-route.txt") WHERE id = $T1; UPDATE tracks SET removed_from_feed_at = NULL, removed_from_feed_confirmed_at = NULL WHERE id IN ($T2, $T3); DELETE FROM playlist_tracks WHERE playlist_id IN (SELECT id FROM playlists WHERE name IN ('ADR 0076 fixture A', 'ADR 0076 fixture B')); DELETE FROM playlists WHERE name IN ('ADR 0076 fixture A', 'ADR 0076 fixture B');"
sqlite3 "$db" "PRAGMA foreign_key_check; PRAGMA integrity_check;"
```

Expect no foreign key row and `ok`.

Keep the backup until the operator accepts this check.

To undo the upgrade, close the app and restore the backup:

```bash
cp "$db" "$db.after-adr-0076-task-003.sqlite"
sqlite3 "$db" ".restore '$backup'"
sqlite3 "$db" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
```

Expect `ok` and `16`. Only a build before this packet can open that version-16 database without a new upgrade.

After acceptance, remove the files that this check made:

```bash
rm -i "$backup" "$db.after-adr-0076-task-003.sqlite" "$db.adr-0076-task-003-route.txt" /tmp/adr-0076-task-003-before.json /tmp/adr-0076-task-003-ready.txt
```

The app preservation directory `.v4vmm-upgrade-*` beside the database is an ADR 0066 artifact. Keep it, or remove it with the other upgrade backups.
