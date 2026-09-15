# ADR 0071: Shared Text Selection And Linux Primary Selection

## Status

Implemented - 2026-09-15. Implementation, mechanical verification, available
X11 operator checks, preservation and cleanup are complete in
[task 001](../tasks/adr-0071-task-001-shared-text-selection.md). IME composition
and Wayland remain untested coverage limits.
ADR 0063 task 005 and ADR 0069 task 001 remain complete.

Amended 2026-09-14: [ADR 0072](0072-pinned-gpui-base-selection-corrections.md)
replaces the published-gpui-base restriction and PRIMARY-only fork fallback
after native checks exposed two upstream selection defects. Other decisions
below remain binding; the fork is pinned.

Updated 2026-09-15: recorded completion after native correction acceptance,
final preservation and confirmed cleanup.

## Context

HIG backlog item 11 requires word selection on double-click, logical-line
selection on triple-click, and Linux PRIMARY across logs and editable fields.
The upstream 0.6.1 release supplies pointer selection and the input editing
engine. Maintaining a vendored 0.5.1 snapshot duplicates those owners.

## Decision

Use published gpui-component, gpui-base and gpui-kit-assets 0.6.1, with
GPUI's gpui-pre and gpui-pre-platform 0.3.1 packages. Pin these versions and the
verified Rust toolchain; review upgrades deliberately. Remove the vendor tree,
old vendor Cargo patch and local text-selection crate. The sole gpui-base
source exception is specified in [ADR 0072](0072-pinned-gpui-base-selection-corrections.md).
There is no data migration.

Optimize `gpui-pre` and `taffy` in development builds. The native resize CPU
capture identifies layout work, and the populated Show mock establishes its
debug redraw cost. Optimizing GPUI also covers its instantiated generic Taffy
algorithms. Keep application code unoptimized, with debug assertions and symbols.
The operator must still accept native resizing and idle responsiveness; lower
mock redraw cost alone does not establish the cursor-flicker cause or its repair.

Publish PRIMARY from the shared selection owners. The log adapter uses
`gpui_base::TextSelectionHandle` for pointer selection and projected UTF-8
ranges. Upstream owns word and logical-line boundaries. Retain the adapter for
exact whitespace-preserving Copy, typed context-menu availability, Select All,
and selection preservation when logs append. The upstream standalone element
and Root Copy handler do not supply that complete contract. View models remain
renderer-free. Existing log geometry and selection colors retain their token
owners. The log handles Root's bound Copy action through its exact-copy method;
a raw key handler runs too late to prevent Root's trimming fallback.

The log owner retains its selected range while the pointer context menu is open,
including between pressing and releasing Copy. Copy and Escape return focus to
the log. The shared pointer menu owns a scoped Escape action, registered during
bootstrap, so app pane shortcuts cannot consume dismissal or close the pane.
Clicking elsewhere can clear or replace the range.

The shared split-pane handle consumes the press that starts resizing. Root's
window-wide text selection must not start a second drag from that same press.
Ordinary text gestures retain the upstream selection path.

Editable fields use the upstream input engine. A shared presentation adapter
observes selection changes and adds middle-click paste through public input
hit testing and insertion APIs. The adapter adds no layout box. It reads PRIMARY
before changing the caret, inserts at the clicked position, respects disabled
and read-only state, emits Change, and keeps upstream normalization, validation
and atomic undo. Click containment uses the input viewport; text bounds move
with scrolling and cannot exclude visible trailing blank rows. Public caret
geometry resolves insertion in those rows. Masked fields do not publish
selection. Changes to text, repainting, focus changes and collapsed selections
do not claim PRIMARY.
Ordinary Copy/Paste retains its separate clipboard. No helper process or
registry-cache edits are permitted.

The input engine retains ownership of focus, IME and Escape. Configuration-editor
Escape still focuses Close without activating it; Close/Reopen retains the draft.
Logs remain read-only. Screens only compose the shared owners.

Keyboard-enabled shared buttons declare an `ActionButton` key context. The
app's pane-level Enter binding excludes that context so the button's existing
key-down handler receives Enter. That handler continues to activate once per
press, suppress held repeats and ignore GPUI's synthesized key-up click.
Pane Enter remains available outside inputs and buttons; input Enter and
the editor's Escape focus transfer retain their existing owners.

## Consequences And Verification

The pre-release GPUI channel requires deliberate dependency updates. The isolated
migration check records the actual API changes and toolchain requirement in the
[task packet](../tasks/adr-0071-task-001-shared-text-selection.md).
The narrow gpui-base fork exception and its removal criteria are owned by
[ADR 0072](0072-pinned-gpui-base-selection-corrections.md).

Mechanical tests cover selection projection, exact copying, append/replacement,
Unicode and paths, independent buffers, normalization, Change events and undo.
Situational guards preserve shared wiring and the accepted Escape path.
The separate [operator check](../runbooks/text-selection-check.md) covers native
X11/Wayland cross-application delivery, highlights, focus, editor layout, undo
and Escape. Green mechanical checks do not close that gate.
