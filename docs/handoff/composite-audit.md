# Composite call-site audit (ADR 0042)

**Method:** for each module under `src/ui/composites/*.rs`, count files
outside `src/ui/composites/` that reference any public symbol the module
re-exports through `src/ui/composites/mod.rs`. Numbers were produced by
the script in `tmp/audit.sh` (kept out of tree); see git log for inputs.

ADR 0042 rule: a composite must have ≥ 2 distinct call sites in shells
or screens, otherwise it collapses into the single shell that uses it.
Internal composites (consumed only by other composites) are exempt and
treated as private helpers.

## Classification

| Composite | Call sites | Composite consumers | Verdict | Action |
|---|---|---|---|---|
| `track_row` | 17 | 0 | **keep** | none |
| `tag_badge` | 13 | 8 | **keep** | none |
| `playlist_popover` | 7 | 0 | **keep** | none |
| `thumbnail` | 7 | 7 | **keep** | none |
| `detail_grid` | 7 | 1 | **keep** | none |
| `disclosure_group` | 6 | 1 | **keep** | none |
| `identity_action` | 5 | 0 | **keep** | none |
| `detail_header` | 4 | 0 | **keep** | none |
| `action_button` | 4 | 0 | **keep** | none |
| `release_detail_surface` | 4 | 0 | **keep** | none |
| `list_row` | 3 | 1 | **keep** | none |
| `track_metadata_grid` | 3 | 0 | **keep** | none |
| `track_detail_surface` | 3 | 1 | **keep** | none |
| `segmented_control` | 2 | 0 | **keep** | none — multi-purpose primitive shape |
| `file_header` | 2 | 0 | **keep** | none — used by two parallel inspector shells |
| `action_row` | 2 | 0 | **keep** | none — used by two parallel inspector shells |
| `musicbrainz_panel` | 2 | 0 | **keep** | none — used by two parallel inspector shells |
| `split_pane` | 2 | 0 | **keep** | none — generic layout shape |
| `recent_feed_tile` | 1 | 0 | **inline** | move into `src/ui/shells/discover/recent.rs` |
| `track_inspector_pane` | 1 | 0 | **inline** | move into `src/ui/shells/discover/track_inspector.rs` |
| `now_playing_bar` | 1 | 0 | **inline** | move into `src/app/playback_bar.rs` |
| `track_header` | 0 | 1 | **internal** | private helper of `track_detail_surface`; keep |

## Inlining checklist

For each composite marked **inline**, the migration commit must:

1. Move the module body into the consuming shell file (preserve types
   and helper fns; drop only the leading `//!` doc preamble that
   describes "composite layer").
2. Demote `pub` symbols to `pub(super)` or remove `pub` entirely if the
   shell does not re-export them.
3. Delete the `pub mod` and `pub use` lines from
   `src/ui/composites/mod.rs`.
4. Delete the `src/ui/composites/<name>.rs` file.
5. Run `cargo fmt && cargo clippy --lib --tests && cargo test --lib`
   and `cargo test --test architecture_tests`.
6. The architecture test suite must continue to pass without changes.

## Deferred

- `track_header` is a private helper for `track_detail_surface`. ADR
  0042 allows internal composites; no action.
- The 2-site cluster around the inspector (`file_header`, `action_row`,
  `musicbrainz_panel`) could collapse if the inspector shells unify in
  a later phase. Tracked as future work, not in this round.

## Outcome

Three single-use composites are scheduled for inlining in the next
phase-C commits, one composite per commit (per todo
`inline-single-use-composites`). Renaming pass (`composite-naming-pass`)
runs after all inlines land.
