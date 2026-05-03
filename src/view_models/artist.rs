//! Artist inspector view-model.
//!
//! Pure projection of [`ArtistView`] + a hydrated `&[Feed]` slice into
//! the display-ready strings the artist inspector renders. No GPUI
//! imports, no service calls — see [`super`] for the layer rules.
//!
//! The shell (`ui::shells::artist::render_artist_view`) constructs an
//! [`ArtistVm`] each render and asks it for the title, subtitle, the
//! formatted track-count label, and the ordered list of detail rows.
//! Every accessor is `const`-style pure and is unit-tested below.

#![warn(clippy::pedantic)]

use crate::api::Feed;
use crate::views::ArtistView;

/// Display-ready projection of an [`ArtistView`].
///
/// Holds short-lived borrows of the screen's owned data; constructed
/// fresh each render. No allocations on the hot path beyond the
/// `String`s the formatted accessors return.
pub struct ArtistVm<'a> {
    view: &'a ArtistView,
    feeds: &'a [Feed],
    has_more_tracks: bool,
    track_count_override: Option<i32>,
}

/// One key/value entry to render in the inspector's detail grid. The
/// screen maps each entry to a `composites::DetailRow` at render time;
/// keeping the projection as plain data lets the VM stay GPUI-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailEntry {
    pub key: &'static str,
    pub value: String,
    /// Maximum number of lines the renderer should show before
    /// truncating with an ellipsis. Mirrors the existing
    /// `composites::DetailRow::text` contract.
    pub max_lines: usize,
}

impl<'a> ArtistVm<'a> {
    #[must_use]
    pub fn new(
        view: &'a ArtistView,
        feeds: &'a [Feed],
        has_more_tracks: bool,
        track_count_override: Option<i32>,
    ) -> Self {
        Self {
            view,
            feeds,
            has_more_tracks,
            track_count_override,
        }
    }

    /// Display title — the artist's name, or "Unknown Artist" if the
    /// upstream record had no name.
    #[must_use]
    pub fn title(&self) -> String {
        self.view
            .name
            .clone()
            .unwrap_or_else(|| "Unknown Artist".to_string())
    }

    /// Static subtitle for the inspector header.
    #[must_use]
    pub fn subtitle(&self) -> &'static str {
        "Feeds with tracks by this artist"
    }

    /// Resolved track count: the explicit `track_count_override`
    /// (passed by the screen during discover-stage hydration) wins
    /// over the artist record, falling back to `0`.
    #[must_use]
    pub fn display_track_count(&self) -> i32 {
        self.track_count_override
            .or(self.view.track_count)
            .unwrap_or(0)
    }

    /// Formatted track count, appending `+` when the screen knows
    /// there are more tracks beyond the loaded set.
    #[must_use]
    pub fn track_count_label(&self) -> String {
        let suffix = if self.has_more_tracks { "+" } else { "" };
        format!("{}{}", self.display_track_count(), suffix)
    }

    /// Number of feeds in the inspector's "Feeds" section.
    #[must_use]
    pub fn feed_count(&self) -> usize {
        self.feeds.len()
    }

    /// Whether the screen should render the "Feeds" list section at
    /// all. Avoids the screen having to repeat the `is_empty` check.
    #[must_use]
    pub fn has_feeds(&self) -> bool {
        !self.feeds.is_empty()
    }

    /// All detail rows the inspector should render, in display order.
    /// Optional fields with `None` or empty strings are filtered out.
    #[must_use]
    pub fn detail_rows(&self) -> Vec<DetailEntry> {
        let mut rows: Vec<DetailEntry> = Vec::with_capacity(6);
        rows.push(DetailEntry {
            key: "Tracks",
            value: self.track_count_label(),
            max_lines: 1,
        });
        rows.push(DetailEntry {
            key: "Feeds",
            value: self.feed_count().to_string(),
            max_lines: 1,
        });
        push_optional(&mut rows, "Sort Name", self.view.sort_name.clone(), 6);
        push_optional(&mut rows, "Area", self.view.area.clone(), 6);
        push_optional(
            &mut rows,
            "Active",
            artist_active_years(self.view.begin_year, self.view.end_year),
            6,
        );
        push_optional(&mut rows, "Website", self.view.url.clone(), 6);
        rows
    }
}

