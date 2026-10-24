# v4vmm Source Map

## Source Minimap

When you need to look something up, consult this map first to find the right file.

### v4vmm source (`src/`)

| What you need | File |
|---------------|------|
| SQLite schema, row types, all queries | `src/db.rs` |
| Startup SQLite read/write/schema verification and preparation | `src/db/startup.rs`; shared schema/read contract in `src/db.rs` |
| Non-mutating database inspection, bounded verified SQLite backup, private candidate and cleanup | `src/db/maintenance.rs` (ADR 0066 task 010) |
| Exclusive database connection, retained source descriptors, private preservation copies and manifests | `src/db/maintenance/preservation.rs`; drained-session handoff in `src/application/commands/maintenance.rs` and `src/app/startup.rs` (ADR 0066 task 011) |
| Reviewed restore candidates, fingerprints, preservation-first SQLite installation and rollback verification | `src/db/maintenance/restore.rs`; explicit session/config-bound command in `src/application/commands/maintenance.rs`; existing drain/resumption in `src/app/startup.rs` (ADR 0066 task 012) |
| Narrow interrupted migration-11 schema recognition, normal migration boundaries and explicit repair/older-backup candidates | `src/db/upgrades.rs`, `src/db.rs`, `src/db/maintenance/restore.rs`; shared command/VM/database tools (ADR 0066 task 013) |
| Repair/diagnostics page menus, separate instructions/report views and responsive action placement | `src/view_models/maintenance.rs`, `src/view_models/settings.rs`, `src/view_models/startup/database.rs`, `src/ui/composites/maintenance_page.rs`; shared menu-trigger focus in `src/ui/primitives/button.rs` (ADR 0074) |
| Shared Settings/recovery database forms, typed reports and independent worker adapter | `src/view_models/startup/database.rs`, `src/presentation/database_tools.rs`, `src/ui/composites/maintenance_forms.rs` |
| Config struct, TOML paths | `src/config.rs` |
| Explicit configuration correction, drafts, backup and revision checks | `src/config/correction.rs`, `src/application/commands/maintenance.rs` (ADR 0066) |
| Shared recovery/Settings editor, retained-draft Close/Reopen, input Escape and focus return, typed state and geometry | `src/view_models/startup/correction.rs`, `src/presentation/configuration_editor.rs`, `src/ui/composites/maintenance_forms.rs` |
| Core startup admission and music storage probes | `src/startup.rs`, `src/startup/storage.rs` |
| Recovery state/actions/reports; shared geometry; screen wiring | `src/view_models/startup.rs`, `src/ui/composites/startup_report.rs`, `src/app/startup.rs` |
| Shared word/line selection and Linux primary paste | `src/view_models/text_selection.rs`, `src/ui/composites/selectable_text.rs`, `src/ui/primitives/primary_selection.rs`, commit-pinned `gpui-base` 0.6.1 (ADRs 0071/0072); migration and acceptance evidence in `docs/tasks/archive/adr-0071-task-001-shared-text-selection.md` |
| Shared log frame, source identity, follow/pause and reading anchors | `src/ui/composites/log_frame.rs`, `src/view_models/log_view.rs`, `src/view_models/show.rs` (ADR 0063); app roots retain `LogFrames` |
| Show log height priority and card overflow scrolling with logs closed or open, with independent sidebar | `src/view_models/show.rs`, `src/ui/composites/show_log_pane.rs`, `src/ui/composites/split_pane.rs`, `src/ui/shells/show.rs` (ADRs 0070/0073) |
| Recovery/Settings page clearance for nested scrollbars | `src/ui/composites/page_scroll_content.rs`, `src/ui/layouts.rs` (ADR 0063) |
| Scoped background issues, persistent Diagnostics tool list and command remedies | `src/application/capability.rs`, `src/view_models/startup/capabilities.rs` |
| Music search results in the list column: the tabs Releases, Artists and Tracks, one row for each item, the Library state of each row (ADR 0084 Decision 2, packet 010) | `src/view_models/search_results/{mod,rows,results,paged_tab,tabs,empty_state}.rs`, `src/ui/shells/search_results_inspector.rs`, `src/ui/shells/search_result_rows.rs`, `src/app/search_dispatch.rs`, `LibrarySearchLookup` in `src/view_models/library.rs` |
| Music saved searches: the table, the Discover rows, the page in the detail column and the delete confirmation (ADR 0084 Decision 3, packet 012) | `src/db/saved_searches.rs`, `src/library/saved_searches.rs`, `src/view_models/search_results/saved_search.rs`, `src/app/saved_search_dispatch.rs`, `src/ui/shells/saved_search_delete_confirmation.rs`, `src/ui/shells/library/views_list.rs` |
| Search failure explanation/disclosure; shared report URL redaction | `src/view_models/search_results/failure.rs`, `src/diagnostics.rs` |
| Settings groups and field/action contracts; shared form/navigation and retained group scroll handles; screen wiring | `src/view_models/settings.rs`, `src/ui/composites/settings.rs`, `src/app/settings.rs` (ADR 0069) |
| Music release page for the Library and the Index: facts, page and row view models, commands, one shell, the two hosts (ADR 0084 Decision 4, packet 021) | `src/view_models/release_page.rs`, `src/ui/shells/release_page.rs`, `src/library/release_page.rs`, `src/ui/shells/library/feed_detail.rs`, `render_index_release_page` in `src/app/search_dispatch.rs` |
| Music selection: the model, the bar, the list column and the detail column (ADR 0084 Decision 10, packets 009, 010, 018 and 019) | `src/view_models/list_selection.rs`, `src/view_models/selection_action_bar.rs`, `src/view_models/detail_selection.rs`, `src/library/detail_selection.rs`, `src/app/detail_selection.rs`, `src/app/search_selection.rs` (packet 010), `src/ui/composites/selection_action_bar.rs`, `src/ui/composites/selectable_tile.rs` (packet 020) |
| Music history scroll position, the Index track target and the toolbar text after Back (ADR 0084 Decision 9, packet 030) | `src/view_models/workspace/nav.rs` (`FrameNavigationState::detail_scroll_target`, `IndexTrackTarget`, `PendingToolbarSearchText`), `DetailScrollBinding` and `LibraryApp::bind_detail_selection` in `src/library/detail_selection.rs` |
| Playlist delete confirmation and the shared menu open state (packet 030) | `src/ui/shells/playlist_delete_confirmation.rs`, `ContextMenuHandle` in `src/ui/primitives/context_menu.rs` |
| Release row menu and the release bar counts of the Releases list, the feed page and the name page (ADR 0084 packet 031) | `src/view_models/release_row_actions.rs`, `src/ui/composites/release_row_menu.rs`, `LibraryApp::run_release_row_command` in `src/library/app_impl.rs` |
| Download state icon of a track row and a release row (ADR 0083 Decision 6, amended on 2026-10-22, ADR 0084 packet 033) | `src/view_models/release_download_state.rs`, `src/ui/composites/release_download_state.rs`, `DownloadSlotState` in `src/view_models/release_page.rs`, `download_status_icon` in `src/ui/composites/track_row.rs`, `IconName::CloudCheck` in `src/ui/icons.rs` |
| Feed links of a release after a download, and the pass at start (ADR 0084 packet 034) | `hydrate_album_identity`, `start_album_identity_backfill` and `apply_library_hydration_result` in `src/library/app_impl.rs`, `feed_ids_with_no_stored_relationship` in `src/db/publisher_relationships.rs` |
| Music download queue and its progress item: the scheduler, the jobs, the completions and the busy state (ADR 0084 Decision 10, packet 022) | `src/library/download_queue.rs`, `src/library/downloads.rs`, `src/view_models/download_progress.rs`, `src/ui/composites/download_status_item.rs`, `finish_index_download` in `src/app/search_dispatch.rs` |
| Music collection art: the logo frame, the mosaic and its composition (ADR 0084 Decision 12, packet 011) | `src/media/logo_frame.rs`, `src/view_models/collection_art.rs`, `src/presentation/collection_art.rs`, `src/ui/composites/collection_art.rs`, `FetchLogoMosaic` in `src/application/queries/images.rs`. The stored image of an artist feed: migration 20 in `src/db/publisher_feed_image.rs` and `src/db/publisher_relationships.rs` (packet 024) |
| Music track row: badges, the download slot, the duration and the row menu on hover, focus and right-click (ADR 0083 Decisions 5 and 6, task 006) | `src/ui/composites/track_row.rs`, `src/ui/composites/track_row_menu.rs`, `TrackRowColumn` in `src/ui/tokens.rs`, the row view models in `src/view_models/release_page.rs`. The playlist, Tracks and Needs attention rows (task 007): `ListRow::menu_cell` in `src/ui/composites/list_row.rs`, `src/ui/shells/playlist.rs`, `src/ui/shells/library/content_list_row_actions.rs`. The drag of a track to a playlist (task 008): `src/ui/composites/track_drag.rs`, the timer in `src/presentation/delayed_presenter.rs`, `SpringLoadVm` in `src/view_models/library.rs` |
| Music track page for the Library and the Index: feed facts, page view model, commands, one shell, the two hosts (ADR 0084 Decision 4, packet 023) | `src/view_models/track_page.rs`, `src/ui/shells/track_page.rs`, `src/library/track_page.rs`, `src/ui/shells/library/track_detail.rs`, `run_index_track_page_command` in `src/app/search_dispatch.rs` |
| Music scopes "Library" and "Everything" (ADR 0084 Decision 8, packet 016): the scope and the name list pages, the Artists merge and classification, the page loads, the status rows | `src/view_models/index_name_list.rs`, `src/view_models/artists_everything.rs`, `src/view_models/releases_everything.rs`, `src/library/music_scopes.rs`, `src/application/queries/name_lists.rs`, `src/ui/shells/library/index_list_status.rs`, the Discover views list in `src/ui/shells/library/views_list.rs`, `src/view_models/index_feed_singles.rs`, the group page in `src/view_models/singles_group.rs` and `src/ui/shells/library/singles_group.rs` (one track row style and the matched singles of packet 032, `ReleaseTrackRowVm::index_single` in `src/view_models/release_page.rs`, `tree_name_feed_matches` in `src/view_models/library.rs`), and the feed requests of the group page and the single page in `src/library/music_scopes.rs` (packet 026) |
| Music content list message while the list loads, when it is empty, and after an error (ADR 0062) | `src/ui/shells/library/content_list_state.rs` |
| Music New releases window groups: the chain, the group tile and the layout, which the view model makes one time for each change (ADR 0086 Decisions 1 to 3, packet 002) | `src/view_models/window_groups.rs`, `LibraryViewModel::refresh_new_releases_layout` in `src/view_models/library.rs`, `discover_items_with_selection` in `src/view_models/detail_selection.rs`, `src/ui/shells/library/window_groups.rs` (the group tile and the group row), `render_discover_new_releases_detail` in `src/ui/shells/library/content_list.rs` |
| Music window group page: the page of one New releases window, with its key, rows, main action and group link (ADR 0086 Decision 4, packet 004) | `WindowGroupKey` and `window_members` in `src/view_models/window_groups.rs`, `src/view_models/window_group_page.rs`, `LibraryViewModel::window_group_page` and `LibraryViewModel::discover_page_rows` in `src/view_models/library.rs`, `discover_grouped_release_rows` in `src/view_models/detail_selection.rs`, the tile handlers in `src/library/detail_selection.rs`, `src/ui/shells/library/window_group.rs` |
| Music saved search window groups: the group rows of the Releases tab of a saved search page, and the window group page that a group row opens (ADR 0086 Decision 6, packet 005) | `src/view_models/search_results/saved_search_groups.rs`, `SearchResultsInspectorPageVm::visible_items` in `src/view_models/search_results/mod.rs`, `WindowGroupSource` in `src/view_models/window_groups.rs`, `LibrarySearchLookup` and `LibraryViewModel::window_group_page` in `src/view_models/library.rs`, `render_window_group_result_row` in `src/ui/shells/search_result_rows.rs`, `open_search_item` in `src/app/search_selection.rs` |
| Music search query parser: the terms, the field table, the scope and the cursor term (ADR 0087 Decision 1, packet 001) | `src/view_models/search_query.rs` |
| Library search query builder: one clause for each field, bound values, ADR 0087 Decision 2, packet 002 | `src/db/library_search.rs` |
| Library search facts that SQL does not hold: the release kind, the release date, the track date and the ready set (ADR 0087 Decision 2, packet 002) | `src/view_models/library_search_facts.rs` |
| Music search run: the typed query of one run, the Index plan, the request and the cause of a skipped Index part (ADR 0087 Decisions 1 and 5, packet 002) | `src/view_models/search_results/search_run.rs` |
| Music search field list: the rows, the keys and the overlay under the toolbar search field (ADR 0087 Decision 3, packet 004), and the values of `artist:`, `release:` and `playlist:` (ADR 0087 Decision 3, packet 005) | `src/view_models/search_field_list.rs`, `src/view_models/search_field_values.rs`, `src/ui/composites/search_field_list.rs`, `src/app/search_field_list.rs` |
| Music search live runs: the pause, the run generation and the cancel (ADR 0087 Decision 4, packet 003) | `src/view_models/live_run.rs`, `src/app/search_live.rs`, `Motion::SearchPause` in `src/ui/tokens.rs` |
| Music search Index results: the step of each Index result, the loading row count and the note of a result that the page did not show (ADR 0087 Decision 4, packet 007) | `index_result_step` in `src/app/search_live.rs`, `src/view_models/search_results/index_result_note.rs`, `index_pending_row_count` in `src/view_models/search_results/mod.rs` |
| Music name level page and the "Matched by name" section (ADR 0084 Decision 7, packet 017) | `src/view_models/name_level_page.rs`, `src/app/name_level_dispatch.rs`, `src/ui/shells/name_level_page.rs`, `src/ui/composites/release_section_list.rs`, `NameFeedMatches` in `src/view_models/artist_identity.rs` |
| Settings cached-file query and saved observation | `src/application/queries/library.rs`, `src/view_models/cached_files.rs`, `src/app.rs` |
| Retained original actions, explicit checked Retry and scoped tool setup | `src/application/capability_recovery.rs`, `src/application/capability_recovery/setup.rs` (ADR 0066) |
| Runtime/cache/optional-tool repair and focused Settings routing in the existing shell | `src/app/capabilities.rs`, existing search/playback/Show adapters |
| Independent maintenance worker and GPUI completion | `src/presentation/maintenance_executor.rs`, `src/presentation/startup_presenter.rs` |
| Session admission, tracked work and maintenance authority | `src/application/session_lifecycle.rs`, `src/application/async_command_runner.rs` (ADR 0066) |
| Session teardown and fresh-session wiring | `src/presentation/session_transition.rs`, `src/app/session.rs`, `src/app/startup.rs` |
| Session actions and recorded reports; shared maintenance forms | `src/view_models/startup/session.rs`, `src/ui/composites/maintenance_forms.rs` |
| MusicIndex HTTP client | `src/api.rs` |
| RSS fetch + Podcasting 2.0 parse | `src/rss/` |
| Playlist RSS check: actor, comparison, apply, holds, report (ADR 0076) | `src/runtime/playlist_rss_check.rs`, `src/rss/compare.rs`, `src/rss/check_apply.rs`, `src/db/rss_check_runs.rs`, `src/db/rss_field_holds.rs`, `src/view_models/playlist_rss_check.rs` |
| Publisher relationship storage (ADR 0077) | `src/db/publisher_relationships.rs` |
| Stored value projection: hold, then MusicIndex fact, then column (ADR 0076) | `src/application/queries/stored_values.rs` |
| Stored payment route and route converter (ADR 0076) | `src/db/payment_routes.rs`, `src/rss/value_routes.rs`, `metadata_service::with_stored_route_frame` |
| Tag update scan, confirmation and write (ADR 0076) | `src/application/queries/tag_update.rs`, `src/application/commands/tag_update.rs`, `src/runtime/tag_update.rs`, `src/view_models/tag_update.rs`, `src/ui/shells/tag_update_confirmation.rs` |
| ID3v2.4 read/write, `AudioTags`, `Id3v24Edit`, `apply_id3v24_edits` | `src/audio_tags.rs` |
| File-byte format detection, `AudioFormat`, existing FLAC/ffmpeg fallback | `src/audio_format.rs` |
| Fresh bounded converter probes and shared setup reports | `src/audio_format/probe.rs`, `src/view_models/startup/converter.rs`, shared correction command/editor (ADR 0066 task 008) |
| RSS vs ID3 vs MusicBrainz diff, `TrackContext` compare | `src/track_compare.rs` |
| `TrackContext`, `TagCompareResult`, `ImageBytes` — no GPUI | `src/metadata.rs` |
| Pure metadata derivation, `id3_edits_for_track_context` | `src/metadata_service.rs` |
| Download + tag + library-membership pipeline, shared conversion replacement | `src/subscribe_service.rs`, `src/subscribe_service/materialization.rs` (ADR 0066) |
| Session-owned conversion retry, original requests and explicit redownload | `src/application/conversion_recovery.rs`, existing download port/commands and capability recovery controls (ADR 0066) |
| Retained artifact identity, contained staging and cleanup | `src/track_compare/retained.rs`, `src/track_compare.rs` (ADR 0066) |
| `ensure_feed_in_db`, feed subscription glue | `src/feed_service.rs` |
| Library membership, `subscribe_then_append_to_playlist` | `src/library_service.rs` |
| Playlist CRUD and track append | `src/playlist_service.rs` |
| Now-playing session state | `src/playback.rs` |
| mpv driver, IPC socket lifecycle | `src/playback_driver/` |
| Long-running playback owner (desktop/daemon) | `src/playback_owner.rs` |
| MusicBrainz query client | `src/musicbrainz.rs` |
| Track identity, local file binding | `src/track_identity.rs` |
| ID3 frame field types | `src/tag_field.rs` |
| GPUI library view (view code only) | `src/library.rs` |
| GPUI discover/search view (view code only) | `src/search.rs` |
| Top-level GPUI window, tab switching | `src/app.rs` |
| Platform shortcut modifier and typed keyboard/menu bindings | `src/app/keyboard.rs`, `src/app/menu.rs` (ADR 0067) |
| GPUI view composition helpers | `src/views.rs` |
| Feed inspector view | `src/ui_feed.rs` |
| Track inspector view | `src/ui_track.rs` |
| Shared UI helpers (status, layout) | `src/ui_common.rs` |
| GPUI context helpers | `src/ui_context.rs` |
| Theme + scrollbar | `src/ui/theme.rs` |
| Image/media cache helpers | `src/media/` |
| CLI commands | `src/cli.rs` |
| CLI debug JSON contracts | `src/debug_contracts.rs` |

