# ADR 0040 Spawn Task 001 — `src/app/search_dispatch.rs` Migration

Status: Proposed - 2026-05-18.

## Goal

Retire two of the three `cx.spawn` call sites in
`src/app/search_dispatch.rs` (lines 142 and 213) by routing the
underlying HTTP fetches through `present_command`. Defines the first
two new `ApplicationCommand` query variants
(`FetchRecentFeedsPage`, `FetchIndexSearchResults`) and proves the
migration pattern on the smallest live surface. The third site (line
577, thumbnail fetch) is deferred to Task 004 (image cache strategy).

Plan reference: `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.

## Files To Inspect

Required:

- `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
  (Decision, Command Placement, Invariants, Risk Areas).
- `docs/adr/0040-async-vm-runtime.md` (status block).
- `src/presentation/async_command_presenter.rs` — bridge signature.
- `src/application/command_bus.rs` — `ApplicationCommand` trait
  (line 53), `execute` signature.
- `src/application/queries/feed.rs` and
  `src/application/queries/search.rs` (full — see how existing query
  commands shape `execute` and pass arguments).
- `src/app/search_dispatch.rs` — read **lines 100-260, 555-600** to
  see all three spawn sites in context.
- `src/app/search_dispatch.rs:1210-1290` — the two fetcher fns
  `fetch_index_search_result_rows` and `fetch_recent_feed_result_rows`
  the new commands will wrap.
- `tests/architecture_tests.rs:10511` — the
  `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
  baseline. Note the file's allowed count and reduce by 2 after
  migration.

Reference only:

- `src/library/app_impl.rs:524` — sample spawn site for Task 002 (do
  not modify).
- `src/discover/app_impl.rs:126` — sample spawn site for Task 003.

## Files Likely To Change

- `src/application/queries/feed.rs` — add `FetchRecentFeedsPage` (new
  query struct + `ApplicationCommand` impl).
- `src/application/queries/search.rs` — add `FetchIndexSearchResults`
  (new query struct + `ApplicationCommand` impl). If the file does not
  exist or does not have the right home for index searches, place
  alongside existing search queries; do not invent a new directory.
- `src/application/queries/mod.rs` — register new symbols if not already
  re-exported.
- `src/app/search_dispatch.rs` — replace two `cx.spawn(...)` blocks
  with `present_command(...)`; remove now-unused
  `fetch_recent_feed_result_rows` / `fetch_index_search_result_rows`
  free fns if no other caller remains (`grep` first).
- `tests/architecture_tests.rs` — update
  `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
  baseline: reduce `src/app/search_dispatch.rs` from 3 to 1.

Should NOT change:

- `src/library/`, `src/discover/` — later tasks.
- The presentation bridge.
- `CommandBus` / `AsyncCommandRunner`.
- `src/app/search_dispatch.rs:577` — Task 004.

## Do Not Touch

- `src/presentation/`.
- `src/library/`, `src/discover/`.
- The feature flag work (already retired).
- The site at `src/app/search_dispatch.rs:577` (thumbnail fetch) — defer.

## Constraints

- The new commands implement `ApplicationCommand` with `execute(self,
  context: &CommandContext) -> CommandResult<Self::Output>`. The
  `execute` body calls the existing fetcher logic; do not duplicate
  HTTP code.
- The `Output` types match the existing call-site contract. For
  `FetchRecentFeedsPage::Output`, this is whatever
  `fetch_recent_feed_result_rows` currently returns (probably
  `RecentFeedsPageBatch` or `Result<RecentFeedsPageBatch>` unwrapped).
  Read both call sites' on-success arms (`detail.finish_load(batch, ...)`,
  `detail.replace_index_results(rows)`) to choose the right output type.
- Call sites become a single-token replacement matching the ADR 0040
  Task 001 pattern: `cx.spawn(...) {...}` becomes
  `present_command(&self.command_runner, command, CommandContext::next(),
  cx, on_success, on_error)`.
- Callbacks must run on the GPUI thread. The presenter already
  guarantees this via `weak.update`.
- The `request_type` / `request_query` / `update_query` closure
  captures in the existing spawn blocks belong inside the
  `on_success` / `on_error` closures, not inside the command. The
  command itself only carries inputs needed by `execute`.
- The two new commands accept a `Client`-equivalent endpoint string
  argument the way the free fns do. Do not invent a new transport
  layer.
- Behavior preservation: the existing `recent_feeds_detail.is_none()`
  re-init guard in the on-success callback, the
  `content_list_nav_is_recent_feeds()` / `content_list_nav_matches_search(...)`
  staleness guards, and the eager-prefetch follow-up
  (`this.start_recent_feeds_load(true, cx)`) must remain functionally
  identical post-migration.
