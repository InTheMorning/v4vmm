# ADR 0038 Task 001: Layer Relocation

## Status

Implemented - 2026-05-03. Visual acceptance caveat closed on
2026-05-04 by the Task 004 operator-navigated Library/Discover light
and dark smoke pass; no screenshot artifacts are retained.

## Goal

Move the four top-level UI shell modules under `src/ui/shells/` so the
layer-7 directory exists, directory-scoped architecture tests cover them
automatically, and the `KNOWN_SHARED_UI_SHELL_FILES` allowlist hack can
be retired. This is the highest-blast-radius single move in the ADR 0038
plan and unlocks every downstream phase.

## Files To Inspect

- `docs/adr/0038-presentation-contract-enforcement.md`
- `docs/plans/adr-0038-presentation-contract-enforcement-phase-plan.md`
- `src/lib.rs`
- `src/ui/mod.rs`
- `src/ui_artist.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`
- `src/library.rs`, `src/search.rs` (importers; ~9 sites total)

## Files To Create / Move

Move (preserve git history with `git mv`):

| From | To |
|---|---|
| `src/ui_artist.rs` | `src/ui/shells/artist.rs` |
| `src/ui_entity.rs` | `src/ui/shells/entity.rs` |
| `src/ui_feed.rs`   | `src/ui/shells/feed.rs` |
| `src/ui_track.rs`  | `src/ui/shells/track.rs` |

Create:

- `src/ui/shells/mod.rs` — module declarations + crate-level doc comment
  describing layer 7.

Update:

- `src/lib.rs` — replace `pub mod ui_artist;` etc. with re-exports from
  `crate::ui::shells` if needed by external callers, or drop the
  re-exports entirely if no external consumer exists.
- `src/ui/mod.rs` — add `pub mod shells;`.
- All importers — change `use crate::ui_artist::…` to
  `use crate::ui::shells::artist::…` (and analogs).
- `tests/architecture_tests.rs` — see Step 5.

## Do Not Touch

- Module *contents* (no logic changes; pure relocation).
- Backend, schema, RSS/ID3, playlist, playback, services.
- Other view-models or composites.
- The four shell modules' public APIs (function signatures unchanged).
- Pre-existing ADR 0037 work (`render_feed_identity_actions`,
  `render_track_identity_actions`).

## Constraints

- Use `git mv` so history is preserved.
- No behavior changes. Compilation, tests, and visual output must be
  identical before and after.
- `cargo fmt`-clean. Don't reformat moved files beyond what `rustfmt`
  applies automatically.
- Keep module `pub use` re-exports in `src/lib.rs` only if external
  callers (binary entry, tests, other crates) depend on them; otherwise
  drop. Verify by grepping for `crate::ui_artist`/`ui_entity`/`ui_feed`/
  `ui_track` and `v4vmm::ui_artist` etc. before deleting re-exports.

## Implementation Steps

### Step 1 — Create the shells directory

```sh
mkdir -p src/ui/shells
```

Create `src/ui/shells/mod.rs`:

```rust
//! UI shells — the **seventh** layer of the design system, sitting above
//! [`crate::ui::composites`] and below screen modules.
//!
//! A shell is a top-level GPUI layout module that consumes view-models
//! and composites to produce a complete page or pane. Shells:
//!
//! * Import view-models, composites, primitives, and tokens.
//! * Do **not** import screens (`src/library.rs`, `src/search.rs`,
//!   `src/app/`), services, or backend modules.
//! * Carry no selected-entity state; that belongs to screens.
//! * Resolve all dimensions through `.scaled(cx)` and all colors
//!   through `SemanticColor`.
//!
//! See `docs/adr/0038-presentation-contract-enforcement.md` for the
//! layer architecture invariant.

#![warn(clippy::pedantic)]

pub mod artist;
pub mod entity;
pub mod feed;
pub mod track;
```

### Step 2 — Move the four shells

