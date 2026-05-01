# ADR 0027 Task 001 Review

## Result

Pass - 2026-05-01.

## Reviewed Scope

- Shared track-row action-state inputs.
- Library and Discover adapters for track-row membership actions.
- Architecture-test tightening for the shared projection boundary.

## Required Fixes

None.

## Optional Improvements

- A later ADR 0027 task should move release-level membership actions onto the
  same action-state pattern.
- A later ADR 0025 task should decide whether `ControlStyle` needs a named
  destructive-quiet row role or whether `RowAction` is sufficient.

## Architectural Drift

None found. The new action state is plain data under the shared projection
layer. Screen adapters still own GPUI handlers, command dispatch, and popover
state.

## Behavior Notes

- Discover track rows still render compact icon controls, but the action kind,
  busy/disabled state, and tone now come from shared descriptors.
- Library album track rows now use quiet row-action treatment for repeated
  remove actions instead of large filled destructive buttons.
- Existing download/remove/playlist handlers were not moved or rewritten.

## Verification

```bash
cargo fmt -- --check
cargo check
cargo test entity_detail
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

All commands passed.

## Merge Recommendation

Mergeable.
