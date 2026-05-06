# ADR 0038 Task 007 — Slice F: Final Guards and Readiness

## Goal

Add the four architecture guards specified in Task 007 spec, verify
entry-module size budgets, and update review documentation. This
slice locks in the decomposition so future regressions are caught
mechanically.

## Preconditions

- All 12 surface slices landed (L1..L6, D1..D6).
- `cargo test` green at HEAD.

## Files to Create

None.

## Files to Modify

1. `tests/architecture_tests.rs` — add four new guards (see below).
2. `docs/reviews/adr-0038-review-checklist.md` — update Task 007 row,
   add per-surface ledger entries, list automated check commands.
3. `docs/tasks/adr-0038-task-007-screen-decomposition.md` — change
   Status to "Completed on YYYY-MM-DD" with a Completed Slices
   section listing each L*/D*/F slice and its commit SHA.

## Architecture Guard Specifications

Add inside `tests/architecture_tests.rs`. Use the existing
`code_lines` and `manifest_path` helpers (consistent with the
existing guards in this file).

### Guard 1: `library_screen_modules_are_decomposed_under_src_ui_shells_library`

Expected files:
- `src/ui/shells/library/mod.rs`
- `src/ui/shells/library/sidebar.rs`
- `src/ui/shells/library/feed_list.rs`
- `src/ui/shells/library/feed_detail.rs`
- `src/ui/shells/library/track_detail.rs`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/ui/shells/library/playlist_detail.rs`

Each must exist and contain at least one `pub(crate) fn` (except
`mod.rs`).

### Guard 2: `discover_screen_modules_are_decomposed_under_src_ui_shells_discover`

Expected files:
- `src/ui/shells/discover/mod.rs`
- `src/ui/shells/discover/search_input.rs`
- `src/ui/shells/discover/result_list.rs`
- `src/ui/shells/discover/recent.rs`
- `src/ui/shells/discover/feed_inspector.rs`
- `src/ui/shells/discover/track_inspector.rs`
- `src/ui/shells/discover/track_inspector_metadata.rs`

### Guard 3: `screen_entry_modules_under_500_loc`

```rust
let ceilings = [("src/library.rs", 500), ("src/search.rs", 500)];
for (path, ceiling) in ceilings {
    let source = read_source(&manifest_path(path));
    let loc = code_lines(&source).count();
    assert!(loc <= ceiling, "{path} exceeds {ceiling} LOC ceiling: {loc}");
}
```

### Guard 4: `surface_modules_under_500_loc`

Walk every `*.rs` file under `src/ui/shells/library/` and
`src/ui/shells/discover/` (excluding `mod.rs`). Apply ceiling 500.
Report all violations in one assertion.

## Review Checklist Updates

In `docs/reviews/adr-0038-review-checklist.md`:

1. Mark the Task 007 row complete.
2. Add per-surface ledger entries listing each new shell module path,
   its surface, the commit SHA where it landed, and the line count.
3. Under "Automated Checks", add the four new guard test names.
4. Note any deferred visual smoke entries.

## Verification

```
cargo fmt -- --check
cargo clippy --lib -- -D warnings
cargo test --lib
cargo test --tests
```

All green. Verify the four new guards pass. Then run:

```
wc -l src/library.rs src/search.rs src/ui/shells/library/*.rs src/ui/shells/discover/*.rs
```

Confirm every line count ≤ 500. If any file is over, the relevant
slice needs follow-up; flag it and pause.

## Commit Message Template

```
Complete ADR 0038 task 007 screen decomposition

Slice F. Add four architecture guards: library and discover surface
file presence; entry-module ≤500 LOC; surface-module ≤500 LOC.
Update review checklist and task spec status. Decomposition is
locked: future regressions caught mechanically.
```

## Constraints

- Do not change rendering behavior in this slice. Only guards and
  documentation.
- If any guard fails, stop and report — do not relax ceilings without
  explicit user approval. Sub-splitting an over-ceiling file is
  the right response, not raising the limit.

## Definition of Done (for the whole task)

- `src/library.rs` and `src/search.rs` are ≤ 500 LOC.
- All 13 expected surface files exist under
  `src/ui/shells/{library,discover}/`, each ≤ 500 LOC.
- Four new architecture guards green.
- Review checklist updated with per-surface ledger.
- Visual smoke pairs (light + dark) per surface — environment
  permitting; deferred items tracked in the checklist.
