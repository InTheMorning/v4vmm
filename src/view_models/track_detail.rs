//! Shared track surface display contract.
//!
//! This module owns the GPUI-free facts used by track rows, inspector panes,
//! and full-detail track surfaces. Screens resolve artwork and wire commands;
//! this module decides labels, fallbacks, row projection, and slot shape.

#![warn(clippy::pedantic)]

use crate::view_models::format::fmt_date;
use crate::view_models::track::fmt_dur;
use crate::view_models::track_metadata_grid::TrackMetadataGridVm;
use crate::views::{TrackRef, TrackView};

const UNTITLED: &str = "Untitled";
const UNKNOWN_ARTIST: &str = "Unknown Artist";
const UNKNOWN_ALBUM: &str = "Unknown Album";
const TRACK_KIND: &str = "track";

/// Surface requesting track display facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackDetailSurfaceContext {
    Library,
    Discover,
}

/// Loading lifecycle for a track surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrackDetailLoadState {
    Loaded,
    Loading,
    Missing,
    Failed { reason: String },
}

/// Canonical user-facing labels for track surfaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackDetailLabels {
    context: TrackDetailSurfaceContext,
}

impl TrackDetailLabels {
    #[must_use]
    pub const fn new(context: TrackDetailSurfaceContext) -> Self {
        Self { context }
    }

    #[must_use]
    pub const fn release_label(self) -> &'static str {
        match self.context {
            TrackDetailSurfaceContext::Library | TrackDetailSurfaceContext::Discover => "Release",
        }
    }

    #[must_use]
    pub const fn artist_label(self) -> &'static str {
        "Artist"
    }

    #[must_use]
    pub const fn track_number_label(self) -> &'static str {
        "Track #"
    }

    #[must_use]
    pub const fn duration_label(self) -> &'static str {
        "Duration"
    }

    #[must_use]
    pub const fn release_date_label(self) -> &'static str {
        "Release Date"
    }

    #[must_use]
    pub const fn publisher_label(self) -> &'static str {
        "Publisher"
    }

    #[must_use]
    pub const fn description_label(self) -> &'static str {
        "Description"
    }

    #[must_use]
    pub const fn summary_section_title(self) -> &'static str {
        "Tags"
    }
}

/// Display-ready key/value row for summary metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackDetailSummaryRow {
    pub label: String,
    pub value: String,
    pub max_lines: usize,
}

impl TrackDetailSummaryRow {
    #[must_use]
    pub fn new(label: impl Into<String>, value: impl Into<String>, max_lines: usize) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            max_lines,
        }
    }
}

/// Display-ready projection of one track detail surface.
#[derive(Debug)]
pub struct TrackDetailVm<'a> {
    track: &'a TrackView,
    context: TrackDetailSurfaceContext,
    override_title: Option<&'a str>,
}

impl<'a> TrackDetailVm<'a> {
    #[must_use]
    pub const fn new(track: &'a TrackView, context: TrackDetailSurfaceContext) -> Self {
        Self {
            track,
            context,
            override_title: None,
        }
    }

    #[must_use]
    pub const fn with_override_title(mut self, title: Option<&'a str>) -> Self {
        self.override_title = title;
        self
    }

    #[must_use]
    pub const fn track(&self) -> &'a TrackView {
        self.track
    }

    #[must_use]
    pub const fn context(&self) -> TrackDetailSurfaceContext {
        self.context
    }

    #[must_use]
    pub const fn labels(&self) -> TrackDetailLabels {
        TrackDetailLabels::new(self.context)
    }

    #[must_use]
    pub fn row(&self) -> TrackRowVm {
        TrackRowVm::from_detail(self)
    }

    #[must_use]
    pub fn display_title(&self) -> String {
        self.override_title
            .and_then(nonempty)
            .map(str::to_owned)
            .or_else(|| {
                self.track
                    .title
                    .as_deref()
                    .and_then(nonempty)
                    .map(str::to_owned)
            })
            .or_else(|| {
                self.track
                    .track_guid
                    .as_deref()
                    .and_then(nonempty)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| UNTITLED.to_string())
    }

    #[must_use]
    pub fn display_artist(&self) -> String {
        self.track
            .artist
            .as_deref()
            .and_then(nonempty)
            .map_or_else(|| UNKNOWN_ARTIST.to_string(), str::to_owned)
    }

    #[must_use]
    pub fn display_album(&self) -> String {
        self.track
            .album
            .as_deref()
            .and_then(nonempty)
            .or_else(|| self.track.feed_title.as_deref().and_then(nonempty))
            .map_or_else(|| UNKNOWN_ALBUM.to_string(), str::to_owned)
    }

    #[must_use]
    pub fn display_release_context(&self) -> String {
        self.display_album()
    }

    #[must_use]
    pub const fn display_kind_badge(&self) -> &'static str {
        TRACK_KIND
    }

    #[must_use]
    pub fn track_number_display(&self) -> Option<String> {
        self.track.track_number.map(|number| number.to_string())
    }

    #[must_use]
    pub fn duration_display(&self) -> Option<String> {
        self.track.duration_secs.map(fmt_dur)
    }

    #[must_use]
    pub fn release_date_display(&self) -> Option<String> {
        self.track.pub_date.and_then(fmt_date)
    }

    #[must_use]
    pub fn publisher_display(&self) -> Option<String> {
        self.track
            .publisher_text
            .as_deref()
            .and_then(nonempty)
            .map(str::to_owned)
    }

    #[must_use]
    pub fn description(&self) -> Option<String> {
        self.track
            .description
            .as_deref()
            .and_then(nonempty)
            .map(str::to_owned)
    }

    #[must_use]
    pub fn summary_rows(&self) -> Vec<TrackDetailSummaryRow> {
        let labels = self.labels();
        let mut rows = vec![TrackDetailSummaryRow::new(
            labels.release_label(),
            self.display_release_context(),
            3,
        )];
        push_optional(
            &mut rows,
            labels.track_number_label(),
            self.track_number_display(),
            1,
        );
        push_optional(
            &mut rows,
            labels.duration_label(),
            self.duration_display(),
            1,
        );
        push_optional(
            &mut rows,
            labels.release_date_label(),
            self.release_date_display(),
            1,
        );
        push_optional(
            &mut rows,
            labels.publisher_label(),
            self.publisher_display(),
            3,
        );
        rows
    }
}

