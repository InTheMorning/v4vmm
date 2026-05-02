//! Track metadata-grid view-model.
//!
//! Owns the shared presentation contract for the compare grid: visible
//! columns, heading labels, heading indentation, and expansion-key lookup.
//! Screens keep GPUI cell rendering because drag/drop and edit callbacks are
//! app-specific.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

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

impl TrackMetadataGridVm {
    #[must_use]
    pub fn tag_column_label(format_label: Option<&str>) -> &str {
        format_label
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .unwrap_or("Tags")
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
