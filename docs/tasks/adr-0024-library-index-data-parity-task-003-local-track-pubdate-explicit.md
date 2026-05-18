# ADR 0024 Library / Index Data Parity Task 003 — Local Track Pubdate and Explicit Projection

## Goal

Load already-persisted local RSS track `pub_date` and `itunes_explicit` values
into the local track read model and shared track detail metadata projections.

This task implements the third loading-shape slice from
`docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`.

## Files To Inspect

- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `src/db.rs`
- `src/views.rs`
- `src/metadata.rs`
- `src/view_models/track_detail.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results/tests.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/db.rs`
- `src/views.rs`
- `src/metadata.rs`
- `src/view_models/track_detail.rs`
- focused unit tests in touched modules
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/discover/**`
- `src/ui/**`
- `src/app.rs`
- `src/app/**`
- SQLite schema / migrations
- ADR 0053 source-fact persistence design
- feed/release publisher, description, language, annotation, or artist-person
  identity behavior

## Constraints

- Use the existing `tracks.pub_date` and `tracks.itunes_explicit` columns.
- Do not add schema columns or migrations.
- Parse RSS `pub_date` at the DB/read-model boundary into the existing
  epoch-seconds `TrackView::pub_date` shape.
- Parse `itunes_explicit` into the existing `TrackView::explicit` shape:
  true for explicit/yes/true, false for clean/no/false, `None` for blank or
  unknown values.
- Do not add renderer-side fallback or inference.
- Do not surface track description, publisher, language, lyrics, or annotation.
- Preserve existing local track query behavior and ordering.
- No new `#[allow(...)]`; use `#[expect(...)]` only if an intentional lint
  suppression is truly required.

## Implementation Steps

1. Extend `db::TrackRow` with local read-model fields for parsed pubdate and
   explicit state.
2. Add `t.pub_date` and `t.itunes_explicit` to every query feeding
   `track_row_from_sql`.
3. Update `track_row_from_sql` to parse those source columns once, at the DB
   read-model boundary.
4. Update `TrackView::from_local_with_identity` so local track details receive
   `pub_date` and `explicit`.
5. Add an explicit summary/metadata row only from `TrackView::explicit` /
   `api::Track::explicit`; show it only when true, matching release/feed
   summary behavior.
6. Ensure existing `Release date` and `RSS item pubdate` metadata rows now
   receive local pubdate through the local projection path.
7. Add unit tests for:
   - RSS pubdate parsing from `TrackRow` into `TrackView`;
   - explicit parsing from local `itunes_explicit`;
   - shared track detail summary rows include release date and explicit only
     from VM data;
   - metadata rows include local release/pubdate and explicit rows.
8. Add or strengthen an architecture guard that pins this loading-shape path:
   DB query columns -> `TrackRow` -> `TrackView::from_local_with_identity` ->
   metadata/detail VM.

## Acceptance Criteria

- Local Library track details can show the persisted RSS item pubdate through
  shared track detail and metadata rows.
- Local Library track details can show explicit state when persisted RSS data
  marks the track explicit.
- No renderer conditionals invent missing source facts.
- No persistence/schema behavior changes.
- Index detail behavior from Task 002 remains intact.
- Regression coverage pins the read-model path.

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
- `docs/tasks/adr-0024-library-index-data-parity-task-003-local-track-pubdate-explicit.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `src/db.rs`
- `src/views.rs`
- `src/metadata.rs`
- `src/view_models/track_detail.rs`
- `tests/architecture_tests.rs`

Goal:
- Load persisted local `tracks.pub_date` and `tracks.itunes_explicit` into
  `TrackRow`, project them into `TrackView`, and surface them through existing
  VM-owned track detail / metadata contracts.

Constraints:
- Use existing columns only; no schema or migration changes.
- Parse data at the DB/read-model boundary or VM layer, not in renderers.
- Show explicit only when true, matching feed/release summary behavior.
- Do not add track description, publisher, language, lyrics, annotation, or
  artist/person source-fact behavior.
- Keep this local projection slice independent from Index remote detail.
- No new `#[allow(...)]`.
- You are not alone in the codebase. Do not revert unrelated edits.

Do not touch:
- `src/discover/**`
- `src/ui/**`
- `src/app.rs`
- `src/app/**`
- SQLite schema / migrations
- ADR 0053 source-fact design

Acceptance criteria:
- `TrackRow` carries parsed local pubdate and explicit state from existing DB
  columns.
- `TrackView::from_local_with_identity` preserves those fields.
- Shared track detail summary rows and metadata rows consume the VM/source
  values without renderer inference.
- Regression coverage pins the path.

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

- Existing local queries cannot safely add the source columns without a query
  helper refactor.
- `itunes_explicit` values have product semantics that conflict with
  feed/release explicit display.
- Adding the metadata row requires UI renderer changes instead of VM-owned data.
