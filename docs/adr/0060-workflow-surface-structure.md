# ADR 0060: Workflow Surface Structure And Vocabulary

## Status

Accepted - 2026-09-07.

Amends ADR 0048 for the app-section tab set. Supersedes the `Broadcast`
workspace frame introduced under ADR 0059 and retains everything else in that
ADR.

Amended 2026-09-11: corrected the obsolete reserved audition ADR number to
[proposed ADR 0068](0068-show-cue-and-audition-isolation.md). The workflow
separation below remains binding; the proposed mechanism is not yet accepted
or implemented.

## Context

`docs/plans/curator-workflow-ui-design-brief.md` records the product intent.
This ADR records the structural decisions that intent requires.

A visual inspection on 2026-09-07 found the following in the shipped interface.
The screenshot is attached to the brief.

- The center content pane holds about 40 percent of the window and shows
  `Select an item to view details` by default. The largest region is empty.
- The music has the least space. A library of 73 tracks appears as six
  collapsed artist names in a 360 pixel sidebar. No artwork appears anywhere.
- The layout is a `Library` frame holding a moveable split, then a moveable
  split, then `Queue` and `Broadcast` at a **fixed** fifty-fifty share. The
  empty region above is the detail side of the `Library` frame's own split, so
  a curator can drag it away. The `Queue` and `Broadcast` share cannot be
  changed at all.
- A curator who is not broadcasting therefore cannot reclaim the space that
  `Queue` and `Broadcast` hold. Nothing adapts to the task, and in the one case
  where adapting would help most, the split is locked.
- The `Broadcast` pane shows three stacked empty states: no source selected,
  publisher not installed, and no broadcast event. A wall of unavailability
  holds premium space permanently.
- Hierarchy is flat. Border treatment, panel chrome, and type size are the same
  everywhere, so nothing directs the eye.
- Now-playing is a small chip in an otherwise empty toolbar, in an app where
  now-playing drives listener payments.
- Three row idioms coexist: loose tree rows, tight queue rows, and label and
  value broadcast rows.

The common cause is that the interface is organized by component rather than by
task. ADR 0046 gave frames, ADR 0047 unified the content surface, and ADR 0048
made search a command. Those decisions were correct and this ADR keeps them.
What is missing is a task-level structure above them.

## Decision

### App Sections Are Music, Show, And Settings

The top toolbar exposes three app sections: `Music`, `Show`, and `Settings`.

ADR 0048 decided the tab set was `Library` and `Settings`. This ADR amends that
set. Everything else in ADR 0048 stands:

- Search remains a toolbar command, not a tab and not a screen.
- One content surface serves library rows and index rows together.
- Breadcrumbs remain frame chrome projected from frame navigation state.

`Music` is the renamed `Library` section. It is the curation surface, and it
holds both index results and the tracks the curator has taken in. The existing
`ContentFilter` of ADR 0047 becomes its scope control, promoted from a chrome
detail to a primary affordance.

`Show` is new. Broadcasting is a different task from curating, with a different
layout, a different reading distance, and a different set of things that
matter. It does not belong inside a browsing surface.

### Show Is A Screen Mount

`Show` is a `WorkspaceScreenMount`, beside `Library` and `Settings`. It is not
a workspace frame.

A workspace frame carries navigation history, breadcrumbs, and a content stack.
The live surface needs none of those and needs a layout the frame lanes do not
serve: few panes, large type, and readability from across a room.

`WorkspaceFrameKind::Broadcast` is removed. The frame, its page view model, its
shell, and its architecture guards are replaced rather than extended.

### The Queue Belongs To Show

The persistent `Queue` pane is removed from the curation surface and becomes
part of `Show`.

Queue and transport are show playback. They belong with the surface that owns
the show. During curation they occupied a locked share of the window for a task
the curator was not doing.

Auditioning stays in `Music` as an inline affordance on a row, and it is a
separate mechanism from show playback. [Proposed ADR 0068](0068-show-cue-and-audition-isolation.md)
records that mechanism; its implementation remains open.

### Curation Shows Nothing Operational

When no show is active, `Music` and `Settings` show no queue, no transport, no
broadcast panes, and no service status. The curator browses, discovers, and
curates with the whole window.

An operational surface that cannot act is not shown as unavailable. It is
absent.

### A Live Status Strip Spans Music And Settings

When a show is active, a compact status strip appears at the top of `Music` and
`Settings`. A show is active when show playback is running or a broadcast is
running.

The strip exists so a curator who steps away from `Show` to find a track or fix
a setting keeps confidence in the broadcast without changing sections.

It is a glance surface, not a control surface. The proposed minimum content:

- the listener-facing now-playing line, artist and title
- one aggregate health indicator that does not rely on color alone
- recording state when a recording is running
- an action that opens `Show`

