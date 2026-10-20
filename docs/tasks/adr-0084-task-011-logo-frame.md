# ADR 0084 Task 011: The Logo Frame For Collections

Status: Implemented - 2026-10-20. Cases R11-01 to R11-11 pass. Mechanical checks are Green. V1 to V7 are open, in the final walk of the run.

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

## Implementation Result - 2026-10-20

### Files Changed

- `src/media/logo_frame.rs` (new), with no renderer type:
  - `LOGO_FRAME_POLYGONS`, `LOGO_FRAME_ASPECT` and `LOGO_FRAME_SHAPE_VERSION`. No other file holds the polygon numbers.
  - `repeat_cells`, the one repeat rule. `LogoMosaicKey`, `BakedShadow`, `frame_height_px` and `canvas_size_px`.
  - `compose_logo_mosaic` crops each cover to fill its cell, and uses the three polygons as the alpha with 4 by 4 supersampling. It bakes the shadow when it gets one. `encode_png` and `logo_frame_svg`.
  - Tests R11-01, R11-07, R11-08 and R11-09 on recorded solid color covers.
- `src/media/image_cache.rs`: the in-memory LRU `logo_mosaics` (256 entries), `peek_logo_mosaic` and `compose_logo_mosaic_blocking(key, shadow)`. A cover that does not decode is dropped, and the repeat rule applies again. Tests R11-08 and R11-09 read recorded covers from the disk cache.
- `src/media/mod.rs`: the new module.
- `src/application/queries/images.rs`: the command `FetchLogoMosaic`, beside `FetchThumbnail`.
- `src/view_models/collection_art.rs` (new), pure: `CollectionKind`, `CollectionArtVm`, `resolve`, `without_provided` and `mosaic_cells`. Tests R11-01 to R11-04.
- `src/view_models/singles_group.rs`: `SinglesGroup::collection_art` replaces `newest_image_href`. `SinglesGroupRowVm::art` replaces `thumbnail_href`. `SinglesGroupPageVm` has the new field `art`.
- `src/view_models/publisher_page.rs`: `PublisherPageVm::collection_art`.
- `src/view_models/library.rs`:
  - `DestinationListRowVm::art`. An Artists row gets the art of `artist_row_art`, and a release row gets `None`.
  - `PlaylistSidebarRowVm::art`, from the new snapshot field `playlist_covers` and `replace_playlist_covers`. `PlaylistDetailVm::collection_art`.
- `src/db.rs`: `playlist_cover_images`, one SQL statement for the first four images of each playlist. `src/playlist_service.rs` and `src/application/queries/playlist.rs` pass it on.
- `src/ui/tokens.rs`: `ArtworkShape::LogoFrame` with `aspect()`, `RASTER_DENSITY` and `ENTITY_FILL_OPACITY`. `ArtworkShadow::baked` gives the `XXl` values as plain numbers. Test R11-10.
- `src/ui/icons.rs`: `LOGO_FRAME_ASSET_PATH`. `InterfaceAssets` serves `artwork/logo-frame.svg` from `logo_frame_svg`.
- `src/ui/composites/collection_art.rs` (new): `CollectionArtImage`, `LogoRaster` and the composite `CollectionArt`.
- `src/ui/composites/detail_header.rs`: `DetailHeader::art` replaces the cover thumbnail with an art element. `src/ui/composites/thumbnail.rs`: `ThumbnailSize::radius` is `pub(crate)`.
- `src/presentation/collection_art.rs` (new): `CollectionArtLoads`, the trait `CollectionArtHost` and `collection_art_image`. It starts each load through `present_command`. The UI thread only peeks.
- `src/library.rs`, `src/library/app_impl.rs`, `src/app.rs` and `src/app/publisher_dispatch.rs`:
  - `LibraryApp` and `TopApp` implement `CollectionArtHost`.
  - The app layer gives the art of the Releases group rows, the Artists rows and the Playlists rows. It also gives the art of the group page, the playlist page and the feed page.
  - `reload_playlists` also reads the playlist images.
