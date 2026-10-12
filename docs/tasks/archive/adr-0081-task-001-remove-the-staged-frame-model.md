# ADR 0081 Task 001: Remove The Staged Frame Model

Status: Complete - 2026-10-11. The mechanical checks are Green. This packet has no visual gate.
Task 015 and dead code removal task 002 own the visual checks of the frame chrome.

## Goal

Delete the frame add, remove, detach and dock model, and the two reserved slots.
Also delete their tests and the guard text that requires them.
After this packet, no `#[cfg(test)]` block keeps a model operation that no production path calls.

## Authority

- [ADR 0081](../../adr/0081-remove-the-staged-frame-model.md) Decisions 1, 2 and 4.
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

## Facts From ADR 0084 Tasks 001 To 003 - 2026-10-11

The orchestrator checked these facts against the code on 2026-10-11. They correct the facts of 2026-10-02.

- Task 015 is complete. `go_forward`, `CannotNavigateForward` and `FrameShellSlots::on_forward` have production callers. Required Change 1 is met.
- ADR 0084 task 001 made `WorkspaceLayout::default_content_frame_id` test-only, because the Library was its one production caller. R81-05 includes it. Tests can use the content-list frame id of `WorkspaceLayout::default_layout()` in its place.
- `src/view_models/workspace/nav.rs` holds no `#[cfg(test)]` block. The guard `adr_0046_forward_go_forward_and_on_forward_have_production_callers` forbids one. Keep it so.
- `default_navigation_entry` in `src/view_models/workspace/mod.rs` gives the `Detail` frame a placeholder `FrameNavigationEntry::Track` key (ADR 0084 task 003). Do not change it. ADR 0084 packet 004 changes the frames.
- These blocks were test-only on 2026-10-11. Examine each one against Decisions 1 and 2 of ADR 0081:
  - `FrameDetachEligibility`, `FrameDockTarget` and two more blocks in `frame.rs`
  - two blocks in `chrome.rs`
  - the `WorkspaceModelError` variants and their `Display` lines in `mod.rs`, and one block near line 470 of `mod.rs`
  - `WorkspaceSlots::queue_now_playing` in `src/ui/shells/workspace.rs`

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

- [Agent rules](../../../AGENTS.md) and the [source map](../../architecture/source-map.md).
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

## Implementation Result - 2026-10-11

Each Required Change is done.

### Files Changed

- `src/view_models/workspace/frame.rs`: deletes `FrameDetachEligibility`, `FrameDockTarget`
  (and its `label` method), and `WorkspaceFrameKind::detach_eligibility`.
- `src/view_models/workspace/mod.rs`: deletes the `WorkspaceModelError` variants
  `LastFrameRemoval`, `DetachDeferred`, `DockDeferred`, and `NotDetachable`, and their `Display`
  lines. Deletes `add_frame`, `add_frame_state`, `remove_frame`, `request_detach`,
  `request_dock`, `frame_detach_eligibility`, `next_frame_id`, and `default_content_frame_id`.
  Keeps `default_detail_frame_id`, `empty`, and `focused_frame` in a narrower `#[cfg(test)]`
  block, with a new doc comment that names each one's test caller.
- `src/view_models/workspace/tests.rs`: deletes seven tests of the removed model operations.
  Removes the `remove_frame` assertion from `invalid_layout_mutations_return_errors`. Rewrites
  `workspace_layout_config_round_trips` to build its extra frame through `WorkspaceLayout::new`,
  not `add_frame`. Rewrites `adr_0046_forward_pop_nav_forward_restores_entry_back_left` to find
  the content-list frame id from `default_layout()`, not `default_content_frame_id`.
- `src/ui/shells/workspace.rs`: deletes the `WorkspaceSlots` builders `queue_now_playing`,
  `detail_filter_chip_strip`, and `on_detail_filter_select`, and the test module that existed
  only to call them. Keeps their three backing fields and the per-frame-kind dispatch arms that
  read them (see "Items Kept, With Their Production Caller").
- `src/config.rs`: rewrites `save_workspace_layout_persists_without_dropping_existing_values` to
  focus an existing default-layout frame, not `add_frame`.
- `tests/architecture_tests.rs`: see "Guards Deleted, Narrowed, Or Added".

### Items Deleted (ADR 0081 Decisions 1 And 2)

