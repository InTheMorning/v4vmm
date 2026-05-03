# ADR 0038 Task 003: Library/Search VM Consolidation

## Status

In progress - first eighty-nine implementation slices completed on 2026-05-03.
May split into Task 003a (Library) and Task 003b (Discover) once the
full inventory is in hand.

## Goal

Hoist all fallback policy from `src/library.rs` and `src/search.rs` into
view-models. Screens read `display_*` accessors; screens never decide
what an empty value means.

## Inventory To Verify Before Starting

Grep evidence (verified 2026-05-02):

- `library.rs:164-165` — track display title with feed-title fallback.
- `library.rs:1604-1605` — `Unknown Artist` fallback.
- `library.rs:1609-1610` — `Unknown Album` fallback.
- `library.rs:2171` — `[untitled]` playlist row title.
- `library.rs:2384` — `feed_url.unwrap_or_default()`.
- `library.rs:2980` — `Tags` section title.
- `ui/shells/track.rs:103` (post-relocation) — track identity branch.
- Multiple `unwrap_or_default()` and `unwrap_or("")` sites in both
  `library.rs` and `search.rs` (~15 from the 2026-05-02 audit).

Re-grep when starting; the 2026-05-02 inventory is a starting list, not
authoritative.

Verified starting notes, 2026-05-03:

- The `Tags` section-title fallback is already owned by
  `TrackMetadataGridVm::tag_column_label`.
- Discover track-inspector feed-link fallback was still split between
  `TrackInspectorHeaderVm` and `src/search.rs`; this task's first slice
  moves the complete feed-link display contract into the VM.

## Files Likely To Change

- `src/view_models/track.rs` — `display_title`, `display_artist`,
  `display_album` accessors.
- `src/view_models/feed.rs` — `display_url -> Option<String>`.
- `src/view_models/library.rs`, `src/view_models/search.rs` — possible
  new accessors; consider splitting under
  `src/view_models/library/` and `src/view_models/discover/` once they
  approach 3,000 LOC.
- `src/view_models/track_metadata_grid.rs` — own the `Tags` fallback.
- (possibly new) `src/view_models/playlist.rs` for `display_name`.
- `src/library.rs`, `src/search.rs` — call-site sweep.
- `src/ui/shells/*.rs` — call-site sweep.
- `tests/architecture_tests.rs` — tighten existing fallback guards;
  add `view_models_own_display_fallbacks_for_library_and_search`.

## Migration Order

1. Discover track-inspector feed link
   - Introduce `TrackFeedLinkDisplay` in `view_models::search`.
   - Make `TrackInspectorHeaderVm::feed_link_display()` own guid
     presence, feed-title label fallback, and URL fallback.
   - Remove `src/search.rs` render-glue fallback from
     `feed_link_label.unwrap_or_else`.
   - Add `view_models_own_display_fallbacks_for_library_and_search`.
2. Re-grep the remaining `library.rs` / `search.rs` fallback inventory.
3. Payment-route address display
   - Add `PaymentRouteVm::address()` so value-route address presence
     and empty-string preservation live in the VM.
   - Remove `src/search.rs` render-glue coercion from
     `route.address.clone().unwrap_or_default()`.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
4. Payment-route custom field display
   - Add `PaymentRouteVm::custom_fields()` so `key ...` / `value ...`
     formatting and empty-string preservation live in the VM.
   - Remove `src/search.rs` render-glue checks of `custom_key` and
     `custom_value`.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
5. Payment-route summary display
   - Add `PaymentRouteVm::summary()` so recipient, route type, split,
     and fee/split label fallbacks enter the renderer as one display
     string.
   - Remove `src/search.rs` render-glue formatting of the primary
     payment-route line.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
6. Discover feed-list tile display
   - Extend `RecentFeedTileDisplay` with the tile id and episode-count
     note.
   - Make `RecentFeedTileVm::display()` own missing-guid and
     missing-episode-note handling for feed-list tiles.
   - Remove `src/search.rs` render-glue formatting of the feed tile id,
     title fallback, and episode note.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
7. Library tree track-number prefix
   - Add `LibraryTrackRowVm::tree_number_prefix()` so the Library
     tree-row zero-padded track-number prefix is VM-owned.
   - Remove `src/library.rs` render-glue formatting of
     `"{n:02} - "`.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
8. Metadata RSS cell value display
   - Add `TrackMetadataGridVm::rss_cell_value()` so missing RSS
     metadata values are normalized by the metadata-grid VM.
   - Remove `row.rss_value.as_deref().unwrap_or("")` from Library and
     Discover metadata cell renderers.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
9. Metadata ID3 cell value display
   - Add `TrackMetadataGridVm::id3_cell_value()` so pending edit value
     precedence and missing ID3 metadata value fallback are normalized
     by the metadata-grid VM.
   - Remove screen-local `pending.value -> row.id3_value -> ""`
     fallback from Library and Discover metadata cell renderers.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
10. Metadata MusicBrainz cell value display
    - Add `TrackMetadataGridVm::musicbrainz_cell_value()` so missing
      MusicBrainz metadata values are normalized by the metadata-grid
      VM.
    - Remove `row.musicbrainz_value.as_deref().unwrap_or("")` from
      Library and Discover metadata cell renderers.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
11. Metadata ID3 cell frame display
    - Add `TrackMetadataGridVm::id3_cell_frame()` so pending edit frame
      precedence and stored ID3 frame fallback are normalized by the
      metadata-grid VM.
    - Remove screen-local `pending.frame -> row.id3_frame` fallback
      from Library and Discover metadata cell renderers.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
12. Metadata drag frame display
    - Add `TrackMetadataGridVm::id3_drag_frame()` so missing source
      frame hints in Discover metadata drag payloads are normalized by
      the metadata-grid VM.
    - Remove `row.id3_frame.clone().unwrap_or_default()` from
      Discover metadata drag payload assembly.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
13. Metadata displayed frame label
    - Add `TrackMetadataGridVm::id3_frame_label()` so missing displayed
      ID3 frame labels are normalized by the metadata-grid VM.
    - Remove screen-local `frame_id.unwrap_or_default()` fallbacks from
      Discover metadata tag renderers.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
14. Metadata contributor summary display
    - Add `TrackMetadataGridVm::contributor_summary()` so contributor
      summarization and fallback-to-display-value policy are normalized
      by the metadata-grid VM.
    - Remove screen-local `summarize_contributor_value(...).unwrap_or_else(...)`
      fallbacks from Library and Discover metadata cell summary helpers.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
15. Metadata value-route summary display
    - Add `TrackMetadataGridVm::value_routes_summary()` so collapsed
      value-route count and malformed-value fallback policy are
      normalized by the metadata-grid VM.
    - Preserve the existing Library and Discover fallback difference as
      an explicit `ValueRoutesSummaryFallback` context policy.
    - Remove screen-local `"[N items]"` and Discover `"[N lines]"`
      summary formatting from metadata cell summary helpers.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
16. Discover track play-audio action display
    - Add `TrackVm::play_audio_display()` so play-audio URL, tooltip,
      and disabled state are projected together by the track VM.
    - Remove the screen-local `"No audio URL"` tooltip fallback from
      Discover's play-audio button.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
17. Discover dead Nostr action renderer cleanup
    - Remove the unused screen-local `render_nostr_icon_button()` helper
      from Discover now that track identity actions render through
      `ui::shells::track::render_track_identity_actions`.
    - Tighten `track_identity_links_use_shared_renderer` so a local
      Nostr button renderer cannot be reintroduced.
18. Metadata group heading display
    - Add `TrackMetadataGridVm::group_heading_label()` so the
      `"{label} ({unused_count} unused)"` heading suffix is normalized
      by the metadata-grid VM.
    - Remove duplicated screen-local unused-group heading formatting
      from Library and Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
19. Metadata value-route item label display
    - Add `TrackMetadataGridVm::value_route_item_label()` and
      `TrackMetadataGridVm::value_route_split_label()` so expanded
      Value Routes item labels and split suffixes are normalized by the
      metadata-grid VM.
    - Preserve the existing context difference: Library includes the
      split suffix in the collapsed sub-item label, while Discover keeps
      the split in the expanded child rows.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
20. Metadata value-route field label display
    - Add `TrackMetadataGridVm::value_route_field_key_label()` and
      `TrackMetadataGridVm::value_route_field_value_label()` so
      expanded Value Routes child-row key/value labels are normalized
      by the metadata-grid VM.
    - Remove the Library-local `route_value_label()` helper and the
      Discover-local JSON value formatter from Value Routes rendering.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
