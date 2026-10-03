# Triage Task 003 — Artist + Playlist Detail Parity

## Goal

Produce a structured triage report comparing every visible field on the
**Library artist detail** and **Library playlist detail** surfaces against
the equivalent **Index (MusicIndex-source) artist** and **playlist** detail
surfaces (or document their absence). For every gap, record file:line
evidence and assign a routing bucket: *loading-shape*, *persistence*, or
*intentional asymmetry*.

This task is research-only. No code changes. The output is a single
documentation file covering both entity families.

## Output

`docs/reviews/library-discover-parity-triage-artist-playlist.md`

Schema (must follow exactly — two top-level sections, one per entity family):

```markdown
# Library / Index Artist + Playlist Detail Parity Triage

## Status

Triage - 2026-05-17.

---

# Artist detail

## Surfaces compared

- **Library:** <file paths actually inspected>
- **Index:** <file paths actually inspected, or "no dedicated Index artist
  detail surface — see open questions">

## Surface inventory

| Field rendered          | Library shows? | Index shows? | Same composite? | Notes |
| ----------------------- | -------------- | ------------ | --------------- | ----- |

## Gap analysis

### Field: <name>

- Library renderer: <file:line>
- Index renderer: <file:line or "not rendered">
- Library VM source: <type::field, file:line>
- Index VM source: <type::field, file:line>
- Local persistence today: <table.column, file:line — or "not persisted">
- Hydration path: <file:line — or "n/a">
- Routing: loading-shape | persistence | intentional asymmetry
- Rationale: <one paragraph>

## Open questions

- <items>

---

# Playlist detail

(Same five subsections as Artist detail above.)

---

## Out of scope (handled by sibling triage tasks)

- Album / release detail → Task 001
- Track detail → Task 002
```

## Files To Inspect

Required (artist side):

- `src/view_models/library.rs` (artist projection types — search for
  `ArtistDetail`, `ArtistView`, name-derived artist projections from ADR 0045)
- `src/ui/shells/library/*.rs` — locate whichever shell renders the artist
  detail body (likely `detail.rs` dispatching by entity kind; confirm)
- `src/view_models/search_results/index_detail.rs`
- `src/view_models/search_results/mod.rs` (`IndexDetailKind::Artist*` if
  present)
- `src/ui/shells/search_results_inspector.rs`

Required (playlist side):

- `src/ui/shells/library/playlist_detail.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/search_results/index_detail.rs` (playlist branch if any)
- `src/ui/shells/search_results_inspector.rs`

Reference only:

- `src/discover/app_impl.rs` — parked; historical artist / playlist
  rendering may be visible here.
- ADR 0045 (name-derived artist views) — clarifies what artist identity
  *means* on the Library side.
- ADR 0029 (artist/person identity persistence) — clarifies what reconciled
  artist identity *could* mean and what is deferred.

## Fields the deferred-work index calls out by name

Definitely include in the inventory for each entity family (mark "Library
shows? no" if absent):

Artist:
- description / biography / annotation
- explicit state (per artist — may not exist at this level; flag if so)
- contributor list (post-ADR-0028 collaborators)
- linked releases / tracks count
- external identifiers (MBID, etc.)

Playlist:
- release date / created date / modified date
- language
- explicit state
- description / annotation
- track count

Plus any other field that one side renders and the other does not.

## Do Not Touch

- Any `src/` code. Triage is read-only.
- The `src/discover/` module — only read for historical reference.
- ADR files.

## Constraints

- File:line evidence is mandatory for every cell in the gap-analysis section.
- Routing bucket choices are limited to the three named buckets. Ambiguous
  cases go to "Open questions".
- Two top-level sections — Artist and Playlist — kept in one file. They
  share enough VM context (`SearchResultsInspector` for both) that splitting
  is wasteful.
- If the Index side has no dedicated detail surface for one of the two
  families, do not invent one — document the absence in the relevant
  "Open questions" subsection and treat every Library-rendered field as a
  candidate persistence / loading-shape question whose resolution depends
  on whether an Index detail surface is even planned.
- Artist identity reconciliation is **out of scope** (ADR 0029 territory).
  Flag any gap whose resolution requires reconciliation as an open question,
  do not assign a routing bucket.

## Implementation Steps

1. Read every file in *Files To Inspect — Required*.
2. Confirm whether the Index side has artist and playlist detail surfaces.
   Record what you find.
3. For each entity family, build the surface inventory.
4. For each gap, open a gap-analysis entry with file:line evidence.
5. Assign the routing bucket. Flag identity-reconciliation gaps as open
   questions instead of routing them.
6. Write the report at the path above with two top-level sections.

## Acceptance Criteria

- File exists at
  `docs/reviews/library-discover-parity-triage-artist-playlist.md`.
- Both entity-family sections are present, each with its own surface
  inventory and gap analysis.
- Every gap has a complete gap-analysis entry with file:line evidence.
- Every gap has a routing bucket, unless it is an identity-reconciliation
  question (which goes to "Open questions").
- No code under `src/` was modified.

## Test Commands

None — documentation only. Verify with:

```bash
ls docs/reviews/library-discover-parity-triage-artist-playlist.md
git status --short
```

## Prompt for lower-context coding model

You are running a bounded research task that produces a single
documentation file covering two entity families (Artist and Playlist).
No code changes.

Read this task file in full, then read every file in *Files To Inspect —
Required* for both families.

Goal: write
`docs/reviews/library-discover-parity-triage-artist-playlist.md` per the
schema in this task file. Two top-level sections (Artist detail, Playlist
detail), each with surfaces compared, surface inventory, gap analysis, and
open questions. For every visible field on either side, fill an inventory
row. For every gap, fill a gap-analysis entry with file:line evidence and
a routing bucket choice (loading-shape, persistence, or intentional
asymmetry). If the Index side has no dedicated detail surface for a family,
document the absence in "Open questions" rather than fabricating one.

Constraints:
- Documentation only. No source edits.
- File:line evidence required on every gap-analysis bullet.
- Routing bucket choices limited to the three named buckets; ambiguous
  cases go to "Open questions".
- Artist identity reconciliation gaps (anything that would require
  cross-feed person/artist matching) go to "Open questions", not to a
  routing bucket — that work is owned by ADR 0029, out of scope here.
- Stay focused on artist + playlist detail. Album / track fields are
  out of scope (sibling tasks handle them).
- Never skip hooks. Do not commit.

At the end, report:
1. Report path.
2. Field count in inventory per family.
3. Gap count grouped by routing bucket per family.
4. Open questions count per family.
5. Whether the Index side has a dedicated detail surface for each family.
6. Any file you needed but could not locate.

## Escalation Triggers

- The Library artist detail surface dispatches through a generic detail
  shell rather than a dedicated `artist_detail.rs`. Record the dispatch
  point (file:line) and treat the rendered region as the surface; do not
  invent a missing file.
- A gap on the artist side appears to require identity reconciliation
  (matching `ArtistRef::Musicindex` to a local person id). Move that gap
  to "Open questions" and reference ADR 0029.
- The Index side has playlist results but no detail view: this is a
  significant finding — surface it at the top of the playlist "Open
  questions" subsection.
