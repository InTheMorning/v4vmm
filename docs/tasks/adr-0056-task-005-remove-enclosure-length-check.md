# ADR 0056 Task 005: Remove The Enclosure Length Check

Status: Implemented - 2026-10-03. Mechanical checks Green. This packet has no visual gate.
This packet changes no presentation. A download that failed before can now complete.

## Goal

A download does not compare the staged file length with the declared enclosure byte count. Container detection stays the content check.

## Authority

- [ADR 0056](../adr/0056-remote-media-fetch-validation-boundary.md), amended 2026-10-03 by the operator: a declared enclosure byte count is not a validation fact.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: MusicIndex is a cache of RSS.

## Recorded Facts - 2026-10-03

- The operator downloaded the HeyCitizen feed on 2026-10-03. Two MP3 files failed with "downloaded enclosure size mismatch".
  "MoeFactz" gave 9114038 bytes against a declared 8745418. "Milves in Teslas" gave 9581964 bytes against a declared 10096714.
- `validate_downloaded_size` in `src/track_compare.rs` rejects each length that differs from a positive declared count.
  `download_track` calls it after the download. `RetainedArtifact::capture` in `src/track_compare/retained.rs` calls it again.
- The declared count comes from `Track::enclosure_bytes` or `source_enclosures`, which are MusicIndex fields. The RSS upsert stores no enclosure length.
- `remote_media::fetch` checks redirects and the success status. No code compares the response `Content-Length`.
- The test that asserts "size mismatch should reject partial downloads" serves a 7-byte body with `Content-Length: 7` and a declared count of 8.

## Required Changes

1. Delete `validate_downloaded_size` and both calls.
2. Delete each parameter, field and value that only fed the length check, when no other live code reads it. Keep `Track::enclosure_bytes` and the decoded MusicIndex fields: the contract guard of ADR 0075 section 6 owns them.
3. Replace the size-mismatch test with the tests in "Mechanical Acceptance Criteria".
4. Keep container detection, the redirect policy and the cleanup of the staging directory on a failure.

## Mechanical Acceptance Criteria

Use the prefix `adr_0056_no_length_check_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R56-5-01 | A served MP3 body whose length differs from the declared count, larger and smaller, downloads and promotes |
| R56-5-02 | Container detection rejects a served HTML body whose declared count equals its length. The staging directory is empty after the failure |
| R56-5-03 | A served body shorter than its `Content-Length` header fails the download, and the staging directory is empty after the failure |
| R56-5-04 | No code in `src/` names `validate_downloaded_size` or the text "size mismatch" |

## Visual Acceptance Criteria

None. The operator can download the HeyCitizen feed again after this packet. The two files then complete.

## Exclusions

- No change to the retained action report, its classification or its dismissal. A later ADR 0066 packet owns them.
- No new check against the RSS enclosure length.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/track_compare.rs`: `download_track`, `download_enclosure`, `validate_downloaded_size`, `select_audio_enclosure`, and the download tests.
- `src/track_compare/retained.rs`: `RetainedArtifact::capture`.
- `src/remote_media.rs`: `fetch`.
- `tests/architecture_tests.rs`: the ADR 0056 guards.

## Checks

```bash
cargo test --lib adr_0056_no_length_check_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0056-task-005-remove-enclosure-length-check.md`
- ADR 0056, with its amendment of 2026-10-03
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": remove the enclosure length check, and keep container detection.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use a local `TcpListener`, as the present download tests do. No test sends a request to a remote host.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The retained action report and `src/application/conversion_recovery.rs`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R56-5-01 to R56-5-04 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- R56-5-03 fails because the transport accepts a short body. Report the transport behavior. Do not add a length check.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-03

### Files Changed

- `src/track_compare.rs`: removed `validate_downloaded_size` and its two calls. Removed the `bytes` field from `SelectedEnclosure`
  and from its two builder functions. Added four tests with the `adr_0056_no_length_check_` prefix, for R56-5-01 through R56-5-04.
  The old container-rejection test is renamed and changed into the R56-5-02 test. The old size-mismatch test is deleted.
- `src/track_compare/retained.rs`: removed the `bytes` parameter and field from `RetainedArtifact::capture` and from the struct.
  Updated `validate` and `stage_existing` to call `capture` with two arguments. Renamed and updated the identity-and-containment test.
- `src/subscribe_service/materialization.rs`: updated two calls to `RetainedArtifact::capture` to its new two-argument form.
- This packet: the status line and this result.

### Tests Run

- `cargo test --lib adr_0056_no_length_check_`: 4 passed.
- `cargo test --lib`: 1793 passed.
- `cargo test --test architecture_tests`: 289 passed.
- `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets` and `cargo build --bin v4vmm`: Green, with no warning.
- R56-5-01 fails against the length check this packet removes. A probe test with the old check in place gave this error: "downloaded
  enclosure size mismatch for .../04 - Song- Title-.mp3: expected 8 bytes, got 35 bytes." The probe test was temporary. It is not in
  the final code.

### Behavior Changed

- A download no longer compares the staged file length with a declared enclosure byte count. A download that failed only on this
  mismatch now completes.
- Container detection still rejects a body that is not a supported audio format. This check does not read any declared byte count.
- A body that ends before its declared `Content-Length` still fails the download. The transport layer rejects it. The app adds no
  separate check for this case.

### Deviations From Task

- The existing container-rejection test is renamed and changed into the R56-5-02 test. No separate, near-identical test stays beside
  it. The declared byte count now equals the served length. This shows that container detection, not a byte-count comparison,
  causes the rejection.
- `src/subscribe_service/materialization.rs` is not in "Files To Inspect". Two of its calls to `RetainedArtifact::capture` needed the
  new two-argument form after the signature change in `src/track_compare/retained.rs`.

### Unresolved Concerns

- None.
