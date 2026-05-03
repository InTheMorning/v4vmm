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

const COMPOSITE_LOOSE_STRING_SIGNATURE_ALLOWLIST: &[CompositeLooseStringSignatureAllowance] = &[
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/action_button.rs",
        pattern: "pub fn action_button(label: &str",
        note: "thin compatibility helper; caller supplies already-approved action label",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/action_row.rs",
        pattern: "pub fn neutral(text: impl Into<SharedString>)",
        note: "status message object; view-model or command outcome owns message text",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/action_row.rs",
        pattern: "pub fn danger(text: impl Into<SharedString>)",
        note: "status message object; view-model or command outcome owns message text",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/detail_grid.rs",
        pattern: "pub fn new(key: impl Into<SharedString>, value: impl IntoElement)",
        note: "generic key/value primitive-composite row; caller supplies display rows",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/detail_grid.rs",
        pattern: "pub fn text(key: impl Into<SharedString>, value: impl Into<String>",
        note: "generic key/value text row; caller supplies display rows",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/detail_header.rs",
        pattern: "pub fn new(kind: EntityKind, title: impl Into<SharedString>)",
        note: "generic header shell; callers pass VM-owned titles",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/detail_header.rs",
        pattern: "pub fn subtitle(mut self, subtitle: impl Into<SharedString>)",
        note: "generic header shell; callers pass VM-owned subtitles",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/disclosure_group.rs",
        pattern: "pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>)",
        note: "generic disclosure shell label, not a fallback policy owner",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/playlist_popover.rs",
        pattern: "pub fn new(id: i64, name: impl Into<SharedString>)",
        note: "PlaylistOption is the display contract for playlist names",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/playlist_popover.rs",
        pattern: "pub fn new(id: impl Into<SharedString>, playlists: Vec<PlaylistOption>)",
        note: "element id plus PlaylistOption display contract",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/playlist_popover.rs",
        pattern: "pub fn trigger_label(mut self, label: impl Into<SharedString>)",
        note: "temporary action-label override; create/select chrome still owned here",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/playlist_popover.rs",
        pattern:
            "pub fn on_create(mut self, handler: impl Fn(&String, &mut Window, &mut App) + 'static)",
        note: "callback payload for new playlist name, not display label input",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/segmented_control.rs",
        pattern: "pub fn new(id: impl Into<ElementId>, key: K, label: impl Into<SharedString>)",
        note: "generic segment label; caller owns option labels",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/release_detail_surface.rs",
        pattern: "pub fn new(id: impl Into<SharedString>)",
        note: "element id, not display copy",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/split_pane.rs",
        pattern: "pub fn new(id: impl Into<SharedString>)",
        note: "element id, not display copy",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/split_pane.rs",
        pattern: "pub fn resize_handle_id(mut self, id: impl Into<SharedString>)",
        note: "element id, not display copy",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/tag_badge.rs",
        pattern: "pub fn from_legacy_str(s: &str)",
        note: "legacy role parser, not display copy",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/tag_badge.rs",
        pattern: "pub fn label(self) -> &'static str",
        note: "role enum owns its static label",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/tag_badge.rs",
        pattern: "pub fn accessibility_label(self) -> &'static str",
        note: "role enum owns its static accessibility label",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/tag_badge.rs",
        pattern: "pub fn label(mut self, label: impl Into<SharedString>)",
        note: "generic badge override; fallback policy must still live in VM",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/track_metadata_grid.rs",
        pattern: "pub fn new(label: impl Into<SharedString>, columns: u16)",
        note: "advanced provenance group label; metadata contract owns source-specific label",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/track_metadata_grid.rs",
        pattern: "pub fn new(label: impl Into<SharedString>, value: impl IntoElement)",
        note: "advanced provenance field label; metadata contract owns source-specific label",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/track_metadata_grid.rs",
        pattern: "pub fn frame_label(mut self, label: impl Into<SharedString>)",
        note: "advanced provenance tag-frame label; metadata contract owns source-specific label",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/track_metadata_grid.rs",
        pattern: "pub fn new(value: impl Into<SharedString>)",
        note: "advanced provenance text value; caller supplies normalized metadata display",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/track_row.rs",
        pattern: "pub fn number(mut self, n: impl Into<String>)",
        note: "TrackRow caller passes TrackVm-owned number label",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/track_row.rs",
        pattern: "pub fn title(mut self, t: impl Into<String>)",
        note: "TrackRow caller passes TrackVm-owned title",
    },
    CompositeLooseStringSignatureAllowance {
        file: "src/ui/composites/track_row.rs",
        pattern: "pub fn duration(mut self, d: Option<String>)",
        note: "TrackRow caller passes TrackVm-owned duration display",
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
struct CompositeLooseStringSignatureAllowance {
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

    let mut violations = Vec::new();
    for file in KNOWN_SHARED_UI_SHELL_FILES {
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
    let source = read_source(&manifest_path("src/search.rs"));
    let start = source
        .find("fn render_recent_feeds_tiles(")
        .expect("Discover recent-feed renderer should exist");
    let end = source[start..]
        .find("\nfn render_inspector_empty(")
        .map_or(source.len(), |offset| start + offset);
    let body = &source[start..end];
    let mut violations = Vec::new();

    if !body.contains("RecentFeedTile::new(") {
        violations.push(
            "src/search.rs: render_recent_feeds_tiles must compose `RecentFeedTile`".to_string(),
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
                "src/search.rs: render_recent_feeds_tiles must not own recent tile chrome or placeholder labels; found `{pattern}`"
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
        ("src/ui_entity.rs", "pub actions: Vec<AnyElement>"),
        ("src/ui_entity.rs", "pub popover: Option<AnyElement>"),
        ("src/ui_entity.rs", "pub primary_actions: Vec<AnyElement>"),
        ("src/ui_entity.rs", "pub identity_actions: Vec<AnyElement>"),
        ("src/ui_entity.rs", "pub action_overlays: Vec<AnyElement>"),
        (
            "src/ui_entity.rs",
            "pub track_rows: Option<Vec<AnyElement>>",
        ),
        ("src/ui_entity.rs", "pub after_section: Vec<AnyElement>"),
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
        "src/ui_entity.rs",
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

    for file in ["src/ui_feed.rs", "src/library.rs"] {
        let source = read_source(&manifest_path(file));
        if source.contains("IdentityActionKind::Rss") {
            violations.push(format!(
                "{file}: ADR 0037 feed RSS identity actions must be rendered by `ui_entity::render_feed_identity_actions`"
            ));
        }
    }

    let ui_entity = read_source(&manifest_path("src/ui_entity.rs"));
    if !ui_entity.contains("fn render_feed_identity_actions") {
        violations.push(
            "src/ui_entity.rs: ADR 0037 must define `fn render_feed_identity_actions`".to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "ADR 0037 feed identity renderer violations:\n{}",
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
            "src/ui_track.rs",
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
fn release_surface_consumers_use_release_detail_vm() {
    let consumers = [
        (
            "src/library.rs",
            "render_release_detail_shell(",
            "ReleaseDetailVm::new(",
        ),
        (
            "src/ui_feed.rs",
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
fn composite_loose_string_display_apis_are_allowlisted() {
    let mut violations = Vec::new();

    for path in rust_files_under("src/ui/composites") {
        let file = rel_path(&path);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if !line.contains("pub fn") {
                continue;
            }
            let mentions_string_api = line.contains("&str")
                || line.contains("String")
                || line.contains("SharedString")
                || line.contains("Into<String>")
                || line.contains("Into<SharedString>");
            if !mentions_string_api {
                continue;
            }
            let allowed_note = COMPOSITE_LOOSE_STRING_SIGNATURE_ALLOWLIST
                .iter()
                .find(|allowance| allowance.file == file && line.contains(allowance.pattern))
                .map(|allowance| allowance.note);
            if allowed_note.is_none() {
                violations.push(format!(
                    "{file}:{line_number}: shared composite string-like public API must be display-contract owned or explicitly allowlisted: `{line}`"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR 0033 composite display-contract signature violations:\n{}",
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

fn compact_source(source: &str) -> String {
    source.chars().filter(|ch| !ch.is_whitespace()).collect()
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
