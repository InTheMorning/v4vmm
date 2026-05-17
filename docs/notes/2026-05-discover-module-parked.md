# discover/ module — parked status (2026-05-16)

## What

`src/discover.rs` + `src/discover/app_impl.rs` + `src/ui/shells/discover/*` +
`src/ui/shells/feed.rs` + `src/ui/shells/track.rs` together implement the
former dedicated Discover (formerly Search) experience: dedicated frame,
track inspector with metadata grid/tree, feed lists, recent items, action
buttons, and related shell glue. After ADR 0048, the visible surface lives
inside the ContentList frame and these modules are not reached from the
composition root.

## Why it survives

The user explicitly chose to rename `src/search/` -> `src/discover/` rather
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

- The next inspector-revision ADR documents what reference material was copied
  out first.
- No remaining caller (`CTRL-F crate::discover` and
  `ui::shells::{discover,feed,track}` returns only intra-discover hits).
- The user explicitly approves the deletion in the same conversation.

## Current call graph (snapshot)

`src/lib.rs` -> `pub mod discover;`

`src/discover.rs` -> `pub(crate) use` re-exports for inspector actions and
track-row helpers

`src/discover/app_impl.rs` -> `SearchApp` entity, query handling, and event
handling

`src/ui/shells/discover/*` -> screen render code and local inspector shells,
all importing `crate::discover`

`src/ui/shells/feed.rs` -> imports `crate::discover::{...}` to render the
Discover feed view

`src/ui/shells/track.rs` -> imports `crate::discover::{...}` for Discover
track-row and identity-action rendering

No render path from `src/app.rs` reaches these shells.

## See also

- ADR 0047 (library/search unification)
- ADR 0048 (ContentList breadcrumb search)
- `docs/reviews/adr-0047-0048-0049-implementation-review.md` (P1 finding)
