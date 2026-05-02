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

const RENDER_HELPER_DUPLICATION_BASELINES: &[RenderHelperDuplicationBaseline] = &[
    RenderHelperDuplicationBaseline {
        helper: "render_musicbrainz_panel",
        files: &["src/library.rs", "src/search.rs"],
        note: "post-ADR0033 MusicBrainz composite task",
    },
    RenderHelperDuplicationBaseline {
        helper: "render_musicbrainz_lookup",
        files: &["src/library.rs", "src/search.rs"],
        note: "post-ADR0033 MusicBrainz composite task",
    },
    RenderHelperDuplicationBaseline {
        helper: "render_musicbrainz_header",
        files: &["src/library.rs", "src/search.rs"],
        note: "post-ADR0033 MusicBrainz composite task",
    },
    RenderHelperDuplicationBaseline {
        helper: "render_musicbrainz_title_bar",
        files: &["src/library.rs", "src/search.rs"],
        note: "post-ADR0033 MusicBrainz composite task",
    },
];

const PLAYLIST_POPOVER_CALLSITE_FILES: &[&str] =
    &["src/library.rs", "src/search.rs", "src/ui_track.rs"];

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
    "src/ui_feed.rs",
    "src/ui_track.rs",
];

/// Top-level shared-UI shell modules. They live alongside screen modules at
/// `src/*.rs` rather than under `src/ui/` for legacy reasons, but they are
/// shared GPUI layout — not screen wiring. Adding a new shared top-level
/// shell requires adding the file here so the ADR 0033 backstop test can
/// distinguish it from an unclassified screen.
const KNOWN_SHARED_UI_SHELL_FILES: &[&str] = &["src/ui_artist.rs", "src/ui_entity.rs"];

const SCREEN_LOCAL_FLOATING_CHROME_FORBIDDEN_PATTERNS: &[&str] = &[
    "gpui_component::popover",
    "SurfaceElevation::Floating",
    ".absolute()",
    ".fixed()",
    ".z_index(",
];

#[derive(Debug)]
struct DeprecatedVisualHelperBaseline {
    file: &'static str,
    helper: &'static str,
    import_patterns: &'static [&'static str],
    usage_pattern: &'static str,
    max_count: usize,
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
    let path = manifest_path("src/ui_entity.rs");
    let source = read_source(&path);
    let mut violations = Vec::new();

    for (line_number, line) in code_lines(&source) {
        for pattern in UI_ENTITY_FORBIDDEN_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!(
                    "src/ui_entity.rs:{line_number}: ADR 0026 UI shell must stay slot-based and avoid screen/service imports; found `{pattern}` in `{line}`"
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
            || PRESENTATION_GLUE_FILES.iter().any(|file| *file == rel)
            || KNOWN_SHARED_UI_SHELL_FILES.iter().any(|file| *file == rel);
        if !classified {
            unclassified.push(rel);
        }
    }

    assert!(
        unclassified.is_empty(),
        "ADR 0033 backstop: every top-level GPUI-importing module must be classified as a screen, presentation glue, or shared-UI shell. Add the file to `SCREEN_FILES`, `PRESENTATION_GLUE_FILES`, or `KNOWN_SHARED_UI_SHELL_FILES` in tests/architecture_tests.rs in the same change that introduces it. Unclassified files:\n{}",
        unclassified.join("\n")
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

fn contains_numeric_px_literal(line: &str) -> bool {
    line.match_indices("px(").any(|(index, _)| {
        line[index + 3..]
            .trim_start()
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
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
