//! Track metadata-grid view-model.
//!
//! Owns the shared presentation contract for the compare grid: visible
//! columns, heading labels, heading indentation, and expansion-key lookup.
//! Screens keep GPUI cell rendering because drag/drop and edit callbacks are
//! app-specific.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

use crate::metadata::{metadata_field_is_expandable, summarize_contributor_value, MetadataColumn};
use crate::track_compare::ComparisonStatus;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackMetadataGroupHeadingDisplay {
    pub label: String,
    pub disclosure_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackMetadataGridExpansion {
    pub rss_expanded: bool,
    pub id3_expanded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackMetadataExpandableCellDisplay {
    pub cell_key: String,
    pub cell_id: String,
    pub header_id: String,
    pub disclosure_glyph: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackMetadataValueRouteItemDisplay {
    pub item_key: String,
    pub item_id: String,
    pub header_id: Option<String>,
    pub disclosure_glyph: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackMetadataSourceDragDisplay {
    pub cell_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueRoutesSummaryFallback {
    DisplayValue,
    MultilineCount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueRouteFieldContext {
    Library,
    Discover,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackMetadataComparisonRole {
    Match,
    Different,
    Missing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackMetadataExpandedFieldKind {
    Artwork,
    Transcript,
    ValueRoutes,
    Text,
}

impl TrackMetadataComparisonRole {
    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Match => "=",
            Self::Different => "\u{2260}",
            Self::Missing => "\u{2205}",
        }
    }
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
    pub fn id3_frame_display_label(frame_id: Option<&str>) -> String {
        Self::id3_frame_label(frame_id).to_string()
    }

    #[must_use]
    pub fn field_label(field: &str) -> String {
        field.to_string()
    }

    #[must_use]
    pub fn compare_row_id(field: &str) -> String {
        let mut out = String::new();
        for ch in field.chars().flat_map(char::to_lowercase) {
            if ch.is_ascii_alphanumeric() {
                out.push(ch);
            } else if !out.ends_with('-') {
                out.push('-');
            }
        }
        out.trim_matches('-').to_string()
    }

    #[must_use]
    pub fn unused_id3_frame_row_id(frame_id: &str) -> String {
        format!("id3-unused-{}", Self::compare_row_id(frame_id))
    }

    #[must_use]
    pub fn used_id3_field_row_id(frame_id: &str) -> String {
        format!("id3-field-{}", Self::compare_row_id(frame_id))
    }

    #[must_use]
    pub fn id3_field_display_label(frame_id: &str) -> String {
        format!("ID3 {frame_id}")
    }

    #[must_use]
    pub fn musicbrainz_cell_value(value: Option<&str>) -> &str {
        value.unwrap_or("")
    }

    #[must_use]
    pub const fn comparison_role(status: &ComparisonStatus) -> Option<TrackMetadataComparisonRole> {
        match status {
            ComparisonStatus::Match => Some(TrackMetadataComparisonRole::Match),
            ComparisonStatus::Different => Some(TrackMetadataComparisonRole::Different),
            ComparisonStatus::MissingSource | ComparisonStatus::MissingTag => {
                Some(TrackMetadataComparisonRole::Missing)
            }
            ComparisonStatus::MissingBoth => None,
        }
    }

    #[must_use]
    pub const fn comparison_glyph(status: &ComparisonStatus) -> Option<&'static str> {
        match Self::comparison_role(status) {
            Some(role) => Some(role.glyph()),
            None => None,
        }
    }

    #[must_use]
    pub fn pending_source_role(
        pending_source: MetadataColumn,
        pending_value: &str,
        column: MetadataColumn,
        cell_value: Option<&str>,
    ) -> Option<TrackMetadataComparisonRole> {
        if pending_source != column {
            return None;
        }
        let cell_value = cell_value
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        if cell_value == pending_value.trim() {
            Some(TrackMetadataComparisonRole::Match)
        } else {
            Some(TrackMetadataComparisonRole::Different)
        }
    }

    #[must_use]
    pub const fn id3_status_role(
        id3_value: Option<&str>,
        rss_value: Option<&str>,
        musicbrainz_value: Option<&str>,
        status: &ComparisonStatus,
    ) -> Option<TrackMetadataComparisonRole> {
        if Self::id3_status_uses_primary_fallback(id3_value, rss_value, musicbrainz_value) {
            None
        } else {
            Self::comparison_role(status)
        }
    }

    #[must_use]
    pub const fn id3_status_uses_primary_fallback(
        id3_value: Option<&str>,
        rss_value: Option<&str>,
        musicbrainz_value: Option<&str>,
    ) -> bool {
        id3_value.is_some() && rss_value.is_none() && musicbrainz_value.is_none()
    }

    #[must_use]
    pub fn display_with_glyph(glyph: Option<&str>, value: &str) -> String {
        match glyph {
            Some(glyph) if !value.is_empty() => format!("{glyph} {value}"),
            Some(glyph) => glyph.to_string(),
            None => value.to_string(),
        }
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
    pub fn artwork_url(raw_value: &str) -> Option<&str> {
        raw_value
            .starts_with("http://")
            .then_some(raw_value)
            .or_else(|| raw_value.starts_with("https://").then_some(raw_value))
    }

    #[must_use]
    pub fn artwork_summary(raw_value: &str, display_value: &str) -> String {
        Self::artwork_url(raw_value).map_or_else(
            || display_value.to_string(),
            |url| url.rsplit('/').next().unwrap_or(url).to_string(),
        )
    }

    #[must_use]
    pub fn expandable_cell_summary(
        field: &str,
        raw_value: &str,
        display_value: &str,
        value_routes_fallback: ValueRoutesSummaryFallback,
    ) -> String {
        match field {
            "Contributors" => Self::contributor_summary(raw_value, display_value),
            "Value Routes" => {
                Self::value_routes_summary(raw_value, display_value, value_routes_fallback)
            }
            "Artwork" => Self::artwork_summary(raw_value, display_value),
            _ => display_value.to_string(),
        }
    }

    #[must_use]
    pub fn expanded_field_kind(field: &str) -> TrackMetadataExpandedFieldKind {
        match field {
            "Artwork" => TrackMetadataExpandedFieldKind::Artwork,
            "Transcript" | "Transcript text" => TrackMetadataExpandedFieldKind::Transcript,
            "Value Routes" => TrackMetadataExpandedFieldKind::ValueRoutes,
            _ => TrackMetadataExpandedFieldKind::Text,
        }
    }

    #[must_use]
    pub fn field_is_expandable(field: &str, raw_value: &str) -> bool {
        metadata_field_is_expandable(field) && !raw_value.is_empty()
    }

    #[must_use]
    pub fn logical_field(field: &str) -> &str {
        match field {
            "TXXX:MusicIndex Contributors" => "Contributors",
            "TXXX:MusicIndex Value Routes" => "Value Routes",
            _ => field,
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
    pub fn group_heading_display(
        label: &str,
        unused_count: usize,
        group_key: Option<&str>,
    ) -> TrackMetadataGroupHeadingDisplay {
        TrackMetadataGroupHeadingDisplay {
            label: Self::group_heading_label(label, unused_count),
            disclosure_id: group_key.map(|key| format!("section:id3-frame-group:{key}")),
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

    #[must_use]
    pub fn value_route_child_field_is_visible(key: &str, context: ValueRouteFieldContext) -> bool {
        match (context, key) {
            (_, "recipient_name") | (ValueRouteFieldContext::Library, "split") => false,
            (_, _) => true,
        }
    }

    #[must_use]
    pub fn json_tree_scalar_label(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => None,
            serde_json::Value::String(value) => Some(value.clone()),
            serde_json::Value::Null => Some("null".to_string()),
            other => Some(other.to_string()),
        }
    }

    #[must_use]
    pub fn source_drag_display(
        column: MetadataColumn,
        row_id: &str,
    ) -> TrackMetadataSourceDragDisplay {
        let column_slug = match column {
            MetadataColumn::Rss => "rss",
            MetadataColumn::MusicBrainz => "musicbrainz",
        };
        TrackMetadataSourceDragDisplay {
            cell_id: format!("metadata-{column_slug}-drag-{row_id}"),
        }
    }

    #[must_use]
    pub fn transcript_line_display(line: &str) -> &str {
        if line.is_empty() {
            " "
        } else {
            line
        }
    }

    #[must_use]
    pub fn value_route_item_key(column: &str, row_id: &str, index: usize) -> String {
        metadata_cell_key(column, &format!("{row_id}:{index}"))
    }

    #[must_use]
    pub fn library_expandable_cell_display(
        column: &str,
        row_id: &str,
        expanded: bool,
    ) -> TrackMetadataExpandableCellDisplay {
        let cell_key = metadata_cell_key(column, row_id);
        TrackMetadataExpandableCellDisplay {
            cell_id: format!("metadata-cell:{cell_key}"),
            header_id: format!("metadata-cell:{cell_key}:header"),
            cell_key,
            disclosure_glyph: disclosure_glyph(expanded),
        }
    }

    #[must_use]
    pub fn discover_expandable_cell_display(
        column: &str,
        field: &str,
        row_id: &str,
        expanded: bool,
    ) -> TrackMetadataExpandableCellDisplay {
        TrackMetadataExpandableCellDisplay {
            cell_key: metadata_cell_key(column, row_id),
            cell_id: format!("expandable-{column}-{field}"),
            header_id: format!("expandable-{column}-{field}-hdr"),
            disclosure_glyph: disclosure_glyph(expanded),
        }
    }

    #[must_use]
    pub fn library_value_route_item_display(
        column: &str,
        row_id: &str,
        index: usize,
        expanded: bool,
    ) -> TrackMetadataValueRouteItemDisplay {
        let item_key = Self::value_route_item_key(column, row_id, index);
        TrackMetadataValueRouteItemDisplay {
            item_id: format!("value-route:{column}:{row_id}:{index}"),
            header_id: Some(format!("value-route:{column}:{row_id}:{index}:header")),
            item_key,
            disclosure_glyph: disclosure_glyph(expanded),
        }
    }

    #[must_use]
    pub fn discover_value_route_item_display(
        column: &str,
        row_id: &str,
        index: usize,
        expanded: bool,
    ) -> TrackMetadataValueRouteItemDisplay {
        TrackMetadataValueRouteItemDisplay {
            item_key: Self::value_route_item_key(column, row_id, index),
            item_id: format!("vr-{column}-{index}"),
            header_id: None,
            disclosure_glyph: disclosure_glyph(expanded),
        }
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

const fn disclosure_glyph(expanded: bool) -> &'static str {
    if expanded {
        "v"
    } else {
        ">"
    }
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
    fn id3_frame_display_label_projects_owned_display_string() {
        assert_eq!(
            TrackMetadataGridVm::id3_frame_display_label(Some("TIT2")),
            "TIT2"
        );
        assert_eq!(TrackMetadataGridVm::id3_frame_display_label(Some("")), "");
        assert_eq!(TrackMetadataGridVm::id3_frame_display_label(None), "");
    }

    #[test]
    fn field_label_preserves_raw_metadata_field_display() {
        assert_eq!(TrackMetadataGridVm::field_label("Title"), "Title");
        assert_eq!(TrackMetadataGridVm::field_label(""), "");
    }

    #[test]
    fn id3_generated_row_display_projects_ids_and_labels() {
        assert_eq!(
            TrackMetadataGridVm::compare_row_id("RSS feed guid"),
            "rss-feed-guid"
        );
        assert_eq!(
            TrackMetadataGridVm::compare_row_id("TRCK (Total tracks, Track #)"),
            "trck-total-tracks-track"
        );
        assert_eq!(
            TrackMetadataGridVm::unused_id3_frame_row_id("TXXX:MusicIndex Feed GUID"),
            "id3-unused-txxx-musicindex-feed-guid"
        );
        assert_eq!(
            TrackMetadataGridVm::used_id3_field_row_id("TIT2"),
            "id3-field-tit2"
        );
        assert_eq!(
            TrackMetadataGridVm::id3_field_display_label("TIT2"),
            "ID3 TIT2"
        );
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
    fn comparison_role_maps_compare_statuses() {
        assert_eq!(
            TrackMetadataGridVm::comparison_role(&ComparisonStatus::Match),
            Some(TrackMetadataComparisonRole::Match)
        );
        assert_eq!(
            TrackMetadataGridVm::comparison_role(&ComparisonStatus::Different),
            Some(TrackMetadataComparisonRole::Different)
        );
        assert_eq!(
            TrackMetadataGridVm::comparison_role(&ComparisonStatus::MissingSource),
            Some(TrackMetadataComparisonRole::Missing)
        );
        assert_eq!(
            TrackMetadataGridVm::comparison_role(&ComparisonStatus::MissingTag),
            Some(TrackMetadataComparisonRole::Missing)
        );
        assert_eq!(
            TrackMetadataGridVm::comparison_role(&ComparisonStatus::MissingBoth),
            None
        );
    }

    #[test]
    fn display_with_glyph_preserves_empty_values() {
        assert_eq!(
            TrackMetadataGridVm::display_with_glyph(Some("="), "Title"),
            "= Title"
        );
        assert_eq!(TrackMetadataGridVm::display_with_glyph(Some("="), ""), "=");
        assert_eq!(
            TrackMetadataGridVm::display_with_glyph(None, "Title"),
            "Title"
        );
        assert_eq!(TrackMetadataGridVm::display_with_glyph(None, ""), "");
    }

    #[test]
    fn pending_source_role_compares_trimmed_values() {
        assert_eq!(
            TrackMetadataGridVm::pending_source_role(
                MetadataColumn::Rss,
                " Title ",
                MetadataColumn::Rss,
                Some("Title"),
            ),
            Some(TrackMetadataComparisonRole::Match)
        );
        assert_eq!(
            TrackMetadataGridVm::pending_source_role(
                MetadataColumn::Rss,
                "New Title",
                MetadataColumn::Rss,
                Some("Old Title"),
            ),
            Some(TrackMetadataComparisonRole::Different)
        );
        assert_eq!(
            TrackMetadataGridVm::pending_source_role(
                MetadataColumn::Rss,
                "Title",
                MetadataColumn::MusicBrainz,
                Some("Title"),
            ),
            None
        );
        assert_eq!(
            TrackMetadataGridVm::pending_source_role(
                MetadataColumn::Rss,
                "Title",
                MetadataColumn::Rss,
                Some("  "),
            ),
            None
        );
    }

    #[test]
    fn id3_status_role_suppresses_standalone_id3_values() {
        assert_eq!(
            TrackMetadataGridVm::id3_status_role(
                Some("Embedded"),
                None,
                None,
                &ComparisonStatus::MissingSource,
            ),
            None
        );
        assert_eq!(
            TrackMetadataGridVm::id3_status_role(
                Some("Embedded"),
                Some("RSS"),
                None,
                &ComparisonStatus::Different,
            ),
            Some(TrackMetadataComparisonRole::Different)
        );
        assert_eq!(
            TrackMetadataGridVm::id3_status_role(
                None,
                Some("RSS"),
                None,
                &ComparisonStatus::MissingTag,
            ),
            Some(TrackMetadataComparisonRole::Missing)
        );
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
    fn expandable_cell_summary_owns_context_fallbacks() {
        assert_eq!(
            TrackMetadataGridVm::expandable_cell_summary(
                "Contributors",
                "role: Alice",
                "Alice",
                ValueRoutesSummaryFallback::DisplayValue,
            ),
            "1 contributor"
        );
        assert_eq!(
            TrackMetadataGridVm::expandable_cell_summary(
                "Value Routes",
                r#"[{"recipient_name":"Alice"}]"#,
                "Alice",
                ValueRoutesSummaryFallback::DisplayValue,
            ),
            "[1 items]"
        );
        assert_eq!(
            TrackMetadataGridVm::expandable_cell_summary(
                "Value Routes",
                "not json",
                "Alice\nBob",
                ValueRoutesSummaryFallback::MultilineCount,
            ),
            "[2 lines]"
        );
        assert_eq!(
            TrackMetadataGridVm::expandable_cell_summary(
                "Artwork",
                "https://cdn.example/art/front.jpg",
                "Full artwork URL",
                ValueRoutesSummaryFallback::DisplayValue,
            ),
            "front.jpg"
        );
        assert_eq!(
            TrackMetadataGridVm::expandable_cell_summary(
                "Transcript",
                "raw transcript",
                "display transcript",
                ValueRoutesSummaryFallback::DisplayValue,
            ),
            "display transcript"
        );
    }

    #[test]
    fn expanded_field_kind_classifies_metadata_fields() {
        assert_eq!(
            TrackMetadataGridVm::expanded_field_kind("Artwork"),
            TrackMetadataExpandedFieldKind::Artwork
        );
        assert_eq!(
            TrackMetadataGridVm::expanded_field_kind("Transcript"),
            TrackMetadataExpandedFieldKind::Transcript
        );
        assert_eq!(
            TrackMetadataGridVm::expanded_field_kind("Transcript text"),
            TrackMetadataExpandedFieldKind::Transcript
        );
        assert_eq!(
            TrackMetadataGridVm::expanded_field_kind("Value Routes"),
            TrackMetadataExpandedFieldKind::ValueRoutes
        );
        assert_eq!(
            TrackMetadataGridVm::expanded_field_kind("Title"),
            TrackMetadataExpandedFieldKind::Text
        );
    }

    #[test]
    fn field_is_expandable_preserves_metadata_gate_and_empty_values() {
        assert!(TrackMetadataGridVm::field_is_expandable(
            "Contributors",
            "Alice"
        ));
        assert!(!TrackMetadataGridVm::field_is_expandable(
            "Contributors",
            ""
        ));
        assert!(!TrackMetadataGridVm::field_is_expandable("Title", "Song"));
    }

    #[test]
    fn logical_field_maps_raw_musicindex_txxx_fields() {
        assert_eq!(
            TrackMetadataGridVm::logical_field("TXXX:MusicIndex Contributors"),
            "Contributors"
        );
        assert_eq!(
            TrackMetadataGridVm::logical_field("TXXX:MusicIndex Value Routes"),
            "Value Routes"
        );
        assert_eq!(TrackMetadataGridVm::logical_field("Title"), "Title");
    }

    #[test]
    fn artwork_url_and_summary_preserve_legacy_http_policy() {
        assert_eq!(
            TrackMetadataGridVm::artwork_url("https://cdn.example/a/b.png"),
            Some("https://cdn.example/a/b.png")
        );
        assert_eq!(
            TrackMetadataGridVm::artwork_summary("http://cdn.example/a/b.png", "Artwork"),
            "b.png"
        );
        assert_eq!(
            TrackMetadataGridVm::artwork_summary("embedded image", "Artwork"),
            "Artwork"
        );
        assert_eq!(
            TrackMetadataGridVm::artwork_url("ftp://example/a.png"),
            None
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
    fn group_heading_display_projects_label_and_disclosure_id() {
        assert_eq!(
            TrackMetadataGridVm::group_heading_display("People", 3, Some("people-credits")),
            TrackMetadataGroupHeadingDisplay {
                label: "People (3 unused)".to_string(),
                disclosure_id: Some("section:id3-frame-group:people-credits".to_string()),
            }
        );
        assert_eq!(
            TrackMetadataGridVm::group_heading_display("People", 0, None),
            TrackMetadataGroupHeadingDisplay {
                label: "People".to_string(),
                disclosure_id: None,
            }
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
    fn value_route_child_field_visibility_preserves_screen_contexts() {
        assert!(!TrackMetadataGridVm::value_route_child_field_is_visible(
            "recipient_name",
            ValueRouteFieldContext::Library,
        ));
        assert!(!TrackMetadataGridVm::value_route_child_field_is_visible(
            "split",
            ValueRouteFieldContext::Library,
        ));
        assert!(TrackMetadataGridVm::value_route_child_field_is_visible(
            "split",
            ValueRouteFieldContext::Discover,
        ));
        assert!(TrackMetadataGridVm::value_route_child_field_is_visible(
            "address",
            ValueRouteFieldContext::Discover,
        ));
    }

    #[test]
    fn json_tree_scalar_label_preserves_raw_json_leaf_display() {
        assert_eq!(
            TrackMetadataGridVm::json_tree_scalar_label(&serde_json::json!(" Alice ")),
            Some(" Alice ".into())
        );
        assert_eq!(
            TrackMetadataGridVm::json_tree_scalar_label(&serde_json::Value::Null),
            Some("null".into())
        );
        assert_eq!(
            TrackMetadataGridVm::json_tree_scalar_label(&serde_json::json!(42)),
            Some("42".into())
        );
        assert_eq!(
            TrackMetadataGridVm::json_tree_scalar_label(&serde_json::json!({"a": 1})),
            None
        );
        assert_eq!(
            TrackMetadataGridVm::json_tree_scalar_label(&serde_json::json!([1, 2])),
            None
        );
    }

    #[test]
    fn source_drag_display_projects_discover_source_cell_ids() {
        assert_eq!(
            TrackMetadataGridVm::source_drag_display(MetadataColumn::Rss, "title"),
            TrackMetadataSourceDragDisplay {
                cell_id: "metadata-rss-drag-title".into(),
            }
        );
        assert_eq!(
            TrackMetadataGridVm::source_drag_display(MetadataColumn::MusicBrainz, "release"),
            TrackMetadataSourceDragDisplay {
                cell_id: "metadata-musicbrainz-drag-release".into(),
            }
        );
    }

    #[test]
    fn transcript_line_display_preserves_blank_visual_rows() {
        assert_eq!(TrackMetadataGridVm::transcript_line_display("Line"), "Line");
        assert_eq!(TrackMetadataGridVm::transcript_line_display(""), " ");
    }

    #[test]
    fn expandable_cell_display_projects_library_and_discover_chrome() {
        assert_eq!(
            TrackMetadataGridVm::library_expandable_cell_display("rss", "title", false),
            TrackMetadataExpandableCellDisplay {
                cell_key: "rss:title".into(),
                cell_id: "metadata-cell:rss:title".into(),
                header_id: "metadata-cell:rss:title:header".into(),
                disclosure_glyph: ">",
            }
        );
        assert_eq!(
            TrackMetadataGridVm::discover_expandable_cell_display(
                "id3",
                "Value Routes",
                "value-routes",
                true
            ),
            TrackMetadataExpandableCellDisplay {
                cell_key: "id3:value-routes".into(),
                cell_id: "expandable-id3-Value Routes".into(),
                header_id: "expandable-id3-Value Routes-hdr".into(),
                disclosure_glyph: "v",
            }
        );
    }

    #[test]
    fn value_route_item_display_projects_screen_specific_chrome() {
        assert_eq!(
            TrackMetadataGridVm::library_value_route_item_display("rss", "value-routes", 2, true),
            TrackMetadataValueRouteItemDisplay {
                item_key: "rss:value-routes:2".into(),
                item_id: "value-route:rss:value-routes:2".into(),
                header_id: Some("value-route:rss:value-routes:2:header".into()),
                disclosure_glyph: "v",
            }
        );
        assert_eq!(
            TrackMetadataGridVm::discover_value_route_item_display("id3", "value-routes", 3, false),
            TrackMetadataValueRouteItemDisplay {
                item_key: "id3:value-routes:3".into(),
                item_id: "vr-id3-3".into(),
                header_id: None,
                disclosure_glyph: ">",
            }
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
