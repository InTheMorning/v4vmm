# ADR 0050 Task 003 — decompose `src/view_models/search_results.rs`

## Goal

Move `src/view_models/search_results.rs` (1,408 LOC) into a submodule
directory `src/view_models/search_results/` with six behavior-grouped
submodules and a `tests.rs` file. No public API change; callers' `use`
statements stay the same via `mod.rs` re-exports.

## Files To Inspect

- `src/view_models/search_results.rs` (full read)
- `src/view_models/mod.rs`
- `docs/adr/0050-post-adr-0048-module-decomposition.md`
- `docs/plans/adr-0050-module-decomposition-phase-plan.md`
- `tests/architecture_tests.rs`
- Callers: `src/app.rs`, `src/library/app_impl.rs`,
  `src/ui/shells/search_results_inspector.rs`,
  `src/view_models/workspace.rs` (or its post-Task-002 directory)

## Files Likely To Change

- `src/view_models/search_results.rs` — deleted (moved into directory)
- `src/view_models/search_results/mod.rs` — new
- `src/view_models/search_results/tabs.rs` — new
- `src/view_models/search_results/results.rs` — new
- `src/view_models/search_results/paged_tab.rs` — new
- `src/view_models/search_results/index_detail.rs` — new
- `src/view_models/search_results/empty_state.rs` — new
- `src/view_models/search_results/tests.rs` — new
- `tests/architecture_tests.rs` — path retargeting only

## Do Not Touch

- Public-API signatures.
- GPUI render code (lives in shells).
- View-model logic outside this module.

## Constraints

- Behavior-preserving move only.
- Re-export discipline: `use crate::view_models::search_results::*` and
  every named import must continue to work unchanged.
- `SearchResultsInspectorPageVm` stays in `mod.rs` (it cross-cuts every
  submodule).
- Use `git mv` so blame survives.
- No commit unless explicitly asked.

## Submodule ownership map

| Submodule | Items | Approx lines in current file |
|---|---|---|
| `mod.rs` | `SearchResultsInspectorPageVm` + re-exports | 474-1408 |
| `tabs.rs` | `SearchResultsTab`, `SearchResultOrigin`, `SearchResultItemId` | 28-75 |
| `results.rs` | `ArtistResultDisplay`, `FeedResultDisplay`, `TrackResultDisplay`, `LocalArtistResult`, `LocalFeedResult` | 77-240 + 866-960 (verify) |
| `paged_tab.rs` | `SearchResultsPagedTab<Row>` and its `impl` blocks | 271-390 |
| `index_detail.rs` | `IndexSearchResultRows`, `IndexDetailKind`, `IndexDetailDisplay` | 391-473 |
| `empty_state.rs` | `EmptyStateDisplay` | 240-270 |
| `tests.rs` | Existing `#[cfg(test)] mod tests` | last block in current file |

Re-verify line ranges when reading; they are post-bee1ac2 snapshots.

## Implementation Steps

1. Read `src/view_models/search_results.rs` in full. Confirm item ranges.
2. Create the directory `src/view_models/search_results/` and an empty
   `mod.rs`.
3. Move items in dependency order:
   1. `tabs.rs` — leaf types, no deps inside the module.
   2. `empty_state.rs` — depends on nothing inside the module.
   3. `results.rs` — depends on `tabs.rs` for `SearchResultItemId`.
   4. `paged_tab.rs` — depends on `results.rs` and `tabs.rs`.
   5. `index_detail.rs` — depends on `results.rs` and shared identity types.
   6. `mod.rs` — `SearchResultsInspectorPageVm` plus re-exports.
   7. `tests.rs` — move `#[cfg(test)] mod tests` block.
4. After each move: `cargo check`; fix imports inside the new file.
5. Bump visibility only as tests require; document bumps.
6. Update `mod.rs` re-exports so the public surface matches the previous
   single-file surface item-for-item. Prefer explicit re-exports:
   ```rust
   pub(crate) use self::tabs::{SearchResultsTab, SearchResultOrigin, SearchResultItemId};
   pub(crate) use self::results::{ArtistResultDisplay, FeedResultDisplay, TrackResultDisplay};
   pub(crate) use self::paged_tab::SearchResultsPagedTab;
   pub(crate) use self::index_detail::{IndexSearchResultRows, IndexDetailKind, IndexDetailDisplay};
   pub(crate) use self::empty_state::EmptyStateDisplay;
   #[cfg(test)] mod tests;
   ```
7. Delete the original `src/view_models/search_results.rs`.
8. Run the 5 gates.
9. Search `tests/architecture_tests.rs` for guards pinning
   `view_models/search_results.rs`. Retarget paths.

## Acceptance Criteria

- `src/view_models/search_results.rs` no longer exists.
- All six new submodules + `tests.rs` exist and compile.
- No caller outside the new directory had to change `use` statements.
- All 5 gates pass.
- `git log --follow` surfaces pre-split history.
- No new `#[allow(...)]` annotations.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded refactor task from a larger plan.

Implement only this task. Behavior-preserving file move only.

Read:
- `docs/adr/0050-post-adr-0048-module-decomposition.md`
- `docs/plans/adr-0050-module-decomposition-phase-plan.md`
- `src/view_models/search_results.rs` in full
- `tests/architecture_tests.rs`
- Caller files listed in the task

Goal:
- Split `src/view_models/search_results.rs` (1,408 LOC) into a
  `src/view_models/search_results/` directory with `mod.rs`, `tabs.rs`,
  `results.rs`, `paged_tab.rs`, `index_detail.rs`, `empty_state.rs`,
  `tests.rs`.
- Use the submodule ownership map in the task file. Move in dependency
  order: tabs → empty_state → results → paged_tab → index_detail → mod →
  tests. Run `cargo check` between moves.

Constraints:
- No public-API change. Re-export everything `pub(crate)` from `mod.rs`.
- `SearchResultsInspectorPageVm` stays in `mod.rs`.
- Visibility bumps only when a test demands it. Document each bump.
- Use `git mv` so blame survives.
- Never skip hooks. Don't commit unless explicitly asked.

Test commands:
- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files created and deleted (with LOC per file)
2. tests run + pass/fail counts
3. visibility bumps applied
4. caller import changes (should be none)
5. deviations
6. unresolved concerns

## Escalation Triggers

- An item is referenced by both `paged_tab.rs` and `index_detail.rs` and
  cannot be cleanly owned by either. Resolution: move it up to `mod.rs`.
- A test reaches a method on `SearchResultsInspectorPageVm` that was
  previously private. Resolution: keep `SearchResultsInspectorPageVm` impl
  in `mod.rs`; bump only the required method's visibility.
- An arch test guard wording does not map cleanly to the new path. Report
  the guard name; do not relax it.
