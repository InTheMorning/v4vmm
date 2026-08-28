# ADR 0056 Task 003: Artifact Content Policy

## Goal

With transport centralized (Task 001) and image classification centralized
(Task 002), close the remaining content gaps on the paths that write local
artifacts. This is the layer that is allowed to differ per artifact, because the
artifacts genuinely differ.

Two gaps remain.

**Enclosure container fallback.** `download_track` validates size only when
`enclosure.bytes` is present and positive, then calls:

```
let detected_format = AudioFormat::detect_from_file(&staged).unwrap_or(declared_format);
```

That `unwrap_or` swallows the `unknown audio format` error and relabels
unrecognized bytes as the RSS-declared format. A feed that declares no byte count
can still promote a small non-audio body as `track.mp3`, with no warning, because
detected then equals declared and the mismatch branch never fires.

**Transcript content.** After Task 001 the transcript path resolves redirects and
rejects non-success responses, but a server that returns HTML with a 200 still
produces a transcript. The existing `transcript is empty` check does not catch it,
because markup is non-empty text.

## Policy Table

The end state this task locks in. Transport rules are identical everywhere and
live in the Task 001 module; only this column varies.

| Artifact | Owner | Content policy |
| --- | --- | --- |
| Enclosure track file | `track_compare` | supported audio container required; declared byte count must match when positive |
| APIC artwork | `audio_tags` | image type required, byte-derived first |
| Transcript text | `audio_tags` | non-empty after parse; markup responses rejected |
| Thumbnail / cover art | `media`, `subscribe_service` | image type required, byte-derived first; no artifact written |

## Files To Inspect

- `docs/adr/0056-remote-media-fetch-validation-boundary.md`
- `src/track_compare.rs`
- `src/audio_format.rs`
- `src/audio_tags.rs`

## Files Likely To Change

- `src/track_compare.rs`
- `src/audio_tags.rs`
- `docs/reviews/adr-0056-task-003-review.md`

## Do Not Touch

- The transport module from Task 001
- The image classifier from Task 002
- `src/audio_format.rs` detection rules
- `src/ui/**`
- `src/view_models/**`
- Schema or migrations
- Staging directory layout or promotion path naming

## Constraints

- `AudioFormat::detect_from_bytes` already covers every container in
  `AudioFormat`: FLAC, WAV, Ogg Vorbis, Ogg Opus, MP4, and MP3 via both `ID3` and
  frame sync. Do not add detection cases to make this task pass.
- Container detection failure is a hard error, not a warning. Unknown bytes are
  not playable by this application under any declared format.
- Keep the existing declared-vs-detected mismatch warning for the case where
  detection succeeds and disagrees with the RSS declaration. That warning is
  useful precisely because it now only fires on real audio.
- Keep `validate_downloaded_size`. Both checks stay; neither replaces the other.
- Preserve staging cleanup through `cleanup_on_err`. A rejected download leaves
  nothing under the staging root.
- Error messages must distinguish container rejection from size mismatch.
- Transcript rejection should key on the declared response type rather than
  sniffing text heuristically. `text/html` is the case that occurs in practice;
  do not build a general markup detector.

## Implementation Steps

1. Replace the `unwrap_or(declared_format)` fallback with a failing path routed
   through `cleanup_on_err`.
2. Keep the ordering explicit: download, size validation, container validation,
   then rename and promotion.
3. Add a transcript content rule rejecting markup responses.
4. Add a regression test: a small non-audio body with `enclosure_bytes: None` is
   rejected and the staging root is left empty.
5. Add a regression test: a valid enclosure whose detected container differs from
   the declared format still succeeds and still reports the mismatch warning.
6. Add a regression test: an HTML body served with 200 does not become transcript
   text.
7. Add `docs/reviews/adr-0056-task-003-review.md` with the result and
   verification commands.

## Acceptance Criteria

- A non-audio body is rejected before promotion whether or not the source
  declares a byte count.
- A non-audio body is rejected even when a declared byte count happens to match.
- Staging is cleaned on rejection.
- Declared-vs-detected mismatch warnings for real audio are unchanged.
- An HTML 200 response does not become a transcript.
- Task 001 and 002 tests pass unmodified.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test track_compare --lib --quiet`
- `cargo test audio_tags --lib --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo build --quiet`
- `git diff --check`

## Expected Final Report Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A real feed enclosure uses a container `AudioFormat::detect_from_bytes` does
  not recognize, making rejection too strict.
- Container validation cannot run before promotion without restructuring the
  staging flow.
- An existing test depends on the declared-format fallback.
- A real feed serves legitimate transcripts under a markup content type.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture. Tasks 001 and 002
have landed; transport and image classification are centralized and are not your
concern.

Read:
- `docs/adr/0056-remote-media-fetch-validation-boundary.md`
- `docs/tasks/adr-0056-task-003-artifact-content-policy.md`
- `src/track_compare.rs`
- `src/audio_format.rs`
- `src/audio_tags.rs`

Goal:
- Reject staged enclosure bytes that are not a supported audio container, and
  reject markup responses on the transcript path.

Constraints:
- Detection failure is a hard error routed through the existing `cleanup_on_err`.
- Do not add cases to `AudioFormat::detect_from_bytes`.
- Keep `validate_downloaded_size` and the declared-vs-detected mismatch warning.
- Error text must distinguish container rejection from size mismatch.
- Transcript rejection keys on the declared response type; do not build a markup
  detector.

Do not touch:
- The transport module or the image classifier
- `src/audio_format.rs` detection rules
- `src/ui/**`
- `src/view_models/**`
- Schema or migrations

Acceptance criteria:
- Non-audio body rejected with and without a declared byte count.
- Staging root empty after rejection.
- Mismatch warning still fires for real audio.
- HTML 200 does not become a transcript.
- Task 001 and 002 tests pass unmodified.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test track_compare --lib --quiet`
- `cargo test audio_tags --lib --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo build --quiet`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
