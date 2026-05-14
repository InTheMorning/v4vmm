# ADR 0047 Task 016: Retire src/search.rs Screen Module

Status: Proposed - 2026-05-14.

## Goal

Delete `src/search.rs` as a standalone screen module. All entity
render (artist / feed / track) flows through shared composites
consumed by `ContentList`, `SearchResultsInspector`, and Library
inspector shells. `WORKSPACE_RENDER_ENABLED` toggle from ADR 0046
task 007 retires alongside.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/plans/unify-discover-library-views.md`
- `docs/tasks/adr-0047-task-014-search-results-inspector-shell.md`
- `docs/tasks/adr-0047-task-015-search-submit-and-saved-search-commands.md`
- `src/search.rs`
- `src/main.rs` (module declarations)
- `src/app.rs` (legacy tab routing + `WORKSPACE_RENDER_ENABLED`)
- `src/library/app_impl.rs`
- `src/ui/composites/` (shared entity composites if present)
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/search.rs` — deleted
- `src/main.rs` — remove `mod search;`
- `src/app.rs` — remove legacy tab routing, retire
  `WORKSPACE_RENDER_ENABLED`
- `src/ui/composites/` — lift any remaining helpers used only by
  search.rs into shared composites
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend HTTP / Musicindex services consumed by the deleted screen
  remain in their service modules (no service deletion in this task)
- Playback
- `src/db.rs`

## Constraints

- Every render path previously in `src/search.rs` must have a shared-
  composite replacement before deletion. If a unique render block
  exists only in search.rs, lift it into `src/ui/composites/` as a
  shared composite first within the same task.
- `WORKSPACE_RENDER_ENABLED` toggle and `render_legacy_tab_content`
  are removed. Workspace render is the only path.
- Architecture guards:
  - `src/search.rs` absent.
  - No `crate::search::*` imports anywhere.
  - No `WORKSPACE_RENDER_ENABLED` references.
  - No `render_legacy_tab_content` references.

## Implementation Steps

1. Audit `src/search.rs` for render blocks not yet replicated in
   shared composites. Lift them into `src/ui/composites/` first.
2. Remove `WORKSPACE_RENDER_ENABLED` toggle and the legacy tab
   render path from `src/app.rs`.
3. Delete `src/search.rs` and its `mod search;` declaration.
4. Delete `SearchApp` and related types if they are not referenced
   elsewhere. If they are referenced by services, escalate.
5. Update tests that imported from `crate::search`.
6. Add architecture guards locking the absences.

## Acceptance Criteria

- [ ] `src/search.rs` is deleted.
- [ ] All entity render paths route through shared composites.
- [ ] `WORKSPACE_RENDER_ENABLED` and the legacy render path are
  removed.
- [ ] Architecture guards forbid re-introducing the search screen
  or the toggle.
- [ ] App builds and runs.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/unify-discover-library-views.md`
- `docs/tasks/adr-0047-task-014-search-results-inspector-shell.md`
- `docs/tasks/adr-0047-task-015-search-submit-and-saved-search-commands.md`
- `src/search.rs`
- `src/main.rs`
- `src/app.rs`
- `src/library/app_impl.rs`
- `src/ui/composites/`
- `tests/architecture_tests.rs`

Goal:
- Delete `src/search.rs`; retire `WORKSPACE_RENDER_ENABLED`. Lift
  any remaining unique render blocks into shared composites first.

Constraints:
- No render path may be lost; every block has a shared-composite
  replacement before deletion.
- Architecture guards forbid re-introduction.

Do not touch:
- Backend services consumed by the deleted screen
- Playback
- `src/db.rs`

Acceptance criteria:
- search.rs gone; toggle gone; tests green; guards lock absence.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Unique render blocks in `src/search.rs` exist that no shared
  composite covers (escalate; this task should not implement large
  new composites — that work belongs to the entity-render
  unification plan).
- `SearchApp` or related types are referenced by services or
  command paths (escalate before keeping vestigial types).
