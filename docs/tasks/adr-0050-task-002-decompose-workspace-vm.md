# ADR 0050 Task 002 — decompose `src/view_models/workspace.rs`

## Goal

Move `src/view_models/workspace.rs` (2,904 LOC) into a submodule directory
`src/view_models/workspace/` with five behavior-grouped submodules and a
`tests.rs` file. No public API change; callers' `use` statements stay the
same via `mod.rs` re-exports.

## Files To Inspect

- `src/view_models/workspace.rs` (full read)
- `src/view_models/mod.rs` (re-export pattern)
- `docs/adr/0050-post-adr-0048-module-decomposition.md`
- `docs/plans/adr-0050-module-decomposition-phase-plan.md`
- `tests/architecture_tests.rs` (guards pinning `view_models/workspace.rs`)
- Callers: `src/app.rs`, `src/library/app_impl.rs`, `src/ui/shells/workspace.rs`,
  `src/ui/composites/frame_shell.rs`, `src/ui/composites/breadcrumb_trail.rs`,
  `src/ui/composites/filter_chip_strip.rs`

## Files Likely To Change

- `src/view_models/workspace.rs` — deleted (moved into directory)
- `src/view_models/workspace/mod.rs` — new
- `src/view_models/workspace/frame.rs` — new
- `src/view_models/workspace/chrome.rs` — new
- `src/view_models/workspace/nav.rs` — new
- `src/view_models/workspace/breadcrumb.rs` — new
- `src/view_models/workspace/tests.rs` — new (contains existing inline tests)
- `tests/architecture_tests.rs` — path retargeting only

## Do Not Touch

- Any public-API signature.
- Any GPUI render code.
- The `WorkspaceLayout` impl block — keep it intact in `mod.rs`.
- View-model logic outside this module.
- The `discover/` module.

## Constraints

- Behavior-preserving move. No new types, no signature changes.
- `WorkspaceLayout` and its `impl` block stay in `mod.rs` (it cross-cuts
  every submodule and splitting it is out of scope per ADR 0050).
- Re-export discipline: `pub(crate)` types must remain reachable via
  `use crate::view_models::workspace::*` exactly as before.
- Existing inline `#[cfg(test)] mod tests` moves to `tests.rs`. If tests
  reference items that were `pub(super)` from inside the file, bump to
  `pub(crate)` only where required and list each bump in the final report.
- Use `git mv` semantics for the move so blame survives.
- No commit unless explicitly asked.

## Submodule ownership map

| Submodule | Items |
|---|---|
| `mod.rs` | `WorkspaceLayout`, `WorkspaceLayoutConfig`, `WorkspaceFrameConfig`, `WorkspaceModelError`, all re-exports |
| `frame.rs` | `WorkspaceFrameId`, `WorkspaceFrameKind`, `FrameDetachEligibility`, `FrameDockTarget`, `FrameSearchScope`, `FrameSearchDescriptor`, `WorkspaceFrameState` |
| `chrome.rs` | `FrameChromeButtonDisplay`, `FrameChromeMenuItemDisplay`, `FrameShellDisplay`, `FilterChipOption`, `FilterChipStripDisplay`, `ContentFilter` |
| `nav.rs` | `FrameNavigationEntry`, `FrameNavigationState` |
| `breadcrumb.rs` | `BreadcrumbTruncation`, `BreadcrumbSegment`, `BreadcrumbDisplay` |
| `tests.rs` | The full existing `#[cfg(test)] mod tests` block |

Use the existing line numbers from the file as a guide:

- `frame.rs` items live around lines 29-247
- `chrome.rs` items live around lines 312-568 (ContentFilter, FilterChip*)
  plus 439-567 (FrameChrome*, FrameShellDisplay)
- `nav.rs` items live around lines 1278-1400
- `breadcrumb.rs` items live around lines 569-720
- `mod.rs` (WorkspaceLayout) lives around lines 720-1278

Re-verify exact ranges when reading the file; the line numbers above are
post-bee1ac2 snapshots.

## Implementation Steps