- `src/ui/shells/library/destination_lists.rs`, `singles_group.rs`, `playlist_detail.rs`, `detail.rs`, `src/ui/shells/playlist.rs` and `src/ui/shells/publisher.rs`: each place renders `CollectionArt`. `destination_lists.rs` has 491 code lines.
- `tests/architecture_tests.rs`: the guard `adr_0084_logo_frame_r11_11_one_shape_owner_and_no_ui_composition` (R11-11).
- R11-05 and R11-06 tests are in `singles_group.rs`, `publisher_page.rs`, `library.rs` and `src/application/queries/playlist.rs`.

### Behavior Changed

- A singles group row shows the newest four covers of its singles in the logo frame at the `Sm` size. Two or three covers repeat. One cover shows as a square.
- A singles group page shows the same art at the `XXl` size, before its title.
- An artist feed page shows its channel image as a square cover, with the `XXl` radius and shadow. With no channel image, or when it does not load, the page shows the logo mosaic of the first four images of `album_image_urls()`.
- An Artists row shows the logo mosaic of the newest four album images of the row. The monogram "AR" no longer shows on an Artists row.
- A playlist page shows the first four images in playlist order in the logo frame. The image of a track is its own image, else its album image. The monogram "PL" no longer shows.
- A Playlists row shows the same art at the `Sm` size.
- A collection with no cover shows the logo shape in its entity color at 45 percent opacity, with no monogram. A mosaic that still loads shows this fallback.
- At the `XXl` size, the composed image holds the artwork shadow, so the shadow follows the logo shape.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_logo_frame_` | Green, 21 tests |
| `cargo test` | Green, 2053 unit tests and 339 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 339 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred. No test sends a request.

### Deviations From The Task

1. `ArtworkShape` did not exist before this packet. It has one variant, `LogoFrame`. It also holds `RASTER_DENSITY` (2) and `ENTITY_FILL_OPACITY` (0.45).
2. The asset `artwork/logo-frame.svg` is not a file. `InterfaceAssets` builds it from `LOGO_FRAME_POLYGONS`, so no asset file holds the numbers.
3. The repeat rule is `repeat_cells` in `src/media/logo_frame.rs`. `mosaic_cells` of the view model calls it, so the view model and the composition use one rule. Thus `src/view_models/collection_art.rs` imports `src/media`.
4. `compose_logo_mosaic_blocking` takes `Option<BakedShadow>` beside the key. The key holds only `shadow: bool`, and `src/media` cannot read the token.
5. The render path has no window scale factor. Thus the composition uses 2 device pixels for each logical pixel at each UI scale.
6. `src/presentation/collection_art.rs` keeps the load state of each image for the session, as the thumbnail maps do. The LRU of the image cache is a second level.
   An LRU alone can drop a visible row and start a new composition at each render.
7. Each art keeps a square slot of its size. The logo is centered in the slot. Thus a row stays aligned, and a header does not move when a square falls back to the logo.
8. A square that still loads shows a square in the entity color, with no monogram. The entity color logo at `XXl` has no shadow, because a box shadow cannot follow the logo shape.
9. The feed page uses the artist color for each feed page, also for a feed with the level `FeedLevel::None`. The header badge still names that feed "feed".
10. The singles group row no longer animates a GIF cover when the pointer is on it. The row shows the composed mosaic.
11. `render_library_singles_group_page` gets an eighth argument, the header art, with `#[expect(clippy::too_many_arguments)]`.
12. `reload_playlists` reads the playlist images on the UI thread, beside the playlist list read that it already did there.

### Unresolved Concerns

1. V1 to V7 below need the operator. No test proves the rendered shape, the shadow, a theme color or the scroll speed.
2. The Artists list and the Playlists list render each row. On the first show, each row starts one composition at the same time. V7 checks the scroll. This item is not in the responsiveness list of the overhaul plan, because this task permitted an edit of this packet only.
3. The new visual checks V1 to V7 are not in `docs/pending-human-checks.md`. The task instructions named only this packet as a document to edit.
4. `cargo clippy --all-targets -- -D warnings` reports findings in test code of files that this packet does not own. The packet check `cargo clippy -- -D warnings` is Green.

