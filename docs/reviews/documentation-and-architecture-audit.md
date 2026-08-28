# Documentation and Architecture Audit

## Reviewed Artifact

`docs/adr/` (56), `docs/plans/` (44), `docs/tasks/` (194), `docs/reviews/` (114),
`docs/plans/deferred-architecture-work-index.md`, and the `src/` tree (92,975
lines) as of 2026-08-28.

## Summary

The documentation system is unusually strong in design and unreliable in
practice. Governance artifacts exist for almost everything -- ADRs, phase plans,
task packets, review checklists, a deferred-work register, prohibited-fix
troubleshooting docs -- and the rules they state are good ones. The failure is
that status is not maintained, so the artifacts cannot be trusted without
reading the code they describe.

The code has two recurring defect classes the repository has already named, one
large block of deliberately dead code, four oversized modules, and no HTTP
timeouts anywhere.

Findings are ordered by how much they cost a reader or maintainer who is trying
to make a correct change.

## P1: ADR Status Does Not Reflect Reality

> Resolved 2026-08-28 by ADR 0057 and the accompanying normalization pass. All
> 57 ADR headers now conform to the four-value vocabulary. The findings below
> are retained as the record of what was wrong.

Eleven ADRs sit at `Proposed` while their own review checklists record
completion. Confirmed cases:

| ADR | ADR status | Review checklist says |
| --- | --- | --- |
| 0038 | `Proposed - 2026-05-03` | `Completed on 2026-05-04. Readiness decision: Proceed` |
| 0034 | `Proposed - 2026-05-02` | `Proceed - 2026-05-02` |
| 0035 | `Proposed - 2026-05-02` | `Proceed` |
| 0036 | `Proposed - 2026-05-02` | `Task 001, Task 002, and Task 003 complete` |

ADR 0038 is the clearest: `docs/plans/deferred-architecture-work-index.md:36`
states it "closed on 2026-05-04 with readiness gate `Proceed`", the checklist
agrees, and the ADR still says `Proposed`.

This violates the project's own Status Hygiene Rule, which requires ADR status,
phase plan, checklist, and deferred-index entry to be reconciled in the same
commit.

Cost: an ADR's status is the first thing a reader checks to know whether a
contract is live. Four of them are lying, so all of them have to be verified
against code. That is the exact cost the ADR system exists to remove.

## P1: No Defined Status Vocabulary

> Resolved 2026-08-28 by `docs/adr/0057-adr-status-vocabulary-and-amendment-policy.md`.

Seven distinct phrasings are in use across 56 ADRs: `Accepted`, `Implemented`,
`Accepted and implemented`, `Implemented for ADR 0047 scope`,
`Accepted - v1 implemented`, `Accepted - ... Implemented.`, and `Proposed`.

`docs/adr/0001-record-architecture-decisions.md` defines only `Accepted` and
`Superseded`. Nothing states whether `Accepted` means decided-but-unbuilt or
shipped. Both readings appear in practice: ADR 0030 is `Accepted` and ADR 0024
is `Accepted and implemented`, which implies `Accepted` alone does not mean
implemented -- but ADR 0051 is `Accepted - 2026-05-17. Implemented.`

Fix is cheap: define the vocabulary in ADR 0001 (or a superseding ADR) and
normalize the 56 headers once.

## P2: ADR 0001 Forbids The Amendment Practice The Project Uses

> Resolved 2026-08-28. ADR 0057 supersedes ADR 0001's immutability clause and
> defines a bounded in-place amendment policy.

ADR 0001 states: "Once accepted, an ADR is immutable except for status updates
such as `Superseded`."

In practice ADRs are amended in place. ADR 0035 records "Amended 2026-05-02 to
widen scope". ADR 0056 was amended substantially on 2026-08-28, including
reversing one of its own rejected alternatives. That amendment was mine, and it
broke this rule.

Either the immutability rule should be superseded to permit dated amendments, or
amendments should become new ADRs. The current state gives no guidance on which
is correct, so both happen.

## P2: Recurring Defect Class -- Render-Time Masking

`docs/troubleshooting/metadata-source-fact-regressions.md` exists because the
same wrong fix keeps getting made: patching renderers and display view models to
hide bad values instead of correcting the boundary that admitted them.

ADR 0056 is the same class in a different subsystem. Remote bytes were admitted
without validation, and the resulting corruption was visible only downstream.

A standing prohibition document is evidence a defect class recurs faster than it
gets designed out. The structural fix -- one owner per boundary, with an
architecture guard -- has been applied to media fetching and to seven ADRs
total. It has not been applied to the metadata boundaries this document covers.

## P2: Recurring Defect Class -- Stale View State

`docs/troubleshooting/immediate-view-state-regressions.md` documents UI state
that becomes correct only after leaving and returning to a view, and prohibits
fixing it by forcing navigation or actor respawns.

The document has to enumerate which cache might own the stale row -- page actor,
detail frame, sidebar tree, queue projection, search result snapshot. That
enumeration is the finding: cache invalidation ownership is diffuse enough that
a fixer has to search for the owner. No guard enforces it.

## P2: 7,596 Lines Of Dead Code Are Still Compiled

