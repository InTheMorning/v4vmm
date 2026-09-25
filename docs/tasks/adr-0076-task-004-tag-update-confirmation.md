# ADR 0076 Task 004: Tag Update Confirmation

Status: Implemented - 2026-09-24. Mechanical checks are Green. The visual gate is open.
This packet adds a button and a popup. The visual gate stays open and paused until the operator walks the procedure below.

## Goal

Show an "Update n file(s)" button when Library file tags differ from the stored metadata.
Open a popup that lists each file. One confirm button writes each listed file through the tag boundary.
Never write a file that the show plays or holds. Keep the count of the files that were not written.

## Authority

- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 8.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) section 7: a metadata refresh writes no audio tag as a side effect.
- [ADR 0004](../adr/0004-format-neutral-audio-tag-boundary.md) and [ADR 0008](../adr/0008-explicit-id3v24-write-boundary.md): the tag write boundary.
- [ADR 0068](../adr/0068-show-cue-and-audition-isolation.md) is Proposed. Until it is accepted, the playback session defines the files in use.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, cautious destructive actions.

## Recorded Facts - 2026-09-24

- `metadata_service::id3_edits_for_track_context` builds the expected ID3 frames of a track. After packet 020, its context reads the projection.
- `audio_tags::write_id3v24_edits(path, edits)` is the one write function. `audio_tags::read_audio_tags` reads the file.
- `playback_sessions` holds `session_id`, `local_track_id`, `playlist_id`, `playlist_position` and `state`. The state `stopped` ends a session.
- `src/library/app_impl.rs` applies pending edits for one track from the inspector. No batch write exists.
- `VmEvent::TrackChanged { track_id }` on the bus invalidates a track row.

## Required Changes

### Difference Scan

Add a query for each Library track with a file. It returns the frames with an expected value that differs from the file value.
The expected frames come from `id3_edits_for_track_context` on the projection of packet 020. The route frame comes from the stored route of packet 003.
A file that cannot be read is listed with its read error and is not written.

The scan runs after a check of packet 002 completes, after a download, and on request. It runs off the render thread through the ADR 0040 runtime.

### In-Use Rule

A file is in use by the show when a playback session with a state other than `stopped` exists, and:

- the session's `local_track_id` is the track, or
- the session's `playlist_id` names a playlist that contains the track.

The popup marks such a file "in use by the show". The confirm button does not write it.

### Button And Popup

The Music section shows one "Update n file(s)" button when the scan finds n files. It is absent when n is 0.
The button opens a popup that lists each file: its title, its album, each frame to write, and the in-use mark.
The popup has one confirm button and one cancel button. Confirm writes each file that is not in use, in order, and reports each result.

After the write, the scan runs again. The button shows the new count, which includes each file that was in use.
The popup and the button read a view model. No renderer decides which file is in use.

### Feed Update Tag Write

ADR 0075 packet 020 found on 2026-09-24 that `feed_service::apply_feed_updates` writes audio tags from a MusicIndex response.
That is a metadata refresh with a tag write as a side effect. ADR 0076 Decision 8 forbids it.
Remove that write. The feed update changes the database only, and the scan of this packet then offers the file write.
Add a test that a feed update writes no tag, and add `apply_feed_updates` to the R4-09 guard.

### Write

Each write goes through `write_id3v24_edits`. A write failure is reported for that file and stops no other file.
After each successful write, the runtime sends `VmEvent::TrackChanged` for the track.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_tag_update_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R4-01 | A file whose title frame differs from the projection is listed with that frame. A file that equals the projection is not listed |
| R4-02 | A file whose route frame differs from the stored route is listed with the route frame |
| R4-03 | A file that cannot be read is listed with its error and has no write action |
| R4-04 | With a playing session on the track, the file is marked in use. With a session on its playlist, the same. With a stopped session, it is not |
| R4-05 | The view model exposes the button only when the count is above 0, with a label that carries the count and an accessibility label |
| R4-06 | Confirm writes each file that is not in use, through `write_id3v24_edits`, and writes no file that is in use |
| R4-07 | After confirm, the count equals the number of files that were in use |
| R4-08 | A write failure on one file is reported, and the other files are written |
| R4-09 | A guard proves that the check module of packet 002 and the scan call no write. Its message names ADR 0076 Decision 8 |
| R4-10 | The scan runs off the render thread. The ADR 0040 `cx.spawn` guard covers the new files |