### GPUI framework (`~/dev/vcs-codebases/github.com/zed-industries/zed/crates/gpui/`)

| What you need | File | Key lines |
|---------------|------|-----------|
| `Render` trait, `IntoElement`, `RenderOnce`, `ParentElement` | `src/element.rs` | 112–177 |
| `Entity<T>`, `WeakEntity<T>`, view creation | `src/view.rs` | 28–74 |
| Ownership & data-flow mental model (read this first) | `src/_ownership_and_data_flow.rs` | 1–139 |
| `Context<T>` — `notify`, `spawn`, `listener`, `observe`, `subscribe`, `emit` | `src/app/context.rs` | 19–260 |
| `Context<T>` — window callbacks, focus listeners, `on_action`, `spawn_in` | `src/app/context.rs` | 425–749 |
| `App` — `open_window`, `quit`, clipboard, displays | `src/app.rs` | 1071–1110 |
| `FocusHandle`, focus tab-index, `track_focus` | `src/window.rs` | 349–419 |
| `actions!` macro, `Action` trait, `build`, `name` | `src/action.rs` | 11–173 |
| Working focus + key-binding example | `examples/focus_visible.rs` | full file |
| Working entity + async example | `examples/testing.rs` | full file |
| Window open/close example | `examples/window.rs` | full file |

