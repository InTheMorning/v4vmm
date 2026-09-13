# ADR 0071 Task 001: Shared Text Selection

Status: Implementation complete - 2026-09-13; required mechanical checks Green; operator V1–V3, preservation and fixture cleanup open.

## Goal And Owners

Implement [ADR 0071](../adr/0071-shared-text-selection-and-linux-primary.md)
for HIG product polish backlog item 11 in one bounded packet.

- Published gpui-component, gpui-base and gpui-kit-assets 0.6.1; gpui-pre and
  gpui-pre-platform 0.3.1; Rust 1.97.1. All are explicitly pinned.
- gpui-base owns pointer boundaries, window selection and input editing.
- `src/ui/composites/selectable_text.rs` adapts the upstream selection handle
  to exact log Copy, Select All, append preservation and the typed context menu.
- `src/view_models/text_selection.rs` retains renderer-free log ranges and Copy
  availability. It no longer owns word or line boundary algorithms.
- `src/ui/primitives/primary_selection.rs` publishes input selection and handles
  middle-click insertion through public APIs, without adding a layout box.
- `src/ui/theme_bridge.rs` supplies the app's palette to component and base tokens.
- `tests/architecture_tests.rs` guards shared ownership, separate buffers,
  published dependencies and the existing configuration-editor Escape chain.

There is no vendor tree, local selection crate, Cargo patch, or git fork.
Config format, recovery phases, playback, persistence and completed packets
remain outside this task. ADR 0063 task 005 remains closed.

## Migration Evidence

An isolated clone at `/tmp/v4vmm-gpui061-trial`, branch `trial/gpui-0.6.1`,
measured the migration before applying it to the working tree. The initial
app check produced 20 errors. Mechanical API changes covered focus context,
`Anchor`, `ScrollbarMode`, scroll offsets, menus, assets, theme fields and the
configuration editor's `TextareaState`. Tests needed two new event fields.
The isolated app check then passed. Required Clippy also needed equivalent
formatting and duration expressions under the newer compiler.

Rust 1.93.1 failed first in gpui-pre's `std::hint::cold_path` calls. Installed
Rust 1.97.1 compiled both the core and the separately packaged Linux platform.
The project toolchain now records that verified version.

The standalone upstream log element does not replace the complete accepted
log contract: Root Copy trims whitespace, and it supplies neither Select All
nor this app's context-menu/append policy. Retaining a small adapter avoids
those regressions while deleting the local boundary implementation. Upstream
word boundaries use character classes; joined emoji can be separate selection
units. Triple-click excludes the LF terminator. Exact drag/Select All copying
still preserves selected whitespace and line endings.

Mock pointer tests found that the public IME hit test misses empty fields and
space after glyphs, and can return an offset on the wrong logical row. The shared adapter checks the returned row and falls back to public caret bounds within
the visible logical rows. It neither estimates positions from font widths nor
changes the caret before reading PRIMARY. Input insertion uses upstream
normalization, validation, Change events and atomic undo.

The lockfile changes from 965 to 906 package records relative to the vendor
attempt: 272 old name/version records disappear and 213 arrive, a net reduction
of 59. The new platform/rendering stack and mock-test support account for new
records; a net reduction of 155 was not borne out by resolution.

## Mechanical Acceptance

1. Mock GPUI tests, mounted through the real component Root, prove upstream
   word/line projection reaches exact log Copy
   and PRIMARY, preserves selected ranges on append, clears on replacement,
   and handles whitespace-only Select All. Renderer-free tests retain exact
   multiline/Unicode Copy and typed availability coverage.
2. Input tests prove nonempty selection publication, masked-field exclusion,
   no PRIMARY claim during text edits, clipboard independence, Unicode offsets,
   normalized single-line paste, preserved multiline paste, Change events,
   disabled/read-only/empty no-ops, and separate atomic undo/redo.
3. Mock pointer events verify middle-click reaches empty fields, line ends, later
   rows and wrapped Unicode text. Input pointer tests prove accented words,
   path segments and whole-value triple-click publish PRIMARY. A validation test
   proves rejection and independent clipboard replacement/undo. A theme test
   checks the component/base palette synchronization.
   These tests use GPUI's in-process test platform, without launching the app.
4. Architecture tests preserve the upstream selection seam, PRIMARY wiring,
   token synchronization and the accepted input-first Escape/Close behavior.

```bash
cargo fmt -- --check
cargo check --locked --offline
cargo clippy --locked --offline -- -D warnings
cargo test --locked --offline
cargo build --locked --offline
```

Required checks Green - 2026-09-13: format, check, strict Clippy, debug build,
1,384 app unit tests (including ten focused migration/selection tests) and 246
architecture tests. Ten existing documentation examples remain ignored. The
full suite used unrestricted local socket fixtures. Changed-document links
and diff whitespace checks are Green. No desktop acceptance is claimed.

The optional exploratory
`cargo clippy --all-targets -- -D warnings` also found existing test-only lint
debt outside this packet; the required Clippy command above is the repository's
gate. This packet does not undertake a test-suite lint cleanup.

## Documentation Inventory And Rollback

Created this task, ADR 0071 and the operator runbook in existing documentation
folders. Removed the now-obsolete vendor patch note. Updated the source map,
ADR/docs indexes, backlog, delivery order, AGENTS and pending-human index.
No documentation folders or root Markdown files were added.

Rollback restores the previously published dependencies, toolchain and original
log selection adapter. No configuration or database migration needs reversal.
Do not restore the vendor attempt. Stop after this packet; ADR 0066 task 007
starts in a later session.

## Operator Visual Check

Follow [Shared text selection check](../runbooks/text-selection-check.md).
Record V1 (Settings/search/clipboard), V2 (logs/editor and migrated theme/scale),
V3 (recovery/IME/Escape), desktop protocol, preservation and cleanup separately.
A failure retains its fixture and leaves this packet open. Mechanical checks
do not establish desktop acceptance or close inherited recovery/playback gates.
