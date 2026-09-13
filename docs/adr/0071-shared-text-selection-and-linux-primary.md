# ADR 0071: Shared Text Selection And Linux Primary Selection

## Status

Accepted - 2026-09-13. Implementation and operator acceptance belong to
[task 001](../tasks/adr-0071-task-001-shared-text-selection.md).
ADR 0063 task 005 and ADR 0069 task 001 remain complete.

## Context

HIG backlog item 11 requires word selection on double-click, logical-line
selection on triple-click, and Linux PRIMARY across logs and editable fields.
The upstream 0.6.1 release supplies pointer selection and the input editing
engine. Maintaining a vendored 0.5.1 snapshot duplicates those owners.

## Decision

Use published gpui-component, gpui-base and gpui-kit-assets 0.6.1, with
GPUI's gpui-pre and gpui-pre-platform 0.3.1 packages. Pin these versions and the
verified Rust toolchain; review upgrades deliberately. Remove the vendor tree,
Cargo patch and local text-selection crate. There is no data migration.

Publish PRIMARY from the shared selection owners. The log adapter uses
`gpui_base::TextSelectionHandle` for pointer selection and projected UTF-8
ranges. Upstream owns word and logical-line boundaries. Retain the adapter for
exact whitespace-preserving Copy, typed context-menu availability, Select All,
and selection preservation when logs append. The upstream standalone element
and Root Copy handler do not supply that complete contract. View models remain
renderer-free. Existing log geometry and selection colors retain their token
owners.

Editable fields use the upstream input engine. A shared presentation adapter
observes selection changes and adds middle-click paste through public input
hit testing and insertion APIs. The adapter adds no layout box. It reads PRIMARY
before changing the caret, inserts at the clicked position, respects disabled
and read-only state, emits Change, and keeps upstream normalization, validation
and atomic undo. Masked fields do not publish selection. Changes to text,
repainting, focus changes and collapsed selections do not claim PRIMARY.
Ordinary Copy/Paste retains its separate clipboard. No helper process or
registry-cache edits are permitted.

The input engine retains ownership of focus, IME and Escape. Configuration-editor
Escape still focuses Close without activating it; Close/Reopen retains the draft.
Logs remain read-only. Screens only compose the shared owners.

## Consequences And Verification

The pre-release GPUI channel requires deliberate dependency updates. The isolated
migration check records the actual API changes and toolchain requirement in the
[task packet](../tasks/adr-0071-task-001-shared-text-selection.md).
If an upstream structural limitation prevents this contract, reassess a pinned
git fork with only the missing PRIMARY integration; do not restore the vendor.

Mechanical tests cover selection projection, exact copying, append/replacement,
Unicode and paths, independent buffers, normalization, Change events and undo.
Situational guards preserve shared wiring and the accepted Escape path.
The separate [operator check](../runbooks/text-selection-check.md) covers native
X11/Wayland cross-application delivery, highlights, focus, editor layout, undo
and Escape. Green mechanical checks do not close that gate.
