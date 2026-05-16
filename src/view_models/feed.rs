//! Feed inspector view-model.
//!
//! Pure projection of [`FeedView`] + a hydrated `&[Track]` slice into
//! the strings, sorted track list, and detail-row entries the feed
//! inspector renders. Same rules as [`super`]: no GPUI imports, no
//! service mutation.
//!
//! Feed identity facts such as publisher, website, Nostr, and description are
//! projected by `view_models::entity_detail` into the shared header. This
//! module keeps the feed-specific detail-grid and track-list projections.

#![warn(clippy::pedantic)]

use crate::api::{Feed, Track};
use crate::view_models::format::{fmt_date, fmt_runtime};
use crate::views::{contributor_views_to_api, FeedView};

/// Display-ready projection of a [`FeedView`].
pub struct FeedVm<'a> {
    view: &'a FeedView,
    tracks: &'a [Track],
    text_filter: Option<String>,
}

/// One scalar key/value entry in the feed-inspector detail grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailEntry {
    pub key: &'static str,
    pub value: String,
}

impl<'a> FeedVm<'a> {
    #[must_use]
    pub fn new(view: &'a FeedView, tracks: &'a [Track]) -> Self {
        Self {
            view,
            tracks,
            text_filter: None,
        }
    }

    pub fn set_text_filter(&mut self, filter: Option<String>) {
        self.text_filter = filter
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty());
    }

    #[must_use]
    pub fn text_filter(&self) -> Option<&str> {
        self.text_filter.as_deref()
    }

    #[must_use]
    pub fn title(&self) -> String {
        self.view
            .title
            .clone()
            .unwrap_or_else(|| "Unknown Feed".to_string())
    }

    #[must_use]
    pub fn artist_label(&self) -> String {
        self.view
            .artist
            .clone()
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Trimmed publisher text, or `None` when the underlying field is
    /// missing or whitespace-only. Callers render either an
    /// interactive link (when `Some`) or the literal string
    /// `"Unknown"` (when `None`).
    #[must_use]
    pub fn publisher_text(&self) -> Option<String> {
        self.view
            .publisher_text
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    /// Scalar detail entries in display order: Release Kind, Release
    /// Date (when known), Language (when known), Explicit (only when
    /// `true`), and Tracks (when known).
    #[must_use]
    pub fn scalar_detail_entries(&self) -> Vec<DetailEntry> {
        let mut rows: Vec<DetailEntry> = Vec::with_capacity(5);
        rows.push(DetailEntry {
            key: "Release Kind",
            value: self
                .view
                .release_kind
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
        });
        if let Some(date) = self.view.release_date.and_then(fmt_date) {
            rows.push(DetailEntry {
                key: "Release Date",
                value: date,
            });
        }
        if let Some(lang) = self
            .view
            .language
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
        {
            rows.push(DetailEntry {
                key: "Language",
                value: lang,
            });
        }
        if self.view.explicit == Some(true) {
            rows.push(DetailEntry {
                key: "Explicit",
                value: "Yes".to_string(),
            });
        }
        if let Some(count) = self.view.episode_count {
            rows.push(DetailEntry {
                key: "Tracks",
                value: count.to_string(),
            });
        }
        rows
    }

    /// Tracks sorted for inspector display: ascending track number
    /// (missing numbers sort last), tie-broken by descending pub
    /// date. Returns a fresh `Vec<Track>` because the legacy renderer
    /// owns the sorted list.
    #[must_use]
    pub fn sorted_tracks(&self) -> Vec<Track> {
        let mut sorted: Vec<Track> = self
            .tracks
            .iter()
            .filter(|track| self.track_matches_text_filter(track))
            .cloned()
            .collect();
        sorted.sort_by(|a, b| {
            let a_num = a.track_number.unwrap_or(i32::MAX);
            let b_num = b.track_number.unwrap_or(i32::MAX);
            a_num.cmp(&b_num).then_with(|| {
                b.pub_date
                    .unwrap_or_default()
                    .cmp(&a.pub_date.unwrap_or_default())
            })
        });
        sorted
    }

    fn track_matches_text_filter(&self, track: &Track) -> bool {
        self.text_filter.as_deref().is_none_or(|filter| {
            [
                track.title.as_deref(),
                track.name.as_deref(),
                track.feed_title.as_deref(),
                track.track_artist.as_deref(),
                track.release_artist.as_deref(),
                track.publisher_text.as_deref(),
                track.description.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(|value| value.to_lowercase().contains(filter))
        })
    }

    /// Sum of every track's known duration in seconds.
    #[must_use]
    pub fn total_runtime_secs(&self) -> i32 {
        self.tracks.iter().filter_map(|t| t.duration_secs).sum()
    }

    /// `"N total"` or `"N total · H h M min"` when there is non-zero
    /// runtime. Matches the legacy summary string exactly.
    #[must_use]
    pub fn track_list_summary(&self) -> String {
        let count = self.tracks.len();
        let total = self.total_runtime_secs();
        if total > 0 {
            format!("{count} total · {}", fmt_runtime(total))
        } else {
            format!("{count} total")
        }
    }

    #[must_use]
    pub fn has_tracks(&self) -> bool {
        !self.tracks.is_empty()
    }

    #[must_use]
    pub fn description(&self) -> Option<String> {
        self.view.description.clone()
    }

    /// Pure projection of [`FeedView`] into the legacy [`Feed`] shape
    /// the existing renderer helpers expect. Replaces the inline
    /// `feed_view_to_api` helper that used to live in `ui_feed.rs`.
    #[must_use]
    pub fn header_feed(&self) -> Feed {
        Feed {
            feed_guid: self.view.feed_guid.clone(),
            feed_url: self.view.feed_url.clone(),
            title: self.view.title.clone(),
            name: self.view.title.clone(),
            release_artist: self.view.artist.clone(),
            image_url: self.view.image_url.clone(),
            release_date: self.view.release_date,
            language: self.view.language.clone(),
            explicit: self.view.explicit,
            episode_count: self.view.episode_count,
            release_kind: self.view.release_kind.clone(),
            publisher_text: self.view.publisher_text.clone(),
            description: self.view.description.clone(),
            payment_routes: Some(self.view.payment_routes.clone()),
            source_contributors: Some(contributor_views_to_api(&self.view.contributors)),
            ..Feed::default()
        }
    }

    /// Optional `Feed` shim used as the per-track parent feed when
    /// rendering the track list section. Returns `None` when neither
    /// guid nor url is known (the legacy renderer's exact rule).
    #[must_use]
    pub fn track_list_feed(&self) -> Option<Feed> {
        if self.view.feed_guid.is_some() || self.view.feed_url.is_some() {
            Some(Feed {
                feed_guid: self.view.feed_guid.clone(),
                feed_url: self.view.feed_url.clone(),
                title: self.view.title.clone(),
                ..Default::default()
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_view() -> FeedView {
        FeedView::default()
    }

    #[test]
    fn title_falls_back_when_missing() {
        let view = empty_view();
        let vm = FeedVm::new(&view, &[]);
        assert_eq!(vm.title(), "Unknown Feed");
        assert_eq!(vm.artist_label(), "Unknown");
    }

    #[test]
    fn publisher_text_trims_and_filters_empty() {
        let mk = |raw: &str| FeedView {
            publisher_text: Some(raw.into()),
            ..FeedView::default()
        };
        assert_eq!(FeedVm::new(&mk(""), &[]).publisher_text(), None);
        assert_eq!(FeedVm::new(&mk("   "), &[]).publisher_text(), None);
        assert_eq!(
            FeedVm::new(&mk("  Acme Records  "), &[]).publisher_text(),
            Some("Acme Records".to_string())
        );
        let none_view = empty_view();
        assert_eq!(FeedVm::new(&none_view, &[]).publisher_text(), None);
    }

    #[test]
    fn scalar_detail_entries_release_kind_unknown_when_missing() {
        let view = empty_view();
        let vm = FeedVm::new(&view, &[]);
        let rows = vm.scalar_detail_entries();
        assert_eq!(rows[0].key, "Release Kind");
        assert_eq!(rows[0].value, "Unknown");
    }

    #[test]
    fn scalar_detail_entries_only_include_known_optionals() {
        let view = FeedView {
            release_kind: Some("album".into()),
            language: None,
            explicit: Some(false),
            episode_count: None,
            release_date: None,
            ..FeedView::default()
        };
        let vm = FeedVm::new(&view, &[]);
        let keys: Vec<&'static str> = vm.scalar_detail_entries().iter().map(|r| r.key).collect();
        assert_eq!(keys, vec!["Release Kind"]);
    }

    #[test]
    fn scalar_detail_entries_explicit_only_when_true() {
        let mut view = FeedView {
            release_kind: Some("album".into()),
            ..FeedView::default()
        };
        view.explicit = Some(true);
        let vm = FeedVm::new(&view, &[]);
        let keys: Vec<&'static str> = vm.scalar_detail_entries().iter().map(|r| r.key).collect();
        assert_eq!(keys, vec!["Release Kind", "Explicit"]);
    }

    #[test]
    fn scalar_detail_entries_full_row_set() {
        let view = FeedView {
            release_kind: Some("album".into()),
            release_date: Some(1_712_275_200), // Apr 5, 2024
            language: Some("en".into()),
            explicit: Some(true),
            episode_count: Some(12),
            ..FeedView::default()
        };
        let vm = FeedVm::new(&view, &[]);
        let rows = vm.scalar_detail_entries();
        let pairs: Vec<(&'static str, &str)> =
            rows.iter().map(|r| (r.key, r.value.as_str())).collect();
        assert_eq!(
            pairs,
            vec![
                ("Release Kind", "album"),
                ("Release Date", "Apr 5, 2024"),
                ("Language", "en"),
                ("Explicit", "Yes"),
                ("Tracks", "12"),
            ]
        );
    }

    fn track(num: Option<i32>, dur: Option<i32>, pub_date: Option<i64>) -> Track {
        Track {
            track_number: num,
            duration_secs: dur,
            pub_date,
            ..Track::default()
        }
    }

    #[test]
    fn sorted_tracks_orders_by_track_number_then_pub_date_desc() {
        let tracks = vec![
            track(Some(2), None, Some(1)),
            track(Some(1), None, Some(1)),
            track(None, None, Some(5)),
            track(None, None, Some(50)),
        ];
        let view = empty_view();
        let vm = FeedVm::new(&view, &tracks);
        let nums: Vec<Option<i32>> = vm.sorted_tracks().iter().map(|t| t.track_number).collect();
        assert_eq!(nums, vec![Some(1), Some(2), None, None]);
        // Tie-broken by descending pub_date for the two None-numbered tracks.
        let dates: Vec<Option<i64>> = vm
            .sorted_tracks()
            .into_iter()
            .filter(|t| t.track_number.is_none())
            .map(|t| t.pub_date)
            .collect();
        assert_eq!(dates, vec![Some(50), Some(5)]);
    }

    #[test]
    fn text_filter_trims_normalizes_and_clears() {
        let view = empty_view();
        let mut vm = FeedVm::new(&view, &[]);
        assert_eq!(vm.text_filter(), None);

        vm.set_text_filter(Some("  Lead Singer  ".to_string()));
        assert_eq!(vm.text_filter(), Some("lead singer"));

        vm.set_text_filter(Some("   ".to_string()));
        assert_eq!(vm.text_filter(), None);

        vm.set_text_filter(Some("album".to_string()));
        vm.set_text_filter(None);
        assert_eq!(vm.text_filter(), None);
    }

    #[test]
    fn sorted_tracks_filters_by_display_track_text() {
        let tracks = vec![
            Track {
                track_number: Some(1),
                title: Some("Opening Song".to_string()),
                ..Track::default()
            },
            Track {
                track_number: Some(2),
                track_artist: Some("Lead Singer".to_string()),
                ..Track::default()
            },
            Track {
                track_number: Some(3),
                description: Some("studio outtake".to_string()),
                ..Track::default()
            },
        ];
        let view = empty_view();
        let mut vm = FeedVm::new(&view, &tracks);

        vm.set_text_filter(Some("singer".to_string()));

        let nums: Vec<Option<i32>> = vm.sorted_tracks().iter().map(|t| t.track_number).collect();
        assert_eq!(nums, vec![Some(2)]);
    }

    #[test]
    fn clearing_text_filter_restores_sorted_tracks() {
        let tracks = vec![
            Track {
                track_number: Some(2),
                title: Some("Filtered".to_string()),
                ..Track::default()
            },
            Track {
                track_number: Some(1),
                title: Some("Restored".to_string()),
                ..Track::default()
            },
        ];
        let view = empty_view();
        let mut vm = FeedVm::new(&view, &tracks);
        vm.set_text_filter(Some("filtered".to_string()));
        let filtered_nums: Vec<Option<i32>> =
            vm.sorted_tracks().iter().map(|t| t.track_number).collect();
        assert_eq!(filtered_nums, vec![Some(2)]);

        vm.set_text_filter(None);

        let restored_nums: Vec<Option<i32>> =
            vm.sorted_tracks().iter().map(|t| t.track_number).collect();
        assert_eq!(restored_nums, vec![Some(1), Some(2)]);
    }

    #[test]
    fn track_list_summary_with_and_without_runtime() {
        let view = empty_view();
        let no_dur = vec![track(None, None, None), track(None, None, None)];
        assert_eq!(FeedVm::new(&view, &no_dur).track_list_summary(), "2 total");
        let with_dur = vec![track(None, Some(3600), None), track(None, Some(60), None)];
        assert_eq!(
            FeedVm::new(&view, &with_dur).track_list_summary(),
            "2 total · 1 h 1 min"
        );
    }

    #[test]
    fn header_feed_mirrors_view_fields() {
        let view = FeedView {
            feed_guid: Some("g".into()),
            feed_url: Some("u".into()),
            title: Some("T".into()),
            artist: Some("A".into()),
            image_url: Some("i".into()),
            release_date: Some(42),
            language: Some("en".into()),
            explicit: Some(true),
            episode_count: Some(5),
            release_kind: Some("album".into()),
            publisher_text: Some("P".into()),
            description: Some("D".into()),
            ..FeedView::default()
        };
        let vm = FeedVm::new(&view, &[]);
        let f = vm.header_feed();
        assert_eq!(f.feed_guid.as_deref(), Some("g"));
        assert_eq!(f.feed_url.as_deref(), Some("u"));
        assert_eq!(f.title.as_deref(), Some("T"));
        assert_eq!(f.name.as_deref(), Some("T"));
        assert_eq!(f.release_artist.as_deref(), Some("A"));
        assert_eq!(f.image_url.as_deref(), Some("i"));
        assert_eq!(f.release_date, Some(42));
        assert_eq!(f.language.as_deref(), Some("en"));
        assert_eq!(f.explicit, Some(true));
        assert_eq!(f.episode_count, Some(5));
        assert_eq!(f.release_kind.as_deref(), Some("album"));
        assert_eq!(f.publisher_text.as_deref(), Some("P"));
        assert_eq!(f.description.as_deref(), Some("D"));
    }

    #[test]
    fn track_list_feed_some_when_guid_or_url_known() {
        let with_guid = FeedView {
            feed_guid: Some("g".into()),
            ..FeedView::default()
        };
        assert!(FeedVm::new(&with_guid, &[]).track_list_feed().is_some());

        let with_url = FeedView {
            feed_url: Some("u".into()),
            ..FeedView::default()
        };
        assert!(FeedVm::new(&with_url, &[]).track_list_feed().is_some());

        let none = empty_view();
        assert!(FeedVm::new(&none, &[]).track_list_feed().is_none());
    }
}