`src/discover.rs`, `src/discover/app_impl.rs`, `src/ui/shells/discover/*`,
`src/ui/shells/feed.rs`, and `src/ui/shells/track.rs` have been unreachable from
the composition root since ADR 0048. They are parked deliberately, with
documented reasons and explicit deletion conditions, in
`docs/notes/2026-05-discover-module-parked.md`.

The decision is defensible. The cost is not free and is worth stating: this is
roughly 8% of `src/`, and it is compiled, linted, and refactored alongside live
code. The ADR 0056 change on 2026-08-28 had to modify `src/discover.rs:225` and
`src/discover/app_impl.rs:553` for no reason other than keeping dead code
compiling.

Every cross-cutting change pays this tax, and the note records no scheduled
return date.

## P2: No HTTP Timeouts Anywhere

No `reqwest` client in `src/` sets a connect or read timeout. Roughly ten sites
construct clients independently: `rss/enrich.rs`, `rss/subscribe.rs`,
`musicbrainz.rs`, `api.rs`, `discover.rs`, `feed_service.rs`,
`application/commands/metadata.rs`, and others.

A feed host that accepts a connection and never responds stalls a blocking fetch
indefinitely. On the subscribe and refresh paths that is a hung operation with no
operator-visible cause.

This includes `src/remote_media.rs`, added on 2026-08-28. It centralized scheme,
redirect, and status policy for media fetches and did not add a timeout, so the
new module inherited the gap rather than closing it. It is now the natural place
to fix it for media; document fetches still have no shared owner.

## P3: Architecture Guards Cover 7 ADRs Of 56

`tests/architecture_tests.rs` contains named guards for ADRs 0024, 0042, 0047,
0048, 0049, 0055, and 0056. The invariants of the other 49 are enforced by review
discipline alone.

Review discipline is exactly what failed inside ADR 0056's own first
implementation: two of three remote fetches in one file were fixed and the third
was missed, with nothing to catch it. The guards added since would have caught
it.

This is not an argument for guarding all 49. It is an argument for guarding the
invariants whose violation is silent, which is the property the two
troubleshooting documents above describe.

## P3: Four Modules Carry 18% Of The Codebase

| File | Lines |
| --- | --- |
| `src/view_models/library.rs` | 5,837 |
| `src/db.rs` | 4,793 |
| `src/metadata.rs` | 4,011 |
| `src/library/app_impl.rs` | 3,009 |

ADR 0055 established module decomposition for the search view model. The same
treatment has not reached these four. `db.rs` in particular is the single owner
of all persistence, which several ADRs rely on as a boundary; its size makes
that boundary hard to audit.

## P3: Documentation Volume Outpaces Its Index

408 documents against 92,975 lines of source: 56 ADRs, 194 task packets, 114
reviews, 44 plans.

`docs/README.md` indexes roughly twenty of them. `docs/tasks/` and
`docs/reviews/` have no index at all, so a reader finds a packet by guessing its
filename convention. The convention is consistent, which is why this is P3 rather
than higher, but 194 packets is past the point where convention substitutes for
an index.

## P3: Stale Cross-References

- `docs/adr/0048-content-list-frame-breadcrumb-search.md:5` read "Implemented in
  commits TBD". Resolved 2026-08-28: the ADR 0048 architecture guards are the
  durable evidence, and the status now cites them.
- `docs/plans/deferred-architecture-work-index.md:93` refers to "Deferred item
  #2, Library/Discover data parity", but current item #2 is "Staged metadata
  durability". The priority list was renumbered and the historical reference now
  points at the wrong item. Numbered lists referenced by number will keep
  drifting; referring to items by name would not.

## Not Problems

Worth recording so they are not re-investigated:

- Production `TODO`/`FIXME` debt is essentially zero. One marker outside test
  code, in `src/app/tab_bar.rs`.
- Panic surface is small. Of 460 `unwrap`/`expect` occurrences, the large
  concentrations are inside `mod tests` (all 60 in
  `src/application/paged_track_list.rs`). The handful in `src/app.rs` around
  lines 880-1005 are guarded by a matching `is_some()` arm -- unidiomatic, not
  unsound.
- `cargo clippy --quiet -- -D warnings` is clean on the library.
  `--all-targets` reports pre-existing findings in `src/app/resize.rs` (f32
  strict comparison), `src/db.rs`, `src/app/search_dispatch.rs`,
  `src/library/app_impl.rs`, `src/runtime/musicbrainz_feed_saga.rs`, and two
  discover test helpers. Narrowing the standard command hides them, but none are
  new.

## Recommended Order

1. Reconcile the eleven `Proposed` ADR statuses and define the status vocabulary.
   Cheap, and everything else in this list is read through those headers.
2. Add HTTP timeouts, starting with `remote_media` and then deciding whether
   document fetches get a shared owner.
3. Decide the ADR amendment rule, so ADR 0001 and practice agree.
4. Take a position on the parked discover module: schedule a decision date or
   accept the tax explicitly in the note.
5. Guard the two recurring defect classes, or accept that the troubleshooting
   documents are the mitigation.
6. Decompose `db.rs` before it is relied on by another boundary ADR.

## Merge Recommendation

Not a code change; no merge. Findings P1 and P2 should be routed to
`docs/plans/deferred-architecture-work-index.md` if they are not being acted on
immediately, per that document's own purpose.
