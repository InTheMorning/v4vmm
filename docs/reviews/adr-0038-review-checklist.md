# ADR 0038 Review Checklist

## Reviewed Artifacts

- `docs/adr/0038-presentation-contract-enforcement.md`
- `docs/plans/adr-0038-presentation-contract-enforcement-phase-plan.md`
- `docs/tasks/adr-0038-task-001-layer-relocation.md`
- `docs/tasks/adr-0038-task-002-composite-display-contract-audit.md`
- `docs/tasks/adr-0038-task-003-library-search-vm-consolidation.md` (stub)
- `docs/tasks/adr-0038-task-004-dark-mode-parity-audit.md` (stub)
- `docs/tasks/adr-0038-task-005-accessibility-label-contract.md` (stub)
- `docs/tasks/adr-0038-task-006-page-vm-generalization.md` (stub)
- `docs/tasks/adr-0038-task-007-screen-decomposition.md` (stub)
- `docs/tasks/adr-0038-task-008-final-sweep-and-readiness-gate.md` (stub)

## Gate Status

Status: Task 003 (Library/Search VM Consolidation) is in progress.
Task 002 migrated `TrackRow`, `DetailHeader`, `DetailGrid`,
`ReleaseDetailSurface::track_section`, `AddToPlaylistPopover`, and
`TrackMetadataGrid` cells off public loose string builders. The
`ActionRow`, `DisclosureGroup`, `SegmentedControl`, `TagBadge`, and
`action_button`
generic-control sub-slices have also moved status messages, disclosure
labels, segment labels, badge labels, and metadata-action labels to
display contracts, tightening the composite display-contract guard.
Task 003 has started by moving the Discover track-inspector feed-link
label and URL fallback into `TrackFeedLinkDisplay`, then moving
payment-route address, custom-field, and summary display into
`PaymentRouteVm`, and feed-list tile id/title/count display into
`RecentFeedTileVm`, and Library tree track-number prefix display into
`LibraryTrackRowVm`, and metadata RSS cell missing-value display into
`TrackMetadataGridVm`, and metadata ID3 cell value precedence into
`TrackMetadataGridVm`, and metadata MusicBrainz cell missing-value
display into `TrackMetadataGridVm`, and metadata ID3 cell frame
precedence into `TrackMetadataGridVm`, and Discover metadata drag frame
display into `TrackMetadataGridVm`, and Discover displayed ID3 frame
label fallback into `TrackMetadataGridVm`.
Contributor metadata summary fallback has also moved into
`TrackMetadataGridVm`.
Collapsed Value Routes summary display has also moved into
`TrackMetadataGridVm`.
Discover track play-audio action display has also moved into
`TrackVm`.
The stale Discover track Nostr action renderer has also been removed in
favor of the shared track identity-action shell.
Metadata group heading display and expanded Value Routes item labels
have also moved into `TrackMetadataGridVm`.
Expanded Value Routes field labels have moved into
`TrackMetadataGridVm`, and Discover track feed-link tooltip display has
moved into `TrackFeedLinkDisplay`.
Metadata comparison role/glyph display, staged pending-source role
display, and standalone-ID3 status fallback display have also moved
into `TrackMetadataGridVm`.
Discover result-pane chrome labels, status error-prefix display,
recent-feeds chrome labels, and publisher-link tooltip display have
also moved into `SearchRenderSnapshot`, `SearchStatusSnapshot`,
`RecentFeedsSnapshot`, and `PublisherLinkDisplay`.
Discover inspector chrome labels, inspector loading/error messages, and
deferred-panel loading labels have also moved into `SearchViewModel`
display contracts.
Library shell chrome labels, playlist sidebar chrome labels,
feed-update toolbar display, and status/empty-state display have also
moved into `LibraryViewModel` and `PlaylistSidebarVm` display
contracts.
Library artist tree-row chrome, album tree-row chrome, and tree-track
row id/title display have also moved into `ArtistNode`,
`AlbumNode`, and `LibraryTrackRowVm` display contracts.
Library artist feed-summary row display, album MusicBrainz action
display, and album playlist popover display have also moved into
`ArtistFeedSummaryVm` and `LibraryAlbumDetailVm` display contracts.
Discover track row controls, Discover track download controls,
Discover inspector playlist popover ids, and Library track playlist
popover display have also moved into `TrackVm`, `TrackRowActionVm`,
`ActionRowVm`, `LibraryTrackRowVm`, and `LibraryTrackActionVm`
display contracts.
Discover feed-list/recent/podroll tile ids, track-inspector play and
feed-link ids, and Library playlist track controls have also moved into
`RecentFeedTileVm`, `TrackVm`, `TrackFeedLinkDisplay`, and
`PlaylistTrackRowVm` display contracts.
Library album-track row/toggle ids, playlist rename/delete actions,
metadata panel loading labels, staged ID3 edit controls, and
deferred-panel error prefixes have also moved into
`LibraryTrackRowVm`, `PlaylistDetailVm`, `TrackMetadataActionState`,
`LazyPanel`, and `LibraryViewModel` display contracts.
Library file-header metadata action labels, duplicate ID3 target
messages, ID3 apply error messages, Discover download success messages,
and Discover results empty-state icons have also moved into
`TrackMetadataActionState`, `SearchSubscriptionCommand`, and
`SearchPaneDisplay` display contracts.
Discover deferred-panel empty-line display, expanded Artwork URL text,
and legacy feed-header title/subtitle display have also moved into
`SearchViewModel` and `TrackMetadataGridVm` display contracts.
Discover type-filter labels/query values and feed-list section heading
display have also moved into `SearchViewModel` display contracts.
Discover result type badge labels, inspector title display, playlist
trigger fallback labels, payment-route group headings, and the Library
album-row MusicBrainz status text binding have also moved into
`ResultRowDisplay`, `SearchViewModel`, `ActionRowVm`,
`PaymentRouteVm`, and `LibraryTrackRowVm` display contracts.
Library artist/album tree titles, artist feed-summary title/media
display, playlist detail header title display, and Library/Discover
metadata disclosure-id binding have also moved into `ArtistNode`,
`AlbumNode`, `ArtistFeedSummaryVm`, `PlaylistDetailVm`, and
`TrackMetadataGridVm` display contracts.
Library, Discover, and shared track-shell playlist popover option
display has also moved into shared `playlist_option_displays()`.
Discover result row ids, podroll section chrome, Library hover
thumbnail ids, and Library album-thumbnail fallback icons have also
moved into `ResultRowDisplay`, `PodrollSectionDisplay`, and
`LibraryViewModel` display contracts.
Discover results-pane control ids, inspector chrome ids, recent-feeds
load-more ids, and Library sidebar/search/feed-update/list ids have
also moved into `SearchPaneDisplay`, `InspectorChromeDisplay`,
`RecentFeedsDisplay`, `PlaylistSidebarVm`, `LibraryChromeDisplay`, and
`FeedUpdateActionDisplay` display contracts.
Library artist, playlist, and track detail scroll ids have also moved
into `LibraryChromeDisplay`.
Library and Discover metadata expandable cell ids, cell keys, header
ids, disclosure glyphs, and nested Value Routes item chrome have also
moved into `TrackMetadataGridVm`.
Discover generated metadata compare-row slugs, unused-ID3 row ids, and
used-ID3 row ids/labels have also moved into `TrackMetadataGridVm`.
Discover metadata source-drag ids, collapsed expandable-cell summaries,
artwork URL/filename summary policy, and Discover transcript blank-line
display have also moved into `TrackMetadataGridVm`.
Library raw metadata logical-field aliases, Library/Discover Value
Routes child-field visibility, and Discover JSON-tree scalar labels
have also moved into `TrackMetadataGridVm`.
Action-row status message severity and width policy, Library track
subscription messages, Library staged-ID3/action-error messages, and
Discover inspector subscription messages have also moved into
`ActionStatusMessageDisplay`, `LibraryTrackActionVm`,
`TrackMetadataActionState`, and `ActionRowVm`.
Metadata expanded field-kind classification and expandability gates have
also moved into `TrackMetadataGridVm`.
Library playlist row text, media lookup, and control projection have
also moved into `PlaylistTrackRowVm::display()`.
Metadata field labels, metadata text value display, expanded
raw/display fallback selection, drag preview display, ID3 frame display
labels, and ID3 frame color-role classification have also moved into
`TrackMetadataGridVm`.
Discover deferred-panel empty labels, Discover feed-inspector track-list
fallback, and Library local subscription progress/error messages have
also moved into `DeferredPanelDisplay`, `SearchViewModel`, and
`LibraryTrackActionVm`.
Library track-subscribe busy/success labels, Discover track-download
success labels, and Library feed-update error formatting have also
moved into `LibraryTrackActionVm`, `SearchSubscriptionCommand`, and
`LibraryViewModel`.
Library and Discover split-pane chrome ids have also moved into
`LibraryChromeDisplay` and `SearchPaneDisplay`.
Library and Discover feed identity-action prefixes have also moved into
`ReleaseDetailPageVm`.
Library and Discover track/contributor identity-action prefixes have
also moved into `TrackDetailVm` and `ContributorRowVm`.
Release detail scroll ids, contributor row chrome, and default release
track row ids have also moved into `ReleaseDetailPageVm`,
`ContributorPersonVm`, and `SharedTrackRowVm`.

