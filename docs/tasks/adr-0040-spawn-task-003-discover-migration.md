# ADR 0040 Spawn Task 003 — `src/discover/app_impl.rs` Migration

Status: Completed - 2026-05-18.

## Goal

Retire 13 of the 15 `cx.spawn` sites in the parked Discover module by
introducing remaining typed fetch / mutation commands and routing
through `present_command`. The two skipped sites are line 354 and line
627 (thumbnail / image fetches), deferred to Task 004.

The Discover module is "compiled but unreachable" per
`docs/notes/2026-05-discover-module-parked.md`. Migrate in place; do
not unpark, do not add new public symbols, do not change call-graph
reachability from the composition root.

Plan reference: `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`.
Prerequisites: Tasks 001 and 002 landed.
`FetchIndexSearchResults`, `ApplyTrackId3Edits`, and (for site 1922)
the existing `LookupMusicBrainzTrack` can be reused.

## Files To Inspect

Required:

- `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
  (Decision, Command Placement, Risk Areas).
- `docs/notes/2026-05-discover-module-parked.md` (full — important
  parked-module context).
- `src/presentation/async_command_presenter.rs` (bridge).
- One migrated call site under `src/library/app_impl.rs` post-Task-002
  (any of `:524, :1185, :1467, :1875, :2050`) — diff shape reference.
- `src/application/commands/metadata.rs` — `LookupMusicBrainzTrack`
  shape (existing; used by site 1922).
- `src/application/queries/feed.rs`, `queries/search.rs`, and
  `queries/library.rs` after Tasks 001-002 — see what's already
  available for reuse.
- `src/discover/app_impl.rs` — read the 13 spawn sites in context:
  **lines 126, 250, 435, 544, 676, 727, 754, 822, 882, 1016, 1817,
  1866, 1922**. (Also skim 354 and 627 to be sure they are image
  fetches — leave them for Task 004.)
- `src/discover/app_impl.rs` fetcher fns at lines 2425
  (`fetch_inspector_detail`), 2588 (`resolve_podroll_feeds`), 2679
  (`download_and_compare_track`), 2739 (`lookup_musicbrainz_track`).
- `src/api.rs:533` (`fetch_value_routes` — `Client` method used by
  several sites).
- `tests/architecture_tests.rs` — the
  `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
  baseline (`("src/discover/app_impl.rs", 15)` → expected
  `("src/discover/app_impl.rs", 2)` after this task) AND the
  `discover_module_public_surface_is_pinned` guard.

Reference only:

- `src/library/app_impl.rs` post-Task-002 — diff shape.

## Files Likely To Change

- `src/application/queries/feed.rs` — add (if not already present):
  - `FetchDiscoverRecentFeeds` (or reuse `FetchRecentFeedsPage` if
    inputs align — they likely do).
  - `FetchInspectorDetail`.
  - `FetchContributors`.
  - `FetchValueRoutes`.
  - `ResolvePodrollFeeds`.
- `src/application/queries/search.rs` — add `FetchDiscoverSearchResults`
  (multi-step library + index merge — wraps the `:250` block body).
- `src/application/queries/library.rs` — add `FetchLocalTrackContext`
  (covers `:544`).
- `src/application/commands/metadata.rs` — `DownloadAndCompareTrack`
  (covers `:1817` and `:1866`; the `force_download` flag becomes a
  struct field).
- `src/application/commands/metadata.rs` — confirm
  `ApplyTrackId3Edits` from Task 002 works for `:1016` (same
  underlying logic — `write_id3v24_edits` + `compare_downloaded_track_path`).
  Reuse, don't duplicate.
- `src/discover/app_impl.rs` — replace 13 `cx.spawn(...)` blocks with
  `present_command(...)`. The composition root field type is already
  `AsyncCommandRunner` from the ADR 0040 retirement.
- `tests/architecture_tests.rs` — baseline:
  `("src/discover/app_impl.rs", 15)` → `("src/discover/app_impl.rs", 2)`.

