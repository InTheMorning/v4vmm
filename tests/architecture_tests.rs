use std::collections::BTreeMap;
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
        file: "src/search.rs",
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
        file: "src/search.rs",
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
        file: "src/search.rs",
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
        file: "src/search.rs",
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
        file: "src/search.rs",
        pattern: "color::diff_",
        max_count: 0,
    },
    DiffHelperBaseline {
        file: "src/search.rs",
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
        file: "src/search.rs",
        pattern: "fn render_add_to_playlist_panel_search(",
        max_count: 0,
        note: "legacy Discover inspector playlist panel",
    },
    ScreenLocalPlaylistPopoverBaseline {
        file: "src/search.rs",
        pattern: ".when(frame.add_to_playlist_open, |el|",
        max_count: 0,
        note: "legacy Discover inspector playlist panel toggle",
    },
    ScreenLocalPlaylistPopoverBaseline {
        file: "src/search.rs",
        pattern: "fn render_row_playlist_popup(",
        max_count: 0,
        note: "legacy Discover row popup compatibility wrapper",
    },
];

const RENDER_HELPER_DUPLICATION_BASELINES: &[RenderHelperDuplicationBaseline] = &[];

const PLAYLIST_POPOVER_CALLSITE_FILES: &[&str] =
    &["src/library.rs", "src/search.rs", "src/ui/shells/track.rs"];

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
    "src/app/bootstrap.rs",
    "src/app/events.rs",
    "src/app/keyboard.rs",
    "src/app/playback_bar.rs",
    "src/app/tab_bar.rs",
    "src/library.rs",
    "src/search.rs",
];