fn push_optional(
    rows: &mut Vec<DetailEntry>,
    key: &'static str,
    value: Option<String>,
    max_lines: usize,
) {
    if let Some(value) = value {
        if !value.is_empty() {
            rows.push(DetailEntry {
                key,
                value,
                max_lines,
            });
        }
    }
}

fn artist_active_years(begin_year: Option<i32>, end_year: Option<i32>) -> Option<String> {
    match (begin_year, end_year) {
        (Some(begin), Some(end)) => Some(format!("{begin}-{end}")),
        (Some(begin), None) => Some(format!("{begin}-")),
        (None, Some(end)) => Some(format!("until {end}")),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_view() -> ArtistView {
        ArtistView::default()
    }

    #[test]
    fn title_falls_back_when_name_is_missing() {
        let view = empty_view();
        let vm = ArtistVm::new(&view, &[], false, None);
        assert_eq!(vm.title(), "Unknown Artist");
    }

    #[test]
    fn title_uses_view_name_when_present() {
        let view = ArtistView {
            name: Some("Aphex Twin".into()),
            ..ArtistView::default()
        };
        let vm = ArtistVm::new(&view, &[], false, None);
        assert_eq!(vm.title(), "Aphex Twin");
    }

    #[test]
    fn track_count_override_wins_over_view_count() {
        let view = ArtistView {
            track_count: Some(3),
            ..ArtistView::default()
        };
        let vm = ArtistVm::new(&view, &[], false, Some(7));
        assert_eq!(vm.display_track_count(), 7);
        assert_eq!(vm.track_count_label(), "7");
    }

    #[test]
    fn track_count_falls_back_to_zero_when_both_missing() {
        let view = empty_view();
        let vm = ArtistVm::new(&view, &[], false, None);
        assert_eq!(vm.display_track_count(), 0);
        assert_eq!(vm.track_count_label(), "0");
    }

    #[test]
    fn track_count_label_appends_plus_when_more_available() {
        let view = ArtistView {
            track_count: Some(12),
            ..ArtistView::default()
        };
        let vm = ArtistVm::new(&view, &[], true, None);
        assert_eq!(vm.track_count_label(), "12+");
    }

    #[test]
    fn detail_rows_include_only_present_optional_fields() {
        let view = ArtistView {
            name: Some("Boards of Canada".into()),
            sort_name: Some("Boards of Canada".into()),
            area: None,
            url: Some(String::new()),
            ..ArtistView::default()
        };
        let vm = ArtistVm::new(&view, &[], false, None);
        let keys: Vec<&'static str> = vm.detail_rows().iter().map(|r| r.key).collect();
        assert_eq!(keys, vec!["Tracks", "Feeds", "Sort Name"]);
    }

    #[test]
    fn detail_rows_active_years_formats() {
        let cases = [
            (Some(1995), Some(2010), Some("1995-2010".to_string())),
            (Some(1995), None, Some("1995-".to_string())),
            (None, Some(2010), Some("until 2010".to_string())),
            (None, None, None),
        ];
        for (begin, end, expected) in cases {
            let view = ArtistView {
                begin_year: begin,
                end_year: end,
                ..ArtistView::default()
            };
            let vm = ArtistVm::new(&view, &[], false, None);
            let active = vm
                .detail_rows()
                .into_iter()
                .find(|r| r.key == "Active")
                .map(|r| r.value);
            assert_eq!(active, expected, "begin={begin:?} end={end:?}");
        }
    }

    #[test]
    fn has_feeds_reflects_slice_emptiness() {
        let view = empty_view();
        let vm_empty = ArtistVm::new(&view, &[], false, None);
        assert!(!vm_empty.has_feeds());
        assert_eq!(vm_empty.feed_count(), 0);
    }

    #[test]
    fn detail_rows_max_lines_match_legacy_render() {
        let view = ArtistView {
            sort_name: Some("Foo".into()),
            ..ArtistView::default()
        };
        let vm = ArtistVm::new(&view, &[], false, None);
        let rows = vm.detail_rows();
        for row in &rows {
            let expected = if row.key == "Tracks" || row.key == "Feeds" {
                1
            } else {
                6
            };
            assert_eq!(row.max_lines, expected, "row {} max_lines", row.key);
        }
    }
}
