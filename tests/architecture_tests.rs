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

const LIBRARY_REMOVAL_PRESENTATION_FILES: &[&str] = &["src/library.rs", "src/library/app_impl.rs"];

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
];

const RENDER_HELPER_DUPLICATION_BASELINES: &[RenderHelperDuplicationBaseline] = &[];

const PLAYLIST_POPOVER_CALLSITE_FILES: &[&str] = &[
    "src/library.rs",
    "src/ui/shells/track.rs",
    "src/ui/shells/library/feed_detail.rs",
    "src/ui/shells/library/track_detail_metadata.rs",
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
    "src/app/session.rs",
    "src/app/capabilities.rs",
    "src/app/settings.rs",
    "src/app/bootstrap.rs",
    "src/app/events.rs",
    "src/app/keyboard.rs",
    "src/app/menu.rs",
    "src/app/name_match_dispatch.rs",
    "src/app/playback_bar.rs",
    "src/app/publisher_dispatch.rs",
    "src/app/resize.rs",
    "src/app/search_dispatch.rs",
    "src/app/tab_bar.rs",
    "src/library.rs",
];

const SCREEN_SURFACE_DIRS: &[&str] = &["src/ui/shells/library"];

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

const PRESENTATION_GLUE_FILES: &[&str] = &[
    "src/app.rs",
    "src/app/playback_bar.rs",
    "src/app/queue_now_playing.rs",
    "src/app/show.rs",
    "src/app/tab_bar.rs",
    "src/library.rs",
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
    // ADR 0075 packet 050, operator decision D50-1: the feed's own
    // publication date, kept as two separate fact keys apart from the
    // oldest-item `release_date` fact above.
    "channel_pub_date",
    "feed_pub_date_claim",
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

/// R15-05 (ADR 0046 Task 015, ADR 0081 Decision 3): `go_forward` and
/// `FrameShellSlots::on_forward` must have a production caller, so neither
/// one stays behind a `#[cfg(test)]` block.
#[test]
fn adr_0046_forward_go_forward_and_on_forward_have_production_callers() {
    let nav_source = read_source(&manifest_path("src/view_models/workspace/nav.rs"));
    assert!(
        nav_source.contains("pub(crate) fn go_forward"),
        "ADR 0046 Task 015: src/view_models/workspace/nav.rs must declare go_forward"
    );
    assert!(
        !nav_source.contains("#[cfg(test)]"),
        "ADR 0046 Task 015: src/view_models/workspace/nav.rs must not gate go_forward behind #[cfg(test)]"
    );

    let mod_source = read_source(&manifest_path("src/view_models/workspace/mod.rs"));
    assert!(
        mod_source.contains("    CannotNavigateForward,")
            && !mod_source.contains("#[cfg(test)]\n    CannotNavigateForward,"),
        "ADR 0046 Task 015: WorkspaceModelError::CannotNavigateForward must not stay test-only"
    );

    let frame_shell_source = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let production_frame_shell = frame_shell_source
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(frame_shell_source.as_str());
    assert!(
        production_frame_shell.contains("pub(crate) fn on_forward"),
        "ADR 0046 Task 015: FrameShellSlots::on_forward must have a production caller outside #[cfg(test)]"
    );

    let workspace_shell_source = read_source(&manifest_path("src/ui/shells/workspace.rs"));
    for required in [
        "on_content_list_forward_select",
        "forward_select_handler_for",
        "shell_slots.on_forward(",
    ] {
        assert!(
            workspace_shell_source.contains(required),
            "ADR 0046 Task 015: src/ui/shells/workspace.rs must wire `{required}`"
        );
    }

    let app_source = read_source(&manifest_path("src/app.rs"));
    for required in [
        "fn handle_content_list_forward_select",
        "pop_nav_forward",
        "on_content_list_forward_select",
    ] {
        assert!(
            app_source.contains(required),
            "ADR 0046 Task 015: src/app.rs must wire `{required}`"
        );
    }
}

/// R15-02 and R15-06 (ADR 0046 Task 015): Forward must restore a page
/// through the same function calls Back uses, so a publisher page, a
/// name-match page, and every other restored page keep the one stale-result
/// check Back already proves.
#[test]
fn adr_0046_forward_restores_pages_through_the_shared_back_restore_path() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let back_body = source_between(
        &app_source,
        "fn handle_content_list_back_select(&mut self, cx: &mut Context<Self>) {",
        "fn handle_content_list_forward_select",
    );
    let forward_body = source_between(
        &app_source,
        "fn handle_content_list_forward_select(&mut self, cx: &mut Context<Self>) {",
        "fn restore_content_list_nav_entry(",
    );
    let shared_body = source_between(
        &app_source,
        "fn restore_content_list_nav_entry(",
        "pub(super) fn focus_global_search(",
    );

    assert!(
        back_body.contains("self.restore_content_list_nav_entry(content_list_id, &entry, cx)"),
        "ADR 0046 Task 015: Back must restore its entry through restore_content_list_nav_entry"
    );
    assert!(
        forward_body.contains("self.restore_content_list_nav_entry(content_list_id, &entry, cx)"),
        "ADR 0046 Task 015: Forward must restore its entry through the same restore_content_list_nav_entry Back uses"
    );

    for required in [
        "restore_publisher_page_for_nav",
        "restore_name_match_page_for_nav",
        "restore_index_feed_detail_for_nav",
        "restore_index_track_detail_for_nav",
        "hydrate_detail_from_nav",
    ] {
        assert!(
            shared_body.contains(required),
            "ADR 0046 Task 015: the shared restore path must call `{required}`"
        );
    }
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
        "fn begin_content_pane_resize(&mut self",
        "fn resize_content_pane(&mut self",
        "fn end_content_pane_resize(&mut self",
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

/// Situational ADR 0046: every Library pane uses the measured shared fitting owner.
#[test]
fn adr_0046_library_split_uses_measured_shared_geometry() {
    let source = read_source(&manifest_path("src/library/app_impl.rs"));
    let render = source_between(&source, "impl Render for LibraryApp", "#[cfg(test)]");
    let splits = render.split("SplitPane::new(chrome.split_pane_id)").skip(1);
    let mut count = 0;
    for split in splits {
        let wiring = split.split(".into_any_element()").next().unwrap();
        for required in [
            ".leading_width(px(self.vm.split_pane_width()))",
            ".fit_to(",
            "self.split_pane_bounds",
            ".stacked_height(self.vm.split_pane_height().map(px))",
            ".on_resize(",
            "layout::scaled_dimension(layout::CONTENT_PANE_MIN_WIDTH, cx)",
            ".on_layout(",
        ] {
            assert!(
                wiring.contains(required),
                "Library split missing {required}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 2, "Cover recent content and selected details");
    assert!(render.contains("this.split_pane_bounds != Some(*bounds)"));
    assert!(render.contains("this.split_pane_bounds = Some(*bounds)"));
    assert!(!render.contains("SPLIT_STACKED_LEADING_FRACTION"));
    assert!(!render.contains("SplitPaneAxis::Vertical"));
}

/// Situational ADR 0066: normal notices preserve content height and open Settings passively.
#[test]
fn adr_0066_normal_notice_uses_bounded_viewport_and_passive_settings_route() {
    let report = read_source(&manifest_path("src/ui/composites/startup_report.rs"));
    for required in [
        "CAPABILITY_NOTICE_MAX_HEIGHT",
        "capability-notice-scroll",
        ".overflow_y_scrollbar()",
        "vm.notice_summary()",
        "CapabilityAction::OpenReport",
    ] {
        assert!(
            report.contains(required),
            "Normal notice missing {required}"
        );
    }
    let adapter = read_source(&manifest_path("src/app/capabilities.rs"));
    let route = source_between(
        &adapter,
        "CapabilityAction::OpenReport | CapabilityAction::Review(_) =>",
        "CapabilityAction::Repair(id) =>",
    );
    assert!(route.contains("SettingsAction::OpenReport"));
    assert!(route.contains("select_tab(AppTab::Settings"));
    for forbidden in [
        "check_capability",
        "retry_intent",
        "present_command",
        "SaveCorrection",
    ] {
        assert!(
            !route.contains(forbidden),
            "Report navigation must not execute {forbidden}"
        );
    }
}

#[test]
fn adr_0066_missing_runtime_has_no_implicit_runner() {
    // Situational: ADR 0066 invariants 2, 5 and 9. Runner/worker tests prove
    // rejection and recovery; this inventory keeps every GUI entry on that path.
    for file in ["src/app.rs", "src/library/app_impl.rs"] {
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
    assert!(read_source(&manifest_path("src/app/settings.rs"))
        .contains("app.render_capabilities(true, cx)"));
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
    // receipt on the ADR 0074 Startup page, outside its content scroll body.
    let heading = source_between(&composite, "let mut heading", ".child(heading)");
    assert!(heading.contains(".feedback()"));
    assert!(heading.contains("navigation.page == RecoveryPage::Startup"));
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
    // ADR 0077 packet 006 deleted the second wiring site: the scoped Index
    // feed-results view. The name-match track page it replaced reports its
    // own failure through `NameMatchPageLoadDisplay`, not this action.
    assert_eq!(
        app.matches(".on_failure_action(").count(),
        1,
        "root search must wire diagnostic actions"
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
        assert!(save.contains("ConfigWriteLease::acquire(cfg_path)?"));
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
        "FrameNavigationEntry::IndexNameMatches(",
        "FrameNavigationEntry::IndexFeedDetail {",
        "FrameNavigationEntry::IndexTrackDetail {",
        "render_index_detail_display(",
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

    if !search_query_source.contains("INDEX_FEED_DETAIL") {
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

    // ADR 0075 packet 047 deleted `index_feed_detail` and
    // `index_track_detail`: an Index search row no longer retains a
    // fetched detail to project. `TopApp::index_feed_detail_display` and
    // `index_track_detail_display` (`src/app/search_dispatch.rs`) select
    // the detail-on-open fetch state instead.
    for forbidden in [
        "pub(crate) fn index_feed_detail(",
        "pub(crate) fn index_track_detail(",
    ] {
        if search_results_mod_source.contains(forbidden) {
            violations.push(format!(
                "src/view_models/search_results/mod.rs: ADR 0075 packet 047 removed the cached-row detail projection; found `{forbidden}`"
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
    // ADR 0077 packet 006 deleted `SearchResultsHeaderMode::Scoped`, the
    // old Index name-route feed results mode. It had no other caller.
    if search_results_shell_source.contains("SearchResultsHeaderMode::Scoped") {
        violations.push(
            "src/ui/shells/search_results_inspector.rs: ADR 0077 packet 006 retired SearchResultsHeaderMode::Scoped; it must not return"
                .to_string(),
        );
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

    // ADR 0075 packet 047 moved the Index track detail fetch out of the
    // search loop and into `FetchIndexTrackDetail`, sent only when the
    // operator opens the row. That command still attaches a rich
    // `TrackView` from the fetched `api::Track`, which is this guard's
    // ADR 0024 rule; only the call site moved.
    for required in [
        "TrackView::from_api(track)",
        "CommandOutcome::without_events(remote_track)",
    ] {
        if !search_query_source.contains(required) {
            violations.push(format!(
                "src/application/queries/search.rs: ADR 0024 Index track detail-on-open command must attach TrackView from fetched api::Track; missing `{required}`"
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
        "adr_0077_name_matches_index_row_opens_tracks_matching_not_artist_page",
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
    // ADR 0060 deleted the Discover compatibility module. This guard no
    // longer checks for its presence; `adr_0060_discover_surface_stays_deleted`
    // checks that it stays gone.

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
        ".push_nav(Self::content_frame_id(), entry)",
        "FrameNavigationEntry::PlaylistDetail(playlist_id)",
        "FrameNavigationEntry::TrackDetail(track.id)",
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
    let keyboard_source = read_source(&manifest_path("src/app/keyboard.rs"));
    assert!(
        keyboard_source.contains("platform_keystroke(self.keystroke, is_macos)"),
        "Situational ADR 0067: src/app/keyboard.rs must resolve its registry through the shared platform adapter"
    );
    assert!(
        !keyboard_source.contains("KeyBinding::new(self.keystroke,"),
        "Situational ADR 0067: src/app/keyboard.rs must not bypass platform resolution"
    );

    // ADR 0046 Task 015 gave src/app/menu.rs a second entry shape,
    // `AppMenuKeystroke::PerPlatform`, beside the original `Shared` shape, so
    // one entry can carry a different keystroke and context for macOS and
    // for every other platform (the Back and Forward binds). The `Shared`
    // shape still resolves every original entry through the one shared
    // platform adapter.
    let menu_source = read_source(&manifest_path("src/app/menu.rs"));
    assert!(
        menu_source.contains("platform_keystroke(keystroke, is_macos)"),
        "Situational ADR 0067: src/app/menu.rs must resolve its Shared entries through the shared platform adapter"
    );
    assert!(
        !menu_source.contains("KeyBinding::new(self.keystroke,"),
        "Situational ADR 0067: src/app/menu.rs must not bypass platform resolution"
    );
}

#[test]
fn adr_0067_mount_and_section_changes_establish_a_persistent_focus_path() {
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    let app = read_source(&manifest_path("src/app.rs"));
    let tabs = read_source(&manifest_path("src/app/tab_bar.rs"));
    assert!(bootstrap.contains("app.focus_active_tab(window, cx);"),
        "Situational ADR 0067: normal mount must focus a control beneath the app action handlers before the first shortcut");
    assert!(app.contains("self.tab = tab;\n        self.focus_active_tab(window, cx);"),
        "Situational ADR 0067: every section transition must replace focus from an input that may leave the mounted tree");
    assert!(tabs.contains(".track_focus(focus_handle)")
        && tabs.contains("focus_handle_for_key(key, self).focus(window, cx);"),
        "Situational ADR 0067: transition focus must use the persistent handles mounted by the shared tab bar");
    let render = app
        .split_once("impl Render for TopApp")
        .expect("app renderer")
        .1;
    assert!(
        !render.contains("focus_active_tab("),
        "Situational ADR 0067: rendering must not repeatedly steal focus from inputs"
    );
}

#[test]
fn adr_0040_settings_cache_reads_leave_the_render_thread() {
    let app = read_source(&manifest_path("src/app.rs"));
    let settings = read_source(&manifest_path("src/app/settings.rs"));
    let render = settings
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
    assert!(
        select.contains("self.cached_files.enter();")
            && select.contains("self.start_cached_load(cx);")
    );
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
        "AppMenuKeystroke::Shared(\"cmd-,\")",
        "AppMenuKeystroke::Shared(\"cmd-h\")",
        "AppMenuKeystroke::Shared(\"cmd-alt-h\")",
        "AppMenuKeystroke::Shared(\"cmd-q\")",
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

    // ADR 0060 deleted `src/discover/app_impl.rs`,
    // `src/ui/shells/discover/search_input.rs`, and the parked query layer
    // and view models of `src/application/queries/search.rs` and
    // `src/view_models/search/`. This guard no longer reads those paths;
    // `adr_0060_discover_surface_stays_deleted` checks that they stay gone.

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
fn settings_form_inputs_fill_scaled_frame_width() {
    // Situational ADR 0069: controls scale and fill the available Settings width.
    let form = read_source(&manifest_path("src/ui/composites/settings.rs"));
    let screen = read_source(&manifest_path("src/app/settings.rs"));
    for required in [
        "pub(crate) fn settings_text_input(",
        "div()\n        .w_full()\n        .min_w_0()\n        .flex()\n        .flex_row()",
        "Input::new(input)",
        ".scaled(Size::Small, cx)\n                .flex_1()\n                .min_w_0()",
    ] {
        assert!(
            form.contains(required),
            "shared Settings form contract missing {required}"
        );
    }
    assert!(screen.contains("settings_text_input(&app.endpoint_input, cx)"));
    // ADR 0066 tasks 006/008 route core/converter paths through the shared
    // maintenance editor. Preserve width/scale proof at that active owner.
    assert!(screen.contains("settings_actions(SettingsVm::converter_setup(), &callback, cx)"));
    let maintenance = read_source(&manifest_path("src/ui/composites/maintenance_forms.rs"));
    let input = source_between(
        &maintenance,
        "fn configuration_input_frame(",
        "fn correction_button(",
    );
    for required in [
        ".input_text_size(",
        ".inset_0()",
        ".w_auto()",
        ".min_w_0()",
        "gpui::AlignItems::Stretch",
    ] {
        assert!(
            input.contains(required),
            "ADR 0066 shared editor width/scale: {required}"
        );
    }
    assert!(screen.contains("settings_message(app.music_dir.display().to_string(), cx)"));
    assert!(!form.contains("SETTINGS_COLUMN_WIDTH") && !form.contains("settings_column_width"));
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
        // ADR 0039: renamed from the retired uniform `.multiplier()` to the
        // named CHROME resolver; geometry never reads the TYPE domain.
        "ScaleFactor::current(cx).chrome_multiplier()",
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

    for file in ["src/ui/control_styles.rs"] {
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

    for file in ["src/library.rs"] {
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

/// Durable token discipline (ADR 0023): production screens use named geometry and colors.
#[test]
fn screens_do_not_reintroduce_raw_color_or_numeric_px_literals() {
    let mut violations = Vec::new();
    for file_name in screen_enforcement_files() {
        let file = file_name.as_str();
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(production_source(&source)) {
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
    // Situational ADR 0034: enforce rendered code; test coordinates are permitted.
    let mut violations = Vec::new();
    for path in rust_files_under("src/ui/composites") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(production_source(&source)) {
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
    let frame_shell = read_source(&manifest_path("src/ui/composites/frame_shell.rs"));
    let entity_shell = read_source(&manifest_path("src/ui/shells/entity.rs"));
    let library_metadata = read_source(&manifest_path(
        "src/ui/shells/library/track_detail_metadata.rs",
    ));
    // ADR 0060 deleted the Discover screen. This guard no longer checks the
    // `skeleton_feed_tile` and `MusicBrainzPanel` pairings that named its
    // files; `adr_0060_discover_surface_stays_deleted` checks that those
    // files stay gone.

    assert!(
        frame_shell.contains("BreadcrumbTrail::new(breadcrumb)")
            && read_source(&manifest_path("src/ui/shells/library/track_detail.rs"))
                .contains("BreadcrumbTrail::new(breadcrumb)"),
        "BreadcrumbTrail must keep both frame-shell and Library track-detail callers"
    );
    assert!(
        library_metadata.contains("MusicBrainzPanel::new(vm)"),
        "MusicBrainzPanel must keep its Library metadata caller"
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

/// Situational ADR 0079 and ADR 0077 Decision 7: the MusicIndex artist subject
/// storage and the ADR 0045 track artist binding stay deleted. Delete this guard
/// when ADR 0079 is superseded.
///
/// The schema history in `src/db.rs` keeps the table names: the migration
/// registry, the frozen version-11 contract, the migration 13 table list, and
/// the DDL of migrations 4 and 5. Unit test modules are not production code.
#[test]
fn adr_0079_removed_artist_storage_stays_deleted() {
    const FIX: &str = "ADR 0079: MusicIndex artist subject storage is deleted. Use ArtistRef::LocalArtistName for a name grouping, or the ADR 0077 publisher feed GUID for an artist identity.";
    let forbidden = [
        "track_artist_source_bindings",
        "artist_source_ids",
        "artist_source_links",
        "artist_source_facts",
        "ArtistRef::Musicindex",
        "persist_musicindex_artist",
        "source_subjects",
    ];
    let schema_history = [
        (
            "const MIGRATIONS: &[Migration] = &[",
            "/// Read compatibility facts",
        ),
        (
            "const VERSION_11_COLUMNS",
            "pub(crate) fn migrate_schema_to(",
        ),
        (
            "fn migration_artist_source_facts(",
            "fn migration_cleanup_placeholder_source_text(",
        ),
        (
            "fn create_artist_source_fact_tables(",
            "fn create_broadcast_event_tables(",
        ),
    ];
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        if path.file_name().is_some_and(|name| name == "tests.rs") {
            continue;
        }
        let file = rel_path(&path);
        let mut source = without_unit_test_module(&path, &read_source(&path));
        if file == "src/db.rs" {
            for (start, end) in schema_history {
                let span = source_between(&source, start, end).to_owned();
                let blank = span
                    .chars()
                    .map(|ch| if ch == '\n' { ch } else { ' ' })
                    .collect::<String>();
                source = source.replacen(&span, &blank, 1);
            }
        }
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: names `{pattern}`: `{line}`\n  {FIX}"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0079 and ADR 0077 Decision 7 removed artist storage violations:\n{}",
        violations.join("\n")
    );
}

/// Returns the line number of each `dead_code` lint attribute in `source`.
///
/// This finds every `#[...]` and `#![...]` attribute by matching brackets,
/// not by scanning one line at a time, so it catches a marker that spans
/// several lines, for example:
///
/// ```text
/// #[cfg_attr(
///     not(test),
///     expect(dead_code, reason = "...")
/// )]
/// ```
///
/// This one scan catches every current form: `#[allow(dead_code)]`,
/// `#[expect(dead_code, reason = "...")]`, `#![expect(dead_code, ...)]` at
/// the top of a module, and `#[cfg_attr(not(test), expect(dead_code, ...))]`.
fn dead_code_attribute_lines(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let mut found_at = Vec::new();
    let mut line_number = 1;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\n' {
            line_number += 1;
            index += 1;
            continue;
        }
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }
        let attr_start_line = line_number;
        let mut bracket_index = index + 1;
        if bytes.get(bracket_index) == Some(&b'!') {
            bracket_index += 1;
        }
        if bytes.get(bracket_index) != Some(&b'[') {
            index += 1;
            continue;
        }
        let mut depth = 0usize;
        let mut scan = bracket_index;
        let mut attr_end = None;
        while scan < bytes.len() {
            match bytes[scan] {
                b'[' => depth += 1,
                b']' => {
                    depth -= 1;
                    if depth == 0 {
                        attr_end = Some(scan);
                        break;
                    }
                }
                _ => {}
            }
            scan += 1;
        }
        let Some(end) = attr_end else {
            index += 1;
            continue;
        };
        let attribute_text = &source[index..=end];
        if attribute_text.contains("dead_code") {
            found_at.push(attr_start_line);
        }
        line_number += attribute_text.matches('\n').count();
        index = end + 1;
    }
    found_at
}

/// Situational ADR 0060, packets 005 and 006, and dead-code-removal task 002:
/// the parked Discover screen (`SearchApp`, `src/discover.rs`,
/// `src/discover/`, and `src/ui/shells/discover/`) stays deleted, and no
/// file in `src/` carries a `dead_code` lint attribute in any form. Delete
/// this guard if ADR 0060 is superseded.
const ADR_0060_DEAD_CODE_FIX: &str = "ADR 0060: a file in src/ must not carry a dead_code lint attribute, in any form. Delete the unreachable item, or move a test-only item into a #[cfg(test)] module.";

#[test]
fn adr_0060_discover_surface_stays_deleted() {
    const FIX: &str = "ADR 0060: Music replaced the Discover surface, and the parked screen is deleted. Build the surface from the live Music view models under src/view_models/search_results/, not a revived SearchApp.";
    let mut violations = Vec::new();

    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("struct SearchApp") {
                violations.push(format!(
                    "{file}:{line_number}: a SearchApp item returned: `{line}`\n  {FIX}"
                ));
            }
            if line.contains("mod discover") {
                violations.push(format!(
                    "{file}:{line_number}: a discover module returned: `{line}`\n  {FIX}"
                ));
            }
        }
        for line_number in dead_code_attribute_lines(&source) {
            violations.push(format!(
                "{file}:{line_number}: a dead_code lint attribute returned\n  {ADR_0060_DEAD_CODE_FIX}"
            ));
        }
    }

    if manifest_path("src/ui/shells/discover").is_dir() {
        violations.push(format!("src/ui/shells/discover/ exists.\n  {FIX}"));
    }

    assert!(
        violations.is_empty(),
        "ADR 0060 Discover surface deletion violations:\n{}",
        violations.join("\n")
    );
}

/// RDC2-03: `adr_0060_discover_surface_stays_deleted` must fail on every
/// current `dead_code` lint attribute form, and its fix message must name
/// ADR 0060 and the fix. These sample sources are strings, not files under
/// `src/`, so they prove the detection logic without writing a throwaway
/// file into the tree.
#[test]
fn adr_0060_discover_surface_guard_catches_every_dead_code_attribute_form() {
    let single_line_expect = r#"
pub(crate) struct FrameShellSlots {
    on_forward: Option<FrameButtonHandler>,
}

impl FrameShellSlots {
    #[expect(dead_code, reason = "deferred frame action wiring consumes this slot")]
    pub(crate) fn on_forward(mut self, handler: Handler) -> Self {
        self.on_forward = Some(handler);
        self
    }
}
"#;
    assert_eq!(
        dead_code_attribute_lines(single_line_expect),
        vec![7],
        "a single-line #[expect(dead_code, ...)] must be caught"
    );

    let whole_module_expect = r#"
#![warn(clippy::pedantic)]
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "workspace contracts land before every frame action is wired"
    )
)]

use super::WorkspaceModelError;
"#;
    assert_eq!(
        dead_code_attribute_lines(whole_module_expect),
        vec![3],
        "a whole-module #![expect(dead_code, ...)] spanning several lines must be caught"
    );

    let cfg_attr_expect = r#"
impl ContentListPageVm {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "tested in library_view_model_content_text_filter_does_not_filter_source_tree"
        )
    )]
    pub(crate) fn content_text_filter(&self) -> Option<&str> {
        self.content_list_page.text_filter()
    }
}
"#;
    assert_eq!(
        dead_code_attribute_lines(cfg_attr_expect),
        vec![3],
        "a #[cfg_attr(not(test), expect(dead_code, ...))] spanning several lines must be caught"
    );

    let clean_source = r#"
impl ContentListPageVm {
    #[cfg(test)]
    pub(crate) fn text_filter(&self) -> Option<&str> {
        self.text_filter.as_deref()
    }
}
"#;
    assert!(
        dead_code_attribute_lines(clean_source).is_empty(),
        "a #[cfg(test)] attribute with no dead_code marker must not be flagged"
    );

    assert!(
        ADR_0060_DEAD_CODE_FIX.contains("ADR 0060"),
        "the dead_code fix message must name ADR 0060"
    );
}

/// Source text before the `#[cfg(test)] mod name {` block at the end of a
/// file. A file that only a test build compiles has no production text: its
/// parent module marks it test-only. ADR 0076 Decision 9 (packet 007).
fn without_unit_test_module(path: &Path, source: &str) -> String {
    if is_test_only_source_file(path) {
        return String::new();
    }
    let lines = source.lines().collect::<Vec<_>>();
    let end = lines
        .windows(2)
        .position(|pair| {
            pair[0].trim() == "#[cfg(test)]"
                && pair[1].trim_start().starts_with("mod ")
                && pair[1].trim_end().ends_with('{')
        })
        .unwrap_or(lines.len());
    lines[..end].join("\n")
}

/// True when a test build is the only build that compiles the file at
/// `path`. The parent module file declares such a file with `#[cfg(test)]`
/// on the line before `mod <name>;`. ADR 0076 packet 007.
///
/// The parent is `<dir>.rs`, next to the directory that holds `path`, for a
/// screen module that keeps its own test file in a sibling directory. The
/// parent is `mod.rs` or `lib.rs` in the same directory as `path` otherwise,
/// for a file such as `src/view_models/workspace/tests.rs` under
/// `src/view_models/workspace/mod.rs`.
fn is_test_only_source_file(path: &Path) -> bool {
    let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    let Some(dir) = path.parent() else {
        return false;
    };
    let mut parents = vec![dir.join("mod.rs"), dir.join("lib.rs")];
    if let Some(dir_name) = dir.file_name().and_then(|value| value.to_str()) {
        if let Some(sibling_dir) = dir.parent() {
            parents.push(sibling_dir.join(format!("{dir_name}.rs")));
        }
    }
    parents
        .into_iter()
        .filter(|parent| parent.is_file())
        .any(|parent| module_declares_test_only_child(&read_source(&parent), name))
}

/// True when `source` declares `mod <name>;` on the line after
/// `#[cfg(test)]`.
fn module_declares_test_only_child(source: &str, name: &str) -> bool {
    let lines = source.lines().collect::<Vec<_>>();
    lines
        .windows(2)
        .any(|pair| pair[0].trim() == "#[cfg(test)]" && line_declares_mod(pair[1], name))
}

/// True when `line` reads `mod <name>;`. An optional leading visibility
/// keyword, such as `pub` or `pub(crate)`, may come before `mod`.
fn line_declares_mod(line: &str, name: &str) -> bool {
    let Some(rest) = line.trim().strip_suffix(';') else {
        return false;
    };
    match rest.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["mod", declared] => *declared == name,
        [visibility, "mod", declared] if visibility.starts_with("pub") => *declared == name,
        _ => false,
    }
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
fn screen_entry_modules_under_500_loc() {
    let ceilings = [("src/library.rs", 500)];
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

    for file in ["src/library.rs"] {
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

    // ADR 0060 deleted `src/ui/shells/discover/track_inspector.rs`. This
    // guard no longer checks its Nostr button pattern;
    // `adr_0060_discover_surface_stays_deleted` checks that the file stays
    // gone.

    let library = read_source(&manifest_path("src/ui/shells/library/track_detail.rs"));
    if !library.contains("render_track_page_identity_actions(&detail_page)")
        || library.contains("\"library-track\"")
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

    for file in ["src/library.rs"] {
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
    // ADR 0060 deleted `src/discover.rs` and its `TrackRow::from_vm(` caller
    // in `src/ui/shells/track.rs`. Neither pattern occurs in live code
    // anymore, so this guard now checks only the Library consumer.
    let consumers = [(
        "src/library.rs",
        "TrackDetailSurface::new(",
        "TrackDetailVm::new(",
    )];
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
            "Library track detail",
            "src/ui/shells/library/track_detail.rs",
            "TrackDetailVm::new(",
            ".page()",
            "track::build_track_detail_surface(",
        ),
        (
            "Library artist detail",
            "src/ui/shells/library/feed_list.rs",
            "LibraryArtistDetailVm::new(",
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

    for file in ["src/library.rs"] {
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
    // ADR 0060 deleted `src/ui/shells/feed.rs`. This guard now checks only
    // the Library consumer.
    let consumers = [(
        "src/library.rs",
        "render_release_detail_shell(",
        "ReleaseDetailVm::new(",
    )];
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
fn view_models_own_display_fallbacks_for_library() {
    let forbidden = [
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
            "src/library.rs",
            ".or(row.id3_value.as_deref())",
            "metadata ID3 cell value fallback belongs in TrackMetadataGridVm::id3_cell_value",
        ),
        (
            "src/library.rs",
            ".or(row.id3_frame.as_deref())",
            "metadata ID3 cell frame fallback belongs in TrackMetadataGridVm::id3_cell_frame",
        ),
        (
            "src/library.rs",
            ".child(SharedString::from(row.field.clone()))",
            "Library metadata field label display belongs in TrackMetadataGridVm::field_label",
        ),
        (
            "src/library.rs",
            "label: SharedString::from(frame_id.to_string())",
            "Library metadata ID3 frame label display belongs in TrackMetadataGridVm::id3_frame_display_label",
        ),
        (
            "src/library.rs",
            "fn id3_frame_color(frame_id: &str)",
            "Library metadata ID3 frame color role belongs in TrackMetadataGridVm::id3_frame_color_role",
        ),
        (
            "src/library.rs",
            "expanded_metadata_display_string(",
            "Library expanded metadata raw/display selection belongs in TrackMetadataGridVm::expanded_display_value",
        ),
        (
            "src/library.rs",
            "SharedString::from(display_value.to_string())",
            "Library metadata text display values belong in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/library.rs",
            "value: SharedString::from(value.to_string())",
            "Library metadata text value projection belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/library.rs",
            "MultilineText::new(raw_value.to_string())",
            "Library expanded metadata raw fallback belongs in TrackMetadataGridVm::text_value_display",
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
            "src/library.rs",
            "disclosure_id.to_string()",
            "Library metadata disclosure id binding should not re-project VM display ids",
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
            "src/ui/shells/track.rs",
            "playlist.name.clone()",
            "Track shell playlist popover option display belongs in playlist_option_displays",
        ),
        (
            "src/library.rs",
            "summarize_contributor_value(raw_value).unwrap_or_else",
            "metadata contributor summary fallback belongs in TrackMetadataGridVm::contributor_summary",
        ),
        (
            "src/library.rs",
            "format!(\"[{} items]\", arr.len())",
            "metadata value-route summary belongs in TrackMetadataGridVm::value_routes_summary",
        ),
        (
            "src/library.rs",
            "raw_value.starts_with(\"http://\") || raw_value.starts_with(\"https://\")",
            "metadata artwork URL summary policy belongs in TrackMetadataGridVm::expandable_cell_summary",
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
            "src/library.rs",
            "ActionRowMessageDisplay {",
            "Library action-row message tone/width belongs in VM display contracts",
        ),
        (
            "src/library.rs",
            "ActionRowMessageTone::",
            "Library action-row message tone belongs in VM display contracts",
        ),
        (
            "src/library.rs",
            "message_is_error()",
            "Library subscription message severity belongs in LibraryTrackActionVm::subscription_message_display",
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
            "src/library.rs",
            "logical_field == \"Value Routes\"",
            "Library expanded metadata field kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/library.rs",
            "field == \"Artwork\"",
            "Library expanded metadata artwork kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/library.rs",
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
            "src/library.rs",
            "fn route_value_label(",
            "metadata value-route field value display belongs in TrackMetadataGridVm::value_route_field_value_label",
        ),
        (
            "src/library.rs",
            "row.musicbrainz_value.as_deref().unwrap_or(\"\")",
            "metadata MusicBrainz cell value fallback belongs in TrackMetadataGridVm::musicbrainz_cell_value",
        ),
        (
            "src/library.rs",
            "fn comparison_status_role(",
            "metadata comparison role display belongs in TrackMetadataGridVm::comparison_role",
        ),
        (
            "src/library.rs",
            "fn comparison_status_glyph(",
            "metadata comparison glyph display belongs in TrackMetadataGridVm::comparison_glyph",
        ),
        (
            "src/library.rs",
            "fn display_with_glyph(",
            "metadata glyph-prefix display belongs in TrackMetadataGridVm::display_with_glyph",
        ),
        (
            "src/library.rs",
            "fn pending_source_role(",
            "metadata pending-source role display belongs in TrackMetadataGridVm::pending_source_role",
        ),
        (
            "src/library.rs",
            "row.id3_value.is_some() && row.rss_value.is_none() && row.musicbrainz_value.is_none()",
            "metadata standalone-ID3 status fallback belongs in TrackMetadataGridVm::id3_status_role",
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
            "src/ui/shells/entity.rs",
            "id_prefix: &str",
            "Feed identity action rendering should consume ReleaseDetailPageVm identity prefix",
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
            "src/library.rs",
            "let glyph = if expanded",
            "Library metadata disclosure glyph belongs in TrackMetadataGridVm expandable display contracts",
        ),
        (
            "src/library.rs",
            "let sub_glyph = if sub_expanded",
            "Library value-route disclosure glyph belongs in TrackMetadataGridVm value-route item display",
        ),
        (
            "src/library.rs",
            "display.cell_key.clone()",
            "Library metadata expansion keys should be consumed by destructuring TrackMetadataExpandableCellDisplay",
        ),
        (
            "src/library.rs",
            "display.item_key.clone()",
            "Library Value Routes item keys should be consumed by destructuring TrackMetadataValueRouteItemDisplay",
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
            "src/library.rs",
            "format!(\"Error applying ID3 edits: {error}\")",
            "Library ID3 apply error message belongs in TrackMetadataActionState",
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
        "ADR 0038 Library VM fallback ownership violations:\n{}",
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
        ".fetch_feed_track_with_profile(feed_guid, &track.item_guid, &LIBRARY_TRACK_DETAIL_SCOPED_TRACK)",
        "crate::subscribe_service::enrich_track_context_from_rss_with_recorder(&mut context, recorder)?;",
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
            compact_source(&feed_service_source).replace(",)", ")").contains(&compact_source(required)),
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
    // ADR 0075 packet 020: the subscribe command writes no MusicIndex feed
    // description, so it cannot persist a placeholder one.
    assert!(
        !production_source(&rss_subscribe_source).contains("set_feed_description("),
        "RSS subscribe must not persist a MusicIndex feed description. ADR 0076 Decision 5 and ADR 0075 packet 020: a subscribe writes the RSS value and hold of each compared slot. Remove the MusicIndex description write from src/rss/subscribe.rs."
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
    // AGENTS.md wraps its prose, so this rule can break across two lines.
    // Compare the text with its whitespace collapsed.
    let agent_text = agent_source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        agent_text.contains("Placeholder-looking source text is a source-boundary problem"),
        "ADR 0075 Decision I: AGENTS.md must keep the source-boundary placeholder rule. \
Restore that sentence in the `Provenance first` paragraph of AGENTS.md."
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
fn adr_0075_rss_observation_parser_boundary_is_guarded() {
    let rss_enrich_source = read_source(&manifest_path("src/rss/enrich.rs"));
    let rss_identity_source = read_source(&manifest_path("src/rss/identity.rs"));
    let production = rss_enrich_source.split("#[cfg(test)]").next().unwrap();
    assert_eq!(
        production.matches("Document::parse(").count(),
        1,
        "ADR 0075 enrichment must use one full DOM parser"
    );
    let enrichment = production
        .split("pub fn fetch_track_enrichment_from_feed")
        .nth(1)
        .unwrap();
    assert!(
        !enrichment.contains("Channel::read_from"),
        "ADR 0075 enrichment must not run a second full RSS parser"
    );
    let materialization = read_source(&manifest_path("src/subscribe_service/materialization.rs"));
    let retry = materialization.split("#[cfg(test)]").next().unwrap();
    assert!(
        !retry.contains("enrich_track") && !retry.contains("fetch_track_enrichment"),
        "ADR 0075 retries must retain the original RSS observation without another fetch"
    );
    for required in [
        "Document::parse(decoded.text())",
        "contracts::decode_rss_body(",
        "RssObservation",
        "RssTxtEvidence",
        "podcast_namespace(node.tag_name().namespace())",
        "context.rss_observation = Some(Arc::clone(&result.observation));",
        "RssTxtValidation::ValidPublicKey",
        "RssTxtValidation::ValidProfile",
    ] {
        assert!(
            rss_enrich_source.contains(required),
            "ADR 0075 RSS evidence must stay in the DOM parser boundary: `{required}`"
        );
    }
    for forbidden in ["nostr_from_extension", "extract_nostr_handle"] {
        assert!(
            !rss_enrich_source.contains(forbidden),
            "ADR 0075 must not restore recursive RSS identity extraction: `{forbidden}`"
        );
    }
    assert!(
        rss_identity_source.contains("fn decode_bech32"),
        "ADR 0075 must retain narrow NIP-19 validation"
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
    ];
    let files = ["src/library.rs"];
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

/// Drops whole-line `//` comments so a substring guard checks actual code,
/// not an explanatory comment that happens to mention the same call syntax.
/// Line-trailing comments are left as-is; none of this file's guards need
/// that finer granularity.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
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

/// Situational ADR 0057: current and archived decisions use one status format.
#[test]
fn adr_0057_status_headers_are_canonical() {
    let mut paths = Vec::new();
    for directory in ["docs/adr", "docs/adr/archive"] {
        let entries = fs::read_dir(manifest_path(directory))
            .unwrap_or_else(|error| panic!("read {directory}: {error}"));
        for entry in entries {
            let path = entry.expect("read ADR directory entry").path();
            let numbered = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.split_once('-'))
                .is_some_and(|(number, _)| {
                    number.len() == 4 && number.bytes().all(|byte| byte.is_ascii_digit())
                });
            if numbered && path.extension().is_some_and(|extension| extension == "md") {
                paths.push(path);
            }
        }
    }
    assert!(!paths.is_empty(), "ADR 0057: no numbered decisions found");
    paths.sort();
    let violations: Vec<_> = paths
        .iter()
        .filter_map(|path| {
            validate_adr_status_header(&read_source(path))
                .err()
                .map(|reason| format!("{}: {reason}", rel_path(path)))
        })
        .collect();
    assert!(
        violations.is_empty(),
        "Situational ADR 0057 status header violations:\n{}",
        violations.join("\n")
    );
}

fn validate_adr_status_header(source: &str) -> Result<(), &'static str> {
    let mut lines = source.lines();
    if !lines.any(|line| line == "## Status") {
        return Err("missing ## Status heading");
    }
    if lines.next() != Some("") {
        return Err("put one blank line after ## Status");
    }
    let (status, dated_sentence) = lines
        .next()
        .and_then(|line| line.split_once(" - "))
        .ok_or("follow the blank line with a status and date separated by ' - '")?;
    let superseded = status
        .strip_prefix("Superseded by ADR ")
        .is_some_and(|number| {
            number.len() == 4 && number.bytes().all(|byte| byte.is_ascii_digit())
        });
    if !matches!(status, "Proposed" | "Accepted" | "Implemented") && !superseded {
        return Err("use Proposed, Accepted, Implemented or Superseded by ADR NNNN");
    }
    let (date, explanation) = dated_sentence
        .split_once('.')
        .ok_or("end the dated status sentence with a period")?;
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| "use a valid calendar date in YYYY-MM-DD format")?;
    if parsed.format("%Y-%m-%d").to_string() != date {
        return Err("use a valid calendar date in YYYY-MM-DD format");
    }
    if !explanation.is_empty() && !explanation.starts_with(' ') {
        return Err("separate any explanatory prose from the status sentence with a space");
    }
    Ok(())
}

/// Situational ADR 0057: the corpus validator rejects the known drift classes.
#[test]
fn adr_0057_status_header_validation_covers_vocabulary_and_dates() {
    for sentence in [
        "Proposed - 2026-09-11.",
        "Accepted - 2026-09-11. Implementation partial: operator check open.",
        "Implemented - 2024-02-29.",
        "Superseded by ADR 0059 - 2026-09-06.",
    ] {
        let source = format!("# ADR\n\n## Status\n\n{sentence}\n\n## Context\n");
        assert_eq!(validate_adr_status_header(&source), Ok(()), "{sentence}");
    }
    for source in [
        "# ADR\n\n## Context\nAccepted - 2026-09-11.",
        "## Status\nAccepted - 2026-09-11.",
        "## Status\n\n\nAccepted - 2026-09-11.",
        "## Status\n\nAccepted and implemented - 2026-09-11.",
        "## Status\n\nImplemented for ADR 0047 scope - 2026-09-11.",
        "## Status\n\nSuperseded - 2026-09-11.",
        "## Status\n\nSuperseded by ADR 59 - 2026-09-11.",
        "## Status\n\nAccepted - 2026-02-29.",
        "## Status\n\nAccepted - 2026-9-11.",
        "## Status\n\nAccepted - 2026-09-11",
        "## Status\n\nAccepted - 2026-09-11.No space.",
    ] {
        assert!(validate_adr_status_header(source).is_err(), "{source}");
    }
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

/// ADR 0077 packet 006: an Index name candidate opens a search result — the
/// "Tracks matching" page — never a detail page titled "Artist". This test
/// replaces `index_artist_activation_is_scoped_feed_route_not_detail_page`,
/// which named the retired scoped Index feed-results route.
#[test]
fn adr_0077_name_matches_index_row_opens_tracks_matching_not_artist_page() {
    let app_source = read_source(&manifest_path("src/app.rs"));
    let search_dispatch_source = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let name_match_dispatch_source = read_source(&manifest_path("src/app/name_match_dispatch.rs"));
    let workspace_nav_source = read_source(&manifest_path("src/view_models/workspace/nav.rs"));
    let workspace_breadcrumb_source =
        read_source(&manifest_path("src/view_models/workspace/breadcrumb.rs"));
    let workspace_tests_source = read_source(&manifest_path("src/view_models/workspace/tests.rs"));
    let library_app_source = read_source(&manifest_path("src/library/app_impl.rs"));
    let index_detail_source = read_source(&manifest_path(
        "src/view_models/search_results/index_detail.rs",
    ));
    let retired_artist_detail_route_name = concat!("IndexArtist", "Detail");
    let retired_feed_scope_route_name = concat!("IndexArtist", "FeedScope");

    for (path, source) in [
        ("src/app.rs", app_source.as_str()),
        (
            "src/app/search_dispatch.rs",
            search_dispatch_source.as_str(),
        ),
        (
            "src/app/name_match_dispatch.rs",
            name_match_dispatch_source.as_str(),
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
            "ADR 0077 packet 006: {path} must not reintroduce a detail-page artist route for an Index name candidate"
        );
        assert!(
            !source.contains(retired_feed_scope_route_name),
            "ADR 0077 packet 006: {path} must not reintroduce the retired scoped feed-results route"
        );
    }

    assert!(
        workspace_nav_source.contains("IndexNameMatches(String)"),
        "ADR 0077 packet 006: the navigation entry must be named for a name match, not an artist"
    );
    assert!(
        search_dispatch_source
            .contains("self.open_name_match_page(artist_name.to_string(), content_frame_id, cx);"),
        "ADR 0077 packet 006: an Index name candidate must open the name-match track page"
    );
    assert!(
        name_match_dispatch_source.contains("FrameNavigationEntry::IndexNameMatches(name.clone())"),
        "ADR 0077 packet 006: opening a name match must push the IndexNameMatches entry"
    );
    assert!(
        app_source.contains(
            "FrameNavigationEntry::IndexNameMatches(_) => \"Tracks matching\".to_string(),"
        ),
        "ADR 0077 packet 006: the frame title for a name match must be \"Tracks matching\", never \"Artist\""
    );
    assert!(
        !app_source.contains("FrameNavigationEntry::IndexNameMatches(_) => \"Artist\".to_string()"),
        "ADR 0077 packet 006: a name match must never show the \"Artist\" frame title"
    );
    assert!(
        app_source.contains("Some(FrameNavigationEntry::IndexNameMatches(_)) => {")
            && app_source.contains("self.render_name_match_page_content(cx);"),
        "ADR 0077 packet 006: the content list must render the name-match track page, not the old scoped feed-results view"
    );
    assert!(
        workspace_tests_source.contains("display.segments[2].target")
            && workspace_tests_source.contains("FrameNavigationEntry::IndexNameMatches")
            && workspace_tests_source.contains("the immediate Index parent must stay selectable"),
        "breadcrumb tests must keep the name-match parent selectable"
    );
    assert!(
        !index_detail_source.contains("IndexDetailKind::Artist")
            && !app_source.contains("ArtistDetailPageVm")
            && !search_dispatch_source.contains("ArtistDetailPageVm"),
        "Index name-match rows must not invent an Index artist detail kind or reuse Library artist detail VM"
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
        "FrameNavigationEntry::IndexNameMatches(_)",
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
/// ADR 0076 Decision 9 amended the route source on 2026-09-24: the repair
/// writes the stored route, not a frame built from a `MusicIndex` track.
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
        "write_id3v24_edits",
        "NoRoutesUpstream",
    ] {
        if !owner_source.contains(required) {
            violations.push(format!(
                "{OWNER}: ADR 0065 payment-route repair owner is missing `{required}`"
            ));
        }
    }
    let owner_production = production_source(&owner_source);
    for required in [
        "db::payment_routes::stored_route(",
        "with_stored_route_frame(",
    ] {
        if !owner_production.contains(required) {
            violations.push(format!(
                "{OWNER}: ADR 0076 Decision 9 amends ADR 0065: the repair writes the route stored in \
                 the database and asks MusicIndex only when the database has none. The owner is \
                 missing `{required}`. Read the stored route first, and build the frame with \
                 metadata_service::with_stored_route_frame."
            ));
        }
    }
    if owner_production.contains("id3_edits_for_track_context") {
        violations.push(format!(
            "{OWNER}: ADR 0076 Decision 9 amends ADR 0065: the repair must not build the route \
             frame from a MusicIndex track context. Use metadata_service::with_stored_route_frame."
        ));
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

const ROUTE_SOURCE_FIX: &str = "ADR 0076 Decision 9: each write of the payment route frame uses the route stored in the database. Pass the edits through metadata_service::with_stored_route_frame before the tag write. That function reads the stored route and stores a MusicIndex route only when the database has none. Do not write a route frame that a MusicIndex response built.";

/// The tag write calls that can carry the payment route frame.
const TAG_WRITE_CALLS: [&str; 2] = ["write_id3v24_edits(", "apply_id3_edits_nonfatal("];

/// Each production function that writes audio tags must pass its edits
/// through `with_stored_route_frame` before the write. The tag writer and
/// its one non-fatal wrapper are the exceptions. The callers of the wrapper
/// are checked.
fn route_source_violations(file: &str, source: &str) -> Vec<String> {
    if file == "src/audio_tags.rs" {
        return Vec::new();
    }
    let production = code_only(&without_unit_test_module(&manifest_path(file), source));
    let starts = production
        .match_indices("fn ")
        .map(|(index, _)| index)
        .filter(|index| {
            *index == 0
                || !production.as_bytes()[index - 1].is_ascii_alphanumeric()
                    && production.as_bytes()[index - 1] != b'_'
        })
        .collect::<Vec<_>>();
    let mut violations = Vec::new();
    for (position, start) in starts.iter().enumerate() {
        let end = starts
            .get(position + 1)
            .copied()
            .unwrap_or(production.len());
        let section = &production[*start..end];
        let name = section[3..]
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .next()
            .unwrap_or_default();
        if file == "src/subscribe_service.rs" && name == "apply_id3_edits_nonfatal" {
            continue;
        }
        let Some(write) = TAG_WRITE_CALLS
            .iter()
            .filter_map(|call| section.find(call))
            .min()
        else {
            continue;
        };
        match section.find("with_stored_route_frame(") {
            Some(stored) if stored < write => {}
            _ => violations.push(format!(
                "{file}: `fn {name}` writes audio tags without a stored route read before the write.\n  {ROUTE_SOURCE_FIX}"
            )),
        }
    }
    violations
}

/// Situational ADR 0076 Decision 9 (packet 003): no site builds the payment
/// route frame of a file from an API response. Each tag write reads the
/// stored route first.
///
/// The Discover tag comparison writes a file with no track row, so the
/// database can have no route for it. Its command carries no stored route,
/// and the packet 003 result records this case.
#[test]
fn adr_0076_route_readiness_route_frame_writes_read_the_stored_route() {
    // The guard fails on a site that builds the frame from an API response.
    const API_FRAME_WRITE: &str = "fn tag_download(track: &api::Track, path: &Path) -> Result<()> {\n    let edits = id3_edits_for_track_context(&TrackContext::new(track.clone(), None));\n    write_id3v24_edits(path, &edits)?;\n    Ok(())\n}\n";
    const LATE_STORED_READ: &str = "fn tag_download(conn: &Connection, track: &api::Track, path: &Path) -> Result<()> {\n    let edits = id3_edits_for_track_context(&TrackContext::new(track.clone(), None));\n    write_id3v24_edits(path, &edits)?;\n    let _ = with_stored_route_frame(conn, 1, None, edits, RouteFrameWrite::Always)?;\n    Ok(())\n}\n";
    const STORED_WRITE: &str = "fn tag_download(conn: &Connection, track: &api::Track, path: &Path) -> Result<()> {\n    let edits = with_stored_route_frame(conn, 1, track.payment_routes.as_deref(), Vec::new(), RouteFrameWrite::Always)?;\n    write_id3v24_edits(path, &edits)?;\n    Ok(())\n}\n";
    assert_eq!(
        route_source_violations("src/example.rs", API_FRAME_WRITE).len(),
        1
    );
    assert_eq!(
        route_source_violations("src/example.rs", LATE_STORED_READ).len(),
        1
    );
    assert!(route_source_violations("src/example.rs", STORED_WRITE).is_empty());

    let mut violations = Vec::new();
    let mut writers = BTreeSet::new();
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        let production = code_only(&without_unit_test_module(&path, &source));
        if file != "src/audio_tags.rs"
            && TAG_WRITE_CALLS.iter().any(|call| production.contains(call))
        {
            writers.insert(file.clone());
        }
        violations.extend(route_source_violations(&file, &source));
    }
    // The write sites of packet 003 and the Library tag apply. ADR 0076
    // Decision 8 removed the feed update write of `src/feed_service.rs`, and
    // packet 004 added the confirmed tag update.
    for expected in [
        "src/subscribe_service/materialization.rs",
        "src/application/commands/tag_update.rs",
        "src/application/commands/payment_routes.rs",
        "src/application/commands/metadata.rs",
    ] {
        assert!(
            writers.contains(expected),
            "{expected} no longer writes audio tags. Update this guard and the packet 003 result.\n  {ROUTE_SOURCE_FIX}"
        );
    }
    assert!(
        violations.is_empty(),
        "ADR 0076 Decision 9 route source violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0076 Decision 9 (packet 007): the route guard reads a
/// test-only file as test code, never as production code. A file that only
/// a test build compiles cannot give a false route violation.
///
/// Incident, ADR 0080 packet 002 on 2026-09-29: a test in
/// `src/discover/tests.rs` wrote a file with `write_id3v24_edits`, and the
/// guard reported it as a route frame write. ADR 0060 deleted that file on
/// 2026-09-30. It was the only file that used the sibling `<dir>.rs` parent
/// pattern of `is_test_only_source_file`; this guard now proves that pattern
/// only through the function's own path logic, not a live file.
#[test]
fn adr_0076_route_readiness_ignores_test_only_files() {
    // R76-7-01: the parent-declared test-only files, and their production kin.
    for test_only_file in [
        "src/view_models/workspace/tests.rs",
        "src/view_models/search_results/tests.rs",
    ] {
        assert!(
            is_test_only_source_file(&manifest_path(test_only_file)),
            "{test_only_file}: its parent declares it under `#[cfg(test)]` and must count as test-only"
        );
    }
    for production_file in ["src/metadata.rs"] {
        assert!(
            !is_test_only_source_file(&manifest_path(production_file)),
            "{production_file} is production code and must not count as test-only"
        );
    }

    // R76-7-02 and R76-7-03: the same tag write, in a test-only file and in
    // a production file.
    const SAMPLE_WRITE: &str =
        "fn a_test_writes_a_tag() {\n    write_id3v24_edits(path, &edits).unwrap();\n}\n";
    assert!(
        route_source_violations("src/view_models/workspace/tests.rs", SAMPLE_WRITE).is_empty(),
        "a test-only file must give the route guard no production text"
    );
    assert_eq!(
        route_source_violations("src/metadata.rs", SAMPLE_WRITE).len(),
        1,
        "a production file must still report the route violation"
    );
}

const TAG_UPDATE_FIX: &str = "ADR 0076 Decision 8: the RSS check, the tag update scan and the feed update change the database only. They write no audio tag and no stored route. Move the write into application::commands::tag_update, which runs only when the operator confirms the \"Update n file(s)\" popup.";

/// The calls that write a file tag or store a route for a tag write.
const NO_TAG_WRITE_CALLS: [&str; 5] = [
    "write_id3v24_edits(",
    "apply_id3_edits_nonfatal(",
    "with_stored_route_frame(",
    "route_for_write(",
    "store_musicindex_route(",
];

fn tag_write_violations(label: &str, production: &str) -> Vec<String> {
    NO_TAG_WRITE_CALLS
        .iter()
        .filter(|call| production.contains(*call))
        .map(|call| format!("{label}: found `{call}`.\n  {TAG_UPDATE_FIX}"))
        .collect()
}

/// Situational ADR 0076 Decision 8 (packet 004, R4-09 and R4-10): the check
/// modules of packet 002, the tag update scan and `apply_feed_updates` call
/// no tag write. The scan and the confirmed write run in the runtime actor,
/// off the render thread. Delete this guard when ADR 0076 is superseded.
#[test]
fn adr_0076_tag_update_checks_and_scans_write_no_tag() {
    // The guard fails on a synthetic check that writes a tag.
    assert_eq!(
        tag_write_violations(
            "src/example.rs",
            "fn apply(path: &Path) { write_id3v24_edits(path, &[]).unwrap(); }"
        )
        .len(),
        1
    );

    let mut violations = Vec::new();
    for file in [
        "src/rss/check_apply.rs",
        "src/rss/compare.rs",
        "src/runtime/playlist_rss_check.rs",
        "src/application/queries/tag_update.rs",
    ] {
        let path = manifest_path(file);
        let source = read_source(&path);
        violations.extend(tag_write_violations(
            file,
            &code_only(&without_unit_test_module(&path, &source)),
        ));
    }
    let service = read_source(&manifest_path("src/feed_service.rs"));
    let update = source_between(
        &service,
        "pub fn apply_feed_updates(",
        "pub fn track_row_to_track_context(",
    );
    violations.extend(tag_write_violations(
        "src/feed_service.rs: apply_feed_updates",
        &code_only(update),
    ));

    // R4-10: the scan and the write run on the blocking pool of the
    // runtime actor. The Music screen and the view model only read its
    // snapshot.
    let actor = read_source(&manifest_path("src/runtime/tag_update.rs"));
    for required in [
        "tokio::task::spawn_blocking(move || {\n                write_tag_updates(",
        "plan_tag_update_scan(&conn, &music_dir)",
        "bus.publish(VmEvent::TrackChanged { track_id })",
    ] {
        if !actor.contains(required) {
            violations.push(format!(
                "src/runtime/tag_update.rs: missing `{required}`. ADR 0040 and ADR 0076 Decision 8: the runtime actor owns the scan and the write, off the render thread."
            ));
        }
    }
    for file in [
        "src/library/app_impl.rs",
        "src/view_models/tag_update.rs",
        "src/ui/shells/tag_update_confirmation.rs",
    ] {
        let path = manifest_path(file);
        let source = code_only(&without_unit_test_module(&path, &read_source(&path)));
        for forbidden in [
            "read_audio_tags(",
            "plan_tag_update_scan(",
            "compare_planned_files(",
            "write_tag_updates(",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{file}: found `{forbidden}`. ADR 0040 and ADR 0076 Decision 8: send the scan or the confirm to the tag update actor through its handle, and read its snapshot."
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0076 Decision 8 tag write violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0076 Decision 7 (packet 003, operator decision
/// 2026-09-24): the removal actions of a removed track reuse the existing
/// removal flows. "Remove from library" is the ADR 0044 Library removal with
/// its confirmation. "Remove from playlist" is the existing playlist entry
/// removal. The playlist row actions come from the view model.
#[test]
fn adr_0076_route_readiness_removal_actions_reuse_existing_flows() {
    const FIX: &str = "ADR 0076 Decision 7: the removal actions of a removed track reuse the existing flows. Dispatch \"Remove from library\" to LibraryApp::remove_track, which runs the ADR 0044 removal plan and its confirmation. Wire \"Remove from playlist\" to the on_remove slot of the row. Add no second removal path.";
    let app = read_source(&manifest_path("src/library/app_impl.rs"));
    let shell = read_source(&manifest_path("src/ui/shells/playlist.rs"));
    let mut violations = Vec::new();

    let dispatch = code_only(source_between(
        &app,
        "pub(crate) fn run_content_list_row_action(",
        "fn confirm_removed_track(",
    ));
    if !compact_source(&dispatch)
        .contains("ContentListRowActionKind::RemoveFromLibrary{track_id}=>{self.remove_track(track_id,window,cx);")
    {
        violations.push(format!(
            "src/library/app_impl.rs: the RemoveFromLibrary row action must call self.remove_track(track_id, window, cx).\n  {FIX}"
        ));
    }
    let remove_track = code_only(source_between(
        &app,
        "pub(crate) fn remove_track(",
        "fn request_library_removal(",
    ));
    if !remove_track.contains(
        "self.request_library_removal(LibraryRemovalIntent::TrackId(track_id), window, cx)",
    ) {
        violations.push(format!(
            "src/library/app_impl.rs: remove_track must request the ADR 0044 removal plan.\n  {FIX}"
        ));
    }
    let removed_block = code_only(source_between(
        &shell,
        "fn render_removed_from_feed(",
        "fn render_removed_track_action(",
    ));
    for required in [
        "let on_remove = slot.on_remove.clone();",
        "slot.on_remove_from_all_playlists.clone()",
        "remove_from_playlist",
        "remove_from_all_playlists",
    ] {
        if !removed_block.contains(required) {
            violations.push(format!(
                "src/ui/shells/playlist.rs: the removed-from-feed row block is missing `{required}`.\n  {FIX}"
            ));
        }
    }
    let action_button = code_only(source_between(
        &shell,
        "fn render_removed_track_action(",
        "/// A bordered text badge",
    ));
    for required in [
        "action.label",
        "action.a11y_label",
        "action.availability.disabled()",
    ] {
        if !action_button.contains(required) {
            violations.push(format!(
                "src/ui/shells/playlist.rs: the removed-track action renders from its view model display; missing `{required}`.\n  {FIX}"
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "ADR 0076 Decision 7 removal action violations:\n{}",
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
        // ADR 0076 packet 003 (operator decision 2026-09-24) gives a removed
        // track two row actions, so the row carries an action list.
        "pub(crate) actions: Vec<ContentListRowActionDisplay>",
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
        "for action in &row.actions",
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
        "this.run_content_list_row_action(kind, window, cx)",
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
    let check_index = combined_command.find("check_feed_batch_for_updates(");
    let apply_index = combined_command.find("apply_stale_feed_updates(");
    let repair_index = combined_command.find("RepairMissingPaymentRouteTags::new");
    match (check_index, apply_index, repair_index) {
        (Some(check), Some(apply), Some(repair)) if check < apply && apply < repair => {}
        _ => violations.push(
            "src/application/commands/feed.rs: ADR 0065 Check all feeds must repair only after stale feed updates apply. Call check_feed_batch_for_updates, then apply_stale_feed_updates, then RepairMissingPaymentRouteTags::new inside CheckFeedsAndRepairRoutes."
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
        "impl Drop for TopApp",
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
        "impl Drop for TopApp",
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
        "impl Drop for TopApp",
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
    let app_render = app_source
        .split_once("impl Render for TopApp")
        .expect("app renderer")
        .1;
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
        "Release(FeedResultDisplay)",
        "Track(TrackResultDisplay)",
        "pub(crate) struct ContentListEntityBadgeDisplay",
        "pub(crate) struct ContentListLibraryBadgeDisplay",
        "ContentListLibraryBadgeDisplay::for_source(source)",
        "pub(crate) fn from_release_result(",
        "pub(crate) fn from_track_result(",
    ] {
        if !row_contract.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: Situational ADR 0062 mixed entity row contract missing `{required}`"
            ));
        }
    }

    // Artist rows and the row-level expansion affordance are deferred until
    // the index recency source exposes artist rows; ADR 0062 scopes the live
    // contract to Release and Track. See dead-code-removal-task-002.
    let variant_count = ["Release(", "Track("]
        .into_iter()
        .filter(|variant| row_contract.contains(variant))
        .count();
    if variant_count != 2 {
        violations.push(format!(
            "src/view_models/library.rs: Situational ADR 0062 mixed entity row contract must have exactly two row-kind cases; found {variant_count}"
        ));
    }

    for required in [
        "pub(crate) id: String",
        "pub(crate) kind: ContentListRowKind",
        "pub(crate) entity_badge: ContentListEntityBadgeDisplay",
        "pub(crate) library_badge: ContentListLibraryBadgeDisplay",
        "pub(crate) source: ContentListRowSource",
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
        library_render,
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
        "ContentListRowDisplay::from_release_result(row.clone())",
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
        "LogFrame::new(frames, source, display.text)",
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

/// Situational ADR 0070: cards yield space through the shared log split; the sidebar stays separate.
#[test]
fn adr_0070_show_log_budget_and_card_scroll_have_shared_owners() {
    let pane = read_source(&manifest_path("src/ui/composites/show_log_pane.rs"));
    let app = read_source(&manifest_path("src/app/show.rs"));
    let shell = read_source(&manifest_path("src/ui/shells/show.rs"));
    for required in [
        ".id(\"show-card-scroll\")",
        ".overflow_y_scrollbar()",
        "page_scroll_content(cx).child(self.main)",
        ".leading(cards.into_any_element())",
        "f32::from(handle_height) / scale",
    ] {
        assert!(pane.contains(required), "Situational ADR 0070: missing {required}; the log composite must own the bounded card scroll region and measured handle.");
    }
    for forbidden in ["grid_rows", "Size::MenuCompact", "panel_open", "panel_mode"] {
        assert!(!pane.contains(forbidden), "Situational ADR 0070: {forbidden} must not let card rows consume log space or change sidebar visibility.");
    }
    assert!(
        compact_source(&app).contains("log_pane.update_geometry(height,handle_height)"),
        "Situational ADR 0070: pass measured split bounds to the renderer-free log budget owner."
    );
    assert!(
        shell.contains(".child(ShowDetailPanel::new("),
        "Situational ADR 0070: keep the sidebar outside the main-column log split."
    );
}

/// Situational ADR 0070: long identities cannot grow the header and remain available for exact copy.
#[test]
fn adr_0070_show_log_header_is_compact_and_preserves_identity() {
    let pane = read_source(&manifest_path("src/ui/composites/show_log_pane.rs"));
    let source = source_between(
        &pane,
        ".id(\"show-log-source\")",
        ".id(\"show-log-metadata\")",
    );
    for required in [
        ".overflow_hidden()",
        ".whitespace_nowrap()",
        "Tooltip::new(source_label.clone())",
        "SelectableText::new(",
        "display.unit_name",
    ] {
        assert!(source.contains(required), "Situational ADR 0070: source header needs {required}; keep one line with full identity available through shared hover and selection owners.");
    }
    assert!(pane.contains("display.header_status"), "Situational ADR 0070: render the view model's brief state rather than parsing the longer service explanation.");
    let count = source_between(
        &pane,
        ".id(\"show-log-line-count\")",
        ".children(display.header_status",
    );
    assert!(
        count.contains("Tooltip::new(count_label.clone())"),
        "Situational ADR 0070: a clipped count needs its own complete hover text."
    );
}

/// Situational ADR 0070: a shared footer must never wrap status text into the log body.
#[test]
fn adr_0070_log_footer_bounds_text_and_retains_the_follow_action() {
    let frame = read_source(&manifest_path("src/ui/composites/log_frame.rs"));
    let footer = source_between(&frame, "fn footer(", "fn observe_user_scroll(");
    for required in [
        ".whitespace_nowrap()",
        ".overflow_hidden()",
        ".flex_shrink_0()",
        "LogFooterLayout::Compact",
        "self.vm.footer_status(self.footer_layout)",
        "Tooltip::new(description)",
        ".a11y_label(action.a11y_label)",
        ".tooltip(action.a11y_label)",
        ".on_activate(",
    ] {
        assert!(footer.contains(required), "Situational ADR 0070: shared footer is missing {required}; preserve bounded status and the accessible following control.");
    }
    assert!(
        !footer.contains(".flex_wrap()"),
        "Situational ADR 0070: use the compact width class instead of wrapping the footer."
    );
    assert!(
        compact_source(&frame)
            .contains("LogFooterLayout::for_width(f32::from(bounds.size.width)/scale"),
        "Situational ADR 0070: classify the measured log viewport in unscaled units."
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
        "TextSelectionRegistration::new(",
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

/// Situational ADR 0071: debug layout retains the measured dependency optimization.
#[test]
fn adr_0071_debug_layout_dependencies_remain_optimized() {
    let manifest = read_source(&manifest_path("Cargo.toml"))
        .parse::<toml::Table>()
        .expect("Cargo.toml must parse");
    let dev = &manifest["profile"]["dev"];
    for package in ["gpui-pre", "taffy"] {
        assert_eq!(
            dev["package"][package]["opt-level"].as_integer(),
            Some(3),
            "ADR 0071: retain the verified debug layout optimization for {package}"
        );
    }
    assert_eq!(
        dev.get("opt-level")
            .and_then(toml::Value::as_integer)
            .unwrap_or(0),
        0
    );
    assert!(dev
        .get("debug-assertions")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true));
}

/// Situational ADR 0072: only the reviewed gpui-base correction may override published packages.
#[test]
fn adr_0072_gpui_base_fork_is_the_only_pinned_source_override() {
    let manifest = read_source(&manifest_path("Cargo.toml"))
        .parse::<toml::Table>()
        .expect("Cargo.toml must parse");
    let patches = manifest["patch"].as_table().unwrap();
    assert_eq!(patches.len(), 1, "ADR 0072: only crates.io may be patched");
    let crates_io = patches["crates-io"].as_table().unwrap();
    assert_eq!(
        crates_io.len(),
        1,
        "ADR 0072: only gpui-base may be patched"
    );
    let base = crates_io["gpui-base"].as_table().unwrap();
    assert_eq!(
        base.len(),
        2,
        "ADR 0072: the override requires only git and rev"
    );
    assert_eq!(
        base["git"].as_str(),
        Some("https://github.com/InTheMorning/gpui-kit")
    );
    assert_eq!(
        base["rev"].as_str(),
        Some("5463fe4e72fd740b0db08da92003488b32661867")
    );
    assert!(!manifest.contains_key("replace"));

    let dependencies = manifest["dependencies"].as_table().unwrap();
    for name in ["gpui-base", "gpui-component", "gpui-kit-assets"] {
        assert_eq!(dependencies[name].as_str(), Some("=0.6.1"));
    }
    for (name, package) in [("gpui", "gpui-pre"), ("gpui_platform", "gpui-pre-platform")] {
        assert_eq!(dependencies[name]["package"].as_str(), Some(package));
        assert_eq!(dependencies[name]["version"].as_str(), Some("=0.3.1"));
        for forbidden in ["git", "path", "branch", "tag", "rev"] {
            assert!(dependencies[name].get(forbidden).is_none());
        }
    }

    let lock = read_source(&manifest_path("Cargo.lock"))
        .parse::<toml::Table>()
        .expect("Cargo.lock must parse");
    let packages = lock["package"].as_array().unwrap();
    for (name, version) in [
        ("gpui-base", "0.6.1"),
        ("gpui-component", "0.6.1"),
        ("gpui-kit-assets", "0.6.1"),
        ("gpui-pre", "0.3.1"),
        ("gpui-pre-platform", "0.3.1"),
    ] {
        let matches: Vec<_> = packages
            .iter()
            .filter(|package| package["name"].as_str() == Some(name))
            .collect();
        assert_eq!(matches.len(), 1, "ADR 0072: require one version of {name}");
        assert_eq!(matches[0]["version"].as_str(), Some(version));
        let source = if name == "gpui-base" {
            "git+https://github.com/InTheMorning/gpui-kit?rev=5463fe4e72fd740b0db08da92003488b32661867#5463fe4e72fd740b0db08da92003488b32661867"
        } else {
            "registry+https://github.com/rust-lang/crates.io-index"
        };
        assert_eq!(matches[0]["source"].as_str(), Some(source));
    }
}

/// Situational ADR 0071: inputs and logs share selection policy and keep Linux buffers separate.
#[test]
fn adr_0071_selection_and_primary_stay_in_shared_owners() {
    let manifest = read_source(&manifest_path("Cargo.toml"));
    assert!(!manifest.contains("v4vmm-text-selection"));
    assert!(manifest.contains("gpui-component = \"=0.6.1\""));
    assert!(manifest.contains("gpui-base = \"=0.6.1\""));
    assert!(!manifest_path("vendor").exists());
    assert!(!manifest_path("crates/text-selection").exists());
    let input = read_source(&manifest_path("src/ui/primitives/primary_selection.rs"));
    let logs = read_source(&manifest_path("src/ui/composites/selectable_text.rs"));
    for required in [
        "observe_inputs::<InputMode>",
        "observe_inputs::<TextareaMode>",
        "previous_range",
        "previous_text",
        "is_masked()",
        "character_index_for_point",
        "range_to_bounds",
        "primary_offset_at",
        "if !input.is_editable()",
        "input.insert(text, window, cx)",
        "MouseButton::Middle",
    ] {
        assert!(
            input.contains(required),
            "ADR 0071: shared input integration requires {required}"
        );
    }
    assert!(
        !source_between(&input, "fn observe_inputs", "#[cfg(all(test")
            .contains("read_from_clipboard")
    );
    let paste = source_between(&input, "fn paste_primary_at", "#[cfg(all(test");
    assert!(paste.find("read_from_primary").unwrap() < paste.find("set_selected_range").unwrap());
    assert!(!paste.contains("write_to_clipboard"));
    for required in [
        "TextSelectionHandle",
        "TextSelectionRun::new",
        "update_runs",
        "projection.ranges()",
    ] {
        assert!(
            compact_source(&logs).contains(&compact_source(required)),
            "ADR 0071: logs must use upstream selection projection: {required}"
        );
    }
    assert!(!logs.contains("SelectionGesture"));
    for file in [
        "src/app/tab_bar.rs",
        "src/library/app_impl.rs",
        "src/ui/shells/playlist.rs",
        "src/ui/composites/settings.rs",
        "src/ui/composites/playlist_popover.rs",
        "src/ui/composites/maintenance_forms.rs",
    ] {
        let source = read_source(&manifest_path(file));
        assert!(
            source.contains(".with_primary_selection("),
            "ADR 0071: {file} must compose shared PRIMARY integration"
        );
    }
    for file in SCREEN_FILES {
        let source = read_source(&manifest_path(file));
        let source = production_source(&source);
        for forbidden in [
            "read_from_primary",
            "write_to_primary",
            "MouseButton::Middle",
        ] {
            assert!(
                !source.contains(forbidden),
                "ADR 0071: {file} must not own {forbidden}"
            );
        }
    }
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    assert!(bootstrap.contains("primary_selection::init(cx)"));
    assert!(bootstrap.contains("context_menu::init(cx)"));
    assert!(bootstrap.contains("gpui_platform::application()"));
    let theme = read_source(&manifest_path("src/ui/theme_bridge.rs"));
    assert!(theme.contains("Theme::sync_base(cx)"));
    assert!(theme.contains("theme.tokens = theme.colors.into()"));
    let editor = read_source(&manifest_path("src/presentation/configuration_editor.rs"));
    assert!(editor.contains("TextareaState::new(window, cx)"));
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
    let right_click = source_between(&text, "MouseButton::Right,", ".on_key_down(");
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
        production_source(&text)
            .matches("cx.write_to_clipboard(")
            .count(),
        1,
        "Situational ADR 0063: keyboard and mouse Copy must use one clipboard path."
    );
    for required in [
        "build_menu_content(&dismiss, self.items, cx)",
        "Surface::new(SurfaceElevation::Floating)",
        ".snap_to_window_with_margin(Spacing::SM.scaled(cx))",
        ".on_mouse_down_out(",
        ".key_context(POINTER_MENU_KEY_CONTEXT)",
        ".on_action(move |_: &Dismiss, window, cx|",
        "self.return_focus.focus(window, cx)",
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
        assert!(command.contains("self.dispatch_show_command("));
        let dispatch = source_between(
            &app,
            "fn dispatch_show_command<C>(",
            "fn record_show_retry_result(",
        );
        assert!(dispatch.contains("this.finish_show_command(command_id, completion, cx)"));
        assert!(dispatch.contains("ShowCommandCompletion::new(Err(error))"));
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
            .matches("let Some(owner) = self.available_playback_owner(&action, cx)")
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

/// Situational ADR 0069: Tab navigation must identify the focused action at the shared primitive.
#[test]
fn adr_0069_keyboard_buttons_show_focus_without_layout_shift() {
    let button = read_source(&manifest_path("src/ui/primitives/button.rs"));
    let enabled = source_between(&button, "if disabled {", "if let Some(icon) = leading_icon");
    let pointer = source_between(
        enabled,
        "if let Some(handler) = on_activate.clone() {",
        "} else if let Some(handler) = on_click {",
    );
    assert!(
        pointer.contains("if pointer_activation_click(event) {"),
        "Situational ADR 0069: GPUI key-up clicks must not duplicate the key-down activation"
    );
    let keyboard = enabled
        .rsplit_once("if on_activate.is_some() || focus.is_some() {")
        .unwrap()
        .1;
    for required in [
        "keyboard_button_focus(hit_target, appearance, focus, cx)",
        ".tab_index(0)",
        ".rounded(radius)",
        "keyboard_activation_key(event)",
    ] {
        assert!(
            keyboard.contains(required),
            "Situational ADR 0069: keyboard buttons must share focus and activation: {required}"
        );
    }
    let focus = source_between(
        &button,
        "fn keyboard_button_focus(",
        "fn button_description(",
    );
    for required in [
        "layout::scaled_dimension(layout::CONTROL_FOCUS_RING_WIDTH, cx)",
        "resolve_color(cx, SemanticColor::Focus, appearance)",
        ".border(focus_ring_width)",
        ".border_color(gpui::transparent_black())",
        ".focus(move |style| style.border_color(focus_color))",
    ] {
        assert!(
            focus.contains(required),
            "Situational ADR 0069: keyboard button focus requires {required}"
        );
    }
    assert!(focus.find(".border(focus_ring_width)").unwrap() < focus.find(".focus(").unwrap());
    assert!(
        !focus.contains("style.border("),
        "Focus must not change layout dimensions"
    );
    let disabled = enabled.split_once("} else {").unwrap().0;
    assert!(!disabled.contains(".tab_index(") && !disabled.contains("keyboard_button_focus("));
}

/// Situational ADR 0069: one live, portable Settings owner; navigation is inert.
#[test]
fn adr_0069_settings_group_ownership() {
    let app = read_source(&manifest_path("src/app.rs"));
    let screen = read_source(&manifest_path("src/app/settings.rs"));
    let vm = read_source(&manifest_path("src/view_models/settings.rs"));
    let form = read_source(&manifest_path("src/ui/composites/settings.rs"));
    let capabilities = read_source(&manifest_path("src/app/capabilities.rs"));
    assert!(
        app.contains("use settings::render_settings;") && app.contains("settings: SettingsVm,")
    );
    assert!(!app.contains("fn render_settings(") && !app.contains("fn render_ui_scale_picker("));
    assert_eq!(screen.matches("fn render_settings(").count(), 1);
    for required in [
        "settings_frame(navigation, content, page_scroll, workspace, cx)",
        "settings_field(field, input, cx)",
        "app.render_capabilities(true, cx)",
        "app.cached_files.status()",
    ] {
        assert!(
            screen.contains(required),
            "Settings must reuse the shared owner: {required}"
        );
    }
    for forbidden in [
        "gpui",
        "serde",
        "std::fs",
        "save_app_settings(",
        "present_command(",
    ] {
        assert!(
            !vm.contains(forbidden),
            "Settings VM must stay portable and session-local: {forbidden}"
        );
    }
    for forbidden in [
        ".lock()",
        "cx.spawn",
        "config::save_app_settings(",
        ".text_size(",
        ".gap(",
        "truncate()",
    ] {
        assert!(
            !screen.contains(forbidden),
            "Settings screen must only compose and dispatch: {forbidden}"
        );
    }
    assert!(form.contains(".flex_wrap()") && form.contains(".on_activate("));
    assert!(form.contains(".a11y_label(") && form.contains(".disabled(display.availability"));
    assert!(
        form.find(".children(navigation)").unwrap()
            < form.find(".id(\"settings-scroll\")").unwrap()
    );
    assert!(form.contains(".min_h_0()") && form.contains(".overflow_y_scroll()"));
    assert!(form.contains(".track_scroll(scroll_handle)"));
    assert!(form.contains(".vertical_scrollbar(scroll_handle)"));
    assert!(app.contains("settings_scroll: crate::ui::composites::settings::SettingsScrollHandles"));
    assert!(screen.contains("app.settings_scroll.handle(app.settings.selected())"));
    assert!(!form.contains("truncate()"));
    let select = source_between(&app, "fn select_tab(", "fn set_frame_filter(");
    let navigation = source_between(select, "AppTab::Settings => {", "AppTab::Music => {");
    assert!(navigation.contains("self.settings.dispatch(SettingsAction::Open)"));
    for forbidden in [
        "save_settings(",
        "set_value(",
        "SettingsVm::default",
        "reload_cached(",
    ] {
        assert!(
            !navigation.contains(forbidden),
            "Settings re-entry must preserve session inputs and observations: {forbidden}"
        );
    }
    for field in ["endpoint_input", "music_dir_input", "flac_path_input"] {
        assert!(app.contains(&format!("{field}: Entity<InputState>")));
    }
    let dispatch = source_between(
        &screen,
        "pub(super) fn settings_action(",
        "fn use_default_settings(",
    );
    assert!(dispatch.contains("SettingsEffect::Navigate => {}"));
    assert!(
        !dispatch.contains("focus_active_tab("),
        "Situational ADR 0069: selecting a group must retain focus on its mounted button"
    );
    let button = read_source(&manifest_path("src/ui/primitives/button.rs"));
    assert!(!button.contains("prevent_default("),
        "Situational ADR 0069: group buttons must keep GPUI mouse-down focus transfer before hiding an input");
    assert_eq!(
        dispatch.matches("self.save_settings(window, cx)").count(),
        1
    );
    assert_eq!(
        dispatch
            .matches("self.use_default_settings(window, cx)")
            .count(),
        1
    );
    assert!(!dispatch.contains("set_value(") && !dispatch.contains("present_command("));
    let defaults = source_between(
        &screen,
        "fn use_default_settings(",
        "pub(super) fn render_settings(",
    );
    assert_eq!(
        defaults.matches("self.save_settings(window, cx)").count(),
        1
    );
    for required in [
        "self.endpoint_input.update",
        "self.flac_path_input.update",
        "self.set_ui_scale(",
        "self.set_theme_profile(",
    ] {
        assert!(defaults.contains(required));
    }
    // ADR 0066 task 006 moves core paths behind managed repair; defaults keep them.
    assert!(!defaults.contains("self.music_dir_input.update"));
    assert!(screen.contains("app.configuration_editor"));
    let save = source_between(&app, "fn save_settings(", "fn reload_cached(");
    assert_eq!(save.matches("config::save_app_settings(").count(), 1);
    for field in [
        "endpoint_input",
        "music_dir_input",
        "flac_path_input",
        "ui_scale",
        "theme_profile",
    ] {
        assert!(save.contains(field), "shared Save must retain {field}");
    }
    let report = source_between(
        &capabilities,
        "CapabilityAction::Configure(dependency) =>",
        "CapabilityAction::CopyReport =>",
    );
    assert!(
        report.find("SettingsAction::OpenReport").unwrap()
            < report.find("self.select_tab(AppTab::Settings").unwrap()
    );
    for file in ["src/config.rs", "src/db.rs"] {
        let source = read_source(&manifest_path(file));
        for forbidden in [
            "SettingsGroup",
            "SettingsVm",
            "settings_group",
            "selected_settings_group",
        ] {
            assert!(
                !source.contains(forbidden),
                "group navigation must not change persistence: {file} {forbidden}"
            );
        }
    }
}

/// Situational ADR 0066, invariants 5–6: core maintenance consumes one proven session drain.
#[test]
fn adr_0066_core_maintenance_drains_the_session() {
    let lifecycle = read_source(&manifest_path("src/application/session_lifecycle.rs"));
    for required in [
        "enum SessionPhase",
        "Running",
        "Draining",
        "Maintenance",
        "Resuming",
        "struct MaintenanceSession",
        "struct SessionDrain",
        "SESSION_DRAIN_TIMEOUT",
        "Arc::try_unwrap(connection)",
        "connection.close()",
        "wait_for_work",
        "state.phase != SessionPhase::Running",
        "self.0.stop.cancel()",
        "!state.owners.is_empty()",
        "shutdown_for_maintenance()",
        "finish_session_playback",
    ] {
        assert!(
            lifecycle.contains(required),
            "ADR 0066 managed maintenance requires {required}"
        );
    }
    assert!(
        !lifecycle.contains("strong_count"),
        "ADR 0066 requires actual close/ownership transfer"
    );
    assert!(
        !lifecycle.contains("pub struct MaintenanceSession"),
        "ADR 0066 capability is internal and cannot be constructed by callers"
    );
    let runner = read_source(&manifest_path("src/application/async_command_runner.rs"));
    let dispatch = runner.find("pub fn dispatch<C>").unwrap();
    let admission = runner[dispatch..].find("self.session.admit(name)").unwrap();
    let spawn = runner[dispatch..]
        .find("runtime_handle.spawn_blocking")
        .unwrap();
    assert!(
        admission < spawn,
        "ADR 0066 admission must count work before enqueueing it"
    );
    assert!(runner.contains("CommandError::SessionDraining"));
    assert!(runner.contains("drop(work)"));
    for path in [
        "src/runtime/actor.rs",
        "src/runtime/playback_polling.rs",
        "src/runtime/broadcast_readiness.rs",
        "src/runtime/broadcast_service_watch.rs",
        "src/runtime/musicbrainz_feed_saga.rs",
    ] {
        let actor = read_source(&manifest_path(path));
        assert!(
            actor.contains("session.spawn_actor("),
            "ADR 0066 untracked actor: {path}"
        );
        assert!(
            actor.contains("stop.cancelled()"),
            "ADR 0066 missing actor stop: {path}"
        );
    }
    let paged = read_source(&manifest_path("src/application/paged_track_list.rs"));
    assert!(paged.contains("conn: Connection"));
    assert!(paged.contains("actor::spawn(self, bus)"));
    assert!(paged.contains("adr_0066_paged_actor_releases_its_exclusive_configured_connection"));
    let transition = read_source(&manifest_path("src/presentation/session_transition.rs"));
    assert!(transition.contains("Arc::try_unwrap(runtime)"));
    assert!(transition.contains("self.drain.finish()?"));
    let screen = read_source(&manifest_path("src/app/startup.rs"));
    for required in [
        "maintenance: Option<MaintenanceSession>",
        "maintenance.begin_resume()",
        "maintenance.resume_failed()",
        "this.normal.take()",
        "this.close_session_resources",
        "worker.retire(result)",
    ] {
        assert!(
            screen.contains(required),
            "ADR 0066 root must retain managed transition: {required}"
        );
    }
    let presenter = read_source(&manifest_path(
        "src/presentation/async_command_presenter.rs",
    ));
    assert!(presenter.contains("!session.accepts(generation)"));
    let form = read_source(&manifest_path("src/ui/composites/maintenance_forms.rs"));
    for forbidden in [
        "std::fs",
        "Connection",
        "RuntimeHost",
        "thread::sleep",
        "spawn_blocking",
    ] {
        assert!(
            !form.contains(forbidden),
            "ADR 0066 maintenance form cannot own {forbidden}"
        );
    }
    for path in [
        "src/app/session.rs",
        "src/application/session_lifecycle.rs",
        "src/presentation/session_transition.rs",
    ] {
        let source = read_source(&manifest_path(path));
        for forbidden in [
            "control::stop",
            "encoder::disconnect",
            "remove_file(db",
            "save_app_settings",
            "cx.spawn",
        ] {
            assert!(
                !source.contains(forbidden),
                "ADR 0066 session teardown must not call {forbidden} in {path}"
            );
        }
    }
}

#[test]
fn adr_0066_shared_guarded_config_repair() {
    // Situational: ADR 0066 invariants 3–6 and 9. Behavioral proof lives beside
    // CorrectionSource, CorrectionCommand, CorrectionVm and the bootstrap lifecycle.
    let settings = read_source(&manifest_path("src/app/settings.rs"));
    let startup = read_source(&manifest_path("src/app/startup.rs"));
    let presenter = read_source(&manifest_path("src/presentation/configuration_editor.rs"));
    let commands = read_source(&manifest_path("src/application/commands/maintenance.rs"));
    let backend = read_source(&manifest_path("src/config/correction.rs"));
    let vm = read_source(&manifest_path("src/view_models/startup/correction.rs"));
    let form = read_source(&manifest_path("src/ui/composites/maintenance_forms.rs"));
    let config = read_source(&manifest_path("src/config.rs"));
    assert!(settings.contains("app.configuration_editor"));
    assert!(startup.contains("ConfigurationEditor::new("));
    assert!(startup.contains("app.configuration_editor.clone_from(&self.editor)"));
    assert_eq!(startup.matches("ConfigurationEditor::new(").count(), 1);
    assert!(compact_source(&presenter).contains("configuration_correction(&self.vm"));
    assert!(presenter.contains("worker.submit(move || command.execute())"));
    assert!(presenter.contains("self.vm.begin(action, path)"));
    assert!(presenter.contains("this.vm.complete(generation, result)"));
    assert!(commands.contains("source.propose(&self.draft)?"));
    assert!(commands.contains("self.access != CorrectionAccess::CoreRecovery"));
    assert!(commands.contains("check_music(music, false, false)"));
    assert!(commands.contains("check_database(database)"));
    assert!(commands.contains("DatabaseReadiness::NeedsPreparation"));
    assert!(startup.contains("this.session_action(SessionAction::EndSession, window, cx)"));
    let release = startup
        .find("this.maintenance = Some(maintenance)")
        .unwrap();
    let core_access = startup
        .find("editor.set_access(CorrectionAccess::CoreRecovery")
        .unwrap();
    assert!(
        release < core_access,
        "core editing requires actual managed release"
    );
    let existing_save = source_between(
        &config,
        "pub fn save_app_settings(",
        "pub(crate) fn save_workspace_layout(",
    );
    assert!(existing_save.contains("read_config_for_save(cfg_path)?"));
    assert!(existing_save.contains("if existing != music_dir"));
    assert!(backend.contains("super::ConfigWriteLease::acquire(&self.path)?"));
    assert!(settings.contains("app.settings.edit_actions(correction_idle)"));
    let refresh = source_between(
        &settings,
        "pub(super) fn refresh_corrected_settings(",
        "pub(super) fn render_settings(",
    );
    assert!(refresh.contains("self.set_ui_scale(scale, window, cx)"));
    assert!(refresh.contains("self.set_theme_profile(profile, window, cx)"));
    for requirement in [
        "self.check_revision()?",
        "create_new(true)",
        "options.mode(0o600)",
        "backup_write(&mut backup_file, &self.bytes)",
        "file.sync_all()",
        "fs::rename(&candidate, self.destination())",
    ] {
        assert!(
            backend.contains(requirement),
            "ADR 0066 preservation requires {requirement}"
        );
    }
    for source in [vm.as_str(), commands.as_str(), backend.as_str()] {
        assert!(!source.contains("use gpui"));
    }
    for source in [form.as_str(), presenter.as_str(), settings.as_str()] {
        for forbidden in [
            "fs::write",
            "fs::rename",
            "Connection::open",
            "source.save(",
        ] {
            assert!(
                !source.contains(forbidden),
                "ADR 0066 storage belongs in the backend: {forbidden}"
            );
        }
    }
    assert!(vm.contains("generation != self.generation"));
    assert!(vm.contains("CorrectionAction::Save | CorrectionAction::Validate"));
    assert!(vm.contains("CorrectionAction::EndSession"));
    assert!(form.contains(".a11y_label(display.a11y_label)"));
    assert!(!form.contains("truncate()"));
    assert!(!commands.contains("load_config_snapshot(") && !commands.contains("prepare_database("));
}

/// Situational ADR 0066: Escape leaves the input and focuses Close without activating it.
#[test]
fn adr_0066_editor_escape_focuses_close_without_closing() {
    let form = read_source(&manifest_path("src/ui/composites/maintenance_forms.rs"));
    let editor = source_between(
        &form,
        "pub(crate) fn configuration_correction(",
        "fn configuration_input_frame(",
    );
    let editor = compact_source(editor);
    assert!(editor.contains("letclose=vm.action(CorrectionAction::CloseEditor)"));
    assert!(editor.contains("body.on_action(move|_:&gpui_component::input::Escape,window,cx|"));
    assert!(editor.contains("letclose_focus=disclosure_focus.clone()"));
    let escape = source_between(&editor, "body.on_action(", ".gap(");
    assert!(escape.contains("cx.stop_propagation()"));
    assert!(escape.contains("close_focus.focus(window,cx)"));
    for forbidden in [
        "callback(",
        ".close_editor(",
        ".reopen_editor(",
        ".edit(",
        ".request(",
        ".sync_input(",
    ] {
        assert!(
            !escape.contains(forbidden),
            "ADR 0066: Escape only moves focus to Close editor; it cannot call {forbidden}"
        );
    }
    assert_eq!(editor.matches(".track_focus(disclosure_focus)").count(), 2);
    for forbidden in [
        ".capture_action",
        ".on_key_down",
        ".capture_key_down",
        "KeyBinding::new",
    ] {
        assert!(!editor.contains(forbidden), "ADR 0066: editor exit must follow the input's existing action handling, not {forbidden}");
    }

    let presenter = read_source(&manifest_path("src/presentation/configuration_editor.rs"));
    assert!(presenter.contains("disclosure_focus: FocusHandle"));
    assert!(presenter.contains("disclosure_focus: cx.focus_handle()"));
    let close = source_between(
        &presenter,
        "CorrectionAction::CloseEditor =>",
        "CorrectionAction::ReopenEditor =>",
    );
    assert!(close.contains("self.vm.close_editor()"));
    assert!(close.contains("self.disclosure_focus.focus(window, cx)"));
    for forbidden in ["self.request", "self.sync_input", "self.input.update"] {
        assert!(!close.contains(forbidden), "ADR 0066: close only hides the retained draft and returns focus; it cannot call {forbidden}");
    }

    let button = read_source(&manifest_path("src/ui/primitives/button.rs"));
    let keyboard = source_between(
        &button,
        "if on_activate.is_some() || focus.is_some() {",
        "if let Some(icon) = leading_icon",
    );
    assert!(keyboard.contains("keyboard_button_focus(hit_target, appearance, focus, cx)"));
    let focus = source_between(
        &button,
        "fn keyboard_button_focus(",
        "fn button_description(",
    );
    assert!(focus.contains(".when_some(focus_handle"));
    assert!(
        focus.contains("button.track_focus(&handle.tab_index(0).tab_stop(true))"),
        "ADR 0066: supplied disclosure focus must remain a keyboard tab stop"
    );
}

/// Situational ADR 0063: every log family shares the framed viewport and reading owner.
#[test]
fn adr_0063_logs_share_frame_following_and_renderer_free_state() {
    let frame = read_source(&manifest_path("src/ui/composites/log_frame.rs"));
    let reading = read_source(&manifest_path("src/view_models/log_view.rs"));
    for required in [
        "LogReadingVm",
        "LogSource",
        "ScrollHandle",
        "SelectableText::new",
        "LOG_TEXT_SIZE",
        "log_font_family",
        "ScrollbarMode::Always",
        "FollowAvailability::Available",
    ] {
        assert!(
            frame.contains(required),
            "ADR 0063 shared log frame is missing {required}"
        );
    }
    for forbidden in [
        "gpui",
        "ScrollHandle",
        "chrono::",
        "Utc::now",
        "SystemTime",
        "std::fs",
    ] {
        assert!(
            !reading.contains(forbidden),
            "ADR 0063 reading model contains renderer/I/O/clock dependency {forbidden}"
        );
    }
    for (file, minimum) in [
        ("src/ui/composites/show_log_pane.rs", 1),
        ("src/ui/composites/startup_report.rs", 2),
        ("src/ui/composites/maintenance_forms.rs", 3),
    ] {
        let source = read_source(&manifest_path(file));
        assert!(
            source.matches("LogFrame::new").count() >= minimum,
            "ADR 0063: {file} bypasses the shared log viewport"
        );
        for forbidden in [".child(vm.report", ".child(previous_report"] {
            assert!(
                !source.contains(forbidden),
                "ADR 0063: {file} renders a report on surrounding chrome"
            );
        }
    }
    let app = read_source(&manifest_path("src/app/show.rs"));
    assert!(app.contains("self.tab == AppTab::Show"));
    assert!(app.contains("refresh_request()"));
    assert!(app.contains("bind_service_source(&selected_host)"));
    let root = read_source(&manifest_path("src/app/startup.rs"));
    assert!(
        root.contains("app.log_frames.clone_from(&self.log_frames)"),
        "ADR 0063: retain report reading positions through managed sessions"
    );
}

/// Situational ADR 0074: instruction scrolling shares clearance, while reports have no scrolling ancestor.
#[test]
fn adr_0074_reports_are_separate_from_instruction_scrolling() {
    for file in [
        "src/ui/composites/maintenance_page.rs",
        "src/ui/composites/settings.rs",
    ] {
        let source = read_source(&manifest_path(file));
        assert!(
            source.contains("page_scroll_content(cx)"),
            "ADR 0074: {file} must retain scrollbar clearance"
        );
    }
    let page = read_source(&manifest_path("src/ui/composites/maintenance_page.rs"));
    let instructions = source_between(
        &page,
        "MaintenanceView::Instructions =>",
        "MaintenanceView::Report =>",
    );
    assert!(!instructions.contains("self.report") && !instructions.contains("LogFrame::new"));
    assert!(page.contains("MaintenanceView::Report => self.report"));
    assert!(page.contains("FontSize::Body.scaled(cx)"));
    let settings = read_source(&manifest_path("src/ui/composites/settings.rs"));
    let workspace = source_between(&settings, ".child(if workspace {", "} else {");
    assert!(workspace.contains("overflow_hidden()"));
    assert!(!workspace.contains("overflow_y_scroll"));
    let recovery = read_source(&manifest_path("src/ui/composites/startup_report.rs"));
    let recovery = source_between(&recovery, "pub(crate) fn startup_report(", "#[cfg(test)]");
    assert!(!recovery.contains("overflow_y_scroll"));
    assert!(recovery.contains("match navigation.page"));
}

/// Situational ADR 0066 invariant 9: repair preserves subject identity across all entry points.
#[test]
fn adr_0066_repair_routes_preserve_action_subject() {
    let owner = read_source(&manifest_path("src/application/capability_recovery.rs"));
    let setup = read_source(&manifest_path(
        "src/application/capability_recovery/setup.rs",
    ));
    let context = read_source(&manifest_path("src/application/command_context.rs"));
    let playback = read_source(&manifest_path("src/application/commands/playback.rs"));
    let adapter = read_source(&manifest_path("src/app/capabilities.rs"));
    let search = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let show = read_source(&manifest_path("src/app/show.rs"));
    let keyboard = read_source(&manifest_path("src/app/keyboard.rs"));
    for required in [
        "enum RecoveryAction",
        "struct RecoveryIntent",
        "config_generation",
        "CapabilityRevision::of",
        "self.session.accepts(self.intent.session)",
        "validate_subject",
        "SubjectChanged",
        "PublisherChanged",
    ] {
        assert!(
            owner.contains(required),
            "Missing recovery contract: {required}"
        );
    }
    for source in [&owner, &setup] {
        for forbidden in [
            "use gpui",
            "cx.spawn",
            "Box<dyn Fn",
            "Rc<dyn Fn",
            "load_config_snapshot(",
            "prepare_database(",
        ] {
            assert!(!production_source(source).contains(forbidden));
        }
    }
    assert!(
        context.contains("subject.action.validate_subject")
            || compact_source(&context).contains("subject.action.validate_subject")
    );
    assert_eq!(
        playback
            .matches("context.validate_retry_subject(&conn)?")
            .count(),
        7
    );
    assert!(
        show.matches("context.validate_retry_subject(&conn)?")
            .count()
            >= 2
    );
    for required in [
        "this.retain_failed_action",
        "self.retry_command(",
        "self.command_with_retry(",
    ] {
        assert!(
            show.contains(required),
            "Show adapters must preserve {required}"
        );
    }
    assert!(
        show.contains("intent.action.event_target_name()")
            || compact_source(&show).contains("intent.action.event_target_name()")
    );
    assert!(show.contains("context.validate_retry_event_target("));
    assert!(search.contains("AppToolbarVm::index_search_availability("));
    assert!(search.contains("self.dispatch_index_search("));
    assert!(
        read_source(&manifest_path("src/app.rs")).contains("self.submit_global_search(window, cx)")
    );
    assert!(read_source(&manifest_path("src/app/tab_bar.rs"))
        .contains("this.submit_global_search(window, cx)"));
    assert!(keyboard.contains("self.toggle_playback_paused(cx)"));
    assert!(adapter.contains("editor.open_for(field, window, cx)"));
    assert!(compact_source(&adapter).contains("self.capability_vm.pending.begin_retry("));
    let saved = source_between(
        &adapter,
        "fn check_saved_capabilities(",
        "fn install_checked_capability(",
    );
    assert!(!saved.contains("retry_retained_action") && !saved.contains("present_command"));
    for path in [
        "src/ui/composites/startup_report.rs",
        "src/ui/composites/maintenance_forms.rs",
        "src/ui/composites/settings.rs",
    ] {
        let source = read_source(&manifest_path(path));
        assert!(
            !source.contains("RetryCommand")
                && !source.contains(".execute(")
                && !source.contains("ConfigSnapshot::read_existing")
        );
    }
}

/// Situational ADR 0066: recovery controls distinguish navigation, checks and execution.
#[test]
fn adr_0066_recovery_controls_explain_effect_and_completion() {
    let vm = read_source(&manifest_path("src/view_models/startup/capabilities.rs"));
    let ui = read_source(&manifest_path("src/ui/composites/startup_report.rs"));
    let adapter = read_source(&manifest_path("src/app/capabilities.rs"));
    assert!(vm.contains("entry.completed()"));
    assert!(vm.contains("run_help(&entry.action)"));
    assert!(ui.contains("display.help"));
    assert!(ui.contains("SemanticColor::SecondaryLabel"));
    for forbidden in ["entry.completed()", "Repair action", "Run search again"] {
        assert!(
            !ui.contains(forbidden),
            "Recovery facts and labels belong to the VM: {forbidden}"
        );
    }
    let review = source_between(
        &adapter,
        "CapabilityAction::OpenReport | CapabilityAction::Review(_) =>",
        "CapabilityAction::Repair(id) =>",
    );
    assert!(review.contains("SettingsAction::OpenReport"));
    assert!(!review.contains("retry_retained_action") && !review.contains("check_capability"));
    for path in [
        "src/app/search_dispatch.rs",
        "src/app/playback_bar.rs",
        "src/app/show.rs",
    ] {
        assert!(
            read_source(&manifest_path(path)).contains(".succeed("),
            "{path} must record successful recovery separately"
        );
    }
}

/// Situational ADR 0060: search content and the selected section share navigation.
#[test]
fn adr_0060_search_uses_music_section_navigation() {
    let app = read_source(&manifest_path("src/app.rs"));
    let toolbar = read_source(&manifest_path("src/app/tab_bar.rs"));
    let search = read_source(&manifest_path("src/app/search_dispatch.rs"));
    let submit = source_between(
        &search,
        "pub(super) fn submit_global_search(",
        "pub(super) fn open_search_results_in_content_list(",
    );
    assert!(submit.contains("self.open_search_results_in_content_list(&query, window, cx)"));
    assert!(
        app.contains("cx.subscribe_in(&global_search_input, window, Self::on_global_search_event)")
    );
    assert!(app.contains("self.submit_global_search(window, cx)"));
    assert!(toolbar.contains("this.submit_global_search(window, cx)"));
    let saved = source_between(&app, "fn open_saved_search(", "fn select_tab(");
    assert!(saved.contains("self.open_search_results_in_content_list(query, window, cx)"));
    let open = source_between(
        &search,
        "pub(super) fn open_search_results_in_content_list(",
        "fn search_results_detail_for_query(",
    );
    let empty = open.find("if query.is_empty()").unwrap();
    let select = open
        .find("self.select_tab(AppTab::Music, window, cx)")
        .unwrap();
    let push = open
        .find(".open_search_results_in_content_list(query.clone())")
        .unwrap();
    assert!(
        empty < select && select < push,
        "Ignore empty input, restore Music navigation, then open the query"
    );
    assert!(
        !open.contains("self.tab ="),
        "Use the shared section transition, including history and focus"
    );
}

/// Situational ADR 0066: converter repair owns fresh bounded probes outside UI code.
#[test]
fn adr_0066_converter_checks_are_refreshable() {
    let audio = read_source(&manifest_path("src/audio_format.rs"));
    let probe = read_source(&manifest_path("src/audio_format/probe.rs"));
    let commands = read_source(&manifest_path("src/application/commands/maintenance.rs"));
    let vm = read_source(&manifest_path("src/view_models/startup/correction.rs"));
    let report = read_source(&manifest_path("src/view_models/startup/converter.rs"));
    let presenter = read_source(&manifest_path("src/presentation/configuration_editor.rs"));
    let form = read_source(&manifest_path("src/ui/composites/maintenance_forms.rs"));
    for source in [&audio, &probe] {
        assert!(!source.contains("OnceLock") && !source.contains("LazyLock"));
    }
    for requirement in [
        "Duration::from_secs(5)",
        "CONVERTER_PROBE_OUTPUT_CAP",
        "reader.set_nonblocking(true)",
        "child.kill()",
        "child.wait()",
        "child.try_wait()",
        "recorded_at: chrono::Utc::now()",
    ] {
        assert!(
            probe.contains(requirement),
            "ADR 0066 bounded observation: {requirement}"
        );
    }
    assert!(audio.contains("ConverterObservation::refresh(binary_override)"));
    assert!(audio.contains("if observation.flac.available()"));
    assert!(audio.contains("if observation.ffmpeg.available()"));
    assert!(audio.contains("&observation.ffmpeg.executable"));
    assert!(commands.contains("CorrectionOperation::TestConverter"));
    assert!(commands.contains("ConverterObservation::refresh(path.as_deref())"));
    assert!(commands.contains("source.converter_path(&self.draft)"));
    assert!(commands.contains(".save(&proposed)"));
    assert!(vm.contains("CorrectionAction::TestConverter"));
    assert!(vm.contains("generation != self.generation"));
    assert!(report.contains("probe.recorded_at.format("));
    assert!(presenter.contains("worker.submit(move || command.execute())"));
    assert!(form.contains("vm.action(CorrectionAction::TestConverter)"));
    assert!(form.contains("converter::INSTALLATION"));
    for path in [
        "src/app.rs",
        "src/app/settings.rs",
        "src/app/startup.rs",
        "src/presentation/configuration_editor.rs",
        "src/ui/composites/maintenance_forms.rs",
        "src/view_models/startup/correction.rs",
        "src/view_models/startup/converter.rs",
    ] {
        let source = read_source(&manifest_path(path));
        let source = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "Command::",
            "ConverterObservation::refresh(",
            "ConverterProbe::flac(",
            "ConverterProbe::ffmpeg(",
            "flac_cli_available(",
            "transcode_wav_to_flac(",
        ] {
            assert!(
                !source.contains(forbidden),
                "ADR 0066: {path} must not execute {forbidden}"
            );
        }
    }
    let tracks = read_source(&manifest_path("src/track_compare.rs"));
    let reuse = source_between(
        &tracks,
        "pub fn ensure_taggable_local_path(",
        "pub fn download_track(",
    );
    assert!(
        !reuse.contains("flac_cli_available(None)"),
        "ADR 0066: preserve an explicit executable choice"
    );
}

/// Situational guard: ADR 0066 conversion retry shares materialization and explicit subjects.
#[test]
fn adr_0066_conversion_retry_uses_existing_materialization() {
    let recovery = read_source(&manifest_path("src/application/conversion_recovery.rs"));
    let recovery = recovery.split("#[cfg(test)]").next().unwrap();
    let subscription = read_source(&manifest_path("src/subscribe_service.rs"));
    let materialization = read_source(&manifest_path("src/subscribe_service/materialization.rs"));
    let context = read_source(&manifest_path("src/application/capability_recovery.rs"));
    let commands = read_source(&manifest_path("src/application/commands/download.rs"));
    let transition = read_source(&manifest_path("src/presentation/session_transition.rs"));
    let artifacts = read_source(&manifest_path("src/track_compare/retained.rs"));
    for required in [
        "subscribe_track_retaining",
        "subscribe_feed_retaining",
        "validate_subject",
        "validate_input",
        "entry.operation.run(",
        "ConversionState::RedownloadRequired",
        "state.entries.remove(&id)",
        "playlist_appended",
        "request_key(&entry.original_request)",
    ] {
        assert!(
            recovery.contains(required),
            "ADR 0066: missing recovery contract {required}"
        );
    }
    assert!(subscription.contains("materialization::Materialization::new("));
    assert!(materialization.contains("super::prepare_track_for_subscription_internal("));
    assert!(materialization.contains("download.promote()?"));
    assert!(materialization.contains("download.undo_promotion(&working_path)?"));
    assert!(materialization.contains("db::delete_local_file(&transaction, previous)?"));
    assert!(context.contains("RecoveryAction::Conversion"));
    assert!(commands.contains("struct RetryConversion"));
    assert!(transition
        .split_whitespace()
        .collect::<String>()
        .contains("conversions.close()"));
    assert!(artifacts.contains("fingerprint(path)?"));
    assert!(artifacts.contains("metadata.ino()"));
    for forbidden in [
        "gpui",
        "cx.spawn",
        "transcode_wav_to_flac(",
        "format_warning.contains(",
    ] {
        assert!(
            !recovery.contains(forbidden),
            "ADR 0066: application recovery must not contain {forbidden}"
        );
    }
    let maintenance = read_source(&manifest_path("src/application/commands/maintenance.rs"));
    for forbidden in ["RetryConversion", "subscribe_track", "conversion_recovery"] {
        assert!(
            !maintenance.contains(forbidden),
            "ADR 0066: configuration Save cannot replay conversion"
        );
    }
}

#[test]
fn adr_0066_database_checks_and_snapshots_have_one_owner() {
    // Situational: ADR 0066 invariant 6 and ADR 0016's schema authority.
    // Behavioral db::maintenance tests prove preservation, WAL data and contention.
    let database = read_source(&manifest_path("src/db/maintenance.rs"));
    let production = production_source(&database);
    for required in [
        "SQLITE_OPEN_READ_ONLY",
        "Backup::new",
        "backup.step(BACKUP_PAGE_BATCH)",
        "MAINTENANCE_DEADLINE",
        "progress_handler",
        "create_new(true)",
        "fs::hard_link",
        "valid_snapshot()",
        "candidate.cleanup()",
    ] {
        assert!(
            compact_source(production).contains(&compact_source(required)),
            "missing database maintenance protection: {required}"
        );
    }
    for forbidden in [
        "open_db(",
        "init_schema(",
        "migrate_schema(",
        "run_to_completion(",
        "fs::copy(",
        "fs::rename(",
        "RuntimeHost",
        "TopApp",
        "use gpui",
    ] {
        assert!(
            !production.contains(forbidden),
            "database maintenance bypass: {forbidden}"
        );
    }
    let registry = read_source(&manifest_path("src/db.rs"));
    assert!(registry.contains("fn inspect_schema("));
    let inspection = source_between(&registry, "fn inspect_schema(", "const BASE_READS:");
    assert!(inspection.contains("MIGRATIONS"));
    for forbidden in [
        "init_schema(",
        "migrate_schema(",
        "record_migration(",
        ".execute(",
    ] {
        assert!(!inspection.contains(forbidden));
    }
    let startup = read_source(&manifest_path("src/db/startup.rs"));
    assert!(startup.contains("super::inspect_schema(conn)"));
    let command = read_source(&manifest_path("src/application/commands/maintenance.rs"));
    assert!(command.contains("maintenance::inspect("));
    assert!(command.contains("maintenance::backup("));
    let presenter = read_source(&manifest_path("src/presentation/database_tools.rs"));
    assert!(presenter.contains("worker.submit(move || command.execute())"));
    for forbidden in ["RuntimeHost", "TopApp", "Connection::", "fs::"] {
        assert!(!presenter.contains(forbidden));
    }
    for path in ["src/app/settings.rs", "src/app/startup.rs"] {
        assert!(read_source(&manifest_path(path)).contains("database_tools"));
    }
    let form = read_source(&manifest_path("src/ui/composites/maintenance_forms.rs"));
    assert!(form.contains("LogSource::Database"));
    assert!(form.contains("DatabaseVm::SCOPE"));
    let fixture = read_source(&manifest_path("docs/runbooks/startup-recovery-fixture.py"));
    assert!(fixture.contains("database-seed"));
    assert!(
        !fixture.contains("CREATE TABLE"),
        "Python fixture must not duplicate the schema registry"
    );
}

/// Situational ADR 0066 invariant 6: drain authority and SQLite exclusion precede file copying.
#[test]
fn adr_0066_database_maintenance_requires_exclusive_access() {
    let path = "src/db/maintenance/preservation.rs";
    let source = read_source(&manifest_path(path));
    let backend = production_source(&source);
    for required in [
        "struct ExclusiveDatabase",
        "connection: Connection",
        "files: Vec<SourceFile>",
        "EXCLUSIVE_ACCESS_DEADLINE: Duration = Duration::from_secs(5)",
        "SQLITE_OPEN_READ_WRITE",
        "SQLITE_OPEN_NOFOLLOW",
        "PRAGMA main.locking_mode=EXCLUSIVE",
        "BEGIN EXCLUSIVE",
        "ROLLBACK",
        "PRAGMA main.journal_mode",
        "mode(0o700)",
        "mode(0o600)",
        "create_new(true)",
        "copy_and_hash",
        "source.file",
        "sync_all()",
        "sha256",
        "database_file_preservation_not_verified_backup",
        "failure.remaining.push(destination)",
    ] {
        assert!(
            compact_source(backend).contains(&compact_source(required)),
            "ADR 0066 preservation missing: {required}"
        );
    }
    assert!(
        backend.find("connection: Connection").unwrap()
            < backend.find("files: Vec<SourceFile>").unwrap(),
        "SQLite must close before raw descriptors release POSIX inode locks"
    );
    assert!(backend.contains("pub(crate) fn preserve(\n        &mut self,"));
    for forbidden in [
        "SQLITE_OPEN_CREATE",
        "fs::copy(",
        "fs::rename(",
        "fs::remove_file(",
        "fs::remove_dir(",
        "open_db(",
        "migrate_schema(",
        "fs::read(",
        "File::open(&self.source)",
    ] {
        assert!(
            !backend.contains(forbidden),
            "ADR 0066 preservation bypass: {forbidden}"
        );
    }
    let command = read_source(&manifest_path("src/application/commands/maintenance.rs"));
    let entry = source_between(
        &command,
        "pub(crate) fn execute_preservation(",
        "pub(crate) fn execute(self) -> DatabaseResult",
    );
    for required in [
        "MaintenanceSession",
        "session.is_ready()",
        "ExclusiveDatabase::acquire",
        "access.preserve",
        "DatabaseOutcome::Preserved",
    ] {
        assert!(entry.contains(required));
    }
    for file in rust_files_under("src") {
        let contents = read_source(&file);
        let production = production_source(&contents);
        if production.contains("ExclusiveDatabase::acquire(") {
            assert!(
                file.ends_with("application/commands/maintenance.rs")
                    || file.ends_with("db/maintenance/upgrade.rs"),
                "exclusive entry bypassed maintenance or pre-session preparation: {}",
                file.display()
            );
        }
        if production.contains("SessionDrain::core_recovery()") {
            assert!(
                file.ends_with("app/startup.rs"),
                "empty recovery authority escaped startup: {}",
                file.display()
            );
        }
    }
    let screen = read_source(&manifest_path("src/app/startup.rs"));
    for required in [
        "self.maintenance.take()",
        "command.execute_preservation(session)",
        "this.maintenance = Some(session)",
        "this.request(StartupAction::CheckAgain, CheckIntent::Check",
        "this.normal.is_none()",
    ] {
        assert!(screen.contains(required));
    }
    for test in [
        "adr_0066_exclusive_modes_exclude_processes_through_each_copy_and_release",
        "adr_0066_real_readers_and_writers_bound_acquisition_without_copying",
        "adr_0066_sqlite_journal_recovery_is_observed_before_preservation",
    ] {
        assert!(source.contains(test));
    }
}

/// Situational — ADR 0066 invariant 6: explicit review, preservation and verified installation.
#[test]
fn adr_0066_restore_uses_validated_maintenance_install() {
    let source = read_source(&manifest_path("src/db/maintenance/restore.rs"));
    let backend = production_source(&source);
    for required in [
        "struct ValidatedRestore",
        "candidate: Candidate",
        "candidate_revision: FileRevision",
        "destination_contents: Option<String>",
        "access: ExclusiveDatabase",
        "self.revalidate_inputs(budget)?",
        "access.source() != self.destination",
        "access.preserve(&self.preservation, budget)",
        "build_snapshot(&access.connection",
        "copy_into(&source, &mut access.connection",
        "Backup::new(source, destination)",
        "backup.step(BACKUP_PAGE_BATCH)",
        "self.verify_installed",
        "crate::db::startup::check_connection",
        "rollback_verified",
        "InstallState::VerificationFailed",
    ] {
        assert!(
            compact_source(backend).contains(&compact_source(required)),
            "ADR 0066 restore missing: {required}"
        );
    }
    for forbidden in [
        "fs::rename(",
        "fs::remove_file(",
        "fs::copy(",
        "open_db(",
        "SQLITE_OPEN_CREATE",
        "File::open(&self.destination)",
    ] {
        assert!(
            !backend.contains(forbidden),
            "ADR 0066 restore bypass: {forbidden}"
        );
    }
    assert!(
        backend.find("access.preserve(&self.preservation").unwrap()
            < backend.find("let installed = copy_into").unwrap()
    );
    assert!(
        backend.find("let installed = copy_into").unwrap()
            < backend.find("self.verify_installed").unwrap()
    );
    let command = read_source(&manifest_path("src/application/commands/maintenance.rs"));
    let entry = source_between(
        &command,
        "pub(crate) fn execute_restore(",
        "pub(crate) fn execute_preservation(",
    );
    for required in [
        "MaintenanceSession",
        "review.validate(&session)",
        "review.candidate.revalidate",
        "ExclusiveDatabase::acquire",
        "review.candidate.install",
    ] {
        assert!(
            compact_source(entry).contains(&compact_source(required)),
            "ADR 0066 command missing: {required}"
        );
    }
    for file in rust_files_under("src") {
        let contents = read_source(&file);
        let production = production_source(&contents);
        if production.contains(".execute_restore(") {
            assert!(
                file.ends_with("app/startup.rs"),
                "restore dispatch escaped startup lifecycle: {}",
                file.display()
            );
        }
        if production.contains("candidate.install(") {
            assert!(
                file.ends_with("application/commands/maintenance.rs"),
                "restore bypassed maintenance authority: {}",
                file.display()
            );
        }
    }
    let screen = read_source(&manifest_path("src/app/startup.rs"));
    for required in [
        "DatabaseEvent::Restore",
        "pending_restore.take()",
        "command.execute_restore(session)",
        "this.maintenance = Some(session)",
        "InstallState::Verified",
        "this.action(StartupAction::OpenApp",
        "std::mem::take(&mut this.resume_after_restore)",
    ] {
        assert!(screen.contains(required));
    }
    for test in [
        "adr_0066_restore_preserves_installs_and_verifies_without_replacing_inode",
        "adr_0066_restore_interrupted_and_cancelled_steps_verify_sqlite_rollback",
        "adr_0066_restore_changed_files_or_records_require_new_review",
        "adr_0066_restore_verification_requires_candidate_records_and_usable_writes",
    ] {
        assert!(source.contains(test));
    }
}

/// Situational — ADR 0016 and ADR 0066 invariant 6: one migration authority,
/// explicit candidate preparation and the existing maintenance installation.
#[test]
fn adr_0066_upgrade_repair_uses_normal_migration_authority() {
    let db = read_source(&manifest_path("src/db.rs"));
    let maintenance = read_source(&manifest_path("src/db/maintenance/restore.rs"));
    let backend = production_source(&maintenance);
    assert!(db.contains("migrate_schema_with(conn, target, |_, _| Ok(()))"));
    assert!(db.contains("record_migration(conn, migration.version, migration.name)?"));
    assert!(backend.contains("super::super::migrate_schema_to(&candidate, target)"));
    assert_eq!(backend.matches("migrate_schema_to(").count(), 1);
    let repair = source_between(
        backend,
        "pub(crate) fn repair_interrupted_upgrade(",
        "fn migrate_candidate(",
    );
    assert!(repair.find("access.preserve(").unwrap() < repair.find("migrate_candidate(").unwrap());
    for required in [
        "build_snapshot(&access.connection",
        "database_digest(&candidate_connection",
        "review.install_prepared",
        "SchemaCompatibility::InterruptedUpgrade",
    ] {
        assert!(
            repair.contains(required),
            "repair authority missing {required}"
        );
    }
    for forbidden in [
        "INSERT INTO schema_migrations",
        "DELETE FROM schema_migrations",
        "record_migration(",
        "CREATE TABLE",
    ] {
        assert!(
            !backend.contains(forbidden),
            "parallel migration authority: {forbidden}"
        );
    }
    let recognizer = read_source(&manifest_path("src/db/upgrades.rs"));
    let recognition = source_between(&recognizer, "fn recognize_migration_11(", "type Column");
    assert!(!recognition.contains("conn.execute"));
    let startup = read_source(&manifest_path("src/db/startup.rs"));
    assert!(startup.contains("SchemaCompatibility::InterruptedUpgrade => Err"));
    let vm = read_source(&manifest_path("src/view_models/startup/database.rs"));
    for required in [
        "DatabaseAction::UpgradeBackup",
        "DatabaseAction::RepairUpgrade",
        "inspection.valid_snapshot()",
        "self.maintenance_ready",
    ] {
        assert!(vm.contains(required));
    }
    for proof in [
        "adr_0066_upgrade_repair_preserves_before_install_and_verifies_failure_rollback",
        "adr_0066_upgrade_backup_requires_explicit_candidate_and_preserves_chosen_source",
    ] {
        assert!(maintenance.contains(proof));
    }
}

/// ADR 0039 task 001 geometry consumers that resolve through the CHROME
/// domain (their `.multiplier()` call site is listed in the task packet).
const ADR_0039_CHROME_GEOMETRY_FILES: &[&str] = &[
    "src/ui/layouts.rs",
    "src/ui/icons.rs",
    "src/ui/primitives/image.rs",
    "src/ui/composites/thumbnail.rs",
    "src/ui/composites/detail_grid.rs",
    "src/ui/composites/show_log_pane.rs",
    "src/ui/composites/log_frame.rs",
    "src/ui/composites/maintenance_page.rs",
    "src/app/show.rs",
    "src/ui/sizable_bridge.rs",
];

/// Situational guard for ADR 0039 tasks 001-003.
/// `FontSize` must use the type resolver.
/// `Spacing`, `Radius`, `Size`, `SkeletonBlock` and geometry bridges must use the chrome resolver.
/// Neither domain may call the other domain's resolver.
/// An unused helper does not satisfy this requirement.
/// The retired uniform `ScaleFactor::multiplier` must not return.
///
/// Task 003 extended this guard on 2026-09-18 to close review finding R2.
/// The earlier guard searched the complete `FontSize` implementation for declarations and calls.
/// It did not require `scaled_px` itself to call `type_multiplier`.
/// This guard checks each function's body separately.
/// `scaled_px` must call `self.type_multiplier`.
/// `scaled` must call `self.scaled_px`.
/// The guard also requires each ratified endpoint arm and rejects the retired identity helper.
/// See docs/reviews/adr-0039-review-checklist.md.
#[test]
fn adr_0039_type_and_chrome_domains_stay_live_and_separate() {
    const FIX: &str = "Fix: route FontSize through `type_multiplier`/`scaled_px` and Spacing/Radius/Size/SkeletonBlock plus the geometry bridges through `chrome_multiplier`/`scale_chrome_px` in src/ui/tokens.rs (ADR 0039).";

    let tokens = read_source(&manifest_path("src/ui/tokens.rs"));
    let mut violations = Vec::new();

    if tokens.contains("fn multiplier(") {
        violations.push(format!(
            "src/ui/tokens.rs: ADR 0039 the retired uniform `ScaleFactor::multiplier` must not be reintroduced. {FIX}"
        ));
    }

    // FontSize owns the TYPE domain: `scaled` must call `scaled_px`, which
    // must call `type_multiplier`, and neither may reach for CHROME.
    let font_size_impl = source_between(&tokens, "impl FontSize {", "/// Type weight token");
    const SCALED_PX_DECL: &str = "fn scaled_px(self, scale: ScaleFactor) -> Pixels {";
    const SCALED_DECL: &str = "fn scaled(self, cx: &App) -> Pixels {";
    for required in [
        "fn type_multiplier(self, scale: ScaleFactor) -> f32 {",
        SCALED_PX_DECL,
        SCALED_DECL,
    ] {
        if !font_size_impl.contains(required) {
            violations.push(format!(
                "src/ui/tokens.rs: ADR 0039 `impl FontSize` is missing TYPE resolver wiring `{required}`. {FIX}"
            ));
        }
    }

    // Review R2: require each call in the body of the function that must make it.
    // A call elsewhere in the implementation does not satisfy this check.
    if font_size_impl.contains(SCALED_PX_DECL) && font_size_impl.contains(SCALED_DECL) {
        let scaled_px_body = source_between(font_size_impl, SCALED_PX_DECL, SCALED_DECL);
        if !scaled_px_body.contains("self.type_multiplier(") {
            violations.push(format!(
                "src/ui/tokens.rs: ADR 0039 (review R2) `scaled_px` must call `self.type_multiplier(scale)` \
                 in its own body. This call keeps font sizes on the per-role type curve. {FIX}"
            ));
        }

        let scaled_start = font_size_impl
            .find(SCALED_DECL)
            .expect("checked by the `contains` guard above");
        let scaled_body = &font_size_impl[scaled_start..];
        if !scaled_body.contains("self.scaled_px(") {
            violations.push(format!(
                "src/ui/tokens.rs: ADR 0039 (review R2) `scaled` must call `self.scaled_px(...)` in its own \
                 body. Check this call separately from the call inside `scaled_px`. {FIX}"
            ));
        }
    }

    // ADR 0039 task 003 M2 requires the endpoint pairs ratified on 2026-09-18.
    // Each `(x-small, x-large)` pair must appear verbatim for its role.
    // The retired identity helper must not return.
    const RATIFIED_TYPE_ENDPOINTS: &[(&str, &str)] = &[
        ("Micro", "(0.91, 1.36)"),
        ("Caption", "(0.90, 1.32)"),
        ("Body", "(0.89, 1.28)"),
        ("Headline", "(0.88, 1.24)"),
        ("Title3", "(0.87, 1.20)"),
        ("Title2", "(0.86, 1.16)"),
        ("Title", "(0.85, 1.12)"),
    ];
    for (role, pair) in RATIFIED_TYPE_ENDPOINTS {
        let arm = format!("Self::{role} => {pair},");
        if !font_size_impl.contains(&arm) {
            violations.push(format!(
                "src/ui/tokens.rs: ADR 0039 task 003 `type_endpoints` is missing the ratified arm \
                 `{arm}`. The type resolver must use the operator's ratified values. {FIX}"
            ));
        }
    }
    if font_size_impl.contains("Self::identity_step(") {
        violations.push(format!(
            "src/ui/tokens.rs: ADR 0039 task 003 `impl FontSize` must not reintroduce task 001's \
             identity placeholder `Self::identity_step`. {FIX}"
        ));
    }

    // Only flag actual calls (`name(`), not the doc-comment cross-reference
    // to `ScaleFactor::chrome_multiplier` in the TYPE resolver's own comment.
    for forbidden in ["chrome_multiplier(", "scale_chrome_px("] {
        if font_size_impl.contains(forbidden) {
            violations.push(format!(
                "src/ui/tokens.rs: ADR 0039 `FontSize` must not call the CHROME resolver `{forbidden}`. {FIX}"
            ));
        }
    }

    // Spacing, Radius, Size and SkeletonBlock own the CHROME domain: each
    // `scaled` must call `scale_chrome_px` and never reach for TYPE.
    for (label, start, end) in [
        ("Spacing", "impl Spacing {", "impl Radius {"),
        ("Radius", "impl Radius {", "// ---"),
        ("Size", "impl Size {", "// ---"),
        ("SkeletonBlock", "impl SkeletonBlock {", "// ---"),
    ] {
        let block = source_between(&tokens, start, end);
        if !block.contains("scale_chrome_px(") {
            violations.push(format!(
                "src/ui/tokens.rs: ADR 0039 `impl {label}` must resolve through `scale_chrome_px`. {FIX}"
            ));
        }
        for forbidden in ["type_multiplier(", "scaled_px("] {
            if block.contains(forbidden) {
                violations.push(format!(
                    "src/ui/tokens.rs: ADR 0039 `impl {label}` must not call the TYPE resolver `{forbidden}`. {FIX}"
                ));
            }
        }
    }

    // `scale_chrome_px` itself must close the loop onto `chrome_multiplier`.
    let chrome_resolver = source_between(&tokens, "fn scale_chrome_px(", "const fn hex(");
    if !chrome_resolver.contains("chrome_multiplier(") {
        violations.push(format!(
            "src/ui/tokens.rs: ADR 0039 `scale_chrome_px` must call `ScaleFactor::chrome_multiplier`. {FIX}"
        ));
    }
    if chrome_resolver.contains("type_multiplier(") {
        violations.push(format!(
            "src/ui/tokens.rs: ADR 0039 `scale_chrome_px` must not call the TYPE resolver. {FIX}"
        ));
    }

    // The eleven direct geometry call sites (ADR 0039 task 001) must resolve
    // through CHROME and never acquire a font curve.
    for file in ADR_0039_CHROME_GEOMETRY_FILES {
        let source = read_source(&manifest_path(file));
        if !source.contains(".chrome_multiplier()") {
            violations.push(format!(
                "{file}: ADR 0039 geometry call sites must resolve through `ScaleFactor::chrome_multiplier`. {FIX}"
            ));
        }
        if source.contains("type_multiplier(") {
            violations.push(format!(
                "{file}: ADR 0039 geometry must not acquire a font curve; found a TYPE-domain reference. {FIX}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0039 domain-ownership violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0039 task 002 M2: the two hard fixed-height caps —
/// ShowCard's `.h(Size::MenuCompact.scaled(cx))` and Button's
/// `.h(self.height(cx))` — call the shared reservation geometry this task
/// lands in `src/ui/layouts.rs`, rather than an ad hoc capacity check local
/// to either file. Removing the call fails this guard.
#[test]
fn adr_0039_capped_surfaces_use_the_shared_reservation() {
    const FIX: &str = "Fix: call `layouts::show_card_summary_reservation`/`show_card_available_inner_height` (ShowCard) or `layout::button_label_reservation`/`layout::available_inner_height` (Button) — ADR 0039 task 002's shared reservation owner in src/ui/layouts.rs — rather than an ad hoc capacity check.";

    let show_card = read_source(&manifest_path("src/ui/composites/show_card.rs"));
    let button = read_source(&manifest_path("src/ui/primitives/button.rs"));
    let mut violations = Vec::new();

    for required in [
        "layouts::show_card_summary_reservation(",
        "layouts::show_card_available_inner_height(",
    ] {
        if !show_card.contains(required) {
            violations.push(format!(
                "src/ui/composites/show_card.rs: ADR 0039 task 002 missing reservation call `{required}`. {FIX}"
            ));
        }
    }
    for required in [
        "layout::button_label_reservation(",
        "layout::available_inner_height(",
    ] {
        if !button.contains(required) {
            violations.push(format!(
                "src/ui/primitives/button.rs: ADR 0039 task 002 missing reservation call `{required}`. {FIX}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0039 task 002 reservation-ownership violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0039 task 002 M4: `render_summary_line`
/// (`src/ui/composites/show_card.rs`) matches the playlist/now-playing
/// single-line pattern — a silent clip via `whitespace_nowrap()` +
/// `overflow_hidden()`, never `.truncate()`. `.truncate()` here would fail
/// `adr_0063_column_text_does_not_truncate`: this div chain has none of
/// `flex_1()`/`max_w(`/`.w(`. Mechanical, not visual — the defect and its
/// correction are independent of scale.
#[test]
fn adr_0039_show_card_summary_line_matches_single_line_pattern() {
    const FIX: &str = "Fix: give `render_summary_line` (src/ui/composites/show_card.rs) `whitespace_nowrap()` + `overflow_hidden()`, following the playlist row (src/ui/shells/playlist.rs:754-755,764-765). Do not use `.truncate()` there — ADR 0039 task 002.";

    let source = read_source(&manifest_path("src/ui/composites/show_card.rs"));
    let function = code_only(source_between(
        &source,
        "fn render_summary_line(",
        "const fn show_card_id(",
    ));
    let mut violations = Vec::new();

    for required in [".whitespace_nowrap()", ".overflow_hidden()"] {
        if !function.contains(required) {
            violations.push(format!(
                "src/ui/composites/show_card.rs: render_summary_line is missing `{required}`. {FIX}"
            ));
        }
    }
    if function.contains(".truncate()") {
        violations.push(format!(
            "src/ui/composites/show_card.rs: render_summary_line must not call `.truncate()`. {FIX}"
        ));
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0039 task 002 ShowCard single-line violations:\n{}",
        violations.join("\n")
    );
}

/// Situational ADR 0039 task 002 M2: the FLOOR/uncapped fixed-height row
/// consumers named alongside ShowCard and Button — `TrackRow`, the queue row
/// and the playlist row — keep the single-line text discipline that makes
/// their line count independent of string length: `.truncate()` inside a
/// `flex_1()`/definite-width ancestor (`Label::truncated()`, or a plain
/// `.truncate()` call), or `whitespace_nowrap()` + `overflow_hidden()`.
/// Removing that discipline, so title text could wrap and grow the row,
/// fails this guard.
#[test]
fn adr_0039_fixed_height_rows_keep_single_line_text() {
    const FIX: &str = "Fix: keep title/artist text single-line — `.truncate()` inside a `flex_1()`/`.w(`/`max_w(` ancestor, or `whitespace_nowrap()` + `overflow_hidden()` — ADR 0039 task 002.";

    let track_row = code_only(&read_source(&manifest_path(
        "src/ui/composites/track_row.rs",
    )));
    let queue = read_source(&manifest_path("src/ui/shells/queue_now_playing.rs"));
    let playlist = read_source(&manifest_path("src/ui/shells/playlist.rs"));
    let mut violations = Vec::new();

    if !track_row.contains(".truncated()") {
        violations.push(format!(
            "src/ui/composites/track_row.rs: the track title must stay single-line via `Label::truncated()`. {FIX}"
        ));
    }

    let queue_row = code_only(source_between(
        &queue,
        "fn render_queue_row(",
        "fn render_control_deck(",
    ));
    if queue_row.matches(".truncate()").count() < 2 {
        violations.push(format!(
            "src/ui/shells/queue_now_playing.rs: render_queue_row's title/artist must stay single-line via `.truncate()`. {FIX}"
        ));
    }

    let playlist_body = code_only(source_between(
        &playlist,
        "fn render_playlist_track_body(",
        "fn render_playlist_thumb_placeholder(",
    ));
    if playlist_body.matches(".whitespace_nowrap()").count() < 2
        || playlist_body.matches(".overflow_hidden()").count() < 2
    {
        violations.push(format!(
            "src/ui/shells/playlist.rs: render_playlist_track_body's title/artist must stay single-line via `whitespace_nowrap()` + `overflow_hidden()`. {FIX}"
        ));
    }

    assert!(
        violations.is_empty(),
        "Situational ADR 0039 task 002 fixed-height row single-line violations:\n{}",
        violations.join("\n")
    );
}

/// Situational — ADR 0075 packet 012: guarded preparation and bounded migration ownership.
#[test]
fn adr_0075_migration_registry_preservation_and_receipts_have_live_callers() {
    let db = read_source(&manifest_path("src/db.rs"));
    let startup = read_source(&manifest_path("src/db/startup.rs"));
    let owner = read_source(&manifest_path("src/db/maintenance/upgrade.rs"));
    let backend = production_source(&owner);
    assert!(
        source_between(&db, "pub fn open_db(", "pub(crate) mod maintenance")
            .contains("startup::prepare_database(db_path)")
    );
    assert!(startup.contains("super::maintenance::upgrade::prepare(path)"));
    for required in [
        "probe.close()",
        "ExclusiveDatabase::acquire",
        "access.preserve",
        "build_snapshot(&access.connection",
        "file.sync_all()",
        "migrate_schema_with",
        "content_digest",
        "PreparationState::RollbackVerified",
        "PreparationState::VerificationFailed",
        "drop(access)",
        "startup::open_existing",
    ] {
        assert!(
            compact_source(backend).contains(&compact_source(required)),
            "migration preparation missing {required}"
        );
    }
    assert!(
        compact_source(backend).find("probe.close()").unwrap()
            < backend.find("ExclusiveDatabase::acquire").unwrap()
    );
    assert!(
        backend.find("preserve_original(&mut access").unwrap()
            < backend.find("crate::db::migrate_schema_to").unwrap()
    );
    let initializer = source_between(
        &db,
        "pub(crate) fn init_schema(",
        "fn create_identity_source_fact_tables",
    );
    assert!(!initializer.contains("provider_snapshot_schema"));
    assert!(!initializer.contains("create_broadcast_event_selection_table"));
    let registry = source_between(&db, "const MIGRATIONS:", "/// Read compatibility facts");
    assert!(registry.contains("provider_snapshot_schema::apply"));
    assert_eq!(
        production_source(&db)
            .matches("INSERT INTO schema_migrations")
            .count(),
        1
    );
    let core = read_source(&manifest_path("src/startup.rs"));
    let bootstrap = read_source(&manifest_path("src/app/bootstrap.rs"));
    let cli = read_source(&manifest_path("src/cli.rs"));
    assert!(core.contains("preparation_receipt: prepared.receipt"));
    assert!(bootstrap.contains("preparation_report(&preparation_receipt)"));
    assert!(cli.contains("preparation_report(&prepared.receipt)"));
    assert!(cli.contains("preparation_failure_report(preparation)"));
    let callbacks = read_source(&manifest_path("src/app/startup.rs"));
    assert!(callbacks.contains("self.vm.take_normal_session_report("));
    assert!(callbacks.contains("normal.read(cx).previous_session_report.clone()"));
    assert!(callbacks.contains("StartupReportVm::drain_report("));
    let restore = read_source(&manifest_path("src/db/maintenance/restore.rs"));
    assert!(restore.contains("migrate_candidate(&candidate.path, 11, budget)"));
    assert!(restore.contains("verify_target(&access.connection, self.target)"));
    let review = source_between(&restore, "fn review_with(", "pub(crate) fn candidate_path(");
    assert!(review.contains("verify_target(&completed, super::super::CURRENT_VERSION)"));
}

/// Situational: ADR 0075 packets 013 and 014 share one response transaction and Library root.
#[test]
fn adr_0075_observation_writer_and_library_retention_have_one_owner() {
    let writer = read_source(&manifest_path("src/db/provider_observations.rs"));
    let writer = production_source(&writer);
    for forbidden in ["DELETE FROM"] {
        assert!(
            !writer.contains(forbidden),
            "ADR 0075 observation writer exceeded its packet: {forbidden}"
        );
    }
    assert_eq!(writer.matches("tx.commit()").count(), 2);
    let query = read_source(&manifest_path("src/application/queries/library.rs"));
    let selected = source_between(
        &query,
        "pub(crate) fn fetch_library_track_context_with_local_fallback(",
        "pub(crate) fn apply_local_track_metadata_defaults(",
    );
    assert!(selected.contains("ProviderObservationRecorder::new"));
    assert!(selected.contains("take_receipts()"));
    assert!(selected.contains("ObservationWriteFailure"));
    let app = read_source(&manifest_path("src/library/app_impl.rs"));
    let callback = source_between(
        &app,
        "fn load_track_source_context(",
        "fn track_breadcrumb_display(",
    );
    assert!(callback.contains("retain_observation_failure(&error)"));
    let vm = read_source(&manifest_path("src/view_models/library.rs"));
    let retention = source_between(
        &vm,
        "pub(crate) fn retain_observation_failure(",
        "pub(crate) fn is_resizing(",
    );
    assert!(!retention.contains("set_error_status("));
    assert!(!retention.contains(".clear()"));
    let capture = read_source(&manifest_path("src/provider_observation/http.rs"));
    assert!(capture.contains("request.timeout(timeout).send()"));
    assert!(capture.contains("crate::http_client::document_timeout()"));
    let rss = read_source(&manifest_path("src/rss/enrich.rs"));
    assert_eq!(
        production_source(&rss).matches("Document::parse(").count(),
        1
    );
}

/// Situational: ADR 0075 packet 038 retains Library reader evidence before selection checks.
#[test]
fn adr_0075_library_observation_callers_and_consumers_are_guarded() {
    let query = read_source(&manifest_path("src/application/queries/library.rs"));
    let hydration = source_between(
        &query,
        "fn hydrate_album_identity_facts(",
        "fn compare_library_track(",
    );
    assert!(hydration.contains("with_observation_recorder(Some(Arc::clone(&recorder)))"));
    assert!(hydration.contains("assemble_observed_query("));
    assert!(hydration.contains("owner_receipts.into_inner()"));
    assert!(
        hydration.find("fetch_feed_with_profile(").unwrap()
            < hydration.find("persist_musicindex_feed(").unwrap()
    );
    let comparison = source_between(
        &query,
        "fn compare_library_track(",
        "fn assemble_observed_query<",
    );
    assert!(comparison.contains("fetch_library_track_context_with_recorder("));
    assert!(comparison.contains("local_provider_request("));
    assert!(comparison.contains("assemble_provider_context("));
    // Packet 018 Part B: the call now also passes `detail_receipts`, so
    // `cargo fmt` wraps it across lines. The check reads past the call's
    // start instead of matching one contiguous, formatting-sensitive line.
    let query_call = comparison
        .find("assemble_observed_query(")
        .map(|start| &comparison[start..]);
    assert!(query_call.is_some_and(|text| text.contains("&recorder,") && text.contains("result,")));
    assert!(!comparison.contains("fetch_library_track_context_with_local_fallback"));
    let assembly = source_between(&query, "fn assemble_observed_query<", "fn poisoned_lock(");
    assert_eq!(assembly.matches("take_receipts()").count(), 1);
    assert!(assembly.contains("CommandError::ObservedQueryFailure"));
    assert!(assembly.contains("CommandError::ObservationWriteFailure"));
    let app = read_source(&manifest_path("src/library/app_impl.rs"));
    for (start, end, retention, selection) in [
        (
            "fn apply_library_comparison_result(",
            "fn apply_library_comparison_failure(",
            "retain_observation_receipts",
            "if let LibraryDetail::Track",
        ),
        (
            "fn apply_library_comparison_failure(",
            "fn apply_library_hydration_result(",
            "retain_library_query_failure",
            "if let LibraryDetail::Track",
        ),
        (
            "fn apply_library_hydration_result(",
            "fn feed_check_route_repair_outcome(",
            "retain_observation_receipts",
            "if let LibraryDetail::Album",
        ),
    ] {
        let callback = source_between(&app, start, end);
        assert!(callback.find(retention).unwrap() < callback.find(selection).unwrap());
    }
    let callbacks = source_between(
        &app,
        "fn start_compare_library_track(",
        "pub(crate) fn toggle_musicbrainz_lookup(",
    );
    assert!(callbacks.contains("Arc::clone(&self.conn)"));
    assert!(callbacks.contains("apply_library_comparison_result("));
    assert!(callbacks.contains("apply_library_comparison_failure("));
    let hydration_callback = source_between(
        &app,
        "fn hydrate_album_identity_on_view(",
        "pub(crate) fn select_artist(",
    );
    assert!(hydration_callback.contains("apply_library_hydration_result("));
    assert!(hydration_callback.contains("retain_library_query_failure("));
    assert!(hydration_callback.contains("album_has_feed_identity_actions"));
    assert!(hydration_callback.contains("album.description.is_some()"));
    assert!(hydration_callback.contains("!album.metadata_facts.is_empty()"));
    let cache = source_between(
        &app,
        "pub(crate) fn toggle_tag_compare(",
        "fn start_compare_library_track(",
    );
    assert!(cache.contains("LazyPanel::Loaded(_)"));
    assert_eq!(cache.matches("self.reload_tag_compare(cx)").count(), 2);
    for path in [
        "src/application/queries/search.rs",
        "src/application/commands/feed.rs",
        "src/cli.rs",
    ] {
        assert!(!production_source(&read_source(&manifest_path(path)))
            .contains("with_observation_recorder("));
    }
}

/// Situational: ADR 0075 requires verified replacement and live typed local reads.
#[test]
fn adr_0075_snapshot_registry_transaction_and_local_read_boundaries() {
    let registry = read_source(&manifest_path("src/provider_observation/contracts.rs"));
    let production = production_source(&registry);
    assert!(production.contains("struct VerifiedCoverage"));
    assert!(production.contains("struct DecodedRssBody"));
    assert!(production.contains("decoded.text() != owner.document().input_text()"));
    assert!(production.contains("ProviderKind::Rss"));
    assert!(!production.contains("ProviderKind::MusicIndex"));
    let writer = read_source(&manifest_path("src/db/provider_observations.rs"));
    let response = source_between(
        &writer,
        "fn record(
",
        "fn read_values(",
    );
    assert!(response.contains("contracts::validate(&token.spec, observation)?"));
    assert!(response.contains("replace_collections(tx, token, observation, id, superseded)?"));
    assert!(!response.contains(".commit()"));
    let query = read_source(&manifest_path("src/application/queries/library.rs"));
    let assembly = source_between(
        &query,
        "fn assemble_provider_context(",
        "pub(crate) fn apply_local_track_metadata_defaults(",
    );
    assert!(assembly.contains("read_track_provider_state"));
    assert!(assembly.contains("write_failure: capsule"));
    assert!(assembly.contains("receipts: receipts.into()"));
    let rss = read_source(&manifest_path("src/rss/enrich.rs"));
    assert_eq!(
        production_source(&rss).matches("Document::parse(").count(),
        1
    );
    assert!(rss.contains("contracts::RSS_DECODER"));
}

/// Situational: ADR 0075 packet 039 retains feed-check and update evidence.
#[test]
fn adr_0075_feed_observation_roots_and_consumers_are_guarded() {
    const FIX: &str = "ADR 0075 packet 039: the four feed roots own one recorder, drain it once, \
and the Library callbacks retain evidence before any result reduction. Packet 018 Part A adds \
that this recorder-bearing client reaches the network only through the shared request owner, \
under an explicit refresh intent.";
    let mut violations = Vec::new();
    let service = read_source(&manifest_path("src/feed_service.rs"));
    let commands = read_source(&manifest_path("src/application/commands/feed.rs"));
    let errors = read_source(&manifest_path("src/application/errors/command.rs"));
    let app = read_source(&manifest_path("src/library/app_impl.rs"));
    let vm = read_source(&manifest_path("src/view_models/library.rs"));

    // The provider client reaches the recorder only through the feed service,
    // and reaches the network only through the shared request owner, under
    // an explicit refresh intent (ADR 0075 packet 018 Part A).
    let staleness = source_between(
        &service,
        "pub fn check_feed_staleness(",
        "pub fn apply_feed_updates(",
    );
    for required in [
        "with_observation_recorder(Some(Arc::clone(recorder)))",
        "client.fetch_feed(&stored.feed_guid, None)",
        ".fetch_feed_with_receipts(key, RefreshIntent::Explicit, || {",
    ] {
        if !staleness.contains(required) {
            violations.push(format!(
                "src/feed_service.rs: check_feed_staleness must keep its observed, owner-routed, explicit request; missing `{required}`. {FIX}"
            ));
        }
    }
    let update = source_between(
        &service,
        "pub fn apply_feed_updates(",
        "pub fn track_row_to_track_context(",
    );
    for required in [
        "with_observation_recorder(Some(Arc::clone(recorder)))",
        "client.fetch_feed_with_profile(&stale.feed_guid, &LIBRARY_FEED_UPDATE_FEED)",
        ".fetch_feed_with_receipts(key, RefreshIntent::Explicit, || {",
        "propagate_storage_failure(fetch_library_track_detail_with_recorder(",
        "RefreshIntent::Explicit,",
        "merge_track_context_with_recorder(",
        "set_feed_musicindex_updated_at(&db, stale.feed_id, stale.new_updated_at)?",
    ] {
        if !update.contains(required) {
            violations.push(format!(
                "src/feed_service.rs: apply_feed_updates must keep observed, owner-routed, explicit requests before legacy writes; missing `{required}`. {FIX}"
            ));
        }
    }
    for (earlier, later) in [
        (
            "propagate_storage_failure(fetch_library_track_detail_with_recorder(",
            "merge_track_context_with_recorder(",
        ),
        (
            "merge_track_context_with_recorder(",
            "persist_musicindex_track(",
        ),
        // ADR 0076 Decision 8: the update writes no tag. The guard
        // `adr_0076_tag_update_checks_and_scans_write_no_tag` rejects a write.
        (
            "persist_musicindex_track(",
            "set_feed_musicindex_updated_at(",
        ),
    ] {
        match (update.find(earlier), update.find(later)) {
            (Some(first), Some(second)) if first < second => {}
            _ => violations.push(format!(
                "src/feed_service.rs: apply_feed_updates must keep `{earlier}` before `{later}`. {FIX}"
            )),
        }
    }

    // Every converted root owns one recorder and one drain.
    let production = production_source(&commands);
    if production.contains("with_observation_recorder(") {
        violations.push(format!(
            "src/application/commands/feed.rs: the provider client belongs to src/feed_service.rs. {FIX}"
        ));
    }
    if production.contains("CheckSubscribedFeeds") {
        violations.push(format!(
            "src/application/commands/feed.rs: the dead subscribed-feed command stays deleted. {FIX}"
        ));
    }
    assert_eq!(
        production.matches("take_receipts()").count(),
        1,
        "ADR 0075 packet 039: feed commands drain the recorder in one place. {FIX}"
    );
    assert_eq!(
        production
            .matches("ProviderObservationRecorder::new(Arc::clone(&self.conn))")
            .count(),
        3,
        "ADR 0075 packet 039: each converted root owns one recorder. {FIX}"
    );
    for (start, end, required) in [
        (
            "impl ApplicationCommand for CheckFeedStaleness",
            "/// Command result for applying remote feed updates",
            "assemble_observed_feed_command(&recorder, result",
        ),
        (
            "impl ApplicationCommand for ApplyFeedUpdates",
            "/// ADR 0075 drains the recorder once",
            "assemble_observed_feed_command(&recorder, result",
        ),
        (
            "impl ApplicationCommand for CheckFeedsAndRepairRoutes",
            "/// Command result for subscribing/downloading a feed.",
            "assemble_observed_feed_command(&recorder, result",
        ),
    ] {
        if !source_between(&commands, start, end).contains(required) {
            violations.push(format!(
                "src/application/commands/feed.rs: `{start}` must assemble receipts; missing `{required}`. {FIX}"
            ));
        }
    }
    for required in [
        "CommandError::ObservationWriteFailure(Arc::new(failure))",
        "observation_storage_failure(&error)",
    ] {
        if !source_between(
            &commands,
            "fn check_feed_batch_for_updates(",
            "fn apply_stale_feed_updates(",
        )
        .contains(required)
        {
            violations.push(format!(
                "src/application/commands/feed.rs: batch checks must classify storage failures; missing `{required}`. {FIX}"
            ));
        }
    }

    // The ordinary wrapper keeps the original cause and hides its contents.
    for required in [
        "pub struct ObservedCommandFailure",
        "cause: Arc<CommandError>",
        "Self::ObservedCommandFailure(failure) => failure.cause().fmt(f)",
        "f.debug_struct(\"ObservedCommandFailure\")",
        "finish_non_exhaustive()",
        "pub(crate) fn attach_observation_receipts(",
    ] {
        if !errors.contains(required) {
            violations.push(format!(
                "src/application/errors/command.rs: the ordinary observation wrapper is incomplete; missing `{required}`. {FIX}"
            ));
        }
    }
    let attach = source_between(
        &errors,
        "pub(crate) fn attach_observation_receipts(",
        "fn merged_receipts(",
    );
    for required in [
        "if receipts.is_empty()",
        "CommandError::ObservationWriteFailure(failure)",
        "CommandError::ObservedQueryFailure(failure)",
        "CommandError::ObservedCommandFailure(failure)",
    ] {
        if !attach.contains(required) {
            violations.push(format!(
                "src/application/errors/command.rs: receipt attachment must preserve each error family; missing `{required}`. {FIX}"
            ));
        }
    }

    // The three live Library callbacks retain evidence before any reduction.
    for (start, end, retention, reduction) in [
        (
            "fn apply_feed_view_check_result(",
            "/// ADR 0075 retains combined-check evidence",
            "retain_observation_receipts",
            "finish_feed_view_check(",
        ),
        (
            "fn retain_feed_check_evidence(",
            "/// ADR 0075 retains update evidence",
            "retain_observation_receipts",
            "feed_check_route_repair_outcome(",
        ),
        (
            "fn apply_feed_updates_result(",
            "fn feed_check_route_repair_outcome(",
            "retain_observation_receipts",
            "finish_apply_feed_updates(",
        ),
    ] {
        let callback = source_between(&app, start, end);
        match (callback.find(retention), callback.find(reduction)) {
            (Some(first), Some(second)) if first < second => {}
            _ => violations.push(format!(
                "src/library/app_impl.rs: `{start}` must retain evidence before `{reduction}`. {FIX}"
            )),
        }
    }
    for (start, end, failure) in [
        (
            "fn check_feed_on_view(",
            "fn check_all_feeds(",
            "finish_feed_view_check_error(feed_id, error)",
        ),
        (
            "fn check_all_feeds(",
            "fn apply_all_feed_updates(",
            "set_feed_check_error(error)",
        ),
        (
            "fn apply_all_feed_updates(",
            "fn repair_broadcast_routes_for_track(",
            "finish_apply_feed_updates_error(error)",
        ),
    ] {
        let callback = source_between(&app, start, end);
        match (
            callback.find("retain_library_query_failure(&mut this.vm, &error)"),
            callback.find(failure),
        ) {
            (Some(first), Some(second)) if first < second => {}
            _ => violations.push(format!(
                "src/library/app_impl.rs: `{start}` must retain evidence before `{failure}`. {FIX}"
            )),
        }
    }
    if !source_between(
        &app,
        "fn command_error_detail(",
        "/// ADR 0075 consumes query evidence",
    )
    .contains("CommandError::ObservedCommandFailure(failure) => {")
    {
        violations.push(format!(
            "src/library/app_impl.rs: command_error_detail must delegate to the original cause. {FIX}"
        ));
    }
    let retention = source_between(
        &vm,
        "pub(crate) fn retain_query_evidence(",
        "pub(crate) fn is_resizing(",
    );
    for required in [
        "CommandError::ObservedCommandFailure(failure)",
        "self.retain_observation_receipts(failure.receipts())",
    ] {
        if !retention.contains(required) {
            violations.push(format!(
                "src/view_models/library.rs: retain_query_evidence must accept the ordinary wrapper; missing `{required}`. {FIX}"
            ));
        }
    }
    if retention.contains("set_error_status(") {
        violations.push(format!(
            "src/view_models/library.rs: evidence retention must not use the broad error status helper. {FIX}"
        ));
    }

    assert!(
        violations.is_empty(),
        "ADR 0075 packet 039 feed observation violations:\n{}",
        violations.join("\n")
    );
}

/// Situational — ADR 0075 section 6, packet 018 Part A (R18A-12): a
/// converted route asks the shared request owner for a MusicIndex
/// resource. It does not send a metadata request through `api::Client`
/// on its own.
#[test]
fn adr_0075_request_reuse_converted_routes_ask_the_owner() {
    const FIX: &str = "ADR 0075 section 6: a caller asks the shared request owner \
(src/application/request_reuse.rs) for a MusicIndex resource, instead of sending the request \
through api::Client on its own. That way, a concurrent duplicate request for the same \
endpoint, scoped identity and profile joins one HTTP request, and packet 018 Part B can \
retain and reuse the response. Route the call through `owner.fetch_feed_with_receipts` or \
`owner.fetch_track_with_receipts`.";
    let mut violations = Vec::new();

    let service = read_source(&manifest_path("src/feed_service.rs"));
    for (start, end, minimum_owner_calls) in [
        (
            "fn fetch_library_track_detail_with_recorder(",
            "fn merge_track_context_with_recorder(",
            3,
        ),
        (
            "pub fn check_feed_staleness(",
            "pub fn apply_feed_updates(",
            1,
        ),
        (
            "pub fn apply_feed_updates(",
            "pub fn track_row_to_track_context(",
            1,
        ),
    ] {
        let body = source_between(&service, start, end);
        let owner_calls = [
            ".fetch_feed_with_receipts(key,",
            ".fetch_track_with_receipts(key,",
        ]
        .iter()
        .map(|pattern| body.matches(pattern).count())
        .sum::<usize>();
        if owner_calls < minimum_owner_calls {
            violations.push(format!(
                "src/feed_service.rs: `{start}` must ask the shared request owner at least \
{minimum_owner_calls} time(s); found {owner_calls}. {FIX}"
            ));
        }
        if !body.contains("request_reuse::") {
            violations.push(format!(
                "src/feed_service.rs: `{start}` must reach the shared request owner module. {FIX}"
            ));
        }
    }

    let library_query = read_source(&manifest_path("src/application/queries/library.rs"));
    let hydration = source_between(
        &library_query,
        "fn hydrate_album_identity_facts(",
        "fn compare_library_track(",
    );
    if hydration.matches(".fetch_feed_with_receipts(key,").count() != 1 {
        violations.push(format!(
            "src/application/queries/library.rs: hydrate_album_identity_facts must ask the \
shared request owner exactly once. {FIX}"
        ));
    }
    if !hydration.contains("request_reuse::") {
        violations.push(format!(
            "src/application/queries/library.rs: hydrate_album_identity_facts must reach the \
shared request owner module. {FIX}"
        ));
    }
    let comparison = source_between(
        &library_query,
        "fn compare_library_track(",
        "fn assemble_observed_query<",
    );
    if !comparison.contains("RefreshIntent::Explicit") {
        violations.push(format!(
            "src/application/queries/library.rs: compare_library_track must carry an explicit \
refresh intent, so it never joins an active passive read. {FIX}"
        ));
    }

    assert!(
        violations.is_empty(),
        "ADR 0075 packet 018 Part A request-owner violations:\n{}",
        violations.join("\n")
    );
}

/// Situational — ADR 0075 packet 018 Part B, R18B-12: the Index route asks
/// the shared request owner too, not only the Library route. A feed
/// identity shares P18-2's window (`owner.fetch_feed_with_receipts`); a
/// track identity carries no accepted window of its own, so it shares
/// only an active request (`owner.fetch_track_shared`), never a retained
/// one. Packet 047 moved each call from the search loop to a
/// detail-on-open command, and this guard also checks that the search
/// loop itself sends no per-hit detail request (R47-02).
#[test]
fn adr_0075_request_reuse_index_routes_ask_the_owner() {
    const FIX: &str = "ADR 0075 section 6, packet 018 R18B-12: an Index or Inspector feed or \
track detail call must ask the shared request owner (src/application/request_reuse.rs), \
instead of sending the request through api::Client on its own. Route a feed identity through \
`owner.fetch_feed_with_receipts` (P18-2's window applies, exactly as for any other feed \
identity) and a track identity through `owner.fetch_track_shared` (active-request sharing \
only: P18-1 names a Library track detail response, not an Index or Inspector one).";
    const DIRECT_CLIENT_CALLS: [&str; 6] = [
        "client.fetch_feed(",
        "client.fetch_feed_with_profile(",
        "client.fetch_track(",
        "client.fetch_track_with_profile(",
        "client.fetch_feed_track(",
        "client.fetch_feed_track_with_profile(",
    ];
    let mut violations = Vec::new();

    let search = read_source(&manifest_path("src/application/queries/search.rs"));
    let search_production = production_source(&search);
    let search_owner_feed = source_between(
        search_production,
        "fn owner_fetch_feed(",
        "fn fetch_index_feed_result_rows(",
    );
    if search_owner_feed
        .matches(".fetch_feed_with_receipts(key,")
        .count()
        < 1
    {
        violations.push(format!(
            "src/application/queries/search.rs: `owner_fetch_feed` must ask the shared \
request owner. {FIX}"
        ));
    }
    let search_owner_track = source_between(
        search_production,
        "fn fetch_index_track_detail(",
        "pub(super) fn index_feed_display(",
    );
    if search_owner_track
        .matches(".fetch_track_shared(key,")
        .count()
        < 2
    {
        violations.push(format!(
            "src/application/queries/search.rs: `fetch_index_track_detail` must ask the \
shared request owner for both its scoped and its unscoped identity. {FIX}"
        ));
    }
    // ADR 0075 packet 047 moved these calls out of the search loops and
    // into a detail-on-open command, sent only when the operator opens the
    // row. Each command still asks the shared request owner.
    for (start, end, calls_helper) in [
        (
            "struct FetchIndexFeedDetail {",
            "fn fetch_index_feed_result_rows(",
            "owner_fetch_feed(",
        ),
        (
            "struct FetchIndexTrackDetail {",
            "pub(super) fn index_feed_display(",
            "fetch_index_track_detail(",
        ),
    ] {
        let body = source_between(search_production, start, end);
        if !body.contains(calls_helper) {
            violations.push(format!(
                "src/application/queries/search.rs: `{start}` must call `{calls_helper}`. {FIX}"
            ));
        }
        for forbidden in DIRECT_CLIENT_CALLS {
            if body.contains(forbidden) {
                violations.push(format!(
                    "src/application/queries/search.rs: `{start}` must not call `{forbidden}` \
directly. {FIX}"
                ));
            }
        }
    }

    // Situational — ADR 0075 packet 047, Required Change 2: an Index
    // search draws each row from the search response's own summary
    // fields, so it must send no per-hit detail request of its own.
    const NO_DETAIL_FIX: &str = "ADR 0075 packet 047, Required Change 2: an Index search draws \
each row from the search response's own summary fields and sends no per-hit detail request. \
Move a needed detail fetch into FetchIndexFeedDetail or FetchIndexTrackDetail, sent only when \
the operator opens the row.";
    for (start, end, forbidden_helper) in [
        (
            "fn fetch_index_feed_result_rows(",
            "fn fetch_index_track_result_rows(",
            "owner_fetch_feed(",
        ),
        (
            "fn fetch_index_track_result_rows(",
            "fn index_artist_candidate_from_feed(",
            "fetch_index_track_detail(",
        ),
    ] {
        let body = source_between(search_production, start, end);
        if body.contains(forbidden_helper) {
            violations.push(format!(
                "src/application/queries/search.rs: `{start}` must send no per-hit detail \
request; found `{forbidden_helper}`. {NO_DETAIL_FIX}"
            ));
        }
    }

    let feed = read_source(&manifest_path("src/application/queries/feed.rs"));
    let feed_production = production_source(&feed);
    let feed_owner_feed = source_between(
        feed_production,
        "fn owner_fetch_feed(",
        "pub(crate) fn fetch_index_publisher_page_albums(",
    );
    if feed_owner_feed
        .matches(".fetch_feed_with_receipts(key,")
        .count()
        < 1
    {
        violations.push(format!(
            "src/application/queries/feed.rs: `owner_fetch_feed` must ask the shared request \
owner. {FIX}"
        ));
    }

    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Situational — ADR 0075 packet 018 Part B: a retained response is a pure
/// fetch-and-cache mechanism. R18B-08 requires that it never creates or
/// resolves a discrepancy, so this guard checks structurally that the
/// retention code never reaches comparison code at all.
#[test]
fn adr_0075_request_reuse_retention_never_reaches_comparison_code() {
    const FIX: &str = "ADR 0075 section 4, packet 018 R18B-08: a reused response must not \
create or resolve a discrepancy. src/application/request_reuse.rs and the retention code in \
src/rss/enrich.rs must not import or call track_compare or a comparison entry point such as \
compare_downloaded_track_path. Keep retention a pure fetch-and-cache mechanism; comparison \
stays a separate concern owned elsewhere.";
    let mut violations = Vec::new();

    let owner = read_source(&manifest_path("src/application/request_reuse.rs"));
    let enrich = read_source(&manifest_path("src/rss/enrich.rs"));

    for (path, source) in [
        ("src/application/request_reuse.rs", &owner),
        ("src/rss/enrich.rs", &enrich),
    ] {
        for forbidden in [
            "track_compare",
            "compare_downloaded_track_path",
            "TagCompareResult",
        ] {
            if source.contains(forbidden) {
                violations.push(format!(
                    "{path}: retention code must not reference `{forbidden}`. {FIX}"
                ));
            }
        }
    }

    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Situational — ADR 0075 packet 017: request profiles own the include
/// literals and carry no provider ownership field.
#[test]
fn adr_0075_request_profile_registry_owns_literals_and_has_no_provider_field() {
    const FIX: &str = "ADR 0075 Decision I: a profile names what the app asks a cache for. \
It must not model provider ownership, because a provider is transport evidence, not a \
source. Route an ADR 0075 include list through \
`src/application/request_profiles.rs`, not as an inline string. \
`src/subscribe_service.rs`, `src/application/commands/payment_routes.rs`, and \
`src/application/commands/feed.rs` stay outside packet 017 and may keep their own \
literals until a later decision changes them.";

    let mut violations = Vec::new();

    let registry = read_source(&manifest_path("src/application/request_profiles.rs"));
    let registry_production = production_source(&registry);
    let struct_body = source_between(
        registry_production,
        "pub(crate) struct RequestProfile {",
        "}",
    );
    for forbidden in ["provider", "owner"] {
        if struct_body.to_lowercase().contains(forbidden) {
            violations.push(format!(
                "src/application/request_profiles.rs: RequestProfile must not carry a `{forbidden}` field. {FIX}"
            ));
        }
    }

    let recorded_literals = [
        "source_links,source_ids,source_release_claims,source_contributors,payment_routes",
        "tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes",
        "source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes",
        "tracks,source_enclosures,source_links,source_ids,source_release_claims,payment_routes",
        "source_links,source_ids,source_release_claims,source_contributors",
    ];
    for relative in [
        "src/feed_service.rs",
        "src/application/queries/search.rs",
        "src/application/queries/feed.rs",
        "src/application/queries/library.rs",
        "src/api.rs",
    ] {
        let source = read_source(&manifest_path(relative));
        let production = production_source(&source);
        for literal in recorded_literals {
            if production.contains(literal) {
                violations.push(format!(
                    "{relative}: an ADR 0075 include literal must live in \
src/application/request_profiles.rs, not as an inline string. {FIX}"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0075 packet 017 request profile violations:\n{}",
        violations.join("\n")
    );
}

/// Situational — ADR 0077 Decision 2, packet 002 (R2-12): the publisher
/// binding stays on the feed. No track table stores a publisher value, and
/// the feed publisher relationship table has no track column.
#[test]
fn adr_0077_publisher_relationship_no_track_table_stores_a_publisher() {
    const FIX: &str = "ADR 0077 Decision 2: an album binds to the publisher that it names. \
The app stores no publisher value on a track, and a feed value never becomes a track value. \
Store the relationship in `feed_publisher_relationships`, keyed by `feed_id`. A track reaches \
its publisher through its album feed.";

    fn table_columns(body: &str) -> Vec<String> {
        let mut columns = Vec::new();
        let mut depth = 0usize;
        let mut current = String::new();
        for character in body.chars() {
            match character {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    columns.push(std::mem::take(&mut current));
                    continue;
                }
                _ => {}
            }
            current.push(character);
        }
        columns.push(current);
        columns
            .iter()
            .filter_map(|definition| definition.split_whitespace().next())
            .map(|name| name.trim_matches(|c| c == '"' || c == '`').to_string())
            .filter(|name| {
                !matches!(
                    name.as_str(),
                    "primary" | "unique" | "check" | "foreign" | "constraint"
                )
            })
            .collect()
    }

    fn created_tables(source: &str) -> Vec<(String, Vec<String>)> {
        let mut tables = Vec::new();
        let mut rest = source;
        while let Some(index) = rest.find("create table") {
            rest = &rest[index + "create table".len()..];
            let mut header = rest.trim_start();
            if let Some(stripped) = header.strip_prefix("if not exists") {
                header = stripped.trim_start();
            }
            let name: String = header
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
                .collect();
            let Some(open) = header.find('(') else {
                continue;
            };
            let mut depth = 0usize;
            let mut end = None;
            for (offset, character) in header[open..].char_indices() {
                match character {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(open + offset);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else {
                continue;
            };
            tables.push((name, table_columns(&header[open + 1..end])));
        }
        tables
    }

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let relative = path
            .strip_prefix(manifest_path(""))
            .unwrap_or(&path)
            .display()
            .to_string();
        let source = read_source(&path);
        let production = code_only(production_source(&source)).to_lowercase();

        for (table, columns) in created_tables(&production) {
            let table = table.rsplit('.').next().unwrap_or(&table).to_string();
            if table.contains("track") {
                for column in columns.iter().filter(|column| column.contains("publisher")) {
                    violations.push(format!(
                        "{relative}: track table `{table}` has the publisher column `{column}`. {FIX}"
                    ));
                }
            }
            if table == "feed_publisher_relationships" {
                for column in columns.iter().filter(|column| column.contains("track")) {
                    violations.push(format!(
                        "{relative}: `feed_publisher_relationships` has the track column `{column}`. {FIX}"
                    ));
                }
            }
        }

        let compact = compact_source(&production);
        let mut rest = compact.as_str();
        while let Some(index) = rest.find("add_column_if_missing(") {
            rest = &rest[index + "add_column_if_missing(".len()..];
            let arguments = rest
                .split(')')
                .next()
                .unwrap_or_default()
                .split(',')
                .map(|argument| argument.trim_matches('"').to_string())
                .collect::<Vec<_>>();
            if let [_, table, column, ..] = arguments.as_slice() {
                if table.contains("track") && column.contains("publisher") {
                    violations.push(format!(
                        "{relative}: track table `{table}` gains the publisher column `{column}`. {FIX}"
                    ));
                }
            }
        }
        let mut rest = compact.as_str();
        while let Some(index) = rest.find("altertable") {
            rest = &rest[index + "altertable".len()..];
            let statement = rest.split(';').next().unwrap_or_default();
            if let Some((table, column)) = statement.split_once("addcolumn") {
                if table.contains("track") && column.contains("publisher") {
                    violations.push(format!(
                        "{relative}: an ALTER TABLE statement adds a publisher column to a track table. {FIX}"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0077 Decision 2 publisher binding violations:\n{}",
        violations.join("\n")
    );
}

/// Situational — ADR 0077 Decision 6, packet 005 (R5-02): `publisher_text`
/// is feed owner text. No entry point opens the removed
/// `/v1/publishers/{publisher_text}` inspector. Delete this guard when ADR
/// 0077 Decision 6 is superseded.
#[test]
fn adr_0077_feed_owner_publisher_text_inspector_stays_removed() {
    const FIX: &str = "ADR 0077 Decision 6: `publisher_text` is the feed owner. No artist page \
and no label page opens from it or from `/v1/publishers`. Show `publisher_text` as inert feed \
owner text instead of reintroducing the publisher inspector.";
    const FORBIDDEN: &[&str] = &[
        "fn fetch_publisher(",
        "EntityDetail::Publisher",
        "InspectorDetailData::Publisher",
        "InspectorDetail::Publisher",
        "PublisherInspectorVm",
        "render_publisher_inspector",
        "struct Publisher {",
        "PublisherSearchResponse",
        "fn search_publishers(",
        "fn fetch_wrapped(",
    ];

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            for pattern in FORBIDDEN {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: names `{pattern}`: `{line}`\n  {FIX}"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0077 Decision 6 feed owner inspector violations:\n{}",
        violations.join("\n")
    );
}

/// Situational — ADR 0077 Decision 1, Task 003 (R3-12): an artist identity
/// comes only from a publisher feed GUID. No code builds
/// `ArtistRef::PublisherFeed` from name text or from `publisher_text`.
/// Delete this guard when ADR 0077 Decision 1 is superseded.
#[test]
fn adr_0077_publisher_page_artist_ref_publisher_feed_built_only_from_guid() {
    const FIX: &str =
        "ADR 0077 Decision 1: an artist identity comes only from a publisher feed GUID. \
Build `ArtistRef::PublisherFeed` from `publisher_feed_guid` or a stored feed GUID, never from name \
text or `publisher_text`.";
    const FORBIDDEN: &[&str] = &["publisher_text", "name", "title"];
    const CONSTRUCTOR: &str = "ArtistRef::PublisherFeed(";

    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(production_source(&source)) {
            let Some(index) = line.find(CONSTRUCTOR) else {
                continue;
            };
            let rest = &line[index + CONSTRUCTOR.len()..];
            let argument = rest.split(')').next().unwrap_or_default().to_lowercase();
            for pattern in FORBIDDEN {
                if argument.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: `ArtistRef::PublisherFeed` built from `{pattern}`: `{line}`\n  {FIX}"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0077 Decision 1 publisher feed identity violations:\n{}",
        violations.join("\n")
    );
}

/// Situational — ADR 0077 packet 004 (R4-06): the durable renderer
/// portability rule in AGENTS.md. The publisher page screen reads the page
/// type, the role labels, the group labels and the "not listed"/"in
/// library" marks from `PublisherPageVm`/`PublisherPageContext` only. It
/// never hardcodes the words those types compute, and it composes no
/// display string, such as a count-and-label pair or a glyph, of its own
/// (orchestrator review, packet 004 fix 2). Delete this guard only when
/// AGENTS.md drops the renderer portability rule.
#[test]
fn adr_0077_publisher_navigation_screen_reads_labels_from_view_model_only() {
    const FIX: &str = "AGENTS.md durable renderer portability rule: a screen decides no page type, \
no role label, no group, no composed display string and no action availability. Read \
`PublisherPageType::label()`, `AlbumRoleDisplay::text()`, `PublisherPageContext::group_labels()`, \
`PublisherPageAlbumVm::NOT_LISTED_LABEL`, `PublisherPageAlbumVm::IN_LIBRARY_LABEL`, \
`AlbumArtistDisplay::display_text()`, `DerivedArtistCount::display_text()` and \
`PublisherPageVm::header_facts()`/`title_text()` from `src/view_models/publisher_page.rs` instead of \
writing the word, the composed string or the glyph directly in the screen.";
    const FORBIDDEN: &[&str] = &[
        "\"Artist\"",
        "\"Label\"",
        "\"Not listed",
        "\"In Library\"",
        "\"Library Albums\"",
        "\"Other Albums\"",
        "\"Owned Albums\"",
        "\"Listed By\"",
    ];
    /// A `\u{...}` escape is a glyph, such as an em dash, composed straight
    /// into the screen. AGENTS.md token discipline: "No raw literal and no
    /// glyph string in a renderer."
    const GLYPH_ESCAPE: &str = "\\u{";
    /// A string literal passed straight to `Label::new` is a display label
    /// the screen invented, not one the view model computed.
    const LITERAL_LABEL: &str = "Label::new(\"";

    let path = manifest_path("src/ui/shells/publisher.rs");
    let source = read_source(&path);
    let mut violations = Vec::new();
    for (line_number, line) in code_lines(&source) {
        for forbidden in FORBIDDEN {
            if line.contains(forbidden) {
                violations.push(format!(
                    "{}:{line_number}: publisher page screen hardcodes {forbidden}: `{line}`\n  {FIX}",
                    rel_path(&path)
                ));
            }
        }
        if line.contains(GLYPH_ESCAPE) {
            violations.push(format!(
                "{}:{line_number}: publisher page screen embeds a glyph escape: `{line}`\n  {FIX}",
                rel_path(&path)
            ));
        }
        if line.contains(LITERAL_LABEL) {
            violations.push(format!(
                "{}:{line_number}: publisher page screen passes a quoted display label to \
`Label::new`: `{line}`\n  {FIX}",
                rel_path(&path)
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0077 packet 004 publisher page screen renderer portability violations:\n{}",
        violations.join("\n")
    );
}

/// Situational: ADR 0076 Decision 5 (packet 002). Each `MusicIndex` writer of
/// a compared slot calls the one hold gate before it writes the slot. Delete
/// this guard when ADR 0076 is superseded.
#[test]
fn adr_0076_rss_comparison_musicindex_writers_call_the_hold_gate() {
    const FIX: &str = "ADR 0076 Decision 5: a MusicIndex write of a compared slot must first call db::rss_field_holds::musicindex_gate and write the slot only when the gate returns Write. Add the gate call before the write, and list the site in this guard and in the packet 002 result. The subscribe command is not a MusicIndex writer (ADR 0075 packet 020): it must write no MusicIndex value into a compared slot.";
    const GATE: &str = "musicindex_gate(";
    // (file, section start, section end, slot write)
    // ADR 0075 packet 020: `rss::subscribe::subscribe_feed` is not a site. A
    // subscribe reads RSS, writes the RSS value and hold of each compared
    // slot, and writes no MusicIndex value into a compared slot.
    let sites = [
        (
            "src/feed_service.rs",
            "pub fn apply_feed_updates(",
            "pub fn track_row_to_track_context(",
            "db::set_feed_description(",
        ),
        (
            "src/application/queries/library.rs",
            "fn hydrate_album_identity_facts(",
            "fn compare_library_track(",
            "db::set_feed_description(",
        ),
        (
            "src/application/queries/library.rs",
            "fn hydrate_album_identity_facts(",
            "fn compare_library_track(",
            "upsert_feed_publisher_relationships(",
        ),
        (
            "src/identity_ingest.rs",
            "fn persist_source_ids(",
            "fn is_nostr_scheme(",
            "db::replace_local_identity_ids(",
        ),
        // ADR 0076 packet 006: the `MusicIndex` credit list.
        (
            "src/identity_ingest.rs",
            "fn persist_contributors(",
            "fn persist_feed_metadata_facts(",
            "db::replace_local_contributors(",
        ),
        (
            "src/identity_ingest.rs",
            "fn persist_feed_metadata_facts(",
            "fn persist_track_metadata_facts(",
            "db::replace_local_metadata_facts(",
        ),
        (
            "src/identity_ingest.rs",
            "fn persist_track_metadata_facts(",
            "fn merge_existing_metadata_facts_for_partial_source(",
            "db::replace_local_metadata_facts(",
        ),
    ];
    let mut violations = Vec::new();
    for (file, start, end, write) in sites {
        let source = read_source(&manifest_path(file));
        let section = code_only(source_between(production_source(&source), start, end));
        match (section.find(GATE), section.find(write)) {
            (Some(gate), Some(slot)) if gate < slot => {}
            (_, None) => violations.push(format!(
                "{file}: the listed site `{start}` no longer writes `{write}`. Update the site list.\n  {FIX}"
            )),
            _ => violations.push(format!(
                "{file}: `{start}` writes `{write}` without a gate call before it.\n  {FIX}"
            )),
        }
    }
    // Each production writer of a gated column slot is a listed site.
    for (write, allowed) in [
        (
            "set_feed_description(",
            &[
                "src/db.rs",
                "src/feed_service.rs",
                "src/application/queries/library.rs",
            ][..],
        ),
        (
            "upsert_feed_publisher_relationships(",
            &[
                "src/db/publisher_relationships.rs",
                "src/application/queries/library.rs",
            ][..],
        ),
        // ADR 0076 packet 006: the gated `MusicIndex` credit writer, and the
        // RSS credit writers of the subscribe and the check.
        (
            "replace_local_contributors(",
            &[
                "src/db.rs",
                "src/identity_ingest.rs",
                "src/rss/subscribe.rs",
            ][..],
        ),
    ] {
        for path in rust_files_under("src") {
            let file = rel_path(&path);
            let source = read_source(&path);
            let production = without_unit_test_module(&path, &source);
            for (line_number, line) in code_lines(&production) {
                if line.contains(write) && !allowed.contains(&file.as_str()) {
                    violations.push(format!(
                        "{file}:{line_number}: an unlisted MusicIndex write site calls `{write}`: `{line}`\n  {FIX}"
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "ADR 0076 Decision 5 hold gate violations:\n{}",
        violations.join("\n")
    );
}

/// Situational: ADR 0076 packet 006. A credit list on a screen or in a tag
/// frame comes from the stored value projection, which shows one list for
/// each owner. Before this packet, a track page listed the `rss` credits and
/// the `musicindex` credits together (packet 020 finding 6). Delete this
/// guard when ADR 0076 is superseded.
#[test]
fn adr_0076_credit_list_readers_use_the_projection() {
    const FIX: &str = "ADR 0076 packet 006: db::local_contributors returns the credit rows of all sources. Read the credit list through application::queries::stored_values (feed_values, track_values or feed_credits). Only the projection and the RSS comparison of rss::check_apply read the stored rows.";
    const READ: &str = "::local_contributors(";
    const ALLOWED: [&str; 2] = [
        "src/application/queries/stored_values.rs",
        "src/rss/check_apply.rs",
    ];
    let mut violations = Vec::new();
    for path in rust_files_under("src") {
        let file = rel_path(&path);
        if ALLOWED.contains(&file.as_str()) {
            continue;
        }
        let source = read_source(&path);
        let production = without_unit_test_module(&path, &source);
        for (line_number, line) in code_lines(&production) {
            if line.contains(READ) {
                violations.push(format!(
                    "{file}:{line_number}: a reader of all credit sources: `{line}`\n  {FIX}"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "ADR 0076 packet 006 credit list violations:\n{}",
        violations.join("\n")
    );
}

/// Situational: ADR 0076 Decision 8 (packet 002). The playlist RSS check
/// changes the database only. Delete this guard when ADR 0076 is superseded.
#[test]
fn adr_0076_rss_comparison_check_writes_no_audio_tag() {
    const FIX: &str = "ADR 0076 Decision 8: the playlist RSS check writes no audio tag. Only the operator-confirmed \"Update n file(s)\" action of packet 004 writes tags. Remove the tag write from the check.";
    let mut violations = Vec::new();
    for file in [
        "src/rss/check_apply.rs",
        "src/rss/compare.rs",
        "src/db/rss_field_holds.rs",
        "src/runtime/playlist_rss_check.rs",
    ] {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for forbidden in ["write_id3v24_edits", "audio_tags::"] {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{file}:{line_number}: the check calls `{forbidden}`: `{line}`\n  {FIX}"
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "ADR 0076 Decision 8 audio tag violations:\n{}",
        violations.join("\n")
    );
}

/// Situational: ADR 0076 Decision 1 (ADR 0075 packet 020, R20-07). The app
/// shows the stored value of each field. The stored value projection in
/// `src/application/queries/stored_values.rs` is the one owner of the order
/// of hold, `MusicIndex` fact and column. Delete this guard when ADR 0076 is
/// superseded.
#[test]
fn adr_0075_projection_views_combine_no_fact_and_column() {
    const FIX: &str = "ADR 0076 Decision 1: the app shows the stored value and selects no source at display time. Read the value from `stored_values::feed_values` or `stored_values::track_values` in src/application/queries/stored_values.rs, and pass `FeedStoredValues` or `TrackStoredValues` to the view. Do not read a metadata fact field in a view or a view model.";
    const FACT_FIELDS: [&str; 7] = [
        "description",
        "language",
        "explicit",
        "pub_date",
        "publisher_text",
        "release_date",
        "release_kind",
    ];
    let mut files = vec!["src/views.rs".to_owned()];
    files.extend(
        rust_files_under("src/view_models")
            .into_iter()
            .map(|path| rel_path(&path)),
    );
    let mut violations = Vec::new();
    for file in &files {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(production_source(&source)) {
            for field in FACT_FIELDS {
                let pattern = format!("facts.{field}");
                if line.contains(&pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: reads the fact field `{pattern}` and can combine it with a column: `{line}`\n  {FIX}"
                    ));
                }
            }
        }
    }

    let views = production_source(&read_source(&manifest_path("src/views.rs"))).to_owned();
    for required in ["values: FeedStoredValues", "values: TrackStoredValues"] {
        if !views.contains(required) {
            violations.push(format!(
                "src/views.rs: a local view constructor must take the projection, `{required}`.\n  {FIX}"
            ));
        }
    }

    let projection = read_source(&manifest_path("src/application/queries/stored_values.rs"));
    if production_source(&projection)
        .matches("pub(crate) fn select<T>(")
        .count()
        != 1
    {
        violations.push(format!(
            "src/application/queries/stored_values.rs: the order must have one owner, `pub(crate) fn select<T>(`.\n  {FIX}"
        ));
    }
    let check = read_source(&manifest_path("src/rss/check_apply.rs"));
    let check = production_source(&check);
    if !check.contains("stored_values::compared_slot(")
        || check.contains("db::local_metadata_fact(")
    {
        violations.push(format!(
            "src/rss/check_apply.rs: the playlist RSS check must read a fact-backed slot through `stored_values::compared_slot(`, not its own fact read.\n  {FIX}"
        ));
    }

    assert!(
        violations.is_empty(),
        "ADR 0076 Decision 1 stored value projection violations:\n{}",
        violations.join("\n")
    );
}

// ===== ADR 0075 Task 051: Contract Field Guard =====
//
// Situational — ADR 0075 section 6. `src/api.rs` must decode only fields
// that the deployed MusicIndex contract declares. The guard below reads the
// stored contract copy at `tests/fixtures/musicindex-openapi-0.2.0.json` and
// each mapped struct body in `src/api.rs`, and fails when a decoded field is
// not a declared property. Delete this guard when ADR 0075 is superseded.

/// The stored MusicIndex contract (ADR 0075 section 6). The file name holds
/// the contract version. A person or an agent replaces this file after each
/// Stophammer release, in the same change as the decode changes.
const MUSICINDEX_CONTRACT_FIXTURE: &str = "tests/fixtures/musicindex-openapi-0.2.0.json";

/// Where a decoded type's declared fields live in the stored contract: a
/// named schema under `components.schemas`, or the inline response schema
/// of one path and method. MusicIndex names its list and detail envelopes
/// inline instead of as a named schema.
enum ContractSchemaRef {
    Named(&'static str),
    Inline {
        path: &'static str,
        method: &'static str,
        status: &'static str,
    },
}

impl ContractSchemaRef {
    /// Returns the text that a failure message uses to name this schema.
    fn display_name(&self) -> String {
        match self {
            ContractSchemaRef::Named(name) => (*name).to_string(),
            ContractSchemaRef::Inline {
                path,
                method,
                status,
            } => format!(
                "the {} {path} {status} response schema",
                method.to_uppercase()
            ),
        }
    }
}

/// One decoded MusicIndex type and the schema, or schemas, that must
/// declare each of its fields. A field counts as declared when any listed
/// schema declares it: `RemoteItem` decodes two endpoint shapes that both
/// carry the fields this app reads.
struct ContractTypeMapping {
    rust_type: &'static str,
    schemas: &'static [ContractSchemaRef],
}

/// ADR 0075 section 6, Required Changes 2: each decoded MusicIndex type
/// with named fields, mapped to its contract schema. Built from the
/// deployed contract version `0.2.0` on 2026-10-01.
const CONTRACT_TYPE_MAP: &[ContractTypeMapping] = &[
    ContractTypeMapping {
        rust_type: "SearchResponse",
        schemas: &[ContractSchemaRef::Inline {
            path: "/v1/search",
            method: "get",
            status: "200",
        }],
    },
    ContractTypeMapping {
        rust_type: "SearchResult",
        schemas: &[ContractSchemaRef::Named("SearchResponseItem")],
    },
    ContractTypeMapping {
        rust_type: "TrackListResponse",
        schemas: &[ContractSchemaRef::Inline {
            path: "/v1/tracks",
            method: "get",
            status: "200",
        }],
    },
    ContractTypeMapping {
        rust_type: "RecentFeedsResponse",
        schemas: &[ContractSchemaRef::Inline {
            path: "/v1/feeds/recent",
            method: "get",
            status: "200",
        }],
    },
    ContractTypeMapping {
        rust_type: "Pagination",
        schemas: &[ContractSchemaRef::Named("Pagination")],
    },
    ContractTypeMapping {
        rust_type: "DetailResponse",
        // Every single-resource route shares this envelope shape. The feed
        // detail route stands for all of them.
        schemas: &[ContractSchemaRef::Inline {
            path: "/v1/feeds/{guid}",
            method: "get",
            status: "200",
        }],
    },
    ContractTypeMapping {
        rust_type: "Feed",
        schemas: &[ContractSchemaRef::Named("FeedResponse")],
    },
    ContractTypeMapping {
        rust_type: "Track",
        schemas: &[ContractSchemaRef::Named("TrackResponse")],
    },
    ContractTypeMapping {
        rust_type: "Contributor",
        schemas: &[ContractSchemaRef::Named("SourceContributorClaimResponse")],
    },
    ContractTypeMapping {
        rust_type: "PaymentRoute",
        schemas: &[ContractSchemaRef::Named("RouteResponse")],
    },
    ContractTypeMapping {
        rust_type: "SourceEntityLink",
        schemas: &[ContractSchemaRef::Named("SourceEntityLinkResponse")],
    },
    ContractTypeMapping {
        rust_type: "SourceEntityId",
        schemas: &[ContractSchemaRef::Named("SourceEntityIdResponse")],
    },
    ContractTypeMapping {
        rust_type: "SourceReleaseClaim",
        schemas: &[ContractSchemaRef::Named("SourceReleaseClaimResponse")],
    },
    ContractTypeMapping {
        rust_type: "SourceEnclosure",
        schemas: &[ContractSchemaRef::Named("SourceItemEnclosureResponse")],
    },
    ContractTypeMapping {
        rust_type: "SourceTranscript",
        schemas: &[ContractSchemaRef::Named("SourceItemTranscriptResponse")],
    },
    ContractTypeMapping {
        rust_type: "SourcePlatformClaim",
        schemas: &[ContractSchemaRef::Named("SourcePlatformClaimResponse")],
    },
    ContractTypeMapping {
        rust_type: "RemoteItem",
        schemas: &[
            ContractSchemaRef::Named("FeedRemoteItemResponse"),
            ContractSchemaRef::Named("TrackRemoteItemResponse"),
        ],
    },
    ContractTypeMapping {
        rust_type: "PublisherRelationship",
        schemas: &[ContractSchemaRef::Named("PublisherResponse")],
    },
    ContractTypeMapping {
        rust_type: "ValueTimeSplit",
        schemas: &[ContractSchemaRef::Named("VtsResponse")],
    },
];

/// Decoded MusicIndex types with no per-field schema check, and the recorded
/// reason (ADR 0075 section 6, Required Changes 2).
const CONTRACT_TYPES_WITHOUT_A_FIELD_CHECK: &[(&str, &str)] = &[
    (
        "PublisherLinkResolution",
        "a transparent string enum, not an object. MusicIndex sends it as the \
plain string value of `PublisherRelationship.publisher_link_resolution`, a \
field the map already checks. It has no properties of its own to declare.",
    ),
    (
        "RoleSource",
        "a transparent string enum, not an object. MusicIndex sends it as the \
plain string value of `PublisherRelationship.role_source`, a field the map \
already checks. It has no properties of its own to declare.",
    ),
    (
        "LiveItemCreateResponse",
        "maps to another service: it decodes `POST /v1/liveitems` of the live \
relay (archived ADR 0018), not the MusicIndex contract.",
    ),
    (
        "LiveMetadataSnapshot",
        "maps to another service: it decodes `GET /v1/liveitems/{event_id}/metadata` \
of the live relay (archived ADR 0018), not the MusicIndex contract.",
    ),
    (
        "EntityDetail",
        "wraps the already-mapped `Feed` type in Rust code. The app builds it \
from a decoded `Feed`; it never deserializes `EntityDetail` from a wire \
response.",
    ),
];

/// Reads the stored MusicIndex contract fixture as a JSON document.
fn load_musicindex_contract() -> serde_json::Value {
    let text = read_source(&manifest_path(MUSICINDEX_CONTRACT_FIXTURE));
    serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!("ADR 0075 task 051: {MUSICINDEX_CONTRACT_FIXTURE} is not valid JSON: {error}")
    })
}

/// Returns the property names of one named contract schema.
fn named_schema_properties(contract: &serde_json::Value, schema_name: &str) -> BTreeSet<String> {
    contract["components"]["schemas"][schema_name]["properties"]
        .as_object()
        .unwrap_or_else(|| {
            panic!(
                "ADR 0075 task 051: {MUSICINDEX_CONTRACT_FIXTURE} has no schema named `{schema_name}`"
            )
        })
        .keys()
        .cloned()
        .collect()
}

/// Returns the property names of one inline path response schema.
fn inline_response_schema_properties(
    contract: &serde_json::Value,
    path: &str,
    method: &str,
    status: &str,
) -> BTreeSet<String> {
    contract["paths"][path][method]["responses"][status]["content"]["application/json"]["schema"]
        ["properties"]
        .as_object()
        .unwrap_or_else(|| {
            panic!(
                "ADR 0075 task 051: {MUSICINDEX_CONTRACT_FIXTURE} has no response schema at \
{method} {path} {status}"
            )
        })
        .keys()
        .cloned()
        .collect()
}

/// Returns the property names that `schema_ref` declares.
fn schema_ref_properties(
    contract: &serde_json::Value,
    schema_ref: &ContractSchemaRef,
) -> BTreeSet<String> {
    match schema_ref {
        ContractSchemaRef::Named(name) => named_schema_properties(contract, name),
        ContractSchemaRef::Inline {
            path,
            method,
            status,
        } => inline_response_schema_properties(contract, path, method, status),
    }
}

/// Joins each schema's display name for a failure message.
fn schema_descriptions(schemas: &[ContractSchemaRef]) -> String {
    schemas
        .iter()
        .map(ContractSchemaRef::display_name)
        .collect::<Vec<_>>()
        .join(" or ")
}

/// Returns the text inside the outer braces of `pub struct {name}` (or
/// `pub enum {name}`) in `source`. It counts braces so a field's generic
/// type, such as `Option<Vec<Track>>`, does not close the struct early.
fn struct_or_enum_body<'a>(source: &'a str, name: &str) -> &'a str {
    let marker = format!("struct {name}");
    let header_index = source
        .match_indices(&marker)
        .map(|(index, _)| index)
        .find(|index| {
            let after = &source[index + marker.len()..];
            after.starts_with('<') || after.starts_with(' ') || after.starts_with('{')
        })
        .unwrap_or_else(|| {
            panic!(
                "ADR 0075 task 051: `{name}` is missing its own struct declaration in src/api.rs"
            )
        });
    let open_offset = source[header_index..]
        .find('{')
        .unwrap_or_else(|| panic!("ADR 0075 task 051: `{name}` has no struct body in src/api.rs"));
    let open_index = header_index + open_offset;
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut index = open_index;
    loop {
        assert!(
            index < bytes.len(),
            "ADR 0075 task 051: `{name}` has no closing brace in src/api.rs"
        );
        match bytes[index] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[open_index + 1..index];
                }
            }
            _ => {}
        }
        index += 1;
    }
}

/// Returns the string value of a `key = "value"` entry inside one
/// `#[serde(...)]` attribute line.
fn attribute_string_value(line: &str, key: &str) -> Option<String> {
    let marker = format!("{key} = \"");
    let start = line.find(&marker)? + marker.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Returns `true` when one `#[serde(...)]` attribute line carries a bare
/// `skip`, which exempts the field below it from this guard (ADR 0075
/// section 6, Required Changes 3). `skip_serializing_if` does not count: the
/// field still decodes from a wire response.
fn has_bare_skip_attribute(line: &str) -> bool {
    let Some(start) = line.find("serde(") else {
        return false;
    };
    let rest = &line[start + "serde(".len()..];
    let Some(end) = rest.rfind(')') else {
        return false;
    };
    rest[..end].split(',').any(|part| part.trim() == "skip")
}

/// Extracts the JSON field names that `struct_name` decodes, applying each
/// `#[serde(rename = "...")]` and dropping a field marked `#[serde(skip)]`
/// (ADR 0075 section 6). It reads one attribute per line, which matches this
/// file's formatting. `#[serde(alias = "...")]` keeps a field's primary name
/// as its own backward-compatible alias; this guard checks only the primary
/// name the operator accepted for the stored contract to declare.
fn decoded_field_names(source: &str, struct_name: &str) -> Vec<String> {
    let body = struct_or_enum_body(source, struct_name);
    let mut names = Vec::new();
    let mut pending_rename: Option<String> = None;
    let mut pending_skip = false;
    for raw_line in body.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("///") || line.starts_with("//") {
            continue;
        }
        if line.starts_with("#[serde(") {
            assert!(
                !line.contains("flatten"),
                "ADR 0075 task 051: `{struct_name}` uses #[serde(flatten)], which \
decoded_field_names does not yet read. Extend it before mapping this type."
            );
            if let Some(rename) = attribute_string_value(line, "rename") {
                pending_rename = Some(rename);
            }
            if has_bare_skip_attribute(line) {
                pending_skip = true;
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some(field_name) = field_declaration_name(line) {
            if !pending_skip {
                names.push(pending_rename.take().unwrap_or(field_name));
            }
            pending_rename = None;
            pending_skip = false;
        }
    }
    names
}

/// Returns the field name of one `pub field_name: Type,` struct body line.
fn field_declaration_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("pub ")?;
    let colon = rest.find(':')?;
    Some(rest[..colon].trim().to_string())
}

/// Returns each field in `decoded` that no schema in `declared` names
/// (ADR 0075 section 6).
fn undeclared_fields(decoded: &[String], declared: &BTreeSet<String>) -> Vec<String> {
    decoded
        .iter()
        .filter(|field| !declared.contains(field.as_str()))
        .cloned()
        .collect()
}

/// Builds the failure message for one undeclared field, in the form ADR
/// 0075 task 051 specifies.
fn contract_field_violation(rust_type: &str, field: &str, schema_description: &str) -> String {
    format!(
        "ADR 0075 section 6: api::{rust_type} decodes `{field}`, and MusicIndex contract 0.2.0 \
does not declare it in {schema_description}.\n\
Remove the field, or replace {MUSICINDEX_CONTRACT_FIXTURE} with the contract that declares it."
    )
}

/// Returns the name of every `struct` or `enum` in `source` whose
/// `#[derive(...)]` list names `Deserialize` (ADR 0075 section 6, R51-04).
/// This is the live census of decoded MusicIndex types: it reads
/// `src/api.rs` itself, instead of a fixed list that a later change could
/// outgrow.
fn deserialize_derived_type_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut search_from = 0usize;
    while let Some(relative_start) = source[search_from..].find("#[derive(") {
        let start = search_from + relative_start;
        let content_start = start + "#[derive(".len();
        let Some(relative_end) = source[content_start..].find(")]") else {
            break;
        };
        let content_end = content_start + relative_end;
        let derive_list = &source[content_start..content_end];
        search_from = content_end + ")]".len();

        let names_deserialize = derive_list
            .split(',')
            .any(|name| name.trim() == "Deserialize");
        if !names_deserialize {
            continue;
        }
        if let Some(type_name) = next_struct_or_enum_name(&source[search_from..]) {
            names.push(type_name);
        }
    }
    names
}

/// Returns the name after the next `struct ` or `enum ` keyword in `rest`.
/// A mapped type's derive attributes sit right above its own declaration,
/// with only other attributes or doc comments between them, so the nearest
/// keyword names the derived type.
fn next_struct_or_enum_name(rest: &str) -> Option<String> {
    let struct_index = rest.find("struct ");
    let enum_index = rest.find("enum ");
    let (keyword_index, keyword_length) = match (struct_index, enum_index) {
        (Some(struct_at), Some(enum_at)) if enum_at < struct_at => (enum_at, "enum ".len()),
        (Some(struct_at), _) => (struct_at, "struct ".len()),
        (None, Some(enum_at)) => (enum_at, "enum ".len()),
        (None, None) => return None,
    };
    let after_keyword = &rest[keyword_index + keyword_length..];
    let name_end = after_keyword
        .find(|ch: char| ch.is_whitespace() || ch == '<' || ch == '{' || ch == '(')
        .unwrap_or(after_keyword.len());
    let name = after_keyword[..name_end].trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Builds the failure message for one `Deserialize` type that the census
/// found in neither list, in the form ADR 0075 task 051 specifies.
fn contract_census_violation(rust_type: &str) -> String {
    format!(
        "ADR 0075 section 6: api::{rust_type} derives `Deserialize`, and tests/architecture_tests.rs \
maps it in neither CONTRACT_TYPE_MAP nor CONTRACT_TYPES_WITHOUT_A_FIELD_CHECK.\n\
Add the type to the contract type map, or record its cause, in tests/architecture_tests.rs."
    )
}

/// R51-01: the guard passes on the present code and the stored `0.2.0`
/// contract.
#[test]
fn adr_0075_task_051_contract_field_guard_passes_on_present_code() {
    let source = production_source(&read_source(&manifest_path("src/api.rs"))).to_owned();
    let contract = load_musicindex_contract();
    let mut violations = Vec::new();

    for mapping in CONTRACT_TYPE_MAP {
        let decoded = decoded_field_names(&source, mapping.rust_type);
        let mut declared = BTreeSet::new();
        for schema_ref in mapping.schemas {
            declared.extend(schema_ref_properties(&contract, schema_ref));
        }
        for field in undeclared_fields(&decoded, &declared) {
            violations.push(contract_field_violation(
                mapping.rust_type,
                &field,
                &schema_descriptions(mapping.schemas),
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0075 task 051 contract field guard violations:\n{}",
        violations.join("\n")
    );
}

/// R51-02: a sample struct with a field the schema does not declare gives
/// one failure that names ADR 0075.
#[test]
fn adr_0075_task_051_contract_field_guard_r51_02_flags_one_undeclared_field() {
    let sample = "
pub struct SampleType {
    pub known_field: Option<String>,
    pub mystery_field: Option<String>,
}
";
    let decoded = decoded_field_names(sample, "SampleType");
    let declared: BTreeSet<String> = ["known_field".to_string()].into_iter().collect();
    let violations: Vec<String> = undeclared_fields(&decoded, &declared)
        .iter()
        .map(|field| contract_field_violation("SampleType", field, "SampleSchema"))
        .collect();

    assert_eq!(violations.len(), 1, "expected exactly one failure");
    assert!(violations[0].contains("ADR 0075 section 6"));
    assert!(violations[0].contains("mystery_field"));
}

/// R51-03: a sample struct with a renamed field passes when the rename
/// matches the schema.
#[test]
fn adr_0075_task_051_contract_field_guard_r51_03_rename_matches_schema() {
    let sample = "
pub struct SampleType {
    #[serde(rename = \"wire_name\")]
    pub local_name: Option<String>,
}
";
    let decoded = decoded_field_names(sample, "SampleType");
    let declared: BTreeSet<String> = ["wire_name".to_string()].into_iter().collect();

    assert!(undeclared_fields(&decoded, &declared).is_empty());
}

/// ADR 0075 section 6, Required Changes 3: a field marked `#[serde(skip)]`
/// is exempt from this guard, and `skip_serializing_if` does not exempt a
/// field, because the field still decodes from a wire response.
#[test]
fn adr_0075_task_051_contract_field_guard_skip_attribute_is_exempt() {
    let sample = "
pub struct SampleType {
    #[serde(skip)]
    pub local_only_field: Option<String>,
    #[serde(skip_serializing_if = \"Option::is_none\")]
    pub wire_field: Option<String>,
}
";
    let decoded = decoded_field_names(sample, "SampleType");
    assert_eq!(decoded, vec!["wire_field".to_string()]);
}

/// R51-04 (orchestrator review, 2026-10-01): the census reads `src/api.rs`
/// itself. Each struct or enum that derives `Deserialize` needs a
/// `CONTRACT_TYPE_MAP` entry or a `CONTRACT_TYPES_WITHOUT_A_FIELD_CHECK`
/// entry.
#[test]
fn adr_0075_task_051_contract_field_guard_r51_04_finds_every_decoded_type_from_the_file() {
    let source = production_source(&read_source(&manifest_path("src/api.rs"))).to_owned();
    let found_types = deserialize_derived_type_names(&source);
    assert!(
        found_types.len() >= 20,
        "ADR 0075 task 051: the census scan found only {} Deserialize types in src/api.rs; \
check the scan itself before trusting this count",
        found_types.len()
    );

    let mapped: BTreeSet<&str> = CONTRACT_TYPE_MAP.iter().map(|m| m.rust_type).collect();
    let recorded: BTreeSet<&str> = CONTRACT_TYPES_WITHOUT_A_FIELD_CHECK
        .iter()
        .map(|(name, _)| *name)
        .collect();
    let mut violations = Vec::new();
    for type_name in &found_types {
        if !mapped.contains(type_name.as_str()) && !recorded.contains(type_name.as_str()) {
            violations.push(contract_census_violation(type_name));
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0075 task 051 contract census violations:\n{}",
        violations.join("\n")
    );
}

/// Orchestrator review, 2026-10-01: a sample source with one new
/// `Deserialize` struct, named in neither list, fails the census with the
/// ADR 0075 section 6 message.
#[test]
fn adr_0075_task_051_contract_field_guard_census_flags_an_unmapped_deserialize_type() {
    let sample = "
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnmappedSampleType {
    pub field: Option<String>,
}
";
    let found_types = deserialize_derived_type_names(sample);
    assert_eq!(found_types, vec!["UnmappedSampleType".to_string()]);

    let mapped: BTreeSet<&str> = CONTRACT_TYPE_MAP.iter().map(|m| m.rust_type).collect();
    let recorded: BTreeSet<&str> = CONTRACT_TYPES_WITHOUT_A_FIELD_CHECK
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert!(!mapped.contains("UnmappedSampleType"));
    assert!(!recorded.contains("UnmappedSampleType"));

    let violation = contract_census_violation("UnmappedSampleType");
    assert!(violation.contains("ADR 0075 section 6"));
    assert!(violation.contains("UnmappedSampleType"));
    assert!(violation.contains("tests/architecture_tests.rs"));
}
