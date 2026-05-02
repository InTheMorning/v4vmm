# ADR 0034 Task 001: Scale Shared Primitives

## Goal

Make shared primitives honor `ui_scale` for user-facing dimensions: surface
padding/radius, button size/padding/font/gap/radius, label text size,
multiline text size/line height, and icon size.

## Files to Inspect

- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `src/ui/tokens.rs`
- `src/ui/primitives/surface.rs`
- `src/ui/primitives/button.rs`
- `src/ui/primitives/label.rs`
- `src/ui/primitives/multiline_text.rs`
- `src/ui/icons.rs`

## Files Likely to Change

- `src/ui/primitives/surface.rs`
- `src/ui/primitives/button.rs`
- `src/ui/primitives/label.rs`
- `src/ui/primitives/multiline_text.rs`
- `src/ui/icons.rs`
- `src/ui/tokens.rs` only if a missing scale helper is required
- Focused primitive tests if existing coverage needs updating
- `docs/reviews/adr-0034-review-checklist.md`

## Do Not Touch

- `src/library.rs`
- `src/search.rs`
- Backend, database, services, schema, playlist behavior, playback behavior
- Theme palette values

## Constraints

- Use existing token accessors before adding new API.
- Keep fixed dimensions only when they are explicitly non-adaptive, such as
  hairlines or documented media/artwork constraints.
- Do not compensate in screens.
- Preserve minimum macOS-style hit targets after scaling.
- Use `#[expect(..., reason = "...")]` rather than broad lint suppression if a
  lint exception is genuinely needed.

## Implementation Steps

1. Convert `Surface` render padding and radius from `.px()` to `.scaled(cx)`.
2. Convert `Button` render dimensions from base values to scaled token values.
   Add private helpers if the existing `height()` API needs an `App` context.
3. Convert `Label` render text size to `.scaled(cx)`.
4. Convert `MultilineText` text size and default line height to scale-aware
   values. Prefer deriving line height from the selected font token.
5. Add an icon-size scale helper or convert `Icon::render` to resolve through
   `ScaleFactor::current(cx)`.
6. Run focused tests and check for compile errors caused by new `cx` usage.
7. Update the ADR 0034 review checklist with task status.

## Acceptance Criteria

- Shared primitive render paths no longer use unscaled token `.px()` for
  user-facing padding, radius, text, icon, gap, or control height.
- Medium scale remains visually close to the current base design.
- Small/large scale changes primitive dimensions coherently.
- No screen-local scale compensation is introduced.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0034-scale-aware-ui-tokens-and-controls.md`
- `docs/plans/adr-0034-scale-aware-ui-tokens-phase-plan.md`
- `docs/tasks/adr-0034-task-001-scale-shared-primitives.md`
- `src/ui/tokens.rs`
- `src/ui/primitives/surface.rs`
- `src/ui/primitives/button.rs`
- `src/ui/primitives/label.rs`
- `src/ui/primitives/multiline_text.rs`
- `src/ui/icons.rs`

Goal:
- Make shared primitives honor `ui_scale` for user-facing dimensions.

Constraints:
- Use existing token `.scaled(cx)` helpers where possible.
- Do not edit screen files.
- Do not change theme palette values.
- Preserve macOS minimum control targets.

Do not touch:
- Backend/database/service/schema files.
- Playlist or playback behavior.
- `src/library.rs` or `src/search.rs`.

Acceptance criteria:
- Primitive render methods use scaled dimensions for user-facing layout.
- No screen-local scale compensation is added.
- Checks pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If scaling a primitive causes widespread compile errors because callers rely
  on fixed `Pixels`, stop and split the helper API change into a smaller task.
- If a dimension appears semantically fixed but affects readability or hit
  targets, stop and document it in the review checklist before allowlisting it.
