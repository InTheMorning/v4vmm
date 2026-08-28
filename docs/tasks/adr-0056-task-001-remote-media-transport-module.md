# ADR 0056 Task 001: Remote Media Transport Module

## Goal

Give remote media fetching one owner. Today the transport policy is copied per
call site, and the copies have already diverged.

Five media fetch sites exist:

| Site | Redirects | Scheme check | Status check |
| --- | --- | --- | --- |
| `track_compare.rs:310` enclosure | bounded loop | yes | yes |
| `audio_tags.rs:942` APIC image | bounded loop | yes | yes |
| `audio_tags.rs:726` transcript | none | none | `error_for_status` only |
| `media/image_cache.rs:145` thumbnail | none | none | `error_for_status` only |
| `subscribe_service.rs:664` cover art | none | none | `error_for_status` only |

The transcript path is the proof that per-site policy does not hold. It sits in
the same module as the APIC path, was untouched by the ADR 0056 implementation,
and embeds whatever it receives into the file's tags. On the redirect that
motivated this ADR it writes the HTML redirect body in as transcript text.

`error_for_status` alone is not a status check here: a 3xx is not an error
status, so every row in the bottom three passes a redirect body straight through
to its consumer.

This task moves transport policy into one module and migrates all five sites. It
changes no content validation rules.

## Scope

In scope: scheme allowlist, bounded redirect resolution, `Location` repair,
non-success rejection, and client redirect configuration.

Out of scope: MIME decisions, container detection, size validation, emptiness
rules, caching, downscaling. Those stay with their artifact owners and are
covered by Tasks 002 and 003.

Also out of scope: `rss/enrich.rs`, `rss/subscribe.rs`, `musicbrainz.rs`,
`api.rs`, `discover.rs`. Feed and API fetches keep their own clients. This module
is for media bytes.

## Files To Inspect

- `docs/adr/0056-remote-media-fetch-validation-boundary.md`
- `docs/adr/0015-non-ui-service-boundaries.md`
- `src/track_compare.rs`
- `src/audio_tags.rs`
- `src/media/image_cache.rs`
- `src/subscribe_service.rs`
- `src/discover.rs`
- `src/api.rs`

## Files Likely To Change

- a new remote media transport module
- `src/track_compare.rs`
- `src/audio_tags.rs`
- `src/media/image_cache.rs`
- `src/subscribe_service.rs`
- `docs/reviews/adr-0056-task-001-review.md`

## Do Not Touch

- `src/ui/**`
- `src/view_models/**`
- Schema or migrations
- `src/rss/**`, `src/musicbrainz.rs`, `src/api.rs` fetch paths
- Enclosure size validation, APIC MIME derivation, cache file format, downscale
  and GIF behavior

## Constraints

- Module placement must satisfy ADR 0015. Do not place it under `src/ui/**` or
  `src/view_models/**`.
- The module exposes transport only. If a signature needs an artifact-specific
  parameter to work, the split is wrong; escalate rather than widening it.
- Redirect depth is one named constant in this module. The
  `MAX_APIC_IMAGE_REDIRECTS` constant and the bare `10` literal in
  `track_compare.rs:319` both go away.
- Callers must not construct their own redirect handling afterwards. Remove both
  existing loops; do not leave one behind as a fallback.
- Boundary tests must use the client configuration production passes in. The
  current enclosure test builds a `Policy::none()` client that no production
  caller uses; that is the defect, not the fix.
- Keep the explicit redirect loop even though clients follow redirects by
  default. A default client stops following and returns the 3xx when `Location`
  will not parse, and 3xx passes `error_for_status`.
- Determine the raw-space `Location` repair empirically. Write a test whose
  `Location` the URL parser rejects outright, then remove the repair and confirm
  the test fails. Keep it only if it does. If it is removed, delete the matching
  ADR paragraphs in the same change.
- Streaming must stay streaming. `download_enclosure` copies the response body to
  a file without buffering it in memory; the module must not force callers to
  materialize bytes.