## Required Questions For Every UI Change

- Which architectural layer does this change live in (1–8)?
- What repeated presentation concept is changing?
- Who owns it: primitive, composite, shell, or VM?
- What display contract carries title, subtitle, metadata, state,
  availability, fallback labels, and command intent?
- For interactive composites: what accessibility label and hint?
- For visible changes: light + dark visual smoke?
- Which screen-local pattern is being deleted or prevented?
- Which architecture test, unit test, visual smoke, or baseline
  reduction blocks the regression from returning?
- If Library and Discover differ, is the difference a named additive
  context policy rather than a forked page skeleton?

## Contract Matrix

| Contract | Pass Condition | Common Failure |
|---|---|---|
| Shared owner | Repeated chrome lives in `src/ui/primitives` or `src/ui/composites` | Copying a row/header/action helper into another screen |
| Display contract | GPUI-free VM or co-located display struct owns labels and state | Screen calls `unwrap_or("Untitled")` or formats availability locally |
| Token/icon discipline | Named tokens and icon catalog are used | Raw `px`, `rgb`, glyph strings, inline SVGs outside allowed layers |
| Additive context policy | Context-specific actions attach through slots | Library/Discover rebuild different page skeletons for the same entity |
| Regression guard | Guard lands with the consolidation | Visual patch lands without test or visual-smoke evidence |
| Page VM | Entity detail pages render through shell helper + `<Entity>DetailPageVm` | Screen assembles a page from individual VM accessors |
| Layer architecture | Imports respect layer order; shells live under `src/ui/shells/` | Top-level `src/ui_*.rs` shell file added; layer skipped |
| Theme adaptivity | Colors resolve through `theme_bridge`; light + dark verified | Raw `rgb(0x…)` outside the token layer |
| Accessibility contract | Interactive composites carry VM-sourced a11y labels | Icon-only button without label; screen-side a11y string |

