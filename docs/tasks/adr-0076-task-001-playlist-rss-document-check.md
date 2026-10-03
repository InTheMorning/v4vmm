# ADR 0076 Task 001: Playlist RSS Document Check

Status: Implemented - 2026-09-24. Mechanical checks are Green. The visual gate is open.
This packet adds a "Check RSS" button and a progress report to the playlist page. The operator has not walked the visual check. Visual checks stay paused.

## Goal

Add the runtime actor that reads the RSS document of each feed in a playlist.
It reads each feed one time, with a conditional GET. It sends one request at a time to each host, to at most four hosts at the same time.
It records one observation for each request. It compares nothing and changes no stored value.

Add the command that starts the check from the playlist page. Add the automatic start when playback starts from that playlist.

## Authority

- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 2 and the accepted values:
  2 seconds between requests to one host, at most four hosts, stored `ETag` and `Last-Modified` validators.
- [ADR 0040](../adr/0040-async-vm-runtime.md): the check is a runtime actor. `src/runtime/playback_polling.rs` is the reference.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) sections 1 and 2: each request records an observation.
- [ADR 0058](../adr/0058-outbound-http-client-policy.md): the blocking client comes from `src/http_client.rs`.
- [ADR 0016](../adr/archive/0016-schema-migration-discipline.md): the run tables go through the migration registry.
- The [phase plan](../plans/adr-0076-playlist-rss-check-phase-plan.md) records the trigger mapping and the open details.

## Recorded Facts - 2026-09-24

- `src/rss/enrich.rs` fetches an RSS document for one track through `crate::http_client::document()`, `request_reuse::single_flight` and `provider_observation::http::capture`.
  It retains the bytes in a document cache for 15 minutes (`RSS_DOCUMENT_REUSE_WINDOW`) with the receipt of the fetch.
- `provider_observation::http::capture` records these response headers in the observation `occurrence`: `etag`, `last-modified`, `retry-after`, `cache-control`, `date` and others.
  A stored observation therefore already holds the validators of the last fetch.
- `capture` marks a response with a status outside 2xx as a failure. A `304` response needs its own handling.
- `contracts::rss_request(feed_url, track_guid, enclosure_url)` names the request slot of the track enrichment fetch. Its profile is `rss_track_enrichment`.
- `db::playlist_tracks(conn, playlist_id)` returns the `TrackRow` list of a playlist with `feed_id`. `db::feed_url_by_id` gives the feed URL.
- `src/runtime/broadcast_readiness.rs` shows the handle pattern: a `watch` snapshot, an `mpsc` inbox, and `SessionLifecycle::spawn_actor`.
- `LibraryAppEvent::PlayPlaylistAt` in `src/library.rs` is emitted by `src/ui/shells/library/playlist_detail.rs` and handled in `src/app.rs`.
- `PlaylistDetailActionsDisplay` in `src/view_models/library.rs` owns the playlist page actions. The shell reads it.
- Schema version 14 is current.

## Required Changes

### Request Slot

Add `contracts::rss_document(feed_url)` with profile `{"version":1,"operation":"rss_playlist_check"}` and no track parameters.
One slot for each feed URL holds the latest document fetch of the check. The track enrichment slot stays separate.

Add a reader in `src/db/provider_observations.rs`. It returns the `ETag` and `Last-Modified` values of the latest successful observation of a request slot.
It reads `last_occurrence_metadata_json`. It returns nothing when the slot has no successful observation.

### Fetch

Each request sends `If-None-Match` and `If-Modified-Since` when the reader returns a validator.
The request uses the document client of `src/http_client.rs` and the existing capture.

A `304` response is an observation with `http_status` 304 and no body. It is not a failure.
The proposal in the phase plan records it with outcome `success`. The implementer reads the `CHECK` constraints in `src/db/provider_snapshot_schema.rs` first, and reports the chosen representation.

A `200` response replaces the retained document of that feed URL in `src/rss/enrich.rs`, with its receipt. A later enrichment call then reuses the checked bytes.

### Actor

Add `src/runtime/playlist_rss_check.rs`. The actor:

- receives one `Start { playlist_id, trigger }` message. A second `Start` for a running playlist joins the running check.
- reads the distinct feeds of the playlist. A feed without a URL is recorded as skipped.
- groups the feeds by host. It sends one request at a time to each host, and waits 2 seconds or more after each request to that host.
- runs at most four host queues at the same time.
- obeys `Retry-After` on any response that carries it.
- stops a host for this check after HTTP `429`. Each remaining feed of that host is recorded as not checked.
- publishes a snapshot after each feed: the counts, each feed result, and the stopped hosts.
- stops when the session stops.

