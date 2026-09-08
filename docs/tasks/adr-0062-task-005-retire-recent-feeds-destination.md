# ADR 0062 Task 005: Retire The Recent Feeds Destination

Status: Ready - 2026-09-07. Do after task 004.

## Goal

Remove the `Recent Feeds` toolbar command, its separate surface, and its
reachability guards. Keep its index query as the data source behind the default
order.

## Files To Inspect

- `docs/adr/0062-music-content-surface.md`
- `docs/adr/0030-discovery-library-ui-fixes.md`, whose invariant this withdraws
- `src/view_models/recent_feeds.rs`
- `src/app/recent_feeds.rs`
- `src/view_models/app_toolbar.rs`
- `tests/architecture_tests.rs`, the guard at the `Recent Feeds reachability is
  invariant` string and the `return_to_recent_feeds` guard
- `docs/plans/post-adr-0048-recent-feeds-route-plan.md`

## Files Likely To Change

- `src/view_models/recent_feeds.rs`
- `src/app/recent_feeds.rs`
- `src/view_models/app_toolbar.rs`
- `src/app.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`
- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/plans/broadcast-chain-delivery-order.md`

## Do Not Touch

- The index query and its cursor paging. Task 002 depends on them.
- The row contract, the filter control, and the view modes.
- `src/ui/shells/discover/**` beyond deleting the guard that names it. That
  module is parked and its removal is separate work.

## Constraints

- **The query survives. The destination goes.** `recent_feeds.rs` holds a
  working index query with cursor paging that task 002 already consumes.
  Deleting it would delete the default view's data source.
- **The ADR 0030 reachability invariant is withdrawn, not ignored.** Amend ADR
  0030 with a dated sentence recording that ADR 0062 removed the destination the
  invariant protected. ADR 0057 governs the amendment.
- Delete the guards with the design, in this change. A guard asserting a
  removed destination is a defect of the same weight as a missing guard.
  ADR 0061.
- One of those guards asserts `return_to_recent_feeds` and `show_recent_feeds`
  in `src/ui/shells/discover/search_input.rs`, which no composition root
  reaches. Delete the guard. Leave the parked module for its own task.
- No toolbar command opens a separate recent-feeds surface after this task.

## Implementation Steps

1. Remove the `Recent Feeds` button from the toolbar view model and its
   renderer, including its identifier, label, and accessibility label.
2. Remove the route and screen mount that opened the separate surface.
3. Keep the index query, its batching, its cursor handling, and its load
   intent. Move them if the module name no longer fits, and say where they went.
4. Delete the `Recent Feeds reachability is invariant` guard.
5. Delete the guard requiring `return_to_recent_feeds` and `show_recent_feeds`
   in the parked discover shell.
6. Amend ADR 0030 with a dated sentence withdrawing the invariant and naming
   ADR 0062.
7. Update `docs/plans/post-adr-0048-recent-feeds-route-plan.md` to record that
   the route is retired, or archive it.
8. Add a guard, situational, citing ADR 0062: no toolbar command and no screen
   mount opens a recent-feeds destination.

## Acceptance Criteria

Mechanical:

- No toolbar command, route, or screen mount opens a recent-feeds surface.
- The index query, its paging, and its load intent still exist and task 002
  still consumes them.
- Both Recent Feeds guards are deleted, and a new guard asserts the destination
  stays absent.
- ADR 0030 carries a dated amendment naming ADR 0062.
- The guard suite is smaller than before this task.

Visual proof, operator only:

- The toolbar no longer shows a `Recent Feeds` button.
- `Music` still opens on recent music, unchanged from task 002.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo run` for the visual check

## Expected Final Report Format

1. Files changed and deleted
2. Where the index query now lives
3. Guards deleted and added, with the suite line count before and after
4. Tests run
5. Screenshots captured, or the visual gate reported open
6. Deviations from task
7. Unresolved concerns

## Escalation Triggers

- Something other than task 002 consumes the recent-feeds surface. Report the
  caller.
- Removing the route breaks a saved workspace layout or a stored navigation
  entry. An older `config.toml` must still load.
- The ADR 0030 amendment would reverse rather than withdraw a decision. That
  needs a new ADR, not an amendment.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0062-music-content-surface.md`, `docs/adr/0030-discovery-library-ui-fixes.md`
- `src/view_models/recent_feeds.rs`, `src/app/recent_feeds.rs`,
  `src/view_models/app_toolbar.rs`

Goal:
- Remove the Recent Feeds command, surface, and guards. Keep its index query as
  the default order's data source.

Constraints:
- The query survives. Deleting it would delete the default view's data source.
- Delete both Recent Feeds guards in this change.
- Amend ADR 0030 with a dated sentence withdrawing its invariant.
- An older `config.toml` must still load.

Do not touch:
- the index query and its paging, the row contract, the filter, the view modes,
  the parked discover module beyond its guard

Acceptance criteria are split into mechanical and visual. Report the visual
gate as open if no window can be opened.

Test commands:
- `cargo fmt -- --check`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed and deleted
2. where the index query now lives
3. guards deleted and added, with suite line count before and after
4. tests run
5. screenshots captured or the visual gate reported open
6. deviations from task
7. unresolved concerns
