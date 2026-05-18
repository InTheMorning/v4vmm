# ADR 0040 Spawn Task 002 — `src/library/app_impl.rs` Migration

Status: Proposed - 2026-05-18.

## Goal

Retire six of the eight `cx.spawn` sites in `src/library/app_impl.rs`
by introducing typed fetch / mutation commands and routing through
`present_command`. The two skipped sites — line 1052 (thumbnail fetch,
Task 004) and line 2279 (MusicBrainz album saga, Task 005) — are
deferred to dedicated packets.

Plan reference: `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.
Prerequisite: Task 001 landed; `FetchRecentFeedsPage` and
`FetchIndexSearchResults` exist and the search-dispatch sites are
migrated.

## Files To Inspect

Required:

- `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
  (Decision, Command Placement).
- `src/presentation/async_command_presenter.rs` (bridge).
- `src/application/queries/feed.rs` post-Task-001 (one migrated query
  for diff shape).
- `src/application/queries/library.rs` — existing library queries; the
  new commands will live alongside.
- `src/application/commands/metadata.rs` — existing metadata mutations.
  `ApplyTrackId3Edits` (new) will likely live here.
- `src/library/app_impl.rs:524` (library tracks tree load).
- `src/library/app_impl.rs:1185` (album identity hydration).
- `src/library/app_impl.rs:1467` (track context with local fallback).
- `src/library/app_impl.rs:1875` (id3 edit apply + compare).
- `src/library/app_impl.rs:2050` (compare_library_track caller).
- `src/library/app_impl.rs:2103` (compare_library_track caller — likely
  near-duplicate of `:2050`; share the same command).
- `src/library/app_impl.rs:1499` (`fetch_library_track_context_with_local_fallback`
  body — the fetcher the `:1467` migration will wrap).
- `src/library/app_impl.rs:2689` (`hydrate_album_identity_facts` body).
- `src/library/app_impl.rs:3487` (`compare_library_track` body).
- `src/library/app_impl.rs:371` — composition root (confirm the
  `command_runner` field still works for these new dispatches; it
  should after the ADR 0040 retirement).
- `tests/architecture_tests.rs:10511-10555` — the
  `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
  baseline. After migration: 8 → 2 for `src/library/app_impl.rs`.

Reference only:

- `src/app/search_dispatch.rs` after Task 001 — migrated pattern.

## Files Likely To Change

- `src/application/queries/library.rs` — add `LoadLibraryTracksTree`,
  `FetchLibraryTrackContext`, `HydrateAlbumIdentity`,
  `CompareLibraryTrack` (4 new query commands). Output types match
  what the current free fns return.
- `src/application/commands/metadata.rs` — add `ApplyTrackId3Edits`
  (1 new mutation command). Mutations live in `commands/`, not
  `queries/`.
- `src/application/queries/mod.rs` and
  `src/application/commands/mod.rs` — register / re-export as needed.
- `src/library/app_impl.rs` — replace six `cx.spawn(...)` blocks with
  `present_command(...)`. If free fns
  (`fetch_library_track_context_with_local_fallback`,
  `hydrate_album_identity_facts`, `compare_library_track`) lose all
  remaining callers, move them into the query module; otherwise leave
  in place and call from `execute`.
- `tests/architecture_tests.rs` — baseline:
  `("src/library/app_impl.rs", 8)` → `("src/library/app_impl.rs", 2)`.

Probable but verify:

- A pre-existing helper at `src/library/app_impl.rs:1499` is currently
  a `Self::` associated fn. If converting requires removing the
  `&self` borrow (because `execute` is `(self, &CommandContext)`),
  consider promoting to a free fn or stand-alone helper inside the
  query module.

## Do Not Touch

- `src/library/app_impl.rs:1052` — Task 004 (image cache).
- `src/library/app_impl.rs:2279` — Task 005 (saga).
- `src/app/`, `src/discover/`, `src/presentation/`.
- `CommandBus` / `AsyncCommandRunner`.

## Constraints

- One-to-one call-site substitution. Match Task 001's pattern.
- Each new command struct is small: inputs needed by `execute`, plus
  any contextual fields (endpoint string, `Arc<Mutex<Connection>>`,
  service references). `execute` body is the existing fetcher /
  mutator logic moved inline.
- `Self::Output` is `Send + Sync + 'static`. Avoid GPUI / `gpui::Entity`
  types. If a fetcher returns something containing a non-Send type,
  decompose into plain data at the boundary.
- Behavior preservation: every on-success callback's staleness guard
  (e.g., `if let Some(frame) = this.selected_track_frame_mut()` and
  the `frame.entity_id == entity_id` filter) belongs inside the
  bridge's on-success closure, not inside `execute`.
- The two `compare_library_track` callers (`:2050` and `:2103`) share
  the same command. Confirm both call sites can be parameterized by
  the same struct shape; if they differ (e.g., one ignores certain
  fields), unify before migrating.