Should NOT change:

- The Discover module's public surface (per
  `discover_module_public_surface_is_pinned` guard).
- Any file under `src/ui/shells/discover/`.
- The parked-module note unless its call-graph snapshot lists a fetcher
  fn that gets relocated as part of this task. In that case, update
  one line in the note. Otherwise leave untouched.

## Do Not Touch

- `src/discover/app_impl.rs:354, :627` — Task 004 (image cache).
- `src/library/`, `src/app/`, `src/presentation/`.
- The feature flag (already retired).
- `LookupMusicBrainzTrack` definition (only the call site moves).

## Constraints

- One-to-one signature replacement at call sites. Match Task 002's
  pattern.
- No behavior change. Discover stays compiled and unreachable.
- The migration does not unpark Discover. No new render path; no
  symbol becomes publicly visible.
- For overlapping commands across library and discover (e.g., the
  ID3 apply on `:1016` mirrors library `:1875`), reuse the existing
  command. Confirm the input/output shape works for both call sites
  before deciding to share; if the inputs diverge, two commands is
  fine.
- The `:250` Discover search is a multi-step block: local library
  rows + index search merge. The new
  `FetchDiscoverSearchResults::execute` body wraps the existing
  block's body verbatim. Output: the same struct the existing block
  produces.
- The `:435` `fetch_inspector_detail` already takes `&Client` plus
  three string args. The command struct mirrors that.
