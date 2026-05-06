# ADR 0026 Review Checklist

Use this checklist for ADR 0026 implementation diffs and final review.

## Architecture

- Shared projection code remains GPUI-free.
- Shared projection code does not import `library`, `search`, or service
  modules.
- Shared projection code does not import `ui`, `ui_entity`, or API client
  types.
- Shared view facts do not expose concrete `api::SourceEntityLink` or
  `api::SourceEntityId` as public identity/provenance fields.
- `src/ui_entity.rs` does not import `library`, `search`, or service modules.
- Fetching and mutation remain in existing screen/application paths, not in
  shared projection or UI modules.
- Library and Discover differences are action descriptors or screen adapters,
  not separate layout contracts for the same entity type.

## Identity and Provenance

- Contributor `href`, `img`, and `npub` fields are deserialized when present.
- Feed and track view contributor collections use `ContributorView`, not
  `api::Contributor`.
- `source_links` and `source_ids` are preserved in local source fact structs
  even when convenience identity fields are populated.
- Nostr and website extraction is conservative and field-based.
- No identity is inferred from names, titles, publisher text, filenames, or
  fuzzy matching.
- Missing identity data renders as absence, not placeholder misinformation.

## UI Boundary

- New icons use the ADR 0025 semantic icon catalog.
- New controls use `ControlStyle` / native button roles.
- Shared UI shells use slot-based action binding; screen adapters own GPUI
  click handlers and popover state.
- Repeated destructive row actions are quiet and not visually dominant.
- Library does not show redundant "downloaded" text for tracks that are
  already represented by a remove action.
- Detail layout remains consistent between Library and Discover for the same
  feed/album content.

## Tests

- API deserialization tests cover new identity fields.
- View fact tests cover identity extraction and raw fact preservation.
- Projection VM tests cover headers, summaries, action descriptors, and empty
  identity states.
- Architecture tests guard shared projection imports.
- Architecture tests guard `src/ui_entity.rs` against screen/service imports.
- Manual visual smoke covers one Discover feed and the same Library album.

## Required Verification

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

Run `cargo test` before marking ADR 0026 implemented.

## Final Review Questions

- Can a future UI render the same entity details without importing GPUI?
- Can a future ADR 0024 query service feed these projections unchanged?
- Can actions be rebound by another UI using only action kind, target, enabled
  state, and tone?
- Can a theme/icon/control change avoid touching Library and Discover screen
  files for ordinary visual adjustments?
- Are MusicIndex identity facts preserved for future provenance/debug UI even
  if not all of them are rendered today?