21. Discover track feed-link tooltip display
    - Add `TrackFeedLinkDisplay::tooltip` so the feed-link tooltip is
      carried by the track-inspector header VM.
    - Remove screen-local `guid` cloning for the feed-link tooltip from
      Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
22. Metadata comparison role/glyph display
    - Add `TrackMetadataGridVm::comparison_role()`,
      `TrackMetadataGridVm::comparison_glyph()`, and
      `TrackMetadataGridVm::display_with_glyph()` so comparison status
      roles and glyph-prefix formatting are normalized by the
      metadata-grid VM.
    - Remove duplicated screen-local comparison role/glyph helpers from
      Library and Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
23. Metadata pending-source role display
    - Add `TrackMetadataGridVm::pending_source_role()` so staged ID3
      copy previews decide match/different state in the metadata-grid
      VM.
    - Remove duplicated Library/Discover pending-source role helpers.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
24. Metadata standalone-ID3 status display
    - Add `TrackMetadataGridVm::id3_status_role()` and
      `TrackMetadataGridVm::id3_status_uses_primary_fallback()` so the
      standalone ID3 primary-color exception is a named VM policy.
    - Remove duplicated Library/Discover raw standalone-ID3 checks.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
25. Discover result-pane chrome display
    - Add `SearchRenderSnapshot::pane_display` so the Discover result
      pane title, search button label, fuzzy-toggle label, empty label,
      and load-more label are VM-owned.
    - Remove screen-local result-pane chrome labels from Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
26. Discover status error-prefix display
    - Add `SearchStatusSnapshot::display_text` so the error glyph
      prefix is projected by the search VM.
    - Remove screen-local `StatusRole::Danger.glyph()` prefix
      formatting from Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
27. Discover recent-feeds chrome display
    - Add `RecentFeedsSnapshot::display` and
      `SearchViewModel::recents_root_title()` so recent-feed panel
      title, empty label, and load-more label are VM-owned.
    - Remove screen-local recent-feed chrome labels from Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
28. Discover publisher-link tooltip display
    - Add `PublisherLinkDisplay` so publisher link id, title, target,
      and tooltip are VM-owned.
    - Remove screen-local publisher tooltip formatting and trimming
      from Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
29. Discover inspector chrome display
    - Add `SearchViewModel::inspector_chrome_display()` so the
      inspector back label and empty-state icon/label are VM-owned.
    - Remove screen-local inspector back/empty chrome from Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
30. Discover inspector status display
    - Add `SearchViewModel::inspector_loading_message()` and
      `SearchViewModel::inspector_error_message()` so inspector loading
      and error messages are VM-owned.
    - Remove screen-local inspector loading/error formatting from
      Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
31. Discover deferred-panel loading display
    - Add `SearchViewModel::deferred_panel_display()` so contributor
      and value-route panel loading labels are VM-owned.
    - Remove screen-local deferred-panel loading labels from Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
32. Library shell chrome display
    - Add `LibraryViewModel::chrome_display()` so Library search
      placeholders, search pane labels, empty-list label, and
      empty-detail label are VM-owned.
    - Remove screen-local Library shell chrome labels from `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
33. Library playlist sidebar chrome display
    - Extend `PlaylistSidebarVm` so the playlist heading, add button
      label, and new-playlist add label are projected with the existing
      disclosure and sort labels.
    - Remove screen-local playlist sidebar chrome labels from
      `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
34. Library feed-update toolbar display
    - Add `LibraryViewModel::feed_update_display()` so the feed-update
      action kind, label, disabled state, and status message are
      VM-owned.
    - Remove screen-local feed-update action label and disabled-state
      branching from `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
35. Library status and empty-state display
    - Add `LibraryViewModel::status_snapshot()` and
      `LibraryViewModel::should_show_empty_library()` so status
      severity and empty-list visibility are VM-owned.
    - Remove screen-local `Error:` prefix checks from Library render
      glue.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
36. Library artist tree row display
    - Add `ArtistNode::tree_display()` so the Library artist tree row
      id, disclosure glyph, and album-count label are VM-owned.
    - Remove screen-local artist row id, arrow, and album-count
      formatting from `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
37. Library album tree row display
    - Add `AlbumNode::tree_display()` so the Library album tree row id,
      disclosure glyph, and track-count label are VM-owned.
    - Remove screen-local album row id, arrow, and track-count
      formatting from `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
38. Library tree track row display
    - Add `LibraryTrackRowVm::tree_display()` so the Library tree
      track row id and prefixed compact title are VM-owned.
    - Remove screen-local tree track row id and
      `tree_number_prefix + compact_title` formatting from
      `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
39. Library artist feed-summary row display
    - Add `ArtistFeedSummaryVm::display()` so the artist detail feed
      row id and track-count label are VM-owned.
    - Remove screen-local artist feed-summary row id and
      `"{count} tracks"` formatting from `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
40. Library album MusicBrainz action display
    - Add `LibraryAlbumDetailVm::musicbrainz_action_vm()` so the
      action label and disabled state are VM-owned.
    - Remove screen-local `MusicBrainz` action label and active-lookup
      disabled-state check from `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
41. Library album playlist popover display
    - Add `LibraryAlbumDetailVm::playlist_display()` so the album
      playlist popover id and trigger label are VM-owned.
    - Remove screen-local album playlist popover id formatting from
      `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
42. Contributor identity action display
    - Add `ContributorRowVm::identity_actions()` so Library and
      Discover contributor website/Nostr action ids, kinds, and
      targets are VM-owned.
    - Remove screen-local contributor identity id formatting from
      `library.rs` and `search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
43. Discover deferred-panel heading display
    - Extend `SearchViewModel::deferred_panel_display()` so contributor
      and value-route panel section ids and heading labels are VM-owned.
    - Remove screen-local deferred-panel heading ids and labels from
      Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
44. Library contributor panel chrome display
    - Add `ReleaseDetailVm::contributor_panel_display()` so Library
      contributor panel id and heading title are VM-owned.
    - Remove screen-local Library contributor panel id/title literals
      from `library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
45. Metadata group disclosure display
    - Add `TrackMetadataGridVm::group_heading_display()` so shared
      metadata group labels and disclosure ids are VM-owned.
    - Remove screen-local metadata group disclosure id formatting from
      `library.rs` and `search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
46. Feed identity action display
    - Add `EntityActionVm::identity_display()` so feed identity action
      ids, display kinds, and payloads are VM-owned.
    - Remove feed identity action id formatting and slug mapping from
      `ui::shells::entity`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
47. Track identity action display
    - Reuse `EntityActionVm::identity_display()` so track identity
      action ids, display kinds, and payloads are VM-owned.
    - Remove track identity action id formatting and slug mapping from
      `ui::shells::track`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
48. Discover track row control display
    - Add `TrackVm::row_controls_display()` so Discover track row ids,
      play-button ids, playlist popover ids, and playlist trigger labels
      are VM-owned.
    - Remove row-control id formatting and local playlist trigger
      literals from `ui::shells::track`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
49. Discover track row download display
    - Add `TrackRowActionVm::download_display()` so download button and
      busy-spinner ids plus the busy tooltip are VM-owned.
    - Remove download-control id formatting from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
50. Discover inspector playlist popover display
    - Add `ActionRowVm::inspector_playlist_display()` so Discover
      inspector playlist popover ids enter the renderer through a VM
      display contract while preserving the existing action label.
    - Remove inspector playlist popover id formatting from
      `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
51. Library track playlist popover display
    - Add `LibraryTrackRowVm::playlist_display()` and
      `LibraryTrackActionVm::playlist_display()` so album-row and track
      inspector playlist popover ids and trigger labels are VM-owned.
    - Remove album-row and inspector playlist popover id/label
      formatting from `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
52. Discover feed tile id display
    - Extend `RecentFeedTileVm::display()` so feed-list, recent-feed,
      and podroll tile ids are VM-owned alongside the feed guid.
    - Remove those tile id formats from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
53. Discover track inspector play/feed-link id display
    - Extend `TrackVm::play_audio_display()` with the track-inspector
      play button id and glyph.
    - Extend `TrackFeedLinkDisplay` with the feed-link element id.
    - Remove those id/glyph literals from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
54. Library playlist track controls display
    - Add `PlaylistTrackRowVm::controls_display()` so playlist row ids,
      body ids, button ids, button glyphs, and availability are
      VM-owned.
    - Remove playlist-track control id/glyph formatting from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
