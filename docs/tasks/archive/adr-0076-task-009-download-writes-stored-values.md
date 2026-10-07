# ADR 0076 Task 009: A Download Writes The Stored Values

Status: Complete - 2026-10-07. The operator passed V1 and V2 on 2026-10-07 with the Index album "Pilot".

## Goal

A fresh download writes the tags of the stored values of its track. The next "Update n file(s)" scan shows no difference for that file.

## Authority

- [ADR 0076](../../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 10, amended 2026-10-03: a download writes the stored values, after the RSS update of its feed, with the projection of the scan.
- ADR 0076 Decisions 8 and 9.
- [ADR 0080](../../adr/0080-tag-frames-follow-their-owner.md): the channel website (RSS channel `<link>`) goes to `WOAR`.

## Recorded Facts - 2026-10-03

- The operator downloaded the HeyCitizen feed again. The "Update 19 files" popup then listed "MoeFactz", "Milves in Teslas", "The Platform" and other files, each with one line:
  `WOAR: https://v4vmusic.com/?publisher=cmne6j9cn9h2bod0i6hjev8uh (file: no value)`.
- The live RSS channel `<link>` holds that URL. MusicIndex records it as `rss_link`, `feed.link`. The stored value is correct.
- `subscribe_feed_retaining` in `src/subscribe_service.rs` builds `TrackContext::new(track, Some(feed.clone()))` from the request feed, then `id3_edits_for_track_context`. It does this before the RSS upsert of each track.
- The request feed has no `source_links`. `api_feed_from_view` in `src/app/search_dispatch.rs` and `api_feed_from_album` in `src/library/app_impl.rs` fill it with `..Default`. Thus the download writes no `WOAR`.
- `subscribe_track_from_search_internal` builds a `refreshed_context`, but writes the earlier `edits`.
- `Materialization::run` in `src/subscribe_service/materialization.rs` writes `self.edits`. Only the route frame comes from the stored route (`with_stored_route_frame`, Decision 9).
- The scan builds its expected edits with `track_row_to_track_context_with_local_identity` and `id3_edits_for_track_context` in `src/application/queries/tag_update.rs`.
- A comparison on 2026-10-03 found one more difference in realistic data: the live download writes `TRCK` with a total, for example `1/1`, from the length of the request feed track list. The stored projection writes only the number.
  `musicindex_total_tracks` reads `feed.episode_count` or `feed.tracks`, and `track_row_to_feed` stores neither.
- MusicIndex copies the item date, duration and description from the RSS item. It fills the item artist, contributors and image from the RSS channel, which the app also stores. Its `episode_count` is a count of the feed items.
- The channel link has two stored copies: `feeds.link` and the feed `rss` row in `entity_identity_links`. `src/rss/subscribe.rs` writes both. The RSS check in `src/rss/check_apply.rs` updates only `feeds.link`. The `WOAR` projection reads only the identity link.

## Required Changes

1. `Materialization::run` builds the edits that it writes after `validate_subject`, from the stored row: `track_row_to_track_context_with_local_identity` and `id3_edits_for_track_context`. The route frame keeps the stored route of Decision 9.
2. The live request context only finds and fetches the enclosure. Delete each early edit construction, edit field and route patch step that the stored edits make unnecessary.
3. The retained-conversion retry path writes the same stored edits. Keep the ADR 0066 task 014 behavior.
4. The stored projection gives the `TRCK` total from the count of stored tracks of the feed. The scan and the download both use it.
5. When the RSS check applies a channel link, it also replaces the feed `rss` website row in `entity_identity_links`. Both copies then hold the same value.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_download_stored_values_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R76-9-01 | A download from a request feed with no `source_links`, for a feed whose RSS has a channel `<link>`, writes `WOAR` with that link |
| R76-9-02 | After the download of R76-9-01, the scan gives no changed frame for that file |
| R76-9-03 | The edits that the download writes equal the edits that the scan expects for that track |
| R76-9-04 | A retained conversion retry writes the same stored edits |
| R76-9-06 | The stored projection of a track of a feed with 19 stored tracks gives `TRCK` with the total 19 |
| R76-9-05 | An RSS check that changes the channel link updates `feeds.link` and the feed `rss` website identity row to the same value |

## Visual Acceptance Criteria

None. The operator check reads the button count after a download.

## Exclusions

- No change to the tag scan, the popup, or the frames that the app writes.
- No change to the failure classes of ADR 0066 task 014.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../architecture/source-map.md).
- `src/subscribe_service.rs`, `src/subscribe_service/materialization.rs`.
- `src/feed_service.rs`: `track_row_to_track_context_with_local_identity`, `hydrate_feed_identity`.
- `src/metadata_service.rs`: `id3_edits_for_track_context`.
- `src/application/queries/tag_update.rs`: the scan projection.
- `src/rss/subscribe.rs` and `src/rss/check_apply.rs`: the two writers of the channel link.
- `src/db/payment_routes.rs`: the stored route of Decision 9.