- The `discover_module_public_surface_is_pinned` guard must continue
  to pass. If the field type or any pub item visibility shifts, the
  guard catches it; update the fixture only if the surface change is
  intended (it shouldn't be — only private field types may change).
- No new `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read this task, the parent plan, the parked-module note, and the
   13 spawn sites in `src/discover/app_impl.rs`.
2. Cross-reference each site against existing commands from Tasks 001
   and 002:
   - `:126` Discover recent feeds → reuse `FetchRecentFeedsPage`
     (Task 001) if the input shape matches (endpoint + cursor + limit);
     otherwise add `FetchDiscoverRecentFeeds`.
   - `:250` Discover search → new `FetchDiscoverSearchResults`.
   - `:435` Inspector detail → new `FetchInspectorDetail`.
   - `:544` Local track context → new `FetchLocalTrackContext`
     (or reuse `FetchLibraryTrackContext` if the shape matches).
   - `:676`, `:822` Contributors → new `FetchContributors`.
   - `:727`, `:882` Value routes → new `FetchValueRoutes`.
   - `:754` Podroll feeds → new `ResolvePodrollFeeds`.
   - `:1016` ID3 apply → reuse `ApplyTrackId3Edits` (Task 002).
   - `:1817`, `:1866` Download-and-compare → new
     `DownloadAndCompareTrack` (`force` field carries the `:1866`
     variant).
   - `:1922` MusicBrainz track → reuse existing `LookupMusicBrainzTrack`.
3. Define each new command. Each `execute` calls the existing fetcher
   fn (`fetch_inspector_detail`, `resolve_podroll_feeds`,
   `download_and_compare_track`, `lookup_musicbrainz_track`,
   `Client::fetch_contributors`, `Client::fetch_value_routes`).
4. After each command lands, `cargo build`. Then migrate the call
   sites that use it.
5. Walk the 13 call sites. For each, substitute the bridge call.
   On-success / on-error closures preserve the existing staleness
   guards (e.g., `if let Some(frame) = this.inspector_stack.last_mut()`
   + `frame.entity_type == entity_type` etc.).
6. After every ~4 sites: `cargo build`.
7. Confirm `discover_module_public_surface_is_pinned` still passes.
   If it fails because a relocated fetcher fn changed visibility,
   inspect the fixture and update only with the same reasoning the
   original fixture lists.
8. Update the architecture-test baseline:
   `("src/discover/app_impl.rs", 15)` → `("src/discover/app_impl.rs", 2)`.
9. Run all five gates.

## Acceptance Criteria

- `grep -n "cx\.spawn(" src/discover/app_impl.rs` returns two hits
  (line ~354 and line ~627, deferred to Task 004).
- The new and reused commands cover all 13 migrated sites.
- Reused commands (`FetchRecentFeedsPage`, `ApplyTrackId3Edits`,
  `LookupMusicBrainzTrack`) are not duplicated.
- Baseline `("src/discover/app_impl.rs", 2)` and the guard passes.
- `discover_module_public_surface_is_pinned` passes without fixture
  changes (unless a relocated fn forced one one-line update —
  document if so).
- All five gates pass.
- No new `#[allow(...)]`.
- Discover remains compiled-but-unreachable.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded migration task — third of seven in
the screen-local `cx.spawn` retirement plan.

Prerequisite: Tasks 001 and 002 landed.
`FetchRecentFeedsPage`, `FetchIndexSearchResults`, library commands,
and `ApplyTrackId3Edits` exist. The existing
`LookupMusicBrainzTrack` is available for reuse at site 1922.

Important context: `src/discover/` is **parked** per
`docs/notes/2026-05-discover-module-parked.md`. It is compiled but
not reachable from the composition root. The migration is required to
remove screen-local spawns but must not unpark the module.

Read in order:

1. This task file in full.
2. `docs/notes/2026-05-discover-module-parked.md` (full).
3. `docs/plans/adr-0040-screen-local-cx-spawn-retirement-plan.md`
   (Decision, Command Placement).
4. `src/presentation/async_command_presenter.rs`.
5. One migrated library call site (any from Task 002).
6. `src/discover/app_impl.rs` — the 13 spawn sites and the four
   fetcher fns at lines 2425, 2588, 2679, 2739.

Goal:

Define remaining `ApplicationCommand` variants (`FetchInspectorDetail`,
`FetchContributors`, `FetchValueRoutes`, `ResolvePodrollFeeds`,
`FetchDiscoverSearchResults`, `FetchLocalTrackContext`,
`DownloadAndCompareTrack`; reuse where possible). Migrate the 13
spawn sites at `src/discover/app_impl.rs:126, 250, 435, 544, 676,
727, 754, 822, 882, 1016, 1817, 1866, 1922` to `present_command(...)`.
Discover stays compiled and unreachable; no new public symbols, no
new render path. The `discover_module_public_surface_is_pinned` guard
must still pass.

Update the
`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
baseline for `src/discover/app_impl.rs` from 15 to 2.

Constraints:

- One-to-one substitution.
- No behavior change.
- No unpark — public surface unchanged.
- No `#[allow(...)]`.
- Don't touch `:354` or `:627`; Task 004 owns those.
- Don't touch other modules.
- Never skip hooks. Don't commit.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. Reused vs new commands (list).
2. 13 migrated call sites (file:line, terse).
3. Baseline diff.
4. Whether `discover_module_public_surface_is_pinned` needed any
   update; if yes, what + why.
5. Five-gate results.
6. Deviations + unresolved concerns.

## Escalation Triggers

- A fetcher fn (e.g., `Client::fetch_contributors`) requires the
  `Client` value, which `execute(self, &CommandContext)` would need to
  own. Either the command carries a `Client` field (cloned at the
  boundary), or the `execute` body resolves a `Client` from
  `CommandContext`'s service references. Pick the cleaner path; report
  the choice.
- A call site's on-success closure references private state that
  doesn't survive `weak.update` re-entry. Report; the bridge
  guarantees `weak.update` re-entry but a missed borrow can still
  surface as a panic.
- `discover_module_public_surface_is_pinned` fails after the
  migration. Inspect what surface drifted and report. Do not soften
  the guard; the fix is to restore visibility, not to weaken the
  pin.
- The `:1817` and `:1866` sites take subtly different inputs beyond
  the `force_download: bool` flag. Report; the right move is usually
  one struct with two boolean fields rather than two commands.
