//! Name-match track page view model (ADR 0077 packet 006, Accepted
//! Refinement "Index artist page by name").
//!
//! The Index name query `/v1/tracks?artist=<name>` gives a search result,
//! not an artist. `src/application/queries/search.rs` builds one
//! [`NameMatchPageFacts`] value from that query. This module turns it into
//! the page's title, its track rows, and its load state. It gives no
//! artist identity, no role, and no page type (ADR 0077 Decision 1).

#![warn(clippy::pedantic)]

use crate::api::Track;
use crate::view_models::search_results::{SearchResultOrigin, TrackResultDisplay};
use crate::views::TrackView;

/// The raw facts one name-match track page query gathers.
#[derive(Clone, Debug, Default)]
pub(crate) struct NameMatchPageFacts {
    /// The name the operator searched for.
    pub(crate) name: String,
    /// Each track `MusicIndex` gave for this exact name.
    pub(crate) tracks: Vec<Track>,
    /// `true` when `MusicIndex` has more tracks than this page shows. This
    /// packet adds no paging; the page only states the fact (Required
    /// Change 3).
    pub(crate) has_more: bool,
}

/// The name-match track page view model (ADR 0077 packet 006).
pub(crate) struct NameMatchPageVm {
    facts: NameMatchPageFacts,
}

impl NameMatchPageVm {
    /// The message the page shows when `MusicIndex` has more tracks than it
    /// lists (R6-05). This packet adds no paging.
    pub(crate) const MORE_TRACKS_MESSAGE: &'static str =
        "MusicIndex has more tracks for this name. This page does not list every track yet.";

    #[must_use]
    pub(crate) const fn new(facts: NameMatchPageFacts) -> Self {
        Self { facts }
    }

    /// The page title text (Required Changes 2 and 3): the same words the
    /// row label uses.
    #[must_use]
    pub(crate) fn title_text(&self) -> String {
        Self::title_text_for_name(&self.facts.name)
    }

    /// The title text for a name, apart from a loaded page. A row label and
    /// a loading message both need this text before a fetch completes.
    #[must_use]
    pub(crate) fn title_text_for_name(name: &str) -> String {
        format!("Tracks matching \"{name}\"")
    }

    /// One row for each returned track (R6-04). Each row's `id` is the
    /// activation id the existing Index track detail route reads.
    #[must_use]
    pub(crate) fn rows(&self) -> Vec<TrackResultDisplay> {
        self.facts.tracks.iter().map(Self::row).collect()
    }

    /// `true` when `MusicIndex` gave no track for this exact name (R6-05,
    /// Visual V3).
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.facts.tracks.is_empty()
    }

    /// The message the page shows for an empty answer (R6-05, Visual V3).
    #[must_use]
    pub(crate) fn no_track_message(&self) -> String {
        format!(
            "MusicIndex gave no track for the exact name \"{}\".",
            self.facts.name
        )
    }

    /// `true` when `MusicIndex` has more tracks than this page shows (R6-05).
    #[must_use]
    pub(crate) const fn has_more(&self) -> bool {
        self.facts.has_more
    }

    fn row(track: &Track) -> TrackResultDisplay {
        let track_guid = track.track_guid.clone().unwrap_or_default();
        let feed_guid = track
            .feed_guid
            .as_deref()
            .map(str::trim)
            .filter(|guid| !guid.is_empty());
        let target = feed_guid.map_or_else(
            || track_guid.clone(),
            |feed_guid| format!("{feed_guid}:{track_guid}"),
        );
        let id = format!("index-track:{target}");
        let label = track.title.clone().unwrap_or_else(|| track_guid.clone());
        let mut display = TrackResultDisplay::new(id, label, SearchResultOrigin::Index);
        let secondary = [track.track_artist.clone(), track.feed_title.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" - ");
        if !secondary.is_empty() {
            display = display.with_secondary_text(secondary);
        }
        if let Some(image_url) = track.image_url.clone() {
            display = display.with_thumbnail_href(image_url);
        }
        display.with_remote_track(TrackView::from_api(track.clone()))
    }
}

/// Display state of the mounted name-match page while it loads, when its
/// fetch fails, or when none is open (R6-05). The app layer only selects
/// which state applies; this module decides its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NameMatchPageLoadDisplay {
    Loading {
        message: String,
    },
    Failed {
        report: &'static str,
        detail: String,
    },
    Empty {
        message: String,
    },
}