- No new `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read the parent plan, this task, Task 001 results (one migrated
   call site), and the six spawn sites.
2. Group sites by underlying fetcher / mutator:
   - `:524` → `LoadLibraryTracksTree`. Wraps `library_service::library_tracks`
     + `build_tree(...)`. Output: `(usize, Tree)` or named struct.
   - `:1185` → `HydrateAlbumIdentity`. Wraps `hydrate_album_identity_facts`.
     Output: `AlbumIdentityHydration` (define if missing).
   - `:1467` → `FetchLibraryTrackContext`. Wraps
     `fetch_library_track_context_with_local_fallback`. Output:
     existing return type.
   - `:1875` → `ApplyTrackId3Edits` (mutation). Wraps the
     `write_id3v24_edits` + `subscribe_service::compare_downloaded_track_path`
     pair. Output: comparison result.
   - `:2050` and `:2103` → `CompareLibraryTrack`. Wraps
     `compare_library_track`. Output: existing return type.
3. Define each command in its target file:
   - `LoadLibraryTracksTree`, `FetchLibraryTrackContext`,
     `HydrateAlbumIdentity`, `CompareLibraryTrack` →
     `src/application/queries/library.rs`.
   - `ApplyTrackId3Edits` → `src/application/commands/metadata.rs`.
4. After each command lands, build (`cargo build`) before touching
   call sites.
5. Walk the six call sites in order. For each:
   - Build the command struct from the captures the existing spawn
     block pulls in (endpoint, conn, track, identifiers).
   - Replace `cx.spawn(...) { ... }` with
     `present_command(&self.command_runner, command, CommandContext::next(),
     cx, on_success, on_error)`.
   - Move the on-success body (post-await `this.update(cx, |this, cx|
     ...)` contents) into a closure `move |this, output, cx| { ... }`.
   - Move the on-error arm into `move |this, error, cx| { ... }`.
6. After every ~3 sites: `cargo build` to catch lifetime / Send issues.
7. If any fetcher / mutator free fn (`fetch_library_track_context_with_local_fallback`,
   `hydrate_album_identity_facts`, `compare_library_track`,
   `write_id3v24_edits`-wrapper) no longer has callers in
   `src/library/app_impl.rs`, evaluate moving it to the queries module.
   Do not delete if other callers remain.
8. Update the architecture-test baseline:
   `("src/library/app_impl.rs", 8)` → `("src/library/app_impl.rs", 2)`.
   The remaining 2 are line 1052 (Task 004) and line 2279 (Task 005).
9. Run all five gates.
10. Smoke (if possible): launch the app, exercise: library reload,
    expand an album (triggers identity hydration), open a track (track
    context fetch), edit ID3, view tag-compare. Each should behave
    identically.

## Acceptance Criteria

- `grep -n "cx\.spawn(" src/library/app_impl.rs` returns two hits
  (line ~1052 and line ~2279).
- Five new commands exist under `src/application/queries/library.rs`
  and `src/application/commands/metadata.rs` and implement
  `ApplicationCommand`.
- All six call sites use `present_command`.
- Baseline `("src/library/app_impl.rs", 2)` and the guard passes.
- All five gates pass.
- No new `#[allow(...)]`.
- No behavior change.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded migration task — second of seven in
the screen-local `cx.spawn` retirement plan.

Prerequisite: Task 001 has landed.
`src/application/queries/feed.rs` and
`src/application/queries/search.rs` already host migrated query
commands you can use as templates.

Read in order:

1. This task file in full.
2. `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
   (Decision, Command Placement).
3. `src/presentation/async_command_presenter.rs` (bridge).
4. One Task 001 query (`FetchRecentFeedsPage` or
   `FetchIndexSearchResults`) for the diff shape.
5. `src/library/app_impl.rs` — the six spawn sites and the fetcher fn
   bodies they call.
6. `src/application/queries/library.rs` and
   `src/application/commands/metadata.rs` (existing shapes).

Goal:

Define five new `ApplicationCommand` variants
(`LoadLibraryTracksTree`, `FetchLibraryTrackContext`,
`HydrateAlbumIdentity`, `CompareLibraryTrack` in
`queries/library.rs`; `ApplyTrackId3Edits` in
`commands/metadata.rs`). Migrate the six spawn sites at
`src/library/app_impl.rs:524, 1185, 1467, 1875, 2050, 2103`. Update the
`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
baseline for the file from 8 to 2.

Constraints:

- One-to-one substitution.
- No behavior change.
- No `#[allow(...)]`.
- Don't touch `:1052` or `:2279`; later tasks own those.
- Don't touch `src/app/`, `src/discover/`, the bridge, the feature
  flag.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. New command names, paths, and output types.
2. Six migrated call sites (file:line before → after — terse).
3. Any free fn relocated or deleted.
4. Baseline diff.
5. Five-gate results.
6. Deviations + unresolved concerns.

## Escalation Triggers

- `compare_library_track` callers `:2050` and `:2103` differ in inputs
  beyond what a single command struct can express. Report the
  difference; the right path is usually two commands with a shared
  internal helper, not branching inside one command.
- A fetcher (`fetch_library_track_context_with_local_fallback`,
  `hydrate_album_identity_facts`) takes a `&Self` receiver. Report;
  the right path is promoting it to a free fn that takes the explicit
  inputs.
- A spawn block's on-success closure dispatches another async
  operation (e.g., chained fetches). Report the chain; the bridge
  supports nested `present_command` calls inside an on-success closure,
  but the chain should be visible in the report.
- The composition root field `command_runner` type changed in a way
  that breaks dispatch. The field should be `AsyncCommandRunner` after
  the ADR 0040 retirement; if it is somehow a `GpuiCommandRunner`,
  Task 001 prerequisite was not met.