## Behavior Changes Expected

These three are intended and should be stated in the review doc:

- Transcript, thumbnail, and cover-art fetches begin resolving redirects.
- Those three begin rejecting non-HTTP(S) schemes.
- Those three begin rejecting 3xx and other non-success responses instead of
  passing the body along.

No other behavior may change in this task.

## Implementation Steps

1. Create the transport module with the scheme allowlist, bounded redirect
   resolution, and status policy. One redirect depth constant.
2. Decide and implement the client redirect configuration. If the boundary needs
   `Policy::none()` to be deterministic, the module owns that construction rather
   than asking callers to remember it. Record the choice in the review doc.
3. Migrate `track_compare::download_enclosure`, preserving streaming to file.
4. Migrate both `audio_tags` remote references: `read_picture_reference` and
   `read_text_reference`. The tag module keeps its APIC and transcript content
   rules and calls the module for bytes.
5. Migrate `media::image_cache::read_or_download` and
   `subscribe_service::download_image`.
6. Delete both duplicated redirect loops and the now-unused constant.
7. Add the `Location` repair test and resolve the repair's fate per the
   constraint above.
8. Add a regression test that the transcript path no longer returns a redirect
   body.
9. Add `docs/reviews/adr-0056-task-001-review.md` with the client policy
   decision, the empirical result for the space repair, and verification
   commands.

## Acceptance Criteria

- Exactly one redirect implementation and one redirect depth constant in `src/`.
- All five media fetch sites route through the module.
- No `reqwest` redirect handling remains in `track_compare`, `audio_tags`,
  `image_cache`, or `subscribe_service`.
- Redirect tests run against the production client configuration, and deleting
  the module's redirect loop makes them fail.
- The transcript path rejects a redirect body.
- Enclosure download still streams to disk.
- Existing enclosure size, APIC MIME, GIF, downscale, and cache tests pass
  unmodified.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test track_compare --lib --quiet`
- `cargo test audio_tags --lib --quiet`
- `cargo test image_cache --lib --quiet`
- `cargo test images --lib --quiet`
- `cargo test subscribe_service --lib --quiet`
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

- ADR 0015 has no acceptable home for the module.
- A caller cannot use the module without pushing artifact-specific validation
  into it.
- Streaming to disk cannot be preserved through the module's signature.
- A real feed relies on a non-HTTP(S) media scheme.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0056-remote-media-fetch-validation-boundary.md`
- `docs/adr/0015-non-ui-service-boundaries.md`
- `docs/tasks/adr-0056-task-001-remote-media-transport-module.md`
- `src/track_compare.rs`
- `src/audio_tags.rs`
- `src/media/image_cache.rs`
- `src/subscribe_service.rs`

Goal:
- Create one remote media transport module owning scheme checks, bounded
  redirects, and status policy, and migrate all five media fetch sites to it.

Constraints:
- Transport only. No MIME, container, size, emptiness, cache, or downscale logic
  in the module.
- One redirect depth constant. Delete the duplicated loops and the old constant.
- Tests use the production client configuration.
- Keep the explicit redirect loop; default clients return the 3xx when Location
  will not parse, and 3xx passes error_for_status.
- Keep the space repair only if a test fails without it; if removed, update the
  ADR in the same change.
- `download_enclosure` must keep streaming to file, not buffer in memory.
- Do not touch rss, musicbrainz, or api fetch paths.

Do not touch:
- `src/ui/**`
- `src/view_models/**`
- Schema or migrations

Acceptance criteria:
- One redirect implementation in `src/`, five callers.
- Transcript, thumbnail, and cover-art paths now resolve redirects and reject
  non-success responses.
- Enclosure download still streams.
- Existing content-validation tests pass unmodified.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test track_compare --lib --quiet`
- `cargo test audio_tags --lib --quiet`
- `cargo test image_cache --lib --quiet`
- `cargo test images --lib --quiet`
- `cargo test subscribe_service --lib --quiet`
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
