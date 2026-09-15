# ADR 0072: Pinned gpui-base Selection Corrections

## Status

Implemented - 2026-09-15. Fork publication, dependency integration, mechanical
verification, focused X11 checks, preservation and cleanup are complete in
[ADR 0071 task 001](../tasks/adr-0071-task-001-shared-text-selection.md).
This decision replaces only ADR 0071's published-gpui-base restriction and
PRIMARY-only fork fallback. Its shared ownership and acceptance requirements
remain binding. No upstream issue or PR submission is authorized.

Updated 2026-09-15: recorded completion after native correction acceptance,
final preservation and confirmed cleanup. IME composition and Wayland remain
untested coverage limits.

## Context

The operator reproduced two editable-input failures on X11: double-clicking
the final `👩‍💻` selects nothing; adding a following period produces a highlight
whose PRIMARY payload is only `👩`. The
[upstream preparation record](../reviews/adr-0071-gpui-base-emoji-selection.md)
preserves exact reproductions, causes, tested patches and verification limits.
The errors belong to gpui-base's pointer resolution and word boundaries.

ADR 0071 removed duplicated selection implementations and vendored sources.
Returning those behaviors to application code would recreate that duplication.
A narrow upstream-source correction preserves the shared owner. ADR 0057
requires a new decision because the earlier fork allowance covered PRIMARY
integration only.

## Decision

Use [InTheMorning/gpui-kit](https://github.com/InTheMorning/gpui-kit), a fork of
`longbridge/gpui-kit`, based on the commit used to publish
gpui-base 0.6.1: `96103905ea0c9c199db206ada7a9b2e3114e6339`. Limit its diff to
glyph hit testing for double-click, complete grapheme boundaries for word
selection, and regression tests in the three files named in the preparation
record. Preserve ordinary caret placement and the existing character-class
word policy.

Keep `gpui-base = "=0.6.1"` and override only that package through
`[patch.crates-io]`, using the published fork URL and a full commit SHA.
The verified correction commit is `5463fe4e72fd740b0db08da92003488b32661867`.
Record both in the task and lockfile before rebuilding the app. Do not use
a branch, tag, local path, vendor tree or modified registry cache as the
application dependency. The fork's workspace lockfile does not replace the
application lockfile.

Keep gpui-component and gpui-kit-assets at `=0.6.1`, and gpui-pre and
gpui-pre-platform at `=0.3.1`. Keep the verified Rust toolchain. GPUI's renderer
is unchanged. PRIMARY publication remains in the existing app adapter.
Retain the separate test and implementation patches for a future upstream PR.

## Consequences And Verification

The fork adds a small maintenance obligation and requires an available remote
commit for reproducible builds. Exact version and commit pins keep updates
deliberate. A later upstream release can replace the override after the same
regressions and native checks pass.

The situational guard
`adr_0072_gpui_base_fork_is_the_only_pinned_source_override` checks the manifest
and lockfile. ADR 0071 guards retain shared selection ownership. Mechanical
acceptance requires the upstream regressions, app tests, check, format and
strict Clippy to be Green.

Native X11 acceptance covers both emoji cases, exact PRIMARY and Copy,
replacement/Undo/Redo, the accepted Escape path, Settings paths and log
boundaries/read-only behavior. IME composition and Wayland remain untested.
The [operator procedure](../runbooks/text-selection-check.md) remains the
regression check. Both accepted fixtures and the temporary source/scratch
artifacts were removed; future runs require fresh fixtures. Existing log-packet
acceptance remains closed.

Rollback removes the gpui-base override and restores its published source in
the lockfile. It restores the two known failures and cannot close acceptance.
