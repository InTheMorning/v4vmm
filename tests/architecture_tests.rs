use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const VIEW_MODEL_FORBIDDEN_PATTERNS: &[&str] = &[
    "use gpui",
    "gpui::",
    "use gpui_component",
    "gpui_component::",
    "crate::ui::",
    "crate::ui_",
    "crate::library::",
    "crate::search::",
    "crate::app::",
    "crate::discover",
];

const ENTITY_DETAIL_FORBIDDEN_PATTERNS: &[&str] = &[
    "use gpui",
    "gpui::",
    "use gpui_component",
    "gpui_component::",
    "crate::api",
    "crate::db",
    "crate::ui::",
    "crate::ui_",
    "crate::library::",
    "crate::search::",
    "crate::app::",
    "crate::feed_service",
    "crate::library_service",
    "crate::metadata_service",
    "crate::playlist_service",
    "crate::subscribe_service",
    "crate::track_compare",
    "rusqlite",
];

const UI_ENTITY_FORBIDDEN_PATTERNS: &[&str] = &[
    "crate::library::",
    "crate::search::",
    "crate::app::",
    "crate::api::",
    "crate::db::",
    "crate::feed_service",
    "crate::library_service",
    "crate::metadata_service",
    "crate::playlist_service",
    "crate::subscribe_service",
    "crate::track_compare",
];

const SHARED_UI_BACKEND_FORBIDDEN_PATTERNS: &[&str] = &[
    "crate::application",
    "crate::api",
    "crate::db",
    "crate::feed_service",
    "crate::library_service",
    "crate::metadata_service",
    "crate::playlist_service",
    "crate::subscribe_service",
    "crate::track_compare",
    "rusqlite",
    "crate::library::",
    "crate::search::",
    "crate::app::",
];

const APPLICATION_FORBIDDEN_PATTERNS: &[&str] = &[
    "use gpui",
    "gpui::",
    "use gpui_component",
    "gpui_component::",
    "crate::ui::",
    "crate::ui_",
    "crate::library::",
    "crate::search::",
    "crate::app::",
];

const NON_UI_CORE_PATHS: &[&str] = &[
    "src/config.rs",
    "src/feed_service.rs",
    "src/local_identity.rs",
    "src/library_service.rs",
    "src/metadata.rs",
    "src/metadata_service.rs",
    "src/musicbrainz.rs",
    "src/playback.rs",
    "src/playback_driver",
    "src/playlist_service.rs",
    "src/rss",
    "src/sources.rs",
    "src/subscribe_service.rs",
    "src/track_compare.rs",
    "src/track_identity.rs",
];

const SCREEN_PLAYLIST_SERVICE_FORBIDDEN_PATTERNS: &[&str] =
    &["use crate::playlist_service", "playlist_service::"];

const SCREEN_SUBSCRIPTION_FORBIDDEN_PATTERNS: &[&str] = &[
    "db::set_feed_subscribed(",
    "db::unsubscribe_feed_tracks(",
    "library_service::set_track_in_library(",
    "library_service::set_track_in_library_by_match(",
    "library_service::delete_local_file(",
    "library_service::delete_cached_file(",
    "library_service::cached_tracks(",
    "library_service::subscribe_then_append_to_playlist(",
    "subscribe_service::subscribe_feed(",
    "subscribe_service::subscribe_track(",
];

const SCREEN_LIBRARY_REMOVAL_LEGACY_PATTERNS: &[&str] = &[
    "RemoveTrackFromLibrary::new",
    "RemoveTrackFromLibraryByMatch::new",
    "UnsubscribeFeedById::new",
    "UnsubscribeFeedByUrl::new",
];

const LIBRARY_REMOVAL_PRESENTATION_FILES: &[&str] = &[
    "src/library.rs",
    "src/library/app_impl.rs",
    "src/discover.rs",
    "src/discover/app_impl.rs",
];

const SCREEN_LIBRARY_REMOVAL_PRESENTATION_FORBIDDEN_PATTERNS: &[(&str, &str)] = &[
    (
        "pending_library_removal_origin",
        "pending library-removal origin belongs in a GPUI-free view model",
    ),
    (
        "ConfirmationDialogDisplay",
        "screen modules must use the shared library-removal confirmation adapter",
    ),
    (
        "ConfirmationDialogHandlers",
        "screen modules must not own confirmation-dialog handler plumbing",
    ),
    (
        "window.open_dialog",
        "screen modules must use the shell-level library-removal confirmation presenter",
    ),
    (
        "library_removal_confirmation_dialog(dialog",
        "legacy library-removal dialog adapters belong in shells, not screens",
    ),
    (
        "fn open_pending_library_removal_dialog",
        "screen-local pending removal dialog presenters duplicate shared shell presentation",
    ),
    (
        "fn removal_confirmation_dialog",
        "screen-local removal confirmation adapters duplicate shared dialog chrome",
    ),
    (
        "fn search_removal_confirmation_dialog",
        "screen-local removal confirmation adapters duplicate shared dialog chrome",
    ),
];

const SCREEN_METADATA_FEED_FORBIDDEN_PATTERNS: &[&str] = &[
    "db::subscribed_feeds_for_stale_check(",
    "feed_service::apply_feed_updates(",
    "feed_service::check_feed_staleness(",
    "feed_service::lookup_musicbrainz_library_track(",
    "feed_service::lookup_musicbrainz_stage_for_track(",
    "feed_service::stage_candidate_for_track(",
    "lookup_releases(",
];

const SCREEN_PLAYBACK_FORBIDDEN_PATTERNS: &[&str] = &[
    "playback_owner.play_playlist_at(",
    "playback_owner.skip_next(",
    "playback_owner.skip_previous(",
    "playback_owner.pause(",
    "playback_owner.stop(",
    "playback::now_playing_update(",
    "db::playback_session(",
    "StartPlayback",
];

const DEPRECATED_VISUAL_HELPER_BASELINES: &[DeprecatedVisualHelperBaseline] = &[
    DeprecatedVisualHelperBaseline {
        file: "src/library.rs",
        helper: "theme::color",
        import_patterns: &[
            "use crate::ui::theme::color;",
            "use crate::ui::theme::{color,",
            "use crate::ui::theme::{color}",
        ],
        usage_pattern: "color::",
        max_count: 0,
    },
    DeprecatedVisualHelperBaseline {
        file: "src/library.rs",
        helper: "theme::badges",
        import_patterns: &[
            "use crate::ui::theme::badges;",
            "use crate::ui::theme::{badges,",
            "use crate::ui::theme::{badges}",
        ],
        usage_pattern: "badges::",
        max_count: 0,
    },
    DeprecatedVisualHelperBaseline {
        file: "src/library.rs",
        helper: "theme::glyphs",
        import_patterns: &[
            "use crate::ui::theme::glyphs;",
            "use crate::ui::theme::{glyphs,",
            "use crate::ui::theme::{glyphs}",
        ],
        usage_pattern: "glyphs::",
        max_count: 0,
    },
    DeprecatedVisualHelperBaseline {
        file: "src/discover.rs",
        helper: "theme::color",
        import_patterns: &[
            "use crate::ui::theme::color;",
            "use crate::ui::theme::{color,",
            "use crate::ui::theme::{color}",
        ],
        usage_pattern: "color::",
        max_count: 0,
    },
    DeprecatedVisualHelperBaseline {
        file: "src/discover.rs",
        helper: "theme::badges",
        import_patterns: &[
            "use crate::ui::theme::badges;",
            "use crate::ui::theme::{badges,",
            "use crate::ui::theme::{badges}",
        ],
        usage_pattern: "badges::",
        max_count: 0,
    },
    DeprecatedVisualHelperBaseline {
        file: "src/discover.rs",
        helper: "theme::glyphs",
        import_patterns: &[
            "use crate::ui::theme::glyphs;",
            "use crate::ui::theme::{glyphs,",
            "use crate::ui::theme::{glyphs}",
        ],
        usage_pattern: "glyphs::",
        max_count: 0,
    },
];

const DEPRECATED_VISUAL_HELPERS: &[DeprecatedVisualHelper] = &[
    DeprecatedVisualHelper {
        helper: "theme::color",
        import_patterns: &[
            "use crate::ui::theme::color;",
            "use crate::ui::theme::{color,",
            "use crate::ui::theme::{color}",
        ],
        usage_pattern: "color::",
    },
    DeprecatedVisualHelper {
        helper: "theme::badges",
        import_patterns: &[
            "use crate::ui::theme::badges;",
            "use crate::ui::theme::{badges,",
            "use crate::ui::theme::{badges}",
        ],
        usage_pattern: "badges::",
    },
    DeprecatedVisualHelper {
        helper: "theme::glyphs",
        import_patterns: &[
            "use crate::ui::theme::glyphs;",
            "use crate::ui::theme::{glyphs,",
            "use crate::ui::theme::{glyphs}",
        ],
        usage_pattern: "glyphs::",
    },
];

const DIRECT_COMPONENT_BUTTON_BASELINES: &[DirectComponentButtonBaseline] = &[
    DirectComponentButtonBaseline {
        file: "src/app.rs",
        max_unmarked_count: 0,
    },
    DirectComponentButtonBaseline {
        file: "src/library.rs",
        max_unmarked_count: 0,
    },
    DirectComponentButtonBaseline {
        file: "src/discover.rs",
        max_unmarked_count: 0,
    },
];

const PROVENANCE_DIFF_HELPER_BASELINES: &[DiffHelperBaseline] = &[
    DiffHelperBaseline {
        file: "src/library.rs",
        pattern: "color::diff_",
        max_count: 0,
    },
    DiffHelperBaseline {
        file: "src/library.rs",
        pattern: "glyphs::DIFF_",
        max_count: 0,
    },
    DiffHelperBaseline {
        file: "src/discover.rs",
        pattern: "color::diff_",
        max_count: 0,
    },
    DiffHelperBaseline {
        file: "src/discover.rs",
        pattern: "glyphs::DIFF_",
        max_count: 0,
    },
];

const SCREEN_LOCAL_PLAYLIST_POPOVER_BASELINES: &[ScreenLocalPlaylistPopoverBaseline] = &[
    ScreenLocalPlaylistPopoverBaseline {
        file: "src/library.rs",
        pattern: "fn render_add_to_playlist_panel(",
        max_count: 0,
        note: "legacy Library track-inspector playlist panel",
    },
    ScreenLocalPlaylistPopoverBaseline {
        file: "src/library.rs",
        pattern: ".when(frame.add_to_playlist_open, |el|",
        max_count: 0,
        note: "legacy Library track-inspector playlist panel toggle",
    },
    ScreenLocalPlaylistPopoverBaseline {
        file: "src/discover.rs",
        pattern: "fn render_add_to_playlist_panel_search(",
        max_count: 0,
        note: "legacy Discover inspector playlist panel",
    },
    ScreenLocalPlaylistPopoverBaseline {
        file: "src/discover.rs",
        pattern: ".when(frame.add_to_playlist_open, |el|",
        max_count: 0,
        note: "legacy Discover inspector playlist panel toggle",
    },
    ScreenLocalPlaylistPopoverBaseline {
        file: "src/discover.rs",
        pattern: "fn render_row_playlist_popup(",
        max_count: 0,
        note: "legacy Discover row popup compatibility wrapper",
    },
];

const RENDER_HELPER_DUPLICATION_BASELINES: &[RenderHelperDuplicationBaseline] = &[];

const PLAYLIST_POPOVER_CALLSITE_FILES: &[&str] = &[
    "src/library.rs",
    "src/discover.rs",
    "src/ui/shells/track.rs",
    "src/ui/shells/library/feed_detail.rs",
    "src/ui/shells/library/track_detail_metadata.rs",
    "src/ui/shells/discover/actions.rs",
];

const RELEASE_PLAYLIST_POPOVER_FORBIDDEN_PATTERNS: &[&str] = &[
    "render_album_track_add_panel",
    "render_album_feed_add_panel",
    "album_track_picker_open",
    "album_feed_picker_open",
    "album_add_open_track",
    "album_add_open_feed",
];

const SHARED_VIEW_FACT_FORBIDDEN_PUBLIC_FIELDS: &[&str] = &[
    "pub contributors: Vec<api::Contributor>",
    "pub source_links: Vec<api::SourceEntityLink>",
    "pub source_ids: Vec<api::SourceEntityId>",
];

const SCREEN_CONTRIBUTOR_PANEL_FORBIDDEN_PATTERNS: &[&str] = &[
    "ContributorVm",
    "contributors: LazyPanel<Vec<Contributor>>",
    "contributors: LazyPanel<Vec<api::Contributor>>",
];

const SCREEN_FILES: &[&str] = &[
    "src/app.rs",
    "src/app/breadcrumb.rs",
    "src/app/startup.rs",
    "src/app/capabilities.rs",
    "src/app/bootstrap.rs",
    "src/app/events.rs",
    "src/app/keyboard.rs",
    "src/app/menu.rs",
    "src/app/playback_bar.rs",
    "src/app/resize.rs",
    "src/app/search_dispatch.rs",
    "src/app/tab_bar.rs",
    "src/library.rs",
    "src/discover.rs",
];

const SCREEN_SURFACE_DIRS: &[&str] = &["src/ui/shells/library", "src/ui/shells/discover"];

const LIBRARY_SCREEN_SURFACE_FILES: &[&str] = &[
    "src/ui/shells/library/mod.rs",
    "src/ui/shells/library/detail.rs",
    "src/ui/shells/library/feed_detail.rs",
    "src/ui/shells/library/feed_list.rs",
    "src/ui/shells/library/playlist_detail.rs",
    "src/ui/shells/library/sidebar.rs",
    "src/ui/shells/library/thumbnail.rs",
    "src/ui/shells/library/track_detail.rs",
    "src/ui/shells/library/track_detail_metadata.rs",
    "src/ui/shells/library/track_detail_metadata_cells.rs",
    "src/ui/shells/library/track_detail_metadata_grid.rs",
    "src/ui/shells/library/track_detail_metadata_values.rs",
];

const DISCOVER_SCREEN_SURFACE_FILES: &[&str] = &[
    "src/ui/shells/discover/mod.rs",
    "src/ui/shells/discover/actions.rs",
    "src/ui/shells/discover/feed_inspector.rs",
    "src/ui/shells/discover/feed_lists.rs",
    "src/ui/shells/discover/recent.rs",
    "src/ui/shells/discover/result_list.rs",
    "src/ui/shells/discover/search_input.rs",
    "src/ui/shells/discover/track_inspector.rs",
    "src/ui/shells/discover/track_inspector_metadata.rs",
    "src/ui/shells/discover/track_inspector_metadata_cells.rs",
    "src/ui/shells/discover/track_inspector_metadata_expandable.rs",
    "src/ui/shells/discover/track_inspector_metadata_grid.rs",
    "src/ui/shells/discover/track_inspector_metadata_test_helpers.rs",
    "src/ui/shells/discover/track_inspector_metadata_tree.rs",
    "src/ui/shells/discover/track_rows.rs",
];

const PRESENTATION_GLUE_FILES: &[&str] = &[
    "src/app.rs",
    "src/app/playback_bar.rs",
    "src/app/queue_now_playing.rs",
    "src/app/show.rs",
    "src/app/tab_bar.rs",
    "src/library.rs",
    "src/discover.rs",
];

const SCREEN_LOCAL_FLOATING_CHROME_FORBIDDEN_PATTERNS: &[&str] = &[
    "gpui_component::popover",
    "SurfaceElevation::Floating",
    ".absolute()",
    ".fixed()",
    ".z_index(",
];

const COMPOSITE_DISPLAY_CONTRACT_STRING_API_ALLOWLIST: &[CompositeStringApiAllowance] = &[
    CompositeStringApiAllowance {
        file: "src/ui/composites/playlist_popover.rs",
        pattern:
            "pub fn on_create(mut self, handler: impl Fn(&String, &mut Window, &mut App) + 'static)",
        note: "callback payload for new playlist name, not display label input",
    },
    CompositeStringApiAllowance {
        file: "src/ui/composites/release_detail_surface.rs",
        pattern: "pub fn new(id: impl Into<SharedString>)",
        note: "element id, not display copy",
    },
    CompositeStringApiAllowance {
        file: "src/ui/composites/split_pane.rs",
        pattern: "pub fn new(id: impl Into<SharedString>)",
        note: "element id, not display copy",
    },
    CompositeStringApiAllowance {
        file: "src/ui/composites/split_pane.rs",
        pattern: "pub fn resize_handle_id(mut self, id: impl Into<SharedString>)",
        note: "element id, not display copy",
    },
    CompositeStringApiAllowance {
        file: "src/ui/composites/tag_badge.rs",
        pattern: "pub fn from_legacy_str(s: &str)",
        note: "legacy role parser, not display copy",
    },
    CompositeStringApiAllowance {
        file: "src/ui/composites/tag_badge.rs",
        pattern: "pub fn label(self) -> &'static str",
        note: "role enum owns its static label",
    },
    CompositeStringApiAllowance {
        file: "src/ui/composites/tag_badge.rs",
        pattern: "pub fn accessibility_label(self) -> &'static str",
        note: "role enum owns its static accessibility label",
    },
];

const SHARED_UI_UNSCALED_TOKEN_PX_ALLOWLIST: &[SharedUiUnscaledTokenPxAllowance] =
    &[SharedUiUnscaledTokenPxAllowance {
        file: "src/ui/icons.rs",
        pattern: "Self::Transport => FontSize::Body.px()",
        note: "base value for IconSize::scaled; render path uses the scaled helper",
    }];

const ADR0054_METADATA_STORAGE_CALLER_ALLOWLIST: &[&str] = &[
    "src/db.rs",
    "src/identity_ingest.rs",
    "src/local_metadata.rs",
    "src/sources.rs",
    "src/feed_service.rs",
    "src/library/app_impl.rs",
];

const ADR0054_FEED_FACT_KEYS: &[&str] = &[
    "publisher_text",
    "musicindex_release_kind",
    "release_date",
    "language",
    "explicit",
    "description",
    "rss_podcast_medium",
];

const ADR0054_TRACK_FACT_KEYS: &[&str] = &["publisher_text", "description", "pub_date", "explicit"];

#[derive(Debug)]
struct DeprecatedVisualHelperBaseline {
    file: &'static str,
    helper: &'static str,
    import_patterns: &'static [&'static str],
    usage_pattern: &'static str,
    max_count: usize,
}

#[derive(Debug)]
struct CompositeStringApiAllowance {
    file: &'static str,
    pattern: &'static str,
    note: &'static str,
}

#[derive(Debug)]
struct SharedUiUnscaledTokenPxAllowance {
    file: &'static str,
    pattern: &'static str,
    note: &'static str,
}

#[derive(Debug)]
struct DeprecatedVisualHelper {
    helper: &'static str,
    import_patterns: &'static [&'static str],
    usage_pattern: &'static str,
}

#[derive(Debug)]
struct DirectComponentButtonBaseline {
    file: &'static str,
    max_unmarked_count: usize,
}

#[derive(Debug)]
struct DiffHelperBaseline {
    file: &'static str,
    pattern: &'static str,
    max_count: usize,
}

#[derive(Debug)]
struct ScreenLocalPlaylistPopoverBaseline {
    file: &'static str,
    pattern: &'static str,
    max_count: usize,
    note: &'static str,
}

#[derive(Debug)]
struct RenderHelperDuplicationBaseline {
    helper: &'static str,
    files: &'static [&'static str],
    note: &'static str,
}

#[test]
fn agent_guidelines_lock_user_confirmed_regression_ratchet() {
    let source = read_source(&manifest_path("docs/architecture/ui-regression-ratchet.md"));
    let mut violations = Vec::new();

    for required in [
        "UI Regression Ratchet",
        "Every user-confirmed bug fix gets a guard",
        "Completed ADR behavior is locked",
        "Visual presentation, button behavior, and user-workflow changes",
        "isolated renderer tweaks for music presentation",
        "Agent Acceptance Checklist",
        "No shell/layout change may land without scroll-chain verification",
        "Search type filters apply to every visible result section",
        "Inspectors must not show raw transport errors",
        "Subagents get bounded write scopes",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "docs/architecture/ui-regression-ratchet.md: regression-ratchet policy missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Agent regression-ratchet guideline violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn agent_guidelines_require_structural_ui_change_ownership() {
    let agent_source = read_source(&manifest_path("AGENTS.md"));
    let boundary_source = read_source(&manifest_path("docs/architecture/ui-backend-boundary.md"));
    let governance_source = read_source(&manifest_path(
        "docs/adr/0033-hig-ui-architecture-governance.md",
    ));

    let mut violations = Vec::new();
    for (file, source, required) in [
        (
            "AGENTS.md",
            agent_source.as_str(),
            "UI change acceptance gate",
        ),
        (
            "AGENTS.md",
            agent_source.as_str(),
            "No isolated visual tweaks",
        ),
        (
            "AGENTS.md",
            agent_source.as_str(),
            "Button and action discipline",
        ),
        (
            "AGENTS.md",
            agent_source.as_str(),
            "Workflow-first requirement",
        ),
        (
            "docs/architecture/ui-backend-boundary.md",
            boundary_source.as_str(),
            "Visual Workflow Ownership Gate",
        ),
        (
            "docs/architecture/ui-backend-boundary.md",
            boundary_source.as_str(),
            "Forbidden Easy Fixes",
        ),
        (
            "docs/architecture/ui-backend-boundary.md",
            boundary_source.as_str(),
            "the smallest change to the correct shared owner",
        ),
        (
            "docs/adr/0033-hig-ui-architecture-governance.md",
            governance_source.as_str(),
            "Agent default choices",
        ),
        (
            "docs/adr/0033-hig-ui-architecture-governance.md",
            governance_source.as_str(),
            "renderer patch for a repeated visual affordance is architectural drift",
        ),
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "{file}: structural UI ownership guidance missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Agent structural UI governance violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn hig_product_polish_backlog_stays_separate_from_restructuring() {
    let backlog_source = read_source(&manifest_path("docs/plans/hig-product-polish-backlog.md"));
    let regression_source =
        read_source(&manifest_path("docs/architecture/ui-regression-ratchet.md"));
    let index_source = read_source(&manifest_path(
        "docs/plans/deferred-architecture-work-index.md",
    ));
    let readme_source = read_source(&manifest_path("docs/README.md"));
    let agent_source = read_source(&manifest_path("AGENTS.md"));

    let mut violations = Vec::new();
    for (file, source, required) in [
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "HIG Product Polish Backlog",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "strategic UI restructuring work that has already landed",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "Track A - Tactical Structural Mop-Ups",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "Track B - HIG Product-Completeness Gaps",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "Recent Searches and Search Suggestions",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "Sidebar Show/Hide and Customization",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "Liquid Glass Material Adoption",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "Keyboard Shortcut Coverage",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "Keep the global toolbar input as the single search entry",
        ),
        (
            "docs/plans/hig-product-polish-backlog.md",
            backlog_source.as_str(),
            "non-conflicting in the target GPUI/macOS context",
        ),
        (
            "docs/architecture/ui-regression-ratchet.md",
            regression_source.as_str(),
            "HIG product-completeness gaps are a separate polish backlog",
        ),
        (
            "docs/plans/deferred-architecture-work-index.md",
            index_source.as_str(),
            "hig-product-polish-backlog.md",
        ),
        (
            "docs/README.md",
            readme_source.as_str(),
            "HIG product polish backlog",
        ),
        (
            "AGENTS.md",
            agent_source.as_str(),
            "HIG product polish is separate from restructuring",
        ),
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "{file}: HIG product-polish backlog guidance missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "HIG product-polish backlog guidance violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn view_models_do_not_import_gpui_or_screen_layers() {
    let mut violations = Vec::new();
    for path in rust_files_under("src/view_models") {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in VIEW_MODEL_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{}:{line_number}: forbidden view-model dependency `{pattern}` in `{line}`",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0023 view-model boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn local_feed_language_parity_is_loaded_through_read_model_path() {
    let db_source = read_source(&manifest_path("src/db.rs"));
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let library_query_source = read_source(&manifest_path("src/application/queries/library.rs"));
    let feed_detail_source = read_source(&manifest_path("src/ui/shells/library/feed_detail.rs"));
    let views_source = read_source(&manifest_path("src/views.rs"));

    for (source_name, source, required) in [
        (
            "src/db.rs",
            db_source.as_str(),
            "pub language: Option<String>",
        ),
        (
            "src/db.rs",
            db_source.as_str(),
            "title, language, description",
        ),
        (
            "src/db.rs",
            db_source.as_str(),
            "pub fn feed_language_by_id",
        ),
        (
            "src/view_models/library.rs",
            library_vm_source.as_str(),
            "pub(crate) language: Option<String>",
        ),
        (
            "src/application/queries/library.rs",
            library_query_source.as_str(),
            "feed_language_cache",
        ),
        (
            "src/application/queries/library.rs",
            library_query_source.as_str(),
            "db::feed_language_by_id(conn, fid)",
        ),
        (
            "src/library/app_impl.rs",
            library_app_source.as_str(),
            "db::feed_language_by_id(&conn, feed_id)",
        ),
        (
            "src/ui/shells/library/feed_detail.rs",
            feed_detail_source.as_str(),
            "language: album.language.clone()",
        ),
        (
            "src/views.rs",
            views_source.as_str(),
            "language: nonempty_owned(f.language)",
        ),
    ] {
        assert!(
            source.contains(required),
            "{source_name}: local feed language parity must route through FeedRow, subscribed_feeds, AlbumNode, and FeedView; missing `{required}`"
        );
    }

    assert!(
        !feed_detail_source.contains("\"Language\""),
        "src/ui/shells/library/feed_detail.rs must not infer or label feed language in the renderer"
    );
}

#[test]
fn workspace_view_model_contract_is_gpui_free() {
    let source = workspace_vm_source();
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        for pattern in [
            "use gpui",
            "gpui::",
            "use gpui_component",
            "gpui_component::",
        ] {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/view_models/workspace/mod.rs:{line_number}: workspace model must stay GPUI-free; found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 workspace model boundary violations:\n{}",
        violations.join("\n")
    );

    for required in [
        "struct WorkspaceFrameId",
        "enum WorkspaceFrameKind",
        "struct WorkspaceFrameState",
        "struct WorkspaceLayout",
        "struct FrameNavigationState",
        "enum FrameNavigationEntry",
        "enum WorkspaceModelError",
        "struct FrameChromeButtonDisplay",
        "struct FrameChromeMenuItemDisplay",
        "struct FrameShellDisplay",
        "pub(crate) fn from_frame",
        "SourceList",
        "ContentList",
        "Detail",
        "QueueNowPlaying",
        "pub(crate) fn focus_frame",
        "pub(crate) fn go_back",
        "pub(crate) fn go_forward",
    ] {
        assert!(
            source.contains(required),
            "ADR 0046 workspace model contract missing `{required}`"
        );
    }
}

#[test]
fn workspace_frame_shell_display_contract_lives_in_workspace_vm() {
    let source = workspace_vm_source();
    let mut violations = Vec::new();

    for required in [
        "pub(crate) struct FrameChromeButtonDisplay",
        "pub(crate) struct FrameChromeMenuItemDisplay",
        "pub(crate) struct FrameShellDisplay",
        "pub(crate) frame_id: WorkspaceFrameId",
        "pub(crate) title: String",
        "pub(crate) subtitle: Option<String>",
        "pub(crate) status: Option<String>",
        "pub(crate) back: FrameChromeButtonDisplay",
        "pub(crate) forward: FrameChromeButtonDisplay",
        "pub(crate) close: Option<FrameChromeButtonDisplay>",
        "pub(crate) action_menu_items: Vec<FrameChromeMenuItemDisplay>",
        "pub(crate) breadcrumb: Option<BreadcrumbDisplay>",
        "pub(crate) content_slot_id: String",
        "pub(crate) fn from_frame(",
        "nav.can_go_back()",
        "nav.can_go_forward()",
        "allow_close.then",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0046 Task 005 frame-shell display contract missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Task 005 frame-shell display contract violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_phase_b_view_model_contracts_are_gpui_free_and_shared() {
    let workspace_source = workspace_vm_source();
    let library_source = read_source(&manifest_path("src/view_models/library.rs"));
    let search_results_mod_source =
        read_source(&manifest_path("src/view_models/search_results/mod.rs"));
    let search_results_tabs_source =
        read_source(&manifest_path("src/view_models/search_results/tabs.rs"));
    let search_results_results_source =
        read_source(&manifest_path("src/view_models/search_results/results.rs"));
    let search_results_empty_state_source = read_source(&manifest_path(
        "src/view_models/search_results/empty_state.rs",
    ));
    let mod_source = read_source(&manifest_path("src/view_models/mod.rs"));
    let mut violations = Vec::new();

    for (path, source) in [
        (
            "src/view_models/workspace/mod.rs",
            workspace_source.as_str(),
        ),
        ("src/view_models/library.rs", library_source.as_str()),
        (
            "src/view_models/search_results/mod.rs",
            search_results_mod_source.as_str(),
        ),
        (
            "src/view_models/search_results/tabs.rs",
            search_results_tabs_source.as_str(),
        ),
        (
            "src/view_models/search_results/results.rs",
            search_results_results_source.as_str(),
        ),
        (
            "src/view_models/search_results/empty_state.rs",
            search_results_empty_state_source.as_str(),
        ),
    ] {
        for (line_number, line) in code_lines(source) {
            for pattern in [
                "use gpui",
                "gpui::",
                "use gpui_component",
                "gpui_component::",
            ] {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{path}:{line_number}: ADR 0047 Phase B VM contract must stay GPUI-free; found `{pattern}` in `{line}`"
                    ));
                }
            }
        }
    }

    for required in [
        "pub(crate) enum ContentFilter",
        "pub(crate) struct LibraryFilterControlDisplay",
        "pub(crate) struct LibraryFilterControlStateDisplay",
        "pub(crate) enum LibraryFilterControlTreatment",
        "pub(crate) struct FilterChipOption",
        "pub(crate) struct FilterChipStripDisplay",
        "pub(crate) fn default_for_content_list",
        "pub(crate) fn default_for_search_inspector",
    ] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0047 Phase B content-filter contract missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) enum InspectorPanelKind",
        "pub(crate) struct LibraryTrackInspectorState",
        "inspector_expanded_panels: BTreeSet<InspectorPanelKind>",
        "pub(crate) const fn compare_id3_enabled",
        "pub(crate) const fn musicbrainz_enabled",
        "pub(crate) enum DescriptionState",
        "pub(crate) const DESCRIPTION_AUTO_COLLAPSE_LINES: usize = 5",
        "pub(crate) struct SavedSearchEntry",
        "pub(crate) fn set_saved_searches",
    ] {
        if !library_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: ADR 0047 Phase B library VM contract missing `{required}`"
            ));
        }
    }

    for required in [
        "use crate::view_models::workspace::{",
        "ContentFilter, FilterChipStripDisplay, FilterChipStripWidthClass",
        "pub(crate) struct SearchResultsInspectorPageVm",
        "pub(crate) fn filter_chip_strip(&self) -> FilterChipStripDisplay",
        "pub(crate) fn filter_chip_strip_for_width_class(",
        "FilterChipStripDisplay::default_for_search_inspector_width_class(",
        "pub(crate) fn set_tab",
        "pub(crate) fn set_filter",
        "pub(crate) fn is_empty",
    ] {
        if !search_results_mod_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/mod.rs: ADR 0047 Phase B search-results contract missing `{required}`"
            ));
        }
    }

    for required in ["pub(crate) enum SearchResultsTab"] {
        if !search_results_tabs_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/tabs.rs: ADR 0047 Phase B search-results contract missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) struct ArtistResultDisplay",
        "pub(crate) struct FeedResultDisplay",
        "pub(crate) struct TrackResultDisplay",
        "struct LocalArtistResult",
        "struct LocalFeedResult",
    ] {
        if !search_results_results_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/results.rs: ADR 0047 Phase B search-results contract missing `{required}`"
            ));
        }
    }

    for required in ["pub(crate) struct EmptyStateDisplay"] {
        if !search_results_empty_state_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/empty_state.rs: ADR 0047 Phase B search-results contract missing `{required}`"
            ));
        }
    }

    if search_results_mod_source.contains("enum ContentFilter") {
        violations.push(
            "src/view_models/search_results/mod.rs: ADR 0047 Phase B must reuse workspace ContentFilter, not define a second enum"
                .to_string(),
        );
    }

    if !mod_source.contains("pub mod search_results;") {
        violations.push(
            "src/view_models/mod.rs: ADR 0047 Phase B search_results module is not exported"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Phase B view-model contract violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_phase_c_inspector_rewire_uses_vm_state_and_shared_disclosures() {
    let library_struct_source = read_source(&manifest_path("src/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let metadata_vm_source = read_source(&manifest_path("src/view_models/entity_detail.rs"));
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let metadata_shell_source = read_source(&manifest_path(
        "src/ui/shells/library/track_detail_metadata.rs",
    ));
    let track_detail_source = read_source(&manifest_path("src/ui/shells/library/track_detail.rs"));
    let feed_detail_source = read_source(&manifest_path("src/ui/shells/library/feed_detail.rs"));
    let disclosure_source = read_source(&manifest_path("src/ui/composites/disclosure_group.rs"));
    let composite_mod_source = read_source(&manifest_path("src/ui/composites/mod.rs"));
    let mut violations = Vec::new();

    for required in [
        "inspector_state: LibraryTrackInspectorState",
        "fn inspector_display(",
        "fn toggle_inspector_panel(&mut self, kind: InspectorPanelKind) -> bool",
        "fn toggle_description(&mut self)",
    ] {
        if !library_struct_source.contains(required) {
            violations.push(format!(
                "src/library.rs: ADR 0047 Phase C inspector frame contract missing `{required}`"
            ));
        }
    }

    for required in [
        "inspector_state: LibraryTrackInspectorState::default()",
        "toggle_inspector_panel(InspectorPanelKind::CompareId3)",
        "toggle_inspector_panel(InspectorPanelKind::MusicBrainz)",
        "fn toggle_track_description(",
        "fn toggle_album_description(",
    ] {
        if !library_app_source.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0047 Phase C app wiring missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "if self.context != EntitySurfaceContext::Library || !self.has_local_file {\n            return None;",
        "if !self.has_local_file {\n            return None;",
    ] {
        if metadata_vm_source.contains(forbidden) {
            violations.push(
                "src/view_models/entity_detail.rs: ADR 0047 Phase C metadata actions must stay visible and disabled when unavailable"
                    .to_string(),
            );
        }
    }

    for required in [
        "pub(crate) const DOWNLOAD_REQUIRED_METADATA_TOOLTIP",
        "pub(crate) fn show_compare_id3_panel",
        "pub(crate) fn show_musicbrainz_panel",
        "pub(crate) const fn compare_id3_tooltip_text",
        "pub(crate) const fn musicbrainz_tooltip_text",
        "pub(crate) fn display_description_text",
        "track_description_states: BTreeMap<i64, DescriptionState>",
        "pub(crate) fn track_description_state",
        "pub(crate) fn set_track_description_state",
        "pub(crate) fn toggle_album_description",
    ] {
        if !library_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: ADR 0047 Phase C library VM projection missing `{required}`"
            ));
        }
    }

    for required in [
        "frame.inspector_display",
        "show_compare_id3_panel()",
        "show_musicbrainz_panel()",
        "compare_id3_tooltip_text()",
        "musicbrainz_tooltip_text()",
        "if disabled {",
        ".on_click(cx.listener(|this, _, _, cx| {",
    ] {
        if !metadata_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/library/track_detail_metadata.rs: ADR 0047 Phase C metadata shell missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "let show_id3_panel = metadata_state.show_compare_panel()",
        "let show_musicbrainz_panel = metadata_state.show_musicbrainz_panel()",
    ] {
        if metadata_shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/library/track_detail_metadata.rs: ADR 0047 Phase C panel visibility must come from inspector display, found `{forbidden}`"
            ));
        }
    }

    for required in [
        "pub struct DisclosureTextPanel",
        "pub struct DisclosureTextPanelDisplay",
        "DisclosureGroup::new",
        "MultilineText::new",
    ] {
        if !disclosure_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/disclosure_group.rs: ADR 0047 Phase C shared disclosure panel missing `{required}`"
            ));
        }
    }

    if !composite_mod_source.contains("DisclosureTextPanel")
        || !composite_mod_source.contains("DisclosureTextPanelDisplay")
    {
        violations.push(
            "src/ui/composites/mod.rs: ADR 0047 Phase C disclosure text panel is not exported"
                .to_string(),
        );
    }

    for required in [
        "DisclosureTextPanel",
        "description_state.is_visible()",
        "LibraryViewModel::display_description_text",
        "toggle_track_description",
    ] {
        if !track_detail_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/library/track_detail.rs: ADR 0047 Phase C track description disclosure missing `{required}`"
            ));
        }
    }

    for required in [
        "DisclosureTextPanel",
        "LibraryViewModel::display_description_text",
        "album_description_state",
        "toggle_album_description",
    ] {
        if !feed_detail_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/library/feed_detail.rs: ADR 0047 Phase C feed description disclosure missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Phase C inspector rewire violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_description_rendering_must_not_infer_placeholder_metadata() {
    let checked_files = [
        "src/view_models/library.rs",
        "src/ui/shells/library/feed_detail.rs",
        "src/ui/shells/library/track_detail.rs",
        "src/ui/composites/disclosure_group.rs",
        "src/ui/composites/track_detail_surface.rs",
        "src/ui/shells/entity.rs",
        "src/ui/shells/track.rs",
    ];
    let forbidden_patterns = [
        ".trim_matches(|ch| ch == '.'",
        ".trim_matches(|ch| ch == '\\u{2026}'",
        ".all(|ch| ch.is_whitespace() || ch == '.'",
        "description.contains(\"...\")",
        "description == \"...\"",
        "value == \"...\"",
        "placeholder-only",
        "placeholder description",
    ];
    let mut violations = Vec::new();

    for path in checked_files {
        let source = read_source(&manifest_path(path));
        for (line_number, line) in code_lines(&source) {
            for forbidden in forbidden_patterns {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{path}:{line_number}: ADR 0047 forbids renderer/view-model placeholder inference for descriptions; fix source hydration instead of matching `{forbidden}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 prohibited description placeholder inference:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_feed_description_panels_render_wrapped_body_text() {
    let release_shell = read_source(&manifest_path("src/ui/shells/entity.rs"));
    let disclosure_panel = read_source(&manifest_path("src/ui/composites/disclosure_group.rs"));
    let mut violations = Vec::new();

    for (path, source) in [
        ("src/ui/shells/entity.rs", release_shell),
        ("src/ui/composites/disclosure_group.rs", disclosure_panel),
    ] {
        if !source.contains(".wrap_lines()") {
            violations.push(format!(
                "{path}: ADR 0047 feed descriptions must wrap body text instead of single-line truncating source lines"
            ));
        }
        if source.contains("MultilineText::new(display.body)\n                    .max_lines(3)") {
            violations.push(format!(
                "{path}: ADR 0047 feed descriptions must not cap release body text to three metadata-style lines"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 feed description panel regressions:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_multiline_text_wrap_policy_does_not_collapse_metadata_grid() {
    let source = read_source(&manifest_path("src/ui/primitives/multiline_text.rs"));
    let mut violations = Vec::new();

    for required in [
        "let policy = layout_policy(self.wrap_lines);",
        "if policy.container_min_w_zero() {",
        "if policy.line_min_w_zero() {",
        "MultilineTextLayoutPolicy::Wrap",
        "MultilineTextLayoutPolicy::Truncate",
        "truncate_branch_keeps_intrinsic_line_width_for_metadata_grid",
        "wrap_branch_allows_flex_shrink_for_description_text",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "src/ui/primitives/multiline_text.rs: ADR 0047 multiline text policy missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "div().flex().flex_col().min_w_0().text_size",
        "div().min_w_0().child(SharedString::from(line))",
        "let mut line_el = div().min_w_0()",
    ] {
        if source.contains(forbidden) {
            violations.push(format!(
                "src/ui/primitives/multiline_text.rs: ADR 0047 truncate-mode metadata text must not inherit wrap-mode flex shrink from `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 multiline text layout regressions:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_library_album_hydration_updates_feed_description_source_fact() {
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let library_query_source = read_source(&manifest_path("src/application/queries/library.rs"));
    let mut violations = Vec::new();

    for required in [
        "source_release_claims",
        "FeedView::from_api(feed.clone()).description",
        "db::set_feed_description",
    ] {
        if !library_query_source.contains(required) {
            violations.push(format!(
                "src/application/queries/library.rs: ADR 0047 library album hydration must preserve feed description source data via `{required}`"
            ));
        }
    }

    for required in ["update_album_description"] {
        if !library_app_source.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0047 library album hydration must preserve feed description source data via `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 feed description hydration regressions:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_frame_shell_composite_owns_shared_frame_chrome() {
    let source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let mod_source = read_source(&manifest_path("src/ui/composites/mod.rs"));
    let icon_source = read_source(&manifest_path("src/ui/icons.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) struct FrameShellSlots",
        "pub(crate) struct FrameShell",
        "pub(crate) fn frame_shell(",
        "FrameShellDisplay",
        "FrameChromeButtonDisplay",
        "FrameChromeMenuItemDisplay",
        "ContextMenuScope::WorkspaceFrame",
        "IconName::ChevronLeft",
        "IconName::ChevronRight",
        "IconName::Close",
        "content.into_any_element()",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: ADR 0046 Task 006 frame-shell composite missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "gpui::rgb(",
        "gpui::px(",
        ".absolute()",
        ".fixed()",
        ".z_index(",
    ] {
        if source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: ADR 0046 Task 006 frame shell must not use `{forbidden}`"
            ));
        }
    }

    for forbidden_screen in ["crate::library", "crate::search", "crate::app", "crate::db"] {
        if source.contains(forbidden_screen) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: ADR 0046 Task 006 frame shell must not import `{forbidden_screen}`"
            ));
        }
    }

    for required in [
        "pub mod frame_shell;",
        "pub(crate) use frame_shell::{frame_shell, FrameShell, FrameShellSlots};",
    ] {
        if !mod_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/mod.rs: ADR 0046 Task 006 composite export missing `{required}`"
            ));
        }
    }

    for required in ["ChevronLeft", "ChevronRight", "Close"] {
        if !icon_source.contains(required) {
            violations.push(format!(
                "src/ui/icons.rs: ADR 0046 Task 006 icon catalog missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Task 006 frame-shell composite violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_screen_mount_boundary_wraps_existing_screens_whole() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let mut violations = Vec::new();

    for required in [
        "enum WorkspaceScreenMount",
        "fn active_workspace_screen_mount(&self) -> WorkspaceScreenMount",
        "fn render_workspace_screen_mount(",
        "WorkspaceScreenMount::Music => self.library.clone().into_any_element()",
        "build_show_screen(self, show_window_width, cx).into_any_element()",
        "WorkspaceScreenMount::Settings => render_settings(self, cx)",
        "workspace render delegates to the active app-section mount",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0046 Task 006a mount boundary missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "render_library_sidebar",
        "render_search_results(",
        "WorkspaceScreenMount::Search",
        "WorkspaceScreenMount::SourceList",
        "WorkspaceScreenMount::ContentList",
        "WorkspaceScreenMount::Detail",
    ] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0046 Task 006a must wrap whole screens and not split panes; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Task 006a screen-mount boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_layout_render_uses_frame_shell_without_screen_internals() {
    let source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let layout_source = read_source(&manifest_path("src/ui/layouts.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let frame_shell_source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let mod_source = read_source(&manifest_path("src/ui/shells/mod.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) struct WorkspaceSlots",
        "pub(crate) fn render_workspace(",
        "WorkspaceLayout",
        "WorkspaceFrameKind",
        "frame_shell(",
        "FrameShellSlots::new().content(content)",
        "QueueNowPlaying",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0046 Task 007 workspace shell missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "crate::library",
        "crate::search",
        "crate::app",
        "crate::db",
        "PlaybackOwner",
        "render_library_sidebar",
        "render_search_results(",
    ] {
        if source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0046 Task 007 shell must not import or duplicate `{forbidden}`"
            ));
        }
    }

    for required in [
        "fn render_workspace_content(",
        "fn transitional_workspace_layout(",
        "WorkspaceSlots::new()",
        "match &current_nav",
        "FrameNavigationEntry::Settings",
        "WorkspaceFrameKind::QueueNowPlaying",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0046 Task 007 app workspace wiring missing `{required}`"
            ));
        }
    }

    for forbidden in ["WORKSPACE_RENDER_ENABLED", "render_legacy_tab_content("] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0047 Task 016 retired the workspace fallback; found `{forbidden}`"
            ));
        }
    }

    if app_source.contains("WorkspaceLayout::default_layout()") {
        violations.push(
            "src/app.rs: ADR 0046 Task 007 must not render the full default layout before SourceList/Detail are extracted"
                .to_string(),
        );
    }

    for required in [
        ".key_context(keyboard::ACTIVE_PANE_KEY_CONTEXT)",
        ".flex()\n                    .flex_col()\n                    .flex_1()",
        ".min_w_0()\n                    .overflow_hidden()",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0046 workspace mount must preserve bounded flex scroll chain; missing `{required}`"
            ));
        }
    }

    for required in [
        ".size_full()",
        ".flex_row()",
        ".min_h_0()",
        ".overflow_hidden()",
        "WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT",
        "WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT",
        "fn should_collapse_frame(",
        "WorkspaceFrameKind::QueueNowPlaying =>",
        "WorkspaceFrameKind::Detail =>",
        "WorkspaceFrameKind::SourceList | WorkspaceFrameKind::ContentList => false",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0046 workspace shell must preserve scrollable child bounds; missing `{required}`"
            ));
        }
    }

    for required in [
        "pub const WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT",
        "pub const WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT",
    ] {
        if !layout_source.contains(required) {
            violations.push(format!(
                "src/ui/layouts.rs: ADR 0046 Task 008 workspace collapse contract missing `{required}`"
            ));
        }
    }

    for required in [
        ".size_full()",
        ".flex()\n                    .flex_col()\n                    .flex_1()",
        ".min_w_0()\n                    .overflow_hidden()",
    ] {
        if !frame_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: FrameShell must preserve bounded scrollable content slot; missing `{required}`"
            ));
        }
    }

    if !mod_source.contains("pub mod workspace;") {
        violations.push(
            "src/ui/shells/mod.rs: ADR 0046 Task 007 workspace shell module is not exported"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Task 007 workspace layout render violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_split_pane_uses_fluid_resize_pattern() {
    let source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let resize_source = read_source(&manifest_path("src/app/resize.rs"));
    let layout_source = read_source(&manifest_path("src/ui/layouts.rs"));
    let mut violations = Vec::new();

    for required in [".on_resize_start(", ".on_resize_move(", ".on_resize_end("] {
        if !source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: P2b fluid resize pattern missing `{required}`"
            ));
        }
    }

    for required in [
        "fn set_content_pane_width(&mut self",
        "fn begin_content_pane_resize(&mut self",
        "fn resize_content_pane(&mut self",
        "fn end_content_pane_resize(&mut self",
        "fn is_content_pane_resizing(&self) -> bool",
    ] {
        if !resize_source.contains(required) {
            violations.push(format!(
                "src/app/resize.rs: P2b fluid resize TopApp methods missing `{required}`"
            ));
        }
    }

    if !app_source.contains("is_content_pane_resizing: bool") {
        violations.push(
            "src/app.rs: P2b TopApp struct missing `is_content_pane_resizing: bool` field"
                .to_string(),
        );
    }

    if !app_source.contains(".on_content_pane_resize_start(") {
        violations.push(
            "src/app.rs: P2b TopApp render_workspace_content must wire .on_content_pane_resize_start"
                .to_string(),
        );
    }

    if !app_source.contains(".on_content_pane_resize_move(") {
        violations.push(
            "src/app.rs: P2b TopApp render_workspace_content must wire .on_content_pane_resize_move"
                .to_string(),
        );
    }

    if !layout_source.contains("pub const CONTENT_PANE_MAX_WIDTH") {
        violations
            .push("src/ui/layouts.rs: P2b missing `pub const CONTENT_PANE_MAX_WIDTH`".to_string());
    }

    assert!(
        violations.is_empty(),
        "P2b workspace fluid split-pane resize pattern violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0066_missing_runtime_has_no_implicit_runner() {
    // Situational: ADR 0066 invariants 2, 5 and 9. Runner/worker tests prove
    // rejection and recovery; this inventory keeps every GUI entry on that path.
    for file in [
        "src/app.rs",
        "src/library/app_impl.rs",
        "src/discover/app_impl.rs",
    ] {
        let source = read_source(&manifest_path(file));
        let source = production_source(&source);
        assert!(
            source.contains("None => AsyncCommandRunner::unavailable("),
            "{file}: missing explicit unavailable runner"
        );
        assert!(source.contains("ExecutionUnavailable::RUNTIME"));
        for forbidden in [
            "AsyncCommandRunner::new(",
            "AsyncCommandRunner::with_vm_bus(",
            "Handle::current(",
            "Runtime::new(",
        ] {
            assert!(
                !source.contains(forbidden),
                "{file}: implicit runtime path {forbidden}"
            );
        }
    }
    let runner = read_source(&manifest_path("src/application/async_command_runner.rs"));
    let dispatch = source_between(
        &runner,
        "pub fn dispatch<C>",
        "fn publish_vm_invalidations(",
    );
    assert!(
        dispatch.find("CommandError::Unavailable").unwrap()
            < dispatch.find("command_bus.execute").unwrap()
    );
    assert!(dispatch.contains("return rx;"));
    let presenter = read_source(&manifest_path(
        "src/presentation/async_command_presenter.rs",
    ));
    assert!(presenter.contains("runner.dispatch(command, context)"));
    for file in [
        "src/app/playback_bar.rs",
        "src/app/search_dispatch.rs",
        "src/app/show.rs",
        "src/library/app_impl.rs",
        "src/discover/app_impl.rs",
    ] {
        let source = read_source(&manifest_path(file));
        let source = production_source(&source);
        assert!(
            source.contains("present_command("),
            "{file}: commands must use the rejecting presenter"
        );
        assert!(!source.contains("command_bus.execute("));
    }
    let keys = read_source(&manifest_path("src/app/keyboard.rs"));
    for route in [
        "self.toggle_playback_paused(cx)",
        "self.skip_playback_next(cx)",
        "self.focus_global_search(window, cx)",
        "self.select_tab(AppTab::Settings, window, cx)",
    ] {
        assert!(keys.contains(route));
    }
    let search = read_source(&manifest_path("src/app/search_dispatch.rs"));
    assert!(search.contains("search_local_library_tracks(&conn, query, None)"));
    assert!(search.contains("self.command_runner.availability().ok()?"));
    let library = read_source(&manifest_path("src/library/app_impl.rs"));
    assert!(library.contains("self.command_runner.availability().ok()?"));
    assert!(library.contains("ExecutionUnavailable::RUNTIME.to_string()"));
}

#[test]
fn adr_0066_runtime_retry_keeps_one_host_and_independent_reports() {
    // Situational: ADR 0066 invariants 2, 5, 7 and 9. VM generations and
    // maintenance-worker tests prove ordering; these are the actual UI callers.
    let source = read_source(&manifest_path("src/app/capabilities.rs"));
    for required in [
        "worker.submit(",
        "present_startup(",
        "self.capability_vm.complete(dependency, generation)",
        "self.runtime_host.is_some()",
        "library.install_runtime(host, cx)",
        "self.capability_vm.report()",
        "bridge_watch(",
    ] {
        assert!(
            source.contains(required),
            "missing retry boundary {required}"
        );
    }
    assert!(
        source
            .find("self.capability_vm.complete(dependency, generation)")
            .unwrap()
            < source.find("self.install_runtime(host, cx)").unwrap()
    );
    let install = source_between(&source, "fn install_runtime(", "\n}");
    for call in [
        "library.install_runtime(",
        "self.maybe_start_broadcast_readiness_watch(",
        "self.maybe_start_broadcast_service_watch(",
    ] {
        assert_eq!(install.matches(call).count(), 1);
    }
    assert!(!install.contains("install_capability_controls("));
    let app = read_source(&manifest_path("src/app.rs"));
    assert!(app.contains("self.playback_polling.is_some()"));
    assert!(app.contains("self.render_capabilities(false, cx)"));
    assert!(app.contains("with_execution_availability(command_runner.availability())"));
    assert!(app.contains("app.render_capabilities(true, cx)"));
    let library = read_source(&manifest_path("src/library/app_impl.rs"));
    for required in [
        "self.runtime_host.is_some()",
        "self.musicbrainz_feed_saga.is_some()",
        "self.start_async_reload_preserving_detail(cx)",
    ] {
        assert!(library.contains(required));
    }
    let show = read_source(&manifest_path("src/app/show.rs"));
    assert!(show.contains("self.broadcast_readiness_watch.is_some()"));
    assert!(show.contains("self.publisher_service_watch.is_some()"));
    assert!(show.contains("with_execution_availability(self.command_runner.availability())"));
    let cache = read_source(&manifest_path("src/media/image_cache.rs"));
    assert!(cache.contains("std::thread::Builder::new()"));
    assert!(!production_source(&cache).contains("std::thread::spawn("));
    assert!(cache.contains("CapabilityFailure::CacheWorker"));
    assert!(cache.contains("CapabilityFailure::CachePrune"));
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    assert!(bootstrap.contains("let runtime_host = runtime.ok()"));
    assert!(bootstrap.contains("ImageCache::new_observed("));
    let vm = read_source(&manifest_path("src/view_models/startup/capabilities.rs"));
    assert!(!production_source(&vm).contains("SystemTime::now"));
    assert!(!vm.contains("use gpui"));
    let host = read_source(&manifest_path("src/presentation/runtime_host.rs"));
    assert!(host.contains("#[cfg(debug_assertions)]"));
    let fixture = read_source(&manifest_path("src/startup/fixture.rs"));
    for required in [
        "V4VMM_STARTUP_FIXTURE",
        "Fixture config mismatch",
        "Fixture binary mismatch",
    ] {
        assert!(fixture.contains(required));
    }
}

#[test]
fn adr_0066_core_recovery_ownership() {
    // Situational: ADR 0066 invariants 1, 5 and 8. Behavioral startup,
    // worker and presentation tests prove failures and generation admission.
    let backend = read_source(&manifest_path("src/startup.rs"));
    let backend = production_source(&backend);
    for forbidden in ["use gpui", "TopApp", "RuntimeHost", "save_workspace_layout"] {
        assert!(!backend.contains(forbidden));
    }
    let database = read_source(&manifest_path("src/db/startup.rs"));
    let checks = source_between(&database, "fn check_with_wait(", "pub fn prepare_database(");
    for forbidden in [
        "init_schema(",
        "migrate_schema(",
        "repair_local_file_paths(",
        "create_dir",
        "open_db(",
    ] {
        assert!(
            !checks.contains(forbidden),
            "check-only path calls {forbidden}"
        );
    }
    let worker = read_source(&manifest_path("src/presentation/maintenance_executor.rs"));
    for required in [
        "thread::Builder",
        "mpsc::sync_channel(1)",
        "AdmissionError::Busy",
        "handle.join()",
    ] {
        assert!(worker.contains(required));
    }
    for forbidden in ["RuntimeHost", "Handle::current", "Connection"] {
        assert!(!production_source(&worker).contains(forbidden));
    }
    let screen = read_source(&manifest_path("src/app/startup.rs"));
    for required in [
        "worker.submit(",
        "CoreResult::Checked(outcome) => Err(outcome)",
        "CoreResult::Prepared(core)",
        "mount_current(",
        "startup_report(&self.vm",
    ] {
        assert!(
            compact_source(&screen).contains(&compact_source(required)),
            "missing startup routing: {required}"
        );
    }
    for forbidden in [
        "Connection::open",
        "save_workspace_layout",
        "cx.spawn(",
        "fs::",
    ] {
        assert!(!screen.contains(forbidden));
    }
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    let entry = source_between(
        &bootstrap,
        "pub fn run_app(",
        "pub(super) struct NormalStartup",
    );
    assert_eq!(entry.matches(".open_window(").count(), 1);
    for forbidden in [
        "TopApp::new",
        "load_config(",
        "db::open_db(",
        "ensure_dirs(",
        "repair_local_file_paths(",
    ] {
        assert!(!entry.contains(forbidden));
    }
    assert!(entry.contains("worker.finish()"));
    let main = read_source(&manifest_path("src/main.rs"));
    assert!(main.contains("ExitCode::FAILURE"));
    assert!(backend.contains("#[cfg(debug_assertions)]\npub mod fixture;"));
}

#[test]
fn adr_0066_window_manager_close_queues_quit() {
    // Situational: ADR 0066 startup lifecycle. A window-manager close must not
    // re-enter the platform while its close callback holds the X11 client.
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    let callback = source_between(
        &bootstrap,
        "window.on_window_should_close(",
        "let view = cx.new(",
    );
    assert!(callback.contains("quit_after_window_close(cx)"));
    assert!(callback.contains("true"));
    for forbidden in ["cx.quit(", "cx.defer(", "remove_window("] {
        assert!(!callback.contains(forbidden));
    }
    let presenter = read_source(&manifest_path("src/presentation/startup_presenter.rs"));
    let queued = compact_source(source_between(
        &presenter,
        "pub(crate) fn quit_after_window_close(",
        "pub(crate) fn present_startup<",
    ));
    assert!(queued.contains(&compact_source("cx.spawn(async move |cx|")));
    assert!(queued.contains(&compact_source("cx.update(|cx| cx.quit())")));
    assert_eq!(queued.matches("cx.quit(").count(), 1);
    assert!(queued.contains(".detach()"));
    for forbidden in ["cx.defer(", "thread::spawn", "sleep(", "timer("] {
        assert!(!queued.contains(forbidden));
    }
    assert!(bootstrap.contains("worker.finish()"));
    assert!(bootstrap.contains("opened.load(Ordering::Acquire)"));
}

#[test]
fn adr_0066_recorded_report_context() {
    // Situational: ADR 0066 invariant 7. Formatting/redaction tests prove the
    // data contract; this guard keeps display and copy on that same owner.
    let vm = read_source(&manifest_path("src/view_models/startup.rs"));
    let formatter = source_between(&vm, "pub fn format_report(", "fn subject(");
    for required in ["observed_at", "chrono::Utc", "Location:", "Next:"] {
        assert!(formatter.contains(required));
    }
    assert!(!formatter.contains("SystemTime::now"));
    let screen = read_source(&manifest_path("src/app/startup.rs"));
    assert!(screen.contains("ClipboardItem::new_string(self.vm.report())"));
    assert!(screen.contains("this.vm.report()"));
    let composite = read_source(&manifest_path("src/ui/composites/startup_report.rs"));
    // ADR 0066 operator feedback: a fast identical failure needs a persistent
    // receipt outside disclosure and the scroll body, using recorded VM state.
    let heading = source_between(&composite, "let mut heading", ".child(heading)");
    assert!(heading.contains("vm.feedback()"));
    assert!(heading.contains("startup-check-feedback"));
    assert!(heading.contains("flex_shrink_0()"));
    assert!(!heading.contains("if vm.details"));
    let feedback = source_between(&vm, "pub fn feedback(", "pub fn action(");
    assert!(feedback.contains("completed_at"));
    assert!(!feedback.contains("SystemTime::now"));
    for required in [
        "vm.report()",
        "vm.action(",
        "a11y_label",
        "whitespace_normal()",
        "overflow_y_scrollbar()",
        "flex_wrap()",
    ] {
        assert!(composite.contains(required));
    }
    for forbidden in ["truncate()", "SystemTime::now", "std::fs", "rusqlite"] {
        assert!(!composite.contains(forbidden));
    }
}

#[test]
fn adr_0066_search_failure_report_stays_readable_and_vm_owned() {
    // Situational: ADR 0066 operator correction during task 003. The reported
    // clipping crossed ADR 0063's column-text rule; keep root/scoped results
    // on one wrapping owner with VM disclosure and exact clipboard delivery.
    let shell = read_source(&manifest_path("src/ui/shells/search_results_inspector.rs"));
    let notice = source_between(
        &shell,
        "fn render_empty_state(",
        "const fn entity_kind_for_index_detail",
    );
    for required in [
        "Size::NoticeWidth",
        "min_w_0()",
        "wrap_lines()",
        "overflow_y_scrollbar()",
        "flex_wrap()",
        "failure.action(intent)",
        "failure.visible_report()",
        "display.availability",
        "display.a11y_label",
    ] {
        assert!(
            notice.contains(required),
            "search failure presentation missing {required}"
        );
    }
    for forbidden in [
        ".truncated()",
        ".truncate()",
        "SystemTime::now",
        "error.to_string()",
    ] {
        assert!(
            !notice.contains(forbidden),
            "search failure presentation must not use {forbidden}"
        );
    }
    let dispatch = read_source(&manifest_path("src/app/search_dispatch.rs"));
    assert!(dispatch
        .contains("detail.set_index_error(&error, &error_endpoint, std::time::SystemTime::now())"));
    assert!(dispatch.contains("detail.activate_failure_action(action)"));
    assert!(dispatch.contains("ClipboardItem::new_string(report)"));
    assert!(!dispatch.contains("fn command_error_detail("));
    let app = read_source(&manifest_path("src/app.rs"));
    assert_eq!(
        app.matches(".on_failure_action(").count(),
        2,
        "root and scoped search must wire diagnostic actions"
    );
    let failure = read_source(&manifest_path("src/view_models/search_results/failure.rs"));
    let failure = production_source(&failure);
    assert!(failure.contains("redact_endpoint_details"));
    assert!(!failure.contains("SystemTime::now"));
    let startup = read_source(&manifest_path("src/startup.rs"));
    assert!(startup.contains("crate::diagnostics::redact_endpoint_details"));
}

#[test]
fn adr_0066_config_creation_and_save_ownership() {
    // Situational: ADR 0066 invariants 3–4. Filesystem/snapshot behavior is
    // exercised by config::tests::adr_0066_*; this guard owns caller routing.
    let config_file = read_source(&manifest_path("src/config.rs"));
    let config = production_source(&config_file);
    assert_adr_0066_first_run_owner(config);

    let readers = source_between(
        config,
        "pub fn load_musicindex_endpoint(",
        "fn read_config_for_save(",
    );
    assert_eq!(
        readers.matches("load_config_snapshot(cfg_path)?").count(),
        1
    );
    let save_guard = source_between(
        config,
        "fn read_config_for_save(",
        "fn write_existing_config(",
    );
    assert!(save_guard.contains("ConfigSnapshot::read_existing(cfg_path)?"));
    assert!(save_guard.contains("snapshot.require_saveable()?"));
    assert!(!save_guard.contains("load_config_snapshot"));
    let writer = source_between(
        config,
        "fn write_existing_config(",
        "pub fn save_app_settings(",
    );
    assert!(writer.contains(".open(cfg_path)"));
    assert!(!writer.contains(".create("));
    assert!(!writer.contains(".create_new("));

    for (start, end) in [
        (
            "pub fn save_app_settings(",
            "pub(crate) fn save_workspace_layout(",
        ),
        (
            "pub(crate) fn save_workspace_layout(",
            "pub(crate) fn save_workspace_layout_prefs(",
        ),
        (
            "pub(crate) fn save_workspace_layout_prefs(",
            "pub fn normalize_musicindex_endpoint(",
        ),
    ] {
        let save = source_between(config, start, end);
        assert!(
            save.contains("read_config_for_save(cfg_path)?"),
            "ADR 0066 unguarded {start}"
        );
        assert!(save.contains("write_existing_config(cfg_path, &table)"));
        for forbidden in [
            "default_config_toml",
            "load_config(",
            "load_config_snapshot(",
            "fs::write",
            ".create(",
            ".exists()",
        ] {
            assert!(
                !save.contains(forbidden),
                "ADR 0066 {start} bypasses save ownership with {forbidden}"
            );
        }
    }

    let app = read_source(&manifest_path("src/app.rs"));
    let save = source_between(&app, "fn save_settings(", "fn reload_cached(");
    assert!(save.contains("config::save_app_settings("));
    // Situational ADR 0066: download-only preparation cannot suppress already saved settings.
    let applied = source_between(
        save,
        "let download_preparation =",
        "self.settings_status = match download_preparation",
    );
    assert!(applied.contains("self.musicindex_endpoint ="));
    assert!(applied.contains("playback_owner.set_music_dir("));
    assert!(!applied.contains("return;"));
    // save_app_settings owns the fresh snapshot and rejects invalid siblings.
    assert!(!save.contains("require_saveable().is_ok()"));
    assert!(!save.contains("config::load_config("));
    let failure = save.rsplit_once("Err(error) => {").unwrap().1;
    assert!(!failure.contains("persist_workspace_layout"));
    let layout = source_between(
        &app,
        "fn persist_workspace_layout(",
        "fn initial_workspace_layout(",
    );
    assert!(layout.contains("config::save_workspace_layout("));
    let resize = read_source(&manifest_path("src/app/resize.rs"));
    assert!(resize.contains("config::save_workspace_layout_prefs("));
}

// Situational helper for ADR 0066 invariants 3–4.
fn assert_adr_0066_first_run_owner(config: &str) {
    let path = source_between(
        config,
        "pub fn config_path(",
        "pub fn load_config_snapshot(",
    );
    assert!(!path.contains("create_dir"));

    let first_run = source_between(
        config,
        "fn load_snapshot_with_defaults(",
        "static DEFAULT_TEMP_SEQUENCE",
    );
    for required in [
        "fs::symlink_metadata",
        "io::ErrorKind::NotFound",
        "require_saveable()?",
        "publish_default_config",
        "eprintln!",
        "ConfigSnapshot::read_existing",
    ] {
        assert!(
            first_run.contains(required),
            "ADR 0066 first-run owner missing {required}"
        );
    }
    assert!(!first_run.contains(".exists()"));
    assert!(!first_run
        .lines()
        .any(|line| line.trim_start().starts_with("println!")));
    let publication = source_between(
        config,
        "fn default_config_temporary(",
        "pub fn load_musicindex_endpoint(",
    );
    for required in [
        "create_new(true)",
        "file.sync_all()",
        "fs::hard_link",
        "io::ErrorKind::AlreadyExists",
        "fs::remove_file",
    ] {
        assert!(
            publication.contains(required),
            "ADR 0066 safe publication missing {required}"
        );
    }
}

#[test]
fn workspace_frame_phase_5_layout_persistence_contract() {
    let workspace_source = workspace_vm_source();
    let config_source = read_source(&manifest_path("src/config.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let bootstrap_source = read_source(&manifest_path("src/app/bootstrap.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) struct WorkspaceLayoutConfig",
        "pub(crate) struct WorkspaceFrameConfig",
        "#[serde(rename_all = \"snake_case\")]",
        "LastFrameRemoval",
        "pub(crate) fn add_frame(",
        "kind: WorkspaceFrameKind",
        "Result<WorkspaceFrameId, WorkspaceModelError>",
        "pub(crate) fn remove_frame(&mut self, id: WorkspaceFrameId) -> Result<(), WorkspaceModelError>",
        "pub(crate) fn to_config(&self) -> WorkspaceLayoutConfig",
        "pub(crate) fn from_config(config: Option<&WorkspaceLayoutConfig>) -> Self",
    ] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0046 Task 012 layout persistence contract missing `{required}`"
            ));
        }
    }

    for required in [
        "WorkspaceLayoutConfig",
        "workspace_layout: ConfigField<Option<WorkspaceLayoutConfig>>",
        "workspace_layout: decode_field(",
        "pub(crate) fn save_workspace_layout(",
        "toml::Value::try_from(workspace_layout)",
        "self.workspace_layout",
    ] {
        if !config_source.contains(required) {
            violations.push(format!(
                "src/config.rs: ADR 0046 Task 012 config persistence contract missing `{required}`"
            ));
        }
    }

    for required in [
        "workspace_layout: WorkspaceLayout",
        "WorkspaceLayout::from_config(config)",
        "initial_workspace_layout",
        "fn persist_workspace_layout(&self)",
        "&self.workspace_layout.to_config()",
        "impl Drop for TopApp",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0046 Task 012 app persistence wiring missing `{required}`"
            ));
        }
    }

    if !bootstrap_source.contains("cfg.workspace_layout") {
        violations.push(
            "src/app/bootstrap.rs: ADR 0046 Task 012 startup must pass persisted workspace_layout into TopApp"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Task 012 layout persistence violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_pane_width_persistence_contract() {
    let config_source = read_source(&manifest_path("src/config.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let bootstrap_source = read_source(&manifest_path("src/app/bootstrap.rs"));
    let resize_source = read_source(&manifest_path("src/app/resize.rs"));
    let mut violations = Vec::new();

    for required in [
        "WorkspaceConfig",
        "WorkspaceLayoutPrefs",
        "fn workspace_preferences(&self)",
        "content_pane_width: snapshot_width(layout)",
        "save_workspace_layout_prefs",
        "self.content_pane_width.unwrap_or_default()",
    ] {
        if !config_source.contains(required) {
            violations.push(format!(
                "src/config.rs: ADR 0051 pane width persistence missing `{required}`"
            ));
        }
    }

    for required in ["Self::initial_content_pane_width(workspace_layout_prefs)"] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0051 pane width persistence wiring missing `{required}`"
            ));
        }
    }

    for required in [
        "initial_content_pane_width",
        "persist_content_pane_width",
        "clamped_content_pane_width",
        "config::save_workspace_layout_prefs",
    ] {
        if !resize_source.contains(required) {
            violations.push(format!(
                "src/app/resize.rs: ADR 0051 pane width resize owner missing `{required}`"
            ));
        }
    }

    for required in [
        ".workspace",
        "workspace.layout",
        "workspace_layout_prefs.as_ref()",
    ] {
        if !bootstrap_source.contains(required) {
            violations.push(format!(
                "src/app/bootstrap.rs: ADR 0051 startup must pass workspace.layout prefs into TopApp; missing `{required}`"
            ));
        }
    }

    if !resize_source.contains("persist_content_pane_width") {
        violations
            .push("src/app/resize.rs: ADR 0051 resize end must persist the pane width".to_string());
    }

    if resize_source.matches("persist_content_pane_width").count() != 2 {
        violations.push(
            "src/app/resize.rs: ADR 0051 pane width persistence must happen once in end_content_pane_resize"
                .to_string(),
        );
    }

    if resize_source.contains("resize_content_pane(&mut self, x: f32, cx: &mut Context<Self>) {")
        && resize_source.contains("persist_content_pane_width")
    {
        let move_fn = resize_source
            .split("pub(super) fn resize_content_pane")
            .nth(1)
            .unwrap_or("");
        let resize_body = move_fn
            .split("pub(super) fn end_content_pane_resize")
            .next()
            .unwrap_or("");
        if resize_body.contains("persist_content_pane_width") {
            violations.push(
                "src/app/resize.rs: ADR 0051 resize_content_pane must not persist config"
                    .to_string(),
            );
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0051 pane width persistence violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_frame_phase_5_multi_frame_commands_are_deferred_until_content_frames_exist() {
    let workspace_vm_source = workspace_vm_source();
    let frame_shell_source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let workspace_shell_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let keyboard_source = read_source(&manifest_path("src/app/keyboard.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let mut violations = Vec::new();

    for required in [
        "action_menu_items: Vec<FrameChromeMenuItemDisplay>",
        "action_menu_items: Vec::new()",
    ] {
        if !workspace_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0046 Task 013 deferred frame action contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "OPEN_NEW_FRAME_MENU_ID",
        "CLOSE_FRAME_MENU_ID",
        "workspace-frame-open-new-frame",
        "workspace-frame-close-frame",
        "Open New Frame",
        "Close Frame",
        "frame_action_menu_items",
    ] {
        if workspace_vm_source.contains(forbidden) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0046 Task 013 must not expose fake multi-frame action `{forbidden}` before real frame content exists"
            ));
        }
    }

    for required in [
        "ContextMenuScope::WorkspaceFrame",
        "Frame actions",
        "on_menu_select",
    ] {
        if !frame_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: ADR 0046 Task 013 frame shell must keep shared context-menu routing; missing `{required}`"
            ));
        }
    }

    for required in ["frame.is_focused()", "SemanticColor::Focus"] {
        if !workspace_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0046 Task 013 focus wiring missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "crate::library::",
        "crate::search::",
        "crate::db",
        "crate::playback",
        "on_open_new_frame",
        "on_close_frame",
    ] {
        if workspace_shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0046 Task 013 workspace shell must not dispatch unavailable multi-frame actions or screen/backend state `{forbidden}`"
            ));
        }
    }

    for forbidden in [
        "OpenNewContentFrame",
        "CloseFocusedFrame",
        "cmd-shift-n",
        "ctrl-shift-n",
        "ctrl-w",
    ] {
        if keyboard_source.contains(forbidden) {
            violations.push(format!(
                "src/app/keyboard.rs: ADR 0046 Task 013 must not bind unavailable multi-frame action `{forbidden}`"
            ));
        }
    }

    for required in [
        "visible_workspace_layout",
        "content_seen",
        "content_list_frame_title",
        "FrameNavigationEntry::Settings",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0046 Task 013 visible workspace projection missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "handle_open_new_content_frame",
        "handle_close_focused_frame",
        "TopApp::handle_open_new_content_frame",
        "TopApp::handle_close_focused_frame",
        "add_frame(WorkspaceFrameKind::ContentList)",
        "self.workspace_layout.remove_frame(id)",
        "content_frame_count",
        "for _ in 0..content_frame_count",
    ] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0046 Task 013 must not expose unavailable multi-frame routing `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Task 013 deferred multi-frame command violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_frame_phase_6_detach_dock_model_only_contract() {
    let workspace_vm_source = workspace_vm_source();
    let app_source = read_source(&manifest_path("src/app.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) enum FrameDetachEligibility",
        "Detachable,",
        "NotDetachable,",
        "pub(crate) enum FrameDockTarget",
        "Leading,",
        "Center,",
        "Trailing,",
        "pub(crate) const fn detach_eligibility",
        "pub(crate) fn request_detach",
        "pub(crate) fn request_dock",
        "DetachDeferred",
        "DockDeferred",
        "NotDetachable",
    ] {
        if !workspace_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0046 Task 014 detach/dock model contract missing `{required}`"
            ));
        }
    }

    for path in rust_files_under("src/ui") {
        let source = read_source(&path);
        for forbidden in [
            "FrameDetachEligibility",
            "FrameDockTarget",
            "request_detach",
            "request_dock",
            "DetachDeferred",
            "DockDeferred",
            "NotDetachable",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{}: ADR 0046 Task 014 detach/dock surface must remain model-only; found `{forbidden}`",
                    rel_path(&path)
                ));
            }
        }
    }

    for forbidden in [
        "request_detach",
        "request_dock",
        "FrameDetachEligibility",
        "FrameDockTarget",
        "DetachDeferred",
        "DockDeferred",
        "NotDetachable",
    ] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0046 Task 014 must not wire detach/dock window commands yet; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Task 014 detach/dock model-only violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_phase_d_filter_controls_render_through_frame_shell() {
    let filter_source = read_source(&manifest_path("src/ui/composites/filter_chip_strip.rs"));
    let library_filter_source = read_source(&manifest_path(
        "src/ui/composites/library_filter_control.rs",
    ));
    let frame_shell_source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let composites_mod_source = read_source(&manifest_path("src/ui/composites/mod.rs"));
    let workspace_source = workspace_vm_source();
    let mut violations = Vec::new();

    for required in [
        "pub(crate) struct FilterChipStrip",
        "pub(crate) struct FilterChipStripSlots",
        "filter_chip_strip(",
        "ContextMenu::new(",
        "ContextMenuScope::WorkspaceFrame",
        "narrow_collapse_to_pulldown",
    ] {
        if !filter_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/filter_chip_strip.rs: ADR 0047 Task 009 filter chip composite missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) struct LibraryFilterControl",
        "pub(crate) struct LibraryFilterControlSlots",
        "library_filter_control(",
        "Button::styled(",
        "SharedString::from(display.id)",
        "control_style(display.current.treatment)",
        ".label_treatment(label_treatment(display.current.treatment))",
        ".on_activate(move |window, cx|",
        "handler(next_filter, window, cx)",
    ] {
        if !library_filter_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/library_filter_control.rs: Situational ADR 0062 library tri-state control composite missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "rgb(",
        ".absolute()",
        ".fixed()",
        ".z_index(",
        "gpui_component::popover",
    ] {
        if filter_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/filter_chip_strip.rs: ADR 0047 Task 009 must reuse primitives/tokens and avoid `{forbidden}`"
            ));
        }
        if library_filter_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/library_filter_control.rs: Situational ADR 0062 library tri-state control must reuse primitives/tokens and avoid `{forbidden}`"
            ));
        }
    }

    for required in [
        "pub mod filter_chip_strip;",
        "pub(crate) use filter_chip_strip::{filter_chip_strip, FilterChipStrip, FilterChipStripSlots}",
        "pub mod library_filter_control;",
        "pub(crate) use library_filter_control::{",
        "library_filter_control, LibraryFilterControl, LibraryFilterControlSlots",
    ] {
        if !composites_mod_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/mod.rs: ADR 0047 Task 009 composite export missing `{required}`"
            ));
        }
    }

    for required in [
        "filter_chip_strip: Option<FilterChipStripDisplay>",
        "pub(crate) fn with_filter_chip_strip",
        "library_filter_control: Option<LibraryFilterControlDisplay>",
        "pub(crate) fn with_library_filter_control",
    ] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0047 Task 009 frame-shell display contract missing `{required}`"
            ));
        }
    }

    for required in [
        "use crate::ui::composites::{",
        "filter_chip_strip",
        "FilterChipStripSlots",
        "library_filter_control",
        "LibraryFilterControlSlots",
        "type FrameFilterSelectHandler",
        "on_filter_select",
        "display.filter_chip_strip.clone()",
        "display.library_filter_control.clone()",
        "library_filter_control_display.is_some()",
        "filter_chip_strip_display.is_some()",
        "library_filter_control(display, filter_slots)",
        "filter_chip_strip(display, filter_slots)",
    ] {
        if !frame_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: ADR 0047 Task 009 frame-shell integration missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Task 009 and Situational ADR 0062 frame filter control violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_task_010a_content_list_page_vm_owns_filter_projection() {
    let library_source = read_source(&manifest_path("src/view_models/library.rs"));
    let mut violations = Vec::new();

    for required in [
        "use crate::view_models::workspace::{",
        "ContentFilter",
        "LibraryFilterControlDisplay",
        "pub(crate) enum ContentListRowSource",
        "pub(crate) const fn matches_filter(self, filter: ContentFilter) -> bool",
        "pub(crate) struct ContentListRowDisplay",
        "pub(crate) struct ContentListEmptyStateDisplay",
        "pub(crate) struct ContentListPageVm",
        "filter_state: ContentFilter",
        "cached_rows: Vec<ContentListRowDisplay>",
        "pub(crate) fn set_filter(&mut self, filter: ContentFilter)",
        "pub(crate) fn visible_rows(&self) -> Vec<&ContentListRowDisplay>",
        "pub(crate) fn empty_state(&self) -> Option<ContentListEmptyStateDisplay>",
        "pub(crate) fn library_filter_control(&self) -> LibraryFilterControlDisplay",
        "pub(crate) fn library_filter_control_display(&self) -> LibraryFilterControlDisplay",
        "LibraryFilterControlDisplay::default_for_content_list(self.filter_state)",
    ] {
        if !library_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: ADR 0047 Task 010a content-list page VM ownership contract missing `{required}`"
            ));
        }
    }

    if library_source.contains("enum ContentFilter") {
        violations.push(
            "src/view_models/library.rs: ADR 0047 Task 010a must reuse workspace ContentFilter, not define a second enum"
                .to_string(),
        );
    }

    for path in rust_files_under("src/ui") {
        let source = read_source(&path);
        for forbidden in ["ContentListRowSource"] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{}: ADR 0047 Task 010a keeps source filtering VM-owned; UI must not reference `{forbidden}`",
                    rel_path(&path)
                ));
            }
        }
    }

    let app_source = read_source(&manifest_path("src/app.rs"));
    for forbidden in ["ContentListPageVm", "SetFrameFilter"] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0047 Task 010a must not wire content-list filter commands yet; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Task 010a content-list page VM violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_task_010_content_list_library_filter_is_frame_local() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let library_source = read_source(&manifest_path("src/view_models/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let workspace_shell_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let mut violations = Vec::new();

    for required in [
        "content_list_page: ContentListPageVm",
        "self.content_list_page",
        "replace_tree_rows(content_list_rows_from_tree(&tree))",
        "pub(crate) fn set_content_filter(&mut self, filter: ContentFilter)",
        "pub(crate) fn content_library_filter_control(&self) -> LibraryFilterControlDisplay",
        "pub(crate) fn content_filter_empty_state(&self) -> Option<ContentListEmptyStateDisplay>",
        "fn content_list_rows_from_tree(tree: &LibraryTree) -> Vec<ContentListRowDisplay>",
    ] {
        if !library_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: ADR 0047 Task 010 content-list filter ownership missing `{required}`"
            ));
        }
    }

    if library_source.contains("filter_tree_to_content_rows(&tree, &self.content_list_page)") {
        violations.push(
            "src/view_models/library.rs: ADR 0049 forbids filtering the Library source tree with the ContentList content filter"
                .to_string(),
        );
    }

    for required in [
        "pub(crate) fn content_library_filter_control(&self) -> LibraryFilterControlDisplay",
        "self.vm.content_library_filter_control()",
        "pub(crate) fn set_content_filter(&mut self, filter: ContentFilter, cx: &mut Context<Self>)",
        "self.vm.set_content_filter(filter)",
    ] {
        if !library_app_source.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0047 Task 010 LibraryApp filter bridge missing `{required}`"
            ));
        }
    }

    for required in [
        "content_list_library_filter_control: Option<LibraryFilterControlDisplay>",
        "on_content_list_filter_select: Option<WorkspaceFilterSelectHandler>",
        "pub(crate) fn content_list_library_filter_control(",
        "pub(crate) fn on_content_list_filter_select(",
        "library_filter_control_for(",
        "filter_select_handler_for(",
        "display.with_library_filter_control(library_filter_control)",
        "shell_slots.on_filter_select",
    ] {
        if !workspace_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0047 Task 010 workspace filter slot missing `{required}`"
            ));
        }
    }

    for required in [
        "fn set_frame_filter(",
        "frame_id: WorkspaceFrameId",
        "filter: ContentFilter",
        "frame.kind() == WorkspaceFrameKind::ContentList",
        ".content_list_library_filter_control(library_filter_control)",
        ".on_content_list_filter_select(move |filter, _window, cx|",
        "this.set_frame_filter(content_frame_id, filter, cx)",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0047 Task 010 app frame-filter dispatch missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "static CONTENT_FILTER",
        "global_content_filter",
        "toolbar_content_filter",
        "WorkspaceSlots::new().content_list_filter_chip_strip(FilterChipStripDisplay::default",
    ] {
        if app_source.contains(forbidden) || workspace_shell_source.contains(forbidden) {
            violations.push(format!(
                "ADR 0047 Task 010 must keep filters frame-local and VM-projected; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Task 010 content-list frame filter violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0049_inspector_source_ownership_is_guarded() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let search_query_source = read_source(&manifest_path("src/application/queries/search.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let search_results_mod_source =
        read_source(&manifest_path("src/view_models/search_results/mod.rs"));
    let search_results_index_detail_source = read_source(&manifest_path(
        "src/view_models/search_results/index_detail.rs",
    ));
    let search_results_shell_source =
        read_source(&manifest_path("src/ui/shells/search_results_inspector.rs"));
    let workspace_shell_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let feed_detail_source = read_source(&manifest_path("src/ui/shells/library/feed_detail.rs"));
    let mut violations = Vec::new();

    for required in [
        "handle_index_feed_result_selected(",
        "handle_index_track_result_selected(",
        "handle_index_artist_result_selected(",
        "strip_prefix(\"index-feed:\")",
        "strip_prefix(\"index-track:\")",
        "strip_prefix(\"index-artist:\")",
        "FrameNavigationEntry::IndexArtistFeedScope(",
        "FrameNavigationEntry::IndexFeedDetail {",
        "FrameNavigationEntry::IndexTrackDetail {",
        "render_index_detail_display(",
        "SearchResultsHeaderMode::Scoped {",
        "content_list_breadcrumb_labeler(",
        "render_index_feed_detail(feed, slots)",
        "hero_image: self.index_feed_hero_image(feed, cx)",
        "fn index_feed_hero_image(",
        "RemoteDetailThumbnailState::Loaded",
        "fn index_feed_artwork_url(",
        "fn index_feed_primary_actions(",
        "fn index_feed_track_rows(",
        "fn render_index_feed_or_fallback_detail(",
        "DisclosureTextPanel::new(",
        "SubscribeFeedRequest {",
        "SubscribeTrackRequest::SearchTrack",
    ] {
        if !app_source.contains(required) && !search_dispatch_source.contains(required) {
            violations.push(format!(
                "src/app.rs or src/app/search_dispatch.rs: ADR 0049 ContentList dispatch/Index activation missing `{required}`"
            ));
        }
    }

    if !search_query_source.contains("INDEX_FEED_DETAIL_INCLUDE") {
        violations.push(
            "src/application/queries/search.rs: ADR 0049 Index fetch query must request rich feed detail includes"
                .to_string(),
        );
    }

    for required in [
        "db::feed_tracks(&conn, feed_id)",
        "LibraryDetail::Album(album)",
        "track.is_in_library = false;",
        "track.local_path = None;",
        "apply_track_subscription_to_album_detail(",
        "track.is_in_library = true;",
        "pub(crate) fn playlists(&self) -> &[db::Playlist]",
    ] {
        if !library_app_source.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0049 album mutation/detail ownership missing `{required}`"
            ));
        }
    }

    if library_vm_source.contains("filter_tree_to_content_rows(&tree, &self.content_list_page)") {
        violations.push(
            "src/view_models/library.rs: ADR 0049 source tree must not be filtered by content filter"
                .to_string(),
        );
    }

    for required in [
        "pub(crate) fn index_feed_detail(",
        "pub(crate) fn index_track_detail(",
        "pub(crate) fn index_feed_label(",
        "pub(crate) fn index_track_label(",
        "tab_was_user_selected",
        "select_first_populated_tab_if_automatic(",
    ] {
        if !search_results_mod_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/mod.rs: ADR 0049 Index drill-down VM contract missing `{required}`"
            ));
        }
    }

    for required in ["pub(crate) struct IndexDetailDisplay"] {
        if !search_results_index_detail_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/index_detail.rs: ADR 0049 Index drill-down VM contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "SearchResultsNoticeDisplay",
        "pub(crate) fn set_notice(",
        "show_index_detail_notice(",
    ] {
        if app_source.contains(forbidden)
            || search_results_mod_source.contains(forbidden)
            || search_results_index_detail_source.contains(forbidden)
        {
            violations.push(format!(
                "ADR 0049 rejected visible notice path must stay removed; found `{forbidden}`"
            ));
        }
    }

    let index_feed_selection = search_dispatch_source
        .split("fn handle_index_feed_result_selected(")
        .nth(1)
        .and_then(|source| source.split("fn push_index_feed_detail(").next())
        .unwrap_or_default();
    if index_feed_selection.is_empty() {
        violations.push(
            "src/app/search_dispatch.rs: ADR 0049 Index feed selection handler not found"
                .to_string(),
        );
    }
    for forbidden in [
        "db::find_feed_id_by_guid",
        "FrameNavigationEntry::AlbumDetail(",
        "album_for_detail_by_feed_id",
        "select_album(",
    ] {
        if index_feed_selection.contains(forbidden) {
            violations.push(format!(
                "src/app/search_dispatch.rs: ADR 0049 index-feed activation must preserve Index detail source; found local redirect `{forbidden}`"
            ));
        }
    }
    if !index_feed_selection
        .contains("self.push_index_feed_detail(content_frame_id, feed_guid, label, cx);")
    {
        violations.push(
            "src/app/search_dispatch.rs: ADR 0049 index-feed activation must push IndexFeedDetail directly"
                .to_string(),
        );
    }

    let index_track_selection = search_dispatch_source
        .split("fn handle_index_track_result_selected(")
        .nth(1)
        .and_then(|source| source.split("fn push_index_track_detail(").next())
        .unwrap_or_default();
    if index_track_selection.is_empty() {
        violations.push(
            "src/app/search_dispatch.rs: ADR 0049 Index track selection handler not found"
                .to_string(),
        );
    }
    for forbidden in [
        "library_service::find_track_id",
        "library_service::track_row_by_id",
        "FrameNavigationEntry::TrackDetail(",
        "select_track(",
    ] {
        if index_track_selection.contains(forbidden) {
            violations.push(format!(
                "src/app/search_dispatch.rs: ADR 0049 index-track activation must preserve Index detail source; found local redirect `{forbidden}`"
            ));
        }
    }
    if !index_track_selection
        .contains("self.push_index_track_detail(content_frame_id, target, label, cx);")
    {
        violations.push(
            "src/app/search_dispatch.rs: ADR 0049 index-track activation must push IndexTrackDetail directly"
                .to_string(),
        );
    }

    for required in [
        "SearchResultsHeaderMode::Scoped",
        "SearchResultsHeaderMode::Tabbed",
        "pub(crate) fn render_index_feed_detail(",
        "EntitySurfaceContext::Library",
    ] {
        if !search_results_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/search_results_inspector.rs: ADR 0049 scoped drill-down chrome missing `{required}`"
            ));
        }
    }

    for required in [
        "content_list_breadcrumb_labeler:",
        "fn breadcrumb_labeler_for(",
        "BreadcrumbDisplay::project(breadcrumb_id, navigation, |entry| labeler(entry))",
    ] {
        if !workspace_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0049 breadcrumb label ownership missing `{required}`"
            ));
        }
    }

    for required in [
        "let active_filter = library_vm.content_filter();",
        "ContentFilter::Library => track.is_in_library",
        "ContentFilter::Index => !track.is_in_library",
    ] {
        if !feed_detail_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/library/feed_detail.rs: ADR 0049 inspector filter projection missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0049 inspector source ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0024_index_track_detail_uses_rich_track_view_path() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let search_query_source = read_source(&manifest_path("src/application/queries/search.rs"));
    let results_source = read_source(&manifest_path("src/view_models/search_results/results.rs"));
    let index_detail_source = read_source(&manifest_path(
        "src/view_models/search_results/index_detail.rs",
    ));
    let search_results_shell_source =
        read_source(&manifest_path("src/ui/shells/search_results_inspector.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) remote_track: Option<TrackView>",
        "pub(crate) fn with_remote_track",
    ] {
        if !results_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/results.rs: ADR 0024 Index track rows must carry rich remote track detail; missing `{required}`"
            ));
        }
    }

    for required in [
        "TrackView::from_api(track.clone())",
        "display = display.with_remote_track(remote_track)",
    ] {
        if !search_query_source.contains(required) {
            violations.push(format!(
                "src/application/queries/search.rs: ADR 0024 Index track detail fetch path must attach TrackView from fetched api::Track; missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) track: Option<TrackView>",
        "display.track.clone_from(&row.remote_track)",
    ] {
        if !index_detail_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/index_detail.rs: ADR 0024 Index detail must propagate rich track projection; missing `{required}`"
            ));
        }
    }

    for required in [
        "if let Some(track) = display.track.as_ref()",
        "TrackDetailVm::new(track, TrackDetailSurfaceContext::Discover).page()",
        "slots.external_links = render_track_page_identity_actions(&page)",
        "build_track_detail_surface(&page, slots)",
    ] {
        if !search_results_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/search_results_inspector.rs: ADR 0024 rich Index track detail must render through shared track surface; missing `{required}`"
            ));
        }
    }

    for required in [
        "render_index_track_detail(track, slots, cx)",
        "fn index_track_detail_slots(",
        "hero_image: self.index_track_hero_image(track, cx)",
        "fn index_track_artwork_url(track: &TrackView)",
    ] {
        if !search_dispatch_source.contains(required) {
            violations.push(format!(
                "src/app/search_dispatch.rs: ADR 0024 rich Index track detail must keep remote artwork in the shared surface slot; missing `{required}`"
            ));
        }
    }

    if !search_results_shell_source.contains("detail_metadata_row(\"Source\", \"Index\", cx)")
        || !search_results_shell_source.contains("detail_metadata_row(\"ID\", &display.id, cx)")
    {
        violations.push(
            "src/ui/shells/search_results_inspector.rs: ADR 0024 sparse Index Source/ID fallback must remain for missing remote track detail"
                .to_string(),
        );
    }

    let index_track_branch = app_source
        .split("Some(FrameNavigationEntry::IndexTrackDetail")
        .nth(1)
        .and_then(|source| source.split("Some(FrameNavigationEntry::Settings)").next())
        .unwrap_or_default();
    if index_track_branch.is_empty() {
        violations
            .push("src/app.rs: ADR 0024 IndexTrackDetail render branch not found".to_string());
    } else if !index_track_branch.contains("render_index_feed_or_fallback_detail(&detail, cx)") {
        violations.push(
            "src/app.rs: ADR 0024 IndexTrackDetail must use TopApp detail slots so rich track artwork resolves"
                .to_string(),
        );
    }
    if index_track_branch.contains("render_index_detail_display(&detail, cx)") {
        violations.push(
            "src/app.rs: ADR 0024 IndexTrackDetail must not bypass TopApp detail slots with the fallback renderer"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0024 rich Index track-detail path violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0024_playlist_local_detail_metadata_is_vm_owned_without_index_detail() {
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let playlist_page_vm_source = read_source(&manifest_path("src/view_models/playlist_detail.rs"));
    let index_detail_source = read_source(&manifest_path(
        "src/view_models/search_results/index_detail.rs",
    ));
    let mut violations = Vec::new();

    for required in [
        "if self.playlist.created_at > 0",
        "fmt_date(self.playlist.created_at)",
        "rows.push((\"Created\".to_string(), label));",
        "if self.playlist.updated_at > 0",
        "fmt_date(self.playlist.updated_at)",
        "rows.push((\"Modified\".to_string(), label));",
        "self.playlist.description.as_deref().map(str::trim)",
        "rows.push((\"Description\".to_string(), description.to_string()));",
    ] {
        if !library_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: ADR 0024 playlist local detail metadata must be projected by PlaylistDetailVm::detail_rows; missing `{required}`"
            ));
        }
    }

    if !playlist_page_vm_source.contains("self.detail.detail_rows()") {
        violations.push(
            "src/view_models/playlist_detail.rs: PlaylistDetailPageVm::detail_rows must pass through PlaylistDetailVm rows"
                .to_string(),
        );
    }

    for forbidden in [
        "IndexDetailKind::Playlist",
        "IndexPlaylistDetail",
        "PlaylistDetailDisplay",
    ] {
        if index_detail_source.contains(forbidden) {
            violations.push(format!(
                "src/view_models/search_results/index_detail.rs: ADR 0024 Task 005 must not introduce Index playlist detail behavior `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0024 playlist local-detail metadata ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0024_loading_shape_readiness_gate_is_locked() {
    let architecture_source = read_source(&manifest_path("tests/architecture_tests.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let workspace_nav_source = read_source(&manifest_path("src/view_models/workspace/nav.rs"));
    let views_source = read_source(&manifest_path("src/views.rs"));
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let playlist_page_vm_source = read_source(&manifest_path("src/view_models/playlist_detail.rs"));
    let search_results_shell_source =
        read_source(&manifest_path("src/ui/shells/search_results_inspector.rs"));
    let feed_detail_shell_source =
        read_source(&manifest_path("src/ui/shells/library/feed_detail.rs"));
    let playlist_shell_source = read_source(&manifest_path("src/ui/shells/playlist.rs"));
    let mut violations = Vec::new();

    for guard_name in [
        "local_feed_language_parity_is_loaded_through_read_model_path",
        "adr_0024_index_track_detail_uses_rich_track_view_path",
        "local_track_pubdate_and_explicit_projection_path_is_guarded",
        "index_artist_activation_is_scoped_feed_route_not_detail_page",
        "adr_0024_playlist_local_detail_metadata_is_vm_owned_without_index_detail",
    ] {
        let fn_signature = format!("fn {guard_name}(");
        if architecture_source.matches(&fn_signature).count() != 1 {
            violations.push(format!(
                "tests/architecture_tests.rs: ADR 0024 readiness gate requires exactly one `{fn_signature}` guard"
            ));
        }
    }

    for live_path in [
        "src/app/search_dispatch.rs",
        "src/view_models/search_results/results.rs",
        "src/view_models/search_results/index_detail.rs",
        "src/ui/shells/search_results_inspector.rs",
    ] {
        let source = read_source(&manifest_path(live_path));
        for forbidden in ["crate::discover", "SearchApp", "render_discover"] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{live_path}: live ADR 0024 Index parity path must not depend on parked Discover pattern `{forbidden}`"
                ));
            }
        }
    }

    for (prefix, parse_pattern) in [
        ("strip_prefix(\"index-track:\")", "parse::<i64>()"),
        ("strip_prefix(\"index-feed:\")", "parse::<i64>()"),
        (
            "strip_prefix(\"index-artist:\")",
            "strip_prefix(\"library-artist:\")",
        ),
    ] {
        let Some(prefix_index) = search_dispatch_source.find(prefix) else {
            violations.push(format!(
                "src/app/search_dispatch.rs: Index selection dispatch missing `{prefix}`"
            ));
            continue;
        };
        let Some(parse_index) = search_dispatch_source[prefix_index..].find(parse_pattern) else {
            violations.push(format!(
                "src/app/search_dispatch.rs: Index selection dispatch missing local parsing boundary `{parse_pattern}` after `{prefix}`"
            ));
            continue;
        };
        if parse_index == 0 {
            violations.push(format!(
                "src/app/search_dispatch.rs: Index prefix `{prefix}` must be handled before local id parsing"
            ));
        }
    }

    for required in [
        "IndexFeedDetail {\n        /// Stable remote feed id.\n        id: String,",
        "IndexTrackDetail {\n        /// Stable remote track activation id.\n        id: String,",
        "FrameNavigationEntry::IndexFeedDetail {\n                id: feed_guid.to_string(),",
        "FrameNavigationEntry::IndexTrackDetail {\n                id: target.to_string(),",
    ] {
        let (source_name, source) = if required.starts_with("Index") {
            (
                "src/view_models/workspace/nav.rs",
                workspace_nav_source.as_str(),
            )
        } else {
            (
                "src/app/search_dispatch.rs",
                search_dispatch_source.as_str(),
            )
        };
        if !source.contains(required) {
            violations.push(format!(
                "{source_name}: ADR 0024 Index detail routes must store remote string ids; missing `{required}`"
            ));
        }
    }

    for (source_name, source, forbidden) in [
        (
            "src/ui/shells/library/feed_detail.rs",
            feed_detail_shell_source.as_str(),
            "\"Language\"",
        ),
        (
            "src/ui/shells/playlist.rs",
            playlist_shell_source.as_str(),
            "\"Created\"",
        ),
        (
            "src/ui/shells/playlist.rs",
            playlist_shell_source.as_str(),
            "\"Modified\"",
        ),
        (
            "src/ui/shells/playlist.rs",
            playlist_shell_source.as_str(),
            "\"Description\"",
        ),
        (
            "src/ui/shells/search_results_inspector.rs",
            search_results_shell_source.as_str(),
            "\"Explicit\"",
        ),
    ] {
        if source.contains(forbidden) {
            violations.push(format!(
                "{source_name}: ADR 0024 parity labels must be VM/query-owned, not renderer-only `{forbidden}`"
            ));
        }
    }

    for required in [
        "language: nonempty_owned(f.language)",
        "track_number: t.track_number",
        "duration_secs: t.duration_secs",
        "pub_date: t.pub_date",
        "explicit: t.explicit",
        "contributors: t\n                .source_contributors",
        "payment_routes: t.payment_routes.unwrap_or_default()",
        "transcript_url: nonempty_owned(transcript_url)",
        "track_number: t.track_number.and_then(|v| v.try_into().ok())",
        "duration_secs: t.duration_seconds.and_then(|v| v.try_into().ok())",
        "pub_date: t.pub_date",
        "explicit: t.explicit",
        "transcript_url: t.transcript_url",
    ] {
        if !views_source.contains(required) {
            violations.push(format!(
                "src/views.rs: ADR 0024 surfaced parity fields must remain owned by FeedView/TrackView projection; missing `{required}`"
            ));
        }
    }

    for required in [
        "rows.push((\"Created\".to_string(), label));",
        "rows.push((\"Modified\".to_string(), label));",
        "rows.push((\"Description\".to_string(), description.to_string()));",
    ] {
        if !library_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: ADR 0024 playlist surfaced parity fields must remain VM-owned; missing `{required}`"
            ));
        }
    }

    if !playlist_page_vm_source.contains("self.detail.detail_rows()") {
        violations.push(
            "src/view_models/playlist_detail.rs: PlaylistDetailPageVm must pass through PlaylistDetailVm::detail_rows"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0024 loading-shape readiness gate violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_task_012_frame_navigation_is_workspace_vm_owned() {
    let workspace_source = workspace_vm_source();
    let library_struct_source = read_source(&manifest_path("src/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let mut violations = Vec::new();

    for required in [
        "frame_navigation: BTreeMap<WorkspaceFrameId, FrameNavigationState>",
        "pub(crate) fn frame_nav(&self, id: WorkspaceFrameId) -> Option<&FrameNavigationState>",
        "pub(crate) fn frame_nav_mut(",
        "pub(crate) fn reset_nav(",
        "pub(crate) fn push_nav(",
        "pub(crate) fn pop_nav(&mut self, id: WorkspaceFrameId) -> Option<FrameNavigationEntry>",
        "fn default_navigation_entry(kind: WorkspaceFrameKind) -> FrameNavigationEntry",
        "BreadcrumbTruncation::MiddleEllipsis",
        "breadcrumb-ellipsis",
    ] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0047 Task 012 workspace-owned frame navigation missing `{required}`"
            ));
        }
    }

    for required in [
        "workspace_layout: WorkspaceLayout",
        "fn default_workspace_layout() -> WorkspaceLayout",
        ".reset_nav(Self::content_frame_id(), FrameNavigationEntry::SourceList)",
        ".push_nav(Self::content_frame_id(), entry)",
        "self.workspace_layout\n            .pop_nav(Self::content_frame_id())",
        "self.workspace_layout.frame_nav(Self::content_frame_id())",
    ] {
        let source = if required == "workspace_layout: WorkspaceLayout" {
            &library_struct_source
        } else {
            &library_app_source
        };
        if !source.contains(required) {
            violations.push(format!(
                "LibraryApp ADR 0047 Task 012 workspace navigation bridge missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "frame_navigation: BTreeMap<WorkspaceFrameId, FrameNavigationState>",
        "fn default_frame_navigation(",
        "self.frame_navigation",
    ] {
        if library_struct_source.contains(forbidden) || library_app_source.contains(forbidden) {
            violations.push(format!(
                "LibraryApp must not own raw frame navigation after ADR 0047 Task 012; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Task 012 frame navigation ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_task_013_frame_shell_renders_breadcrumb_chrome() {
    let workspace_source = workspace_vm_source();
    let frame_shell_source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) breadcrumb: Option<BreadcrumbDisplay>",
        "breadcrumb: None",
        "pub(crate) fn with_breadcrumb(mut self, display: BreadcrumbDisplay) -> Self",
    ] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0047 Task 013 frame-shell breadcrumb display contract missing `{required}`"
            ));
        }
    }

    for required in [
        "type FrameBreadcrumbSelectHandler",
        "on_breadcrumb_select: Option<FrameBreadcrumbSelectHandler>",
        "pub(crate) fn on_breadcrumb_select(",
        "let breadcrumb_display = display.breadcrumb.clone();",
        "BreadcrumbTrail::new(breadcrumb)",
        ".appearance(",
        ".on_select(move |entry, window, cx",
        "px(Spacing::MD.scaled(cx))",
        "pb(Spacing::XS.scaled(cx))",
    ] {
        if !frame_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: ADR 0047 Task 013 breadcrumb chrome missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "crate::library",
        "crate::search",
        "crate::app",
        "crate::db",
        "gpui::rgb(",
        "gpui::px(",
        ".absolute()",
        ".fixed()",
        ".z_index(",
    ] {
        if frame_shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: ADR 0047 Task 013 frame shell must not use `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Task 013 frame-shell breadcrumb violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_breadcrumb_unification_guards_frame_shell_helpers_removed() {
    let frame_shell_source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let mut violations = Vec::new();

    for forbidden_fn in [
        "fn frame_breadcrumb_row",
        "fn frame_breadcrumb_segment",
        "fn frame_breadcrumb_separator",
    ] {
        if frame_shell_source.contains(forbidden_fn) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: breadcrumb unification must remove `{forbidden_fn}` hand-rolled helper"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 breadcrumb unification violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_task_014_search_results_inspector_shell_contract() {
    let shell_source = read_source(&manifest_path("src/ui/shells/search_results_inspector.rs"));
    let row_shell_source = read_source(&manifest_path("src/ui/shells/search_result_rows.rs"));
    let shells_mod_source = read_source(&manifest_path("src/ui/shells/mod.rs"));
    let workspace_shell_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let search_results_source =
        read_source(&manifest_path("src/view_models/search_results/mod.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) struct SearchResultsInspectorSlots",
        "pub(crate) fn render_search_results_inspector",
        "SearchResultsInspectorPageVm",
        "SegmentedControl::new(selected)",
        "Segment::new(tab_segment_display(SearchResultsTab::Artists))",
        "Segment::new(tab_segment_display(SearchResultsTab::Feeds))",
        "Segment::new(tab_segment_display(SearchResultsTab::Tracks))",
        "vm.empty_state()",
        "render_empty_state(",
        "render_active_result_list(",
        "window.peek_row(index)",
        "RowSlot::Ready(row)",
        "RowSlot::Pending(placeholder)",
    ] {
        if !shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/search_results_inspector.rs: ADR 0047 Task 014 shell contract missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) fn render_result_row(",
        "pub(crate) fn render_pending_result_row(",
        "ListRow::new(",
        "Thumbnail::new(kind, ThumbnailSize::Sm)",
        "TagBadge::new(TagBadgeDisplay",
    ] {
        if !row_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/search_result_rows.rs: ADR 0047 Task 014 shared row shell contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "crate::search",
        "crate::library",
        "crate::app",
        "crate::db",
        "gpui::rgb(",
        "gpui::px(",
        ".absolute()",
        ".fixed()",
        ".z_index(",
        "ControlStyle::Pill",
    ] {
        if shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/search_results_inspector.rs: ADR 0047 Task 014 shell must not use `{forbidden}`"
            ));
        }
    }

    for required in [
        "pub mod search_results_inspector;",
        "detail_filter_chip_strip: Option<FilterChipStripDisplay>",
        "on_detail_filter_select: Option<WorkspaceFilterSelectHandler>",
        "pub(crate) fn detail_filter_chip_strip(",
        "pub(crate) fn on_detail_filter_select(",
        "WorkspaceFrameKind::Detail => self.detail_filter_chip_strip.clone()",
        "WorkspaceFrameKind::Detail => self.on_detail_filter_select.clone()",
    ] {
        let source = if required == "pub mod search_results_inspector;" {
            &shells_mod_source
        } else {
            &workspace_shell_source
        };
        if !source.contains(required) {
            violations.push(format!(
                "ADR 0047 Task 014 workspace search-inspector mounting support missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) fn filter_chip_strip(&self) -> FilterChipStripDisplay",
        "pub(crate) fn filter_chip_strip_for_width_class(",
        "FilterChipStripDisplay::default_for_search_inspector_width_class(",
    ] {
        if !search_results_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/mod.rs: ADR 0047 Task 014 search-results filter display missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Task 014 search-results inspector shell violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_task_015_search_submit_and_saved_search_commands() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let library_event_source = read_source(&manifest_path("src/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let workspace_source = workspace_vm_source();
    let workspace_shell_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) const fn default_detail_frame_id() -> WorkspaceFrameId",
        "pub(crate) fn replace_nav(",
        "pub(crate) fn open_search_results_in_content_list(",
        "FrameNavigationEntry::Search(query)",
        "nav.replace_active_search_or_push(FrameNavigationEntry::Search(query));",
        "pub(crate) fn replace_active_search_or_push(",
        ".rposition(|candidate| matches!(candidate, FrameNavigationEntry::Search(_)))",
        "self.focus_frame(content_list_frame_id)?;",
        "pub(crate) fn display_label(&self) -> String",
        "format!(\"Search: {query}\")",
    ] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0047 Task 015 workspace-owned search frame command missing `{required}`"
            ));
        }
    }

    for required in [
        "search_results_detail: Option<SearchResultsInspectorPageVm>",
        "fn open_saved_search(",
        "fn set_search_results_filter(",
        "fn set_search_results_tab(",
        "SearchResultsInspectorSlots::new()",
        ".on_tab_select(move |tab, _window, cx|",
        ".on_clear_filter(move |_window, cx|",
        "SearchResultsHeaderMode::Tabbed,",
        "LibraryAppEvent::OpenSavedSearch",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0047 Task 015 app-level search inspector routing missing `{required}`"
            ));
        }
    }

    for required in [
        "fn open_search_results_in_content_list(",
        "fn search_results_detail_for_query(",
        ".search_local_library_tracks(&conn, query, None)",
        "SearchResultsInspectorPageVm::from_local_library_tracks(query, &local_tracks)",
    ] {
        if !search_dispatch_source.contains(required) {
            violations.push(format!(
                "src/app/search_dispatch.rs: ADR 0047 Task 015 app-level search inspector routing missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "self.tab = AppTab::Search;",
        "search.run_global_search(query, cx)",
    ] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0047 Task 015 must not route toolbar submit through the legacy Search tab; found `{forbidden}`"
            ));
        }
    }

    for required in [
        ".frame_nav(frame.id())",
        "BreadcrumbDisplay::project(",
        "FrameNavigationEntry::display_label",
        "fn should_render_breadcrumb(",
        "matches!(kind, WorkspaceFrameKind::ContentList)",
        "nav.has_history()",
        "FrameNavigationEntry::Search(_)",
        ".on_back(move |window, cx|",
    ] {
        if !workspace_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0048 ContentList breadcrumb/back projection missing `{required}`"
            ));
        }
    }

    for required in ["OpenSavedSearch {", "saved_search_id: i64", "query: String"] {
        if !library_event_source.contains(required) {
            violations.push(format!(
                "src/library.rs: ADR 0047 Task 015 saved-search event missing `{required}`"
            ));
        }
    }

    for required in [
        "SavedSearchesSectionDisplay",
        "fn open_saved_search(&mut self, saved_search_id: i64",
        ".saved_searches()",
        "LibraryAppEvent::OpenSavedSearch",
        "self.vm.saved_searches_section()",
        "ListRow::compact(SharedString::from(row_id))",
        "this.open_saved_search(saved_search_id, cx);",
    ] {
        if !library_app_source.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0047 Task 015 source-list saved-search command missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "this.select_playlist(saved_search_id",
        "this.select_track(saved_search_id",
        "select_playlist_with_history(saved_search_id",
        "select_track_with_history(saved_search_id",
    ] {
        if library_app_source.contains(forbidden) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0047 Task 015 saved-search activation must not disturb source-list selection; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Task 015 search-submit and saved-search command violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_task_016_retires_standalone_search_module_and_workspace_toggle() {
    let lib_source = read_source(&manifest_path("src/lib.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let mut violations = Vec::new();

    for retired_path in [
        manifest_path("src/search.rs"),
        manifest_path("src/search/app_impl.rs"),
        manifest_path("src/search/tests.rs"),
    ] {
        if retired_path.exists() {
            violations.push(format!(
                "{}: ADR 0047 Task 016 retired the standalone search module path",
                rel_path(&retired_path)
            ));
        }
    }

    if lib_source.contains("pub mod search;") {
        violations.push(
            "src/lib.rs: ADR 0047 Task 016 must not export the retired top-level search module"
                .to_string(),
        );
    }
    if !lib_source.contains("pub mod discover;") {
        violations.push(
            "src/lib.rs: ADR 0047 Task 016 preserved Discover behavior under `discover`"
                .to_string(),
        );
    }

    for required in ["pub struct SearchApp", "mod app_impl;", "mod tests;"] {
        let discover_source = read_source(&manifest_path("src/discover.rs"));
        if !discover_source.contains(required) {
            violations.push(format!(
                "src/discover.rs: ADR 0047 Task 016 Discover compatibility module missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "WORKSPACE_RENDER_ENABLED",
        "render_legacy_tab_content",
        "if WORKSPACE_RENDER_ENABLED",
    ] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0047 Task 016 retired workspace fallback still present as `{forbidden}`"
            ));
        }
    }

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for forbidden in ["crate::search", "src/search.rs"] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{file}:{line_number}: ADR 0047 Task 016 forbids retired search-module references; found `{forbidden}` in `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 Task 016 search-module retirement violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn discover_module_public_surface_is_pinned() {
    let source = read_source(&manifest_path("src/discover.rs"));
    let expected = [
        "ArtistContext",
        "discover_inspector_action_row",
        "FeedTrackListContext",
        "InspectorDetail",
        "InspectorFrame",
        "is_local_library_track",
        "render_play_icon_button_with_id",
        "render_track_download_button",
        "render_track_list_rows",
    ];
    let mut actual = BTreeSet::new();
    let mut pending_use: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(buffer) = pending_use.as_mut() {
            buffer.push(' ');
            buffer.push_str(trimmed);
            if trimmed.ends_with(';') {
                if let Some(names) = pub_crate_use_names(buffer) {
                    actual.extend(names);
                }
                pending_use = None;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("pub(crate) enum ") {
            actual.insert(name_from_decl(rest));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("pub(crate) struct ") {
            actual.insert(name_from_decl(rest));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("pub(crate) fn ") {
            actual.insert(name_from_decl(rest));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("pub(crate) type ") {
            actual.insert(name_from_decl(rest));
            continue;
        }
        if trimmed.starts_with("pub(crate) use ") {
            if trimmed.ends_with(';') {
                if let Some(names) = pub_crate_use_names(trimmed) {
                    actual.extend(names);
                }
            } else {
                pending_use = Some(trimmed.to_string());
            }
        }
    }

    let expected = expected
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "src/discover.rs: parked Discover surface changed; update the fixture only with deliberate maintenance"
    );
}

fn name_from_decl(rest: &str) -> String {
    rest.split(|ch: char| ch == '{' || ch == '(' || ch == '<' || ch.is_whitespace())
        .next()
        .unwrap_or(rest)
        .trim()
        .to_string()
}

fn pub_crate_use_names(line: &str) -> Option<Vec<String>> {
    let rest = line.strip_prefix("pub(crate) use ")?;
    let rest = rest.strip_suffix(';').unwrap_or(rest);

    if let Some(braced) = rest.split_once('{') {
        let (_, tail) = braced;
        let body = tail.strip_suffix('}')?;
        let mut names = Vec::new();

        for item in body.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let item = item.split_whitespace().next().unwrap_or(item);
            let name = item.rsplit("::").next().unwrap_or(item).trim();
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }

        return Some(names);
    }

    let name = rest.rsplit("::").next().unwrap_or(rest).trim();
    if name.is_empty() {
        None
    } else {
        Some(vec![name.to_string()])
    }
}

#[test]
fn active_frame_search_dispatch_phase_1_vm_contracts_are_owned_by_view_models() {
    let workspace_source = workspace_vm_source();
    let library_source = read_source(&manifest_path("src/view_models/library.rs"));
    let feed_source = read_source(&manifest_path("src/view_models/feed.rs"));
    let playlist_detail_source = read_source(&manifest_path("src/view_models/playlist_detail.rs"));
    let search_results_source =
        read_source(&manifest_path("src/view_models/search_results/mod.rs"));
    let queue_source = read_source(&manifest_path("src/view_models/queue_now_playing.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) enum FrameSearchScope",
        "pub(crate) struct FrameSearchDescriptor",
        "pub(crate) fn focused_search_descriptor(&self) -> Option<FrameSearchDescriptor>",
        "frame_id: WorkspaceFrameId",
        "kind: WorkspaceFrameKind",
        "nav: FrameNavigationEntry",
        "scope: FrameSearchScope",
        "placeholder: &'static str",
        "FrameSearchScope::Sidebar",
        "FrameSearchScope::LibraryRows",
        "FrameSearchScope::SettingsRows",
        "FrameSearchScope::InspectorQuery",
        "FrameSearchScope::DetailTracks",
        "FrameSearchScope::QueueRows",
        "Filter sidebar...",
        "Search library...",
        "Search settings...",
        "Refine search...",
        "Filter tracks...",
        "Filter queue...",
    ] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: active-frame search descriptor contract missing `{required}`"
            ));
        }
    }

    for required in [
        "text_filter: Option<String>",
        "pub(crate) fn set_text_filter(&mut self, filter: Option<String>)",
        "pub(crate) fn text_filter(&self) -> Option<&str>",
        "pub(crate) fn set_content_text_filter(&mut self, filter: Option<String>)",
        "pub(crate) fn set_source_text_filter(&mut self, filter: Option<String>)",
        "normalize(filter)",
    ] {
        if !library_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: active-frame content/source text filter contract missing `{required}`"
            ));
        }
    }

    for required in [
        "pub fn set_text_filter(&mut self, filter: Option<String>)",
        "pub fn text_filter(&self) -> Option<&str>",
        "fn track_matches_text_filter(&self, track: &Track) -> bool",
    ] {
        if !feed_source.contains(required) {
            violations.push(format!(
                "src/view_models/feed.rs: active-frame feed detail text filter contract missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) fn set_text_filter(&mut self, filter: Option<String>)",
        "pub(crate) fn text_filter(&self) -> Option<&str>",
        "self.detail.set_text_filter(filter);",
    ] {
        if !playlist_detail_source.contains(required) {
            violations.push(format!(
                "src/view_models/playlist_detail.rs: active-frame playlist page text filter contract missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) fn set_query(&mut self, query: String)",
        "pub(crate) fn clear_query(&mut self)",
        "self.refresh_empty_state();",
    ] {
        if !search_results_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/mod.rs: active-frame search inspector query contract missing `{required}`"
            ));
        }
    }

    for required in [
        "all_rows: Vec<QueueRowDisplay>",
        "text_filter: Option<String>",
        "pub(crate) fn set_text_filter(&mut self, filter: Option<String>)",
        "pub(crate) fn text_filter(&self) -> Option<&str>",
        "queue_row_matches_text_filter(row, filter)",
    ] {
        if !queue_source.contains(required) {
            violations.push(format!(
                "src/view_models/queue_now_playing.rs: active-frame queue text filter contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "static QUEUE_TEXT_FILTERS",
        "OnceLock<Mutex<HashMap",
        "thread_local!",
    ] {
        if queue_source.contains(forbidden) {
            violations.push(format!(
                "src/view_models/queue_now_playing.rs: queue text filter state must be owned by QueueNowPlayingPageVm, found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Active-frame search dispatch Phase 1 VM contract violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn view_models_do_not_reintroduce_file_local_text_filter_normalizers() {
    let mut violations = Vec::new();

    for path in rust_files_under("src/view_models") {
        let file = rel_path(&path);
        let source = read_source(&path);
        if file != "src/view_models/text_filter.rs" && source.contains("fn normalize_text_filter(")
        {
            violations.push(format!(
                "{file}: text filters must use src/view_models/text_filter.rs instead of a file-local `normalize_text_filter` helper"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Shared text-filter helper architecture violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_frame_phase_4_guards_queue_now_playing_vm_contract() {
    let source = read_source(&manifest_path("src/view_models/queue_now_playing.rs"));
    let mod_source = read_source(&manifest_path("src/view_models/mod.rs"));
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        for pattern in [
            "use gpui",
            "gpui::",
            "use gpui_component",
            "gpui_component::",
            "PlaybackOwner",
            "TrackRow",
        ] {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/view_models/queue_now_playing.rs:{line_number}: ADR 0046 Phase 4 queue VM must stay GPUI-free and avoid backend row handles; found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    for required in [
        "pub(crate) struct QueueNowPlayingPageVm",
        "pub(crate) struct QueueRowDisplay",
        "pub(crate) struct TransportDisplay",
        "pub(crate) enum TransportState",
        "pub(crate) struct QueueTrackInput",
        "FrameChromeButtonDisplay",
        "QueueNowPlayingPageVmBuilder",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "src/view_models/queue_now_playing.rs: ADR 0046 Phase 4 queue VM contract missing `{required}`"
            ));
        }
    }

    if !mod_source.contains("pub(crate) mod queue_now_playing;") {
        violations.push(
            "src/view_models/mod.rs: ADR 0046 Phase 4 queue VM module is not exported".to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Phase 4 queue VM contract violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_frame_phase_4_guards_queue_frame_shell_wiring() {
    let shell_source = read_source(&manifest_path("src/ui/shells/queue_now_playing.rs"));
    let workspace_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let show_source = read_source(&manifest_path("src/app/show.rs"));
    let adapter_source = read_source(&manifest_path("src/app/queue_now_playing.rs"));
    let mod_source = read_source(&manifest_path("src/ui/shells/mod.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) fn render_queue_cuelist(",
        "pub(crate) fn render_queue_transport(",
        "QueueNowPlayingPageVm",
        "QueueNowPlayingSlots",
        "IconName::Previous",
        "IconName::Pause",
        "IconName::Next",
    ] {
        if !shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/queue_now_playing.rs: ADR 0046 Phase 4 queue shell missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "crate::library",
        "crate::search",
        "crate::app",
        "PlaybackOwner",
        "crate::db",
    ] {
        if shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/queue_now_playing.rs: ADR 0046 Phase 4 queue shell must not import screen/backend module `{forbidden}`"
            ));
        }
    }

    for forbidden in [
        ".queue_now_playing(queue_frame)",
        "build_queue_now_playing_frame(self, cx)",
    ] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0060 moves queue rendering into Show; found `{forbidden}`"
            ));
        }
    }

    for required in [
        "mod queue_now_playing;",
        "mod show;",
        "build_show_screen(self, show_window_width, cx).into_any_element()",
        "if matches!(mount, WorkspaceScreenMount::Show)",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0060 Show mount must own queue rendering; missing `{required}`"
            ));
        }
    }

    for required in [
        "app.show_page.clone()",
        "ShowSlots::new()",
        "queue_transport_action(",
        "TopApp::skip_playback_previous",
        "TopApp::toggle_playback_paused",
        "TopApp::skip_playback_next",
    ] {
        if !show_source.contains(required) {
            violations.push(format!(
                "src/app/show.rs: ADR 0060 Show adapter queue wiring missing `{required}`"
            ));
        }
    }

    for required in [
        "QueueNowPlayingPageVm::builder()",
        "services: &ApplicationServices",
        "queue_tracks_for_session(",
        "playlist_queue_projection(",
        ".skip_availability(",
        "queue_transport_action(",
        "entity.update(cx",
    ] {
        if !adapter_source.contains(required) {
            violations.push(format!(
                "src/app/queue_now_playing.rs: ADR 0046 Phase 4 queue VM adapter missing `{required}`"
            ));
        }
    }

    if !app_source.contains("mod queue_now_playing;") {
        violations.push(
            "src/app.rs: ADR 0046 Phase 4 app module must declare queue_now_playing adapter"
                .to_string(),
        );
    }

    for required in ["FrameShellSlots::new().content(content)", "QueueNowPlaying"] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: ADR 0046 legacy queue frame support must remain behind shared frame shell; missing `{required}`"
            ));
        }
    }

    if !mod_source.contains("pub mod queue_now_playing;") {
        violations.push(
            "src/ui/shells/mod.rs: ADR 0046 Phase 4 queue shell module is not exported".to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Phase 4 queue frame shell wiring violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_frame_phase_4_guards_playback_commands_stay_out_of_toolbar() {
    let playback_source = read_source(&manifest_path("src/app/playback_bar.rs"));
    let toolbar_source = read_source(&manifest_path("src/app/tab_bar.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(super) fn play_playlist_at(",
        "pub(super) fn toggle_playback_paused(",
        "fn run_playback_command",
        "this.refresh_show_page(cx)",
    ] {
        if !playback_source.contains(required) {
            violations.push(format!(
                "src/app/playback_bar.rs: ADR 0046 playback command binding missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "NowPlayingBar",
        "NowPlayingData",
        "build_playback_bar",
        "\"Nothing playing\"",
        "on_prev",
        "on_next",
        "on_stop",
        "\"np-prev\"",
        "\"np-next\"",
        "\"np-playpause\"",
        "\"np-stop\"",
        "play_pause_a11y_label",
        "on_play_pause",
        "transport_btn(",
        "Button::styled",
        "StopPlayback",
        "playback_bar_state",
        "IconName::Previous",
        "IconName::Next",
        "IconName::Stop",
    ] {
        if playback_source.contains(forbidden) {
            violations.push(format!(
                "src/app/playback_bar.rs: ADR 0046 Phase 4 toolbar card must stay compact; found `{forbidden}`"
            ));
        }
    }

    for forbidden in [
        "QueueNowPlayingPageVm",
        "ContextMenuScope::WorkspaceFrame",
        "display.now_playing",
        "app-toolbar-now-playing",
        "build_playback_bar",
    ] {
        if toolbar_source.contains(forbidden) {
            violations.push(format!(
                "src/app/tab_bar.rs: ADR 0046 Phase 4 toolbar must not own queue controls; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Phase 4 compact toolbar guard violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_frame_phase_2_guards_workspace_vm_contract_is_gpui_free_and_typed() {
    let source = workspace_vm_source();
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        for pattern in [
            "use gpui",
            "gpui::",
            "use gpui_component",
            "gpui_component::",
        ] {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/view_models/workspace/mod.rs:{line_number}: ADR 0046 Phase 2 workspace model must stay GPUI-free; found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    for required in [
        "struct WorkspaceFrameId",
        "enum WorkspaceFrameKind",
        "struct WorkspaceFrameState",
        "struct WorkspaceLayout",
        "struct FrameNavigationState",
        "enum FrameNavigationEntry",
        "SourceList",
        "ContentList",
        "Detail",
        "QueueNowPlaying",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0046 Phase 2 workspace VM contract missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0046 Task 004 workspace-frame Phase 2 VM guard violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_frame_phase_2_guards_frame_navigation_is_wired_in_library_app_impl() {
    let source = read_source(&manifest_path("src/library/app_impl.rs"));

    for required in [
        "WorkspaceLayout",
        "FrameNavigationState",
        "FrameNavigationEntry",
        "fn default_workspace_layout() -> WorkspaceLayout",
        ".reset_nav(Self::content_frame_id(), FrameNavigationEntry::SourceList)",
        "fn push_frame_navigation(",
        "fn restore_frame_navigation(",
        "fn frame_back_destination(&self)",
        ".push_nav(Self::content_frame_id(), entry)",
        "self.workspace_layout\n            .pop_nav(Self::content_frame_id())",
        "FrameNavigationEntry::PlaylistDetail(playlist_id)",
        "FrameNavigationEntry::TrackDetail(track.id)",
        "self.restore_frame_navigation()",
    ] {
        assert!(
            source.contains(required),
            "src/library/app_impl.rs: ADR 0046 Phase 2 frame-navigation wiring missing `{required}`"
        );
    }

    for forbidden in [
        "fn default_frame_navigation(",
        "frame_navigation:",
        "self.frame_navigation",
        "frame.origin =",
        "InspectorOrigin",
        "origin: Option<InspectorOrigin>",
    ] {
        assert!(
            !source.contains(forbidden),
            "src/library/app_impl.rs: ADR 0046 Phase 2 navigation must not use inspector origin `{forbidden}`"
        );
    }
}

#[test]
fn workspace_frame_phase_2_guards_inspector_origin_navigation_is_absent() {
    let source = read_source(&manifest_path("src/library.rs"));

    for forbidden in [
        "pub(crate) enum InspectorOrigin",
        "InspectorOrigin",
        "pub(crate) origin: Option<InspectorOrigin>",
        "origin: Option<InspectorOrigin>",
    ] {
        assert!(
            !source.contains(forbidden),
            "src/library.rs: ADR 0046 Phase 2 must not retain inspector-origin navigation `{forbidden}`"
        );
    }
}

#[test]
fn workspace_frame_phase_2_guards_inspector_local_playlist_back_control_is_absent() {
    let track_detail_source = read_source(&manifest_path(
        "src/ui/shells/library/track_detail_metadata.rs",
    ));
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));

    for forbidden in [
        "playlist_return_display",
        "LibraryTrackPlaylistReturnDisplay",
        "return_to_playlist",
        "navigate_back_to_playlist",
        "Back to Playlist",
        "track-detail-return-playlist",
    ] {
        assert!(
            !track_detail_source.contains(forbidden),
            "src/ui/shells/library/track_detail_metadata.rs: ADR 0046 Phase 2 track inspector must not retain local playlist Back control `{forbidden}`"
        );
        assert!(
            !library_vm_source.contains(forbidden),
            "src/view_models/library.rs: ADR 0046 Phase 2 library VM must not retain local playlist Back display `{forbidden}`"
        );
    }
}

#[test]
fn entity_detail_projection_does_not_import_api_ui_or_services() {
    let path = manifest_path("src/view_models/entity_detail.rs");
    let source = read_source(&path);
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        for pattern in ENTITY_DETAIL_FORBIDDEN_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/view_models/entity_detail.rs:{line_number}: ADR 0026/0027 shared projections must use `views` facts and stay UI/service-free; found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0026 entity-detail projection boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_entity_shell_does_not_import_screens_or_services() {
    let path = manifest_path("src/ui/shells/entity.rs");
    let source = read_source(&path);
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        for pattern in UI_ENTITY_FORBIDDEN_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/ui/shells/entity.rs:{line_number}: ADR 0026 UI shell must stay slot-based and avoid screen/service imports; found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0026 ui_entity shell boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn shared_ui_components_do_not_import_backend_or_screen_layers() {
    let mut violations = Vec::new();
    for relative_dir in ["src/ui/primitives", "src/ui/composites"] {
        for path in rust_files_under(relative_dir) {
            let source = read_source(&path);
            for (line_number, line) in code_lines(&source) {
                for pattern in SHARED_UI_BACKEND_FORBIDDEN_PATTERNS {
                    if line.contains(pattern) {
                        violations.push(format!(
                            "{}:{line_number}: ADR 0033 shared UI must accept display-ready data and callbacks, not backend/screen dependency `{pattern}`: `{line}`",
                            rel_path(&path)
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 shared UI backend/screen boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn shared_ui_render_paths_use_scale_aware_tokens() {
    let mut files = Vec::new();
    files.extend(rust_files_under("src/ui/primitives"));
    files.extend(rust_files_under("src/ui/composites"));
    files.push(manifest_path("src/ui/icons.rs"));
    files.sort();

    let mut violations = Vec::new();
    for path in files {
        let relative = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if !contains_unscaled_token_px_call(&line) {
                continue;
            }
            if shared_ui_unscaled_token_px_is_allowed(&relative, &line) {
                continue;
            }
            violations.push(format!(
                "{relative}:{line_number}: ADR 0034 shared UI render paths must use scale-aware token accessors, not unscaled `.px()`: `{line}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0034 shared UI scale-token violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn tooltip_chrome_routes_through_primitive() {
    let mut violations = Vec::new();

    for path in rust_files_under("src") {
        let relative = rel_path(&path);
        if relative == "src/ui/primitives/tooltip.rs" {
            continue;
        }

        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("gpui_component::tooltip::Tooltip") {
                violations.push(format!(
                    "{relative}:{line_number}: tooltip chrome must route through `src/ui/primitives/tooltip.rs`, not direct gpui_component tooltip usage: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "tooltip primitive routing violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn shared_header_badges_use_intrinsic_flex_rows() {
    let mut violations = Vec::new();
    for file in [
        "src/ui/composites/detail_header.rs",
        "src/ui/composites/track_header.rs",
    ] {
        let source = read_source(&manifest_path(file));
        let compact = compact_source(&source);
        if compact.contains(".child(div().mb(Spacing::") {
            violations.push(format!(
                "{file}: header badges must not sit in block-width margin wrappers; wrap `TagBadge` in an intrinsic flex row"
            ));
        }
        if !compact.contains(".flex().flex_row().items_start().mb(Spacing::")
            || !compact.contains(".child(badge)")
        {
            violations.push(format!(
                "{file}: expected header badge wrapper to use an intrinsic `.flex().flex_row().items_start()` row"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0034 shared header badge layout violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn presentation_modules_do_not_hand_roll_floating_chrome() {
    let mut violations = Vec::new();
    for file in PRESENTATION_GLUE_FILES {
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in SCREEN_LOCAL_FLOATING_CHROME_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: ADR 0033 floating chrome belongs in shared primitives/composites, not presentation modules; found `{pattern}` in `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 screen-local floating chrome violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn application_layer_does_not_import_gpui_or_screen_layers() {
    let mut violations = Vec::new();
    for path in rust_files_under("src/application") {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in APPLICATION_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{}:{line_number}: forbidden application-layer dependency `{pattern}` in `{line}`",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0024 application-layer boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn top_level_keyboard_shortcuts_route_through_key_binding_taxonomy() {
    let keyboard_source = read_source(&manifest_path("src/app/keyboard.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let bootstrap_source = read_source(&manifest_path("src/app/bootstrap.rs"));
    let mut violations = Vec::new();

    for required in [
        "APP_KEY_BINDING_SPECS",
        "TogglePlayback",
        "SkipPlaybackNext",
        "SkipPlaybackPrevious",
        "FocusSearch",
        "NewPlaylist",
        "SelectMusicTab",
        "SelectSettingsTab",
        "MoveSelectionUp",
        "MoveSelectionDown",
        "ConfirmSelection",
    ] {
        if !keyboard_source.contains(required) {
            violations.push(format!(
                "src/app/keyboard.rs: missing keyboard taxonomy entry `{required}`"
            ));
        }
    }

    if !bootstrap_source.contains("install_key_bindings(cx)") {
        violations.push(
            "src/app/bootstrap.rs: app bootstrap must install the keyboard binding registry"
                .to_string(),
        );
    }

    if app_source.contains(".on_key_down(cx.listener(TopApp::handle_key_down))") {
        violations.push(
            "src/app.rs: top-level keyboard routing must use typed actions, not raw key-down matching"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "top-level keyboard shortcut taxonomy violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0067_keyboard_and_menu_share_platform_modifier_resolution() {
    for file in ["src/app/keyboard.rs", "src/app/menu.rs"] {
        let source = read_source(&manifest_path(file));
        assert!(
            source.contains("platform_keystroke(self.keystroke, is_macos)"),
            "Situational ADR 0067: {file} must resolve its registry through the shared platform adapter"
        );
        assert!(
            !source.contains("KeyBinding::new(self.keystroke,"),
            "Situational ADR 0067: {file} must not bypass platform resolution"
        );
    }
}

#[test]
fn adr_0067_mount_and_section_changes_establish_a_persistent_focus_path() {
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    let app = read_source(&manifest_path("src/app.rs"));
    let tabs = read_source(&manifest_path("src/app/tab_bar.rs"));
    assert!(bootstrap.contains("app.focus_active_tab(window);"),
        "Situational ADR 0067: normal mount must focus a control beneath the app action handlers before the first shortcut");
    assert!(app.contains("self.tab = tab;\n        self.focus_active_tab(window);"),
        "Situational ADR 0067: every section transition must replace focus from an input that may leave the mounted tree");
    assert!(tabs.contains(".track_focus(focus_handle)")
        && tabs.contains("focus_handle_for_key(key, self).focus(window);"),
        "Situational ADR 0067: transition focus must use the persistent handles mounted by the shared tab bar");
    let render = source_between(&app, "impl Render for TopApp", "fn render_ui_scale_picker");
    assert!(
        !render.contains("focus_active_tab("),
        "Situational ADR 0067: rendering must not repeatedly steal focus from inputs"
    );
}

#[test]
fn adr_0040_settings_cache_reads_leave_the_render_thread() {
    let app = read_source(&manifest_path("src/app.rs"));
    let render = app
        .split_once("fn render_settings(")
        .expect("Settings renderer")
        .1;
    for forbidden in ["reload_cached(", ".lock()", "cached_tracks(", "build_tree("] {
        assert!(
            !render.contains(forbidden),
            "Situational ADR 0040: Settings must paint cached observations without database work: {forbidden}"
        );
    }
    let load = source_between(&app, "fn reload_cached(", "fn delete_cached_file(");
    assert!(load.contains("present_command(") && load.contains("LoadCachedTracksTree::new("));
    assert!(!load.contains(".lock()") && !load.contains("build_tree("));
    assert!(render.contains("app.cached_files.status()"));
    let select = source_between(&app, "fn select_tab(", "fn content_list_frame_id(");
    assert!(select.contains("self.reload_cached(cx);"));
    for path in ["src/app/events.rs", "src/app/capabilities.rs"] {
        assert!(read_source(&manifest_path(path)).contains("self.reload_cached(cx);"),
            "Situational ADR 0040: mutations and runtime recovery must refresh the mounted Settings observation");
    }
}

#[test]
fn macos_app_menu_bootstrap_exposes_standard_app_commands() {
    let menu_source = read_source(&manifest_path("src/app/menu.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let bootstrap_source = read_source(&manifest_path("src/app/bootstrap.rs"));
    let mut violations = Vec::new();

    for required in [
        "MenuItem::action(\"Preferences...\", OpenPreferences)",
        "MenuItem::os_submenu(\"Services\", SystemMenuType::Services)",
        "MenuItem::action(\"Hide Application\", HideApp)",
        "MenuItem::action(\"Hide Others\", HideOtherApps)",
        "MenuItem::action(\"Show All\", ShowAllApps)",
        "MenuItem::action(\"Quit Application\", QuitApp)",
        "keystroke: \"cmd-,\"",
        "keystroke: \"cmd-h\"",
        "keystroke: \"cmd-alt-h\"",
        "keystroke: \"cmd-q\"",
    ] {
        if !menu_source.contains(required) {
            violations.push(format!(
                "src/app/menu.rs: missing standard app-menu contract `{required}`"
            ));
        }
    }

    if !bootstrap_source.contains("install_app_menu(cx)") {
        violations.push(
            "src/app/bootstrap.rs: app bootstrap must install the macOS app menu".to_string(),
        );
    }

    if !app_source.contains(".on_action(cx.listener(TopApp::handle_open_preferences))") {
        violations.push(
            "src/app.rs: Preferences menu action must route to the Settings surface".to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "macOS app-menu bootstrap violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn app_shell_avoids_premature_product_naming() {
    let mut violations = Vec::new();
    for file in ["src/app/tab_bar.rs", "src/app/menu.rs"] {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for forbidden in ["V4V Music Manager", "MusicIndex"] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{file}:{line_number}: top-level shell must avoid premature product branding; keep `{forbidden}` attribution for About/settings surfaces: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "premature product naming violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn app_toolbar_exposes_tabs_and_global_search_without_now_playing_chip() {
    let toolbar_source = read_source(&manifest_path("src/app/tab_bar.rs"));
    let playback_source = read_source(&manifest_path("src/app/playback_bar.rs"));
    let vm_source = read_source(&manifest_path("src/view_models/app_toolbar.rs"));
    let mut violations = Vec::new();

    for required in [
        "AppToolbarVm::new().display()",
        "display.leading_id",
        "display.center_id",
        ".max_w(TokenSize::ColumnTall.scaled(cx))",
    ] {
        if !toolbar_source.contains(required) {
            violations.push(format!(
                "src/app/tab_bar.rs: ADR 0043 toolbar frame missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) struct AppToolbarDisplay",
        "pub(crate) struct AppToolbarTabDisplay",
        "pub(crate) struct GlobalSearchDisplay",
        "mark_a11y_label",
        "a11y_label",
    ] {
        if !vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/app_toolbar.rs: ADR 0043 toolbar VM missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "NowPlayingFrameDisplay",
        "now_playing: NowPlayingFrameDisplay",
        "display.now_playing",
        "app-toolbar-now-playing",
        "NowPlayingData",
        "NowPlayingBar",
        "build_playback_bar",
        "\"Nothing playing\"",
        "on_play_pause",
        "\"np-playpause\"",
    ] {
        if vm_source.contains(forbidden)
            || toolbar_source.contains(forbidden)
            || playback_source.contains(forbidden)
        {
            violations.push(format!(
                "ADR 0060 task 004 removes toolbar now-playing ownership; found `{forbidden}`"
            ));
        }
    }

    for forbidden in ["Button::styled", "ControlStyle::ToolbarIcon"] {
        if playback_source.contains(forbidden) {
            violations.push(format!(
                "src/app/playback_bar.rs: ADR 0060 task 004 playback command binding must not render toolbar controls; found `{forbidden}`"
            ));
        }
    }

    for path in rust_files_under("src/ui/composites") {
        let source = read_source(&path);
        if source.contains("NowPlayingBar") || source.contains("NowPlayingData") {
            violations.push(format!(
                "{}: ADR 0060 task 004 removes the toolbar Now Playing card; found old app-shell type in composites",
                rel_path(&path)
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 toolbar live-status ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn global_search_contract_has_toolbar_vm_and_local_query_boundary() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let query_source = read_source(&manifest_path("src/application/queries/search.rs"));
    let db_source = read_source(&manifest_path("src/db.rs"));
    let service_source = read_source(&manifest_path("src/library_service.rs"));
    let vm_source = read_source(&manifest_path("src/view_models/app_toolbar.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) struct GlobalSearchDisplay",
        "input_id",
        "search_button_id",
        "search_button_label",
        "search_button_a11y_label",
    ] {
        if !vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/app_toolbar.rs: ADR 0043 global search VM contract missing `{required}`"
            ));
        }
    }

    for path in rust_files_under("src") {
        let source = read_source(&path);
        for forbidden in [
            "GlobalSearchScope",
            "global_search_scope",
            "set_global_search_scope",
            "APP_TOOLBAR_SCOPE_BREAKPOINT",
            "ContextMenuScope::GlobalSearchScope",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{}: ADR 0047 Task 011 retired toolbar global-search scope controls; remove `{forbidden}`",
                    rel_path(&path)
                ));
            }
        }
    }

    for required in [
        "global_search_input: Entity<InputState>",
        "AppToolbarVm::new().display().global_search",
        "InputState::new(window, cx).placeholder(global_search_display.placeholder)",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0043 TopApp global search ownership missing `{required}`"
            ));
        }
    }

    for required in [
        "DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT: usize = 50",
        "pub fn search_local_library_tracks(",
        "normalized_global_search_query",
        "library_service::search_library_tracks(",
    ] {
        if !query_source.contains(required) {
            violations.push(format!(
                "src/application/queries/search.rs: ADR 0043 local search query boundary missing `{required}`"
            ));
        }
    }

    for required in [
        "pub fn search_library_tracks(",
        "conn: &Connection",
        "query: &str",
        "limit: usize",
        "WHERE t.is_in_library = 1",
        "LIKE ?1 ESCAPE '\\\\'",
        "LIMIT ?2",
    ] {
        if !db_source.contains(required) {
            violations.push(format!(
                "src/db.rs: ADR 0043 in-library search storage query missing `{required}`"
            ));
        }
    }

    if !service_source.contains("db::search_library_tracks(conn, query, limit)") {
        violations.push(
            "src/library_service.rs: ADR 0043 local search must route through library_service"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0043 global search contract violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn global_search_replaces_screen_local_search_chrome() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let keyboard_source = read_source(&manifest_path("src/app/keyboard.rs"));
    let toolbar_source = read_source(&manifest_path("src/app/tab_bar.rs"));
    let icon_source = read_source(&manifest_path("src/ui/icons.rs"));
    let library_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let search_app_source = read_source(&manifest_path("src/discover/app_impl.rs"));
    let search_query_source = read_source(&manifest_path("src/application/queries/search.rs"));
    let search_shell_source = read_source(&manifest_path("src/ui/shells/discover/search_input.rs"));
    let search_vm_source = search_vm_source();
    let mut violations = Vec::new();

    if !app_source.contains("fn on_global_search_event(") {
        violations.push(
            "src/app.rs: ADR 0043 toolbar search routing missing `fn on_global_search_event(`"
                .to_string(),
        );
    }

    for required in [
        "fn submit_global_search(",
        "self.open_search_results_in_content_list(",
    ] {
        if !search_dispatch_source.contains(required) {
            violations.push(format!(
                "src/app/search_dispatch.rs: ADR 0043 toolbar search routing missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "self.tab = AppTab::Search;",
        "search.run_global_search(query, cx)",
    ] {
        if app_source.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0047 Task 015 toolbar search submit must route through the workspace Detail frame, found `{forbidden}`"
            ));
        }
    }

    for required in [
        "Input::new(&app.global_search_input)",
        ".prefix(InputIconName::Search)",
        "layout::APP_TOOLBAR_GLOBAL_SEARCH_COMPACT_BREAKPOINT",
        "toolbar_width,",
        "let use_compact_search =",
        "display.search_button_id",
        ".label(display.search_button_label)",
        "UiButton::styled(display.search_button_id, ControlStyle::ToolbarIcon)",
        ".leading_icon(IconName::Search)",
        ".tooltip(display.search_button_a11y_label)",
    ] {
        if !toolbar_source.contains(required) {
            violations.push(format!(
                "src/app/tab_bar.rs: ADR 0043 toolbar search render missing `{required}`"
            ));
        }
    }

    for required in [
        "use gpui_component::{Icon as ComponentIcon, IconName as ComponentIconName};",
        "fn component_icon(self) -> Option<ComponentIconName>",
        "Self::Search => Some(ComponentIconName::Search)",
        "ComponentIcon::new(component_icon).size(size)",
        "Self::Rss | Self::Nostr | Self::Search => None",
    ] {
        if !icon_source.contains(required) {
            violations.push(format!(
                "src/ui/icons.rs: compact search submit must use the shared vector icon contract `{required}`"
            ));
        }
    }

    if !keyboard_source.contains("self.focus_global_search(window, cx)") {
        violations.push(
            "src/app/keyboard.rs: FocusSearch must focus the toolbar search field".to_string(),
        );
    }
    if keyboard_source.contains("focus_active_search") {
        violations.push(
            "src/app/keyboard.rs: FocusSearch must not route to screen-local search fields"
                .to_string(),
        );
    }

    for forbidden in [
        "search_input: Entity<InputState>",
        "Input::new(&self.search_input)",
        "fn apply_search(",
        "fn focus_search(",
        "on_search_event",
    ] {
        if library_source.contains(forbidden) {
            violations.push(format!(
                "src/library/app_impl.rs: Library must not retain visible/local search chrome `{forbidden}`"
            ));
        }
    }

    for forbidden in [
        "Input::new(&params.input)",
        "DiscoverSearchInputParams",
        "render_discover_search_input",
    ] {
        if search_shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/discover/search_input.rs: Search workspace controls must not render duplicate search input `{forbidden}`"
            ));
        }
    }

    for required in [
        "pub(crate) fn run_global_search(",
        "SearchResultSource::Library",
        "load_local_track_inspector(",
    ] {
        if !search_app_source.contains(required) {
            violations.push(format!(
                "src/discover/app_impl.rs: Search workspace global routing missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) struct FetchDiscoverSearchResults",
        "fn fetch_local_library_search_rows(",
    ] {
        if !search_query_source.contains(required) {
            violations.push(format!(
                "src/application/queries/search.rs: Search workspace query ownership missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) enum SearchResultSource",
        "pub(crate) struct SearchResultSection",
        "library_results: Vec<ResultRow>",
        "active_filter: ContentFilter",
        "index_controls: IndexControlsVisibility",
        "ContentFilter::All",
        "show_recents_command = !show_recents_root",
    ] {
        if !search_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/search/: grouped search VM contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "show_scope_controls",
        "SegmentedControl::new(app.global_search_scope)",
        "ContextMenuScope::GlobalSearchScope",
        "include_search_action",
        "APP_TOOLBAR_SCOPE_BREAKPOINT",
    ] {
        if toolbar_source.contains(forbidden) {
            violations.push(format!(
                "src/app/tab_bar.rs: ADR 0047 Task 011 retired toolbar scope controls; remove `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0043 duplicate-search replacement violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0055_search_view_model_is_decomposed_under_module_tree() {
    let mut violations = Vec::new();
    let legacy_path = manifest_path("src/view_models/search.rs");
    if legacy_path.exists() {
        violations.push(
            "src/view_models/search.rs: ADR 0055 retired the single-file search VM".to_string(),
        );
    }

    let mod_path = manifest_path("src/view_models/search/mod.rs");
    if !mod_path.is_file() {
        violations.push("src/view_models/search/mod.rs: ADR 0055 module root missing".to_string());
    }

    let mod_source = read_source(&mod_path);
    for required in [
        "mod actions;",
        "mod controls;",
        "mod feed_detail;",
        "mod lazy;",
        "mod recent;",
        "mod results;",
        "mod track;",
        "mod tests;",
    ] {
        if !mod_source.contains(required) {
            violations.push(format!(
                "src/view_models/search/mod.rs: ADR 0055 module wiring missing `{required}`"
            ));
        }
    }

    for required_file in [
        "src/view_models/search/actions.rs",
        "src/view_models/search/controls.rs",
        "src/view_models/search/feed_detail.rs",
        "src/view_models/search/lazy.rs",
        "src/view_models/search/recent.rs",
        "src/view_models/search/results.rs",
        "src/view_models/search/track.rs",
        "src/view_models/search/tests.rs",
    ] {
        if !manifest_path(required_file).is_file() {
            violations.push(format!("{required_file}: ADR 0055 expected module missing"));
        }
    }

    for (file, source) in search_vm_sources() {
        for (line_number, line) in code_lines(&source) {
            for forbidden in VIEW_MODEL_FORBIDDEN_PATTERNS {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{file}:{line_number}: search view-model modules must remain GPUI-free and renderer-free; found `{forbidden}` in `{line}`"
                    ));
                }
            }
        }
    }

    let private_modules = [
        "actions",
        "common",
        "controls",
        "feed_detail",
        "lazy",
        "recent",
        "results",
        "track",
    ];
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        if file.starts_with("src/view_models/search/") {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for module in private_modules {
                if line.contains(&format!("view_models::search::{module}::"))
                    || (line.contains("view_models::search::{")
                        && line.contains(&format!("{module}::")))
                {
                    violations.push(format!(
                        "{file}:{line_number}: callers must import through `crate::view_models::search`, not deep private search module `{module}`: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0055 search VM decomposition violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn settings_form_inputs_fill_scaled_frame_width() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let mut violations = Vec::new();

    for required in [
        "let settings_column_width = layout::scaled_dimension(layout::SETTINGS_COLUMN_WIDTH, cx);",
        ".w(settings_column_width)\n                .max_w(relative(1.0))",
        "fn render_settings_text_input(",
        "div()\n        .w_full()\n        .min_w_0()\n        .flex()\n        .flex_row()",
        "Input::new(input)",
        ".scaled(Size::Small, cx)\n                .flex_1()\n                .min_w_0()",
        "render_settings_text_input(&endpoint_input, cx)",
        "render_settings_text_input(&music_dir_input, cx)",
        "render_settings_text_input(&flac_path_input, cx)",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: settings form width/scale contract missing `{required}`"
            ));
        }
    }

    if app_source.contains(".max_w(layout::SETTINGS_COLUMN_WIDTH)") {
        violations.push(
            "src/app.rs: settings form must not use unscaled SETTINGS_COLUMN_WIDTH directly"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "settings form width/scale violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn interactive_surfaces_route_through_minimum_hit_target_token() {
    let token_source = read_source(&manifest_path("src/ui/tokens.rs"));
    let layout_source = read_source(&manifest_path("src/ui/layouts.rs"));
    let mut violations = Vec::new();

    if !token_source.contains("MinHitTarget") || !token_source.contains("px(44.0)") {
        violations.push(
            "src/ui/tokens.rs: minimum hit target must be a named 44 px size token".to_string(),
        );
    }

    if !layout_source.contains("MIN_HIT_TARGET: Pixels = px(44.0)") {
        violations.push(
            "src/ui/layouts.rs: layout constants must expose `MIN_HIT_TARGET` at the HIG 44 px floor"
                .to_string(),
        );
    }

    for (file, required) in [
        ("src/app/tab_bar.rs", "layout::MIN_HIT_TARGET"),
        (
            "src/ui/composites/disclosure_group.rs",
            "Size::MinHitTarget",
        ),
        ("src/ui/composites/list_row.rs", "Size::MinHitTarget"),
        ("src/ui/composites/track_row.rs", "layouts::MIN_HIT_TARGET"),
        ("src/ui/icons.rs", "layout::MIN_HIT_TARGET"),
        ("src/ui/primitives/button.rs", "Size::MinHitTarget"),
        (
            "src/ui/shells/discover/actions.rs",
            "layout::MIN_HIT_TARGET",
        ),
    ] {
        let source = read_source(&manifest_path(file));
        if !source.contains(required) {
            violations.push(format!(
                "{file}: shared interactive surface must route hit sizing through `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "minimum hit-target contract violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn playlist_rows_scale_through_design_tokens() {
    let playlist_shell = read_source(&manifest_path("src/ui/shells/playlist.rs"));
    let thumbnail_shell = read_source(&manifest_path("src/ui/shells/library/thumbnail.rs"));
    let library_shell = read_source(&manifest_path("src/library/app_impl.rs"));
    let layout_source = read_source(&manifest_path("src/ui/layouts.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub fn scaled_dimension(base: Pixels, cx: &App) -> Pixels",
        "pub fn scaled_f32(base: f32, cx: &App) -> Pixels",
        "ScaleFactor::current(cx).multiplier()",
    ] {
        if !layout_source.contains(required) {
            violations.push(format!(
                "src/ui/layouts.rs: scaled legacy layout bridge missing `{required}`"
            ));
        }
    }

    for required in [
        "layout::scaled_dimension(layout::PLAYLIST_THUMB_SLOT, cx)",
        "layout::scaled_dimension(layout::PLAYLIST_TITLE_OFFSET, cx)",
        "TokenSize::MinHitTarget.scaled(cx)",
        "FontSize::Caption.scaled(cx)",
        "render_playlist_thumb_placeholder(cx)",
    ] {
        if !playlist_shell.contains(required) {
            violations.push(format!(
                "src/ui/shells/playlist.rs: playlist row UI-scale contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        ".text_xs()",
        "Radius::SM.px()",
        ".text_size(layout::ACTION_ICON_INNER_SIZE)",
    ] {
        if playlist_shell.contains(forbidden) || thumbnail_shell.contains(forbidden) {
            violations.push(format!(
                "playlist row/thumbnail render paths must not use unscaled `{forbidden}`"
            ));
        }
    }

    for required in [
        "render_album_thumb(image: Option<Arc<Image>>, size: f32, cx: &App)",
        "let size = layout::scaled_f32(size, cx);",
        "Radius::SM.scaled(cx)",
        "FontSize::Headline.scaled(cx)",
    ] {
        if !thumbnail_shell.contains(required) {
            violations.push(format!(
                "src/ui/shells/library/thumbnail.rs: album thumbnail UI-scale contract missing `{required}`"
            ));
        }
    }

    if !library_shell.contains("Label::new(playlist_heading).weight(FontWeight::SEMIBOLD)") {
        violations.push(
            "src/library/app_impl.rs: playlist sidebar heading must render through the scaled Label primitive"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "playlist UI-scale violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn animation_paths_route_through_reduce_motion_environment() {
    let token_source = read_source(&manifest_path("src/ui/tokens.rs"));
    let library_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let theme_source = read_source(&manifest_path("src/ui/theme_bridge.rs"));
    let mut violations = Vec::new();

    for required in ["reduce_motion: bool", "allows_motion"] {
        if !token_source.contains(required) {
            violations.push(format!(
                "src/ui/tokens.rs: Environment must expose Reduce Motion contract `{required}`"
            ));
        }
    }

    if !theme_source.contains("reduce_motion: false") {
        violations.push(
            "src/ui/theme_bridge.rs: environment bootstrap must initialize Reduce Motion policy"
                .to_string(),
        );
    }

    if !library_source.contains("Environment::current(cx).allows_motion()") {
        violations.push(
            "src/library/app_impl.rs: animated thumbnail loading must respect the shared Reduce Motion environment"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "Reduce Motion routing violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn row_context_menu_chrome_has_shared_primitive_contract() {
    let primitive_source = read_source(&manifest_path("src/ui/primitives/context_menu.rs"));
    let mod_source = read_source(&manifest_path("src/ui/primitives/mod.rs"));
    let icon_source = read_source(&manifest_path("src/ui/icons.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub struct ContextMenu",
        "pub struct ContextMenuItem",
        "pub struct ContextMenuItemDisplay",
        "pub enum ContextMenuScope",
        "ContextMenuScope::FeedList",
        "ContextMenuScope::TrackList",
        "ContextMenuScope::PlaylistTrack",
        "ControlStyle::RowAction",
        "ControlStyle::DestructiveRowAction",
        "Size::MenuRegular",
        "Spacing::SM",
        "Popover::new",
    ] {
        if !primitive_source.contains(required) {
            violations.push(format!(
                "src/ui/primitives/context_menu.rs: missing shared context-menu contract `{required}`"
            ));
        }
    }

    if !mod_source.contains("pub mod context_menu")
        || !mod_source.contains("pub use context_menu::{")
    {
        violations.push(
            "src/ui/primitives/mod.rs: context-menu primitive must be exported from the primitive layer"
                .to_string(),
        );
    }

    if !icon_source.contains("IconName::More") && !icon_source.contains("More =>") {
        violations.push(
            "src/ui/icons.rs: context-menu trigger affordance must use the semantic icon catalog"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "row context-menu primitive violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn pressable_button_chrome_does_not_use_on_accent_on_ghost_surfaces() {
    let mut violations = Vec::new();

    for file in [
        "src/ui/control_styles.rs",
        "src/ui/shells/discover/actions.rs",
        "src/ui/shells/discover/search_input.rs",
    ] {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            if line.contains("ControlStyle::Pill") && line.contains("OnAccent") {
                violations.push(format!(
                    "{file}:{line_number}: tinted pill buttons must use accent text, not OnAccent"
                ));
            }
            if line.contains(".ghost()") {
                let following = source
                    .lines()
                    .skip(line_number)
                    .take(12)
                    .collect::<Vec<_>>()
                    .join("\n");
                if following.contains("text_on_accent()") {
                    violations.push(format!(
                        "{file}:{line_number}: ghost buttons must not render OnAccent text on transparent surfaces"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "pressable button contrast routing violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn discover_type_filter_uses_segmented_control_contract() {
    let source = read_source(&manifest_path("src/ui/shells/discover/search_input.rs"));

    assert!(
        source.contains("render_type_filter_control(params.type_filter, cx)")
            && source.contains("SegmentedControl::new(selected)")
            && source.contains(".filter_style()"),
        "Discover type filters must render through the shared segmented-control filter style"
    );
    assert!(
        !source.contains("fn render_filter_button")
            && !source.contains("(\"type-filter\", idx)")
            && !source.contains("option.index == params.type_filter"),
        "Discover type filters must not swap screen-local Ghost/Pill button styles"
    );
}

#[test]
fn core_non_ui_modules_do_not_import_ui_modules() {
    let mut violations = Vec::new();
    for path in non_ui_core_rust_files() {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in APPLICATION_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{}:{line_number}: core non-UI code must stay UI-free; found `{pattern}` in `{line}`",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 core non-UI boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn shared_view_facts_do_not_expose_api_identity_rows() {
    let source = read_source(&manifest_path("src/views.rs"));
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        for pattern in SHARED_VIEW_FACT_FORBIDDEN_PUBLIC_FIELDS {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/views.rs:{line_number}: ADR 0026 shared view facts must use local identity/contributor facts, not `{pattern}`: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0026 shared view fact boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screen_contributor_panels_use_shared_projection_facts() {
    let mut violations = Vec::new();

    for file in ["src/discover.rs", "src/library.rs"] {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for pattern in SCREEN_CONTRIBUTOR_PANEL_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: ADR 0026/0028 contributor panels must use `ContributorView` and shared contributor projections, not `{pattern}`: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0026 contributor projection boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_reintroduce_raw_color_or_numeric_px_literals() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("rgb(") {
                violations.push(format!(
                    "{file}:{line_number}: raw `rgb(...)` must live in tokens/theme, not screens: `{line}`"
                ));
            }
            if contains_numeric_px_literal(&line) {
                violations.push(format!(
                    "{file}:{line_number}: numeric `px(...)` literal must be named in theme/layout tokens: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0023 screen literal boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn composites_do_not_reintroduce_raw_color_or_numeric_px_literals() {
    let mut violations = Vec::new();
    for path in rust_files_under("src/ui/composites") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("rgb(") {
                violations.push(format!(
                    "{file}:{line_number}: raw `rgb(...)` must live in tokens/theme, not composites: `{line}`"
                ));
            }
            if contains_numeric_px_literal(&line) {
                violations.push(format!(
                    "{file}:{line_number}: numeric `px(...)` literal must be named in theme/layout tokens: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0034 composite literal boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0042_composite_call_site_reconciliation_is_current() {
    let adr = read_source(&manifest_path("docs/adr/0042-layer-consolidation.md"));
    let audit = read_source(&manifest_path("docs/research/composite-audit-adr-0042.md"));
    let composites_mod = read_source(&manifest_path("src/ui/composites/mod.rs"));
    let recent_shell = read_source(&manifest_path("src/ui/shells/discover/recent.rs"));
    let frame_shell = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let entity_shell = read_source(&manifest_path("src/ui/shells/entity.rs"));
    let library_metadata = read_source(&manifest_path(
        "src/ui/shells/library/track_detail_metadata.rs",
    ));
    let discover_metadata = read_source(&manifest_path(
        "src/ui/shells/discover/track_inspector_metadata.rs",
    ));

    assert!(
        !composites_mod.contains("skeleton_feed_tile"),
        "ADR 0042 reconciliation inlines Discover-only skeleton_feed_tile out of composites"
    );
    assert!(
        recent_shell.contains("struct SkeletonFeedTile"),
        "Discover recent shell should own its local skeleton feed tile"
    );
    assert!(
        frame_shell.contains("BreadcrumbTrail::new(breadcrumb)")
            && read_source(&manifest_path("src/ui/shells/library/track_detail.rs"))
                .contains("BreadcrumbTrail::new(breadcrumb)"),
        "BreadcrumbTrail must keep both frame-shell and Library track-detail callers"
    );
    assert!(
        library_metadata.contains("MusicBrainzPanel::new(vm)")
            && discover_metadata.contains("MusicBrainzPanel::new(vm)"),
        "MusicBrainzPanel must keep Library and Discover metadata callers"
    );
    assert!(
        entity_shell.contains("ReleaseDetailSurface::new(page.detail_scroll_id)")
            && adr.contains("release_detail_surface` is retained")
            && audit.contains("`release_detail_surface` now has one direct Rust caller"),
        "ReleaseDetailSurface single direct caller must remain documented as the shared entity shell contract"
    );
}

/// Renders run inside an active `entity.update`, so re-reading the owning
/// entity (or any chain rooted at `cx.entity()`) panics with
/// `cannot read X while it is already being updated`. Forbid that pattern in
/// any file that participates in a screen render.
#[test]
fn screens_do_not_reread_owning_entity_during_render() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("cx.entity().read(") || line.contains("entity.read(cx)") {
                violations.push(format!(
                    "{file}:{line_number}: re-reading the owning entity during render \
                     causes a GPUI re-entrancy panic; pass the data in via parameter: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "GPUI re-entrancy hazard:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_call_migrated_playlist_service_paths() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in SCREEN_PLAYLIST_SERVICE_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: migrated playlist workflows must go through ADR 0024 commands/queries, not `{pattern}`: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0024 playlist screen boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_call_migrated_subscription_remove_paths() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in SCREEN_SUBSCRIPTION_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: migrated subscription/remove workflows must go through ADR 0024 commands, not `{pattern}`: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0024 subscription/remove screen boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_access_track_artist_binding_storage() {
    let forbidden = [
        "track_artist_source_bindings",
        "TrackArtistSourceBinding",
        "track_artist_source_bindings_for_track(",
        "replace_track_artist_source_bindings",
        "db::artist_source_fact(",
    ];
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: ADR 0045 binding storage and artist-source fact lookups must stay in DB/ingest/read-model helpers, not screens/UI: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0045 screen binding-storage violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_and_view_models_do_not_access_metadata_source_fact_storage() {
    let forbidden = [
        "entity_metadata_facts",
        "LocalMetadataFact",
        "LocalMetadataOwner",
        "LocalMetadataValue",
        "local_metadata_facts(",
        "replace_local_metadata_facts(",
        "crate::local_metadata",
        "local_metadata::",
    ];
    let mut files = screen_enforcement_files();
    files.push("src/views.rs".to_owned());
    files.extend(
        rust_files_under("src/ui")
            .into_iter()
            .map(|path| rel_path(&path)),
    );
    files.extend(
        rust_files_under("src/view_models")
            .into_iter()
            .map(|path| rel_path(&path)),
    );
    files.sort();
    files.dedup();

    let mut violations = Vec::new();
    for file in files {
        let path = manifest_path(&file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: ADR 0054 metadata source-fact storage must stay out of UI/view-model layers: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0054 UI/view-model metadata source-fact storage violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn metadata_source_fact_table_access_is_owned_by_db() {
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("entity_metadata_facts")
                && file != "src/db.rs"
                && !file.starts_with("src/db/")
            {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0054 raw metadata fact table access belongs in the db module: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0054 raw metadata fact table ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn metadata_source_fact_storage_helpers_have_explicit_callers() {
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            let calls_storage_helper = line.contains("local_metadata_facts(")
                || line.contains("replace_local_metadata_facts(");
            if calls_storage_helper
                && !ADR0054_METADATA_STORAGE_CALLER_ALLOWLIST.contains(&file.as_str())
            {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0054 metadata fact storage helpers must stay at approved DB/ingest/read-model/service boundaries: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0054 metadata storage helper caller violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn metadata_source_fact_release_kind_and_rss_medium_stay_distinct() {
    let identity_ingest = read_source(&manifest_path("src/identity_ingest.rs"));
    let local_metadata = read_source(&manifest_path("src/local_metadata.rs"));
    let compact_identity_ingest = compact_source(&identity_ingest);

    assert!(
        identity_ingest.contains("\"musicindex_release_kind\"")
            && identity_ingest.contains("\"rss_podcast_medium\""),
        "ADR 0054 ingest must keep both musicindex_release_kind and rss_podcast_medium source facts visible"
    );
    assert!(
        !compact_identity_ingest.contains(
            "fact_key:\"musicindex_release_kind\".to_owned(),value:LocalMetadataValue::Text(\"rss_podcast_medium\""
        ),
        "ADR 0054 rss_podcast_medium must not be collapsed into musicindex_release_kind ingest facts"
    );
    assert!(
        local_metadata.contains("\"musicindex_release_kind\" if facts.release_kind.is_none()")
            && !local_metadata.contains("\"rss_podcast_medium\" if facts.release_kind.is_none()"),
        "ADR 0054 read-model release_kind hydration must use musicindex_release_kind only"
    );
}

#[test]
fn metadata_source_fact_keys_stay_owner_scoped() {
    let identity_ingest = read_source(&manifest_path("src/identity_ingest.rs"));
    let local_metadata = read_source(&manifest_path("src/local_metadata.rs"));
    let views = read_source(&manifest_path("src/views.rs"));

    let feed_ingest = source_between(
        &identity_ingest,
        "fn feed_metadata_facts_by_source(",
        "fn track_metadata_facts(",
    );
    let track_ingest = source_between(
        &identity_ingest,
        "fn track_metadata_facts(",
        "fn push_grouped_text_metadata_fact(",
    );
    let feed_hydration = source_between(
        &local_metadata,
        "fn feed_facts_from_rows(",
        "fn track_facts_from_rows(",
    );
    let track_hydration = source_between(
        &local_metadata,
        "fn track_facts_from_rows(",
        "fn text_value(",
    );

    assert_fact_key_set(
        "ADR 0054 feed ingest",
        feed_ingest,
        ADR0054_FEED_FACT_KEYS,
        &ADR0054_FEED_FACT_KEYS[..ADR0054_FEED_FACT_KEYS.len() - 1],
    );
    assert_fact_key_set(
        "ADR 0054 track ingest",
        track_ingest,
        ADR0054_TRACK_FACT_KEYS,
        ADR0054_TRACK_FACT_KEYS,
    );
    assert_fact_key_set(
        "ADR 0054 feed hydration",
        feed_hydration,
        &ADR0054_FEED_FACT_KEYS[..ADR0054_FEED_FACT_KEYS.len() - 1],
        &ADR0054_FEED_FACT_KEYS[..ADR0054_FEED_FACT_KEYS.len() - 1],
    );
    assert_fact_key_set(
        "ADR 0054 track hydration",
        track_hydration,
        ADR0054_TRACK_FACT_KEYS,
        ADR0054_TRACK_FACT_KEYS,
    );

    for required in [
        "pub struct FeedMetadataFacts",
        "pub publisher_text: Option<String>",
        "pub release_kind: Option<String>",
        "pub release_date: Option<i64>",
        "pub language: Option<String>",
        "pub explicit: Option<bool>",
        "pub description: Option<String>",
        "pub struct TrackMetadataFacts",
        "pub publisher_text: Option<String>",
        "pub description: Option<String>",
        "pub pub_date: Option<i64>",
        "pub explicit: Option<bool>",
    ] {
        assert!(
            views.contains(required),
            "ADR 0054 metadata fact view projection missing `{required}`"
        );
    }
}

#[test]
fn track_artist_binding_storage_is_owned_by_db_ingest_and_read_models() {
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            let raw_table_access = line.contains("FROM track_artist_source_bindings")
                || line.contains("INTO track_artist_source_bindings")
                || line.contains("UPDATE track_artist_source_bindings")
                || line.contains("CREATE TABLE IF NOT EXISTS track_artist_source_bindings")
                || line.contains("ON track_artist_source_bindings");
            if raw_table_access && file != "src/db.rs" {
                violations.push(format!(
                    "{file}:{line_number}: raw ADR 0045 binding table access belongs in src/db.rs: `{line}`"
                ));
            }
            if (line.contains("replace_track_artist_source_bindings(")
                || line.contains("replace_track_artist_source_bindings_for_source("))
                && !matches!(
                    file.as_str(),
                    "src/db.rs" | "src/identity_ingest.rs" | "src/sources.rs"
                )
            {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0045 binding writes belong in DB/ingest helpers: `{line}`"
                ));
            }
            if line.contains("track_artist_source_bindings_for_track(")
                && !matches!(
                    file.as_str(),
                    "src/db.rs" | "src/identity_ingest.rs" | "src/sources.rs"
                )
            {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0045 binding reads belong in DB/source read-model helpers: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0045 binding storage ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screen_library_removal_entry_points_use_canonical_plan() {
    let mut violations = Vec::new();
    for file in LIBRARY_REMOVAL_PRESENTATION_FILES {
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in SCREEN_LIBRARY_REMOVAL_LEGACY_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: library-removal UI entry points must resolve through `LibraryRemovalIntent` / `library_removal_plan`, not legacy matched/url command `{pattern}`: `{line}`"
                    ));
                }
            }
            for (pattern, note) in SCREEN_LIBRARY_REMOVAL_PRESENTATION_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: {note}; found `{pattern}`: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Library removal entry-point violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn app_shell_hosts_gpui_component_root_layers() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    assert!(
        app_source.contains("render_window_layers(window, cx)"),
        "app shell must render shared GPUI Root layers so open_dialog/open_sheet/notifications become visible"
    );

    let layer_source = read_source(&manifest_path("src/ui/shells/window_layers.rs"));
    for pattern in [
        "Root::render_dialog_layer",
        "Root::render_sheet_layer",
        "Root::render_notification_layer",
    ] {
        assert!(
            layer_source.contains(pattern),
            "window layer shell must host `{pattern}`"
        );
    }
}

#[test]
fn library_removal_confirmation_presentation_has_shell_owner() {
    let composite_source = read_source(&manifest_path("src/ui/composites/confirmation_dialog.rs"));
    for forbidden in [
        "LibraryRemovalConfirmationDisplay",
        "view_models::library_removal",
        "library_removal_confirmation_dialog",
    ] {
        assert!(
            !composite_source.contains(forbidden),
            "generic confirmation dialog composite must stay domain-agnostic; found `{forbidden}`"
        );
    }

    let shell_source = read_source(&manifest_path(
        "src/ui/shells/library_removal_confirmation.rs",
    ));
    for required in [
        "LibraryRemovalConfirmationDisplay",
        "confirmation_dialog(",
        "window.open_dialog",
    ] {
        assert!(
            shell_source.contains(required),
            "library-removal confirmation shell must own `{required}`"
        );
    }
}

#[test]
fn screens_do_not_call_migrated_feed_update_paths() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in SCREEN_METADATA_FEED_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: migrated feed-update workflows must go through ADR 0024 commands/queries, not `{pattern}`: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0024 feed-update screen boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_call_migrated_playback_paths() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in SCREEN_PLAYBACK_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: migrated playback workflows must go through ADR 0024 commands/queries, not `{pattern}`: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0024 playback screen boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn adr_0047_membership_buttons_use_download_remove_vocabulary() {
    let checked_files = [
        "src/view_models/library.rs",
        "src/view_models/entity_detail.rs",
        "src/ui/shells/discover/actions.rs",
        "src/ui/shells/library/feed_detail.rs",
        "src/ui/shells/library/track_detail.rs",
        "src/ui/shells/library/track_detail_metadata.rs",
    ];
    let forbidden = [
        "\"Subscribe Feed\"",
        "\"Unsubscribe Feed\"",
        "\"Subscribe Track\"",
        "\"Unsubscribe Track\"",
        "\"Subscribing...\"",
        "\"Unsubscribing...\"",
        "\"Subscribing track...\"",
        "\"Unsubscribing track...\"",
    ];
    let mut violations = Vec::new();

    for file in checked_files {
        let path = manifest_path(file);
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for pattern in forbidden {
            if contents.contains(pattern) {
                violations.push(format!(
                    "{file}: membership action buttons must use Download/Remove vocabulary; found `{pattern}`"
                ));
            }
        }
    }
    for (file, contents) in search_vm_sources() {
        for pattern in forbidden {
            if contents.contains(pattern) {
                violations.push(format!(
                    "{file}: membership action buttons must use Download/Remove vocabulary; found `{pattern}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0047 membership action vocabulary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_add_unapproved_hardcoded_dark_defaults() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if (line.contains("Appearance::Dark") || line.contains("ThemeProfile::Dark"))
                && !appearance_dark_is_approved(file, &source, line_number)
            {
                violations.push(format!(
                    "{file}:{line_number}: hardcoded dark theme default needs an explicit architecture-test approval: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0023 hardcoded appearance violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_grow_deprecated_visual_helper_usage() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for baseline in DEPRECATED_VISUAL_HELPER_BASELINES {
            if baseline.file == file {
                let count = deprecated_helper_count(&source, baseline);
                if count > baseline.max_count {
                    violations.push(format!(
                        "{file}: `{}` usage grew from allowed baseline {} to {count}; migrate to ADR 0025 tokens/profiles/icons/control roles instead",
                        baseline.helper, baseline.max_count
                    ));
                }
            }
        }
        for helper in DEPRECATED_VISUAL_HELPERS {
            if deprecated_helper_has_baseline(file, helper.helper) {
                continue;
            }
            let source_imports_helper = source.lines().map(strip_line_comment).any(|line| {
                helper
                    .import_patterns
                    .iter()
                    .any(|pattern| line.contains(pattern))
            });
            for (line_number, line) in code_lines(&source) {
                let imports_helper = helper
                    .import_patterns
                    .iter()
                    .any(|pattern| line.contains(pattern));
                if imports_helper || (source_imports_helper && line.contains(helper.usage_pattern))
                {
                    violations.push(format!(
                        "{file}:{line_number}: new screen usage of deprecated `{}` helper is not allowed: `{line}`",
                        helper.helper
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 deprecated visual helper violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_define_inline_icon_svg_helpers() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("ImageFormat::Svg") || line.contains("<svg") {
                violations.push(format!(
                    "{file}:{line_number}: screen-level inline SVG icons must move behind `ui::icons`: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 inline icon SVG violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_buttons_do_not_reintroduce_raw_leading_glyphs() {
    let mut violations = Vec::new();
    for path in rust_files_under("src/ui") {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("leading_glyph") {
                violations.push(format!(
                    "{}:{line_number}: button leading icons must use `IconName`, not raw glyphs: `{line}`",
                    rel_path(&path)
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 button icon role violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_components_do_not_bypass_theme_profile_resolution() {
    let mut violations = Vec::new();
    for relative_dir in ["src/ui/primitives", "src/ui/composites"] {
        for path in rust_files_under(relative_dir) {
            let source = read_source(&path);
            for (line_number, line) in code_lines(&source) {
                if line.contains("Appearance::current(cx)") {
                    violations.push(format!(
                        "{}:{line_number}: use `tokens::color` or `resolve_color` so active `ThemeProfile` is honored: `{line}`",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 theme-profile bypass violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_style_does_not_reintroduce_layout_namespace() {
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("ui::style::layout")
                || line.contains("use crate::ui::style::layout")
                || (line.contains("style::{") && line.contains("layout"))
                || (rel_path(&path) == "src/ui/style.rs" && line.contains("pub mod layout"))
            {
                violations.push(format!(
                    "{}:{line_number}: fixed layout geometry belongs in `ui::layouts`, not `ui::style::layout`: `{line}`",
                    rel_path(&path)
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 layout-boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_style_does_not_reintroduce_status_roles() {
    let path = manifest_path("src/ui/style.rs");
    let source = read_source(&path);
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        if line.contains("StatusRole")
            || line.contains("status_success")
            || line.contains("status_warning")
            || line.contains("status_danger")
        {
            violations.push(format!(
                "src/ui/style.rs:{line_number}: status color and glyph semantics belong in typed UI roles, not `ui::style`: `{line}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 status-role boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_style_does_not_reintroduce_provenance_diff_roles() {
    let path = manifest_path("src/ui/style.rs");
    let source = read_source(&path);
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        if line.contains("diff_match")
            || line.contains("diff_different")
            || line.contains("diff_missing")
        {
            violations.push(format!(
                "src/ui/style.rs:{line_number}: provenance/diff color and glyph semantics belong in `ProvenanceRole`, not `ui::style`: `{line}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 provenance-role style-boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn interactive_composites_carry_accessibility_labels() {
    // ADR 0038 task 005: every interactive composite must expose an
    // `a11y_label` (or a named action-group a11y label) on its display
    // contract so view-models own the accessibility surface alongside
    // visible labels. Do not remove entries.
    let migrated_composites: &[(&str, &str, &str)] = &[
        (
            "ActionButtonDisplay",
            "src/ui/composites/action_button.rs",
            "a11y_label",
        ),
        (
            "ConfirmationDialogDisplay",
            "src/ui/composites/confirmation_dialog.rs",
            "confirm_a11y_label",
        ),
        (
            "LibraryRemovalConfirmationDisplay",
            "src/view_models/library_removal.rs",
            "remove_a11y_label",
        ),
        (
            "IdentityActionButtonDisplay",
            "src/ui/composites/identity_action.rs",
            "a11y_label",
        ),
        (
            "ActionRowDisplay",
            "src/ui/composites/action_row.rs",
            "a11y_label",
        ),
        (
            "AddToPlaylistDisplay",
            "src/ui/composites/playlist_popover.rs",
            "trigger_a11y_label",
        ),
        (
            "PlaylistOptionDisplay",
            "src/ui/composites/playlist_popover.rs",
            "a11y_label",
        ),
        (
            "TrackRowDisplay",
            "src/ui/composites/track_row.rs",
            "a11y_label",
        ),
        ("ListRow", "src/ui/composites/list_row.rs", "a11y_label"),
        (
            "RecentFeedTileDisplay",
            "src/view_models/search/recent.rs",
            "a11y_label",
        ),
        (
            "DisclosureGroupDisplay",
            "src/ui/composites/disclosure_group.rs",
            "a11y_label",
        ),
        (
            "SegmentDisplay",
            "src/ui/composites/segmented_control.rs",
            "a11y_label",
        ),
        (
            "TransportDisplay",
            "src/view_models/queue_now_playing.rs",
            "play_pause_a11y_label",
        ),
        (
            "ReleaseDetailPageVm",
            "src/view_models/entity_detail.rs",
            "actions_a11y_label",
        ),
        (
            "ReleaseDetailSurface",
            "src/ui/composites/release_detail_surface.rs",
            "actions_a11y_label",
        ),
        (
            "TrackDetailSurface",
            "src/ui/composites/track_detail_surface.rs",
            "primary_actions_a11y_label",
        ),
        (
            "TrackRowVm",
            "src/view_models/track_detail.rs",
            "a11y_label",
        ),
    ];

    let mut violations = Vec::new();
    for (display, path, label_field) in migrated_composites {
        let source = read_source(&manifest_path(path));
        let exact_struct = format!("struct {display} ");
        let braced_struct = format!("struct {display} {{");
        let generic_struct = format!("struct {display}<");
        let Some(struct_offset) = source
            .find(&exact_struct)
            .or_else(|| source.find(&braced_struct))
            .or_else(|| source.find(&generic_struct))
        else {
            violations.push(format!("{path}: expected `struct {display}` to exist"));
            continue;
        };

        let after = &source[struct_offset..];
        let Some(open) = after.find('{') else {
            violations.push(format!("{path}: `{display}` has no struct body"));
            continue;
        };
        let body_start = struct_offset + open;
        let close = source[body_start..]
            .find('}')
            .map_or(source.len(), |i| body_start + i);
        let body = &source[body_start..close];

        if !body.contains(label_field) {
            violations.push(format!(
                "{path}: `{display}` must declare `{label_field}` on its display contract (ADR 0038 task 005)"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 task 005 a11y-label coverage violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ui_style_resolves_colors_through_token_layer() {
    let path = manifest_path("src/ui/style.rs");
    let source = read_source(&path);
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        if line.contains("gpui::rgb(") || line.contains("rgb(0x") {
            violations.push(format!(
                "src/ui/style.rs:{line_number}: raw rgb literals must move to `tokens::SemanticColor`; resolve via `role(...)`: `{line}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 task 004 style-layer raw-color violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_grow_unmarked_direct_component_button_usage() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        let unmarked = unmarked_direct_component_button_lines(&source);
        let max_unmarked_count = direct_component_button_baseline(file).unwrap_or(0);
        if unmarked.len() > max_unmarked_count {
            violations.push(format!(
                "{file}: unmarked direct `gpui_component::Button` usage grew from allowed baseline {max_unmarked_count} to {}",
                unmarked.len()
            ));
            for (line_number, line) in unmarked {
                violations.push(format!(
                    "{file}:{line_number}: direct `gpui_component::Button` compatibility usage needs preceding or same-line `CONTROL-COMPAT(reason): ...`: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 direct component button compatibility violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_grow_loose_provenance_diff_helpers() {
    let mut violations = Vec::new();
    for baseline in PROVENANCE_DIFF_HELPER_BASELINES {
        let path = manifest_path(baseline.file);
        let source = read_source(&path);
        let count = source
            .lines()
            .map(strip_line_comment)
            .filter(|line| line.contains(baseline.pattern))
            .count();
        if count > baseline.max_count {
            violations.push(format!(
                "{}: loose provenance/diff helper `{}` grew from allowed baseline {} to {count}; use `ui::composites::ProvenanceRole` instead",
                baseline.file, baseline.pattern, baseline.max_count
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0025 provenance/diff helper violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_grow_screen_local_playlist_popover_panels() {
    let mut violations = Vec::new();
    for baseline in SCREEN_LOCAL_PLAYLIST_POPOVER_BASELINES {
        let path = manifest_path(baseline.file);
        let source = read_source(&path);
        let matches = source
            .lines()
            .map(strip_line_comment)
            .filter(|line| line.contains(baseline.pattern))
            .count();
        if matches > baseline.max_count {
            violations.push(format!(
                "{}: screen-local playlist popover pattern `{}` grew from allowed baseline {} to {matches} ({})",
                baseline.file, baseline.pattern, baseline.max_count, baseline.note
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0032 playlist popover ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_duplicate_render_helpers_without_baseline() {
    let mut helpers: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            if let Some(helper) = render_helper_name(&line) {
                helpers
                    .entry(helper)
                    .or_default()
                    .push(format!("{file}:{line_number}"));
            }
        }
    }

    let mut violations = Vec::new();
    for (helper, locations) in helpers {
        let distinct_files = distinct_location_files(&locations);
        if distinct_files.len() < 2 {
            continue;
        }
        match render_helper_duplication_baseline(&helper) {
            Some(baseline) if same_file_set(&distinct_files, baseline.files) => {}
            Some(baseline) => violations.push(format!(
                "`{helper}` appears in [{}], but its baseline `{}` allows only [{}] ({})",
                distinct_files.join(", "),
                baseline.helper,
                baseline.files.join(", "),
                baseline.note
            )),
            None => violations.push(format!(
                "`{helper}` appears in multiple screen files at [{}]; move the shared affordance into `src/ui/primitives` or `src/ui/composites` instead of copying render helpers",
                locations.join(", ")
            )),
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 render-helper duplication violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_inline_value_route_recipient_label_fallbacks() {
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let source = read_source(&manifest_path(file));
        assert!(
            !source.contains(".get(\"recipient_name\")"),
            "{file}: value-route recipient display labels must be projected by `view_models::metadata::value_route_recipient_label`, not rebuilt in screen code"
        );
    }
}

#[test]
fn shared_top_level_ui_shells_do_not_import_screen_modules() {
    let forbidden = ["crate::search", "crate::library", "SearchApp", "LibraryApp"];
    let screen_free_shells = [
        "src/ui/shells/artist.rs",
        "src/ui/shells/entity.rs",
        "src/ui/shells/playlist.rs",
    ];

    let mut violations = Vec::new();
    for file in screen_free_shells {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            if let Some(pattern) = forbidden.iter().find(|pattern| line.contains(**pattern)) {
                violations.push(format!(
                    "{file}:{line_number}: shared top-level UI shells must not depend on screen modules; found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 shared UI shell boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn library_screen_modules_are_decomposed_under_src_ui_shells_library() {
    assert_screen_surface_files("Library", LIBRARY_SCREEN_SURFACE_FILES);
}

#[test]
fn discover_screen_modules_are_decomposed_under_src_ui_shells_discover() {
    assert_screen_surface_files("Discover", DISCOVER_SCREEN_SURFACE_FILES);
}

#[test]
fn screen_entry_modules_under_500_loc() {
    let ceilings = [("src/library.rs", 500), ("src/discover.rs", 500)];
    let mut violations = Vec::new();

    for (file, ceiling) in ceilings {
        let source = read_source(&manifest_path(file));
        let loc = code_lines(&source).count();
        if loc > ceiling {
            violations.push(format!("{file} exceeds {ceiling} LOC ceiling: {loc}"));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 Task 007 screen entry module LOC violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn surface_modules_under_500_loc() {
    let mut violations = Vec::new();

    for dir in SCREEN_SURFACE_DIRS {
        for path in rust_files_under(dir) {
            let file = rel_path(&path);
            if file.ends_with("/mod.rs") {
                continue;
            }
            let source = read_source(&path);
            let loc = code_lines(&source).count();
            if loc > 500 {
                violations.push(format!("{file} exceeds 500 LOC ceiling: {loc}"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 Task 007 screen surface module LOC violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn library_release_detail_playlist_popovers_use_shared_composite() {
    let files = [
        "src/library.rs",
        "src/ui/shells/library/feed_detail.rs",
        "src/ui/shells/library/track_detail_metadata.rs",
    ];
    let mut violations = Vec::new();
    let mut shared_popover_count = 0;

    for file in files {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for pattern in RELEASE_PLAYLIST_POPOVER_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: ADR 0032 Library release-detail playlist chrome must use `AddToPlaylistPopover`, not `{pattern}`: `{line}`"
                    ));
                }
            }
        }

        shared_popover_count += source
            .lines()
            .map(strip_line_comment)
            .filter(|line| line.contains("AddToPlaylistPopover::new("))
            .count();
    }
    if shared_popover_count < 2 {
        violations.push(format!(
            "Library release-detail shells: ADR 0032 expects feed and track playlist actions to use `AddToPlaylistPopover`; found {shared_popover_count} call(s)"
        ));
    }

    assert!(
        violations.is_empty(),
        "ADR 0032 Library release-detail playlist popover violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn playlist_popover_calls_wire_create_mode() {
    let mut violations = Vec::new();

    for file in PLAYLIST_POPOVER_CALLSITE_FILES {
        let path = manifest_path(file);
        let source = read_source(&path);
        let lines: Vec<&str> = source.lines().collect();
        for (index, raw) in lines.iter().enumerate() {
            let line = strip_line_comment(raw);
            if !line.contains("AddToPlaylistPopover::new(") {
                continue;
            }
            let next_call = lines[index + 1..]
                .iter()
                .position(|candidate| {
                    strip_line_comment(candidate).contains("AddToPlaylistPopover::new(")
                })
                .map_or(lines.len(), |offset| index + 1 + offset);
            let end = (index + 80).min(next_call).min(lines.len());
            let chain = lines[index..end]
                .iter()
                .map(|candidate| strip_line_comment(candidate))
                .collect::<Vec<_>>()
                .join("\n");
            if !chain.contains(".on_create(") {
                violations.push(format!(
                    "{file}:{}: ADR 0032 playlist popovers must expose `+ New Playlist` via `.on_create(...)`: `{}`",
                    index + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0032 playlist popover create-mode violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn playlist_popover_menu_rows_use_leading_alignment_and_token_padding() {
    let source = read_source(&manifest_path("src/ui/composites/playlist_popover.rs"));
    let mut violations = Vec::new();

    if !source.contains(".surface_padding(Spacing::SM)") {
        violations.push(
            "src/ui/composites/playlist_popover.rs: playlist popover surface padding must use the shared compact menu token `Spacing::SM`".to_string(),
        );
    }

    let leading_alignment_count = source.matches(".align_leading()").count();
    if leading_alignment_count < 3 {
        violations.push(format!(
            "src/ui/composites/playlist_popover.rs: playlist menu rows, create command, and back command must be leading-aligned; found {leading_alignment_count} `.align_leading()` call(s)"
        ));
    }

    assert!(
        violations.is_empty(),
        "ADR 0036 playlist popover visual-system violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn release_detail_surface_uses_scale_aware_spacing_tokens() {
    let source = read_source(&manifest_path(
        "src/ui/composites/release_detail_surface.rs",
    ));
    let mut violations = Vec::new();

    for forbidden in [
        "crate::ui::style",
        "spacing::",
        "typography::",
        "color::text_",
    ] {
        if source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/release_detail_surface.rs: release detail surface must use scale-aware tokens, not legacy `{forbidden}`"
            ));
        }
    }

    for required in [
        "Spacing::LG.scaled(cx)",
        "Spacing::SM.scaled(cx)",
        "FontSize::Caption.scaled(cx)",
        "color(cx, SemanticColor::TertiaryLabel)",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "src/ui/composites/release_detail_surface.rs: expected scale-aware token usage `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0036 release detail visual-system violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn library_advanced_provenance_cells_use_shared_grid_composites() {
    let files = [
        "src/ui/shells/library/track_detail_metadata_cells.rs",
        "src/ui/shells/library/track_detail_metadata_values.rs",
    ];
    let source = files
        .iter()
        .map(|file| read_source(&manifest_path(file)))
        .collect::<Vec<_>>()
        .join("\n");
    let mut violations = Vec::new();

    for required in [
        "TrackMetadataGroupCell::new",
        "TrackMetadataFieldCell::new",
        "TrackMetadataSourceCell::new",
        "TrackMetadataTagCell::new",
        "TrackMetadataTextValue::new",
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "Library metadata shell: advanced Library provenance grid must use shared `{required}` grammar"
            ));
        }
    }

    for file in files {
        let file_source = read_source(&manifest_path(file));
        for forbidden in [
            "w(layout::COMPACT_COLUMN_WIDTH)",
            "w(layout::METADATA_LABEL_WIDTH)",
        ] {
            if file_source.contains(forbidden) {
                violations.push(format!(
                    "{file}: advanced provenance cell widths belong in `src/ui/composites/track_metadata_grid.rs`, not screen-local `{forbidden}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0036 advanced provenance panel ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn discovery_recent_tiles_use_shared_composite() {
    let source = read_source(&manifest_path("src/ui/shells/discover/recent.rs"));
    let start = source
        .find("fn render_discover_recent(")
        .expect("Discover recent-feed renderer should exist");
    let body = &source[start..];
    let mut violations = Vec::new();

    if !body.contains("RecentFeedTile::new(") {
        violations.push(
            "src/ui/shells/discover/recent.rs: render_discover_recent must compose `RecentFeedTile`"
                .to_string(),
        );
    }

    for pattern in [
        "Label::new(title)",
        "Label::new(artist)",
        "EntityKind::Feed.emoji()",
        "layout::THUMBNAIL_XL",
        "child(\"...\")",
    ] {
        if body.contains(pattern) {
            violations.push(format!(
                "src/ui/shells/discover/recent.rs: render_discover_recent must not own recent tile chrome or placeholder labels; found `{pattern}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 Discovery recent tile ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_inline_unknown_artist_or_album_fallbacks() {
    let forbidden = ["\"Unknown Artist\"", "\"Unknown Album\""];
    let mut violations = Vec::new();

    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: fallback display labels belong in view-models, not screens; found `{pattern}` in `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 screen fallback-label violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_inline_untitled_fallback() {
    let forbidden = ["\"Untitled\"", "\"[untitled]\""];
    let mut violations = Vec::new();

    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: title fallback labels belong in view-models, not screens; found `{pattern}` in `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 screen title-fallback violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn track_detail_labels_owns_canonical_field_labels() {
    let canonical_labels = ["Album", "Feed", "Release", "Tags"];
    let render_call_patterns = [
        "Label::new(",
        "text(",
        "SectionHeader::new(",
        "DetailRow::text(",
        ".label(",
        ".title(",
    ];
    let allowed_composites = [
        "src/ui/composites/track_detail_surface.rs",
        "src/ui/composites/track_row.rs",
    ];
    let mut violations = Vec::new();

    let screen_paths = SCREEN_FILES.iter().map(|file| manifest_path(file)).chain(
        rust_files_under("src/ui/composites")
            .into_iter()
            .filter(|path| {
                let rel = rel_path(path);
                !allowed_composites.contains(&rel.as_str())
            }),
    );

    for path in screen_paths {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if !render_call_patterns
                .iter()
                .any(|pattern| line.contains(pattern))
            {
                continue;
            }
            for label in canonical_labels {
                let literal = format!("\"{label}\"");
                if line.contains(&literal) {
                    violations.push(format!(
                        "{file}:{line_number}: track detail label `{label}` belongs in `TrackDetailLabels`, not local render code: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0035 track-detail label ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn track_surface_slots_are_typed() {
    let files = [
        "src/ui/composites/track_detail_surface.rs",
        "src/ui/composites/track_row.rs",
    ];
    let slot_method_markers = [
        "primary_actions(",
        "external_links(",
        "sections(",
        "section_elements(",
        "advanced_panels(",
        "from_vm(",
    ];
    let forbidden = ["AnyElement", "impl IntoElement", "gpui::IntoElement"];
    let mut violations = Vec::new();

    for file in files {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            if !line.contains("pub fn")
                || !slot_method_markers
                    .iter()
                    .any(|marker| line.contains(marker))
            {
                continue;
            }
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: ADR 0035 track surface slot APIs must be typed, not `{pattern}`: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0035 typed track-surface slot violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn release_surface_slots_are_typed() {
    let forbidden = [
        (
            "src/ui/composites/release_detail_surface.rs",
            "header: Option<AnyElement>",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "actions: Option<AnyElement>",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "details: Option<AnyElement>",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "panels: Vec<AnyElement>",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "section_rows: Vec<AnyElement>",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "after_section: Vec<AnyElement>",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "pub fn header(mut self, header: AnyElement)",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "pub fn actions(mut self, actions: AnyElement)",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "pub fn details(mut self, details: AnyElement)",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "pub fn panel(mut self, panel: AnyElement)",
        ),
        (
            "src/ui/composites/release_detail_surface.rs",
            "pub fn after_section(mut self, child: AnyElement)",
        ),
        ("src/ui/shells/entity.rs", "pub actions: Vec<AnyElement>"),
        ("src/ui/shells/entity.rs", "pub popover: Option<AnyElement>"),
        (
            "src/ui/shells/entity.rs",
            "pub primary_actions: Vec<AnyElement>",
        ),
        (
            "src/ui/shells/entity.rs",
            "pub identity_actions: Vec<AnyElement>",
        ),
        (
            "src/ui/shells/entity.rs",
            "pub action_overlays: Vec<AnyElement>",
        ),
        (
            "src/ui/shells/entity.rs",
            "pub track_rows: Option<Vec<AnyElement>>",
        ),
        (
            "src/ui/shells/entity.rs",
            "pub after_section: Vec<AnyElement>",
        ),
    ];
    let mut violations = Vec::new();

    for (file, pattern) in forbidden {
        let source = read_source(&manifest_path(file));
        if source.contains(pattern) {
            violations.push(format!(
                "{file}: ADR 0036 release surface slots must use `ReleaseSurfaceElement`, not `{pattern}`"
            ));
        }
    }

    for file in [
        "src/ui/composites/release_detail_surface.rs",
        "src/ui/shells/entity.rs",
    ] {
        let source = read_source(&manifest_path(file));
        if !source.contains("ReleaseSurfaceElement") {
            violations.push(format!(
                "{file}: ADR 0036 release surface slot boundary must name `ReleaseSurfaceElement`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0036 typed release-surface slot violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn release_feed_identity_actions_use_shared_renderer() {
    let mut violations = Vec::new();

    for file in ["src/ui/shells/feed.rs", "src/library.rs"] {
        let source = read_source(&manifest_path(file));
        if source.contains("IdentityActionKind::Rss") {
            violations.push(format!(
                "{file}: ADR 0037 feed RSS identity actions must be rendered by `ui::shells::entity::render_feed_identity_actions`"
            ));
        }
    }

    let ui_entity = read_source(&manifest_path("src/ui/shells/entity.rs"));
    if !ui_entity.contains("fn render_feed_identity_actions") {
        violations.push(
            "src/ui/shells/entity.rs: ADR 0037 must define `fn render_feed_identity_actions`"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0037 feed identity renderer violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn track_identity_links_use_shared_renderer() {
    let mut violations = Vec::new();

    let search = read_source(&manifest_path("src/ui/shells/discover/track_inspector.rs"));
    if search.contains("fn render_nostr_icon_button") {
        violations.push(
            "src/ui/shells/discover/track_inspector.rs: ADR 0037 track Nostr identity links must not keep a screen-local Nostr button renderer"
                .to_string(),
        );
    }
    if search.contains("render_nostr_icon_button(npub, \"track\"") {
        violations.push(
            "src/ui/shells/discover/track_inspector.rs: ADR 0037 track Nostr identity links must be rendered by `ui::shells::track::render_track_page_identity_actions`"
                .to_string(),
        );
    }
    if !(search.contains("render_track_page_identity_actions(&detail_page)")
        && !search.contains("\"discover-track\""))
    {
        violations.push(
            "src/ui/shells/discover/track_inspector.rs: ADR 0037 Discover track detail must call `render_track_page_identity_actions(&detail_page)` and leave the prefix in TrackDetailPageVm"
                .to_string(),
        );
    }

    let library = read_source(&manifest_path("src/ui/shells/library/track_detail.rs"));
    if !(library.contains("render_track_page_identity_actions(&detail_page)")
        && !library.contains("\"library-track\""))
    {
        violations.push(
            "src/ui/shells/library/track_detail.rs: ADR 0037 Library track detail must call `render_track_page_identity_actions(&detail_page)` and leave the prefix in TrackDetailPageVm"
                .to_string(),
        );
    }

    let ui_track = read_source(&manifest_path("src/ui/shells/track.rs"));
    if !ui_track.contains("fn render_track_page_identity_actions") {
        violations.push(
            "src/ui/shells/track.rs: ADR 0037 must define `fn render_track_page_identity_actions`"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0037 track identity renderer violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_define_local_track_detail_surface_chrome() {
    let forbidden = [
        "TrackHeader::new(",
        "TrackHeaderVm::new(",
        "key: \"Release\"",
        "key: \"Track #\"",
        "key: \"Duration\"",
        "key: \"Publisher\"",
    ];
    let mut violations = Vec::new();

    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: track-detail chrome must be owned by `TrackDetailSurface`, not rebuilt in screens; found `{pattern}` in `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0035 screen-local track detail surface chrome violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_define_local_track_row_chrome() {
    let forbidden = ["TrackRow::new(", "TrackRowComposite::new("];
    let mut violations = Vec::new();

    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                let mut search_start = 0;
                while let Some(idx) = line[search_start..].find(pattern) {
                    let absolute = search_start + idx;
                    // Skip if the match is the suffix of a longer identifier
                    // (e.g. `SkeletonTrackRow::new(` ends with `TrackRow::new(`).
                    let preceded_by_ident =
                        absolute > 0 && line.as_bytes()[absolute - 1].is_ascii_alphanumeric();
                    if !preceded_by_ident {
                        violations.push(format!(
                            "{file}:{line_number}: track row chrome must be owned by `TrackRow` through `TrackRowVm`, not locally rebuilt; found `{pattern}` in `{line}`"
                        ));
                    }
                    search_start = absolute + pattern.len();
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0035 screen-local track row chrome violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_construct_track_inspector_pane_locally() {
    let forbidden = ["TrackHeader::new(", "TrackHeaderVm::new("];
    let mut violations = Vec::new();

    for file in ["src/discover.rs", "src/library.rs"] {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: track inspector pane chrome belongs in `TrackInspectorPane` / `TrackDetailSurface`, not screen code: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0035 track inspector pane ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn track_surface_consumers_use_track_detail_vm() {
    let consumers = [
        (
            "src/library.rs",
            "TrackDetailSurface::new(",
            "TrackDetailVm::new(",
        ),
        (
            "src/discover.rs",
            "TrackDetailSurface::new(",
            "TrackDetailVm::new(",
        ),
        (
            "src/ui/shells/track.rs",
            "TrackRow::from_vm(",
            "TrackDetailVm::new(",
        ),
    ];
    let mut violations = Vec::new();

    for (file, consumer, required_vm) in consumers {
        let source = read_source(&manifest_path(file));
        if source.contains(consumer) && !source.contains(required_vm) {
            violations.push(format!(
                "{file}: `{consumer}` consumers must be fed from the `TrackDetailVm` family"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0035 track surface VM consumption violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn entity_detail_pages_render_through_shell_helper_and_page_vm() {
    let consumers = [
        (
            "Library release detail",
            "src/ui/shells/library/feed_detail.rs",
            "ReleaseDetailVm::new(",
            ".page()",
            "render_release_detail_shell(&page",
        ),
        (
            "Discover release detail",
            "src/ui/shells/feed.rs",
            "ReleaseDetailVm::new(",
            ".page()",
            "render_release_detail_shell(&page",
        ),
        (
            "Library track detail",
            "src/ui/shells/library/track_detail.rs",
            "TrackDetailVm::new(",
            ".page()",
            "track::build_track_detail_surface(",
        ),
        (
            "Discover track detail",
            "src/ui/shells/discover/track_inspector.rs",
            "TrackDetailVm::new(",
            ".page()",
            "track::build_track_detail_surface(",
        ),
        (
            "Library artist detail",
            "src/ui/shells/library/feed_list.rs",
            "LibraryArtistDetailVm::with_view(",
            ".page()",
            "render_artist_detail_shell(",
        ),
        (
            "Library playlist detail",
            "src/ui/shells/library/playlist_detail.rs",
            "PlaylistDetailVm::new(",
            ".page(",
            "render_playlist_detail_shell(",
        ),
        (
            "Discover artist detail",
            "src/ui/shells/artist.rs",
            "ArtistVm::new(",
            ".page()",
            "render_artist_detail_shell(",
        ),
    ];
    let mut violations = Vec::new();

    for (surface, file, vm_ctor, page_call, shell_helper) in consumers {
        let source = read_source(&manifest_path(file));
        if !(source.contains(vm_ctor)
            && source.contains(page_call)
            && source.contains(shell_helper))
        {
            violations.push(format!(
                "{file}: {surface} must construct a PageVm and render through `{shell_helper}`"
            ));
        }
    }

    for file in ["src/library.rs", "src/discover.rs"] {
        let source = read_source(&manifest_path(file));
        if source.contains("TrackDetailSurface::new(") {
            violations.push(format!(
                "{file}: track screens must not construct `TrackDetailSurface`; use `ui::shells::track::build_track_detail_surface`"
            ));
        }
    }

    let track_vm = read_source(&manifest_path("src/view_models/track_detail.rs"));
    if !track_vm.contains("pub struct TrackDetailPageVm") {
        violations.push(
            "src/view_models/track_detail.rs: Task 006 requires `TrackDetailPageVm`".to_string(),
        );
    }

    let artist_vm = read_source(&manifest_path("src/view_models/artist_detail.rs"));
    if !artist_vm.contains("pub struct ArtistDetailPageVm") {
        violations.push(
            "src/view_models/artist_detail.rs: Task 006 requires `ArtistDetailPageVm`".to_string(),
        );
    }

    let playlist_vm = read_source(&manifest_path("src/view_models/playlist_detail.rs"));
    if !playlist_vm.contains("pub(crate) struct PlaylistDetailPageVm") {
        violations.push(
            "src/view_models/playlist_detail.rs: Task 006 requires `PlaylistDetailPageVm`"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 Task 006 page VM shell violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn release_surface_consumers_use_release_detail_vm() {
    let consumers = [
        (
            "src/library.rs",
            "render_release_detail_shell(",
            "ReleaseDetailVm::new(",
        ),
        (
            "src/ui/shells/feed.rs",
            "render_release_detail_shell(",
            "ReleaseDetailVm::new(",
        ),
    ];
    let mut violations = Vec::new();

    for (file, consumer, required_vm) in consumers {
        let source = read_source(&manifest_path(file));
        if source.contains(consumer) && !source.contains(required_vm) {
            violations.push(format!(
                "{file}: `{consumer}` consumers must be fed from `ReleaseDetailVm`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0036 release surface VM consumption violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn screens_do_not_coerce_empty_feed_url_to_empty_string() {
    let mut violations = Vec::new();

    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            if line.contains("feed_url") && line.contains("unwrap_or_default") {
                violations.push(format!(
                    "{file}:{line_number}: feed URL display/default policy belongs in a view-model, not screen coercion: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 feed URL fallback violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn view_models_own_display_fallbacks_for_library_and_search() {
    let forbidden = [
        (
            "src/discover.rs",
            "feed_link_label.unwrap_or_else",
            "Discover track feed-link label fallback belongs in TrackInspectorHeaderVm::feed_link_display",
        ),
        (
            "src/discover.rs",
            "header_vm.feed_link_label(",
            "Discover track feed-link label should enter the screen through TrackFeedLinkDisplay",
        ),
        (
            "src/discover.rs",
            "header_vm.feed_link_url()",
            "Discover track feed-link URL should enter the screen through TrackFeedLinkDisplay",
        ),
        (
            "src/discover.rs",
            "let tooltip = guid.clone();",
            "Discover track feed-link tooltip should enter the screen through TrackFeedLinkDisplay",
        ),
        (
            "src/discover.rs",
            "route.address.clone().unwrap_or_default()",
            "payment-route address presence belongs in PaymentRouteVm::address",
        ),
        (
            "src/discover.rs",
            "route.address.is_some()",
            "payment-route address presence belongs in PaymentRouteVm::address",
        ),
        (
            "src/discover.rs",
            "route.custom_key.is_some()",
            "payment-route custom field presence belongs in PaymentRouteVm::custom_fields",
        ),
        (
            "src/discover.rs",
            "route.custom_value.is_some()",
            "payment-route custom field presence belongs in PaymentRouteVm::custom_fields",
        ),
        (
            "src/discover.rs",
            "&route.custom_key",
            "payment-route custom field display belongs in PaymentRouteVm::custom_fields",
        ),
        (
            "src/discover.rs",
            "&route.custom_value",
            "payment-route custom field display belongs in PaymentRouteVm::custom_fields",
        ),
        (
            "src/discover.rs",
            "vm.recipient_name()",
            "payment-route primary summary belongs in PaymentRouteVm::summary",
        ),
        (
            "src/discover.rs",
            "vm.route_type()",
            "payment-route primary summary belongs in PaymentRouteVm::summary",
        ),
        (
            "src/discover.rs",
            "vm.kind_label()",
            "payment-route primary summary belongs in PaymentRouteVm::summary",
        ),
        (
            "src/discover.rs",
            "let split = vm.split()",
            "payment-route primary summary belongs in PaymentRouteVm::summary",
        ),
        (
            "src/discover.rs",
            "feed.feed_guid.clone().unwrap_or_default()",
            "Discover feed-list tile id fallback belongs in RecentFeedTileVm::display",
        ),
        (
            "src/discover.rs",
            "feed.tracks.clone().unwrap_or_default()",
            "Discover feed-inspector missing-track fallback belongs in SearchViewModel::feed_inspector_tracks",
        ),
        (
            "src/discover.rs",
            "let episode_note =",
            "Discover feed-list episode note belongs in RecentFeedTileVm::display",
        ),
        (
            "src/discover.rs",
            "Label::new(feed_display_title(&feed))",
            "Discover feed-list title fallback belongs in RecentFeedTileVm::display",
        ),
        (
            "src/discover.rs",
            "let guid = display.id.clone()",
            "Discover feed-list navigation id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/discover.rs",
            "SharedString::from(display.feed_list_tile_id)",
            "Discover feed-list tile id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/discover.rs",
            "let click_guid = display.id.clone()",
            "Discover podroll tile id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/discover.rs",
            "SharedString::from(display.podroll_tile_id)",
            "Discover podroll tile id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/discover.rs",
            "let element_id = link.element_id",
            "Discover track feed-link display should be consumed from TrackFeedLinkDisplay",
        ),
        (
            "src/discover.rs",
            "let title = link.label",
            "Discover track feed-link label should be consumed from TrackFeedLinkDisplay",
        ),
        (
            "src/discover.rs",
            "let display = PublisherLinkDisplay::new",
            "Discover publisher link display should be consumed from PublisherLinkDisplay",
        ),
        (
            "src/discover.rs",
            "let guid = match feed.feed_guid.clone()",
            "Discover recent-feed navigation id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/discover.rs",
            "SharedString::from(audio_display.button_id.clone())",
            "Discover track play-button id should be consumed by the TrackPlayAudioDisplay renderer",
        ),
        (
            "src/discover.rs",
            "display.recent_tile_id.clone()",
            "Discover recent-feed tile id should be consumed by RecentFeedTile",
        ),
        (
            "src/discover.rs",
            "snapshot.status.display_text.clone()",
            "Discover status display text should be consumed from SearchRenderSnapshot",
        ),
        (
            "src/discover.rs",
            "release_subscription_action.label.clone()",
            "Discover feed subscription action label should be consumed from EntityActionVm",
        ),
        (
            "src/discover.rs",
            "action.label.clone()",
            "Discover track row action labels should be consumed from EntityActionVm",
        ),
        (
            "src/discover.rs",
            "self.label.clone()",
            "Discover metadata drag preview should consume TrackMetadataDragPreviewDisplay",
        ),
        (
            "src/discover.rs",
            "self.value.clone()",
            "Discover metadata drag preview should consume TrackMetadataDragPreviewDisplay",
        ),
        (
            "src/discover.rs",
            "self.display.label.clone()",
            "Discover metadata drag preview should consume TrackMetadataDragPreviewDisplay without renderer-side label cloning",
        ),
        (
            "src/discover.rs",
            "self.display.value.clone()",
            "Discover metadata drag preview should consume TrackMetadataDragPreviewDisplay without renderer-side value cloning",
        ),
        (
            "src/discover.rs",
            "LoadingMessage::new(message.clone())",
            "Discover inspector loading text should be consumed without renderer-side message cloning",
        ),
        (
            "src/library.rs",
            "LoadingMessage::new(label.clone())",
            "Library deferred metadata-panel empty labels should be consumed without renderer-side label cloning",
        ),
        (
            "src/library.rs",
            "display.label.clone()",
            "Library metadata group labels should be consumed from TrackMetadataGroupHeadingDisplay",
        ),
        (
            "src/library.rs",
            "label: label.clone()",
            "Library metadata group disclosure labels should be duplicated inside TrackMetadataGroupCell",
        ),
        (
            "src/library.rs",
            "status.text.clone()",
            "Library status display text should be consumed from LibraryStatusSnapshot",
        ),
        (
            "src/library.rs",
            "primary_action.label.clone()",
            "Library track primary-action label should be consumed from EntityActionVm",
        ),
        (
            "src/library.rs",
            "feed_update.status_message.clone()",
            "Library feed-update status should be consumed from FeedUpdateDisplay",
        ),
        (
            "src/library.rs",
            "feed_update.action.clone()",
            "Library feed-update action should be consumed from FeedUpdateDisplay",
        ),
        (
            "src/ui/shells/track.rs",
            "display.payload.clone()",
            "Track identity action payload should be consumed from IdentityActionDisplay",
        ),
        (
            "src/ui/shells/entity.rs",
            "display.payload.clone()",
            "Feed identity action payload should be consumed from IdentityActionDisplay",
        ),
        (
            "src/library.rs",
            "let target_for_click = action.target.clone()",
            "Library contributor identity action target should be consumed from ContributorIdentityActionDisplay",
        ),
        (
            "src/discover.rs",
            "let target_for_click = action.target.clone()",
            "Discover contributor identity action target should be consumed from ContributorIdentityActionDisplay",
        ),
        (
            "src/library.rs",
            "format!(\"{n:02} - \")",
            "Library tree track-number prefix belongs in LibraryTrackRowVm::tree_number_prefix",
        ),
        (
            "src/library.rs",
            "row.rss_value.as_deref().unwrap_or(\"\")",
            "metadata RSS cell value fallback belongs in TrackMetadataGridVm::rss_cell_value",
        ),
        (
            "src/discover.rs",
            "row.rss_value.as_deref().unwrap_or(\"\")",
            "metadata RSS cell value fallback belongs in TrackMetadataGridVm::rss_cell_value",
        ),
        (
            "src/library.rs",
            ".or(row.id3_value.as_deref())",
            "metadata ID3 cell value fallback belongs in TrackMetadataGridVm::id3_cell_value",
        ),
        (
            "src/discover.rs",
            ".or(row.id3_value.as_deref())",
            "metadata ID3 cell value fallback belongs in TrackMetadataGridVm::id3_cell_value",
        ),
        (
            "src/library.rs",
            ".or(row.id3_frame.as_deref())",
            "metadata ID3 cell frame fallback belongs in TrackMetadataGridVm::id3_cell_frame",
        ),
        (
            "src/discover.rs",
            ".or(row.id3_frame.as_deref())",
            "metadata ID3 cell frame fallback belongs in TrackMetadataGridVm::id3_cell_frame",
        ),
        (
            "src/discover.rs",
            "row.id3_frame.clone().unwrap_or_default()",
            "metadata drag frame fallback belongs in TrackMetadataGridVm::id3_drag_frame",
        ),
        (
            "src/discover.rs",
            "frame_id_owned.unwrap_or_default()",
            "metadata ID3 displayed frame label fallback belongs in TrackMetadataGridVm::id3_frame_label",
        ),
        (
            "src/discover.rs",
            "frame_id.unwrap_or_default()",
            "metadata ID3 displayed frame label fallback belongs in TrackMetadataGridVm::id3_frame_label",
        ),
        (
            "src/library.rs",
            ".child(SharedString::from(row.field.clone()))",
            "Library metadata field label display belongs in TrackMetadataGridVm::field_label",
        ),
        (
            "src/discover.rs",
            ".child(SharedString::from(row.field.clone()))",
            "Discover metadata field label display belongs in TrackMetadataGridVm::field_label",
        ),
        (
            "src/discover.rs",
            "field: row.field.clone()",
            "Discover metadata drag field label display belongs in TrackMetadataGridVm::field_label",
        ),
        (
            "src/discover.rs",
            "label: drag.field.clone()",
            "Discover metadata drag preview label belongs in TrackMetadataGridVm::drag_preview_display",
        ),
        (
            "src/discover.rs",
            "value: drag.value.clone()",
            "Discover metadata drag preview value belongs in TrackMetadataGridVm::drag_preview_display",
        ),
        (
            "src/library.rs",
            "label: SharedString::from(frame_id.to_string())",
            "Library metadata ID3 frame label display belongs in TrackMetadataGridVm::id3_frame_display_label",
        ),
        (
            "src/discover.rs",
            "SharedString::from(frame_label.to_string())",
            "Discover metadata ID3 frame label display belongs in TrackMetadataGridVm::id3_frame_display_label",
        ),
        (
            "src/discover.rs",
            "SharedString::from(frame_label.clone())",
            "Discover metadata ID3 frame label display should be consumed without renderer-side cloning",
        ),
        (
            "src/library.rs",
            "fn id3_frame_color(frame_id: &str)",
            "Library metadata ID3 frame color role belongs in TrackMetadataGridVm::id3_frame_color_role",
        ),
        (
            "src/discover.rs",
            "fn id3_frame_version_color(",
            "Discover metadata ID3 frame color role belongs in TrackMetadataGridVm::id3_frame_color_role",
        ),
        (
            "src/discover.rs",
            "fn id3_frame_version(",
            "Discover metadata ID3 frame version classification belongs in metadata/view-model contracts",
        ),
        (
            "src/discover.rs",
            "enum Id3FrameVersion",
            "Discover metadata ID3 frame version classification belongs in metadata/view-model contracts",
        ),
        (
            "src/discover.rs",
            "frame.map(id3_frame_base).map(id3_frame_version_color)",
            "Discover metadata ID3 frame color role belongs in TrackMetadataGridVm::id3_frame_color_role",
        ),
        (
            "src/library.rs",
            "expanded_metadata_display_string(",
            "Library expanded metadata raw/display selection belongs in TrackMetadataGridVm::expanded_display_value",
        ),
        (
            "src/discover.rs",
            "expanded_metadata_display_string(",
            "Discover expanded metadata raw/display selection belongs in TrackMetadataGridVm::expanded_display_value",
        ),
        (
            "src/library.rs",
            "SharedString::from(display_value.to_string())",
            "Library metadata text display values belong in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/discover.rs",
            "SharedString::from(display_value.to_string())",
            "Discover metadata text display values belong in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/library.rs",
            "value: SharedString::from(value.to_string())",
            "Library metadata text value projection belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/discover.rs",
            "MultilineText::new(value.to_string())",
            "Discover metadata text value projection belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/library.rs",
            "MultilineText::new(raw_value.to_string())",
            "Library expanded metadata raw fallback belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/discover.rs",
            "SharedString::from(line.to_string())",
            "Discover expanded metadata line display belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/discover.rs",
            "SharedString::from(raw_value.to_string())",
            "Discover expanded artwork URL display belongs in TrackMetadataGridVm::artwork_url_display",
        ),
        (
            "src/discover.rs",
            "fn muted_line(value: &str)",
            "Discover deferred-panel empty-line display belongs in SearchViewModel::deferred_panel_empty_line",
        ),
        (
            "src/discover.rs",
            "SharedString::from(value.to_string())",
            "Discover deferred-panel empty-line display belongs in SearchViewModel::deferred_panel_empty_line",
        ),
        (
            "src/discover.rs",
            "title: title.to_string().into()",
            "Discover feed header title display belongs in SearchViewModel::feed_header_display",
        ),
        (
            "src/discover.rs",
            ".filter(|value| !value.trim().is_empty())",
            "Discover feed header subtitle filtering belongs in SearchViewModel::feed_header_display",
        ),
        (
            "src/discover.rs",
            "const TYPE_LABELS",
            "Discover type-filter labels belong in SearchViewModel::type_filter_options",
        ),
        (
            "src/discover.rs",
            "const TYPE_VALUES",
            "Discover type-filter query values belong in SearchViewModel::type_filter_value",
        ),
        (
            "src/discover.rs",
            "TYPE_VALUES[intent.type_filter()]",
            "Discover type-filter query values belong in SearchViewModel::type_filter_value",
        ),
        (
            "src/discover.rs",
            ".label(SharedString::from(label.to_string()))",
            "Discover type-filter labels belong in SearchViewModel::type_filter_options",
        ),
        (
            "src/discover.rs",
            "render_feed_list_section(\"Feeds\"",
            "Discover feed-list section heading belongs in SearchViewModel::feed_list_section_display",
        ),
        (
            "src/discover.rs",
            "SectionHeader::new(heading.to_string())",
            "Discover feed-list section heading belongs in SearchViewModel::feed_list_section_display",
        ),
        (
            "src/discover.rs",
            "SharedString::from(row.entity_type.clone())",
            "Discover result type badge label belongs in ResultRowDisplay",
        ),
        (
            "src/discover.rs",
            "Label::new(title.to_string())",
            "Discover inspector title display belongs in SearchViewModel::inspector_title_display",
        ),
        (
            "src/discover.rs",
            "vm.add_to_playlist_label().to_string()",
            "Discover playlist trigger fallback belongs in ActionRowVm::playlist_trigger_label",
        ),
        (
            "src/discover.rs",
            "group_heading(group.to_string())",
            "Discover payment-route group heading belongs in PaymentRouteVm::group_display",
        ),
        (
            "src/discover.rs",
            "pub(crate) fn render_collapsed_text_section",
            "Dead Discover collapsed text section render helpers must not reintroduce screen-local display strings",
        ),
        (
            "src/library.rs",
            "SharedString::from(text.to_string())",
            "Library MusicBrainz status text belongs in LibraryTrackRowVm::mb_status_text",
        ),
        (
            "src/library.rs",
            ".child(SharedString::from(artist.name.clone()))",
            "Library artist tree title belongs in ArtistNode::tree_display",
        ),
        (
            "src/library.rs",
            ".child(SharedString::from(album.name.clone()))",
            "Library album tree title belongs in AlbumNode::tree_display",
        ),
        (
            "src/library.rs",
            ".child(SharedString::from(summary.feed_name.clone()))",
            "Library artist feed-summary title belongs in ArtistFeedSummaryVm::display",
        ),
        (
            "src/library.rs",
            "let feed_name_for_click = display.title.clone()",
            "Library artist feed-summary click title should be consumed from ArtistFeedSummaryDisplay",
        ),
        (
            "src/library.rs",
            "summary.thumb_url",
            "Library artist feed-summary thumbnail URL belongs in ArtistFeedSummaryVm::display",
        ),
        (
            "src/library.rs",
            "title: SharedString::from(playlist_name.clone())",
            "Library playlist detail header title belongs in PlaylistDetailVm::header_display",
        ),
        (
            "src/library.rs",
            "display.disclosure_id.as_deref()",
            "Library metadata disclosure id binding should consume TrackMetadataGridVm display ids directly",
        ),
        (
            "src/discover.rs",
            "display.disclosure_id.as_deref()",
            "Discover metadata disclosure id binding should consume TrackMetadataGridVm display ids directly",
        ),
        (
            "src/library.rs",
            "disclosure_id.to_string()",
            "Library metadata disclosure id binding should not re-project VM display ids",
        ),
        (
            "src/discover.rs",
            "disclosure_id.to_string()",
            "Discover metadata disclosure id binding should not re-project VM display ids",
        ),
        (
            "src/library.rs",
            "playlist.name.clone()",
            "Library playlist popover option display belongs in playlist_option_displays",
        ),
        (
            "src/library.rs",
            "SharedString::from(row.element_id.clone())",
            "Library playlist sidebar row id should be consumed from PlaylistSidebarRowVm",
        ),
        (
            "src/library.rs",
            "Label::new(row.name.clone())",
            "Library playlist sidebar row name should be consumed from PlaylistSidebarRowVm",
        ),
        (
            "src/library.rs",
            "Label::new(row.track_count_label.clone())",
            "Library playlist sidebar count should be consumed from PlaylistSidebarRowVm",
        ),
        (
            "src/discover.rs",
            "playlist.name.clone()",
            "Discover playlist popover option display belongs in playlist_option_displays",
        ),
        (
            "src/ui/shells/track.rs",
            "playlist.name.clone()",
            "Track shell playlist popover option display belongs in playlist_option_displays",
        ),
        (
            "src/discover.rs",
            "fn compare_row_id(",
            "Discover metadata compare-row slug display belongs in TrackMetadataGridVm::compare_row_id",
        ),
        (
            "src/discover.rs",
            "format!(\"id3-unused-{}\"",
            "Discover unused ID3 frame row id belongs in TrackMetadataGridVm::unused_id3_frame_row_id",
        ),
        (
            "src/discover.rs",
            "format!(\"id3-field-{}\"",
            "Discover used ID3 field row id belongs in TrackMetadataGridVm::used_id3_field_row_id",
        ),
        (
            "src/discover.rs",
            "format!(\"ID3 {frame_id}\")",
            "Discover unused ID3 frame label belongs in TrackMetadataGridVm::id3_field_display_label",
        ),
        (
            "src/discover.rs",
            "format!(\"ID3 {}\", field.frame_id)",
            "Discover used ID3 field label belongs in TrackMetadataGridVm::id3_field_display_label",
        ),
        (
            "src/discover.rs",
            "format!(\"metadata-rss-drag-{}\"",
            "Discover RSS metadata source-drag id belongs in TrackMetadataGridVm::source_drag_display",
        ),
        (
            "src/discover.rs",
            "format!(\"metadata-musicbrainz-drag-{}\"",
            "Discover MusicBrainz metadata source-drag id belongs in TrackMetadataGridVm::source_drag_display",
        ),
        (
            "src/library.rs",
            "summarize_contributor_value(raw_value).unwrap_or_else",
            "metadata contributor summary fallback belongs in TrackMetadataGridVm::contributor_summary",
        ),
        (
            "src/discover.rs",
            "summarize_contributor_value(raw_value).unwrap_or_else",
            "metadata contributor summary fallback belongs in TrackMetadataGridVm::contributor_summary",
        ),
        (
            "src/library.rs",
            "format!(\"[{} items]\", arr.len())",
            "metadata value-route summary belongs in TrackMetadataGridVm::value_routes_summary",
        ),
        (
            "src/discover.rs",
            "format!(\"[{} items]\", arr.len())",
            "metadata value-route summary belongs in TrackMetadataGridVm::value_routes_summary",
        ),
        (
            "src/discover.rs",
            "format!(\"[{lines} lines]\")",
            "metadata value-route multiline fallback belongs in TrackMetadataGridVm::value_routes_summary",
        ),
        (
            "src/discover.rs",
            "fn expandable_cell_summary(",
            "Discover expandable metadata summary policy belongs in TrackMetadataGridVm::expandable_cell_summary",
        ),
        (
            "src/library.rs",
            "raw_value.starts_with(\"http://\") || raw_value.starts_with(\"https://\")",
            "metadata artwork URL summary policy belongs in TrackMetadataGridVm::expandable_cell_summary",
        ),
        (
            "src/discover.rs",
            "raw_value.starts_with(\"http://\") || raw_value.starts_with(\"https://\")",
            "metadata artwork URL summary policy belongs in TrackMetadataGridVm::artwork_url",
        ),
        (
            "src/discover.rs",
            "let line = if line.is_empty() { \" \" } else { line };",
            "metadata transcript blank-line display belongs in TrackMetadataGridVm::transcript_line_display",
        ),
        (
            "src/library.rs",
            "fn metadata_logical_field(",
            "Library raw metadata logical-field aliases belong in TrackMetadataGridVm::logical_field",
        ),
        (
            "src/library.rs",
            "\"TXXX:MusicIndex Contributors\" => \"Contributors\"",
            "Library raw contributor metadata alias belongs in TrackMetadataGridVm::logical_field",
        ),
        (
            "src/library.rs",
            "\"TXXX:MusicIndex Value Routes\" => \"Value Routes\"",
            "Library raw Value Routes metadata alias belongs in TrackMetadataGridVm::logical_field",
        ),
        (
            "src/library.rs",
            "matches!(key.as_str(), \"recipient_name\" | \"split\")",
            "Library Value Routes child-field visibility belongs in TrackMetadataGridVm::value_route_child_field_is_visible",
        ),
        (
            "src/discover.rs",
            "if key == \"recipient_name\"",
            "Discover Value Routes child-field visibility belongs in TrackMetadataGridVm::value_route_child_field_is_visible",
        ),
        (
            "src/discover.rs",
            "serde_json::Value::String(s) => s.clone()",
            "Discover JSON-tree scalar display belongs in TrackMetadataGridVm::json_tree_scalar_label",
        ),
        (
            "src/discover.rs",
            "serde_json::Value::Null => \"null\".into()",
            "Discover JSON-tree null display belongs in TrackMetadataGridVm::json_tree_scalar_label",
        ),
        (
            "src/library.rs",
            "ActionRowMessageDisplay {",
            "Library action-row message tone/width belongs in VM display contracts",
        ),
        (
            "src/discover.rs",
            "ActionRowMessageDisplay {",
            "Discover action-row message tone/width belongs in VM display contracts",
        ),
        (
            "src/library.rs",
            "ActionRowMessageTone::",
            "Library action-row message tone belongs in VM display contracts",
        ),
        (
            "src/discover.rs",
            "ActionRowMessageTone::",
            "Discover action-row message tone belongs in VM display contracts",
        ),
        (
            "src/library.rs",
            "message_is_error()",
            "Library subscription message severity belongs in LibraryTrackActionVm::subscription_message_display",
        ),
        (
            "src/discover.rs",
            "message_is_error()",
            "Discover subscription message severity belongs in ActionRowVm::subscription_message_display",
        ),
        (
            "src/library.rs",
            ".max_width(layout::CONFLICT_MESSAGE_WIDTH)",
            "Library staged-ID3 conflict message width belongs in TrackMetadataActionState display",
        ),
        (
            "src/library.rs",
            ".max_width(layout::ACTION_MESSAGE_WIDTH)",
            "Library ID3 apply-error message width belongs in TrackMetadataActionState display",
        ),
        (
            "src/library.rs",
            "metadata_field_is_expandable(logical_field) && !raw_value.is_empty()",
            "Library metadata expandability gate belongs in TrackMetadataGridVm::field_is_expandable",
        ),
        (
            "src/discover.rs",
            "metadata_field_is_expandable(&row.field) && !value.is_empty()",
            "Discover metadata expandability gate belongs in TrackMetadataGridVm::field_is_expandable",
        ),
        (
            "src/library.rs",
            "logical_field == \"Value Routes\"",
            "Library expanded metadata field kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/discover.rs",
            "field == \"Value Routes\"",
            "Discover expanded metadata field kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/library.rs",
            "field == \"Artwork\"",
            "Library expanded metadata artwork kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/discover.rs",
            "field == \"Artwork\"",
            "Discover expanded metadata artwork kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/discover.rs",
            "matches!(field, \"Artwork\")",
            "Discover expanded metadata artwork kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/discover.rs",
            "matches!(field, \"Transcript\" | \"Transcript text\")",
            "Discover expanded transcript kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/library.rs",
            "format!(\"{} ({} unused)\", group.label, group.unused_count)",
            "metadata group heading fallback belongs in TrackMetadataGridVm::group_heading_label",
        ),
        (
            "src/discover.rs",
            "format!(\"{} ({} unused)\", group.label, group.unused_count)",
            "metadata group heading fallback belongs in TrackMetadataGridVm::group_heading_label",
        ),
        (
            "src/library.rs",
            "format!(\"{name} {split}\")",
            "metadata value-route item label fallback belongs in TrackMetadataGridVm::value_route_item_label",
        ),
        (
            "src/library.rs",
            "strip_suffix(\".0\")",
            "metadata value-route split label fallback belongs in TrackMetadataGridVm::value_route_split_label",
        ),
        (
            "src/library.rs",
            "format!(\"{key}: \")",
            "metadata value-route field key display belongs in TrackMetadataGridVm::value_route_field_key_label",
        ),
        (
            "src/discover.rs",
            "format!(\"{key}: \")",
            "metadata value-route field key display belongs in TrackMetadataGridVm::value_route_field_key_label",
        ),
        (
            "src/library.rs",
            "fn route_value_label(",
            "metadata value-route field value display belongs in TrackMetadataGridVm::value_route_field_value_label",
        ),
        (
            "src/discover.rs",
            "serde_json::Value::Bool(b) => b.to_string()",
            "metadata value-route field value display belongs in TrackMetadataGridVm::value_route_field_value_label",
        ),
        (
            "src/discover.rs",
            "\"No audio URL\"",
            "track play-audio tooltip fallback belongs in TrackVm::play_audio_display",
        ),
        (
            "src/discover.rs",
            "url.clone().unwrap_or_else(|| \"No audio URL\".into())",
            "track play-audio tooltip fallback belongs in TrackVm::play_audio_display",
        ),
        (
            "src/library.rs",
            "row.musicbrainz_value.as_deref().unwrap_or(\"\")",
            "metadata MusicBrainz cell value fallback belongs in TrackMetadataGridVm::musicbrainz_cell_value",
        ),
        (
            "src/discover.rs",
            "row.musicbrainz_value.as_deref().unwrap_or(\"\")",
            "metadata MusicBrainz cell value fallback belongs in TrackMetadataGridVm::musicbrainz_cell_value",
        ),
        (
            "src/library.rs",
            "fn comparison_status_role(",
            "metadata comparison role display belongs in TrackMetadataGridVm::comparison_role",
        ),
        (
            "src/discover.rs",
            "fn comparison_status_role(",
            "metadata comparison role display belongs in TrackMetadataGridVm::comparison_role",
        ),
        (
            "src/library.rs",
            "fn comparison_status_glyph(",
            "metadata comparison glyph display belongs in TrackMetadataGridVm::comparison_glyph",
        ),
        (
            "src/discover.rs",
            "fn comparison_status_glyph(",
            "metadata comparison glyph display belongs in TrackMetadataGridVm::comparison_glyph",
        ),
        (
            "src/library.rs",
            "fn display_with_glyph(",
            "metadata glyph-prefix display belongs in TrackMetadataGridVm::display_with_glyph",
        ),
        (
            "src/discover.rs",
            "fn display_with_glyph(",
            "metadata glyph-prefix display belongs in TrackMetadataGridVm::display_with_glyph",
        ),
        (
            "src/library.rs",
            "fn pending_source_role(",
            "metadata pending-source role display belongs in TrackMetadataGridVm::pending_source_role",
        ),
        (
            "src/discover.rs",
            "fn source_cell_role(",
            "metadata pending-source role display belongs in TrackMetadataGridVm::pending_source_role",
        ),
        (
            "src/library.rs",
            "row.id3_value.is_some() && row.rss_value.is_none() && row.musicbrainz_value.is_none()",
            "metadata standalone-ID3 status fallback belongs in TrackMetadataGridVm::id3_status_role",
        ),
        (
            "src/discover.rs",
            "row.id3_value.is_some() && row.rss_value.is_none() && row.musicbrainz_value.is_none()",
            "metadata standalone-ID3 status fallback belongs in TrackMetadataGridVm::id3_status_role",
        ),
        (
            "src/discover.rs",
            "StatusRole::Danger.glyph()",
            "Discover status error-prefix display belongs in SearchStatusSnapshot",
        ),
        (
            "src/discover.rs",
            "\"Fuzzy: On\"",
            "Discover fuzzy-toggle label display belongs in SearchRenderSnapshot",
        ),
        (
            "src/discover.rs",
            "\"Fuzzy: Off\"",
            "Discover fuzzy-toggle label display belongs in SearchRenderSnapshot",
        ),
        (
            "src/discover.rs",
            "\"No results\"",
            "Discover empty-results label display belongs in SearchRenderSnapshot",
        ),
        (
            "src/discover.rs",
            "\"Load more\"",
            "Discover load-more label display belongs in SearchRenderSnapshot or RecentFeedsSnapshot",
        ),
        (
            "src/discover.rs",
            "\"Recent Feeds\"",
            "Discover recent-feeds panel title belongs in RecentFeedsSnapshot",
        ),
        (
            "src/discover.rs",
            "\"No recent feeds\"",
            "Discover recent-feeds empty label belongs in RecentFeedsSnapshot",
        ),
        (
            "src/discover.rs",
            "format!(\"Open publisher: {publisher_text}\")",
            "Discover publisher-link tooltip display belongs in PublisherLinkDisplay",
        ),
        (
            "src/discover.rs",
            "format!(\"Loading {title}...\")",
            "Discover inspector loading display belongs in SearchViewModel::inspector_loading_message",
        ),
        (
            "src/discover.rs",
            "LoadingMessage::new(format!(\"Error: {error}\"))",
            "Discover inspector error display belongs in SearchViewModel::inspector_error_message",
        ),
        (
            "src/discover.rs",
            "\"\u{2190} Back\"",
            "Discover inspector back label belongs in SearchViewModel::inspector_chrome_display",
        ),
        (
            "src/discover.rs",
            "\"Select a result to inspect\"",
            "Discover empty-inspector label belongs in SearchViewModel::inspector_chrome_display",
        ),
        (
            "src/discover.rs",
            "text_3xl().opacity(0.4).child(\"\u{1F50D}\")",
            "Discover empty-inspector icon belongs in SearchViewModel::inspector_chrome_display",
        ),
        (
            "src/discover.rs",
            "\"Loading contributors...\"",
            "Discover contributor-panel loading label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/discover.rs",
            "\"Loading value routes...\"",
            "Discover value-route-panel loading label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/discover.rs",
            "SplitPane::new(\"pane-container\")",
            "Discover split-pane container id belongs in SearchViewModel render snapshot display",
        ),
        (
            "src/discover.rs",
            "resize_handle_id(\"resize-handle\")",
            "Discover split-pane resize handle id belongs in SearchViewModel render snapshot display",
        ),
        (
            "src/discover.rs",
            "\"No contributors found\"",
            "Discover contributor-panel empty label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/discover.rs",
            "\"No value routes found\"",
            "Discover value-route-panel empty label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/discover.rs",
            "id: \"section:contributors\".into()",
            "Discover contributor-panel heading id belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/discover.rs",
            "label: \"Contributors\".into()",
            "Discover contributor-panel heading label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/discover.rs",
            "id: \"section:value-routes\".into()",
            "Discover value-route-panel heading id belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/discover.rs",
            "label: \"Value Routes\".into()",
            "Discover value-route-panel heading label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/library.rs",
            "\"Search your library...\"",
            "Library search placeholder belongs in LibraryViewModel::chrome_display",
        ),
        (
            "src/library.rs",
            "\"New playlist name\u{2026}\"",
            "Library new-playlist placeholder belongs in LibraryViewModel::chrome_display",
        ),
        (
            "src/library.rs",
            "child(\"Playlists\")",
            "Library playlist sidebar heading belongs in LibraryViewModel::playlist_sidebar",
        ),
        (
            "src/library.rs",
            "child(\"Search Library\")",
            "Library search pane heading belongs in LibraryViewModel::chrome_display",
        ),
        (
            "src/library.rs",
            "label(\"Search\")",
            "Library search action label belongs in LibraryViewModel::chrome_display",
        ),
        (
            "src/library.rs",
            "label(\"Add\")",
            "Library new-playlist action label belongs in LibraryViewModel::playlist_sidebar",
        ),
        (
            "src/library.rs",
            "format!(\"Apply updates ({stale_count})\")",
            "Library feed-update action label belongs in LibraryViewModel::feed_update_display",
        ),
        (
            "src/library.rs",
            "finish_feed_view_check(feed_id, Err(format!",
            "Library single-feed check error formatting belongs in LibraryViewModel",
        ),
        (
            "src/library.rs",
            "set_feed_check_error(format!",
            "Library feed-check error formatting belongs in LibraryViewModel",
        ),
        (
            "src/library.rs",
            "finish_apply_feed_updates(format!(\"Feed update error:",
            "Library feed-update apply error formatting belongs in LibraryViewModel",
        ),
        (
            "src/library.rs",
            "SplitPane::new(\"library-pane-container\")",
            "Library split-pane container id belongs in LibraryViewModel::chrome_display",
        ),
        (
            "src/library.rs",
            "resize_handle_id(\"library-resize-handle\")",
            "Library split-pane resize handle id belongs in LibraryViewModel::chrome_display",
        ),
        (
            "src/library.rs",
            "render_feed_identity_actions(&page, \"library-feed\")",
            "Library feed identity action prefix belongs in ReleaseDetailPageVm",
        ),
        (
            "src/ui/shells/feed.rs",
            "render_feed_identity_actions(&page, \"discover-feed\")",
            "Discover feed identity action prefix belongs in ReleaseDetailPageVm",
        ),
        (
            "src/ui/shells/entity.rs",
            "id_prefix: &str",
            "Feed identity action rendering should consume ReleaseDetailPageVm identity prefix",
        ),
        (
            "src/discover.rs",
            "\"discover-track\"",
            "Discover track identity action prefix belongs in TrackDetailVm",
        ),
        (
            "src/library.rs",
            "\"library-track\"",
            "Library track identity action prefix belongs in TrackDetailVm",
        ),
        (
            "src/ui/shells/track.rs",
            "id_prefix: &str",
            "Track identity action rendering should consume TrackDetailVm identity prefix",
        ),
        (
            "src/discover.rs",
            ".identity_actions(\"contributor\")",
            "Discover contributor identity action prefix belongs in ContributorRowVm",
        ),
        (
            "src/library.rs",
            ".identity_actions(\"library-contributor\")",
            "Library contributor identity action prefix belongs in ContributorRowVm",
        ),
        (
            "src/view_models/entity_detail.rs",
            "identity_actions(&self, id_prefix: &str)",
            "Contributor identity action prefix should be derived from ContributorRowVm context",
        ),
        (
            "src/library.rs",
            "\"album-detail-scroll\"",
            "Library release detail scroll id belongs in ReleaseDetailPageVm",
        ),
        (
            "src/ui/shells/feed.rs",
            "\"discover-feed-detail\"",
            "Discover feed detail scroll id belongs in ReleaseDetailPageVm",
        ),
        (
            "src/ui/shells/entity.rs",
            "format!(\"contributor:{",
            "Contributor person row id belongs in ContributorPersonVm",
        ),
        (
            "src/ui/shells/entity.rs",
            "format!(\"contributor-role:",
            "Contributor role row id belongs in ContributorPersonVm",
        ),
        (
            "src/ui/shells/entity.rs",
            "format!(\"- {role}\")",
            "Contributor role row label belongs in ContributorPersonVm",
        ),
        (
            "src/ui/shells/entity.rs",
            "format!(\"entity-track:{index}\")",
            "Release track row id belongs in SharedTrackRowVm",
        ),
        (
            "src/discover.rs",
            "\"Discover artists, feeds, and tracks...\"",
            "Discover search input placeholder belongs in SearchViewModel::search_input_display",
        ),
        (
            "src/ui/shells/entity.rs",
            "let label = person.name().to_string()",
            "Contributor person row display text belongs in ContributorPersonVm",
        ),
        (
            "src/ui/shells/entity.rs",
            "contributor.href()",
            "Contributor person row href display belongs in ContributorPersonVm",
        ),
        (
            "src/ui/shells/entity.rs",
            "panel.body.as_deref().unwrap_or_default().to_string()",
            "Release description panel body fallback belongs in ReleasePanelVm::text_display",
        ),
        (
            "src/ui/shells/entity.rs",
            "title: hero.title.to_string().into()",
            "Release hero header title belongs in ReleaseHeroVm::display",
        ),
        (
            "src/ui/shells/entity.rs",
            "subtitle: hero.subtitle.map(|subtitle| subtitle.to_string().into())",
            "Release hero header subtitle belongs in ReleaseHeroVm::display",
        ),
        (
            "src/ui/shells/entity.rs",
            "label: \"Publisher\".into()",
            "Release hero supporting-line label belongs in ReleaseHeroVm::display",
        ),
        (
            "src/ui/shells/entity.rs",
            "value: supporting_line.to_string().into()",
            "Release hero supporting-line value belongs in ReleaseHeroVm::display",
        ),
        (
            "src/ui/shells/entity.rs",
            "SharedString::from(title.to_string())",
            "Release text-panel title belongs in ReleaseTextPanelDisplay",
        ),
        (
            "src/ui/shells/entity.rs",
            "SharedString::from(role.id.clone())",
            "Contributor role row id should be consumed from ContributorRoleRowVm",
        ),
        (
            "src/ui/shells/entity.rs",
            "SharedString::from(role.label.clone())",
            "Contributor role row label should be consumed from ContributorRoleRowVm",
        ),
        (
            "src/discover.rs",
            "EntityKind::from_legacy_str(&row.entity_type)",
            "Discover result thumbnail kind belongs with ResultRowDisplay kind projection",
        ),
        (
            "src/discover.rs",
            "let key = row.key()",
            "Discover result row selection key belongs in ResultRowRenderItem",
        ),
        (
            "src/discover.rs",
            "let entity_type = row.entity_type.clone()",
            "Discover result row navigation target belongs in ResultRowRenderItem",
        ),
        (
            "src/discover.rs",
            "let entity_id = row.entity_id.clone()",
            "Discover result row navigation target belongs in ResultRowRenderItem",
        ),
        (
            "src/discover.rs",
            "let title = row.inspector_title()",
            "Discover result row navigation title belongs in ResultRowRenderItem",
        ),
        (
            "src/library.rs",
            "\"Checking...\"",
            "Library feed-update checking label belongs in LibraryViewModel::feed_update_display",
        ),
        (
            "src/library.rs",
            "\"Check all feeds\"",
            "Library feed-update check label belongs in LibraryViewModel::feed_update_display",
        ),
        (
            "src/library.rs",
            "status_text.starts_with(\"Error:\")",
            "Library status severity belongs in LibraryViewModel::status_snapshot",
        ),
        (
            "src/library.rs",
            "self.vm.status().starts_with(\"Error:\")",
            "Library empty-state visibility belongs in LibraryViewModel::should_show_empty_library",
        ),
        (
            "src/library.rs",
            "\"No library tracks yet\"",
            "Library empty-list label belongs in LibraryViewModel::chrome_display",
        ),
        (
            "src/library.rs",
            "\"Select an item to view details\"",
            "Library empty-detail label belongs in LibraryViewModel::chrome_display",
        ),
        (
            "src/library.rs",
            "format!(\"artist-{}\"",
            "Library artist tree row id belongs in ArtistNode::tree_display",
        ),
        (
            "src/library.rs",
            "SharedString::from(artist_display.element_id.clone())",
            "Library artist tree row id should be consumed from LibraryArtistTreeDisplay",
        ),
        (
            "src/library.rs",
            "album_count == 1",
            "Library artist album-count label belongs in ArtistNode::tree_display",
        ),
        (
            "src/library.rs",
            "SharedString::from(album_count_label)",
            "Library artist album-count label should render through DisclosureSupplementLabel",
        ),
        (
            "src/library.rs",
            "format!(\"album-{}-{}\"",
            "Library album tree row id belongs in AlbumNode::tree_display",
        ),
        (
            "src/library.rs",
            "SharedString::from(album_display.element_id.clone())",
            "Library album tree row id should be consumed from LibraryAlbumTreeDisplay",
        ),
        (
            "src/library.rs",
            "format!(\"({track_count})\"",
            "Library album track-count label belongs in AlbumNode::tree_display",
        ),
        (
            "src/library.rs",
            "SharedString::from(track_count_label)",
            "Library album tree track-count label should render through DisclosureSupplementLabel",
        ),
        (
            "src/library.rs",
            "Label::new(track_count_label)",
            "Library sidebar count labels should render through DisclosureSupplementLabel",
        ),
        (
            "src/library.rs",
            "SharedString::from(disclosure_glyph)",
            "Library tree disclosure glyph should render through DisclosureIndicator",
        ),
        (
            "src/library.rs",
            "SharedString::from(playlist_disclosure_glyph)",
            "Library playlist disclosure glyph should render through DisclosureIndicator",
        ),
        (
            "src/library.rs",
            "format!(\"tree-track-{}\"",
            "Library tree track row id belongs in LibraryTrackRowVm::tree_display",
        ),
        (
            "src/library.rs",
            "SharedString::from(track_display.element_id.clone())",
            "Library tree track row id should be consumed from LibraryTreeTrackDisplay",
        ),
        (
            "src/library.rs",
            "format!(\"{num}{title}\")",
            "Library tree track title belongs in LibraryTrackRowVm::tree_display",
        ),
        (
            "src/library.rs",
            "format!(\"artist-feed-{}\"",
            "Library artist feed-summary row id belongs in ArtistFeedSummaryVm::display",
        ),
        (
            "src/library.rs",
            "format!(\"{} tracks\", summary.track_count)",
            "Library artist feed-summary count label belongs in ArtistFeedSummaryVm::display",
        ),
        (
            "src/library.rs",
            "SharedString::from(\"MusicBrainz\")",
            "Library album MusicBrainz action label belongs in LibraryAlbumDetailVm::musicbrainz_action_vm",
        ),
        (
            "src/library.rs",
            ".disabled(vm.has_active_musicbrainz())",
            "Library album MusicBrainz action availability belongs in LibraryAlbumDetailVm::musicbrainz_action_vm",
        ),
        (
            "src/library.rs",
            "format!(\"album-feed-add:{fid}\")",
            "Library album playlist popover id belongs in LibraryAlbumDetailVm::playlist_display",
        ),
        (
            "src/library.rs",
            "format!(\"library-contributor-website:{label}:{href}\")",
            "Library contributor website action display belongs in ContributorRowVm::identity_actions",
        ),
        (
            "src/library.rs",
            "format!(\"library-contributor-nostr:{label}:{npub}\")",
            "Library contributor Nostr action display belongs in ContributorRowVm::identity_actions",
        ),
        (
            "src/discover.rs",
            "format!(\"contributor-website:{label}:{href}\")",
            "Discover contributor website action display belongs in ContributorRowVm::identity_actions",
        ),
        (
            "src/discover.rs",
            "format!(\"contributor-nostr:{label}:{npub}\")",
            "Discover contributor Nostr action display belongs in ContributorRowVm::identity_actions",
        ),
        (
            "src/library.rs",
            "\"library-contributors\"",
            "Library contributor panel id belongs in ReleaseDetailVm::contributor_panel_display",
        ),
        (
            "src/library.rs",
            "        \"Contributors\",",
            "Library contributor panel title belongs in ReleaseDetailVm::contributor_panel_display",
        ),
        (
            "src/library.rs",
            "format!(\"section:id3-frame-group:{group_key}\")",
            "Library metadata group disclosure id belongs in TrackMetadataGridVm::group_heading_display",
        ),
        (
            "src/discover.rs",
            "format!(\"section:id3-frame-group:{group_key}\")",
            "Discover metadata group disclosure id belongs in TrackMetadataGridVm::group_heading_display",
        ),
        (
            "src/library.rs",
            "format!(\"metadata-cell:{cell_key}\")",
            "Library metadata expandable cell id belongs in TrackMetadataGridVm::library_expandable_cell_display",
        ),
        (
            "src/library.rs",
            "format!(\"metadata-cell:{cell_key}:header\")",
            "Library metadata expandable header id belongs in TrackMetadataGridVm::library_expandable_cell_display",
        ),
        (
            "src/discover.rs",
            "format!(\"expandable-rss-{}\", field)",
            "Discover RSS expandable cell id belongs in TrackMetadataGridVm::discover_expandable_cell_display",
        ),
        (
            "src/discover.rs",
            "format!(\"expandable-rss-{}-hdr\", field)",
            "Discover RSS expandable header id belongs in TrackMetadataGridVm::discover_expandable_cell_display",
        ),
        (
            "src/discover.rs",
            "format!(\"expandable-id3-{}\", field)",
            "Discover ID3 expandable cell id belongs in TrackMetadataGridVm::discover_expandable_cell_display",
        ),
        (
            "src/discover.rs",
            "format!(\"expandable-id3-{}-hdr\", field)",
            "Discover ID3 expandable header id belongs in TrackMetadataGridVm::discover_expandable_cell_display",
        ),
        (
            "src/library.rs",
            "format!(\"value-route:{column}:{row_id}:{index}\")",
            "Library value-route item id belongs in TrackMetadataGridVm::library_value_route_item_display",
        ),
        (
            "src/library.rs",
            "format!(\"value-route:{column}:{row_id}:{index}:header\")",
            "Library value-route item header id belongs in TrackMetadataGridVm::library_value_route_item_display",
        ),
        (
            "src/discover.rs",
            "format!(\"vr-{column}-{i}\")",
            "Discover value-route item id belongs in TrackMetadataGridVm::discover_value_route_item_display",
        ),
        (
            "src/library.rs",
            "let glyph = if expanded",
            "Library metadata disclosure glyph belongs in TrackMetadataGridVm expandable display contracts",
        ),
        (
            "src/discover.rs",
            "let glyph = if expanded",
            "Discover metadata disclosure glyph belongs in TrackMetadataGridVm expandable display contracts",
        ),
        (
            "src/library.rs",
            "let sub_glyph = if sub_expanded",
            "Library value-route disclosure glyph belongs in TrackMetadataGridVm value-route item display",
        ),
        (
            "src/discover.rs",
            "let sub_glyph = if sub_expanded",
            "Discover value-route disclosure glyph belongs in TrackMetadataGridVm value-route item display",
        ),
        (
            "src/library.rs",
            "display.cell_key.clone()",
            "Library metadata expansion keys should be consumed by destructuring TrackMetadataExpandableCellDisplay",
        ),
        (
            "src/discover.rs",
            "display.cell_key.clone()",
            "Discover metadata expansion keys should be consumed by destructuring TrackMetadataExpandableCellDisplay",
        ),
        (
            "src/library.rs",
            "display.item_key.clone()",
            "Library Value Routes item keys should be consumed by destructuring TrackMetadataValueRouteItemDisplay",
        ),
        (
            "src/discover.rs",
            "display.item_key.clone()",
            "Discover Value Routes item keys should be consumed by destructuring TrackMetadataValueRouteItemDisplay",
        ),
        (
            "src/ui/shells/entity.rs",
            "format!(\"{id_prefix}-{}:{payload}\", kind_slug(kind))",
            "feed identity action id display belongs in EntityActionVm::identity_display",
        ),
        (
            "src/ui/shells/track.rs",
            "format!(\"{id_prefix}-{}:{payload}\", kind_slug(kind))",
            "track identity action id display belongs in EntityActionVm::identity_display",
        ),
        (
            "src/ui/shells/entity.rs",
            "const fn kind_slug(kind: IdentityActionKind)",
            "feed identity action slug display belongs in EntityActionVm::identity_display",
        ),
        (
            "src/ui/shells/track.rs",
            "const fn kind_slug(kind: IdentityActionKind)",
            "track identity action slug display belongs in EntityActionVm::identity_display",
        ),
        (
            "src/ui/shells/track.rs",
            "format!(\"track-row:{guid}\")",
            "Discover track row id display belongs in TrackVm::row_controls_display",
        ),
        (
            "src/ui/shells/track.rs",
            "format!(\"track-row-play:{guid}\")",
            "Discover track play-button id display belongs in TrackVm::row_controls_display",
        ),
        (
            "src/ui/shells/track.rs",
            "format!(\"add-pl:{guid}\")",
            "Discover track playlist popover id display belongs in TrackVm::row_controls_display",
        ),
        (
            "src/ui/shells/track.rs",
            "SharedString::from(\"+ Playlist\")",
            "Discover track playlist trigger label belongs in TrackVm::row_controls_display",
        ),
        (
            "src/ui/shells/track.rs",
            "SharedString::from(controls_display.play_button_id.clone())",
            "Discover track play-button id should be consumed from TrackRowControlsDisplay",
        ),
        (
            "src/ui/shells/track.rs",
            "SharedString::from(controls_display.playlist_popover_id.clone())",
            "Discover track playlist popover id should be consumed from TrackRowControlsDisplay",
        ),
        (
            "src/discover.rs",
            "format!(\"track-row-download-spin:{key}\")",
            "Discover track download spinner id display belongs in TrackRowActionVm::download_display",
        ),
        (
            "src/discover.rs",
            "format!(\"track-row-download:{key}\")",
            "Discover track download button id display belongs in TrackRowActionVm::download_display",
        ),
        (
            "src/discover.rs",
            "format!(\"inspector-add:{}\", frame.entity_id)",
            "Discover inspector playlist popover id belongs in ActionRowVm::inspector_playlist_display",
        ),
        (
            "src/library.rs",
            "format!(\"album-track-add:{track_id}\")",
            "Library album-track playlist popover id belongs in LibraryTrackRowVm::playlist_display",
        ),
        (
            "src/library.rs",
            "format!(\"track-inspector-add:{track_id}\")",
            "Library track inspector playlist popover id belongs in LibraryTrackActionVm::playlist_display",
        ),
        (
            "src/library.rs",
            "SharedString::from(\"+ Playlist\")",
            "Library album-track playlist trigger label belongs in LibraryTrackRowVm::playlist_display",
        ),
        (
            "src/discover.rs",
            "format!(\"feed-tile:{guid}\")",
            "Discover feed-list tile id display belongs in RecentFeedTileVm::display",
        ),
        (
            "src/discover.rs",
            "format!(\"recent-tile:{guid}\")",
            "Discover recent-feed tile id display belongs in RecentFeedTileVm::display",
        ),
        (
            "src/discover.rs",
            "format!(\"podroll-tile:{guid}\")",
            "Discover podroll tile id display belongs in RecentFeedTileVm::display",
        ),
        (
            "src/discover.rs",
            "SharedString::from(\"track-play-audio\")",
            "Discover track-inspector play button id belongs in TrackVm::play_audio_display",
        ),
        (
            "src/discover.rs",
            ".label(\"▶\")",
            "Discover play button glyph belongs in TrackVm::play_audio_display",
        ),
        (
            "src/discover.rs",
            "format!(\"track-feed-link:{guid}\")",
            "Discover track feed-link id belongs in TrackFeedLinkDisplay",
        ),
        (
            "src/library.rs",
            "row.controls_display(pl_id)",
            "Library playlist row controls belong in PlaylistTrackRowVm::display",
        ),
        (
            "src/library.rs",
            "row.title()",
            "Library playlist row title fallback belongs in PlaylistTrackRowVm::display",
        ),
        (
            "src/library.rs",
            "row.artist()",
            "Library playlist row artist fallback belongs in PlaylistTrackRowVm::display",
        ),
        (
            "src/library.rs",
            "row.duration_label()",
            "Library playlist row duration display belongs in PlaylistTrackRowVm::display",
        ),
        (
            "src/library.rs",
            "row.position_label()",
            "Library playlist row position display belongs in PlaylistTrackRowVm::display",
        ),
        (
            "src/library.rs",
            "row.thumb_url()",
            "Library playlist row thumbnail lookup key belongs in PlaylistTrackRowVm::display",
        ),
        (
            "src/library.rs",
            "row_display.thumb_url",
            "Library playlist row thumbnail display should be consumed from PlaylistTrackRowDisplay",
        ),
        (
            "src/library.rs",
            "row_display.position",
            "Library playlist row position should be consumed from PlaylistTrackRowDisplay",
        ),
        (
            "src/library.rs",
            "row_display.position_label",
            "Library playlist row position label should be consumed from PlaylistTrackRowDisplay",
        ),
        (
            "src/library.rs",
            "row_display.title",
            "Library playlist row title should be consumed from PlaylistTrackRowDisplay",
        ),
        (
            "src/library.rs",
            "row_display.artist",
            "Library playlist row artist should be consumed from PlaylistTrackRowDisplay",
        ),
        (
            "src/library.rs",
            "row_display.duration_label",
            "Library playlist row duration should be consumed from PlaylistTrackRowDisplay",
        ),
        (
            "src/library.rs",
            "format!(\"playlist-move-up-{pl_id}-{position}\")",
            "Library playlist move-up fallback id belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            "format!(\"playlist-move-down-{pl_id}-{position}\")",
            "Library playlist move-down fallback id belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            "format!(\"playlist-drag-handle-{pl_id}-{position}\")",
            "Library playlist drag handle id belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            "format!(\"playlist-remove-{pl_id}-{position}\")",
            "Library playlist remove control id belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            "format!(\"playlist-play-{pl_id}-{position}\")",
            "Library playlist play control id belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            "\"playlist-track-{track_id}-{position}\"",
            "Library playlist row id belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            "\"playlist-row-body-{pl_id}-{position}\"",
            "Library playlist row body id belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            ".label(\"Move Up\")",
            "Library playlist move-up fallback label belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            ".label(\"Move Down\")",
            "Library playlist move-down fallback label belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            ".label(\"Remove\")",
            "Library playlist remove fallback label belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            ".label(\"▶\")",
            "Library playlist play glyph belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            "format!(\"lib-toggle-{track_id}\")",
            "Library track toggle id belongs in LibraryTrackRowVm::row_display",
        ),
        (
            "src/library.rs",
            "SharedString::from(row_display.toggle_button_id.clone())",
            "Library album-track toggle id should be consumed from LibraryTrackRowDisplay",
        ),
        (
            "src/library.rs",
            "SharedString::from(controls_display.move_up_menu_item.id.clone())",
            "Library playlist move-up fallback id should be consumed from PlaylistTrackControlsDisplay",
        ),
        (
            "src/library.rs",
            "SharedString::from(controls_display.move_down_menu_item.id.clone())",
            "Library playlist move-down fallback id should be consumed from PlaylistTrackControlsDisplay",
        ),
        (
            "src/library.rs",
            "SharedString::from(controls_display.remove_menu_item.id.clone())",
            "Library playlist remove fallback id should be consumed from PlaylistTrackControlsDisplay",
        ),
        (
            "src/library.rs",
            "SharedString::from(controls_display.play_button_id.clone())",
            "Library playlist play id should be consumed from PlaylistTrackControlsDisplay",
        ),
        (
            "src/library.rs",
            "format!(\"album-track-{track_id}\")",
            "Library album-track row id belongs in LibraryTrackRowVm::row_display",
        ),
        (
            "src/library.rs",
            "format!(\"playlist-rename-{playlist_id}\")",
            "Library playlist rename id belongs in PlaylistDetailVm::actions_display",
        ),
        (
            "src/library.rs",
            "format!(\"playlist-delete-{playlist_id}\")",
            "Library playlist delete id belongs in PlaylistDetailVm::actions_display",
        ),
        (
            "src/library.rs",
            ".label(\"Rename\")",
            "Library playlist rename label belongs in PlaylistDetailVm::actions_display",
        ),
        (
            "src/library.rs",
            ".label(\"Delete\")",
            "Library playlist delete label belongs in PlaylistDetailVm::actions_display",
        ),
        (
            "src/library.rs",
            "LoadingMessage::new(\"Reading embedded metadata...\")",
            "Library metadata compare loading label belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "LoadingMessage::new(\"Searching MusicBrainz...\")",
            "Library MusicBrainz loading label belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "format!(\"Apply tags ({count})\")",
            "Library staged ID3 apply label belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "format!(\"Duplicate target: {conflict_text}\")",
            "Library staged ID3 conflict message belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "SharedString::from(\"Discard staged\")",
            "Library staged ID3 discard label belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "LazyPanel::Empty(format!(\"Error: {error}\"))",
            "Library deferred-panel error prefix belongs in LibraryViewModel",
        ),
        (
            "src/library.rs",
            "\"Subscribing track...\"",
            "Library track subscribe busy status belongs in LibraryTrackActionVm",
        ),
        (
            "src/library.rs",
            "\"Downloaded track\"",
            "Library track subscribe success label belongs in LibraryTrackActionVm",
        ),
        (
            "src/library.rs",
            "frame.subscription_message = Some(\"Subscribing...\"",
            "Library local subscription progress message belongs in LibraryTrackActionVm",
        ),
        (
            "src/library.rs",
            "frame.subscription_message = Some(\"Unsubscribing...\"",
            "Library local unsubscription progress message belongs in LibraryTrackActionVm",
        ),
        (
            "src/library.rs",
            "let action = if subscribe",
            "Library local subscription error label belongs in LibraryTrackActionVm",
        ),
        (
            "src/library.rs",
            "format!(\"{action} error: {err:#}\")",
            "Library local subscription error message belongs in LibraryTrackActionVm",
        ),
        (
            "src/discover.rs",
            "LazyPanel::Empty(format!(\"Error: {error}\"))",
            "Discover deferred-panel error prefix belongs in LazyPanel",
        ),
        (
            "src/discover.rs",
            "\"Downloaded track\"",
            "Discover track download success label belongs in SearchSubscriptionCommand",
        ),
        (
            "src/library.rs",
            "SharedString::from(\"Re-read\")",
            "Library file-header re-read label belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "SharedString::from(\"Re-download\")",
            "Library file-header re-download label belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "Resolve duplicate ID3 target{}: {}",
            "Library duplicate ID3 target message belongs in TrackMetadataActionState",
        ),
        (
            "src/discover.rs",
            "Resolve duplicate ID3 target{}: {}",
            "Discover duplicate ID3 target message belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "format!(\"Error applying ID3 edits: {error}\")",
            "Library ID3 apply error message belongs in TrackMetadataActionState",
        ),
        (
            "src/discover.rs",
            "format!(\"Error applying ID3 edits: {error}\")",
            "Discover ID3 apply error message belongs in TrackMetadataActionState",
        ),
        (
            "src/discover.rs",
            "format!(\", applied {} ID3 edit{}\"",
            "Discover download success ID3 edit suffix belongs in SearchSubscriptionCommand",
        ),
        (
            "src/discover.rs",
            "Some(format!(\"Downloaded track{edit_text}\"))",
            "Discover download success message belongs in SearchSubscriptionCommand",
        ),
        (
            "src/discover.rs",
            ".child(\"🔍\")",
            "Discover results empty-state icon belongs in SearchPaneDisplay",
        ),
        (
            "src/discover.rs",
            "\"result-item:{}:{}\"",
            "Discover result row id belongs in ResultRowDisplay",
        ),
        (
            "src/discover.rs",
            ".child(\"Podroll\")",
            "Discover podroll heading label belongs in PodrollSectionDisplay",
        ),
        (
            "src/discover.rs",
            "\"podroll-scroll:{}\"",
            "Discover podroll scroll id belongs in PodrollSectionDisplay",
        ),
        (
            "src/discover.rs",
            "Button::new(\"search-btn\")",
            "Discover search button id belongs in SearchPaneDisplay",
        ),
        (
            "src/discover.rs",
            "\"fuzzy-toggle\"",
            "Discover fuzzy-toggle id belongs in SearchPaneDisplay",
        ),
        (
            "src/discover.rs",
            ".id(\"results-scroll\")",
            "Discover results scroll id belongs in SearchPaneDisplay",
        ),
        (
            "src/discover.rs",
            "UiButton::styled(\"load-more\"",
            "Discover result load-more id belongs in SearchPaneDisplay",
        ),
        (
            "src/discover.rs",
            "UiButton::styled(\"inspector-back\"",
            "Discover inspector back id belongs in InspectorChromeDisplay",
        ),
        (
            "src/discover.rs",
            ".id(\"inspector-scroll\")",
            "Discover inspector scroll id belongs in InspectorChromeDisplay",
        ),
        (
            "src/discover.rs",
            "UiButton::styled(\"recent-load-more\"",
            "Discover recent-feed load-more id belongs in RecentFeedsDisplay",
        ),
        (
            "src/library.rs",
            "format!(\"thumb-{url}\")",
            "Library hover thumbnail id belongs in LibraryViewModel",
        ),
        (
            "src/library.rs",
            ".child(\"\\u{1F3B5}\")",
            "Library album thumbnail fallback glyph belongs in LibraryViewModel",
        ),
        (
            "src/library.rs",
            ".id(\"playlists-header\")",
            "Library playlist sidebar header id belongs in PlaylistSidebarVm",
        ),
        (
            "src/library.rs",
            "UiButton::styled(\"playlists-sort\"",
            "Library playlist sort id belongs in PlaylistSidebarVm",
        ),
        (
            "src/library.rs",
            "UiButton::styled(\"playlists-add\"",
            "Library playlist add id belongs in PlaylistSidebarVm",
        ),
        (
            "src/library.rs",
            ".id(\"playlist-new-input\")",
            "Library new-playlist input id belongs in PlaylistSidebarVm",
        ),
        (
            "src/library.rs",
            "UiButton::styled(\"playlist-add-btn\"",
            "Library new-playlist add id belongs in PlaylistSidebarVm",
        ),
        (
            "src/library.rs",
            "UiButton::styled(\"lib-search-btn\"",
            "Library search button id belongs in LibraryChromeDisplay",
        ),
        (
            "src/library.rs",
            "UiButton::styled(\"apply-feed-updates\"",
            "Library apply-feed-updates button id belongs in FeedUpdateDisplay",
        ),
        (
            "src/library.rs",
            "UiButton::styled(\"check-all-feeds\"",
            "Library check-all-feeds button id belongs in FeedUpdateDisplay",
        ),
        (
            "src/library.rs",
            ".id(\"library-list\")",
            "Library list scroll id belongs in LibraryChromeDisplay",
        ),
        (
            "src/library.rs",
            ".id(\"artist-detail-scroll\")",
            "Library artist detail scroll id belongs in LibraryChromeDisplay",
        ),
        (
            "src/library.rs",
            ".id(\"playlist-detail-scroll\")",
            "Library playlist detail scroll id belongs in LibraryChromeDisplay",
        ),
        (
            "src/library.rs",
            ".id(\"track-detail-scroll\")",
            "Library track detail scroll id belongs in LibraryChromeDisplay",
        ),
    ];
    let mut violations = Vec::new();

    for (file, pattern, note) in forbidden {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            if line.contains(pattern) {
                violations.push(format!("{file}:{line_number}: {note}: `{line}`"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 Library/Search VM fallback ownership violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn playlist_reorder_display_contract_uses_drag_handle_and_menu_fallbacks() {
    let view_model_source = read_source(&manifest_path("src/view_models/library.rs"));
    for required in [
        "pub(crate) struct PlaylistTrackMenuItemDisplay",
        "pub(crate) drag_handle_id: String",
        "pub(crate) drag_handle_a11y_label: &'static str",
        "pub(crate) actions_menu_id: String",
        "pub(crate) actions_menu_a11y_label: &'static str",
        "pub(crate) move_up_menu_item: PlaylistTrackMenuItemDisplay",
        "pub(crate) move_down_menu_item: PlaylistTrackMenuItemDisplay",
        "pub(crate) remove_menu_item: PlaylistTrackMenuItemDisplay",
        "drag_handle_id: format!(\"playlist-drag-handle-{playlist_id}-{position}\")",
        "drag_handle_a11y_label: \"Drag to reorder playlist track\"",
        "actions_menu_id: format!(\"playlist-actions-{playlist_id}-{position}\")",
        "actions_menu_a11y_label: \"Playlist track actions\"",
        "id: format!(\"playlist-move-up-{playlist_id}-{position}\")",
        "label: \"Move Up\"",
        "a11y_label: \"Move track up\"",
        "disabled: !self.can_move_up()",
        "id: format!(\"playlist-move-down-{playlist_id}-{position}\")",
        "label: \"Move Down\"",
        "a11y_label: \"Move track down\"",
        "disabled: !self.can_move_down()",
        "id: format!(\"playlist-remove-{playlist_id}-{position}\")",
        "label: \"Remove\"",
        "a11y_label: \"Remove track from playlist\"",
        "destructive: true",
    ] {
        assert!(
            view_model_source.contains(required),
            "Playlist reorder display contract must include `{required}`"
        );
    }

    for forbidden in [
        "move_up_button_id",
        "move_up_label",
        "move_up_enabled",
        "move_down_button_id",
        "move_down_label",
        "move_down_enabled",
        "playlist-up-{",
        "playlist-down-{",
        "\"▲\"",
        "\"▼\"",
    ] {
        assert!(
            !view_model_source.contains(forbidden),
            "Playlist row display contract must not expose arrow-button reorder field `{forbidden}`"
        );
    }

    let playlist_shell_source = read_source(&manifest_path("src/ui/shells/playlist.rs"));
    for required in [
        "Icon::new(IconName::DragHandle)",
        "ContextMenu::new(",
        "ContextMenuScope::PlaylistTrack",
        ".can_drop(",
        ".drag_over(",
        ".on_drop(",
        "playlist_reorder_target(payload.from_position, drop_index)",
        "playlist_row_drop_index(payload.from_position, row_drop_index)",
        "playlist_reorder_target(payload.from_position, drop_index).is_none()",
        "let drop_index = playlist_row_drop_index(payload.from_position, row_drop_index);",
        "match playlist_row_insertion_edge(payload.from_position, row_drop_index)",
        "Some(PlaylistInsertionEdge::Before) =>",
        "Some(PlaylistInsertionEdge::After) =>",
        "render_playlist_rows_with_reorder_targets(page.playlist_id(), rows, on_reorder.as_ref(), cx)",
        "on_reorder.cloned()",
        ".border_t(drop_indicator)",
        ".border_b(drop_indicator)",
        "if target == from",
        ".cursor_no_drop()",
        "render_playlist_rows_with_reorder_targets(",
        "fn playlist_row_drop_index_quantizes_by_drag_direction()",
    ] {
        assert!(
            playlist_shell_source.contains(required),
            "Playlist shell drag/menu contract must include `{required}`"
        );
    }

    assert_eq!(
        playlist_shell_source.matches(".on_drag(").count(),
        1,
        "Playlist shell must attach drag only once, to the handle"
    );

    for forbidden in [
        "move_up_button_id",
        "move_up_label",
        "move_up_enabled",
        "move_down_button_id",
        "move_down_label",
        "move_down_enabled",
        ".label(\"Move Up\")",
        ".label(\"Move Down\")",
        ".label(\"Remove\")",
        "\"▲\"",
        "\"▼\"",
        "\"✕\"",
        "\"☰\"",
    ] {
        assert!(
            !playlist_shell_source.contains(forbidden),
            "Playlist shell must not own playlist reorder display fallback `{forbidden}`"
        );
    }

    let icon_source = read_source(&manifest_path("src/ui/icons.rs"));
    for required in [
        "DragHandle",
        "Self::DragHandle => Some(\"\\u{2630}\")",
        "NotAllowed",
        "Self::NotAllowed => Some(\"\\u{2298}\")",
    ] {
        assert!(
            icon_source.contains(required),
            "Playlist drag handle must use semantic icon catalog contract `{required}`"
        );
    }
}

#[test]
fn playlist_refresh_and_frame_navigation_preserve_context() {
    let library_source = read_source(&manifest_path("src/library/app_impl.rs"));
    for required in [
        "enum LibraryReloadMode",
        "ResetDetail",
        "PreserveDetail",
        "enum FrameHistoryMode",
        "Record",
        "Restore",
        "workspace_layout: Self::default_workspace_layout(),",
        "fn default_workspace_layout() -> WorkspaceLayout",
        ".reset_nav(Self::content_frame_id(), FrameNavigationEntry::SourceList)",
        "pub fn refresh(&mut self, cx: &mut Context<Self>) {\n        self.start_async_reload_preserving_detail(cx);",
        "pub(crate) fn start_async_reload(&mut self, cx: &mut Context<Self>) {\n        self.start_async_reload_with_mode(LibraryReloadMode::ResetDetail, cx);",
        "if mode == LibraryReloadMode::ResetDetail",
        "if mode == LibraryReloadMode::PreserveDetail",
        "self.refresh_selected_detail(cx);",
        "if state.playlist_id == playlist_id",
        "state.handle.try_send(PagedTrackListMsg::Refresh)",
        "self.playlist_actor = None;\n        // Open a dedicated connection for the actor",
        "self.spawn_playlist_actor(id, &tracks, cx);",
        "actor.prime_initial_rows(initial_rows.iter().cloned());",
        "pub(crate) fn select_playlist_track(",
        "FrameNavigationEntry::PlaylistDetail(playlist_id)",
        "FrameNavigationEntry::TrackDetail(track.id)",
        "self.restore_frame_navigation()",
        "self.select_playlist_with_history(playlist_id, FrameHistoryMode::Restore, cx);",
        "fn apply_library_removal_result_to_selected_detail(",
        "this.apply_library_removal_result_to_selected_detail(result.target());",
        "frame.local_subscription = false;",
        "frame.track.is_in_library = false;",
    ] {
        assert!(
            library_source.contains(required),
            "Library playlist refresh/frame navigation contract must include `{required}`"
        );
    }
    assert!(
        !library_source.contains("frame.origin ="),
        "Playlist track selection must not write inspector origin; frame history owns return navigation"
    );
    for forbidden in [
        "pub(crate) fn navigate_back_to_playlist(",
        "InspectorOrigin",
        "origin: Option<InspectorOrigin>",
        "this.navigate_back_to_frame_history(cx);\n                }\n                this.start_async_reload_preserving_detail(cx);",
    ] {
        assert!(
            !library_source.contains(forbidden),
            "Library frame navigation must not retain inspector-origin return contract `{forbidden}`"
        );
    }

    assert!(
        !library_source.contains(
            ".id(playlist_header_id)\n                .px(spacing::SM)\n                .py(spacing::XS)\n                .rounded(spacing::XS)\n                .cursor_pointer()"
        ),
        "Playlist sidebar header must not make the entire header a disclosure click target"
    );
    assert!(
        library_source.contains(
            ".items_baseline()\n                        .cursor_pointer()\n                        .hover(|el| el.bg(color::bg_surface_hi()))\n                        .on_click(cx.listener(|this, _, _, cx| {"
        ),
        "Playlist sidebar disclosure click target must stay on the heading cluster"
    );

    let library_struct_source = read_source(&manifest_path("src/library.rs"));
    for forbidden in [
        "pub(crate) enum InspectorOrigin",
        "Playlist(i64)",
        "Album(i64)",
        "Artist(String)",
        "pub(crate) origin: Option<InspectorOrigin>",
    ] {
        assert!(
            !library_struct_source.contains(forbidden),
            "ADR 0046 Task 003 retires inspector-origin navigation state `{forbidden}`"
        );
    }

    let playlist_detail_source =
        read_source(&manifest_path("src/ui/shells/library/playlist_detail.rs"));
    assert!(
        playlist_detail_source
            .contains("this.select_playlist_track(playlist_id, &track_for_select, cx);"),
        "Playlist row selection must open track detail with playlist origin"
    );

    let track_detail_source = read_source(&manifest_path(
        "src/ui/shells/library/track_detail_metadata.rs",
    ));
    for forbidden in [
        "frame_back_destination: Option<&FrameNavigationEntry>",
        "FrameNavigationEntry::PlaylistDetail(playlist_id)",
        "LibraryTrackActionVm::playlist_return_display",
        "track-detail-return-playlist",
        ".leading_icon(crate::ui::icons::IconName::Back)",
        "this.navigate_back_to_playlist",
    ] {
        assert!(
            !track_detail_source.contains(forbidden),
            "Track detail must not render inspector-local playlist return control `{forbidden}`"
        );
    }

    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    for forbidden in [
        "LibraryTrackPlaylistReturnDisplay",
        "playlist_return_display",
        "Back to Playlist",
        "track-detail-return-playlist",
    ] {
        assert!(
            !library_vm_source.contains(forbidden),
            "Library VM must not retain inspector-local playlist return display `{forbidden}`"
        );
    }
}

#[test]
fn source_fact_placeholder_and_breadcrumb_regressions_are_guarded() {
    let feed_service_source = read_source(&manifest_path("src/feed_service.rs"));
    let db_source = read_source(&manifest_path("src/db.rs"));
    let metadata_source = read_source(&manifest_path("src/metadata.rs"));
    let rss_enrich_source = read_source(&manifest_path("src/rss/enrich.rs"));
    let rss_helpers_source = read_source(&manifest_path("src/rss/helpers.rs"));
    let views_source = read_source(&manifest_path("src/views.rs"));
    for required in [
        "source_text_missing",
        ".fetch_feed_track(feed_guid, &track.item_guid, include)",
        "crate::subscribe_service::enrich_track_context_from_rss(&mut track, Some(&mut feed));",
        "library_track_context_rejects_placeholder_source_text_at_boundary",
        // Local-row read boundary: polluted DB rows must not become display
        // facts. Identity columns pass through, but display text is
        // collapsed to `None` via the local `drop_placeholder` helper.
        "local_track_row_strips_placeholder_text_at_projection_boundary",
        "fn drop_placeholder(value: Option<String>) -> Option<String>",
        "drop_placeholder(track.feed_title.clone())",
        // DB-write boundary: MusicIndex feed descriptions must not write
        // placeholder text into `feeds.description`.
        "if !source_text_missing(feed.description.as_deref())",
    ] {
        assert!(
            feed_service_source.contains(required),
            "Metadata placeholder mitigation must stay at the source boundary: `{required}`"
        );
    }

    let subscribe_service_source = read_source(&manifest_path("src/subscribe_service.rs"));
    for required in [
        "fn drop_placeholder(value: Option<String>) -> Option<String>",
        "drop_placeholder(row.track_title.clone())",
        "drop_placeholder(row.artist_name.clone())",
        "drop_placeholder(row.album_artist_name.clone())",
        "drop_placeholder(row.feed_title.clone())",
        "!source_text_missing(Some(url.as_str()))",
        "sanitize_feed_source_text(&mut feed);",
        "sanitize_track_source_text(&mut track_for_persistence);",
        "sanitize_track_source_text(&mut track_for_metadata);",
        "sanitize_track_context_source_text(&mut track_context);",
        "sanitize_track_context_source_text(&mut refreshed_context);",
    ] {
        assert!(
            subscribe_service_source.contains(required),
            "Local TrackRow projection must strip placeholder text before display: `{required}`"
        );
    }

    let rss_subscribe_source = read_source(&manifest_path("src/rss/subscribe.rs"));
    assert!(
        rss_subscribe_source
            .contains("if !crate::metadata::source_text_missing(api_feed.description.as_deref())"),
        "RSS subscribe must not persist placeholder MusicIndex feed descriptions"
    );
    for required in [
        "name: \"cleanup_placeholder_source_text\"",
        "name: \"cleanup_markup_placeholder_source_text\"",
        "migration_cleanup_placeholder_source_text",
        "migration_cleanup_markup_placeholder_source_text",
        "cleanup_placeholder_source_text_columns(conn, null_placeholder_text_column)",
        "cleanup_placeholder_source_text_columns(conn, null_markup_placeholder_text_column)",
        "migration_cleanup_placeholder_source_text_nulls_only_placeholder_payloads",
    ] {
        assert!(
            db_source.contains(required),
            "Polluted source-text cleanup must stay in the migration path: `{required}`"
        );
    }
    for required in [
        "pub(crate) fn source_text_is_placeholder(value: &str) -> bool",
        "pub(crate) fn sanitize_track_context_source_text(context: &mut TrackContext)",
        "pub(crate) fn drop_placeholder_source_text(value: Option<String>) -> Option<String>",
        "sanitize_track_context_source_text_clears_placeholder_display_facts",
        "compare_track_rows_drop_placeholder_source_values",
        "aligned_compare_rows_refills_placeholder_result_sources_from_context",
        "track_metadata_rows_drop_markup_placeholder_source_values",
        "drop_placeholder_source_text(row.source_value.clone())",
        "source_value_for_metadata_field(row.field, track_context)",
        "sanitize_source_release_claims",
        "sanitize_source_contributors",
        "sanitize_track_context_strips_placeholder_contributor_names",
        "contributor_id3_rows_skip_placeholder_names_and_roles",
        "source_placeholder_char",
        "source_placeholder_scan",
        "placeholder_entity_len",
        "placeholder_markup_len",
        "\"<p>...</p><p>...</p>\"",
        "\"&hellip;\"",
        "'\\u{2026}'",
    ] {
        assert!(
            metadata_source.contains(required),
            "Source placeholder classification must stay centralized: `{required}`"
        );
    }
    for required in [
        "use crate::metadata::source_text_missing;",
        ".filter(|value| !source_text_missing(Some(value)))",
    ] {
        assert!(
            rss_helpers_source.contains(required),
            "RSS text projection must reject placeholder-only payloads: `{required}`"
        );
    }
    for required in [
        "use crate::metadata::drop_placeholder_source_text;",
        "drop_placeholder_source_text(value)",
        "from_api_projection_drops_placeholder_source_text",
    ] {
        assert!(
            views_source.contains(required),
            "API-to-view projection must reject placeholder-only payloads: `{required}`"
        );
    }

    let search_source = read_source(&manifest_path("src/application/queries/feed.rs"));
    for required in [
        "sanitize_feed_source_text(&mut feed);",
        "let mut track_context = TrackContext { track, feed };",
        "sanitize_track_context_source_text(&mut track_context);",
    ] {
        assert!(
            search_source.contains(required),
            "Search inspector query facts must be sanitized before display: `{required}`"
        );
    }
    for required in [
        "apply_track_enrichment",
        "rss_enrichment_replaces_placeholder_core_fields",
        "set_text_if_missing(&mut track.title",
    ] {
        assert!(
            rss_enrich_source.contains(required),
            "RSS re-read must restore core source facts before rendering: `{required}`"
        );
    }

    let library_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let library_query_source = read_source(&manifest_path("src/application/queries/library.rs"));
    for required in [
        "fn track_breadcrumb_display(&self) -> Option<BreadcrumbDisplay>",
        "pub(crate) fn select_frame_breadcrumb(",
        "TrackSubscriptionAction::Download(track)",
        "SubscribeTrackRequest::LibraryTrack",
        "frame.track.local_path = result.relative_path().cloned();",
        "frame.source_context = None;",
        "fn load_track_source_context(&mut self, track: TrackRow",
    ] {
        assert!(
            library_source.contains(required),
            "Library track detail must preserve breadcrumb/download contracts: `{required}`"
        );
    }
    for required in [
        // Album hydration must skip writes when MusicIndex feed description
        // is placeholder-only; otherwise an already-good RSS description gets
        // wiped to NULL and the metadata grid renders empty source facts.
        "if description.is_some() {",
        "db::set_feed_description(&db, feed_id, description.as_deref())?;",
    ] {
        assert!(
            library_query_source.contains(required),
            "Library track detail hydration must preserve source-fact contracts: `{required}`"
        );
    }
    assert!(
        !library_source.contains("SetTrackLibraryMembership::new"),
        "Track detail download must run the real SubscribeTrack path, not a membership-only toggle"
    );

    let track_detail_source = read_source(&manifest_path("src/ui/shells/library/track_detail.rs"));
    assert!(
        track_detail_source.contains("BreadcrumbTrail::new(breadcrumb)"),
        "Track detail must expose frame-history breadcrumbs"
    );
    let track_detail_metadata_source = read_source(&manifest_path(
        "src/ui/shells/library/track_detail_metadata.rs",
    ));
    assert!(
        !track_detail_metadata_source.contains("BreadcrumbTrail"),
        "Breadcrumb navigation must not return as an inspector action-row control"
    );

    let agent_source = read_source(&manifest_path("AGENTS.md"));
    assert!(
        agent_source.contains("Placeholder-looking source text is a source-boundary problem"),
        "Future agents must see the source-boundary placeholder mitigation rule"
    );
    let troubleshooting_source = read_source(&manifest_path(
        "docs/troubleshooting/metadata-source-fact-regressions.md",
    ));
    assert!(
        troubleshooting_source
            .contains("Do not patch Library/Search renderers, composites, or display view-models"),
        "Metadata source-fact regression runbook must record the prohibited fix"
    );
}

#[test]
fn local_track_pubdate_and_explicit_projection_path_is_guarded() {
    let db_source = read_source(&manifest_path("src/db.rs"));
    let views_source = read_source(&manifest_path("src/views.rs"));
    let metadata_source = read_source(&manifest_path("src/metadata.rs"));
    let track_detail_source = read_source(&manifest_path("src/view_models/track_detail.rs"));

    for required in [
        "pub pub_date: Option<i64>",
        "pub explicit: Option<bool>",
        "t.pub_date",
        "t.itunes_explicit",
        "parse_local_track_pub_date(row.get::<_, Option<String>>(18)?.as_deref())",
        "parse_itunes_explicit(row.get::<_, Option<String>>(19)?.as_deref())",
        "track_row_loads_local_pubdate_and_explicit_columns",
    ] {
        assert!(
            db_source.contains(required),
            "Local track DB rows must keep pubdate/explicit loading at the read-model boundary: `{required}`"
        );
    }

    for required in ["pub_date: t.pub_date", "explicit: t.explicit"] {
        assert!(
            views_source.contains(required),
            "TrackView::from_local_with_identity must preserve local pubdate/explicit values: `{required}`"
        );
    }

    for required in [
        "\"Explicit\"",
        "if let Some(explicit) = track.explicit.and_then(explicit_metadata_value)",
        "track.explicit.and_then(explicit_metadata_value)",
        "explicit.then(|| \"Yes\".to_string())",
        "track_metadata_rows_include_local_pubdate_and_explicit_true_only",
    ] {
        assert!(
            metadata_source.contains(required),
            "Track metadata rows must surface explicit only from VM data: `{required}`"
        );
    }

    for required in [
        "if self.track.explicit == Some(true)",
        "TrackDetailSummaryRow::new(\"Explicit\", \"Yes\", 1)",
        "summary_rows_omit_non_explicit_state",
    ] {
        assert!(
            track_detail_source.contains(required),
            "Track detail summary rows must surface explicit only when true: `{required}`"
        );
    }
}

#[test]
fn immediate_view_state_regressions_are_guarded() {
    let app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    for required in [
        "PagedTrackListMsg::PrimeRows(initial_rows.to_vec())",
        "fn refresh_origin_playlist_actor(&mut self)",
        "this.refresh_origin_playlist_actor();",
    ] {
        assert!(
            app_source.contains(required),
            "Library mutations must refresh the currently mounted playlist rows: `{required}`"
        );
    }

    let actor_source = read_source(&manifest_path("src/application/paged_track_list.rs"));
    for required in [
        "PrimeRows(Vec<TrackRow>)",
        "PagedTrackListMsg::PrimeRows(rows)",
        "prime_rows_replaces_cached_body_for_same_playlist_refresh",
    ] {
        assert!(
            actor_source.contains(required),
            "Paged playlist actors must support same-view cache replacement: `{required}`"
        );
    }

    let agent_source = read_source(&manifest_path("AGENTS.md"));
    assert!(
        agent_source.contains("Current-view state must update in place"),
        "Future agents must see the no-navigation-refresh regression rule"
    );
    let troubleshooting_source = read_source(&manifest_path(
        "docs/troubleshooting/immediate-view-state-regressions.md",
    ));
    assert!(
        troubleshooting_source
            .contains("Do not rely on navigation, tab changes, playlist switches"),
        "Immediate view-state regression runbook must record the prohibited fix"
    );
}

#[test]
fn screen_level_fallback_expressions_stay_domain_only() {
    let allowed = [
        (
            "src/library.rs",
            ".unwrap_or_default();",
            "playlist track query failure tolerance is command/domain plumbing, not display fallback",
        ),
        (
            "src/library.rs",
            ".unwrap_or(\"\")",
            "MusicBrainz candidate title scoring fallback is matching logic, not display fallback",
        ),
        (
            "src/library.rs",
            ".unwrap_or_default(),",
            "identity source-fact default is source data hydration, not display fallback",
        ),
        (
            "src/library.rs",
            "let track_context = frame.source_context.clone().unwrap_or(fallback_context);",
            "source context fallback is metadata command context plumbing, not display fallback",
        ),
        (
            "src/library.rs",
            "let feed_id = album.feed_id.unwrap_or(0);",
            "album feed id fallback is command identity plumbing, not display fallback",
        ),
        (
            "src/library.rs",
            "id: album.feed_id.unwrap_or(0),",
            "release detail feed id fallback is command identity plumbing, not display fallback",
        ),
        (
            "src/library.rs",
            "let context = frame.source_context.as_ref().unwrap_or(&context);",
            "metadata source context fallback is command context plumbing, not display fallback",
        ),
        (
            "src/library.rs",
            ".unwrap_or_else(color::text_primary);",
            "metadata cell default color is token render chrome, not label fallback",
        ),
        (
            "src/library.rs",
            ".unwrap_or_else(|| id3_cell_status_color(row, cx));",
            "ID3 status default color is token render chrome, not label fallback",
        ),
        (
            "src/library.rs",
            ".unwrap_or_else(|| comparison_status_color(&row.musicbrainz_status, cx));",
            "MusicBrainz status default color is token render chrome, not label fallback",
        ),
        (
            "src/library.rs",
            ".unwrap_or_else(|_| track_row_to_track_context(track));",
            "debug conversion fallback is data-contract compatibility, not display fallback",
        ),
        (
            "src/discover.rs",
            ".unwrap_or(false);",
            "boolean state fallback is command/control state, not display fallback",
        ),
        (
            "src/discover.rs",
            ".unwrap_or(false)",
            "boolean state fallback is command/control state, not display fallback",
        ),
        (
            "src/discover.rs",
            ".unwrap_or_else(|| row.entity_id.clone()),",
            "result navigation target fallback is identity routing, not display label fallback",
        ),
        (
            "src/discover.rs",
            "artist_track_count_by_feed.get(guid).copied().unwrap_or(0);",
            "artist feed count fallback is numeric aggregation, not display fallback",
        ),
        (
            "src/discover.rs",
            ".unwrap_or_default();",
            "podroll dedupe key fallback is feed identity plumbing, not display fallback",
        ),
        (
            "src/discover.rs",
            ".unwrap_or(artist_context.tracks.len() as i32);",
            "artist track-count fallback is numeric aggregation, not display fallback",
        ),
        (
            "src/discover.rs",
            ".unwrap_or_else(color::text_primary);",
            "metadata cell default color is token render chrome, not label fallback",
        ),
        (
            "src/discover.rs",
            ".unwrap_or_else(|| id3_cell_status_color(row, cx));",
            "ID3 status default color is token render chrome, not label fallback",
        ),
        (
            "src/discover.rs",
            ".unwrap_or_else(|| comparison_status_color(&row.musicbrainz_status, cx));",
            "MusicBrainz status default color is token render chrome, not label fallback",
        ),
        (
            "src/discover.rs",
            "let frame_color = frame_color.unwrap_or_else(color::text_muted);",
            "ID3 frame default color is token render chrome, not label fallback",
        ),
        (
            "src/discover.rs",
            "crate::view_models::track::fmt_dur((ms / 1000).try_into().unwrap_or(i32::MAX))",
            "duration range clamp is numeric conversion safety, not display fallback",
        ),
    ];
    let files = ["src/library.rs", "src/discover.rs"];
    let mut violations = Vec::new();

    for file in files {
        let source = read_source(&manifest_path(file));
        let allowed_lines: Vec<(&str, &str)> = allowed
            .iter()
            .filter(|(allowed_file, _, _)| *allowed_file == file)
            .map(|(_, snippet, note)| (*snippet, *note))
            .collect();

        for (line_number, line) in code_lines(&source) {
            if !line.contains("unwrap_or") {
                continue;
            }
            if line.contains("clippy::") {
                continue;
            }
            let allowed_note = allowed_lines
                .iter()
                .find(|(snippet, _)| line == *snippet)
                .map(|(_, note)| *note);
            if allowed_note.is_none() {
                violations.push(format!(
                    "{file}:{line_number}: screen-level `unwrap_or*` expression is not documented as domain-only: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 screen-level fallback expressions must be VM-owned or explicitly domain-only:\n{}",
        violations.join("\n")
    );
}

#[test]
fn composite_signatures_take_display_contracts_not_loose_strings() {
    let mut violations = Vec::new();

    for path in rust_files_under("src/ui/composites") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, signature) in public_function_signatures(&source) {
            let compact_signature = compact_source(&signature);
            let mentions_string_api = compact_signature.contains("&str")
                || compact_signature.contains("String")
                || compact_signature.contains("SharedString")
                || compact_signature.contains("Into<String>")
                || compact_signature.contains("Into<SharedString>");
            if !mentions_string_api {
                continue;
            }
            let allowed_note = COMPOSITE_DISPLAY_CONTRACT_STRING_API_ALLOWLIST
                .iter()
                .find(|allowance| {
                    allowance.file == file
                        && compact_signature.contains(&compact_source(allowance.pattern))
                })
                .map(|allowance| allowance.note);
            if allowed_note.is_none() {
                violations.push(format!(
                    "{file}:{line_number}: shared composite string-like public API must be display-contract owned or explicitly allowlisted: `{}`",
                    signature.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 composite display-contract signature violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn top_level_gpui_modules_are_classified_as_screen_or_shared_ui() {
    let mut candidates = Vec::new();
    let src_dir = manifest_path("src");
    for entry in
        fs::read_dir(&src_dir).unwrap_or_else(|err| panic!("read {}: {err}", src_dir.display()))
    {
        let entry = entry.expect("read src entry");
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            candidates.push(path);
        }
    }
    let app_dir = manifest_path("src/app");
    if app_dir.is_dir() {
        for entry in
            fs::read_dir(&app_dir).unwrap_or_else(|err| panic!("read {}: {err}", app_dir.display()))
        {
            let entry = entry.expect("read src/app entry");
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                candidates.push(path);
            }
        }
    }
    candidates.sort();

    let mut unclassified = Vec::new();
    for path in candidates {
        let source = read_source(&path);
        let imports_gpui = source
            .lines()
            .map(strip_line_comment)
            .any(|line| line.contains("use gpui") || line.contains("gpui_component::"));
        if !imports_gpui {
            continue;
        }
        let rel = rel_path(&path);
        let classified = SCREEN_FILES.iter().any(|file| *file == rel)
            || PRESENTATION_GLUE_FILES.iter().any(|file| *file == rel);
        if !classified {
            unclassified.push(rel);
        }
    }

    assert!(
        unclassified.is_empty(),
        "ADR 0033 backstop: every top-level GPUI-importing module must be classified as a screen or presentation glue. Shared shells belong under src/ui/shells/. Unclassified files:\n{}",
        unclassified.join("\n")
    );
}

#[test]
fn top_level_shells_live_under_src_ui_shells() {
    let manifest = manifest_path("src");
    let entries = fs::read_dir(&manifest)
        .unwrap_or_else(|err| panic!("read {}: {err}", manifest.display()))
        .filter_map(Result::ok);
    let mut violations = Vec::new();

    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "ui_context.rs" {
            continue;
        }
        if name.starts_with("ui_") && name.ends_with(".rs") {
            violations.push(format!(
                "{name}: top-level shell modules must live under src/ui/shells/, not src/"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 layer relocation violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn shared_ui_callbacks_do_not_smuggle_backend_types() {
    let mut violations = Vec::new();
    for relative_dir in ["src/ui/primitives", "src/ui/composites"] {
        for path in rust_files_under(relative_dir) {
            let source = read_source(&path);
            for (line_number, line) in code_lines(&source) {
                if !line_mentions_callback(&line) {
                    continue;
                }
                for pattern in CALLBACK_BACKEND_TYPE_FORBIDDEN_PATTERNS {
                    if line.contains(pattern) {
                        violations.push(format!(
                            "{}:{line_number}: ADR 0033 shared UI callbacks must not carry backend types; found `{pattern}` in `{line}`",
                            rel_path(&path)
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 callback backend-type smuggling violations:\n{}",
        violations.join("\n")
    );
}

const CALLBACK_BACKEND_TYPE_FORBIDDEN_PATTERNS: &[&str] = &[
    "db::",
    "api::",
    "crate::db",
    "crate::api",
    "feed_service::",
    "library_service::",
    "metadata_service::",
    "playlist_service::",
    "subscribe_service::",
    "track_compare::",
    "rusqlite",
];

fn line_mentions_callback(line: &str) -> bool {
    line.contains("Fn(")
        || line.contains("FnMut(")
        || line.contains("FnOnce(")
        || line.contains("Fn ->")
        || line.contains("dyn Fn")
}

fn rust_files_under(relative_dir: &str) -> Vec<PathBuf> {
    let root = manifest_path(relative_dir);
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);
    files.sort();
    files
}

fn screen_enforcement_files() -> Vec<String> {
    let mut files = SCREEN_FILES
        .iter()
        .map(|file| (*file).to_string())
        .collect::<Vec<_>>();
    for dir in SCREEN_SURFACE_DIRS {
        files.extend(
            rust_files_under(dir)
                .into_iter()
                .map(|path| rel_path(&path)),
        );
    }
    files.sort();
    files.dedup();
    files
}

fn assert_screen_surface_files(surface_name: &str, expected_files: &[&str]) {
    let mut violations = Vec::new();

    for file in expected_files {
        let path = manifest_path(file);
        if !path.is_file() {
            violations.push(format!("{file} is missing"));
            continue;
        }

        let source = read_source(&path);
        if source.trim().is_empty() {
            violations.push(format!("{file} is empty"));
        }
        if !file.ends_with("/mod.rs")
            && !source.contains("pub(crate) fn")
            && !source.contains("pub(super) fn")
        {
            violations.push(format!(
                "{file} must expose at least one bounded screen-surface function"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0038 Task 007 {surface_name} screen decomposition violations:\n{}",
        violations.join("\n")
    );
}

fn non_ui_core_rust_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for relative_path in NON_UI_CORE_PATHS {
        let path = manifest_path(relative_path);
        if path.is_dir() {
            collect_rust_files(&path, &mut files);
        } else {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display())) {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

fn code_lines(source: &str) -> impl Iterator<Item = (usize, String)> + '_ {
    source.lines().enumerate().filter_map(|(index, raw)| {
        let line = strip_line_comment(raw).trim().to_string();
        (!line.is_empty()).then_some((index + 1, line))
    })
}

fn production_source(source: &str) -> &str {
    source.split("#[cfg(test)]").next().unwrap_or(source)
}

fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(index) => &line[..index],
        None => line,
    }
}

fn compact_source(source: &str) -> String {
    source.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("source section missing start marker `{start}`"));
    let rest = &source[start_index..];
    let end_index = rest
        .find(end)
        .unwrap_or_else(|| panic!("source section missing end marker `{end}`"));
    &rest[..end_index]
}

fn assert_fact_key_set(context: &str, source: &str, allowed_keys: &[&str], required_keys: &[&str]) {
    let allowed = allowed_keys.iter().copied().collect::<BTreeSet<_>>();
    let required = required_keys.iter().copied().collect::<BTreeSet<_>>();
    let mut found = BTreeSet::new();
    let mut violations = Vec::new();

    for literal in string_literals(source) {
        if literal.starts_with("$.") || literal == "musicindex" {
            continue;
        }
        if allowed.contains(literal.as_str()) {
            found.insert(literal);
        } else {
            violations.push(format!(
                "{context}: unsupported ADR 0054 metadata fact key `{literal}`"
            ));
        }
    }

    for key in required {
        if !found.contains(key) {
            violations.push(format!(
                "{context}: missing approved ADR 0054 metadata fact key `{key}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0054 metadata fact key violations:\n{}",
        violations.join("\n")
    );
}

fn string_literals(source: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }

        let mut literal = String::new();
        let mut escaped = false;
        for next in chars.by_ref() {
            if escaped {
                literal.push(next);
                escaped = false;
                continue;
            }
            match next {
                '\\' => escaped = true,
                '"' => break,
                _ => literal.push(next),
            }
        }
        literals.push(literal);
    }

    literals
}

fn public_function_signatures(source: &str) -> Vec<(usize, String)> {
    let mut signatures = Vec::new();
    let mut current: Option<(usize, String)> = None;

    for (line_number, line) in code_lines(source) {
        if current.is_none() && !line.contains("pub fn") {
            continue;
        }

        let fragment = strip_line_comment(&line).trim();
        if fragment.is_empty() {
            continue;
        }

        if current.is_none() {
            current = Some((line_number, String::new()));
        }

        let (_, signature) = current
            .as_mut()
            .expect("signature accumulator is initialized above");
        if !signature.is_empty() {
            signature.push(' ');
        }
        signature.push_str(fragment);

        if fragment.contains('{') || fragment.ends_with(';') {
            if let Some((start, signature)) = current.take() {
                signatures.push((start, signature));
            }
        }
    }

    signatures
}

fn contains_numeric_px_literal(line: &str) -> bool {
    line.match_indices("px(").any(|(index, _)| {
        line[index + 3..]
            .trim_start()
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
    })
}

fn contains_unscaled_token_px_call(line: &str) -> bool {
    line.contains(".px()")
        && [
            "Spacing::",
            "Radius::",
            "FontSize::",
            "Size::",
            ".padding.px()",
            ".radius.px()",
            ".size.px()",
        ]
        .iter()
        .any(|pattern| line.contains(pattern))
}

fn shared_ui_unscaled_token_px_is_allowed(file: &str, line: &str) -> bool {
    SHARED_UI_UNSCALED_TOKEN_PX_ALLOWLIST
        .iter()
        .any(|allowance| {
            allowance.file == file && line.contains(allowance.pattern) && !allowance.note.is_empty()
        })
}

fn appearance_dark_is_approved(file: &str, source: &str, line_number: usize) -> bool {
    match file {
        "src/app.rs" => {
            nearby_source_mentions(source, line_number, &["Apply scale change immediately"])
        }
        "src/app/bootstrap.rs" => nearby_source_mentions(
            source,
            line_number,
            &[
                "Pre-config: install with default scale",
                "Re-apply theme now that config has provided",
            ],
        ),
        _ => false,
    }
}

fn deprecated_helper_count(source: &str, baseline: &DeprecatedVisualHelperBaseline) -> usize {
    let imports_helper = source.lines().map(strip_line_comment).any(|line| {
        baseline
            .import_patterns
            .iter()
            .any(|pattern| line.contains(pattern))
    });
    if !imports_helper {
        return 0;
    }

    source
        .lines()
        .map(strip_line_comment)
        .filter(|line| {
            line.contains(baseline.usage_pattern)
                || baseline
                    .import_patterns
                    .iter()
                    .any(|pattern| line.contains(pattern))
        })
        .count()
}

fn deprecated_helper_has_baseline(file: &str, helper: &str) -> bool {
    DEPRECATED_VISUAL_HELPER_BASELINES
        .iter()
        .any(|baseline| baseline.file == file && baseline.helper == helper)
}

fn direct_component_button_baseline(file: &str) -> Option<usize> {
    DIRECT_COMPONENT_BUTTON_BASELINES
        .iter()
        .find(|baseline| baseline.file == file)
        .map(|baseline| baseline.max_unmarked_count)
}

fn render_helper_duplication_baseline(
    helper: &str,
) -> Option<&'static RenderHelperDuplicationBaseline> {
    RENDER_HELPER_DUPLICATION_BASELINES
        .iter()
        .find(|baseline| baseline.helper == helper)
}

fn render_helper_name(line: &str) -> Option<String> {
    let code = line.trim_start();
    let code = code.strip_prefix("pub(crate) ").unwrap_or(code);
    let code = code.strip_prefix("pub(super) ").unwrap_or(code);
    let code = code.strip_prefix("pub ").unwrap_or(code);
    let name_start = code.strip_prefix("fn render_")?;
    let name_end = name_start.find('(')?;
    Some(format!("render_{}", &name_start[..name_end]))
}

fn distinct_location_files(locations: &[String]) -> Vec<String> {
    let mut files = Vec::new();
    for location in locations {
        let Some((file, _line)) = location.rsplit_once(':') else {
            continue;
        };
        if files.iter().all(|existing| existing != file) {
            files.push(file.to_string());
        }
    }
    files
}

fn same_file_set(actual: &[String], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && expected
            .iter()
            .all(|file| actual.iter().any(|actual_file| actual_file == file))
}

fn unmarked_direct_component_button_lines(source: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let uses_component_button = lines
        .iter()
        .any(|line| line.contains("use gpui_component::button::{Button"));
    if !uses_component_button {
        return Vec::new();
    }

    lines
        .iter()
        .enumerate()
        .filter_map(|(index, raw)| {
            let code = strip_line_comment(raw).trim();
            if !code.contains("Button::new(") {
                return None;
            }
            let has_same_line_marker = raw.contains("CONTROL-COMPAT(reason):");
            let has_previous_line_marker = index
                .checked_sub(1)
                .and_then(|previous| lines.get(previous))
                .is_some_and(|line| line.contains("CONTROL-COMPAT(reason):"));
            (!has_same_line_marker && !has_previous_line_marker)
                .then_some((index + 1, code.to_string()))
        })
        .collect()
}

fn nearby_source_mentions(source: &str, line_number: usize, needles: &[&str]) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    let start = line_number.saturating_sub(8);
    let end = (line_number + 8).min(lines.len());
    let context = lines[start..end].join("\n");
    needles.iter().any(|needle| context.contains(needle))
}

fn read_source(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn workspace_vm_source() -> String {
    [
        "src/view_models/workspace/mod.rs",
        "src/view_models/workspace/frame.rs",
        "src/view_models/workspace/chrome.rs",
        "src/view_models/workspace/nav.rs",
        "src/view_models/workspace/breadcrumb.rs",
    ]
    .into_iter()
    .map(|file| read_source(&manifest_path(file)))
    .collect::<Vec<_>>()
    .join("\n")
}

fn search_vm_sources() -> Vec<(String, String)> {
    rust_files_under("src/view_models/search")
        .into_iter()
        .map(|path| {
            let file = rel_path(&path);
            let source = read_source(&path);
            (file, source)
        })
        .collect()
}

fn search_vm_source() -> String {
    search_vm_sources()
        .into_iter()
        .map(|(_, source)| source)
        .collect::<Vec<_>>()
        .join("\n")
}

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn rel_path(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}

const RUNTIME_FORBIDDEN_PATTERNS: &[&str] = &[
    "use gpui",
    "gpui::",
    "use gpui_component",
    "gpui_component::",
    "crate::ui::",
    "crate::ui_",
    "crate::library::",
    "crate::search::",
    "crate::app::",
    "crate::presentation",
];

#[test]
fn runtime_layer_does_not_import_gpui_or_ui() {
    let mut violations = Vec::new();
    for path in rust_files_under("src/runtime") {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in RUNTIME_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{}:{line_number}: ADR 0040 runtime boundary violation `{pattern}` in `{line}`",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0040 runtime layer must not import gpui/ui:\n{}",
        violations.join("\n")
    );
}

#[test]
fn gpui_command_runner_is_retired() {
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let source = read_source(&path);
        if source.contains("GpuiCommandRunner") || source.contains("gpui_command_runner") {
            violations.push(rel_path(&path));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0040 retired GpuiCommandRunner surface reintroduced:\n{}",
        violations.join("\n")
    );
}

#[test]
fn async_runtime_feature_flag_is_retired() {
    let manifest = read_source(&manifest_path("Cargo.toml"));
    assert!(
        !manifest.contains("async-runtime"),
        "ADR 0040 retired the async-runtime Cargo feature; Cargo.toml must not mention it"
    );

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let source = read_source(&path);
        for pattern in [
            "cfg(feature = \"async-runtime\")",
            "cfg(not(feature = \"async-runtime\"))",
            "#![cfg(feature = \"async-runtime\")]",
        ] {
            if source.contains(pattern) {
                violations.push(format!("{}: found `{pattern}`", rel_path(&path)));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0040 retired async-runtime cfg gates:\n{}",
        violations.join("\n")
    );
}

#[test]
fn cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap() {
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        let count = source.matches("cx.spawn(").count();
        if count == 0 {
            continue;
        }
        if file.starts_with("src/presentation/")
            || file.starts_with("src/runtime/")
            || file == "src/app/bootstrap.rs"
        {
            continue;
        }
        violations.push(format!(
            "{file}: {count} cx.spawn calls outside presentation/runtime/bootstrap"
        ));
    }

    assert!(
        violations.is_empty(),
        "ADR 0040 screen-local cx.spawn restriction failed:\n{}",
        violations.join("\n")
    );
}

#[test]
fn musicbrainz_feed_saga_is_runtime_owned() {
    let runtime_source = read_source(&manifest_path("src/runtime/musicbrainz_feed_saga.rs"));
    let runtime_mod_source = read_source(&manifest_path("src/runtime/mod.rs"));
    let library_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let library_struct_source = read_source(&manifest_path("src/library.rs"));

    for required in [
        "pub enum MusicBrainzFeedSagaState",
        "pub struct StartFeedLookup",
        "pub struct MusicBrainzFeedSagaHandle",
        "tokio::sync::{mpsc, watch}",
        "LookupMusicBrainzAlbumReleases",
        "StageMusicBrainzTrack",
        "StageMusicBrainzCandidate",
        "fn match_candidate_to_track",
    ] {
        assert!(
            runtime_source.contains(required),
            "src/runtime/musicbrainz_feed_saga.rs: MusicBrainz feed saga actor missing `{required}`"
        );
    }

    assert!(
        runtime_mod_source.contains("pub mod musicbrainz_feed_saga"),
        "src/runtime/mod.rs: MusicBrainz feed saga module must be registered"
    );

    for forbidden in [
        "cx.spawn(",
        "musicbrainz_feed_per_track(",
        "fn lookup_musicbrainz_stage_for_track(",
        "fn match_candidate_to_track(",
        "fn stage_candidate_for_track(",
    ] {
        assert!(
            !library_source.contains(forbidden),
            "src/library/app_impl.rs: MusicBrainz feed saga must not remain screen-local; found `{forbidden}`"
        );
    }

    for required in [
        "bridge_watch(",
        "StartFeedLookup::new(",
        "apply_musicbrainz_feed_saga_state(",
        "MusicBrainzFeedSagaState::TrackDone",
        "stage_musicbrainz_lookup_for_track(track_id, lookup)",
    ] {
        assert!(
            library_source.contains(required),
            "src/library/app_impl.rs: Library must reduce saga snapshots through existing VM/screen methods; missing `{required}`"
        );
    }

    assert!(
        library_struct_source.contains("musicbrainz_feed_saga: Option<MusicBrainzFeedSagaHandle>"),
        "src/library.rs: LibraryApp must retain the MusicBrainz feed saga handle"
    );
}

#[test]
fn playback_polling_is_runtime_owned() {
    let runtime_source = read_source(&manifest_path("src/runtime/playback_polling.rs"));
    let runtime_mod_source = read_source(&manifest_path("src/runtime/mod.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));

    for required in [
        "pub struct PlaybackTickSnapshot",
        "pub enum PlaybackTickOutcome",
        "pub struct PlaybackPollingHandle",
        "tokio::time::sleep(PLAYBACK_POLL_INTERVAL)",
        "tokio::task::spawn_blocking",
        "PlaybackOwner",
        "PollOutcome",
    ] {
        assert!(
            runtime_source.contains(required),
            "src/runtime/playback_polling.rs: playback polling actor missing `{required}`"
        );
    }

    assert!(
        runtime_mod_source.contains("pub mod playback_polling"),
        "src/runtime/mod.rs: playback polling module must be registered"
    );

    assert!(
        !app_source.contains("cx.spawn(") && !app_source.contains("fn poll_playback_owner("),
        "src/app.rs: playback polling must not remain screen-local"
    );

    for required in [
        "playback_polling: Option<PlaybackPollingHandle>",
        "runtime_host: Option<Arc<crate::presentation::RuntimeHost>>",
        "crate::runtime::playback_polling::spawn(",
        "bridge_watch(",
        "fn apply_playback_tick(",
        "PlaybackTickOutcome::Advanced =>",
        "self.settings_status.clear();",
        "PlaybackTickOutcome::Error(error)",
    ] {
        assert!(
            app_source.contains(required),
            "src/app.rs: TopApp must bridge playback polling snapshots through the reducer; missing `{required}`"
        );
    }
}

#[test]
fn adr_0059_broadcast_observation_actor_is_runtime_owned() {
    let runtime_source = read_source(&manifest_path("src/runtime/broadcast_observation.rs"));
    let runtime_mod_source = read_source(&manifest_path("src/runtime/mod.rs"));
    let mut violations = Vec::new();

    for required in [
        "const BROADCAST_POLL_INTERVAL: Duration = Duration::from_secs(1)",
        "pub struct BroadcastObservationSnapshot",
        "pub enum BroadcastObservationOutcome",
        "pub struct BroadcastObservationHandle",
        "tokio::sync::{oneshot, watch}",
        "tokio::time::sleep(interval)",
        "tokio::task::spawn_blocking",
        "fetch_live_metadata_optional",
        "BroadcastObservationOutcome::NoEvent",
        "BroadcastObservationOutcome::Live",
        "BroadcastObservationOutcome::Empty",
        "BroadcastObservationOutcome::Dead",
        "BroadcastObservationOutcome::Error",
    ] {
        if !runtime_source.contains(required) {
            violations.push(format!(
                "src/runtime/broadcast_observation.rs: ADR 0059 broadcast observation actor missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "use gpui",
        "gpui::",
        "use gpui_component",
        "gpui_component::",
        "crate::db",
        "crate::ui",
        "crate::library",
        "crate::search",
        "crate::app",
    ] {
        if runtime_source.contains(forbidden) {
            violations.push(format!(
                "src/runtime/broadcast_observation.rs: ADR 0059 observation actor must stay GPUI-free and DB-free; found `{forbidden}`"
            ));
        }
    }

    if !runtime_mod_source.contains("pub mod broadcast_observation") {
        violations
            .push("src/runtime/mod.rs: broadcast observation module must be registered".to_owned());
    }
    if !runtime_mod_source.contains("BroadcastObservationHandle")
        || !runtime_mod_source.contains("BroadcastObservationOutcome")
        || !runtime_mod_source.contains("BroadcastObservationSnapshot")
    {
        violations.push(
            "src/runtime/mod.rs: broadcast observation runtime types must be re-exported"
                .to_owned(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 broadcast observation actor violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn global_search_routes_to_content_list() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));

    // Guard: submit_global_search must call open_search_results_in_content_list
    assert!(
        search_dispatch_source.contains("pub(super) fn submit_global_search(")
            && search_dispatch_source.contains("open_search_results_in_content_list("),
        "submit_global_search must route to open_search_results_in_content_list"
    );

    // Guard: Forbid dead old paths
    assert!(
        !app_source.contains("SubmitModifier")
            && !search_dispatch_source.contains("SubmitModifier")
            && !app_source.contains("submit_global_search_with(")
            && !search_dispatch_source.contains("submit_global_search_with(")
            && !app_source.contains("fn dispatch_active_frame_search("),
        "Legacy paths (SubmitModifier, submit_global_search_with, dispatch_active_frame_search) must be removed"
    );
}

#[test]
fn index_artist_activation_is_scoped_feed_route_not_detail_page() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let workspace_nav_source = read_source(&manifest_path("src/view_models/workspace/nav.rs"));
    let workspace_breadcrumb_source =
        read_source(&manifest_path("src/view_models/workspace/breadcrumb.rs"));
    let workspace_tests_source = read_source(&manifest_path("src/view_models/workspace/tests.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let index_detail_source = read_source(&manifest_path(
        "src/view_models/search_results/index_detail.rs",
    ));
    let retired_artist_detail_route_name = concat!("IndexArtist", "Detail");

    for (path, source) in [
        ("src/app.rs", app_source.as_str()),
        (
            "src/app/search_dispatch.rs",
            search_dispatch_source.as_str(),
        ),
        (
            "src/view_models/workspace/nav.rs",
            workspace_nav_source.as_str(),
        ),
        (
            "src/view_models/workspace/breadcrumb.rs",
            workspace_breadcrumb_source.as_str(),
        ),
        (
            "src/view_models/workspace/tests.rs",
            workspace_tests_source.as_str(),
        ),
        ("src/library/app_impl.rs", library_app_source.as_str()),
    ] {
        assert!(
            !source.contains(retired_artist_detail_route_name),
            "{path}: Index artist activation must be named as a scoped feed-results route"
        );
    }

    assert!(
        workspace_nav_source.contains("IndexArtistFeedScope(String)")
            && search_dispatch_source
                .contains("FrameNavigationEntry::IndexArtistFeedScope(artist_name.to_string())")
            && app_source.contains("FrameNavigationEntry::IndexArtistFeedScope(_)")
            && app_source.contains("SearchResultsHeaderMode::Scoped")
            && app_source.contains("tab: SearchResultsTab::Feeds")
            && app_source.contains("filter: ContentFilter::Index"),
        "Index artist activation must route to scoped Index feed results"
    );
    assert!(
        workspace_tests_source.contains("display.segments[2].target")
            && workspace_tests_source.contains("FrameNavigationEntry::IndexArtistFeedScope")
            && workspace_tests_source.contains("the immediate Index parent must stay selectable"),
        "breadcrumb tests must keep the scoped artist feed parent selectable"
    );
    assert!(
        !index_detail_source.contains("IndexDetailKind::Artist")
            && !app_source.contains("ArtistDetailPageVm")
            && !search_dispatch_source.contains("ArtistDetailPageVm"),
        "Index artist rows must not invent an Index artist detail kind or reuse Library artist detail VM"
    );
}

#[test]
fn nav_top_drives_content_list_body_switch() {
    let app_source = read_source(&manifest_path("src/app.rs"));

    // Guard: render_workspace_content must match on all nav top variants
    for nav_variant in [
        "FrameNavigationEntry::Search(_)",
        "FrameNavigationEntry::TrackDetail(_)",
        "FrameNavigationEntry::AlbumDetail(_)",
        "FrameNavigationEntry::ArtistDetail(_)",
        "FrameNavigationEntry::PlaylistDetail(_)",
        "FrameNavigationEntry::IndexArtistFeedScope(_)",
        "FrameNavigationEntry::IndexFeedDetail { .. }",
        "FrameNavigationEntry::IndexTrackDetail { .. }",
        "FrameNavigationEntry::Settings",
        "FrameNavigationEntry::SourceList",
    ] {
        assert!(
            app_source.contains(nav_variant),
            "render_workspace_content body switch must explicitly match on `{nav_variant}`"
        );
    }

    // Ensure the match pattern is in render_workspace_content
    assert!(
        app_source.contains("fn render_workspace_content(")
            && app_source.contains("match &current_nav"),
        "render_workspace_content must have exhaustive match on nav top"
    );

    assert!(
        !app_source.contains(".content_list(active_screen)"),
        "ContentList body must be selected from nav top, not the active toolbar tab mount"
    );
}

#[test]
fn shared_search_result_rows_accept_resolved_artwork_thumbnails() {
    let result_row_shell_source =
        read_source(&manifest_path("src/ui/shells/search_result_rows.rs"));
    let search_inspector_source =
        read_source(&manifest_path("src/ui/shells/search_results_inspector.rs"));
    let search_vm_source = read_source(&manifest_path("src/view_models/search_results/mod.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));

    assert!(
        result_row_shell_source.contains(".image(thumbnail)"),
        "src/ui/shells/search_result_rows.rs: shared result rows must accept resolved artwork slots"
    );

    for required in [
        "thumbnail_href: Option<&'a str>",
        "row.thumbnail_href.as_deref()",
    ] {
        assert!(
            result_row_shell_source.contains(required),
            "src/ui/shells/search_result_rows.rs: shared result row fields must expose row artwork hrefs; missing `{required}`"
        );
    }

    for required in ["thumbnail_hrefs_for_scope(", "visible_thumbnail_hrefs("] {
        assert!(
            search_vm_source.contains(required),
            "src/view_models/search_results/mod.rs: search-results VM must expose visible thumbnail hrefs; missing `{required}`"
        );
    }

    for required in [
        "thumbnails: BTreeMap<String, Option<Arc<Image>>>",
        "pub(crate) fn with_thumbnails(",
        "fn thumbnail_for_href(&self, href: &str) -> Option<Arc<Image>>",
        ".thumbnail_href",
        "thumbnail_for_href(href)",
    ] {
        assert!(
            search_inspector_source.contains(required),
            "src/ui/shells/search_results_inspector.rs: search result rows must consume resolved artwork slots; missing `{required}`"
        );
    }

    for required in [
        "resolve_search_result_thumbnails(",
        "thumbnail_hrefs_for_scope(",
        "index_remote_detail_hero_image(&href, cx)",
        ".with_thumbnails(thumbnails)",
    ] {
        assert!(
            app_source.contains(required),
            "src/app.rs: search result rows must resolve artwork through TopApp image cache; missing `{required}`"
        );
    }
}

#[test]
fn index_feed_detail_track_rows_preserve_artwork_fallbacks() {
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));

    for required in [
        "thumbnail: self.index_track_row_thumbnail(feed, track, cx)",
        "fn index_track_row_artwork_url",
        "index_track_artwork_url(track).or_else(|| index_feed_artwork_url(feed))",
    ] {
        assert!(
            search_dispatch_source.contains(required),
            "src/app/search_dispatch.rs: Index feed detail track rows must receive track/feed artwork thumbnails; missing `{required}`"
        );
    }
}

#[test]
fn adr_0048_removes_search_tab_and_workspace_mount() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let toolbar_vm_source = read_source(&manifest_path("src/view_models/app_toolbar.rs"));
    let toolbar_source = read_source(&manifest_path("src/app/tab_bar.rs"));
    let keyboard_source = read_source(&manifest_path("src/app/keyboard.rs"));

    for forbidden in [
        "AppTab::Search",
        "WorkspaceScreenMount::Search",
        "AppToolbarTabKey::Search",
        "search_tab_focus",
        "SelectDiscoverTab",
    ] {
        assert!(
            !app_source.contains(forbidden)
                && !toolbar_vm_source.contains(forbidden)
                && !toolbar_source.contains(forbidden)
                && !keyboard_source.contains(forbidden),
            "ADR 0048 retired the Search tab/mount; found `{forbidden}`"
        );
    }

    assert!(
        toolbar_vm_source.contains("tabs: [AppToolbarTabDisplay; 3]")
            && toolbar_vm_source.contains("label: \"Music\"")
            && toolbar_vm_source.contains("label: \"Show\"")
            && toolbar_vm_source.contains("label: \"Settings\""),
        "toolbar VM must expose the ADR 0060 app sections while keeping Search out of tabs"
    );
}

#[test]
fn adr_0048_library_settings_tabs_drive_content_list_nav() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let workspace_source = workspace_vm_source();

    for required in [
        "fn select_tab(&mut self, tab: AppTab",
        "AppTab::Settings =>",
        ".reset_nav(content_list_id, FrameNavigationEntry::Settings)",
        "last_music_content_nav: Option<FrameNavigationState>",
        "self.last_music_content_nav = Some(nav)",
        "replace_nav(content_list_id, nav)",
        "WorkspaceFrameKind::SourceList | WorkspaceFrameKind::ContentList",
        "FrameNavigationEntry::Settings",
    ] {
        assert!(
            app_source.contains(required) || workspace_source.contains(required),
            "Library/Settings tab switching must be owned by ContentList nav; missing `{required}`"
        );
    }
}

#[test]
fn adr_0048_content_list_frame_back_is_wired() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let workspace_shell_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));

    for required in [
        "fn handle_content_list_back_select(",
        "self.workspace_layout.pop_nav(content_list_id)",
        "on_content_list_back_select",
        ".on_back(move |window, cx|",
    ] {
        assert!(
            app_source.contains(required) || workspace_shell_source.contains(required),
            "ContentList frame back must be wired through workspace shell; missing `{required}`"
        );
    }
}

#[test]
fn adr_0048_forbids_secondary_search_frame_path() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let workspace_source = workspace_vm_source();

    assert!(
        !app_source.contains("submit_global_search_with(")
            && !search_dispatch_source.contains("submit_global_search_with(")
            && !app_source.contains("SubmitModifier")
            && !search_dispatch_source.contains("SubmitModifier")
            && !workspace_source.contains("open_search_results_frame("),
        "ADR 0048 forbids secondary/new-frame toolbar search paths"
    );
}

#[test]
fn adr_0048_index_search_is_async_and_vm_owned() {
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let search_query_source = read_source(&manifest_path("src/application/queries/search.rs"));
    let search_results_vm_source =
        read_source(&manifest_path("src/view_models/search_results/mod.rs"));
    let inspector_source = read_source(&manifest_path("src/ui/shells/search_results_inspector.rs"));

    for required in [
        "IndexSearchResultRows",
        "fn mark_index_loading(",
        "fn replace_index_results(",
        "fn set_index_error(",
        "fn is_index_loading(",
    ] {
        assert!(
            search_results_vm_source.contains(required),
            "SearchResultsInspectorPageVm must own Index loading/results/error state; missing `{required}`"
        );
    }

    for required in [
        "fn start_index_search_for_query(",
        "FetchIndexSearchResults::new(",
        "present_command(",
        "content_list_nav_matches_search",
        "detail.replace_index_results(rows)",
        "set_index_error(",
    ] {
        assert!(
            search_dispatch_source.contains(required),
            "TopApp must present Index search results and race-guard ContentList nav; missing `{required}`"
        );
    }

    for required in [
        "pub(crate) struct FetchIndexSearchResults",
        "impl ApplicationCommand for FetchIndexSearchResults",
        "fetch_index_search_result_rows(",
        "fetch_index_feed_result_rows(",
        "fetch_index_track_result_rows(",
        "Some(\"feed\")",
        "Some(\"track\")",
        "index_artist_candidates_from_track(",
    ] {
        assert!(
            search_query_source.contains(required),
            "src/application/queries/search.rs: Index search query command missing `{required}`"
        );
    }

    assert!(
        inspector_source.contains("vm.is_index_loading()")
            && inspector_source.contains("render_pending_result_row(tab, kind, index)"),
        "SearchResultsInspector renderer must expose VM-owned loading via pending rows"
    );
}

#[test]
fn breadcrumb_pop_syncs_library_detail() {
    let breadcrumb_source = read_source(&manifest_path("src/app/breadcrumb.rs"));

    // Guard: handle_content_list_breadcrumb_select must call hydrate_detail_from_nav
    assert!(
        breadcrumb_source.contains("fn handle_content_list_breadcrumb_select(")
            && breadcrumb_source.contains("hydrate_detail_from_nav"),
        "handle_content_list_breadcrumb_select must call hydrate_detail_from_nav to sync LibraryApp detail"
    );
}

#[test]
fn search_results_detail_syncs_with_search_nav_flow() {
    let breadcrumb_source = read_source(&manifest_path("src/app/breadcrumb.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));

    // Guard: sync_search_results_detail_with_nav must exist and be called.
    assert!(
        search_dispatch_source.contains("fn sync_search_results_detail_with_nav("),
        "src/app/search_dispatch.rs must define sync_search_results_detail_with_nav helper"
    );

    // Guard: it must be called from handle_content_list_breadcrumb_select
    assert!(
        breadcrumb_source.contains("fn handle_content_list_breadcrumb_select(")
            && breadcrumb_source.contains("self.sync_search_results_detail_with_nav("),
        "sync_search_results_detail_with_nav must be called from handle_content_list_breadcrumb_select"
    );

    // Guard: it must be called from handle_search_result_selected
    assert!(
        search_dispatch_source.contains("fn handle_search_result_selected(")
            && count_matches(&search_dispatch_source, "self.sync_search_results_detail_with_nav(")
                >= 2,
        "sync_search_results_detail_with_nav must be called from both breadcrumb and result-select handlers"
    );
}

fn count_matches(source: &str, pattern: &str) -> usize {
    source.matches(pattern).count()
}

/// ADR 0056: transport policy has exactly one owner.
///
/// The rules used to live at each call site, which is how one of five media
/// fetches shipped with no redirect handling while two others were being fixed
/// in the same file. A second implementation anywhere is a defect even if it
/// currently behaves correctly.
#[test]
fn adr_0056_media_transport_has_one_owner() {
    const TRANSPORT_OWNER: &str = "src/remote_media.rs";
    const TRANSPORT_MARKERS: &[(&str, &str)] = &[
        ("is_redirection()", "redirect handling"),
        ("header::LOCATION", "redirect Location parsing"),
        ("redirect::Policy", "client redirect policy"),
        ("reqwest::blocking::get(", "ad-hoc media fetch"),
    ];

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let relative = rel_path(&path);
        if relative.ends_with(TRANSPORT_OWNER) {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for (marker, what) in TRANSPORT_MARKERS {
                if line.contains(marker) {
                    violations.push(format!(
                        "{relative}:{line_number}: {what} belongs in {TRANSPORT_OWNER}, found `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0056 transport ownership violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0056: modules that write media artifacts state content rules only. They
/// call the transport module for bytes rather than speaking HTTP themselves.
#[test]
fn adr_0056_artifact_owners_do_not_speak_http() {
    const ARTIFACT_OWNERS: &[&str] = &[
        "src/track_compare.rs",
        "src/audio_tags.rs",
        "src/media/image_cache.rs",
        "src/media/image_type.rs",
    ];

    let mut violations = Vec::new();
    for owner in ARTIFACT_OWNERS {
        let source = read_source(&manifest_path(owner));
        for (line_number, line) in code_lines(&source) {
            if line.contains("reqwest") {
                violations.push(format!(
                    "{owner}:{line_number}: artifact owners fetch through `remote_media`, found `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0056 artifact owner HTTP violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0056: image classification has one owner, and it is not the tag writer.
///
/// While the sniffer was private to `audio_tags`, the thumbnail cache could not
/// reach it and trusted the response `Content-Type` instead, so a real JPEG
/// served as `application/octet-stream` produced no artwork at all.
#[test]
fn adr_0056_image_classification_has_one_owner() {
    const CLASSIFIER: &str = "src/media/image_type.rs";
    // Unambiguous markers only: the full PNG signature, and the old helper
    // names. `RIFF` is shared with WAV (owned by `audio_format`) and `WEBP`
    // collides with ID3 `ARTISTWEBPAGE` frame labels.
    const MAGIC_BYTE_MARKERS: &[&str] = &["\\x89PNG\\r\\n\\x1a\\n", "image_mime_type"];

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let relative = rel_path(&path);
        if relative.ends_with(CLASSIFIER) {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for marker in MAGIC_BYTE_MARKERS {
                if line.contains(marker) {
                    violations.push(format!(
                        "{relative}:{line_number}: image classification belongs in {CLASSIFIER}, found `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0056 image classification ownership violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0056: no path guesses a format for unrecognized bytes.
///
/// Both fallbacks below shipped in the original implementation. Guessing turns
/// a clear failure into a mystery: `unwrap_or(declared_format)` relabeled a
/// redirect body as the expected audio format, and `unwrap_or(ImageFormat::Jpeg)`
/// handed markup to the JPEG decoder.
#[test]
fn adr_0056_no_silent_format_fallbacks() {
    const FORBIDDEN_FALLBACKS: &[&str] = &[
        "unwrap_or(ImageFormat::Jpeg)",
        "unwrap_or(declared_format)",
        "unwrap_or_else(|| ImageFormat::Jpeg)",
    ];

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for fallback in FORBIDDEN_FALLBACKS {
                if line.contains(fallback) {
                    violations.push(format!(
                        "{}:{line_number}: silent format fallback `{fallback}` reintroduced",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0056 silent format fallback violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0056 with ADR 0015: media fetching stays out of the UI layers.
#[test]
fn adr_0056_ui_layers_do_not_fetch_media() {
    let mut violations = Vec::new();
    for dir in ["src/ui", "src/view_models"] {
        for path in rust_files_under(dir) {
            let source = read_source(&path);
            for (line_number, line) in code_lines(&source) {
                if line.contains("reqwest") || line.contains("remote_media") {
                    violations.push(format!(
                        "{}:{line_number}: media fetching belongs in non-UI services, found `{line}`",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0056 UI media fetch violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0056: the cover-art fetch stays on the transport module.
///
/// `subscribe_service` cannot be banned from `reqwest` wholesale -- it builds
/// the MusicBrainz client, which is a document fetch and deliberately outside
/// this boundary. So the guard scopes to `download_image` itself, which is the
/// media fetch in that module.
#[test]
fn adr_0056_cover_art_fetch_uses_the_transport_module() {
    const OWNER: &str = "src/subscribe_service.rs";
    const FN_SIGNATURE: &str = "pub fn download_image(";

    let source = read_source(&manifest_path(OWNER));
    let start = source
        .find(FN_SIGNATURE)
        .unwrap_or_else(|| panic!("{OWNER} no longer defines `{FN_SIGNATURE}`"));
    // The function body ends at the first closing brace in column zero.
    let body_end = source[start..]
        .find("\n}")
        .map(|offset| start + offset)
        .unwrap_or(source.len());
    let body = &source[start..body_end];

    assert!(
        body.contains("remote_media::fetch"),
        "{OWNER}: `download_image` must fetch through the transport module"
    );

    let mut violations = Vec::new();
    for marker in [".send()", "error_for_status", "reqwest::blocking::get"] {
        if body.contains(marker) {
            violations.push(format!(
                "{OWNER}: `download_image` performs its own HTTP (`{marker}`)"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0056 cover-art transport violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0056: APIC artwork is accepted on recognized bytes alone.
///
/// The invariant is stricter than the display paths on purpose: APIC writes an
/// artifact, so a declared `image/*` type on a 200 response is not sufficient.
#[test]
fn adr_0056_apic_does_not_accept_declared_type_alone() {
    const OWNER: &str = "src/audio_tags.rs";

    let source = read_source(&manifest_path(OWNER));
    let mut violations = Vec::new();
    for (line_number, line) in code_lines(&source) {
        if line.contains("image_type::classify") {
            violations.push(format!(
                "{OWNER}:{line_number}: APIC must use byte recognition only, not the \
                 declared-type fallback in `classify`: `{line}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0056 APIC classification violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0058: HTTP client construction has one owner.
///
/// The risk is not a missing timeout today -- `reqwest`'s blocking default
/// supplies one -- but an unowned value that a dependency bump can move, and
/// per-site builders drifting apart the way the media fetch policy did before
/// ADR 0056.
#[test]
fn adr_0058_http_clients_are_built_by_one_owner() {
    const OWNER: &str = "src/http_client.rs";
    const CONSTRUCTORS: &[&str] = &[
        "blocking::Client::new()",
        "ReqwestClient::new()",
        "blocking::Client::builder()",
        "ReqwestClient::builder()",
    ];

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let relative = rel_path(&path);
        if relative.ends_with(OWNER) {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for constructor in CONSTRUCTORS {
                if line.contains(constructor) {
                    violations.push(format!(
                        "{relative}:{line_number}: build HTTP clients through {OWNER} so the \
                         timeout policy is owned in one place: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0058 HTTP client ownership violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0064: `local_files.path` is relative to `music_dir`.
#[test]
fn adr_0064_local_file_paths_resolve_only_through_library_path() {
    const OWNER: &str = "src/library_path.rs";
    const FORBIDDEN_PATTERNS: &[&str] = &[
        ".strip_prefix(music_dir",
        ".strip_prefix(&music_dir",
        ".strip_prefix(cfg.music_dir",
        ".strip_prefix(&cfg.music_dir",
        ".strip_prefix(self.music_dir",
        ".strip_prefix(&self.music_dir",
        "music_dir.join(local_path",
        "music_dir.join(&local_path",
        "cfg.music_dir.join(local_path",
        "cfg.music_dir.join(&local_path",
        "self.music_dir.join(local_path",
        "self.music_dir.join(&local_path",
        ".music_dir.join(local_path",
        ".music_dir.join(&local_path",
    ];

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let relative = rel_path(&path);
        if relative == OWNER {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{relative}:{line_number}: ADR 0064 local file paths must use {OWNER} \
                         for stored path construction and resolution: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0064 local file path resolver violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0065: payment-route tag repair stays an explicit command.
#[test]
fn adr_0065_payment_route_repair_stays_in_command_boundary() {
    const OWNER: &str = "src/application/commands/payment_routes.rs";
    const COMMAND_ENTRYPOINTS: &[&str] = &[
        "RepairPaymentRoutesForTrack",
        "RepairMissingPaymentRouteTags",
        "repair_payment_routes_for_track_with_client",
        "repair_missing_payment_routes_with_client",
    ];
    const FEED_FORBIDDEN: &[&str] = &[
        "RepairPaymentRoutesForTrack",
        "RepairMissingPaymentRouteTags",
        "repair_payment_routes",
        "repair-routes",
        "payment_routes_absent",
    ];

    let mut violations = Vec::new();
    let owner_source = read_source(&manifest_path(OWNER));
    for required in [
        "Client::new_with_base_url",
        "const PAYMENT_ROUTES_INCLUDE: &str = \"payment_routes\"",
        "id3_edits_for_track_context",
        "write_id3v24_edits",
        "NoRoutesUpstream",
    ] {
        if !owner_source.contains(required) {
            violations.push(format!(
                "{OWNER}: ADR 0065 payment-route repair owner is missing `{required}`"
            ));
        }
    }

    let feed_source = read_source(&manifest_path("src/feed_service.rs"));
    for (line_number, line) in code_lines(&feed_source) {
        for forbidden in FEED_FORBIDDEN {
            if line.contains(forbidden) {
                violations.push(format!(
                    "src/feed_service.rs:{line_number}: ADR 0065 keeps payment-route tag repair \
                     out of feed refresh and inside {OWNER}: `{line}`"
                ));
            }
        }
    }

    for path in rust_files_under("src") {
        let relative = rel_path(&path);
        if relative == OWNER
            || relative == "src/application/commands/mod.rs"
            || relative == "src/application/commands/feed.rs"
            || relative == "src/cli.rs"
            || relative == "src/library/app_impl.rs"
        {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for entrypoint in COMMAND_ENTRYPOINTS {
                if line.contains(entrypoint) {
                    violations.push(format!(
                        "{relative}:{line_number}: ADR 0065 payment-route repair runs only \
                         through commands owned by {OWNER}: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0065 payment-route repair boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0065: readiness row state labels are not actions.
#[test]
fn adr_0065_readiness_rows_keep_state_labels_separate_from_actions() {
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let content_shell_source = read_source(&manifest_path("src/ui/shells/library/content_list.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let feed_command_source = read_source(&manifest_path("src/application/commands/feed.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) action: Option<ContentListRowActionDisplay>",
        "pub(crate) activation: ContentListRowActivation",
        "ContentListRowActionKind::RepairBroadcastRoutes",
        "BROADCAST_ROUTE_REPAIR_AVAILABLE_LABEL",
        "BROADCAST_ROUTE_PUBLISHER_DETAIL",
    ] {
        if !library_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: ADR 0065 readiness row action contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "\"Fix routes\"",
        "\"Fixing...\"",
        "\"Publisher must add payment routes.\"",
    ] {
        if content_shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/library/content_list.rs: ADR 0065 row action and publisher labels belong to the view model; found `{forbidden}`"
            ));
        }
    }

    let render_row = source_between(
        &content_shell_source,
        "fn render_content_list_row(",
        "fn render_content_list_tile(",
    );
    if !render_row.contains("row.accepts_row_click()") {
        violations.push(
            "src/ui/shells/library/content_list.rs: ADR 0065 readiness rows must not inherit unconditional whole-row click handlers"
                .to_string(),
        );
    }
    if !render_row.contains("render_content_list_row_action(action, cx)") {
        violations.push(
            "src/ui/shells/library/content_list.rs: ADR 0065 readiness row actions must render through the explicit row action contract"
                .to_string(),
        );
    }
    let state_label_block = source_between(
        render_row,
        "if let Some(state_label) = row.state_label",
        "if let Some(action) = row.action.as_ref()",
    );
    for forbidden in [".on_click", "UiButton::styled", ".cursor_pointer()"] {
        if state_label_block.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/library/content_list.rs: ADR 0065 state labels must not carry click handlers or button styling; found `{forbidden}`"
            ));
        }
    }

    let render_action = source_between(
        &content_shell_source,
        "fn render_content_list_row_action(",
        "fn render_content_list_load_more(",
    );
    for required in [
        "action.label.clone()",
        "action.a11y_label.clone()",
        "action.disabled()",
        "this.run_content_list_row_action(kind, cx)",
    ] {
        if !render_action.contains(required) {
            violations.push(format!(
                "src/ui/shells/library/content_list.rs: ADR 0065 row action renderer must consume VM action data; missing `{required}`"
            ));
        }
    }

    let check_all = source_between(
        &library_app_source,
        "fn check_all_feeds(",
        "fn apply_all_feed_updates(",
    );
    for required in [
        "CheckFeedsAndRepairRoutes::new",
        "finish_all_feed_check_with_route_repair",
        "refresh_current_broadcast_readiness_report()",
    ] {
        if !check_all.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0065 Check all feeds must check, apply, repair, and refresh; missing `{required}`"
            ));
        }
    }

    let row_repair = source_between(
        &library_app_source,
        "fn repair_broadcast_routes_for_track(",
        "pub(crate) fn select_track(",
    );
    for required in [
        "RepairPaymentRoutesForTrack::new",
        "begin_broadcast_route_repair(track_id)",
        "finish_broadcast_route_repair",
        "refresh_current_broadcast_readiness_report()",
    ] {
        if !row_repair.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0065 row repair must use the single-track command and refresh the readiness list; missing `{required}`"
            ));
        }
    }

    let combined_command = source_between(
        &feed_command_source,
        "impl ApplicationCommand for CheckFeedsAndRepairRoutes",
        "/// Command result for subscribing/downloading a feed.",
    );
    let check_index = combined_command.find("CheckSubscribedFeeds::new");
    let apply_index = combined_command.find("ApplyFeedUpdates::new");
    let repair_index = combined_command.find("RepairMissingPaymentRouteTags::new");
    match (check_index, apply_index, repair_index) {
        (Some(check), Some(apply), Some(repair)) if check < apply && apply < repair => {}
        _ => violations.push(
            "src/application/commands/feed.rs: ADR 0065 Check all feeds must repair only after stale feed updates apply"
                .to_string(),
        ),
    }

    assert!(
        violations.is_empty(),
        "ADR 0065 readiness row action violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0059: v4vmm reads live metadata but does not publish it.
#[test]
fn adr_0059_v4vmm_does_not_publish_live_metadata() {
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("publish_live_metadata") {
                violations.push(format!(
                    "{}:{line_number}: ADR 0059 removes the v4vmm live metadata publish path: `{line}`",
                    rel_path(&path)
                ));
            }
        }
    }

    let api_source = read_source(&manifest_path("src/api.rs"));
    let api_compact = api_source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    for call in [
        "post_json(&[\"v1\",\"liveitems\",event_id,\"metadata\"]",
        "post_json_with_bearer(&[\"v1\",\"liveitems\",event_id,\"metadata\"]",
    ] {
        if api_compact.contains(call) {
            violations.push(format!(
                "src/api.rs: ADR 0059 keeps metadata relay access read-only; found `{call}`"
            ));
        }
    }

    let cli_source = read_source(&manifest_path("src/cli.rs"));
    for (line_number, line) in code_lines(&cli_source) {
        if line.contains("--token")
            || line.contains("MUSICINDEX_LIVEITEM_TOKEN")
            || line.contains("broadcaster_token")
            || line.contains("create_live_item")
        {
            violations.push(format!(
                "src/cli.rs:{line_number}: ADR 0059 forbids broadcaster tokens in v4vmm CLI commands: `{line}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 live metadata publish violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0059: broadcast tokens stay in files, not UI or database text columns.
#[test]
fn adr_0059_broadcast_event_schema_keeps_tokens_out_of_storage_and_ui() {
    let mut violations = Vec::new();
    let db_source = read_source(&manifest_path("src/db.rs"));
    let schema_start = db_source
        .find("CREATE TABLE IF NOT EXISTS broadcast_events")
        .expect("broadcast_events schema should exist");
    let schema_end = db_source[schema_start..]
        .find("CREATE INDEX IF NOT EXISTS idx_broadcast_events_created_at")
        .map(|offset| schema_start + offset)
        .expect("broadcast_events schema should include its first index");
    let schema_line_start = db_source[..schema_start].lines().count() + 1;
    let schema = &db_source[schema_start..schema_end];
    for (offset, line) in schema.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("token ") || trimmed.starts_with("token\t") {
            violations.push(format!(
                "src/db.rs:{}: ADR 0059 stores token_path only, not token text: `{}`",
                schema_line_start + offset,
                trimmed.trim_end()
            ));
        }
    }

    for dir in ["src/ui", "src/view_models"] {
        for path in rust_files_under(dir) {
            let source = read_source(&path);
            for (line_number, line) in code_lines(&source) {
                if line.contains("broadcast_events") {
                    violations.push(format!(
                        "{}:{line_number}: ADR 0059 broadcast event table access belongs outside UI/view-model layers: `{line}`",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 broadcast event storage violations:\n{}",
        violations.join("\n")
    );
}

/// ADR 0059: broadcast services are GPUI-free and use the shared API client.
#[test]
fn adr_0059_broadcast_services_stay_gpui_free_and_use_api_client() {
    let mut violations = Vec::new();
    for path in rust_files_under("src/broadcast") {
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for forbidden in [
                "use gpui",
                "gpui::",
                "use gpui_component",
                "gpui_component::",
                "reqwest::",
                "crate::http_client",
                "http_client::",
            ] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number}: ADR 0059 broadcast services must stay GPUI-free and build relay clients through api::Client; found `{forbidden}` in `{line}`",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 broadcast service boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: systemd service commands stay in broadcast control.
#[test]
fn adr_0059_publisher_service_control_boundary_is_broadcast_owned() {
    let mut violations = Vec::new();
    let control_source = read_source(&manifest_path("src/broadcast/control.rs"));
    let broadcast_mod_source = read_source(&manifest_path("src/broadcast/mod.rs"));

    for required in [
        "pub mod control;",
        "pub trait CommandRunner",
        "const SYSTEMCTL: &str = \"systemctl\"",
        "const JOURNALCTL: &str = \"journalctl\"",
        "--property=LoadState,ActiveState,SubState,Result",
        "reset-failed",
        "pub fn show(&self, transport: &Transport, unit: &UnitRef)",
        "pub fn logs(&self, transport: &Transport, unit: &UnitRef, lines: usize)",
    ] {
        if !control_source.contains(required) && !broadcast_mod_source.contains(required) {
            violations.push(format!(
                "src/broadcast/control.rs: Situational ADR 0059 service-control boundary missing `{required}`. Fix: keep systemd command construction and journal reads in broadcast control."
            ));
        }
    }

    for path in rust_files_under("src") {
        if rel_path(&path) == "src/broadcast/control.rs" {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for forbidden in ["systemctl", "journalctl"] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number}: Situational ADR 0059 service-control boundary forbids `{forbidden}` outside src/broadcast/control.rs. Fix: route user service commands through broadcast::control.",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 service-control boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Packet 010 SSH command construction stays in broadcast.
#[test]
fn adr_0059_ssh_transport_boundary_is_broadcast_owned() {
    let mut violations = Vec::new();
    let transport_source = read_source(&manifest_path("src/broadcast/transport.rs"));
    let broadcast_mod_source = read_source(&manifest_path("src/broadcast/mod.rs"));

    for required in [
        "pub mod transport;",
        "pub enum Transport",
        "Ssh {",
        "const SSH: &str = \"ssh\"",
        "BatchMode=yes",
        "ConnectTimeout=5",
        "fn ssh_args(",
        "Reachability::NotReachable",
    ] {
        if !transport_source.contains(required) && !broadcast_mod_source.contains(required) {
            violations.push(format!(
                "src/broadcast/transport.rs: Situational ADR 0059 Packet 010 SSH transport missing `{required}`. Fix: keep SSH wrapping and reachability classification in broadcast::transport."
            ));
        }
    }

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        if file.starts_with("src/broadcast/") || file == "src/config.rs" {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for forbidden in [
                "Command::new(\"ssh\")",
                "Command::new(SSH",
                "std::process::Command::new(\"ssh\")",
                ".run(\"ssh\"",
                "runner.run(\"ssh\"",
                "program: \"ssh\"",
                "\"ssh\"",
            ] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number}: Situational ADR 0059 Packet 010 forbids SSH command construction outside src/broadcast. Fix: route remote host commands through broadcast::transport.",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 Packet 010 SSH transport boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Packet 011 mpv drop-file production stays in broadcast.
#[test]
fn adr_0059_mpv_drop_file_producer_boundary_is_broadcast_owned() {
    let mut violations = Vec::new();
    let producer_source = read_source(&manifest_path("src/broadcast/producer.rs"));
    let production_source = producer_source.split("#[cfg(test)]").next().unwrap_or("");
    let broadcast_mod_source = read_source(&manifest_path("src/broadcast/mod.rs"));

    for required in [
        "pub mod producer;",
        "pub struct DropFileProducer",
        "pub const DROP_FILE_SCHEMA: &str = \"musicindex.nowplaying/1\"",
        "pub const DROP_FILE_SUFFIX: &str = \".nowplaying.json\"",
        "TXXX:MusicIndex Value Routes",
        "EMBEDDED_ID3_ROUTES_SOURCE: &str = \"embedded-id3\"",
        "fn write_atomic(&self, content: &str)",
        "fs::rename(&temp_path, &self.path)",
        "remove_file_if_exists(&self.path)",
    ] {
        if !producer_source.contains(required) && !broadcast_mod_source.contains(required) {
            violations.push(format!(
                "src/broadcast/producer.rs: Situational ADR 0059 Packet 011 mpv drop-file producer missing `{required}`. Fix: keep mpv now-playing drop-file construction in broadcast::producer."
            ));
        }
    }

    if production_source.contains("value_block") {
        violations.push(
            "src/broadcast/producer.rs: Situational ADR 0059 Packet 011 forbids building value routes from NowPlayingUpdate.value_block. Fix: read embedded MusicIndex Value Routes from the audio file tag.".to_owned(),
        );
    }

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        if file == "src/broadcast/producer.rs" {
            continue;
        }
        let source = read_source(&path);
        let production_source = source.split("#[cfg(test)]").next().unwrap_or("");
        for (line_number, line) in code_lines(production_source) {
            for forbidden in [
                "DROP_FILE_SUFFIX",
                ".nowplaying.json",
                "musicindex.nowplaying/1",
            ] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number}: Situational ADR 0059 Packet 011 forbids mpv drop-file naming or schema construction outside src/broadcast/producer.rs. Fix: route mpv drop-file writes through broadcast::producer.",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 Packet 011 mpv drop-file producer boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Packet 015 encoder command construction stays in broadcast.
#[test]
fn adr_0059_stream_encoder_control_boundary_is_broadcast_owned() {
    let mut violations = Vec::new();
    let encoder_source = read_source(&manifest_path("src/broadcast/encoder.rs"));
    let broadcast_mod_source = read_source(&manifest_path("src/broadcast/mod.rs"));

    for required in [
        "pub mod encoder;",
        "pub struct EncoderTarget",
        "pub enum EncoderState",
        "Connected",
        "Connecting",
        "Disconnected",
        "NotInstalled",
        "NotReachable",
        "pub enum RecordingState",
        "pub enum AudioSignalState",
        "const DEFAULT_ENCODER_BINARY: &str = \"butt\"",
        "const STATUS_OPTION: &str = \"-S\"",
        "const CONNECT_OPTION: &str = \"-s\"",
        "const DISCONNECT_OPTION: &str = \"-d\"",
        "pub fn status(&self, target: &EncoderTarget)",
        "pub fn connect(&self, target: &EncoderTarget, server_name: Option<&str>)",
        "pub fn disconnect(&self, target: &EncoderTarget)",
    ] {
        if !encoder_source.contains(required) && !broadcast_mod_source.contains(required) {
            violations.push(format!(
                "src/broadcast/encoder.rs: Situational ADR 0059 Packet 015 encoder control missing `{required}`. Fix: keep stream encoder command construction and status parsing in broadcast::encoder."
            ));
        }
    }

    for (line_number, line) in code_lines(&encoder_source) {
        if line.contains("\"-u\"") {
            violations.push(format!(
                "src/broadcast/encoder.rs:{line_number}: Situational ADR 0059 Packet 015 forbids sending song titles to the encoder. Fix: remove the encoder `-u` option and let the producer own song text."
            ));
        }
    }

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        if file == "src/broadcast/encoder.rs" || file == "src/config.rs" {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for forbidden in [
                "Command::new(\"butt\")",
                "std::process::Command::new(\"butt\")",
                ".run(\"butt\"",
                "runner.run(\"butt\"",
                "program: \"butt\"",
            ] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number}: Situational ADR 0059 Packet 015 forbids encoder command construction outside src/broadcast/encoder.rs. Fix: route stream encoder commands through broadcast::encoder.",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 Packet 015 stream encoder boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Packet 014 publisher target commands stay in broadcast.
#[test]
fn adr_0059_publisher_target_command_boundary_is_broadcast_owned() {
    let mut violations = Vec::new();
    let target_source = read_source(&manifest_path("src/broadcast/publisher_targets.rs"));
    let broadcast_mod_source = read_source(&manifest_path("src/broadcast/mod.rs"));

    for required in [
        "pub mod publisher_targets;",
        "const PUBLISHER_BINARY: &str = \"musicindex-live-publisher\"",
        "pub struct PublisherTargetControl",
        "pub enum PublisherTargetCommandError",
        "CommandsUnavailable",
        "target list",
        "target add",
        "target remove",
        "--token-file",
        "ServiceControl::new(self.runner.clone())",
        ".restart(transport, &unit)",
    ] {
        if !target_source.contains(required) && !broadcast_mod_source.contains(required) {
            violations.push(format!(
                "src/broadcast/publisher_targets.rs: Situational ADR 0059 Packet 014 publisher target control missing `{required}`. Fix: keep target command construction and publisher restart in broadcast::publisher_targets."
            ));
        }
    }

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        if file.starts_with("src/broadcast/") {
            continue;
        }
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for forbidden in [
                "Command::new(\"musicindex-live-publisher\")",
                "std::process::Command::new(\"musicindex-live-publisher\")",
                ".run(\"musicindex-live-publisher\"",
                "runner.run(\"musicindex-live-publisher\"",
                "program: \"musicindex-live-publisher\"",
            ] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number}: Situational ADR 0059 Packet 014 forbids publisher target command construction outside src/broadcast. Fix: route target list/add/remove through broadcast::publisher_targets.",
                        rel_path(&path)
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 Packet 014 publisher target boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Packet 009 screens and shells do not run service tools.
#[test]
fn adr_0059_show_screen_and_shell_do_not_call_service_processes() {
    let mut violations = Vec::new();

    for root in ["src/app", "src/ui/shells"] {
        for path in rust_files_under(root) {
            let source = read_source(&path);
            for (line_number, line) in code_lines(&source) {
                for forbidden in ["systemctl", "journalctl"] {
                    if line.contains(forbidden) {
                        violations.push(format!(
                            "{}:{line_number}: Situational ADR 0059 Packet 009 forbids `{forbidden}` in screens and shells. Fix: route publisher service observation and logs through runtime actors and broadcast::control.",
                            rel_path(&path)
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 Packet 009 screen/shell service-process violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Packet 012 renderer path does not scan readiness files.
#[test]
fn adr_0059_broadcast_readiness_file_scan_stays_out_of_renderers() {
    let mut violations = Vec::new();

    for file in ["src/app/show.rs", "src/ui/shells/show.rs"] {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for forbidden in ["read_audio_tags", "MusicIndex Value Routes", ".is_file()"] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{file}:{line_number}: Situational ADR 0059 Packet 012 forbids readiness file scanning in the renderer path. Fix: keep payment-route tag reads in application queries or runtime actors."
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 Packet 012 readiness renderer violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: relay create responses must not print broadcaster
/// tokens.
///
/// Secret handling may deserve the durable set. ADR 0061 says adding to that set
/// is its own decision, and ADR 0059 has no such record, so this stays
/// situational until one exists.
#[test]
fn adr_0059_live_item_create_response_debug_redacts_broadcaster_token() {
    let source = read_source(&manifest_path("src/api.rs"));
    let response_start = source
        .find("pub struct LiveItemCreateResponse")
        .expect("LiveItemCreateResponse exists");
    let derive_region = source[..response_start]
        .lines()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .join("\n");
    let debug_impl = source_between(
        &source,
        "impl fmt::Debug for LiveItemCreateResponse",
        "#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct LiveMetadataSnapshot",
    );
    let mut violations = Vec::new();

    if derive_region.contains("Debug") {
        violations.push(
            "src/api.rs: ADR 0059 forbids derived Debug for LiveItemCreateResponse because it carries broadcaster_token"
                .to_owned(),
        );
    }

    for required in [
        ".debug_struct(\"LiveItemCreateResponse\")",
        ".field(\"event_id\", &self.event_id)",
        ".field(\"broadcaster_token\", &\"<redacted>\")",
        ".field(\"metadata_url\", &self.metadata_url)",
    ] {
        if !debug_impl.contains(required) {
            violations.push(format!(
                "src/api.rs: ADR 0059 redacted Debug implementation missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 LiveItemCreateResponse Debug violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: broadcaster token text stays at the API, registry, and
/// file boundary.
#[test]
fn adr_0059_broadcast_token_text_has_single_storage_boundary() {
    let mut violations = Vec::new();

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        let source = production_source(&source);

        for (line_number, line) in code_lines(source) {
            if (line.contains("write_token_file(") || line.contains("read_token_file("))
                && !matches!(
                    file.as_str(),
                    "src/broadcast/registry.rs" | "src/broadcast/tokens.rs"
                )
            {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0059 token file reads and writes belong only to the broadcast registry/token boundary: `{line}`"
                ));
            }

            if line.contains("broadcaster_token")
                && !matches!(file.as_str(), "src/api.rs" | "src/broadcast/registry.rs")
            {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0059 broadcaster token text escaped the API/registry boundary: `{line}`"
                ));
            }

            let mentions_token_text = line.contains("broadcaster_token")
                || line.contains("read_token_file(")
                || line.contains("write_token_file(");
            let prints_or_formats = [
                "println!",
                "eprintln!",
                "format!",
                "dbg!",
                ".context(",
                ".with_context(",
            ]
            .iter()
            .any(|pattern| line.contains(pattern));
            if mentions_token_text && prints_or_formats {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0059 forbids token text in output or log-adjacent formatting: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 broadcast token text boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: publisher configuration is changed only through
/// publisher commands.
#[test]
fn adr_0059_publisher_configuration_changes_use_publisher_tools() {
    let control_source = read_source(&manifest_path("src/broadcast/publisher_targets.rs"));
    let mut violations = Vec::new();

    for required in [
        "const PUBLISHER_BINARY: &str = \"musicindex-live-publisher\";",
        "\"target\".to_owned()",
        "\"add\".to_owned()",
        "\"remove\".to_owned()",
        "\"--config\".to_owned()",
        "\"--replace\".to_owned()",
        "ServiceControl::new(self.runner.clone())",
        ".restart(transport, &unit)",
    ] {
        if !control_source.contains(required) {
            violations.push(format!(
                "src/broadcast/publisher_targets.rs: ADR 0059 publisher target mutation must use publisher CLI tools and restart the unit; missing `{required}`"
            ));
        }
    }

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        if file == "src/broadcast/publisher_targets.rs" {
            continue;
        }

        let source = read_source(&path);
        let source = production_source(&source);
        for (line_number, line) in code_lines(source) {
            if line.contains("musicindex-live-publisher") && line.contains("config.toml") {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0059 publisher config path access belongs to broadcast::publisher_targets and the publisher CLI: `{line}`"
                ));
            }

            let writes_file = line.contains("fs::write")
                || line.contains("File::create")
                || line.contains("OpenOptions")
                || line.contains("write_all(");
            if writes_file && line.contains("musicindex-live-publisher") {
                violations.push(format!(
                    "{file}:{line_number}: ADR 0059 forbids direct writes to publisher-owned files: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 publisher configuration boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: now-playing source-kind names stay at adapter and
/// config boundaries.
#[test]
fn adr_0059_source_kind_literals_stay_at_adapter_boundaries() {
    let allowed_files = BTreeSet::from([
        "src/broadcast/producer.rs",
        "src/config.rs",
        "src/playback_driver/mod.rs",
        "src/playback_driver/mpv.rs",
    ]);
    let source_kinds = BTreeSet::from(["mpv", "mixxx", "external", "liquidsoap"]);
    let mut violations = Vec::new();

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        let source = production_source(&source);
        for literal in string_literals(source) {
            if source_kinds.contains(literal.as_str()) && !allowed_files.contains(file.as_str()) {
                violations.push(format!(
                    "{file}: ADR 0059 source kind literal `{literal}` belongs in a source adapter or config boundary, not shared display/control logic"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 source-kind literal boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Show broadcast sections stay separate
/// from the queue transport.
#[test]
fn adr_0059_show_broadcast_sections_stay_separate_from_queue_now_playing() {
    let show_source = read_source(&manifest_path("src/view_models/show.rs"));
    let queue_files = [
        "src/app/queue_now_playing.rs",
        "src/ui/shells/queue_now_playing.rs",
        "src/view_models/queue_now_playing.rs",
    ];
    let mut violations = Vec::new();

    for required in [
        "pub(crate) source: Option<SourceSectionDisplay>",
        "pub(crate) publisher: Option<PublisherSectionDisplay>",
        "pub(crate) event: Option<EventSectionDisplay>",
        "pub(crate) stream: Option<StreamSectionDisplay>",
        "pub(crate) queue: QueueNowPlayingPageVm",
        "ShowCardKind::Source",
        "ShowCardKind::LiveMetadata",
        "ShowCardKind::Stream",
    ] {
        if !show_source.contains(required) {
            violations.push(format!(
                "src/view_models/show.rs: ADR 0059 Show surface must keep broadcast sections and queue transport as separate fields; missing `{required}`"
            ));
        }
    }

    for file in queue_files {
        let source = read_source(&manifest_path(file));
        for forbidden in [
            "SourceSectionDisplay",
            "PublisherSectionDisplay",
            "EventSectionDisplay",
            "StreamSectionDisplay",
            "BroadcastObservation",
            "BroadcastReadiness",
            "PublisherSection",
            "EventSection",
            "StreamSection",
            "ShowCardKind",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{file}: ADR 0059 forbids broadcast section state inside QueueNowPlaying; found `{forbidden}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0059 Show and QueueNowPlaying separation violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Show broadcast shell uses semantic colors for theme parity.
#[test]
fn adr_0059_show_shell_uses_semantic_colors_for_dark_mode_parity() {
    let mut violations = Vec::new();

    for file in [
        "src/ui/shells/show.rs",
        "src/ui/composites/show_card.rs",
        "src/ui/composites/show_detail_panel.rs",
    ] {
        let source = read_source(&manifest_path(file));
        for required in ["SemanticColor", "color(cx,"] {
            if !source.contains(required) {
                violations.push(format!(
                    "{file}: Situational ADR 0059 Show shell theme parity requires semantic color tokens; missing `{required}`"
                ));
            }
        }

        for (line_number, line) in code_lines(&source) {
            for forbidden in [
                "rgb(",
                "rgba(",
                "hsla(",
                "Appearance::Dark",
                "ThemeProfile::Dark",
                "dark_mode",
            ] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{file}:{line_number}: Situational ADR 0059 forbids display-local dark-mode branching or raw colors in the Show broadcast shell: `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 dark-mode parity violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0059: Show shell controls consume view-model accessibility labels.
#[test]
fn adr_0059_show_shell_consumes_vm_owned_accessibility_labels() {
    let vm_source = read_source(&manifest_path("src/view_models/show.rs"));
    let shell_source = read_source(&manifest_path("src/ui/shells/show.rs"));
    let card_source = read_source(&manifest_path("src/ui/composites/show_card.rs"));
    let panel_source = read_source(&manifest_path("src/ui/composites/show_detail_panel.rs"));
    let mut violations = Vec::new();

    for (struct_name, start, end, field) in [
        (
            "ShowNowPlayingDisplay",
            "pub(crate) struct ShowNowPlayingDisplay",
            "/// Display-ready Source section",
            "pub(crate) a11y_label: String",
        ),
        (
            "SourceReadinessActionDisplay",
            "pub(crate) struct SourceReadinessActionDisplay",
            "impl SourceReadinessActionDisplay",
            "pub(crate) a11y_label: String",
        ),
        (
            "PublisherActionDisplay",
            "pub(crate) struct PublisherActionDisplay",
            "impl PublisherActionDisplay",
            "pub(crate) a11y_label: String",
        ),
        (
            "EventActionDisplay",
            "pub(crate) struct EventActionDisplay",
            "impl EventActionDisplay",
            "pub(crate) a11y_label: String",
        ),
        (
            "StreamActionDisplay",
            "pub(crate) struct StreamActionDisplay",
            "impl StreamActionDisplay",
            "pub(crate) a11y_label: String",
        ),
        (
            "ShowCardDisplay",
            "pub(crate) struct ShowCardDisplay",
            "/// Side-panel mode",
            "pub(crate) a11y_label: String",
        ),
        (
            "ShowPanelActionDisplay",
            "pub(crate) struct ShowPanelActionDisplay",
            "impl ShowPanelActionDisplay",
            "pub(crate) a11y_label: &'static str",
        ),
    ] {
        let struct_source = source_between(&vm_source, start, end);
        if !struct_source.contains(field) {
            violations.push(format!(
                "src/view_models/show.rs: Situational ADR 0059 {struct_name} must own `{field}` before it renders"
            ));
        }
    }

    for required in [
        "let label = SharedString::from(now_playing.a11y_label);",
        "let tooltip_label = SharedString::from(self.display.a11y_label.clone());",
        ".a11y_label(display.a11y_label)",
        ".tooltip(display.a11y_label)",
        ".a11y_label(display.a11y_label.clone())",
        ".tooltip(display.a11y_label)",
        ".a11y_label(a11y_label.clone())",
        ".tooltip(a11y_label)",
    ] {
        if !shell_source.contains(required)
            && !card_source.contains(required)
            && !panel_source.contains(required)
        {
            violations.push(format!(
                "Show UI: Situational ADR 0059 controls must consume VM-owned accessibility labels; missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0059 accessibility-label ownership violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: broadcasting is not a workspace frame.
#[test]
fn adr_0060_workspace_has_no_broadcast_frame_kind() {
    let mut violations = Vec::new();

    for file in [
        "src/view_models/broadcast.rs",
        "src/ui/shells/broadcast.rs",
        "src/app/broadcast.rs",
    ] {
        if manifest_path(file).exists() {
            violations.push(format!(
                "{file}: ADR 0060 supersedes the Broadcast workspace frame; delete this file"
            ));
        }
    }

    for (file, source) in [
        ("src/view_models/workspace", workspace_vm_source()),
        ("src/app.rs", read_source(&manifest_path("src/app.rs"))),
        (
            "src/app/search_dispatch.rs",
            read_source(&manifest_path("src/app/search_dispatch.rs")),
        ),
        (
            "src/ui/shells/workspace.rs",
            read_source(&manifest_path("src/ui/shells/workspace.rs")),
        ),
        (
            "src/ui/shells/mod.rs",
            read_source(&manifest_path("src/ui/shells/mod.rs")),
        ),
        (
            "src/view_models/mod.rs",
            read_source(&manifest_path("src/view_models/mod.rs")),
        ),
    ] {
        for forbidden in [
            "WorkspaceFrameKind::Broadcast",
            "FrameSearchScope::BroadcastRows",
            "FrameNavigationEntry::Broadcast",
            "WORKSPACE_BROADCAST_FRAME_ID",
            "build_broadcast_frame",
            "submit_broadcast_rows_search",
            "Filter broadcast",
            "broadcast: Option<AnyElement>",
            "pub(crate) fn broadcast(",
            "pub mod broadcast;",
            "pub(crate) mod broadcast;",
            "mod broadcast;",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{file}: ADR 0060 removes the Broadcast workspace frame; found `{forbidden}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 Broadcast frame removal violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: Show is a screen mount, not a workspace frame.
#[test]
fn adr_0060_show_is_screen_mount_not_frame_kind() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let toolbar_vm_source = read_source(&manifest_path("src/view_models/app_toolbar.rs"));
    let toolbar_source = read_source(&manifest_path("src/app/tab_bar.rs"));
    let keyboard_source = read_source(&manifest_path("src/app/keyboard.rs"));
    let workspace_source = workspace_vm_source();
    let mut violations = Vec::new();

    for required in [
        "AppTab::Show",
        "WorkspaceScreenMount::Show",
        "show_tab_focus: gpui::FocusHandle",
        "build_show_screen(self, show_window_width, cx).into_any_element()",
        "if matches!(mount, WorkspaceScreenMount::Show)",
        "self.queue_text_filter = None;",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 002 Show screen mount missing `{required}`"
            ));
        }
    }

    for required in [
        "AppToolbarTabKey::Show",
        "label: \"Show\"",
        "a11y_label: \"Open Show\"",
        "tabs: [AppToolbarTabDisplay; 3]",
    ] {
        if !toolbar_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/app_toolbar.rs: ADR 0060 task 002 toolbar tab missing `{required}`"
            ));
        }
    }

    for required in [
        "AppToolbarTabKey::Show => AppTab::Show",
        "AppToolbarTabKey::Show => &app.show_tab_focus",
    ] {
        if !toolbar_source.contains(required) {
            violations.push(format!(
                "src/app/tab_bar.rs: ADR 0060 task 002 Show tab renderer missing `{required}`"
            ));
        }
    }

    for required in [
        "SelectShowTab",
        "AppKeyCommand::SelectShowTab",
        "keystroke: \"cmd-2\"",
        "keystroke: \"cmd-3\"",
        "self.select_tab(AppTab::Show, window, cx)",
        "TopApp::handle_select_show_tab",
    ] {
        if !keyboard_source.contains(required) && !app_source.contains(required) {
            violations.push(format!(
                "src/app/keyboard.rs: ADR 0060 task 002 Show keyboard routing missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "WorkspaceFrameKind::Show",
        "FrameNavigationEntry::Show",
        "FrameSearchScope::Show",
    ] {
        if workspace_source.contains(forbidden) || app_source.contains(forbidden) {
            violations.push(format!(
                "ADR 0060 task 002 forbids modeling Show as a workspace frame; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 Show screen-mount violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: queue and transport render inside Show only.
#[test]
fn adr_0060_queue_is_not_mounted_in_curation_workspace() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let show_adapter_source = read_source(&manifest_path("src/app/show.rs"));
    let show_shell_source = read_source(&manifest_path("src/ui/shells/show.rs"));
    let workspace_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let layout_source = workspace_vm_source();
    let render_workspace_content = source_between(
        &app_source,
        "fn render_workspace_content(",
        "fn transitional_workspace_layout(",
    );
    let visible_workspace_layout = source_between(
        &app_source,
        "fn visible_workspace_layout(",
        "fn content_list_frame_title(",
    );
    let default_layout = source_between(
        &layout_source,
        "pub(crate) fn default_layout() -> Self",
        "pub(crate) fn empty() -> Self",
    );
    let mut violations = Vec::new();

    for forbidden in [
        ".queue_now_playing(",
        "build_queue_now_playing_frame",
        "queue_frame",
    ] {
        if render_workspace_content.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 002 curation workspace must not mount queue content; found `{forbidden}`"
            ));
        }
    }

    for forbidden in [
        "WorkspaceFrameState::with_default_title(\n                Self::QUEUE_NOW_PLAYING_ID",
        "WorkspaceFrameKind::QueueNowPlaying,",
        "QUEUE_NOW_PLAYING_ID",
    ] {
        if default_layout.contains(forbidden) {
            violations.push(format!(
                "src/view_models/workspace/mod.rs: ADR 0060 task 002 default layout must not include QueueNowPlaying; found `{forbidden}`"
            ));
        }
    }

    for required in [
        "WorkspaceFrameKind::QueueNowPlaying",
        "WorkspaceFrameKind::Detail",
        "WorkspaceFrameKind::SourceList => None",
    ] {
        if !visible_workspace_layout.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 002 visible curation layout must filter operational frames; missing `{required}`"
            ));
        }
    }

    for required in [
        "app.show_page.clone()",
        "ShowSlots::new()",
        "queue_transport_action(",
        "render_queue_transport(transport, self.slots.queue)",
        "ShowDetailPanel::new(",
    ] {
        if !show_adapter_source.contains(required) && !show_shell_source.contains(required) {
            violations.push(format!(
                "src/app/show.rs or src/ui/shells/show.rs: ADR 0060 task 002 Show must render queue and transport; missing `{required}`"
            ));
        }
    }

    if !workspace_source.contains("pub(crate) fn queue_now_playing(") {
        violations.push(
            "src/ui/shells/workspace.rs: ADR 0060 task 002 keeps legacy QueueNowPlaying slot until the frame kind is removed"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 queue relocation violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: Show VM and shell keep their layer boundaries.
#[test]
fn adr_0060_show_vm_and_shell_layer_boundaries() {
    let vm_source = read_source(&manifest_path("src/view_models/show.rs"));
    let shell_source = read_source(&manifest_path("src/ui/shells/show.rs"));
    let app_source = read_source(&manifest_path("src/app/show.rs"));
    let view_models_mod_source = read_source(&manifest_path("src/view_models/mod.rs"));
    let shells_mod_source = read_source(&manifest_path("src/ui/shells/mod.rs"));
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&vm_source) {
        for pattern in VIEW_MODEL_FORBIDDEN_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/view_models/show.rs:{line_number}: ADR 0060 task 002 Show VM must stay renderer-free; found `{pattern}` in `{line}`"
                ));
            }
        }
        for pattern in [
            "crate::db",
            "rusqlite",
            "PlaybackOwner",
            "ConfiguredPlaybackDriver",
        ] {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/view_models/show.rs:{line_number}: ADR 0060 task 002 Show VM must not read backend/playback handles; found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    for required in [
        "pub(crate) struct ShowPageVm",
        "pub(crate) struct ShowEmptyStateDisplay",
        "pub(crate) struct ShowNowPlayingDisplay",
        "QueueNowPlayingPageVm",
        "pub(crate) fn from_queue(",
        "pub(crate) const fn is_active(",
        "show-empty-state",
    ] {
        if !vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/show.rs: ADR 0060 task 002 Show VM contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "crate::app",
        "crate::library",
        "crate::db",
        "crate::playback",
        "crate::broadcast",
        "FrameShell",
        "FrameNavigation",
        "WorkspaceFrameKind",
        "Breadcrumb",
    ] {
        if shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/show.rs: ADR 0060 task 002 Show shell must not import screen/backend/frame chrome `{forbidden}`"
            ));
        }
    }

    for required in [
        "pub(crate) fn render_show(",
        "pub(crate) struct ShowSlots",
        "on_skip_previous(",
        "on_play_pause(",
        "on_skip_next(",
        "render_queue_transport(transport, self.slots.queue)",
        "ShowDetailPanel::new(",
    ] {
        if !shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/show.rs: ADR 0060 task 002 Show shell missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) mod show;",
        "pub mod show;",
        "pub(super) fn build_show_screen(",
    ] {
        if !view_models_mod_source.contains(required)
            && !shells_mod_source.contains(required)
            && !app_source.contains(required)
        {
            violations.push(format!(
                "ADR 0060 task 002 Show module export or adapter missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 Show layer-boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: Music is the curation surface and owns its filter.
#[test]
fn adr_0060_music_surface_vocabulary_and_primary_filter_are_guarded() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let toolbar_vm_source = read_source(&manifest_path("src/view_models/app_toolbar.rs"));
    let toolbar_source = read_source(&manifest_path("src/app/tab_bar.rs"));
    let keyboard_source = read_source(&manifest_path("src/app/keyboard.rs"));
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let workspace_chrome_source =
        read_source(&manifest_path("src/view_models/workspace/chrome.rs"));
    let db_source = read_source(&manifest_path("src/db.rs"));
    let cli_source = read_source(&manifest_path("src/cli.rs"));
    let render_workspace_content = source_between(
        &app_source,
        "fn render_workspace_content(",
        "fn transitional_workspace_layout(",
    );
    let set_frame_filter = source_between(
        &app_source,
        "fn set_frame_filter(",
        "fn set_search_results_filter(",
    );
    let mut violations = Vec::new();

    for required in [
        "AppTab::Music",
        "WorkspaceScreenMount::Music",
        "WorkspaceScreenMount::Music => self.library.clone().into_any_element()",
        "last_music_content_nav: Option<FrameNavigationState>",
        "music_tab_focus: gpui::FocusHandle",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 003 Music app-section vocabulary missing `{required}`"
            ));
        }
    }

    for required in [
        "AppToolbarTabKey::Music",
        "id: \"app-tab-music\"",
        "label: \"Music\"",
        "a11y_label: \"Open Music\"",
        "label: \"Library\"",
        "ContentFilter::Library",
        "label: \"Index\"",
    ] {
        if !toolbar_vm_source.contains(required) && !workspace_chrome_source.contains(required) {
            violations.push(format!(
                "ADR 0060 task 003 Music/Library/Index display vocabulary missing `{required}`"
            ));
        }
    }

    for required in [
        "AppToolbarTabKey::Music => AppTab::Music",
        "AppToolbarTabKey::Music => &app.music_tab_focus",
    ] {
        if !toolbar_source.contains(required) {
            violations.push(format!(
                "src/app/tab_bar.rs: ADR 0060 task 003 Music tab renderer missing `{required}`"
            ));
        }
    }

    for required in [
        "SelectMusicTab",
        "AppKeyCommand::SelectMusicTab",
        "keystroke: \"cmd-1\"",
        "label: \"Music\"",
        "self.select_tab(AppTab::Music, window, cx)",
        "TopApp::handle_select_music_tab",
    ] {
        if !keyboard_source.contains(required) && !app_source.contains(required) {
            violations.push(format!(
                "src/app/keyboard.rs: ADR 0060 task 003 Music keyboard routing missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "AppTab::Library",
        "WorkspaceScreenMount::Library",
        "AppToolbarTabKey::Library",
        "SelectLibraryTab",
        "handle_select_library_tab",
        "library_tab_focus",
        "last_library_content_nav",
        "id: \"app-tab-library\"",
        "a11y_label: \"Show Library\"",
    ] {
        if app_source.contains(forbidden)
            || toolbar_vm_source.contains(forbidden)
            || toolbar_source.contains(forbidden)
            || keyboard_source.contains(forbidden)
        {
            violations.push(format!(
                "ADR 0060 task 003 app-section internals must use Music, found `{forbidden}`"
            ));
        }
    }

    for required in [
        "if matches!(mount, WorkspaceScreenMount::Music)",
        ".content_list_library_filter_control(library_filter_control)",
        "this.set_frame_filter(content_frame_id, filter, cx)",
    ] {
        if !render_workspace_content.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 003 Music default state must expose the content filter; missing `{required}`"
            ));
        }
    }

    if !set_frame_filter.contains("matches!(mount, WorkspaceScreenMount::Music)") {
        violations.push(
            "src/app.rs: ADR 0060 task 003 Music filter selection must route from the default state"
                .to_string(),
        );
    }

    for required in [
        "LibraryFilterControlDisplay::default_for_content_list(self.filter_state)",
        "pub(crate) const fn state_displays()",
        "keyboard_cycle_order",
        "next_filter",
    ] {
        if !workspace_chrome_source.contains(required) && !library_vm_source.contains(required) {
            violations.push(format!(
                "Situational ADR 0062 library tri-state control guard missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) enum FilterChipStripWidthClass",
        "FilterChipStripWidthClass::Normal",
        "FilterChipStripWidthClass::Narrow",
        "filter_chip_strip_for_width_class",
        "FILTER_CHIP_STRIP_NARROW_COLLAPSE_BREAKPOINT",
        "filter_chip_strip_width_class(window.bounds().size.width)",
    ] {
        if !workspace_chrome_source.contains(required)
            && !library_vm_source.contains(required)
            && !app_source.contains(required)
        {
            violations.push(format!(
                "Situational ADR 0060 task 003 normal-width content-filter reachability guard missing `{required}`"
            ));
        }
    }
    if library_vm_source.contains("FilterChipStripDisplay::default_for_content_list") {
        violations.push(
            "src/view_models/library.rs: Situational ADR 0062 library tri-state control guard forbids content-list chip-strip projection"
                .to_string(),
        );
    }

    for forbidden in [
        "has_filterable_content_detail",
        "FrameNavigationEntry::SourceList",
    ] {
        if set_frame_filter.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 003 Music filter selection must not depend on prior detail/navigation state; found `{forbidden}`"
            ));
        }
    }

    if !db_source.contains("is_in_library") {
        violations.push(
            "src/db.rs: ADR 0060 task 003 must not rename the `is_in_library` database column"
                .to_string(),
        );
    }
    if !cli_source.contains("v4vmm library tracks --json") {
        violations.push(
            "src/cli.rs: ADR 0060 task 003 must not rename the `v4vmm library tracks` CLI contract"
                .to_string(),
        );
    }
    for required_path in ["src/library.rs", "src/library/app_impl.rs"] {
        if !manifest_path(required_path).is_file() {
            violations.push(format!(
                "{required_path}: ADR 0060 task 003 must not rename library modules"
            ));
        }
    }

    for file in display_surface_files() {
        let source = read_source(&manifest_path(&file));
        for literal in string_literals(&source) {
            if literal.to_ascii_lowercase().contains("collection") {
                violations.push(format!(
                    "{file}: ADR 0060 task 003 forbids user-facing `collection` display text: `{literal}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 Music vocabulary/filter violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: Music shows curation content, not operational panes.
#[test]
fn adr_0060_music_surface_is_dominant_content_without_operational_panes() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let render_workspace_content = source_between(
        &app_source,
        "fn render_workspace_content(",
        "fn transitional_workspace_layout(",
    );
    let visible_workspace_layout = source_between(
        &app_source,
        "fn visible_workspace_layout(",
        "fn content_list_frame_title(",
    );
    let library_render = source_between(
        &library_app_source,
        "impl Render for LibraryApp",
        "#[cfg(test)]",
    );
    let content_marker = "let content = if matches!(self.detail, LibraryDetail::None)";
    let mut violations = Vec::new();

    for required in [
        "WorkspaceFrameKind::QueueNowPlaying",
        "WorkspaceFrameKind::Detail",
        "WorkspaceFrameKind::SourceList => None",
    ] {
        if !visible_workspace_layout.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 003 Music visible layout must filter non-content panes; missing `{required}`"
            ));
        }
    }

    for forbidden in [
        ".queue_now_playing(",
        "build_queue_now_playing_frame",
        "queue_transport_action(",
        "build_show_screen(self, cx)",
    ] {
        if render_workspace_content.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 003 Music workspace content must not mount operational Show controls; found `{forbidden}`"
            ));
        }
    }

    let Some(content_index) = library_render.find(content_marker) else {
        violations.push(format!(
            "src/library/app_impl.rs: ADR 0060 task 003 no-selection Music body missing `{content_marker}`"
        ));
        assert!(
            violations.is_empty(),
            "ADR 0060 Music dominant-content violations:\n{}",
            violations.join("\n")
        );
        return;
    };
    let before_content_branch = &library_render[..content_index];
    let content_branch = &library_render[content_index..];
    if before_content_branch.contains("render_library_detail(") {
        violations.push(
            "src/library/app_impl.rs: ADR 0060 task 003 must not build the empty detail pane before the no-selection branch"
                .to_string(),
        );
    }
    for required in [
        "leading_pane",
        "SplitPane::new(chrome.split_pane_id)",
        "render_library_detail(",
    ] {
        if !content_branch.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: ADR 0060 task 003 selected detail must still open inside the Music surface; missing `{required}`"
            ));
        }
    }

    for (file, source) in [
        (
            "src/library.rs",
            read_source(&manifest_path("src/library.rs")),
        ),
        ("src/library/app_impl.rs render", library_render.to_string()),
    ] {
        for forbidden in [
            "QueueNowPlaying",
            "render_queue_cuelist",
            "render_queue_transport",
            "TransportDisplay",
            "BroadcastObservation",
            "BroadcastStatus",
            "broadcast_status",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{file}: ADR 0060 task 003 curation surface must not carry operational pane/status `{forbidden}`"
                ));
            }
        }
    }
    for path in rust_files_under("src/ui/shells/library") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for forbidden in [
            "QueueNowPlaying",
            "render_queue_cuelist",
            "render_queue_transport",
            "TransportDisplay",
            "BroadcastObservation",
            "BroadcastStatus",
            "broadcast_status",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{file}: ADR 0060 task 003 curation shell must not render operational pane/status `{forbidden}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 Music dominant-content violations:\n{}",
        violations.join("\n")
    );
}

/// Durable ADR 0061: element hierarchy omits redundant root section headers.
#[test]
fn adr_0061_root_content_frame_header_omits_redundant_section_title() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let workspace_vm_source = read_source(&manifest_path("src/view_models/workspace/chrome.rs"));
    let frame_shell_source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let content_list_frame_title = source_between(
        &app_source,
        "fn content_list_frame_title(",
        "fn unused_workspace_frame_id(",
    );
    let mut violations = Vec::new();

    for required in [
        "fn root_content_frame_title() -> String",
        "String::new()",
        "FrameNavigationEntry::SourceList | FrameNavigationEntry::Settings",
    ] {
        if !content_list_frame_title.contains(required) && !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: Durable ADR 0061 element hierarchy guard missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "FrameNavigationEntry::SourceList => mount.frame_title().to_string()",
        "FrameNavigationEntry::Settings => \"Settings\".to_string()",
    ] {
        if content_list_frame_title.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: Durable ADR 0061 element hierarchy guard forbids redundant section frame title `{forbidden}`"
            ));
        }
    }

    for required in [
        "pub(crate) fn header_visible(&self) -> bool",
        "!self.title.is_empty()",
        "|| !self.back.disabled",
    ] {
        if !workspace_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/chrome.rs: Durable ADR 0061 element hierarchy guard missing `{required}`"
            ));
        }
    }

    for required in [
        "let header_visible = display.header_visible();",
        "if header_visible {",
    ] {
        if !frame_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: Durable ADR 0061 element hierarchy guard missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Durable ADR 0061 root content frame header violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: curation labels are VM-owned display facts.
#[test]
fn adr_0060_music_row_state_labels_are_vm_owned() {
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let feed_detail_source = read_source(&manifest_path("src/ui/shells/library/feed_detail.rs"));
    let mut violations = Vec::new();

    for required in [
        "const UPDATE_AVAILABLE_LABEL: &str = \"Update available\"",
        "const NEW_RELEASE_LABEL: &str = \"New\"",
        "pub(crate) state_label: Option<&'static str>",
        "state_label: has_stale.then_some(UPDATE_AVAILABLE_LABEL)",
        "state_label: row_state_label_for_source(source)",
        "state_label: (!self.track.is_in_library).then_some(NEW_RELEASE_LABEL)",
    ] {
        if !library_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: ADR 0060 task 003 row state label contract missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "feed update available",
        "feed update pending",
        "\"Update available\"",
    ] {
        if library_app_source.contains(forbidden) || feed_detail_source.contains(forbidden) {
            violations.push(format!(
                "ADR 0060 task 003 renderers must consume VM-owned update labels, found `{forbidden}`"
            ));
        }
    }
    if feed_detail_source.contains("\"New\"") {
        violations.push(
            "src/ui/shells/library/feed_detail.rs: ADR 0060 task 003 renderer must consume VM-owned `New` row labels"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 Music row-label ownership violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: unacted output controls stay deleted.
#[test]
fn adr_0060_queue_has_no_volume_or_output_picker_display() {
    let mut violations = Vec::new();

    for file in [
        "src/view_models/queue_now_playing.rs",
        "src/ui/shells/queue_now_playing.rs",
        "src/app/queue_now_playing.rs",
    ] {
        let source = read_source(&manifest_path(file));
        for forbidden in [
            "LiveValueDeviceDisplay",
            "LiveValueDeviceOption",
            "VolumeDisplay",
            "queue-livevalue-output",
            "livevalue-output-unavailable",
            "No liveValue output",
            "queue-output-volume",
            "Output volume",
            "render_output_picker",
            "render_volume",
            ".live_value(",
            ".volume(",
            "Slider::new(&state)",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{file}: ADR 0060 task 003 removes unacted output controls; found `{forbidden}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 queue output-control deletion violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: live status is a passive strip above Music/Settings.
#[test]
fn adr_0060_live_status_strip_contract_and_mount_are_guarded() {
    let vm_source = read_source(&manifest_path("src/view_models/live_status.rs"));
    let vm_mod_source = read_source(&manifest_path("src/view_models/mod.rs"));
    let composite_source = read_source(&manifest_path("src/ui/composites/live_status_strip.rs"));
    let composite_mod_source = read_source(&manifest_path("src/ui/composites/mod.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let show_adapter_source = read_source(&manifest_path("src/app/show.rs"));
    let app_render = source_between(
        &app_source,
        "impl Render for TopApp",
        "fn render_ui_scale_picker",
    );
    let build_live_status_strip = source_between(
        &show_adapter_source,
        "pub(super) fn build_live_status_strip(",
        "impl TopApp",
    );
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&vm_source) {
        for pattern in VIEW_MODEL_FORBIDDEN_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/view_models/live_status.rs:{line_number}: ADR 0060 task 004 LiveStatus VM must stay renderer-free; found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    for required in [
        "pub(crate) struct LiveStatusDisplay",
        "pub(crate) active: bool",
        "pub(crate) heading_label: &'static str",
        "pub(crate) summary_label: String",
        "pub(crate) struct LiveStatusNowPlayingDisplay",
        "pub(crate) enum LiveStatusHealthState",
        "pub(crate) enum LiveStatusHealthIconRole",
        "pub(crate) struct LiveStatusHealthDisplay",
        "pub(crate) struct LiveStatusRecordingDisplay",
        "pub(crate) struct LiveStatusOpenShowActionDisplay",
        "pub(crate) struct LiveStatusProjection",
        "pub(crate) fn from_show_page(show: &ShowPageVm) -> Self",
        "Health unknown",
        "Recording",
        "Open Show",
    ] {
        if !vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/live_status.rs: ADR 0060 task 004 display contract missing `{required}`"
            ));
        }
    }

    if !vm_mod_source.contains("pub(crate) mod live_status;") {
        violations.push(
            "src/view_models/mod.rs: ADR 0060 task 004 live_status VM module is not exported"
                .to_string(),
        );
    }

    for required in [
        "pub(crate) fn live_status_strip(",
        "pub(crate) struct LiveStatusStripSlots",
        "pub(crate) fn on_open_show(",
        "Icon::new(icon).size(IconSize::Action)",
        "SharedString::from(health.label)",
        "let now_playing_state_label = now_playing.map(|value| value.state_label);",
        "SharedString::from(state_label)",
        "recording.summary_label",
        "open_show.label",
    ] {
        if !composite_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/live_status_strip.rs: ADR 0060 task 004 strip composite missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "crate::db",
        "PlaybackOwner",
        "PausePlayback",
        "ResumePlayback",
        "SkipPlayback",
        "StopPlayback",
        "queue_transport_action",
        "present_command(",
        "ApplicationCommand",
        "BroadcastObservation",
        "value.artist",
        "when_some(artist",
    ] {
        if composite_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/live_status_strip.rs: ADR 0060 task 004 strip is a glance surface; found command/state source `{forbidden}`"
            ));
        }
    }

    for required in [
        "pub(crate) use live_status_strip::{",
        "live_status_strip",
        "LiveStatusStrip",
        "LiveStatusStripSlots",
    ] {
        if !composite_mod_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/mod.rs: ADR 0060 task 004 live status strip export missing `{required}`"
            ));
        }
    }

    for required in [
        "let live_status_strip = build_live_status_strip(self, mount, cx)",
        ".when_some(live_status_strip, gpui::ParentElement::child)",
        "WorkspaceScreenMount::Music | WorkspaceScreenMount::Settings",
        "LiveStatusDisplay::from_show_page(&app.show_page)",
        "this.select_tab(AppTab::Show, window, cx);",
    ] {
        if !app_render.contains(required) && !show_adapter_source.contains(required) {
            violations.push(format!(
                "src/app.rs or src/app/show.rs: ADR 0060 task 004 live strip mount/action missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "skip_playback",
        "toggle_playback",
        "PausePlayback",
        "ResumePlayback",
        "SkipPlayback",
        "StopPlayback",
        "queue_transport_action",
        "present_command(",
    ] {
        if build_live_status_strip.contains(forbidden) {
            violations.push(format!(
                "src/app/show.rs: ADR 0060 task 004 live strip may only open Show; found `{forbidden}`"
            ));
        }
    }

    for forbidden in [
        "build_playback_bar",
        "playback_bar_state",
        "queue_now_playing_vm(",
        "playback_snapshot(",
        "db::playback_session(",
        ".lock().expect(\"lock db\")",
    ] {
        if app_render.contains(forbidden) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 004 render path must not read show state; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 live status strip violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: Show and the strip share one cached projector.
#[test]
fn adr_0060_live_status_and_show_share_cached_projection() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let show_adapter_source = read_source(&manifest_path("src/app/show.rs"));
    let queue_adapter_source = read_source(&manifest_path("src/app/queue_now_playing.rs"));
    let playback_source = read_source(&manifest_path("src/app/playback_bar.rs"));
    let events_source = read_source(&manifest_path("src/app/events.rs"));
    let bootstrap_source = read_source(&manifest_path("src/app/bootstrap.rs"));
    let mut violations = Vec::new();

    for required in [
        "show_page: ShowPageVm",
        // ADR 0066 replaces assumed idle state when the first query cannot run.
        // ADR 0060's single cached projector and invalidation rules still apply.
        "ShowPageVm::idle().with_execution_availability(command_runner.availability())",
        "PlaybackTickOutcome::Advanced =>",
        "self.refresh_show_page(cx)",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: ADR 0060 task 004 cached Show projection missing `{required}`"
            ));
        }
    }

    for required in [
        "app.show_page.clone()",
        "pub(super) fn refresh_show_page(&self, cx: &mut Context<Self>)",
        "struct RefreshShowPage",
        "this.reproject_show_page(projection.queue)",
        "queue_now_playing_vm(",
    ] {
        if !show_adapter_source.contains(required) {
            violations.push(format!(
                "src/app/show.rs: ADR 0060 task 004 Show/strip shared projection missing `{required}`"
            ));
        }
    }
    if !show_adapter_source.contains("ShowPageVm::from_queue_and_publisher(")
        && !show_adapter_source.contains("ShowPageVm::from_queue_publisher_and_readiness(")
        && !show_adapter_source.contains("ShowPageVm::from_queue_publisher_readiness_and_event(")
    {
        violations.push(
            "src/app/show.rs: ADR 0060 task 004 Show/strip shared projection missing ShowPageVm constructor"
                .to_string(),
        );
    }

    for required in [
        "services: &ApplicationServices",
        "text_filter: Option<String>",
        "queue_tracks_for_session(services, conn, session)",
    ] {
        if !queue_adapter_source.contains(required) {
            violations.push(format!(
                "src/app/queue_now_playing.rs: ADR 0060 task 004 queue projection must be callable from a background refresh; missing `{required}`"
            ));
        }
    }

    if !playback_source.contains("this.refresh_show_page(cx)") {
        violations.push(
            "src/app/playback_bar.rs: ADR 0060 task 004 playback commands must refresh the cached Show projection"
                .to_string(),
        );
    }
    if !events_source.contains("fn affects_show_surface(event: &ApplicationEvent) -> bool")
        || !events_source.contains("ApplicationEvent::Playback(_)")
    {
        violations.push(
            "src/app/events.rs: ADR 0060 task 004 playback events must invalidate Show/live status"
                .to_string(),
        );
    }
    if !bootstrap_source.contains("app.refresh_show_page(cx);") {
        violations.push(
            "src/app/bootstrap.rs: ADR 0060 task 004 startup must refresh the cached Show projection"
                .to_string(),
        );
    }

    for forbidden in [
        "ShowPageVm::from_queue(queue_now_playing_vm(app))",
        "build_playback_bar(self)",
        "playback_bar_state()",
    ] {
        if app_source.contains(forbidden)
            || show_adapter_source.contains(forbidden)
            || playback_source.contains(forbidden)
        {
            violations.push(format!(
                "ADR 0060 task 004 forbids render-path Show/toolbar state reads; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 cached live-status projection violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0060: now-playing leaves the toolbar VM and renderer.
#[test]
fn adr_0060_toolbar_no_longer_carries_now_playing_chip() {
    let toolbar_vm_source = read_source(&manifest_path("src/view_models/app_toolbar.rs"));
    let toolbar_source = read_source(&manifest_path("src/app/tab_bar.rs"));
    let layout_source = read_source(&manifest_path("src/ui/layouts.rs"));
    let playback_source = read_source(&manifest_path("src/app/playback_bar.rs"));
    let mut violations = Vec::new();

    for forbidden in [
        "NowPlayingFrameDisplay",
        "now_playing: NowPlayingFrameDisplay",
        "display.now_playing",
        "app-toolbar-now-playing",
        "APP_TOOLBAR_NOW_PLAYING_COMPACT_BREAKPOINT",
        "build_playback_bar",
        "NowPlayingBar",
        "NowPlayingData",
        "\"Nothing playing\"",
    ] {
        if toolbar_vm_source.contains(forbidden)
            || toolbar_source.contains(forbidden)
            || layout_source.contains(forbidden)
            || playback_source.contains(forbidden)
        {
            violations.push(format!(
                "ADR 0060 task 004 replaces the toolbar now-playing chip with the live status strip; found `{forbidden}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 toolbar now-playing removal violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0063: column text does not call `truncate()`.
///
/// `truncate()` on text stacked in a flex column renders `...` and drops the
/// text. This shipped on 2026-09-08 and made every card summary line and every
/// panel value unreadable. Neither `w_full()` nor moving `min_w_0` to the parent
/// repaired it.
///
/// A row item is different. `src/ui/shells/queue_now_playing.rs` truncates
/// correctly, and `show_detail_panel.rs` truncates its header title, because
/// both sit in a flex row that gives the element a width.
///
/// Use `overflow_hidden()` for column text. The text clips and stays readable.
#[test]
fn adr_0063_column_text_does_not_truncate() {
    let mut violations = Vec::new();

    for relative in [
        "src/ui/composites/show_card.rs",
        "src/ui/composites/show_detail_panel.rs",
        "src/ui/composites/show_log_pane.rs",
    ] {
        let source = read_source(&manifest_path(relative));
        let lines: Vec<&str> = source.lines().collect();

        for (index, line) in lines.iter().enumerate() {
            if !line.contains(".truncate()") {
                continue;
            }

            let mut start = index;
            while start > 0 && !lines[start].contains("div()") {
                start -= 1;
            }

            // Two shapes truncate correctly, because both give the element a
            // definite width: a row item that flexes, and an element with an
            // explicit width bound.
            let chain = lines[start..=index].join("\n");
            if chain.contains("flex_1()") || chain.contains("max_w(") || chain.contains(".w(") {
                continue;
            }

            violations.push(format!(
                "{relative}:{}: Situational ADR 0063 forbids `truncate()` on column text. Fix: use `overflow_hidden()`, or the element renders `...` and drops the text.",
                index + 1
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0063 column truncation violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0063: Show dashboard card contract stays renderer-free.
#[test]
fn adr_0063_show_card_contract_is_renderer_free_and_column_only() {
    let show_source = read_source(&manifest_path("src/view_models/show.rs"));
    let card_struct = source_between(
        &show_source,
        "pub(crate) struct ShowCardDisplay",
        "/// Side-panel mode for the Show screen.",
    );
    let page_struct = source_between(
        &show_source,
        "pub(crate) struct ShowPageVm",
        "impl ShowPageVm",
    );
    let mut violations = Vec::new();

    for required in [
        "pub(crate) enum ShowCardKind",
        "Source",
        "LiveMetadata",
        "Stream",
        "const ORDER: [Self; 3]",
        "pub(crate) enum ShowCardStateKind",
        "Ok",
        "Attention",
        "Failed",
        "Unknown",
        "pub(crate) enum ShowPanelMode",
        "Cuelist",
        "Detail(ShowCardKind)",
        "pub(crate) enum ShowWidthClass",
        "Compact",
        "Medium",
        "Wide",
        "const SHOW_GRID_MEDIUM_MIN: f32 = 712.0;",
        "const SHOW_GRID_WIDE_MIN: f32 = 1_056.0;",
        "pub(crate) fn for_window_width(window_width: f32) -> Self",
        "pub(crate) const fn columns(self) -> u16",
        "pub(crate) fn with_window_width(mut self, window_width: f32) -> Self",
        "Self::Compact => 1",
        "Self::Medium => 2",
        "Self::Wide => 3",
    ] {
        if !show_source.contains(required) {
            violations.push(format!(
                "src/view_models/show.rs: Situational ADR 0063 Show card contract missing `{required}`. Fix: keep card identity, state, panel mode, and column count in the view model."
            ));
        }
    }

    for required in [
        "pub(crate) kind: ShowCardKind",
        "pub(crate) title: &'static str",
        "pub(crate) state_label: String",
        "pub(crate) state: ShowCardStateKind",
        "pub(crate) primary: String",
        "pub(crate) secondary: String",
        "pub(crate) a11y_label: String",
    ] {
        if !card_struct.contains(required) {
            violations.push(format!(
                "src/view_models/show.rs: Situational ADR 0063 card summary shape missing `{required}`. Fix: every card must carry the same display-ready field set."
            ));
        }
    }

    for required in [
        "pub(crate) cards: Vec<ShowCardDisplay>",
        "pub(crate) width_class: ShowWidthClass",
        "pub(crate) panel_mode: ShowPanelMode",
        "pub(crate) panel_open: bool",
    ] {
        if !page_struct.contains(required) {
            violations.push(format!(
                "src/view_models/show.rs: Situational ADR 0063 ShowPageVm missing `{required}`. Fix: keep dashboard card and panel state in the page VM."
            ));
        }
    }

    for (line_number, line) in code_lines(&show_source) {
        for pattern in [
            "use gpui",
            "gpui::",
            "use gpui_component",
            "gpui_component::",
            "px(",
            ".px()",
            "Pixels",
            "window.bounds",
            "size.width",
        ] {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/view_models/show.rs:{line_number}: Situational ADR 0063 Show card contract must stay renderer-free and map plain width input, found `{pattern}` in `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0063 Show card contract violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0063: Show card grid reads the VM contract only.
#[test]
fn adr_0063_show_card_grid_shell_uses_vm_contract() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let show_adapter_source = read_source(&manifest_path("src/app/show.rs"));
    let shell_source = read_source(&manifest_path("src/ui/shells/show.rs"));
    let card_source = read_source(&manifest_path("src/ui/composites/show_card.rs"));
    let composites_mod_source = read_source(&manifest_path("src/ui/composites/mod.rs"));
    let render_show_body = source_between(
        &shell_source,
        "impl RenderOnce for ShowShell",
        "fn render_show_summary(",
    );
    let card_grid_source = source_between(
        &shell_source,
        "fn render_show_card_grid(",
        "fn show_card_selected(",
    );
    let compact_card_grid = compact_source(card_grid_source);
    let mut violations = Vec::new();

    for required in [
        "let show_window_width = f32::from(window.bounds().size.width);",
        "build_show_screen(self, show_window_width, cx).into_any_element()",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: Situational ADR 0063 task 002 window-width observation missing `{required}`. Fix: observe width in the app layer and pass it to Show."
            ));
        }
    }

    for required in [
        "window_width: f32",
        "app.show_page.clone().with_window_width(window_width)",
        ".on_select_card(",
    ] {
        if !show_adapter_source.contains(required) {
            violations.push(format!(
                "src/app/show.rs: Situational ADR 0063 task 002 Show adapter missing `{required}`. Fix: pass observed width to the VM and expose card selection through ShowSlots."
            ));
        }
    }

    for required in ["pub mod show_card;", "pub(crate) use show_card::ShowCard;"] {
        if !composites_mod_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/mod.rs: Situational ADR 0063 task 002 show-card composite export missing `{required}`"
            ));
        }
    }

    for required in [
        "ShowCard, ShowDetailPanel, ShowDetailPanelDisplay, ShowDetailPanelSlots,",
        "card: ShowCardSlots",
        "type ShowCardClickHandler",
        "pub(crate) fn on_select_card(",
        "cards,",
        "width_class,",
        "panel_mode,",
        "panel_open,",
        "render_show_card_grid(",
        "ShowCard::new(card).selected(selected)",
    ] {
        if !shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/show.rs: Situational ADR 0063 task 002 card-grid shell missing `{required}`"
            ));
        }
    }

    if render_show_body.contains("..") {
        violations.push(
            "src/ui/shells/show.rs: Situational ADR 0063 task 002 render_show must destructure ShowPageVm exhaustively, without `..`"
                .to_string(),
        );
    }

    if !compact_card_grid.contains(".grid().grid_cols(width_class.columns())") {
        violations.push(
            "src/ui/shells/show.rs: Situational ADR 0063 task 002 card grid must read `width_class.columns()`"
                .to_string(),
        );
    }

    for forbidden in [
        "fn render_source_section(",
        "fn render_publisher_section(",
        "fn render_event_section(",
        "fn render_stream_section(",
        "fn render_source_readiness_row(",
        "fn render_publisher_service(",
        "fn render_event_identity_row(",
        "fn render_event_target_row(",
        "fn render_stream_status_row(",
        "fn render_stream_recording_row(",
        "fn render_publisher_log_panel(",
        "source_reachability_color",
        "event_target_icon",
        "stream_icon_name",
    ] {
        if shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/show.rs: Situational ADR 0063 task 002 deleted section renderer/helper still present `{forbidden}`"
            ));
        }
    }

    for (file, source) in [
        ("src/ui/shells/show.rs", shell_source.as_str()),
        ("src/ui/composites/show_card.rs", card_source.as_str()),
    ] {
        for (line_number, line) in code_lines(source) {
            for forbidden in [
                "BREAKPOINT",
                "SHOW_GRID_",
                "window.bounds",
                "size.width",
                "f32::from",
                "ScrollableElement",
                "overflow_y_scrollbar",
                "overflow_scrollbar",
            ] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{file}:{line_number}: Situational ADR 0063 task 002 forbids breakpoint/window-width and scroll ownership in the shell/card region; found `{forbidden}` in `{line}`"
                    ));
                }
            }

            if line.contains(".grid_cols(") && !line.contains("width_class.columns()") {
                violations.push(format!(
                    "{file}:{line_number}: Situational ADR 0063 task 002 grid columns must come from `width_class.columns()`, not a literal: `{line}`"
                ));
            }
        }
    }

    for forbidden in [
        "\"Active\"",
        "\"Needs attention\"",
        "\"Failed\"",
        "\"Unknown\"",
        "\"Reachable\"",
        "\"Connected\"",
        "\"Disconnected\"",
        "\"Live\"",
        "\"Dead\"",
        "\"No event\"",
    ] {
        if card_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/show_card.rs: Situational ADR 0063 task 002 card composite must not map state to display text; found `{forbidden}`"
            ));
        }
    }

    for required in [
        "display: ShowCardDisplay",
        "selected: bool",
        "status_badge::render_state_badge",
        "on_select: Option<ShowCardClickHandler>",
        ".h(Size::MenuCompact.scaled(cx))",
    ] {
        if !card_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/show_card.rs: Situational ADR 0063 task 002 show-card composite missing `{required}`"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0063 Show card-grid shell violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0063: Show detail panel owns detail; transport stays outside.
#[test]
fn adr_0063_show_detail_panel_owns_detail_and_transport_stays_on_show() {
    let vm_source = read_source(&manifest_path("src/view_models/show.rs"));
    let app_source = read_source(&manifest_path("src/app/show.rs"));
    let shell_source = read_source(&manifest_path("src/ui/shells/show.rs"));
    let queue_shell_source = read_source(&manifest_path("src/ui/shells/queue_now_playing.rs"));
    let panel_source = read_source(&manifest_path("src/ui/composites/show_detail_panel.rs"));
    let composites_mod_source = read_source(&manifest_path("src/ui/composites/mod.rs"));
    let card_grid_source = source_between(
        &shell_source,
        "fn render_show_card_grid(",
        "fn show_card_selected(",
    );
    let mut violations = Vec::new();

    for required in [
        "pub(crate) struct ShowPanelChromeDisplay",
        "pub(crate) panel_chrome: ShowPanelChromeDisplay",
        "pub(crate) fn with_panel_state(",
        "pub(crate) fn show_card_detail(mut self, kind: ShowCardKind) -> Self",
        "self.set_panel_mode(ShowPanelMode::Detail(kind));",
        "self.panel_open = true;",
        "pub(crate) fn show_cuelist_panel(mut self) -> Self",
        "self.set_panel_mode(ShowPanelMode::Cuelist);",
        "pub(crate) fn close_panel(mut self) -> Self",
        "pub(crate) fn open_panel(mut self) -> Self",
    ] {
        if !vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/show.rs: Situational ADR 0063 task 003 panel state contract missing `{required}`"
            ));
        }
    }

    for required in [
        ".with_panel_state(panel_mode, panel_open)",
        ".on_select_card(move |kind, _, _, cx|",
        "this.select_show_card_detail(kind, cx);",
        ".on_open_show_panel(",
        ".on_close_show_panel(",
        ".on_show_cuelist(",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app/show.rs: Situational ADR 0063 task 003 app panel wiring missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) fn render_queue_cuelist(",
        "pub(crate) fn render_queue_transport(",
    ] {
        if !queue_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/queue_now_playing.rs: Situational ADR 0063 task 003 queue renderer split missing `{required}`"
            ));
        }
    }

    for required in [
        "pub mod show_detail_panel;",
        "pub(crate) use show_detail_panel::{ShowDetailPanel, ShowDetailPanelDisplay, ShowDetailPanelSlots};",
    ] {
        if !composites_mod_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/mod.rs: Situational ADR 0063 task 003 detail-panel composite export missing `{required}`"
            ));
        }
    }

    for required in [
        "ShowDetailPanel::new(",
        "ShowDetailPanelDisplay {",
        "panel_chrome,",
        "render_queue_transport(transport, self.slots.queue)",
    ] {
        if !shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/show.rs: Situational ADR 0063 task 003 Show shell panel/transport composition missing `{required}`"
            ));
        }
    }

    for required in [
        "pub(crate) struct ShowDetailPanel",
        "pub(crate) struct ShowDetailPanelDisplay",
        "pub(crate) struct ShowDetailPanelSlots",
        ".w(Size::ColumnRegular.scaled(cx))",
        "render_queue_cuelist(queue)",
        "ShowPanelMode::Cuelist =>",
        "ShowPanelMode::Detail(kind) =>",
        "fn render_source_detail(",
        "fn render_publisher_detail(",
        "fn render_event_detail(",
        "fn render_stream_detail(",
        ".overflow_y_scroll()",
    ] {
        if !panel_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/show_detail_panel.rs: Situational ADR 0063 task 003 detail-panel composite missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "render_queue_transport",
        "render_control_deck",
        "render_transport(",
    ] {
        if panel_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/show_detail_panel.rs: Situational ADR 0063 task 003 transport must stay outside the panel composite; found `{forbidden}`"
            ));
        }
    }

    // The closed rail was absolute and overlaid the last card. A panel in either
    // state is a layout child, so the grid shrinks beside it.
    for forbidden in [".absolute()", ".right_0()", ".left_0()"] {
        if panel_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/show_detail_panel.rs: Situational ADR 0063 task 003 forbids `{forbidden}`. Fix: the panel is a layout child in both states, or it clips the card beneath it."
            ));
        }
    }

    for forbidden in [
        "PublisherLogPanelState",
        "fn render_publisher_log_panel(",
        "show-publisher-log-strip",
    ] {
        if shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/show.rs: Situational ADR 0063 task 003 inline publisher log state/strip must not live in the Show shell; found `{forbidden}`"
            ));
        }
    }

    for (line_number, line) in code_lines(card_grid_source) {
        for forbidden in [
            "ScrollableElement",
            "overflow_y_scroll",
            "overflow_y_scrollbar",
            "overflow_scrollbar",
        ] {
            if line.contains(forbidden) {
                violations.push(format!(
                    "src/ui/shells/show.rs:{line_number}: Situational ADR 0063 task 003 card grid must not own a scroll container; found `{forbidden}` in `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0063 Show detail-panel violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0062: mixed Music rows carry a kind and shared badges.
#[test]
fn adr_0062_mixed_entity_row_contract_is_kind_backed() {
    let library_source = read_source(&manifest_path("src/view_models/library.rs"));
    let search_results_source =
        read_source(&manifest_path("src/view_models/search_results/results.rs"));
    let row_contract = source_between(
        &library_source,
        "pub(crate) enum ContentListEntityKind",
        "/// Empty-state display for a filtered content-list frame.",
    );
    let row_struct = source_between(
        &library_source,
        "pub(crate) struct ContentListRowDisplay",
        "impl ContentListRowDisplay",
    );
    let mut violations = Vec::new();

    for required in [
        "pub(crate) enum ContentListRowKind",
        "Artist(ArtistResultDisplay)",
        "Release(FeedResultDisplay)",
        "Track(TrackResultDisplay)",
        "pub(crate) struct ContentListEntityBadgeDisplay",
        "pub(crate) struct ContentListLibraryBadgeDisplay",
        "pub(crate) enum ContentListRowExpansionDisplay",
        "pub(crate) enum ContentListRowExpansionState",
        "ContentListLibraryBadgeDisplay::for_source(source)",
        "ContentListRowExpansionDisplay::for_kind(entity_kind, expanded)",
        "pub(crate) fn from_artist_result(",
        "pub(crate) fn from_release_result(",
        "pub(crate) fn from_track_result(",
    ] {
        if !row_contract.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 mixed entity row contract missing `{required}`"
            ));
        }
    }

    let variant_count = ["Artist(", "Release(", "Track("]
        .into_iter()
        .filter(|variant| row_contract.contains(variant))
        .count();
    if variant_count != 3 {
        violations.push(format!(
            "src/view_models/library.rs: Situational ADR 0062 mixed entity row contract must have exactly three row-kind cases; found {variant_count}"
        ));
    }

    for required in [
        "pub(crate) id: String",
        "pub(crate) kind: ContentListRowKind",
        "pub(crate) entity_badge: ContentListEntityBadgeDisplay",
        "pub(crate) library_badge: ContentListLibraryBadgeDisplay",
        "pub(crate) source: ContentListRowSource",
        "pub(crate) expansion: ContentListRowExpansionDisplay",
    ] {
        if !row_struct.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 mixed entity row display missing `{required}`"
            ));
        }
    }

    for forbidden in [
        "artist: Option",
        "release: Option",
        "feed: Option",
        "track: Option",
        "artist_display",
        "release_display",
        "feed_display",
        "track_display",
        "title: String",
        "secondary_text: String",
    ] {
        if row_struct.contains(forbidden) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 mixed entity row display must not carry top-level kind-specific or copied fields; found `{forbidden}`"
            ));
        }
    }

    for required in [
        "pub(crate) struct ArtistResultDisplay",
        "pub(crate) struct FeedResultDisplay",
        "pub(crate) struct TrackResultDisplay",
    ] {
        if !search_results_source.contains(required) {
            violations.push(format!(
                "src/view_models/search_results/results.rs: Situational ADR 0062 must compose existing result displays; missing `{required}`"
            ));
        }
    }

    for (path, source) in [
        ("src/view_models/library.rs", library_source.as_str()),
        (
            "src/view_models/search_results/results.rs",
            search_results_source.as_str(),
        ),
    ] {
        for (line_number, line) in code_lines(source) {
            for pattern in VIEW_MODEL_FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{path}:{line_number}: Situational ADR 0062 mixed entity row VM must stay renderer-free; found `{pattern}` in `{line}`"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0062 mixed entity row contract violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0062: Music opens on recent content rows.
#[test]
fn adr_0062_music_default_content_projects_recent_music_rows() {
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let library_source = read_source(&manifest_path("src/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let content_shell_source = read_source(&manifest_path("src/ui/shells/library/content_list.rs"));
    let library_render = source_between(
        &library_app_source,
        "impl Render for LibraryApp",
        "#[cfg(test)]",
    );
    let no_selection_branch = source_between(
        &library_render,
        "let content = if matches!(self.detail, LibraryDetail::None) {",
        "} else {",
    );
    let mut violations = Vec::new();

    for required in [
        "pub(crate) enum ContentListPageSource",
        "RecentMusic",
        "pub(crate) fn begin_recent_music_load(&mut self, append: bool)",
        "pub(crate) fn replace_recent_feeds_page(&mut self, page: &RecentFeedsPageVm)",
        "fn content_list_rows_from_recent_feeds(",
        "ContentListRowDisplay::from_release_result(row.clone(), false)",
        "pub(crate) fn page_state_display(&self) -> Option<ContentListPageStateDisplay>",
        "pub(crate) fn load_more_display(&self) -> Option<ContentListLoadMoreDisplay>",
    ] {
        if !library_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 Music default content must project Recent Feeds into ContentList rows; missing `{required}`. Fix: project the recency source through ContentListPageVm before rendering."
            ));
        }
    }

    for required in [
        "recent_music_page: RecentFeedsPageVm",
        "recent_music_scroll: ScrollHandle",
    ] {
        if !library_source.contains(required) {
            violations.push(format!(
                "src/library.rs: Situational ADR 0062 Music default content must own the existing Recent Feeds pager instance; missing `{required}`. Fix: store RecentFeedsPageVm on LibraryApp, not a new pager type."
            ));
        }
    }

    for required in [
        "app.start_recent_music_load(false, cx);",
        "render_library_content_list(",
        "self.vm.content_list_page()",
        "&self.recent_music_scroll",
    ] {
        if !library_app_source.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: Situational ADR 0062 Music default content must render recent rows in the content region; missing `{required}`. Fix: mount render_library_content_list in the no-selection Music content branch."
            ));
        }
    }

    for required in [
        "SplitPane::new(chrome.split_pane_id)",
        ".leading(leading_pane)",
        ".trailing(trailing_pane)",
        "render_library_content_list(",
    ] {
        if !no_selection_branch.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: Situational ADR 0062 Music no-selection branch must keep the source tree separate from content rows; missing `{required}`. Fix: render a source pane plus content-list pane."
            ));
        }
    }

    if no_selection_branch.trim() == "leading_pane" {
        violations.push(
            "src/library/app_impl.rs: Situational ADR 0062 Music no-selection branch must not return the navigation tree as the content region. Fix: render ContentListPageVm::visible_rows() in the content pane."
                .to_string(),
        );
    }

    for required in [
        "page.visible_rows()",
        "page.page_state_display()",
        "page.load_more_display()",
        "render_content_list_row(",
    ] {
        if !content_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/library/content_list.rs: Situational ADR 0062 Music content renderer must consume ContentListPageVm rows and displays; missing `{required}`. Fix: render the VM row/status/load-more projections, not the source tree."
            ));
        }
    }

    if content_shell_source.contains("render_library_sidebar") {
        violations.push(
            "src/ui/shells/library/content_list.rs: Situational ADR 0062 Music content renderer must not render the navigation tree. Fix: keep source-list rendering in the source pane."
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0062 Music default content violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0062: Music recency rows reuse the Recent Feeds pager.
#[test]
fn adr_0062_music_default_content_reuses_recent_feeds_pager() {
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let library_source = read_source(&manifest_path("src/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let recent_vm_source = read_source(&manifest_path("src/view_models/recent_feeds.rs"));
    let content_shell_source = read_source(&manifest_path("src/ui/shells/library/content_list.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) fn begin_load(&mut self, append: bool)",
        "pub(crate) fn finish_load(&mut self, batch: RecentFeedsPageBatch, append: bool)",
        "pub(crate) const fn has_more(&self) -> bool",
    ] {
        if !recent_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/recent_feeds.rs: Situational ADR 0062 must consume the existing RecentFeedsPageVm pager; missing `{required}`. Fix: keep cursor paging in RecentFeedsPageVm."
            ));
        }
    }

    for required in [
        "recent_music_page: RecentFeedsPageVm",
        "self.recent_music_page.begin_load(append)",
        "this.recent_music_page.finish_load(batch, append)",
        "this.recent_music_page.fail_load(",
        "if append { loaded_row_count } else { 0 }",
        "replace_recent_music_content(&self.recent_music_page)",
        "replace_recent_music_content(&this.recent_music_page)",
    ] {
        if !library_source.contains(required) && !library_app_source.contains(required) {
            violations.push(format!(
                "src/library.rs or src/library/app_impl.rs: Situational ADR 0062 Music recency must reuse RecentFeedsPageVm pagination; missing `{required}`. Fix: call the existing begin_load/finish_load/fail_load path."
            ));
        }
    }

    for forbidden in [
        "ContentListLoadIntent",
        "ContentListPageBatch",
        "content_list_cursor",
        "recent_music_cursor",
        "cursor: Option<String>",
        "fn begin_content_list_load",
    ] {
        if library_vm_source.contains(forbidden)
            || library_source.contains(forbidden)
            || library_app_source.contains(forbidden)
            || content_shell_source.contains(forbidden)
        {
            violations.push(format!(
                "Situational ADR 0062 forbids a second pager beside RecentFeedsPageVm; found `{forbidden}`. Fix: keep cursor state in src/view_models/recent_feeds.rs."
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0062 Recent Feeds pager reuse violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0062: the Recent Feeds destination is retired, while its pager survives.
#[test]
fn adr_0062_recent_feeds_destination_is_retired() {
    let nav_source = read_source(&manifest_path("src/view_models/workspace/nav.rs"));
    let toolbar_vm_source = read_source(&manifest_path("src/view_models/app_toolbar.rs"));
    let toolbar_source = read_source(&manifest_path("src/app/tab_bar.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let shells_mod_source = read_source(&manifest_path("src/ui/shells/mod.rs"));
    let recent_vm_source = read_source(&manifest_path("src/view_models/recent_feeds.rs"));
    let feed_query_source = read_source(&manifest_path("src/application/queries/feed.rs"));
    let library_source = read_source(&manifest_path("src/library.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let adr_0030_source = read_source(&manifest_path(
        "docs/adr/0030-discovery-library-ui-fixes.md",
    ));
    let route_plan_source = read_source(&manifest_path(
        "docs/plans/post-adr-0048-recent-feeds-route-plan.md",
    ));
    let mut violations = Vec::new();

    for path in ["src/app/recent_feeds.rs", "src/ui/shells/recent_feeds.rs"] {
        if manifest_path(path).exists() {
            violations.push(format!(
                "{path}: Situational ADR 0062 retired Recent Feeds destination guard found a route module. Fix: keep the recency pager, not a separate destination."
            ));
        }
    }

    for (label, source, forbidden) in [
        (
            "src/view_models/workspace/nav.rs",
            &nav_source,
            "FrameNavigationEntry::RecentFeeds",
        ),
        (
            "src/view_models/workspace/nav.rs",
            &nav_source,
            "\"Recent Feeds\".to_string()",
        ),
        (
            "src/view_models/app_toolbar.rs",
            &toolbar_vm_source,
            "recent_feeds_button_id",
        ),
        (
            "src/view_models/app_toolbar.rs",
            &toolbar_vm_source,
            "app-toolbar-recent-feeds",
        ),
        (
            "src/app/tab_bar.rs",
            &toolbar_source,
            "render_recent_feeds_button",
        ),
        (
            "src/app/tab_bar.rs",
            &toolbar_source,
            "open_recent_feeds_in_content_list",
        ),
        ("src/app.rs", &app_source, "mod recent_feeds;"),
        (
            "src/app.rs",
            &app_source,
            "recent_feeds_detail: Option<RecentFeedsPageVm>",
        ),
        (
            "src/app/search_dispatch.rs",
            &search_dispatch_source,
            "pub(super) fn open_recent_feeds_in_content_list(",
        ),
        (
            "src/app/search_dispatch.rs",
            &search_dispatch_source,
            "pub(super) fn start_recent_feeds_load(",
        ),
        (
            "src/app/search_dispatch.rs",
            &search_dispatch_source,
            "handle_recent_feed_selected(",
        ),
        (
            "src/ui/shells/mod.rs",
            &shells_mod_source,
            "pub mod recent_feeds;",
        ),
    ] {
        if source.contains(forbidden) {
            violations.push(format!(
                "{label}: Situational ADR 0062 retired Recent Feeds destination guard found `{forbidden}`. Fix: do not restore a toolbar command, route, or screen mount for the destination."
            ));
        }
    }

    for (label, source, required) in [
        (
            "src/view_models/recent_feeds.rs",
            &recent_vm_source,
            "pub(crate) struct RecentFeedsPageVm",
        ),
        (
            "src/view_models/recent_feeds.rs",
            &recent_vm_source,
            "pub(crate) struct RecentFeedsPageBatch",
        ),
        (
            "src/view_models/recent_feeds.rs",
            &recent_vm_source,
            "pub(crate) struct RecentFeedsLoadIntent",
        ),
        (
            "src/application/queries/feed.rs",
            &feed_query_source,
            "pub(crate) struct FetchRecentFeedsPage",
        ),
        (
            "src/application/queries/feed.rs",
            &feed_query_source,
            "fetch_recent_feeds(Some(crate::api::PAGE_LIMIT), cursor)",
        ),
        (
            "src/library.rs",
            &library_source,
            "recent_music_page: RecentFeedsPageVm",
        ),
        (
            "src/library/app_impl.rs",
            &library_app_source,
            "FetchRecentFeedsPage::new(",
        ),
        (
            "docs/adr/0030-discovery-library-ui-fixes.md",
            &adr_0030_source,
            "ADR 0062 withdraws the Recent Feeds",
        ),
        (
            "docs/plans/post-adr-0048-recent-feeds-route-plan.md",
            &route_plan_source,
            "Retired by ADR 0062",
        ),
    ] {
        if !source.contains(required) {
            violations.push(format!(
                "{label}: Situational ADR 0062 retired Recent Feeds destination guard missing `{required}`. Fix: retire only the destination and keep the Music recency data source documented."
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0062 retired Recent Feeds destination violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0062: the blocking recency query stays off the render path.
#[test]
fn adr_0062_music_default_content_query_stays_off_render_path() {
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let content_shell_source = read_source(&manifest_path("src/ui/shells/library/content_list.rs"));
    let library_render = source_between(
        &library_app_source,
        "impl Render for LibraryApp",
        "#[cfg(test)]",
    );
    let mut violations = Vec::new();

    for required in [
        "pub(crate) fn start_recent_music_load(&mut self, append: bool, cx: &mut Context<Self>)",
        "FetchRecentFeedsPage::new(",
        "present_command(",
        "CommandContext::next()",
    ] {
        if !library_app_source.contains(required) {
            violations.push(format!(
                "src/library/app_impl.rs: Situational ADR 0062 Music recency query must run through the runtime command path; missing `{required}`. Fix: keep the blocking index request inside start_recent_music_load."
            ));
        }
    }

    for forbidden in [
        "FetchRecentFeedsPage::new(",
        "fetch_recent_feeds(",
        "Client::new_with_base_url",
        "begin_load(append)",
        "present_command(",
    ] {
        if library_render.contains(forbidden) || content_shell_source.contains(forbidden) {
            violations.push(format!(
                "Situational ADR 0062 forbids blocking recency work on the render path; found `{forbidden}`. Fix: trigger start_recent_music_load from lifecycle or event callbacks only."
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0062 render-path query violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0062: library membership is one tri-state control.
#[test]
fn adr_0062_library_tri_state_control_contract_is_vm_owned() {
    let workspace_source = workspace_vm_source();
    let library_source = read_source(&manifest_path("src/view_models/library.rs"));
    let library_filter_source = read_source(&manifest_path(
        "src/ui/composites/library_filter_control.rs",
    ));
    let button_source = read_source(&manifest_path("src/ui/primitives/button.rs"));
    let mut violations = Vec::new();

    for required in [
        "pub(crate) enum LibraryFilterControlTreatment",
        "pub(crate) struct LibraryFilterControlStateDisplay",
        "pub(crate) struct LibraryFilterControlDisplay",
        "text_label: &'static str",
        "state_label: &'static str",
        "a11y_label: &'static str",
        "next_filter: ContentFilter",
        "keyboard_cycle_order: [ContentFilter; 3]",
        "ContentFilter::All => LibraryFilterControlStateDisplay",
        "text_label: \"Library\"",
        "state_label: \"Any\"",
        "a11y_label: \"Any library status\"",
        "treatment: LibraryFilterControlTreatment::Off",
        "ContentFilter::Library => LibraryFilterControlStateDisplay",
        "state_label: \"In library\"",
        "a11y_label: \"In library\"",
        "treatment: LibraryFilterControlTreatment::Highlighted",
        "ContentFilter::Index => LibraryFilterControlStateDisplay",
        "state_label: \"Not in library\"",
        "a11y_label: \"Not in library\"",
        "treatment: LibraryFilterControlTreatment::StruckThrough",
        "matches!(self, Self::StruckThrough)",
        "ContentFilter::All => ContentFilter::Library",
        "ContentFilter::Library => ContentFilter::Index",
        "ContentFilter::Index => ContentFilter::All",
    ] {
        if !workspace_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/chrome.rs: Situational ADR 0062 library tri-state control contract missing `{required}`. Fix: keep labels, a11y text, treatments, and cycle order in the view model."
            ));
        }
    }

    for required in [
        "pub(crate) fn library_filter_control_display(&self) -> LibraryFilterControlDisplay",
        "LibraryFilterControlDisplay::default_for_content_list(self.filter_state)",
        "pub(crate) fn content_library_filter_control(&self) -> LibraryFilterControlDisplay",
    ] {
        if !library_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 library tri-state control projection missing `{required}`. Fix: project the Music content filter through LibraryFilterControlDisplay."
            ));
        }
    }

    for required in [
        "library_rows: Vec<ContentListRowDisplay>",
        "let library_rows = cached_rows.clone();",
        "self.library_rows = cached_rows;",
        "self.library_rows.clone_from(&cached_rows);",
        "&self.library_rows",
        "self.has_more && !matches!(self.filter_state, ContentFilter::Library)",
        "if self.has_more()",
    ] {
        if !library_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 library tri-state data-source contract missing `{required}`. Fix: keep local library rows available when the recent-music source is active, and do not expose remote pagination for the Library-only state."
            ));
        }
    }

    for required in [
        "Button::styled(",
        "SharedString::from(display.id)",
        "control_style(display.current.treatment)",
        ".label(display.current.text_label)",
        ".a11y_label(display.current.a11y_label)",
        ".label_treatment(label_treatment(display.current.treatment))",
        "LibraryFilterControlTreatment::StruckThrough => ButtonLabelTreatment::LineThrough",
        "LibraryFilterControlTreatment::StruckThrough => ControlStyle::Secondary",
        "LibraryFilterControlTreatment::Off => Some(SemanticColor::SecondaryLabel)",
        "button.foreground(foreground)",
        ".on_activate(move |window, cx|",
        "handler(next_filter, window, cx)",
    ] {
        if !library_filter_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/library_filter_control.rs: Situational ADR 0062 library tri-state renderer bridge missing `{required}`. Fix: render the VM display contract and dispatch the VM-projected next filter."
            ));
        }
    }

    for forbidden in [
        "ContentFilter::All",
        "ContentFilter::Library",
        "ContentFilter::Index",
        "text_label:",
        "a11y_label:",
        "\"Library:",
        "\"Not in library\"",
    ] {
        if library_filter_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/composites/library_filter_control.rs: Situational ADR 0062 forbids renderer-owned tri-state labels or state branches; found `{forbidden}`. Fix: keep state strings and state mapping in the view model."
            ));
        }
    }

    for required in [
        "pub const fn label_treatment(mut self, treatment: ButtonLabelTreatment)",
        "pub fn on_activate<F>(mut self, handler: F) -> Self",
        ".tab_index(0)",
        ".on_key_down(move |event, window, cx|",
        "keyboard_activation_key(event)",
    ] {
        if !button_source.contains(required) {
            violations.push(format!(
                "src/ui/primitives/button.rs: Situational ADR 0062 library tri-state control primitive support missing `{required}`. Fix: keep Button text treatment and keyboard activation support."
            ));
        }
    }

    if !button_source.contains("ButtonLabelTreatment::LineThrough") {
        violations.push(
            "src/ui/primitives/button.rs: Situational ADR 0062 library tri-state control needs a primitive text-treatment hook. Fix: expose label_treatment on Button so exclusion is not color-only."
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0062 library tri-state control violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0062: Music content uses one persisted tile/list mode over one row contract.
#[test]
fn adr_0062_music_content_tile_and_list_modes_share_row_contract() {
    let workspace_chrome_source =
        read_source(&manifest_path("src/view_models/workspace/chrome.rs"));
    let library_vm_source = read_source(&manifest_path("src/view_models/library.rs"));
    let content_shell_source = read_source(&manifest_path("src/ui/shells/library/content_list.rs"));
    let frame_shell_vm_source = read_source(&manifest_path("src/view_models/workspace/chrome.rs"));
    let frame_shell_source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let workspace_shell_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    let view_mode_control_source =
        read_source(&manifest_path("src/ui/composites/view_mode_control.rs"));
    let config_source = read_source(&manifest_path("src/config.rs"));
    let tokens_source = read_source(&manifest_path("src/ui/tokens.rs"));
    let app_source = read_source(&manifest_path("src/app.rs"));
    let resize_source = read_source(&manifest_path("src/app/resize.rs"));
    let mut violations = Vec::new();

    let content_view_mode_enum_count = rust_files_under("src")
        .into_iter()
        .map(|path| read_source(&path))
        .filter(|source| source.contains("enum ContentViewMode"))
        .count();
    if content_view_mode_enum_count != 1 {
        violations.push(format!(
            "Situational ADR 0062 content view-mode guard expects exactly one ContentViewMode enum; found {content_view_mode_enum_count}. Fix: promote the existing mode enum instead of defining another."
        ));
    }
    if rust_files_under("src")
        .into_iter()
        .map(|path| read_source(&path))
        .any(|source| source.contains("enum RecentFeedsViewMode"))
    {
        violations.push(
            "Situational ADR 0062 content view-mode guard forbids a RecentFeedsViewMode enum. Fix: keep Music content on the shared ContentViewMode enum."
                .to_string(),
        );
    }

    for required in [
        "pub(crate) enum ContentViewMode",
        "#[serde(rename_all = \"snake_case\")]",
        "Tiles",
        "List",
        "pub(crate) struct ContentViewModeControlDisplay",
        "pub(crate) struct ContentViewModeOptionDisplay",
        "default_for_content_list(selected: ContentViewMode)",
        "options: [ContentViewModeOptionDisplay; 2]",
        "content_list_a11y_label",
    ] {
        if !workspace_chrome_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/chrome.rs: Situational ADR 0062 content view-mode contract missing `{required}`. Fix: keep labels, options, and accessibility in the VM layer."
            ));
        }
    }

    for required in [
        "view_mode: ContentViewMode",
        "pub(crate) const fn view_mode(&self) -> ContentViewMode",
        "pub(crate) fn set_view_mode(&mut self, view_mode: ContentViewMode)",
        "pub(crate) fn view_mode_control_display(&self) -> ContentViewModeControlDisplay",
        "ContentViewModeControlDisplay::default_for_content_list(self.view_mode)",
        "pub(crate) const fn content_view_mode(&self) -> ContentViewMode",
        "pub(crate) fn set_content_view_mode(&mut self, view_mode: ContentViewMode)",
        "pub(crate) fn content_view_mode_control(&self) -> ContentViewModeControlDisplay",
    ] {
        if !library_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 content view-mode projection missing `{required}`. Fix: own Music content mode in ContentListPageVm and expose it through LibraryViewModel."
            ));
        }
    }

    for required in [
        "fn content_list_rows_from_tree(tree: &LibraryTree) -> Vec<ContentListRowDisplay>",
        "let album_image_href = album.image_href.as_deref();",
        "ContentListRowDisplay::from_track_with_album_image(",
        "pub(crate) fn from_track_with_album_image(",
        ".or_else(|| album_image_href.map(str::to_owned))",
        "display = display.with_thumbnail_href(thumbnail_href);",
    ] {
        if !library_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 tree artwork projection missing `{required}`. Fix: carry AlbumNode image_href into tree-derived ContentListRowDisplay thumbnails."
            ));
        }
    }

    for required in [
        "match page.view_mode()",
        "ContentViewMode::List =>",
        "render_content_list_rows(page, render_state, thumbnails, scroll_handle, cx)",
        "ContentViewMode::Tiles =>",
        "render_content_list_tiles(page, render_state, thumbnails, scroll_handle, cx)",
        "fn render_content_list_rows(",
        "fn render_content_list_tiles(",
        "fn render_content_list_tile(",
        "fn render_empty_content_tile_artwork(",
        "row.entity_badge.label",
        "row.library_badge.label",
        "ImagePrimitive::new(image)",
        "ContentListPageVm::skeleton_tile_id(index)",
    ] {
        if !content_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/library/content_list.rs: Situational ADR 0062 tile/list renderer contract missing `{required}`. Fix: have both presentations render the same ContentListRowDisplay fields."
            ));
        }
    }

    for forbidden in [
        "layout::SEARCH_TILE_WIDTH",
        "layout::THUMBNAIL_XL",
        "gpui::px(",
        ".emoji()",
    ] {
        if content_shell_source.contains(forbidden) {
            violations.push(format!(
                "src/ui/shells/library/content_list.rs: Situational ADR 0062 tile geometry and empty artwork must use tokens and explicit VM data; found `{forbidden}`."
            ));
        }
    }

    for required in ["ContentTileWidth", "ContentTileArtwork"] {
        if !tokens_source.contains(required) {
            violations.push(format!(
                "src/ui/tokens.rs: Situational ADR 0062 content tile geometry token missing `{required}`. Fix: keep Music tile dimensions in named tokens."
            ));
        }
    }

    for required in [
        "view_mode_control: Option<ContentViewModeControlDisplay>",
        "pub(crate) fn with_view_mode_control(",
    ] {
        if !frame_shell_vm_source.contains(required) {
            violations.push(format!(
                "src/view_models/workspace/chrome.rs: Situational ADR 0062 frame-shell view-mode display missing `{required}`. Fix: carry view-mode chrome through FrameShellDisplay."
            ));
        }
    }

    for required in [
        "view_mode_control(display, view_mode_slots)",
        "pub(crate) fn on_view_mode_select(",
        "handler(view_mode, window, cx)",
    ] {
        if !frame_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/frame_shell.rs: Situational ADR 0062 frame-shell view-mode wiring missing `{required}`. Fix: render shared view-mode chrome through the frame shell."
            ));
        }
    }

    for required in [
        "content_list_view_mode_control: Option<ContentViewModeControlDisplay>",
        "pub(crate) fn content_list_view_mode_control(",
        "on_content_list_view_mode_select: Option<WorkspaceViewModeSelectHandler>",
        "pub(crate) fn on_content_list_view_mode_select(",
        "WorkspaceFrameKind::ContentList => self.content_list_view_mode_control.clone()",
        "display = display.with_view_mode_control(view_mode_control);",
    ] {
        if !workspace_shell_source.contains(required) {
            violations.push(format!(
                "src/ui/shells/workspace.rs: Situational ADR 0062 workspace-shell view-mode wiring missing `{required}`. Fix: route Music content view mode as frame-local chrome."
            ));
        }
    }

    for required in [
        "pub(crate) fn view_mode_control(",
        "ContentViewModeControlDisplay",
        "SegmentedControl::new(selected)",
        ".filter_style()",
        "label: SharedString::from(option.label)",
        "a11y_label: SharedString::from(option.a11y_label)",
    ] {
        if !view_mode_control_source.contains(required) {
            violations.push(format!(
                "src/ui/composites/view_mode_control.rs: Situational ADR 0062 view-mode composite missing `{required}`. Fix: adapt the VM display contract without renderer-owned labels."
            ));
        }
    }

    for required in [
        "content_list_view_mode: Option<ContentViewMode>",
        "#[serde(default, skip_serializing_if = \"Option::is_none\")]",
        "mode.id_suffix().to_string()",
    ] {
        if !config_source.contains(required) {
            violations.push(format!(
                "src/config.rs: Situational ADR 0062 content view-mode persistence missing `{required}`. Fix: persist the selected Music content mode while keeping old configs loadable."
            ));
        }
    }

    for required in [
        "Self::initial_content_list_view_mode(workspace_layout_prefs)",
        ".content_list_view_mode_control(content_view_mode_control)",
        ".on_content_list_view_mode_select(",
        "this.set_content_list_view_mode(view_mode, cx)",
        "fn set_content_list_view_mode(",
        "persist_content_list_view_mode(view_mode)",
    ] {
        if !app_source.contains(required) {
            violations.push(format!(
                "src/app.rs: Situational ADR 0062 content view-mode app wiring missing `{required}`. Fix: route the frame chrome selection back through TopApp."
            ));
        }
    }

    for required in [
        "pub(super) fn initial_content_list_view_mode(",
        "content_list_view_mode: Some(content_list_view_mode)",
        "pub(super) fn persist_content_list_view_mode(",
    ] {
        if !resize_source.contains(required) {
            violations.push(format!(
                "src/app/resize.rs: Situational ADR 0062 content view-mode persistence bridge missing `{required}`. Fix: save pane width and content mode together."
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0062 Music content tile/list mode violations:\n{}",
        violations.join("\n")
    );
}

fn display_surface_files() -> Vec<String> {
    let mut files = Vec::new();
    files.push("src/app.rs".to_string());
    for dir in ["src/app", "src/ui", "src/view_models"] {
        files.extend(
            rust_files_under(dir)
                .into_iter()
                .map(|path| rel_path(&path)),
        );
    }
    files.sort();
    files.dedup();
    files
}

/// Situational ADR 0063: logs leave the side panel and use the shared vertical split.
#[test]
fn adr_0063_logs_use_an_independent_bottom_pane_and_current_request() {
    let panel = read_source(&manifest_path("src/ui/composites/show_detail_panel.rs"));
    let pane = read_source(&manifest_path("src/ui/composites/show_log_pane.rs"));
    let shell = read_source(&manifest_path("src/ui/shells/show.rs"));
    let app = read_source(&manifest_path("src/app/show.rs"));
    let vm = read_source(&manifest_path("src/view_models/show.rs"));
    let log_path = source_between(
        &app,
        "    fn open_publisher_logs(",
        "    fn run_event_target_command(",
    );
    for forbidden in [
        "render_publisher_logs",
        "render_log_output",
        "log_pane",
        "log_panel",
        "PublisherLogPanelState",
    ] {
        assert!(!panel.contains(forbidden), "Situational ADR 0063: detail panel contains {forbidden}. Move log text to ShowLogPane; keep service actions in the panel.");
    }
    for forbidden in [
        ".truncate()",
        ".whitespace_normal()",
        ".text_ellipsis()",
        "format!(",
        "cursor_row_resize",
        "on_mouse_move",
    ] {
        assert!(!pane.contains(forbidden), "Situational ADR 0063: log pane contains {forbidden}. Use VM labels, unwrapped scrolling lines, and the shared SplitPane handle.");
    }
    for required in [
        "SplitPane::new(\"show-log-split\")",
        ".axis(SplitPaneAxis::Vertical)",
        ".leading_height(",
        ".leading_min_height(",
        ".overflow_scroll()",
        ".items_start()",
        "SelectableText::new(\"show-log-text\", display.text)",
        "display.unit_name",
        "display.line_count_label",
        "display.close.a11y_label",
    ] {
        assert!(pane.contains(required), "Situational ADR 0063: log pane is missing {required}. Restore the shared split and complete unwrapped journal viewport.");
    }
    for forbidden in [
        "show_card_detail",
        "select_card",
        "panel_mode",
        "panel_open",
    ] {
        assert!(!log_path.contains(forbidden), "Situational ADR 0063: log path changes {forbidden}. Log actions and results must leave the side panel alone.");
    }
    for required in [
        "toggle_publisher_logs(role)",
        "apply_result(logs.request_id",
        "apply_result(request_id",
        "self.show_page.close_publisher_logs()",
    ] {
        assert!(log_path.contains(required), "Situational ADR 0063: log path is missing {required}. Gate both success and failure on the current request.");
    }
    assert!(
        app.contains("self.show_page.log_pane.clone()"),
        "Situational ADR 0063: preserve log state and request identity when reprojecting Show."
    );
    assert!(
        vm.contains("pub(crate) log_pane: ShowLogPaneDisplay"),
        "Situational ADR 0063: ShowPageVm must own log pane state."
    );
    let publisher = source_between(
        &vm,
        "pub(crate) struct PublisherSectionDisplay",
        "/// Display-ready Event section",
    );
    assert!(
        !publisher.contains("log_pane"),
        "Situational ADR 0063: publisher detail must not own the log pane."
    );
    assert!(
        shell.contains("ShowLogPane::new("),
        "Situational ADR 0063: mount ShowLogPane in the main region."
    );
    assert!(
        shell.contains("render_queue_transport(transport, self.slots.queue)"),
        "Situational ADR 0063: keep transport outside the log split."
    );
}

/// Situational ADR 0063: the log selection owner is plain text, read-only, and does not wrap.
#[test]
fn adr_0063_log_text_is_selectable_and_copied_without_rewriting() {
    let text = read_source(&manifest_path("src/ui/composites/selectable_text.rs"));
    for required in [
        "StyledText::new(self.value.clone())",
        ".whitespace_nowrap()",
        ".track_focus(&self.focus)",
        ".on_mouse_down(",
        ".on_mouse_move(",
        ".on_key_down(",
        "self.selection.selected_text(&self.value)",
        "ClipboardItem::new_string(text.to_owned())",
        "if self.value != value",
    ] {
        assert!(text.contains(required), "Situational ADR 0063: selectable log text needs {required}. Preserve exact plain text and focus/selection behavior.");
    }
    for forbidden in [
        "TextView::html",
        "TextView::markdown",
        ".trim()",
        ".truncate()",
        "replace_text_in_range",
        "Input::new",
    ] {
        assert!(!text.contains(forbidden), "Situational ADR 0063: selectable logs contain {forbidden}. Logs must remain read-only plain text with exact clipboard contents.");
    }
}

/// Situational ADR 0063: mouse copying shares the exact selection path and menu owner.
#[test]
fn adr_0063_log_copy_menu_preserves_selection_and_uses_typed_actions() {
    let text = read_source(&manifest_path("src/ui/composites/selectable_text.rs"));
    let menu = read_source(&manifest_path("src/ui/primitives/context_menu.rs"));
    let vm = read_source(&manifest_path("src/view_models/text_selection.rs"));
    for required in [
        "PointerContextMenu::new(",
        "self.selection.copy_action(&self.value)",
        "disabled: copy.availability.disabled()",
        "a11y_label: copy.a11y_label.into()",
        "self.copy_selection(cx)",
        "this.copy_selection(cx)",
    ] {
        assert!(text.contains(required), "Situational ADR 0063: log menu needs {required}. Use the shared menu and exact clipboard path.");
    }
    let right_click = source_between(&text, "MouseButton::Right,", "MouseButton::Left,");
    assert!(
        !right_click.contains("this.selection"),
        "Situational ADR 0063: opening the menu must preserve the current selection."
    );
    let update = source_between(&text, "fn update_value", "fn copy_selection");
    assert!(
        update.contains("self.menu_position = None"),
        "Situational ADR 0063: replacing log text must dismiss its old menu."
    );
    assert_eq!(
        text.matches("cx.write_to_clipboard(").count(),
        1,
        "Situational ADR 0063: keyboard and mouse Copy must use one clipboard path."
    );
    for required in [
        "build_menu_content(&dismiss, self.items, cx)",
        "Surface::new(SurfaceElevation::Floating)",
        ".snap_to_window_with_margin(Spacing::SM.scaled(cx))",
        ".on_mouse_down_out(",
        "event.keystroke.key == \"escape\"",
        "self.return_focus.focus(window)",
    ] {
        assert!(menu.contains(required), "Situational ADR 0063: pointer menu needs {required}. Keep chrome, anchoring, dismissal, and focus in the shared owner.");
    }
    assert!(
        vm.contains("enum TextCopyAvailability"),
        "Situational ADR 0063: Copy requires typed availability before rendering."
    );
    for forbidden in ["use gpui", "SharedString", "FocusHandle"] {
        assert!(!vm.contains(forbidden), "Situational ADR 0063: text selection view model must remain renderer-free; found {forbidden}.");
    }
}

/// Situational ADR 0059: Event belongs to Live Metadata; registry actions cannot configure a publisher.
#[test]
fn adr_0059_event_row_precedes_services_and_registry_actions_do_not_attach() {
    let vm = read_source(&manifest_path("src/view_models/show.rs"));
    let kinds = source_between(&vm, "pub(crate) enum ShowCardKind", "impl ShowCardKind");
    assert!(
        !code_lines(kinds).any(|(_, line)| line.trim() == "Event,"),
        "ADR 0059: Event is a row, never a card kind"
    );
    let publisher = source_between(
        &vm,
        "pub(crate) struct PublisherSectionDisplay",
        "/// Display-ready Event",
    );
    assert!(publisher.contains("pub(crate) event: Option<EventSectionDisplay>"));
    let page = source_between(&vm, "pub(crate) struct ShowPageVm", "impl ShowPageVm");
    assert!(!page.contains("pub(crate) event:"));
    let panel = read_source(&manifest_path("src/ui/composites/show_detail_panel.rs"));
    let detail = source_between(
        &panel,
        "fn render_publisher_detail(",
        "fn render_publisher_service(",
    );
    assert!(
        detail.find("render_event_detail(").unwrap()
            < detail.find("for service in section.services").unwrap(),
        "ADR 0059: Event must render before Producer and Publisher"
    );
    assert!(panel.contains("EventControlIntent::CopyFeedTag"));
    assert!(panel.contains("cx.write_to_clipboard(ClipboardItem::new_string(tag.clone()))"));
    assert!(vm.contains("action: actions.copy_feed_tag.clone()"));
    let app = read_source(&manifest_path("src/app/show.rs"));
    let command = source_between(
        &app,
        "struct EventRegistryCommand",
        "fn event_registry_error(",
    );
    let compact_command = command.split_whitespace().collect::<String>();
    for required in [
        "registry.create_event(None)",
        "registry.check_event(&event.event_id)",
    ] {
        assert!(
            compact_command.contains(required),
            "ADR 0059: registry command must call {required}"
        );
    }
    for forbidden in [
        "publisher_targets",
        "Transport",
        "attach_event(",
        "detach_target(",
        "forget_event(",
        "fs::write",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !command.contains(forbidden),
            "ADR 0059: registration/checking must not use {forbidden}"
        );
    }
    let wiring = source_between(
        &app,
        "fn run_event_registry_command(",
        "fn run_event_target_command(",
    );
    assert!(wiring.contains("this.run_event_registry_command(next, cx)"));
    assert!(
        wiring
            .find("this.reproject_show_page_from_current_queue()")
            .unwrap()
            < wiring
                .find("this.run_event_registry_command(next, cx)")
                .unwrap(),
        "ADR 0059: project the registered event before requesting its initial check"
    );
}

/// Situational ADR 0059: every Show projection reapplies command ownership before rendering.
#[test]
fn adr_0059_show_command_feedback_survives_all_reprojections() {
    let app = read_source(&manifest_path("src/app/show.rs"));
    let projection = source_between(
        &app,
        "fn reproject_show_page(",
        "fn reproject_show_page_from_current_queue(",
    );
    assert!(projection.contains(".with_command_state(&self.show_commands)"));
    assert!(projection.contains(".with_status_message(&self.settings_status)"));
    let watch = source_between(
        &app,
        "fn apply_publisher_service_snapshot(",
        "fn apply_broadcast_readiness_snapshot(",
    );
    assert!(
        watch.find("self.show_commands.observe(&snapshot)").unwrap()
            < watch
                .find("self.reproject_show_page_from_current_queue()")
                .unwrap()
    );
    let completion = source_between(&app, "fn finish_show_command(", "fn open_publisher_logs(");
    assert!(
        completion.find("self.show_commands.complete(").unwrap()
            < completion.find("self.settings_status =").unwrap()
    );
    for (start, end) in [
        (
            "fn run_publisher_service_command(",
            "fn finish_show_command(",
        ),
        (
            "fn run_stream_encoder_command(",
            "fn invalidate_publisher_service_snapshot(",
        ),
    ] {
        let command = source_between(&app, start, end);
        assert!(command.contains("let Some(command_id) = self.show_commands.begin_"));
        assert!(command.contains("this.finish_show_command(command_id, completion, cx)"));
        assert!(command.contains("ShowCommandCompletion::new(Err(error))"));
    }
    for (start, end) in [
        (
            "impl ApplicationCommand for PublisherServiceCommand",
            "struct ReadPublisherLogs",
        ),
        (
            "impl ApplicationCommand for StreamEncoderCommand",
            "fn selected_broadcast_host(",
        ),
    ] {
        assert!(
            source_between(&app, start, end)
                .split_whitespace()
                .collect::<String>()
                .contains("ShowCommandCompletion::new(result"),
            "ADR 0059: stamp command return in the worker, not the presentation callback"
        );
    }
}

/// Situational ADR 0059: Working uses typed unavailable stream controls, preserving the action row.
#[test]
fn adr_0059_stream_working_retains_typed_actions() {
    let vm = read_source(&manifest_path("src/view_models/show.rs"));
    let working = source_between(&vm, "fn mark_stream_working(", "fn with_command_state(");
    assert!(!working.contains("stream.actions = None"));
    assert!(working.contains("stream_actions(true, StreamConnectionState::Working)"));
}

/// Situational ADR 0065: all feed-check counts have a full-width row below the triggering control.
#[test]
fn adr_0065_feed_check_result_has_its_own_full_width_row() {
    let library = read_source(&manifest_path("src/library/app_impl.rs"));
    let section = source_between(
        &library,
        "let leading_pane = div()",
        ".id(chrome.list_scroll_id)",
    );
    let result_at = section.find(".when_some(feed_status,").unwrap();
    assert!(result_at > section.find("this.check_all_feeds(cx)").unwrap());
    assert!(result_at > section.find("this.apply_all_feed_updates(cx)").unwrap());
    let result = &section[result_at..];
    assert!(section.contains(".id(\"feed-update-section\")"));
    assert!(result.contains(".id(\"feed-update-result\")"));
    assert!(result.contains(".w_full()"));
    assert!(result.contains(".child(SharedString::from(msg))"));
    assert!(!result.contains(".truncate()"));
    assert!(!result.contains("UiButton"));
}

/// Situational ADR 0063: cards and compact items share one labeled badge; disclosure owns diagnostics.
#[test]
fn adr_0063_compact_items_share_badges_and_event_log_disclosure() {
    let badge = read_source(&manifest_path("src/ui/primitives/status_badge.rs"));
    for required in [
        ".child(SharedString::from(state_label))",
        "const fn state_badge_tokens(state: ShowCardStateKind)",
        "ShowCardStateKind::Ok => (SemanticColor::Success, SemanticColor::OnSuccess)",
        "ShowCardStateKind::Attention => (SemanticColor::Warning, SemanticColor::OnWarning)",
        "ShowCardStateKind::Failed => (SemanticColor::Danger, SemanticColor::OnDanger)",
        "ShowCardStateKind::Unknown => (SemanticColor::Info, SemanticColor::OnInfo)",
    ] {
        assert!(
            badge.contains(required),
            "ADR 0063: shared badge missing {required}"
        );
    }
    for file in [
        "src/ui/composites/show_card.rs",
        "src/ui/composites/show_detail_panel.rs",
    ] {
        let source = read_source(&manifest_path(file));
        assert!(source.contains("status_badge::render_state_badge"));
        assert!(!source.contains("fn state_badge_tokens"));
    }
    let panel = read_source(&manifest_path("src/ui/composites/show_detail_panel.rs"));
    let event = source_between(
        &panel,
        "fn render_event_detail(",
        "fn render_stream_detail(",
    );
    for required in [
        "section.badge",
        "section.activity",
        "section.picker",
        "section.primary",
        "section.logs",
        "section.overflow",
        "section.hint",
    ] {
        assert!(
            event
                .split_whitespace()
                .collect::<String>()
                .contains(required),
            "ADR 0063: compact event missing {required}"
        );
    }
    for forbidden in [
        "token_path",
        "registration_message",
        "check_message",
        "diagnostics",
        ".truncate()",
        "render_detail_row(",
    ] {
        assert!(
            !event.contains(forbidden),
            "ADR 0063: inline event detail contains {forbidden}"
        );
    }
    let app = read_source(&manifest_path("src/app/show.rs"));
    assert!(app.contains("if self.show_page.log_pane.shows_event()"));
    assert!(app.contains("self.show_page.log_pane.show_event(input)"));
    let vm = read_source(&manifest_path("src/view_models/show.rs"));
    let event_logs = source_between(&vm, "pub(crate) fn show_event(", "fn event_result_hint(");
    for required in [
        "self.request = None",
        "self.role = None",
        "self.event_source = Some",
        "self.unit_name = format!",
        "self.text = event_diagnostics(input)",
    ] {
        assert!(
            event_logs.contains(required),
            "ADR 0063: event source must move title and text together: {required}"
        );
    }
    let menu = read_source(&manifest_path("src/ui/primitives/context_menu.rs"));
    assert!(menu.contains(".max_h(Size::MenuRegular.scaled(cx))"));
    assert!(menu.contains(".overflow_y_scroll()"));
}

/// Situational ADR 0063: item activity keeps its width and cannot grow the compact header vertically.
#[test]
fn adr_0063_item_activity_keeps_its_width_and_single_line() {
    let panel = read_source(&manifest_path("src/ui/composites/show_detail_panel.rs"));
    let header = source_between(&panel, "fn render_item_header(", "fn compact_detail(");
    let activity = source_between(
        header,
        ".children(activity.map(",
        ".child(render_state_badge(",
    );
    for required in [".flex_shrink_0()", ".whitespace_nowrap()"] {
        assert!(
            activity.contains(required),
            "ADR 0063: item activity must resist letter-by-letter wrapping: {required}"
        );
    }
}

/// Situational ADR 0063: reports use recorded facts and preserve their plain text through the shared pane.
#[test]
fn adr_0063_event_reports_use_recorded_times_and_plain_text() {
    let report = read_source(&manifest_path("src/view_models/show/event_report.rs"));
    assert!(!report.contains("Utc::now()") && !report.contains("SystemTime::now()"));
    assert!(!report.contains("use gpui"));
    assert!(report.contains("at: DateTime<Utc>"));
    let render = source_between(&report, "pub(super) fn render(", "fn target_description(");
    assert!(render.contains("time(entry.at)"));
    assert!(!render.contains("EventCommandState::Idle"));
    let vm = read_source(&manifest_path("src/view_models/show.rs"));
    let diagnostics = source_between(&vm, "fn event_diagnostics(", "impl ShowLogPaneDisplay {");
    assert!(diagnostics.contains("event_report::render(input)"));
    assert!(vm.contains("self.text = event_diagnostics(input)"));
}

/// Situational ADR 0059: target mutation reserves Publisher using the existing bounded readback policy.
#[test]
fn adr_0059_event_target_commands_share_publisher_ownership() {
    let app = read_source(&manifest_path("src/app/show.rs"));
    let target = source_between(
        &app,
        "fn run_event_target_command(",
        "fn run_stream_encoder_command(",
    );
    assert!(target.contains("self.show_commands.begin_service("));
    assert!(target.contains("PublisherServiceRole::Publisher"));
    assert!(target.contains("input.feedback.target_mutation = EventCommandState::Working"));
    assert!(target.contains("input.targets = EventTargetListInput::Unknown"));
    let finish = source_between(&app, "fn finish_event_target(", "struct SelectShowEvent");
    assert!(finish.contains(".complete(command_id, completion.returned_at, true)"));
    assert!(finish.contains("self.invalidate_publisher_service_snapshot()"));
    assert!(finish.contains("self.read_event_targets(request, cx)"));
    let command = source_between(
        &app,
        "struct EventTargetCommand",
        "fn event_target_name_for_operation(",
    );
    assert!(command.contains("selection.revision != self.selection_revision"));
    assert!(command.contains("Target configuration saved; Publisher restart failed"));
}

#[test]
fn adr_0066_whole_config_adapter_callers_are_explicit() {
    // Situational: ADR 0066 task 004. The whole-config caller inventory is empty.
    let mut files = Vec::new();
    collect_rust_files(&manifest_path("src"), &mut files);
    for file in files {
        let source = read_source(&file);
        for forbidden in ["legacy_config(", "load_config(", "pub struct Config {"] {
            assert!(
                !source.contains(forbidden),
                "ADR 0066: whole-config adapter reintroduced in {}: {forbidden}",
                file.display()
            );
        }
    }
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    let preparation = source_between(
        &bootstrap,
        "pub(super) fn prepare_normal(",
        "pub(super) fn mount_normal",
    );
    assert!(!preparation.contains("ConfigSnapshot::read_existing"));
    assert!(!preparation.contains("load_musicindex_endpoint"));
    let cli = read_source(&manifest_path("src/cli.rs"));
    assert_eq!(
        production_source(&cli)
            .matches("load_config_snapshot(")
            .count(),
        1
    );
    assert!(cli.contains("OnceCell<Result<config::ConfigSnapshot>>"));
}

#[test]
fn adr_0066_optional_dependencies_are_scoped() {
    // Situational: ADR 0066 task 004. Behavioral proofs live beside the factory,
    // API, RSS service, Show VM and CLI; this guard keeps their live wiring.
    let startup = read_source(&manifest_path("src/startup.rs"));
    let preparation = source_between(
        &startup,
        "fn prepare_playback_with(",
        "pub(crate) fn prepare_local_paths(",
    );
    assert!(preparation.contains("snapshot.playback().ok()?"));
    assert!(preparation.contains("producer.prepare_directory()?"));
    assert!(preparation.contains("Dependency::Playback"));
    for forbidden in ["NullDriver::new", "Driver::Null", ".ping(", ".load("] {
        assert!(!preparation.contains(forbidden));
    }
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    assert!(bootstrap.contains("Option<Arc<Mutex<PlaybackOwner<ConfiguredPlaybackDriver>>>>"));
    assert!(!production_source(&bootstrap).contains("maybe_start_playback_polling"));
    // Situational ADR 0066: shared configuration reports must not also become Show status.
    let mount = source_between(
        &bootstrap,
        "pub(super) fn mount_normal",
        "pub(super) fn cache_worker_for_config",
    );
    assert!(mount.contains("normal_startup_status(&notices)"));
    assert!(!mount.contains("issue.cause"));
    let playback = read_source(&manifest_path("src/app/playback_bar.rs"));
    assert_eq!(
        playback
            .matches("let Some(owner) = self.available_playback_owner(cx)")
            .count(),
        4
    );
    assert!(playback.contains(".require(crate::application::capability::Dependency::Playback)"));
    assert!(playback.contains("this.maybe_start_playback_polling(cx)"));
    let keyboard = read_source(&manifest_path("src/app/keyboard.rs"));
    assert!(keyboard.contains("self.toggle_playback_paused(cx)"));
    let api = read_source(&manifest_path("src/api.rs"));
    assert!(api.contains("self.base_url.require()?"));
    let search = read_source(&manifest_path("src/app/search_dispatch.rs"));
    assert!(search.contains("AppToolbarVm::index_search_availability("));
    assert!(search.contains("search_local_library_tracks("));
    let show = read_source(&manifest_path("src/app/show.rs"));
    assert!(show.contains("broadcast_watch_inputs(&self.broadcast)"));
    assert!(show.contains(".with_feature_availability("));
    let rss = read_source(&manifest_path("src/rss/subscribe.rs"));
    assert!(rss.contains("(feed_guid.as_deref(), musicindex_endpoint.require())"));
    let cli = read_source(&manifest_path("src/cli.rs"));
    assert!(cli.contains("forget_local_event("));
    assert!(cli.contains("db::broadcast_events("));
}

#[test]
fn adr_0066_repair_failure_preserves_bindings() {
    // Situational: ADR 0066 task 004; retain ADR 0064 skip and binding safety.
    let startup = read_source(&manifest_path("src/startup.rs"));
    let repair = source_between(
        &startup,
        "pub(crate) fn prepare_local_paths(",
        "#[cfg(test)]",
    );
    for required in [
        "repair_local_file_paths(connection, music_dir)",
        "storage::check_music(music_dir, false, false)",
        "database::check_database(db_path)",
        "CoreCheckOutcome::blocked",
    ] {
        assert!(
            repair.contains(required),
            "missing repair containment: {required}"
        );
    }
    for forbidden in [
        "DELETE FROM",
        "UPDATE local_files",
        "prepare_database(",
        "create_dir",
    ] {
        assert!(!repair.contains(forbidden));
    }
    let database = read_source(&manifest_path("src/db.rs"));
    let binding = source_between(&database, "fn local_path_from_sql(", "\n}");
    assert!(binding.contains("LibraryRelativePath::from_stored"));
    assert!(binding.contains(".ok()"));
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    assert!(bootstrap.contains("prepare_local_paths("));
    assert!(bootstrap.contains("CapabilityFailure::PathRepair"));
    assert!(
        startup.contains("fn adr_0066_partial_repair_keeps_committed_and_unvalidated_bindings()")
    );
}
