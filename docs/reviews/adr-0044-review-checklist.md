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

- [x] 2026-09-16 follow-up: normal-build restart removes the reported pause during reordering and horizontal out-of-bounds dragging; post-check preservation passes.
- [ ] Light: upward/downward drag shows the correct insertion edge and commits in place.
- [ ] Dark: the same drag behavior and legibility.
- [ ] Both themes: original-slot/outside drops do not move rows; row-body selection still works.
- [ ] Both themes: Actions menu moves rows; first/last boundaries are unavailable.
- [ ] Both themes: unavailable row remains readable; removal in the disposable library is reflected on frame return.
- [ ] Both themes: no stale playlist, mouse-dependent placeholder flash, or overwrite-like row tint.

## Evidence

### Drag Pause And Gesture Isolation — 2026-09-16

The operator reported a brief app pause while dragging the handle in ADR 0066
V2 fixture `/tmp/v4vmm-startup-4yqtadz3`; menu moves did not pause. Both paths use
the existing asynchronous reorder command. A focused interaction test reproduced
Root text selection remaining active after a handle drop. The shared handle now
stops mouse-down propagation, as other gesture-owning controls do.
`adr_0044_playlist_drag_does_not_start_text_selection` failed before correction
and passes afterward, including one drop with the original subject, completed
drag cleanup, no unsolicited text selection and subsequent ordinary selection.

Mechanical verification is Green: formatting, compile, strict Clippy, debug
build, 1,410 unit tests and 251 architecture guards; ten existing documentation
examples remain ignored.

This establishes a gesture conflict and its bounded correction, not desktop
timing. The reported pause and existing Light/Dark visual gate remain open.
The owning packet provides the retained-fixture recheck and cleanup route.

The operator recheck fails: pressing the handle immediately pauses the app for
4–5 seconds with a hand cursor; the move occurs afterward. The gesture correction
did not resolve this native stall. The handle press/preview constructor has no
database work, and reorder dispatch occurs on drop. Native stacks are requested
using the packet's timed capture procedure before another behavioral change.
No new implementation or mechanical-suite result is claimed for this diagnostic
documentation update.

The supplied `playlist-drag-stacks.txt` captures PID 41408 successfully. Thread 1
is computing nested Taffy block/flex layout; the 40-frame trace ends before the
outer caller. This establishes layout work at one instant, not its duration or
the distinction between an expensive frame and repeated redraws. Capture SHA-256:
`fe87894b347b4e823cda0b1be351edfc5c1ac28ee5c12a63c6b0b0764c5639b8`.

A temporary simulated-window probe of the three-track shared playlist and Root
took about 3–4 ms per dispatch/redraw. Adding workspace frame and configuration
notices at 1400×850 took about 11–13 ms. It did not reproduce the native pause and
was removed. No additional production fix or visual pass is claimed. A native
main-thread CPU profile is requested by the owning packet; the capture and
fixture remain retained. Formatting, diff whitespace and documentation links
are Green for this diagnostic-only update.

The subsequent CPU report contains 846 samples, none lost, and approximately
8.55 seconds of main-thread CPU time. Flex layout is the largest self symbol
(21.16%), followed by block inner computation (6.38%) and further layout helpers.
The operator also reports a longer pause on horizontal out-of-bounds dragging.
The `addr2line` error concerns the report's debug-file analysis. Its app build ID
matches the worktree binary, but most caller chains are absent in the text report.
Offline recovery from the subsequently supplied raw recording finds whole-window
layout called by GPUI's synchronous test-only `App::flush_effects` drawing loop.
Running the architecture tests reproduced the exact captured binary build ID;
a normal desktop build excludes the test loop. The startup fixture launcher now
rebuilds the normal binary before opening the app and stops on build failure.
Its 16 fixture tests and 251 architecture guards are Green.

The operator subsequently reports **pass** for the instructed restart,
reordering and horizontal out-of-bounds drag check. The supplied preservation
inspection for `/tmp/v4vmm-startup-4yqtadz3` passes every configuration check,
original/private-backup preservation, music, library, bindings, tool blockers
and migrations 1–11, with no residual candidates or probes. The drag pause
correction is accepted. The broader theme-specific checklist above remains
open; the latest check did not request those separate cases. Task 007's ordinary
Null-player Play check and final preservation inspection subsequently pass.
Fixture cleanup and directory absence are subsequently confirmed under that
packet on 2026-09-16. The broader checklist remains open.

### Earlier Evidence

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
