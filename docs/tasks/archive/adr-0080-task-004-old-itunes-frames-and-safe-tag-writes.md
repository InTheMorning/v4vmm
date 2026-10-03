# ADR 0080 Task 004: Old iTunes Frames And Safe Tag Writes

Status: Complete - 2026-10-03. Mechanical checks Green. The operator passed V1 on the real Library on 2026-10-03.

## Goal

A tag write succeeds on an MP3 file with an old ID3v2.2 tag from iTunes or Logic Pro. The write keeps the values of that tag. A failed write never removes the old tag of a file. A download reports a tag-write failure to the operator.

## Authority

- [ADR 0080](../../adr/0080-tag-frames-follow-their-owner.md) Decision 6: "A write keeps each value that neither RSS nor MusicBrainz supplied, for example a value from another tool."
- [ADR 0066](../../adr/0066-configuration-and-startup-failure-recovery.md): a failure is visible to the operator, not confined to stderr.
- AGENTS.md "Write reports for the operator".

## Recorded Facts - 2026-10-03

- The operator downloaded "Disco Swag - The Album" by The Doerfels. 17 files got their tags. "Make It" has no ID3 tag at all.
- The enclosure `https://www.doerfelverse.com/tracks/make-it.mp3` (7528361 bytes) starts with an ID3v2.2 tag of 10601 bytes. Its frames are `TSS`, `COM`, `COM`, `TT2`, `TP1`, `TAL` and `TSP` ("Doerfels"). `TSP` is the iTunes v2.2 performer sort frame.
- `write_mp3_edits` in `src/audio_tags.rs` reads the old tag with `Tag::read_from_path` and keeps each frame. id3 1.16.4 converts v2.2 frame IDs with a fixed table that has no `TSP`, `TS2`, `TSA`, `TST`, `TSC` or `TCP`. The frame keeps its 3-byte ID.
- `tag.write_to_path(path, Version::Id3v24)` then fails: `InvalidInput: Frame ID must be 4 bytes long`.
- The id3 encoder writes over the old tag area. After the failure, the file was 7517760 bytes (7528361 − 10601), and it held no tag.
- `apply_id3_edits_nonfatal` in `src/subscribe_service.rs` prints `skip tag write for ...` to stderr and returns 0. The download then saves the file as a success.
- Each caller of `write_id3v24_edits` has the same exposure: the download, "Update n files" (`src/application/commands/metadata.rs`), the payment-route write (`src/application/commands/payment_routes.rs`), `src/playback_owner.rs` and `src/application/queries/broadcast.rs`.
- The scratchpad copy of the real file is not in the repository. A test builds its own fixture.

## Required Changes

1. Before an MP3 write, convert each iTunes v2.2 frame that the id3 table does not convert: `TSP` → `TSOP`, `TSA` → `TSOA`, `TST` → `TSOT`, `TS2` → `TSO2`, `TSC` → `TSOC`, `TCP` → `TCMP`. Keep the value.
2. Remove each other frame whose ID is not 4 bytes long. The write result names each removed frame.
3. A write encodes the tag into memory first, and then replaces the file through a staged copy and a rename in the same directory. A failed encode or a failed write leaves the file unchanged.
4. A download that cannot write tags saves the track, and its result says so, with the error text: "App saved the track but could not write its tags: ...". Use the warning that the download result already carries. Do not count the track as a download failure.
5. Each other caller gets the safe write through the shared writer. No caller changes its own error rule.

## Mechanical Acceptance Criteria

