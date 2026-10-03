# ADR 0076 Task 005: Check And Scan Follow-Ups

Status: Implemented - 2026-09-25. Mechanical checks are Green. The visual gate is open.
This packet changes the check report, the playlist rows and the tag scan triggers. Visual checks are paused, so the gate stays open until the operator walks it.

## Goal

Apply four operator decisions and correct one open defect. Each item is small, and all of them change code of ADR 0076 packets 001 to 004.

## Authority

- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 2, amended on 2026-09-25 with the 60-second limit, and Decisions 4, 7 and 8.
- The operator decisions of 2026-09-25, recorded in the [phase plan](../plans/adr-0076-playlist-rss-check-phase-plan.md#operator-details).
- AGENTS.md design philosophy: a mutation refreshes the mounted view.

## Required Changes

### 1. Retry-After Limit

In `src/runtime/playlist_rss_check.rs`, a `Retry-After` value of 60 seconds or less delays the next request to that host, as today.
A larger value stops that host for the check, as HTTP `429` does. Each remaining feed of that host gets the outcome `not_checked`.
The stored run and the report name the host and the requested wait.

### 2. Copy Feed URL

The report row of each stale feed keeps its podping.me link. It also gets a "Copy feed URL" action, adjacent to the link.
The action puts the feed URL on the clipboard through the existing clipboard owner. It carries typed availability and an accessibility label from the view model.

### 3. Two Scan Triggers

The tag scan of packet 004 also runs:

- when a playback session changes to the state `stopped`,
- after a download from the search results completes.

No scan button is added. The operator decided this on 2026-09-25.

### 4. Confirm Updates The Playlist Row In Place

A Confirm in the readiness list sets `removed_from_feed_confirmed_at`. Today, the playlist row keeps its error until the next check of that playlist or the next app start.
The cause is that the check actor loads the marks of a playlist one time for each session.
After a Confirm, the check actor reloads the marks of each playlist that holds the track, and the mounted playlist page updates in place.
The removal actions of packet 003 do the same for the playlists that they change.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_follow_up_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R5-01 | `Retry-After: 60` delays the next request to that host by 60 seconds on the injected clock |
| R5-02 | `Retry-After: 61` stops the host. Each remaining feed has outcome `not_checked`, and the run names the host and the wait |
| R5-03 | The report view model exposes "Copy feed URL" for each stale feed, with an accessibility label. The action carries the feed URL |
| R5-04 | A session change to `stopped` starts one tag scan |
| R5-05 | A completed search download starts one tag scan |
| R5-06 | After a Confirm, the playlist snapshot of each playlist that holds the track has no removed mark for it, without a new check |
| R5-07 | After "Remove from all playlists", no playlist snapshot holds the track |

## Visual Acceptance Criteria

For the operator. No test proves them.

- V1: the report shows "Copy feed URL" adjacent to the podping.me link, and the pasted text is the feed URL.
- V2: after a Confirm in the readiness list, the playlist page shows no error on that row, without navigation.

## Exclusions

- No scan button.
- No change to the request interval or the host limit.
- No change to the credit lists. Packet 006 owns them.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/runtime/playlist_rss_check.rs`, `src/runtime/tag_update.rs`, `src/view_models/playlist_rss_check.rs`.
- `src/library/app_impl.rs`: the Confirm dispatch and the removal actions of packet 003.
- `src/playback.rs` and `src/app.rs`: the session stop. The search download completion in the search app.
- The Implementation Result sections of packets 001 to 004.

## Checks

```bash
cargo test --lib adr_0076_follow_up
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree. This packet adds no migration.

## Implementation Result - 2026-09-25

### 1. Retry-After Limit

`src/runtime/playlist_rss_check.rs` has the constant `MAX_RETRY_AFTER` (60 seconds).

- A `Retry-After` value of 60 seconds or less delays the next request to that host, as before.
- A larger value stops the host for the check. Each remaining feed of that host gets the outcome `not_checked`.
- `host_stop` decides the stop for HTTP 429 and for the limit. The stop text gives the requested wait in whole seconds. It increases a fraction to the next second.
- `PlaylistRssRun.stopped_hosts` is now a list of `StoppedHost`. Each item has the host and a `HostStopReason`: `TooManyRequests { requested_wait }` or `RetryAfterLimit { requested_wait }`.
- An HTTP 429 stop now also names the requested wait when the response had a `Retry-After` value.

The stored run names the host and the requested wait in the existing `message` column of `rss_check_feed_results`. This packet adds no migration.

- The response that stopped the host gets one more sentence in its message. An example is "limited.test requested a wait of 61 seconds (Retry-After). The limit is 60 seconds, so the check sent no more requests to limited.test."
- Each remaining feed of the host gets a `not_checked` message with the host and the requested wait.

The report gives one sentence for each stopped host with the host and the requested wait.
A feed row with the outcome `document` or `not_modified` now also shows its stored message. Before this packet, the row did not show it.

### 2. Copy Feed URL

`PodpingLinkDisplay` in `src/view_models/playlist_rss_check.rs` has the new field `copy_feed_url: CopyFeedUrlDisplay`.

| Field | Value |
|---|---|
| `label` | "Copy feed URL" |
| `a11y_label` | "Copy the feed URL of {feed} to the clipboard", or "The app has no feed URL for {feed}, so it cannot copy it" |
| `availability` | `CopyFeedUrlAvailability::Available` or `NoFeedUrl` |
| `enabled` | `true` only for `Available` |
| `feed_url` | The trimmed feed URL of the stored difference |

The new function `render_podping_link` in `src/ui/shells/playlist.rs` renders the podping.me button and the "Copy feed URL" button in one row, below the sentence of the stale feed.
The click handler calls `cx.write_to_clipboard(ClipboardItem::new_string(...))`. The app has no clipboard module. Each shell that copies text calls this GPUI method, for example `src/ui/shells/library/feed_detail.rs`. This packet adds no second clipboard owner.

### 3. Two Scan Triggers

Session stop:

- `PollOutcome` in `src/playback_owner.rs` has the new variant `Stopped`. `PlaybackOwner::poll` returns it when the session is `stopped` and the owner holds a loaded track. A later poll of the same stopped session returns `Reconciled(None)`, as before.
- `PlaybackTickOutcome` in `src/runtime/playback_polling.rs` has the new variant `SessionStopped`.
- `TopApp::apply_playback_tick` in `src/app.rs` refreshes Show and calls `LibraryApp::scan_tag_updates` one time for `SessionStopped`.

Search download:

- `src/app/search_dispatch.rs` has the new method `scan_tags_after_search_download`. The success handlers of `download_index_feed`, `download_index_track` and `subscribe_then_append_to_playlist` call it one time.

No scan button is added.

### 4. Confirm Updates The Playlist Row In Place

The check actor has two new messages:

- `ReloadTrackMarks { track_id }` reads the marks again for each loaded playlist that has a mark for the track.
- `ReloadPlaylistMarks { playlist_id }` reads the marks of the playlist again when the snapshot holds its marks.

Both run in the actor task set and send no request. A failed read keeps the earlier marks.
The handle has `reload_track_marks` and `reload_playlist_marks`. `LibraryApp` sends them after these commands:

| Action | Message |
|---|---|
| Confirm in the readiness list (`confirm_removed_track`) | `ReloadTrackMarks` |
| "Remove from all playlists" (`remove_track_from_all_playlists`) | `ReloadTrackMarks` |
| "Remove from playlist" and each other playlist entry removal (`remove_playlist_track_at`) | `ReloadPlaylistMarks` |

A snapshot change reaches the mounted playlist page through the existing `bridge_watch`, and the page renders the rows again.

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R5-01 | `adr_0076_follow_up_retry_after_60_delays_the_next_request` | `src/runtime/playlist_rss_check.rs` |
| R5-02 | `adr_0076_follow_up_retry_after_61_stops_the_host` | `src/runtime/playlist_rss_check.rs` |
| R5-02, report | `adr_0076_follow_up_report_names_host_and_requested_wait` | `src/view_models/playlist_rss_check.rs` |
| R5-03 | `adr_0076_follow_up_report_exposes_copy_feed_url_for_each_stale_feed` | `src/view_models/playlist_rss_check.rs` |
| R5-04 | `adr_0076_follow_up_session_stop_starts_one_tag_scan` | `src/runtime/playback_polling.rs` |
| R5-05 | `adr_0076_follow_up_search_download_starts_one_tag_scan` | `src/app/search_dispatch.rs` |
| R5-06 | `adr_0076_follow_up_confirm_reloads_the_marks_of_each_playlist` | `src/runtime/playlist_rss_check.rs` |
| R5-07 | `adr_0076_follow_up_remove_from_all_playlists_reloads_the_marks` | `src/runtime/playlist_rss_check.rs` |

The test `adr_0076_follow_up_remove_from_playlist_reloads_that_playlist` also passes. It proves that "Remove from playlist" reloads only the changed playlist.
No test sends a network request.

### Findings And Deviations

1. A stored run gives only the HTTP 429 stops in `stopped_hosts`. A stopped host of a `Retry-After` value is not rebuilt from the database. The feed rows of a loaded run still name the host and the requested wait through their stored messages.
   The observation store is not a reliable source for this value. An equal later response can replace the retained headers of the same observation row.
2. The feed rows with the outcome `document` or `not_modified` now show a stored message. This also shows the existing comparison failure message of packet 002. Before this packet, the report did not show that message.
3. R5-04 and R5-05 prove the trigger below the GPUI layer. No GPUI test drives `TopApp`. R5-04 proves the `SessionStopped` tick with a real playback owner and database. It then reads the `SessionStopped` arm of `src/app.rs`. R5-05 reads the success handlers of the search downloads. R1-14 of packet 001 uses the same method.
4. The app has no Stop control. A session changes to `stopped` through the CLI command `stop`, through another process or through the ADR 0066 session finish. The playback polling actor sees the change. That actor runs only with a live playback driver. Without a live driver, a session stop starts no scan. The confirm still reads the sessions again before each write.
5. The existing test `owner_poll_stops_driver_when_session_is_stopped` in `src/playback_owner.rs` now expects `PollOutcome::Stopped`. It also proves that a second poll gives `Reconciled(None)`.
6. The existing R1-05 test `adr_0076_playlist_check_http_429_stops_the_host` now compares `StoppedHost` values. It also proves that the 429 stop keeps the requested wait of 300 seconds.
7. `src/discover/app_impl.rs` also has download handlers. No composition root constructs `SearchApp`, so this packet does not change them. The live search results are in `src/app/search_dispatch.rs`.
8. `MAX_RETRY_AFTER` uses `Duration::from_mins(1)`, because the clippy lint `duration_suboptimal_units` rejects `from_secs(60)`.
9. This packet does not change `AGENTS.md`, the phase plans, the ADRs, the source map or the delivery order. The orchestrator owns them.

### Checks - 2026-09-25

| Check | Result |
|---|---|
| `cargo test --lib adr_0076_follow_up` | Green, 9 tests |
| `cargo test --lib adr_0076_playlist_check` | Green, 21 tests |
| `cargo test --lib adr_0076_tag_update` | Green, 16 tests |
| `cargo test` | Green, 1800 unit tests and 279 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 279 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

## Operator Visual Check

Run this check only after the operator resumes visual checks.

This check needs a Linux desktop session, this checkout, the `sqlite3` command and a network connection.
It needs the schema version 17 of packet 003. This packet adds no migration.
It needs one playlist with a downloaded track.

**The check in V1 sends real HTTP requests to the feed hosts of the playlist.** Wavlake throttles crawlers. Do not start many checks in a short time.
The check also writes RSS values into the stored feed and track values. Only the backup restores the earlier values.

Do V2 before V1. A check clears the "removed from feed" mark of each track that RSS still lists.

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

   Expect `17`. A different value is wrong. Stop, and complete the check of packet 003 first.
3. Make the backup and verify it:

   ```bash
   backup="$db.before-adr-0076-task-005.sqlite"
   test ! -e "$backup" && sqlite3 "$db" ".backup '$backup'"
   sqlite3 "$backup" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
   ```

   Expect `ok` and `17`. Do not continue without a verified backup.
4. Find a playlist track with an HTTP feed URL:

   ```bash
   sqlite3 "$db" "SELECT pt.playlist_id, p.name, t.feed_id, t.id, t.track_title, f.feed_url FROM playlist_tracks pt JOIN playlists p ON p.id = pt.playlist_id JOIN tracks t ON t.id = pt.track_id JOIN feeds f ON f.id = t.feed_id WHERE f.feed_url LIKE 'http%' LIMIT 5;"
   ```

   Select one row. Set its values in the shell:

   ```bash
   P=<playlist_id>
   F=<feed_id>
   T=<track id>
   ```

   An empty list is wrong for this check. Add a downloaded track to a playlist in the app first.
5. Keep the feed title, and then apply the fixture:

   ```bash
   sqlite3 "$db" "SELECT quote(title) FROM feeds WHERE id = $F;" > "$db.adr-0076-task-005-title.txt"
   sqlite3 "$db" "UPDATE feeds SET title = title || ' (old)' WHERE id = $F; UPDATE tracks SET removed_from_feed_at = $(date +%s)000000, removed_from_feed_confirmed_at = NULL WHERE id = $T;"
   sqlite3 "$db" "SELECT feed_url FROM feeds WHERE id = $F;"
   ```

   Write down the feed URL that the last command shows. An error is wrong. Stop, and run the cleanup.
6. Build the binary and open the app:

   ```bash
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   ```

7. **V2 - Confirm updates the playlist row.** Open **Music**, and select the playlist `P`.
   - The row of `T` shows the red error text "Removed from feed on ...". This is the start state.
   - Open **Show**. On the **Source** card, click **Open**. The readiness list opens.
   - Click **Confirm** on the row of `T`.
   - Go back to **Music** and to the playlist `P`. Do not click **Check RSS**, and do not restart the app.
   - The row of `T` shows no error and no **Remove from playlist** button.

   Each of these results is wrong:
   - a row of `T` without the error before the Confirm,
   - a row of `T` that still shows the error after the Confirm,
   - an error that goes away only after **Check RSS** or after a restart.
8. **V1 - Copy feed URL.** On the playlist page of `P`, click **Check RSS**. Wait until the **RSS check** section says that the check finished.
   - The report has a difference for the title of feed `F`, with the old value "(old)".
   - Below the differences, the stale feed sentence names the feed URL of step 5.
   - The **Open podping.me** button and the **Copy feed URL** button are in one row, adjacent.
   - Click **Copy feed URL**. In a terminal, press Ctrl+Shift+V. The pasted text is the feed URL of step 5, with no other text.
   - Open **Settings → General** with Ctrl+Comma. Select Light, and read the report. Then select Dark, and read it again.

   Each of these results is wrong:
   - a missing **Copy feed URL** button, or a button that is not adjacent to **Open podping.me**,
   - pasted text that is not the feed URL,
   - a button that you cannot read in one theme.
9. Close the app.

### Cleanup And Restore

Undo the fixture of step 5. The check of step 8 normally restores the feed title. This command also restores it when the check failed:

```bash
sqlite3 "$db" "UPDATE feeds SET title = $(cat "$db.adr-0076-task-005-title.txt") WHERE id = $F; UPDATE tracks SET removed_from_feed_at = NULL, removed_from_feed_confirmed_at = NULL WHERE id = $T;"
sqlite3 "$db" "PRAGMA integrity_check;"
```

Expect `ok`.

The check also wrote RSS values, a check run and difference rows. To undo them, close the app and restore the backup:

```bash
cp "$db" "$db.after-adr-0076-task-005.sqlite"
sqlite3 "$db" ".restore '$backup'"
sqlite3 "$db" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
```

Expect `ok` and `17`.

After acceptance, remove the files that this check made:

```bash
rm -i "$backup" "$db.adr-0076-task-005-title.txt"
rm -i "$db.after-adr-0076-task-005.sqlite"
```

The second command applies only when you restored the backup.

The gate stays open in this `Status:` line and in [pending human checks](../pending-human-checks.md) until the operator walks it.
