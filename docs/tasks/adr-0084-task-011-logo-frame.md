# ADR 0084 Task 011: The Logo Frame For Collections

Status: Ready - 2026-10-20.

## Goal

Each collection shows its art in the MusicIndex logo frame: a singles group, an artist page and a playlist. The frame holds a mosaic of up to four covers.
A collection with one cover shows that cover as a square. A collection with no cover shows its entity color. The Artists rows lose the monogram "AR".

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 12, amended on 2026-10-20: the logo frame, the mosaic, the square, the entity color, and the image that an artist feed gives.
- [ADR 0083](../adr/0083-design-language.md) Decision 4: the artwork tokens, the artwork shadow, and the placeholder in the entity color.
- [ADR 0040](../adr/README.md): the composition runs on the async command runner, off the UI thread.
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md): packet 024 stores the image of an artist feed. This packet shows the image only where the code already has it.

## Recorded Facts - 2026-10-20

- The logo shape is the `mi-mark-clip` clip path of `../musicindex/search.html` (lines 1709 to 1713). The same three polygons are in `../musicindex/assets/musicindex-mark.svg` and `src/assets/music_network_logo_vectorized.svg`. The bounding box is 735 by 681, not a square.
- The repeat rule is `mosaicCells` of `search.html` (lines 2342 to 2347): two covers give A B / B A, three covers give A B / C A, and four or more give the first four.
- `search.html` shows one image as a square cover, not in the logo (`heroHtml`, lines 2430 to 2433).
- GPUI 0.3.1 has no clip path and no mask for `img()`. `svg()` paints one tint color only (`src/ui/icons.rs`). A layered element thus cannot clip four covers to the logo.
- `ImageCache::read_or_download` (`src/media/image_cache.rs`) keeps downscaled covers of 512 pixels on disk. `FetchThumbnail` (`src/application/queries/images.rs`) and `thumbnail_for_url` (`src/library/app_impl.rs`) load them off the UI thread, and the UI thread only does a `peek`.
- Collection art today:
  - Singles group row: the cover of the newest single (`src/view_models/singles_group.rs`, `src/ui/shells/library/destination_lists.rs`).
  - Singles group page: no art (`SinglesGroupPageVm`, `src/ui/shells/library/singles_group.rs`).
  - Artist feed page: `feed_image_url`, else the monogram "AR", at the `Lg` size (`src/view_models/publisher_page.rs`, `src/ui/shells/publisher.rs`).
  - Artists rows: the monogram "AR" (`thumbnail_href: None` in `artist_list_row`, `src/view_models/library.rs`).
  - Playlist page: the monogram "PL" at the `Lg` size (`src/ui/shells/playlist.rs`).
  - Playlists rows: no image.
- The Library does not store the image of an artist feed. Packet 024 adds it.

## Design

### The token

- Add `ArtworkShape::LogoFrame` to `src/ui/tokens.rs`, with `aspect()` equal to 735 / 681.
- Put the normalized polygons in `src/media/logo_frame.rs` as `LOGO_FRAME_POLYGONS`. The file has no renderer type, because `src/media` does not import `src/ui`. The token names that constant. No other file holds the numbers.
- Add one SVG asset, `artwork/logo-frame.svg`, through `InterfaceAssets` (`src/ui/icons.rs`), for the entity color fallback.

### The view model

Add `src/view_models/collection_art.rs`. It is pure.

```rust
pub(crate) enum CollectionKind { SinglesGroup, Artist, Playlist }
pub(crate) enum CollectionArtVm {
    ProvidedImage { url: String, fallback: Box<CollectionArtVm> },
    SingleCover(String),
    Mosaic([String; 4]),
    EntityColor(CollectionKind),
}
```

- `resolve(provided, covers, kind)` trims each URL, drops each empty URL and each repeated URL, keeps the order (newest first), and keeps four at most.
- The order of choice: a provided image, then one cover gives `SingleCover`, two or more give `Mosaic`, and none gives `EntityColor`.
- `mosaic_cells` applies the repeat rule. It gives `None` for fewer than two covers.
- `without_provided()` gives the fallback, for an image that does not load.
- The entity colors: `Artist` uses `EntityKind::Artist`, `SinglesGroup` uses `EntityKind::Release`, and `Playlist` uses `EntityKind::Playlist`. The fallback shows no monogram.

### The composition

- Add the command `FetchLogoMosaic` beside `FetchThumbnail`. It runs through `present_command`, as `thumbnail_for_url` does.
- `ImageCache::compose_logo_mosaic_blocking(key)` reads each cover and crops it to fill its cell. It draws the four cells, and uses the three polygons as the alpha, with 4 by 4 supersampling. At the `XXl` size it bakes the shadow from the `ArtworkShadow::XXl` token values, given as plain numbers. It encodes a PNG and keeps it in an in-memory LRU.
- `LogoMosaicKey { cells: [String; 4], width_px: u32, shadow: bool, shape_version: u8 }`. The cells come after the repeat rule, so equal art gives one key.
- A cover that does not decode is dropped, and the repeat rule applies again. With no cover left, the result is `None`, and the renderer shows the entity color. With one cover left, it fills the logo.
- No file in `src/ui` calls the composition.

