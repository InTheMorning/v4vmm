# ADR 0080 Task 005: MusicIndex Image Frame

Status: Open - implementation and mechanical checks are complete on 2026-10-04. The operator check is pending: [pending check 38](../pending-human-checks.md#38-musicindex-image-frame--adr-0080-task-005).

## Goal

Write the artwork URL of each track into the frame `TXXX:MusicIndex Image`.
The broadcast producers then send that URL as the artwork of a live track, so an animated GIF stays animated.

## Authority

- [ADR 0080](../adr/0080-tag-frames-follow-their-owner.md) Decision 9, and Decisions 3, 4 and 6.
- [The image tag request](../plans/musicindex-image-tag-request.md) of `musicindex-live-publisher`.

## Recorded Facts - 2026-10-04

- `metadata::artwork_url` in `src/metadata.rs` gives the track image, else the feed image. The "Artwork" row uses it, and `id3_frame_hint` maps that row to `APIC`, the embedded picture.
- `id3_frame_hint` maps no row to `TXXX:MusicIndex Image`.
- `src/broadcast/producer.rs` reads `TXXX:MusicIndex Image` into the `image` field of the drop file. `mixxx-now-playing` reads the same frame.
- The stored track image comes from the RSS item `itunes:image` (`src/rss/subscribe.rs`, `track_image_href`). The stored feed image comes from the RSS channel.
- MusicIndex on 2026-10-04 gave these images for the three tracks of the request:

  | Track | Item image | Channel image |
  |---|---|---|
  | How Bout You? | `HowBoutYou.gif` | `The-Heycitizen-Experience.jpg` |
  | ZZXX | `ZZXX.gif` | `The-Heycitizen-Experience.jpg` |
  | Disco Swag (The Album) | `discoswag-thealbum.gif` | the same GIF |

- The app writes `TXXX:MusicIndex Contributors` and `TXXX:MusicIndex Value Routes` today. Use their path as the model for the new frame: the field rows, the tag edits, the Vorbis and MP4 keys, the scan and the compare.

## Required Changes

1. Add a field row for the artwork URL. Map it to `TXXX:MusicIndex Image` in `id3_frame_hint`, and give it the Vorbis and MP4 keys that the other `MusicIndex` text frames use.
2. Its value is `artwork_url` of the track and feed. Write it only when it parses as an `http` or `https` URL of 2,048 characters or fewer. Otherwise the row gives no edit.
3. Include the row in each write of the MusicIndex tags: a download, and "Update n files".
4. The compare and the tag scan check the frame against the value of step 2 (Decision 3). A file without the frame shows as a difference (Decision 4).
5. A second write with the same inputs changes no frame (Decision 6). A write replaces an earlier app value of the frame.
6. Keep the "Artwork" row and `APIC` as they are.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R80-51 | The tag edits for a track with its own image hold that URL in `TXXX:MusicIndex Image` |
| R80-52 | The tag edits for a track with no image of its own hold the feed image URL |
| R80-53 | A `data:` URL, a relative URL, an `ftp` URL and an `https` URL of 2,049 characters each give no edit |
| R80-54 | A round-trip test: the compare of freshly built edits against the same track and feed reports no difference for the frame |
| R80-55 | The scan reports a file without the frame as a difference when the track has an artwork URL |
| R80-56 | Two writes of the same edits give one frame value |
| R80-57 | A FLAC file and an MP4 file get the frame under their keys, and the scan reads it back as equal |

## Operator Acceptance Criteria

These are for the operator. They stay open until a person walks them.

- V80-51: after "Update n files", `mid3v2 -l` on "How Bout You?" shows `TXXX=MusicIndex Image=https://files.heycitizen.xyz/Songs/Albums/The-Heycitizen-Experience/HowBoutYou.gif`.
- V80-52: the next scan shows no difference for that file.
- V80-53: the publisher repository repeats its probe and finds the frame in each file with an image URL. The publisher owns this step.

## Exclusions

- No change to `APIC` or to the embedded picture.
- No change to the producer in `src/broadcast/producer.rs`.
- No database change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/metadata.rs`: `artwork_url`, `id3_frame_hint`, the field rows, and the Contributors and Value Routes paths.
- `src/audio_tags.rs`: the writer, the Vorbis and MP4 keys.
- `src/application/queries/tag_update.rs`: the scan.
- `src/broadcast/producer.rs`: the reader of the frame. Read only.

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data. A file that got the frame keeps it, and the producers read it.

## Result - 2026-10-04

- `src/metadata.rs` has a "MusicIndex Image" row beside "Artwork". `id3_frame_hint` maps it to `TXXX:MusicIndex Image`. The row is in the write rows and in the compare rows.
- `valid_musicindex_image_url` accepts only an `http` or `https` URL of 2,048 characters or fewer. It uses the URL parser of `reqwest`, as the website frames do.
- The Vorbis key is `MUSICINDEX IMAGE`. The MP4 key is `----:com.apple.iTunes:MusicIndex Image`. The generic `TXXX` key path gives both, and `src/audio_tags.rs` has no production change.
- The orchestrator changed the compare row to hold the validated value, as Decision 3 requires. The "Artwork" row still shows an invalid URL. `adr_0080_image_compare_row_holds_only_the_value_that_the_writer_writes` guards it.
- The MP4 proof uses the writer and reader key functions and a scan test with synthetic tags. The repository has no MP4 fixture file.
- Proof:
  - R80-51: `adr_0080_image_edit_holds_the_track_artwork_url`.
  - R80-52: `adr_0080_image_edit_falls_back_to_the_feed_artwork_url`.
  - R80-53: `adr_0080_image_edit_rejects_an_unsupported_or_oversized_url`.
  - R80-54: `adr_0080_image_round_trip_edit_gives_no_scan_difference`.
  - R80-55: `adr_0080_image_missing_frame_is_reported_as_a_difference`.
  - R80-56: `adr_0080_image_edit_write_is_idempotent`.
  - R80-57: `adr_0080_image_flac_write_reads_back_as_equal`, `adr_0080_image_mp4_frame_reads_back_as_equal` and `lofty_musicindex_image_round_trips_its_mp4_freeform_key`.
- The project gate is Green.

## Operator Check Procedure

1. Run `cargo build --bin v4vmm`, then `./target/debug/v4vmm`.
2. Open Music. Expected: "Update n files" counts each Library file with an artwork URL, because no file has the frame yet.
3. Click "Update n files". Expected: each file lists `TXXX:MusicIndex Image`. Click Write Tags.
4. At a terminal, find the file: `find ~/V4Vmusic -iname '*How Bout You*'`.
5. Run `mid3v2 -l "<path from step 4>" | grep 'MusicIndex Image'`. Expected: `TXXX=MusicIndex Image=https://files.heycitizen.xyz/Songs/Albums/The-Heycitizen-Experience/HowBoutYou.gif`. Wrong: no line, or another URL.
6. Do steps 4 and 5 for "ZZXX" and "Disco Swag". Expected: `ZZXX.gif` and `discoswag-thealbum.gif`.
7. Open Music again. Expected: "Update n files" does not list these files for `TXXX:MusicIndex Image`.
8. Tell the publisher session that the frame is written. Its probe is V80-53.

The steps write tags into the Library files. That write is the purpose of the packet, so no cleanup applies.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0080-task-005-musicindex-image-frame.md`
- ADR 0080, and the image tag request
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the artwork URL in `TXXX:MusicIndex Image` for each write, scan and compare.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Treat each RSS and MusicIndex value as untrusted input.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git mv` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `APIC`, the embedded picture and the producer.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The `../musicindex`, `../stophammer` and publisher checkouts.

Acceptance criteria:
- Each case R80-51 to R80-57 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. the Vorbis and MP4 keys of the frame
5. deviations from task
6. unresolved concerns
