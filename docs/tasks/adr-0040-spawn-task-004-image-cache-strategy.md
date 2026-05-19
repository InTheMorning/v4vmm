# ADR 0040 Spawn Task 004 — Image Cache Strategy + Migration

Status: Completed - 2026-05-18.

## Goal

Retire the four `cx.spawn` sites whose work is image-cache fetching:

- `src/app/search_dispatch.rs:577` — remote inspector thumbnail.
- `src/library/app_impl.rs:1052` — library thumbnail (with animated
  fallback toggle).
- `src/discover/app_impl.rs:354` — discover thumbnail (animated).
- `src/discover/app_impl.rs:627` — inspector image download.

Each spawn block runs `image_cache.fetch_*_blocking(&url)` on the
background executor and writes the resulting `Arc<Image>` into a
per-screen thumbnails map keyed by URL. Behavior is the same shape as
a domain fetch, but the work is presentation infrastructure, not
domain.

This task **first decides** between two retirement strategies, **then**
migrates all four sites under the chosen strategy.

Plan reference: `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.
Prerequisites: Tasks 001, 002, and 003 landed; only image-cache sites
remain in `src/app/`, `src/library/`, and `src/discover/`.

## Strategy Options

**Option A — Typed `FetchThumbnail` command + presentation bridge.**

Define `FetchThumbnail { url: String, animated: bool }` with output
`Option<Arc<Image>>`. `execute` calls
`image_cache.fetch_blocking` or `image_cache.fetch_static_blocking`
based on `animated`. Migrate all four sites via `present_command(...)`.

Pros: consistent with domain-fetch retirement pattern; no new actor;
zero new runtime concepts.
Cons: stretches "command" to cover pure cache-population (no service
side effects, no domain identity). Treats `image_cache` as a hidden
dependency injected via `CommandContext` or directly cloned into the
command struct.

**Option B — Dedicated `ImageCacheActor` + per-screen
`watch::Receiver` map.**

Spawn one actor at app boot owning the `image_cache`. Screens send
`FetchThumbnail { url, animated }` messages; the actor publishes the
resulting image via a per-URL `watch::Sender<Option<Arc<Image>>>` (or
a single `broadcast` channel keyed by URL). Screens hold a `Receiver`
and read on render.

Pros: image caching becomes a proper runtime concept; coalescing
across screens (if two screens want the same URL, one fetch); explicit
backpressure if needed; matches ADR 0040 "actors own state, snapshots
flow via `watch`" rule.
Cons: heavier scaffolding for what is essentially a get-or-fetch
cache; requires new VM-bus event type or new channel topology;
behavior change at the boundary (per-URL deduplication wasn't a
feature before — though it's strictly better).

**Recommendation:** Option A for this task, with a follow-up note in
the deferred-index pointing at Option B if image-cache backpressure
or cross-screen coalescing become real needs. Option A is the smallest
mechanical change consistent with the migration pattern; Option B is
a real architectural addition that deserves its own ADR.

The implementer should confirm Option A with the user (or explicitly
choose Option A and report the rationale). Do not implement Option B
in this task without prior user agreement.

## Files To Inspect

Required:

- `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.
- `src/presentation/async_command_presenter.rs`.
- The image cache definition: `grep -rn "fetch_blocking\|fetch_static_blocking"
  src/` to locate the cache type (likely under
  `src/infrastructure/` or `src/services/`).
- All four spawn sites in context.
- `src/application/command_bus.rs:53` — `ApplicationCommand` trait.
- `tests/architecture_tests.rs` — baselines for the three files.

Reference only:

- `src/application/queries/feed.rs` — example query command shape from
  earlier tasks.

## Files Likely To Change (Option A path)

- `src/application/queries/mod.rs` (or new
  `src/application/queries/images.rs`) — add `FetchThumbnail`.
- `src/app/search_dispatch.rs` — migrate `:577`.
- `src/library/app_impl.rs` — migrate `:1052`.
- `src/discover/app_impl.rs` — migrate `:354` and `:627`.
- `tests/architecture_tests.rs` — baselines:
  - `("src/app/search_dispatch.rs", 1)` → remove the entry (now 0).
  - `("src/library/app_impl.rs", 2)` → 1 (only `:2279` remains).
  - `("src/discover/app_impl.rs", 2)` → 0 (Discover is clean).

## Do Not Touch

- `src/library/app_impl.rs:2279` — Task 005 (saga).
- `src/app.rs:321` — Task 006 (polling).
- `src/app/bootstrap.rs:135` — Task 007 (window lifecycle).
- The image cache implementation. The new command wraps existing
  `fetch_blocking` / `fetch_static_blocking`; no new caching logic.
- `CommandBus` / `AsyncCommandRunner`.

## Constraints

- The `FetchThumbnail` command's `execute` is one branch on `animated`
  + one call to the cache.
- Output type: `Option<Arc<Image>>`. The current sites all match this
  shape (some return `Option<Arc<Image>>`, some return `Arc<Image>`
  directly — normalize to `Option<Arc<Image>>` in the command, and let
  the on-success closure handle `None`).
- The new command does NOT publish a `VmEvent` invalidation. Image
  fetches are screen-local presentation; broadcasting them would be
  noise.
- Each call site's on-success closure inserts into the per-screen
  thumbnails map keyed by the URL. Match the existing key shape (URL
  string, or `(url, animated)` for the library case).
