# ADR 0083 Task 002: Font And Artwork Tokens

Status: Complete - 2026-10-03. The operator passed V83-11 to V83-15 on 2026-10-03. The operator also saw the Play icon of "Open Show" as a color emoji. [Task 003](../adr-0083-task-003-one-icon-set.md) owns that defect.

## Goal

Embed Figtree as the interface font. Add the display size for page titles.
Give artwork a shadow token, and replace the emoji placeholder with a type monogram.

## Authority

- [ADR 0083](../../adr/0083-design-language.md) Decisions 3 and 4.
- [ADR 0025](../../adr/0025-theme-icon-style-boundary.md): the theme boundary.
- The type ramp guards of the archived ADRs 0034 and 0039 stay in force.
- Durable set in `AGENTS.md`: token discipline.

## Recorded Facts - 2026-10-03

- The app embeds no font. GPUI uses the system interface font. `log_font_family` in `src/ui/tokens.rs` reads `mono_font_family` from the gpui-component theme. `src/ui/theme_bridge.rs` writes the theme through `Theme::global_mut`.
- `../musicindex/assets/fonts/` holds Figtree as `woff2` files and `Figtree-OFL.txt`. The GPUI text system loads TTF and OTF files, not `woff2`.
- Figtree is licensed under the SIL Open Font License 1.1. The license permits embedding when the license text goes with the font.
- `Weight` in `src/ui/tokens.rs` has four values: `Regular`, `Medium`, `Semibold` and `Bold`.
- `FontSize` has seven roles from `Micro` 11 to `Title` 24 at M. Each role has type endpoints for the UI scale steps (ADR 0039). Tests in `src/ui/tokens.rs` and `src/ui/layouts.rs` list each role.
- `ImageSize` in `src/ui/primitives/image.rs` already has `Xl` 152 and `XXl` 200. ADR 0083 needs no new artwork size.
- `src/ui` has no shadow token, and no element draws a shadow.
- `Thumbnail` in `src/ui/composites/thumbnail.rs` draws `EntityKind::emoji` when no image exists.

## Required Changes

1. Add the static Figtree TTF files for the four weights, and the OFL license text, under `src/assets/fonts/`. Take them from the upstream Figtree release. Record the source URL and the version in the packet result.
2. Load the four files into the GPUI text system at startup, before the first window opens. Set Figtree as the interface font family through the theme bridge. Keep the monospace font for logs and identifiers.
3. Add a `Display` role to `FontSize`: 30 pixels at M. Give it type endpoints that continue the pattern of `Title`. Add it to each test that lists the roles.
4. Use `Display` for the title in the header of the album, artist and publisher pages.
5. Add an artwork shadow token in `src/ui/tokens.rs`. A larger image gets a larger shadow. The `Image` primitive applies it to `Xl` and `XXl`, and the content tile applies it to its artwork. No other element uses it.
6. Replace the emoji placeholder of `Thumbnail` with a two-letter type monogram, for example "AL" for a release. Delete `EntityKind::emoji`.
7. Add a situational guard that cites ADR 0083. It fails when a file in `src/ui` other than `tokens.rs` builds a shadow. Its message names ADR 0083 Decision 4 and the fix: use the artwork shadow token.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R83-11 | A test asserts that the four Figtree files load into a GPUI test text system, and that the family name resolves |
| R83-12 | A test asserts that `FontSize::Display` is 30 at M, and that its scaled values grow from XSmall to XLarge |
| R83-13 | The type ramp tests list `Display` and stay Green |
| R83-14 | A test asserts that the artwork shadow of `XXl` is larger than that of `Xl` |
| R83-15 | `Thumbnail` with no image produces the monogram of its kind, and `src/` has no `fn emoji` |
| R83-16 | The ADR 0083 shadow guard fails for a sample source in `src/ui/shells/` that builds a shadow, and its message names ADR 0083 |

## Visual Acceptance Criteria

These are for the operator. They stay open until a person walks them.

- V83-11: Each interface text is Figtree: rounder letters than the system font. Logs stay monospace.
- V83-12: The album, artist and publisher page titles are larger than other titles, and do not wrap badly at the narrow width.
- V83-13: Large covers and tile covers have a soft shadow. Rows and buttons have no shadow.
- V83-14: An item with no cover shows a tinted square with two letters, not an emoji.
- V83-15: Each check holds at the S and L UI scales.

## Exclusions

