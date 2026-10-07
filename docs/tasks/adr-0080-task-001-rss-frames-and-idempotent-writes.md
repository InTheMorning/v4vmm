# ADR 0080 Task 001: RSS Frames And Idempotent Writes

Status: Open - the operator passed V1 to V3 on 2026-10-07: `WOAR` of "How Bout You?" holds the plain channel link, and the scan shows no difference. On 2026-10-07 the operator moved V4 to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07), under Phase 4: Inspect sources.

## Goal

The tag writer puts each RSS value in the frame of its owner, and a second write changes nothing.
The tag compare and the "Update n file(s)" scan use the same resolution as the writer, so a file that the app wrote shows no difference.

## Authority

- [ADR 0080](../adr/0080-tag-frames-follow-their-owner.md) Decisions 1 to 8.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decisions 8 and 9: the scan, the confirmation and the route frame stay as they are.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: RSS values and MusicBrainz values stay apart.
- ADR 0004 and ADR 0008: the tag boundary and the ID3v2.4 write boundary.

## Recorded Facts - 2026-09-29

- `id3_frame_hint` in `src/metadata.rs` maps "Website" and "RSS feed website" to `WOAR`, and "Nostr handle" and "RSS feed nostr handle" to `TXXX:RSS Nostr Handle`.
- `source_value_for_metadata_field` gives the "Description" row the track description, else the feed description.
- `format_source_value_for_id3v24` puts `download for free (url, forward):` before each RSS website value in `WOAR`.
  The read side near `src/metadata.rs:831` removes that text again for the compare.
- `write_mp3_edits` in `src/audio_tags.rs` calls `tag.add_frame` for each edit. It removes no earlier `WOAR` value. A changed value becomes a second frame.
- `write_lofty_edits` writes FLAC, Ogg and MP4 files. It must follow the same rules.
- `changed_frames` in `src/application/queries/tag_update.rs` counts a frame as equal when any file value matches. It does not report an extra stale value.
- `validate_nostr_identity` in `src/rss/identity.rs` already validates NIP-19 values. Use it, and accept only an `npub`. Do not add a crate.
- The fixed edits of `metadata_service::tests` (ADR 0075 packet 022, R22-06) record the present output. This packet changes them on purpose.

## Required Changes

### 1. Nostr Key (Decisions 1 and 7)

- `TXXX:RSS Nostr Handle` gets one value: the valid item key, else the valid channel key.
- A valid key passes NIP-19 validation as an `npub`. An invalid item key does not block the channel key.
- With no valid key, the edits hold no Nostr frame.

### 2. Website Frames (Decisions 2 and 8)

