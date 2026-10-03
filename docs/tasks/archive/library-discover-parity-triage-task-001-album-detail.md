# Triage Task 001 — Album / Release Detail Parity

## Goal

Produce a structured triage report comparing every visible field on the
**Library album / release detail** surface against the equivalent **Index
(MusicIndex-source) album / release detail** surface. For every gap, record
file:line evidence and assign a routing bucket: *loading-shape*,
*persistence*, or *intentional asymmetry*.

This task is research-only. No code changes. The output is a single
documentation file.

## Output

`docs/reviews/library-discover-parity-triage-album.md`

Schema (must follow exactly):

```markdown
# Library / Index Album-Detail Parity Triage

## Status

Triage - 2026-05-17.

## Surfaces compared

- **Library:** <file paths actually inspected, e.g. src/ui/shells/library/detail.rs>
- **Index:** <file paths actually inspected, e.g. src/view_models/search_results/index_detail.rs>

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

- Track detail fields → Task 002
- Artist + Playlist detail fields → Task 003
```

## Files To Inspect

Required:

- `src/ui/shells/library/detail.rs`
- `src/ui/shells/library/feed_detail.rs`
- `src/view_models/library.rs` (album/release projection types and their
  field sets — search for `AlbumDetail`, `ReleaseDetail`, `FeedView`, or
  equivalent)
- `src/view_models/search_results/index_detail.rs`
- `src/view_models/search_results/mod.rs` (entry types: `IndexDetailKind`,
  `IndexDetailDisplay`)
- `src/ui/shells/search_results_inspector.rs` (where Index detail renders)

Reference only (do not propose changes to):

- `src/discover/app_impl.rs` — parked module; useful to see what Discover
  historically surfaced for these fields, but the live comparison is against
  the Index detail VM, not this code.

For "local persistence today" cells, consult:

- `src/db.rs` or whatever owns the SQLite schema (grep for `CREATE TABLE` on
  feed / release / album).
- Source-fact persistence per ADR 0028.

## Fields the deferred-work index calls out by name

Definitely include in the inventory (mark "Library shows? no" if absent):

- release date
- language
- explicit state
- description / summary / annotation
- contributor identity (already closed by post-ADR-0028; include for
  completeness so the report is self-contained — mark as "closed,
  see post-ADR-0028 task 001")

Plus any other field that one side renders and the other does not.

## Do Not Touch

- Any `src/` code. Triage is read-only.
- The `src/discover/` module — only read for historical reference.
- ADR files. The synthesis step (main thread) handles ADR drafting.

## Constraints

- File:line evidence is mandatory for every cell in the gap-analysis section.
  Surface-inventory cells may use "@line" shorthand.
- Routing bucket choices are limited to the three named buckets. If a gap
  does not fit any bucket, list it under "Open questions" instead.
- No code change recommendations in the report. Routing only — the
  downstream packet decides the concrete fix.
- Stay focused on album / release detail. Track-level fields belong to
  Task 002 even if they appear on the album surface; cross-reference rather
  than duplicate.

## Implementation Steps

1. Read every file in *Files To Inspect — Required*.
2. Build the surface inventory: list every field rendered on either side.
3. For each row where the two columns disagree (or the field is missing on
   one side), open a gap-analysis entry.
4. For each gap, trace the data path on both sides to fill all six bullets.
   When tracing persistence, follow the projection back to its source
   (Library: usually the SQLite read model; Index: the MusicIndex API
   response DTO).
5. Assign the routing bucket. If persistence on the local side is the
   blocker, choose *persistence*. If the data is in a local row but not
   projected into the VM, choose *loading-shape*. If the field is
   semantically remote-only or deliberately omitted, choose *intentional
   asymmetry* and cite the ADR/invariant that justifies it.
6. Write the report at the path above.

## Acceptance Criteria

- File exists at `docs/reviews/library-discover-parity-triage-album.md`.
- Surface-inventory table has at least one row per field rendered on either
  side; the five named index-entry fields are present even if only on one
  side.
- Every gap has a complete gap-analysis entry with file:line evidence on all
  six bullets.
- Every gap has a routing bucket. "Open questions" is used for genuinely
  ambiguous routing, not as a default.
- No code under `src/` was modified (git status clean for tracked source).

## Test Commands

None — documentation only. Verify with:

```bash
ls docs/reviews/library-discover-parity-triage-album.md
git status --short
```

`git status` should show only the new report file.

## Prompt for lower-context coding model

You are running a bounded research task that produces a single
documentation file. No code changes.

Read this task file in full, then read every file in *Files To Inspect —
Required*. Reference-only files are for historical context, do not propose
changes to them.

Goal: write `docs/reviews/library-discover-parity-triage-album.md` per the
schema in this task file. Compare Library album / release detail rendering
against Index (MusicIndex-source) album detail rendering. For every visible
field, fill the inventory row. For every gap, fill a gap-analysis entry with
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
- Stay focused on album / release detail. Track / artist / playlist fields
  are out of scope (sibling tasks handle them).
- Never skip hooks. Do not commit.

At the end, report:
1. Report path.
2. Field count in inventory.
3. Gap count grouped by routing bucket.
4. Open questions count.
5. Any file you needed but could not locate (escalation candidate).

## Escalation Triggers

- A required file does not exist or has been renamed. Stop, report, do not
  guess at the new location.
- A field's data path passes through code that has been intentionally
  removed but is still referenced elsewhere (suggesting a regression rather
  than a parity gap). Report and stop.
- The Library and Index renderers turn out to share a composite for the
  field in question (so the "gap" is configuration, not divergence). Note
  this in the inventory and skip the gap-analysis entry.
