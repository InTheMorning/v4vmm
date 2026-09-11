# ADR 0069 Task 001: Grouped Settings Foundation

Status: Ready - 2026-09-11. Implementation not started.
Operator visual check specified; not runnable or accepted yet.

## Goal

Replace the long Settings form with General, Library and Diagnostics group
navigation, using existing controls and behavior. Preserve edits across group
and app-section changes. Give reports a direct Diagnostics destination.

This is the first independent packet under
[ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md) and its
[phase plan](../plans/adr-0069-settings-presets-phase-plan.md). Do not implement
the shared transaction editor, mode/resource selectors, presets or audio here.

## Files To Inspect

- `AGENTS.md`, `.github/copilot-instructions.md`, `docs/adr/README.md`
- `src/app.rs` — Settings inputs, rendering, save/defaults, cached files and screen mount
- `src/app/capabilities.rs`, `src/app/keyboard.rs`, `src/app/menu.rs`
- `src/config.rs`, `src/view_models/mod.rs`, `src/view_models/cached_files.rs`
- `src/view_models/startup/capabilities.rs`, `src/application/capability.rs`
- `src/ui/primitives/`, `src/ui/composites/`, `src/ui/tokens.rs` — inspect actual navigation/form owners before adding one
- `tests/architecture_tests.rs` — existing Settings, token, focus, runtime and report guards
- `docs/runbooks/startup-recovery-fixture.py`, `docs/runbooks/inherited-ui-checks.md`
- `docs/troubleshooting/column-text-truncation.md`

## Files Likely To Change

- `src/app.rs`, `src/app/settings.rs` (new), `src/app/capabilities.rs`
- `src/view_models/settings.rs` (new), `src/view_models/mod.rs`
- Existing navigation/form primitives and tokens only where shared behavior needs extension
- `src/ui/composites/settings.rs` (new only if needed), `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`
- `docs/runbooks/settings-foundation-check.md` (new; operator procedure)
- This packet, its phase plan/review checklist, delivery index, source map,
  `AGENTS.md` and `docs/pending-human-checks.md` when the visual gate is runnable

## Do Not Touch

Configuration keys, ordinary writer semantics, database schema/content, preset
files, service adapters, audio drivers, playback commands, other repositories,
unrelated UI polish or the accepted Event/Producer/Publisher controls in Show.
No new background discovery, report polling, core-path retarget behavior,
reset behavior or runtime apply lifecycle. Never run the app as an agent.

## Constraints And Implementation Steps

1. Add a renderer-independent Settings group model for the three delivered
   groups. General owns theme/scale; Library owns endpoint/music folder/flac;
   Diagnostics owns existing background reports and cached-file maintenance.
   Keep group labels, selected state and action presentation in the view model.
   Render navigation with accessible typed actions and shared tokens.

2. Extract Settings composition to `src/app/settings.rs`. Reuse existing field
   controls, cached-file view model, report composites and commands. Do not copy
   the old form and leave a second route alive. Screens must not invent
   enablement from strings or filesystem checks. Preserve existing command-side
   save guards; presentation state is not authority to bypass them.

3. Keep the selected group and input entities alive for the app session.
   Initial ordinary Settings entry opens General; subsequent ordinary entry
   restores the selected group. Group switching and returning from Music/Show
   preserve unsaved text and existing appearance previews without saving.
   Do not add a persisted group-selection key. Maintain a usable focus target
   when the focused field is unmounted; preserve ADR 0067 app shortcuts.

4. Keep Save and Use Defaults shared across existing editable groups. Label
   their scope so the operator knows they act on General and Library together.
   Preserve their existing validation, application and global-default behavior.
   Do not introduce Cancel/Apply or claim transactional draft behavior here.
   Show existing field/command errors at their relevant surface; do not move
   every useful correction message into Diagnostics.

5. Make Open report enter Settings and select Diagnostics in one dispatch.
   Preserve reports, their actual recorded times, copy actions and unavailable
   runtime behavior. General/Library must remain reachable with long reports.
   A tab switch does not start a new cache scan; retain existing query caching
   and explicit refresh semantics. Show no queue, transport or broadcast-status
   panel in Settings. Do not add Live Metadata/Audio tabs or preset controls
   before their real editors exist.

6. Bound the content scroll area beneath the group navigation. Keep labels and
   actions reachable at narrow widths and larger existing UI scales, using
   named dimensions. No column `truncate()` fix, raw glyph or ad hoc per-screen
   style. Extend a shared owner if the same geometry is needed repeatedly.

