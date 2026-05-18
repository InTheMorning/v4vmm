# Triage Task 002 — Track Detail Parity

## Goal

Produce a structured triage report comparing every visible field on the
**Library track detail** surface (including the metadata grid / tree) against
the equivalent **Index (MusicIndex-source) track detail** surface. For every
gap, record file:line evidence and assign a routing bucket: *loading-shape*,
*persistence*, or *intentional asymmetry*.

This task is research-only. No code changes. The output is a single
documentation file.

## Output

`docs/reviews/library-discover-parity-triage-track.md`

Schema (must follow exactly):

```markdown
# Library / Index Track-Detail Parity Triage

## Status

Triage - 2026-05-17.

## Surfaces compared

- **Library:** <file paths actually inspected>
- **Index:** <file paths actually inspected>

## Surface inventory

| Field rendered          | Library shows? | Index shows? | Same composite? | Notes |
| ----------------------- | -------------- | ------------ | --------------- | ----- |
| <field name>            | yes/no @line   | yes/no @line | yes/no          | <terse> |

## Gap analysis

### Field: <name>

- Library renderer: <file:line>
- Index renderer: <file:line or "not rendered">
- Library VM source: <type::field, file:line>
- Index VM source: <type::field, file:line>
- Local persistence today: <table.column, schema file:line — or "not persisted">
- Hydration path: <where the value enters the VM, file:line — or "n/a">
- Routing: loading-shape | persistence | intentional asymmetry
- Rationale: <one paragraph>

(Repeat per gap. Group by routing bucket within the section.)

## Open questions

- <items needing user decision before downstream packets can route>

## Out of scope (handled by sibling triage tasks)

- Album / release detail fields → Task 001
- Artist + Playlist detail fields → Task 003
```

## Files To Inspect

Required:

- `src/ui/shells/library/track_detail.rs`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/ui/shells/library/track_detail_metadata_grid.rs`
- `src/ui/shells/library/track_detail_metadata_cells.rs`
- `src/ui/shells/library/track_detail_metadata_values.rs`
- `src/view_models/library.rs` (track projection types — search for
  `TrackDetail`, `TrackView`, or equivalent and the metadata grid types)
- `src/view_models/search_results/index_detail.rs`
- `src/view_models/search_results/mod.rs` (`IndexDetailKind::Track*` if
  present)
- `src/ui/shells/search_results_inspector.rs` (Index track detail render
  path)

Reference only (do not propose changes to):

- `src/discover/app_impl.rs` — parked module; useful to see what Discover
  historically surfaced for track-level fields. Live comparison is against
  the Index detail VM, not this code.

For "local persistence today" cells, consult the SQLite schema (grep for
`CREATE TABLE` on track-related tables).

## Fields the deferred-work index calls out by name

Definitely include in the inventory (mark "Library shows? no" if absent):

- release date (track release date may differ from album release date —
  include both columns if both exist)
- language
- explicit state
- description / lyrics / annotation
- contributor identity per-track (note: post-ADR-0028 contributor panel
  is album-level — confirm whether per-track contributor surfacing exists)

Plus any other field that one side renders and the other does not.

## Do Not Touch

- Any `src/` code. Triage is read-only.
- The `src/discover/` module — only read for historical reference.
- ADR files.

## Constraints

- File:line evidence is mandatory for every cell in the gap-analysis section.
- Routing bucket choices are limited to the three named buckets. Ambiguous
  cases go to "Open questions".
- Stay focused on track-level fields. Album-level fields belong to Task 001
  even if they appear on the track surface as breadcrumb context;
  cross-reference rather than duplicate.
- The metadata grid is a structured surface — list each cell as a field,
  including cells that are populated only via expand / lazy load.

## Implementation Steps

1. Read every file in *Files To Inspect — Required*.
2. Build the surface inventory: list every field rendered on either side,
   including each metadata-grid cell.
3. For each row where the two columns disagree, open a gap-analysis entry.
4. Trace the data path on both sides to fill all six bullets.
5. Assign the routing bucket per the definitions in
   `docs/plans/library-discover-parity-triage-plan.md`.
6. Write the report at the path above.

## Acceptance Criteria

- File exists at `docs/reviews/library-discover-parity-triage-track.md`.
- Surface inventory includes every metadata-grid cell on the Library side
  and every field rendered on the Index side.
- Every gap has a complete gap-analysis entry with file:line evidence on
  all six bullets.
- Every gap has a routing bucket.
- No code under `src/` was modified.

## Test Commands

None — documentation only. Verify with:

```bash
ls docs/reviews/library-discover-parity-triage-track.md
git status --short
```

## Prompt for lower-context coding model

You are running a bounded research task that produces a single
documentation file. No code changes.

Read this task file in full, then read every file in *Files To Inspect —
Required*. Reference-only files are for historical context.

Goal: write `docs/reviews/library-discover-parity-triage-track.md` per the
schema in this task file. Compare Library track detail rendering (including
the metadata grid / cells / values modules) against Index track detail
rendering. For every visible field — including each metadata-grid cell —
fill an inventory row. For every gap, fill a gap-analysis entry with
file:line evidence and a routing bucket choice (loading-shape, persistence,
or intentional asymmetry).

Constraints:
- Documentation only. No source edits.
- File:line evidence is required on every gap-analysis bullet.
- Routing bucket choices limited to the three named buckets; ambiguous
  cases go to "Open questions".
- Five named index-entry fields (release date, language, explicit state,
  description, contributor identity) must be present in the inventory
  even if only on one side.
- Stay focused on track-level fields. Album / artist / playlist fields are
  out of scope.
- Never skip hooks. Do not commit.

At the end, report:
1. Report path.
2. Field count in inventory (including metadata-grid cells).
3. Gap count grouped by routing bucket.
4. Open questions count.
5. Any file you needed but could not locate.

## Escalation Triggers

- The metadata-grid cell set is generated dynamically (e.g., from a key/value
  map rather than fixed cells). Report the structure; the gap analysis then
  needs to enumerate the populated key set per side, not a hardcoded list.
- The Index side has no track detail view at all. Report; treat every
  Library-rendered field as a "persistence vs loading-shape" gap on the
  Index side or, if the gap is "no Index track detail surface exists",
  raise that as the single top-level open question and stop.
