//! Metadata comparison projections.
//!
//! These view-models format already-loaded metadata comparison data into plain
//! display values for shared UI composites.

#![warn(clippy::pedantic)]

use crate::metadata::TagCompareResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileHeaderVm {
    pub badge_label: String,
    pub title: String,
    pub path: String,
}

impl FileHeaderVm {
    #[must_use]
    pub fn new(result: &TagCompareResult) -> Self {
        let badge_label = embedded_tag_label(result);
        let title = result
            .rows
            .iter()
            .find(|row| row.field == "Title")
            .and_then(|row| row.tag_value.clone())
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| badge_label.clone());
        Self {
            badge_label,
            title,
            path: result.path.clone(),
        }
    }
}

#[must_use]
pub fn value_route_recipient_label(route: &serde_json::Value) -> String {
    route
        .get("recipient_name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| "Unknown".to_string(), str::to_string)
}

fn embedded_tag_label(result: &TagCompareResult) -> String {
    result.format.map_or_else(
        || "Embedded tags".into(),
        |format| format!("Embedded {}", format.display_label()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_format::AudioFormat;
    use crate::track_compare::{ComparisonRow, ComparisonStatus};

    fn tag_compare_result(title: Option<&str>, format: Option<AudioFormat>) -> TagCompareResult {
        TagCompareResult {
            path: "/music/artist/release/track.mp3".into(),
            rows: vec![ComparisonRow {
                field: "Title",
                source_value: None,
                tag_value: title.map(str::to_string),
                status: ComparisonStatus::Match,
            }],
            file_image: None,
            contributors: Vec::new(),
            value_routes: Vec::new(),
            id3_fields: Vec::new(),
            total_tracks: None,
            format,
        }
    }

    #[test]
    fn file_header_vm_uses_title_row_when_present() {
        let result = tag_compare_result(Some("Tagged title"), Some(AudioFormat::Mp3));
        let vm = FileHeaderVm::new(&result);

        assert_eq!(vm.badge_label, "Embedded MP3");
        assert_eq!(vm.title, "Tagged title");
        assert_eq!(vm.path, "/music/artist/release/track.mp3");
    }

    #[test]
    fn file_header_vm_falls_back_to_embedded_format_label() {
        let result = tag_compare_result(Some(""), Some(AudioFormat::Flac));
        let vm = FileHeaderVm::new(&result);

        assert_eq!(vm.badge_label, "Embedded FLAC");
        assert_eq!(vm.title, "Embedded FLAC");
    }

    #[test]
    fn file_header_vm_falls_back_to_generic_embedded_tags() {
        let result = tag_compare_result(None, None);
        let vm = FileHeaderVm::new(&result);

        assert_eq!(vm.badge_label, "Embedded tags");
        assert_eq!(vm.title, "Embedded tags");
    }

    #[test]
    fn value_route_recipient_label_trims_and_falls_back() {
        assert_eq!(
            value_route_recipient_label(&serde_json::json!({"recipient_name": " Alice "})),
            "Alice"
        );
        assert_eq!(
            value_route_recipient_label(&serde_json::json!({"recipient_name": "   "})),
            "Unknown"
        );
        assert_eq!(
            value_route_recipient_label(&serde_json::json!({})),
            "Unknown"
        );
    }
}