impl NameMatchPageLoadDisplay {
    /// The report for a name-match page that failed to load.
    pub(crate) const FAILED_REPORT: &'static str =
        "The app could not load these tracks from MusicIndex.";

    /// The message shown when no name-match page is open.
    pub(crate) const EMPTY_MESSAGE: &'static str = "No name-match track page is open.";

    /// The display for a name-match page in flight.
    #[must_use]
    pub(crate) fn loading(name: &str) -> Self {
        Self::Loading {
            message: format!("Loading tracks matching \"{name}\"..."),
        }
    }

    /// The display for a name-match page whose fetch failed. `detail` is
    /// the transport error, already turned into text by the app layer.
    #[must_use]
    pub(crate) fn failed(detail: impl Into<String>) -> Self {
        Self::Failed {
            report: Self::FAILED_REPORT,
            detail: detail.into(),
        }
    }

    /// The display for a frame with no mounted name-match page.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self::Empty {
            message: Self::EMPTY_MESSAGE.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(track_guid: &str, feed_guid: Option<&str>, title: &str) -> Track {
        Track {
            track_guid: Some(track_guid.to_string()),
            feed_guid: feed_guid.map(str::to_string),
            title: Some(title.to_string()),
            ..Track::default()
        }
    }

    fn facts(name: &str, tracks: Vec<Track>) -> NameMatchPageFacts {
        NameMatchPageFacts {
            name: name.to_string(),
            tracks,
            has_more: false,
        }
    }

    /// R6-01: the row label and the page title use the same quoted words.
    #[test]
    fn adr_0077_name_matches_title_text_quotes_the_name() {
        assert_eq!(
            NameMatchPageVm::title_text_for_name("Survival Guide"),
            "Tracks matching \"Survival Guide\""
        );
    }

    /// R6-04: one row for each returned track, and each row's id is the
    /// activation id the existing Index track detail route reads
    /// (`index-track:<feed_guid>:<track_guid>`, or `index-track:<track_guid>`
    /// with no feed GUID).
    #[test]
    fn adr_0077_name_matches_rows_carry_the_existing_track_detail_activation_id() {
        let vm = NameMatchPageVm::new(facts(
            "DETOX",
            vec![
                track("t1", Some("f1"), "Track One"),
                track("t2", None, "Track Two"),
            ],
        ));

        let rows = vm.rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "index-track:f1:t1");
        assert_eq!(rows[0].label, "Track One");
        assert_eq!(rows[1].id, "index-track:t2");
        assert_eq!(rows[1].label, "Track Two");
        assert!(
            rows[0].remote_track.is_some(),
            "a row keeps its full track detail so the existing Index track detail route can show it"
        );
    }

    /// R6-05: an empty answer gives the empty state message, naming the
    /// exact searched name.
    #[test]
    fn adr_0077_name_matches_empty_answer_names_the_exact_name() {
        let vm = NameMatchPageVm::new(facts("DETOX", Vec::new()));
        assert!(vm.is_empty());
        assert_eq!(
            vm.no_track_message(),
            "MusicIndex gave no track for the exact name \"DETOX\"."
        );
    }

    /// R6-05: a failed fetch gives a report first, and its technical detail
    /// after it, apart from the report.
    #[test]
    fn adr_0077_name_matches_load_display_failed_separates_report_and_detail() {
        let display = NameMatchPageLoadDisplay::failed("HTTP 503 from /v1/tracks");
        assert_eq!(
            display,
            NameMatchPageLoadDisplay::Failed {
                report: NameMatchPageLoadDisplay::FAILED_REPORT,
                detail: "HTTP 503 from /v1/tracks".into(),
            }
        );
    }

    /// R6-05: a page with more tracks than it lists states that fact.
    #[test]
    fn adr_0077_name_matches_more_tracks_state_is_named() {
        let mut page_facts = facts("DETOX", vec![track("t1", Some("f1"), "Track One")]);
        page_facts.has_more = true;
        let vm = NameMatchPageVm::new(page_facts);
        assert!(vm.has_more());
        assert!(!NameMatchPageVm::MORE_TRACKS_MESSAGE.is_empty());
    }
}