- No new `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read the parent plan, this task, the bridge, and the two spawn
   sites in full.
2. Decide query struct shapes:
   - `FetchRecentFeedsPage { endpoint: String, cursor: Option<String>,
     resume_after: usize }`. Output: same type
     `fetch_recent_feed_result_rows` returns today (call it
     `RecentFeedsPageBatch` — find the actual type in
     `fetch_recent_feed_result_rows`'s signature).
   - `FetchIndexSearchResults { endpoint: String, query: String }`.
     Output: same as `fetch_index_search_result_rows`.
3. Define both commands in their queries-folder files. Each `execute`
   body calls the existing fetcher fn — keep the fetcher fn private to
   `src/app/search_dispatch.rs` if it has no other caller, or move it
   into the queries module if cleaner. Prefer moving to keep the
   queries folder self-contained.
4. Update `src/app/search_dispatch.rs:142` (Recent Feeds load):
   - Replace the `cx.spawn(...)` block with
     `present_command(&self.command_runner, FetchRecentFeedsPage { ... },
     CommandContext::next(), cx, on_success, on_error)`.
   - `on_success` body: the existing post-await block
     (`if !this.content_list_nav_is_recent_feeds() { return; }`,
     `if this.recent_feeds_detail.is_none() { ... }`,
     `detail.finish_load(batch, append)`, eager prefetch).
   - `on_error` body: the `Err(error)` arm
     (`detail.fail_load("Recent Feeds unavailable", format!("{error:#}"),
     append)`). Note that the existing code wraps both arms in a
     single match — the bridge separates them. Decide whether to keep
     a shared closure that takes a `Result` (call `present_command`
     with on_success/on_error that branch to the same fn), or split
     cleanly. Split cleanly unless duplication exceeds ~10 LOC.
5. Update `src/app/search_dispatch.rs:213` (Index search) the same
   way, using `FetchIndexSearchResults`.
6. Run `cargo build`. Fix any type / lifetime errors before continuing.
7. Update the architecture-test baseline:
   - In `tests/architecture_tests.rs`, find the `baseline` BTreeMap
     in `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
     (around line 10512).
   - Change `("src/app/search_dispatch.rs", 3)` to
     `("src/app/search_dispatch.rs", 1)`. The remaining 1 is line 577
     (thumbnail), retired in Task 004.
8. If the fetcher free fns are no longer called anywhere, remove them
   from `src/app/search_dispatch.rs`. Check first:
   `grep -rn "fetch_recent_feed_result_rows\|fetch_index_search_result_rows" src/`.
9. Run all five gates.
10. Smoke: launch the app, hit the Recent Feeds toolbar button, submit
    a global search. Both formerly-spawn-driven flows should behave
    identically.

## Acceptance Criteria

- `grep -n "cx\.spawn(" src/app/search_dispatch.rs` returns one hit
  (line ~577, thumbnail — deferred to Task 004).
- `FetchRecentFeedsPage` and `FetchIndexSearchResults` exist as
  `ApplicationCommand`-implementing structs under
  `src/application/queries/`.
- `present_command` is called from `src/app/search_dispatch.rs`
  in place of the two retired spawn blocks.
- `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
  baseline for `src/app/search_dispatch.rs` is 1.
- `cargo build`, `cargo test --lib`, `cargo test --test architecture_tests`,
  `cargo clippy -- -D warnings`, `cargo fmt --check` all pass.
- No new `#[allow(...)]`.
- No behavior change at either migrated site.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded migration task — first of seven in the
screen-local `cx.spawn` retirement plan.

Read in order:

1. This task file in full.
2. `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
   (Decision, Command Placement, Invariants).
3. `src/presentation/async_command_presenter.rs` (bridge signature).
4. `src/application/command_bus.rs:53` (`ApplicationCommand` trait).
5. `src/application/queries/feed.rs` and
   `src/application/queries/search.rs` (existing query shapes).
6. `src/app/search_dispatch.rs:100-260` and `:1200-1290` (sites + fetcher fns).

Goal:

Add two `ApplicationCommand` query variants — `FetchRecentFeedsPage`
and `FetchIndexSearchResults` — to `src/application/queries/feed.rs`
and `src/application/queries/search.rs`. Each `execute` calls the
existing fetcher logic. Migrate the two `cx.spawn` sites at
`src/app/search_dispatch.rs:142` (Recent Feeds load) and `:213` (Index
search) to `present_command(...)`. Preserve all existing on-success /
on-error behavior, including the staleness guards and eager prefetch.
Update the `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
baseline for `src/app/search_dispatch.rs` from 3 to 1 (the remaining
hit at line 577 is for Task 004).

Constraints:

- One-to-one substitution at call sites.
- No behavior change.
- No `#[allow(...)]`.
- No touching `src/library/`, `src/discover/`, the bridge, or the
  feature flag.
- Don't migrate `:577` — Task 004 owns it.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. New command struct names, file paths, output types.
2. Lines migrated (file:line before → call-site shape after).
3. Whether `fetch_recent_feed_result_rows` / `fetch_index_search_result_rows`
   free fns were deleted or relocated.
4. The architecture-guard baseline diff.
5. Five-gate results.
6. Deviations + unresolved concerns.

## Escalation Triggers

- The fetcher fn signature requires arguments the new command can't
  carry (e.g., a non-Send borrow). Report; the bridge requires Send
  inputs. The right path is usually cloning at the boundary.
- A call site closure mutates state in a way that doesn't survive
  the bridge's `weak.update` re-entry (e.g., a borrow that lives
  across the await). Report; the bridge guarantees the callback runs
  inside `weak.update`, so any borrow that compiles inside an existing
  `weak.update` body will compile inside the bridge callback. If it
  doesn't, the original spawn block was probably playing tricks; flag
  it.
- The fetcher fn does something more than a single HTTP request (e.g.,
  the Index search fetcher fans out to multiple endpoints). Report
  the fan-out; one command can still wrap a multi-step `execute` body,
  but the structure should be visible in the report.
- `Self::Output` for one of the commands is awkwardly large or
  involves GPUI types. Report; the bridge requires `Send + Sync +
  'static`. Strip GPUI types or downgrade to plain data.
