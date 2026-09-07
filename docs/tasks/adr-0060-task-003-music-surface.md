# ADR 0060 Task 003: Music Surface

Status: Ready - 2026-09-07. Do after task 002.

## Goal

Rename the curation section to `Music`, promote the content filter to a primary
control, give the surface the whole window, and delete two controls that cannot
act.

This is the packet that answers the visual inspection in ADR 0060. The largest
region of the window shows music instead of an empty prompt.

## Files To Inspect

- `docs/adr/0060-workflow-surface-structure.md`
- `docs/adr/0047-library-search-unification.md`
- `docs/adr/0048-content-list-frame-breadcrumb-search.md`
- `src/view_models/workspace/chrome.rs`, for `ContentFilter`
- `src/view_models/app_toolbar.rs`
- `src/app.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/app_toolbar.rs`
- `src/view_models/workspace/chrome.rs`
- `src/app.rs`
- `src/ui/shells/workspace.rs`
- `src/ui/composites/` for the filter control
- `src/view_models/queue_now_playing.rs`
- `src/ui/shells/queue_now_playing.rs`
- `src/app/queue_now_playing.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `is_in_library` in the database
- The `v4vmm library tracks` CLI command and every other CLI name
- Module names under `src/`
- `src/broadcast/**`, `src/runtime/**`
- The `ContentList` navigation stack, breadcrumb, and search routing

## Constraints

- **The rename is user-facing only.** ADR 0060. The curator sees `Music` for the
  surface and `Library` for the subset they have taken in. The database and the
  CLI keep `library`. No user-facing string says `collection`.
- `ContentFilter` already carries `All`, `Library`, and `Index`. Do not invent a
  second scope model. Promote the control that exists.
- **Curation shows nothing operational.** With no show active, this surface
  renders no queue, no transport, and no broadcast status. A surface that cannot
  act is absent, not disabled.
- Search stays a toolbar command. This task does not make any section a search
  destination.
- The detail region is part of this surface, not a separate pane competing with
  it. An empty detail region must not hold the largest share of the window.
- Two labels replace internal terms: `Update available` for changed upstream
  source data, and `New` for an unreviewed release. Both belong in a view model.
- **Delete the volume slider and the liveValue output picker.** Neither can act.
  Traced on 2026-09-07:
  - `VolumeDisplay::new(1.0, true)` is hardcoded. No playback driver,
    playback owner, or command implements volume. ADR 0021 deferred volume and
    the deferred index holds it as item 4.
  - `LiveValueDeviceDisplay::unavailable()` is hardcoded in the adapter. The
    picker has only ever rendered one disabled option reading
    `No liveValue output`. No output routing exists anywhere.

  ADR 0060 says a surface that cannot act is absent, not disabled. A control
  reporting `100%` when the app cannot read or set volume is worse than absent,
  because it answers a question it does not know.

## Implementation Steps

1. Change the toolbar label and accessibility label for the curation section to
   `Music`. Rename `AppToolbarTabKey::Library` and the matching `AppTab` and
   `WorkspaceScreenMount` variants to `Music`, because they are internal names
   and not a public contract.
2. Leave every database column, CLI command, and file path unchanged. Add a
   guard that no user-facing string says `collection`, and that
   `is_in_library` and the CLI names are untouched.
3. Promote the content filter to a primary control on the `Music` surface, with
   the shared filter chip composite. `Library` and `Index` must be reachable in
   one action from the default state.
4. Set the default layout for `Music` so the content region holds the dominant
   share of the window. The detail region opens on selection and does not
   reserve the largest share while empty.
5. Remove any remaining operational pane from the curation layout. After task
   002 the queue lives in `Show`, so this step confirms nothing else remains.
6. Add `Update available` and `New` as view-model owned labels on the row
   display contract. Do not render either from a renderer conditional.
7. Delete the volume slider and the liveValue picker:
   - `VolumeDisplay`, `LiveValueDeviceDisplay`, `LiveValueDeviceOption`, and
     their builder methods, fields, and tests in
     `src/view_models/queue_now_playing.rs`
   - `render_volume` and the picker renderer in
     `src/ui/shells/queue_now_playing.rs`
   - the hardcoded constructions in `src/app/queue_now_playing.rs`
8. **Delete the guards that require them.** `tests/architecture_tests.rs` has
   ADR 0046 Phase 4 guards asserting the queue view model contains
   `LiveValueDeviceDisplay` and `VolumeDisplay`, and that the adapter contains
   `LiveValueDeviceDisplay::unavailable()` and `VolumeDisplay::new(1.0, true)`.
   Those guards enforce decoration and block this deletion. Remove them.
9. Review the toolbar-ownership guard that names queue, liveValue, and volume.
   The queue half stays meaningful. Narrow it rather than deleting it whole.
10. Add guards, marked situational and citing ADR 0060:
    - the curation surface renders no queue, transport, or broadcast status
    - no user-facing string says `collection`
    - the filter control is reachable from the default state
    - no view model carries a volume or output-picker display
11. Capture screenshots: the default state with content, a `Library` filter, an
    `Index` filter, and a selected detail.

## Acceptance Criteria

- The section is labeled `Music` and the subset is labeled `Library`.
- No database column, CLI command, or module name changed.
- The content region holds the dominant share of the window by default.
- An empty detail region does not dominate the layout.
- The filter reaches `Library` and `Index` in one action.
- No operational surface renders during curation.
- `Update available` and `New` come from a view model.
- No volume control and no output picker render anywhere.
- The guards that required them are gone, and the guard suite is smaller.
- Four screenshots exist.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo run` for the visual check

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Screenshots captured
5. Deviations from task
6. Unresolved concerns

## Escalation Triggers

- The default layout share cannot change without altering the split behavior
  that ADR 0051 persists.
- A row needs a readiness fact the current display contract does not carry.
  Report it. `Dump`, audition, and rotation warnings each need their own ADR and
  are not part of this task.
- Renaming an internal variant reaches a public CLI or database name. Stop.
  That boundary is the point of the vocabulary rule.
- Deleting the volume or picker types breaks a caller outside the queue view
  model, its shell, and its adapter. Report the caller. Nothing else should
  depend on a control that never acted.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0060-workflow-surface-structure.md`
- `src/view_models/workspace/chrome.rs`, `src/view_models/app_toolbar.rs`,
  `src/app.rs`, `src/ui/shells/workspace.rs`

Goal:
- Label the curation section `Music`, promote the content filter, and give the
  content region the dominant share of the window.

Constraints:
- User-facing rename only. `is_in_library`, CLI names, and module names stay.
- No user-facing string says `collection`.
- Promote the existing `ContentFilter`. Do not add a second scope model.
- No queue, transport, or broadcast status during curation.
- An empty detail region must not hold the largest share.
- Delete the volume slider and the liveValue picker. Neither can act. Delete the
  ADR 0046 Phase 4 guards that require them to exist.

Acceptance criteria:
- Section reads `Music`, subset reads `Library`, filter reachable in one action.
- No database, CLI, or module rename.
- Four screenshots.

Test commands:
- `cargo fmt -- --check`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. screenshots captured
5. deviations from task
6. unresolved concerns
