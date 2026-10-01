# Dead Code Removal Task 002: Expected Dead Code

Status: Ready - 2026-10-01. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

Delete each item that an `expect(dead_code)` marker hides, and the marker. Measure and delete the candidates that task 001 did not measure.
After this packet, no file in `src/` has a `dead_code` lint attribute, and a guard keeps it so.

## Authority

- The working rule in [AGENTS.md](../../AGENTS.md): "Delete dead code. Code no composition root reaches is removed, not parked. Copy any pattern worth keeping into the live surface first."
- [ADR 0060](../adr/0060-workflow-surface-structure.md) and the guard `adr_0060_discover_surface_stays_deleted`, which task 001 extended to `allow(dead_code)`.
- [Dead code removal task 001](dead-code-removal-task-001-measure-and-delete-unreachable-code.md): the method, and the findings of its orchestrator review.

## Incident

Task 001 removed each `allow(dead_code)`. Its first version then added `expect(dead_code)` markers on dead items. An `expect` marker hides dead code as `allow` did.
35 such markers were in the tree before task 001.

## Recorded Facts - 2026-10-01

These markers stay in `src/`. Each line gives the file, the line, the reason text and the item.

| File and line | Reason text | Item |
|---|---|---|
| `src/app.rs:1339` | ADR 0046 Task 007 architecture guard keeps the legacy fallback constructor visible | `transitional_workspace_layout` |
| `src/ui/composites/frame_shell.rs:81`, `:91`, `:101` | deferred frame action wiring consumes this slot | `on_forward`, `on_close`, `on_menu_select` |
| `src/ui/composites/frame_shell.rs:139` | ADR 0046 Task 008+ wires per-frame appearance | `appearance` |
| `src/ui/shells/workspace.rs:71` | ADR 0046 Task 008+ wires source-list content | `source_list` |
| `src/ui/shells/workspace.rs:85`, `:132`, `:160` | Stage 5: Detail frame reserved for future workflows | `detail`, `detail_filter_chip_strip`, `on_detail_filter_select` |
| `src/ui/shells/workspace.rs:95` | ADR 0060 task 002 keeps the QueueNowPlaying frame slot until the frame kind is removed | `queue_now_playing` |
| `src/library/app_impl.rs:649`, `:2158` | ADR 0046 frame chrome back controls will consume this when navigation buttons are wired | `frame_back_destination`, `navigate_back_to_frame_history` |
| `src/view_models/search_results/mod.rs:11` | ADR 0048 routing consumes these VM contracts from GPUI renderers | a whole-module marker |
| `src/view_models/workspace/nav.rs:11`, `mod.rs:12`, `frame.rs:7` | workspace contracts land before every frame action is wired | whole-module markers |
| `src/view_models/queue_now_playing.rs:13` | active-frame search dispatch lands queue filter VM state before UI routing consumes it | a whole-module marker |
| `src/view_models/playlist_detail.rs:48`, `:78`, `:89` | a focused state accessor, and active-frame search dispatch text state | `is_empty`, `text_filter`, `set_text_filter` |
| `src/view_models/library.rs:1165`, `:1200` | ADR 0062 task 002 release rows before expansion controls | `expansion`, `from_artist_result` |
| `src/view_models/library.rs:2471`, `:2965`, `:2977`, `:3390` | a state accessor that only tests read | `content_text_filter`, `status`, `search_query`, `is_renaming_playlist` |
| `src/view_models/library.rs:2616`, `:4032` | ADR 0047 Phase B before the loader or Phase C | `set_saved_searches`, `description_state` |
| `src/view_models/library.rs:2853`, `:2909`, `:2988`, `:3448`, `:3460` | future screen, operation or tree rendering | `selected_playlist_id`, `clear_busy_track`, `set_source_text_filter`, `is_artist_expanded`, `is_album_expanded` |
| `src/app/resize.rs:61`, `:91` | called via closure in render_workspace_content | `set_content_pane_width`, `is_content_pane_resizing` |