55. Library album track row control display
    - Add `LibraryTrackRowVm::row_display()` so album track row ids and
      primary toggle button ids are VM-owned.
    - Remove album-track row and toggle id formatting from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
56. Library playlist detail action display
    - Add `PlaylistDetailVm::actions_display()` so playlist rename and
      delete button ids and labels are VM-owned.
    - Remove playlist detail action id/label literals from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
57. Library metadata panel loading display
    - Add `TrackMetadataActionState` loading-message accessors for ID3
      compare and MusicBrainz panels.
    - Remove metadata panel loading label literals from `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
58. Library staged ID3 edits display
    - Add `TrackMetadataActionState::staged_id3_edits_display()` so
      staged edit count text, apply label, conflict message, discard
      label, and availability are VM-owned.
    - Remove staged ID3 action/message formatting from `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
59. Deferred-panel error-prefix display
    - Add `LazyPanel::error()` for Discover deferred panels and
      `LibraryViewModel::deferred_panel_error_message()` for Library's
      still-local panel enum.
    - Remove screen-local deferred-panel `"Error: ..."` formatting from
      `src/search.rs` and `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
60. Library file-header metadata action display
    - Add `TrackMetadataActionState::file_actions_display()` so
      Re-read/Re-download file action labels are VM-owned.
    - Remove file-header metadata action label literals from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
61. Duplicate ID3 target message display
    - Add `TrackMetadataActionState::duplicate_id3_target_message()` so
      duplicate-target singular/plural formatting is VM-owned.
    - Remove duplicate ID3 target message formatting from
      `src/library.rs` and `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
62. ID3 apply error display
    - Add `TrackMetadataActionState::id3_apply_error_message()` so ID3
      apply error-prefix formatting is VM-owned.
    - Remove ID3 apply error formatting from `src/library.rs` and
      `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
63. Discover download success ID3 edit display
    - Extend `SearchSubscriptionCommand` with `success_message()` so
      Downloaded-track and applied-ID3-edit success text is VM-owned.
    - Remove Discover subscription success-message suffix formatting
      from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
64. Discover results empty-state icon display
    - Extend `SearchPaneDisplay` with `empty_icon`.
    - Remove raw Discover result empty-state icon glyphs from
      `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
65. Discover result row id display
    - Extend `ResultRowDisplay` with the list-row element id.
    - Remove result-row id formatting from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
66. Discover podroll section display
    - Add `PodrollSectionDisplay` so the podroll heading and scroll id
      are VM-owned.
    - Remove podroll heading and scroll-id literals from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
67. Library hover thumbnail id display
    - Add `LibraryViewModel::hover_thumb_display()` so hover thumbnail
      ids are VM-owned.
    - Remove hover thumbnail id formatting from `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
68. Library album thumbnail fallback display
    - Add `LibraryViewModel::album_thumb_display()` so the fallback
      thumbnail glyph is VM-owned.
    - Remove the raw album thumbnail fallback glyph from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
69. Discover results pane control id display
    - Extend `SearchPaneDisplay` with search, fuzzy-toggle, results
      scroll, and load-more ids.
    - Remove results-pane control and scroll id literals from
      `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
70. Discover inspector chrome id display
    - Extend `InspectorChromeDisplay` with back-button and scroll ids.
    - Remove inspector chrome id literals from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
71. Discover recent-feeds load-more id display
    - Extend `RecentFeedsDisplay` with the recent-feeds load-more id.
    - Remove the recent-feeds load-more id literal from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
72. Library sidebar, search, feed-update, and list control id display
    - Extend `PlaylistSidebarVm`, `LibraryChromeDisplay`, and
      `FeedUpdateActionDisplay` with static control ids used by the
      Library shell.
    - Remove those static Library control and scroll id literals from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
73. Library artist detail scroll id display
    - Extend `LibraryChromeDisplay` with the artist detail scroll id.
    - Remove the artist detail scroll id literal from `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
74. Library playlist detail scroll id display
    - Extend `LibraryChromeDisplay` with the playlist detail scroll id.
    - Remove the playlist detail scroll id literal from `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
75. Library track detail scroll id display
    - Extend `LibraryChromeDisplay` with the track detail scroll id.
    - Remove the track detail scroll id literal from `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
76. Library metadata expandable cell display
    - Add `TrackMetadataGridVm::library_expandable_cell_display()` so
      metadata cell keys, ids, header ids, and disclosure glyphs are
      VM-owned for Library metadata cells.
    - Remove Library metadata expandable cell id/glyph formatting from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
77. Discover metadata expandable cell display
    - Add `TrackMetadataGridVm::discover_expandable_cell_display()` so
      metadata cell keys, ids, header ids, and disclosure glyphs are
      VM-owned for Discover metadata cells.
    - Remove Discover RSS/ID3 expandable cell id/glyph formatting from
      `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
78. Library Value Routes item display
    - Add `TrackMetadataGridVm::library_value_route_item_display()` so
      nested Value Routes item keys, ids, header ids, and disclosure
      glyphs are VM-owned for Library metadata cells.
    - Remove Library nested Value Routes item id/glyph formatting from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
79. Discover Value Routes item display
    - Add `TrackMetadataGridVm::discover_value_route_item_display()` so
      nested Value Routes item keys, ids, and disclosure glyphs are
      VM-owned for Discover metadata cells.
    - Remove Discover nested Value Routes item id/glyph formatting from
      `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
80. Discover metadata compare-row slug display
    - Add `TrackMetadataGridVm::compare_row_id()` so test-row slug
      generation is metadata-grid-owned.
    - Remove `compare_row_id()` from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
81. Discover generated unused-ID3 row display
    - Add `TrackMetadataGridVm::unused_id3_frame_row_id()` and
      `id3_field_display_label()` so generated unused-frame row ids and
      labels are metadata-grid-owned.
    - Remove unused-ID3 row id/label formatting from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
82. Discover generated used-ID3 row display
    - Add `TrackMetadataGridVm::used_id3_field_row_id()` and reuse
      `id3_field_display_label()` so generated used-frame row ids and
      labels are metadata-grid-owned.
    - Remove used-ID3 row id/label formatting from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
83. Discover metadata source-drag id display
    - Add `TrackMetadataGridVm::source_drag_display()` so RSS and
      MusicBrainz source-drag cell ids are metadata-grid-owned.
    - Remove source-drag id formatting from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
84. Metadata expandable-cell summary display
    - Add `TrackMetadataGridVm::expandable_cell_summary()` so
      Contributors, Value Routes, Artwork, and transcript collapsed
      summary policy is metadata-grid-owned.
    - Preserve the existing Library/Discover Value Routes fallback
      difference as the named `ValueRoutesSummaryFallback` context.
    - Remove summary match/fallback duplication from `src/library.rs`
      and `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
85. Metadata artwork URL display policy
    - Add `TrackMetadataGridVm::artwork_url()` and
      `artwork_summary()` so HTTP/HTTPS artwork URL recognition and
      filename summary display are metadata-grid-owned.
    - Remove screen-local URL-prefix checks used for summary and
      expanded artwork-link activation.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
86. Discover transcript blank-line display
    - Add `TrackMetadataGridVm::transcript_line_display()` so blank
      transcript lines keep their visual row without a screen-local
      fallback.
    - Remove local blank-line replacement from Discover transcript and
      JSON line renderers.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