The strip does not exist when no show is active. The current toolbar
now-playing chip is replaced by it, so now-playing has one owner rather than
two.

### Vocabulary

The curator sees `Music` and `Library`:

- `Music` is the surface. It holds everything the curator can see, index and
  local together.
- `Library` is the subset the curator has taken in.

The rename is user-facing only. `is_in_library`, `v4vmm library tracks`, and
module names keep the existing vocabulary. The ADR 0017 CLI debug contract does
not change.

Two label pairs replace internal terms:

| Internal | Curator-facing |
|---|---|
| stale, feed update available | `Update available` |
| unreviewed release | `New` |

`Cached` leaves the curation surface. Cache state appears in `Settings` under
storage, with a size and a purge control. ADR 0061 owns the policy.

### Sections Earn Their Place By Mode

A section that cannot apply to the selected production mode is not rendered as
a disabled block. Mode-aware readiness projections decide what a surface shows,
and that decision belongs to a view model, not to a renderer condition.

## Invariants

- The toolbar exposes exactly three app sections: `Music`, `Show`, `Settings`.
- Search is a command. No app section is a search destination.
- `Show` is a screen mount. No workspace frame kind is named for broadcasting.
- Queue and transport render only inside `Show`.
- Audition renders only inside `Music` and never sets now-playing state.
- `Music` and `Settings` render no operational surface when no show is active.
- The live status strip renders only when a show is active, and it dispatches no
  broadcast command other than opening `Show`.
- The user-facing words are `Music` and `Library`. The database and the CLI keep
  `library`, and no user-facing string says `collection`.
- A section that does not apply to the selected mode is absent, not disabled.

## Alternatives Considered

### Keep The Library And Settings Tab Set

Rejected. It preserves ADR 0048 exactly and leaves the observed defect in
place. The live surface then lives inside a browsing frame, which is what
produced three competing panes.

### Discover, Collection, And Show

Rejected. This was the brief's proposal. It re-creates the separate browse
surface that ADR 0047 collapsed on purpose, and it revives modules that
`docs/notes/2026-05-discover-module-parked.md` records as parked for good
reasons. `Music` with a scope control gives the same reachability with one
surface.

### Show As A Workspace Frame

Rejected, and it was the shipped design. A frame brings navigation history,
breadcrumbs, and a content stack that a live console does not use, and it
constrains the layout to the frame lanes. The empty-state wall in the
screenshot is what that constraint produced.

### Rename Library To Collection Everywhere

Rejected. It touches 127 references in `src/`, a public CLI contract under ADR
0017, and the titles of two completed ADRs, to change one word. `Music` as the
surface name already solves the reading problem the brief raised.

### Keep The Queue Visible During Curation

Rejected. It holds a locked share of the window for a task the curator is not
doing while browsing. A curator who wants to hear a track uses audition.

### Show Broadcast Status Permanently

Rejected. It is what the screenshot shows, and it produces a wall of empty
states during the hours a curator spends not broadcasting. The live status
strip gives the same confidence during the minutes it matters and costs
nothing the rest of the time.

## Consequences

Positive:

- The largest region of the window shows music instead of an empty prompt.
- Curating and broadcasting stop competing for the same screen.
- The empty-state wall disappears, because a surface that does not apply is
  absent rather than rendered as unavailable.
- One vocabulary reaches the curator, and the code keeps the vocabulary it has.

Negative and risks:

- Tasks 005, 006, and 007 are discarded. Their architecture guards must be
  removed with them, not left asserting against absent code.
- A third screen mount adds a layout to maintain that does not reuse the frame
  chrome.
- Moving the queue into `Show` means a curator who wants a queue while browsing
  has no affordance. Audition is the answer, and audition does not queue.
- The live status strip is a second place now-playing appears. It must project
  from the same view model as the `Show` surface, or the two will disagree.
- A user-facing to internal vocabulary mapping is permanent, and every future
  packet must respect it.

## Follow-Up Work

- ADR 0061, cache and dump policy. `Dump` cannot ship without it.
- [ADR 0068, Show cue and audition isolation](0068-show-cue-and-audition-isolation.md), Proposed.
- ADR 0063, play history and rotation warnings.
- Mine the parked discover modules for the metadata grid and tree patterns,
  then delete them.
- Revise the curator workflow design brief against this ADR.
- Revise every pending ADR 0059 packet before implementation resumes.

## References

- ADR 0046 - Workspace frame architecture
- ADR 0047 - Library and search unification
- ADR 0048 - ContentList frame breadcrumb search
- ADR 0057 - ADR status vocabulary and amendment policy
- ADR 0059 - Broadcast control surface
- `docs/plans/curator-workflow-ui-design-brief.md`
- `docs/notes/2026-05-discover-module-parked.md`
