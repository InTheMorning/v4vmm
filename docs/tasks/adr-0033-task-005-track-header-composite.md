# ADR 0033 Task 005: Track Header Composite

## Goal

Consolidate the Library and Discover track inspector headers behind one shared UI composite and one shared display projection so header behavior cannot drift by screen.

## Files to inspect

- `docs/adr/0033-ui-design-system-and-backend-boundaries.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/track.rs`
- `src/view_models/search.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`

## Files likely to change

- `src/view_models/track.rs`
- `src/view_models/search.rs`
- `src/ui/composites/track_header.rs`
- `src/ui/composites/mod.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0033-task-005-review.md`

## Do not touch

- Backend schema, migrations, command handlers, importer logic, or metadata write flows.
- Playlist popover behavior; this packet only removes the track-header duplication baseline.
- Existing screen-specific action rows other than passing Search's supplementary track controls into the composite.

## Constraints

- Preserve the Library title override contract: a non-empty inspector title wins; an empty inspector title falls back to the track title fallback chain.
- Preserve the shared artist fallback chain: `track_artist -> release_artist -> "Unknown"`.
- Keep image resolution and command callbacks in screen code.
- Keep composite code token-driven and free of backend/service imports.
- Do not introduce another screen-local `render_track_header` helper.

## Implementation steps

1. Add shared title/artist projection methods to `TrackVm`.
2. Add `TrackHeaderVm` as the display contract consumed by the UI composite.
3. Add `TrackHeader` under `src/ui/composites/`.
4. Wire Library and Discover track inspectors to use `TrackHeader`.
5. Leave Discover's feed/play/Nostr row screen-owned and pass it as a supplementary row.
6. Remove `render_track_header` from the render-helper duplication baseline.
7. Add focused unit coverage for the new display contract.

## Acceptance criteria

- `src/library.rs` and `src/search.rs` no longer define `render_track_header`.
- Track header title and artist fallbacks live in `src/view_models/track.rs`.
- Header chrome lives in `src/ui/composites/track_header.rs`.
- `tests/architecture_tests.rs` no longer allows duplicated `render_track_header`.
- Formatting, compile, architecture tests, full tests, clippy, and diff whitespace checks are green.

## Test commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test
cargo clippy --lib --tests -- -D warnings
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-ui-design-system-and-backend-boundaries.md`
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/track.rs`
- `src/view_models/search.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`

Goal:
- Replace duplicated Library/Search track inspector header rendering with a shared token-driven `TrackHeader` composite and shared `TrackHeaderVm`.

Constraints:
- Preserve title, artist, image, and Discover supplementary-control behavior exactly.
- Do not move command callbacks or image loading into the composite.
- Do not add a new screen-local `render_track_header` replacement.

Do not touch:
- Backend services, migrations, playlist popover implementation, metadata write logic, or unrelated screen helpers.

Acceptance criteria:
- No duplicated `render_track_header` helper remains.
- `TrackVm` owns title override and artist fallback rules.
- Architecture duplication baseline no longer contains `render_track_header`.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation triggers

- The shared composite needs command execution or service access.
- The title/artist fallback behavior cannot be preserved without changing API data.
- Architecture tests require broad baseline changes outside `render_track_header`.
