# ADR 0030 Task 001: Discovery Backslash Search

## Status

Implemented - 2026-05-01.

## Goal

Prevent backslash characters in Discovery search queries from reaching the
remote API query parser.

## Files To Inspect

- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/plans/discovery-library-ui-fixes.md`
- `src/api.rs`
- `src/search.rs`
- `src/view_models/search.rs`

## Files Likely To Change

- `src/api.rs`
- `docs/tasks/adr-0030-task-001-backslash-search.md`
- `docs/reviews/adr-0030-task-001-review.md`

## Do Not Touch

- `src/db.rs`
- `src/library.rs`
- `src/metadata.rs`
- `src/ui/composites/`
- Persistence, download, playback, playlist, and MusicBrainz code.

## Constraints

- Extend `sanitize_api_query_value`; do not add a Discovery-only sanitizer.
- Keep path and query normalization behavior centralized in `src/api.rs`.
- Add a focused test proving backslashes do not appear as `%5C` in URLs.
- Do not change search result rendering or remote endpoint paths.

## Implementation Steps

1. Update `sanitize_api_query_value` so `\` maps to a space.
2. Add or extend a unit test around `Client::build_url`.
3. Verify query strings containing `\`, `\\`, and embedded backslashes sanitize
   to space-separated terms.
4. Run the required gates for this bounded task.

## Acceptance Criteria

- [x] `john\doe` is encoded without `%5C` and decodes as `john doe`.
- [x] A query of only backslashes sanitizes to an empty query value, not a literal
  backslash.
- [x] Existing control-character sanitization still works.

## Implementation Summary

- Updated the shared API query sanitizer in `src/api.rs` to replace `\` with
  whitespace before whitespace collapsing.
- Added `build_url_sanitizes_backslash_query_values` covering embedded
  backslash, repeated backslashes, and multiple backslash-separated terms.
- Preserved the existing control-character and path/query sanitizer behavior.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test api::tests::build_url_sanitizes_backslash_query_values
cargo test api::tests::build_url_sanitizes_metadata_path_segments_and_query_values
cargo clippy -- -D warnings
```

Verified 2026-05-01.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/tasks/adr-0030-task-001-backslash-search.md`
- `src/api.rs`

Goal:
- Prevent backslash characters in API query values from reaching remote query
  parsers.

Constraints:
- Change `sanitize_api_query_value` only as needed.
- Add focused unit coverage in `src/api.rs`.
- Do not change endpoint construction, search UI, or API response types.

Do not touch:
- `src/db.rs`
- `src/library.rs`
- `src/metadata.rs`
- `src/search.rs` unless sanitizer-only changes prove insufficient.

Acceptance criteria:
- `john\doe` produces a URL with no `%5C` and query pair `("q", "john doe")`.
- Backslash-only queries do not forward a literal backslash.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test api::tests::build_url_sanitizes_backslash_query_values`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
