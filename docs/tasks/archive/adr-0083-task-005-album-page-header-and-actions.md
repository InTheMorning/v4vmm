# ADR 0083 Task 005: Album Page Header And Actions

Status: Complete - 2026-10-08. Implementation and mechanical checks are complete on 2026-10-07. The operator passed the visual check on 2026-10-08.

## Goal

Give the Library album page and the Index album page one header and one action row.
The header shows the cover at 200 pixels over a backdrop in the main color of the cover. The actions follow the action hierarchy of ADR 0083.

## Authority

- [ADR 0083](../../adr/0083-design-language.md) Decisions 4, 5 and 10, with the 2026-10-07 amendment of Decision 5.
- [ADR 0037](../../adr/0037-same-entity-surface-parity.md): one page grammar for local and Index origins.
- The album page rules in the [overhaul plan](../../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07), section "Phase 4: Album Page": §3, §15 and §30.

## Recorded Facts - 2026-10-07

- `render_library_feed_detail` in `src/ui/shells/library/feed_detail.rs` builds the Library album actions as one row of plain buttons: "Download Feed" or "Remove Feed", "MusicBrainz", "Add feed to playlist ▾" and "Open publisher".
- `index_feed_primary_actions` in `src/app/search_dispatch.rs` builds the Index album actions: "Download Feed", "MusicBrainz" and "Add feed to playlist ▾".
- `ActionState::playlist_action` in `src/view_models/entity_detail.rs` puts "▾" and "▴" into the label.
- Both pages use `render_release_detail_shell` in `src/ui/shells/entity.rs`. Its header is `DetailHeader` with the `Display` title size. The header cover is 80 pixels.
- `LibraryAlbumDetailVm::release_action_state` gives `RemoteOnly`, `Downloading`, `InLibrary` and `Removing`.
- `ImageCache` in `src/media/image_cache.rs` downloads and downscales covers off the UI thread in `fetch_blocking` and `downscale`. The `image` crate is a dependency.
- GPUI has no image blur. ADR 0083 Decision 4 leaves the backdrop technique to a packet.

## Required Changes

1. Add one album page action model in `src/view_models/` for both origins, like `TrackPageActions`:

   | Album state | Filled action | Plain actions | "⋯" items |
   |---|---|---|---|
   | Index album | "Download album" | none | "Copy feed URL" |
   | Library album with no Library track (`RemoteOnly`) | "Download album" | none | "Copy feed URL", "MusicBrainz lookup" |
   | Library album with Library tracks (`InLibrary`) | "Add to playlist" | none | "Copy feed URL", "MusicBrainz lookup", "Remove album…" |

   - A busy download or removal makes the filled action and "Remove album…" unavailable.
   - "Remove album…" is last, after a divider, in the danger color. It always asks for confirmation (`RemovalConfirmation::Always`).
   - "MusicBrainz lookup" keeps the availability and the reason of today's album lookup.
2. The artist name and the publisher under the album title are links. The artist name opens the artist page by name. The publisher link opens the publisher page (ADR 0077). Its label is "Publisher", because `publisher_text` is the feed owner (ADR 0077 Decision 6). Delete the "Open publisher" button.
3. The header cover uses `ImageSize::XXl` (200 pixels), which has the artwork shadow.
4. The backdrop:
   - When an album page asks for the color of its cover, `ImageCache` computes the main color from the downscaled cached image. Other thumbnails get no color, because a computation for each thumbnail uses CPU time for no result.
   - The computation runs off the UI thread. The cache keeps the color beside the image.
   - The album page view model gets the color as a plain value with no renderer type. The screen resolves it like the cover image.
   - The header draws a gradient from the color to the background color. Its height and strength are named tokens. Text keeps its contrast: the title and the names sit on the background end of the gradient, or a contrast test proves them.
   - With no color, the header draws no backdrop.
5. No action label holds an icon character. "Add to playlist" replaces "Add feed to playlist ▾" on both pages.
6. Delete each builder and label that no path reaches after the change.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R83-51 | For each album state of the table, the model gives exactly one filled action, the plain actions and the "⋯" items |
| R83-52 | "Remove album…" is last, marked destructive, and its command confirms each plan |
| R83-53 | The model gives the artist name and the publisher name as link targets, and no "Open publisher" action exists |
| R83-54 | The cover color of a known image is one stable value. An image with no pixels gives no color |
| R83-55 | The header cover size is `ImageSize::XXl`, and the backdrop height and strength are tokens |
| R83-56 | No label in `src/view_models` holds "▾" or "▴", and the ADR 0083 guards stay Green |
| R83-57 | The Library album page and the Index album page build through `render_release_detail_shell` |

## Visual Acceptance Criteria

These are for the operator. They stay open until a person walks them.

- V83-51: a Library album page and the Index page of the same album show the same header and action row. Check in Light and Dark.
- V83-52: the cover is large, with a soft shadow, over a backdrop in its main color. The backdrop fades into the page before the facts.
- V83-53: one filled button, and "⋯" opens a menu with "Remove album…" last and red.
- V83-54: the artist and publisher names are links. No "Open publisher" button shows.
- V83-55: the moved rules §3, §15 and §30 hold on the new page.
- V83-56: normal and narrow widths show each element in its place, with no clipped text.