1. Read `src/view_models/workspace.rs` in full. Confirm the item ranges
   above match the current file.
2. Create the directory `src/view_models/workspace/` and an empty `mod.rs`.
3. Move items from smallest-coupling to largest:
   1. `nav.rs` — has the fewest cross-references.
   2. `breadcrumb.rs` — depends on `nav.rs` for `FrameNavigationEntry`.
   3. `frame.rs` — depends on `nav.rs` for nav-state types.
   4. `chrome.rs` — depends on `frame.rs` for filter chip glue.
   5. `mod.rs` — `WorkspaceLayout` impl + re-exports.
   6. `tests.rs` — move existing `#[cfg(test)] mod tests` block.
4. After each move: `cargo check` then fix imports inside the new file
   (use `use super::*` plus explicit `use crate::view_models::workspace::*`
   if it helps).
5. Bump visibility from `pub(super)` to `pub(crate)` only when a test in
   `tests.rs` requires it; document each bump.
6. Update `mod.rs` re-exports so the public surface matches the previous
   single-file surface byte-for-byte:
   ```rust
   pub(crate) use self::breadcrumb::*;
   pub(crate) use self::chrome::*;
   pub(crate) use self::frame::*;
   pub(crate) use self::nav::*;
   #[cfg(test)] mod tests;
   ```
   (Restrict each re-export to the exact item list rather than `*` if
   blanket re-export causes name collisions.)
7. Delete the original `src/view_models/workspace.rs` once the directory
   is fully populated.
8. Run the 5-gate.
9. Search `tests/architecture_tests.rs` for guards pinning
   `view_models/workspace.rs`. Retarget paths to the new directory.

## Acceptance Criteria

- `src/view_models/workspace.rs` no longer exists.
- `src/view_models/workspace/{mod,frame,chrome,nav,breadcrumb,tests}.rs`
  all exist and compile.
- No caller outside the new directory had to change `use` statements.
- All 5 gates pass.
- `git log --follow` on the new files surfaces the pre-split history.
- No new `#[allow(...)]` annotations.
- Operator launch: app starts, workspace renders, toolbar search round-trips.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded refactor task from a larger plan.

Implement only this task. Behavior-preserving file move only.

Read:
- `docs/adr/0050-post-adr-0048-module-decomposition.md`
- `docs/plans/adr-0050-module-decomposition-phase-plan.md`
- `src/view_models/workspace.rs` in full
- `tests/architecture_tests.rs`
- Caller files listed in the task

Goal:
- Split `src/view_models/workspace.rs` (2,904 LOC) into a
  `src/view_models/workspace/` directory with `mod.rs`, `frame.rs`,
  `chrome.rs`, `nav.rs`, `breadcrumb.rs`, `tests.rs`.
- Use the submodule ownership map in the task file. Items per file are
  enumerated there.
- Move items in this order: `nav.rs`, `breadcrumb.rs`, `frame.rs`,
  `chrome.rs`, `mod.rs`, `tests.rs`. Run `cargo check` between moves.

Constraints:
- No public-API change. Re-export everything `pub(crate)` from `mod.rs`.
- `WorkspaceLayout` impl stays in `mod.rs`.
- Visibility bumps only when a test demands it. Document each bump.
- Use `git mv` so blame survives.
- Never skip hooks. Don't commit unless explicitly asked.

Test commands:
- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files created and deleted (with LOC per file)
2. tests run + pass/fail counts
3. visibility bumps applied
4. caller import changes (should be none)
5. deviations
6. unresolved concerns

## Escalation Triggers

- A submodule needs an item from a sibling that creates a circular
  dependency. (Resolution: move the shared item up to `mod.rs`, not
  duplicate it.)
- A test in `tests.rs` reaches a method on `WorkspaceLayout` that was
  previously private. (Resolution: keep `WorkspaceLayout` impl in
  `mod.rs`; bump only that one method's visibility if needed.)
- An arch test guard wording does not map cleanly to the new path layout.
  Report the guard name; do not relax it.
