# ADR 0023 Task 001: Documentation Architecture Cleanup

## Status

Completed 2026-04-30.

## Goal

Organize repository documentation into the canonical docs folders and update
ADR 0023 references so implementation work points at stable paths.

## Scope

- Move root-level docs into `docs/architecture/`, `docs/plans/`,
  `docs/runbooks/`, `docs/schema/`, and `docs/research/`.
- Keep ADRs in `docs/adr/`; do not renumber historical ADR files.
- Move old roadmap notes to `docs/archive/`.
- Update `README.md`, `docs/README.md`, ADR references, and Rust doc comments
  that mention moved paths.
- Add the ADR 0023 task packets and review checklist.

## Out Of Scope

- Rewriting old ADR decisions.
- Changing code behavior.
- Renaming modules or crates.

## Tests

- `rg` for old doc paths returns no stale references.
- `cargo fmt -- --check`
- `cargo check`

## Result

- Organized tracked docs into the purpose-based folders listed in
  `docs/README.md`.
- Updated old-path references in `README.md`, ADRs, plans, and Rust doc
  comments.
- Added ADR 0023 task packets and the review checklist.
- Verified old doc path search, formatting, check, clippy, and full tests as
  part of the ADR 0023 implementation slice.

## Prompt For Lower-Context Coding Model

You are editing documentation only. Keep ADR files in `docs/adr/`, move
non-ADR docs into the purpose-based folders described in `docs/README.md`,
and update every repository link to the new paths. Do not change Rust behavior.
Run `rg` for old paths, then run `cargo fmt -- --check` and `cargo check`.
