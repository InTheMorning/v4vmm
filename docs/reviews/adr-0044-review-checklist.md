# ADR 0044 Review Checklist

## Gate Status

Open - reconciled 2026-09-10. The handle, menu, insertion feedback, and mounted
playlist updates retain a light/dark operator recheck under
[task 003](../tasks/adr-0044-task-003-playlist-reorder-guards-visual.md).

## Requirement Disposition

| Earlier requirement | Disposition and owner |
|---|---|
| Handle-only drag, same-playlist moves, correct insertion edge, no-op drops, Move Up/Down boundary availability | Retained under ADR 0044 |
| Reorder/removal updates the mounted playlist; no placeholder flash waiting for mouse motion | Retained; AGENTS.md requires current-view updates |
| Inspector-owned Back to Playlist button and InspectorOrigin | Retired by ADR 0046 frame navigation and ADR 0047 shared inspectors. Return through frame Back/breadcrumbs |
| Playlist reached through the old Library section | Replaced by Music under ADR 0060 |
| Standalone thin separator targets or a whole-row overwrite tint | Retired by ADR 0044's recorded May 14 correction: row hover gives an insertion cue at the actual destination edge |
| Light/dark handle/menu/unavailable-row/insertion proof | Retained |

No check requires reintroducing inspector-local navigation.

## Mechanical Ownership

Existing guards in tests/architecture_tests.rs:

- playlist_reorder_display_contract_uses_drag_handle_and_menu_fallbacks
- workspace_frame_phase_2_guards_inspector_origin_navigation_is_absent
- workspace_frame_phase_2_guards_inspector_local_playlist_back_control_is_absent

Current owners: src/view_models/library.rs, src/ui/shells/playlist.rs,
src/ui/shells/library/playlist_detail.rs, and the existing application reorder
command. The guards establish ownership, not pointer interaction quality.

## Operator Visual Check

Follow [Playlist Reordering](../runbooks/inherited-ui-checks.md#playlist-reordering--adr-0044-task-003).

- [ ] Light: upward/downward drag shows the correct insertion edge and commits in place.
- [ ] Dark: the same drag behavior and legibility.
- [ ] Both themes: original-slot/outside drops do not move rows; row-body selection still works.
- [ ] Both themes: Actions menu moves rows; first/last boundaries are unavailable.
- [ ] Both themes: unavailable row remains readable; removal in the disposable library is reflected on frame return.
- [ ] Both themes: no stale playlist, mouse-dependent placeholder flash, or overwrite-like row tint.

## Evidence

The May 13-14 operator reviews found stale mounted rows, precise drop-target
requirements, placeholder flashes, and misleading insertion cues. Follow-up
fixes and intermediate passes were recorded. The final light/dark inspection
remained open. Back to Playlist and the Playlists add control had passed
before frame navigation replaced the former.

This reconciliation records no new visual pass and no regression in those
earlier accepted subchecks.

## Merge Recommendation

Keep ADR 0044 Accepted until the surviving visual gate passes. Reconcile task
003, this checklist, delivery, and pending checks together.