The clock is injected. A test proves the interval and the host limit without a real wait.

### Run Tables

Add schema version 15. It creates:

- `rss_check_runs`: `id`, `playlist_id`, `trigger` (`button` or `playback_start`), `started_at_us`, `finished_at_us`, and the counts of checked, not modified, failed and not checked feeds.
- `rss_check_feed_results`: `run_id`, `feed_id`, `outcome` (`document`, `not_modified`, `failed`, `not_checked`), `http_status`, `observation_id`, `message`.

Packet 002 adds the differences to this run. The playlist page reads the latest run of its playlist.

### Command And Trigger

Add `CheckPlaylistRss { playlist_id }` to `src/application/commands/playlist.rs`. It sends `Start` to the actor.
The playlist page gets a "Check RSS" action in `PlaylistDetailActionsDisplay`, with typed availability and an accessibility label.
The action is unavailable while a check of that playlist runs, and for an empty playlist.

`src/app.rs` sends the same command with trigger `playback_start` when it handles `LibraryAppEvent::PlayPlaylistAt`. Playback does not wait.
The phase plan records this mapping. ADR 0068 moves it to the cue load when the operator accepts that ADR.

### Progress And Result Display

The playlist page view model exposes the latest run: its trigger, its start time, each feed outcome, and each host that the check stopped.
The mounted playlist page updates in place while the check runs, through the actor snapshot.

The report text follows the AGENTS.md rules for operator reports. It names the feed, the host and the HTTP status. It does not name an internal phase.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_playlist_check_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R1-01 | A playlist with tracks from three feeds, one of them twice, produces three requests |
| R1-02 | Two feeds on one host: the second request starts 2 seconds or more after the first, on the injected clock |
| R1-03 | Six feeds on six hosts: at most four requests are active at one time |
| R1-04 | A `Retry-After: 30` response delays the next request to that host by 30 seconds on the injected clock |
| R1-05 | After HTTP `429`, no further request goes to that host in the run. Each remaining feed of the host has outcome `not_checked`, and the snapshot names the host |
| R1-06 | With a stored validator, the request carries `If-None-Match` and `If-Modified-Since`. Without one, it carries neither |
| R1-07 | A `304` response records an observation with status 304 and no body, and the feed result is `not_modified`. The stored validator is unchanged |
| R1-08 | A `200` response records an observation with the body, and the retained document of that feed URL holds the new bytes and receipt |
| R1-09 | A failed request records a failed observation and the feed result `failed`. No feed or track column changes |
| R1-10 | A second `Start` for a running playlist sends no extra request |
| R1-11 | The run row and its feed result rows exist after the run, with the counts equal to the results |
| R1-12 | A version 14 database migrates to version 15 with the two empty tables. A fresh database reaches version 15 |
| R1-13 | The playlist page view model exposes the "Check RSS" action as unavailable during a run and for an empty playlist |
| R1-14 | `PlayPlaylistAt` starts a run with trigger `playback_start`. The playback command completes before the run finishes |
| R1-15 | No screen calls `cx.spawn` for the check. The existing ADR 0040 guard covers the new files |

## Visual Acceptance Criteria

For the operator. No test proves them.

- V1: the playlist page shows the "Check RSS" button with its state, in Light and Dark themes.
- V2: during a check, the page shows the progress in place, without navigation.
- V3: after the check, the page shows each feed outcome and the stopped hosts in readable text.

## Exclusions

- No comparison, no stored value change, no difference report. Packet 002 owns them.
- No change to the track enrichment fetch or its slot.
- No periodic check.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/rss/enrich.rs`: the fetch, the retained document cache, `rss_fetch_registry`.
- `src/provider_observation/http.rs` and `src/provider_observation/contracts.rs`.
- `src/db/provider_observations.rs`: `read_request_refresh` and the request slot tables.
- `src/runtime/broadcast_readiness.rs` and `src/runtime/playback_polling.rs`.
- `src/application/commands/playlist.rs`, `src/library/app_impl.rs`, `src/app.rs`.
- `src/view_models/library.rs`: `PlaylistDetailActionsDisplay`. `src/ui/shells/library/playlist_detail.rs`.
- `src/db.rs`: `MIGRATIONS`, `schema_contract`. The packet 002 result of ADR 0077 shows the migration pattern.

## Checks

```bash
cargo test --lib adr_0076_playlist_check
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree before the first run on a real database. After migration 15, the two tables stay and hold run rows.

