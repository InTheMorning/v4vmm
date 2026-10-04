# ADR 0083 Task 001: Palette And Color Roles

Status: Ready - 2026-10-03. Implementation has not started. The operator visual check opens when the packet is complete.

## Goal

Change the semantic colors to the search.html palette of ADR 0083 Decision 1.
Give each entity kind its own color, and stop the use of status colors for entity kinds.
Make the dark warning color orange, so that gold is free for the value palette.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decisions 1 and 2.
- [ADR 0025](../adr/0025-theme-icon-style-boundary.md), as amended by ADR 0083: the theme and badge boundary.
- Durable set in `AGENTS.md`: token discipline, never rely on color alone.

## Recorded Facts - 2026-10-03

- `SemanticColor::dark_palette` and `light_palette` in `src/ui/tokens.rs` hold the Dark and Light values. `src/ui/theme_profiles.rs` holds the two high-contrast profiles.
- Today the dark profile is blue-violet: `SystemBackground` `#0f1117`, `Label` `#eceef5`, `Accent` `#8b9bff`. The light profile follows Apple HIG: `Accent` `#007aff`.
- The app has three background tokens: `SystemBackground`, `SecondarySystemBackground` and `TertiarySystemBackground`. The website has four: background, sidebar, surface and raised surface.
- The app has four label tokens. The website has two: text and muted text.
- The dark `Warning` and `WarningLabel` are `#ffd666`. ADR 0083 uses `#ffd666` for the largest payment share. The light `Warning` is orange `#ff9500`.
- `EntityKind::fill_token` and `on_fill_token` in `src/ui/composites/tag_badge.rs` map each kind to a status token: Artist to `Success`, Feed to `Warning`, Track and Playlist to `Info`, Publisher to `Danger`, Release and Recording to `Accent`. Only `tag_badge.rs` calls them. `EntityKind` has these variants: `Artist`, `Feed`, `Track`, `Publisher`, `Release`, `Recording`, `Playlist` and `Generic`.
- The entity colors of `../musicindex/search.html`:

  | Kind | Dark | Light |
  |---|---|---|
  | feed | `#c4965b` | `#8a5e21` |
  | track | `#008b99` | `#008e9c` |
  | playlist | `#8edfad` | `#006238` |
  | artist | `#fab5e5` | `#723763` |
  | publisher | `#7c60d4` | `#8267db` |

- `src/ui/contrast.rs` tests a WCAG matrix for each profile.
- No surface draws a payment split bar today. The value palette waits for the Phase 4 packet that draws one, because a token with no caller is dead code.

## Required Changes

1. Change the Dark and Light values of the background, label, separator, fill, selection and accent tokens to the values of ADR 0083 Decision 1. Use this mapping:

   | Website role | Token |
   |---|---|
   | background | `SystemBackground` |
   | surface | `SecondarySystemBackground` |
   | raised surface | `TertiarySystemBackground` |
   | sidebar | a new `SidebarBackground` token, used by the sidebar of the workspace shell |
   | text | `Label` |
   | muted text | `SecondaryLabel` |

   Derive `TertiaryLabel`, `QuaternaryLabel`, the fills, `AccentHover`, `AccentPressed`, `Focus` and `Info` from the same palette. Each derived value must pass the contrast matrix. Record each derived value and its source in the packet result.
2. Make the dark `Warning` and `WarningLabel` orange. Start from `#ff9f0a` and keep the dark contrast pairs. Move the dark `DiffDifferent` and `Id3FrameV23Only` away from gold in the same way. Do not change the light values of these tokens unless a contrast test fails.
3. Add one entity color token for each entity kind, for Dark and Light, with the values of the table above. `Release` uses the feed color, and `Recording` uses the track color. `Generic` has no entity color and uses a neutral fill.
4. Change `EntityKind` so that it resolves only entity tokens. Delete `on_fill_token` and `on_fill_color` when no caller remains.
5. Change the entity badge to a small dot in the entity color, followed by the kind word in a label color. No text is drawn on an entity color. The thumbnail placeholder may keep its tint.
6. Keep the high-contrast profiles. Give the new tokens high-contrast values that pass the matrix.
7. Add the contrast pairs of the new tokens to `src/ui/contrast.rs`: each entity color as a dot against each background, and each label color against `SidebarBackground`.
8. Add a situational guard that cites ADR 0083. It fails when `EntityKind` in `tag_badge.rs` names a status, accent or diff token. Its message names ADR 0083 Decision 2 and the fix: use the entity token of the kind.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R83-01 | A test asserts the Dark and Light values of each Decision 1 role through `SemanticColor::resolve` |
| R83-02 | The contrast matrix passes for Dark, Light and both high-contrast profiles, with the new pairs |
| R83-03 | A test asserts that each `EntityKind` resolves to an entity token, and that no two kinds of different website colors share one token |
| R83-04 | The ADR 0083 guard fails for a sample source in which `fill_token` returns `SemanticColor::Warning`, and its message names ADR 0083 |
| R83-05 | A test asserts that the dark `Warning` is not `#ffd666` |
| R83-06 | `screens_do_not_reintroduce_raw_color_or_numeric_px_literals` stays Green |

## Visual Acceptance Criteria

These are for the operator. They stay open until a person walks them.

- V83-01: Music, Show and Settings in Dark look neutral near-black with one blue accent, as on the website.
- V83-02: The same screens in Light are white and gray with the darker blue accent.
- V83-03: An entity kind shows as a colored dot and a word. Track and playlist differ, and publisher is not red.
- V83-04: A warning, such as files to update, is orange in Dark.
- V83-05: The high-contrast profiles look as before.

## Exclusions

- No font change, no radius change and no shadow. Task 002 owns them.
- No value palette token. The Phase 4 split bar packet adds it.
- No layout change, no new screen and no status bar.
- No database change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/ui/tokens.rs`, `src/ui/theme_profiles.rs`, `src/ui/contrast.rs`, `src/ui/theme_bridge.rs`.
- `src/ui/composites/tag_badge.rs` and `src/ui/composites/thumbnail.rs`.
- The sidebar of `src/ui/shells/workspace.rs`.
- `../musicindex/search.html`, lines 40 to 130: the website tokens. Read only.
- `tests/architecture_tests.rs`: the raw literal guard.

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

Revert the working tree. This packet adds no migration and no stored data. A saved theme profile still loads.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0083-task-001-palette-and-color-roles.md`
- ADR 0083 and ADR 0025
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the search.html palette, the entity color tokens, the entity dot, the orange dark warning, the contrast pairs and the ADR 0083 guard.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Each color is a named token. No raw color in a screen, shell or composite.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git mv` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- Font, radius, shadow and artwork tokens.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The `../musicindex` and `../stophammer` checkouts.

Acceptance criteria:
- Each case R83-01 to R83-06 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. each derived color value and its source
5. deviations from task
6. unresolved concerns
