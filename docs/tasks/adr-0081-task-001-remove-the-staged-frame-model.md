# ADR 0081 Task 001: Remove The Staged Frame Model

Status: Ready - 2026-10-02. It runs after [ADR 0046 task 015](adr-0046-task-015-forward-navigation.md). Implementation has not started.
This packet has no visual gate. Task 015 and dead code removal task 002 own the visual checks of the frame chrome.

## Goal

Delete the frame add, remove, detach and dock model, and the two reserved slots.
Also delete their tests and the guard text that requires them.
After this packet, no `#[cfg(test)]` block keeps a model operation that no production path calls.

## Authority

- [ADR 0081](../adr/0081-remove-the-staged-frame-model.md) Decisions 1, 2 and 4.
- [Dead code removal task 002](dead-code-removal-task-002-expected-dead-code.md), section "Durable, deliberately staged model code".

## Recorded Facts - 2026-10-02

- Dead code removal task 002 moved these items into `#[cfg(test)]` blocks:
  - `src/view_models/workspace/mod.rs`: `WorkspaceLayout::add_frame`, `add_frame_state`, `remove_frame`, `request_detach`, `request_dock`, `frame_detach_eligibility`, `next_frame_id`, `empty`, `focused_frame`, `default_detail_frame_id`, and the `WorkspaceModelError` variants `LastFrameRemoval`, `CannotNavigateForward`, `DetachDeferred`, `DockDeferred` and `NotDetachable`.
  - `src/view_models/workspace/frame.rs`: `FrameDetachEligibility`, `FrameDockTarget`, `WorkspaceFrameKind::detach_eligibility`, `WorkspaceFrameState::with_subtitle` and `with_status`.
  - `src/view_models/workspace/nav.rs`: `FrameNavigationState::go_forward`. Task 015 moves it back into production.
  - `src/ui/shells/workspace.rs`: `WorkspaceSlots::queue_now_playing`, `detail_filter_chip_strip` and `on_detail_filter_select`.
- These guards name that text:
  - `workspace_frame_phase_5_layout_persistence_contract` (ADR 0046 task 012)
  - `workspace_frame_phase_5_multi_frame_commands_are_deferred_until_content_frames_exist` (ADR 0046 task 013)
  - `workspace_frame_phase_6_detach_dock_model_only_contract` (ADR 0046 task 014)
  - `adr_0060_queue_is_not_mounted_in_curation_workspace` (ADR 0060 task 002)
  - `adr_0047_task_014_search_results_inspector_shell_contract` (ADR 0047 task 014)
- Task 002 added three tests only to call the three slot builders in the test build.
- Layout persistence (ADR 0046 Invariant 7) serializes the workspace layout. Its live parts stay.

## Required Changes

1. Confirm that task 015 is complete: `go_forward` and `CannotNavigateForward` have a production caller.
2. Delete each item of Decision 1 of ADR 0081 that no production path calls, with its tests. Keep each item that layout persistence or another live path uses. Record each kept item with its caller.
3. Delete the three slot builders and the three tests that task 002 added. Delete a backing field when no frame kind reads it.
4. Change the five guards:
   - Delete each guard that protects only deleted code.
   - Narrow each guard that also protects live code, for example the live layout persistence. Keep its ADR citation.
   - Record each one.
5. Add a guard named for ADR 0081. It fails when one of the deleted names of Decisions 1 and 2 returns in `src/`. Its message names ADR 0081 and the fix: write a new ADR before the code.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R81-01 | `src/view_models/workspace/` declares no deleted item of ADR 0081 Decision 1 |
| R81-02 | `WorkspaceSlots` declares no builder of ADR 0081 Decision 2 |
| R81-03 | The ADR 0081 guard fails for a sample source with `fn request_detach(` and names ADR 0081 |
| R81-04 | Each kept item has a recorded production caller |
| R81-05 | No `#[cfg(test)]` block in `src/view_models/workspace/` and `src/ui/shells/workspace.rs` holds a model operation that no production path calls |

## Exclusions

- No change to the Forward control of task 015.
- No change to a live screen.
- No database change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/workspace/mod.rs`, `frame.rs`, `nav.rs`, `tests.rs`, and `src/ui/shells/workspace.rs`.
- The config loader that reads the workspace layout.
- `tests/architecture_tests.rs`: the five guards of "Recorded Facts".

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data. A config file with a saved layout still loads.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0081-task-001-remove-the-staged-frame-model.md`
- ADR 0081, and the dead code removal task 002 document
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": delete the staged model and the two slots, change the guards, and add the ADR 0081 guard.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- An old `config.toml` with a saved workspace layout must still load. Prove it with a test when a deleted item touched the layout format.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The Forward control and the key bindings of task 015.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R81-01 to R81-05 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- Task 015 is not complete.
- A deletion changes the stored layout format so that an old config fails to load.
- A change needs a file in "Do not touch".
