# ADR 0043 Review Checklist

## Gate Status

Complete - 2026-09-18. The operator passed normal/narrow toolbar checks in Light
and Dark and confirmed fixture cleanup.
[Task 004](../tasks/adr-0043-task-004-guards-and-visual-readiness.md) is complete.
The evidence below records each batch. No ADR 0043 gate remains open.

## Requirement Disposition

| Earlier requirement | Disposition and owner |
|---|---|
| Three-zone toolbar with trailing Now Playing frame; compact player width; player composition ownership | Retired. ADR 0046 moved transport into the queue frame; ADR 0060 removed the toolbar player; ADR 0063 owns Show transport placement |
| Global All/Library/Index scope controls, GlobalSearchScope, and grouped results in a Search workspace | Retired. ADR 0047 moved filtering to frame chrome and unified origins; ADR 0048 owns ContentList search navigation |
| Rename Discover tab to Search; test a separate Search workspace | Retired. ADRs 0048/0060 define search as a command and Music/Show/Settings as sections |
| Recent Feeds as the empty-query Search root | Retired. ADR 0062 removed the separate destination and owns Music's default content |
| Single toolbar search, VM-owned display facts, input focus, Enter/Search submission | Retained on current Music routing |
| Readable normal/narrow toolbar in Light and Dark | Retained. Operator checks passed on 2026-09-18. |
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

- [x] Normal width, Light: input, clear control, Search action, focus, and submission passed on 2026-09-18.
- [x] Narrow width, Light: input layout, compact Search and the single global input in details passed on 2026-09-18.
- [x] Normal width, Dark: keyboard search, clear control, Search-button submission and layout passed on 2026-09-18.
- [x] Narrow width, Dark: layout, compact Search and the single global input in details passed on 2026-09-18.

No extra global input may appear in entity details. Frame-local filtering is
allowed by ADR 0047. Enter and the Search action must reach the same current
result surface.

## Evidence

### Operator Batches — 2026-09-18

The operator prepared `/tmp/v4vmm-governance.jTg6NGQf` from the normal library.
The operator reported successful fixture creation and copying.
The fixture uses a private database, copied audio and separate configuration/data/cache paths.
The requested build selected the normal `v4vmm` binary.

Batch 3 requested Light theme, Medium scale and a window approximately 1400 pixels wide.
The operator reported these results with query `heycitizen`:

| Check | Result |
|---|---|
| Ctrl+F directs typing to the toolbar input | Pass |
| Enter opens results in Music | Pass |
| Input, clear control and Search action remain readable without clipping or overlap | Pass |

Batch 4 retained Light theme, Medium scale and the wide window.
The operator reported that the clear control removed the query.
The Search button passed with query `heycitizen`.
The operator identified both Library and Index results.
The Light normal-width toolbar check is accepted.

Batch 5 requested Light theme, Medium scale and a window approximately 560 pixels wide.
The operator passed the narrow layout, compact Search action and detail toolbar checks.
The requested detail check required one global search field, with frame-local filters permitted.
Both Light width checks are accepted.

Batch 6 requested Dark theme, Medium scale and a window approximately 1400 pixels wide.
The operator reported that all three checks passed with query `heycitizen`:
keyboard search, clear/Search-button operation and toolbar layout.
The Dark normal-width check is accepted.

Batch 7 requested Dark theme, Medium scale and a window approximately 560 pixels wide.
The operator reported that narrow layout, compact Search and the detail toolbar all passed.
All four theme/width checks are accepted.
Batch 8 instructed the operator to quit the fixture app before cleanup.
The cleanup command checked the exact directory marker and rejected a symbolic-link root.
The operator returned:

```text
Removed fixture: /tmp/v4vmm-governance.jTg6NGQf
```

Fixture cleanup is confirmed. This evidence comes from the operator's report.
The agent did not run the app or delete the fixture.

Procedure correction: `Find` names the keyboard action in `src/app/keyboard.rs`.
`src/app/menu.rs` installs native menus only on macOS and contains no Find item.
The Linux procedure now checks Ctrl+F without requiring a nonexistent Find menu.
ADR 0067 owns the platform shortcut. This correction supplies no additional visual pass.

### Earlier Evidence

The 2026-05-14 review recorded mechanical checks Green and repeated narrow
toolbar clipping fixes. Its last light/dark recheck remained outstanding.
Later ADRs replaced the player and scope structures involved. This
reconciliation retires those structures but supplies no new toolbar acceptance.

## Merge Recommendation

Task 004 is complete. ADR 0043 is Implemented.
All surviving visual requirements passed, and fixture cleanup is confirmed.
No runtime code changed during this acceptance pass.
The existing procedure remains the regression check.