Use the prefix `adr_0080_old_itunes_frames_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R80-4-01 | A fixture MP3 with a v2.2 tag that holds `TT2` and `TSP` takes a write of `TIT2`, `TALB`, `TPE1` and a `TXXX`. The file then has a v2.4 tag with the written values and `TSOP` with the `TSP` value |
| R80-4-02 | A fixture with a v2.2 frame that has no mapping takes the write. The frame is removed, and the write result names it |
| R80-4-03 | A write that fails during the encode leaves the file bytes unchanged |
| R80-4-04 | A download whose tag write fails saves the track, and its result carries the tag-write warning |
| R80-4-05 | A write on a file with a v2.3 or v2.4 tag gives the same result as before this packet |

## Visual Acceptance Criteria

None. The operator check reads tags and the download report.

## Exclusions

- No change to the frames that the app supplies, the scan, or the popup.
- No change to the failure classes of ADR 0066 task 014. A tag-write failure is a warning on a saved track.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../../.github/copilot-instructions.md).
- `src/audio_tags.rs`: `write_id3v24_edits`, `write_mp3_edits`, `no_tag_ok`, and the ADR 0080 tests.
- `src/subscribe_service.rs`: `apply_id3_edits_nonfatal`, `SubscribeTrackOutcome`.
- `src/subscribe_service/materialization.rs`: `run_steps` and `outcome`.
- `src/application/commands/download.rs`: `SubscribeTrackResult`.
- The id3 crate source in `~/.cargo/registry`: the v2.2 conversion table and `Encoder`.

## Checks

```bash
cargo test --lib adr_0080_old_itunes_frames_
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

This check needs a Linux desktop session, this checkout, network access to MusicIndex and to
`www.doerfelverse.com`, and the `mid3v2` command. The check uses an isolated fixture with an empty
database and an empty music folder. No step reads or writes the real configuration, database or
music folder. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

1. Close v4vmm. Build the normal desktop binary:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```

   Do this step after each `cargo test`. A test build can replace `target/debug/v4vmm`.
2. Create the fixture. The script copies only the MusicIndex endpoint from the real configuration:

   ```bash
   gate_dir=$(mktemp -d /tmp/v4vmm-adr-0080-task-004.XXXXXXXX)
   mkdir -p "$gate_dir/config/v4vmm" "$gate_dir/data" "$gate_dir/cache" "$gate_dir/music"
   python3 -c 'import json, pathlib, sys, tomllib
root = pathlib.Path(sys.argv[1]).resolve()
cfg = tomllib.loads((pathlib.Path.home()/".config/v4vmm/config.toml").read_text())
settings = {"music_dir": str(root/"music"), "db_path": str(root/"app.sqlite")}
settings.update({"musicindex_endpoint": cfg["musicindex_endpoint"]} if "musicindex_endpoint" in cfg else {})
(root/"config/v4vmm/config.toml").write_text("\n".join(k+" = "+json.dumps(v) for k,v in settings.items())+"\n")
print("Fixture ready:", root)' "$gate_dir"
   ```

   Expect the printed path to equal `$gate_dir`. Stop, and report the result, if the script fails.
3. Start the app on the fixture:

   ```bash
   env XDG_CONFIG_HOME="$gate_dir/config" XDG_DATA_HOME="$gate_dir/data" \
     XDG_CACHE_HOME="$gate_dir/cache" target/debug/v4vmm
   ```

4. In Music, search for `Disco Swag`. Open the album "Disco Swag - The Album" by The Doerfels.
   Download the album.
5. Look at the download result for each track.
   - Correct: "Make It" downloads with no tag-write warning.
   - Wrong: a track shows "App saved the track but could not write its tags: ...". Write down
     the full warning text.
   - Wrong: a track shows as a download failure because of its tags.
6. Close the app. Find the file of "Make It", and read its tags:

   ```bash
   f=$(find "$gate_dir/music" -iname '*make it*' -o -iname '*make-it*' | head -n 1)
   echo "$f"
   mid3v2 -l "$f"
   ```

   - Correct: the output shows `TIT2=Make It` and `TSOP=Doerfels`, and the album and artist frames.
   - Wrong: `$f` is empty, `mid3v2` reports no tag, or `TSOP=Doerfels` is absent.
7. Read the tags of one other track of the album with `mid3v2 -l`. Expect its title, album and
   artist frames, as before this packet.
8. Clean up the fixture:

   ```bash
   rm -rf "$gate_dir"
   ```

   Then start the real app again with `target/debug/v4vmm`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/archive/adr-0080-task-004-old-itunes-frames-and-safe-tag-writes.md`