### gpui-component library (`~/dev/vcs-codebases/github.com/longbridge/gpui-component/`)

| What you need | File | Key lines |
|---------------|------|-----------|
| `gpui_component::init(cx)` — call at startup | `crates/ui/src/lib.rs` | 102–125 |
| `Root::new(view, window, cx)` — required window wrapper | `crates/ui/src/root.rs` | 27–89 |
| `cx.theme()` / `Theme::global` / `Theme::change` / token fields | `crates/ui/src/theme/mod.rs` | 24–180 |
| `Button` — variants, `primary()`, `ghost()`, `on_click` | `crates/ui/src/button/button.rs` | 39–260 |
| `ButtonGroup`, `DropdownButton`, `Toggle` | `crates/ui/src/button/` | — |
| `Input`, `InputState` | `crates/ui/src/input/input.rs` | 33–49 |
| `DataTable`, `TableState` | `crates/ui/src/table/data_table.rs` | 77–163 |
| `TableDelegate` trait — columns, rows, `render_td`, selection | `crates/ui/src/table/delegate.rs` | 16–230 |
| `List`, `ListState` | `crates/ui/src/list/list.rs` | 90–120 |
| `ListDelegate` trait — `items_count`, `render_item`, sections | `crates/ui/src/list/delegate.rs` | 10–171 |
| `Dialog` — `title`, `on_ok`, `on_cancel`, `window.close_dialog` | `crates/ui/src/dialog/dialog.rs` | 34–260 |
| `Sheet` / drawer — `window.open_sheet`, overlay dismiss | `crates/ui/src/sheet.rs` | 44–205 |
| `Sidebar`, `SidebarItem` | `crates/ui/src/sidebar/mod.rs` | 27–153 |
| Scrollable areas — `.overflow_scrollbar()` etc. | `crates/ui/src/scroll/scrollable.rs` | 13–208 |
| Full working example app | `examples/hello_world/src/main.rs` | full file |
| Full dialog + sheet example | `examples/dialog_overlay/src/main.rs` | full file |

