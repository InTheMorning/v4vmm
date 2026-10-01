# Dead Code Removal Task 001: Measure And Delete Unreachable Code

Status: Ready - 2026-10-01. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

Measure each item in `src/` that the app binary cannot reach, and delete it.
After this packet, no file in `src/` has `allow(dead_code)`, and a guard keeps it so.

## Authority

- The working rule in [AGENTS.md](../../AGENTS.md): "Delete dead code. Code no composition root reaches is removed, not parked."
- [ADR 0060](../adr/0060-workflow-surface-structure.md) and its [packet 005](adr-0060-task-005-delete-parked-discover-code.md): the incident and the guard `adr_0060_discover_surface_stays_deleted`.
- [ADR 0061](../adr/0061-executable-governance.md) and the AGENTS.md rule "A guard names its class and its ADR". The guard of this packet is situational. It cites ADR 0060.

## Incident

The old Discover surface stayed in `src/` under `#![allow(dead_code)]` after ADR 0060 replaced it. No composition root constructed `SearchApp`.
On 2026-09-26 a packet aimed a change at it as though it were live (ADR 0077 packet 005). ADR 0060 packets 005 and 006 deleted about 14,200 lines on 2026-09-30.

## Recorded Facts - 2026-10-01

- `src/lib.rs` declares each module with `pub mod`. The compiler therefore treats each `pub` item as used. A plain `cargo check` cannot find this dead code.
- ADR 0060 packet 005 recorded a method that works. It uses these temporary steps:
  1. Declare each module in `src/lib.rs` as `pub(crate) mod`.
  2. Export with `pub use` only the items that `src/main.rs` uses.
  3. Remove each `allow(dead_code)`, and run `cargo check --lib`.
  4. Restore each changed file after the measurement.
- `tests/architecture_tests.rs` reads source text only. It has no `use v4vmm::` line.
- These six files have `allow(dead_code)`:

  | File | Lines | Allowance sites |
  |---|---|---|
  | `src/library.rs` | 214 | 208 |
  | `src/library/app_impl.rs` | 4,223 | 598, 608, 2872 |
  | `src/view_models/library.rs` | 8,364 | 301, 2477, 2482 |
  | `src/view_models/paged_playlist_detail.rs` | 266 | 18, a whole-file allowance |
  | `src/view_models/paged_feed_detail.rs` | 240 | 18, a whole-file allowance |
  | `src/presentation/gpui_vm_bridge.rs` | 98 | 85 |

- ADR 0060 packets 005 and 006 recorded these unreachable items outside their scope:
  - `FeedVm::scalar_detail_entries` in `src/view_models/feed.rs`. It still shows an old-meaning "Release Date".
  - `TrackVm::play_url` and its private helpers `primary_source_enclosure_url`, `first_source_enclosure_url` and `nonempty_url` in `src/view_models/track.rs`.
  - `lookup_musicbrainz_track` and `download_and_compare_track` in `src/subscribe_service.rs`.
- The first measurement of packet 005 gave 482 dead-code warnings across the crate, for example an unused `Button::Lg` variant. That measurement also had the Discover code, which is now deleted.

## Required Changes

### 1. Measure

- Run the measurement of "Recorded Facts" over the whole crate.
- Record the unreachable list in this packet, grouped by file, with an approximate line count for each group. Record the method.
- For each `allow(dead_code)` site, record what it covers: an unreachable item, an item that only tests use, or a live item that the compiler cannot see as used.

### 2. Size Gate

- When the unreachable list is 3,000 lines or fewer, continue with section 3.
- When it is more than 3,000 lines, stop after section 1. Report the list and wait for a division of the work.

### 3. Delete

- Delete each unreachable item. Delete each test that tests only deleted code, and each guard that names only deleted code. A guard that also covers live code is changed, and keeps its ADR citation. Record each one.
- For an item that only tests use, move it into a `#[cfg(test)]` module, or delete it with its tests when it tests nothing live.
- For a live item that the compiler cannot see as used, remove the allowance and record the reason that no warning appears.
- Remove each `allow(dead_code)` in `src/`.

### 4. Guard

- Change `adr_0060_discover_surface_stays_deleted`, or add a guard named for ADR 0060, so that it fails when any file in `src/` has `allow(dead_code)`.
- The failure message names ADR 0060 and the fix: delete the unreachable item, or move a test-only item into `#[cfg(test)]`.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| RDC-01 | The packet records the method and the unreachable list before deletion |
| RDC-02 | No file in `src/` has `allow(dead_code)` |
| RDC-03 | A repeat of the measurement lists no unreachable item |
| RDC-04 | The guard fails for a sample source with `#[allow(dead_code)]` and names ADR 0060 |
| RDC-05 | `FeedVm::scalar_detail_entries`, `TrackVm::play_url`, `lookup_musicbrainz_track` and `download_and_compare_track` are deleted, or the packet names a live caller |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the Library list, an album page, a playlist page and a track page work as before, with paging.
- V2: Music search, the Index pages, the publisher page and the Show section work as before.

## Exclusions

- No change of behavior on a live screen.
- No change to the database schema or to the migration registry.
- No change to a `pub` item that `src/main.rs` uses.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- The ADR 0060 packet 005 and 006 documents: the measurement method and their findings.
- `src/lib.rs`, `src/main.rs`, and each file of "Recorded Facts".
- `tests/architecture_tests.rs`: `adr_0060_discover_surface_stays_deleted`, and each guard that names a deleted item.

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
- This packet: `docs/tasks/dead-code-removal-task-001-measure-and-delete-unreachable-code.md`
- The ADR 0060 packet 005 and 006 documents
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": measure, apply the size gate, delete, and guard.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Record the unreachable list in this packet before you delete an item.
- Keep a copy of each file that the measurement changes, and restore it after the measurement.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The behavior of a live screen.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case RDC-01 to RDC-05 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 and V2.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The unreachable list is more than 3,000 lines.
- A live screen reads an item of the unreachable list.
- An `allow(dead_code)` covers a live item, and its removal gives a warning that no change in this packet can remove.
- A change needs a file in "Do not touch".