## Implementation Result - 2026-09-24

### Request Slot And Validators

`contracts::rss_document(feed_url)` in `src/provider_observation/contracts.rs` names the slot of the check.
Its profile is `{"version":1,"operation":"rss_playlist_check"}`, and its parameters are an empty object. The track enrichment slot is unchanged.

`read_request_validators` in `src/db/provider_observations.rs` returns the `ETag` and `Last-Modified` values of the latest successful observation of a slot.
It reads `last_occurrence_metadata_json` and decodes the retained header bytes.
It uses only an observation with a 2xx status. A 304 observation is also `success`, but its headers can omit a validator. Thus a 304 response never changes the stored validators.
The reader returns nothing when the slot has no 2xx observation, or when that observation has neither header.

### Fetch And The 304 Representation

`http::conditional_request` builds the GET with the document client of `src/http_client.rs`.
It adds `If-None-Match` for a stored `ETag` and `If-Modified-Since` for a stored `Last-Modified`. Without a stored validator it adds neither header.

`http::capture_conditional` uses the existing `capture`. It changes only a 304 response:

- `http_status` is 304, and the captured headers stay.
- The outcome is `success`, and `failure` is empty.
- The body is absent, so `body_sha256` is `NULL`. The interpretation gives the body state `absent`.

The `CHECK` constraints of `src/db/provider_snapshot_schema.rs` accept this row. `outcome` allows `success`, and `body_sha256` allows `NULL`.
The phase plan proposal is thus used without a change. Finding 1 gives the two effects on the store.

A 2xx response goes through `rss::check_rss_document`, which uses the decode and root rules of the enrichment parse.
A readable document gets the outcome `document`. `rss::retain_checked_document` then replaces the retained document of that feed URL with the bytes and the receipt of this fetch.
A later enrichment call for a track of the feed reuses those bytes and replays that receipt.

### Actor

`src/runtime/playlist_rss_check.rs` holds the actor. `LibraryApp` starts it with the MusicBrainz saga, and `release_session_actors` drops it.

- `Start { playlist_id, trigger }` starts a run. A `Start` for a running playlist joins that run and sends no request.
- `Load { playlist_id }` reads the latest stored run into the snapshot. The playlist page sends it when the operator selects a playlist.
- The run reads the distinct feeds of the playlist in the order of their first track. A feed without an HTTP or HTTPS address gets `not_checked`.
- The run makes one queue for each host. Each queue sends one request at a time. It waits `MIN_HOST_INTERVAL` (2 seconds) or the `Retry-After` value, whichever is longer, after each response.
- A `Semaphore` of `MAX_PARALLEL_HOSTS` (4) permits holds the number of active host queues.
- After HTTP 429, the queue records each remaining feed of that host as `not_checked`. The snapshot names the host.
- The actor publishes a snapshot after each feed result. `bridge_watch` updates the mounted playlist page.
- When the session stops, the actor stops each host queue. It waits for the requests in progress, and then its task ends.

`CheckClock` and `RssDocumentFetcher` are injected. The production clock uses `tokio::time`, and the production fetcher uses `capture_conditional`.
The test clock moves to each deadline without a real wait. The test fetcher returns fixed observations. No test of the actor sends a network request.

### Migration 15

`MIGRATIONS` in `src/db.rs` has version 15, `playlist_rss_check_runs`. `CURRENT_VERSION` is 15.
The new module `src/db/rss_check_runs.rs` holds the DDL, the column contract, the writers and the reader.

- `rss_check_runs` has `id`, `playlist_id`, `trigger`, `started_at_us`, `finished_at_us`, `document_count`, `not_modified_count`, `failed_count` and `not_checked_count`.
- `rss_check_feed_results` has `run_id`, `feed_id`, `outcome`, `http_status`, `observation_id` and `message`. Its key is `run_id` and `feed_id`.
- A run row refers to `playlists(id)`, and a result row refers to `feeds(id)`. Both use `ON DELETE CASCADE`. `observation_id` refers to `metadata_observations(id)` with `ON DELETE RESTRICT`.
- Each result write updates the count of its outcome in the same transaction.

Migration 15 uses the shared transaction of migrations 12 to 14, with its ledger record.
After migration 15, the registry verifies the exact version-15 schema. It also compares `retained_digest` before and after, and it makes sure that the two new tables are empty.
`schema_contract(15)` adds the two tables. `inspect_schema` reports a version-14 database that matches its contract as "upgrade required". `upgrades::create_fixture` accepts target 15.