- ADR 0080 Decision 6
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": convert old iTunes frames, write safely, and report a download tag-write failure.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment, each report text and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests build their own fixtures. No test sends a request to a remote host.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The tag scan, the popup and `src/ui/`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R80-4-01 to R80-4-05 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The id3 crate gives no way to encode a tag into memory, and a staged copy cannot keep the audio bytes unchanged.
- A caller depends on a write that changes the file in place.

## Implementation Result - 2026-10-03

### Files Changed

- `src/audio_tags.rs`: the type `TagWriteResult` and the conversion of old iTunes frames.
  The file removes each frame with an ID that is not 4 bytes long. It holds the safe write
  through a staged copy. It holds the tests R80-4-01, R80-4-02, R80-4-03 and R80-4-05.
- `src/subscribe_service.rs`: `apply_id3_edits_nonfatal` returns the tag-write warning. It prints
  nothing to stderr now.
- `src/subscribe_service/materialization.rs`: the download adds the tag-write warning to the
  warning of its result, and the test R80-4-04.
- `src/application/commands/tag_update.rs` and `src/application/commands/payment_routes.rs`:
  these callers read the count from `TagWriteResult::applied`. Their error rules stay the same.
- This packet: the status, this result and the operator check.

### Tests Run

- Before the change, R80-4-01 failed with `InvalidInput: Frame ID must be 4 bytes long`.
- Before the change, R80-4-03 failed. The failed encode removed the old tag, and only the audio
  bytes stayed in the file.
- `cargo test --lib adr_0080_old_itunes_frames_`: 5 tests, Green.
- `cargo test`: Green. `cargo test --test architecture_tests`: Green.
- `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets` and
  `cargo build --bin v4vmm`: Green. `cargo check --all-targets` gives no warning.
- A manual check on a copy of the real "Make It" file: the write applied 4 edits and removed no
  frame. `mid3v2 -l` shows `TIT2=Make It` and `TSOP=Doerfels`. The bytes after the tag are equal to
  the bytes after the old tag. The repository holds no copy of that file.

### Behavior Changed

- An MP3 write converts `TSP`, `TSA`, `TST`, `TS2`, `TSC` and `TCP` to `TSOP`, `TSOA`, `TSOT`,
  `TSO2`, `TSOC` and `TCMP`, and keeps each value.
- An MP3 write removes each other frame with an ID that is not 4 bytes long.
  `TagWriteResult::removed_frames` names each removed frame.
- An MP3 write encodes the tag into memory first. The id3 crate then writes the tag into a staged
  copy in the same directory, and a rename replaces the file. A failed encode, copy or write
  leaves the file unchanged and removes the staged copy. The copy keeps the file mode.
- The writer resolves a symbolic link first, and replaces the target file, not the link.
- A download that cannot write tags saves the track. Its result warning holds "App saved the
  track but could not write its tags: ..." with the error text. The track is no download failure.
- Each other caller gets the safe write through `write_id3v24_edits`.

### Deviations From The Task

- `write_id3v24_edits` returns `TagWriteResult` and not a count. Two callers read
  `TagWriteResult::applied`. This change gives the removed frame names to the caller.
- R80-4-04 uses a library track whose file is present. That run uses the same materialization
  step and the same non-fatal tag write as a new download. No test sends a request to a remote host.
- The download warning goes through `redact_endpoint_details`, as the tag comparison warning does.
- The retry path and the conversion path keep their earlier rule. A tag-write failure there is
  a file failure, as before.

### Unresolved Concerns

- A rename gives the file a new inode. A hard link to the earlier file keeps the earlier bytes.
  No caller compares the inode of a tagged file.
- A write now needs write access to the directory of the file, and free space for one copy of the
  file.
- No production caller shows `TagWriteResult::removed_frames` to the operator yet.
- Cleanup of the evidence fixture `/tmp/v4vmm-governance.ie6k8TQf` stays unconfirmed. This is
  unrelated to this packet.
