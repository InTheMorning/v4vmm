# ADR 0060 Task 005: Delete The Parked Discover Code

Status: Ready - 2026-09-30. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

Delete each item of the old Discover surface that the app binary cannot reach. Keep each item that a live screen uses.
The `#![allow(dead_code)]` of `src/discover.rs` is then unnecessary, and it goes.

## Authority

- [ADR 0060](../adr/0060-workflow-surface-structure.md): Music replaces the Discover surface.
- [ADR 0023](../adr/0023-design-system-and-view-models.md): the legacy discover screen migrates to shared view models.
- The working rule in [AGENTS.md](../../AGENTS.md): "Delete dead code. Code no composition root reaches is removed, not parked."
- The [ADR 0077 phase plan](../plans/adr-0077-publisher-artist-phase-plan.md#follow-up-findings) records the finding.

## Recorded Facts - 2026-09-30

- `src/discover.rs` has `#![allow(dead_code)]`. `SearchApp` has a `new` function in `src/discover/app_impl.rs`, and no code in `src/` calls it.
- `src/lib.rs` declares each module with `pub mod`. The compiler therefore treats each `pub` item as used. A check with the allowance removed lists only nine dead items, and that list is not the full dead set.
- Live screens import from `crate::discover`: `src/ui/shells/track.rs` imports `render_play_icon_button_with_id`, `render_track_download_button` and `SearchApp`, and `src/ui/shells/feed.rs` imports from it too.
  Fifteen files under `src/ui/shells/discover/` import `SearchApp`.
- The Index search keeps two parked paths in `src/application/queries/search.rs`: `fetch_search_batch`, `fetch_partitioned_search_batch` and `fetch_artist_search_batch` reach only `FetchDiscoverSearchResults`.
  They still pass the undeclared `fuzzy` parameter to `api::Client::search`.
- `apply_pending_id3_edits` in `src/discover/app_impl.rs` has no live caller (ADR 0080 packet 002).
- Other files with `allow(dead_code)`: `src/library.rs`, `src/library/app_impl.rs`, `src/view_models/library.rs`, `src/view_models/paged_playlist_detail.rs`, `src/view_models/paged_feed_detail.rs`, `src/presentation/gpui_vm_bridge.rs` and `src/ui/shells/discover/track_inspector_metadata_test_helpers.rs`.

## Required Changes

### 1. Find The Reachable Set

- Measure what the binary reaches. One method: in a temporary change, declare each module in `src/lib.rs` as `pub(crate) mod`, and export with `pub use` only the items that `src/main.rs` and `tests/architecture_tests.rs` use.
  Then `cargo check --lib` lists each unreachable item. Restore `src/lib.rs` after the measurement.
- Record the list in the packet, grouped by file. Record the method that you used.

### 2. Delete The Parked Discover Code

- Delete each unreachable item in `src/discover.rs`, `src/discover/`, `src/ui/shells/discover/`, `src/view_models/search/`, and the parked query paths of `src/application/queries/search.rs`.
- A live screen can import an item from `crate::discover` only because `SearchApp` is there. Move that item out of `crate::discover`. Keep its behavior equal.
- Delete the `fuzzy` parameter of `api::Client::search` when no caller passes `true`.
- Delete `#![allow(dead_code)]` from `src/discover.rs`, or delete `src/discover.rs` when nothing stays.
- Delete each test that tests only deleted code. Delete each guard that names only deleted code. Record each one.

### 3. Report The Other Allowances

- For each other file with `allow(dead_code)`, record the unreachable items that the measurement finds. Do not delete them in this packet.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R60-5-01 | The packet records the measurement method and the unreachable list before deletion |
| R60-5-02 | No file under `src/discover*` and `src/ui/shells/discover/` has `allow(dead_code)` |
| R60-5-03 | A repeat of the measurement lists no unreachable item in the deleted scope |
| R60-5-04 | `api::Client::search` has no `fuzzy` parameter, or the packet names the live caller that passes `true` |
| R60-5-05 | A guard, named for ADR 0060, fails when `src/discover.rs` or a file under `src/ui/shells/discover/` has `allow(dead_code)` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: Music search, the Index feed and track pages, the Library track page and the album page work as before.
- V2: the metadata compare grid on a Library track page works as before, with its MusicBrainz column.

## Exclusions

- No change of behavior on a live screen.
- No deletion outside the discover scope. Section 3 only records.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/lib.rs`, `src/main.rs`, `src/discover.rs`, `src/discover/`, `src/ui/shells/discover/`, `src/ui/shells/track.rs`, `src/ui/shells/feed.rs`, `src/view_models/search/`, `src/application/queries/search.rs`, `src/api.rs`.
- `tests/architecture_tests.rs`: each guard that names a discover path.

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
It states the needed state, what counts as wrong, and the cleanup. The check only reads pages.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0060-task-005-delete-parked-discover-code.md`
- ADR 0060 and ADR 0023
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": measure the reachable set, delete the parked discover code, and record the other allowances.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Record the unreachable list in the packet before you delete an item.
- Restore `src/lib.rs` after the measurement. The final diff has no change to its `pub mod` lines.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The behavior of a live screen.
- Dead code outside the discover scope.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R60-5-01 to R60-5-05 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 and V2.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A live screen needs `SearchApp` itself, not only a function in its module.
- The unreachable list holds more than 5,000 lines in the discover scope. Report the list, and wait for a division of the work.
- A change needs a file in "Do not touch".
