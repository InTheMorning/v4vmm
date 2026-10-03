# ADR 0080 Task 003: The Vorbis Date Shares One Key

Status: Complete - 2026-10-03. Mechanical checks Green. The operator passed V1 on the real Library on 2026-10-03.
This packet changes no presentation. Its operator check reads tags and the button count.

## Goal

A FLAC, Ogg Vorbis or Ogg Opus file with a release date settles after one "Update n files" write. The next scan shows no difference for that file.

## Authority

- [ADR 0080](../../adr/0080-tag-frames-follow-their-owner.md) Decision 6: a write is idempotent. A second write with the same inputs changes no frame.
- [ADR 0076](../../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 8: the scan counts the files that differ from the stored values.

## Recorded Facts - 2026-10-03

- The operator wrote tags to 6 new Delta OG FLAC files. The button count stayed at 6 after each write. 28 MP3 files settled after one write.
- One written FLAC file holds only `DATE=2024`. The stored item date is `Mon, 20 May 2024 19:54:50 +0000`.
- `track_metadata_rows` in `src/metadata.rs` gives "Release date" (`TDRC`) and "Release year" (`TYER`) from the same value.
- `TagFieldId::from_id3_label` in `src/tag_field.rs` maps `TDRC` and `TYER` to `Date`. Its Vorbis key is `DATE`. Thus the two edits share one key on Vorbis Comments.
- The reader gives each `DATE` value as frame `TDRC`. The test `adr_0080_two_edits_sharing_one_vorbis_key_keep_both_values` in `src/audio_tags.rs` asserts this.
- `changed_frames` in `src/application/queries/tag_update.rs` finds the file values by `pending_id3_target_key`. For the `TYER` edit it looks for frame `TYER`, and finds none. Thus a Vorbis file with a date always differs.
- `TagUpdateFile::edits` gives only the changed frames. `remove_stale_lofty_keyed_items` in `src/audio_tags.rs` removes each `DATE` value before it adds the edits.
  A write of `TYER` alone thus also removes the `TDRC` value. The next scan then reports `TDRC`. The file changes between the two states at each write.
- No test writes the full set of edits of `id3_edits_for_track_context` to a FLAC file and scans it again.

## Required Changes

1. The scan compares a file value with an expected value by the storage key of the file format. On Vorbis Comments, the `TYER` edit and the `TDRC` edit both read the `DATE` values.
   Keep the present rule of `changed_frames`: a frame is equal when one file value matches the expected value.
2. A write includes each expected edit that shares a storage key with a changed edit. A write never removes the value of an unchanged edit with the same key.
3. MP3 behavior does not change. `TDRC` and `TYER` stay two frames on ID3v2.
4. Keep both values on Vorbis Comments, as the present test requires. Do not change which frames the app writes.

## Mechanical Acceptance Criteria

Use the prefix `adr_0080_shared_key_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R80-3-01 | A FLAC fixture written with the full output of `id3_edits_for_track_context` for a track with an item date gives no changed frame in the next scan |
| R80-3-02 | A FLAC fixture that holds only `DATE=<year>` gives a scan that lists the `TDRC` change. After the write of that scan, the file holds both values, and the next scan gives no changed frame |
| R80-3-03 | A second write of the R80-3-01 file changes no value and adds no duplicate `DATE` value |
| R80-3-04 | An MP3 fixture with the same track gives the same scan result as before this packet |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: after one "Update n files" write on the real Library, the count goes to 0 when no other file differs. A FLAC file then holds both `DATE` values.

## Exclusions

- No change to the confirmation popup. [ADR 0076 packet 008](../adr-0076-task-008-confirmation-list-with-many-items.md) owns it.
- No change to the frames that the app writes, and no change to the date rule of ADR 0075 packet 049.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../architecture/source-map.md).
- `src/application/queries/tag_update.rs`: `changed_frames`, `frame_is_compared`, `TagUpdateFile::edits`, and the `settle` test.
- `src/audio_tags.rs`: `write_id3v24_edits`, `remove_stale_lofty_keyed_items`, and the ADR 0080 tests.
- `src/metadata.rs`: `pending_id3_target_key`, `frame_destination_for_format`, `track_metadata_rows`.
- `src/tag_field.rs`: `TagFieldId::from_id3_label` and `vorbis_key`.
- `docs/runbooks/fixtures/conversion.flac`: the FLAC fixture.

## Checks

```bash
cargo test --lib adr_0080_shared_key_
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

This check writes tags to the real Library once. It reads the tags of one Delta OG FLAC file before and after that write.
It does not change the database, and it starts no fixture.

Needs: a Linux desktop session, the `metaflac` command (package `flac`), and the 6 Delta OG FLAC files under
`/home/citizen/V4Vmusic/artists/Delta OG/Aged Friends & Old Whiskey/`. Before this check, each file holds only `DATE=2024`,
and the "Update n files" button shows 6. Close the app before step 1.

**Setup**

1. Read the tags of each Delta OG file before the write:

   ```bash
   cd "/home/citizen/V4Vmusic/artists/Delta OG/Aged Friends & Old Whiskey/"
   for f in *.flac; do echo "== $f"; metaflac --export-tags-to=- "$f" | grep '^DATE='; done
   ```

   Each file shows one line, `DATE=2024`. Record the output.

2. Build and open the desktop binary:

   ```bash
   cd /home/citizen/build/v4vmm && cargo build --bin v4vmm && target/debug/v4vmm
   ```

   Run this command first. A prior `cargo test` run can leave a GPUI test-support binary at `target/debug/v4vmm`.

**V1 - one write settles each FLAC file**

3. Open Music's Library. Wait for the "Update n files" button. Read its count.
   - Expected: 6, or 6 plus other files that differ.
4. Select the button. In the popup, read the frames of one Delta OG file.
   - Expected: the file shows a `TDRC` frame.
   - This result is wrong: the file shows a `TYER` frame.
5. Select "Write Tags". Wait until the write report shows.
6. Read the "Update n files" button again.
   - Expected: the count goes to 0, when no other file differs.
   - This result is wrong: the count stays at 6.
   - When the count is above 0, select the button. Confirm that no Delta OG file is in the list.
7. Close the app. Read the tags of each Delta OG file again:

   ```bash
   cd "/home/citizen/V4Vmusic/artists/Delta OG/Aged Friends & Old Whiskey/"
   for f in *.flac; do echo "== $f"; metaflac --export-tags-to=- "$f" | grep '^DATE='; done
   ```

   - Expected: each file shows two lines. One line is `DATE=2024`. The other line is a full date, for example `DATE=2024-05-20`.
   - This result is wrong: a file shows only one `DATE` line.
   - This result is wrong: a file shows the same `DATE` line two times.
8. Open the app again with `target/debug/v4vmm`. Open Music's Library.
   - Expected: no Delta OG file comes back in the "Update n files" count.

**Cleanup**

9. No cleanup is necessary. The write is the intended correction of the Library files.
   The app made no fixture. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/archive/adr-0080-task-003-vorbis-date-shares-one-key.md`
- ADR 0080 Decision 6
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the scan compares by the storage key of the format, and a write includes each edit that shares a key with a changed edit.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use the fixture files in the repository. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The confirmation popup and `src/ui/`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R80-3-01 to R80-3-04 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The reader gives the two `DATE` values in a form that a key comparison cannot separate.
- A change needs a different set of frames than the app writes today.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-03

### Files Changed

- `src/audio_tags.rs`: the new `vorbis_storage_key` gives the Vorbis Comment key of a frame label. It uses the key function of the writer.
- `src/application/queries/tag_update.rs`: the scan finds the file values of a frame by its storage key on FLAC, Ogg Vorbis and Ogg Opus.
  The `Differs` content holds `shared_key_edits`, and `TagUpdateFile::edits` adds them after the changed frames. Four tests have the prefix `adr_0080_shared_key_`.
- `src/view_models/tag_update.rs`: the match on `Differs` ignores the new field. The test fixture gives an empty list.
- This packet: the status line, this result, and the operator check.

### Tests Run

- `cargo test --lib adr_0080_shared_key_`: 4 passed.
- `cargo test`: 1791 library tests and 289 guard tests passed. 10 documentation tests are ignored.
- `cargo test --test architecture_tests`: 289 passed.
- `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets` and `cargo build --bin v4vmm`: Green, with no warning.
- Each new FLAC test fails without the fix. R80-3-01, R80-3-02 and R80-3-03 fail without the key comparison.
  R80-3-02 also fails without the shared-key edits. R80-3-04 passes with and without the fix, because it guards the MP3 result.

### Behavior Changed

- On a FLAC, Ogg Vorbis or Ogg Opus file, the `TYER` edit and the `TDRC` edit both read the `DATE` values. Each edit is equal when one `DATE` value matches it.
- A write of a changed `TDRC` edit also writes the unchanged `TYER` edit, and the reverse. The file keeps both values.
- On these formats, each other frame also uses its Vorbis key. The description frame thus reads each `COMMENT` value, not only the first one.
- MP3 and MP4 files keep the ID3 target key. Their scan result and their write do not change.

### Deviations From Task

- `src/view_models/tag_update.rs` changed in two lines, because it matches and builds the `Differs` content. The popup text does not change.

### Unresolved Concerns

- MP4 files have a related defect. The writer keeps `TDRC` and `TYER` in two freeform atoms, but the reader gives both as `TDRC`.
  Thus the scan always reports `TYER` on an MP4 file with a date. The reader label cannot show the atom, so a key comparison cannot separate the two values.
  This packet does not change MP4. A later packet must decide the MP4 fix.