- The `inspector_stack` / `selected_track_frame` staleness guards
  remain in the on-success closure, not in `execute`.
- No new `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read this task, the parent plan, and all four sites.
2. Confirm Option A with the user, or explicitly choose Option A and
   record the rationale in the final report.
3. Define `FetchThumbnail` in `src/application/queries/images.rs`
   (or alongside an existing file in `queries/` if there's a natural
   home — `feed.rs` is acceptable). `execute` body branches on
   `animated` and calls the cache.
4. Determine how `execute` accesses the `image_cache`. Two paths:
   - The command carries an `Arc<ImageCache>` field (or its
     interface type). Simplest; clone the `Arc` into the command at
     the call site.
   - `CommandContext` exposes the cache via `application_services`. If
     ApplicationServices already holds the cache, prefer this path; the
     command becomes `{ url: String, animated: bool }` and `execute`
     resolves the cache from the context.
   - Pick the path that fits the existing layer rules. Report the choice.
5. Migrate each call site:
   - `src/app/search_dispatch.rs:577` (single-arg static fetch).
   - `src/library/app_impl.rs:1052` (animated branch — pass
     `animated` boolean).
   - `src/discover/app_impl.rs:354` (animated fetch).
   - `src/discover/app_impl.rs:627` (uses
     `download_image(...).map(image_from_bytes)` — a slightly
     different shape; may need a separate `DownloadInspectorImage`
     command, or a flag on `FetchThumbnail`. Report and decide).
6. Update the architecture-test baseline. After this task,
   `src/discover/app_impl.rs` should be removed from the baseline
   (count 0), `src/app/search_dispatch.rs` removed (count 0), and
   `src/library/app_impl.rs` reduced to 1 (only `:2279`).
7. Run all five gates.
8. Smoke: trigger thumbnail rendering for an inspector card, a library
   row, and a Discover tile. Each should populate as before with no
   visible delay regression.

## Acceptance Criteria

- `grep -n "cx\.spawn(" src/app/search_dispatch.rs` returns no hits.
- `grep -n "cx\.spawn(" src/library/app_impl.rs` returns one hit
  (line ~2279).
- `grep -n "cx\.spawn(" src/discover/app_impl.rs` returns no hits.
- `FetchThumbnail` (and any sibling command for the inspector
  download case) exists under `src/application/queries/`.
- Baseline diffs reflect the new counts.
- All five gates pass.
- No new `#[allow(...)]`.
- Thumbnails render identically in inspector, library, and discover
  contexts.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded migration task — fourth of seven in
the screen-local `cx.spawn` retirement plan. This task carries a
strategy decision.

Prerequisites: Tasks 001-003 landed. The only remaining
non-presentation/runtime `cx.spawn` sites outside the planned
exemptions are four image-cache fetches.

Read in order:

1. This task file in full.
2. `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.
3. `src/presentation/async_command_presenter.rs`.
4. The four spawn sites (`src/app/search_dispatch.rs:577`,
   `src/library/app_impl.rs:1052`, `src/discover/app_impl.rs:354`,
   `src/discover/app_impl.rs:627`).
5. The image cache type (find with `grep -rn "fetch_blocking\|fetch_static_blocking"
   src/`).

Goal:

Choose between Option A (typed `FetchThumbnail` command via bridge)
and Option B (`ImageCacheActor` with `watch`-published snapshots).
Default to Option A unless the user has indicated otherwise. Document
the choice in the final report.

Implement the chosen strategy. Migrate all four sites. Update the
`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
baseline so that `src/app/search_dispatch.rs` and
`src/discover/app_impl.rs` are removed (zero) and
`src/library/app_impl.rs` drops to 1.

For Option A:

- Define `FetchThumbnail` (and, if necessary, a sibling for the `:627`
  download case) under `src/application/queries/`.
- Each command's `execute` calls the existing cache fns; no new
  caching logic.
- Per-site on-success closures preserve the existing thumbnails-map
  insertion and staleness guards.

Constraints:

- No behavior change at any site.
- No new `#[allow(...)]`.
- No publishing `VmEvent` invalidations from the new command(s).
- Don't touch sites outside the four listed.
- Don't introduce per-URL deduplication or coalescing unless the user
  has approved Option B.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. Chosen option (A or B) and rationale (one paragraph).
2. New command struct names, paths, output types.
3. Four migrated call sites (file:line before → after — terse).
4. How the image-cache reference reaches `execute` (carried in the
   struct vs resolved from `CommandContext`).
5. Baseline diff.
6. Five-gate results.
7. Deviations + unresolved concerns.

## Escalation Triggers

- The image cache's `fetch_blocking` signature is not Send / Sync.
  Report; the bridge requires `Send` outputs. The right path is
  usually narrowing what crosses the boundary (e.g., return
  `Vec<u8>` and convert to `Image` in the on-success closure).
- The four sites' output types are subtly different (e.g., one returns
  `Arc<Image>` and another returns `Option<Arc<Image>>`). Normalize to
  `Option<Arc<Image>>` and document.
- The `:627` site uses `download_image(...).map(image_from_bytes)`
  rather than the standard `image_cache.fetch_*_blocking`. Decide
  whether this is a different command (e.g., `DownloadInspectorImage`)
  or a `FetchThumbnail` variant. Report.
- The user requests Option B but the existing `ApplicationServices`
  composition doesn't have a clean home for the actor. Report the
  composition path; do not stub.
