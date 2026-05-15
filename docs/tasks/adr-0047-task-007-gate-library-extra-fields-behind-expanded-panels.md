# ADR 0047 Task 007: Gate Library-Extra Fields Behind Expanded Panels

Status: Implemented - 2026-05-14.

## Goal

Gate Library-only inspector fields (local file path, ingest
timestamps, ID3 frame groups, format warnings, MusicBrainz match
detail) so they render only when the corresponding panel is expanded
AND the track is downloaded.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-002-inspector-panel-state-vm.md`
- `docs/tasks/adr-0047-task-006-disable-compare-musicbrainz-on-undownloaded.md`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/ui/shells/library/track_detail_metadata.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend, db, playback
- Compare ID3 / MusicBrainz business logic (only render gating
  changes here)

## Constraints

- Library-extra fields render only when:
  - `inspector_expanded_panels.contains(InspectorPanelKind::*)` for
    the relevant kind, AND
  - The track is downloaded (`compare_id3_enabled` /
    `musicbrainz_enabled` returns `true`).
- Clicking the (enabled) control toggles panel expansion.
- Closing the panel returns the inspector to compact view.
- Compact view fields stay: title, artist, album, duration, release
  date, contributors, value routes, description, image.
- Non-downloaded items cannot reveal the gated groups (controls are
  disabled).

## Implementation Steps

1. Identify the Library-extra field render blocks in
   `track_detail_metadata.rs`. Group them by `InspectorPanelKind`.
2. Wrap each block in a guard: render only when the panel kind is
   in `inspector_expanded_panels`.
3. Wire the Compare ID3 / MusicBrainz button click handlers to
   toggle panel expansion in the inspector VM.
4. Ensure the disabled controls from task 006 cannot reach the
   click handler (no handler attached when disabled).
5. Architecture guard: assert each Library-extra block reads
   `inspector_expanded_panels` before rendering.

## Acceptance Criteria

- [x] Library-extra fields hidden by default.
- [x] Clicking enabled Compare ID3 / MusicBrainz toggles panel
  expansion and reveals the corresponding fields.
- [x] Compact view preserved when panel is collapsed.
- [x] Non-downloaded items never reveal gated fields.
- [x] Architecture guard locks the contract.

## Implementation Notes

- `InspectorFrame` now owns `LibraryTrackInspectorState`; renderer code
  consumes `frame.inspector_display(...)` rather than deriving panel
  visibility directly from loaded panel data.
- Compare ID3 and MusicBrainz toggles preserve loaded panel results
  while collapsed, so reopening does not discard cache state.
- `adr_0047_phase_c_inspector_rewire_uses_vm_state_and_shared_disclosures`
  guards inspector-state ownership and shell consumption.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-002-inspector-panel-state-vm.md`
- `docs/tasks/adr-0047-task-006-disable-compare-musicbrainz-on-undownloaded.md`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

Goal:
- Gate Library-extra inspector fields behind expanded-panel
  membership and download state.

Constraints:
- Compact view stays. Extra fields hidden by default.
- Disabled controls cannot toggle expansion.
- Architecture guard records the gating.

Do not touch:
- Backend, db, playback
- Compare/MusicBrainz business logic

Acceptance criteria:
- Library-extra fields hidden until panel expanded for downloaded
  tracks.
- Architecture guard enforces.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Library-extra blocks share render scope with compact-view fields
  and cannot be wrapped cleanly (signals deeper inspector refactor
  needed).
