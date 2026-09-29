# ADR 0080 Task 001: RSS Frames And Idempotent Writes

Status: Ready - 2026-09-29. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

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

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
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

## Operator Visual Check

The implementer writes this section at completion, with numbered steps for V1 to V4, the needed state, what counts as wrong, and the cleanup.
Each step uses a copy of a music file in a temporary folder, never a Library file. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
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
