# ADR 0083 Task 003: One Icon Set

Status: Open - implementation and mechanical checks are complete on 2026-10-04. The operator visual check is pending: [pending check 37](../pending-human-checks.md#37-one-icon-set--adr-0083-task-003).

## Goal

Draw each interface icon from the Lucide set of `gpui-kit-assets`. No icon is a text character.
This corrects a defect of task 002: the Play icon of "Open Show" shows as a color emoji.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decision 10.
- [ADR 0025](../adr/0025-theme-icon-style-boundary.md): the icon boundary.
- Durable set in `AGENTS.md`: token discipline, no glyph string in a renderer.

## Recorded Facts - 2026-10-03

- On 2026-10-03 the operator saw the Play icon of "Open Show" as an orange color emoji. Before task 002 it was a blue triangle.
- Cause: `Icon::render` in `src/ui/icons.rs` draws most icons as a text character in the interface font. Figtree has no character for `▶`, so the text system takes it from a color emoji font. Other icon characters can show the same defect: `⏸`, `⏹`, `⏮`, `⏭`, `⚠`, `⋯`, `☰`, `⊘`, `⌄` and `✓`.
- `IconName::glyph` gives a character for each icon except `Rss`, `Nostr` and `Search`. `Search` uses `ComponentIconName::Search`. `Rss` and `Nostr` use catalog SVG images.
- These files also put icon characters into text:
  - `src/view_models/library.rs`: disclosure characters `▼` and `▶` (two places), and `play_label: "▶"` (three places).
  - `src/ui/composites/disclosure_group.rs`: the disclosure character `▶`.
  - `src/ui/composites/tag_badge.rs`: `StatusRole::glyph`, with `✓` and `⚠`.
- `gpui-kit-assets` 0.6.1 holds these Lucide files: `play.svg`, `pause.svg`, `square.svg` or `circle-stop.svg`, `skip-back.svg`, `skip-forward.svg`, `plus.svg`, `arrow-left.svg`, `chevron-left.svg`, `chevron-right.svg`, `chevron-down.svg`, `check.svg`, `x.svg`, `info.svg`, `ellipsis.svg`, `grip-vertical.svg`, `ban.svg` and `triangle-alert.svg`.

## Required Changes

1. Give each `IconName` a Lucide icon. Use `ComponentIconName` where it has the icon. Otherwise load the SVG of `gpui-kit-assets` through the asset path that `Search` uses. Keep `Rss` and `Nostr` as they are.
2. Delete `IconName::glyph` and the text branch of `Icon::render` when no caller remains.
3. Change the view models in `src/view_models/library.rs` so that they carry an `IconName` or a typed disclosure state, not a character. The renderer draws the icon.
4. Change `disclosure_group.rs` and the `StatusRole` mark of `tag_badge.rs` to draw an `Icon`. A status mark keeps its word beside the icon (durable set: never rely on color alone).
5. Add a situational guard that cites ADR 0083. It fails when a non-test file in `src/ui` or `src/view_models` holds an icon character of "Recorded Facts". The guard finds a literal character and a `\u{...}` escape. Its message names ADR 0083 Decision 10 and the fix: use an `IconName`.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R83-21 | A test asserts that each `IconName` other than `Rss` and `Nostr` resolves to a Lucide icon |
| R83-22 | `src/` declares no `fn glyph` on `IconName` |
| R83-23 | The library view models expose an `IconName` or a disclosure state for each disclosure and play control, and no character |
| R83-24 | The ADR 0083 icon guard fails for a sample source with `"\u{25B6}"` in `src/view_models/`, and its message names ADR 0083 |

## Visual Acceptance Criteria

These are for the operator. They stay open until a person walks them.

- V83-21: "Open Show" shows a line-style Play icon in the button color, not an emoji.
- V83-22: disclosure arrows in the Library sidebar and in collapsible groups are line icons, and they turn when the group opens.
- V83-23: the Show transport, the "⋯" menus, the drag handles and the status marks show line icons of one style.
- V83-24: each icon keeps its size at the S and L UI scales.

## Exclusions

- No new action, no layout change and no new icon placement.
- No color change.
- No database change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/ui/icons.rs`, `src/ui/composites/disclosure_group.rs`, `src/ui/composites/tag_badge.rs`, `src/ui/shells/playlist.rs`.
- `src/view_models/library.rs`: the disclosure and `play_label` fields and their renderers.
- The `gpui-kit-assets` 0.6.1 crate: its asset source and its `icons/` folder.
- `tests/architecture_tests.rs`: the icon and raw literal guards.

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Result - 2026-10-04

- Each `IconName` other than `Rss` and `Nostr` is a Lucide icon. Fifteen come from `ComponentIconName`. `Stop` (square), `Previous` (skip-back), `Next` (skip-forward), `DragHandle` (grip-vertical) and `NotAllowed` (ban) are not in the default bundle of `gpui-kit-assets`. A new `InterfaceAssets` source in `src/ui/icons.rs` embeds these five SVG files with the default bundle. `src/app/bootstrap.rs` registers it.
- `registered_asset_source_serves_every_icon_name_path` loads the path of each `IconName` through `InterfaceAssets` and asserts that it gives bytes.
- `IconName::glyph` and the text branch of `Icon::render` are deleted.
- The library tree and the playlist sidebar carry `expanded: bool`. The playlist row carries `PlaylistPlayAffordance`: a Play icon, or the words "Repair playback" as before.
- `DisclosureIndicator` and the disclosure of `SectionHeader` in `src/ui/primitives/section_header.rs` draw a chevron icon. `SectionHeader` drew `>` and `v` before. The chevrons use `IconSize::Transport`, so they can look a little larger than before.
- `StatusRole::glyph` became `StatusRole::icon`. No screen calls it today. Screens use only `StatusRole::color`.
- The new guard reads the full file, test blocks included, because `src/view_models/library.rs` has a test block before production code.
- Proof:
  - R83-21: `every_icon_name_other_than_rss_and_nostr_resolves_to_a_lucide_icon`.
  - R83-22: `adr_0083_icon_name_has_no_text_glyph_fallback`.
  - R83-23: the library tree, playlist sidebar and playlist row tests in `src/view_models/library.rs`, and `indicator_owns_display_expanded_state`.
  - R83-24: `adr_0083_icon_characters_stay_out_of_ui_and_view_model_source` and `adr_0083_icon_guard_fails_for_a_sample_play_character`.
- The project gate is Green.

## Operator Visual Check

1. In a desktop session, run `cargo build --bin v4vmm`, then `./target/debug/v4vmm`.
2. Look at "Open Show". Expected: a line-style triangle in the button color. Wrong: an orange emoji, or no icon.
3. In the Library sidebar, open and close an artist, an album and the Playlists group. Expected: a line chevron that points right when closed and down when open. Wrong: a filled triangle, an empty box, or no arrow.
4. Open a page with a collapsible description and open and close it. Expected: the same chevron, with its word beside it. Wrong: `>` or `v`.
5. Open a playlist with a playable track. Expected: a line Play icon on the row, and a dotted grip icon for the drag handle. A track that cannot play shows "Repair playback".
6. Open a "⋯" menu button. Expected: three dots in the same line style.
7. If a show is active, look at the Show transport. Expected: Previous, Play or Pause, and Next in the same line style.
8. In Settings, General, set UI scale to S, then L, and repeat steps 2 to 4. Expected: each icon stays sharp and in line with its text. Do not click Save.
9. Cleanup: set UI scale back to M.

No special hardware or system state is needed. Step 7 needs an active show, and the check can pass without it.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0083-task-003-one-icon-set.md`
- ADR 0083 and ADR 0025
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": a Lucide icon for each `IconName`, no icon character in a view model or renderer, and the ADR 0083 icon guard.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Each icon keeps its accessibility label.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git mv` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- Color, font, size and shadow tokens.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The `../musicindex` and `../stophammer` checkouts.

Acceptance criteria:
- Each case R83-21 to R83-24 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. the Lucide icon chosen for each `IconName`
5. deviations from task
6. unresolved concerns
