# ADR 0047 Task 005: Saved Search VM

Status: Implemented - 2026-05-14.

## Goal

Introduce a GPUI-free `SavedSearchEntry` display contract and source-
list integration so saved searches surface beneath playlists in
`SourceList`. No DB schema change in this task — back the contract
with an in-memory list seeded for tests; persistence is layered in a
later task (or via config.toml per ADR 0046 task 012).

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `src/view_models/library.rs` (source-list/sidebar VM)
- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/library.rs` (or wherever source-list VM lives)
- `tests/architecture_tests.rs`

## Do Not Touch

- DB schema
- Backend HTTP
- Any `src/ui/*` rendering
- Playback

## Constraints

- GPUI-free; `M-CANONICAL-DOCS` on public types.
- `SavedSearchEntry { id: i64, query: String, label: String,
  a11y_label: String }`.
- Source-list VM exposes `saved_searches: Vec<SavedSearchEntry>`.
- Add `SavedSearchesSectionDisplay` if section chrome (heading,
  disclosure) is needed in display contract form.
- No persistence layer in this task; in-memory seeded list +
  setter for testing only.

## Implementation Steps

1. Define `SavedSearchEntry` (and optional
   `SavedSearchesSectionDisplay`) in the source-list VM module.
2. Add `saved_searches` field with default empty `Vec`.
3. Add `set_saved_searches(Vec<SavedSearchEntry>)` setter for tests
   and future loader.
4. Unit tests: empty list yields no section render data; seeded
   list passes through with stable ordering.
5. Architecture guard: assert `SavedSearchEntry` lives in the
   source-list VM module and is GPUI-free.

## Acceptance Criteria

- [ ] `SavedSearchEntry` exists and is documented.
- [ ] Source-list VM exposes `saved_searches` field.
- [ ] Unit tests cover empty + seeded cases.
- [ ] No UI module changed.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test saved_search
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `src/view_models/library.rs`
- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

Goal:
- Add `SavedSearchEntry` display contract and expose
  `saved_searches` on the source-list VM. No persistence yet.

Constraints:
- GPUI-free; documented.
- In-memory only; seeded via a setter for tests.

Do not touch:
- DB schema
- Backend HTTP
- `src/ui/*`
- Playback

Acceptance criteria:
- Type and field exist; unit tests cover passthrough.
- Architecture guard records placement.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test saved_search`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Persisting saved searches without DB schema change is impossible
  (escalate; persistence is a follow-up task).