## Exclusions

- No track row change. A later packet adds hover actions to rows.
- No payment split bar.
- No database change.

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
cargo build --release --bin v4vmm
```

## Result - 2026-10-07

Mechanical checks are Green: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo check --all-targets`, `cargo test` with 1855 unit tests and 303 guards, `cargo build --bin v4vmm` and `cargo build --release --bin v4vmm`.

| Case | Proof |
|---|---|
| R83-51 | `view_models::album_page::tests::adr_0083_album_page_actions_follow_the_state_table`, `adr_0083_album_page_busy_states_disable_the_filled_action_and_removal`, `adr_0083_album_page_without_feed_url_has_no_copy_item` |
| R83-52 | `adr_0083_album_page_removal_is_last_and_destructive`, and `view_models::library_removal::tests::always_confirmation_defers_each_album_removal` |
| R83-53 | `view_models::entity_detail::tests::adr_0083_album_name_links_name_the_artist_and_the_publisher` and `adr_0083_album_name_links_skip_missing_names` |
| R83-54 | `media::cover_color::tests::adr_0083_cover_color_of_a_known_image_is_stable` and `adr_0083_cover_color_of_an_empty_image_is_none` |
| R83-55 | `ui::shells::entity::tests::adr_0083_album_header_cover_is_xxl`, `ui::composites::thumbnail::tests::adr_0083_thumbnail_sizes_match_their_image_sizes` and the guard `adr_0083_cover_backdrop_uses_its_tokens` |
| R83-56 | The guard `adr_0083_no_view_model_label_carries_a_disclosure_arrow`. The other ADR 0083 guards stay Green |
| R83-57 | The guard `adr_0083_both_album_page_origins_build_one_shared_surface` |

Changes:

- `src/view_models/album_page.rs` is the one album action model. `src/media/cover_color.rs` computes the main color. `ImageCache::fetch_cover_color_blocking` and `FetchCoverColor` run it off the UI thread.
- `ui::tokens::CoverBackdrop` keeps the cover hue and sets the luminance for each appearance. The test `adr_0083_cover_backdrop_keeps_header_text_contrast` proves that `Label` and `SecondaryLabel` keep 4.5:1, and `TertiaryLabel` keeps 3:1, for 125 cover colors in Light and Dark.
- `ui::composites::CoverBackdrop` draws the gradient behind the header. The header padding does not change when the color arrives.
- "Remove album…" uses `RemovalConfirmation::Always`. The confirmation says "Remove Album from Library?" and has a text for an album with no track in a playlist.
- The artist link opens the Library artist page from a Library album. From an Index album, it opens the tracks that match the name, because the Index has no artist page.
- The track page publisher link also shows "Publisher" now, for ADR 0077 Decision 6.
- Deleted: the "Open publisher" action and `EntityActionKind::OpenPublisher`, `ReleaseActionState`, the release `primary_actions` projection that no renderer read, `PlaylistActionState::Open`, the old album builders of `LibraryAlbumDetailVm`, and the Index "Add feed to playlist" methods. The hero subtitle is deleted, because the artist link replaces it.

## Operator Visual Check

Use `./target/release/v4vmm`. Each step uses search, the sidebar or Page Down, with no mouse wheel.

1. Start the app. In the sidebar, open an artist that has a downloaded album, then open that album.
   - V83-52: the cover is large, about 200 pixels, with a soft shadow. A band in the main color of the cover is behind the header and fades into the page. The title and the names are easy to read.
   - V83-53: one filled button, "Add to playlist", and a "⋯" button. "⋯" shows "Copy feed URL", "MusicBrainz lookup", a divider and "Remove album…" in red, last.
   - V83-54: under the title, the artist name is a link, and "Publisher" is a link when the album has a publisher. No "Open publisher" button shows.
   - Wrong: two filled buttons, a "▾" in a label, text on the backdrop that is hard to read, or a backdrop color that has no relation to the cover.
2. Click "Remove album…", then click "Cancel". A confirmation must show before any removal. Wrong: the album goes out of the Library with no confirmation.
3. Click the artist link. The Library artist page opens. Use Back. Click "Publisher" if it shows. The publisher page opens. Use Back.
4. Search for the same album name. In the results, select the Index tab and the Feeds tab, then open the album.
   - V83-51: the header and the action row have the same layout. The filled button is "Download album". "⋯" shows "Copy feed URL" only.
   - Click the artist link. A page of the tracks that match the name opens.
5. Use Settings to change the appearance between Light and Dark, and look at steps 1 and 4 again. Make the window narrow and look again.
   - V83-56: each element stays in its place, with no clipped text.
6. V83-55: walk the album page rules §3, §15 and §30 of the overhaul plan on the new page.

Cleanup: none. Step 2 cancels the removal.

## Rollback

Revert the working tree. This packet adds no migration and no stored data. The thumbnail cache keeps its files.
