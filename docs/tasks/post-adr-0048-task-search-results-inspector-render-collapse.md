# Post-ADR-0048 Task — collapse search-results-inspector render entry points

## Goal

`src/ui/shells/search_results_inspector.rs` exposes two near-identical
public render functions:

- `render_search_results_inspector(vm, slots, cx)` at line 89 (tabbed mode)
- `render_search_results_inspector_scoped(vm, slots, tab, filter, cx)` at
  line 106 (single-tab mode used for Index detail bodies)

Both delegate to a private `render_search_results_inspector_with_scope`
that takes a `SearchResultsHeaderMode` (`Tabbed` / `Scoped`).

Collapse to one public function that takes `SearchResultsHeaderMode`.

## Files To Inspect

- `src/ui/shells/search_results_inspector.rs`
- `src/app.rs` (call sites at lines 45-46, 1794, 1817)
- `tests/architecture_tests.rs` (if any guards reference either function
  by name)
- `docs/reviews/adr-0047-0048-0049-implementation-review.md` (P2 finding)

## Files Likely To Change

- `src/ui/shells/search_results_inspector.rs` — collapse two public fns
  to one
- `src/app.rs` — update the two call sites and the imports at lines 45-46
- `tests/architecture_tests.rs` — only if a guard names the removed fn

## Do Not Touch

- The `SearchResultsHeaderMode` enum (already exists; keep it).
- The private `render_search_results_inspector_with_scope` body.
- VM logic.
- Other UI shells or composites.

## Constraints

- Behavior-preserving. The two scoped vs tabbed call sites must produce
  identical output before and after.
- New public signature should be ergonomic at call sites. Suggested:
  ```rust
  pub(crate) fn render_search_results_inspector(
      vm: &SearchResultsInspectorPageVm,
      slots: &SearchResultsInspectorSlots,
      mode: SearchResultsHeaderMode,
      cx: &App,
  ) -> AnyElement
  ```
  with `SearchResultsHeaderMode::Tabbed` carrying nothing and
  `SearchResultsHeaderMode::Scoped { tab, filter }` carrying the tab +
  filter the current `_scoped` variant takes.
- No commit unless explicitly asked.

## Implementation Steps

1. Read the two existing public fns and the private
   `render_search_results_inspector_with_scope` to confirm the signature
   collapse is mechanical.
2. Decide whether to enrich `SearchResultsHeaderMode` to carry the scoped
   variant's args, or to keep them as separate parameters. Prefer enriched
   enum: call sites read better and "scoped mode requires tab + filter"
   becomes type-enforced.
3. Update `SearchResultsHeaderMode` if needed.
4. Replace the two public fns with one. Keep the same fn name
   `render_search_results_inspector` (it is the more common one); the
   `_scoped` variant goes away.
5. Update `src/app.rs:45-46` imports: remove
   `render_search_results_inspector_scoped`.
6. Update `src/app.rs:1794` and `src/app.rs:1817` to pass the appropriate
   `SearchResultsHeaderMode` variant.
7. Run the 5 gates.

## Acceptance Criteria

- Only one public render fn exists in `search_results_inspector.rs`.
- `src/app.rs` imports a single entry point.
- Both call sites compile and the visual output is unchanged (no behavior
  change).
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

You are implementing one small bounded refactor.

Read:
- This task file
- `src/ui/shells/search_results_inspector.rs` (full)
- `src/app.rs` lines 40-60 and 1780-1830

Goal:
- Collapse `render_search_results_inspector` and
  `render_search_results_inspector_scoped` into one public fn that takes
  `SearchResultsHeaderMode`. Enrich the enum to carry the scoped variant's
  `tab` and `filter` arguments.
- Update the two call sites in `src/app.rs` and the imports.

Constraints:
- Behavior-preserving.
- Single fn name: `render_search_results_inspector`.
- Never skip hooks. Don't commit unless explicitly asked.

Test commands:
- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. enum shape after change
3. call-site diff
4. test results
5. deviations
6. unresolved concerns

## Escalation Triggers

- The two existing public fns differ in something other than header mode
  (e.g., one runs an additional VM hook). Report; do not silently merge.
- A test exists that pins the two-fn shape. Update the test only if the
  test was pinning structure, not behavior.
