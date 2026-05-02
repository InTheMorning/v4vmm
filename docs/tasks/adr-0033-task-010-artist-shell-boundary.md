# ADR 0033 Task 010: Artist Shell Screen Boundary

## Goal

Remove the shared top-level artist UI shell's dependency on the Discover screen
module so the shell can later move under `src/ui/` without importing screen
types.

## Files to Inspect

- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/ui_artist.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/ui_artist.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend services, API models, database schema, and search result fetching.
- Feed tile behavior beyond moving ownership of the rendered section.
- Other top-level shell modules.

## Constraints

- The artist shell must not import `crate::search`, `SearchApp`, or other
  screen-owned types.
- Discover remains responsible for feed tile actions, thumbnail lookup, and
  inspector navigation.
- The artist shell remains layout-only and accepts caller-provided rendered
  slots for surface-specific sections.

## Implementation Steps

1. Change `render_artist_view` to accept an optional feed-section element
   instead of `SearchApp`, `Context<SearchApp>`, and a Discover helper import.
2. Build the feed-section slot in `search.rs` with the existing
   `render_feed_list_section` helper.
3. Add an architecture guard that prevents known shared top-level UI shells
   from importing screen modules.

## Acceptance Criteria

- `src/ui_artist.rs` no longer imports `crate::search` or `SearchApp`.
- Discover artist inspector behavior is still wired by `search.rs`.
- The new architecture guard covers `KNOWN_SHARED_UI_SHELL_FILES`.
- Verification is green.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/post-adr-0033-ui-consolidation-plan.md`
- `src/ui_artist.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

Goal:
- Decouple the shared artist UI shell from Discover screen types by passing
  the feed-section rendering as a slot.

Constraints:
- Do not move files in this task.
- Do not change feed tile behavior.
- Keep screen-specific actions in `search.rs`.

Do not touch:
- Backend services.
- Database schema.
- Other top-level UI shell files.

Acceptance criteria:
- `ui_artist.rs` has no screen-module imports.
- `search.rs` still provides the feed-list section for artist inspectors.
- Architecture tests enforce the shared top-level shell boundary.

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

## Escalation Triggers

- The shell still needs to know about screen event types after slot extraction.
- The feed tile slot cannot preserve existing navigation behavior.
- The architecture guard catches existing shell dependencies outside this task.