- Frame add and remove: `add_frame`, `add_frame_state`, `remove_frame`, `next_frame_id`, and the
  error `LastFrameRemoval`.
- Detach and dock: `request_detach`, `request_dock`, `frame_detach_eligibility`,
  `FrameDetachEligibility`, `FrameDockTarget`, `WorkspaceFrameKind::detach_eligibility`, and the
  errors `DetachDeferred`, `DockDeferred`, and `NotDetachable`.
- The seven dedicated tests of the deleted model operations in
  `src/view_models/workspace/tests.rs`.
- The `WorkspaceSlots` builders `queue_now_playing`, `detail_filter_chip_strip`, and
  `on_detail_filter_select`, and the three tests task 002 added only to call them.
- `WorkspaceLayout::default_content_frame_id`. ADR 0081 Decision 1 does not name it. The
  2026-10-11 facts flag it as included in R81-05: ADR 0084 task 001 already made it test-only,
  and it then had one remaining test caller. That caller,
  `adr_0046_forward_pop_nav_forward_restores_entry_back_left`, now finds the content-list frame
  id from `WorkspaceLayout::default_layout()` directly.

### Items Kept, With Their Production Caller (R81-04)

These items have no production caller. Each one still backs a test of other, live behavior, or
another guard still requires it:

- `WorkspaceLayout::default_detail_frame_id` (`src/view_models/workspace/mod.rs`). The guard
  `adr_0047_task_015_search_submit_and_saved_search_commands` (`tests/architecture_tests.rs`)
  requires it to exist. It also backs `replace_nav_preserves_full_history_for_visible_layout_projection`,
  a test of the live `replace_nav` layout-projection path.
- `WorkspaceLayout::empty` (`src/view_models/workspace/mod.rs`). It backs
  `empty_layout_has_no_focus_and_rejects_focus_mutation`, a test of the live `focus_frame`
  empty-layout error path. The guard `adr_0060_queue_is_not_mounted_in_curation_workspace` also
  uses its signature text as a scan boundary marker.
- `WorkspaceLayout::focused_frame` (`src/view_models/workspace/mod.rs`). It backs five tests of
  the live `WorkspaceLayout::new`, `default_layout`, and `focus_frame` focus invariant, in
  `src/view_models/workspace/tests.rs`. It also backs two tests of the live layout-persistence
  load path in `src/config.rs` (`load_config_parses_workspace_layout`,
  `load_config_accepts_workspace_layout_with_queue_from_before_show_mount`).
- The `WorkspaceSlots` field `queue_now_playing` and its read in `take()`
  (`src/ui/shells/workspace.rs`). `WorkspaceShell::render` reads it for every frame of kind
  `WorkspaceFrameKind::QueueNowPlaying`, including one a pre-ADR-0060 persisted config can still
  carry through the live `WorkspaceLayout::from_config` path.
- The `WorkspaceSlots` fields `detail_filter_chip_strip` and `on_detail_filter_select`, and their
  reads in `filter_chip_strip_for` and `filter_select_handler_for` (`src/ui/shells/workspace.rs`).
  `WorkspaceShell::render` reads each one for every `WorkspaceFrameKind::Detail` frame, the frame
  Music renders at all times.
- `WorkspaceFrameState::with_subtitle` and `with_status` (`src/view_models/workspace/frame.rs`),
  and `LibraryFilterControlTreatment::line_through` and `LibraryFilterControlDisplay::state_displays`
  (`src/view_models/workspace/chrome.rs`). ADR 0081 Decision 1 and Decision 2 name none of these
  four. Each one still backs only its own dedicated test today, unchanged by this packet.

### Guards Deleted, Narrowed, Or Added

- Deleted: `workspace_frame_phase_6_detach_dock_model_only_contract` (ADR 0046 Task 014). This
  packet deletes every item the guard required.