## Operator Visual Check

This check needs a Linux desktop session with a network connection and this checkout. It also needs these items:

- A Library singles group with four or more singles that have covers.
- An artist feed page with a channel image, and an artist feed page with no channel image. Both need two or more album covers.
- An Artists list with an artist that has no cover.
- A playlist with four or more tracks that have images, and an empty playlist.

V6 turns off the network and moves the thumbnail cache. The cleanup steps undo both.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. Open Music, then Releases, in the sidebar.

### V1: The mosaic at the page size

3. Open the singles group page. Look at the art before the title.
4. Look for four covers in the logo shape: three pillars. The background must show in the gaps between the pillars.

**Wrong if:** the art is a square, a cover is stretched, or a gap shows a color that is not the page background.

### V2: The logo at the row size

5. Look at the singles group row in the Releases list. Make sure that the art shows the logo shape at the row size.
6. Look at the rows above and below it. The titles of each row must start in one column.

**Wrong if:** the logo is cut, it is not centered in its slot, or a row title moves to the left or the right.

### V3: The shadow follows the logo

7. Look at the art of the singles group page again, below each pillar.
8. Look for a soft dark shadow under each pillar, and no shadow in the gaps.

**Wrong if:** a rectangle of shadow shows around the logo, or the shadow is cut at an edge.

### V4: The provided image of an artist feed page

9. Open Music, then Artists. Click the row of the artist feed with a channel image.
10. Look for the channel image as a square, with round corners and a shadow, the same as a release page cover.
11. Open the artist feed with no channel image. Look for the logo mosaic of its album covers.

**Wrong if:** the channel image shows in the logo shape, or the page shows the monogram "AR".

### V5: The entity colors in the two themes

12. Open Music, then Artists. Find the artist with no cover. Its row must show the logo in a pale pink artist color.
13. Open Music, then Playlists. The empty playlist must show the logo in a pale green playlist color.
14. Open Settings. Set "Theme" to Light. Do steps 12 and 13 again.

**Wrong if:** a fallback shows a monogram, or a color does not match the entity dot of the same kind in that theme.

### V6: A fallback with no flash

15. Close the app. In a terminal, run `nmcli networking off`.
16. Run `mv ~/.config/v4vmm/thumbnail-cache ~/.config/v4vmm/thumbnail-cache.v6`.
17. Start `target/debug/v4vmm`. Open the artist feed page with a channel image.
18. Look at the header art while the page opens. It can show a square in the artist color for a short time. Then it must show the logo in the artist color.

**Wrong if:** the art shows the monogram "AR" at any time, or the square stays.

Cleanup:

19. Close the app. Run `nmcli networking on`.
20. Run `rm -r ~/.config/v4vmm/thumbnail-cache`, then `mv ~/.config/v4vmm/thumbnail-cache.v6 ~/.config/v4vmm/thumbnail-cache`.

### V7: The lists scroll with no stutter

21. Start `target/debug/v4vmm`. Open Music, then Artists. Scroll the list from the top to the end and back.
22. Open Music, then Playlists. Do step 21 again.

**Wrong if:** the scroll stops, jumps or pauses while the logo images come in.

## Orchestrator Review - 2026-10-20

Pass. The code needs no fix.

- `cargo fmt -- --check` and `cargo clippy -- -D warnings` are Green. `cargo test` gives 2053 library tests and 339 guards, all passing.
- The deviations agree with the packet. The repeat rule lives in `src/media/logo_frame.rs`, so the polygon owner and the rule owner are one file.
- The singles group row no longer animates a GIF cover on hover, because it shows a mosaic now. The orchestrator accepts this.
- V6 turns the network off for a short time, and its cleanup turns it on again.
- The orchestrator added two concerns to the responsiveness list of the overhaul plan, as items 14 and 15.

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
