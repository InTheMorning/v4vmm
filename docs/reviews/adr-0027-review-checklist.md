# ADR 0027 Review Checklist

Use this checklist for ADR 0027 implementation diffs.

## Architecture

- Shared action-state inputs are plain Rust data.
- `src/views.rs` and shared projection modules remain GPUI-free.
- Shared projection modules do not import `library`, `search`, services,
  SQLite, or MusicIndex clients.
- Screen adapters own command dispatch, popover state, image handles, and GPUI
  handlers.
- ADR 0024 boundaries are preserved for commands and future query work.

## Behavior

- Library and Discover use the same action descriptor vocabulary for equivalent
  membership actions.
- Repeated destructive row actions use quiet destructive treatment.
- Busy and disabled states are visible and consistent.
- Redundant downloaded detail rows are suppressed when membership is already
  represented by actions.
- Existing command semantics remain unchanged.

## Tests

- Projection tests cover every new action-state enum variant.
- Adapter tests cover conversion from screen-local state into shared inputs
  where practical.
- Architecture tests protect the GPUI-free and screen-free boundary.
- Manual visual smoke compares the same release in Library and Discover before
  final acceptance.

## Required Verification

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

Run broader `cargo test` before marking ADR 0027 implemented.