## Build, Test, and Lint

```bash
cargo build                          # build the binary
cargo run                            # start the desktop UI
cargo test --lib                     # run all unit tests
cargo test --lib <module>::<test>    # run a single test
cargo clippy -- -D warnings          # lint (must pass clean)
cargo fmt -- --check                 # format check
```

Each ADR phase must pass all four checks above before proceeding.

## Architecture Overview

`v4vmm` is a single-binary Rust desktop app (GPUI frontend) with a small CLI surface. It is **not** a general-purpose media player or tag editor — it manages a MusicIndex-backed music library stored in a local SQLite database.

### Data sources (kept separate, not merged)

1. **RSS** — source-of-record for subscription import; stored in SQLite
2. **MusicIndex HTTP API** — discovery, feed/track detail hydration, `updated_at` freshness checks
3. **Embedded ID3 tags** — read/write on local MP3 files; other formats are downloaded but not fully editable
4. **MusicBrainz** — optional metadata enrichment via metadata queries (no fingerprinting)

### Module roles

| Module | Role |
|--------|------|
| `db.rs` | All SQLite schema, row types (`FeedRow`, `TrackRow`, `PlaybackSessionRow`), and queries |
| `config.rs` | `Config` struct, `~/.config/v4vmm/config.toml` read/write |
| `api.rs` | MusicIndex HTTP client |
| `rss/` | RSS fetch, parse, Podcasting 2.0 extensions |
| `audio_tags.rs` | ID3v2.4 read/write; `lofty` for non-MP3 formats |
| `audio_format.rs` | File-byte format detection |
| `track_compare.rs` | Side-by-side provenance diff (RSS vs ID3 vs MusicBrainz) |
| `metadata.rs` | `TrackContext`, `TagCompareResult` — must have **no GPUI imports** (ADR 0022) |
| `metadata_service.rs` | Pure metadata derivation: `id3_edits_for_track_context` |
| `subscribe_service.rs` | Canonical download + tag + library-membership pipeline |
| `feed_service.rs` | `ensure_feed_in_db`, feed subscription glue |
| `library_service.rs` | Library membership, `subscribe_then_append_to_playlist` |
| `playlist_service.rs` | Playlist CRUD and track append |
| `playback.rs` / `playback_owner.rs` / `playback_driver/` | Now-playing session state and mpv integration |
| `library.rs` / `search.rs` | **GPUI view code only** — no domain logic (ADR 0022 in progress) |
| `ui_*.rs`, `ui/`, `views.rs`, `app.rs` | GPUI rendering helpers and top-level window |
| `cli.rs` | One-shot CLI commands; shares service modules with UI |

