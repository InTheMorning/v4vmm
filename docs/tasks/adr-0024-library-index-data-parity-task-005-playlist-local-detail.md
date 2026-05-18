# ADR 0024 Library / Index Data Parity Task 005 - Playlist Local Detail Metadata

## Goal

Surface already-persisted local playlist metadata through the existing
`PlaylistDetailVm` contract.

This task implements the fifth loading-shape slice from
`docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md` for local
Library playlists only.

## Files To Inspect

- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`
- `src/db.rs`
- `src/view_models/library.rs`
- `src/view_models/playlist_detail.rs`
- `src/ui/shells/playlist.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/library.rs`
- `src/view_models/playlist_detail.rs`
- focused unit tests in touched modules
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/discover/**`
- `src/app.rs`
- `src/app/**`
- `src/ui/shells/playlist.rs` unless a compile break proves the existing
  `PlaylistDetailPageVm::detail_rows` contract cannot carry the data
- `src/db.rs`
- SQLite schema / migrations
- Index search result/detail VMs
- ADR 0053 source-fact design
- playlist language, explicit state, release-date semantics, or MusicIndex
  playlist entities

## Constraints

- Use existing `db::Playlist` fields: `description`, `created_at`, and
  `updated_at`.
- Do not add schema columns or migrations.
- Keep the presentation facts in `PlaylistDetailVm` / `PlaylistDetailPageVm`;
  do not add renderer-side field inference.
- Use the shared GPUI-free date formatter from `src/view_models/format.rs`.
- Do not show dates for non-positive timestamp sentinels used by tests.
- Show playlist description only when it is non-empty after trimming.
- Keep this Library-local. Do not invent Index playlist result rows, detail
  pages, or parity behavior.
- No new `#[allow(...)]`; use `#[expect(...)]` only if an intentional lint
  suppression is truly required.

## Implementation Steps

1. Extend `PlaylistDetailVm::detail_rows` so it owns local playlist metadata
   rows in display order:
   - `Tracks`
   - `Duration` when known
   - `Created` when `created_at > 0` and format succeeds
   - `Modified` when `updated_at > 0` and format succeeds
   - `Description` when trimmed description is non-empty
2. Keep `PlaylistDetailPageVm::detail_rows` as the passthrough to the detail
   VM; do not move row construction into the renderer.
3. Add unit tests for:
   - empty/default playlists still omit sentinel dates and blank descriptions;
   - persisted created/updated dates render as formatted dates;
   - blank descriptions are omitted and non-empty descriptions render trimmed;
   - page VM passes the enriched rows through.
4. Add or strengthen an architecture guard proving playlist metadata rows are
   VM-owned and no Index playlist detail surface is introduced.

## Acceptance Criteria

- Library playlist detail can render created date, modified date, and
  description when those persisted local values exist.
- Empty/default sentinel timestamps do not render as `Jan 1, 1970`.
- Blank playlist descriptions do not create empty rows.
- No renderer conditionals invent playlist metadata.
- No Index playlist detail/result behavior is added.
- Regression coverage pins the VM-owned row path.

## Test Commands

```bash
cargo fmt -- --check
cargo check --quiet
cargo build --quiet
cargo test --lib --quiet
cargo test --test architecture_tests --quiet
cargo clippy --quiet -- -D warnings
git diff --check
```

## Expected Final Summary Format

1. Files changed.
2. Tests run.
3. Behavior changed.
4. Deviations from task.
5. Unresolved concerns.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/tasks/adr-0024-library-index-data-parity-task-005-playlist-local-detail.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`
- `src/db.rs`
- `src/view_models/library.rs`
- `src/view_models/playlist_detail.rs`
- `tests/architecture_tests.rs`

Goal:
- Surface existing local playlist `description`, `created_at`, and
  `updated_at` through `PlaylistDetailVm` / `PlaylistDetailPageVm`.

Constraints:
- Use existing persisted `db::Playlist` fields only.
- Do not touch schema, migrations, Index search/detail, Discover, or ADR 0053.
- Keep metadata row construction in GPUI-free view models, not renderers.
- Use `view_models::format::fmt_date`.
- Omit non-positive timestamp sentinels.
- Omit blank descriptions after trimming.
- Do not add playlist language, explicit state, release date, or Index
  playlist entities.
- No new `#[allow(...)]`.
- You are not alone in the codebase. Do not revert unrelated edits.

Do not touch:
- `src/discover/**`
- `src/app.rs`
- `src/app/**`
- `src/ui/shells/playlist.rs` unless required by the existing page VM contract
- `src/db.rs`
- SQLite schema / migrations
- Index search result/detail VMs
- ADR 0053 source-fact design

Acceptance criteria:
- `PlaylistDetailVm::detail_rows` owns Created, Modified, and Description
  rows.
- `PlaylistDetailPageVm::detail_rows` passes those rows through.
- Unit tests cover formatted dates, blank-description omission, and page VM
  passthrough.
- Architecture tests guard VM ownership and no Index playlist detail.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo build --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The existing `PlaylistDetailPageVm::detail_rows` contract cannot carry
  description text cleanly.
- Surfacing these fields requires renderer-side metadata inference.
- Any path requires Index playlist entities, schema changes, or ADR 0053
  source-fact decisions.