- Narrowed, with the ADR citation kept:
  - `workspace_frame_phase_5_layout_persistence_contract` (ADR 0046 Task 012): drops its
    `LastFrameRemoval`, `add_frame`, and `remove_frame` required text. Keeps the live
    `WorkspaceLayoutConfig`, `WorkspaceFrameConfig`, `to_config`, and `from_config` requirements.
  - `workspace_frame_phase_5_multi_frame_commands_are_deferred_until_content_frames_exist`
    (ADR 0046 Task 013): drops the two forbidden-text checks that named the deleted
    `add_frame`/`remove_frame` calls. Keeps the live action-menu, context-menu, and
    visible-layout requirements.
  - `adr_0060_queue_is_not_mounted_in_curation_workspace` (ADR 0060 Task 002): drops the
    required `queue_now_playing(` builder text, and the now-unused `workspace_source` binding.
    Keeps the rest of the guard, which still checks that the curation workspace mounts no queue
    content.
  - `adr_0047_task_014_search_results_inspector_shell_contract` (ADR 0047 Task 014): drops the
    required `detail_filter_chip_strip(`/`on_detail_filter_select(` builder text. Keeps the
    field and dispatch-arm requirements (see "Items Kept, With Their Production Caller"), and
    every other live requirement.
- Added: `adr_0081_staged_frame_model_stays_deleted` (Situational ADR 0081), with its helper
  `adr_0081_staged_frame_model_violations` and its sample-string test
  `adr_0081_staged_frame_model_guard_catches_a_reintroduced_name` (R81-03). The guard fails when
  one of the ADR 0081 Decision 1 or 2 names returns in `src/`. Its message names ADR 0081 and
  the fix.

### Behavior Changed

None expected. Every deleted item had no production caller before this packet. The kept fields
and dispatch arms already returned `None` before this packet, because no production code ever
called their builders.

### Checks

| Command | Result |
|---|---|
| `cargo test` | Green, 1,863 unit tests and 307 guards |
| `cargo test --test architecture_tests` | Green, 307 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations

1. ADR 0081 Decision 1 does not name `WorkspaceLayout::default_content_frame_id`. This packet
   deletes it anyway. The 2026-10-11 facts flag it as included in R81-05 and name its one
   remaining test caller. This packet rewrites that one caller in place.
2. `workspace_layout_config_round_trips` and
   `save_workspace_layout_persists_without_dropping_existing_values` (`src/config.rs`) each
   tested a live round trip (`to_config`/`from_config`, or `save_workspace_layout`) through a
   layout that `add_frame` built. This packet rewrites each one to build the same layout shape
   through `WorkspaceLayout::new` or `focus_frame`. The live round-trip coverage survives the
   deletion of `add_frame`.
3. This packet keeps the `WorkspaceSlots` fields `queue_now_playing`, `detail_filter_chip_strip`,
   and `on_detail_filter_select`, and their per-frame-kind dispatch arms, rather than deleting
   them with their builders. Each field still has one frame kind that reads it in
   `WorkspaceShell::render`. Required Change 3's "delete a backing field when no frame kind
   reads it" does not apply to any of the three for that reason.

### Unresolved Concerns

- None found in this packet's scope.

## Orchestrator Review - 2026-10-11

The orchestrator reviewed the diff against ADR 0081, this packet and the facts of 2026-10-11. The deletions, the guard changes and the new guard keep to the packet. Two parts did not meet ADR 0081, and the orchestrator corrected them in review:

1. `WorkspaceLayout::default_detail_frame_id`, `empty` and `focused_frame` stayed in a `#[cfg(test)]` block. Decision 1 says that the model carries no operation that no production path calls, and R81-05 forbids such a block. The three functions are deleted. The tests use local helpers that read the live accessors `frames` and `focused_frame_id`. The tests of `src/config.rs` do the same. The ADR 0081 guard now also blocks `fn default_detail_frame_id(`, `fn default_content_frame_id(` and `fn focused_frame(`.
2. Five `WorkspaceSlots` fields stayed with no builder, so production never filled them: `source_list`, `detail`, `queue_now_playing`, `detail_filter_chip_strip` and `on_detail_filter_select`. Decision 2 says that a slot is added when a screen fills it. The five fields are deleted, and each frame kind with no screen gets `None`. ADR 0084 packet 004 adds the sidebar and detail slots when its screens fill them.

Two guards changed with the corrections:

- `adr_0047_task_014_search_results_inspector_shell_contract` required the Detail filter slot. ADR 0081 Decision 4 replaces that rule, so the requirement text is deleted. The module requirement stays.
- `adr_0060_queue_is_not_mounted_in_curation_workspace` used the deleted `empty` function as an end marker. It now uses `new`.

After the corrections, each check is Green: 1,863 unit tests, 307 guards, `fmt`, `clippy -D warnings`, `check --all-targets` with no warning, and `cargo build --bin v4vmm`.

Merge recommendation: pass. ADR 0084 packet 004 can start.