```sh
git mv src/ui_artist.rs src/ui/shells/artist.rs
git mv src/ui_entity.rs src/ui/shells/entity.rs
git mv src/ui_feed.rs   src/ui/shells/feed.rs
git mv src/ui_track.rs  src/ui/shells/track.rs
```

### Step 3 — Wire the new module into the tree

In `src/ui/mod.rs`:

```rust
// Add (alphabetical order with existing modules):
pub mod shells;
```

In `src/lib.rs`:

- Remove `pub mod ui_artist;`, `pub mod ui_entity;`, `pub mod ui_feed;`,
  `pub mod ui_track;`.
- If a grep shows external consumers (test binaries, integration tests)
  using `crate::ui_artist::…` paths, keep a re-export shim:

  ```rust
  pub use crate::ui::shells::artist as ui_artist;
  pub use crate::ui::shells::entity as ui_entity;
  pub use crate::ui::shells::feed as ui_feed;
  pub use crate::ui::shells::track as ui_track;
  ```

  Otherwise drop the names entirely. Verify by:

  ```sh
  grep -rn "crate::ui_artist\|crate::ui_entity\|crate::ui_feed\|crate::ui_track\|v4vmm::ui_artist\|v4vmm::ui_entity\|v4vmm::ui_feed\|v4vmm::ui_track" src/ tests/
  ```

  Update or remove paths as appropriate. ADR 0037 task documents and
  similar prose may also reference the old paths; leave docs alone in
  this task (they're historical).

### Step 4 — Update import sites

Known sites to update (verified 2026-05-03):

- `src/library.rs:77` — `use crate::ui_entity::…`
- `src/library.rs:3045` — `crate::ui_track::render_track_identity_actions`
- `src/search.rs:67`   — `use crate::ui_entity::…`
- `src/search.rs:2517` — `crate::ui_artist::render_artist_view`
- `src/search.rs:2542` — `crate::ui_feed::render_feed_view`
- `src/search.rs:2644` — `crate::ui_track::render_track_identity_actions`
- `src/search.rs:4439`,`4448` — `crate::ui_track::render_track_row`,
  `crate::ui_track::TrackRowMode::Discover`

Re-grep before editing to catch anything new:

```sh
grep -rn "crate::ui_artist\|crate::ui_entity\|crate::ui_feed\|crate::ui_track" src/ tests/
```

Replace `crate::ui_artist::X` with `crate::ui::shells::artist::X` (etc.).
Prefer `use` statements at file top over inline `crate::…` paths where
the file already follows that convention.

### Step 5 — Update architecture tests

In `tests/architecture_tests.rs`:

1. **Remove** `KNOWN_SHARED_UI_SHELL_FILES` (currently
   `["src/ui_artist.rs", "src/ui_entity.rs"]`) and any code that reads
   it. The directory scope of `src/ui/shells/` now covers these
   automatically.
2. **Update** `PRESENTATION_GLUE_FILES` — drop `src/ui_feed.rs` and
   `src/ui_track.rs`. They are layer-7 shells, not glue.
3. **Update** any path literal in test bodies that mentions the old
   files (e.g. ADR 0036 release-surface guard at line 1763 that lists
   `("src/ui_entity.rs", …)`). Replace with the new path.
4. **Add** the new guard
   `top_level_shells_live_under_src_ui_shells`:

   ```rust
   #[test]
   fn top_level_shells_live_under_src_ui_shells() {
       // Forbid any new top-level src/ui_*.rs file from sneaking back
       // in. The four legacy shells are now under src/ui/shells/.
       let manifest = manifest_path("src");
       let entries = std::fs::read_dir(&manifest)
           .expect("read src/")
           .filter_map(Result::ok);
       let mut violations = Vec::new();
       for entry in entries {
           let name = entry.file_name();
           let name = name.to_string_lossy();
           if name.starts_with("ui_") && name.ends_with(".rs") {
               violations.push(format!(
                   "{name}: top-level shell modules must live under \
                    src/ui/shells/, not src/"
               ));
           }
       }
       assert!(
           violations.is_empty(),
           "ADR 0038 layer relocation violations:\n{}",
           violations.join("\n")
       );
   }
   ```

5. **Run** the full architecture test suite. Any guard that read paths
   from the now-removed allowlist must be updated to read from
   `src/ui/shells/` directly.

### Step 6 — Verify

```sh
cargo fmt -- --check
cargo check
cargo test top_level_shells_live_under_src_ui_shells
cargo test
cargo clippy -- -D warnings
git diff --check
```

Visual smoke: this was a no-op move; the running app should look
identical. Initial coordinate-driven X11 captures from 2026-05-03 were
discarded and do not count as evidence. The visual acceptance criterion
was later closed by the 2026-05-04 Task 004 operator-navigated
Library/Discover light and dark smoke pass recorded in
`docs/reviews/adr-0038-review-checklist.md`. Per operator instruction,
screenshots were transient only and no screenshot artifacts are retained
or committed.

## Acceptance Criteria

- The four files exist at their new paths under `src/ui/shells/`. Git
  history is preserved (`git log --follow` shows the move).
- `KNOWN_SHARED_UI_SHELL_FILES` is removed from
  `tests/architecture_tests.rs`.
- `top_level_shells_live_under_src_ui_shells` exists and is green.
- All importers compile against the new paths.
- `src/ui/shells/mod.rs` documents layer 7 per the ADR.
- All checks green; baselines unchanged.
- Library + Discover shell visual acceptance is recorded in the Task
  004 visual smoke ledger; no screenshot artifacts are retained.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test top_level_shells_live_under_src_ui_shells`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

## Escalation Triggers

- If `cargo check` reveals an importer outside `src/` (e.g. a
  benchmark, an example binary, a downstream crate), update it in this
  task — relocation must be complete.
- If a guard test other than `KNOWN_SHARED_UI_SHELL_FILES` references
  the old paths and updating it would change a baseline, stop and split
  off a follow-up: relocation should be no-op for baselines.
- If the four shell modules turn out to import from each other in ways
  that conflict with the new module structure (e.g. cyclic via
  `mod.rs`), stop and surface the cycle. Don't paper over it.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign architecture.

Read:
- `docs/adr/0038-presentation-contract-enforcement.md`
- `docs/plans/adr-0038-presentation-contract-enforcement-phase-plan.md`
- `docs/tasks/adr-0038-task-001-layer-relocation.md`
- `src/lib.rs`, `src/ui/mod.rs`
- `src/ui_artist.rs`, `src/ui_entity.rs`, `src/ui_feed.rs`,
  `src/ui_track.rs`
- `tests/architecture_tests.rs`

Goal:
- `git mv` the four shells into `src/ui/shells/`.
- Create `src/ui/shells/mod.rs` documenting layer 7 and declaring the
  four submodules.
- Wire the new path into `src/lib.rs` and `src/ui/mod.rs`.
- Update every importer (~9 sites in `src/library.rs` and
  `src/search.rs`).
- Remove `KNOWN_SHARED_UI_SHELL_FILES` and update
  `PRESENTATION_GLUE_FILES` in the architecture tests.
- Add `top_level_shells_live_under_src_ui_shells` per Step 5.
- Update any guard test that references the old paths.

Constraints:
- No logic changes. Compilation and tests pass before and after.
- Use `git mv` to preserve history.
- Leave doc files (ADR 0037 task notes, plan files referring to old
  paths) alone — they are historical.

Acceptance criteria:
- Four shells under `src/ui/shells/`.
- Allowlist removed; new guard green.
- All checks green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test top_level_shells_live_under_src_ui_shells`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

At the end, report:
1. files moved
2. files modified
3. tests run
4. deviations from task
5. unresolved concerns