### Command, Action And Trigger

`CheckPlaylistRss { checker, playlist_id, trigger }` in `src/application/commands/playlist.rs` sends `Start` and returns at once.
`LibraryApp::check_playlist_rss` dispatches it through the existing command runner.

`PlaylistDetailActionsDisplay` has the field `check_rss`. It has a label, an accessibility label and a typed `PlaylistRssCheckAvailability`: `Available`, `Running`, `EmptyPlaylist` or `RuntimeUnavailable`.
The shell renders the button from that display, and it attaches the click handler only when the action is available.

The `PlayPlaylistAt` handling in `src/app.rs` first dispatches playback. It then calls `check_playlist_rss` with the trigger `playback_start`. Playback does not wait for the check.

### Progress And Result Display

`src/view_models/playlist_rss_check.rs` builds the report from the snapshot. The report gives these items:

- the trigger and the recorded start time, in UTC,
- the progress while the check runs, or the recorded finish time,
- the counts of the four outcomes,
- one row for each feed, with the feed title, the host and the HTTP status or the failure reason,
- one sentence for each host that the check stopped after HTTP 429.

The shell renders the report between the playlist actions and the track rows. A failed row uses the danger color, and its text starts with "Failed".

### Behavioral Tests

| Case | Test | Location |
|---|---|---|
| R1-01 | `adr_0076_playlist_check_three_feeds_one_repeated_send_three_requests` | `src/runtime/playlist_rss_check.rs` |
| R1-02 | `adr_0076_playlist_check_same_host_requests_wait_two_seconds` | `src/runtime/playlist_rss_check.rs` |
| R1-03 | `adr_0076_playlist_check_six_hosts_run_at_most_four_requests` | `src/runtime/playlist_rss_check.rs` |
| R1-04 | `adr_0076_playlist_check_retry_after_delays_the_next_request` | `src/runtime/playlist_rss_check.rs` |
| R1-05 | `adr_0076_playlist_check_http_429_stops_the_host` | `src/runtime/playlist_rss_check.rs` |
| R1-06 | `adr_0076_playlist_check_request_carries_stored_validators_only` | `src/runtime/playlist_rss_check.rs` |
| R1-06 | `adr_0076_playlist_check_stored_validators_reach_the_second_request` | `src/runtime/playlist_rss_check.rs` |
| R1-07 | `adr_0076_playlist_check_not_modified_keeps_validators` | `src/runtime/playlist_rss_check.rs` |
| R1-07 | `adr_0076_playlist_check_not_modified_is_a_success_without_body` | `src/provider_observation/http.rs` |
| R1-08 | `adr_0076_playlist_check_document_replaces_the_retained_document` | `src/runtime/playlist_rss_check.rs` |
| R1-09 | `adr_0076_playlist_check_failure_changes_no_feed_or_track_column` | `src/runtime/playlist_rss_check.rs` |
| R1-10 | `adr_0076_playlist_check_second_start_joins_the_running_check` | `src/runtime/playlist_rss_check.rs` |
| R1-11 | `adr_0076_playlist_check_run_rows_match_the_results` | `src/runtime/playlist_rss_check.rs` |
| R1-12 | `adr_0076_playlist_check_version_14_migrates_to_15_with_empty_tables` | `src/db/rss_check_runs.rs` |
| R1-12 | `adr_0076_playlist_check_fresh_database_reaches_version_15` | `src/db/rss_check_runs.rs` |
| R1-13 | `adr_0076_playlist_check_action_unavailable_while_running_or_empty` | `src/view_models/library.rs` |
| R1-14 | `adr_0076_playlist_check_playback_start_runs_without_delaying_playback` | `src/application/commands/playlist.rs` |
| R1-15 | `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap` and `runtime_layer_does_not_import_gpui_or_ui` | `tests/architecture_tests.rs` |

These tests also pass:

- `adr_0076_playlist_check_failed_migration_15_leaves_version_14` stops migration 15 after its ledger record. The database stays at version 14.
- `adr_0076_playlist_check_result_counts_follow_rows_and_constraints_hold` proves the counts and the `CHECK` and foreign-key constraints.
- `adr_0076_playlist_check_availability_follows_run_and_tracks` proves each availability value.
- `adr_0076_playlist_check_feed_host_and_retry_after_parse` proves the host rule and the two `Retry-After` forms.

