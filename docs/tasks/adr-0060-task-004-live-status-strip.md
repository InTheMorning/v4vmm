# ADR 0060 Task 004: Live Status Strip

Status: Implemented - 2026-09-08. Mechanical and visual acceptance met.

## Goal

Add a compact status strip across `Music` and `Settings` that appears only when
a show is active. It replaces the toolbar now-playing chip, so now-playing has
one owner.

## Files To Inspect

- `docs/adr/0060-workflow-surface-structure.md`
- `docs/adr/0040-async-vm-runtime.md`
- `src/view_models/show.rs`
- `src/view_models/app_toolbar.rs`, for the current now-playing chip
- `src/runtime/broadcast_observation.rs`
- `src/playback.rs`, for session state
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/live_status.rs` (new)
- `src/ui/composites/live_status_strip.rs` (new)
- `src/view_models/app_toolbar.rs`
- `src/view_models/mod.rs`, `src/ui/composites/mod.rs`
- `src/app.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/view_models/show.rs` display contract beyond reading from it
- `src/broadcast/**` and the observation actor
- The `ContentList` stack and search routing

## Constraints

- **The strip renders only when a show is active.** A show is active when show
  playback is running or a broadcast is running. With no show, `Music` and
  `Settings` render no operational surface at all. ADR 0060.
- **It is a glance surface, not a control surface.** The only action it
  dispatches is opening `Show`. No transport, no start, no stop.
- **It projects from the same view model as `Show`.** Two independent
  projections will disagree during a live show, which is when an operator
  trusts the display least. One source, two renderers.
- Health must not rely on color alone. Pair every state with a shape, a glyph
  from the icon catalog, or text.
- It replaces the toolbar now-playing chip. Remove the chip in the same change,
  so now-playing has one owner.
- Reading show state blocks. Read it through the runtime, never on the render
  path.

## Implementation Steps

1. Add `src/view_models/live_status.rs` with `LiveStatusDisplay`:
   - `active`, whether a show is running
   - the listener-facing now-playing line, artist and title
   - one aggregate health state, as an enum, with a text label and an icon role
   - recording state and elapsed label, present only while recording
   - an open-show action with its accessibility label
2. Derive the display from the same source `Show` uses. Add a shared projector
   rather than a second query path.
3. Add `src/ui/composites/live_status_strip.rs` that renders the display and
   takes one callback for opening `Show`.
4. Mount the strip at the top of `Music` and `Settings`, and render nothing when
   `active` is false.
5. Remove the now-playing region from the toolbar view model and its renderer.
6. Add guards, marked situational and citing ADR 0060:
   - the strip dispatches no command other than opening `Show`
   - the strip and `Show` read one projector, not two
   - the toolbar view model no longer carries now-playing state
7. Add view-model tests: no show, playing, recording, and a degraded health
   state.
8. Capture screenshots: `Music` with no show, `Music` with a show running, and
   `Settings` with a show running.

## Acceptance Criteria

- The strip is absent when no show is active, in both sections.
- The strip shows now-playing, health, and recording when a show runs.
- Health never depends on color alone.
- The only action is opening `Show`.
- The toolbar no longer carries a now-playing chip.
- One projector feeds both the strip and `Show`.
- No blocking read on the render path.

## Visual Acceptance

A person judges these. They are open until an operator runs the app outside a
headless session and reports each line. Never report them as met from a passing
mechanical run.

- The strip reads as status, not as a second toolbar, and does not crowd the
  section below it.
- Health is legible without color, by shape or text, at a glance.
- The strip appearing and disappearing does not shift the content underneath
  in a way that loses the reader's place.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test live_status --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- Do not run the app. Write the operator visual check instead, as AGENTS.md
  requires.

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Screenshots captured
5. Deviations from task
6. Unresolved concerns

## Escalation Triggers

- `Show` and the strip cannot share a projector without a change to the `Show`
  view model contract. Report it rather than duplicating the projection.
- The aggregate health state needs a service that the blocked ADR 0059 packets
  have not built. Show `Unknown` and say so.
- Removing the toolbar chip breaks a keyboard shortcut or a focus path.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0060-workflow-surface-structure.md`
- `src/view_models/show.rs`, `src/view_models/app_toolbar.rs`

Goal:
- Add a live status strip to `Music` and `Settings` that appears only when a
  show is active, and remove the toolbar now-playing chip.

Constraints:
- Renders only when a show is active. Absent otherwise, never disabled.
- Glance surface. The only action opens `Show`.
- One projector feeds both the strip and `Show`.
- Health never relies on color alone.
- No blocking read on the render path.

Acceptance criteria:
- Absent with no show, complete with one, toolbar chip gone.
- Guards are situational and cite ADR 0060.
- Three screenshots.

Test commands:
- `cargo fmt -- --check`
- `cargo test live_status --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. screenshots captured
5. deviations from task
6. unresolved concerns
