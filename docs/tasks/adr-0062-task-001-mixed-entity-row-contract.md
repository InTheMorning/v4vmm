# ADR 0062 Task 001: Mixed Entity Row Contract

Status: Ready - 2026-09-07.

## Goal

Define one row contract that carries artist, release, and track rows together,
with an entity badge and a library badge. View model only. Nothing renders
differently after this task.

## Files To Inspect

- `docs/adr/0062-music-content-surface.md`
- `docs/adr/0061-executable-governance.md`
- `src/view_models/search_results/results.rs`, for the three entity displays
- `src/view_models/search_results/tabs.rs`, for `SearchResultsTab` and
  `SearchResultOrigin`
- `src/view_models/library.rs`, for `ContentListRowDisplay`,
  `ContentListRowSource`, and `ContentListPageVm`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/library.rs`
- `src/view_models/search_results/results.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/**`. This task changes no renderer.
- `src/view_models/recent_feeds.rs`
- The `ContentFilter` enum and its three values
- `SearchResultsTab`. Task 005 retires it, not this task.

## Constraints

- **Reuse the three displays that exist.** `ArtistResultDisplay`,
  `FeedResultDisplay`, and `TrackResultDisplay` are built and tested. The row
  contract composes them. It does not replace them and does not copy their
  fields.
- **The row must not become a union of optional fields.** A row carries a kind
  and the display for that kind. A caller matches on the kind. It does not read
  four optional fields and infer which is populated.
- Every row carries two badges as view-model facts:
  - an entity badge naming artist, release, or track
  - a library badge stating whether the row is in the library
- The library badge derives from `ContentListRowSource`, which already answers
  this. Do not add a second source of truth for library membership.
- A release row and an artist row carry an expansion state. A track row does
  not. Expansion is a view-model fact, never a renderer decision.
- GPUI-free. No renderer type in any public field.

## Implementation Steps

1. Define the row kind enum with artist, release, and track cases, each holding
   its existing display type.
2. Define the entity badge and the library badge as display contracts with
   labels and accessibility labels. Neither relies on color.
3. Extend `ContentListRowDisplay` to carry the kind, both badges, and an
   expansion state for the two expandable kinds.
4. Keep `matches_filter` working against `ContentListRowSource`. The filter
   semantics do not change in this task.
5. Add a projector from each of the three existing display types into a row.
6. Add unit tests: one row of each kind, badge labels for each kind, library and
   index badge states, expansion available on release and artist and absent on
   track, and filter behavior unchanged for each kind.
7. Add a guard, situational, citing ADR 0062: the row contract carries a kind
   enum and no field named for a single entity type at the top level.

## Acceptance Criteria

Mechanical:

- The row kind enum has exactly three cases, each holding an existing display
  type.
- No top-level row field is specific to one entity kind.
- Every row exposes an entity badge and a library badge with labels and
  accessibility labels.
- Release and artist rows expose an expansion state. Track rows do not.
- `matches_filter` behaves identically to before for every kind.
- No `gpui` import in the changed view-model files.

Visual proof: none. This task changes no rendering.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test library --lib --quiet`
- `cargo test search_results --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- The three existing displays do not share enough shape to compose into one row
  without copying fields. Report which fields conflict.
- `ContentListRowSource` cannot answer library membership for an artist or a
  release. That is a data question, not a view-model one.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture. Change no renderer.

Read:
- `docs/adr/0062-music-content-surface.md`
- `src/view_models/search_results/results.rs`, `src/view_models/library.rs`

Goal:
- One row contract carrying artist, release, and track rows, with an entity
  badge and a library badge.

Constraints:
- Compose the three existing display types. Do not copy their fields.
- A row carries a kind and the display for that kind. No union of optionals.
- Badges are view-model facts and never rely on color.
- Expansion state on release and artist rows only.
- GPUI-free.

Do not touch:
- `src/ui/**`, `recent_feeds.rs`, `ContentFilter`, `SearchResultsTab`

Acceptance criteria are mechanical only. This task has no visual gate.

Test commands:
- `cargo fmt -- --check`
- `cargo test library --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
