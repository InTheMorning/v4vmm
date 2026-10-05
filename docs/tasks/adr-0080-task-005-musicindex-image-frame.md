# ADR 0080 Task 005: MusicIndex Image Frame

Status: Ready - 2026-10-04. Implementation has not started. The operator check opens when the packet is complete.

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

## Operator Check

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
