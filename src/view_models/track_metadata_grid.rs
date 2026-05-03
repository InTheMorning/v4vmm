//! Track metadata-grid view-model.
//!
//! Owns the shared presentation contract for the compare grid: visible
//! columns, heading labels, heading indentation, and expansion-key lookup.
//! Screens keep GPUI cell rendering because drag/drop and edit callbacks are
//! app-specific.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

use crate::metadata::summarize_contributor_value;

#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct TrackMetadataGridVm {
    headings: Vec<TrackMetadataGridHeading>,
    columns: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackMetadataGridHeading {
    pub label: String,
    pub indent: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackMetadataGridExpansion {
    pub rss_expanded: bool,
    pub id3_expanded: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueRoutesSummaryFallback {
    DisplayValue,
    MultilineCount,
}

impl TrackMetadataGridVm {
    #[must_use]
    pub fn tag_column_label(format_label: Option<&str>) -> &str {
        format_label
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .unwrap_or("Tags")
    }

    #[must_use]
    pub fn rss_cell_value(value: Option<&str>) -> &str {
        value.unwrap_or("")
    }

    #[must_use]
    pub fn id3_cell_value<'a>(
        pending_value: Option<&'a str>,
        row_value: Option<&'a str>,
    ) -> &'a str {
        pending_value.or(row_value).unwrap_or("")
    }

    #[must_use]
    pub fn id3_cell_frame<'a>(
        pending_frame: Option<&'a str>,
        row_frame: Option<&'a str>,
    ) -> Option<&'a str> {
        pending_frame.or(row_frame)
    }

    #[must_use]
    pub fn id3_drag_frame(row_frame: Option<&str>) -> String {
        row_frame.unwrap_or("").to_string()
    }

    #[must_use]
    pub fn id3_frame_label(frame_id: Option<&str>) -> &str {
        frame_id.unwrap_or("")
    }

    #[must_use]
    pub fn musicbrainz_cell_value(value: Option<&str>) -> &str {
        value.unwrap_or("")
    }

    #[must_use]
    pub fn contributor_summary(raw_value: &str, display_value: &str) -> String {
        summarize_contributor_value(raw_value).unwrap_or_else(|| display_value.to_string())
    }

    #[must_use]
    pub fn value_routes_summary(
        raw_value: &str,
        display_value: &str,
        fallback: ValueRoutesSummaryFallback,
    ) -> String {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) {
            format!("[{} items]", arr.len())
        } else {
            match fallback {
                ValueRoutesSummaryFallback::DisplayValue => display_value.to_string(),
                ValueRoutesSummaryFallback::MultilineCount => {
                    let lines = display_value.lines().count();
                    if lines > 1 {
                        format!("[{lines} lines]")
                    } else {
                        display_value.to_string()
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn group_heading_label(label: &str, unused_count: usize) -> String {
        if unused_count == 0 {
            label.to_string()
        } else {
            format!("{label} ({unused_count} unused)")
        }
    }

    #[must_use]
    pub fn value_route_item_label(recipient_label: &str, split_label: Option<&str>) -> String {
        split_label.map_or_else(
            || recipient_label.to_string(),
            |split| format!("{recipient_label} {split}"),
        )
    }

    #[must_use]
    pub fn value_route_split_label(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::Number(number) => {
                let raw = number.to_string();
                let trimmed = raw.strip_suffix(".0").unwrap_or(&raw);
                Some(format!("{trimmed}%"))
            }
            _ => json_value_display_label(value),
        }
    }

    #[must_use]
    pub fn value_route_field_key_label(key: &str) -> String {
        format!("{key}: ")
    }

    #[must_use]
    pub fn value_route_field_value_label(value: &serde_json::Value) -> Option<String> {
        json_value_display_label(value)
    }

    pub fn new(show_id3: bool, show_musicbrainz: bool, tag_column_label: &str) -> Self {
        let mut headings = vec![TrackMetadataGridHeading {
            label: "RSS".to_string(),
            indent: 96.0,
        }];
        if show_id3 {
            headings.push(TrackMetadataGridHeading {
                label: tag_column_label.to_string(),
                indent: 12.0,
            });
        }
        if show_musicbrainz {
            headings.push(TrackMetadataGridHeading {
                label: "MusicBrainz".to_string(),
                indent: 12.0,
            });
        }

        Self {
            columns: u16::try_from(headings.len()).unwrap_or(u16::MAX),
            headings,
        }
    }

    #[must_use]
    pub fn headings(&self) -> &[TrackMetadataGridHeading] {
        &self.headings
    }

    #[must_use]
    pub const fn columns(&self) -> u16 {
        self.columns
    }

    #[must_use]
    pub fn expansion_for(
        &self,
        row_id: &str,
        expanded_cells: &BTreeSet<String>,
    ) -> TrackMetadataGridExpansion {
        TrackMetadataGridExpansion {
            rss_expanded: expanded_cells.contains(&metadata_cell_key("rss", row_id)),
            id3_expanded: expanded_cells.contains(&metadata_cell_key("id3", row_id)),
        }
    }
}

fn metadata_cell_key(column: &str, row_id: &str) -> String {
    format!("{column}:{row_id}")
}

fn json_value_display_label(value: &serde_json::Value) -> Option<String> {
    let label = match value {
        serde_json::Value::Null => return None,
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        other => other.to_string(),
    };
    let label = label.trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_follow_visible_columns() {
        let vm = TrackMetadataGridVm::new(false, false, "Tags");
        assert_eq!(vm.columns(), 1);
        assert_eq!(
            vm.headings(),
            &[TrackMetadataGridHeading {
                label: "RSS".into(),
                indent: 96.0,
            }]
        );

        let vm = TrackMetadataGridVm::new(true, true, "ID3v2.4");
        assert_eq!(vm.columns(), 3);
        assert_eq!(
            vm.headings(),
            &[
                TrackMetadataGridHeading {
                    label: "RSS".into(),
                    indent: 96.0,
                },
                TrackMetadataGridHeading {
                    label: "ID3v2.4".into(),
                    indent: 12.0,
                },
                TrackMetadataGridHeading {
                    label: "MusicBrainz".into(),
                    indent: 12.0,
                },
            ]
        );
    }

    #[test]
    fn tag_column_label_defaults_to_tags_for_missing_or_blank_format() {
        assert_eq!(TrackMetadataGridVm::tag_column_label(None), "Tags");
        assert_eq!(TrackMetadataGridVm::tag_column_label(Some("  ")), "Tags");
        assert_eq!(TrackMetadataGridVm::tag_column_label(Some("MP3")), "MP3");
    }

    #[test]
    fn rss_cell_value_preserves_empty_vs_missing_display() {
        assert_eq!(TrackMetadataGridVm::rss_cell_value(Some("Title")), "Title");
        assert_eq!(TrackMetadataGridVm::rss_cell_value(Some("")), "");
        assert_eq!(TrackMetadataGridVm::rss_cell_value(None), "");
    }

    #[test]
    fn id3_cell_value_prefers_pending_then_preserves_empty_vs_missing_display() {
        assert_eq!(
            TrackMetadataGridVm::id3_cell_value(Some("Pending"), Some("Stored")),
            "Pending"
        );
        assert_eq!(
            TrackMetadataGridVm::id3_cell_value(Some(""), Some("Stored")),
            ""
        );
        assert_eq!(
            TrackMetadataGridVm::id3_cell_value(None, Some("Stored")),
            "Stored"
        );
        assert_eq!(TrackMetadataGridVm::id3_cell_value(None, Some("")), "");
        assert_eq!(TrackMetadataGridVm::id3_cell_value(None, None), "");
    }

    #[test]
    fn id3_cell_frame_prefers_pending_then_preserves_empty_vs_missing_display() {
        assert_eq!(
            TrackMetadataGridVm::id3_cell_frame(Some("TIT2"), Some("TT2")),
            Some("TIT2")
        );
        assert_eq!(
            TrackMetadataGridVm::id3_cell_frame(Some(""), Some("TT2")),
            Some("")
        );
        assert_eq!(
            TrackMetadataGridVm::id3_cell_frame(None, Some("TT2")),
            Some("TT2")
        );
        assert_eq!(
            TrackMetadataGridVm::id3_cell_frame(None, Some("")),
            Some("")
        );
        assert_eq!(TrackMetadataGridVm::id3_cell_frame(None, None), None);
    }

    #[test]
    fn id3_drag_frame_preserves_empty_vs_missing_display() {
        assert_eq!(TrackMetadataGridVm::id3_drag_frame(Some("TIT2")), "TIT2");
        assert_eq!(TrackMetadataGridVm::id3_drag_frame(Some("")), "");
        assert_eq!(TrackMetadataGridVm::id3_drag_frame(None), "");
    }

    #[test]
    fn id3_frame_label_preserves_empty_vs_missing_display() {
        assert_eq!(TrackMetadataGridVm::id3_frame_label(Some("TIT2")), "TIT2");
        assert_eq!(TrackMetadataGridVm::id3_frame_label(Some("")), "");
        assert_eq!(TrackMetadataGridVm::id3_frame_label(None), "");
    }

    #[test]
    fn musicbrainz_cell_value_preserves_empty_vs_missing_display() {
        assert_eq!(
            TrackMetadataGridVm::musicbrainz_cell_value(Some("Recording")),
            "Recording"
        );
        assert_eq!(TrackMetadataGridVm::musicbrainz_cell_value(Some("")), "");
        assert_eq!(TrackMetadataGridVm::musicbrainz_cell_value(None), "");
    }

    #[test]
    fn contributor_summary_falls_back_to_display_value_when_unsummarized() {
        assert_eq!(
            TrackMetadataGridVm::contributor_summary(
                "guitarist: Alice / musician: Alice / audio engineer: Bob",
                "Alice: guitarist, musician\nBob: audio engineer",
            ),
            "2 contributors"
        );
        assert_eq!(
            TrackMetadataGridVm::contributor_summary("", "No contributors"),
            "No contributors"
        );
        assert_eq!(TrackMetadataGridVm::contributor_summary("", ""), "");
    }

    #[test]
    fn value_routes_summary_counts_routes_and_owns_fallback_policy() {
        assert_eq!(
            TrackMetadataGridVm::value_routes_summary(
                r#"[{"recipient_name":"Alice"},{"recipient_name":"Bob"}]"#,
                "Alice\nBob",
                ValueRoutesSummaryFallback::DisplayValue,
            ),
            "[2 items]"
        );
        assert_eq!(
            TrackMetadataGridVm::value_routes_summary(
                "not json",
                "Alice\nBob",
                ValueRoutesSummaryFallback::DisplayValue,
            ),
            "Alice\nBob"
        );
        assert_eq!(
            TrackMetadataGridVm::value_routes_summary(
                "not json",
                "Alice\nBob",
                ValueRoutesSummaryFallback::MultilineCount,
            ),
            "[2 lines]"
        );
        assert_eq!(
            TrackMetadataGridVm::value_routes_summary(
                "not json",
                "Alice",
                ValueRoutesSummaryFallback::MultilineCount,
            ),
            "Alice"
        );
    }

    #[test]
    fn group_heading_label_appends_unused_count_only_when_present() {
        assert_eq!(
            TrackMetadataGridVm::group_heading_label("People", 0),
            "People"
        );
        assert_eq!(
            TrackMetadataGridVm::group_heading_label("People", 3),
            "People (3 unused)"
        );
    }

    #[test]
    fn value_route_item_label_appends_split_when_present() {
        assert_eq!(
            TrackMetadataGridVm::value_route_item_label("Alice", None),
            "Alice"
        );
        assert_eq!(
            TrackMetadataGridVm::value_route_item_label("Alice", Some("25%")),
            "Alice 25%"
        );
    }

    #[test]
    fn value_route_split_label_formats_percent_and_ignores_empty_values() {
        assert_eq!(
            TrackMetadataGridVm::value_route_split_label(&serde_json::json!(25.0)),
            Some("25%".into())
        );
        assert_eq!(
            TrackMetadataGridVm::value_route_split_label(&serde_json::json!("  custom split  ")),
            Some("custom split".into())
        );
        assert_eq!(
            TrackMetadataGridVm::value_route_split_label(&serde_json::json!("   ")),
            None
        );
        assert_eq!(
            TrackMetadataGridVm::value_route_split_label(&serde_json::Value::Null),
            None
        );
    }

    #[test]
    fn value_route_field_key_label_adds_separator() {
        assert_eq!(
            TrackMetadataGridVm::value_route_field_key_label("custom_key"),
            "custom_key: "
        );
    }

    #[test]
    fn value_route_field_value_label_trims_and_suppresses_empty_values() {
        assert_eq!(
            TrackMetadataGridVm::value_route_field_value_label(&serde_json::json!("  Alice  ")),
            Some("Alice".into())
        );
        assert_eq!(
            TrackMetadataGridVm::value_route_field_value_label(&serde_json::json!(true)),
            Some("true".into())
        );
        assert_eq!(
            TrackMetadataGridVm::value_route_field_value_label(&serde_json::json!("   ")),
            None
        );
        assert_eq!(
            TrackMetadataGridVm::value_route_field_value_label(&serde_json::Value::Null),
            None
        );
    }

    #[test]
    fn expansion_uses_shared_metadata_cell_keys() {
        let mut expanded = BTreeSet::new();
        expanded.insert("rss:title".to_string());
        expanded.insert("id3:artist".to_string());
        let vm = TrackMetadataGridVm::new(true, false, "Tags");

        assert_eq!(
            vm.expansion_for("title", &expanded),
            TrackMetadataGridExpansion {
                rss_expanded: true,
                id3_expanded: false,
            }
        );
        assert_eq!(
            vm.expansion_for("artist", &expanded),
            TrackMetadataGridExpansion {
                rss_expanded: false,
                id3_expanded: true,
            }
        );
    }
}