## Checks

```bash
cargo test --lib adr_0076_download_stored_values_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

This check downloads one album into an isolated fixture. The fixture has its own configuration, database and music folder.
The check never writes the real Library. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

Needs:

- A Linux desktop session.
- Network access to MusicIndex and to the RSS host and the enclosure host of the album.
- The `sqlite3` binary and the `mid3v2` binary of `python-mutagen`.

**Setup**

Corrected on 2026-10-07: the fixture sets no `flac_path`. The app rejects `flac_path = false` as a setup issue, and that issue pauses configuration saves (ADR 0066).

1. Build the desktop binary. A prior `cargo test` run can leave a test binary at `target/debug/v4vmm`.

   ```bash
   cd /home/citizen/build/v4vmm && cargo build --bin v4vmm
   ```

2. Make the fixture. The commands copy the real database, and they read the real configuration only.

   ```bash
   FX=$(mktemp -d /tmp/v4vmm-task009.XXXXXX)
   CFG="${XDG_CONFIG_HOME:-$HOME/.config}/v4vmm/config.toml"
   mkdir -p "$FX/config/v4vmm" "$FX/data" "$FX/cache" "$FX/music"
   REAL_DB=$(python3 -c 'import os,sys,tomllib; print(os.path.expanduser(tomllib.load(open(sys.argv[1],"rb"))["db_path"]))' "$CFG")
   sqlite3 "$REAL_DB" ".backup '$FX/app.sqlite'"
   python3 - "$CFG" "$FX" <<'PY'
   import re, sys
   source, fx = sys.argv[1], sys.argv[2]
   keep = [line for line in open(source).read().splitlines()
           if not re.match(r"\s*(music_dir|db_path|flac_path)\s*=", line)]
   head = [f'music_dir = "{fx}/music"', f'db_path = "{fx}/app.sqlite"']
   open(f"{fx}/config/v4vmm/config.toml", "w").write("\n".join(head + keep) + "\n")
   PY
   fx_run() { ( export XDG_CONFIG_HOME="$1/config" XDG_DATA_HOME="$1/data" XDG_CACHE_HOME="$1/cache"; shift; /home/citizen/build/v4vmm/target/debug/v4vmm "$@" ) }
   ```

   The fixture music folder is empty. Thus the scan finds no file of the copied Library.

**V1: A fresh download adds no file to "Update n files"**

3. Start the app on the fixture: `fx_run "$FX"`.
4. Open **Music → Library**. Wait for the first scan. Write down the count of the "Update n files" button.
   Expect no button, because the fixture music folder has no file.
5. Use the Search toolbar command to find one album in the Index that is not in the copied Library. An album in the Library shows "Remove Feed" and no Download command.
   Open the album, and click its **Download** command. Wait until the download is complete.
6. Open **Music → Library** again, and wait for the next scan. Read the "Update n files" button.
   - Expect the count of step 4, or no button.
   - This result is wrong: the button count includes a file of the downloaded album.
   - If a button shows, click it and read the popup. This result is wrong: a file of the downloaded album with a line such as `WOAR: <channel link> (file: no value)`.
   - Click **Cancel**. Do not confirm.
7. Close the app.

**V2: One new file holds `WOAR` and the `TRCK` total**

8. Read the channel link and the count of stored tracks of the album. Use the album title of step 5:

   ```bash
   sqlite3 "$FX/app.sqlite" "SELECT f.link, COUNT(t.id) FROM feeds f JOIN tracks t ON t.feed_id = f.id WHERE f.title = '<album title>' GROUP BY f.id;"
   ```

9. Read the tags of one new file:

   ```bash
   find "$FX/music" -name '*.mp3' | head -n 1
   mid3v2 -l "<the path from the output>"
   ```

   - Expect `WOAR=<the link of step 8>`.
   - Expect `TRCK=<n>/<the count of step 8>`.
   - This result is wrong: no `WOAR` line, or a `TRCK` line with no total.
   - If the album has only FLAC files, use `metaflac --export-tags-to=- "<path>"`. Expect the same link and the same track total in the Vorbis comments.

**Cleanup**

10. Remove the fixture. The real configuration, the real database and the real music folder stay unchanged.

    ```bash
    rm -rf "$FX"
    ```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0076-task-009-download-writes-stored-values.md`
