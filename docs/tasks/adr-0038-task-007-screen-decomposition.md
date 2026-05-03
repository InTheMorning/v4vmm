# ADR 0038 Task 007: Screen Decomposition (Stub)

## Status

Stub. Starts after Task 006 (PageVm Generalization) lands. Likely
splits into Task 007a (Library) and Task 007b (Discover).

## Goal

Split `src/library.rs` (3,907 LOC) and `src/search.rs` (6,445 LOC)
along surface lines under `src/ui/shells/library/` and
`src/ui/shells/discover/`. Each per-surface file ≤ 500 LOC. The
top-level entry modules shrink to thin command/state wiring.

## Target File Layout

```
src/ui/shells/library/
├── mod.rs
├── sidebar.rs
├── feed_list.rs
├── feed_detail.rs
├── track_detail.rs
├── playlist_detail.rs
└── now_playing.rs

src/ui/shells/discover/
├── mod.rs
├── recent.rs
├── result_list.rs
├── feed_inspector.rs
├── track_inspector.rs
└── search_input.rs
```

Refine when starting; the layout is a target, not a contract.

## Why This Is Phase 7 (Not Phase 1)

Decomposing `library.rs`/`search.rs` before VM consolidation (Task 003)
and PageVm generalization (Task 006) just relocates the mess. After
Tasks 002–006, fallback policy lives in VMs, page assembly lives in
shell helpers, and the screen-side code is mostly command dispatch and
selected-entity state. At that point the split is mechanical.

## Files Likely To Change

- New: `src/ui/shells/library/*.rs`, `src/ui/shells/discover/*.rs`.
- Reduced: `src/library.rs`, `src/search.rs` to thin entry modules
  (target ≤ 500 LOC each; ideally ≤ 300).
- `src/lib.rs` — module wiring.
- `tests/architecture_tests.rs` — new guards:
  - `library_screen_modules_are_decomposed_under_src_ui_shells_library`
  - `discover_screen_modules_are_decomposed_under_src_ui_shells_discover`
  - Optional: `screen_entry_modules_under_500_loc` with explicit
    per-file ceilings.

## Open Questions

1. **Selected-entity state.** Where does the currently-selected
   feed/track/playlist live after the split? Likely the entry module
   (`library.rs`) keeps it; surface modules accept it as input.
   Confirm before splitting.
2. **Event wiring.** GPUI event listeners (`cx.listener(...)`) often
   reference `&mut self` on the screen. Plan how the surface modules
   call back into the entry module's mutators without forming cycles.
3. **Test coverage during the split.** Each surface move is one
   commit; visual smoke and `cargo check` must stay green throughout.
   Don't bundle.
4. **Grouping rule.** A surface boundary is "the user's mental
   surface": sidebar, feed list, feed inspector, track inspector,
   recent tiles, now-playing. Don't fork on internal boundaries
   (e.g., "table of feeds vs. metadata grid for one feed" is one
   surface, not two).

## Constraints

- Each split commit moves *one* surface to its own file. Compile and
  test green at every commit.
- No behavior changes during the split. Visual smoke must match
  pre-split for every surface.
- If the split surfaces a structural issue (e.g., a function genuinely
  belongs in two surfaces), pause and resolve via Task 002/003
  patterns rather than duplicating.

## Definition of Done

- `src/library.rs` and `src/search.rs` are ≤ 500 LOC and contain only
  command wiring and selected-entity state.
- Every surface lives in its own file under
  `src/ui/shells/{library,discover}/`.
- New guards green.
- Visual smoke for every surface (light + dark).

## When To Start

After Task 006 lands. Replace this stub with a fully-specified task
listing the per-surface migration order (smallest surface first).
