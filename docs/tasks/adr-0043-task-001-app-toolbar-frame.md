# ADR 0043 Task 001: App Toolbar Frame and Now Playing Space

## Goal

Convert the current top tab-bar strip into an app toolbar with stable
zones and give Now Playing its own framed trailing control space. Do not
change search behavior yet.

## Files to Inspect

- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/app/playback_bar.rs`
- `src/app/keyboard.rs`
- `src/ui/tokens.rs`
- `src/ui/layouts.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/app/tab_bar.rs` or a rename to `src/app/toolbar.rs`
- `src/app.rs`
- `src/app/playback_bar.rs`
- `src/view_models/mod.rs`
- `src/view_models/app_toolbar.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/search/app_impl.rs`
- `src/library/app_impl.rs`
- Database or application query code
- Playback command/query behavior

## Constraints

- Now Playing stays app-shell-owned under `src/app/`.
- Do not introduce a new `ui/composites` Now Playing component.
- Use named tokens for spacing, radius, colors, typography, and sizes.
- Preserve existing playback action handlers and keyboard shortcuts.
- Keep the visible Library and Discover search fields unchanged in this
  task.

## Implementation Steps

1. Add `view_models::app_toolbar` with display data for app navigation,
   toolbar ids, Now Playing frame labels, and accessibility labels.
2. Reframe `render_tab_bar` as an app toolbar. Rename the module only if
   the diff remains focused and mechanical.
3. Keep a subtle leading app mark and Library/Search/Settings navigation in
   the leading toolbar zone. Do not add visible product naming.
4. Render the Now Playing element inside a subtle trailing frame with a
   stable width range, tokenized padding, and truncating track text.
5. Ensure the transport icon buttons keep stable hit targets and disabled
   visual state.
6. Add or update architecture tests that assert Now Playing remains
   app-shell-owned and toolbar labels come from a view model.

## Acceptance Criteria

- Top bar reads as one toolbar with a distinct Now Playing frame.
- Top-level chrome avoids premature product naming; MusicIndex attribution is
  reserved for a future About/settings surface.
- Existing playback controls still dispatch through current handlers.
- No search behavior changes.
- No raw color or numeric layout literals are added outside allowed
  token layers.
- Now Playing is not extracted into `ui/composites`.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/app/playback_bar.rs`
- `tests/architecture_tests.rs`

Goal:
- Convert the top strip into a toolbar-shaped app shell and give Now
  Playing a distinct framed trailing space without changing search or
  playback behavior.

Constraints:
- Keep Now Playing in `src/app/`.
- Use tokens and view-model display contracts.
- Preserve existing playback command handlers.

Do not touch:
- `src/search/app_impl.rs`
- `src/library/app_impl.rs`
- Database/query code

Acceptance criteria:
- Framed Now Playing renders in the toolbar.
- Existing playback controls still work.
- Architecture tests cover ownership.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The toolbar cannot fit at existing minimum window widths without
  hiding navigation or playback controls.
- Existing token APIs do not expose a suitable surface/frame role.