### Each place

| Place | After | Size | Covers | Provided image |
|---|---|---|---|---|
| Singles group row | Mosaic | `Sm` | The newest four images of the singles | None in this packet |
| Singles group page | Header art | `XXl` | The same as the row | None in this packet |
| Artist feed page | The provided image as a square, else the mosaic | `XXl` | The first four images of `album_image_urls()` | `feed_image_url`, as today |
| Artists rows | Mosaic. Packet 024 adds the provided image | `Sm` | The newest four images of the member albums | Packet 024 |
| Playlist page | Mosaic | `XXl` | The first four track images, else the album images, in playlist order | None |
| Playlists rows | Mosaic | `Sm` | One database read for the first four images of each playlist | None |

- The read for the Playlists rows is one SQL statement, not one read for each row.
- A provided image uses the `XXl` radius and shadow of a square cover.

## Required Changes

1. Add the token, the polygon module and the asset.
2. Add `collection_art.rs` and its tests.
3. Add the composition command and the cache.
4. Give each place in the table its `CollectionArtVm`, and render it.
5. Add the guards.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_logo_frame_` for unit tests beside the owning code. Each new guard cites ADR 0084 Decision 12.

| Case | Required proof |
|---|---|
| R11-01 | `mosaic_cells` gives A B B A for two covers, A B C A for three, and the first four for five. It gives `None` for none and for one |
| R11-02 | `resolve` drops each empty URL and each repeated URL before it counts |
| R11-03 | A provided image wins over four covers. `without_provided()` gives the mosaic |
| R11-04 | One cover gives `SingleCover`. No cover gives `EntityColor` of the collection kind |
| R11-05 | Each place in the table gives a `CollectionArtVm` from the sources that the table names |
| R11-06 | An Artists row gives `Mosaic` or `SingleCover` or `EntityColor`, never the monogram |
| R11-07 | Equal cells and an equal size give one `LogoMosaicKey`. A changed cover order gives a different key |
| R11-08 | For a recorded 2 by 2 fixture, the result has alpha 0 at a point between the pillars, and alpha 255 in a pillar |
| R11-09 | The composition gives `None` when no cover decodes |
| R11-10 | `ArtworkShape::LogoFrame.aspect()` is 735 / 681 |
| R11-11 | A source guard: only `src/media/logo_frame.rs` holds the polygon numbers, and no file in `src/ui` calls `compose_logo_mosaic` |

## Visual Acceptance Criteria

These are for the operator, in the final walk of the run. No test proves them.

- V1: at the page size, the mosaic is clear, and the gaps between the pillars show the background.
- V2: at the row size, the shape is still the logo, and each row stays aligned.
- V3: the shadow follows the logo shape, with no rectangle.
- V4: the provided image of an artist feed page is a square with the radius and the shadow of a cover.
- V5: the entity colors are correct in the Dark and the Light theme.
- V6: an image that does not load falls back with no flash of a monogram.
- V7: the Artists list and the Playlists list scroll with no stutter.

## Exclusions

- No storage of the image of an artist feed. Packet 024 owns it.
- No name level page. Packet 017 owns it.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `../musicindex/search.html` and `../musicindex/assets/musicindex-mark.svg`, read only.
- `src/ui/tokens.rs`, `src/ui/icons.rs`, `src/ui/primitives/thumbnail.rs`, `src/ui/composites/detail_header.rs`.
- `src/media/image_cache.rs`, `src/application/queries/images.rs`, `src/library/app_impl.rs` (`thumbnail_for_url`).
- `src/view_models/singles_group.rs`, `src/view_models/publisher_page.rs`, `src/view_models/library.rs` (`artist_list_row`, `PlaylistSidebarRowVm`).
- `src/ui/shells/library/destination_lists.rs`, `src/ui/shells/library/singles_group.rs`, `src/ui/shells/publisher.rs`, `src/ui/shells/playlist.rs`.
- `src/db.rs` for the playlist image read.
- `tests/architecture_tests.rs`: `adr_0083_only_tokens_builds_a_shadow`, the `cx.spawn` guard of screens, and `surface_modules_under_500_loc`.

## Checks

```bash
cargo test --lib adr_0084_logo_frame_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. The composed images live in memory only.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-011-logo-frame.md`
- ADR 0084 Decision 12 with its amendment of 2026-10-20, and ADR 0083 Decision 4
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the token, the view model, the composition and each place.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each art choice. The screen only composes. Use named tokens. No raw literal in a renderer.
- No decode and no composition on the UI thread. The UI thread only reads the cache.
- Tests use recorded values and recorded images. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The download queue, the selection model and the release page assembly.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R11-01 to R11-11 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V7.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The image crate of the project cannot draw an alpha mask.
- The present command runner cannot return an image to the cache.
- A file in `src/ui/shells/library` goes above the ADR 0038 line ceiling, and a split would move code out of its owner.
- A change needs a file in "Do not touch".