- No color change. Task 001 owns colors.
- No cover backdrop. A Phase 4 packet owns it.
- No layout change other than the header title size.
- No database change.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../architecture/source-map.md).
- `src/ui/tokens.rs`, `src/ui/layouts.rs`, `src/ui/theme_bridge.rs`, `src/app/bootstrap.rs`.
- `src/ui/primitives/image.rs`, `src/ui/composites/thumbnail.rs`, `src/ui/composites/tag_badge.rs`, `src/ui/composites/detail_header.rs`.
- The content tile composite of the Library grid.
- `tests/architecture_tests.rs`: the type ramp and raw literal guards.

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

## Result - 2026-10-03

- Font source: `erikdkennedy/figtree`, tag `v2.0.3`, commit `be6cb018f2f93a9b1195f3dfd077123f718c65f8`. The four static TTF files and the OFL text are in `src/assets/fonts/figtree/`. The `google/fonts` copy has only a variable font.
- `src/ui/fonts.rs` loads the fonts. `src/app/bootstrap.rs` calls it before the first window opens. `src/ui/theme_bridge.rs` sets the interface font family.
- `FontSize::Display` is 30 pixels at M, with the type endpoints `(0.84, 1.08)`.
- `ArtworkShadow` has two values:
  - `Xl`: 16 pixels of blur, 4 pixels of offset, 28 % black.
  - `XXl`: 24 pixels of blur, 6 pixels of offset, 34 % black.
- The album, artist and publisher page headers use `Display` through a `title_size` builder on `DetailHeader`. The playlist header keeps `Title2`.
- Only the Music content tile uses `ImageSize::Xl`, so it is the one artwork with a shadow today. The page headers show 80 pixel artwork, which has no shadow. `ImageSize::XXl` has no screen caller. A Phase 4 packet moves the header covers to 200 pixels.
- Monograms: AL release, AR artist, FD feed, TR track, PB publisher, RC recording, PL playlist, IT generic.
- `ttf-parser` is a new dev-dependency. The GPUI test text system does not read font names, so a test parses the files to prove the family name.
- Proof:
  - R83-11: `adr_0083_figtree_fonts_load_into_the_test_text_system` and `adr_0083_figtree_file_family_names_resolve`.
  - R83-12: `adr_0083_display_is_30_at_medium_and_grows_with_scale`.
  - R83-13: the six `adr_0039_type_*` tests in `src/ui/tokens.rs` list `Display`.
  - R83-14: `adr_0083_artwork_shadow_grows_with_image_size`.
  - R83-15: `thumbnail_fallback_shows_the_entity_kind_monogram`, `entity_kind_monogram_is_two_letters_for_every_kind` and `adr_0083_no_emoji_fallback_function_remains`.
  - R83-16: `adr_0083_only_tokens_builds_a_shadow` and `adr_0083_shadow_guard_fails_for_a_shell_sample`.
- The project gate is Green.

## Operator Visual Check

1. In a desktop session, run `cargo build --bin v4vmm`, then `./target/debug/v4vmm`.
2. Look at the text on any screen. Expected: Figtree, with rounder letters than before. Bold and semibold text, such as titles and selected buttons, must look heavier than body text. Wrong: the old system font, or all text at one weight.
3. Open Settings, then the diagnostics or a log. Expected: log text stays monospace.
4. Open an album page, an artist page and a publisher page. Expected: the title is larger than the other titles on the page.
5. Make the window narrow. Expected: the title wraps or fits with no clipped letters. Open a playlist page. Expected: its title has the size of before.
6. Open Music in Tiles view. Expected: each cover has a soft shadow below it. Expected: rows, buttons and the small cover in a page header have no shadow.
7. Find an item with no cover, or watch a tile while its cover loads. Expected: a tinted square with two letters, for example "AL". Wrong: an emoji.
8. In Settings, General, set UI scale to S, then L, and repeat steps 4 to 6. Do not click Save.
9. Cleanup: set UI scale back to M.

No special hardware or system state is needed.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0083-task-002-font-and-artwork-tokens.md`
- ADR 0083 and ADR 0025
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": embed Figtree, add the display size, add the artwork shadow, replace the emoji placeholder, and add the ADR 0083 shadow guard.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Each size and shadow is a named token. No raw literal in a screen, shell or composite.
- Keep the OFL license text beside the font files.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git mv` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- Color tokens and theme profile colors.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The `../musicindex` and `../stophammer` checkouts.

Acceptance criteria:
- Each case R83-11 to R83-16 has proof.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. the font source URL and version
5. deviations from task
6. unresolved concerns
