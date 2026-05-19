# ADR 0040 Screen-Local `cx.spawn` Retirement Plan

## Status

Completed - 2026-05-18. Follow-up to the ADR 0040 legacy scheduling
retirement (Tasks 001-004, completed 2026-05-18). The residual
"screen-local `cx.spawn` retirement" item from
`docs/plans/deferred-architecture-work-index.md` is closed, and
ADR 0040's Status block now points to the strict
`cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap` guard.

## Goal

Retire every remaining `cx.spawn` call site outside
`src/presentation/` and `src/runtime/` by routing the underlying work
through either the presentation bridge (one-shot commands) or a runtime
actor (recurring / multi-step flows). After this plan lands, the
`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime` guard
can be replaced with a strict allowlist that names only `src/app/bootstrap.rs`
(window-lifecycle initialization, not domain work).

## Context

The ADR 0040 Task 004 retirement deleted `GpuiCommandRunner` and the
`async-runtime` Cargo feature, but a pre-existing baseline of 28 `cx.spawn`
calls survived because they were never routed through the legacy command
runner in the first place. They are pinned today by
`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`
(`tests/architecture_tests.rs:10511`) which permits the baseline but
forbids growth.

Baseline today (verified by guard fixture):

| File                              | Sites | Category                                     |
|-----------------------------------|------:|----------------------------------------------|
| `src/app.rs:321`                  | 1     | Playback polling (1Hz heartbeat)             |
| `src/app/bootstrap.rs:135`        | 1     | Window-activation timing (pure GPUI)         |
| `src/app/search_dispatch.rs:142`  | 1     | Recent Feeds page fetch                      |
| `src/app/search_dispatch.rs:213`  | 1     | Index search fetch                           |
| `src/app/search_dispatch.rs:577`  | 1     | Inspector remote thumbnail fetch             |
| `src/library/app_impl.rs:524`     | 1     | Library tracks tree load                     |
| `src/library/app_impl.rs:1052`    | 1     | Library thumbnail fetch                      |
| `src/library/app_impl.rs:1185`    | 1     | Album identity hydration                     |
| `src/library/app_impl.rs:1467`    | 1     | Track context with local fallback            |
| `src/library/app_impl.rs:1875`    | 1     | ID3 edit apply + compare                     |
| `src/library/app_impl.rs:2050`    | 1     | Track tag compare                            |
| `src/library/app_impl.rs:2103`    | 1     | Track tag compare (duplicate caller)         |
| `src/library/app_impl.rs:2279`    | 1     | MusicBrainz album + per-track saga           |
| `src/discover/app_impl.rs:126`    | 1     | Discover recent feeds                        |
| `src/discover/app_impl.rs:250`    | 1     | Discover search (library + index merge)      |
| `src/discover/app_impl.rs:354`    | 1     | Discover thumbnail fetch                     |
| `src/discover/app_impl.rs:435`    | 1     | Inspector detail fetch                       |
| `src/discover/app_impl.rs:544`    | 1     | Local track context fetch                    |
| `src/discover/app_impl.rs:627`    | 1     | Inspector image download                     |
| `src/discover/app_impl.rs:676`    | 1     | Contributors fetch                           |
| `src/discover/app_impl.rs:727`    | 1     | Value routes fetch                           |
| `src/discover/app_impl.rs:754`    | 1     | Podroll feed resolve                         |
| `src/discover/app_impl.rs:822`    | 1     | Contributors fetch (duplicate caller)        |
| `src/discover/app_impl.rs:882`    | 1     | Value routes fetch (duplicate caller)        |
| `src/discover/app_impl.rs:1016`   | 1     | ID3 edit apply + compare (mirror of library) |
| `src/discover/app_impl.rs:1817`   | 1     | Download-and-compare track                   |
| `src/discover/app_impl.rs:1866`   | 1     | Download-and-compare track (force variant)   |
| `src/discover/app_impl.rs:1922`   | 1     | MusicBrainz track lookup (existing command)  |
| **Total**                         | **28**|                                              |

Five categories emerge:

- **Domain fetches (24 sites).** A screen-local free function performs
  one HTTP / DB request; the spawn block awaits it and writes the result
  back via `weak.update`. Shape is exactly what `present_command` was
  built for. Each site needs a typed `ApplicationCommand` variant
  (most don't have one yet) and a single-token call-site substitution.
- **Image cache fetches (4 sites: `search_dispatch.rs:577`,
  `library/app_impl.rs:1052`, `discover/app_impl.rs:354`,
  `discover/app_impl.rs:627`).** Fetch an image into a per-screen
  `thumbnails` map. Not domain work in the ADR 0040 sense, but the spawn
  shape is identical. Two viable retirement paths: (a) typed
  `FetchThumbnail` command + bridge, (b) dedicated image-cache actor
  with VmBus invalidations. Worth a focused decision packet.
- **Multi-step saga (1 site: `library/app_impl.rs:2279`).** The
  MusicBrainz album-then-per-track lookup is a stateful sequence of
  fetches interleaved with progressive `vm.begin_*_stage(...)` UI
  updates. `present_command` can't express a saga; needs an Actor.
- **Recurring timer (1 site: `app.rs:321`).** 1Hz playback driver
  polling. The right shape is a runtime actor publishing snapshots via
  VmBus.
- **Window-lifecycle (1 site: `bootstrap.rs:135`).** A 16ms + 100ms
  defer that nudges GPUI to refresh windows during startup. Not domain
  work. The cleanest outcome is an explicit allowlist exception with a
  comment naming the GPUI quirk it works around.

## Decision

Seven sequential bounded tasks, sized for sonnet subagents. Each ships
green under the five gates before the next starts.

1. **Task 001 — `src/app/search_dispatch.rs` migration.** Smallest
   surface. Define `FetchRecentFeedsPage` and `FetchIndexSearchResults`
   commands (or queries-as-commands; see *Command Placement* below).
   Migrate 2 of 3 sites; defer the thumbnail site to Task 004.
2. **Task 002 — `src/library/app_impl.rs` migration.** Define ~5 library
   fetch commands. Migrate 6 of 8 sites; defer thumbnail (1052) to
   Task 004 and the MusicBrainz saga (2279) to Task 005.
3. **Task 003 — `src/discover/app_impl.rs` migration.** Reuse Task 002
   commands where applicable; define remaining ~6 fetch commands.
   Migrate 13 of 15 sites; defer 2 image sites (354, 627) to Task 004.
4. **Task 004 — Image cache strategy + migration.** Decide
   command-vs-actor, document, and migrate all 4 image sites.
5. **Task 005 — MusicBrainz feed saga actor.** Introduce a saga actor
   in `src/runtime/` that emits progress events; replace the inline
   saga at `library/app_impl.rs:2279`.
6. **Task 006 — Playback polling actor.** Replace `app.rs:321` 1Hz
   polling with an actor that publishes playback snapshots via VmBus.
7. **Task 007 — Bootstrap exemption + strict guard + ADR refresh.**
   Document `bootstrap.rs:135` as window-lifecycle exempt with an
   inline comment; replace the debt-baseline guard with a strict
   allowlist; update ADR 0040 status to drop the caveat; move the
   deferred-index item to "Recently Resolved".

Tasks 001 → 003 are largely mechanical (define command, substitute call
site). Tasks 004 → 006 are engineering. Task 007 is bookkeeping but
gated by 001-006 leaving zero non-presentation/runtime spawns outside
`bootstrap.rs`.

## Command Placement

The new fetch commands live in the existing `src/application/commands/`
and `src/application/queries/` folders, grouped by domain:

- `src/application/queries/feed.rs` — `FetchRecentFeedsPage`,
  `FetchInspectorDetail`, `FetchContributors`, `FetchValueRoutes`,
  `ResolvePodrollFeeds`.
- `src/application/queries/search.rs` — `FetchIndexSearchResults`,
  `FetchDiscoverSearchResults`.
- `src/application/queries/library.rs` — `LoadLibraryTracksTree`,
  `FetchLibraryTrackContext`, `FetchLocalTrackContext`,
  `HydrateAlbumIdentity`, `CompareLibraryTrack`.
- `src/application/commands/metadata.rs` — `ApplyTrackId3Edits`,
  `DownloadAndCompareTrack`. (Mutations — these live in `commands/`.)

Pure reads ("queries") still implement `ApplicationCommand` so the
bridge accepts them; `ApplicationCommand` is the bridge's input type
and is not exclusive to mutations. The folder split is conventional
(read = `queries/`, write = `commands/`), not load-bearing.

## Invariants (additions to ADR 0040)

After Task 007 lands:

- The only `cx.spawn` call sites outside `src/presentation/` and
  `src/runtime/` are an allowlisted set in
  `src/app/bootstrap.rs` for GPUI window-lifecycle setup. Every other
  domain-shaped spawn has been retired.
- `cx_spawn_debt_does_not_grow_outside_presentation_and_runtime` is
  replaced by `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`
  which fails on any non-allowlist hit.
- The presentation bridge (`present_command`) remains the sole
  command/result GPUI bridge. New screen-bound fetches MUST route
  through it.
- Recurring loops (timers, polling) MUST live as actors in
  `src/runtime/` that publish via VmBus or `watch::Sender`. Screens
  subscribe; they do not spawn.
- Multi-step sagas with interleaved UI updates MUST live as actors that
  emit progress events. Screens render the published state.

## Non-Goals

- No retirement of `CommandBus`. Synchronous bus stays for CLI / tests
  / domain-level usage per ADR 0040.
- No reorganization of `src/application/` beyond adding the new fetch
  commands / queries.
- No unparking of `src/discover/`. Task 003 migrates in place; the
  parked-module note still applies. (Mirrors Task 003 of the original
  ADR 0040 retirement.)
- No behavior change at any call site. Every migration is mechanical
  on-success / on-error preservation through the bridge.
- No new HTTP / DB code paths. Migrations call the same underlying
  service functions; only the scheduler changes.
- No CI workflow creation (separate concern, mirroring ADR 0040 plan).
- No retroactive policy on image-cache callers — Task 004's outcome
  governs that.

## Risk Areas

- **Command surface proliferation.** Defining ~15 new commands in
  `src/application/` adds layer LOC. Mitigation: each fetch is a tiny
  struct + one-fn `execute` body that calls the existing service /
  service helper. Aim for ≤ 30 LOC per command and prefer to colocate
  related commands in one file.
- **Closure shape variance.** Several call sites pass closures that
  reach into screen-private state (e.g., `inspector_stack.last_mut()`
  filter, `selected_track_frame_mut().entity_id == entity_id` guard).
  `present_command` runs the callback via `weak.update`; any borrow
  pattern that compiles inside the existing spawn block also compiles
  there. Per-site adaptation expected only when closures capture an
  `Arc` they would otherwise move into the spawn — pass through the
  bridge boundary instead.
- **Discover module size.** Task 003 migrates 13 sites inside a parked
  module. The `discover_module_public_surface_is_pinned` guard must
  continue to pass; no new public symbols should leak.
- **Saga refactor (Task 005) is non-mechanical.** The MusicBrainz
  album-then-per-track flow has progressive UI updates
  (`vm.begin_musicbrainz_album_track_stage(...)`) interleaved with
  per-track lookups, a fallback into per-track search on failure, and
  staging the final candidate. The actor must emit a discriminated
  event stream that the VM reduces. The right grain is one event per
  stage transition (Started, ProgressedTo(N, total), TrackMatched(...),
  CandidatesStaged(...), FellBackToPerTrack, Failed).
- **Playback polling actor (Task 006) coupling.** The current poll
  reads `PlaybackOwner` state under a `Mutex` lock. An actor needs to
  either own the `PlaybackOwner` (heavier rebuild) or call into it via
  a thread-safe interface. Task 006 spec must pick one before
  implementation.
- **Image cache decision (Task 004) deferral cost.** Tasks 001-003
  defer 4 image sites. The guard baseline must still permit them while
  Task 004 is pending. Update the baseline numbers per file as each
  migration task lands.

## Proposed Sequence

```
Task 001 → 002 → 003       (mechanical migrations; reduce baseline)
              ↓
Task 004 (image)
              ↓
Task 005 (saga)
              ↓
Task 006 (polling)
              ↓
Task 007 (exemption + strict guard + ADR)
```

Tasks 001-003 can serialize at sonnet sub-agent pace (one packet each).
Tasks 004-006 are larger and may want opus-level review of the design
before sonnet implements. Task 007 is paperwork and a guard flip.

Per-task baseline math: after each migration task, update the
`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime` baseline
fixture to reflect the new lower count for the touched file. Guard
keeps pinning the residual until Task 007 replaces it.

## Verification

End-to-end after Task 007 lands:

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

Plus:

- `grep -rn "cx\.spawn" src/ | grep -vE 'src/(presentation|runtime|app/bootstrap)'`
  returns no hits.
- New guard `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`
  passes.
- ADR 0040 Status block no longer mentions the
  "broader screen-local cx.spawn cleanup is not complete" caveat.
- Deferred-index item #2 ("Screen-local `cx.spawn` retirement") moved
  to "Recently Resolved".
- Smoke: launch the app, run through every former spawn site at least
  once — Recent Feeds load + paginate, global search, Library tree
  reload, Library album view (triggers identity hydration), Library
  track context view, ID3 edit apply, tag compare, MusicBrainz album
  lookup, playback play/pause (polling), startup window activation
  (bootstrap) — each should behave identically.

## References

- ADR 0040 — `docs/adr/0040-async-vm-runtime.md`
- ADR 0040 legacy retirement plan — `docs/plans/adr-0040-legacy-scheduling-retirement-plan.md`
- Deferred index — `docs/plans/deferred-architecture-work-index.md` (item #2)
- Bridge — `src/presentation/async_command_presenter.rs`
- Runner — `src/application/async_command_runner.rs`
- Guard — `tests/architecture_tests.rs:10511`
  (`cx_spawn_debt_does_not_grow_outside_presentation_and_runtime`)
- Tasks — `docs/tasks/adr-0040-spawn-task-{001..007}-*.md`
