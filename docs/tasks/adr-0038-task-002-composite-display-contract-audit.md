# ADR 0038 Task 002: Composite Display-Contract Audit (Stub)

## Status

Stub. Starts after Task 001 (Layer Relocation) lands.

## Goal

Every public composite signature accepts a view-model field, a
co-located display struct, or a pure passthrough. No composite accepts a
policy-bearing `String`/`&str` for a label, fallback, or state value.

## Files To Inspect (preliminary)

- `src/ui/composites/*.rs` (every public composite)
- `src/view_models/*.rs`
- `src/ui/shells/*.rs` (post-Task-001 paths)
- `src/library.rs`, `src/search.rs`
- `tests/architecture_tests.rs`

## Open Questions To Resolve Before Implementation

1. **Pure passthrough vs. policy-bearing.** Define the test: a
   composite parameter is "policy-bearing" if it has a fallback rule
   ("when empty, show X"), a truncation rule, a casing rule, or a
   format rule that the screen could plausibly get wrong. A pure
   passthrough is a label the screen has already decided.
2. **Per-composite VM vs. co-located display struct.** When a full VM
   is overkill (e.g. `ActionRow` with a label + tone), define a small
   display struct in the composite's own module. When the data is
   entity-derived (e.g. `TrackHeader` with title + subtitle + state),
   take the existing VM.
3. **Allowlist for genuine passthrough.** A new guard
   `composite_signatures_take_display_contracts_not_loose_strings`
   needs an allowlist of legitimate passthrough APIs. List them
   explicitly with a one-line justification each.
4. **Doc-comment requirement.** Every composite gets a module-level
   doc comment naming its display contract. Decide format
   (e.g. `//! ## Display contract: TrackHeaderVm`).

## Sketch of Implementation Approach

1. Inventory every `pub fn` in `src/ui/composites/*.rs` taking
   `String`/`&str`/`SharedString`/`impl Into<String>`. Grep:
   ```sh
   grep -rn "pub fn\|impl Into<String>\|: SharedString\|: String\|: &str" \
     src/ui/composites/
   ```
2. For each parameter, classify (passthrough vs. policy-bearing).
3. Migrate policy-bearing parameters to VM fields or display structs.
   Update callers.
4. Add the guard.
5. Add a per-composite doc comment naming the contract.

## Constraints

- One composite at a time. Don't bundle.
- Don't rewrite composites that already take a VM (e.g.
  `MusicBrainzPanel`, `PlaylistOption`, `RecentFeedTile`, …).
- ADR 0037 work is settled; do not modify
  `render_feed_identity_actions` or the identity-action atom.
- Keep the test allowlist explicit; never use a wildcard.

## Definition of Done

- Every public composite signature is documented in a doc comment naming
  its contract type.
- `composite_signatures_take_display_contracts_not_loose_strings` is
  green with an explicit allowlist.
- Every caller compiles against the new contract type.

## When To Start

After Task 001 is merged, replace this stub with a fully-specified task
(structure mirrors Task 001) listing the per-composite migration order.