- The item page (the track's own website) goes to `WOAF`. The channel website goes to `WOAR`.
- Each URL frame value is a plain URL. Delete the `download for free (url, forward):` prefix from `format_source_value_for_id3v24`.
- The writer writes a URL only when it parses as a URL.

### 3. Description Frames (Decision 5)

- `COMM:MusicIndex Description` holds the item description only.
- When the item has no description, `COMM:MusicIndex Album Description` holds the channel description.
- Add the album description row with its frame hint. The view model gives its label.

### 4. Idempotent Writes (Decision 6)

- For each frame that the app owns, a write removes each value that the app supplies, and then adds the current values.
- The write recognizes each earlier form of an app value as an app value: a value with a label before a URL, a track page in `WOAR`, and the channel description in `COMM:MusicIndex Description`.
- A write keeps each value that neither RSS nor MusicBrainz supplied.
- A write never removes a MusicBrainz value. Packet 002 changes the MusicBrainz URL values. Until then, keep each MusicBrainz value that the present code writes.
- Apply the same rules in `write_mp3_edits` and in `write_lofty_edits`.

### 5. Compare And Scan (Decisions 3 and 4)

- The compare grid checks each frame against the value that the writer would write for the same track and feed.
- `changed_frames` reports a difference when the file lacks an expected value, and when the file holds an earlier form of an app value.
- A value that neither source supplied is not a difference.
- The scan reports each file with an earlier mapping. "Update n file(s)" rewrites it only after the operator confirms.

## Mechanical Acceptance Criteria

Use the prefix `adr_0080_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R80-01 | The edits for a track with a valid own Nostr key hold that key. Without one, they hold the valid channel key |
| R80-02 | An invalid item key gives the valid channel key. Two invalid keys give no Nostr edit |
| R80-03 | The edits hold the item page in `WOAF` and the channel website in `WOAR`, and neither value in the other frame |
| R80-04 | Each URL edit value parses as a URL and holds no label text |
| R80-05 | A track without its own description gives `COMM:MusicIndex Album Description` with the channel description, and no `COMM:MusicIndex Description` |
| R80-06 | A write of the same edits twice to an MP3 file gives equal frames. Repeat for FLAC |
| R80-07 | An MP3 file with `WOAR` = `download for free (url, forward): <channel url>` and `WOAR` = `<item page>` holds only `WOAR` = `<channel url>` and `WOAF` = `<item page>` after one write |
| R80-08 | A write keeps a `WOAR` value that no source supplied, and a MusicBrainz `WOAR` value |
| R80-09 | Round trip: for each frame, the compare of fresh edits against the same track and feed reports no difference |
| R80-10 | `changed_frames` reports a file that holds an earlier labeled `WOAR` value, and does not report a file that holds a foreign `WOAR` value |
| R80-11 | Update the packet 022 fixed edits (R22-06) to the new output. Each changed line matches a decision of ADR 0080 |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: after "Update n file(s)" on a test copy, an external tag reader (for example `mid3v2 -l` or `kid3-cli`) shows the item page in `WOAF`, the channel website in `WOAR`, and plain URLs.
- V2: a second "Update n file(s)" on the same file adds no frame in the external reader.
- V3: after the update, the next scan shows no difference for that file.
- V4: the compare grid shows the album description row with its owner, in Light and Dark themes, with no clipped text.

## Exclusions

- No change to the MusicBrainz URL values. Packet 002 owns them.
- No change to the route frame (ADR 0076 Decision 9) or to the confirmation flow.
- No change to the database schema. The database keeps each value with its owner.
- No `WXXX` frame.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/metadata.rs`: `id3_frame_hint`, `track_metadata_rows`, `source_value_for_metadata_field`, `format_source_value_for_id3v24`, `expand_woar_metadata_rows`, `woar_metadata_urls`, and the URL read near line 831.
- `src/metadata_service.rs`: `id3_edits_for_track_context` and the R22-06 tests.
- `src/audio_tags.rs`: `write_id3v24_edits`, `write_mp3_edits`, `write_lofty_edits`, and the frame list.
- `src/tag_field.rs`: the `WOAR` and `WOAF` mappings for Vorbis and MP4.
- `src/application/queries/tag_update.rs`: `changed_frames` and `values_match`.
- `src/application/commands/tag_update.rs`: `write_tag_updates`.

## Checks

```bash
cargo test --lib adr_0080_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data. A file that a test build wrote keeps its new frames. Use a test copy of each file for the visual check.

## Implementation Result - 2026-09-29

### Nostr Key (R80-01, R80-02)

`metadata_service::id3_edits_for_track_context` now resolves one Nostr key. It calls the new
`apply_resolved_nostr_key`, which removes any Nostr edit that the row pass staged. It then adds one
edit with the item's own key, when `rss::validate_nostr_identity` reports a valid `npub`. It falls
back to the channel's key, under the same rule. When neither key is valid, it adds no edit.

### Website Frames (R80-03, R80-04)

`id3_frame_hint`, in `src/metadata.rs`, now maps "Website" (the item's own page) to `WOAF`. It maps
"RSS feed website" (the channel) to `WOAR`. `track_metadata_rows` now always adds the channel row,
because the two frames no longer collide. `format_source_value_for_id3v24` no longer adds the
"download for free (url, forward):" label. `format_drag_value_for_id3v24` now sends `WOAR` and
`WOAF` values through the new `format_id3_website_url`. It parses the value with `reqwest::Url`
(an existing dependency), and writes no edit when the value does not parse.

### Description Frames (R80-05)

`source_value_for_metadata_field` now splits "Description" (the item only) from a new
"Album description" field (the channel, used only when the item has none). `id3_frame_hint` maps
"Album description" to `COMM:MusicIndex Album Description`. `track_metadata_rows` and
`aligned_compare_rows` add this row only when the item has no description of its own.
`id3_value_for_field` and `id3_compare_value_for_field` now match these two frames by their exact
label, through the new `id3_exact_frame_value`. The loose needle search that the other `COMM`
fields use would match both frames for either row, because their needles overlap.

### Idempotent Writes (R80-06, R80-07, R80-08)

`write_mp3_edits`, in `src/audio_tags.rs`, now calls `remove_stale_id3_website_frames` and
`remove_stale_id3_description_frames` before it adds each edit. The website cleanup removes a
`WOAR` or `WOAF` frame whose embedded URL, with any label stripped, is about to be written again
under either label. The description cleanup clears both description frames whenever an edit sets
either one, because a track holds at most one of them.

`write_lofty_edits` gained the same two proof points, plus a third pass. Three cleanup passes now
run once, before the edit loop: `remove_stale_lofty_website_items` (the website value match),
`remove_stale_lofty_description_items` (Decision 5, described below), and the new
`remove_stale_lofty_keyed_items`. That last pass removes, for each key that this write's edits
give a non-website field, an existing item under that key. It removes each key once, not once for
each edit.

The edit loop itself now only pushes. `remove_lofty_item_by_key` no longer runs inside it. A
standard key, such as `TITLE`, reads back as its own typed `ItemKey`. It does not read back as
`ItemKey::Unknown("TITLE")`. So `insert_unchecked` could not replace it by comparing `ItemKey`
values.

`remove_lofty_item_by_key` compares the mapped key string instead, and it runs once, ahead of the
loop. `remove_stale_lofty_website_items` still mirrors the website value match, for the Vorbis key
and the MP4 freeform key, so `WOAR` can still carry more than one value.

### Album Description Round Trip On FLAC And MP4 (Orchestrator Review, Defect 1)

Vorbis Comments and MP4 freeform atoms mapped both `COMM:MusicIndex Description` and
`COMM:MusicIndex Album Description` to the one shared Comment key
(Vorbis `COMMENT`, MP4 `©cmt`), through `TagFieldId::Comment`. A file could hold only one of the
two. A scan of a FLAC track with a channel description, and no item description, always reported a
difference. The value read back only under the item's label.

The new `lofty_field_for_label` gives `COMM:MusicIndex Album Description` its own key, the same
way a `TXXX` descriptor does (`TagFieldId::Custom("MusicIndex Album Description")`).
`COMM:MusicIndex Description` keeps the shared Comment key. `remove_stale_lofty_description_items`
clears each label's key, whenever an edit sets either one, so a track holds at most one of the two
(Decision 5). `add_lofty_album_description_alias` runs in `read_lofty_tags`'s alias step. It reads
the round-tripped `TXXX`-style field back as `COMM:MusicIndex Album Description`,
case-insensitively. The compare grid and the scan then find it under the frame the writer gives it
(Decision 3).

The new `adr_0080_flac_album_description_round_trips_with_no_difference` test, beside
`changed_frames`, proves this: a FLAC file with a written album description shows no difference
against the same edit.

### Compare And Scan (R80-09, R80-10)

`application::queries::tag_update::changed_frames` needed no code change. It already compares the
writer's current edits against the file's raw frame values, so it inherits every fix above. Two
`adr_0080_` tests confirm this. One is a round-trip write-then-read check, beside
`id3_edits_for_track_context`. The other is a scan test: it reports an earlier labeled `WOAR`
value, and it stays silent on an unrelated one.

### Files Changed

- `src/metadata.rs`
- `src/metadata_service.rs`
- `src/audio_tags.rs`
- `src/discover/tests.rs`
- `src/discover/app_impl.rs`
- `src/ui/shells/discover/track_inspector_metadata_test_helpers.rs`
- `src/application/queries/tag_update.rs`
- This packet document.

### Tests

Each command in "Checks" is Green.

- `cargo test --lib adr_0080_`: 13 tests pass.
- `cargo test`: 1,880 library tests pass, 283 architecture-guard tests pass, and 10 doc-tests stay
  ignored. All Green.
- `cargo test --test architecture_tests`: 283 tests pass.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green, with no warning.
- `cargo build --bin v4vmm`: Green.

### Orchestrator Review Fixes - 2026-09-29 (Defects 2 To 4)

The orchestrator reviewed the first result and found four defects. Defect 1 (the FLAC and MP4
album description round trip) is described above, under its own heading.

**Defect 2.** `remove_lofty_item_by_key` ran once for each edit, inside the write loop. Two edits
that share one Vorbis key, such as `TDRC` and `TYER` (both `DATE`), gave only the second edit's
value. The second edit's own removal erased the first edit's item. The fix moves this removal to
the new `remove_stale_lofty_keyed_items`, which runs once for each owned key, before the loop. The
new `adr_0080_two_edits_sharing_one_vorbis_key_keep_both_values` test proves both values now stay.

**Defect 3.** `format_source_value_for_id3v24` no longer takes a `source: MetadataColumn`
parameter. Its two call sites, in `src/metadata.rs` and `src/discover/app_impl.rs`, no longer pass
one. `src/ui/shells/discover/track_inspector_metadata_test_helpers.rs` no longer carries its own
copy of `id3_frame_hint`. It now re-exports the production function from `crate::metadata`, so the
two mappings cannot drift apart again.

**Defect 4.** The "Operator Visual Check" section now runs the app on an isolated fixture, copied
from the real database, with a music folder that starts empty. It follows the database-copy
pattern in `docs/runbooks/inherited-ui-checks.md`. Every write in the check, and every `mid3v2`
edit, now lands in that fixture. Only one step still reads a file under the real music folder, to
copy it into the fixture once. The confirm step ("Write Tags") that step 10 skipped is now its own
step 11.

### Deviations From The Task

None. The two deviations of the first result are corrected above. `format_source_value_for_id3v24`
has no unused parameter now. The stale test-helper copy of `id3_frame_hint` is
gone.

### Unresolved Concerns

- Cleanup of the evidence fixture `/tmp/v4vmm-governance.ie6k8TQf` stays unconfirmed. This is
  unrelated to this packet. The standing note in `AGENTS.md` records this fact.

## Operator Visual Check

This check needs a Linux desktop session, this checkout, and the `sqlite3`, `python3` and `mid3v2`
commands (`kid3-cli` also works for the read steps).

**This check runs the app on an isolated fixture, copied from the real database.** The fixture's
own music folder starts empty and gains only the one or two test files this check copies into it.
No step writes to a file under the real music folder. Only step 6 reads one.

1. Close v4vmm. Build the app:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```

2. Open the real app, read-only, to find the two tracks this check needs. Open Library tracks
   until you find one whose "Website" row (the item's own page) and "RSS feed website" row (the
   channel's website) both show, with different URLs. Write down its track id and each URL.

   Stop, and report the result, if no track has both rows with different URLs.
3. In the same app session, open Library tracks until you find one whose page shows an
   "Album description" row. Write down its track id. Close the app.

   Stop, and report the result, if no track shows that row. This row already proves ADR 0080
   Decision 5's condition: the item states no description of its own, and the channel does.
4. Set the two track ids and the two URLs from steps 2 and 3:

   ```bash
   T=<track id from step 2>
   D=<track id from step 3>
   ITEM_URL=<the "Website" row value>
   CHANNEL_URL=<the "RSS feed website" row value>
   ```

   `T` and `D` can be the same track, when one track satisfies both steps.
5. Create the fixture root, and copy the real database into it. The fixture's own music folder
   starts empty:

   ```bash
   gate_dir=$(mktemp -d /tmp/v4vmm-adr-0080-task-001.XXXXXXXX)
   python3 -c 'import json, os, pathlib, sqlite3, sys, tomllib
root = pathlib.Path(sys.argv[1]).resolve()
source = pathlib.Path(os.environ.get("XDG_CONFIG_HOME", str(pathlib.Path.home()/".config")))/"v4vmm/config.toml"
cfg = tomllib.loads(source.read_text())
target = root/"config/v4vmm"
target.mkdir(parents=True)
(root/"music").mkdir()
src = sqlite3.connect(pathlib.Path(cfg["db_path"]).resolve().as_uri()+"?mode=ro", uri=True, timeout=5)
dst = sqlite3.connect(root/"app.sqlite", timeout=5)
src.backup(dst)
dst.close()
src.close()
settings = {"music_dir": str(root/"music"), "db_path": str(root/"app.sqlite")}
settings.update({"musicindex_endpoint": cfg["musicindex_endpoint"]} if "musicindex_endpoint" in cfg else {})
(target/"config.toml").write_text("\n".join(k+" = "+json.dumps(v) for k,v in settings.items())+"\n")
print("Fixture database copy ready:", root)' "$gate_dir"
   ```

   Expect the printed path to equal `$gate_dir`. Stop, and report the result, if the script fails.
   The scan of step 10 skips a track record whose file it cannot find on disk. A track whose file
   this check does not copy in step 6 stays absent from the fixture's scans for this reason.
6. Read the real file path of each track, from the fixture's own database copy. Copy each file into
   the fixture's music folder only:

   ```bash
   real_music=$(sed -n 's/^music_dir = "\(.*\)"$/\1/p' ~/.config/v4vmm/config.toml)
   for id in $T $D; do
     p=$(sqlite3 "$gate_dir/app.sqlite" "SELECT path FROM local_files WHERE track_id = $id;")
     case "$p" in /*) real_f="$p" ;; *) real_f="$real_music/$p" ;; esac
     mkdir -p "$(dirname "$gate_dir/music/$p")"
     cp -p "$real_f" "$gate_dir/music/$p"
   done
   ```

   The copy keeps the stored relative path, because the app finds a file at the music folder joined with that path.
   Stop, and report the result, if a real file is missing. No later step reads or writes a file
   under `$real_music`. Every later step reads and writes only `$gate_dir/music`.
7. Set the fixture's own copy of track `$T`'s file:

   ```bash
   p=$(sqlite3 "$gate_dir/app.sqlite" "SELECT path FROM local_files WHERE track_id = $T;")
   f="$gate_dir/music/$p"
   ```

**V1 — the item page and the channel website go to their own frame, as plain URLs**

8. Add the earlier mapping to the fixture's own copy of the file:

   ```bash
   mid3v2 --WOAR "download for free (url, forward): $CHANNEL_URL" "$f"
   mid3v2 --WOAR "$ITEM_URL" "$f"
   ```

9. Launch the fixture from this terminal:

   ```bash
   (
     export XDG_CONFIG_HOME="$gate_dir/config"
     export XDG_DATA_HOME="$gate_dir/data"
     export XDG_CACHE_HOME="$gate_dir/cache"
     target/debug/v4vmm
   )
   ```

10. Look at the header row of the Music source list. It shows an "Update n file(s)" button, with a
    count of one. Click it, and find track `$T`'s row in the popup.
11. Click **Write Tags**. Wait for the report below the header row. Close the app.
12. Read the file's tags:

    ```bash
    mid3v2 -l "$f"
    ```

    - Correct: `WOAF` holds one value, the item's own page (`$ITEM_URL`). `WOAR` holds one value,
      the channel website (`$CHANNEL_URL`). Neither value carries text before the URL.
    - Wrong: `WOAR` holds more than one value.
    - Wrong: a value carries text before the URL, such as "download for free".
    - Wrong: the item's page sits in `WOAR`, or the channel's website sits in `WOAF`.

**V2 — a second "Update n file(s)" adds no frame**

13. Launch the fixture again, the same way as step 9.
    - Correct: the "Update n file(s)" button is absent.
14. If the button shows, click it, then click **Write Tags** again. Close the app. Read the file's
    tags:

    ```bash
    mid3v2 -l "$f"
    ```

    - Correct: the file has the same `WOAF` value and the same `WOAR` value as step 12, each once.
    - Wrong: a frame value repeats, or a value changed.

**V3 — the next scan shows no difference**

15. Launch the fixture once more, the same way as step 9. Look at the header row of the Music
    source list.
    - Correct: the "Update n file(s)" button is absent.
    - Wrong: the button shows track `$T` after two confirmed writes.
16. Close the app.

**V4 — the album description row, in Light and Dark**

17. Launch the fixture again, the same way as step 9. Open Settings → General (`Ctrl+Comma`).
    Select Light. Open track `$D`'s page, and open its Metadata compare grid.
    - Correct: a row reads "Album description", holding the channel's description text. Its ID3
      frame column reads `COMM:MusicIndex Album Description`.
    - Wrong: no such row shows.
    - Wrong: the row's text is cut off, in a way that a wider window would not fix by wrapping.
18. Select Dark (Settings → General). Open the same page and the same grid again.
    - Wrong: the row reads in only one of the two themes.
    - Wrong: the two themes show the row with the same text but a difference only in its color.
19. Close the app.

**Cleanup**

20. Remove the fixture root:

    ```bash
    rm -rf "$gate_dir"
    ```

    Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. It is a separate, unrelated fixture.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0080-task-001-rss-frames-and-idempotent-writes.md`
- ADR 0080, all decisions
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the Nostr key, the website frames, the description frames, idempotent writes, and the compare and scan.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Write R80-07 and R80-08 first, with real temporary files, before you change the writer.
- The view model gives each row label. The screen only composes.
- Treat RSS and MusicIndex values as untrusted input.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The MusicBrainz lookup and `release_url_values`.
- The route frame and `with_stored_route_frame`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R80-01 to R80-11 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0080_`
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
- The writer cannot tell a MusicBrainz value from an RSS value in a frame.
- The lofty writer cannot remove one value of a multi-value item and keep the others.
- The NIP-19 validation needs a new crate.
- A change needs a file in "Do not touch".

## Orchestrator Review - 2026-09-29

The orchestrator reviewed the diff two times and ran each check. Each check is Green: 1,880 unit tests, 283 guards, and no warning.

The first review found four defects. The implementer corrected each one:

- On FLAC, Ogg and MP4, the album description shared the comment key, and each scan reported a difference. It now has its own key, and a FLAC round-trip test proves it.
- The FLAC writer removed a key once for each edit. It now removes each owned key once, before the loop.
- An unused parameter and a stale test copy of `id3_frame_hint` are deleted.
- The operator check wrote to the Library. It now uses an isolated database copy, and one music folder that holds only the test copies.

The orchestrator corrected one more defect in the check. The copy step put each file at its base name, but a stored path can hold folders. The copy now keeps the stored relative path.
Stored paths are relative (`LibraryPath::resolve` in `src/library_path.rs`), so the fixture app cannot reach a file in the real music folder.
