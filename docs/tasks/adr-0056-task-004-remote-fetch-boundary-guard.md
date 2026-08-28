# ADR 0056 Task 004: Remote Fetch Boundary Guard

## Goal

Make the ownership established by Tasks 001-003 structurally enforced, so the
next remote media fetch cannot quietly get its own policy.

This is a guard-only packet. It must not change product behavior, schema, or
visual presentation. Run it last: the guards describe the finished layering.

The failure this prevents already happened once. `audio_tags::read_text_reference`
sat in the same module as the APIC fetch, was missed by the original ADR 0056
implementation, and shipped with no redirect handling, no scheme check, and only
`error_for_status` for status. Nothing in the build could have caught that.

## Layering To Guard

- Transport: the Task 001 module owns scheme checks, bounded redirects, and
  status policy. Nothing else performs media HTTP.
- Classification: `src/audio_format.rs` owns audio containers, `src/media/` owns
  image types. Nothing else decides what bytes are.
- Artifact policy: `track_compare`, `audio_tags`, `image_cache`, and
  `subscribe_service` own their own content rules and nothing broader.

## Files To Inspect

- `docs/adr/0056-remote-media-fetch-validation-boundary.md`
- `docs/tasks/adr-0056-task-001-remote-media-transport-module.md`
- `docs/tasks/adr-0056-task-002-image-classification-owner.md`
- `docs/tasks/adr-0056-task-003-artifact-content-policy.md`
- `tests/architecture_tests.rs`
- `src/track_compare.rs`
- `src/audio_tags.rs`
- `src/media/**`
- `src/subscribe_service.rs`
- `src/ui/**`
- `src/view_models/**`

## Files Likely To Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0056-task-004-review.md`

## Do Not Touch

- The transport module, the image classifier, or any artifact policy
- `src/ui/**`
- `src/view_models/**`
- Schema or migrations
- Runtime behavior

## Constraints

- Add or strengthen guards only. If a guard needs a runtime change to pass, stop
  and escalate.
- Follow the existing conventions in `tests/architecture_tests.rs`:
  `rust_files_under`, `read_source`, `code_lines`, `rel_path`, and a collected
  `violations` vector reported through one assert.
- Guard the layering, not the line-by-line implementation. A guard that breaks on
  ordinary refactoring inside a module is too tight.
- Keep the allowed-owner lists explicit and short. Adding an owner should require
  editing the guard; that visibility is the point.
- Do not duplicate module tests. Redirect, size, container, MIME, GIF, and cache
  behavior belong to their modules.
- Feed and API fetches (`src/rss/**`, `src/musicbrainz.rs`, `src/api.rs`,
  `src/discover.rs`) are deliberately outside this ADR. Do not guard them into
  the media transport module.

## Implementation Steps

1. Guard that media fetch primitives (`reqwest::blocking::get`, `.send()` on a
   media fetch) do not appear outside the transport module, excluding the feed
   and API paths listed above.
2. Guard that no media fetch appears in `src/ui/**` or `src/view_models/**`.
3. Guard that image MIME classification exists only in `src/media/`, so the
   sniffer cannot migrate back into `audio_tags` or get re-copied.
4. Guard that no `unwrap_or(ImageFormat::Jpeg)` or equivalent silent
   format-guessing fallback returns to `src/`.
5. Guard that the enclosure path retains container validation, so
   `unwrap_or(declared_format)` cannot come back.
6. Guard that display-only fetch results are not routed into APIC writes or local
   media file writes, keeping their out-of-artifact status true.
7. Record in the review doc which ADR 0056 rules are structurally guarded and
   which remain unit-test-only, so the coverage shape is legible to the next
   reader.
8. Add `docs/reviews/adr-0056-task-004-review.md` with the final result,
   verification commands, and merge recommendation.

## Acceptance Criteria

- Guards fail if a new media fetch bypasses the transport module.
- Guards fail if a media fetch appears in UI or view-model layers.
- Guards fail if image classification appears outside `src/media/`.
- Guards fail if a silent image-format or audio-format fallback returns.
- Guards fail if a display-only fetch result reaches an artifact write.
- Feed and API fetch paths are untouched by the guards.
- Existing module tests remain the source of truth for behavior.
- No runtime or visual behavior changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo test --lib --quiet`
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

- A guard requires runtime code changes to pass.
- Tasks 001-003 have not all landed, so the guarded shape does not exist.
- A legitimate caller trips a guard and cannot be expressed as an explicit
  allowed owner.
- Distinguishing a media fetch from a feed or API fetch by source text proves too
  brittle to guard reliably.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture. Tasks 001-003 have
landed.

Read:
- `docs/adr/0056-remote-media-fetch-validation-boundary.md`
- `docs/tasks/adr-0056-task-004-remote-fetch-boundary-guard.md`
- `tests/architecture_tests.rs`
- `src/track_compare.rs`
- `src/audio_tags.rs`
- `src/media/**`
- `src/subscribe_service.rs`

Goal:
- Add ADR 0056 architecture guards for transport ownership, classification
  ownership, and artifact policy placement.

Constraints:
- Guards only. No runtime, schema, UI, or view-model changes.
- Follow existing `tests/architecture_tests.rs` helpers and violation-list style.
- Guard layering, not implementation details.
- Do not guard `src/rss/**`, `src/musicbrainz.rs`, `src/api.rs`, or
  `src/discover.rs` into the media transport module.
- Do not duplicate module-level behavior tests.

Do not touch:
- The transport module, image classifier, or artifact policy code
- `src/ui/**`
- `src/view_models/**`
- Schema or migrations

Acceptance criteria:
- Guards cover transport bypass, UI/view-model fetches, classifier placement,
  silent format fallbacks, enclosure container validation, and display-only fetch
  isolation.
- Feed and API paths untouched.
- Task review doc added.
- All required test commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo test --lib --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo build --quiet`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