## Task Results

| # | Task | Status | Required Evidence |
|---|---|---|---|
| 1 | Layer Relocation                              | Implemented with visual-proof caveat | Files moved under `src/ui/shells/`; `KNOWN_SHARED_UI_SHELL_FILES` removed; `top_level_shells_live_under_src_ui_shells` green. Visual proof remains open; no provisional screenshot artifacts are retained. |
| 2 | Composite Display-Contract Audit              | Implemented | `TrackRow` row number/title/duration now enter through `TrackRowVm` or `SharedTrackRowVm`; `DetailHeader` title/subtitle/data rows now enter through `DetailHeaderDisplay`; `DetailGrid` key/value rows now enter through `DetailElementRow` or `DetailTextRow`; release track sections now enter through `ReleaseTrackSectionDisplay`; playlist popovers now enter through `AddToPlaylistDisplay` and `PlaylistOptionDisplay`; metadata grid cells now enter through `TrackMetadataGroupDisplay`, `TrackMetadataFieldDisplay`, `TrackMetadataFrameDisplay`, `TrackMetadataTagDisplay`, and `TrackMetadataTextDisplay`; action-row status messages now enter through `ActionRowMessageDisplay`; disclosure headings now enter through `DisclosureGroupDisplay`; segmented options now enter through `SegmentDisplay`; badge labels now enter through `TagBadgeDisplay`; metadata-action labels now enter through `ActionButtonDisplay`; guard renamed/tightened to scan multi-line signatures; allowlist shrank by the old `TrackRow`, `DetailHeader`, `DetailGrid`, release track-section, playlist popover, metadata-grid cell, action-row message, disclosure-group, segmented-control, tag-badge, and action-button string builders |
| 3 | Library/Search VM Consolidation               | In progress - first one hundred forty implementation slices completed | `TrackMetadataGridVm` now owns metadata field labels, metadata text value display, expanded raw/display fallback selection, expanded Artwork URL text, drag preview label/value display, ID3 frame display labels, ID3 frame color-role classification, metadata cell fallback/precedence, expansion, grouping, source-drag ids, Value Routes summaries, JSON scalar labels, comparison roles/glyphs, pending-source roles, and disclosure ids used by Library and Discover. `SearchViewModel` also owns deferred-panel empty-line text, deferred-panel empty labels, feed-header title/subtitle projection, feed-inspector track-list fallback, type-filter labels/query values, feed-list section heading display, and inspector title display. `ResultRowDisplay`, `ActionRowVm`, `PaymentRouteVm`, `LibraryTrackRowVm`, `LibraryTrackActionVm`, `SearchSubscriptionCommand`, `ArtistNode`, `AlbumNode`, `ArtistFeedSummaryVm`, `PlaylistDetailVm`, `LibraryViewModel`, `LibraryChromeDisplay`, `SearchPaneDisplay`, `ReleaseDetailPageVm`, `TrackDetailVm`, `ContributorRowVm`, `ContributorPersonVm`, `SharedTrackRowVm`, and shared `playlist_option_displays()` own the newly migrated result badge, playlist trigger, payment-route group heading, MusicBrainz status text, local subscription/download messages, Discover download success labels, feed-update errors, split-pane chrome ids, identity-action prefixes, release detail scroll ids, contributor row chrome, default release track row ids, tree-title, feed-summary, playlist-header, and playlist-option display contracts. Library/Search no longer perform the migrated fallback/classification work screen-locally, and `view_models_own_display_fallbacks_for_library_and_search` has been tightened to block those patterns from returning. |
| 4 | HIG Dark-Mode Parity Audit                    | Stub           | `style.rs` resolution; light + dark screenshots per surface |
| 5 | HIG Accessibility-Label Contract              | Stub           | A11y labels per interactive composite; new guard; coverage table |
| 6 | PageVm Generalization                         | Stub           | `<Entity>DetailPageVm` per surface; shell helpers; new guard |
| 7 | Screen Decomposition                          | Stub           | `library.rs`/`search.rs` ≤ 500 LOC; per-surface files under `src/ui/shells/{library,discover}/`; new guards |
| 8 | Final Sweep + Readiness Gate                  | Stub           | All baselines zero; full visual smoke; gate decision recorded |

