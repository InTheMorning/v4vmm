# Post-ADR-0048 Task — extract `view_models/text_filter` helper

## Goal

Eliminate the duplication of `normalize_text_filter` and per-VM matcher
functions added by commit `64e24cb` (text filtering across view models).

Today:

- `normalize_text_filter` is defined as a file-local free fn in **both**
  `src/view_models/queue_now_playing.rs:431` and
  `src/view_models/library.rs:2147`.
- Per-VM matchers (`track_matches_text_filter` in `feed.rs:154`,
  `track_row_matches_text_filter` in `library.rs:2153`,
  `queue_row_matches_text_filter` in `queue_now_playing.rs:437`,
  `matches_text_filter` in `library.rs:692`) each search a different subset
  of fields with no shared baseline.

After this task:

- One `normalize` definition in `src/view_models/text_filter.rs`.
- One `contains_normalized(haystack: &str, needle: &str) -> bool` helper.
- Per-VM matchers call the shared helpers; field selection stays per-VM but
  uses one normalize path and one contains path.
- An architecture guard prevents future regressions.

## Files To Inspect

- `src/view_models/queue_now_playing.rs`
- `src/view_models/library.rs`
- `src/view_models/feed.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/search_results.rs` (or its post-Task-003 directory)
- `src/view_models/mod.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0047-0048-0049-implementation-review.md` (P2 finding)

## Files Likely To Change

- `src/view_models/text_filter.rs` — new module
- `src/view_models/mod.rs` — register the new module + re-export
- `src/view_models/queue_now_playing.rs` — drop local `normalize_text_filter`
  and `queue_row_matches_text_filter`, call shared helpers
- `src/view_models/library.rs` — drop local `normalize_text_filter` and
  `track_row_matches_text_filter`, call shared helpers
- `src/view_models/feed.rs` — refactor `track_matches_text_filter` to call
  shared helpers
- `src/view_models/playlist_detail.rs` — same treatment if it has a similar
  helper
- `tests/architecture_tests.rs` — add the arch guard

## Do Not Touch

- Public API of any VM.
- Render code.
- Field selection per VM (which fields each VM searches stays per VM; only
  the normalize + contains primitives are shared).

## Constraints

- Behavior-preserving when matched against existing unit tests. The shared
  `normalize` must produce identical output to the two existing duplicates
  for every input.
- The new module has zero dependencies on UI, services, or domain types.
  Pure string helpers only.
- No new `#[allow(...)]`.
- No commit unless explicitly asked.

## Proposed module API

```rust
//! Shared text-filter helpers for view models.
//!
//! View models filter row sets by free-text input from the search input or
//! toolbar. This module owns the normalization and substring-match path so
//! the behavior stays consistent across VMs.

/// Normalize a raw filter input. Returns `None` when the trimmed input is
/// empty so callers can short-circuit "no filter".
pub(crate) fn normalize(input: Option<String>) -> Option<String> {
    input
        .map(|raw| raw.trim().to_lowercase())
        .filter(|s| !s.is_empty())
}

/// Case-insensitive substring contains using normalized lowercase. Both
/// arguments are normalized in place; do not pre-normalize callers' fields.
pub(crate) fn contains_normalized(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}
```

(Treat the API above as the proposal. Verify against the exact bodies of
the two existing `normalize_text_filter` definitions; if they differ in
treatment of whitespace, NFC normalization, or empty-string handling,
adopt the union of behavior and document it in the module doc comment.)

## Architecture guard

Add to `tests/architecture_tests.rs`:

```rust
#[test]
fn normalize_text_filter_lives_only_in_view_models_text_filter() {
    // Find every src/view_models/**/*.rs that defines a file-local
    // `fn normalize_text_filter` or `pub(crate) fn normalize_text_filter`.
    // Assert the only match is src/view_models/text_filter.rs.
}
```

Mirror the style of existing path-walk guards in the test file (use the
same walker helper if one exists, e.g. for "no inline icon SVG" guards).

## Implementation Steps

1. Read the two existing `normalize_text_filter` definitions; confirm they
   produce identical output for the same input.
2. Read each per-VM matcher to identify which fields are searched per VM.
3. Create `src/view_models/text_filter.rs` with `normalize` and
   `contains_normalized`.
4. Register the module in `src/view_models/mod.rs`. Decide visibility:
   `pub(crate) mod text_filter;`.
5. Rewrite `queue_now_playing.rs::queue_row_matches_text_filter` to call
   `contains_normalized`; drop the local `normalize_text_filter` and call
   the shared `text_filter::normalize` instead.
6. Repeat for `library.rs::track_row_matches_text_filter` and
   `library.rs::matches_text_filter`.
7. Repeat for `feed.rs::track_matches_text_filter` if the change reduces
   duplication; if it already calls a `contains` path, just route through
   the shared helpers.
8. Repeat for `playlist_detail.rs` if it has a parallel matcher.
9. Add the arch guard. Confirm it fails before adding the new module and
   passes after.
10. Run the 5 gates.

## Acceptance Criteria

- Exactly one definition of `fn normalize_text_filter` (or `fn normalize`
  in the new module) exists in `src/view_models/`.
- The arch guard fails when a file-local `normalize_text_filter` is
  reintroduced anywhere outside the new module.
- Every previously-existing per-VM unit test for text filtering still
  passes byte-identically.
- All 5 gates pass.
- No new `#[allow(...)]`.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded refactor task. Behavior-preserving
deduplication only.

Read:
- This task file
- `src/view_models/queue_now_playing.rs` (focus: `normalize_text_filter`,
  `queue_row_matches_text_filter`)
- `src/view_models/library.rs` (focus: `normalize_text_filter`,
  `track_row_matches_text_filter`, `matches_text_filter`)
- `src/view_models/feed.rs` (focus: `track_matches_text_filter`)
- `src/view_models/playlist_detail.rs` (skim for similar helpers)
- `src/view_models/mod.rs`
- `tests/architecture_tests.rs`

Goal:
- Create `src/view_models/text_filter.rs` with shared `normalize` and
  `contains_normalized` helpers.
- Rewrite the per-VM matchers to call the shared helpers.
- Add an arch guard preventing reintroduction of file-local
  `normalize_text_filter` outside the new module.

Constraints:
- Identical behavior for normalize + contains relative to existing usage.
- Field selection per VM stays per VM.
- No `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

Test commands:
- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed (with LOC delta)
2. matchers consolidated (list per VM)
3. behavior differences relative to the prior duplicates (should be none)
4. arch guard added (name + location)
5. deviations
6. unresolved concerns

## Escalation Triggers

- The two existing `normalize_text_filter` bodies are not byte-equivalent
  in behavior; one trims newlines and the other doesn't, for example.
  Report the diff; propose which behavior to adopt; do not silently pick.
- A per-VM matcher does more than substring-match (e.g., regex, fuzzy).
  Leave that matcher alone and document why in the report.