87. Library metadata logical-field aliases
    - Add `TrackMetadataGridVm::logical_field()` so raw MusicIndex TXXX
      aliases for Contributors and Value Routes are metadata-grid-owned.
    - Remove the screen-local `metadata_logical_field()` helper from
      `src/library.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
88. Value Routes child-field visibility
    - Add `ValueRouteFieldContext` and
      `TrackMetadataGridVm::value_route_child_field_is_visible()` so
      Library and Discover child-row visibility differences are named
      context policy.
    - Preserve Library hiding `split` because the collapsed row already
      carries split, while Discover keeps showing it in expanded rows.
    - Remove screen-local `recipient_name`/`split` field checks from
      Library and Discover.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
89. Discover JSON-tree scalar display
    - Add `TrackMetadataGridVm::json_tree_scalar_label()` so Discover
      expanded JSON leaves, including `null`, are metadata-grid-owned.
    - Remove screen-local JSON scalar match arms from `src/search.rs`.
    - Extend `view_models_own_display_fallbacks_for_library_and_search`.
90. Migrate remaining fallback batches, smallest blast radius first.

## Constraints

- Each VM accessor lands with three-case unit tests: present, empty
  string, `None`.
- Preserve the empty-vs-unknown distinction. `Option<String>` is
  preferred for fields where an empty state has a different visual
  treatment from a labeled fallback.
- One fallback at a time per commit.
- Existing `screens_do_not_inline_unknown_artist_or_album_fallbacks` and
  related guards must stay green throughout.

## Open Questions

1. **`view_models/library.rs` and `view_models/search.rs` size.** Both
   are ~2,800 LOC. Split now, or after this task? Recommendation:
   split as the consolidation lands, so new accessors land in the new
   submodule rather than enlarging the monolith.
2. **Feed-title fallback chain.** `track.title or track.feed_title or
   "Untitled"` is multi-source. The VM accessor must take both.
   Confirm the precedence rule (title-first is current behavior).
3. **Playlist row fallback.** `[untitled]` versus `Untitled` — pick one
   and document it. Current code has both.

## Definition of Done

- Every fallback string in the grep inventory has an owning VM
  accessor.
- Screens contain zero string-literal fallbacks for these concepts.
- New guard
  `view_models_own_display_fallbacks_for_library_and_search` is green.
- VM unit tests cover present / empty / `None` per accessor.

## First-Slice Implementation Notes

- `TrackFeedLinkDisplay` now carries the Discover track-inspector feed
  guid, visible label, and target URL as one VM-projected display
  contract.
- `TrackInspectorHeaderVm::feed_link_display()` returns `None` when
  there is no usable feed guid; otherwise it uses the existing
  `feed_title -> guid` label fallback and `feed_url -> feed_guid` URL
  fallback.
- `src/search.rs` no longer accepts an optional feed-link label and no
  longer reconstructs the guid fallback in render glue.
- The new architecture guard
  `view_models_own_display_fallbacks_for_library_and_search` blocks the
  separated feed-link fallback calls from returning to `src/search.rs`.

## Second-Slice Implementation Notes

- `PaymentRouteVm::address()` now carries optional payment-route address
  display while preserving the old distinction between `Some("")` and
  `None`.
- `src/search.rs` no longer checks `route.address.is_some()` or coerces
  `route.address.clone().unwrap_or_default()` inside the value-routes
  renderer.
- The architecture guard now also blocks screen-local payment-route
  address presence/coercion from returning to `src/search.rs`.

## Third-Slice Implementation Notes

- `PaymentRouteVm::custom_fields()` now carries optional `key ...` /
  `value ...` display while preserving the old distinction between
  present-empty values and absent values.
- `src/search.rs` no longer checks `route.custom_key` or
  `route.custom_value` directly inside the value-routes renderer.
- The architecture guard now also blocks screen-local payment-route
  custom-field presence/formatting from returning to `src/search.rs`.

## Fourth-Slice Implementation Notes

- `PaymentRouteVm::summary()` now carries the primary payment-route
  display line, including recipient fallback, route-type fallback, split
  fallback, and fee/split label.
- `src/search.rs` no longer assembles the primary value-route label from
  `recipient_name`, `route_type`, `split`, and `kind_label`.
- The architecture guard now also blocks screen-local payment-route
  summary formatting from returning to `src/search.rs`.

## Fifth-Slice Implementation Notes

- `RecentFeedTileDisplay` now carries the Discover feed-list tile id and
  optional episode note in addition to title, subtitle, and image URL.
- `RecentFeedTileVm::display()` owns the legacy
  `feed_guid.unwrap_or_default()` id fallback and the optional
  `"{count} tracks"` episode note.
- `src/search.rs` no longer derives feed-list tile ids, title fallback,
  or episode notes locally in `render_feed_list_section`.
- The architecture guard now also blocks screen-local feed-list tile id,
  title, and episode-note fallback from returning to `src/search.rs`.

## Sixth-Slice Implementation Notes

- `LibraryTrackRowVm::tree_number_prefix()` now carries the Library
  tree-row zero-padded track-number prefix.
- `src/library.rs` no longer formats `"{n:02} - "` while rendering the
  album-expanded track rows in the Library left tree.
- The architecture guard now also blocks the screen-local tree-row
  track-number prefix from returning to `src/library.rs`.

## Seventh-Slice Implementation Notes

- `TrackMetadataGridVm::rss_cell_value()` now carries the metadata-grid
  RSS cell missing-value fallback while preserving present-empty values.
- `src/library.rs` and `src/search.rs` no longer coerce
  `row.rss_value.as_deref().unwrap_or("")` in metadata cell renderers.
- The architecture guard now blocks that screen-local RSS metadata value
  fallback from returning to either screen.

## Eighth-Slice Implementation Notes

- `TrackMetadataGridVm::id3_cell_value()` now carries metadata-grid ID3
  cell value precedence: pending edit value, then stored ID3 value, then
  empty display for missing values.
- `src/library.rs` and `src/search.rs` no longer assemble that ID3
  value fallback chain in metadata cell renderers.
- The architecture guard now blocks the screen-local ID3 metadata value
  fallback from returning to either screen.

## Ninth-Slice Implementation Notes

- `TrackMetadataGridVm::musicbrainz_cell_value()` now carries the
  metadata-grid MusicBrainz cell missing-value fallback while preserving
  present-empty values.
- `src/library.rs` and `src/search.rs` no longer coerce
  `row.musicbrainz_value.as_deref().unwrap_or("")` in metadata cell
  renderers.
- The architecture guard now blocks that screen-local MusicBrainz
  metadata value fallback from returning to either screen.

## Tenth-Slice Implementation Notes

- `TrackMetadataGridVm::id3_cell_frame()` now carries metadata-grid ID3
  frame precedence: pending edit frame, then stored ID3 frame, then no
  frame display when both are missing.
- `src/library.rs` and `src/search.rs` no longer assemble that ID3 frame
  fallback chain in metadata cell renderers.
- The architecture guard now blocks the screen-local ID3 metadata frame
  fallback from returning to either screen.

## Eleventh-Slice Implementation Notes

- `TrackMetadataGridVm::id3_drag_frame()` now carries the Discover
  metadata drag payload frame fallback while preserving present-empty
  frame hints.
- `src/search.rs` no longer coerces missing `row.id3_frame` to an empty
  string while assembling metadata drag payloads.
- The architecture guard now blocks that screen-local drag frame
  fallback from returning to `src/search.rs`.

## Twelfth-Slice Implementation Notes

- `TrackMetadataGridVm::id3_frame_label()` now carries displayed ID3
  frame label fallback for Discover metadata tag cells.
- `src/search.rs` no longer coerces missing frame IDs to an empty string
  in `expandable_tag_cell` or `compare_tag_cell`.
- The architecture guard now blocks that screen-local displayed frame
  label fallback from returning to `src/search.rs`.

## Thirteenth-Slice Implementation Notes

- `TrackMetadataGridVm::contributor_summary()` now carries contributor
  summary display, including fallback to the already-rendered display
  value when the raw contributor value cannot be summarized.
- `src/library.rs` and `src/search.rs` no longer call
  `summarize_contributor_value(raw_value).unwrap_or_else(...)` inside
  metadata cell summary helpers.
- The architecture guard now blocks that screen-local contributor
  summary fallback from returning to either screen.

## Fourteenth-Slice Implementation Notes

- `TrackMetadataGridVm::value_routes_summary()` now carries collapsed
  Value Routes summary display for parsed route arrays and malformed
  raw values.
- `ValueRoutesSummaryFallback` makes the existing context difference
  explicit: Library falls back to the display value; Discover counts
  multi-line malformed displays before falling back to the display value.
- `src/library.rs` and `src/search.rs` no longer format collapsed
  Value Routes summaries locally.
- The architecture guard now blocks screen-local Value Routes summary
  count and multiline fallback formatting from returning.

## Fifteenth-Slice Implementation Notes

- `TrackVm::play_audio_display()` now carries the Discover track
  play-audio action URL, tooltip, and disabled state as one GPUI-free
  display contract.
- The play button renderer receives `TrackPlayAudioDisplay` instead of
  a raw optional URL, so the `"No audio URL"` fallback no longer lives
  in `src/search.rs`.
- The architecture guard now blocks that screen-local play-audio
  tooltip fallback from returning to Discover.

## Sixteenth-Slice Implementation Notes

- The unused `render_nostr_icon_button()` helper has been removed from
  `src/search.rs`.
- Track identity actions continue to render through the shared
  `ui::shells::track::render_track_identity_actions` path backed by
  `TrackDetailVm::identity_actions()`.
- The architecture guard now blocks reintroducing a screen-local Nostr
  button renderer in Discover.

## Seventeenth-Slice Implementation Notes

- `TrackMetadataGridVm::group_heading_label()` now carries metadata
  group heading display, including the unused-frame count suffix.
- `src/library.rs` and `src/search.rs` no longer duplicate
  `"{label} ({unused_count} unused)"` formatting in metadata group
  renderers.
- The architecture guard now blocks screen-local metadata group heading
  fallback formatting from returning.

## Eighteenth-Slice Implementation Notes

- `TrackMetadataGridVm::value_route_item_label()` now carries expanded
  Value Routes item-label display.
- `TrackMetadataGridVm::value_route_split_label()` now carries split
  suffix display, including numeric percent formatting and empty-value
  suppression.
- Library now asks the metadata-grid VM for the item label with the
  split suffix; Discover asks for the same item-label contract with no
  split suffix, preserving current rendered behavior.
- The architecture guard now blocks screen-local Value Routes item and
  split label formatting from returning.

## Nineteenth-Slice Implementation Notes

- `TrackMetadataGridVm::value_route_field_key_label()` now carries
  expanded Value Routes child-row key labels, including the `": "`
  suffix.
- `TrackMetadataGridVm::value_route_field_value_label()` now carries
  expanded Value Routes child-row value display, including null and
  empty-value suppression.
- `src/library.rs` no longer carries a local `route_value_label()`
  helper, and `src/search.rs` no longer inlines Value Routes field
  value conversion.
- The architecture guard now blocks screen-local Value Routes child-row
  key/value label formatting from returning.

## Twentieth-Slice Implementation Notes

- `TrackFeedLinkDisplay` now carries the Discover track feed-link
  tooltip.
- `render_feed_link_value()` now receives the full display contract
  instead of separate guid/title/url values, so the tooltip fallback
  remains GPUI-free in the header VM.
- The architecture guard now blocks screen-local guid cloning for the
  feed-link tooltip from returning.

## Twenty-First-Slice Implementation Notes

- `TrackMetadataGridVm::comparison_role()` now carries metadata
  comparison status roles, and `TrackMetadataGridVm::comparison_glyph()`
  carries their glyph display.
- `TrackMetadataGridVm::display_with_glyph()` now owns glyph-prefix
  formatting for metadata cell values.
- `src/library.rs` and `src/search.rs` no longer carry duplicated
  `comparison_status_role()`, `comparison_status_glyph()`, or
  `display_with_glyph()` helpers.
- The architecture guard now blocks screen-local metadata comparison
  role, glyph, and glyph-prefix formatting from returning.

## Twenty-Second-Slice Implementation Notes

- `TrackMetadataGridVm::pending_source_role()` now carries the staged
  ID3 copy preview role, including source-column matching, trimmed
  value comparison, and empty-value suppression.
- `src/library.rs` and `src/search.rs` no longer carry duplicated
  pending-source/source-cell role helpers.
- The architecture guard now blocks screen-local pending-source role
  display from returning.

## Twenty-Third-Slice Implementation Notes

- `TrackMetadataGridVm::id3_status_role()` now carries ID3 comparison
  status role projection for metadata-grid cells.
- `TrackMetadataGridVm::id3_status_uses_primary_fallback()` now names
  the existing standalone-ID3 primary-color exception, preserving the
  old visual distinction without raw renderer conditionals.
- The architecture guard now blocks duplicated standalone-ID3 status
  fallback checks from returning to Library or Discover.

## Twenty-Fourth-Slice Implementation Notes

- `SearchRenderSnapshot::pane_display` now carries Discover result-pane
  chrome labels: panel heading, search button label, fuzzy-toggle label,
  empty-results label, and load-more label.
- `src/search.rs` still owns GPUI controls and style state, but no
  longer carries those result-pane labels as renderer literals.
- The architecture guard now blocks result-pane chrome labels from
  returning to Discover render glue.

## Twenty-Fifth-Slice Implementation Notes

- `SearchStatusSnapshot::display_text` now carries the status text with
  the error glyph prefix when the status is an error.
- `src/search.rs` still maps the status severity to GPUI color, but no
  longer formats the error prefix with `StatusRole::Danger.glyph()`.
- The architecture guard now blocks screen-local Discover status
  error-prefix formatting from returning.

## Twenty-Sixth-Slice Implementation Notes

- `RecentFeedsSnapshot::display` now carries recent-feed panel chrome:
  heading, empty-state label, and load-more label.
- `SearchViewModel::recents_root_title()` gives the inspector shell the
  same VM-owned title when the recent-feeds root is shown.
- The architecture guard now blocks recent-feed panel chrome labels from
  returning to Discover render glue.

## Twenty-Seventh-Slice Implementation Notes

- `PublisherLinkDisplay` now carries publisher-link id, visible title,
  target, and tooltip.
- `render_publisher_link_value()` now receives one display projection
  instead of trimming and formatting the tooltip locally.
- The architecture guard now blocks screen-local publisher-link tooltip
  formatting from returning.

## Twenty-Eighth-Slice Implementation Notes

- `SearchViewModel::inspector_chrome_display()` now carries the
  Discover inspector back label and empty-state icon/label.
- `render_inspector()` and `render_inspector_empty()` now render those
  strings from the VM contract instead of screen-local literals.
- The architecture guard now blocks the screen-local inspector back and
  empty-state chrome from returning.

## Twenty-Ninth-Slice Implementation Notes

- `SearchViewModel::inspector_loading_message()` now carries the
  dynamic `"Loading {title}..."` inspector message.
- `SearchViewModel::inspector_error_message()` now carries the
  inspector error display.
- The architecture guard now blocks screen-local inspector loading and
  error formatting from returning.

## Thirtieth-Slice Implementation Notes

- `SearchViewModel::deferred_panel_display()` now carries loading
  labels for deferred contributor and value-route inspector panels.
- `render_lazy_contributors()` and `render_lazy_value_routes()` now ask
  the VM for those labels while keeping GPUI loading rendering local.
- The architecture guard now blocks screen-local deferred-panel loading
  labels from returning.

## Thirty-First-Slice Implementation Notes

- `LibraryViewModel::chrome_display()` now carries Library search
  placeholders, search pane labels, empty-list label, and empty-detail
  label.
- `LibraryApp::new()`, the Library render path, and empty detail
  rendering now consume that display contract.
- The architecture guard now blocks those Library shell chrome labels
  from returning to `src/library.rs`.

## Thirty-Second-Slice Implementation Notes

- `PlaylistSidebarVm` now carries the playlist heading, add button
  label, and new-playlist add label alongside its existing disclosure
  and sort labels.
- The Library sidebar renderer now receives those labels from the VM
  instead of carrying local literals.
- The architecture guard now blocks playlist sidebar chrome labels from
  returning to `src/library.rs`.

## Thirty-Third-Slice Implementation Notes

- `LibraryViewModel::feed_update_display()` now carries feed-update
  toolbar status, action kind, action label, and disabled state.
- The Library renderer still chooses the GPUI button style and command
  target, but no longer derives the action label or disabled state.
- The architecture guard now blocks screen-local feed-update toolbar
  labels from returning.

## Thirty-Fourth-Slice Implementation Notes

- `LibraryViewModel::status_snapshot()` now carries status text and
  `Error:` severity classification for the Library shell.
- `LibraryViewModel::should_show_empty_library()` now carries the
  empty-list visibility rule.
- The architecture guard now blocks screen-local Library status
  severity and empty-list visibility checks from returning.

## Thirty-Fifth-Slice Implementation Notes

- `ArtistNode::tree_display()` now carries Library artist tree row id,
  disclosure glyph, and album-count label display.
- `src/library.rs` no longer builds the artist row id, arrow glyph, or
  singular/plural album-count label in render glue.
- The architecture guard now blocks screen-local Library artist tree
  row chrome from returning.

## Thirty-Sixth-Slice Implementation Notes

- `AlbumNode::tree_display()` now carries Library album tree row id,
  disclosure glyph, and track-count label display.
- `src/library.rs` no longer builds the album row id, arrow glyph, or
  track-count label in render glue.
- The architecture guard now blocks screen-local Library album tree row
  chrome from returning.

## Thirty-Seventh-Slice Implementation Notes

- `LibraryTrackRowVm::tree_display()` now carries Library tree track
  row id and the prefixed compact title.
- `src/library.rs` no longer builds the tree track row id or joins the
  tree-number prefix and compact title in render glue.
- The architecture guard now blocks screen-local Library tree track row
  id/title formatting from returning.

## Thirty-Eighth-Slice Implementation Notes

- `ArtistFeedSummaryVm::display()` now carries Library artist-detail
  feed-summary row id and track-count label display.
- `src/library.rs` no longer builds `artist-feed-*` row ids or
  `"{count} tracks"` labels in render glue.
- The architecture guard now blocks screen-local artist feed-summary row
  id and count-label formatting from returning.

## Thirty-Ninth-Slice Implementation Notes

- `LibraryAlbumDetailVm::musicbrainz_action_vm()` now carries the
  Library album `MusicBrainz` action label and disabled state.
- `src/library.rs` still wires the click handler, but no longer owns the
  action label or active-lookup availability rule.
- The architecture guard now blocks screen-local album `MusicBrainz`
  action label and disabled-state checks from returning.

## Fortieth-Slice Implementation Notes

- `LibraryAlbumDetailVm::playlist_display()` now carries the Library
  album playlist popover id and trigger label.
- `src/library.rs` still wires playlist selection/create handlers, but
  no longer formats the album playlist popover id locally.
- The architecture guard now blocks screen-local album playlist popover
  id formatting from returning.

## Forty-First-Slice Implementation Notes

- `ContributorRowVm::identity_actions()` now carries Library and
  Discover contributor website/Nostr action ids, action kinds, and
  click/copy targets.
- `src/library.rs` and `src/search.rs` still wire GPUI click handlers,
  but no longer format contributor identity action ids locally.
- The architecture guard now blocks screen-local contributor identity
  action id formatting from returning.

## Forty-Second-Slice Implementation Notes

- `SearchViewModel::deferred_panel_display()` now carries contributor
  and value-route section ids and heading labels alongside the existing
  loading labels.
- Discover deferred panel headings now use the VM display contract while
  keeping disclosure toggle wiring in the screen.
- The architecture guard now blocks screen-local deferred-panel heading
  ids and labels from returning.

## Forty-Third-Slice Implementation Notes

- `ReleaseDetailVm::contributor_panel_display()` now carries Library
  contributor panel id and title display.
- `src/library.rs` still supplies thumbnails and action handlers, but no
  longer owns the contributor panel chrome literals.
- The architecture guard now blocks screen-local Library contributor
  panel id/title literals from returning.

## Forty-Fourth-Slice Implementation Notes

- `TrackMetadataGridVm::group_heading_display()` now carries metadata
  group labels and disclosure ids.
- `src/library.rs` and `src/search.rs` still wire disclosure toggles,
  but no longer format metadata group disclosure ids locally.
- The architecture guard now blocks screen-local metadata group
  disclosure id formatting from returning.

## Forty-Fifth-Slice Implementation Notes

- `EntityActionVm::identity_display()` now carries feed identity action
  ids, display kinds, and payloads.
- `ui::shells::entity::render_feed_identity_actions()` still maps VM
  display kinds to GPUI identity buttons and wires click/copy behavior,
  but no longer formats action ids or owns slug mapping.
- The architecture guard now blocks feed identity action id formatting
  and slug helpers from returning to the entity shell.

## Forty-Sixth-Slice Implementation Notes

- Track identity actions now use the same `EntityActionVm::identity_display()`
  display contract.
- `ui::shells::track::render_track_identity_actions()` still maps VM
  display kinds to GPUI identity buttons and wires click/copy behavior,
  but no longer formats action ids or owns slug mapping.
- The architecture guard now blocks track identity action id formatting
  and slug helpers from returning to the track shell.

## Forty-Seventh-Slice Implementation Notes

- `TrackVm::row_controls_display()` now carries Discover track row ids,
  play-button ids, playlist popover ids, and playlist trigger labels.
- `ui::shells::track::render_discover_track_row()` still wires click,
  playlist select/create, and play behavior, but no longer formats row
  control ids or owns the playlist trigger literal.
- The architecture guard now blocks those Discover row-control
  presentation facts from returning to the track shell.

## Forty-Eighth-Slice Implementation Notes

- `TrackRowActionVm::download_display()` now carries Discover download
  button ids, busy-spinner ids, and busy tooltips.
- `src/search.rs` still maps the action tone to button style and wires
  download/remove callbacks, but no longer formats download-control ids.
- The architecture guard now blocks screen-local Discover download
  control id formatting from returning.

## Forty-Ninth-Slice Implementation Notes

- `ActionRowVm::inspector_playlist_display()` now carries Discover
  inspector playlist popover ids while preserving the existing
  feed/track action-label contract.
- `src/search.rs` still determines feed playlist availability from the
  release action state and wires select/create handlers, but no longer
  formats inspector playlist popover ids locally.
- The architecture guard now blocks screen-local Discover inspector
  playlist popover id formatting from returning.

## Fiftieth-Slice Implementation Notes

- `LibraryTrackRowVm::playlist_display()` now carries Library album-row
  playlist popover ids and trigger labels.
- `LibraryTrackActionVm::playlist_display()` now carries Library track
  inspector playlist popover ids and trigger labels.
- `src/library.rs` still wires playlist select/create handlers, but no
  longer formats those playlist popover ids or local trigger labels.
- The architecture guard now blocks those Library track playlist
  presentation facts from returning.

## Fifty-First-Slice Implementation Notes

- `RecentFeedTileVm::display()` now carries Discover feed-list,
  recent-feed, and podroll tile ids alongside the existing feed guid,
  title, subtitle, image URL, and episode note.
- `src/search.rs` still decides whether podroll/recent tiles with empty
  guids are skipped and still wires navigation, but no longer formats
  those tile element ids locally.
- The architecture guard now blocks screen-local Discover feed tile id
  formatting from returning.

## Fifty-Second-Slice Implementation Notes

- `TrackVm::play_audio_display()` now carries the track-inspector play
  button id and glyph.
- `TrackFeedLinkDisplay` now carries the track-inspector feed-link
  element id.
- `src/search.rs` still wires the play and feed-link click handlers,
  but no longer owns those id/glyph presentation facts.
- The architecture guard now blocks screen-local track-inspector play
  and feed-link id/glyph formatting from returning.

## Fifty-Third-Slice Implementation Notes

- `PlaylistTrackRowVm::controls_display()` now carries Library playlist
  track row ids, row-body ids, play/move/remove button ids, button
  glyphs, and play/move availability.
- `src/library.rs` still wires playlist movement, remove, select, and
  playback callbacks, but no longer formats those control ids or glyphs.
- The architecture guard now blocks screen-local Library playlist track
  control id/glyph formatting from returning.

## Fifty-Fourth-Slice Implementation Notes

- `LibraryTrackRowVm::row_display()` now carries Library album-track row
  ids and primary toggle button ids.
- `src/library.rs` still wires subscribe/remove and selection callbacks,
  but no longer formats those row/control ids.
- The architecture guard now blocks screen-local Library album-track row
  and toggle id formatting from returning.

## Fifty-Fifth-Slice Implementation Notes

- `PlaylistDetailVm::actions_display()` now carries playlist rename and
  delete button ids and labels.
- `src/library.rs` still wires playlist delete and the existing rename
  placeholder callback, but no longer owns those action ids or labels.
- The architecture guard now blocks screen-local playlist rename/delete
  id and label literals from returning.

## Fifty-Sixth-Slice Implementation Notes

- `TrackMetadataActionState::compare_panel_loading_message()` and
  `TrackMetadataActionState::musicbrainz_panel_loading_message()` now
  carry Library metadata panel loading labels.
- `src/library.rs` still selects which panel state to render, but no
  longer owns those loading-message literals.
- The architecture guard now blocks those screen-local loading labels
  from returning.

## Fifty-Seventh-Slice Implementation Notes

- `TrackMetadataActionState::staged_id3_edits_display()` now carries
  staged edit count text, apply label, apply availability, conflict
  message, discard label, and discard visibility.
- `src/library.rs` still wires apply/discard callbacks and arranges the
  action rows, but no longer formats staged ID3 action/message text.
- The architecture guard now blocks those staged ID3 screen-local labels
  and messages from returning.

## Fifty-Eighth-Slice Implementation Notes

- Discover deferred-panel errors now use `LazyPanel::error()` for the
  `"Error: ..."` prefix.
- Library deferred-panel errors now use
  `LibraryViewModel::deferred_panel_error_message()` while the local
  Library panel enum remains in place for a later structural pass.
- The architecture guard now blocks screen-local deferred-panel
  `"Error: ..."` formatting from returning in Library and Discover.

## Fifty-Ninth-Slice Implementation Notes

- `TrackMetadataActionState::file_actions_display()` now carries the
  Library file-header Re-read and Re-download action labels.
- `src/library.rs` still wires the reload callbacks, but no longer owns
  those action label literals.
- The architecture guard now blocks screen-local file-header metadata
  action labels from returning.

## Sixtieth-Slice Implementation Notes

- `TrackMetadataActionState::duplicate_id3_target_message()` now carries
  duplicate-ID3-target singular/plural and joined-conflict formatting.
- `src/library.rs` and `src/search.rs` still decide when duplicate
  conflicts block an operation, but no longer format that message.
- The architecture guard now blocks duplicate-target message formatting
  from returning in both screens.

## Sixty-First-Slice Implementation Notes

- `TrackMetadataActionState::id3_apply_error_message()` now carries the
  ID3 apply error-prefix display.
- `src/library.rs` and `src/search.rs` still catch apply failures, but
  no longer format the operator-facing error prefix locally.
- The architecture guard now blocks screen-local ID3 apply error
  formatting from returning.

## Sixty-Second-Slice Implementation Notes

- `SearchSubscriptionCommand::success_message()` now carries the
  Discover downloaded-track success text and optional applied-ID3-edit
  suffix.
- `src/search.rs` still handles the subscription command result and
  clears metadata state, but no longer formats the success message.
- The architecture guard now blocks the screen-local downloaded-track
  ID3 edit suffix from returning.

## Sixty-Third-Slice Implementation Notes

- `SearchPaneDisplay::empty_icon` now carries the Discover result
  empty-state icon alongside the existing empty label.
- `src/search.rs` still renders the empty-state layout, but no longer
  owns the raw search icon glyph.
- The architecture guard now blocks screen-local Discover result
  empty-state icon literals from returning.

## Sixty-Fourth-Slice Implementation Notes

- `ResultRowDisplay::element_id` now carries Discover result list-row
  ids.
- `src/search.rs` still wires result selection and thumbnail rendering,
  but no longer formats result row ids.
- The architecture guard now blocks screen-local Discover result-row id
  formatting from returning.

## Sixty-Fifth-Slice Implementation Notes

- `PodrollSectionDisplay` now carries Discover podroll heading labels
  and scroll ids.
- `src/search.rs` still renders podroll tiles and click handlers, but
  no longer owns the podroll section label or scroll id.
- The architecture guard now blocks screen-local podroll section chrome
  from returning.

## Sixty-Sixth-Slice Implementation Notes

- `LibraryViewModel::hover_thumb_display()` now carries Library hover
  thumbnail element ids.
- `src/library.rs` still wires hover enter/leave behavior, but no
  longer formats hover thumbnail ids.
- The architecture guard now blocks screen-local hover thumbnail id
  formatting from returning.

## Sixty-Seventh-Slice Implementation Notes

- `LibraryViewModel::album_thumb_display()` now carries the Library
  album-thumbnail fallback icon.
- `src/library.rs` still renders the fallback thumbnail box, but no
  longer owns the raw fallback glyph.
- The architecture guard now blocks the screen-local thumbnail fallback
  glyph from returning.

## Sixty-Eighth-Slice Implementation Notes

- `SearchPaneDisplay` now carries Discover results-pane control ids for
  search, fuzzy toggle, results scrolling, and load more.
- `src/search.rs` still wires search actions, focus tracking, and load
  pagination, but no longer owns those static results-pane ids.
- The architecture guard now blocks those screen-local Discover results
  id literals from returning.

## Sixty-Ninth-Slice Implementation Notes

- `InspectorChromeDisplay` now carries Discover inspector back-button
  and scroll ids.
- `src/search.rs` still wires inspector navigation and body rendering,
  but no longer owns the inspector chrome ids.
- The architecture guard now blocks those screen-local inspector id
  literals from returning.

## Seventieth-Slice Implementation Notes

- `RecentFeedsDisplay` now carries the Discover recent-feeds load-more
  button id.
- `src/search.rs` still wires recent-feed pagination, but no longer
  owns that static id literal.
- The architecture guard now blocks the screen-local recent-feeds
  load-more id from returning.

## Seventy-First-Slice Implementation Notes

- `PlaylistSidebarVm`, `LibraryChromeDisplay`, and
  `FeedUpdateActionDisplay` now carry Library sidebar, search,
  feed-update, and list-scroll ids.
- `src/library.rs` still wires Library click handlers and feed-update
  commands, but no longer owns those static control ids.
- The architecture guard now blocks the screen-local Library control
  and scroll id literals from returning.

## Seventy-Second-Slice Implementation Notes

- `LibraryChromeDisplay` now carries the Library artist detail scroll
  id.
- `src/library.rs` still renders the artist detail body, but no longer
  owns that static detail-pane id literal.
- The architecture guard now blocks the screen-local artist detail
  scroll id from returning.

## Seventy-Third-Slice Implementation Notes

- `LibraryChromeDisplay` now carries the Library playlist detail scroll
  id.
- `src/library.rs` still renders playlist detail actions and rows, but
  no longer owns that static detail-pane id literal.
- The architecture guard now blocks the screen-local playlist detail
  scroll id from returning.

## Seventy-Fourth-Slice Implementation Notes

- `LibraryChromeDisplay` now carries the Library track detail scroll
  id.
- `src/library.rs` still renders track detail windows and metadata
  panels, but no longer owns that static detail-pane id literal.
- The architecture guard now blocks the screen-local track detail scroll
  id from returning.

## Seventy-Fifth-Slice Implementation Notes

- `TrackMetadataGridVm::library_expandable_cell_display()` now carries
  Library metadata expandable cell keys, ids, header ids, and disclosure
  glyphs.
- `src/library.rs` still wires metadata expansion clicks and content
  rendering, but no longer owns those top-level cell id/glyph rules.
- The architecture guard now blocks screen-local Library metadata
  expandable cell chrome from returning.

## Seventy-Sixth-Slice Implementation Notes

- `TrackMetadataGridVm::discover_expandable_cell_display()` now carries
  Discover metadata expandable cell keys, ids, header ids, and
  disclosure glyphs.
- `src/search.rs` still wires Discover metadata expansion clicks and
  content rendering, but no longer owns those top-level RSS/ID3 cell
  id/glyph rules.
- The architecture guard now blocks screen-local Discover metadata
  expandable cell chrome from returning.

## Seventy-Seventh-Slice Implementation Notes

- `TrackMetadataGridVm::library_value_route_item_display()` now carries
  Library nested Value Routes item keys, ids, header ids, and disclosure
  glyphs.
- `src/library.rs` still wires nested route expansion clicks and field
  rows, but no longer owns those nested item id/glyph rules.
- The architecture guard now blocks screen-local Library Value Routes
  item chrome from returning.

## Seventy-Eighth-Slice Implementation Notes

- `TrackMetadataGridVm::discover_value_route_item_display()` now carries
  Discover nested Value Routes item keys, ids, and disclosure glyphs.
- `src/search.rs` still wires nested route expansion clicks and field
  rows, but no longer owns those nested item id/glyph rules.
- The architecture guard now blocks screen-local Discover Value Routes
  item chrome from returning.

## Seventy-Ninth-Slice Implementation Notes

- `TrackMetadataGridVm::compare_row_id()` now carries metadata compare
  row slug generation.
- Discover tests still create fixture rows in `src/search.rs`, but no
  longer own the slug-generation rule.
- The architecture guard now blocks screen-local compare row slug
  helpers from returning.

## Eightieth-Slice Implementation Notes

- `TrackMetadataGridVm::unused_id3_frame_row_id()` and
  `id3_field_display_label()` now carry generated unused-ID3 row ids
  and labels.
- Discover tests still create unused-ID3 fixture rows in `src/search.rs`,
  but no longer own the row id or label format.
- The architecture guard now blocks screen-local unused-ID3 row display
  formatting from returning.

## Eighty-First-Slice Implementation Notes

- `TrackMetadataGridVm::used_id3_field_row_id()` and
  `id3_field_display_label()` now carry generated used-ID3 row ids and
  labels.
- Discover tests still create used-ID3 fixture rows in `src/search.rs`,
  but no longer own the row id or label format.
- The architecture guard now blocks screen-local used-ID3 row display
  formatting from returning.

## Eighty-Second-Slice Implementation Notes

- `TrackMetadataGridVm::source_drag_display()` now carries Discover RSS
  and MusicBrainz source-drag cell ids.
- `src/search.rs` still wires drag payloads and callbacks, but no
  longer owns the draggable-source id formats.
- The architecture guard now blocks screen-local metadata source-drag
  id formatting from returning.

## Eighty-Third-Slice Implementation Notes

- `TrackMetadataGridVm::expandable_cell_summary()` now carries
  collapsed metadata summary policy for Contributors, Value Routes,
  Artwork, transcript fields, and default display values.
- Library and Discover retain their named Value Routes fallback context
  through `ValueRoutesSummaryFallback`, but no longer duplicate the
  summary match logic.
- The architecture guard now blocks screen-local artwork URL summary
  checks from returning.

## Eighty-Fourth-Slice Implementation Notes

- `TrackMetadataGridVm::artwork_url()` and `artwork_summary()` now
  carry artwork URL recognition and filename summary display.
- `src/search.rs` still wires the middle-click open behavior for
  expanded artwork rows, but asks the VM whether the row is a URL.
- The architecture guard now blocks screen-local artwork URL-prefix
  checks from returning.

## Eighty-Fifth-Slice Implementation Notes

- `TrackMetadataGridVm::transcript_line_display()` now carries the
  blank-line visual fallback used by Discover transcript expansion.
- `src/search.rs` still renders transcript and JSON lines, but no
  longer owns the empty-line replacement string.
- The architecture guard now blocks the screen-local blank-line
  fallback from returning.

## Eighty-Sixth-Slice Implementation Notes

- `TrackMetadataGridVm::logical_field()` now carries raw MusicIndex
  TXXX logical-field aliases for Contributors and Value Routes.
- `src/library.rs` still passes field context into metadata rendering,
  but no longer owns the alias table.
- The architecture guard now blocks the screen-local logical-field
  helper and alias literals from returning.

## Eighty-Seventh-Slice Implementation Notes

- `ValueRouteFieldContext` and
  `TrackMetadataGridVm::value_route_child_field_is_visible()` now carry
  Library/Discover Value Routes child-field visibility policy.
- Library keeps hiding `split` because its collapsed route label already
  includes that suffix; Discover still shows `split` as an expanded
  child row.
- The architecture guard now blocks screen-local child-field skip checks
  from returning.

## Eighty-Eighth-Slice Implementation Notes

- `TrackMetadataGridVm::json_tree_scalar_label()` now carries Discover
  expanded JSON-tree scalar labels, including the explicit `null`
  display.
- `src/search.rs` still renders JSON tree shape and indentation, but no
  longer owns scalar label conversion.
- The architecture guard now blocks screen-local JSON scalar match arms
  from returning.

## Test Commands

```sh
cargo fmt -- --check
cargo check
cargo test track_inspector_header_vm_projects_feed_link_display_contract
cargo test payment_route_vm_projects_address_without_coercing_presence
cargo test payment_route_vm_projects_custom_fields_without_coercing_presence
cargo test payment_route_vm_projects_primary_summary
cargo test recent_feed_tile_vm_projects_id_and_episode_note
cargo test tree_number_prefix_preserves_legacy_zero_padded_display
cargo test rss_cell_value_preserves_empty_vs_missing_display
cargo test id3_cell_value_prefers_pending_then_preserves_empty_vs_missing_display
cargo test id3_cell_frame_prefers_pending_then_preserves_empty_vs_missing_display
cargo test id3_drag_frame_preserves_empty_vs_missing_display
cargo test id3_frame_label_preserves_empty_vs_missing_display
cargo test id3_generated_row_display_projects_ids_and_labels
cargo test source_drag_display_projects_discover_source_cell_ids
cargo test contributor_summary_falls_back_to_display_value_when_unsummarized
cargo test value_routes_summary_counts_routes_and_owns_fallback_policy
cargo test expandable_cell_summary_owns_context_fallbacks
cargo test artwork_url_and_summary_preserve_legacy_http_policy
cargo test transcript_line_display_preserves_blank_visual_rows
cargo test logical_field_maps_raw_musicindex_txxx_fields
cargo test value_route_child_field_visibility_preserves_screen_contexts
cargo test json_tree_scalar_label_preserves_raw_json_leaf_display
cargo test group_heading_label_appends_unused_count_only_when_present
cargo test value_route_item_label_appends_split_when_present
cargo test value_route_split_label_formats_percent_and_ignores_empty_values
cargo test value_route_field_key_label_adds_separator
cargo test value_route_field_value_label_trims_and_suppresses_empty_values
cargo test expandable_cell_display_projects_library_and_discover_chrome
cargo test value_route_item_display_projects_screen_specific_chrome
cargo test track_inspector_header_vm_projects_feed_link_display_contract
cargo test play_audio_display_uses_url_tooltip_else_missing_audio_fallback
cargo test musicbrainz_cell_value_preserves_empty_vs_missing_display
cargo test comparison_role_maps_compare_statuses
cargo test display_with_glyph_preserves_empty_values
cargo test pending_source_role_compares_trimmed_values
cargo test id3_status_role_suppresses_standalone_id3_values
cargo test group_heading_display_projects_label_and_disclosure_id
cargo test search_status_snapshot_prefixes_error_display
cargo test search_render_snapshot_projects_result_pane_display_labels
cargo test recent_feeds_snapshot_projects_panel_display_labels
cargo test publisher_link_display_trims_title_and_tooltip
cargo test inspector_chrome_display_projects_back_and_empty_state
cargo test inspector_status_messages_are_vm_owned
cargo test deferred_panel_display_projects_heading_and_loading_labels
cargo test library_chrome_display_projects_shell_labels
cargo test library_status_snapshot_classifies_error_prefix
cargo test feed_update_display_projects_toolbar_action_labels
cargo test library_tree_artist_display_projects_row_chrome
cargo test library_tree_album_display_projects_row_chrome
cargo test library_tree_track_display_projects_id_and_prefixed_title
cargo test artist_feed_summary_display_projects_row_id_and_track_count
cargo test album_detail_vm_musicbrainz_action_projects_label_and_disabled_state
cargo test album_detail_vm_playlist_display_projects_popover_id_and_label
cargo test contributor_identity_actions_project_ids_kinds_and_targets
cargo test contributor_identity_actions_omit_absent_targets
cargo test contributor_panel_display_projects_surface_chrome
cargo test identity_action_display_projects_id_kind_and_payload
cargo test track_detail_identity_action_display_projects_ids
cargo test row_controls_display_projects_track_row_ids_and_playlist_label
cargo test track_row_action_vm_download_display_projects_ids_and_tooltip
cargo test action_row_vm_inspector_playlist_display_projects_id_and_label
cargo test library_track_row_vm_playlist_display_projects_album_track_controls
cargo test library_track_action_vm_formats_playlist_label_and_message_status
cargo test recent_feed_tile_vm_projects_id_and_episode_note
cargo test play_audio_display_uses_url_tooltip_else_missing_audio_fallback
cargo test track_inspector_header_vm_projects_feed_link_display_contract
cargo test playlist_track_row_vm_controls_display_projects_ids_labels_and_availability
cargo test library_track_row_vm_projects_row_and_toggle_ids
cargo test playlist_detail_vm_projects_rename_and_delete_controls
cargo test track_metadata_action_state_projects_loading_and_staged_id3_display
cargo test lazy_panel_error_owns_error_prefix_display
cargo test library_view_model_deferred_panel_error_message_owns_error_prefix
cargo test track_metadata_action_state_projects_file_actions_and_id3_errors
cargo test search_subscription_command_formats_begin_and_error_messages
cargo test search_render_snapshot_projects_result_pane_display_labels
cargo test result_row_key_display_and_inspector_title_are_pure
cargo test podroll_section_display_projects_heading_and_scroll_id
cargo test library_view_model_projects_thumbnail_display_contracts
cargo test search_render_snapshot_projects_result_pane_display_labels
cargo test inspector_chrome_display_projects_back_and_empty_state
cargo test recent_feeds_snapshot_projects_panel_display_labels
cargo test library_view_model_playlist_sidebar_projects_rows_and_header_state
cargo test library_chrome_display_projects_shell_labels
cargo test feed_update_display_projects_toolbar_action_labels
cargo test library_chrome_display_projects_shell_labels
cargo test view_models_own_display_fallbacks_for_library_and_search
cargo test track_identity_links_use_shared_renderer
cargo test
cargo clippy -- -D warnings
git diff --check
```

## Expected Final Report

- Name the fallback migrated.
- Name the VM/display contract that owns it.
- Name the guard added or tightened.
- Report automated gate status.
- Explicitly say whether visual evidence was needed and, if not, why.
