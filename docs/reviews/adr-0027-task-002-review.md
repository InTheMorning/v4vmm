# ADR 0027 Task 002 Review

## Result

Pass - 2026-05-01.

## Reviewed Scope

- Shared release-level action-state inputs.
- Library album-detail action adapter.
- Discover feed action-row adapter.

## Required Fixes

None identified before verification.

## Optional Improvements

- A later ADR 0027 task should move MusicBrainz and compare/provenance action
  state into the same descriptor vocabulary.
- A later visual smoke should compare the same release fixture in Library and
  Discover after Task 003 lands.

## Architectural Drift

None found in the intended slice. Release action state remains plain data under
the shared projection layer. Screen adapters still own GPUI handlers, popover
state, and command dispatch.

## Behavior Notes

- Library and Discover now share `Remove Feed`, `Download Feed`, and
  `Add feed to playlist` labels for feed-level release actions.
- Library keeps the existing unsubscribe handler, but the visible action text
  now matches the shared membership vocabulary.
- Library album-level remove action no longer uses the filled destructive
  treatment.

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
