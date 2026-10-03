# ADR 0080 Task 003: The Vorbis Date Shares One Key

Status: Ready - 2026-10-03. Implementation has not started.
This packet changes no presentation. Its operator check reads tags and the button count.

## Goal

A FLAC, Ogg Vorbis or Ogg Opus file with a release date settles after one "Update n files" write. The next scan shows no difference for that file.

## Authority

- [ADR 0080](../adr/0080-tag-frames-follow-their-owner.md) Decision 6: a write is idempotent. A second write with the same inputs changes no frame.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 8: the scan counts the files that differ from the stored values.

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

- No change to the confirmation popup. [ADR 0076 packet 008](adr-0076-task-008-confirmation-list-with-many-items.md) owns it.
- No change to the frames that the app writes, and no change to the date rule of ADR 0075 packet 049.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
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

The implementer writes this section at completion. It gives numbered steps for V1.
The steps read one Delta OG FLAC file with `metaflac --export-tags-to=-` before and after one write, and read the button count.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0080-task-003-vorbis-date-shares-one-key.md`
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