const PRESENTATION_GLUE_FILES: &[&str] = &[
    "src/app.rs",
    "src/app/playback_bar.rs",
    "src/app/tab_bar.rs",
    "src/library.rs",
    "src/search.rs",
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

    for file in ["src/search.rs", "src/library.rs"] {
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
    for file in SCREEN_FILES {
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
fn screens_do_not_call_migrated_playlist_service_paths() {
    let mut violations = Vec::new();
    for file in SCREEN_FILES {
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
    for file in SCREEN_FILES {
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
fn screens_do_not_call_migrated_feed_update_paths() {
    let mut violations = Vec::new();
    for file in SCREEN_FILES {
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
    for file in SCREEN_FILES {
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
fn screens_do_not_add_unapproved_hardcoded_dark_defaults() {
    let mut violations = Vec::new();
    for file in SCREEN_FILES {
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
    for file in SCREEN_FILES {
        let path = manifest_path(file);
        let source = read_source(&path);
        for baseline in DEPRECATED_VISUAL_HELPER_BASELINES {
            if baseline.file == *file {
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
    for file in SCREEN_FILES {
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
            "src/view_models/search.rs",
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
            "NowPlayingData",
            "src/ui/composites/now_playing_bar.rs",
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
    for file in SCREEN_FILES {
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

    for file in SCREEN_FILES {
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
    for file in SCREEN_FILES {
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
fn library_release_detail_playlist_popovers_use_shared_composite() {
    let path = manifest_path("src/library.rs");
    let source = read_source(&path);
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        for pattern in RELEASE_PLAYLIST_POPOVER_FORBIDDEN_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/library.rs:{line_number}: ADR 0032 Library release-detail playlist chrome must use `AddToPlaylistPopover`, not `{pattern}`: `{line}`"
                ));
            }
        }
    }

    let shared_popover_count = source
        .lines()
        .map(strip_line_comment)
        .filter(|line| line.contains("AddToPlaylistPopover::new("))
        .count();
    if shared_popover_count < 2 {
        violations.push(format!(
            "src/library.rs: ADR 0032 expects Library feed and track release-detail playlist actions to use `AddToPlaylistPopover`; found {shared_popover_count} call(s)"
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
    let source = read_source(&manifest_path("src/library.rs"));
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
                "src/library.rs: advanced Library provenance grid must use shared `{required}` grammar"
            ));
        }
    }

    for forbidden in [
        "w(layout::COMPACT_COLUMN_WIDTH)",
        "w(layout::METADATA_LABEL_WIDTH)",
    ] {
        if source.contains(forbidden) {
            violations.push(format!(
                "src/library.rs: advanced provenance cell widths belong in `src/ui/composites/track_metadata_grid.rs`, not screen-local `{forbidden}`"
            ));
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

    for file in SCREEN_FILES {
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

    for file in SCREEN_FILES {
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
        "src/ui/composites/track_inspector_pane.rs",
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
        "src/ui/composites/track_inspector_pane.rs",
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

    let search = read_source(&manifest_path("src/search.rs"));
    if search.contains("fn render_nostr_icon_button") {
        violations.push(
            "src/search.rs: ADR 0037 track Nostr identity links must not keep a screen-local Nostr button renderer"
                .to_string(),
        );
    }
    if search.contains("render_nostr_icon_button(npub, \"track\"") {
        violations.push(
            "src/search.rs: ADR 0037 track Nostr identity links must be rendered by `ui::shells::track::render_track_page_identity_actions`"
                .to_string(),
        );
    }
    if !(search.contains("render_track_page_identity_actions(&detail_page)")
        && !search.contains("\"discover-track\""))
    {
        violations.push(
            "src/search.rs: ADR 0037 Discover track detail must call `render_track_page_identity_actions(&detail_page)` and leave the prefix in TrackDetailPageVm"
                .to_string(),
        );
    }

    let library = read_source(&manifest_path("src/library.rs"));
    if !(library.contains("render_track_page_identity_actions(&detail_page)")
        && !library.contains("\"library-track\""))
    {
        violations.push(
            "src/library.rs: ADR 0037 Library track detail must call `render_track_page_identity_actions(&detail_page)` and leave the prefix in TrackDetailPageVm"
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

    for file in SCREEN_FILES {
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

    for file in SCREEN_FILES {
        let source = read_source(&manifest_path(file));
        for (line_number, line) in code_lines(&source) {
            for pattern in forbidden {
                if line.contains(pattern) {
                    violations.push(format!(
                        "{file}:{line_number}: track row chrome must be owned by `TrackRow` through `TrackRowVm`, not locally rebuilt; found `{pattern}` in `{line}`"
                    ));
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

    for file in ["src/search.rs", "src/library.rs"] {
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
            "src/search.rs",
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
            "src/library.rs",
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
            "src/library.rs",
            "TrackDetailVm::new(",
            ".page()",
            "track::build_track_detail_surface(",
        ),
        (
            "Discover track detail",
            "src/search.rs",
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

    for file in ["src/library.rs", "src/search.rs"] {
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

    for file in SCREEN_FILES {
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
            "src/search.rs",
            "feed_link_label.unwrap_or_else",
            "Discover track feed-link label fallback belongs in TrackInspectorHeaderVm::feed_link_display",
        ),
        (
            "src/search.rs",
            "header_vm.feed_link_label(",
            "Discover track feed-link label should enter the screen through TrackFeedLinkDisplay",
        ),
        (
            "src/search.rs",
            "header_vm.feed_link_url()",
            "Discover track feed-link URL should enter the screen through TrackFeedLinkDisplay",
        ),
        (
            "src/search.rs",
            "let tooltip = guid.clone();",
            "Discover track feed-link tooltip should enter the screen through TrackFeedLinkDisplay",
        ),
        (
            "src/search.rs",
            "route.address.clone().unwrap_or_default()",
            "payment-route address presence belongs in PaymentRouteVm::address",
        ),
        (
            "src/search.rs",
            "route.address.is_some()",
            "payment-route address presence belongs in PaymentRouteVm::address",
        ),
        (
            "src/search.rs",
            "route.custom_key.is_some()",
            "payment-route custom field presence belongs in PaymentRouteVm::custom_fields",
        ),
        (
            "src/search.rs",
            "route.custom_value.is_some()",
            "payment-route custom field presence belongs in PaymentRouteVm::custom_fields",
        ),
        (
            "src/search.rs",
            "&route.custom_key",
            "payment-route custom field display belongs in PaymentRouteVm::custom_fields",
        ),
        (
            "src/search.rs",
            "&route.custom_value",
            "payment-route custom field display belongs in PaymentRouteVm::custom_fields",
        ),
        (
            "src/search.rs",
            "vm.recipient_name()",
            "payment-route primary summary belongs in PaymentRouteVm::summary",
        ),
        (
            "src/search.rs",
            "vm.route_type()",
            "payment-route primary summary belongs in PaymentRouteVm::summary",
        ),
        (
            "src/search.rs",
            "vm.kind_label()",
            "payment-route primary summary belongs in PaymentRouteVm::summary",
        ),
        (
            "src/search.rs",
            "let split = vm.split()",
            "payment-route primary summary belongs in PaymentRouteVm::summary",
        ),
        (
            "src/search.rs",
            "feed.feed_guid.clone().unwrap_or_default()",
            "Discover feed-list tile id fallback belongs in RecentFeedTileVm::display",
        ),
        (
            "src/search.rs",
            "feed.tracks.clone().unwrap_or_default()",
            "Discover feed-inspector missing-track fallback belongs in SearchViewModel::feed_inspector_tracks",
        ),
        (
            "src/search.rs",
            "let episode_note =",
            "Discover feed-list episode note belongs in RecentFeedTileVm::display",
        ),
        (
            "src/search.rs",
            "Label::new(feed_display_title(&feed))",
            "Discover feed-list title fallback belongs in RecentFeedTileVm::display",
        ),
        (
            "src/search.rs",
            "let guid = display.id.clone()",
            "Discover feed-list navigation id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/search.rs",
            "SharedString::from(display.feed_list_tile_id)",
            "Discover feed-list tile id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/search.rs",
            "let click_guid = display.id.clone()",
            "Discover podroll tile id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/search.rs",
            "SharedString::from(display.podroll_tile_id)",
            "Discover podroll tile id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/search.rs",
            "let element_id = link.element_id",
            "Discover track feed-link display should be consumed from TrackFeedLinkDisplay",
        ),
        (
            "src/search.rs",
            "let title = link.label",
            "Discover track feed-link label should be consumed from TrackFeedLinkDisplay",
        ),
        (
            "src/search.rs",
            "let display = PublisherLinkDisplay::new",
            "Discover publisher link display should be consumed from PublisherLinkDisplay",
        ),
        (
            "src/search.rs",
            "let guid = match feed.feed_guid.clone()",
            "Discover recent-feed navigation id should be consumed from RecentFeedTileDisplay",
        ),
        (
            "src/search.rs",
            "SharedString::from(audio_display.button_id.clone())",
            "Discover track play-button id should be consumed by the TrackPlayAudioDisplay renderer",
        ),
        (
            "src/search.rs",
            "display.recent_tile_id.clone()",
            "Discover recent-feed tile id should be consumed by RecentFeedTile",
        ),
        (
            "src/search.rs",
            "snapshot.status.display_text.clone()",
            "Discover status display text should be consumed from SearchRenderSnapshot",
        ),
        (
            "src/search.rs",
            "release_subscription_action.label.clone()",
            "Discover feed subscription action label should be consumed from EntityActionVm",
        ),
        (
            "src/search.rs",
            "action.label.clone()",
            "Discover track row action labels should be consumed from EntityActionVm",
        ),
        (
            "src/search.rs",
            "self.label.clone()",
            "Discover metadata drag preview should consume TrackMetadataDragPreviewDisplay",
        ),
        (
            "src/search.rs",
            "self.value.clone()",
            "Discover metadata drag preview should consume TrackMetadataDragPreviewDisplay",
        ),
        (
            "src/search.rs",
            "self.display.label.clone()",
            "Discover metadata drag preview should consume TrackMetadataDragPreviewDisplay without renderer-side label cloning",
        ),
        (
            "src/search.rs",
            "self.display.value.clone()",
            "Discover metadata drag preview should consume TrackMetadataDragPreviewDisplay without renderer-side value cloning",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
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
            "src/search.rs",
            "row.rss_value.as_deref().unwrap_or(\"\")",
            "metadata RSS cell value fallback belongs in TrackMetadataGridVm::rss_cell_value",
        ),
        (
            "src/library.rs",
            ".or(row.id3_value.as_deref())",
            "metadata ID3 cell value fallback belongs in TrackMetadataGridVm::id3_cell_value",
        ),
        (
            "src/search.rs",
            ".or(row.id3_value.as_deref())",
            "metadata ID3 cell value fallback belongs in TrackMetadataGridVm::id3_cell_value",
        ),
        (
            "src/library.rs",
            ".or(row.id3_frame.as_deref())",
            "metadata ID3 cell frame fallback belongs in TrackMetadataGridVm::id3_cell_frame",
        ),
        (
            "src/search.rs",
            ".or(row.id3_frame.as_deref())",
            "metadata ID3 cell frame fallback belongs in TrackMetadataGridVm::id3_cell_frame",
        ),
        (
            "src/search.rs",
            "row.id3_frame.clone().unwrap_or_default()",
            "metadata drag frame fallback belongs in TrackMetadataGridVm::id3_drag_frame",
        ),
        (
            "src/search.rs",
            "frame_id_owned.unwrap_or_default()",
            "metadata ID3 displayed frame label fallback belongs in TrackMetadataGridVm::id3_frame_label",
        ),
        (
            "src/search.rs",
            "frame_id.unwrap_or_default()",
            "metadata ID3 displayed frame label fallback belongs in TrackMetadataGridVm::id3_frame_label",
        ),
        (
            "src/library.rs",
            ".child(SharedString::from(row.field.clone()))",
            "Library metadata field label display belongs in TrackMetadataGridVm::field_label",
        ),
        (
            "src/search.rs",
            ".child(SharedString::from(row.field.clone()))",
            "Discover metadata field label display belongs in TrackMetadataGridVm::field_label",
        ),
        (
            "src/search.rs",
            "field: row.field.clone()",
            "Discover metadata drag field label display belongs in TrackMetadataGridVm::field_label",
        ),
        (
            "src/search.rs",
            "label: drag.field.clone()",
            "Discover metadata drag preview label belongs in TrackMetadataGridVm::drag_preview_display",
        ),
        (
            "src/search.rs",
            "value: drag.value.clone()",
            "Discover metadata drag preview value belongs in TrackMetadataGridVm::drag_preview_display",
        ),
        (
            "src/library.rs",
            "label: SharedString::from(frame_id.to_string())",
            "Library metadata ID3 frame label display belongs in TrackMetadataGridVm::id3_frame_display_label",
        ),
        (
            "src/search.rs",
            "SharedString::from(frame_label.to_string())",
            "Discover metadata ID3 frame label display belongs in TrackMetadataGridVm::id3_frame_display_label",
        ),
        (
            "src/search.rs",
            "SharedString::from(frame_label.clone())",
            "Discover metadata ID3 frame label display should be consumed without renderer-side cloning",
        ),
        (
            "src/library.rs",
            "fn id3_frame_color(frame_id: &str)",
            "Library metadata ID3 frame color role belongs in TrackMetadataGridVm::id3_frame_color_role",
        ),
        (
            "src/search.rs",
            "fn id3_frame_version_color(",
            "Discover metadata ID3 frame color role belongs in TrackMetadataGridVm::id3_frame_color_role",
        ),
        (
            "src/search.rs",
            "fn id3_frame_version(",
            "Discover metadata ID3 frame version classification belongs in metadata/view-model contracts",
        ),
        (
            "src/search.rs",
            "enum Id3FrameVersion",
            "Discover metadata ID3 frame version classification belongs in metadata/view-model contracts",
        ),
        (
            "src/search.rs",
            "frame.map(id3_frame_base).map(id3_frame_version_color)",
            "Discover metadata ID3 frame color role belongs in TrackMetadataGridVm::id3_frame_color_role",
        ),
        (
            "src/library.rs",
            "expanded_metadata_display_string(",
            "Library expanded metadata raw/display selection belongs in TrackMetadataGridVm::expanded_display_value",
        ),
        (
            "src/search.rs",
            "expanded_metadata_display_string(",
            "Discover expanded metadata raw/display selection belongs in TrackMetadataGridVm::expanded_display_value",
        ),
        (
            "src/library.rs",
            "SharedString::from(display_value.to_string())",
            "Library metadata text display values belong in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/search.rs",
            "SharedString::from(display_value.to_string())",
            "Discover metadata text display values belong in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/library.rs",
            "value: SharedString::from(value.to_string())",
            "Library metadata text value projection belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/search.rs",
            "MultilineText::new(value.to_string())",
            "Discover metadata text value projection belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/library.rs",
            "MultilineText::new(raw_value.to_string())",
            "Library expanded metadata raw fallback belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/search.rs",
            "SharedString::from(line.to_string())",
            "Discover expanded metadata line display belongs in TrackMetadataGridVm::text_value_display",
        ),
        (
            "src/search.rs",
            "SharedString::from(raw_value.to_string())",
            "Discover expanded artwork URL display belongs in TrackMetadataGridVm::artwork_url_display",
        ),
        (
            "src/search.rs",
            "fn muted_line(value: &str)",
            "Discover deferred-panel empty-line display belongs in SearchViewModel::deferred_panel_empty_line",
        ),
        (
            "src/search.rs",
            "SharedString::from(value.to_string())",
            "Discover deferred-panel empty-line display belongs in SearchViewModel::deferred_panel_empty_line",
        ),
        (
            "src/search.rs",
            "title: title.to_string().into()",
            "Discover feed header title display belongs in SearchViewModel::feed_header_display",
        ),
        (
            "src/search.rs",
            ".filter(|value| !value.trim().is_empty())",
            "Discover feed header subtitle filtering belongs in SearchViewModel::feed_header_display",
        ),
        (
            "src/search.rs",
            "const TYPE_LABELS",
            "Discover type-filter labels belong in SearchViewModel::type_filter_options",
        ),
        (
            "src/search.rs",
            "const TYPE_VALUES",
            "Discover type-filter query values belong in SearchViewModel::type_filter_value",
        ),
        (
            "src/search.rs",
            "TYPE_VALUES[intent.type_filter()]",
            "Discover type-filter query values belong in SearchViewModel::type_filter_value",
        ),
        (
            "src/search.rs",
            ".label(SharedString::from(label.to_string()))",
            "Discover type-filter labels belong in SearchViewModel::type_filter_options",
        ),
        (
            "src/search.rs",
            "render_feed_list_section(\"Feeds\"",
            "Discover feed-list section heading belongs in SearchViewModel::feed_list_section_display",
        ),
        (
            "src/search.rs",
            "SectionHeader::new(heading.to_string())",
            "Discover feed-list section heading belongs in SearchViewModel::feed_list_section_display",
        ),
        (
            "src/search.rs",
            "SharedString::from(row.entity_type.clone())",
            "Discover result type badge label belongs in ResultRowDisplay",
        ),
        (
            "src/search.rs",
            "Label::new(title.to_string())",
            "Discover inspector title display belongs in SearchViewModel::inspector_title_display",
        ),
        (
            "src/search.rs",
            "vm.add_to_playlist_label().to_string()",
            "Discover playlist trigger fallback belongs in ActionRowVm::playlist_trigger_label",
        ),
        (
            "src/search.rs",
            "group_heading(group.to_string())",
            "Discover payment-route group heading belongs in PaymentRouteVm::group_display",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "display.disclosure_id.as_deref()",
            "Discover metadata disclosure id binding should consume TrackMetadataGridVm display ids directly",
        ),
        (
            "src/library.rs",
            "disclosure_id.to_string()",
            "Library metadata disclosure id binding should not re-project VM display ids",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "playlist.name.clone()",
            "Discover playlist popover option display belongs in playlist_option_displays",
        ),
        (
            "src/ui/shells/track.rs",
            "playlist.name.clone()",
            "Track shell playlist popover option display belongs in playlist_option_displays",
        ),
        (
            "src/search.rs",
            "fn compare_row_id(",
            "Discover metadata compare-row slug display belongs in TrackMetadataGridVm::compare_row_id",
        ),
        (
            "src/search.rs",
            "format!(\"id3-unused-{}\"",
            "Discover unused ID3 frame row id belongs in TrackMetadataGridVm::unused_id3_frame_row_id",
        ),
        (
            "src/search.rs",
            "format!(\"id3-field-{}\"",
            "Discover used ID3 field row id belongs in TrackMetadataGridVm::used_id3_field_row_id",
        ),
        (
            "src/search.rs",
            "format!(\"ID3 {frame_id}\")",
            "Discover unused ID3 frame label belongs in TrackMetadataGridVm::id3_field_display_label",
        ),
        (
            "src/search.rs",
            "format!(\"ID3 {}\", field.frame_id)",
            "Discover used ID3 field label belongs in TrackMetadataGridVm::id3_field_display_label",
        ),
        (
            "src/search.rs",
            "format!(\"metadata-rss-drag-{}\"",
            "Discover RSS metadata source-drag id belongs in TrackMetadataGridVm::source_drag_display",
        ),
        (
            "src/search.rs",
            "format!(\"metadata-musicbrainz-drag-{}\"",
            "Discover MusicBrainz metadata source-drag id belongs in TrackMetadataGridVm::source_drag_display",
        ),
        (
            "src/library.rs",
            "summarize_contributor_value(raw_value).unwrap_or_else",
            "metadata contributor summary fallback belongs in TrackMetadataGridVm::contributor_summary",
        ),
        (
            "src/search.rs",
            "summarize_contributor_value(raw_value).unwrap_or_else",
            "metadata contributor summary fallback belongs in TrackMetadataGridVm::contributor_summary",
        ),
        (
            "src/library.rs",
            "format!(\"[{} items]\", arr.len())",
            "metadata value-route summary belongs in TrackMetadataGridVm::value_routes_summary",
        ),
        (
            "src/search.rs",
            "format!(\"[{} items]\", arr.len())",
            "metadata value-route summary belongs in TrackMetadataGridVm::value_routes_summary",
        ),
        (
            "src/search.rs",
            "format!(\"[{lines} lines]\")",
            "metadata value-route multiline fallback belongs in TrackMetadataGridVm::value_routes_summary",
        ),
        (
            "src/search.rs",
            "fn expandable_cell_summary(",
            "Discover expandable metadata summary policy belongs in TrackMetadataGridVm::expandable_cell_summary",
        ),
        (
            "src/library.rs",
            "raw_value.starts_with(\"http://\") || raw_value.starts_with(\"https://\")",
            "metadata artwork URL summary policy belongs in TrackMetadataGridVm::expandable_cell_summary",
        ),
        (
            "src/search.rs",
            "raw_value.starts_with(\"http://\") || raw_value.starts_with(\"https://\")",
            "metadata artwork URL summary policy belongs in TrackMetadataGridVm::artwork_url",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "if key == \"recipient_name\"",
            "Discover Value Routes child-field visibility belongs in TrackMetadataGridVm::value_route_child_field_is_visible",
        ),
        (
            "src/search.rs",
            "serde_json::Value::String(s) => s.clone()",
            "Discover JSON-tree scalar display belongs in TrackMetadataGridVm::json_tree_scalar_label",
        ),
        (
            "src/search.rs",
            "serde_json::Value::Null => \"null\".into()",
            "Discover JSON-tree null display belongs in TrackMetadataGridVm::json_tree_scalar_label",
        ),
        (
            "src/library.rs",
            "ActionRowMessageDisplay {",
            "Library action-row message tone/width belongs in VM display contracts",
        ),
        (
            "src/search.rs",
            "ActionRowMessageDisplay {",
            "Discover action-row message tone/width belongs in VM display contracts",
        ),
        (
            "src/library.rs",
            "ActionRowMessageTone::",
            "Library action-row message tone belongs in VM display contracts",
        ),
        (
            "src/search.rs",
            "ActionRowMessageTone::",
            "Discover action-row message tone belongs in VM display contracts",
        ),
        (
            "src/library.rs",
            "message_is_error()",
            "Library subscription message severity belongs in LibraryTrackActionVm::subscription_message_display",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "metadata_field_is_expandable(&row.field) && !value.is_empty()",
            "Discover metadata expandability gate belongs in TrackMetadataGridVm::field_is_expandable",
        ),
        (
            "src/library.rs",
            "logical_field == \"Value Routes\"",
            "Library expanded metadata field kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/search.rs",
            "field == \"Value Routes\"",
            "Discover expanded metadata field kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/library.rs",
            "field == \"Artwork\"",
            "Library expanded metadata artwork kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/search.rs",
            "field == \"Artwork\"",
            "Discover expanded metadata artwork kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/search.rs",
            "matches!(field, \"Artwork\")",
            "Discover expanded metadata artwork kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/search.rs",
            "matches!(field, \"Transcript\" | \"Transcript text\")",
            "Discover expanded transcript kind belongs in TrackMetadataGridVm::expanded_field_kind",
        ),
        (
            "src/library.rs",
            "format!(\"{} ({} unused)\", group.label, group.unused_count)",
            "metadata group heading fallback belongs in TrackMetadataGridVm::group_heading_label",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "format!(\"{key}: \")",
            "metadata value-route field key display belongs in TrackMetadataGridVm::value_route_field_key_label",
        ),
        (
            "src/library.rs",
            "fn route_value_label(",
            "metadata value-route field value display belongs in TrackMetadataGridVm::value_route_field_value_label",
        ),
        (
            "src/search.rs",
            "serde_json::Value::Bool(b) => b.to_string()",
            "metadata value-route field value display belongs in TrackMetadataGridVm::value_route_field_value_label",
        ),
        (
            "src/search.rs",
            "\"No audio URL\"",
            "track play-audio tooltip fallback belongs in TrackVm::play_audio_display",
        ),
        (
            "src/search.rs",
            "url.clone().unwrap_or_else(|| \"No audio URL\".into())",
            "track play-audio tooltip fallback belongs in TrackVm::play_audio_display",
        ),
        (
            "src/library.rs",
            "row.musicbrainz_value.as_deref().unwrap_or(\"\")",
            "metadata MusicBrainz cell value fallback belongs in TrackMetadataGridVm::musicbrainz_cell_value",
        ),
        (
            "src/search.rs",
            "row.musicbrainz_value.as_deref().unwrap_or(\"\")",
            "metadata MusicBrainz cell value fallback belongs in TrackMetadataGridVm::musicbrainz_cell_value",
        ),
        (
            "src/library.rs",
            "fn comparison_status_role(",
            "metadata comparison role display belongs in TrackMetadataGridVm::comparison_role",
        ),
        (
            "src/search.rs",
            "fn comparison_status_role(",
            "metadata comparison role display belongs in TrackMetadataGridVm::comparison_role",
        ),
        (
            "src/library.rs",
            "fn comparison_status_glyph(",
            "metadata comparison glyph display belongs in TrackMetadataGridVm::comparison_glyph",
        ),
        (
            "src/search.rs",
            "fn comparison_status_glyph(",
            "metadata comparison glyph display belongs in TrackMetadataGridVm::comparison_glyph",
        ),
        (
            "src/library.rs",
            "fn display_with_glyph(",
            "metadata glyph-prefix display belongs in TrackMetadataGridVm::display_with_glyph",
        ),
        (
            "src/search.rs",
            "fn display_with_glyph(",
            "metadata glyph-prefix display belongs in TrackMetadataGridVm::display_with_glyph",
        ),
        (
            "src/library.rs",
            "fn pending_source_role(",
            "metadata pending-source role display belongs in TrackMetadataGridVm::pending_source_role",
        ),
        (
            "src/search.rs",
            "fn source_cell_role(",
            "metadata pending-source role display belongs in TrackMetadataGridVm::pending_source_role",
        ),
        (
            "src/library.rs",
            "row.id3_value.is_some() && row.rss_value.is_none() && row.musicbrainz_value.is_none()",
            "metadata standalone-ID3 status fallback belongs in TrackMetadataGridVm::id3_status_role",
        ),
        (
            "src/search.rs",
            "row.id3_value.is_some() && row.rss_value.is_none() && row.musicbrainz_value.is_none()",
            "metadata standalone-ID3 status fallback belongs in TrackMetadataGridVm::id3_status_role",
        ),
        (
            "src/search.rs",
            "StatusRole::Danger.glyph()",
            "Discover status error-prefix display belongs in SearchStatusSnapshot",
        ),
        (
            "src/search.rs",
            "\"Fuzzy: On\"",
            "Discover fuzzy-toggle label display belongs in SearchRenderSnapshot",
        ),
        (
            "src/search.rs",
            "\"Fuzzy: Off\"",
            "Discover fuzzy-toggle label display belongs in SearchRenderSnapshot",
        ),
        (
            "src/search.rs",
            "\"No results\"",
            "Discover empty-results label display belongs in SearchRenderSnapshot",
        ),
        (
            "src/search.rs",
            "\"Load more\"",
            "Discover load-more label display belongs in SearchRenderSnapshot or RecentFeedsSnapshot",
        ),
        (
            "src/search.rs",
            "\"Recent Feeds\"",
            "Discover recent-feeds panel title belongs in RecentFeedsSnapshot",
        ),
        (
            "src/search.rs",
            "\"No recent feeds\"",
            "Discover recent-feeds empty label belongs in RecentFeedsSnapshot",
        ),
        (
            "src/search.rs",
            "format!(\"Open publisher: {publisher_text}\")",
            "Discover publisher-link tooltip display belongs in PublisherLinkDisplay",
        ),
        (
            "src/search.rs",
            "format!(\"Loading {title}...\")",
            "Discover inspector loading display belongs in SearchViewModel::inspector_loading_message",
        ),
        (
            "src/search.rs",
            "LoadingMessage::new(format!(\"Error: {error}\"))",
            "Discover inspector error display belongs in SearchViewModel::inspector_error_message",
        ),
        (
            "src/search.rs",
            "\"\u{2190} Back\"",
            "Discover inspector back label belongs in SearchViewModel::inspector_chrome_display",
        ),
        (
            "src/search.rs",
            "\"Select a result to inspect\"",
            "Discover empty-inspector label belongs in SearchViewModel::inspector_chrome_display",
        ),
        (
            "src/search.rs",
            "text_3xl().opacity(0.4).child(\"\u{1F50D}\")",
            "Discover empty-inspector icon belongs in SearchViewModel::inspector_chrome_display",
        ),
        (
            "src/search.rs",
            "\"Loading contributors...\"",
            "Discover contributor-panel loading label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/search.rs",
            "\"Loading value routes...\"",
            "Discover value-route-panel loading label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/search.rs",
            "SplitPane::new(\"pane-container\")",
            "Discover split-pane container id belongs in SearchViewModel render snapshot display",
        ),
        (
            "src/search.rs",
            "resize_handle_id(\"resize-handle\")",
            "Discover split-pane resize handle id belongs in SearchViewModel render snapshot display",
        ),
        (
            "src/search.rs",
            "\"No contributors found\"",
            "Discover contributor-panel empty label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/search.rs",
            "\"No value routes found\"",
            "Discover value-route-panel empty label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/search.rs",
            "id: \"section:contributors\".into()",
            "Discover contributor-panel heading id belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/search.rs",
            "label: \"Contributors\".into()",
            "Discover contributor-panel heading label belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/search.rs",
            "id: \"section:value-routes\".into()",
            "Discover value-route-panel heading id belongs in SearchViewModel::deferred_panel_display",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
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
            "src/search.rs",
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
            "src/search.rs",
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
            "src/search.rs",
            "EntityKind::from_legacy_str(&row.entity_type)",
            "Discover result thumbnail kind belongs with ResultRowDisplay kind projection",
        ),
        (
            "src/search.rs",
            "let key = row.key()",
            "Discover result row selection key belongs in ResultRowRenderItem",
        ),
        (
            "src/search.rs",
            "let entity_type = row.entity_type.clone()",
            "Discover result row navigation target belongs in ResultRowRenderItem",
        ),
        (
            "src/search.rs",
            "let entity_id = row.entity_id.clone()",
            "Discover result row navigation target belongs in ResultRowRenderItem",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "format!(\"contributor-website:{label}:{href}\")",
            "Discover contributor website action display belongs in ContributorRowVm::identity_actions",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
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
            "src/search.rs",
            "format!(\"expandable-rss-{}\", field)",
            "Discover RSS expandable cell id belongs in TrackMetadataGridVm::discover_expandable_cell_display",
        ),
        (
            "src/search.rs",
            "format!(\"expandable-rss-{}-hdr\", field)",
            "Discover RSS expandable header id belongs in TrackMetadataGridVm::discover_expandable_cell_display",
        ),
        (
            "src/search.rs",
            "format!(\"expandable-id3-{}\", field)",
            "Discover ID3 expandable cell id belongs in TrackMetadataGridVm::discover_expandable_cell_display",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "format!(\"vr-{column}-{i}\")",
            "Discover value-route item id belongs in TrackMetadataGridVm::discover_value_route_item_display",
        ),
        (
            "src/library.rs",
            "let glyph = if expanded",
            "Library metadata disclosure glyph belongs in TrackMetadataGridVm expandable display contracts",
        ),
        (
            "src/search.rs",
            "let glyph = if expanded",
            "Discover metadata disclosure glyph belongs in TrackMetadataGridVm expandable display contracts",
        ),
        (
            "src/library.rs",
            "let sub_glyph = if sub_expanded",
            "Library value-route disclosure glyph belongs in TrackMetadataGridVm value-route item display",
        ),
        (
            "src/search.rs",
            "let sub_glyph = if sub_expanded",
            "Discover value-route disclosure glyph belongs in TrackMetadataGridVm value-route item display",
        ),
        (
            "src/library.rs",
            "display.cell_key.clone()",
            "Library metadata expansion keys should be consumed by destructuring TrackMetadataExpandableCellDisplay",
        ),
        (
            "src/search.rs",
            "display.cell_key.clone()",
            "Discover metadata expansion keys should be consumed by destructuring TrackMetadataExpandableCellDisplay",
        ),
        (
            "src/library.rs",
            "display.item_key.clone()",
            "Library Value Routes item keys should be consumed by destructuring TrackMetadataValueRouteItemDisplay",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "format!(\"track-row-download-spin:{key}\")",
            "Discover track download spinner id display belongs in TrackRowActionVm::download_display",
        ),
        (
            "src/search.rs",
            "format!(\"track-row-download:{key}\")",
            "Discover track download button id display belongs in TrackRowActionVm::download_display",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "format!(\"feed-tile:{guid}\")",
            "Discover feed-list tile id display belongs in RecentFeedTileVm::display",
        ),
        (
            "src/search.rs",
            "format!(\"recent-tile:{guid}\")",
            "Discover recent-feed tile id display belongs in RecentFeedTileVm::display",
        ),
        (
            "src/search.rs",
            "format!(\"podroll-tile:{guid}\")",
            "Discover podroll tile id display belongs in RecentFeedTileVm::display",
        ),
        (
            "src/search.rs",
            "SharedString::from(\"track-play-audio\")",
            "Discover track-inspector play button id belongs in TrackVm::play_audio_display",
        ),
        (
            "src/search.rs",
            ".label(\"▶\")",
            "Discover play button glyph belongs in TrackVm::play_audio_display",
        ),
        (
            "src/search.rs",
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
            "format!(\"playlist-up-{pl_id}-{position}\")",
            "Library playlist move-up control id belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            "format!(\"playlist-down-{pl_id}-{position}\")",
            "Library playlist move-down control id belongs in PlaylistTrackRowVm::controls_display",
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
            ".label(\"▲\")",
            "Library playlist move-up glyph belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            ".label(\"▼\")",
            "Library playlist move-down glyph belongs in PlaylistTrackRowVm::controls_display",
        ),
        (
            "src/library.rs",
            ".label(\"✕\")",
            "Library playlist remove glyph belongs in PlaylistTrackRowVm::controls_display",
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
            "SharedString::from(controls_display.move_up_button_id.clone())",
            "Library playlist move-up id should be consumed from PlaylistTrackControlsDisplay",
        ),
        (
            "src/library.rs",
            "SharedString::from(controls_display.move_down_button_id.clone())",
            "Library playlist move-down id should be consumed from PlaylistTrackControlsDisplay",
        ),
        (
            "src/library.rs",
            "SharedString::from(controls_display.remove_button_id.clone())",
            "Library playlist remove id should be consumed from PlaylistTrackControlsDisplay",
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
            "src/search.rs",
            "LazyPanel::Empty(format!(\"Error: {error}\"))",
            "Discover deferred-panel error prefix belongs in LazyPanel",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            "Resolve duplicate ID3 target{}: {}",
            "Discover duplicate ID3 target message belongs in TrackMetadataActionState",
        ),
        (
            "src/library.rs",
            "format!(\"Error applying ID3 edits: {error}\")",
            "Library ID3 apply error message belongs in TrackMetadataActionState",
        ),
        (
            "src/search.rs",
            "format!(\"Error applying ID3 edits: {error}\")",
            "Discover ID3 apply error message belongs in TrackMetadataActionState",
        ),
        (
            "src/search.rs",
            "format!(\", applied {} ID3 edit{}\"",
            "Discover download success ID3 edit suffix belongs in SearchSubscriptionCommand",
        ),
        (
            "src/search.rs",
            "Some(format!(\"Downloaded track{edit_text}\"))",
            "Discover download success message belongs in SearchSubscriptionCommand",
        ),
        (
            "src/search.rs",
            ".child(\"🔍\")",
            "Discover results empty-state icon belongs in SearchPaneDisplay",
        ),
        (
            "src/search.rs",
            "\"result-item:{}:{}\"",
            "Discover result row id belongs in ResultRowDisplay",
        ),
        (
            "src/search.rs",
            ".child(\"Podroll\")",
            "Discover podroll heading label belongs in PodrollSectionDisplay",
        ),
        (
            "src/search.rs",
            "\"podroll-scroll:{}\"",
            "Discover podroll scroll id belongs in PodrollSectionDisplay",
        ),
        (
            "src/search.rs",
            "Button::new(\"search-btn\")",
            "Discover search button id belongs in SearchPaneDisplay",
        ),
        (
            "src/search.rs",
            "\"fuzzy-toggle\"",
            "Discover fuzzy-toggle id belongs in SearchPaneDisplay",
        ),
        (
            "src/search.rs",
            ".id(\"results-scroll\")",
            "Discover results scroll id belongs in SearchPaneDisplay",
        ),
        (
            "src/search.rs",
            "UiButton::styled(\"load-more\"",
            "Discover result load-more id belongs in SearchPaneDisplay",
        ),
        (
            "src/search.rs",
            "UiButton::styled(\"inspector-back\"",
            "Discover inspector back id belongs in InspectorChromeDisplay",
        ),
        (
            "src/search.rs",
            ".id(\"inspector-scroll\")",
            "Discover inspector scroll id belongs in InspectorChromeDisplay",
        ),
        (
            "src/search.rs",
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
            "src/search.rs",
            ".unwrap_or(false);",
            "boolean state fallback is command/control state, not display fallback",
        ),
        (
            "src/search.rs",
            ".unwrap_or(false)",
            "boolean state fallback is command/control state, not display fallback",
        ),
        (
            "src/search.rs",
            ".unwrap_or_else(|| row.entity_id.clone()),",
            "result navigation target fallback is identity routing, not display label fallback",
        ),
        (
            "src/search.rs",
            "artist_track_count_by_feed.get(guid).copied().unwrap_or(0);",
            "artist feed count fallback is numeric aggregation, not display fallback",
        ),
        (
            "src/search.rs",
            ".unwrap_or_default();",
            "podroll dedupe key fallback is feed identity plumbing, not display fallback",
        ),
        (
            "src/search.rs",
            ".unwrap_or(artist_context.tracks.len() as i32);",
            "artist track-count fallback is numeric aggregation, not display fallback",
        ),
        (
            "src/search.rs",
            ".unwrap_or_else(color::text_primary);",
            "metadata cell default color is token render chrome, not label fallback",
        ),
        (
            "src/search.rs",
            ".unwrap_or_else(|| id3_cell_status_color(row, cx));",
            "ID3 status default color is token render chrome, not label fallback",
        ),
        (
            "src/search.rs",
            ".unwrap_or_else(|| comparison_status_color(&row.musicbrainz_status, cx));",
            "MusicBrainz status default color is token render chrome, not label fallback",
        ),
        (
            "src/search.rs",
            "let frame_color = frame_color.unwrap_or_else(color::text_muted);",
            "ID3 frame default color is token render chrome, not label fallback",
        ),
        (
            "src/search.rs",
            "crate::view_models::track::fmt_dur((ms / 1000).try_into().unwrap_or(i32::MAX))",
            "duration range clamp is numeric conversion safety, not display fallback",
        ),
    ];
    let files = ["src/library.rs", "src/search.rs"];
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

fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(index) => &line[..index],
        None => line,
    }
}

fn compact_source(source: &str) -> String {
    source.chars().filter(|ch| !ch.is_whitespace()).collect()
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
        "src/search.rs" => nearby_source_mentions(
            source,
            line_number,
            &[
                "pub fn run_search_app()",
                "gpui_component::init(cx)",
                "cfg.ui_scale.into()",
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

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn rel_path(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}
