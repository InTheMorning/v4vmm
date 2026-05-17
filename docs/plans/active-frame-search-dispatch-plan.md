# Active-Frame Search Dispatch

Status: Superseded - 2026-05-16.

This plan's toolbar-dispatch model was retired by ADR 0048 and
`docs/plans/search-in-library-frame-plan.md`.

Toolbar Search now opens `FrameNavigationEntry::Search(query)` in the
workspace `ContentList` frame. It pushes from non-search content and replaces
the active search flow from an existing search so repeated searches do not
stack query crumbs. Search results render in that frame, with breadcrumb/back
navigation through the same `ContentList` stack. The old Detail-frame helper
and secondary "Search in new frame" path are not part of the active contract.

The Phase 1 VM contracts (`FrameSearchScope`, `FrameSearchDescriptor`, and
page-VM text filter hooks) remain as infrastructure for a future explicit
in-frame find/filter affordance. They are not driven by the toolbar Search
submit.

Current source of truth:

- `docs/adr/0048-content-list-frame-breadcrumb-search.md`
- `docs/plans/search-in-library-frame-plan.md`
- `tests/architecture_tests.rs::global_search_routes_to_content_list`
- `tests/architecture_tests.rs::adr_0048_forbids_secondary_search_frame_path`