### Service boundary rules (ADR 0015 / ADR 0022)

- Service modules (`*_service.rs`, `db`, `api`, `musicbrainz`, `audio_tags`, `track_compare`, `rss`, `playback`) must **not** import `gpui` or `gpui_component`.
- Service modules must **not** import `library`, `search`, `app`, or any `ui_*` module.
- Service functions are **blocking**; the UI moves them off the foreground executor.
- `library.rs` and `search.rs` must contain only `*App` state structs, event handlers, `Render` impls, and async glue. Domain logic belongs in service modules.

## Key Conventions

### Provenance model

The app is **provenance-first**: RSS, ID3, and MusicBrainz values stay visible as separate columns in the compare view. Conflicts are surfaced, not auto-resolved. Never silently merge metadata sources.

### ID3 write boundary (ADR 0008)

All ID3v2.4 writes go through `audio_tags::apply_id3v24_edits`. Drag/drop staging is in-memory only. The write path is only called after an explicit operator action. Duplicate writes to the same effective ID3 target are treated as conflicts and must be resolved before apply.

### Schema migrations (ADR 0016)

New durable tables and columns must go through the migration registry in `db.rs` (monotonically increasing versions recorded in `schema_migrations`). The inline `CREATE TABLE IF NOT EXISTS` block initializes fresh databases; migrations handle existing ones. Both paths must be idempotent.

