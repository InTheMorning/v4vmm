# ADR 0060 Task 001: Remove The Broadcast Frame

Status: Ready - 2026-09-07.

## Goal

Delete the `Broadcast` workspace frame, its view model, its shell, its adapter,
and the guards that assert them. Record the patterns worth keeping first.

This task removes code. It adds no surface. `Show` arrives in a later packet.

## Files To Inspect

- `docs/adr/0060-workflow-surface-structure.md`
- `docs/adr/0061-executable-governance.md`
- `docs/tasks/adr-0059-task-005-broadcast-page-vm.md`
- `docs/tasks/adr-0059-task-006-broadcast-frame-kind.md`
- `docs/tasks/adr-0059-task-007-broadcast-shell-and-adapter.md`
- `src/view_models/workspace/frame.rs`
- `src/app.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

Deleted:

- `src/view_models/broadcast.rs`
- `src/ui/shells/broadcast.rs`
- `src/app/broadcast.rs`

Edited:

- `src/view_models/workspace/frame.rs`
- `src/view_models/mod.rs`
- `src/app.rs`
- `src/ui/shells/mod.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`
- `docs/plans/broadcast-chain-delivery-order.md`

New:

- `docs/notes/2026-09-07-broadcast-shell-patterns.md`

## Do Not Touch

The service layer and the runtime stay. They are ADR 0059 backend work that ADR
0060 does not affect.

- `src/broadcast/**`
- `src/runtime/broadcast_observation.rs`
- `src/api.rs` live item create and read paths
- The `v4vmm broadcast` CLI commands
- `src/view_models/queue_now_playing.rs` and its shell

## Constraints

- **Record the patterns before deleting.** ADR 0060 keeps the feed-tag
  requirement, so `render_feed_tag` must survive as a documented pattern even
  though its file does not.
- Delete the situational guards with the code. A guard that asserts a
  superseded rule is a defect of the same weight as a missing guard. ADR 0061.
- **A stored workspace layout that names the removed frame must still load.**
  ADR 0046 task 012 persists frame layout in `config.toml`. An operator config
  written before this change must fall back to the default layout, not fail.
- Do not add a replacement surface. `Show` is a later packet.
- Do not weaken the `QueueNowPlaying` frame. It keeps its name and meaning
  until the `Show` packet moves the queue.

## Implementation Steps

1. Write `docs/notes/2026-09-07-broadcast-shell-patterns.md` before deleting
   anything. Record, with code excerpts:
   - `render_feed_tag`, the `podcast:liveValue` element and its copy action
   - the section composition helper and how the three sections were laid out
   - the slots builder pattern for callbacks
   - the empty-state handling for a section that cannot act
   Say which ADR 0060 rule each pattern serves.
2. Delete `src/ui/shells/broadcast.rs` and its declaration in
   `src/ui/shells/mod.rs`.
3. Delete `src/app/broadcast.rs` and its declaration and use in `src/app.rs`.
4. Remove the broadcast frame build and mount from `src/app.rs`, including
   `WORKSPACE_BROADCAST_FRAME_ID` and the default-layout insertion.
5. Remove the broadcast render path from `src/ui/shells/workspace.rs`.
6. Delete `src/view_models/broadcast.rs` and its declaration in
   `src/view_models/mod.rs`.
7. Remove `WorkspaceFrameKind::Broadcast` and `FrameSearchScope::BroadcastRows`
   from `src/view_models/workspace/frame.rs`, and fix every match the compiler
   reports.
8. Delete the ADR 0059 task 006 and task 007 guards from
   `tests/architecture_tests.rs`.
9. Add one guard for the ADR 0060 invariant: no workspace frame kind is named
   for broadcasting. Mark it situational and cite ADR 0060 in its failure
   message.
10. Add or confirm a test that a stored layout naming an unknown frame kind
    falls back to the default layout.
11. Update the delivery order table: mark 005, 006 and 007 removed rather than
    superseded.

## Acceptance Criteria

- The three files are deleted and no reference to them remains.
- `WorkspaceFrameKind` has four variants and no broadcasting name.
- The ADR 0059 task 006 and 007 guards are gone.
- One new guard asserts the ADR 0060 invariant, and its message cites ADR 0060.
- A `config.toml` written before this change still loads.
- The service layer, the observation actor, and the CLI are untouched.
- The patterns note exists and covers all four items in step 1.
- The guard suite is smaller than before this change.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo run` to confirm the app starts with the frame gone

## Expected Final Report Format

1. Files deleted
2. Files changed
3. Guards removed, and the line count before and after
4. Tests run
5. Deviations from task
6. Unresolved concerns

## Escalation Triggers

- The service layer or the observation actor turns out to depend on the deleted
  view model. Report it. The dependency direction is wrong and needs a fix, not
  a workaround.
- Removing the frame kind breaks stored-layout loading in a way the fallback
  does not cover.
- A guard being deleted also asserts a rule that no other guard covers. Report
  which rule, so it can be re-homed rather than lost.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture. Add no new surface.

Read:
- `docs/adr/0060-workflow-surface-structure.md`
- `docs/adr/0061-executable-governance.md`
- `src/view_models/workspace/frame.rs`, `src/app.rs`,
  `src/ui/shells/workspace.rs`, `tests/architecture_tests.rs`

Goal:
- Delete the `Broadcast` frame, its view model, its shell, its adapter, and the
  guards that assert them.

Constraints:
- Write the patterns note first. `render_feed_tag` must survive as documentation.
- Delete the situational guards with the code.
- A stored layout naming the removed frame must still load.
- Do not touch `src/broadcast/**`, the observation actor, the API, or the CLI.
- Do not add a replacement surface.

Acceptance criteria:
- Three files deleted, four frame kinds remain, old guards gone.
- One new guard for the ADR 0060 invariant, message cites ADR 0060.
- Older `config.toml` still loads. Guard suite is smaller.

Test commands:
- `cargo fmt -- --check`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files deleted
2. files changed
3. guards removed, line count before and after
4. tests run
5. deviations from task
6. unresolved concerns
