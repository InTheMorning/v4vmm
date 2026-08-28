# ADR 0056 Task 002: Image Classification Owner

## Goal

Give "what image is this?" one owner, in the module that already owns images.

This is the task that makes album art appear before download. It fixes two
distinct causes of missing artwork, only one of which is redirect-related.

Today the byte sniffer is private in `src/audio_tags.rs:1003`
(`image_mime_type_for_bytes`, plus `image_mime_type` at `:988`). A tag-writing
module is the wrong owner for the classification used by the thumbnail cache and
cover-art lookup. The display paths do not use it, and instead require a correct
`Content-Type`:

```
// src/media/image_cache.rs:152
.filter(|v| v.starts_with("image/"))
```

A server returning a valid JPEG as `application/octet-stream` therefore yields no
artwork, with no redirect involved.

Four sites independently guess when the type is unknown, each assuming JPEG:

- `src/media/mod.rs:11` `image_from_bytes`
- `src/media/image_cache.rs:92` `fetch_blocking`
- `src/media/image_cache.rs:128` `fetch_static_blocking`
- `src/subscribe_service.rs:663` `download_image` stores the raw `Content-Type`,
  so `text/html` reaches those fallbacks and is decoded as JPEG

Task 001 must land first: redirect resolution is not this task's job.

## Files To Inspect

- `docs/adr/0056-remote-media-fetch-validation-boundary.md`
- `src/audio_tags.rs`
- `src/media/mod.rs`
- `src/media/image_cache.rs`
- `src/subscribe_service.rs`
- `src/metadata.rs`
- `src/application/queries/images.rs`

## Files Likely To Change

- `src/media/mod.rs`
- `src/media/image_cache.rs`
- `src/audio_tags.rs`
- `src/subscribe_service.rs`
- `docs/reviews/adr-0056-task-002-review.md`

## Do Not Touch

- `src/ui/**`
- `src/view_models/**`
- Schema or migrations
- The transport module from Task 001
- Enclosure size or container validation
- Cache entry file format in `read_cache_entry` / `write_cache_entry`

## Constraints

- Move both helpers to `src/media/`. Do not copy them, and do not leave a
  re-export in `audio_tags` that keeps the tag module looking like the owner.
- Keep the current format coverage: PNG, JPEG, GIF, WebP. Do not add formats to
  make a test pass. If real feed artwork needs more, that is a separate decision.
- One precedence rule everywhere: sniff bytes first, fall back to a declared
  `image/*` type, otherwise reject. This is the rule the APIC path already uses
  after ADR 0056; the display paths adopt it.
- Rejection means no image. Remove all four `unwrap_or(ImageFormat::Jpeg)`
  fallbacks. Guessing JPEG re-hides exactly the failures this task surfaces.
- `download_image` must stop returning a non-image MIME string. `None` is the
  correct result for a non-image response.
- Preserve the `image/gif` branches exactly. Animated GIFs must not collapse to
  static previews.
- Preserve the downscale path's rewrite of MIME to `image/jpeg`, and keep GIFs
  out of downscaling.
- Do not cache failed fetches. Current code returns before `write_cache_entry`;
  keep that ordering.
- Cached entries already require an `image/` prefix on read
  (`image_cache.rs:221`). Sniffed types must stay compatible with that.

## Implementation Steps

1. Move `image_mime_type` and `image_mime_type_for_bytes` into `src/media/`,
   exported for use by `audio_tags`, `image_cache`, and `subscribe_service`.
2. Apply the sniff-then-declared-type rule in `image_cache::read_or_download`,
   replacing the `Content-Type`-only filter.
3. Apply the same rule in `subscribe_service::download_image`, returning `None`
   for non-image responses.
4. Point the APIC path at the moved helpers with no change to its rules.
5. Replace the four JPEG fallbacks with rejection. `image_from_bytes` returns no
   image for an unclassifiable type; adjust its signature and callers if needed.
6. Add a regression test: a valid JPEG served as `application/octet-stream`
   produces a thumbnail.
7. Add a regression test: an HTML body produces no image and no cache entry on
   both display paths.
8. Add a regression test: a GIF still animates and a downscaled image still
   reports `image/jpeg`.
9. Add `docs/reviews/adr-0056-task-002-review.md` noting which surfaces gain
   artwork and any caller signature changes from step 5.

## Acceptance Criteria

- One image classification implementation in `src/`, owned by `src/media/`.
- `audio_tags` no longer defines image MIME helpers.
- A valid image under a non-image `Content-Type` yields a thumbnail.
- An HTML body yields no image and no cache entry.
- No `unwrap_or(ImageFormat::Jpeg)` remains in `src/`.
- `download_image` never returns a non-image MIME type.
- GIF animation, static preview, downscale, and cache format behavior unchanged.
- APIC behavior unchanged.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test image_cache --lib --quiet`
- `cargo test images --lib --quiet`
- `cargo test audio_tags --lib --quiet`
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

- Removing the JPEG fallback changes a public signature used across many UI call
  sites in a way this task cannot contain.
- A real feed serves artwork whose bytes match none of the four formats and whose
  `Content-Type` is absent or wrong.
- `image_cache` has no `mod tests` today; its coverage lives in
  `src/application/queries/images.rs` via `ImageCache::with_capacity`. If adding
  tests requires restructuring that, escalate rather than moving existing tests.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture. Task 001 has landed;
redirect handling is already centralized and is not your concern.

Read:
- `docs/adr/0056-remote-media-fetch-validation-boundary.md`
- `docs/tasks/adr-0056-task-002-image-classification-owner.md`
- `src/audio_tags.rs`
- `src/media/mod.rs`
- `src/media/image_cache.rs`
- `src/subscribe_service.rs`

Goal:
- Move image type classification from `audio_tags` into `src/media/`, apply one
  precedence rule on every path, and remove every silent JPEG fallback.

Constraints:
- Move, do not copy. No re-export left in `audio_tags`.
- Keep coverage at PNG, JPEG, GIF, WebP. Do not add formats.
- Rule everywhere: sniff bytes, then declared `image/*`, else reject.
- Remove all four `unwrap_or(ImageFormat::Jpeg)` fallbacks.
- `download_image` returns `None` for non-image responses.
- Do not change GIF handling, downscale behavior, cache file format, or APIC
  rules.
- Do not cache failed fetches.

Do not touch:
- `src/ui/**`
- `src/view_models/**`
- Schema or migrations
- The transport module

Acceptance criteria:
- One classifier, owned by `src/media/`.
- Valid image under a wrong content type yields a thumbnail.
- HTML body yields no image and no cache entry.
- No `unwrap_or(ImageFormat::Jpeg)` in `src/`.
- GIF, downscale, cache, and APIC tests pass unmodified.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test image_cache --lib --quiet`
- `cargo test images --lib --quiet`
- `cargo test audio_tags --lib --quiet`
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
