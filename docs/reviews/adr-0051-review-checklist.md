# ADR 0051 review checklist

## Reviewed Artifact

ADR 0051 Task 001 implementation diff.

## Required Checks

- Config change is additive and preserves existing keys.
- `[workspace_layout]` frame persistence remains unchanged.
- `[workspace.layout]` accepts missing and malformed values without breaking
  config load.
- Loaded width clamps to `CONTENT_PANE_MIN_WIDTH..=CONTENT_PANE_MAX_WIDTH`.
- `resize_content_pane` does not write config.
- `end_content_pane_resize` writes config exactly once per completed drag.
- No `SplitPane` or workspace shell persistence ownership was introduced.
- Architecture tests pin the config/app/resize ownership boundaries.
- No new `#[allow(...)]` annotations.

## Test Gate

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo build --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Merge Recommendation

Merge only if the implementation is behavior-preserving outside pane-width
restore/persist and the full gate is green.