R1-11 also reads the stored run into a second actor through `Load`.

### Findings And Deviations

1. The 304 representation has two effects on the observation store.
   - The request slot points `latest_observation_id` at the 304 observation. Its headers can omit `Last-Modified`. Thus the validator reader selects the latest 2xx observation of the slot, not the slot pointer.
   - The ADR 0075 identity rules merge two equal 304 responses into one observation row, with a larger `occurrence_count`. Thus a second equal 304 records an occurrence of the earlier observation, not a new row. A 200 response with the same bytes follows the same rule.
2. The packet names the counts "checked, not modified, failed and not checked". The columns use the four outcome names: `document_count`, `not_modified_count`, `failed_count` and `not_checked_count`. Each count thus has one outcome.
3. A 2xx response replaces the retained document only when it is a readable RSS document. A response that is not readable gets the outcome `failed`, and its observation gets the reason `xml_decode`, as in the enrichment fetch. This keeps rule P18-4: the app never retains a failure.
4. The check obeys each `Retry-After` value with no upper limit. A large value keeps that host queue, and one of the four host slots, until the value expires or the session stops. A limit is a new product policy. The operator can decide it.
5. The 2-second interval starts at the response, not at the request. The next request thus starts 2 seconds or more after the previous request ended.
6. A later enrichment call that reuses a checked document replays the receipt of the check. That receipt names an observation of the `rss_playlist_check` slot. The packet requires this reuse.
7. `src/app.rs` starts the check for each `PlayPlaylistAt` event, also when the player is not available and the app shows a playback repair. The recovery retry of a playback command does not start a second check.
8. A playlist deletion deletes its run rows, and a feed deletion deletes its result rows, through `ON DELETE CASCADE`. The packet did not state a deletion rule.
9. R1-14 proves the command order and the independent completion with the command bus. The test also reads `src/app.rs`, and it requires the trigger `playback_start` in the `PlayPlaylistAt` handling. No GPUI test drives `TopApp`.
10. R1-15 has no new guard. The two existing ADR 0040 guards read each file under `src/`, so they cover the new files.
11. A stored run with no finish time shows "The app stopped before the check had a result for each feed." The snapshot also derives the stopped hosts of a stored run from its HTTP 429 results.
12. When the session stops, the actor waits for the requests in progress. Each request stops at the 30-second document timeout or before it. The session drain thus ends after the last write of the check.
13. The test `adr_0076_playlist_check_not_modified_is_a_success_without_body` uses a loopback TCP server, as the other capture tests of `src/provider_observation/http.rs` do. It sends no request to another host.
14. These tests expected version 14. They now expect version 15 or `MIGRATIONS.len()`:
   - in `src/db.rs`, `test_migrations_record_versions_on_fresh_schema`,
   - in `src/db/upgrades.rs`, the two tests that name `current: 14`,
   - in `src/db/maintenance.rs`, the inspection test,
   - in `src/db/maintenance/upgrade.rs`, three tests. The boundary test also stops migration 15 at each boundary. Each stop gives a verified rollback to version 11,
   - in `src/db/publisher_relationships.rs`, the R2-06 test. A version-14 database is now "upgrade required",
   - in `src/view_models/startup/database.rs`, the repair report test.
15. The guard `adr_0075_rss_observation_parser_boundary_is_guarded` allows one `Document::parse` call in the production part of `src/rss/enrich.rs`.
    Thus `check_rss_document` calls the enrichment parse without a track. It adds no second DOM parser.
16. The fallback preparation failure text now reads "Apply migrations 12 to 15 and verify retained records".
17. This packet does not change these documents. The orchestrator owns them: the status of ADR 0076, the ADR index, `AGENTS.md`, the phase plan, the source map and `docs/plans/broadcast-chain-delivery-order.md`.

### Checks - 2026-09-24

| Check | Result |
|---|---|
| `cargo test --lib adr_0076_playlist_check` | Green, 21 tests |
| `cargo test` | Green, 1717 unit tests and 273 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 273 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

## Operator Visual Check

Run this check only after the operator resumes visual checks.

This check needs a Linux desktop session, this checkout, the `sqlite3` command and a network connection.
It needs one playlist with tracks from two or more feeds, and one playlist with no tracks. The playback part of V2 needs one downloaded track in the first playlist and a working player.

**Each check sends real HTTP requests to the feed hosts of the playlist.** It sends one request for each feed. It records each response in the database.
Wavlake throttles crawlers. Do not start many checks in a short time.