### Audio format handling

- Format is detected from **file bytes** after download, not from MIME type or URL extension alone.
- WAV downloads are silently re-encoded to FLAC via the `flac` CLI (`flac_path` config key or `$PATH`). If `flac` is absent, the WAV is kept and left untagged.
- The richest compare/edit workflows require MP3/ID3 files. Other formats may be stored locally but do not have symmetric tag-edit support.

### Enclosure selection priority

1. Primary source enclosure (when present)
2. First supported source enclosure
3. Track's direct enclosure URL

### File layout

```
<music_dir>/artists/<artist>/<feed-or-release>/<filename>
```

Path segments are sanitized for invalid characters, reserved names, and length.

### `metadata.rs` image handling (ADR 0022)

Cover art is carried as `ImageBytes { data: Vec<u8>, mime_type: String }` — not `Arc<gpui::Image>`. The UI layer converts at render-preparation time. Do not introduce `gpui::ImageFormat` or `SharedString` into `metadata.rs`.

### CLI JSON output

All CLI inspection commands output structured JSON and call the same non-UI service functions as the UI. Add new CLI commands only when the backing service exists.

### Playback session

`playback_sessions` is the authoritative now-playing state. Player adapters report transport facts (position, pause) into this model — they do not define metadata identity. One-shot CLI commands do not spawn or reuse mpv; that belongs to the long-running desktop/daemon owner.