/// Row-shaped projection of [`TrackDetailVm`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackRowVm {
    pub element_key: String,
    pub number: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub duration: Option<String>,
}

impl TrackRowVm {
    #[must_use]
    pub fn from_detail(detail: &TrackDetailVm<'_>) -> Self {
        Self {
            element_key: track_element_key(detail.track),
            number: detail
                .track
                .track_number
                .map_or_else(|| "\u{00B7}".to_string(), |number| number.to_string()),
            title: detail.display_title(),
            subtitle: Some(detail.display_artist()).filter(|artist| artist != UNKNOWN_ARTIST),
            duration: detail.duration_display(),
        }
    }
}

/// Typed non-artwork slots accepted by the shared track detail surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackDetailSlots {
    pub primary_actions: Vec<ActionRowItem>,
    pub summary_metadata: Option<TrackMetadataGridVm>,
    pub sections: Vec<TrackDetailSection>,
    pub advanced_panels: Vec<TrackDetailAdvancedPanel>,
    pub back_navigation: Option<NavigationContext>,
    pub external_links: Vec<ExternalLinkItem>,
    pub contributors: Vec<ContributorItem>,
    pub value_routes: Vec<ValueRouteItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRowItem {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalLinkItem {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContributorItem {
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueRouteItem {
    pub recipient: String,
    pub split: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackDetailSection {
    pub id: String,
    pub label: String,
    pub empty_label: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackDetailAdvancedPanel {
    pub id: String,
    pub label: String,
    pub empty_label: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationContext {
    pub label: String,
}

fn push_optional(
    rows: &mut Vec<TrackDetailSummaryRow>,
    label: &str,
    value: Option<String>,
    max_lines: usize,
) {
    if let Some(value) = value.and_then(|value| nonempty(&value).map(str::to_owned)) {
        rows.push(TrackDetailSummaryRow::new(label, value, max_lines));
    }
}

fn track_element_key(track: &TrackView) -> String {
    match &track.id {
        Some(TrackRef::Musicindex(id)) => format!("musicindex:{id}"),
        Some(TrackRef::LocalTrackId(id)) => format!("local:{id}"),
        None => track.track_guid.as_deref().and_then(nonempty).map_or_else(
            || "track:unknown".to_string(),
            |guid| format!("guid:{guid}"),
        ),
    }
}

fn nonempty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::TrackRef;

    fn track() -> TrackView {
        TrackView {
            id: Some(TrackRef::Musicindex("t1".to_string())),
            track_guid: Some("guid-1".to_string()),
            feed_title: Some("Release title".to_string()),
            title: Some("Track title".to_string()),
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            track_number: Some(7),
            duration_secs: Some(125),
            pub_date: Some(1_712_275_200),
            publisher_text: Some("Publisher".to_string()),
            description: Some("Description".to_string()),
            ..TrackView::default()
        }
    }

    #[test]
    fn display_title_prefers_nonempty_override() {
        let track = track();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_override_title(Some("Override"));

        assert_eq!(vm.display_title(), "Override");
    }

    #[test]
    fn display_title_falls_back_to_track_guid_then_untitled() {
        let mut track = track();
        track.title = None;
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);
        assert_eq!(vm.display_title(), "guid-1");

        track.track_guid = None;
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);
        assert_eq!(vm.display_title(), "Untitled");
    }

    #[test]
    fn display_artist_and_album_have_canonical_fallbacks() {
        let mut track = track();
        track.artist = None;
        track.album = None;
        track.feed_title = None;
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);

        assert_eq!(vm.display_artist(), "Unknown Artist");
        assert_eq!(vm.display_album(), "Unknown Album");
    }

    #[test]
    fn summary_rows_use_canonical_label_order() {
        let track = track();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);
        let rows = vm.summary_rows();

        assert_eq!(
            rows.iter()
                .map(|row| (row.label.as_str(), row.value.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Release", "Album"),
                ("Track #", "7"),
                ("Duration", "2:05"),
                ("Release Date", "Apr 5, 2024"),
                ("Publisher", "Publisher"),
            ]
        );
    }

    #[test]
    fn row_projection_is_subset_of_detail_contract() {
        let track = track();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);
        let row = vm.row();

        assert_eq!(row.element_key, "musicindex:t1");
        assert_eq!(row.number, "7");
        assert_eq!(row.title, "Track title");
        assert_eq!(row.subtitle.as_deref(), Some("Artist"));
        assert_eq!(row.duration.as_deref(), Some("2:05"));
    }

    #[test]
    fn load_state_can_represent_failure() {
        let state = TrackDetailLoadState::Failed {
            reason: "missing".to_string(),
        };

        assert_eq!(
            state,
            TrackDetailLoadState::Failed {
                reason: "missing".to_string()
            }
        );
    }
}
