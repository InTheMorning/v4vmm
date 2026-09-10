# ADR 0043 Review Checklist

## Gate Status

Open - reconciled 2026-09-10. Only current toolbar readability and interaction
need operator recheck. [Task 004](../tasks/adr-0043-task-004-guards-and-visual-readiness.md)
owns that gate. Historical mechanical results do not establish current visual
acceptance.

## Requirement Disposition

| Earlier requirement | Disposition and owner |
|---|---|
| Three-zone toolbar with trailing Now Playing frame; compact player width; player composition ownership | Retired. ADR 0046 moved transport into the queue frame; ADR 0060 removed the toolbar player; ADR 0063 owns Show transport placement |
| Global All/Library/Index scope controls, GlobalSearchScope, and grouped results in a Search workspace | Retired. ADR 0047 moved filtering to frame chrome and unified origins; ADR 0048 owns ContentList search navigation |
| Rename Discover tab to Search; test a separate Search workspace | Retired. ADRs 0048/0060 define search as a command and Music/Show/Settings as sections |
| Recent Feeds as the empty-query Search root | Retired. ADR 0062 removed the separate destination and owns Music's default content |
| Single toolbar search, VM-owned display facts, input focus, Enter/Search submission | Retained on current Music routing |
| Readable normal/narrow toolbar in Light and Dark | Retained; open visual gate |
| Local query returns library members only | Retained mechanical boundary; no global-scope control is required |

The retired requirements are not part of the checklist below. No structural
retirement is recorded as a visual pass.

## Mechanical Ownership

Existing guards in tests/architecture_tests.rs:

- app_toolbar_exposes_tabs_and_global_search_without_now_playing_chip
- global_search_contract_has_toolbar_vm_and_local_query_boundary
- global_search_replaces_screen_local_search_chrome
- global_search_routes_to_content_list
- adr_0060_toolbar_no_longer_carries_now_playing_chip

These replace repeated implementation assertions. Current owners are
src/view_models/app_toolbar.rs, src/app/tab_bar.rs, src/app/search_dispatch.rs,
and src/app/keyboard.rs.

## Operator Visual Check

Follow [Search Toolbar](../runbooks/inherited-ui-checks.md#search-toolbar--adr-0043-task-004).

- [ ] Normal width, Light: input, clear control, Search action, focus, and submission.
- [ ] Narrow width, Light: readable input and reachable compact action/menu.
- [ ] Normal width, Dark: the same controls and behavior.
- [ ] Narrow width, Dark: the same compact layout and behavior.

No extra global input may appear in entity details. Frame-local filtering is
allowed by ADR 0047. Enter and the Search action must reach the same current
result surface.

## Evidence

The 2026-05-14 review recorded mechanical checks Green and repeated narrow
toolbar clipping fixes. Its last light/dark recheck remained outstanding.
Later ADRs replaced the player and scope structures involved. This
reconciliation retires those structures but supplies no new toolbar acceptance.

## Merge Recommendation

Keep ADR 0043 Accepted until the surviving visual gate passes; record each
result here and in task 004, then reconcile delivery and pending checks.
