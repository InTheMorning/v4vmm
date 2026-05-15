# ADR 0047 Task 010: Wire Filter Chips into ContentList Frame

Status: Blocked - 2026-05-15.

## Goal

Wire the filter chip strip into the `ContentList` frame VM and
shell. Filter changes apply only to that frame's visible rows.
Dispatch `SetFrameFilter(frame_id, ContentFilter)` to mutate state.

## Blocker

2026-05-15 exploration found that the current `ContentList` frame is still the
ADR 0046 transitional whole-screen mount around Library/Search/Settings. There
is no real GPUI-free `ContentList` page VM that can own per-frame
`filter_state`, filter-aware row projection, and empty-filter state. The
escalation trigger applies; do not implement this task by rendering chips over
the transitional mount without row filtering.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs` (ContentList page VM)
- `src/ui/shells/workspace.rs`
- `src/ui/shells/library/playlist_detail.rs` (row-rendering precedent)

## Files Likely to Change

- `src/view_models/library.rs` (add `filter_state` + filter-aware row
  projection)
- `src/ui/shells/workspace.rs` (consume the optional strip slot)
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend / Musicindex / db
- Playback
- Toolbar global search (ADR 0043)

## Constraints

- Each content-showing frame VM owns its own `filter_state`. No
  global filter store.
- `SetFrameFilter(frame_id, ContentFilter)` is the only mutator.
- Row projection consumes `filter_state`; cached rows are filtered
  in place when possible.
- Default filter is `ContentFilter::All`.
- Empty filter result triggers the empty-state contract from task 004
  (re-use that contract where possible, or add a minimal one for
  content-list).

## Implementation Steps

1. Add `filter_state: ContentFilter` to the ContentList page VM.
2. Add `set_filter(ContentFilter)` mutator.
3. Update row projection to filter by `is_in_library` / source
   provenance per `ContentFilter` semantics.
4. Add `filter_chip_strip` projection on the page VM (returns
   `FilterChipStripDisplay`).
5. Workspace shell renders the strip via `FrameShellDisplay` slot.
6. Architecture guards:
   - ContentList page VM exposes `filter_state` + `set_filter`.
   - Workspace shell passes the strip display to `frame_shell`.
   - No global filter store.

## Acceptance Criteria

- [ ] ContentList frame renders the filter chip strip.
- [ ] Filter changes apply to the frame's visible rows only.
- [ ] Empty filter result renders an empty-state notice.
- [ ] Architecture guards record the contracts.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test library
cargo test workspace
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `src/ui/shells/workspace.rs`

Goal:
- Add per-frame `filter_state` to ContentList page VM and render the
  filter chip strip in the frame.

Constraints:
- No global filter store.
- Row projection consumes filter state.

Do not touch:
- Backend, db, Musicindex
- Toolbar global search

Acceptance criteria:
- Strip renders; filter changes only that frame's rows.
- Empty-state notice renders for zero-row filters.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test library`
- `cargo test workspace`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- ContentList page VM cannot host filter state without splitting
  responsibilities (escalate).
