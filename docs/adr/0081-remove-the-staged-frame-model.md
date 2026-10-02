# ADR 0081: Remove The Staged Frame Model

## Status

Accepted - 2026-10-02. The operator chose this plan on 2026-10-02, after dead code removal task 002.

It supersedes [ADR 0046](0046-workspace-frame-architecture.md) Architectural Invariant 8 and the model sentence of its Resolved Open Question 5.
It also removes two slot rules: the Queue slot that ADR 0060 task 002 kept, and the Detail filter-chip slots of ADR 0047 task 014.

## Context

ADR 0046 Invariant 8 states: "Detach/dock is model-only in v1. Detach/dock metadata exists on `WorkspaceFrameKind`. Commands return a deferred-error variant."
Its Resolved Open Question 5 states: "The workspace model may carry add/remove operations before every frame kind is mounted."

ADR 0046 tasks 012 and 014 built that model on 2026-05-15. Task 013, the visible multi-frame commands, is Deferred, because no frame has its own content owner.
No production path calls the model. Dead code removal task 002 moved it, on 2026-10-01, into test builds only. Four guards require it to exist with no caller.

ADR 0060 made the app structure fixed: Music, Show and Settings. With no show active, Music shows no queue.
The Queue slot of the curation workspace exists only "until the frame kind is removed".
ADR 0047 task 014 kept two Detail-frame filter-chip slots for an inspector chrome that no design describes.

ADR 0046 Invariant 2 states that the frame chrome owns Back, Forward, close and history. Back is live. The Forward model (`go_forward`) exists, and no control calls it.

## Decision

### 1. No Staged Frame Model

The workspace model carries no operation that no production path calls. Delete these items, their tests, and the guard text that requires them:

- frame add and remove: `add_frame`, `add_frame_state`, `remove_frame`, `next_frame_id`, and the error `LastFrameRemoval`
- detach and dock: `request_detach`, `request_dock`, `frame_detach_eligibility`, `FrameDetachEligibility`, `FrameDockTarget`, `WorkspaceFrameKind::detach_eligibility`, and the errors `DetachDeferred`, `DockDeferred` and `NotDetachable`
- the other test-only model items of task 002 that only these operations use

Layout persistence (ADR 0046 Invariant 7) stays where production code uses it.

### 2. No Reserved Slots

Delete the `WorkspaceSlots` builders `queue_now_playing`, `detail_filter_chip_strip` and `on_detail_filter_select`, with their tests.
A slot is added when a screen fills it.

### 3. Forward Completes Invariant 2

The frame chrome gets a Forward control adjacent to Back, with a keyboard shortcut. Its availability is typed, and it is unavailable when the forward history is empty.
[ADR 0046 task 015](../tasks/adr-0046-task-015-forward-navigation.md) owns it. `go_forward` and `CannotNavigateForward` then have a production caller.

### 4. A Later Feature Starts From A Decision

Multi-frame commands, detach to a second window, and a filter strip for the Detail frame each need a new ADR before any code. Git keeps the deleted model as reference.

## Consequences

- No guard protects code that no person can reach.
- ADR 0046 task 013 stays Deferred. Its blocker stays: no frame has its own content owner.
- A person who restores the model from git must also restore its tests and decide its UI in the same change.

## Alternatives Considered

- Keep the model in test builds. Rejected: tests and guards then protect code that no person can reach.
- Wire frame add and remove, and detach. Rejected: the fixed structure of ADR 0060 has no workflow that needs a second free frame, and ADR 0046 already rejected a second OS window.
- Delete Forward too. Rejected: Back without Forward breaks a history that the app already keeps. The HIG and common desktop apps pair them.

## Verification

Mechanical, phrased at the owning layer:

- `src/view_models/workspace/` declares none of the items of Decision 1.
- `WorkspaceSlots` declares none of the builders of Decision 2.
- A guard named for this ADR fails when one of those names returns in `src/`.
- Task 015 tests prove the Forward availability and the shortcut.

Visual, for the operator after the visual pause ends:

- The frame chrome shows Forward adjacent to Back. Forward is unavailable until the operator goes Back.
