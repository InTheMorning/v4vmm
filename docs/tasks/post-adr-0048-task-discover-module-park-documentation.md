# Post-ADR-0048 Task — document parked `discover/` module

## Goal

The `bee1ac2` commit renamed `src/search/` → `src/discover/` and removed the
Search top-level tab. The ~5,000 LOC under `src/discover.rs`,
`src/discover/app_impl.rs`, and `src/ui/shells/discover/*` (plus
`src/ui/shells/feed.rs` and `src/ui/shells/track.rs`) compiles but is not
reachable from the composition root.

The rename was deliberate (preserves capability). The risk is that a future
audit reads this as dead code and quietly deletes it, or it rots under
refactor pressure and gets deleted in haste.

After this task:

- A note file `docs/notes/2026-05-discover-module-parked.md` documents
  why the module survives, what capability it represents, when it returns
  to the visible UI, and the conditions for acceptable deletion.
- An architecture test pins the module's `pub(crate)` surface so future
  reductions are conscious choices, not drift.

## Files To Inspect

- `src/discover.rs`
- `src/discover/app_impl.rs`
- `src/ui/shells/discover/*` (every file)
- `src/ui/shells/feed.rs`
- `src/ui/shells/track.rs`
- `src/lib.rs` (confirm `mod discover;` registration)
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0047-0048-0049-implementation-review.md` (P1 finding)
- `docs/adr/0048-content-list-frame-breadcrumb-search.md` (mentions
  "Discover module temporarily dead UI")

## Files Likely To Change

- `docs/notes/2026-05-discover-module-parked.md` — new
- `tests/architecture_tests.rs` — new guard

## Do Not Touch

- Any code under `src/discover/`.
- Any code under `src/ui/shells/discover/`, `src/ui/shells/feed.rs`,
  `src/ui/shells/track.rs`.
- The decision to keep the module compiled (that decision was made by the
  user; this task documents it, does not reopen it).

## Constraints

- Documentation + arch test only. No code changes.
- The note file is a `notes/` doc per the repo-docs-organizer pattern,
  not an ADR (a parked-module note is not an architectural decision).
- The arch test pins the surface, it does not enforce reachability. The
  module being unreachable is the documented state; the guard prevents
  silent surface shrinkage.
- No commit unless explicitly asked.

## Note file outline

`docs/notes/2026-05-discover-module-parked.md`:

```
# discover/ module — parked status (2026-05-16)

## What

src/discover.rs + src/discover/app_impl.rs + src/ui/shells/discover/* +
src/ui/shells/feed.rs + src/ui/shells/track.rs together implement the
former dedicated Discover (formerly Search) experience: dedicated frame,
track inspector with metadata grid/tree, feed lists, recent items, action
buttons, etc. After ADR 0048, the surface lives inside the ContentList
frame and these modules are not reached from the composition root.

## Why it survives

The user explicitly chose to rename `src/search/` → `src/discover/` rather
than delete. Reasons:

1. Preserves the deeper inspector pattern (metadata grid/tree, expandable
   cells, action affordances) as reference material for the next inspector
   ADR.
2. Avoids irreversible loss of working code that might re-enter the visible
   UI under a future feature.
3. Keeps `cargo doc` historically honest about what shipped.

## When it returns

No scheduled return. Candidates:

- A "compare" affordance that needs a side-by-side detail surface.
- A dedicated "Discover" view if the Index browsing surface diverges enough
  from Library detail to warrant its own shell stack.

## Acceptable conditions for deletion

Delete only when ALL of:

- The next inspector-revision ADR documents what reference material was
  copied out first.
- No remaining caller (CTRL-F `crate::discover` and
  `ui::shells::{discover,feed,track}` returns only intra-discover hits).
- The user explicitly approves the deletion in the same conversation.

## Current call graph (snapshot)

src/lib.rs                          mod discover;
src/discover.rs                     pub use ... (re-exports)
src/discover/app_impl.rs            SearchApp entity, query/event handling
src/ui/shells/discover/*            shell render code (~15 files)
src/ui/shells/feed.rs               imports crate::discover::*
src/ui/shells/track.rs              imports crate::discover::*

No render path from src/app.rs reaches these shells.

## See also

- ADR 0047 (library/search unification)
- ADR 0048 (ContentList breadcrumb search)
- docs/reviews/adr-0047-0048-0049-implementation-review.md (P1 finding)
```

## Architecture guard outline

Add to `tests/architecture_tests.rs`:

```rust
#[test]
fn discover_module_public_surface_is_pinned() {
    // Walk src/discover.rs and assert the file's `pub(crate) use` /
    // `pub(crate) fn` / `pub(crate) struct` / `pub(crate) enum` items
    // match a known fixture. If a maintainer removes one, the test fails
    // and the diff prompts a conscious decision.
    //
    // The fixture is the list of pub(crate) names as of 2026-05-16,
    // captured at the time of this guard's introduction. Update the
    // fixture only with deliberate maintenance, not in passing.
}
```

The fixture is the actual `pub(crate)` names from `src/discover.rs` (it has
`pub(crate) use` re-exports of the inspector internals). Read the file
when implementing and snapshot the exact list.

## Implementation Steps

1. Read `src/discover.rs` and enumerate every `pub(crate) use`,
   `pub(crate) fn`, `pub(crate) struct`, `pub(crate) enum`,
   `pub(crate) mod`, `pub(crate) type`.
2. Write the note file at `docs/notes/2026-05-discover-module-parked.md`
   using the outline above. Replace the call-graph snapshot with the
   current real call graph (grep for `use crate::discover`).
3. Add the architecture guard with the captured fixture.
4. Run the 5 gates.

## Acceptance Criteria

- `docs/notes/2026-05-discover-module-parked.md` exists, follows the
  outline, and matches the real current call graph.
- `tests/architecture_tests.rs` contains
  `discover_module_public_surface_is_pinned` (or equivalent name) and the
  test passes against the current `src/discover.rs`.
- All 5 gates pass.
- No code under `src/discover/` or related shells changed.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded documentation + arch-guard task.

Read:
- This task file
- `src/discover.rs` in full
- `src/lib.rs` to confirm `mod discover;`
- `tests/architecture_tests.rs` (skim existing path-walk guards to match
  style)

Goal:
- Write `docs/notes/2026-05-discover-module-parked.md` per the outline in
  the task file. Substitute the real current call graph (grep for
  `use crate::discover`).
- Add an arch guard `discover_module_public_surface_is_pinned` to
  `tests/architecture_tests.rs` capturing the current `pub(crate)` surface
  of `src/discover.rs` as a fixture.

Constraints:
- No code changes to `src/discover/` or related shells.
- Note file goes under `docs/notes/`, not `docs/adr/` (parked-module note
  is not an architectural decision).
- Never skip hooks. Don't commit unless explicitly asked.

Test commands:
- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. note file path + key sections written
2. arch guard name + fixture contents
3. test pass confirmation
4. deviations
5. unresolved concerns

## Escalation Triggers

- An existing arch guard already pins the discover surface in a different
  way. Report; do not add a duplicate.
- The `pub(crate)` surface is large enough that capturing it in-source is
  unreadable. Move the fixture to a tests/-adjacent text file and load it.