## Visual Acceptance Criteria

For the operator. No test proves them.

- V1: the button shows the count. The popup lists each file with its frames, in Light and Dark themes.
- V2: a file in use by the show shows its mark, and confirm leaves it. The count after confirm equals the files in use.
- V3: the popup is readable at normal and narrow widths. Stacked text follows the column text rule.

## Exclusions

- No automatic write.
- No per-file checkbox. The operator kept one confirm button on 2026-09-24.
- No change to the inspector's single-track apply.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/metadata_service.rs`, `src/metadata.rs`, `src/audio_tags.rs`.
- `src/playback.rs` and `src/db.rs`: `PlaybackSessionRow`, `playlist_tracks`.
- `src/library/app_impl.rs`: the pending edit apply. `src/runtime/vm_bus.rs`.
- `src/ui/composites/`: the existing popup and confirmation composites.
- [Column text truncation](../troubleshooting/column-text-truncation.md).

## Checks

```bash
cargo test --lib adr_0076_tag_update
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree. This packet adds no migration. A written file keeps its new tags. The operator restores a file from a backup.

## Implementation Result - 2026-09-24

### Difference Scan

`src/application/queries/tag_update.rs` holds the scan. It reads the database and the files. It writes no file and no database row.

- `plan_tag_update_scan` reads each Library track with a recorded file that exists. A track without a file, or with a missing file, is not planned. The readiness report lists those tracks.
- The expected frames come from `id3_edits_for_track_context` on `track_row_to_track_context_with_local_identity`. That context reads the projection of packet 020.
- The scan removes the route frame from these frames. The expected route comes from `db::payment_routes::stored_route`. The scan does not call `route_for_write`, because that function can store a route.
- A stored route with no recipient gives no route frame, as in `with_stored_route_frame`.
- `compare_planned_files` reads the tags. It holds no database lock.
- A frame differs when no file frame with the same target key has the same value. The title comparison uses the title sanitizer of the tag writer. The track and disc numbers compare the number before `/`.
- The route frame differs when the file has no route or when `payment_routes_equal` is false.
- A file that the scan cannot read is listed with its error. It has no write action.

The count of the button is the number of listed files. It includes each file in use and each file that the scan cannot read.

### Scan Triggers

`src/runtime/tag_update.rs` holds the actor. `LibraryApp` starts it with the other runtime actors, and `release_session_actors` drops it. The scan and the write run in `spawn_blocking` of the ADR 0040 runtime.

The Library sends a scan request at these events:

- the start of the actor, at app start and after a runtime repair,
- the end of a playlist RSS check of packet 002, when a playlist in the snapshot stops running,
- a successful Library download,
- a successful feed update and a successful "Check all feeds" workflow,
- a successful tag apply in the track inspector, and a change of the music directory,
- the end of each confirm.

`PlaylistRssCheckSnapshot::running_playlists` is new. The Library compares two snapshots with it.

### In-Use Rule

`db::tracks_in_use_by_show` returns the tracks of each playback session with a state other than `stopped`: the session track, and each track of the session playlist.
The scan marks each such file. The confirm reads the sessions again before the first write. That read decides, because a show can start or stop after the scan.

### Button And Popup

`src/view_models/tag_update.rs` holds the displays. The renderer decides nothing.

- `button` returns no display when the count is 0. The label is "Update 1 file" or "Update n files". The accessibility label is "Show the n files whose tags differ from the stored metadata".
- While a confirm writes, the label is "Updating n files", and the button is not available.
- `popup` gives the title "Update n files?", a message with the number of files to write, and one row for each file. A row has the title, the album, one line for each frame and a mark.
- A frame line shows the frame label, the stored value and the file value. A value longer than 80 characters ends with an ellipsis.
- The mark "In use by the show" uses the warning label color. The mark "Cannot read" uses the danger label color. The mark text states its meaning.
- The popup has one cancel button, "Cancel", and one confirm button, "Write Tags". The confirm style is primary, not destructive.
- `report` gives a summary with the recorded UTC time of the confirm, and one line for each file result.

The button is in the header row of the Music source list, before the feed update button. The report shows below that row.
`src/ui/shells/tag_update_confirmation.rs` renders the button, the report and the popup. The popup uses the shared confirmation composite.

`ConfirmationDialogDisplay` has a new field `items`. The composite shows the items in a scrolling column below the message. Its stacked text uses `overflow_hidden()` and no truncation. The removal confirmation passes an empty list.

### Write Path

`src/application/commands/tag_update.rs` holds `write_tag_updates`.

- It writes the listed frames of each listed file, in list order, through `write_id3v24_edits`.
- A listed route frame goes through `with_stored_route_frame` with `RouteFrameWrite::WhenSelected`. The file thus gets the stored route at the time of the write.
- It does not write a file in use or a file that the scan could not read.
- A write failure is recorded for its file. The other files continue.
- When the database lock or the session read fails, it writes no file and returns the error.
- After each successful write, the actor sends `VmEvent::TrackChanged`. Then it scans again. The Library reads the readiness report again when a confirm result changes.

### Feed Update Change

`feed_service::apply_feed_updates` writes no audio tag now. ADR 0076 Decision 8 and ADR 0075 section 7 own this rule.

- The update still fetches the feed and the detail of each track with a recorded file. It keeps the RSS enrichment of the merge, which retains its observation (packet 039). It stores the `MusicIndex` records and the feed marker.
- It builds no tag edit and calls no `with_stored_route_frame`. The parameter `music_dir` is removed.
- `FeedApplyOutcome` has one field, `tracks_refreshed`: the tracks whose `MusicIndex` record the update stored.
- `ApplyFeedUpdatesResult` has `tracks_refreshed()` and `feed_errors()`. `tracks_updated()`, `edits_written()` and `id3_errors()` are deleted.
- `apply_stale_feed_updates_from`, `MusicDirSource` and `feed_service::configured_music_dir` are deleted. No caller reached them after the change.
- The "Check all feeds" result counts feed errors only as feed update issues.

The feed update message changed:

| Before | After |
|---|---|
| "No edits written" | "Stored no MusicIndex track record. No audio file changed" |
| "Applied {e} edit(s) to {n} track(s)" | "Stored the MusicIndex record of {n} track(s). No audio file changed" |
| "Tag write errors ({n}): ..." | Deleted |

### Tests And Guards Changed By The Feed Update Change

| Item | Change |
|---|---|
| `adr_0076_route_readiness_feed_update_writes_the_stored_route` (`src/feed_service.rs`) | Deleted. It proved the removed write. `adr_0076_tag_update_feed_update_writes_no_tag` replaces it |
| `adr_0075_request_profile_library_feed_update_sends_l1` | Uses the new signature and `tracks_refreshed` |
| `check_feeds_and_repair_routes_result_exposes_counts` | Counts feed errors |
| `apply_feed_updates_empty_input_emits_feed_metadata_events` | Expects the new message |
| `adr_0075_feed_observation_update_preserves_requests_files_and_markers` | Expects the file bytes and the embedded title unchanged, and one refreshed track. The request order is unchanged |
| `adr_0075_feed_observation_ordinary_failures_keep_skips_and_messages` | The configuration failure part is deleted, because the update reads no configuration. A missing file now gives no error, and the four requests stay |
| `adr_0075_feed_observation_retained_bodies_survive_a_database_reopen` and the storage failure tests | Use the fixture `apply()` without a music directory. The comment "and tag generation" is deleted |
| Guard `adr_0075_feed_observation_roots_and_consumers_are_guarded` | The order pairs with `write_id3v24_edits(` are replaced by `persist_musicindex_track(` before `set_feed_musicindex_updated_at(`. The comment names ADR 0076 Decision 8. The section end marker is `pub fn apply_feed_updates(` |
| Guard `adr_0075_request_reuse_converted_routes_ask_the_owner` section list | The end marker `pub fn configured_music_dir(` is now `pub fn apply_feed_updates(` |
| Guard `adr_0076_route_readiness_route_frame_writes_read_the_stored_route` (R3-11) | The expected writer `src/feed_service.rs` is replaced by `src/application/commands/tag_update.rs`. The comment names ADR 0076 Decision 8 |

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R4-01 | `adr_0076_tag_update_title_difference_is_listed_and_equal_file_is_not` | `src/application/queries/tag_update.rs` |
| R4-02 | `adr_0076_tag_update_route_difference_is_listed` | `src/application/queries/tag_update.rs` |
| R4-03 | `adr_0076_tag_update_unreadable_file_is_listed_without_write_action` | `src/application/queries/tag_update.rs` |
| R4-04 | `adr_0076_tag_update_in_use_follows_the_playback_session` | `src/application/queries/tag_update.rs` |
| R4-05 | `adr_0076_tag_update_button_shows_only_above_zero_with_count` | `src/view_models/tag_update.rs` |
| R4-06, R4-07 | `adr_0076_tag_update_confirm_skips_files_in_use_and_keeps_their_count` | `src/application/commands/tag_update.rs` |
| R4-06 | `adr_0076_tag_update_confirm_reads_sessions_again_before_the_write` | `src/application/commands/tag_update.rs` |
| R4-08 | `adr_0076_tag_update_write_failure_is_reported_and_others_are_written` | `src/application/commands/tag_update.rs` |
| R4-09 | `adr_0076_tag_update_checks_and_scans_write_no_tag` | `tests/architecture_tests.rs` |
| R4-10 | `adr_0076_tag_update_actor_scans_writes_and_sends_track_changed` | `src/runtime/tag_update.rs` |
| R4-10 | `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap` covers each new file | `tests/architecture_tests.rs` |
| Feed update | `adr_0076_tag_update_feed_update_writes_no_tag` | `src/feed_service.rs` |

These tests also pass:

- `adr_0076_tag_update_compare_step_needs_no_connection`
- `adr_0076_tag_update_confirm_writes_a_file_that_the_show_released`
- `adr_0076_tag_update_confirm_writes_the_current_stored_route`
- `adr_0076_tag_update_popup_lists_files_with_frames_and_marks`
- `adr_0076_tag_update_report_names_each_result_and_its_time`
- `adr_0076_tag_update_presenter_maps_files_to_dialog_items`

The R4-09 guard is situational. It names ADR 0076 Decision 8 and the fix. It first proves that it fails on a synthetic write. It rejects `write_id3v24_edits(`, `apply_id3_edits_nonfatal(`, `with_stored_route_frame(`, `route_for_write(` and `store_musicindex_route(` in these places:

- `src/rss/check_apply.rs`, `src/rss/compare.rs` and `src/runtime/playlist_rss_check.rs`, the check modules of packet 002,
- `src/application/queries/tag_update.rs`, the scan,
- the body of `apply_feed_updates`.

For R4-10, the guard requires the `spawn_blocking` write, the scan plan and the `TrackChanged` event in the actor. It rejects each scan and write call in `src/library/app_impl.rs`, the view model and the shell.

### Findings And Deviations

1. The scan does not compare `APIC`, `USLT` and `SYLT`. The stored value of these frames is a reference to data, so a text comparison always differs. A file with other artwork or lyrics is not listed for that reason.
2. The comparison of a file that is not MP3 uses the frame labels of the `lofty` reader. The scan skips a frame that the format cannot store. Each mechanical test uses MP3. A FLAC, Ogg or MP4 file can show a frame that stays listed after a write. The operator check can include such a file.
3. The track and disc numbers compare the number only. A total that only one side has is not a difference.
4. The packet says that the scan runs on request. The app has no control that requests a scan. The triggers above send the request. A playback stop sends no scan, so a mark can be old until the next scan. The confirm reads the sessions again, and that read decides. A control for a scan request needs a product decision.
5. A download from the search results sends no scan request. The next trigger includes the file.
6. Before this packet, the feed update stored a `MusicIndex` route for a track with no stored route, through `route_for_write`. That was part of the removed write. The feed update now stores no route. The ADR 0065 route repair still does, as ADR 0076 Decision 9 states.
7. "Check all feeds" still runs the ADR 0065 route repair after the feed update. That repair writes route tags. ADR 0076 Decision 9 keeps it. Only the feed update part of the workflow writes no tag now.
8. `cargo test --lib adr_0076_route_readiness` runs 18 tests. Packet 003 recorded 19. The deleted test is the feed update write test above.
9. The scan reads the projection of each Library track while it holds the database lock. The time for a large library is not measured.
10. This packet does not change `AGENTS.md`, the phase plans, the ADRs, the source map or the delivery order. The orchestrator owns them.

### Checks - 2026-09-24

| Check | Result |
|---|---|
| `cargo test --lib adr_0076_tag_update` | Green, 16 tests |
| `cargo test --lib adr_0076_route_readiness` | Green, 18 tests |
| `cargo test --lib adr_0076_rss_comparison` | Green, 24 tests |
| `cargo test` | Green, 1785 unit tests and 278 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 278 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

No application launch and no change to production data occurred.

## Operator Visual Check

Run this check only after the operator resumes visual checks. Run it after the packet 003 check, because it needs schema version 17.

This check needs a Linux desktop session, this checkout, the `sqlite3` command and the `mid3v2` command.
It needs two downloaded MP3 Library tracks in one playlist, and one downloaded MP3 Library track that is not in that playlist.

**This packet writes audio tags.** Copy the three fixture files before the first confirm. The confirm also writes each other listed file.

The fixture changes the title tag of three files. It adds one playback session row with the state `paused`. The app playback owner uses the session `default` only, so it does not play the fixture session.

1. Close v4vmm. Set the database path and the music directory:

   ```bash
   cd /home/citizen/build/v4vmm
   db=$(sed -n 's/^db_path = "\(.*\)"$/\1/p' ~/.config/v4vmm/config.toml)
   music=$(sed -n 's/^music_dir = "\(.*\)"$/\1/p' ~/.config/v4vmm/config.toml)
   echo "$db" "$music"
   sqlite3 "$db" "SELECT max(version) FROM schema_migrations;"
   ```

   Expect two paths and `17`. An empty path or another version is wrong. Stop, and report the result.
2. Build the app:

   ```bash
   cargo build --bin v4vmm
   ```

3. Open the app once, and look at the header row of the Music source list. Write down the count of the "Update n file(s)" button. When the button is absent, the count is 0. Close the app.

   A count above 0 is a real result. **Write Tags** in step 9 also writes those files. To keep them unchanged, copy the music directory first: `cp -a "$music" "$music.before-adr-0076-task-004"`. Otherwise, accept that those files get the stored metadata.
4. List the playlists with two or more downloaded MP3 tracks:

   ```bash
   sqlite3 "$db" "SELECT p.id, p.name, t.id, t.track_title FROM playlist_tracks pt JOIN playlists p ON p.id = pt.playlist_id JOIN tracks t ON t.id = pt.track_id JOIN local_files lf ON lf.track_id = t.id WHERE t.is_in_library = 1 AND lf.path LIKE '%.mp3' ORDER BY p.id, pt.position LIMIT 20;"
   ```

   Select one playlist and two of its tracks. Set them in the shell:

   ```bash
   P=<playlist id>
   T1=<first track id>
   T2=<second track id>
   ```

5. List the downloaded MP3 tracks that are not in that playlist. Select one, and set it:

   ```bash
   sqlite3 "$db" "SELECT t.id, t.track_title FROM tracks t JOIN local_files lf ON lf.track_id = t.id WHERE t.is_in_library = 1 AND lf.path LIKE '%.mp3' AND t.id NOT IN (SELECT track_id FROM playlist_tracks WHERE playlist_id = $P) LIMIT 10;"
   T3=<track id>
   ```

   An empty list in step 4 or step 5 is wrong for this check. Stop, and report the result.
6. Find the three files, and copy them:

   ```bash
   backup=$(mktemp -d /tmp/v4vmm-adr-0076-task-004.XXXXXX)
   for id in $T1 $T2 $T3; do
     p=$(sqlite3 "$db" "SELECT path FROM local_files WHERE track_id = $id;")
     case "$p" in /*) f="$p" ;; *) f="$music/$p" ;; esac
     test -f "$f" && cp -p "$f" "$backup/$id.mp3" && printf '%s\t%s\n' "$id" "$f" >> "$backup/paths.tsv"
   done
   cat "$backup/paths.tsv"
   ```

   Expect three lines, one for each track id. Fewer lines are wrong. Do not continue without three copies.
7. Apply the fixture. Change the title tag of the three files, and add the paused session on playlist `P`:

   ```bash
   cut -f2 "$backup/paths.tsv" | while IFS= read -r f; do mid3v2 --TIT2 "ADR 0076 fixture title" "$f"; done
   sqlite3 "$db" "INSERT INTO playback_sessions(session_id, local_track_id, playlist_id, playlist_position, started_at, state) VALUES('adr-0076-task-004-fixture', $T2, $P, 0, datetime('now'), 'paused');"
   ```

8. **V1 - the button and the popup.** Open the app with `./target/debug/v4vmm`. Look at the header row of the Music source list.
   - The button shows the count of step 3 plus 3. Its text is "Update n files".
   - Click the button. The popup title is "Update n files?". The popup has one **Cancel** button and one **Write Tags** button.
   - Each fixture track has a row with its stored title and its album. The row has the line `TIT2: <stored title> (file: ADR 0076 fixture title)`.
   - Click **Cancel**. The popup closes, and the button stays.
   - Open **Settings → General** with Ctrl+Comma. Select Light, and open the popup again. Then select Dark, and open it again.

   Each of these results is wrong:
   - The button is absent, or its count is not the count of step 3 plus 3.
   - A fixture row has no title, no album, or no `TIT2` line.
   - The popup has more than one confirm button, or it has a checkbox for each file.
   - Text is not readable in one theme.
9. **V2 - the files in use.** Open the popup.
   - The rows of `T1` and `T2` show the mark "In use by the show". The row of `T3` has no mark.
   - Click **Write Tags**. Below the header row, a report shows the UTC time of the write. It says that the app wrote the files that were not in use, and it names each file that it did not write.
   - The button count is now the count of step 3 plus 2, minus each file of step 3 that the write changed. When step 3 had 0, the button shows "Update 2 files".
   - Check the files at a terminal:

     ```bash
     cut -f2 "$backup/paths.tsv" | while IFS= read -r f; do mid3v2 -l "$f" | grep '^TIT2='; done
     ```

     The files of `T1` and `T2` show `TIT2=ADR 0076 fixture title`. The file of `T3` shows its stored title.
   - Stop the fixture session, and write again:

     ```bash
     sqlite3 "$db" "UPDATE playback_sessions SET state = 'stopped' WHERE session_id = 'adr-0076-task-004-fixture';"
     ```

     Open the popup, and click **Write Tags**. The popup can still show the old mark, because no scan ran after the session change. The report says that the app wrote the two files. The button is absent when step 3 had 0.

   Each of these results is wrong:
   - A file of `T1` or `T2` changes while the session is `paused`.
   - The file of `T3` keeps the fixture title after the first write.
   - The count after the first write does not include the two files in use.
   - The report has no time, or it does not name a file.
10. **V3 - normal and narrow widths.** Make the fixture again with step 7, and restart the app. Open the popup at the normal window width. Then make the window narrow, and open it again.
    - Each row shows its title, album, mark and frame lines. A long line wraps or clips at the edge.
    - A row that shows only `...` with no words is wrong. A row that covers another row is wrong.
    - The list scrolls when the rows do not fit.
11. Cleanup. Close the app. Restore the three files, delete the fixture session, and remove the copies:

    ```bash
    while IFS=$'\t' read -r id f; do cp -p "$backup/$id.mp3" "$f" && cmp "$backup/$id.mp3" "$f"; done < "$backup/paths.tsv"
    sqlite3 "$db" "DELETE FROM playback_sessions WHERE session_id = 'adr-0076-task-004-fixture';"
    sqlite3 "$db" "SELECT count(*) FROM playback_sessions WHERE session_id = 'adr-0076-task-004-fixture';"
    rm -r "$backup"
    ```

    Expect no `cmp` output and `0`. A `cmp` difference is wrong. Keep the copies, and report the result.
    When you copied the music directory in step 3, and you want the earlier tags back, move that copy back to `$music`. Then remove the copy.