The new binary upgrades the configured database to version 15 when it opens. Migration 15 adds two empty tables and changes no existing row.
Make the backup with SQLite while the app is closed.

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

   Expect `14`. If the version is 15, the database has already migrated. Use the backup of the first run.
3. Make the backup and verify it:

   ```bash
   backup="$db.before-adr-0076-task-001.sqlite"
   test ! -e "$backup" && sqlite3 "$db" ".backup '$backup'"
   sqlite3 "$backup" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
   ```

   Expect `ok` and `14`. A different result is wrong. Do not continue without a verified backup.
4. Find a playlist with two or more feeds:

   ```bash
   sqlite3 "$db" "SELECT p.name, count(DISTINCT t.feed_id) FROM playlists p JOIN playlist_tracks pt ON pt.playlist_id = p.id JOIN tracks t ON t.id = pt.track_id GROUP BY p.id HAVING count(DISTINCT t.feed_id) >= 2 ORDER BY 2 LIMIT 5;"
   ```

   Write down one name with a small feed count. If the list is empty, add tracks from two albums to a playlist in the app first.
5. Build and open the app:

   ```bash
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   ```

6. **V1 - the button and its state.** Open **Music**, and select the playlist from step 4 in the playlist list.
   - The actions row shows **Rename**, **Delete** and **Check RSS**. **Check RSS** is enabled.
   - Create an empty playlist with the **New playlist name** field, and select it. **Check RSS** is disabled. Its tooltip says that the playlist has no tracks.
   - Open **Settings → General** with Ctrl+Comma. Select Light, and look at both playlists again. Then select Dark, and look again.

   A missing button, an enabled button on the empty playlist, or a label that you cannot read in one theme is wrong.
7. **V2 - progress in place.** Select the playlist from step 4 again, and click **Check RSS**. Do not move to another page.
   - Below the actions row, an **RSS check** section appears. It says that the Check RSS button started the check, with a UTC time.
   - While the check runs, the section gives the number of feeds with a result. **Check RSS** is disabled.
   - Each feed row changes from "Waiting for the request to ..." to its result. The page updates without navigation.
   - After the check finishes, click the play control of a downloaded track in the same playlist. Playback starts at once. It does not wait for the check.
   - The section then says that playback from this playlist started a check, with a new UTC time. The rows update in place again.

   A section that updates only after navigation, or a delay before playback, is wrong.
8. **V3 - the result.** Wait until the section gives a finish time.
   - The section gives the counts: documents received, not modified, failed and not checked.
   - Each feed row names the feed and its host. A received document shows "HTTP 200". A second check of an unchanged feed can show "reported no change" and "HTTP 304".
   - A failed row starts with "Failed", and it gives the host and the HTTP status or the reason.
   - If a host sent HTTP 429, a sentence names that host, and each other feed of that host says "Not checked".
   - Switch between Light and Dark again. Read each row in both themes.

   Each of these results is wrong:
   - text that you cannot read,
   - a row without its feed or host,
   - a failed row that the text does not call "Failed",
   - a feed value that changed on a track or album page. This packet changes no feed or track value.
9. Close the app. Confirm the migration and the stored run:

   ```bash
   sqlite3 "$db" "SELECT version, name FROM schema_migrations WHERE version = 15;"
   sqlite3 "$db" "SELECT id, trigger, finished_at_us IS NOT NULL, document_count, not_modified_count, failed_count, not_checked_count FROM rss_check_runs ORDER BY id DESC LIMIT 3;"
   ```

   Expect `15|playlist_rss_check_runs`, and one row for each check with a finish time.

### Cleanup And Restore

Delete the empty playlist of step 6 in the app, if you do not need it.

Keep the backup until the operator accepts this check.

To undo the upgrade, close the app and restore the backup:

```bash
cp "$db" "$db.after-adr-0076-task-001.sqlite"
sqlite3 "$db" ".restore '$backup'"
sqlite3 "$db" "PRAGMA integrity_check; SELECT max(version) FROM schema_migrations;"
```

Expect `ok` and `14`. Only a build before this packet can open that version-14 database without a new upgrade.
A restore also removes the observations that the checks recorded.

After acceptance, remove the files that this check made:

```bash
rm -i "$backup" "$db.after-adr-0076-task-001.sqlite"
```

The app preservation directory `.v4vmm-upgrade-*` beside the database is an ADR 0066 artifact. Keep it, or remove it with the other upgrade backups.
