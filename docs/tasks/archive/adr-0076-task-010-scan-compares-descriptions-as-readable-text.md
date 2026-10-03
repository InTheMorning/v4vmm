# ADR 0076 Task 010: The Scan Compares Descriptions As Readable Text

Status: Complete - 2026-10-03. Mechanical checks Green. The operator passed V1 on the real Library on 2026-10-03.

## Goal

The "Update n file(s)" scan compares each description frame by readable text. HTML formatting and equivalent whitespace alone do not make a file differ.

## Authority

- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md): "Compare descriptions by readable text. HTML formatting and equivalent whitespace alone do not create a discrepancy."
- [ADR 0076](../../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decisions 3 and 8: descriptions compare as readable text, and the scan counts the files that differ from the stored values.

## Recorded Facts - 2026-10-03

- The operator downloaded "Disco Swag - The Album" by The Doerfels with the build of ADR 0076 task 009. The popup "Update 36 files" listed each new file with one line, for example:
  `COMM:MusicIndex Description: <p>1. Disco Swag</p> (file: 1. Disco Swag)`.
- `values_match` in `src/application/queries/tag_update.rs` compares `TIT2` after title sanitizing, `TRCK` and `TPOS` by their leading number, and each other frame as exact text.
- `readable_text` and `description_equal` in `src/rss/compare.rs` give the readable text of a description, plain text or HTML. The playlist RSS check uses them.

## Required Changes

1. `values_match` compares each description frame by readable text: the item description, the album description and each other `COMM` description that the app writes. Use the shared owner in `src/rss/compare.rs`. Do not write a second HTML reader.
2. Each other frame keeps its present comparison.
3. No change to the value that a write puts into the file.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_scan_readable_description_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R76-10-01 | A stored description `<p>1. Disco Swag</p>` and a file value `1. Disco Swag` give no changed frame |
| R76-10-02 | Values that differ only in whitespace or HTML entities give no changed frame |
| R76-10-03 | Values with different readable text give a changed frame, with the stored value as the expected value |
| R76-10-04 | A `TIT2`, `TRCK` and `WOAR` comparison gives the same result as before this packet |

## Visual Acceptance Criteria

None. The operator check reads the button count.

## Exclusions

- No change to the tag writer, and no decision about HTML in file descriptions.
- No change to the playlist RSS check.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../../.github/copilot-instructions.md).
- `src/application/queries/tag_update.rs`: `values_match`, `changed_frames` and the tests.
- `src/rss/compare.rs`: `readable_text`, `description_equal`, `TextRepresentation`.
- `src/metadata.rs`: the description frame labels.

## Checks

```bash
cargo test --lib adr_0076_scan_readable_description_
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

This check reads the "Update n files" count on a Library the app already scanned. It needs no new fixture.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

Needs:

- A Linux desktop session with the real Library and the real configuration.
- A Library that holds the file from the Recorded Facts, "Disco Swag - The Album" by The Doerfels, from the ADR 0076 task 009 download. Any Library with an older description frame also shows the fix.

**Setup**

1. Build the desktop binary.

   ```bash
   cd /home/citizen/build/v4vmm && cargo build --bin v4vmm
   ```

2. Start the app: `target/debug/v4vmm`.

**V1: The scan reports no description-only difference for a readable match**

3. Open **Music → Library**. Wait for the scan to finish.
4. Read the "Update n files" button.
   - Expect a count with no Disco Swag file listed for only a description line, for example `COMM:MusicIndex Description: <p>1. Disco Swag</p> (file: 1. Disco Swag)`.
   - This result is wrong: a Disco Swag file still shows that one line as its only listed change.
5. Click the "Update n files" button to read the popup list.
   - Expect no line of the form `COMM:MusicIndex Description: <HTML text> (file: <the same text with no tags>)`.
   - Click **Cancel**. Do not confirm.
6. Close the app.

**Cleanup**

No fixture to remove. The check reads the real Library and writes no file.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/archive/adr-0076-task-010-scan-compares-descriptions-as-readable-text.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the scan compares description frames by readable text.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The tag writer, `src/audio_tags.rs` and `src/ui/`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R76-10-01 to R76-10-04 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The shared readable-text owner cannot serve the scan without a change outside the files above.

## Implementation Result - 2026-10-03

1. Files changed:
   - `src/application/queries/tag_update.rs`: `values_match` adds a `"COMM"` branch. It calls `description_equal` from `src/rss/compare.rs`, with each side read as HTML, so a comparison treats the stored value and the file value the same way. The branch covers each `COMM` description frame the app writes (`COMM:MusicIndex Description` and `COMM:MusicIndex Album Description`), because `id3_frame_base` gives `"COMM"` for both labels. Four tests with the prefix `adr_0076_scan_readable_description_` are added beside `values_match` and `changed_frames`.
2. Tests run:
   - R76-10-01 failed before the change: `values_match("COMM:MusicIndex Description", "1. Disco Swag", "<p>1. Disco Swag</p>")` gave `false`, because the old code compared the two values as exact text.
   - `cargo test --lib adr_0076_scan_readable_description_` passes all four cases.
   - `cargo test` (1810 passed), `cargo test --test architecture_tests` (289 passed), `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets` and `cargo build --bin v4vmm` are Green.
3. Behavior changed:
   - The stored value and the file value of a description frame can give the same readable text. The "Update n files" scan does not list the file for that frame alone. A write still puts the stored value, HTML and all, into the file (Required Change 3).
   - Each other compared frame (`TIT2`, `TRCK`, `TPOS`, `WOAR`, and so on) keeps its present comparison.
4. Deviations from task: none. The branch matches on `id3_frame_base(frame_label) == "COMM"`, not a named list of the two frame labels. `src/audio_tags.rs` already owns that list (`DESCRIPTION_FRAME_LABELS`), and that file is on the Do-not-touch list. The `"COMM"` base match gives the same two labels today and needs no import from the excluded file.
5. Unresolved concerns: none. The shared readable-text owner in `src/rss/compare.rs` served the scan with no change outside the files named in "Files To Inspect".

Remind the operator: run `cargo build --bin v4vmm`, then restart the app with `target/debug/v4vmm` before the operator visual check below.