7. Add focused behavioral tests and the situational ADR 0069 ownership guard
   below. Update still-valid existing guards to the new owner without weakening
   their requirements. Publish the operator procedure and build before opening
   this packet's visual gate in all three indexes.

## Acceptance Criteria

Mechanical; names below are new proof targets, not existing tests.

| ID | Proof owner | Required assertion |
|---|---|---|
| M1 | Settings VM tests | General/Library/Diagnostics expose stable labels, selected state, typed accessible navigation and defined field/report membership |
| M2 | Settings navigation/dispatch tests | Group changes and ordinary Settings re-entry retain group/input values and dispatch no save, defaults, service or playback command; explicit Open report selects Diagnostics |
| M3 | Existing save-command tests and focused dispatch coverage | Shared Save/Use Defaults still address all existing settings once; invalid config cannot bypass current writer guards through a new group route |
| M4 | Report/cached-files owner tests | Group navigation preserves issue identity/results and cannot schedule a second cache query when cached data is valid or one query is in flight |
| M5 | New situational guard `adr_0069_settings_group_ownership` | Settings model has no GPUI dependency; thin app composition reaches one live shared form/navigation path and reuses current save/report owners; no new persistence schema |

Visual V1–V3 below are separate gates. Mechanical tests cannot establish
readability, focus delivery or scrolling in the running app.

## Test Commands

Run focused Settings/dispatch tests during edits, then the required checks:

```bash
cargo fmt -- --check
cargo check --locked --offline --quiet
cargo test --locked --offline --quiet adr_0069_
cargo test --locked --offline --test architecture_tests --quiet
cargo clippy --locked --offline --quiet -- -D warnings
cargo build --locked --offline --quiet
git diff --check
```

Also run affected existing config, capability, cached-file and keyboard tests.
Record exact filters and counts; do not use an empty test selection as evidence.
No unrelated lint cleanup and no agent GUI execution.

## Operator Visual Check

Not runnable yet. Implementation must deliver
`docs/runbooks/settings-foundation-check.md` with exact terminal commands for
setup, app launch, inspection and cleanup. Reuse the existing startup fixture
contract and create a fresh disposable fixture; do not clean up or repurpose the
operator's retained ADR 0066 task 004 fixture. The runbook is a deliverable,
not a file or command claimed to exist in this planning packet.

Prerequisites: Linux desktop, rebuilt debug binary, isolated fixture with local
tracks and existing invalid-optional configuration cases. No audio hardware,
reachable MusicIndex endpoint, running broadcast service or playback is needed.

1. V1: Open Settings with Ctrl+Comma. Walk the three groups with mouse and
   keyboard in Light/Dark, normal/narrow widths and an existing larger scale.
   Inspect labels, selected state, scroll bounds and reachable controls. Clipped
   tabs, inaccessible actions or reports displacing group navigation fail.
2. V2: Edit disposable endpoint text without saving, switch groups and visit
   Music, then return. Text and selected group remain. Check Ctrl+F and section
   shortcuts from a field. Verify the shared Save/defaults scope using the
   disposable config and inspection; hidden-group values must not reset.
3. V3: Use an invalid-optional fixture and Open report from its notice. It must
   reach Diagnostics immediately, retain all issues and permit navigation back
   to editable groups. Check report copying, scrolling and cached-file access.
   Close the app and inspect configuration/music preservation before cleanup.

The runbook must name any intended config edits separately from preservation
assertions, and clean up only its own fixture. Add this runnable gate to the
packet Status, delivery Progress row and pending-human index. Retain unrelated
inherited checks; accept only the subclaims the operator actually walks.

## Final Report, Escalation And Rollback

Report changed owners, behavior, exact verification and deviations. Say Green
for passed checks. End with Operator visual check commands or their runbook
link, prerequisites and cleanup; leave acceptance open until walked.

Stop this packet's expansion and document a plan correction if it requires a
new config key, writer, background actor, service operation or audio owner.
Routine composition choices do not need renewed permission. Roll back code as
one change if needed; preserve all operator files and outstanding evidence.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- This entire packet and its Files To Inspect.
- ADR 0069, its phase plan/review checklist and the delivery order.

Goal:
- Group existing Settings under General, Library and Diagnostics.

Constraints:
- Follow steps 1–7 and shared owner, token, action and focus contracts.
- Preserve current saves, global defaults and preview behavior in this packet.

Do not touch:
- Configuration schema, preset persistence, service/audio behavior or other repositories.

Acceptance criteria:
- Prove M1–M5 at their named owners; publish V1–V3 without claiming operator acceptance.

Test commands:
- Run the Test Commands section and affected owner tests; never run the app.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
