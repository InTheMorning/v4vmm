# ADR 0052-0054 Review R3: Composite Call-Site Reconciliation

## Reviewed Artifact

- Source review: `docs/reviews/adr-0052-0054-implementation-review.md`
- Remediation: R3 composite call-site audit + ADR 0042 reconciliation
- Diff scope:
  - `docs/adr/0042-layer-consolidation.md`
  - `docs/research/composite-audit-adr-0042.md`
  - `docs/architecture/architecture-current-snapshot.md`
  - `src/ui/composites/mod.rs`
  - `src/ui/shells/discover/recent.rs`
  - `tests/architecture_tests.rs`

## Result

Pass.

## Required Fixes

None.

## Architectural Drift

No drift found.

`SkeletonFeedTile` was Discover-recent-only and now lives inside
`src/ui/shells/discover/recent.rs`, matching ADR 0042's rule that single-shell
page-section blocks belong in their consuming shell.

The other reviewed call-site concerns were reconciled as current-state
documentation:

- `BreadcrumbTrail` has both `frame_shell` and Library track-detail callers.
- `MusicBrainzPanel` has both Library and Discover track-metadata callers.
- `ReleaseDetailSurface` has one direct Rust caller, but that caller is the
  shared `src/ui/shells/entity.rs` release/feed shell for Library and Index
  projections. ADR 0042 now documents that retained shape.

## Regression Guard

- `adr_0042_composite_call_site_reconciliation_is_current`

## Verification

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --test architecture_tests adr_0042_composite_call_site_reconciliation_is_current --quiet`
- `cargo test recent --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo test --lib --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo build --quiet`
- `git diff --check`

## Visual Smoke

Not run. The code path is a move of the existing Discover recent skeleton
placeholder into its owning shell; operator visual smoke is recommended for the
Discover recent-feeds loading state if that state is easy to capture.

## Merge Recommendation

Merge R3.
