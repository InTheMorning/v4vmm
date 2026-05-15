# ADR 0047 Task 011: Retire GlobalSearchScope

Status: Implemented - 2026-05-15. Automated gates green and
user visual proof confirmed toolbar scope controls are gone.

## Goal

Remove the toolbar segmented control for `GlobalSearchScope` and
delete the type now that filter chips live inside each frame.
Toolbar keeps the search input and submit button (ADR 0043).

Task 010 user visual confirmation exposed duplicate toolbar scope
controls after the Library-backed content frame gained its own chip
strip. This task implements the planned Phase D cleanup; it does not
redesign search submit routing or per-frame filter ownership.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-010-wire-filter-chips-into-content-list-frame.md`
- `src/app/tab_bar.rs`
- `src/app.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs` (if present)
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/app/tab_bar.rs`
- `src/app.rs`
- `src/view_models/library.rs` or `view_models/search.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend / Musicindex
- Playback
- Toolbar search input + submit button

## Constraints

- Delete the segmented-control render and the `GlobalSearchScope`
  type. Remove all `set_global_search_scope` / `global_search_scope`
  references.
- Architecture guards forbid a toolbar filter control.
- Search submit continues to fire; the scope is no longer attached.
  Phase E adds the new submit semantics; this task only removes the
  scope control + type.
- Treat Task 010 visual confirmation as evidence of the duplicate
  toolbar-control problem, not as fresh proof that this task's code has
  been implemented or visually verified.

## Implementation Steps

1. Remove the `SegmentedControl` render block from
   `src/app/tab_bar.rs` (the scope-chip render).
2. Remove `GlobalSearchScope` field on `TopApp` and any setter.
3. Remove `GlobalSearchScope` enum definition.
4. Remove all `global_search_scope` references.
5. Update or remove tests that asserted the scope-chip contract.
6. Architecture guards:
   - `src/app/tab_bar.rs` does not contain `SegmentedControl::new(...
     GlobalSearchScope`.
   - No `GlobalSearchScope` symbol anywhere in `src/`.

## Acceptance Criteria

- [x] Toolbar no longer renders the scope segmented control.
- [x] `GlobalSearchScope` type and references are deleted.
- [x] Toolbar search input + submit still render.
- [x] Architecture guards forbid re-introducing the scope control.
- [x] Fresh visual proof confirms the toolbar no longer duplicates the
      per-frame filter chips.

## Implementation Notes

- `GlobalSearchScope`, `global_search_scope`, and
  `set_global_search_scope` were removed from `src/`.
- Toolbar global search now submits without a toolbar-owned source
  scope; Search's legacy result-type filters remain screen-local until
  Phase E replaces the Search screen route.
- `SearchViewModel` uses the shared `ContentFilter` for its temporary
  source-filter state so no second filter enum survives.
- User visual proof on 2026-05-15 confirmed the toolbar keeps the
  search field and submit button, no longer renders `All / Library /
  Index` beside it, and still submits into Search results.

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
- `docs/tasks/adr-0047-task-010-wire-filter-chips-into-content-list-frame.md`
- `src/app/tab_bar.rs`
- `src/app.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `tests/architecture_tests.rs`

Goal:
- Remove the toolbar `GlobalSearchScope` segmented control and the
  type. Keep search input and submit.

Constraints:
- No new feature behavior.
- Architecture guards forbid the toolbar scope control.

Do not touch:
- Backend / Musicindex
- Playback
- Toolbar search input + submit

Acceptance criteria:
- Scope control gone; type gone; tests green.
- Guards lock the absence.

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

- Existing call sites of `GlobalSearchScope` outside the toolbar
  prevent clean deletion (escalate before keeping a vestigial type).
