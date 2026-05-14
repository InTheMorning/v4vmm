# ADR 0047 Task 002: Inspector Panel State VM

Status: Implemented - 2026-05-14.

## Goal

Introduce `InspectorPanelKind` enum, `inspector_expanded_panels` set
on the track-inspector VM, and `compare_id3_enabled` /
`musicbrainz_enabled` predicates that gate Library-extra fields.
GPUI-free; downstream phases consume these contracts.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `src/view_models/library.rs` (track inspector VM)
- `src/view_models/entity_detail.rs` (related action-state)
- `src/library.rs` (`InspectorFrame` shape)
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Any `src/ui/*` rendering
- Backend command paths, playback, db schema

## Constraints

- GPUI-free; M-CANONICAL-DOCS on public types.
- `InspectorPanelKind` enum: `CompareId3`, `MusicBrainz`.
- `inspector_expanded_panels: BTreeSet<InspectorPanelKind>` field on
  the inspector VM or on a sibling state struct projected per render.
- Predicates are pure functions of inspector state +
  `is_downloaded` boolean drawn from the track.
- No mutation of `is_downloaded` here.

## Implementation Steps

1. Define `InspectorPanelKind` in `src/view_models/library.rs` (or
   `src/view_models/workspace.rs` if shared with non-library
   inspectors — pick one and document the choice).
2. Add `inspector_expanded_panels: BTreeSet<InspectorPanelKind>` to
   the track-inspector VM. Default empty.
3. Add `compare_id3_enabled(is_downloaded: bool) -> bool` and
   `musicbrainz_enabled(is_downloaded: bool) -> bool` predicates.
   Initial policy: both return `is_downloaded`.
4. Add helpers to toggle panel expansion: `expand_panel(kind)`,
   `collapse_panel(kind)`, `is_panel_expanded(kind) -> bool`.
5. Unit tests covering: default empty set; expand/collapse round-
   trip; predicates return `false` for `!is_downloaded`.

## Acceptance Criteria

- [ ] `InspectorPanelKind` defined and documented.
- [ ] `inspector_expanded_panels` set + helpers exist on the
  inspector VM.
- [ ] Predicates compile and return the documented values.
- [ ] Unit tests cover the boundary cases.
- [ ] No UI module changed.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test inspector
cargo test library
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `src/view_models/library.rs`
- `src/library.rs`
- `tests/architecture_tests.rs`

Goal:
- Add `InspectorPanelKind`, `inspector_expanded_panels` set,
  and `compare_id3_enabled` / `musicbrainz_enabled` predicates
  to the track-inspector VM. GPUI-free.

Constraints:
- Documented public types.
- No UI changes.

Do not touch:
- `src/ui/*`
- Backend, playback, db

Acceptance criteria:
- Types and helpers compile with docs and unit tests.
- Predicates return false when not downloaded.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test inspector`
- `cargo test library`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Track inspector VM cannot host the new set without restructuring
  `InspectorFrame` (escalate first).