Status of the named ADRs on 2026-10-01: ADR 0046, ADR 0047 and ADR 0048 are Implemented. ADR 0062 is Accepted, and its broader mixed-row search and expansion scope still needs an evidence review.

Task 001 left these candidates unmeasured: `subscribe_track`, `subscribe_feed` and related functions, three functions in `src/library_service.rs`, and `LibraryApp::new`.

The guard `active_frame_search_dispatch_phase_1_vm_contracts_are_owned_by_view_models` pins the text of `set_content_text_filter` and `ContentListPageVm::set_text_filter`. Production code calls neither. Task 001 moved both into `#[cfg(test)]` blocks.

An `expect(dead_code)` on an item that is used fails the build. So each marked item is dead in the present build. The reason "called via closure" in `src/app/resize.rs` does not match that fact.

## Required Changes

### 1. Measure

- For each marker, confirm the dead item with the compiler and with a search of `src/`. Record the result in the packet.
- Measure the unmeasured candidates of task 001 with its method. Record the list.
- Trace each whole-module marker. List each item that it hides and that no code uses.

### 2. Size Gate

- When the total deletion is 2,000 lines or fewer, continue.
- When it is more than 2,000 lines, stop after section 1, report the list, and wait for a division.

### 3. Delete

- Delete each dead item and its marker. Delete each test that tests only deleted code.
- Delete the guard `active_frame_search_dispatch_phase_1_vm_contracts_are_owned_by_view_models`, or narrow it to live code, and record the choice. Delete each guard that keeps only a dead item visible, for example the ADR 0046 Task 007 guard of `transitional_workspace_layout`.
- An item that a live test needs as a helper moves into `#[cfg(test)]`. An item that only its own tests use is deleted with them.
- For a whole-module marker, delete the dead items, then delete the marker.
- Record, for each named ADR, the unfinished scope that lost its scaffolding: ADR 0046 frame chrome buttons and per-frame appearance, ADR 0047 saved searches and description state, ADR 0048 active-frame text filters, and ADR 0062 release-row expansion. Git keeps the code.

### 4. Guard

- Extend the ADR 0060 guard so that it fails on any `dead_code` lint attribute in `src/`: `allow`, `expect` and `cfg_attr` forms.
- The failure message names ADR 0060 and the fix: delete the dead item, or move a test-only helper into `#[cfg(test)]`.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| RDC2-01 | The packet records the measurement and the list before deletion |
| RDC2-02 | No file in `src/` has a `dead_code` lint attribute |
| RDC2-03 | The guard fails for sample sources with `#[expect(dead_code)]`, `#![expect(dead_code)]` and `#[cfg_attr(not(test), expect(dead_code))]`, and names ADR 0060 |
| RDC2-04 | Each candidate of task 001 is deleted, or the packet names its live caller |
| RDC2-05 | The packet lists the unfinished ADR scope that lost its scaffolding |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the workspace frames, the breadcrumb, Back and Forward, and the pane resize work as before.
- V2: the Library list, a playlist page with rename, Music search and the Show queue work as before.

## Exclusions

- No new feature, and no change of behavior on a live screen.
- No ADR text change. The orchestrator records the lost scaffolding in the ADR status after review.
- No database change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- The task 001 document.
- Each file of "Recorded Facts", `src/subscribe_service.rs`, `src/library_service.rs`, `src/library.rs`.
- `tests/architecture_tests.rs`: `adr_0060_discover_surface_stays_deleted`, `active_frame_search_dispatch_phase_1_vm_contracts_are_owned_by_view_models`, and each guard that names a deleted item.

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

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 and V2.
It states the needed state, what counts as wrong, and the cleanup. The check only reads pages and resizes panes.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/dead-code-removal-task-002-expected-dead-code.md`
- The task 001 document
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": measure, apply the size gate, delete, and guard.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Record the list in this packet before you delete an item.
- Restore each temporary measurement change by hand. Never run `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The behavior of a live screen.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case RDC2-01 to RDC2-05 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 and V2.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The total deletion is more than 2,000 lines.
- A live screen uses an item that a marker hides.
- A change needs a file in "Do not touch".
