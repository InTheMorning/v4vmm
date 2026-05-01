# ADR 0029 Review Checklist

Use this checklist for ADR 0029 task diffs.

## Architecture

- Source facts are preserved before convenience display fields.
- No name-only identity merge is introduced.
- Contributor position is not treated as durable person identity.
- `src/views.rs` and `src/view_models/*` remain GPUI-free and database-free.
- Screens do not reconstruct identity facts from ad hoc JSON.

## Schema

- Any schema is additive.
- Replacement scope is source-scoped and explicit.
- Cascade behavior is specified.
- Raw source payload retention is specified.
- Migration tests cover valid and invalid owner/key shapes.

## Implementation Scope

- Task 001 remains documentation-only.
- Later tasks touch only their bounded modules.
- No MusicIndex, RSS, metadata, or UI behavior changes leak into the wrong
  phase.

## Verification

For documentation-only tasks:

```bash
cargo fmt -- --check
cargo check
```

For runtime tasks:

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```