## GPUI Reference

### Rendering model

`Render` is the core view trait. GPUI calls `render()` every frame, builds an element tree, lays it out, paints it, then drops the tree.

```rust
impl Render for MyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().gap_2().child("hello")
    }
}
```

### Entity / View system

A "view" is `Entity<T>` where `T: Render`. Entities are owned by `App`; all access goes through context methods.

```rust
// create
let view = cx.new(|cx| MyView::new(cx));

// update from outside
view.update(cx, |this, cx| {
    this.count += 1;
    cx.notify();   // triggers re-render
});
```

Use `cx.weak_entity()` / `WeakEntity<T>` in async closures and listeners — call `.upgrade()` before touching state to avoid dangling references.

### Context types

| Type | When you have it | Key methods |
|------|-----------------|-------------|
| `Context<T>` | Inside entity methods | `cx.notify()`, `cx.spawn()`, `cx.emit()`, `cx.listener()`, `cx.observe()`, `cx.subscribe()`, `cx.focus_handle()` |
| `App` | App-level callbacks | `cx.open_window()`, `cx.quit()` |
| `Window` | Render / event handlers | `window.focus()`, `window.open_sheet()`, `window.close_dialog()`, `window.remove_window()` |

`Context<T>` derefs to `App`, so app-level methods are always available inside entity methods.

### Element / layout system

`div()` is the primary building block. All style methods are fluent chains:

```rust
div()
    .flex()
    .flex_col()
    .gap_4()
    .p_2()
    .bg(cx.theme().background)
    .child(Label::new("title"))
    .children(items.iter().map(|i| render_item(i)))
```

`RenderOnce` is for stateless one-shot components that don't need to be retained as entities.

### Event handling

```rust
// inside render(), wire a closure back to self
.on_click(cx.listener(|this, _event, _window, cx| {
    this.count += 1;
    cx.notify();
}))
```

For entity-to-entity communication use `EventEmitter` + `cx.subscribe()`:

```rust
// define
impl EventEmitter<MyEvent> for MyView {}

// emit
cx.emit(MyEvent::Selected(id));

// subscribe in parent
cx.subscribe(&child_entity, |this, _child, event, cx| {
    // handle event
});
```

### Actions

```rust
actions!(my_app, [Confirm, Cancel]);

// bind keys (usually in main or app setup)
cx.bind_keys([KeyBinding::new("enter", Confirm, None)]);

// handle in render
div()
    .track_focus(&self.focus_handle)
    .on_action(cx.listener(Self::on_confirm))

fn on_confirm(&mut self, _: &Confirm, _window: &mut Window, cx: &mut Context<Self>) { ... }
```