## Visual Smoke Ledger

To be filled per task. All entries require both themes. File path
convention: `docs/reviews/screenshots/adr-0038-{surface}-{theme}.png`.

| Surface | Light | Dark | Fixture | Status |
|---|---|---|---|---|
| Library list                | TBD | TBD | TBD | Pending Task 004 |
| Library inspector           | TBD | TBD | TBD | Pending Task 004 |
| Library track detail        | TBD | TBD | TBD | Pending Task 004 |
| Library feed detail         | TBD | TBD | TBD | Pending Task 004 |
| Discover list               | TBD | TBD | TBD | Pending Task 004 |
| Discover inspector          | TBD | TBD | TBD | Pending Task 004 |
| Discover track detail       | TBD | TBD | TBD | Pending Task 004 |
| Discover feed detail        | TBD | TBD | TBD | Pending Task 004 |
| Playlist popover            | TBD | TBD | TBD | Pending Task 004 |
| Now-playing bar             | TBD | TBD | TBD | Pending Task 004 |
| Recent feed tiles           | TBD | TBD | TBD | Pending Task 004 |
| Search results              | TBD | TBD | TBD | Pending Task 004 |

Task 001 relocation smoke (light theme only, per task packet):

| Surface | Screenshot | Status |
|---|---|---|
| Library shell | None retained | Needs deterministic/manual recapture before counting as acceptance evidence |
| Discover shell | None retained | Needs deterministic/manual recapture before counting as acceptance evidence |

Visual-proof caveat, 2026-05-03: coordinate-driven X11 captures were
discarded and are not retained in the repository. Automated gates are
green, but visual acceptance for the relocation remains open until a
reviewer captures or verifies Library and Discover through a
deterministic/manual process.

## Accessibility Coverage Ledger

To be filled by Task 005. Composites that render interactive chrome:

| Composite | A11y label source | A11y hint? | Guard entry | Status |
|---|---|---|---|---|
| `action_button`              | TBD | TBD | TBD | Pending |
| `ActionRow`                  | TBD | TBD | TBD | Pending |
| `identity_action_button`     | TBD | TBD | TBD | Pending |
| `AddToPlaylistPopover`       | TBD | TBD | TBD | Pending |
| `TrackRow`                   | TBD | TBD | TBD | Pending |
| `ListRow`                    | TBD | TBD | TBD | Pending |
| `RecentFeedTile`             | TBD | TBD | TBD | Pending |
| `DisclosureGroup`            | TBD | TBD | TBD | Pending |
| `SegmentedControl`           | TBD | TBD | TBD | Pending |
| `NowPlayingBar`              | TBD | TBD | TBD | Pending |
| Release detail action overlays | TBD | TBD | TBD | Pending |
| Track detail action overlays  | TBD | TBD | TBD | Pending |

