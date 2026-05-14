# ADR 0047 Task 006: Disable Compare ID3 + MusicBrainz on Undownloaded

Status: Proposed - 2026-05-14.

## Goal

Render the Compare ID3 and MusicBrainz controls in disabled (HIG-
dimmed + tooltip) state when the selected track is not downloaded.
Wire `compare_id3_enabled` / `musicbrainz_enabled` predicates from
task 002 into the inspector render path.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `docs/tasks/adr-0047-task-002-inspector-panel-state-vm.md`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/view_models/library.rs`
- `src/ui/primitives/button.rs`
- `src/ui/primitives/tooltip.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/ui/shells/library/track_detail_metadata.rs`
- `src/view_models/library.rs` (display contract update)
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend command paths
- `src/db.rs`
- Playback

## Constraints

- Apple HIG: disabled controls render dimmed glyph + label; no click
  handler attached; tooltip explains why (e.g., "Download track to
  enable").
- Predicates from task 002 are the single source of truth.
- Display contract carries the disabled flag + a11y label; the shell
  consumes the flag, does not recompute.
- No raw glyph/color/spacing literals; consume tokens.

## Implementation Steps

1. Add `disabled` + `tooltip_text` fields to the Compare ID3 and
   MusicBrainz control display structs (or extend the existing
   `ActionButtonDisplay` shape used here).
2. Project the disabled flag from `compare_id3_enabled` /
   `musicbrainz_enabled` predicates.
3. Update the shell to honor the disabled flag (no click handler,
   tokenized dim style) and render a tooltip.
4. Architecture guard: assert the shell consumes the disabled flag
   from the VM and does not compute `is_downloaded` locally.

## Acceptance Criteria

- [ ] Compare ID3 and MusicBrainz controls render disabled when
  `is_downloaded = false`.
- [ ] Disabled state uses tokenized dim style and shows a tooltip.
- [ ] No raw color or spacing literal added.
- [ ] Architecture guard locks the disabled-from-VM contract.

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
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/view_models/library.rs`
- `src/ui/primitives/button.rs`
- `src/ui/primitives/tooltip.rs`
- `tests/architecture_tests.rs`

Goal:
- Render Compare ID3 + MusicBrainz controls disabled on undownloaded
  items; consume the predicates from task 002.

Constraints:
- HIG-compliant disabled state + tooltip.
- Disabled flag sourced from VM; no local computation.

Do not touch:
- Backend, db, playback

Acceptance criteria:
- Controls disabled when not downloaded; tooltip explains why.
- Architecture guard records the contract.

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

- Button primitive cannot render a dim+tooltip variant without a
  signature change.