- ADR 0076 Decisions 8, 9 and 10
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": a download writes the stored values, and the RSS check keeps both copies of the channel link equal.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use a local `TcpListener` and a parsed RSS fixture. No test sends a request to a remote host.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The tag scan, the popup and `src/ui/`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R76-9-01 to R76-9-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The stored row does not exist yet at the point where the download must build its edits.
- A stored projection omits a value that the live download writes today in realistic data, other than the `TRCK` total. Name each such frame. The operator decided on 2026-10-03 that frames which only artificial MusicIndex data gives are not a stop condition.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-03

1. Files changed:
   - `src/subscribe_service/materialization.rs`: each run reads the stored context of its track after `validate_subject`. It then builds the edits with `id3_edits_for_track_context` and `with_stored_route_frame`. The `edits` field and constructor argument are deleted. The tag comparison reads the stored context.
   - `src/subscribe_service.rs`: the `edits` field of `SubscribeTrackRequest::SearchTrack` and `SearchTrackSubscription` is deleted. Each early edit construction is deleted. The feed download does not read RSS before the materialization, because the materialization reads RSS again.
   - `src/feed_service.rs`: `hydrate_feed_identity` sets `episode_count` from the count of stored tracks of the feed.
   - `src/db.rs`: new `feed_track_count`.
   - `src/rss/check_apply.rs`: when the RSS check applies a channel link, it also replaces the feed `rss` website row.
   - `src/rss/subscribe.rs`: `rss_feed_link_inputs` is `pub(crate)`, so that the RSS check uses the same row.
   - `src/app/search_dispatch.rs`, `src/application/commands/download.rs` and `src/application/ports/download_manager.rs`: the deleted `edits` field is removed from each request.
2. Tests run:
   - R76-9-01, R76-9-02, R76-9-03 and R76-9-06 failed before the change. The new files had no `WOAR`. The download wrote 24 edits, and the projection expected 26. The scan listed two files, each with one `WOAR` line. The projection gave `TRCK` `4`, not `4/19`.
   - `cargo test --lib adr_0076_download_stored_values_` passes all six cases.
   - `cargo test`, `cargo test --test architecture_tests`, `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets` and `cargo build --bin v4vmm` are Green.
3. Behavior changed:
   - A feed download, an Index track download, a Library download and a conversion retry write the edits of the stored values. The route frame keeps the stored route of Decision 9.
   - An Index track download wrote only the route frame before this change. It now writes all stored frames.
   - The stored projection gives the `TRCK` total. The Library track detail reads the same projection, so its "Track #" row now shows the total.
   - The RSS check keeps `feeds.link` and the feed `rss` website row equal.
4. Deviations from task:
   - Three files outside "Files To Inspect" lose the deleted `edits` field. They need no other change.
   - The test `adr_0066_conversion_retry_keeps_edits_one_binding_and_original_wav` now expects the stored title. The test `adr_0076_route_readiness_download_writes_the_stored_route` no longer passes edits.
5. Unresolved concerns:
   - The `TRCK` total counts each stored track of the feed. This count includes a track with a "removed from feed" mark. MusicIndex counts only the current items. The scan compares only the track number, so the button count does not change.
   - An earlier RSS check can have left a stale feed `rss` website row. The next subscribe or the next link change corrects it.
   - `cargo clippy --all-targets -- -D warnings` reports findings in test code of other modules. No finding is in a changed line. The packet check does not use `--all-targets`.

## Orchestrator Review - 2026-10-03

- The `TRCK` total counted each stored track, also a track that the RSS check marked as removed. The orchestrator changed `feed_track_count` to count only the tracks that the feed still lists. R76-9-06 now also stores a removed track and expects the total 19.
- Gate: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets` with no warning, `cargo test` and `cargo build --bin v4vmm` are Green.
