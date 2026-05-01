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

const SCREEN_PLAYLIST_SERVICE_FORBIDDEN_PATTERNS: &[&str] =
    &["use crate::playlist_service", "playlist_service::"];

const SCREEN_SUBSCRIPTION_FORBIDDEN_PATTERNS: &[&str] = &[
    "db::set_feed_subscribed(",
    "db::unsubscribe_feed_tracks(",
    "library_service::set_track_in_library(",
    "library_service::set_track_in_library_by_match(",
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
];

const SCREEN_FILES: &[&str] = &["src/app.rs", "src/library.rs", "src/search.rs"];

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
fn screens_do_not_add_unapproved_hardcoded_dark_defaults() {
    let mut violations = Vec::new();
    for file in SCREEN_FILES {
        let path = manifest_path(file);
        let source = read_source(&path);
        for (line_number, line) in code_lines(&source) {
            if line.contains("Appearance::Dark")
                && !appearance_dark_is_approved(file, &source, line_number)
            {
                violations.push(format!(
                    "{file}:{line_number}: hardcoded `Appearance::Dark` needs an explicit architecture-test approval: `{line}`"
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

fn rust_files_under(relative_dir: &str) -> Vec<PathBuf> {
    let root = manifest_path(relative_dir);
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);
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
        "src/app.rs" => nearby_source_mentions(
            source,
            line_number,
            &[
                "Apply scale change immediately",
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