Pure-text/pure-layout composites are exempt:
`detail_grid`, `detail_header`, `multiline_text`, `divider`, `loading`.

## Automated Checks

For Task 001:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test top_level_shells_live_under_src_ui_shells`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

For each subsequent task: re-run the full set, plus the task's
targeted guard test.

For Task 002 current slices:

- `cargo test composite_signatures_take_display_contracts_not_loose_strings`

For Task 003 current slices:

- `cargo test track_inspector_header_vm_projects_feed_link_display_contract`
- `cargo test payment_route_vm_projects_address_without_coercing_presence`
- `cargo test payment_route_vm_projects_custom_fields_without_coercing_presence`
- `cargo test payment_route_vm_projects_primary_summary`
- `cargo test recent_feed_tile_vm_projects_id_and_episode_note`
- `cargo test tree_number_prefix_preserves_legacy_zero_padded_display`
- `cargo test rss_cell_value_preserves_empty_vs_missing_display`
- `cargo test id3_cell_value_prefers_pending_then_preserves_empty_vs_missing_display`
- `cargo test id3_cell_frame_prefers_pending_then_preserves_empty_vs_missing_display`
- `cargo test id3_drag_frame_preserves_empty_vs_missing_display`
- `cargo test id3_frame_label_preserves_empty_vs_missing_display`
- `cargo test contributor_summary_falls_back_to_display_value_when_unsummarized`
- `cargo test value_routes_summary_counts_routes_and_owns_fallback_policy`
- `cargo test group_heading_label_appends_unused_count_only_when_present`
- `cargo test value_route_item_label_appends_split_when_present`
- `cargo test value_route_split_label_formats_percent_and_ignores_empty_values`
- `cargo test value_route_field_key_label_adds_separator`
- `cargo test value_route_field_value_label_trims_and_suppresses_empty_values`
- `cargo test play_audio_display_uses_url_tooltip_else_missing_audio_fallback`
- `cargo test musicbrainz_cell_value_preserves_empty_vs_missing_display`
- `cargo test comparison_role_maps_compare_statuses`
- `cargo test display_with_glyph_preserves_empty_values`
- `cargo test pending_source_role_compares_trimmed_values`
- `cargo test id3_status_role_suppresses_standalone_id3_values`
- `cargo test search_status_snapshot_prefixes_error_display`
- `cargo test search_render_snapshot_projects_result_pane_display_labels`
- `cargo test recent_feeds_snapshot_projects_panel_display_labels`
- `cargo test publisher_link_display_trims_title_and_tooltip`
- `cargo test inspector_chrome_display_projects_back_and_empty_state`
- `cargo test inspector_status_messages_are_vm_owned`
- `cargo test deferred_panel_display_projects_heading_and_loading_labels`
- `cargo test library_chrome_display_projects_shell_labels`
- `cargo test library_status_snapshot_classifies_error_prefix`
- `cargo test feed_update_display_projects_toolbar_action_labels`
- `cargo test library_tree_artist_display_projects_row_chrome`
- `cargo test library_tree_album_display_projects_row_chrome`
- `cargo test library_tree_track_display_projects_id_and_prefixed_title`
- `cargo test artist_feed_summary_display_projects_row_id_and_track_count`
- `cargo test album_detail_vm_musicbrainz_action_projects_label_and_disabled_state`
- `cargo test album_detail_vm_playlist_display_projects_popover_id_and_label`
- `cargo test contributor_identity_actions_project_ids_kinds_and_targets`
- `cargo test contributor_identity_actions_omit_absent_targets`
- `cargo test contributor_panel_display_projects_surface_chrome`
- `cargo test group_heading_display_projects_label_and_disclosure_id`
- `cargo test identity_action_display_projects_id_kind_and_payload`
- `cargo test track_detail_identity_action_display_projects_ids`
- `cargo test view_models_own_display_fallbacks_for_library_and_search`
- `cargo test track_identity_links_use_shared_renderer`

## Readiness Gate (filled by Task 008)

| Question | Answer | Evidence |
|---|---|---|
| All architecture guards green, baselines zero?           | TBD | TBD |
| Every entity detail page on PageVm + shell helper?       | TBD | TBD |
| Every interactive composite carries a11y label?           | TBD | TBD |
| Light + dark visual smoke covers every main surface?     | TBD | TBD |
| `library.rs` and `search.rs` are thin entries?           | TBD | TBD |
| Zero remaining screen-local fallback strings?            | TBD | TBD |
| Deferred-architecture-work index reconciled?             | TBD | TBD |

Decision: TBD (`Proceed` or `Defer`).