### Async tasks

Keep service calls off the foreground executor. The standard pattern:

```rust
// in an event handler
cx.spawn(async move |this, mut cx| {
    // cx is AsyncContext here — cannot call notify directly
    let result = cx.background_executor()
        .spawn(async move { blocking_service_call() })
        .await;

    this.update(&mut cx, |view, cx| {
        view.result = result;
        cx.notify();
    }).ok();
})
.detach();
```

- `cx.spawn(...)` → entity-scoped, use `this.update(...)` to write back
- `cx.spawn_in(window, ...)` → window-scoped
- `cx.background_executor().spawn(...)` → pure background, no UI access

### Focus and keyboard

```rust
struct MyView { focus_handle: FocusHandle }

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle() }
    }
}

impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            // focus listeners
            .on_focus(cx.listener(|this, _, _, cx| { ... }))
    }
}

// to focus programmatically
window.focus(&self.focus_handle, cx);
```

### Window management

```rust
// open
let handle = cx.open_window(WindowOptions { .. }, |window, cx| {
    let view = cx.new(|cx| MyView::new(cx));
    cx.new(|cx| Root::new(view, window, cx))  // Root is required (see below)
});

// close from inside
window.remove_window();
```

---

## gpui-component Reference

### Required initialization

Call once at app startup before any component is used:

```rust
gpui_component::init(cx);
```

### Root wrapper — required

Every top-level window view must be wrapped in `Root`:

```rust
cx.open_window(opts, |window, cx| {
    let content = cx.new(|cx| MyApp::new(cx));
    cx.new(|cx| Root::new(content, window, cx))
});
```

`Root` manages dialog, sheet, and dock overlay layers. Without it, `Dialog` and `Sheet` will not render.

### Theme

```rust
// access current theme tokens in render
let bg = cx.theme().background;
let border = cx.theme().border;

// change theme mode
Theme::change(ThemeMode::Dark, None, cx);
```

Tokens include `.background`, `.foreground`, `.border`, `.sidebar`, `.table`, and family-specific palettes.

### Common components

All components use a **builder / method-chain** pattern and are `RenderOnce`:

```rust
// Button
Button::new("save").primary().label("Save").on_click(|_, _, _| { ... })
Button::new("del").ghost().icon(Icon::Trash).on_click(cx.listener(...))

// Input — requires an Entity<InputState>
let state = cx.new(|cx| InputState::new(window, cx));
Input::new(&state).placeholder("Search…").cleanable(true)

// DataTable — requires Entity<TableState<D>> where D: TableDelegate
DataTable::new(&table_state).stripe(true).bordered(true)

// Sidebar
Sidebar::new("main-sidebar")
    .header(...)
    .child(SidebarItem::new(...))
    .footer(...)
```

### Table delegate pattern

```rust
struct MyDelegate { rows: Vec<MyRow> }

impl TableDelegate for MyDelegate {
    fn columns_count(&self) -> usize { 3 }
    fn rows_count(&self) -> usize { self.rows.len() }
    fn column(&self, ix: usize) -> Arc<dyn TableColumn> { ... }
    fn render_td(&self, row: usize, col: usize, _window: &mut Window, cx: &mut App)
        -> impl IntoElement { ... }
}

// create state
let state = cx.new(|cx| TableState::new(Box::new(MyDelegate { .. }), window, cx));
```

### List delegate pattern

```rust
impl ListDelegate for MyDelegate {
    fn items_count(&self, _section: usize, _cx: &App) -> usize { self.items.len() }
    fn render_item(&self, ix: IndexPath, _window: &mut Window, _cx: &mut App)
        -> Option<impl IntoElement> { Some(div().child(self.items[ix.item].clone())) }
}

let state = cx.new(|cx| ListState::new(Box::new(MyDelegate { .. }), window, cx));
```

### Dialog

```rust
// open a dialog — Root must be present
Dialog::new(cx)
    .title("Confirm")
    .child(Label::new("Are you sure?"))
    .on_ok(|window, cx| {
        // return true to close, false to keep open
        true
    })
    .open(window, cx);

// close manually
window.close_dialog(cx);
```

### Sheet / drawer

```rust
window.open_sheet(cx, |sheet, window, cx| {
    sheet.title("Details").child(MyDetailView::new(cx))
});
```
